# Jarrow

[![CI](https://github.com/MosheByte/jarrow/actions/workflows/ci.yml/badge.svg)](https://github.com/MosheByte/jarrow/actions/workflows/ci.yml)

Drop a Minecraft Fabric mod JAR on a web page and get the same mod back for NeoForge, with a proof that it behaves the same.

**Status: pre-alpha — milestone M0; nothing converts yet.** The repository, CI, site skeleton and docs are being set up. There is no converter, no harness and no board yet. See [ROADMAP.md](ROADMAP.md) for what comes when.

## Links

| What | Where |
| --- | --- |
| Site | https://jarrow.pages.dev |
| Docs | [docs/](docs/) in this repository; Rust API docs at https://moshebyte.github.io/jarrow/ |
| Board | Not built yet (milestone M1). The site shows its counter, which is zero until the first nightly run. |
| Roadmap | [ROADMAP.md](ROADMAP.md) and the project board at https://github.com/users/MosheByte/projects/1 |
| Discussions | https://github.com/MosheByte/jarrow/discussions |

## How it will work

Every JAR is sorted into one tier before anything else happens:

- **Tier A** converts in your browser (Rust compiled to WebAssembly). The JAR never leaves your machine.
- **Tier B** runs a short-lived GitHub Actions job (decompile, rules, agent fix, recompile, harness) for signed-in users, and the workspace is wiped afterwards.
- **Unsupported** means the page tells you plainly why (native libraries, bytecode transformers, obfuscated JARs, or a license that forbids modification) and converts nothing.

A result is "green" only when the parity harness says so. The harness starts a Fabric server with the original mod and a NeoForge server with the converted one, dumps registries, recipes, tags, loot tables and a worldgen sample, and compares them. Until the harness exists, nothing is green.

Jarrow never hosts, caches or links to converted JARs. Conversion is for your own game unless you are the mod's author. Authors can opt their mod out completely. Details are in [docs/legal.md](docs/legal.md).

## Built by an agent

Jarrow is built by Claude, an AI agent made by Anthropic, for the project owner ([@MosheByte](https://github.com/MosheByte)). Every commit is co-authored by Claude and says so in its trailer. The point of the project is to show agent-built software that proves its own results, so the authorship is part of the story, not a footnote.

## Contributing

Read [CONTRIBUTING.md](CONTRIBUTING.md). Security issues go through [SECURITY.md](SECURITY.md). Everyone follows the [Code of Conduct](CODE_OF_CONDUCT.md).

## License

The code is MIT ([LICENSE](LICENSE)). The rule tables under `rules/` are CC0-1.0, so anyone can reuse what they describe about the Fabric and NeoForge APIs.

---

Not an official Minecraft product. Not approved by or associated with Mojang or Microsoft. Not affiliated with the Fabric or NeoForge projects.
