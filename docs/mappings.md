# Mappings (`crates/mappings`)

**Status: exists (milestone M1).** `jarrow-mappings` reads and writes the mapping formats v1 needs, holds them in one model, and composes them into the chains a conversion uses. It does not remap class files; that is `crates/remap` (M2). Parchment is not built yet.

A mod JAR names Minecraft's classes, fields and methods in its loader's runtime namespace. Fabric (and Quilt) mods ship intermediary names. NeoForge runs on Mojang's names. Forge 1.20.1 runs on SRG names. Converting a mod rewrites every such name, and this crate builds the table for it.

## Rules the crate keeps

- **No I/O.** No `std::fs`, no network, no threads. The crate builds for `wasm32-unknown-unknown`. Hosts (the CLI, the board, the browser) fetch mapping files from the URLs pinned in [`research-lock.json`](../research-lock.json), check the hashes, and pass the bytes in.
- **Mojang's mappings never enter the repository.** Their license header allows development use and forbids redistributing the file ([research notes](research/fabric-neoforge-notes.md#1-names-at-runtime-mappings)). The test fixtures are hand-written and every name in them is invented.
- **Loaders and game versions are inputs.** The crate knows namespace *names*, not which loader runs on which namespace for which version. That comes from the loader profiles. Every function takes namespace names as strings. The [`ns`](../crates/mappings/src/ns.rs) module only spells the usual ones.
- **Malformed input is an `Err`, never a panic.** Parse errors give the format and the 1-based line number.
- **Deterministic.** Insertion order is kept everywhere. Hash maps are used for lookups only and are never iterated to produce output, so the same input always gives the same bytes.

## Namespaces

| Name (`ns::`) | What it is | Where it comes from |
| --- | --- | --- |
| `obf` (`OBF`) | Mojang's obfuscated names in the vanilla jars | the right side of Mojang's ProGuard files; intermediary's first column |
| `intermediary` (`INTERMEDIARY`) | Fabric's stable names: `net/minecraft/class_2960`, `field_N`, `method_N`, `comp_N` | intermediary's tiny v2 file |
| `mojang` (`MOJANG`) | Mojang's deobfuscated names: `net/minecraft/resources/ResourceLocation` | the left side of Mojang's ProGuard files |
| `srg` (`SRG`) | Forge's SRG names: `f_12345_`, `m_12345_` | MCPConfig's TSRG2 file |
| `named` (`NAMED`) | Yarn's names | Yarn's tiny v2 file |

Fabric calls the obfuscated column `official`. That is easy to confuse with Mojang's official (deobfuscated) names, which `docs/rules.md` also calls "official". So [`intermediary::load`](../crates/mappings/src/intermediary.rs) renames Fabric's `official` column to `obf`, and Mojang's names are always `mojang` here. A rule table's `names = "official"` corresponds to `mojang` in this crate.

## The model

A `MappingSet` has an ordered list of namespaces. The first one is **primary**.

- Every class, field, method and package has a name in the primary namespace, and the set is indexed by it.
- Member descriptors are stored in the primary namespace.
- A name in another namespace may be missing (`None`; an empty string in a file). It means the entry has no name there. It never means "same as the primary name".
- Parameters are keyed by local-variable index and local variables by (index, start offset, LVT row). Any of their names may be missing, the primary one included, because obfuscated code has no parameter names.
- Adding an entry whose key already exists merges the two. Missing names are filled in, and two different names for the same key are an `Error::Conflict`. This is how a ProGuard method listed once per line range becomes one method, and how `merge` joins Mojang's client and server files.

Lookups:

| Call | Keyed by |
| --- | --- |
| `set.class(name)` | primary class name |
| `set.field(owner, name, Some(desc))` / `set.field(owner, name, None)` | primary owner, name and optional descriptor. With `None` the first field of that name wins; `fields_named` lists them all |
| `set.method(owner, name, desc)` | primary owner, name and descriptor |
| `set.mapper(from, to)` | builds a `Mapper` that takes names and descriptors in `from` and answers in `to`: `map_class`, `map_field`, `map_method`, `map_field_desc`, `map_method_desc`, `map_signature`, `map_class_or_array` |

Transformations:

