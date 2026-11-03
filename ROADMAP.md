# Roadmap

The goal for Jarrow v1 is any loader to any loader: convert a mod between Fabric, NeoForge, Forge and Quilt, every pair in both directions (12 directions), on Minecraft 1.20.1, 1.21.1 and 1.21.11, and prove each result with a behaviour harness. This scope is decision D-029 in [DECISIONS.md](DECISIONS.md). Nothing converts yet.

Each milestone closes only when its exit criterion is green on `main`, checked by CI or the nightly run, not by anyone's opinion.

Live tracking: https://github.com/users/MosheByte/projects/1 (issues labelled `roadmap` and `planned`).

Last updated: 2026-11-03.

## Milestones

Milestones M0 to M6 build the one engine that every direction shares, and the first direction, Fabric → NeoForge. The other 11 directions follow in the fixed order under [Directions](#directions).

| Milestone | Deliverable | Exit criterion | Status |
| --- | --- | --- | --- |
| M0 — Ground | Repository and settings, `AGENTS.md`, `research-lock.json`, `infra/INVENTORY.md`, `infra/TODO-OWNER.md`, CI skeleton, site skeleton on `jarrow.pages.dev` with a "coming soon" page that already shows the board counter (zero) | `ci.yml` green on Linux, macOS and Windows; site reachable; inventory complete with every service classified | In progress |
| M1 — Measure | Crates `jar`, `classfile`, `mappings`, `manifest`, `classify`, `report`; the parity harness; the board pipeline running against the 1,000-mod list with conversion stubbed (every mod "red: no converter yet") | The nightly posts its first Discussion; `LEDGER.md` is generated; class-file round-trip tests pass on the whole board set | Not started |
| M2 — Convert | Crates `remap`, `rules`, `rewrite`, `engine`; manifest, mappings, entrypoints, registries, resources, mixins and access wideners | At least 30% of the 1,000 mods green in the nightly; CLI nightly prerelease | Not started |
| M3 — Browser | Crate `wasm`, the converter page, the report view, the license gate, the telemetry opt-out | A green mod converts in the browser in under 60 s on a 2020 laptop; WASM at most 4 MB gzipped; Lighthouse targets met | Not started |
| M4 — Deep | Tier B pipeline, queue Worker, Turnstile, GitHub OAuth, author-mode pull requests, client smoke test in the harness | 10 real author-mode PRs opened on fixture repositories the project owns; Tier B lifts the board to at least 50% green | Not started |
| M5 — Public | Complete docs, board filters, badges, status page, error and product monitoring, Discord webhooks, release v0.1 with signed artefacts on every channel | Definition of done, items 3 to 6 | Not started |
| M6 — Done | Rule-table growth driven by the "top red reasons" queue until the Fabric → NeoForge column crosses its bar; fixes from the adversarial review | Definition of done, items 1 (at least 60% green) and 2 | Not started |

The board, the 1,000-mod list and the percentages in M1 to M6 are those of the Fabric → NeoForge column.

## Directions

How every direction is built:

- **One engine** for all 12 directions, with one profile per loader (`loaders/<name>.toml`).
- **Rule tables in inverse pairs**, A → B together with B → A. Each pair is tested by a round trip, A → B → A, scored by the harness against the original mod.
- **A page before the tables.** Each direction gets `docs/directions/<from>-<to>.md`, written before its rule tables. [docs/directions/](docs/directions/) holds the status of all 12.
- **A board column per direction.** A direction's bar is the share of its column the harness marks green. No direction is advertised on the site before its column exists.

The build order is fixed:

| Order | Directions | Bar for each column | Status |
| --- | --- | --- | --- |
| 1 | Fabric → NeoForge | At least 60% green | In progress: milestones M0 to M6 above |
| 2 | NeoForge → Fabric | At least 50% green | Not started |
| 3 | Fabric → Quilt, Quilt → Fabric | At least 50% green | Not started |
| 4 | Forge → NeoForge, NeoForge → Forge | At least 50% green | Not started |
| 5 | Fabric → Forge, Forge → Fabric | At least 50% green | Not started |
| 6 | Quilt → Forge, Forge → Quilt, Quilt → NeoForge, NeoForge → Quilt | At least 50% green | Not started |

No direction has a board column yet, and none is offered on the site.

## Definition of done for v1

All of these must be true at the same time. None is true today.

1. The nightly board's Fabric → NeoForge column shows at least 60% of the 1,000 mods green for Tier A, and every red row carries a classified reason.
2. A new user converts a green mod in the browser in under 60 seconds on a laptop, and the result loads in a real NeoForge client without errors.
3. `cargo test`, clippy, the Python tests, the harness suite and the site build pass on `main` on Linux, macOS and Windows.
4. Signed releases exist for the CLI (GitHub Releases, Homebrew, Scoop, winget, AUR) and for the npm WASM package, and `LEDGER.md` agrees with the board.
5. Every connected service is configured, monitored and documented; every service that is not connected has a working fallback and a note in `infra/INVENTORY.md`.
6. Docs cover install, first conversion, tiers, author mode, the license policy, the rule-table format and how to add a rule.
7. All 12 directions have a board column, and each column is at least 50% green (Fabric → NeoForge at least 60%, item 1).

## Not in v1

These are out of scope for v1. They will not be built or stubbed before v1 ships.

- Other platforms: Bukkit/Paper, Bedrock.
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
