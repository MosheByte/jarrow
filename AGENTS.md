# AGENTS.md — the map for the next session

You are continuing Jarrow, a Fabric to NeoForge mod converter with a parity harness. The specification is [BUILD-BOOK.md](BUILD-BOOK.md); it is the only source of requirements. This file tells you where things are and how to work here. Keep it current: if you change a crate boundary, a command or a rule below, update this file in the same pull request.

Last updated: 2026-11-03, during milestone M0.

## Start of every session

1. Read, in order: `BUILD-BOOK.md`, this file, `docs/architecture.md`, `DECISIONS.md`, `LEDGER.md` (once it exists), `docs/book-amendments.md`, and the doc of the subsystem you will touch.
2. Re-run the environment discovery protocol (book section 2): list every tool, MCP server and shell credential; verify each with one harmless real call; update `infra/INVENTORY.md`. Never assume a service is still connected because it was last time.
3. Find the first milestone whose exit criterion is not green on `main` (see `ROADMAP.md`) and resume there.

## Fixed facts

| Fact | Value |
| --- | --- |
| Repository | https://github.com/MosheByte/jarrow (moves to org `jarrow-dev` when the owner creates it, amendment A-1) |
| Default branch | `main` |
| Site | `SITE_URL` = https://jarrow.pages.dev (Cloudflare Pages project `jarrow`) |
| Rust API docs | https://moshebyte.github.io/jarrow/ (GitHub Pages) |
| Roadmap project | https://github.com/users/MosheByte/projects/1 |
| Pinned game versions | 1.21.1 and 1.21.11 (evidence: `docs/research/game-versions.json`; pins in `research-lock.json`) |
| Cloudflare bindings | KV `jarrow-board` as `BOARD`, D1 `jarrow` as `DB`, R2 `jarrow-reports` as `REPORTS` |
| Rust packages | `jarrow-<crate>` in `crates/<crate>/`, lib name `jarrow_<crate>`; the CLI package is `jarrow` (binary `jarrow`) in `crates/cli/`; edition 2024 |
| Licenses | Code MIT; rule tables under `rules/` CC0-1.0 |

## Crates

Only `crates/report` and `crates/cli` exist after M0. Everything else is planned; the milestone says when it lands. The full table with data flows is in `docs/architecture.md`.

| Path | Package | Owns | Status |
| --- | --- | --- | --- |
| `crates/jar` | `jarrow-jar` | Reading and writing JARs; deterministic, byte-stable output | Planned (M1) |
| `crates/classfile` | `jarrow-classfile` | Class-file parser and writer: constant pool, attributes, stack maps | Planned (M1) |
| `crates/mappings` | `jarrow-mappings` | Intermediary, Mojang official, Yarn, Parchment; tiny v2 and ProGuard; composition and inversion | Planned (M1) |
| `crates/remap` | `jarrow-remap` | Class, member and descriptor remapping, including strings that name classes | Planned (M2) |
| `crates/manifest` | `jarrow-manifest` | `fabric.mod.json` to `neoforge.mods.toml`, mixin configs, access wideners to access transformers | Planned (M1) |
| `crates/rules` | `jarrow-rules` | Rule-table loader and matcher | Planned (M2) |
| `crates/rewrite` | `jarrow-rewrite` | Applies rules to bytecode; every rewrite emits a report entry | Planned (M2) |
| `crates/classify` | `jarrow-classify` | Tier classifier: A, B or unsupported, with ordered reasons | Planned (M1) |
| `crates/report` | `jarrow-report` | Report model, JSON and Markdown rendering, badge data, design tokens | Exists (M0 skeleton: tokens, first report types) |
| `crates/engine` | `jarrow-engine` | `convert()`, the one public API | Planned (M2) |
| `crates/wasm` | `jarrow-wasm` | `wasm-bindgen` surface for the browser | Planned (M3) |
| `crates/cli` | `jarrow` | The `jarrow` binary: convert, check, report, board-run | Exists (M0 skeleton: `--version`, `tokens`; `check` M1, `convert` M2) |
| `crates/harness` | `jarrow-harness` | Headless Fabric and NeoForge servers, dumps, diff, score | Planned (M1) |
| `crates/board` | `jarrow-board` | Top-1,000 list from Modrinth, nightly classify, convert and harness | Planned (M1) |

When you create a crate, add or update its row here and in `docs/architecture.md`. CI checks that every directory under `crates/` is listed in `docs/architecture.md`.

## Other directories

| Path | What lives there |
| --- | --- |
| `rules/fabric-neoforge/<game-version>/` | Rule tables (TOML), one file per API area, plus `rules/SOURCES.md` |
| `site/` | Astro + Starlight site on Cloudflare Pages; `site/wrangler.toml` holds the Pages bindings (amendment A-5) |
| `site/src/styles/tokens.css` | Generated from `crates/report`. Never edit it by hand: change the tokens in `crates/report/src/tokens.rs`, then run `JARROW_BLESS=1 cargo test -p jarrow-report --test tokens_current`. Without `JARROW_BLESS`, that test fails when the file drifts. `jarrow tokens` prints the same CSS |
| `tools/` | Python tools: `pr_ready.py`, `ledger.py`, `board_summary.py`, research scripts; tests in `tools/tests/`; Tier B job in `tools/tierb/` (M4) |
| `tests/fixtures/` | Permissively licensed sample mods, vendored with their LICENSE, and golden outputs |
| `docs/` | `architecture.md`, `harness.md`, `rules.md`, `legal.md`, `telemetry.md`, `book-amendments.md`, `research/`, `journal/` |
| `infra/` | `INVENTORY.md`, `TODO-OWNER.md`, Worker configs other than the Pages one, D1 migrations, dashboards and monitors as code |
| `.github/` | Workflows, issue forms, PR template, CODEOWNERS, FUNDING.yml, labeler and Dependabot config |

