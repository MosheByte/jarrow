# Decisions

Every default the build agent takes is logged here (BUILD-BOOK.md section 13). The format is checked by `tools/pr_ready.py`.

## D-001 — Repository lives under MosheByte until an org exists

**Date:** 2026-11-03
**Decision:** The repository is `MosheByte/jarrow`. The book asks for an org named `jarrow`, with `jarrow-dev` as the fallback.
**Why:** `jarrow` is an existing GitHub user, so no org can take that name. `POST admin/organizations` returned 404 because it exists only on GitHub Enterprise Server, and github.com offers no API to create organisations. `jarrow-dev` is still free.
**Reversal cost:** Low. The owner creates `jarrow-dev` and runs the one-line transfer in infra/TODO-OWNER.md item 4. GitHub redirects the old URLs.
**Links:** infra/INVENTORY.md, docs/book-amendments.md A-1

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
**Links:** infra/TODO-OWNER.md item 2, docs/architecture.md

## D-004 — Classic Pages project to keep jarrow.pages.dev

**Date:** 2026-11-03
**Decision:** The `jarrow` project was created with `wrangler pages project create --force` as a classic Pages project.
**Why:** By default, wrangler now delegates new Pages projects to Workers static assets, which serve under the account's `workers.dev` subdomain. The book asks for `*.pages.dev`, and `jarrow.pages.dev` was free.
**Reversal cost:** Low. The built site is static output plus an adapter worker, which moves to Workers assets with a config change.
**Links:** docs/book-amendments.md A-7

## D-005 — Monitoring: Better Stack for the status page, UptimeRobot as the second alert path

**Date:** 2026-11-03
**Decision:** Both monitors are connected. Better Stack hosts the status page, the monitors and the nightly heartbeat. UptimeRobot runs duplicate monitors as a second alert path.
**Why:** This is the section 13 default for the case where both are connected.
**Reversal cost:** Low. The monitors are defined as code in infra/monitoring/ (from M5).
**Links:** infra/INVENTORY.md

## D-006 — Analytics: PostHog

**Date:** 2026-11-03
**Decision:** Product events go to a dedicated PostHog project, created together with the telemetry code in M3. Cloudflare Web Analytics is not used.
**Why:** PostHog is connected (section 13 default). No telemetry exists before the converter does.
**Reversal cost:** Low. Events are defined in docs/telemetry.md and can be pointed at the D1 `events` fallback table.
**Links:** docs/telemetry.md

## D-007 — No email in v1 until a sending domain exists

**Date:** 2026-11-03
**Decision:** Resend is connected but has no domain. v1 sends no email, and Tier B results are announced on the page and through the GitHub PR or Gist the job creates.
**Why:** A sending domain needs a registered domain and DNS, which is an owner purchase (section 2 fallback).
**Reversal cost:** Low. Once the owner buys a domain, the agent adds it to Resend and wires the Worker.
**Links:** infra/TODO-OWNER.md item 11

## D-008 — Supabase is not used

**Date:** 2026-11-03
**Decision:** Auth and storage use Cloudflare D1 and KV with GitHub OAuth in a Worker. Supabase is not used.
**Why:** This is the v1 default in section 13. The connected Supabase org is already at its free-tier project count, so a new project would need a paid confirmation.
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
**Links:** docs/book-amendments.md A-2

## D-011 — Discussion categories created through the owner's signed-in browser

**Date:** 2026-11-03
**Decision:** The categories "Nightly results" (announcement format), "Rule proposals" and "Show your port" were created in the GitHub web UI, through the owner's signed-in Chrome session (Claude in Chrome). `tools/board_summary.py` posts to "Nightly results".
**Why:** The API has no way to create categories. GraphQL `createDiscussionCategory` returned `Field 'createDiscussionCategory' doesn't exist on type 'Mutation'`, REST `POST repos/MosheByte/jarrow/discussions/categories` returned 404, and `gh discussion` has no category command.
**Reversal cost:** None.
**Links:** docs/book-amendments.md A-3

## D-012 — Name claims on crates.io and npm ship real code, published by a workflow

**Date:** 2026-11-03
**Decision:** The 0.0.1 releases that claim `jarrow` contain working code from this repository, not an empty placeholder: the CLI (`--version`, `tokens`) and `jarrow-report` on crates.io, and the design tokens package on npm. `claim-names.yml` publishes them once the owner sets `CARGO_REGISTRY_TOKEN` and `NPM_TOKEN`.
**Why:** No registry token is present on this machine. Both registries discourage reserving names without content, so the claim ships something real and honest.
**Reversal cost:** None. Yanking is possible, deletion is not.
**Links:** infra/TODO-OWNER.md item 3

