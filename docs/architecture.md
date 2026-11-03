# Architecture

One Rust workspace is the product. The website, the CLI and the GitHub Actions jobs are thin shells around it. Nothing conversion-related is written in TypeScript. This page is the map; the decisions behind it are logged in [DECISIONS.md](../DECISIONS.md).

Status on 2026-11-03 (milestone M0): only `crates/report` and `crates/cli` exist, as skeletons. Nothing converts yet. Everything below marked "planned" is a design, not code.

## Crates

Every package is named `jarrow-<crate>` (library `jarrow_<crate>`), except the CLI package, which is `jarrow` with the binary `jarrow`. Edition 2024, MIT. CI checks that every directory under `crates/` appears in this table.

| Crate | Owns | Notes | Status |
| --- | --- | --- | --- |
| `crates/jar` | Reading and writing JARs and manifests; signatures stripped; deterministic output | Pure Rust zip; byte-stable output so two runs give identical hashes | Exists (M1: read with zip64 and zip-bomb limits, deterministic write, manifests, signature stripping, nested jars; [jar.md](jar.md)) |
| `crates/classfile` | Java class-file parser and writer: constant pool, attributes, stack maps | Bytecode only, no decompilation. Round-trip tests on the JDK class library and the board set | Exists (M1: parser, byte-exact writer, instruction decoder, constant-pool helpers; JDK round trip in CI, fuzzed by ClusterFuzzLite; board-set round trip not run yet; [classfile.md](classfile.md)) |
| `crates/mappings` | Intermediary, Mojang official, Yarn, Parchment; tiny v2 and ProGuard formats; composition and inversion | Mojang mappings are fetched from the version manifest at runtime, never vendored | Exists (M1: tiny v1 and v2, ProGuard, TSRG and TSRG2 read and write; compose, invert, chain; see [mappings.md](mappings.md). Parchment not built yet) |
| `crates/remap` | Class, member and descriptor remapping over `classfile`, including string constants that name classes (reflection, mixin targets) | Differential-tested against tiny-remapper output | Planned (M2) |
| `crates/manifest` | `fabric.mod.json` to `neoforge.mods.toml`, mixin configs, access wideners to access transformers, `pack.mcmeta` | Lossless where possible; the report lists every field it could not translate | Planned (M1) |
| `crates/rules` | Rule-table loader and matcher: call rewrites, class substitutions, event bridges, entrypoint adapters | Data-driven; tables are TOML under `rules/fabric-neoforge/<game-version>/` ([rules.md](rules.md)) | Planned (M2) |
| `crates/rewrite` | Applies rules to bytecode: call-site rewriting, generated adapter classes, entrypoint shims | Every rewrite emits a report entry | Planned (M2) |
| `crates/classify` | The tier classifier: scans a JAR and decides A, B or unsupported, with ordered reasons | Budget: under 300 ms for a 5 MB JAR in WASM | Planned (M1) |
| `crates/report` | Report model, JSON and Markdown rendering, badge data, design tokens | One report format everywhere: browser, CLI, Actions, board. Generates `site/tokens.css`; a test fails when the file drifts | Exists (M0 skeleton: tokens and the first report types; full model in M1) |
| `crates/engine` | Orchestrates the crates above into `convert()` | The one public API | Planned (M2) |
| `crates/wasm` | `wasm-bindgen` surface for the browser: `classify`, `convert`, streaming progress | Budget: 4 MB gzipped | Planned (M3) |
| `crates/cli` | The `jarrow` binary: `convert`, `check`, `report`, `board-run` | Released with cargo-dist | Exists (M0 skeleton: `--version` and `tokens` only; `check` in M1, `convert` in M2) |
| `crates/harness` | Starts headless Fabric and NeoForge servers, dumps registries, recipes, tags, loot and worldgen, diffs and scores | Needs a JDK; runs in Actions and locally ([harness.md](harness.md)) | Planned (M1) |
| `crates/board` | Fetches the top-1,000 list from Modrinth, runs classify, convert and harness, writes results | Nightly job | Exists (M1 wave 1: `list`, `shard`, `merge` and `board.yml`; classify, convert and the harness are not wired yet, so every mod is red with "no converter yet"; [board-pipeline.md](board-pipeline.md)) |

### Dependency direction

Lower layers never depend on higher ones.

```
report                          (leaf: no Jarrow dependencies)
jar   classfile   mappings   manifest
remap (classfile, mappings)
rules (classfile)
rewrite (classfile, rules, report)
classify (jar, classfile, manifest, rules, report)
engine (all of the above)
wasm (engine)     cli (engine, harness, board)
harness (jar, report)           board (engine, classify, harness, report)
```

## Tier A data flow (browser and CLI)

Planned for M2 (CLI) and M3 (browser).

1. The browser loads the `wasm` bundle.
2. `classify(jar)` reads the manifest and scans the classes. The page shows mod id, name, version, loader, game version, license, dependencies and the tier with its reasons, within two seconds.
3. The license gate decides what is offered ([legal.md](legal.md), "License policy"): permissive and copyleft licenses convert; restrictive or missing licenses convert for personal use only; a mod on the opt-out list is refused.
4. `convert(jar, target)` runs `manifest`, `remap`, `rules` and `rewrite` through `engine`, and streams progress.
5. The result is a blob URL that exists only in that tab, plus the report.

