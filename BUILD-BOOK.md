# Jarrow — Build Book v1

Nov 3, 2026 · @Moshe Barami

*Name: Jarrow. Chosen 2026-11-03 after checking crates.io, npm and GitHub: the crate and npm package names are free and no software product uses it; it is an English town, it carries "jar" and sounds like "jar + arrow", and the Jarrow March of 1936 was people carrying their cause across the country — a mod carried across loaders. A trademark check by the owner is still due before launch; the domain is the owner's choice (jarrow.dev if free, else getjarrow.dev or jarrow.app). The agent treats the name as one constant, PRODUCT\_NAME = "Jarrow"; CLI and package name jarrow; GitHub org jarrow (fallback jarrow-dev).*

## 0. Agent operating contract

You are the sole engineer. The owner will not review code, answer questions mid-build, or approve steps. Your job ends when a stranger can open the website, drop a Fabric mod JAR, and get a working NeoForge JAR with a parity report — and when the repository, CI, releases, docs and the public board all exist and work. Anything short of that is not done.

**Rules of engagement**

1. Never stop to ask. When a decision is open, take the default in section 13, log it in `DECISIONS.md` as `D-<n>`, and continue. Only the three conditions in section 2 ("When you may stop") justify pausing.
2. Work in small pull requests on branches, merged by CI. Every PR runs `tools/pr_ready.py` and the harnesses its changes touch. `main` is always releasable.
3. Every commit carries `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. Transparency about agent authorship is a feature of this project, not a disclosure to hide.
4. Read before you write. Before touching a subsystem, read `AGENTS.md`, `docs/architecture.md` and the subsystem's doc. Keep both files current: they are the map the next session uses.
5. The proof decides, not your confidence. A conversion is "working" when the parity harness says so, never because the code compiled.
6. Clean room. You learn Fabric and NeoForge from their public documentation, their open-source API repositories (Apache/MIT/LGPL as licensed) and from mods you are permitted to read. You never copy code from a user's mod into the engine. Rule tables describe API shapes, not mod bodies.
7. No invented icons, logos or screenshots. Icons come from the sets in section 8; screenshots come from real runs.
8. Ship early, ship nightly. A nightly release exists from milestone M1 on. A broken nightly is a P0.
9. Keep a `LEDGER.md` the owner can read in five minutes: what works, what is partial, what is unsupported, with numbers from the last nightly.
10. When the book and reality conflict, reality wins — record the conflict in `DECISIONS.md` and amend the book in `docs/book-amendments.md`.

## 1. Mission and definition of done

**One sentence:** Drop a Minecraft mod JAR on a web page; get the same mod back for another loader, with a proof that it behaves the same.

**Why it wins:** today a Fabric mod only reaches NeoForge players if the author ports it or if players install a runtime shim (Sinytra Connector + Forgified Fabric API). Nobody converts the JAR itself. The convert step is mechanical for most content mods; the hard remainder is where an autonomous agent plus a behaviour harness changes the economics.

**v1 ships exactly this**

- Direction: Fabric → NeoForge. Game versions: 1.21.x (pin the exact list in `research-lock.json` on day one; start with the two most-downloaded 1.21 minors on Modrinth).
- Tier A conversions run entirely in the browser (Rust → WASM). The JAR never leaves the user's machine.
- Tier B conversions (decompile → rules → agent fix → recompile → harness) run as ephemeral GitHub Actions jobs for signed-in users, with a queue and Turnstile.
- A parity report per conversion and a public board: the 1,000 most-downloaded Fabric mods for the pinned versions, re-run nightly, each marked green / yellow / red with the reason.
- A CLI (`jarrow convert, jarrow` `check`) and the WASM package on npm, so modders and CI can use the engine without the site.
- Author mode: a mod author signs in with GitHub, converts their own mod, and receives a pull request against their repository with the port and the report.
- Website: landing, converter, board, docs, blog of nightly results, status, badges.

**Not in v1 (do not build, do not stub)**

- NeoForge → Fabric, Forge, Quilt, Bukkit/Paper, Bedrock.
- Game-version translation (1.20 → 1.21). Reserve the tab and the route; ship a clear "planned" page.
- Mod compatibility checker, dependency updater, profiler (family B of the idea list). Later tabs.
- Any hosting, caching or redistribution of converted JARs. Ever.
- Accounts beyond GitHub OAuth. No email/password.
- Payments.

**Done means all of the following are true at once**

1. The nightly board shows ≥ 60% of the 1,000 mods green for Tier A, with every red carrying a classified reason.
2. A new user converts a green mod in the browser in under 60 seconds on a laptop, and the result loads in a real NeoForge client without errors.
3. `cargo test`, clippy, the Python tests, the harness suite and the site build all pass on `main`, on Linux, macOS and Windows runners.
4. Signed releases exist for the CLI (via cargo-dist: GitHub Releases, Homebrew tap, Scoop, winget, AUR) and for the npm WASM package; `LEDGER.md` and the board agree with each other.
5. Every service in section 9 that is connected is configured, monitored and documented; every service that is not connected has a working fallback and a one-line note in `infra/INVENTORY.md`.
6. Docs cover install, first conversion, tiers, author mode, the license policy, the rule-table format and how to add a rule.

## 2. Environment discovery protocol (run first, re-run every session)

The owner has connected a set of services through MCP connectors and the shell. Which ones exist is not fixed; you find out, you never assume. The output of this protocol is `infra/INVENTORY.md`, committed, and it is the first thing you write in milestone M0.

**Step 1 — enumerate.** List every tool available to you: MCP servers (names and tool lists), shell credentials (`gh auth status`, `wrangler whoami`, `supabase projects list`, `sentry-cli info`, environment variables that look like API keys), repositories you can push to, and files the owner attached. Record each as a row: service · what it is for in this project · evidence that it is connected · status.

**Step 2 — classify every service into exactly one of three columns.**

| Column | Meaning | What you do |
| --- | --- | --- |
| Connected | A tool or credential works end to end (a read call succeeds) | Configure it fully yourself, document it, wire it into CI |
| Self-configurable | Not connected, but you can create or set it up with what you have (a GitHub Action, a `wrangler` deploy, a D1 database, a Discord webhook via a token you were given) | Create it, record what you created and where its secrets live |
| Needs the owner | Requires a human action you cannot perform: paying, verifying a domain's DNS, approving an OAuth app, accepting terms, signing up for an OSS program, adding a secret to GitHub that only they hold | Do everything up to that point, write the exact steps in `infra/TODO-OWNER.md`, use the fallback, and continue |

**Step 3 — fallbacks (never block on a missing service).**

| Wanted | Fallback if absent |
| --- | --- |
| Custom domain | `*.pages.dev`; all links built from one `SITE_URL` constant |
| Supabase | Cloudflare D1 + KV, GitHub OAuth in a Worker (this is also the v1 default; see section 9) |
| Sentry | Structured logs to Workers logs + a `errors` table in D1; swap when Sentry is connected |
| PostHog | Cloudflare Web Analytics; events to a `events` table in D1 |
| Resend | No email in v1; notifications through the GitHub PR/issue the job created |
| UptimeRobot / BetterStack | Upptime (GitHub Actions + Pages) |
| GitLab | Mirror job disabled with a note |
| Self-hosted runner | GitHub-hosted runners only; Tier B queue depth capped |

**Step 4 — verify, don't trust.** For every "Connected" row, perform one harmless real call (list projects, read a dashboard, send a test event to a test environment) and record the result. A connector that is listed but fails is "Needs the owner" with the error pasted.

**Step 5 — secrets.** Secrets live in GitHub Actions secrets and Cloudflare bindings, never in the repository, never in `INVENTORY.md`. Record the secret's *name* and where it is used. If you created a key, say so, so the owner can rotate it.

**Step 6 — "Needs the owner" is a proof of failure, not an estimate.** An item goes into `infra/TODO-OWNER.md` only after you tried REST, GraphQL, the vendor CLI and the vendor API with the credentials you already hold, and you can paste the exact command and error. A connector you can reach is yours to finish.

**Step 7 — reporting never pauses work.** Commit and push what is done; keep working while you report. The only owner-only item in M0 is the Minecraft EULA. OAuth app, Sponsors, Discord, registry tokens and the org transfer belong under their milestone headings in `infra/TODO-OWNER.md`, not in a milestone report.

*(Steps 6 and 7 recorded on 2026-11-03 from the owner's correction message; see docs/book-amendments.md A-9.)*

**When you may stop (the only three cases)**

1. A legal boundary: you would have to redistribute someone else's mod, assets or decompiled source, or violate a license you detected.
2. A destructive, irreversible action on something you did not create (deleting the owner's repository, database or domain configuration).
3. A payment or a terms acceptance.

In every other case you choose, log the choice, and proceed.

## 3. Product spec

**The main flow (Convert tab).** Drop zone accepts one or more `.jar` files. Within two seconds the page shows, for each: mod id, name, version, loader, game version, license (from `fabric.mod.json`), dependencies, and the **tier** the classifier assigned (A / B / unsupported) with the reason. The user picks the target (v1: NeoForge, pinned version list). Tier A converts in the browser with a progress log; the result is a download link that exists only in that tab, plus a **report** (what was remapped, which rules fired, which API calls were rewritten, what was left untouched and why). Tier B offers "Queue a deep conversion" for signed-in users and shows queue position; the result and report arrive on the page and, optionally, as a GitHub Gist the user owns.

**Tiers**

| Tier | What it covers | Where it runs | Proof |
| --- | --- | --- | --- |
| A | Manifest, mappings, entrypoints, registries, resources, mixins, access wideners, Fabric API calls covered by rule tables | Browser (WASM) or CLI | Static checks + harness in nightly |
| B | Everything in A plus API calls with no rule, event-model differences, networking, rendering hooks the agent can port | GitHub Actions job (JVM + agent) | Full parity harness before the result is returned |
| Unsupported | Native libraries, coremod-style bytecode transformers, deep rendering pipelines, obfuscated or packed JARs, mods whose license forbids modification | Nowhere | The page says so plainly and names the reason |

**Board tab.** The 1,000-mod nightly: searchable table with mod · downloads · tier · result · reason · last run · badge link. Each row opens the full report. A mod author who verified ownership can mark "official port exists" (link) or "do not port" (honoured: the mod is hidden from the board and refused in the converter).

**Author mode.** Sign in with GitHub → the site lists repositories that contain a `fabric.mod.json` → "Port this" runs Tier B on the repository's latest release JAR and opens a pull request on a new branch with: the converted JAR's build setup (a Gradle subproject or a `neoforge/` source set when decompilation succeeded cleanly), the report, and a badge snippet for the README. Authors are the people whose goodwill decides this project; author mode is where the product is kindest.

**Docs tab.** Starlight site: install, first conversion, tiers, report format, author mode, rule tables and how to contribute one, the harness, the license policy, FAQ ("is this legal", "why not Connector", "why is my mod red").

**Later tabs (routes reserved, pages say "planned", linked to the roadmap issue):** Version translate · Compat check · Dependency updater · Profiler.

**License policy (enforced in code, not only in text)**

1. Read the license field of every manifest. Known permissive (MIT, Apache-2.0, LGPL, MPL, CC0, Unlicense): convert. Known copyleft with modification allowed (GPL): convert, and the report says the result inherits the license. "All rights reserved", missing, or `SEE LICENSE IN` pointing to a restrictive file: convert for personal use only, show a notice, and never offer the Gist/PR path unless the signed-in user owns the repository.
2. A mod on the author "do not port" list is refused everywhere.
3. The site stores zero JAR bytes. Tier B jobs delete their workspace at the end; the Actions log must not contain decompiled source.
4. The public board links to the original mod's page; it never links to a converted JAR.

## 4. Architecture

One Rust workspace is the product; the website, the CLI and the Actions job are three thin shells around it. Nothing conversion-related is written in TypeScript.

**Crates (`crates/`)**

| Crate | Owns | Notes |
| --- | --- | --- |
| `jar` | Reading and writing JARs, manifests, signatures stripped, deterministic output | Pure Rust zip; byte-stable output so two runs give identical hashes |
| `classfile` | Java class-file parser and writer, constant pool, attributes, stack maps | No decompilation here; bytecode-level only. Round-trip tests on the whole JDK class library |
| `mappings` | Intermediary, Mojang official, Yarn, Parchment; tiny/v2 and ProGuard formats; composition and inversion | Mojang mappings are fetched from the version manifest at runtime, never vendored |
| `remap` | Class, member and descriptor remapping over `classfile`, including string constants that name classes (reflection, mixin targets) | Differential-tested against tiny-remapper output |
| `manifest` | `fabric.mod.json` ⇄ `neoforge.mods.toml`, `mixins.json`, access wideners ⇄ access transformers, `pack.mcmeta` | Lossless where possible; the report lists every field it could not translate |
| `rules` | Rule-table loader and matcher: API call rewrites, class substitutions, event bridges, entrypoint adapters | Tables are TOML under `rules/<from>-<to>/<game-version>/`; the crate is data-driven |
| `rewrite` | Applies rules to bytecode: call-site rewriting, synthetic adapter classes, entrypoint shims | Every rewrite emits a report entry |
| `classify` | The tier classifier: scans a JAR and decides A / B / unsupported with reasons | Must run in under 300 ms for a 5 MB JAR in WASM |
| `report` | Report model, JSON + Markdown rendering, badge data | Same report everywhere: browser, CLI, Actions, board |
| `engine` | Orchestrates the above into `convert()` | The one public API |
| `wasm` | `wasm-bindgen` surface for the browser: `classify`, `convert`, streaming progress | Size budget 4 MB gzipped |
| `cli` | `jarrow` binary: convert, check, report, board-run | Built with cargo-dist |
| `harness` | Launches headless Fabric and NeoForge servers, dumps registries/recipes/tags/loot/worldgen, diffs, scores | Needs a JDK; runs in Actions and locally |
| `board` | Fetches the top-1,000 list from Modrinth, runs classify + convert + harness, writes results to D1 and R2 | Nightly job |

**Tier B pipeline (`tools/tierb/`, Python + the CLI, runs in a GitHub Actions job)**

1. Download the JAR from the user's upload token (short-lived R2 presigned URL; deleted after the job).
2. `jarrow convert --tier a` → if green in static checks, skip to 6.
3. Decompile with Vineflower into a scratch dir; remap to Mojang official names; generate a Gradle project against the NeoForge toolchain.
4. Apply rule tables as source transforms where bytecode rewriting was not possible; compile. Compile errors go to the agent with the rules' documentation and the NeoForge API index as context; the agent edits only files under `scratch/src/`, with a hard cap on iterations and tokens per job.
5. Build the JAR.
6. Run the parity harness; attach the report; return the result; wipe the workspace.

**Web (`site/`)**: Astro + Starlight on Cloudflare Pages; a `functions/` Worker for API routes (`/api/board`, `/api/queue`, `/api/auth/github`, `/api/badge/:mod.svg`); D1 for board rows, queue and users; KV for cached board JSON; R2 for reports and rule-table snapshots (never JARs). GitHub OAuth only.

**Data flow for a Tier A conversion:** browser loads `wasm` → `classify(jar)` → shows tier and license → `convert(jar, target)` streams progress → result blob + report → download. No network call carries the JAR. The only telemetry is an anonymous `conversion_completed` event with mod id, tier, duration, success, rule-hit counts — opt-out in settings.

**Determinism:** same input JAR + same rule-table version + same mapping version = byte-identical output. This is a test, not an aspiration; the board relies on it to dedupe.

## 5. Conversion engine spec — Fabric → NeoForge, 1.21.x

Research first, then tables, then code. Before writing a rule, read the two APIs from their public repositories and docs (Fabric API, Fabric Loader, NeoForge, NeoForge docs, Sinytra Connector's public issue tracker for *known* divergences, Architectury's public source for *how* the two models are commonly bridged). Record every source in `rules/SOURCES.md`. Never copy Connector or Architectury code; learn the shape, write your own.

**5.1 Manifest.** `fabric.mod.json` → `META-INF/neoforge.mods.toml`: id, version, name, description, authors, contact, license, icon, environment (`client`/`server`/`*` → `side`), dependencies with version ranges converted between Fabric's semver predicates and NeoForge's Maven ranges, `provides`, `custom` namespaces preserved under `[modproperties]`. Unknown keys → report.

**5.2 Mappings.** Fabric production JARs use intermediary names; NeoForge 1.20.2+ uses Mojang official names at runtime. Chain: intermediary → official via the version's `intermediary` tiny file + Mojang's ProGuard mappings (downloaded from the version manifest under its license; cached in R2 per version). Remap class references, members, descriptors, signatures, and string constants that name classes or members (mixin `@At` targets, reflection, `Identifier` strings are *not* touched). Verify by remapping Fabric API itself and diffing against Yarn-to-Mojang truth tables.

**5.3 Entrypoints.** `ModInitializer`, `ClientModInitializer`, `DedicatedServerModInitializer`, `PreLaunchEntrypoint` → one generated `@Mod` class per mod that calls the original initializers at the matching NeoForge lifecycle stage (`FMLCommonSetupEvent`, `FMLClientSetupEvent`, `FMLDedicatedServerSetupEvent`). Custom entrypoint keys (e.g. mod-menu, REI) → report as "plugin entrypoint for `<key>`, not translated".

**5.4 Registries.** Fabric's direct `Registry.register` at init time → NeoForge's `DeferredRegister` model. Strategy: a generated registration bridge that captures `register` calls during the original init and replays them into `RegisterEvent` for the right registry key. Order and ids preserved. This is the single most important rule family; measure it with the registry dump in the harness.

**5.5 Events.** Fabric API event interfaces (`ServerTickEvents`, `PlayerBlockBreakEvents`, `UseBlockCallback`, `AttackEntityCallback`, `ServerLifecycleEvents`, …) → NeoForge event bus subscribers, one rule per event with a documented semantic difference (cancellation, phases, return values). Events without a clean counterpart are Tier B with the difference stated.

**5.6 Mixins.** Both loaders run Mixin. Rewrite `mixins.json` package/refmap references; remap targets to official names; regenerate the refmap. Mixins into Fabric API classes → rule or Tier B. `@Overwrite` on vanilla methods → allowed, flagged in the report.

**5.7 Access.** Access wideners → access transformers (`META-INF/accesstransformer.cfg`), with name remapping.

**5.8 Resources.** Data packs, resource packs, lang files, models: copied; loader-specific resource conditions (`fabric:load_conditions` ↔ `neoforge:conditions`) translated.

**5.9 Fabric API surface.** Build an index of every public Fabric API class/method for the pinned version (from the published jars) and a table mapping each to: `rule:<id>` · `tier-b` · `unsupported`. The ledger metric is the share of the index covered by rules, weighted by how often the 1,000 mods call each symbol. The agent extends the table where the weighted coverage gains most.

**5.10 Tier classifier.** Scans without converting: loader manifest present · native libs (`.so/.dll/.dylib`) · custom language adapters · `fabric-loader` internals use · bytecode transformer registration · obfuscation signatures · Fabric API symbols hit, mapped through 5.9 · license. Output: tier + ordered reasons + an estimate of rule coverage.

**5.11 Report entries** are structured (`kind`, `location`, `before`, `after`, `rule`, `confidence`) so the board can aggregate "top 50 reasons mods are red" — which is the agent's own work queue.

## 6. Proof: the parity harness and the board

The project's credibility is one number that goes up every night and a report behind every row. Build the harness in M1, before the converter is good, so the converter is measured from its first day.

**6.1 The harness (`crates/harness`)**

1. Provision two headless dedicated servers for the pinned game version: Fabric Loader + Fabric API with the original JAR; NeoForge with the converted JAR. Pinned loader/API versions live in `research-lock.json`. Servers run with the same seed, `online-mode=false`, no other mods, a fixed `server.properties`.
2. A tiny probe mod (one per loader, part of this repository, MIT) dumps after startup: registries (blocks, items, entities, block entities, fluids, sounds, recipe serializers, loot functions, attributes, enchantments, biomes, features) with ids and ordinal positions; recipes, tags, loot tables, advancements as canonical JSON; data-pack and resource conditions resolved; creative tabs and their contents; a worldgen sample (N chunks at fixed coordinates, block-state histogram); server log with loader-specific noise filtered.
3. Diff canonical dumps. Score: `1.0` identical; `0.75` only ordering/noise differences; `0.5` missing or extra entries under 1%; `0.25` the mod loads but entries differ materially; `0` the mod fails to load. A red reason is attached from the log (class not found, mixin failure, registry conflict, crash).
4. Optional client smoke (M4+): headless NeoForge client with a software renderer in CI, joins the server, screenshots the creative tab; the image is kept as evidence, not scored.

**6.2 The nightly board (`crates/board` + `.github/workflows/board.yml`)**

- Source list: Modrinth API, top 1,000 Fabric projects by downloads for the pinned versions, refreshed weekly; CurseForge later if its API terms allow.
- Per mod: classify → convert (Tier A only in the nightly; Tier B results come from author mode and from a weekly batch limited to 50 mods) → harness → row in D1 + report in R2.
- Output: `board.json` to KV, a Markdown summary posted as a GitHub Discussion ("Nightly 2026-11-04: 612 green, 231 yellow, 157 red; top 5 reasons…"), badges regenerated.
- Budget: the nightly must finish under 5 hours on GitHub-hosted runners with a matrix of 10 shards; when it cannot, shard further or move the harness to the self-hosted runner if one is connected.

**6.3 Ledger and badges**

- `LEDGER.md`: coverage of the Fabric API index (weighted), board totals, open red reasons with counts, harness pass rate, WASM size, median conversion time — regenerated by the nightly, committed by the bot.
- Badges (shields.io endpoint from `board.json`): `parity: 61% green`, `fabric-api coverage: 74%`, `nightly: passing`. A per-mod badge `jarrow: green` that authors can put in their README.

**6.4 Differential tests for the engine itself**

- `remap` vs tiny-remapper on Fabric API and on 100 sampled mods: identical class files or an explained diff.
- `classfile` round-trip on the JDK and on every JAR in the board set: byte-identical re-serialisation.
- `manifest` round-trip on all 1,000 manifests.
- Determinism test: two runs, identical hashes.

Nothing is marked "works" in docs or the site unless a harness result backs it. Where the harness cannot measure something (client-only rendering), the docs say "not measured".

## 7. Repository layout and GitHub infrastructure

One public repository under a GitHub organisation named after the product. The repository is itself part of the product: people will star it for how it is run.

```
AGENTS.md  CLAUDE.md → AGENTS.md  CONTRIBUTING.md  LICENSE (MIT)  README.md  ROADMAP.md  LEDGER.md  DECISIONS.md  SECURITY.md  CODE_OF_CONDUCT.md
research-lock.json          pinned game, loader, API, mapping versions and hashes
crates/                     the workspace (section 4)
rules/<fabric-neoforge>/<1.21.x>/*.toml   rule tables, one file per API area, + SOURCES.md
site/                       Astro + Starlight + Workers functions
tools/                      pr_ready.py, ledger.py, board_summary.py, tierb/
tests/fixtures/             permissively-licensed sample mods (vendored with their LICENSE), golden outputs
docs/                       architecture.md, harness.md, rules.md, book-amendments.md
infra/                      INVENTORY.md, TODO-OWNER.md, wrangler.toml, d1/migrations, dashboards-as-code
.github/                    workflows, issue forms, PR template, CODEOWNERS, FUNDING.yml, dependabot.yml
```

**Workflows (`.github/workflows/`)**

| Workflow | Trigger | Does |
| --- | --- | --- |
| `ci.yml` | every PR, push to main | fmt, clippy (deny warnings), `cargo test` on Linux/macOS/Windows, WASM build + size check, Python tests, site build, `pr_ready.py` |
| `harness.yml` | PR touching `crates/` or `rules/` | runs the harness on the 25-mod smoke set |
| `board.yml` | nightly 02:00 UTC, manual | the 1,000-mod run, sharded; writes D1/R2/KV; posts the Discussion |
| `tierb.yml` | `workflow_dispatch` from the queue Worker | one conversion job; wipes workspace; 30-minute cap |
| `release.yml` | tag `v*` | cargo-dist: binaries for 3 OS, Homebrew/Scoop/winget/AUR, SBOM (syft), Sigstore signing, SLSA provenance; npm publish of the WASM package with provenance |
| `nightly.yml` | nightly after board | `nightly-YYYYMMDD` prerelease of CLI + WASM |
| `deploy-site.yml` | push to main under `site/` | Cloudflare Pages deploy, preview URL on PRs |
| `mirror.yml` | push to main | push mirror to GitLab (and Codeberg if configured) |
| `codeql.yml`, `scorecard.yml` | weekly | security scanning, OpenSSF Scorecard badge |
| `stale.yml`, `labeler.yml`, `release-please.yml` | — | hygiene, auto-labels, changelog + version PRs |

**Repository settings you configure yourself (via `gh api`)**: branch ruleset on `main` (CI + harness required, linear history, signed commits optional), merge queue, Discussions on (categories: Announcements, Nightly results, Rule proposals, Q&A, Show your port), Projects board "Roadmap" fed by labels, Issue Forms (`bug`, `red-mod-report` with mod URL + report hash, `rule-proposal`, `author-opt-out`), labels (`tier-a`, `tier-b`, `rule`, `harness`, `site`, `needs-proof`, `good-first-rule`), Dependabot + Renovate for Rust/npm/Actions, secret scanning + push protection, CodeQL, Sponsors enabled (`FUNDING.yml`), social preview image generated from the site.

**`AGENTS.md`** is the map: crates and what each owns, how to build and test fast, where each rule family lives, how to run the harness locally, the three stop conditions, and "rules that bite" (never commit a JAR, never commit decompiled source, never touch `research-lock.json` without a `D-` entry).

## 8. Website and design direction

The site has one job above the fold: a drop zone that works. Everything else supports it.

**Pages**: `/` (drop zone, three-line explanation, live counter from `board.json`, three badges) · `/convert` (the full converter with report view) · `/board` (the nightly table, filters, per-mod pages at `/board/<modid>`) · `/docs/*` (Starlight) · `/authors` (author mode) · `/roadmap` (rendered from `ROADMAP.md` + the Projects board) · `/status` (Upptime or the connected monitor) · `/blog` (nightly summaries and release notes, generated) · `/badge/<modid>.svg` (Worker).

**Design direction.** Saturated, playful, confident — but legible. Rules: one accent hue per tier (green/amber/red are reserved for results and never used decoratively); a second brand accent for actions; dark mode first; large type; the drop zone animates on hover and on completion (a short, satisfying "JAR flips over" transition), nothing else moves on its own. Motion must respect `prefers-reduced-motion`. Measure with Lighthouse in CI: performance ≥ 95, accessibility 100 on `/` and `/board`.

**Icons and imagery — hard rules**

- UI icons: Lucide. Brand and loader logos: Simple Icons where present; Fabric and NeoForge logos only under their published brand guidelines, linked in `site/BRAND-SOURCES.md`; if a guideline forbids use, use a text wordmark.
- Mod icons on the board come from the mod's own Modrinth page URL at render time (hot-linked with attribution and a link back), never copied into the repository.
- No generated illustrations, no AI-drawn mascots, no stock photos. Screenshots on the site are real screenshots from the harness or the converter, captured by CI.
- OG images per board page are rendered by Cloudflare Browser Rendering from a template, not drawn.

**Copy.** Plain, short, honest. Every result page explains what a tier means in one sentence. The word "guaranteed" never appears; "verified by the harness" does, with a link to the report.

**Tooling.** Astro + Starlight, Tailwind with a tokens file (`site/tokens.css`) generated from `crates/report` so badge colours and site colours are one source; Pagefind for docs search (Algolia DocSearch later if granted); Fontsource for self-hosted fonts; no third-party scripts except the analytics in section 9.

## 9. Per-service setup

Everything below is conditional on section 2: a service you find connected gets the full setup; one you do not find gets its fallback and a line in `infra/TODO-OWNER.md`. All configuration is code in `infra/` (wrangler config, D1 migrations, dashboards exported as JSON, monitor definitions), applied by a workflow, so the owner can rebuild everything from the repository.

| Service | Use in this project | You configure | Owner may need to |
| --- | --- | --- | --- |
| Cloudflare Pages + Workers | Site, API routes, badges, OAuth callback | Project, bindings, preview deployments, `SITE_URL`, caching headers for `board.json` | Attach the custom domain (DNS) when they buy one |
| Cloudflare D1 | Board rows, queue, users, errors (fallback), events (fallback) | Database, migrations under `infra/d1/`, nightly export to R2 | — |
| Cloudflare KV | Cached `board.json`, rate-limit counters | Namespace + bindings | — |
| Cloudflare R2 | Reports, rule-table snapshots, mapping cache, short-lived Tier B upload bucket with lifecycle delete at 1 hour | Buckets, lifecycle rules, presign Worker | — |
| Cloudflare Turnstile | Guard the Tier B queue and the author PR action | Site key in the page, secret in a binding | Create the widget if the API token lacks the permission |
| Cloudflare Browser Rendering | OG images for board pages | Worker route, template | Enable the free tier if not on the account |
| Supabase (if connected) | Auth (GitHub provider), users, author opt-outs; D1 stays the board store | Project schema, RLS policies, keep-alive ping workflow every 3 days | Enable the GitHub provider with the OAuth app's secret |
| Sentry | Errors from the WASM bundle, the Workers, the Tier B job | Projects (`site`, `workers`, `tierb`), source maps upload in CI, release tagging, alert rule to the Discord webhook | Apply for the open-source plan |
| PostHog | Product events (`conversion_started/completed`, tier, duration, rule hits), feature flags (`tier_b_enabled`), session replay off by default | Project, event schema in `docs/telemetry.md`, flags, one dashboard "Conversions" exported to `infra/posthog/` | — |
| Resend | Email to signed-in users when a Tier B job finishes; weekly digest opt-in | Domain claim, templates, sending via a Worker, suppression list | Verify the sending domain's DNS |
| UptimeRobot / BetterStack | Monitors for `/`, `/api/board`, `/badge/health.svg`, a heartbeat for the nightly | Monitors, status page, alerts to Discord | Choose which one stays (keep both only if both are connected) |
| GitLab | Mirror of the repository and of CI (a `.gitlab-ci.yml` that runs `ci` only) | Project, mirror token, pipeline | Create the project if the token cannot |
| Discord | Community; webhook channels for nightly results, releases, alerts | Webhooks wired from Actions, Sentry, monitors | Create the server and give you a webhook URL |
| Firecrawl, Context7 | Research during the build: Fabric/NeoForge docs, API indexes, Modrinth pages | Use them; cite in `rules/SOURCES.md` | — |
| GitHub OAuth app | Sign-in for the site and author mode | Register under the org with `gh api`; store secrets as Worker bindings | Approve the app if org settings require it |

**Monitoring as code.** Every dashboard, monitor and alert is exported to `infra/` and re-applied by `infra.yml` on change. The owner should be able to delete a service and recreate it from the repository in one run.

**Telemetry rules.** Anonymous by default; no IPs stored; mod ids are public data; opt-out toggle on the site; `docs/telemetry.md` lists every event and field. Tier B jobs never send mod bytes or decompiled source to any analytics or error service — scrub stack traces for user class names before Sentry.

## 10. Legal, trust and community policy

The people who can sink this project are mod authors. The policy is written for them first.

- **No redistribution.** The service never stores, caches, hosts or links to a converted JAR. Conversions are local (Tier A) or ephemeral (Tier B, result returned once, workspace wiped).
- **Personal use by default.** Converting a mod you downloaded, for your own game, is how the converter is presented. Publishing a conversion is the author's right; author mode exists to make that easy for them and hard for everyone else.
- **Opt-out is absolute.** An author who verifies ownership (GitHub sign-in matching the mod's repository, or a Modrinth-linked account) can mark their mod "do not port"; the converter refuses it, the board hides it, and the refusal explains why. This is honoured even for permissive licenses.
- **License detection is enforced,** not advisory (section 3). Unknown or restrictive licenses get the personal-use path only.
- **Clean room.** The engine learns from public API sources and documentation. Rule tables reference API symbols, never mod code. Decompiled user code exists only inside a Tier B job and is never logged, stored or sent to a model outside the job's own context.
- **Trademarks.** "Minecraft" is Mojang's; "Fabric" and "NeoForge" belong to their projects. The product name, logo and domain must not include them; the site carries the standard "not affiliated" notice in the footer and in the README.
- **Agent authorship is disclosed** in every commit, the README and the About page. The project is a public example of agent-built software with proof; hiding that would be both dishonest and a missed story.
- **Reporting.** Vulnerability policy in `SECURITY.md`; a `red-mod-report` issue form for conversions that look wrong; an `author-opt-out` form for authors without GitHub.
- **Conduct.** Contributor Covenant; be polite to Sinytra, Fabric and NeoForge maintainers in every public word — they are the ecosystem this lives in, and the report should credit Connector where its public issues informed a rule.

The owner is not a lawyer and neither are you. `docs/legal.md` states the policy plainly and recommends authors read their own license. Where a question of law decides whether to ship a feature, apply section 2's first stop condition.

## 11. Milestones

Each milestone has an exit criterion the CI can check. You do not start the next until the current one is green on `main`; you may work ahead on branches.

| Milestone | Deliverable | Exit criterion |
| --- | --- | --- |
| M0 — Ground | Repository, org, settings, `AGENTS.md`, `research-lock.json`, `infra/INVENTORY.md`, `TODO-OWNER.md`, CI skeleton, site skeleton deployed to `*.pages.dev` with a "coming soon" that already shows the board counter (zero) | `ci.yml` green on 3 OS; site reachable; inventory complete with every service classified |
| M1 — Measure | `jar`, `classfile`, `mappings`, `manifest`, `classify`, `report`; the harness; the board pipeline running against the 1,000 list with convert stubbed (every mod "red: no converter yet") | Nightly posts its first Discussion; `LEDGER.md` generated; round-trip tests pass on the whole board set |
| M2 — Convert | `remap`, `rules`, `rewrite`, `engine`; manifest, mappings, entrypoints, registries, resources, mixins, access | ≥ 30% of the 1,000 green in the nightly; CLI nightly prerelease |
| M3 — Browser | `wasm`, the converter page, report view, license gate, telemetry opt-out | A green mod converts in the browser in < 60 s on a 2020 laptop; WASM ≤ 4 MB gzipped; Lighthouse targets met |
| M4 — Deep | Tier B pipeline, queue Worker, Turnstile, GitHub OAuth, author mode PRs, client smoke in the harness | 10 real author-mode PRs opened on fixture repositories you own; Tier B lifts the board to ≥ 50% green |
| M5 — Public | Docs complete, board filters, badges, status page, Sentry/PostHog/monitors wired, Discord webhooks, release v0.1 with signed artefacts on every channel | Definition of done, items 3–6 |
| M6 — Done | Rule-table growth driven by the "top red reasons" queue until the board crosses the v1 bar; polish from the adversarial review | Definition of done, item 1 (≥ 60% green) and item 2 |

**Cadence.** Nightly release from M1. A `DECISIONS.md` entry for every default taken. A weekly `docs/journal/<date>.md` with the numbers, what moved, what blocked — written for the owner, two paragraphs.

**Adversarial review at M3 and M6.** Spawn a reviewer agent that has not seen the code: give it the site URL, the CLI, five random green mods and five random red ones, and the book. It must try to break the converter, find a misleading claim on the site, and find a license case the gate mishandles. Fix what it finds before the milestone closes.

## 12. Quality bars and forbidden shortcuts

**Bars (checked in CI, failing blocks merge)**

- Rust: stable toolchain pinned in `rust-toolchain.toml`; `cargo fmt`; `clippy -D warnings`; `#![forbid(unsafe_code)]` in every crate except `wasm` bindings if unavoidable; `cargo deny` for licenses and advisories; MSRV documented.
- Tests: unit + property tests (proptest) for `classfile` and `remap`; golden tests for `manifest` and `rules`; the differential suite in 6.4; coverage reported to Codecov with a floor of 80% on `crates/` (excluding `cli` and `wasm` glue).
- Performance budgets: classify ≤ 300 ms, Tier A convert ≤ 10 s for a 5 MB JAR in WASM on a 2020 laptop; WASM ≤ 4 MB gzipped; board nightly ≤ 5 h.
- Site: Lighthouse ≥ 95 / 100 accessibility; no console errors; works without JavaScript for docs and board (SSR), converter needs JS and says so.
- Docs: every public CLI flag and every rule-table key documented; `docs/architecture.md` regenerated when crate boundaries change.
- Supply chain: pinned Action SHAs, Sigstore-signed releases, SBOM, OpenSSF Scorecard ≥ 7.

**Forbidden**

- Marking anything green without a harness result; editing `LEDGER.md` by hand.
- Committing JARs, decompiled sources, mapping files with restrictive terms, or any user upload.
- Copying code from Sinytra Connector, Architectury, Forgified Fabric API or any mod into `crates/` or `rules/`.
- Generating icons, logos, mascots, screenshots or testimonials.
- Silent fallbacks in the converter: when a rule cannot apply, the report says so; the output never pretends.
- `unwrap()` in library crates; panics in the WASM path.
- Stubbing a v1 feature with a fake (a converter that returns the input renamed, a board with sample rows). A missing feature shows as missing.
- Widening scope: any work on the "later tabs" before M6 is closed goes to an issue, not a branch.

## 13. Decisions you make yourself (defaults) and the decision log

Take the default unless evidence during the build says otherwise; either way, write a `D-<n>` entry: decision, reason, date, what it would take to reverse.

| Open question | Default |
| --- | --- |
| Product name | `Jarrow (decided; see the note under the title). First task of M0: claim jarrow on crates.io and npm with a placeholder 0.0.1, and the GitHub org jarrow or jarrow-dev` |
| Game versions pinned | The two most-downloaded 1.21 minors on Modrinth on day one |
| Decompiler | Vineflower (Apache-2.0) |
| Mapping chain | intermediary → Mojang official, Parchment for parameter names in Tier B only |
| Bytecode library | Own `classfile` crate (needed for WASM); no JVM in Tier A |
| Auth/DB in v1 | Cloudflare D1 + KV + GitHub OAuth in a Worker; Supabase only if connected, and then only for auth and opt-outs |
| Tier B compute | GitHub Actions; self-hosted runner if connected; never a paid service |
| Queue | D1 table + cron Worker; no Cloudflare Queues (paid tier) |
| Analytics | PostHog if connected, else Cloudflare Web Analytics |
| Email | Resend if connected, else none |
| Monitoring | Whichever of UptimeRobot / BetterStack is connected; both → BetterStack for the status page, UptimeRobot kept as a second alert path |
| Site framework | Astro + Starlight + Tailwind |
| License of the project | MIT; rule tables CC0 so anyone can reuse the knowledge |
| Mod source list | Modrinth API only in v1 |
| Agent model in Tier B | The same model running this build, with a per-job token cap of 400k and 6 iterations |
| Board refresh | Nightly 02:00 UTC |

`DECISIONS.md` format: `## D-012 — <title>` · Date · Decision · Why · Reversal cost · Links.

## 14. Appendix — the launch prompt

Paste this as the first message of a fresh Claude Code session in the project folder, with this book saved as `BUILD-BOOK.md` beside it. Replace `<ORG>` with the GitHub organisation you create (`jarrow`, or `jarrow-dev` if taken). Do not add instructions between sessions; put them in the book.

```markdown
You are the sole engineer of Jarrow: an open-source web service and CLI that converts a Minecraft Fabric mod JAR into a NeoForge JAR and proves the result with a behaviour harness. The complete specification is in BUILD-BOOK.md. Read it entirely before doing anything else. It is the only source of requirements; nothing in it is optional unless it says so.

Operating mode: fully autonomous, no checkpoints. I will not answer questions during the build. When a decision is open, take the default from section 13, record it in DECISIONS.md, and continue. Stop only for the three conditions in section 2 (a legal boundary, destroying something you did not create, a payment or terms acceptance); in that case write what you need in infra/TODO-OWNER.md, use the fallback, and keep working on everything else.

Start with section 2 now: enumerate every tool, credential and connector available to you; verify each with a harmless real call; write infra/INVENTORY.md with every service classified as Connected, Self-configurable or Needs the owner; create what you can create; then begin milestone M0, whose first task is claiming the name jarrow on crates.io, npm and GitHub.

Work through milestones M0 to M6 in order. Each milestone closes only when its exit criterion is green on main. Use branches and pull requests merged by CI; never push to main directly after M0. Every commit carries Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>.

The product is done when every item under "Done means all of the following are true at once" in section 1 is true, verified by the nightly board and CI, not by your judgement. Until then, do not report completion. When it is done, write docs/journal/DONE.md with the final numbers, the links (site, repo, releases, board), the open TODO-OWNER items, and the three things you would build next — and stop.

Constraints that override everything: no redistribution of converted mods; no copying from Connector, Architectury or any mod; no generated icons or imagery; nothing marked green without a harness result; the repository must be rebuildable from itself.

Repository: github.com/<ORG>/jarrow. Create it if it does not exist.
```

**How to resume** after a session ends: open a new session in the repository with `Continue Jarrow from BUILD-BOOK.md, AGENTS.md, DECISIONS.md and LEDGER.md. Re-run section 2 first. Resume at the first milestone whose exit criterion is not green on main.`
