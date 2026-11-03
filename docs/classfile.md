# The class-file crate

**Status: exists (milestone M1).** `crates/classfile` (package `jarrow-classfile`, library `jarrow_classfile`) reads and writes Java class files. It is bytecode only: it never decompiles. It knows nothing about loaders or game versions, so Fabric, NeoForge, Forge and Quilt classes on any pinned Minecraft version go through the same code; the loader and version decisions live in the crates above it (`remap`, `rules`, `rewrite`).

What is measured today: every class in the JDK 21 class library round-trips byte for byte (below). What is not measured yet: the round trip on the board's 1,000-mod set, which needs the board pipeline (M1, in progress). Until that runs, nothing here claims the board set works.

## What it does

| Piece | API | Notes |
| --- | --- | --- |
| Parse | `ClassFile::parse(&[u8]) -> Result<ClassFile, Error>` | Malformed input is an `Err`, never a panic |
| Write | `ClassFile::to_bytes()`, `ClassFile::write_to(&mut Vec<u8>)` | Byte-identical to the input for an unmodified class |
| Constant pool | `ConstantPool`, `Constant`, `ReferenceKind`, `tag::*` | Every tag of JVMS 4.4, including `Long`/`Double` taking two slots, `MethodHandle`, `MethodType`, `Dynamic`, `InvokeDynamic`, `Module` and `Package` |
| Lookups | `utf8`, `utf8_str`, `class_name`, `name_and_type`, `member_ref` | Resolve indices to names, checking each entry's kind |
| Mutation | `get_or_add_utf8`, `_utf8_bytes`, `_class`, `_string`, `_name_and_type`, `_fieldref`, `_methodref`, `_interface_methodref`, `get_or_add`, `push`, `replace` | Return the lowest existing matching index or append; never renumber |
| Attributes | `Attribute { name_index, body: AttributeBody }` | Typed bodies listed below; everything else `AttributeBody::Unknown(bytes)` |
| Instructions | `Code::instructions()`, `Instructions::new(&[u8])`, `decode_instruction`, `Instruction::encode` | Every opcode, switch padding and `wide` |
| Modified UTF-8 | `mutf8::{encode, decode, decode_lossless, encode_pieces, decode_lossy}` | Lone surrogates kept as raw bytes |
| Flags | `access::*` | Plain `u16` constants |

The crate has no runtime dependencies, uses no I/O, threads or `unsafe`, and builds for `wasm32-unknown-unknown` (`cargo build -p jarrow-classfile --target wasm32-unknown-unknown`).

### Typed attributes

