# Decisions

Every default the build agent takes is logged here. The project requires this log, and `tools/pr_ready.py` checks its format.

## D-001 — Repository lives under MosheByte until an org exists

**Date:** 2026-11-03
**Decision:** The repository is `MosheByte/jarrow`. The plan was an org named `jarrow`, with `jarrow-dev` as the fallback.
**Why:** `jarrow` is an existing GitHub user, so no org can take that name. `POST admin/organizations` returned 404 because it exists only on GitHub Enterprise Server, and github.com offers no API to create organisations. `jarrow-dev` is still free.
**Reversal cost:** Low. The owner creates `jarrow-dev` and runs the one-line transfer listed under M5 in infra/TODO-OWNER.md. GitHub redirects the old URLs.
**Links:** infra/INVENTORY.md, docs/spec-amendments.md A-1

## D-002 — Pinned game versions are 1.21.1 and 1.21.11

**Date:** 2026-11-03
**Decision:** v1 targets Minecraft 1.21.1 and 1.21.11.
**Why:** Modrinth publishes no per-version download statistic. `tools/research_versions.py` takes the top 200 Fabric mods by downloads and sums their version-file downloads, splitting each file evenly across the 1.21 minors it lists. 1.21.1 and 1.21.11 rank first and second. The method and the full ranking are in docs/research/game-versions.json.
**Reversal cost:** Medium. Rule tables and the harness are per version, so changing a pin means re-locking `research-lock.json` and re-running the board.
**Links:** docs/research/game-versions.json, tools/research_versions.py, research-lock.json

## D-003 — Cloudflare from the maintainer machine until a CI token exists; the board is ingested by pull

**Date:** 2026-11-03
**Decision:** Until the owner adds `CLOUDFLARE_API_TOKEN`, site deploys and D1 migrations run from the maintainer machine through wrangler's OAuth login. The nightly board publishes `board.json` and the reports as assets of a GitHub release named `board-latest`, and a scheduled Cloudflare Worker pulls them into KV, D1 and R2.
**Why:** The wrangler OAuth login works but cannot mint API tokens, and copying a personal OAuth token into CI would be unsafe. With pull-based ingest, the nightly needs no Cloudflare secret at all.
**Reversal cost:** Low. With the token set, `deploy-site.yml` deploys on push, and the board workflow can also write directly.
**Links:** infra/TODO-OWNER.md M0, docs/architecture.md

## D-004 — Classic Pages project to keep jarrow.pages.dev

**Date:** 2026-11-03
**Decision:** The `jarrow` project was created with `wrangler pages project create --force` as a classic Pages project.
**Why:** By default, wrangler now delegates new Pages projects to Workers static assets, which serve under the account's `workers.dev` subdomain. The project requires the site at `*.pages.dev`, and `jarrow.pages.dev` was free.
**Reversal cost:** Low. The built site is static output plus an adapter worker, which moves to Workers assets with a config change.
**Links:** docs/spec-amendments.md A-7

## D-005 — Monitoring: Better Stack for the status page, UptimeRobot as the second alert path

**Date:** 2026-11-03
**Decision:** Both monitors are connected. Better Stack hosts the status page, the monitors and the nightly heartbeat. UptimeRobot runs duplicate monitors as a second alert path.
**Why:** This is the project's default when both services are connected.
**Reversal cost:** Low. The monitors are defined as code in infra/monitoring/ (from M5).
**Links:** infra/INVENTORY.md

## D-006 — Analytics: PostHog

**Date:** 2026-11-03
**Decision:** Product events go to a dedicated PostHog project, created together with the telemetry code in M3. Cloudflare Web Analytics is not used.
**Why:** PostHog is connected, and using it when it is connected is the project's default. No telemetry exists before the converter does.
**Reversal cost:** Low. Events are defined in docs/telemetry.md and can be pointed at the D1 `events` fallback table.
**Links:** docs/telemetry.md

## D-007 — No email in v1 until a sending domain exists

**Date:** 2026-11-03
**Decision:** Resend is connected but has no domain. v1 sends no email, and Tier B results are announced on the page and through the GitHub PR or Gist the job creates.
**Why:** A sending domain needs a registered domain and DNS, which is an owner purchase. A payment is a stop condition (AGENTS.md), so the no-email fallback is in use.
**Reversal cost:** Low. Once the owner buys a domain, the agent adds it to Resend and wires the Worker.
**Links:** infra/TODO-OWNER.md M5

