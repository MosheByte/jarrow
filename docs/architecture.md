# Architecture

One Rust workspace is the product. The website, the CLI and the GitHub Actions jobs are thin shells around it. Nothing conversion-related is written in TypeScript. This page is the map; the decisions behind it are logged in [DECISIONS.md](../DECISIONS.md).

Status on 2026-11-03: `crates/report` and `crates/cli` exist as M0 skeletons, and `crates/jar`, `crates/classfile`, `crates/mappings`, `crates/harness` (Fabric and NeoForge only) and `crates/board` exist from M1 (see the table). Nothing converts yet, in any direction. The nightly board measures one direction, Fabric → NeoForge, with conversion stubbed, so every mod in that column is red; no other direction has a board column. Everything below marked "planned" is a design, not code.

## One engine, four loader profiles

The goal for v1 is any loader to any loader: Fabric, NeoForge, Forge and Quilt, all 12 directions, on Minecraft 1.20.1, 1.21.1 and 1.21.11 (D-029). There is one engine for all of them, not one converter per pair.

- **Loader profiles** ([loaders.md](loaders.md)). Everything that differs between loaders is data in `loaders/<name>.toml`: the manifest, mod-id rules, entrypoints, event and registration models, runtime names, Mixin, access files, resource conditions, networking, how the harness installs a server, and the API library. A profile is filled only from research. A fact that is not researched is `"unknown"`, and the engine stops on it instead of guessing. Today Fabric and NeoForge are researched for 1.21.1 and 1.21.11 (two NeoForge fields still open). For Forge, Quilt and 1.20.1, `research-lock.json` pins the builds and records their runtime names, Mixin builds and server installs, and [research/loaders.md](research/loaders.md) now describes much of their mod-facing APIs; the profiles are not updated from it yet ([loaders/README.md](../loaders/README.md)).
- **Rule tables** ([rules.md](rules.md)). What one loader's API shape becomes in another's lives in `rules/<from>-<to>/<game-version>/`. Tables come in inverse pairs (A → B with B → A), and each pair is tested by an A → B → A round trip that the harness scores.
- **Direction pages** ([directions/](directions/)). Each of the 12 directions has `docs/directions/<from>-<to>.md`, written before its tables.
- **The build order is fixed**: Fabric → NeoForge; NeoForge → Fabric; Fabric ↔ Quilt; Forge ↔ NeoForge; Fabric ↔ Forge; then Quilt ↔ Forge and Quilt ↔ NeoForge. Milestones M0 to M6 in [ROADMAP.md](../ROADMAP.md) build the engine and the first direction.

The engine's entry point is `convert(jar, source loader, target loader, game version)`. The crates below read the two profiles and the direction's rule tables; none of them names a loader in code.

## Crates

Every package is named `jarrow-<crate>` (library `jarrow_<crate>`), except the CLI package, which is `jarrow` with the binary `jarrow`. Edition 2024, MIT. CI checks that every directory under `crates/` appears in this table.

