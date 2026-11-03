# The jar crate

`crates/jar` (package `jarrow-jar`, library `jarrow_jar`) reads and writes JARs for the engine. It is an engine crate: it builds for `wasm32-unknown-unknown`, does no I/O, uses no threads, and depends only on [`miniz_oxide`](https://crates.io/crates/miniz_oxide) (pure-Rust DEFLATE). It knows nothing about loaders or game versions: nested-jar directories are archive layout, and which loader declares what is the manifest crate's business.

Status: built in M1. It has no binary and no CLI flags.

The zip format follows [PKWARE's APPNOTE.TXT](https://pkware.cachefly.net/webdocs/casestudies/APPNOTE.TXT) and the manifest format follows the [JAR File Specification](https://docs.oracle.com/en/java/javase/21/docs/specs/jar/jar.html). The reader and writer are this crate's own code, written against those two documents. Byte-level output had to be under our control, and so did the checks below.

## API at a glance

| Call | Does |
| --- | --- |
| `Jar::read(&[u8])`, `Jar::read_with(&[u8], &ReadOptions)` | Parse an archive. Never panics. This is the fuzzing entry point |
| `jar.write(&WriteOptions)` | Deterministic bytes |
| `jar.entries` | `Vec<Entry>`, public and in order; `get`, `get_mut`, `put`, `remove`, `names` help |
| `jar.manifest()`, `jar.set_manifest(&Manifest)` | The parsed `META-INF/MANIFEST.MF` |
| `Manifest::parse`, `Manifest::to_bytes` | Manifest text, both ways |
| `jar.is_signed()`, `jar.strip_signatures()` | Signature detection and removal, with a report |
| `jar.nested_jar_entries()`, `jar.read_nested(&ReadOptions)`, `jar.read_entry_as_jar(name, &ReadOptions)` | Jar-in-jar |
| `check_name`, `crc32`, `is_signature_file`, `is_valid_attribute_name` | The building blocks, for other crates |

An `Entry` has `name` (a name ending in `/` is a directory), `data` (uncompressed), `method` (`Stored` or `Deflated`: how the writer stores it under `Compression::PerEntry`) and `original`. `original` is `Some` for entries read from an archive and records how the entry looked there: method, CRC-32, compressed and uncompressed size, whether an extra field was present, the Unix mode when the archive set one, and whether zip64 fields were used. The writer ignores `original`.

## Reading

`Jar::read` locates the end-of-central-directory record (and the zip64 record when a locator is present), reads the central directory, and returns entries in central-directory order. The JVM's `ZipFile` uses the same order. Sizes and CRCs come from the central directory, so data descriptors (general-purpose flag bit 3) are read correctly. Bytes prepended to the archive, such as a launcher script, are tolerated the way the JDK tolerates them.

Every check runs before anything is decompressed. Each failure is an `Error`. None is a skipped entry.

| Refused | Error |
| --- | --- |
| No end record | `NotAZip` |
| Truncated or inconsistent structures, data running into the central directory, a missing zip64 field | `Malformed` |
| Split archives | `MultiDisk` |
| Unsafe names (below) | `UnsafeName { name, reason }` |
| A name that is not UTF-8 (the JVM decodes names as UTF-8) | `NameNotUtf8` |
| Two entries with the same name | `DuplicateName` |
| Encrypted entries | `Encrypted` |
| Methods other than 0 and 8 | `UnsupportedMethod` |
| A local header whose name or method differs from the central record | `LocalHeaderMismatch` |
| Entries whose bytes overlap (the overlapping-file zip bomb) | `OverlappingEntries` |
| A directory with data | `DirectoryWithData` |
| A limit exceeded (below) | `LimitExceeded { limit, requested, max }` |
| Data that is shorter or longer than declared | `SizeMismatch` |
| A bad CRC | `CrcMismatch` |
| A broken deflate stream | `Inflate` |

### Unsafe names

`check_name` refuses:

- empty names
- absolute paths (`/x`) and drive letters (`C:x`)
- backslashes and NUL bytes
- `..` components
- components made only of dots (`.`, `...`)
- empty components (`a//b`)

A directory's single trailing `/` is allowed, and so are dots inside a component (`lang/en_us..json`). The writer applies the same check, so it never writes a name it would refuse to read.

### Limits (zip-bomb guards)

`ReadOptions` is `#[non_exhaustive]`: start from `ReadOptions::default()` and set fields.

| Field | Default | Checked against |
| --- | --- | --- |
| `max_entries` | 100,000 | The entry count the end record declares, before the directory is parsed |
| `max_entry_size` | 256 MiB | Each entry's declared uncompressed size |
| `max_total_size` | 1 GiB | The sum of declared uncompressed sizes |
| `max_nesting_depth` | 8 | Nesting levels `read_nested` follows (the outer jar is level 0), never more than `HARD_MAX_NESTING_DEPTH` = 64 |

The decompressor is capped at the declared size plus one byte. An entry that inflates past its declaration therefore fails with `SizeMismatch` after at most one extra byte of output, and memory never grows beyond the limits. For nested jars the entry and size limits are one budget for the whole tree.

## Writing

`jar.write(&options)` is a pure function of the entries and the options. The same input gives the same bytes on every run and every OS. The tests check this, along with exact bytes for a small stored jar.

- Entries are written in the order given, except that `META-INF/MANIFEST.MF` comes first when present. If no entry has that exact name, the first entry matching it ignoring ASCII case comes first, which is how the JDK looks the manifest up.
- Every header carries the DOS timestamp 1980-01-01 00:00:00 (time `0x0000`, date `0x0021`).
- Headers have no extra fields, no comments, no data descriptors, no Unix mode and no archive comment. "Version made by" names MS-DOS. External attributes are `0x10` for directories and 0 for files. Names that are not ASCII set the UTF-8 flag (bit 11).
- Zip64 fields appear only when the format needs them: a size or offset of 4 GiB or more, or 65,535 entries or more.
- Directories and empty files are always stored. Files follow `WriteOptions::compression`: `PerEntry` (the default) uses each entry's `method`, `Deflate` deflates every file and `Store` stores every file. `deflate_level` is 0 to 9 and defaults to 6.
- The writer refuses unsafe or duplicate names, a directory with data and a deflate level above 9.

Deflated bytes depend on the `miniz_oxide` version, which `Cargo.lock` pins. A bump of that crate can change the output, so it counts as an engine change under the determinism contract in [architecture.md](architecture.md).

## Manifests

`Manifest` has `main: Vec<Attribute>` and `sections: Vec<Section>`, where each `Section` has a `name` (the value of its `Name` header) and `attributes`.

- **Parse.** Lines may end in CR LF, LF or CR. A line that starts with a space continues the previous header; the bytes are joined first and then decoded as UTF-8, as the JDK joins them. Sections are separated by blank lines, and more than one blank line is fine. A final line without a newline is kept. Attribute names compare ASCII case-insensitively, and `get` returns the last value when a name repeats, as `java.util.jar.Attributes` does. Order, repeated attributes and repeated sections are all kept.
- **Errors.** Each comes with its line number: a header without `": "`, a name that is not 1 to 70 letters, digits, `-` or `_`, a continuation line with no header before it, an individual section that does not start with `Name`, and text that is not UTF-8.
- **Write.** Lines end in CR LF and hold at most 72 bytes; continuation lines are a space plus up to 71 bytes. `Manifest-Version` (or, without it, `Signature-Version`) comes first, as the specification orders it, and every section ends with a blank line. One difference from the JDK is deliberate: the JDK cuts at exactly 72 bytes even inside a multi-byte UTF-8 character, and this writer moves the cut back to a character boundary so every line is valid UTF-8 on its own. For ASCII the output is identical to the JDK's. Values containing CR, LF or NUL are refused.

## Signatures

`strip_signatures()` removes:

- Signature files directly under `META-INF/` (any case) ending in `.SF`, `.RSA`, `.DSA` or `.EC`, or starting with `SIG-`.
- Every `*-Digest` attribute (`SHA-256-Digest`, `SHA1-Digest`, ...) in the manifest's individual sections. A section left with no attributes is dropped.

Other attributes and sections stay, for example `Sealed`. The manifest is rewritten only when a digest was removed, so an unsigned jar's manifest bytes are untouched. The return value, `StrippedSignatures { files, digests, sections }`, lists what went, in order, for the report. If the manifest cannot be parsed the call fails and the jar is unchanged. A converted jar's bytes differ from the signed ones, so a kept signature could only fail verification. Fabric API 0.116.17+1.21.1 ships `META-INF/FABRIC1.SF` and `FABRIC1.RSA`, so this path matters from the first direction on.

## Nested jars

`NestedDir::Jars` is `META-INF/jars/`, where Fabric Loom places jar-in-jar files and which Quilt builds share. `NestedDir::JarJar` is `META-INF/jarjar/`, used by NeoForge's and Forge's JarJar. A nested jar is a file ending in `.jar` anywhere below one of these prefixes. The prefixes are matched case-sensitively, as the loaders read them. `META-INF/jarjar/metadata.json` is not a jar and is not listed.

`read_nested` returns a tree of `NestedJar { path, dir, jar, nested }`. An error inside a nested jar comes back as `Error::Nested { path, source }`, one layer per level, so the path to the broken jar is in the error. To read a jar declared somewhere else, such as a `fabric.mod.json` `jars` entry outside these directories, use `read_entry_as_jar`.

## Tests

`cargo test -p jarrow-jar` runs these suites:

| File | Covers |
| --- | --- |
| `tests/roundtrip.rs` | Proptest: arbitrary jars (files, directories, Unicode names, stored and deflated, a manifest anywhere) written with every compression mode and level read back identical, and writing what was read gives the same bytes. Also manifest-first ordering and the empty jar |
| `tests/determinism.rs` | Two writes are identical. Exact bytes of a small stored jar. Every header has the fixed timestamp and no extra field. Input metadata (timestamps, Unix modes, extra fields, comments, data descriptors, a prefix) never reaches the output. 70,000 entries produce a zip64 end record |
| `tests/guards.rs` | Traversal and other unsafe names (from the reader and to the writer), duplicates, non-UTF-8 names, encryption, unsupported methods, CRC and size lies, every truncation of a valid jar, the zip-bomb limits, overlapping entries, and tolerated real-world shapes |
| `tests/zip64.rs` | Zip64 extra fields and end records built field by field, and their failure modes |
| `tests/manifest.rs` | Continuation lines, every line ending, 72-byte wrapping exactly as the JDK does it, UTF-8 boundaries, errors with line numbers, and a proptest round trip |
| `tests/signatures.rs` | Stripping, the report, idempotence, untouched unsigned manifests, and a malformed manifest leaving the jar unchanged |
| `tests/nested.rs` | Nested jars built in the test, in both directories and two levels deep, plus depth limits, the hard cap and a nested zip bomb |
| `tests/no_panic.rs` | Proptest: arbitrary and mutated bytes through read, manifest, nested read, strip and write, none of which may panic |

Two suites are `#[ignore]`d because they need things CI does not have. Neither uses the network.

```sh
# The JDK's own zip code reads what we write (listing and extracting byte for byte),
# and we read what `jar --create` writes (deflated with data descriptors, and stored).
# Uses $JAVA_HOME/bin/jar, else `jar` on PATH.
cargo test -p jarrow-jar --test jdk_interop -- --ignored

# Every jar in a local directory: read, nested read, manifest, strip, and a deterministic
# write that reads back to the same entries. Jars are never committed.
JARROW_JAR_CORPUS=/path/to/mods cargo test -p jarrow-jar --release --test corpus -- --ignored --nocapture
```

Both passed on 2026-11-03 with JDK 21.0.12 (Temurin). The corpus run covered six local jars, among them Fabric API 0.116.17+1.21.1 (49 nested jars, signed) and a combined Fabric and NeoForge jar with 88 nested jars.

## Fuzzing

`Jar::read` is the fuzzing entry point. The classfile package owns the fuzzing infrastructure, and the target to add there is:

```rust
fuzz_target!(|data: &[u8]| {
    if let Ok(jar) = jarrow_jar::Jar::read(data) {
        let _ = jar.read_nested(&jarrow_jar::ReadOptions::default());
    }
});
```

## Not done

- The whole archive is held in memory; there is no streaming reader. For the board's mods this is fine, and in the browser the limits bound memory.
- Archive and entry comments, timestamps and Unix modes are read only as far as `Original` records them. They are never written back.
- Duplicate names are refused. The JDK accepts them and picks one entry, and that ambiguity is what the refusal avoids. The nightly board will show whether real mods contain duplicates. If any do, a policy for them is a later decision.
- Zip64 combined with bytes prepended to the archive is refused, because the zip64 locator's offset no longer points at its record.
- Entries over 4 GiB cannot exist on `wasm32` (one `Vec` cannot hold them). The writer handles them on 64-bit hosts, but only the lowered-threshold unit test exercises that path.