`Code` (max stack, max locals, code bytes, exception table, nested attributes), `StackMapTable` (every frame type and verification type), `ConstantValue`, `Exceptions`, `InnerClasses`, `EnclosingMethod`, `Synthetic`, `Signature`, `SourceFile`, `LineNumberTable`, `LocalVariableTable`, `LocalVariableTypeTable`, `Deprecated`, `RuntimeVisibleAnnotations`, `RuntimeInvisibleAnnotations`, both parameter-annotation attributes, both type-annotation attributes (full `target_info` and `type_path`), `AnnotationDefault`, `BootstrapMethods`, `MethodParameters`, `NestHost`, `NestMembers`, `Record` (with each component's attributes) and `PermittedSubclasses`.

An attribute is typed only where JVMS table 4.7-C defines it: `Code` on a method, `StackMapTable` inside `Code`, `Record` on a class, and so on. The same name anywhere else is kept raw, as the JVM ignores it there. `Module`, `ModulePackages`, `ModuleMainClass`, `SourceDebugExtension` and vendor attributes are kept raw on purpose: remapping does not need them.

## Design choices

- **Exact round trip by construction.** `Utf8` entries keep their raw bytes; `Float` and `Double` keep their IEEE bits, so NaN payloads survive; each stack map frame type has its own variant (`Same` with delta 3 and `SameExtended` with delta 3 are different bytes); element-value tags and type-annotation `target_type` are kept. Lengths and counts are recomputed on write, so a typed attribute must fill its declared length exactly or the parse fails (`Error::AttributeLength`).
- **Strict where a wrong answer would be silent.** The parser rejects what HotSpot's format check rejects and what would make later stages guess: a bad magic number, `constant_pool_count` of 0, an unknown tag, a `Long` or `Double` in the last slot, a constant-pool reference to the wrong kind of entry (including `MethodHandle` kinds), `this_class`, `super_class`, interfaces, member names and descriptors, and attribute names that are not the right kind, a malformed typed attribute, and trailing bytes. A class Jarrow cannot read fully is reported, not converted with names it might have missed.
- **Not checked on purpose.** Indices inside attributes (an annotation's `type_index`, a `Signature` index), version-dependent rules, and bytecode verification. Those belong to the stages that use them; the lookups (`utf8_str`, `class_name`, `member_ref`) check kinds when called.
- **Hostile input is bounded.** Annotations and element values may nest at most `MAX_NESTING` (64) levels, so a small file cannot exhaust the stack. List capacities are capped by the bytes left, so a lying count cannot force a large allocation. A `tableswitch` or `lookupswitch` whose entries cannot fit in the remaining code fails before allocating.
- **Modified UTF-8 has three levels.** `decode` is strict and returns a `str` (borrowed when the bytes are already plain UTF-8, which is nearly every name). `decode_lossless` never fails: it splits bytes into `Piece::Text` and `Piece::Raw` runs, and `encode_pieces` gives back exactly the same bytes for any input; lone surrogates and malformed sequences are the raw runs. `decode_lossy` shows raw runs as U+FFFD, for display only.
- **The decoder reports `wide` as a flag.** For `wide iload 300` the instruction's `opcode` is `iload`, `wide` is true and `length` is 4. Branch and switch offsets are relative, as in the class file. Switch padding bytes are skipped without checking their values (HotSpot requires zeros only from class version 51); the reserved bytes of `invokeinterface` and `invokedynamic` must be zero.
- **The encoder exists for rewriting and for testing the decoder.** `Instruction::encode` writes zero padding computed from `Instruction::offset`. Re-encoding the JDK's 12.5 million instructions reproduces every code array, which checks the decoder's operands, not just its lengths.

## Tests

```sh
cargo test -p jarrow-classfile                       # everything below
cargo test -p jarrow-classfile --test jdk -- --nocapture   # the JDK round trip, with its counts
```

| Test file | What it checks |
| --- | --- |
| `tests/jdk.rs` | Every class in `$JAVA_HOME/jmods/*.jmod` parses, writes back byte-identical, every `Utf8` survives `decode_lossless`, every instruction decodes, names a constant of the kind its opcode needs, and re-encodes to the original code array. Asserts that the typed attributes a JDK 21 build contains were parsed as typed. |
| `tests/roundtrip.rs` | A hand-built class using every constant tag and every typed attribute round-trips; every truncation and every single-byte flip either fails or round-trips; specific errors (bad magic, trailing bytes, wrong reference kinds, attribute length, nesting depth, writer limits) |
| `tests/properties.rs` | proptest: MUTF-8 (`str` round trip, lossless round trip of any bytes), constant-pool get-or-add (index matches, repeats return the same index, nothing renumbered, pool round-trips through a class), random bytes and random mutations never panic and round-trip when they parse, the decoder on random bytes |
| `tests/decoder.rs` | Hand-assembled bytecode: every opcode's operands, `wide`, branches, `tableswitch` and `lookupswitch` at all four alignments, truncation, malformed switches, reserved bytes, encoder limits |

No test needs the network, and no test reads a committed class or JAR: inputs are built in code, and the JDK test reads the installed JDK.

**The JDK test and `JARROW_REQUIRE_JDK`.** The test finds the JDK through `JAVA_HOME`, then through `java` on `PATH` (following symlinks). It reads each jmod with a small zip reader in the test (a jmod is `JM 01 00` followed by a zip; stored, deflate and zip64 are handled, inflating with the `miniz_oxide` dev-dependency). With no JDK found it prints a skip note and passes, unless the environment variable `JARROW_REQUIRE_JDK` is `1`, in which case it fails. CI's `rust` job installs Temurin 21 with `actions/setup-java` and sets `JARROW_REQUIRE_JDK: "1"` on Linux, macOS and Windows.

Measured on 2026-11-03 with Temurin 21.0.12 on Windows: 70 jmods, 27,889 classes, 212,093 methods with code and 12,518,321 instructions, all round-tripping, in about 3 seconds on 24 cores in a debug build. The JDK contains no type annotations or parameter annotations; those are covered by the hand-built class and the property tests.

## Fuzzing

`fuzz/` is a [cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz) project with its own `[workspace]` table, so the root workspace, its `Cargo.lock` and `cargo deny` never include it (its `libfuzzer-sys` dependency bundles LLVM's libFuzzer under the NCSA license; nothing in `fuzz/` is shipped).

| Target | What it does |
| --- | --- |
| `classfile_parse` | Parses arbitrary bytes. When they parse, asserts that writing gives back exactly the input, that every `Utf8` survives `decode_lossless`, and that every decoded instruction re-encodes to its own length |
| `jar_read` | Reads arbitrary bytes as a JAR with `jarrow_jar::Jar::read_with` (limits tighter than the defaults: 4,096 entries, 4 MiB per entry, 8 MiB in all, nesting depth 3), then reads nested jars and the manifest. Every `.class` entry that parses must write back to its own bytes; a jar that writes must give the same bytes twice and read back with the same entries |

`fuzz/dicts/<target>.dict` are libFuzzer dictionaries: for `classfile_parse` the magic number, constant-pool entry shapes and attribute names with their length prefixes; for `jar_read` the zip record signatures and the entry names a mod jar uses. There is no committed seed corpus, because seeds would be class files and JARs.

Run locally (needs a nightly toolchain and `cargo install cargo-fuzz`):

```sh
cargo +nightly fuzz run classfile_parse -- -dict=fuzz/dicts/classfile_parse.dict -max_total_time=300
```

**ClusterFuzzLite** runs the same targets in GitHub Actions, which is what OpenSSF Scorecard's Fuzzing check looks for:

- `.clusterfuzzlite/Dockerfile` starts from `gcr.io/oss-fuzz-base/base-builder-rust:v1`, pinned by digest. That image's nightly (2025-09-05, rustc 1.91) is older than the workspace MSRV (1.99), so the Dockerfile installs `nightly-2026-10-15` and sets `RUSTUP_TOOLCHAIN` to it, which also overrides `rust-toolchain.toml`. Bump the date when the MSRV passes it.
- `.clusterfuzzlite/build.sh` runs `cargo fuzz build -O --debug-assertions` and copies every target from `cargo fuzz list`, with its dictionary, to `$OUT`, so a new target needs no change here.
- `.github/workflows/cflite_pr.yml` builds with AddressSanitizer and fuzzes for 300 seconds on pull requests that touch `crates/`, `fuzz/`, `.clusterfuzzlite/`, the workspace manifests or the toolchain. A crash fails that workflow; it is not part of `ci-ok`.
- `.github/workflows/cflite_batch.yml` fuzzes for an hour every Sunday at 03:41 UTC (and on manual dispatch), keeping the corpus as a workflow artifact that the pull-request run starts from.

Both workflows use only the built-in token, with `contents: read` at the top and `actions: read` on the job (to fetch the corpus artifact). `google/clusterfuzzlite` publishes a single moving tag, `v1`; the workflows pin the commit it pointed at on 2026-11-03.

## Not done

- The board-set round trip (needs the board pipeline; `crates/jar` now exists).
- Typed `Module*` attributes, kept raw.
- Computing `max_stack` or stack map frames for rewritten code. That is the `rewrite` crate's job (M2); this crate writes back what it is given.
