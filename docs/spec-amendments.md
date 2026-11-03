# Specification amendments

When the project specification (private) and reality disagree, reality wins. Each amendment below says what the specification asked for, what is true instead, what the project does about it, and what would undo it. The matching decision is logged in `DECISIONS.md`.

## A-1 — Repository lives under the owner's account

- **Spec:** one public repository under a GitHub organisation named `jarrow` (fallback `jarrow-dev`).
- **Reality (2026-11-03):** `jarrow` is already taken on GitHub by a user account, and github.com has no API for creating organisations.
- **What we do:** the repository is https://github.com/MosheByte/jarrow until the owner creates the organisation `jarrow-dev` and transfers the repository to it. GitHub redirects the old URLs after a transfer.
- **Reverts when:** the owner creates `jarrow-dev` and transfers the repository (listed in `infra/TODO-OWNER.md`). Links in this repository then move to the organisation.

## A-2 — Required checks and auto-merge instead of a merge queue

- **Spec:** a merge queue on `main`.
- **Reality:** merge queues are not available on repositories owned by a user account.
- **What we do:** the ruleset on `main` requires the CI checks and linear history, and pull requests merge through auto-merge (squash only) once the checks pass.
- **Reverts when:** the repository moves to an organisation (A-1); then the merge queue is turned on and becomes the required path.

## A-3 — Discussion categories were created in the web UI

- **Spec:** Discussions with the categories Announcements, Nightly results, Rule proposals, Q&A and Show your port.
- **Reality (2026-11-03):** GitHub has no API for creating categories. GraphQL `createDiscussionCategory` fails with `Field 'createDiscussionCategory' doesn't exist on type 'Mutation'`, REST `POST repos/{owner}/{repo}/discussions/categories` returns 404, and `gh discussion` has no category command.
- **What we do:** the agent created "Nightly results" (announcement format), "Rule proposals" and "Show your port" through the GitHub web UI in an authenticated maintainer browser session. The nightly posts to "Nightly results".
- **Reverts when:** nothing to revert.

## A-4 — Tier B uploads expire after 1 day, deleted explicitly after download

- **Spec:** a short-lived Tier B upload bucket with lifecycle delete at 1 hour.
- **Reality:** R2 lifecycle rules work in whole days, so the shortest automatic expiry is 1 day.
- **What we do:** the upload bucket expires objects after 1 day as a backstop, and every Tier B job deletes its object explicitly right after downloading it. The 1-hour target is met by the explicit delete; the lifecycle rule only catches jobs that failed before deleting.
- **Reverts when:** R2 supports expiry in hours.

## A-5 — The Pages Worker configuration lives in `site/`

- **Spec:** `wrangler.toml` lives in `infra/`.
- **Reality:** `wrangler pages` reads its configuration from the project directory, which is `site/`.
- **What we do:** the Pages project's configuration and bindings are in `site/wrangler.toml`. The configurations of other Workers (board ingest, queue) live in `infra/`.
- **Reverts when:** wrangler can point Pages at a configuration file elsewhere. There is little reason to move it.

## A-6 — No Minecraft server runs until the owner accepts the EULA

- **Spec:** the harness starts headless Fabric and NeoForge dedicated servers.
- **Reality:** running a Minecraft dedicated server requires accepting the Minecraft EULA. That is a terms acceptance, which only the owner can give (stop condition 3 in `AGENTS.md`).
- **What we do:** the harness refuses to start a server unless the owner's repository secret `MINECRAFT_EULA_ACCEPTED`, or the variable `MINECRAFT_EULA`, is `true`. The owner created that secret on 2026-11-03. Locally, a person who has accepted the EULA themselves sets the environment variable `MINECRAFT_EULA=true`. The harness writes `eula=true` into a server directory only when the gate is open. Until then, every harness run ends as "not run: EULA not accepted", so no mod can be green.
- **Reverts when:** the owner sets the variable (listed in `infra/TODO-OWNER.md`). The gate stays in the code either way.

## A-7 — The Pages project was created as a classic Pages project