## D-008 — Supabase is not used

**Date:** 2026-11-03
**Decision:** Auth and storage use Cloudflare D1 and KV with GitHub OAuth in a Worker. Supabase is not used.
**Why:** D1 and KV are the project's v1 default for auth and storage. The connected Supabase org is already at its free-tier project count, so a new project would need a paid confirmation.
**Reversal cost:** Medium. Auth would move to Supabase, with RLS policies and a keep-alive workflow.
**Links:** infra/INVENTORY.md

## D-009 — Sentry projects are created with the code that reports to them

**Date:** 2026-11-03
**Decision:** Sentry projects `jarrow-site`, `jarrow-workers` and `jarrow-tierb` are created in M3 and M4, when the WASM bundle, the Workers and the Tier B job exist. Until then, errors go to Workers logs and the D1 `errors` table.
**Why:** An empty Sentry project monitors nothing, and its DSN would be dead configuration.
**Reversal cost:** None.
**Links:** infra/INVENTORY.md

## D-010 — Required check and auto-merge instead of a merge queue

**Date:** 2026-11-03
**Decision:** The `main` ruleset requires the `ci-ok` status check, linear history and pull requests with squash merges. PRs merge through GitHub auto-merge once CI is green.
**Why:** Merge queues are only available on organisation-owned repositories (see D-001).
**Reversal cost:** Low. After the org transfer, add a `merge_queue` rule to the ruleset.
**Links:** docs/spec-amendments.md A-2

## D-011 — Discussion categories created through the GitHub web UI

**Date:** 2026-11-03
**Decision:** The categories "Nightly results" (announcement format), "Rule proposals" and "Show your port" were created through the GitHub web UI in an authenticated maintainer browser session. `tools/board_summary.py` posts to "Nightly results".
**Why:** The API has no way to create categories. GraphQL `createDiscussionCategory` returned `Field 'createDiscussionCategory' doesn't exist on type 'Mutation'`, REST `POST repos/MosheByte/jarrow/discussions/categories` returned 404, and `gh discussion` has no category command.
**Reversal cost:** None.
**Links:** docs/spec-amendments.md A-3

## D-012 — Name claims on crates.io and npm ship real code, published by a workflow

**Date:** 2026-11-03
**Decision:** The 0.0.1 releases that claim `jarrow` contain working code from this repository, not an empty placeholder: the CLI (`--version`, `tokens`) and `jarrow-report` on crates.io, and the design tokens package on npm. A publish workflow, written in M5, publishes them once `CARGO_REGISTRY_TOKEN` and `NPM_TOKEN` exist. It does not exist yet.
**Why:** No registry token is present on this machine. Both registries discourage reserving names without content, so the claim ships something real and honest.
**Reversal cost:** None. Yanking is possible, deletion is not.
**Links:** infra/TODO-OWNER.md M5

## D-013 — The harness requires the owner's Minecraft EULA acceptance