## Build and test fast

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace                 # add -p jarrow-report to test one crate

py -3.12 -m pytest tools/tests         # on Linux and macOS: python -m pytest tools/tests

cd site && npm ci && npm run build     # the site

py -3.12 tools/pr_ready.py             # what CI runs before a merge; run it before every PR
```

## Rule families

Rule tables live in `rules/fabric-neoforge/1.21.1/*.toml` and `rules/fabric-neoforge/1.21.11/*.toml`, one file per API area. The file stem is the Fabric API module name without the `fabric-` prefix, for example `lifecycle-events-v1.toml` for `fabric-lifecycle-events-v1`. Areas that are not a Fabric API module use a plain name: `loader-entrypoints.toml` (Fabric Loader entrypoints, book 5.3) and `registries.toml` (vanilla registration, book 5.4).

The format is in `docs/rules.md`. It is a draft until the `rules` crate implements it in M2. Every source a rule relies on goes in `rules/SOURCES.md`.

## The harness locally

Not built yet (milestone M1). The design is in `docs/harness.md`. It will need JDK 21 and network access to download the Minecraft server, Fabric Loader, Fabric API and NeoForge at the versions pinned in `research-lock.json`.

The harness refuses to start a Minecraft server unless `MINECRAFT_EULA_ACCEPTED` (or `MINECRAFT_EULA`) is `true` (amendment A-6). In CI it comes from the owner's repository secret `MINECRAFT_EULA_ACCEPTED`, because accepting the Minecraft EULA is a terms acceptance only the owner may give. Never set it yourself.

## When you may stop

Only these three cases (book section 2):

1. **A legal boundary.** You would have to redistribute someone else's mod, assets or decompiled source, or violate a license you detected.
2. **Destroying something you did not create.** A destructive, irreversible action on the owner's repository, database or domain configuration.
3. **A payment or a terms acceptance.**

In each case: write what you need in `infra/TODO-OWNER.md`, use the fallback, and keep working on everything else. In every other case, choose, log the choice in `DECISIONS.md` as `D-<n>`, and continue.

## Rules that bite

- Never commit a JAR. `.gitignore` blocks `*.jar`; do not work around it. Fixture mods are vendored only when their license allows it, with their LICENSE beside them.
- Never commit decompiled source, and never let it reach a log, an issue, a model outside the Tier B job, or an analytics or error service.
- Never touch `research-lock.json` without a `D-` entry in `DECISIONS.md` that says why.
- Never hand-edit `LEDGER.md`. It is written only by running `tools/ledger.py`.
- Every commit carries `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- After M0, pull requests only. Branch, open a PR, let CI merge it with auto-merge (squash, linear history). Never push to `main`.
- Re-run book section 2 at the start of each session and update `infra/INVENTORY.md`.
- Nothing is green, "works" or "supported" without a harness result behind it. A missing feature says "not built yet (milestone Mx)". No fakes, no sample rows, no placeholder that pretends.
- Never promise a result. The book bans one word outright in all copy (section 8, "Copy"); write "verified by the harness" with a link to the report instead.
- Never print, log or commit a token or secret. Secrets live in GitHub Actions secrets and Cloudflare bindings; `infra/INVENTORY.md` records only their names.
- Never copy code from Sinytra Connector, Architectury, Forgified Fabric API or any mod. Rule tables describe API shapes, with sources.
- Never vendor Mojang's mappings. They are fetched from the version manifest at runtime.
- No generated icons, logos, illustrations or screenshots. UI icons come from Lucide; brand logos from Simple Icons; screenshots only from real CI runs.
- Green, amber and red are reserved for results. Never use them decoratively. Colours come from the tokens in `crates/report`.
- When the book and reality disagree, reality wins: record a `D-` entry and add an amendment to `docs/book-amendments.md`.
- Later tabs (Version translate, Compat check, Dependency updater, Profiler) get an issue, never a branch, until M6 is closed.

## Where decisions and status live

| File | Purpose | Who writes it |
| --- | --- | --- |
| `DECISIONS.md` | Every default taken, as `D-<n>` | The lead agent |
| `docs/book-amendments.md` | Where the book had to change to match reality | The lead agent |
| `infra/INVENTORY.md` | Every service: Connected, Self-configurable or Needs the owner | The lead agent, every session |
| `infra/TODO-OWNER.md` | Exact steps only the owner can take | The lead agent |
| `LEDGER.md` | Numbers from the last nightly | `tools/ledger.py` only |
| `ROADMAP.md` | Milestones and their status | Updated when a milestone moves |
| `docs/journal/<date>.md` | Weekly two-paragraph note for the owner | The agent |