| Call | Result |
| --- | --- |
| `project(&[ns, ...])` | a subset or reordering of the namespaces. The first becomes primary. Descriptors are rewritten into it through the set's own class names. Entries with no name in the new primary namespace are left out, because they cannot be looked up from it |
| `with_primary(ns)` | `ns` first, the others in their order |
| `invert()` | the namespaces reversed, so `a -> b` becomes `b -> a` |
| `filter(\|entry\| ...)` | keeps the packages, classes, fields and methods the closure accepts. Dropping a class drops its members |
| `merge(&other)` | adds every entry of a set with the same namespaces in the same order |
| `rename_namespace(from, to)` | renames one namespace |
| `compose(a, b, shared)` | joins two sets on a namespace they share. The result's namespaces are `shared`, then `a`'s others, then `b`'s others. Entries match by name in `shared`, and members by name and descriptor in `shared`. An entry on one side only has no names in the other side's namespaces |
| `chain(source, target, &[sets])` | treats the sets as a graph of namespaces, takes the shortest path from `source` to `target` (ties go to the set listed first), composes along it, and projects to `(source, target)`. Sets off the path are ignored |

## The Fabric to NeoForge chain

```rust
let intermediary = intermediary::load(&tiny_bytes)?;            // (obf, intermediary)
let mojang = proguard::parse_bytes(&server_txt, "mojang", "obf")?; // (mojang, obf)
let set = chain("intermediary", "mojang", &[&intermediary, &mojang])?;
let mapper = set.mapper("intermediary", "mojang")?;
mapper.map_class("net/minecraft/class_2960"); // 1.21.1: net/minecraft/resources/ResourceLocation
```

`chain` composes intermediary's `(obf, intermediary)` with Mojang's `(mojang, obf)` through `obf` and projects to `(intermediary, mojang)` with intermediary descriptors.

The same call works for any pair the sets connect. With MCPConfig's TSRG2 file (`obf srg id`), `chain("intermediary", "srg", ...)` gives the Fabric to Forge chain on game versions where Forge runs on SRG. Which namespace a loader needs is the loader profile's decision, not this crate's.

Mojang's server file covers only the dedicated server's classes. For mods that touch client classes, merge the client file into the server file (`mojang.merge(&server)`) before the chain. On 1.21.1 that takes the number of intermediary classes without a Mojang name from 2,115 (server file only) to 0 (live test below).

Composition only knows declared members. A field or method that a class inherits is listed under the class that declares it, and resolving a reference through the class hierarchy is the remapper's job (M2).

## Global member map

Intermediary member names are unique on their own, so a reference can be remapped without its owner. That is what reflection strings and mixin targets need. `GlobalMemberMap::build_filtered(&set, "intermediary", "mojang", is_intermediary_member_name)` builds that map. It never answers for a name with two different targets. It records each such name as a `MemberConflict` that lists every target and the first owner that gave it, and `field(name)` / `method(name)` return `GlobalLookup::Conflict`.

The pinned files have such names. On both 1.21.1 and 1.21.11, the chained set has 8 conflicting intermediary names. One example is `field_28859`, which is `sleepStatus` in `class_3218` and `DEEPSLATE_BRICKS` in `class_1802`. Members without a name in the target namespace are skipped, not counted as conflicts. Obfuscated names are not unique at all, and the map reports that too (see `tests/chain.rs`).

## Formats

| Format | Module | Reads | Writes | Notes |
| --- | --- | --- | --- | --- |
| tiny v2 | `tiny2::{parse, parse_bytes, write}` | header properties, `escaped-names`, classes, fields, methods, parameters, local variables, comments at every level | everything it reads | Escapes `\\ \n \r \t \0`. Comments are always escaped; names and descriptors only under `escaped-names`, which the writer sets when a name needs it. Unknown sections are skipped with their children. Only version 2.0 is accepted |
| tiny v1 | `tiny1::{parse, parse_bytes, write}` | `CLASS`, `FIELD`, `METHOD`; `#` comments | names and descriptors | A member may come before its owner's `CLASS` line |
| ProGuard | `proguard::{parse, parse_bytes, parse_mojang, write, write_mojang}` | classes (inner classes with `$`), fields, methods with `start:end:` line ranges and `:origStart[:origEnd]` original lines; `#` and R8 JSON comments are skipped | names, line ranges, inline frames | Types are converted between Java source form and descriptors. A method listed once per line range becomes one method with several ranges. A method qualified with another class (`a.B.helper`) is inlined code, kept as an `InlineFrame` of the class but not as a member |
| TSRG | `tsrg::{parse_v1, parse_v1_bytes, write_v1}` | packages, classes, fields (no descriptor), methods | the same | No header; the caller names the two namespaces |
| TSRG2 | `tsrg::{parse_v2, parse_v2_bytes, write_v2}` | header, packages, classes, fields with or without a descriptor, methods, `static` markers, parameters | the same | Every name in every namespace is required |
| intermediary jar | `intermediary::load` | the bytes of `mappings/mappings.tiny` (`intermediary::JAR_ENTRY`) | | Tiny v2 (or v1 for old jars). Renames `official` to `obf` and puts `obf` first |

