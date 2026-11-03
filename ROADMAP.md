# Roadmap

Jarrow v1 converts Fabric mods to NeoForge for Minecraft 1.21.1 and 1.21.11 and proves each result with a behaviour harness. Each milestone closes only when its exit criterion is green on `main`, checked by CI or the nightly run, not by anyone's opinion.

Live tracking: https://github.com/users/MosheByte/projects/1 (issues labelled `roadmap` and `planned`).

Last updated: 2026-11-03.

## Milestones

| Milestone | Deliverable | Exit criterion | Status |
| --- | --- | --- | --- |
| M0 — Ground | Repository and settings, `AGENTS.md`, `research-lock.json`, `infra/INVENTORY.md`, `infra/TODO-OWNER.md`, CI skeleton, site skeleton on `jarrow.pages.dev` with a "coming soon" page that already shows the board counter (zero) | `ci.yml` green on Linux, macOS and Windows; site reachable; inventory complete with every service classified | In progress |
| M1 — Measure | Crates `jar`, `classfile`, `mappings`, `manifest`, `classify`, `report`; the parity harness; the board pipeline running against the 1,000-mod list with conversion stubbed (every mod "red: no converter yet") | The nightly posts its first Discussion; `LEDGER.md` is generated; class-file round-trip tests pass on the whole board set | Not started |
| M2 — Convert | Crates `remap`, `rules`, `rewrite`, `engine`; manifest, mappings, entrypoints, registries, resources, mixins and access wideners | At least 30% of the 1,000 mods green in the nightly; CLI nightly prerelease | Not started |
| M3 — Browser | Crate `wasm`, the converter page, the report view, the license gate, the telemetry opt-out | A green mod converts in the browser in under 60 s on a 2020 laptop; WASM at most 4 MB gzipped; Lighthouse targets met | Not started |
| M4 — Deep | Tier B pipeline, queue Worker, Turnstile, GitHub OAuth, author-mode pull requests, client smoke test in the harness | 10 real author-mode PRs opened on fixture repositories the project owns; Tier B lifts the board to at least 50% green | Not started |
| M5 — Public | Complete docs, board filters, badges, status page, error and product monitoring, Discord webhooks, release v0.1 with signed artefacts on every channel | Definition of done, items 3 to 6 | Not started |
| M6 — Done | Rule-table growth driven by the "top red reasons" queue until the board crosses the v1 bar; fixes from the adversarial review | Definition of done, items 1 (at least 60% green) and 2 | Not started |

## Definition of done for v1

All of these must be true at the same time. None is true today.

1. The nightly board shows at least 60% of the 1,000 mods green for Tier A, and every red row carries a classified reason.
2. A new user converts a green mod in the browser in under 60 seconds on a laptop, and the result loads in a real NeoForge client without errors.
3. `cargo test`, clippy, the Python tests, the harness suite and the site build pass on `main` on Linux, macOS and Windows.
4. Signed releases exist for the CLI (GitHub Releases, Homebrew, Scoop, winget, AUR) and for the npm WASM package, and `LEDGER.md` agrees with the board.
5. Every connected service is configured, monitored and documented; every service that is not connected has a working fallback and a note in `infra/INVENTORY.md`.
6. Docs cover install, first conversion, tiers, author mode, the license policy, the rule-table format and how to add a rule.

## Not in v1

These are out of scope for v1. They will not be built or stubbed before v1 ships.

- Other directions and platforms: NeoForge to Fabric, Forge, Quilt, Bukkit/Paper, Bedrock.
- Hosting, caching or redistributing converted JARs. Not in v1, and not ever.
- Accounts other than GitHub sign-in. No email and password.
- Payments.

## Later tabs (planned)

The site reserves a route for each of these. Each page says "planned" and links to its roadmap issue. Work on them waits until M6 is closed; ideas go to an issue labelled `planned`, not to a branch.

| Tab | What it would do | Status |
| --- | --- | --- |
| Version translate | Move a mod between game versions, for example 1.20 to 1.21 | Planned, after v1 |
| Compat check | Tell you whether a set of mods is likely to work together | Planned, after v1 |
| Dependency updater | Suggest and apply dependency version updates for a mod | Planned, after v1 |
| Profiler | Show what a mod costs at startup and per tick | Planned, after v1 |