**Date:** 2026-11-03
**Decision:** The harness writes `eula=true` and starts servers only when the environment variable `MINECRAFT_EULA_ACCEPTED` (fed from the owner's repository secret of the same name) or `MINECRAFT_EULA` is `true`, `yes`, `1` or `accepted`. Without it, every harness run ends as "not run: EULA not accepted", which is never green.
**Why:** A dedicated server requires accepting Mojang's EULA. That is a terms acceptance, one of the three stop conditions in AGENTS.md, so only the owner can give it.
**Reversal cost:** None.
**Links:** infra/TODO-OWNER.md M0, docs/spec-amendments.md A-6

## D-014 — Dependabot without Renovate

**Date:** 2026-11-03
**Decision:** Dependabot updates Cargo, npm, GitHub Actions and pip dependencies weekly. Renovate is not configured.
**Why:** Renovate needs its GitHub App installed, which is an owner action. Dependabot covers the same ecosystems.
**Reversal cost:** Low. Add `renovate.json` after the app is installed.
**Links:** .github/dependabot.yml

## D-015 — GitHub Pages hosts the rustdoc API reference

**Date:** 2026-11-03
**Decision:** GitHub Pages serves `cargo doc` output at https://moshebyte.github.io/jarrow/. The product site stays on Cloudflare Pages.
**Why:** By design the product site runs on Cloudflare, and GitHub Pages is one of the GitHub features the project configures. The API reference is a useful job for it that never competes with the site.
**Reversal cost:** None.
**Links:** .github/workflows/pages.yml

## D-016 — Astro server routes through the Cloudflare adapter

**Date:** 2026-11-03
**Decision:** API routes (`/api/board.json` and later `/api/queue`, `/api/auth/github` and `/badge/:mod.svg`) and the server-rendered board pages are Astro routes built with `@astrojs/cloudflare`, not a separate `functions/` directory.
**Why:** The board has to work without JavaScript, so it must be server-rendered. One adapter worker serves both the pages and the API with the same bindings.
**Reversal cost:** Low. Routes can move to Pages Functions or a standalone Worker.
**Links:** docs/architecture.md

## D-017 — Rust toolchain 1.99.1, edition 2024, MSRV equals the pin

**Date:** 2026-11-03
**Decision:** `rust-toolchain.toml` pins stable 1.99.1, and `rust-version` is 1.99.
**Why:** It is the current stable on the build machine. An MSRV below the pin would be a claim CI does not test.
**Reversal cost:** Low. Lower the MSRV once a CI job tests it.
**Links:** rust-toolchain.toml

## D-018 — Design tokens owned by crates/report

**Date:** 2026-11-03
**Decision:** The palette is dark first. The action accent is pink (`#ff5ca8`). Tiers use sky (A), violet (B) and slate (unsupported). Results use green, amber, red and grey, and those four never appear decoratively. `crates/report` generates `site/src/styles/tokens.css`, and a test fails when the two drift.
**Why:** The project requires one source for badge and site colours, saturated and legible, with result colours reserved.
**Reversal cost:** Low.
**Links:** crates/report

## D-019 — Owner items are grouped by milestone and need a recorded failure

**Date:** 2026-11-03
**Decision:** infra/TODO-OWNER.md lists items under the milestone that needs them. An item is listed as needing the owner only after every route (REST, GraphQL, the vendor CLI and the vendor API) failed with a recorded error, and it enters with the exact command and that error. Sponsors, the OAuth app, Discord, the registry tokens, the distribution tokens and the org transfer are attempted when their milestone starts.
**Why:** "Needs the owner" is a proof of failure, not an estimate. Grouping by milestone keeps reporting from pausing work: an item waits only on the milestone that needs it. Process rule added 2026-11-03.
**Reversal cost:** None.
**Links:** infra/TODO-OWNER.md, docs/spec-amendments.md A-9

## D-020 — Cloudflare CI token stays a proven owner item; the fallback deploys from the maintainer machine

**Date:** 2026-11-03
**Decision:** No `CLOUDFLARE_API_TOKEN` exists yet. Site deploys and D1 migrations run through wrangler on the maintainer machine, and `deploy-site.yml` skips with a notice until the secret is set.
**Why:** Token creation was refused on every route: wrangler OAuth REST (403, code 9109), the Cloudflare API MCP (403, it needs `account-api-tokens.write`) and Composio (400, code 6003). The agent does not move a personal OAuth refresh token into CI.
**Reversal cost:** Low. One `gh secret set`, after which Actions deploys on the next push.
**Links:** infra/TODO-OWNER.md M0

## D-021 — Cloudflare is fully connected through the owner's account token

**Date:** 2026-11-03
**Decision:** CI uses the owner's `CLOUDFLARE_API_TOKEN` secret for everything Cloudflare does for Jarrow. That covers Pages deploys (`deploy-site.yml`), D1 migrations and the rest of `infra/` (`infra.yml`), the Turnstile widget, Browser Rendering for OG images, R2 lifecycle rules, the nightly D1 export and cron Workers. The agent creates these itself as each milestone needs them. Every workflow that uses the token first runs `wrangler whoami` and prints only whether the account id matched, never the token or the account e-mail. The token reaches only Cloudflare's API, and GitHub masks it in logs. Site deploys run only from Actions; the maintainer machine no longer deploys.
**Why:** The secret is a deliberately broad account token, so Cloudflare counts as fully connected and the token is not narrowed. This supersedes the fallback in D-003 and D-020. The board can now be written to D1, KV and R2 directly from the nightly.
**Reversal cost:** Low. Rotating the token is one `gh secret set`.
**Links:** .github/workflows/deploy-site.yml, .github/workflows/infra.yml, infra/TODO-OWNER.md

## D-022 — Loader, API and toolchain pins in research-lock.json

**Date:** 2026-11-03
**Decision:** Fabric Loader 0.20.1 for both versions. Fabric API 0.116.17+1.21.1 and 0.141.6+1.21.11. NeoForge 21.1.265 and 21.11.48 (newest stable, no beta). Parchment 2024.11.17 and 2025.12.20. Vineflower 1.12.0. JDK 21. Every hash comes from the publisher (Mojang manifest, Fabric and NeoForge maven sidecars, Modrinth). `tools/research_lock.py --verify` re-checks 83 pins against live sources, and CI runs the offline `--check`.
**Why:** These are the newest stable releases for the pinned game versions on 2026-11-03, which matches the project's default of using the most-downloaded minors with their current stable loaders.
**Reversal cost:** Medium. Changing a pin means re-verifying rules and re-running the harness for that version.
**Links:** research-lock.json, tools/research_lock.py, docs/research/fabric-neoforge-notes.md

## D-023 — Fabric `main` initializers run inside NeoForge's registration window

**Date:** 2026-11-03
**Decision:** The generated `@Mod` class runs each Fabric `main` initializer during the first `RegisterEvent`, with `Registry.register` redirected to the registration bridge. Client and server initializers run at `FMLClientSetupEvent` and `FMLDedicatedServerSetupEvent`. Work that needs a later stage is reported, never guessed.
**Why:** The first entrypoint design ran `ModInitializer` at `FMLCommonSetupEvent`, but NeoForge freezes registries before common setup. The registration bridge needs the `register` calls captured during the original init and replayed into `RegisterEvent`. Both can only hold if init runs inside the registration window. The harness's registry dump decides whether this works (M2).
**Reversal cost:** Medium; confined to the entrypoint adapter.
**Links:** docs/spec-amendments.md A-10, docs/research/fabric-neoforge-notes.md

## D-024 — Board vocabulary and the score-to-outcome mapping

**Date:** 2026-11-03
**Decision:** Tiers serialise as `a`, `b`, `unsupported`, and outcomes as `green`, `yellow`, `red`, `not-run` (`crates/report` is the source). Harness score 1.0 (identical) and 0.75 (only ordering or noise differences) are green, 0.5 is yellow, and 0.25 and 0 are red. A mod that was not measured is never green. On the board it counts as red with a classified reason (`no_converter`, `harness_not_run`, `client_only_not_measured` is yellow), so `green + yellow + red = mods` always holds.
**Why:** Jarrow's design fixes the harness scores and requires every red to carry a reason, but left the colours open. Score 0.75 means the dumps differ only in order or noise, so the mod behaves the same.
**Reversal cost:** Medium after the first nightly, because it changes the headline number.
**Links:** docs/board.md, docs/harness.md, crates/report

## D-025 — Site: Astro 7, Starlight, Tailwind 4, Workers adapter output assembled for Pages

**Date:** 2026-11-03
**Decision:** The site uses Astro 7.4, Starlight 0.42, Tailwind 4.3 and `@astrojs/cloudflare` 14.4. Since v13 the adapter targets Workers only, so `scripts/pages-assemble.mjs` turns its output into a Pages advanced-mode directory (`dist/` plus `dist/_worker.js/` and `_routes.json`). `site/wrangler.toml` is the single source of bindings, and the adapter gets a config derived from it. Fonts come from Fontsource (Bricolage Grotesque, Atkinson Hyperlegible Next, JetBrains Mono) and icons from Lucide. CI runs `npm test`, `astro check` and the build on all three OS.
**Why:** The project's stack is Astro, Starlight and Tailwind on Cloudflare Pages (D-004 keeps `*.pages.dev`), and the current adapter no longer emits Pages output by itself.
**Reversal cost:** Low. Moving to Workers static assets removes the assembly step.
**Links:** site/scripts/pages-assemble.mjs, docs/spec-amendments.md A-11

## D-026 — History rewritten once to keep the build specification private

**Date:** 2026-11-03
**Decision:** The project specification and its start note are private, machine-local files. They are untracked and listed in `.gitignore`. Their earlier commits were removed by replacing the history of `main` with a single orphan root commit, which holds the current, neutral tree, and force-pushing it once. Public files were rewritten into neutral project language. The owner's name appears only in `LICENSE`, and no personal instructions, accounts or other projects are named. No further force-push to `main` is allowed; every later change goes through a pull request.
**Why:** The specification holds the owner's private instructions. On the day of the rewrite the repository had no forks, watchers or clones besides the maintainer's, so a one-time rewrite was the cleanest way to remove it.
**Reversal cost:** None to keep. GitHub may still serve the old commits by direct SHA until its caches expire. A full purge needs a GitHub Support request (noted in docs/spec-amendments.md A-12).
**Links:** .gitignore, docs/spec-amendments.md A-12

## D-027 — macOS CI runs on the Intel image

**Date:** 2026-11-03
**Decision:** The macOS leg of `ci.yml` uses `macos-15-intel` instead of `macos-latest` (arm64).
**Why:** On 2026-11-03, GitHub could not schedule `macos-latest` jobs for this repository. Run https://github.com/MosheByte/jarrow/actions/runs/41656296502 ended with "The job was not acquired by Runner of type hosted even after multiple attempts" and the notice "Due to capacity constraints, jobs targeting macOS arm64 runners may experience longer queue times". The rerun and the next run stayed queued for more than 20 minutes. Releases still build arm64 macOS binaries by cross-compiling (M5).
**Reversal cost:** Low: change the matrix label back once arm64 capacity is reliable.
**Links:** .github/workflows/ci.yml

## D-028 — "Who works on Jarrow" is generated, never edited by hand

**Date:** 2026-11-03
**Decision:** `tools/contributors.py` builds the list in three groups (people, agents, bots and automation) from `git log`, the GitHub API and `.github/`. It writes `.all-contributorsrc` (the all-contributors spec, with the custom contribution types `agent`, `bot` and `ci`), the README section between the all-contributors markers, `docs/team.md`, and `site/src/data/team.json`, which the site's `/about` page renders. `.github/workflows/contributors.yml` regenerates the list at 02:30 UTC and pushes any change as jarrow-bot with the `BOT_DEPLOY_KEY` deploy key. The all-contributors app and its bot are not installed.
**Why:** People are shown by GitHub login only, and the project owner is always listed. Agents have no account here, so they are counted from the Co-Authored-By trailers of the commits they wrote. A bot or app is listed only when something shows that it acts on the repository: a check run or a non-empty check suite, a pull request, a commit, the deploy key, or its config in `.github/`. Listing installed apps directly (`gh api repos/MosheByte/jarrow/installations`, then `user/installations`) needs a GitHub App token and fails with 404 and 403 for a user token, so check suites stand in. GitHub creates an empty check suite for every installed app on each push, so an empty suite does not count: on 2026-11-03 the Claude GitHub App was installed but had no check run, pull request or comment here, so it is not listed. Renovate is not enabled (D-014), release-please is not used, and the stale sweep is a GitHub Actions workflow. Refresh commits carry the trailer `Jarrow-Generated: contributors` and are not counted, so a refresh never causes another one.
**Reversal cost:** Low: delete the workflow, the script and the four generated files.
**Links:** tools/contributors.py, .github/workflows/contributors.yml, docs/team.md, .all-contributorsrc, site/src/pages/about.astro

## D-029 — v1 is any loader to any loader

**Date:** 2026-11-03
**Decision:** v1 converts mods between Fabric, NeoForge, Forge and Quilt, in all 12 ordered directions, on Minecraft 1.20.1, 1.21.1 and 1.21.11. It is done when every direction has a board column and each column is at least 50% green, with Fabric → NeoForge at least 60%. The build order is fixed: Fabric → NeoForge; NeoForge → Fabric; Fabric → Quilt and Quilt → Fabric; Forge → NeoForge and NeoForge → Forge; Fabric → Forge and Forge → Fabric; then Quilt → Forge, Forge → Quilt, Quilt → NeoForge and NeoForge → Quilt. No direction is offered or advertised on the site before its board column exists. One engine serves every direction: each loader is described once, in a profile `loaders/<name>.toml`; rule tables live in `rules/<from>-<to>/<game-version>/` and come in inverse pairs (A → B with B → A), and the harness scores each pair by an A → B → A round trip; each direction has a page `docs/directions/<from>-<to>.md`, written before its rule tables. Milestones M0 to M6 keep their meaning for the engine and the first direction, and the other directions follow in the order above. `research-lock.json` gains 1.20.1 and the Forge and Quilt pins for all three game versions (D-030 to D-036). This extends D-002. On this date no direction converts and none has a board column.
**Why:** The project owner widened v1 on 2026-11-03. With four loaders and twelve directions, loader facts belong in data that one engine reads, not in twelve converters, and inverse pairs let the harness check each pair by a round trip. 1.20.1 is in scope by this change, not by the download ranking behind D-002.
**Reversal cost:** Medium, and higher with every rule table. The board contract, the D1 schema, the site, the harness design and the lock all carry directions and the third game version. No rule table exists yet, so narrowing the scope again would lose no conversion work.
**Links:** docs/directions/README.md, loaders/README.md, docs/loaders.md, ROADMAP.md, research-lock.json, docs/spec-amendments.md A-13

## D-030 — Forge is pinned to its recommended builds

**Date:** 2026-11-03
**Decision:** Forge is pinned to the `recommended` promotion of each game version in `promotions_slim.json`: 47.4.28 (1.20.1), 52.1.20 (1.21.1) and 61.2.4 (1.21.11), with installer hashes from the maven.minecraftforge.net sidecars. A `latest` build is allowed only for a game version without a recommended one, and then needs a `promotion_note`. The `mods.toml` facts in the lock name the exact game version as the `minecraft` range (`[1.21.1]`, for example), and `loaderVersion` is `[<Forge major>,)`.
**Why:** Forge publishes no release tags and no stable flag; the recommended promotion is the build its maintainers mark for players. All three versions have one, so no fallback was needed. Newer `latest` builds (52.1.21 and 61.2.5) exist, and `tools/research_lock.py --verify` reports them as information. A conversion stays within one game version, so the exact range claims no more than what is pinned.
**Reversal cost:** Medium, as in D-022: a new pin means re-verifying rules and re-running the harness for that version.
**Links:** research-lock.json, tools/research_lock.py, docs/research/loaders.md

## D-031 — NeoForge on 1.20.1 is its own line

**Date:** 2026-11-03
**Decision:** NeoForge for 1.20.1 is pinned as the Forge 47.1 fork, `net.neoforged:forge` 1.20.1-47.1.106, and the lock and the profiles describe it apart from NeoForge 21.x: `META-INF/mods.toml`, loader dependency id `forge`, SRG names for fields and methods at runtime, Mixin 0.8.5 and EventBus 6.0.5. `tools/research_lock.py` derives the expected artifact, Maven version, installer name, manifest and runtime names from the game version.
**Why:** NeoForge's 1.20.1 builds keep Forge's packages, file names and dependency id, and their installer renames the server into SRG member names; NeoForge dropped SRG only from 1.20.2 on. Treating 47.1 as one more NeoForge version would carry the 1.21 facts into a loader they do not describe.
**Reversal cost:** Low: one branch in the lock checker and one table per profile.
**Links:** research-lock.json, tools/research_lock.py, docs/research/loaders.md, loaders/neoforge.toml

## D-032 — A stable Quilt Loader has no pre-release suffix

**Date:** 2026-11-03
**Decision:** Quilt Loader is pinned to 0.30.2 for all three game versions: the newest version on meta.quiltmc.org without a pre-release suffix. The lock records this rule next to the pin, and `tools/research_lock.py --check` refuses a Quilt Loader version with a suffix such as `-beta.5`.
**Why:** Quilt meta v3 has no stable flag. 0.31.0-beta.5 is newer but a beta, and the project pins stable releases, as it does for NeoForge (D-022).
**Reversal cost:** Low. If Quilt meta gains a stable flag, the rule reads it instead.
**Links:** research-lock.json, tools/research_lock.py, docs/research/loaders.md

## D-033 — Quilt's API library is pinned per game version

**Date:** 2026-11-03
**Decision:** On 1.20.1 the Quilt harness uses Quilted Fabric API 7.7.0+0.92.2-1.20.1, which bundles the Quilt Standard Libraries 6.3.0 and Fabric API 0.92.2, pinned to the Modrinth file and its Modrinth hashes. On 1.21.1 and 1.21.11 it uses the pinned Fabric API, and the lock says so (`kind = "fabric-api"`, `same_as = "fabric.api"`) with the reason.
**Why:** Modrinth lists no Quilted Fabric API or Quilt Standard Libraries release for 1.21.1 or 1.21.11, only 1.21 alphas, and Quilt tells modders to use the Fabric API equivalents. On 1.20.1 the Quilt maven serves a different file under the same coordinate, so the pin is the file that players install.
**Reversal cost:** Low. If a Quilt API release appears for a 1.21 pin, its entry changes to `kind = "qfapi"` with a D- entry.
**Links:** research-lock.json, docs/research/loaders.md, loaders/quilt.toml

## D-034 — Servers run on the Java of their game version

**Date:** 2026-11-03
**Decision:** `research-lock.json` records the Java major per game version in `jdk.by_game_version`: 17 for 1.20.1, 21 for 1.21.1 and 1.21.11, each equal to that version's Mojang `javaVersion.majorVersion`. The harness starts each version's servers on that Java. JDK 21 stays the build JDK.
**Why:** Mojang's version JSON asks for Java 17 for 1.20.1. Whether the 1.20.1 servers also start on JDK 21 is not measured, so it is not assumed; it is an open question for the harness.
**Reversal cost:** Low. If the harness shows that 1.20.1 runs on JDK 21, one JDK can serve all three versions.
**Links:** research-lock.json, docs/harness.md, docs/research/loaders.md

## D-035 — The 1.21 pins stay as they were when Mojang re-publishes

**Date:** 2026-11-03
**Decision:** Mojang re-published the 1.21.1 and 1.21.11 version JSON files again on 2026-11-03. Their pins were not changed. `tools/research_lock.py --verify` reports the new manifest hashes as drift warnings and still checks the pinned files, so `--verify --strict` exits 1 until the pins are refreshed on purpose.
**Why:** The pinned URLs are content-addressed and still serve the files whose hashes are recorded. Re-pinning in the same change as the scope widening would mix two changes; a re-pin gets its own D- entry once the new files are compared.
**Reversal cost:** Low: re-lock the two entries with a D- entry.
**Links:** research-lock.json, tools/research_lock.py

## D-036 — Lock layout for the third game version

**Date:** 2026-11-03
**Decision:** 1.20.1 is added after the two 1.21 pins in `research-lock.json`, with a `pinned_by` field naming D-029, and `decided_by` names both D-002 and D-029. Every game version lists all four loaders. A loader entry may be `null` only next to a non-empty `<loader>_note` that says why, as Parchment already could.
**Why:** The order keeps the existing entries and their diffs readable, and `pinned_by` shows that 1.20.1 comes from the scope change, not from the download ranking. A missing loader must be a stated fact, never a gap the checker overlooks.
**Reversal cost:** Low.
**Links:** research-lock.json, tools/research_lock.py

## D-037 — A planned leaf crate, crates/loaders, reads the profiles

**Date:** 2026-11-03
**Decision:** A new crate, `crates/loaders` (package `jarrow-loaders`), is planned for M1. It loads and validates `loaders/<name>.toml` against `docs/loaders.md` and has no Jarrow dependency. `manifest`, `rules`, `rewrite`, `classify`, `harness` and `board` read loader facts through it, and none of them names a loader in code.
**Why:** With one engine for 12 directions, loader facts need one reader. As a leaf crate it serves the harness and the board without pulling in the converter.
**Reversal cost:** Low until M1: no code exists yet.
**Links:** docs/architecture.md, docs/loaders.md, loaders/README.md

## D-038 — Loader profiles hold only researched facts

**Date:** 2026-11-03
**Decision:** Every value in a loader profile is copied from `research-lock.json` or a note in `docs/research/`, with its source. A fact that is not researched is the string `"unknown"`, with a comment naming the research still needed; no field is left out and none is guessed. Code that meets `"unknown"` stops with that reason and never borrows another loader's value. Fabric on 1.20.1 runs the same Fabric Loader 0.20.1, so it carries the loader's own facts (manifest, mod id, entrypoints, Mixin, access widener); what depends on Fabric API 0.92.12 stays `"unknown"` until it is researched.
**Why:** A profile is what the engine trusts. A guessed field would turn into a wrong conversion that looks deliberate.
**Reversal cost:** None.
**Links:** docs/loaders.md, loaders/README.md

## D-039 — Rules are written in Mojang names for every direction

**Date:** 2026-11-03
**Decision:** Every rule table, for every direction and game version, names Minecraft classes and members in Mojang names. The engine remaps the source into Mojang names, applies the rules, then remaps into the target's runtime names. The table header key `names` takes the value `"mojang"`; the earlier draft value `"official"` is dropped.
**Why:** The four loaders run on three namespaces (intermediary, Mojang, and SRG members on 1.20.1), so one shared namespace keeps a rule readable in both halves of an inverse pair. `"official"` is what intermediary files call the obfuscated names, so the old value was misleading.
**Reversal cost:** Low before M2, when the first table is written.
**Links:** docs/rules.md, docs/architecture.md, docs/loaders.md

## D-040 — An inverse table lands with its pair

**Date:** 2026-11-03
**Decision:** Rule tables live in `rules/<from>-<to>/<game-version>/<area>.toml`. Every A → B table lands in the same pull request as its B → A inverse, or with a "not invertible" mark and a reason, whatever the build order says about B → A. Both headers name each other, and each rule names its inverse rule or says why it has none.
**Why:** The round trip needs both halves. The build order decides when a direction gets its board column and source list; it does not hold back an inverse table.
**Reversal cost:** Low before M2.
**Links:** docs/rules.md, docs/directions/README.md

## D-041 — The harness judges a mod against its loader's baseline

**Date:** 2026-11-03
**Decision:** For each loader and game version, the harness first dumps a server with only the probe and the API library. A mod is judged on what it adds over its loader's baseline, and the probes are checked by comparing baselines on the entries vanilla defines. The A → B → A round trip runs the original and the round-tripped JAR on loader A servers and uses the same scores as any conversion. A round-trip score never replaces the A → B score.
**Why:** Each loader adds content of its own, so two loaders never dump the same thing, even with no mod. Comparing what the mod adds keeps the score about the mod.
**Reversal cost:** Medium after M1, because scores would change.
**Links:** docs/harness.md

## D-042 — One board row per mod per direction column

**Date:** 2026-11-03
**Decision:** `board.json` schema 2 lists all 12 directions in build order, each with `column`, `bar_percent`, its own totals and its top reasons. A row is one mod in one direction column, with one result per game version, and the same Modrinth project can have a row in several columns. The top-level `totals` is the sum over columns, and a page that shows one direction reads that direction's totals.
**Why:** Each direction is measured against its own bar, so its numbers must stand on their own. Summing over columns keeps `green + yellow + red = mods` true at every level.
**Reversal cost:** Medium after the first nightly: readers and stored history change with the schema.
**Links:** docs/board.md, site/src/lib/board.ts

## D-043 — The site refuses a board.json it does not understand

**Date:** 2026-11-03
**Decision:** `site/src/lib/board.ts` validates `board.json` strictly: schema 2, all 12 directions in build order with the expected ids, loaders and bars, totals that add up, and no rows for a direction without a column. A file that fails is reported as invalid and the site shows the board as unavailable, never a part of it. A missing KV key shows the empty board.
**Why:** The board is the project's proof. Showing numbers from a file that breaks the contract could show something as green that the harness never measured.
**Reversal cost:** Low.
**Links:** site/src/lib/board.ts, docs/board.md

## D-044 — D1 board tables are recreated with a direction

**Date:** 2026-11-03
**Decision:** Migration `infra/d1/migrations/0002_directions.sql` drops `board_runs` and `board_rows` and creates them again with a direction: one `board_rows` row per mod, direction and game version, keyed by run, direction, project and game version. Checks in the table keep any row from being green without a harness score of 1.0 or 0.75.
**Why:** No nightly has run, so both tables are empty and dropping them loses nothing. Altering them in place would keep a primary key without the direction.
**Reversal cost:** Low today. Once rows exist, the next schema change needs a real data migration.
**Links:** infra/d1/migrations/0002_directions.sql, docs/board.md

## D-045 — The four loaders are named in plain text

**Date:** 2026-11-03
**Decision:** The site shows no logo of Fabric, NeoForge, Forge or Quilt. All four are named in plain text.
**Why:** The project shows a loader logo only under that loader's published brand guidelines. None was found for Fabric, NeoForge or Forge. Quilt's branding files are CC0, but CC0 grants no trademark rights and is not a guideline for showing the logo next to another project. Simple Icons has none of the four.
**Reversal cost:** Low: add a logo to `site/BRAND-SOURCES.md` with its guideline, then to the site.
**Links:** site/BRAND-SOURCES.md, docs/legal.md