| Crate | Owns | Notes | Status |
| --- | --- | --- | --- |
| `crates/loaders` | Loads and validates the loader profiles in `loaders/` | Refuses a profile that breaks [loaders.md](loaders.md); answers "unknown" as an error, never as a default | Planned (M1) |
| `crates/jar` | Reading and writing JARs and manifests; signatures stripped; deterministic output | Pure Rust zip; byte-stable output so two runs give identical hashes | Exists (M1: read with zip64 and zip-bomb limits, deterministic write, manifests, signature stripping, nested jars; [jar.md](jar.md)) |
| `crates/classfile` | Java class-file parser and writer: constant pool, attributes, stack maps | Bytecode only, no decompilation. Round-trip tests on the JDK class library and the board set | Exists (M1: parser, byte-exact writer, instruction decoder, constant-pool helpers; JDK round trip in CI, fuzzed by ClusterFuzzLite; board-set round trip not run yet; [classfile.md](classfile.md)) |
| `crates/mappings` | Mapping files and formats: intermediary, Mojang, Yarn, Parchment, and the SRG (MCP config) and Quilt hashed sets once they are pinned; tiny v2 and ProGuard; composition and inversion | Mojang mappings are fetched from the version manifest at runtime, never vendored. Namespace ids are listed in [loaders.md](loaders.md#versionsvmappings) | Exists (M1: tiny v1 and v2, ProGuard, TSRG and TSRG2 read and write; compose, invert, chain; see [mappings.md](mappings.md). Parchment not built yet) |
| `crates/remap` | Class, member and descriptor remapping over `classfile`, including string constants that name classes (reflection, mixin targets) | Remaps between the namespaces the profiles name: `intermediary`, `mojang`, `srg` and `quilt-hashed`. Rules match in Mojang names, so a conversion remaps the source to `mojang`, applies rules, then remaps to the target's runtime namespace. Differential-tested against tiny-remapper output | Planned (M2) |
| `crates/manifest` | Reads and writes each loader's manifest, mixin configs, access files and `pack.mcmeta`, as the profiles describe them | Profile-driven. The first pair is `fabric.mod.json` and `neoforge.mods.toml`, access wideners and access transformers. Lossless where possible; the report lists every field it could not translate | Planned (M1) |
| `crates/rules` | Rule-table loader and matcher: call rewrites, class substitutions, event bridges, entrypoint adapters | Data-driven; tables are TOML under `rules/<from>-<to>/<game-version>/` ([rules.md](rules.md)). Checks that every table has its inverse or a "not invertible" mark | Planned (M2) |
| `crates/rewrite` | Applies rules to bytecode: call-site rewriting, generated adapter classes, entrypoint shims | The target profile's entrypoint, event and registration models decide which adapters it generates. Every rewrite emits a report entry | Planned (M2) |
| `crates/classify` | The tier classifier: detects the source loader from the manifests the profiles name, scans the JAR and decides A, B or unsupported for a target, with ordered reasons | Budget: under 300 ms for a 5 MB JAR in WASM | Planned (M1) |
| `crates/report` | Report model, JSON and Markdown rendering, badge data, design tokens | One report format everywhere: browser, CLI, Actions, board. Generates `site/src/styles/tokens.css`; a test fails when the file drifts | Exists (M0 skeleton: tokens and the first report types; full model in M1) |
| `crates/engine` | Orchestrates the crates above into `convert(jar, source, target, game version)` | The one public API, the same for every direction | Planned (M2) |
| `crates/wasm` | `wasm-bindgen` surface for the browser: `classify`, `convert`, streaming progress | Budget: 4 MB gzipped | Planned (M3) |
| `crates/cli` | The `jarrow` binary: `convert`, `check`, `report`, `board-run` | Released with cargo-dist | Exists (M0 skeleton: `--version` and `tokens` only; `check` in M1, `convert` in M2) |
| `crates/harness` | Starts headless servers for all four loaders as their profiles describe, dumps registries, recipes, tags, loot and worldgen, diffs and scores; also scores round trips | Needs a JDK; runs in Actions and locally ([harness.md](harness.md)) | Exists (M1: dump schema and validation, hash-verified Fabric and NeoForge provisioning, EULA-gated runner, baseline noise, diff and score; Forge and Quilt not built yet) |
| `crates/board` | Fetches each source loader's top-1,000 list from Modrinth, runs classify, convert and harness per direction column, writes results | Nightly job ([board.md](board.md)) | Exists (M1 wave 1: `list`, `shard`, `merge` and `board.yml`, running the Fabric → NeoForge column only; classify, convert and the harness are not wired yet, so every mod is red with "no converter yet"; [board-pipeline.md](board-pipeline.md)) |

### Dependency direction

Lower layers never depend on higher ones.

```
report   loaders                (leaves: no Jarrow dependencies)
jar   classfile   mappings
manifest (loaders)
remap (classfile, mappings)
rules (classfile, loaders)
rewrite (classfile, rules, loaders, report)
classify (jar, classfile, manifest, rules, loaders, report)
engine (all of the above)
wasm (engine)     cli (engine, harness, board)
harness (jar, loaders, report)  board (engine, classify, harness, loaders, report)
```

## Conversion data flow

The same flow serves every direction. The source and target profiles and the direction's rule tables are its only inputs besides the JAR.

1. `classify(jar)` looks for each profile's manifest path. When the JAR carries manifests for more than one loader, it reports all of them and the caller names the source loader. It then reads the manifest, scans the classes and decides the tier for the chosen target, with reasons.
2. The engine refuses a direction whose rule tables do not exist, or a game version where either profile is `"not-researched"`, and says which.
3. `convert(jar, source, target, game version)` runs `manifest`, `remap` (source runtime names → Mojang names), `rules`, `rewrite` and `remap` again (Mojang names → the target's runtime names).
4. The result is the converted JAR plus the report.

### Tier A (browser and CLI)

Planned for M2 (CLI) and M3 (browser), for the first direction.

1. The browser loads the `wasm` bundle.
2. `classify(jar)` reads the manifest and scans the classes. The page shows mod id, name, version, loader, game version, license, dependencies and the tier with its reasons, within two seconds.
3. The license gate decides what is offered ([legal.md](legal.md), "License policy"): permissive and copyleft licenses convert; restrictive or missing licenses convert for personal use only; a mod on the opt-out list is refused.
4. `convert` runs through `engine` and streams progress. The page offers only directions that have a board column.
5. The result is a blob URL that exists only in that tab, plus the report.

No network request carries the JAR. The only telemetry is the anonymous event described in [telemetry.md](telemetry.md), and the user can turn it off.

The CLI runs the same `engine` code: `jarrow convert` and `jarrow check`.

## Tier B pipeline

Planned for M4. Lives in `tools/tierb/` (Python plus the CLI) and runs as `.github/workflows/tierb.yml`, one GitHub Actions job per conversion, 30-minute cap.

1. Download the JAR through a short-lived R2 presigned URL. The job deletes the object right after the download; the bucket also expires objects after 1 day as a backstop (amendment A-4).
2. Run `jarrow convert --tier a`. If static checks pass, skip to step 6.
3. Decompile with Vineflower into a scratch directory; remap to Mojang names; generate a Gradle project against the target loader's toolchain (for the first direction, NeoForge).
4. Apply rule tables as source transforms where bytecode rewriting was not possible, then compile. Compile errors go to the agent with the rule docs and the target loader's API index as context. The agent edits only files under `scratch/src/`, capped at 6 iterations and 400k tokens per job.
5. Build the JAR.
6. Run the parity harness, attach the report, return the result once, wipe the workspace.

Decompiled code never reaches the Actions log, an analytics or error service, or a model outside the job's own context.

## Web

`site/` is Astro + Starlight on Cloudflare Pages, project `jarrow`, served at `SITE_URL` = https://jarrow.pages.dev. Rust API docs are separate, on GitHub Pages at https://moshebyte.github.io/jarrow/.

- **Server routes and SSR pages** use the `@astrojs/cloudflare` adapter. The adapter compiles them into the Pages project's Worker, so they share one set of bindings. The first route is `/api/board.json`. Planned routes: `/api/queue` and `/api/auth/github` (M4), and the badge route `/badge/<modid>.svg` (M5). They were first specified as a separate `functions/` Worker; with the adapter they are Astro endpoints in the same deployment, with the same routes and bindings (D-016, amendment A-8 in [spec-amendments.md](spec-amendments.md)).
- **Bindings** are declared in `site/wrangler.toml` (amendment A-5):

  | Binding | Resource | Holds |
  | --- | --- | --- |
  | `DB` | D1 database `jarrow` | Board rows per direction, queue, users, `errors` and `events` fallbacks |
  | `BOARD` | KV namespace `jarrow-board` | Cached `board.json`, rate-limit counters |
  | `REPORTS` | R2 bucket `jarrow-reports` | Per-mod reports and rule-table snapshots. Never JARs |

- **Directions on the site.** The site shows a direction only when the board has a column for it, and then shows that column's real numbers. Before that, a direction is not offered, linked or counted.
- **Static pages** work without JavaScript (docs and board are server-rendered). The converter needs JavaScript and says so.
- **Design tokens** come from `crates/report`, which generates `site/src/styles/tokens.css`, so badge colours and site colours have one source. Green, amber and red are reserved for results.
- **Auth** is GitHub OAuth only (M4). No email and password.

Other Workers (the board ingest Worker, the Tier B queue Worker) keep their configuration in `infra/`.

## Board data flow

Built in M1, wave 1, for one column: the pipeline is `crates/board` and `.github/workflows/board.yml` ([board-pipeline.md](board-pipeline.md)), and the contract is [board.md](board.md). The classifier, the converter and the harness are not wired into it yet.

1. The board has one column per direction. A column exists once `.github/workflows/board.yml` runs that direction. Today it runs one, Fabric → NeoForge; the others follow the build order.
2. Each direction's source list is the top 1,000 mods of its source loader on Modrinth for the pinned versions. Directions with the same source loader share one list (`board-list-<loader>.json`).
3. The nightly runs at 02:00 UTC, in 10 shards. Each shard runs `jarrow-board shard` over its slice of a column: download and verify each file, inspect it, then classify, convert (Tier A only in the nightly) and harness, once those exist.
4. A final job merges the shards into `board.json`, `board-summary.json` and one report JSON per mod, direction and game version, and writes them to D1 (`DB`), KV (`BOARD`: every report, then `board.json`) and R2 (`REPORTS`: the reports tarball) with the `CLOUDFLARE_API_TOKEN` secret (D-021). The same files go to the GitHub release `board-latest` as a public mirror. These files hold results and reports only, never a JAR and never decompiled code.
5. The same workflow runs `tools/ledger.py` to regenerate `LEDGER.md` and posts a summary per column to GitHub Discussions in "Nightly results" (D-011). Badges come in M5.

The site reads `board.json` from KV at `/api/board.json`. Until the first nightly run, the board has no columns and no rows, and every count is zero.

## Determinism contract

Same input JAR + same direction + same loader profiles + same rule-table version + same mapping version + same engine version = byte-identical output JAR and identical report. This is a test, not an aspiration; the board relies on it to skip mods whose inputs have not changed.

What makes it hold:

- `crates/jar` writes entries in a fixed order with fixed timestamps and fixed compression settings, and strips signatures.
- No step reads the clock, the host name, the locale, environment variables or random numbers, and no output depends on hash-map iteration order.
- The rule-table version is a content hash of `rules/<from>-<to>/<game-version>/`; the profile version is a content hash of the two profiles; the mapping version is the hash of the downloaded mapping files, recorded in `research-lock.json`.
- The test converts the same input twice, in separate processes, and compares SHA-256 hashes. It runs in CI on all three operating systems, and outputs must match across them.

The board's cache key is the tuple (input JAR SHA-256, direction, rule-table hash, profile hash, mapping hash, engine version).
