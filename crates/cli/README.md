# jarrow

The command-line tool of [Jarrow](https://github.com/jarrow-dev/jarrow), a converter from Fabric to NeoForge for Minecraft mod JARs, with a behaviour harness that checks each result.

**This is a pre-alpha release, and it converts nothing yet.** It has two commands:

- `jarrow --version` prints the version.
- `jarrow tokens` prints the project's design tokens as CSS, and `jarrow tokens --json` prints them as JSON.

`jarrow check` (classify a JAR) is not built yet (milestone M1), and neither is `jarrow convert` (milestone M2). The CLI rejects both rather than pretend to run them.

## Install

The only release is a pre-release, so Cargo needs the exact version:

```sh
cargo install jarrow --version 0.1.0-alpha.1 --locked
```

It needs Rust 1.99 or newer. Nightly binaries for Linux, Windows and macOS are on the [releases page](https://github.com/jarrow-dev/jarrow/releases).

## Links

- Repository, roadmap and progress: https://github.com/jarrow-dev/jarrow
- How releases work: [docs/releases.md](https://github.com/jarrow-dev/jarrow/blob/main/docs/releases.md)

MIT licensed.