No network request carries the JAR. The only telemetry is the anonymous event described in [telemetry.md](telemetry.md), and the user can turn it off.

The CLI runs the same `engine` code: `jarrow convert` and `jarrow check`.

## Tier B pipeline

Planned for M4. Lives in `tools/tierb/` (Python plus the CLI) and runs as `.github/workflows/tierb.yml`, one GitHub Actions job per conversion, 30-minute cap.

1. Download the JAR through a short-lived R2 presigned URL. The job deletes the object right after the download; the bucket also expires objects after 1 day as a backstop (amendment A-4).
2. Run `jarrow convert --tier a`. If static checks pass, skip to step 6.
3. Decompile with Vineflower into a scratch directory; remap to Mojang official names; generate a Gradle project against the NeoForge toolchain.
4. Apply rule tables as source transforms where bytecode rewriting was not possible, then compile. Compile errors go to the agent with the rule docs and the NeoForge API index as context. The agent edits only files under `scratch/src/`, capped at 6 iterations and 400k tokens per job.
5. Build the JAR.
6. Run the parity harness, attach the report, return the result once, wipe the workspace.

Decompiled code never reaches the Actions log, an analytics or error service, or a model outside the job's own context.

## Web

`site/` is Astro + Starlight on Cloudflare Pages, project `jarrow`, served at `SITE_URL` = https://jarrow.pages.dev. Rust API docs are separate, on GitHub Pages at https://moshebyte.github.io/jarrow/.

- **Server routes and SSR pages** use the `@astrojs/cloudflare` adapter. The adapter compiles them into the Pages project's Worker, so they share one set of bindings. The first route is `/api/board.json`. Planned routes: `/api/queue` and `/api/auth/github` (M4), and the badge route `/badge/<modid>.svg` (M5). They were first specified as a separate `functions/` Worker; with the adapter they are Astro endpoints in the same deployment, with the same routes and bindings (D-016, amendment A-8 in [spec-amendments.md](spec-amendments.md)).
- **Bindings** are declared in `site/wrangler.toml` (amendment A-5):

  | Binding | Resource | Holds |
  | --- | --- | --- |
  | `DB` | D1 database `jarrow` | Board rows, queue, users, `errors` and `events` fallbacks |
  | `BOARD` | KV namespace `jarrow-board` | Cached `board.json`, rate-limit counters |
  | `REPORTS` | R2 bucket `jarrow-reports` | Per-mod reports and rule-table snapshots. Never JARs |

- **Static pages** work without JavaScript (docs and board are server-rendered). The converter needs JavaScript and says so.
- **Design tokens** come from `crates/report`, which generates `site/tokens.css`, so badge colours and site colours have one source; `site/scripts/check-colors.mjs` fails the site build on any other colour. Green, amber and red are reserved for result tiers and warnings.
- **Auth** is GitHub OAuth only (M4). No email and password.

Other Workers (the board ingest Worker, the Tier B queue Worker) keep their configuration in `infra/`.

## Board data flow

Planned for M1.

1. `.github/workflows/board.yml` runs nightly at 02:00 UTC, sharded across 10 runners. Each shard runs `jarrow board-run` over its slice of the top-1,000 Fabric mods for the pinned versions: classify, convert (Tier A only in the nightly), harness.
2. A final job merges the shards into `board.json` plus one report JSON per mod, and uploads them as assets of a GitHub release tagged `board-latest`, replacing the previous night's assets. These files hold results and reports only, never a JAR and never decompiled code.
3. A scheduled Cloudflare Worker (configured in `infra/`) fetches the `board-latest` assets from GitHub's public API and writes `board.json` to KV (`BOARD`), rows to D1 (`DB`) and reports to R2 (`REPORTS`). This keeps a Cloudflare API token out of GitHub. If a `CLOUDFLARE_API_TOKEN` secret exists, the workflow may write to Cloudflare directly instead.
4. The same workflow runs `tools/ledger.py` to regenerate `LEDGER.md`, regenerates the badges, and posts a summary to GitHub Discussions (in "Announcements" until the owner creates "Nightly results", amendment A-3).

The site reads `board.json` from KV at `/api/board.json`. Until the first nightly run, the board has no rows and the counter shows zero.

## Determinism contract

Same input JAR + same rule-table version + same mapping version + same engine version = byte-identical output JAR and identical report. This is a test, not an aspiration; the board relies on it to skip mods whose inputs have not changed.

What makes it hold:

- `crates/jar` writes entries in a fixed order with fixed timestamps and fixed compression settings, and strips signatures.
- No step reads the clock, the host name, the locale, environment variables or random numbers, and no output depends on hash-map iteration order.
- The rule-table version is a content hash of the `rules/` tree for the game version; the mapping version is the hash of the downloaded mapping files, recorded in `research-lock.json`.
- The test converts the same input twice, in separate processes, and compares SHA-256 hashes. It runs in CI on all three operating systems, and outputs must match across them.

The board's cache key is the tuple (input JAR SHA-256, rule-table hash, mapping hash, engine version).