`detect(text)` guesses the format from the first meaningful line. Every parser accepts `\n` and `\r\n` line endings and a UTF-8 byte-order mark.

What the writers do with what a format cannot hold:

- **Name mappings that do not fit are an error** (`Error::Unrepresentable`). This covers a field without a descriptor (tiny), a missing name (ProGuard, TSRG, TSRG2), package mappings (tiny, ProGuard), whitespace in a name (ProGuard, TSRG) and tabs or line breaks in tiny v1.
- **Metadata the format has no place for is left out.** That is line ranges and inline frames outside ProGuard, comments and local variables outside tiny v2, parameters outside tiny v2 and TSRG2, static markers outside TSRG2, and field descriptors in TSRG.

`project`, `compose` and `chain` keep line ranges, comments, parameters, variables and static markers. ProGuard inline frames belong to one `(original, obfuscated)` pair, so only an unchanged projection keeps them.

## Descriptors and signatures

`desc` works on JVM internal names (JVMS 4.3 and 4.7.9.1). Each remapping function takes a class-name callback that returns `Some(new)` for a mapped class and `None` to keep the name. That is how JDK, library and mod classes pass through.

- `remap_field_desc`, `remap_method_desc`, `remap_class_or_array` (a `CONSTANT_Class` name that may be an array), `validate_field_desc`, `validate_method_desc`, `for_each_class_in_desc`.
- `remap_signature` handles class, method and field signatures: type parameters with class and interface bounds, wildcards, type variables, arrays, throws clauses and inner classes (`Lpkg/Outer<TT;>.Inner;`). For an inner class it looks up the full name `pkg/Outer$Inner`. When that is unmapped, the inner simple name stays under the remapped outer class.
- `java_type_to_desc`, `desc_to_java_type`, `java_method_to_desc`, `method_desc_to_java` convert to and from the Java source types of ProGuard files.

## Tests

```sh
cargo test -p jarrow-mappings                                  # offline: unit, fixtures, properties, doctests
cargo build -p jarrow-mappings --target wasm32-unknown-unknown # the engine-crate build check
cargo test -p jarrow-mappings --test live_chain -- --ignored --nocapture   # network, see below
```

- `tests/formats.rs` covers every format against a hand-written fixture in `tests/fixtures/`. It checks what each parser reads, then writes, reads back and compares the sets, and a second write must give the same bytes. It also checks escapes, CRLF, the byte-order mark and the writer errors.
- `tests/properties.rs` holds proptest properties over generated sets whose descriptors reference each other, primitives, arrays, inner classes and a JDK class:
  - inverting gives the set built over the swapped namespaces;
  - every lookup answers in both directions;
  - composing through a shared namespace, with a random part of the classes missing on one side, maps exactly the shared entries and leaves unmapped classes unchanged in descriptors;
  - a chain of three sets equals the directly built set and does not depend on the order of the sets or on unrelated sets;
  - composition is associative;
  - every format round trips;
  - the global map has no conflicts when names are consistent.
- `tests/chain.rs` covers the intermediary, obf, mojang chain on the fixtures, including descriptor and signature remapping and the global member map.
- `tests/live_chain.rs` is `#[ignore]`d because it needs network access and `curl` on `PATH`. It reads the URLs and sha1 values from `research-lock.json` and downloads intermediary's v2 jar and Mojang's server mappings into a new directory under the system temp directory, never the repository, and deletes it afterwards. It checks both sha1 values, reads the jar entry, composes the chain and checks `net/minecraft/class_2960`:
  - 1.21.1: `net/minecraft/resources/ResourceLocation`;
  - 1.21.11: `net/minecraft/resources/Identifier`.

  It also round-trips both real files through the writers, in memory only. A third test merges the 1.21.1 client and server files.

Measured on the maintainer machine (2026-11-03, release build): loading 1.21.1 intermediary (8,254 classes, 37,679 fields, 41,243 methods) takes about 55 ms, Mojang's server file (6,148 classes, 28,057 fields, 57,276 methods) about 95 ms, and the chain about 320 ms.

## Not built yet

- **Parchment** (parameter names for Tier B decompiled sources) is not built yet. Its files are JSON inside a zip, not one of the formats above.
- **Yarn** reads with `tiny2::parse`, but nothing pins or fetches it, and v1 does not need it.
- **The mapping version hash** for the determinism contract (`docs/architecture.md`) is computed by the hosts over the downloaded bytes. This crate does not hash.
- **Inheritance-aware lookups** (a member referenced through a subclass) belong to `crates/remap` (M2).