- **Spec:** a Cloudflare Pages project serving the site at `*.pages.dev`.
- **Reality:** by default, wrangler now hands new Pages projects to Workers with static assets, which are served under the account's `workers.dev` subdomain instead of `*.pages.dev`.
- **What we do:** the project `jarrow` was created as a classic Pages project with `--force`, to keep the address `https://jarrow.pages.dev` that `SITE_URL` and every link rely on.
- **Reverts when:** the project migrates to Workers with static assets. That is a deliberate change with its own decision entry, not a side effect.

## A-8 — API routes are Astro server routes, not a separate `functions/` Worker

- **Spec:** the API routes (`/api/board`, `/api/queue`, `/api/auth/github`, `/api/badge/:mod.svg`) live in a `functions/` Worker.
- **Reality:** the board has to work without JavaScript, so it is server-rendered. Its pages and the API share the same bindings.
- **What we do:** routes and server-rendered pages are Astro routes, built by `@astrojs/cloudflare` into one Pages worker (D-016).
- **Reverts when:** a route needs its own Worker, for example a cron. Such Workers live in `infra/workers/`.

## A-9 — Owner items need a recorded failure and are grouped by milestone

- **Spec:** Process rule added 2026-11-03: "Needs the owner" is a proof of failure, not an estimate, and reporting never pauses work.
- **What we do:** an item is listed as needing the owner only after every route (REST, GraphQL, the vendor CLI and the vendor API) was tried with the credentials already held and failed with a recorded error. The exact command and error go with the item. The only owner-only item in M0 is the Minecraft EULA. Every other owner item goes under its milestone heading in `infra/TODO-OWNER.md` (D-019).
- **Reverts when:** never. This is a standing project rule.

## A-10 — Fabric `main` initializers run during `RegisterEvent`, not at common setup

- **Spec:** the entrypoint adapter runs `ModInitializer` at `FMLCommonSetupEvent`. The registration bridge captures the `Registry.register` calls made during init and replays them into `RegisterEvent`.
- **Reality:** NeoForge fires `RegisterEvent` and freezes registries before `FMLCommonSetupEvent`, so registrations made at common setup are too late.
- **What we do:** run `main` initializers inside the first `RegisterEvent` with registration redirected to the bridge (D-023). The registry dump in the harness verifies it.
- **Reverts when:** never; this is how the two lifecycles line up.

## A-11 — `site/src/styles/tokens.css` and the Pages assembly step

- **Spec:** the tokens file is named `site/tokens.css`, and the site uses Astro on Pages directly.
- **Reality:** the generated tokens file lives at `site/src/styles/tokens.css`, where Astro's CSS pipeline imports it. The Cloudflare adapter now emits Workers output, which `site/scripts/pages-assemble.mjs` rearranges for Pages (D-025).
- **Reverts when:** the site moves to Workers static assets.

## A-12 — The specification is private, and history was rewritten once

- **Spec:** the specification was first committed next to the code.
- **Reality (2026-11-03):** it contains private instructions, so it must not be public.
- **What we do:** it stays on the maintainer machine (untracked, in `.gitignore`), and `main`'s history was replaced once with a single orphan root commit (D-026). Old commit SHAs may remain reachable on GitHub by direct link until GitHub's caches expire; a GitHub Support request purges them.
- **Reverts when:** never.

## A-13 — v1 is any loader to any loader

- **Spec:** v1 converts Fabric mods to NeoForge, on two 1.21 minors (1.21.1 and 1.21.11).
- **Reality (2026-11-03):** the project owner widened v1 to any loader to any loader: Fabric, NeoForge, Forge and Quilt, all 12 directions, on Minecraft 1.20.1, 1.21.1 and 1.21.11.
- **What we do:** one engine, with one profile per loader in `loaders/<name>.toml`. Rule tables live in `rules/<from>-<to>/<game-version>/` and come in inverse pairs, and the harness scores each pair by an A → B → A round trip. Each direction gets its page in `docs/directions/` before its rule tables, and its own board column. v1 is done when every column is at least 50% green, and Fabric → NeoForge at least 60%. The directions are built in a fixed order that starts with Fabric → NeoForge, and none is advertised on the site before its column exists. `research-lock.json` pins 1.20.1 and the Forge and Quilt builds (D-029 to D-036). On 2026-11-03 no direction converts and none has a board column.
- **Reverts when:** never.