## D-013 — The harness requires the owner's Minecraft EULA acceptance

**Date:** 2026-11-03
**Decision:** The harness writes `eula=true` and starts servers only when the environment variable `MINECRAFT_EULA_ACCEPTED` (fed from the owner's repository secret of the same name) or `MINECRAFT_EULA` is `true`, `yes`, `1` or `accepted`. Without it, every harness run ends as "not run: EULA not accepted", which is never green.
**Why:** A dedicated server requires accepting Mojang's EULA. That is a terms acceptance, one of the three stop conditions, so only the owner can give it.
**Reversal cost:** None.
**Links:** infra/TODO-OWNER.md item 1, docs/book-amendments.md A-6

## D-014 — Dependabot without Renovate

**Date:** 2026-11-03
**Decision:** Dependabot updates Cargo, npm, GitHub Actions and pip dependencies weekly. Renovate is not configured.
**Why:** Renovate needs its GitHub App installed, which is an owner action. Dependabot covers the same ecosystems.
**Reversal cost:** Low. Add `renovate.json` after the app is installed.
**Links:** .github/dependabot.yml

## D-015 — GitHub Pages hosts the rustdoc API reference

**Date:** 2026-11-03
**Decision:** GitHub Pages serves `cargo doc` output at https://moshebyte.github.io/jarrow/. The product site stays on Cloudflare Pages.
**Why:** The book puts the site on Cloudflare and lists Pages among the GitHub features to configure. The API reference is a useful job for it that never competes with the site.
**Reversal cost:** None.
**Links:** .github/workflows/pages.yml

## D-016 — Astro server routes through the Cloudflare adapter

**Date:** 2026-11-03
**Decision:** API routes (`/api/board.json` and later `/api/queue`, `/api/auth/github` and `/badge/:mod.svg`) and the server-rendered board pages are Astro routes built with `@astrojs/cloudflare`, not a separate `functions/` directory.
**Why:** The board has to work without JavaScript (section 12), so it must be server-rendered. One adapter worker serves both the pages and the API with the same bindings.
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
**Why:** The book asks for one source for badge and site colours (section 8), saturated and legible, with result colours reserved.
**Reversal cost:** Low.
**Links:** crates/report

## D-019 — Owner items are grouped by milestone and need a pasted failure

**Date:** 2026-11-03
**Decision:** infra/TODO-OWNER.md lists items under the milestone that needs them. An item enters only with the exact command and error from REST, GraphQL, the vendor CLI and the vendor API. Sponsors, the OAuth app, Discord, the registry tokens, the distribution tokens and the org transfer are attempted when their milestone starts.
**Why:** This is the owner's correction of 2026-11-03, recorded as section 2 steps 6 and 7.
**Reversal cost:** None.
**Links:** infra/TODO-OWNER.md, docs/book-amendments.md A-9

## D-020 — Cloudflare CI token stays a proven owner item; the fallback deploys from the maintainer machine

**Date:** 2026-11-03
**Decision:** No `CLOUDFLARE_API_TOKEN` exists yet. Site deploys and D1 migrations run through wrangler on the maintainer machine, and `deploy-site.yml` skips with a notice until the secret is set.
**Why:** Token creation was refused on every route: wrangler OAuth REST (403, code 9109), the Cloudflare API MCP (403, it needs `account-api-tokens.write`) and Composio (400, code 6003). The agent does not move a personal OAuth refresh token into CI.
**Reversal cost:** Low. One `gh secret set`, after which Actions deploys on the next push.
**Links:** infra/TODO-OWNER.md M0

## D-021 — Cloudflare is fully connected through the owner's account token

**Date:** 2026-11-03
**Decision:** CI uses the owner's `CLOUDFLARE_API_TOKEN` secret for everything Cloudflare does in section 9. That covers Pages deploys (`deploy-site.yml`), D1 migrations and the rest of `infra/` (`infra.yml`), the Turnstile widget, Browser Rendering for OG images, R2 lifecycle rules, the nightly D1 export and cron Workers. The agent creates these itself as each milestone needs them. Every workflow that uses the token first runs `wrangler whoami` and prints only whether the account id matched, never the token or the account e-mail. The token reaches only Cloudflare's API, and GitHub masks it in logs. Site deploys run only from Actions; the maintainer machine no longer deploys.
**Why:** The owner added a broad account token on purpose and asked that Cloudflare be treated as fully connected, without narrowing the token. This supersedes the fallback in D-003 and D-020. The board can now be written to D1, KV and R2 directly from the nightly.
**Reversal cost:** Low. Rotating the token is one `gh secret set`.
**Links:** .github/workflows/deploy-site.yml, .github/workflows/infra.yml, infra/TODO-OWNER.md

## D-022 — Loader, API and toolchain pins in research-lock.json

**Date:** 2026-11-03
**Decision:** Fabric Loader 0.20.1 for both versions. Fabric API 0.116.17+1.21.1 and 0.141.6+1.21.11. NeoForge 21.1.265 and 21.11.48 (newest stable, no beta). Parchment 2024.11.17 and 2025.12.20. Vineflower 1.12.0. JDK 21. Every hash comes from the publisher (Mojang manifest, Fabric and NeoForge maven sidecars, Modrinth). `tools/research_lock.py --verify` re-checks 83 pins against live sources, and CI runs the offline `--check`.
**Why:** These are the newest stable releases for the pinned game versions on 2026-11-03, which matches the section 13 default of using the most-downloaded minors with their current stable loaders.
**Reversal cost:** Medium. Changing a pin means re-verifying rules and re-running the harness for that version.
**Links:** research-lock.json, tools/research_lock.py, docs/research/fabric-neoforge-notes.md

## D-023 — Fabric `main` initializers run inside NeoForge's registration window

**Date:** 2026-11-03
**Decision:** The generated `@Mod` class runs each Fabric `main` initializer during the first `RegisterEvent`, with `Registry.register` redirected to the registration bridge (section 5.4). Client and server initializers run at `FMLClientSetupEvent` and `FMLDedicatedServerSetupEvent`. Work that needs a later stage is reported, never guessed.
**Why:** Section 5.3 says to run `ModInitializer` at `FMLCommonSetupEvent`, but NeoForge freezes registries before common setup. Section 5.4 needs the `register` calls captured during the original init and replayed into `RegisterEvent`. Both sections can only hold if init runs inside the registration window. The harness's registry dump decides whether this works (M2).
**Reversal cost:** Medium; confined to the entrypoint adapter.
**Links:** docs/book-amendments.md A-10, docs/research/fabric-neoforge-notes.md

## D-024 — Board vocabulary and the score-to-outcome mapping

**Date:** 2026-11-03
**Decision:** Tiers serialise as `a`, `b`, `unsupported`, and outcomes as `green`, `yellow`, `red`, `not-run` (`crates/report` is the source). Harness score 1.0 (identical) and 0.75 (only ordering or noise differences) are green, 0.5 is yellow, and 0.25 and 0 are red. A mod that was not measured is never green. On the board it counts as red with a classified reason (`no_converter`, `harness_not_run`, `client_only_not_measured` is yellow), so `green + yellow + red = mods` always holds.
**Why:** The book defines scores (6.1) and requires every red to carry a reason (section 1), but leaves colours open. Score 0.75 means the dumps differ only in order or noise, so the mod behaves the same.
**Reversal cost:** Medium after the first nightly, because it changes the headline number.
**Links:** docs/board.md, docs/harness.md, crates/report

## D-025 — Site: Astro 7, Starlight, Tailwind 4, Workers adapter output assembled for Pages

**Date:** 2026-11-03
**Decision:** The site uses Astro 7.4, Starlight 0.42, Tailwind 4.3 and `@astrojs/cloudflare` 14.4. Since v13 the adapter targets Workers only, so `scripts/pages-assemble.mjs` turns its output into a Pages advanced-mode directory (`dist/` plus `dist/_worker.js/` and `_routes.json`). `site/wrangler.toml` is the single source of bindings, and the adapter gets a config derived from it. Fonts come from Fontsource (Bricolage Grotesque, Atkinson Hyperlegible Next, JetBrains Mono) and icons from Lucide. CI runs `npm test`, `astro check` and the build on all three OS.
**Why:** The book asks for Astro, Starlight and Tailwind on Cloudflare Pages (D-004 keeps `*.pages.dev`), and the current adapter no longer emits Pages output by itself.
**Reversal cost:** Low. Moving to Workers static assets removes the assembly step.
**Links:** site/scripts/pages-assemble.mjs, docs/book-amendments.md A-11
