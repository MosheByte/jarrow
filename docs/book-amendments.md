# Book amendments

When [BUILD-BOOK.md](../BUILD-BOOK.md) and reality disagree, reality wins (book section 0, rule 10). Each amendment below says what the book asks for, what is true instead, what the project does about it, and what would undo it. The matching decision is logged in `DECISIONS.md`. The book itself is edited only to record the owner's own corrections (A-9).

## A-1 — Repository lives under the owner's account

- **Book:** section 7 and the note under the title: one public repository under a GitHub organisation named `jarrow` (fallback `jarrow-dev`).
- **Reality (2026-11-03):** `jarrow` is already taken on GitHub by a user account, and github.com has no API for creating organisations.
- **What we do:** the repository is https://github.com/MosheByte/jarrow until the owner creates the organisation `jarrow-dev` and transfers the repository to it. GitHub redirects the old URLs after a transfer.
- **Reverts when:** the owner creates `jarrow-dev` and transfers the repository (listed in `infra/TODO-OWNER.md`). Links in this repository then move to the organisation.

## A-2 — Required checks and auto-merge instead of a merge queue

- **Book:** section 7: a merge queue on `main`.
- **Reality:** merge queues are not available on repositories owned by a user account.
- **What we do:** the ruleset on `main` requires the CI checks and linear history, and pull requests merge through auto-merge (squash only) once the checks pass.
- **Reverts when:** the repository moves to an organisation (A-1); then the merge queue is turned on and becomes the required path.

## A-3 — Discussion categories were created in the web UI

- **Book:** section 7 asks for Discussions with the categories Announcements, Nightly results, Rule proposals, Q&A and Show your port.
- **Reality (2026-11-03):** GitHub has no API for creating categories. GraphQL `createDiscussionCategory` fails with `Field 'createDiscussionCategory' doesn't exist on type 'Mutation'`, REST `POST repos/{owner}/{repo}/discussions/categories` returns 404, and `gh discussion` has no category command.
- **What we do:** the agent created "Nightly results" (announcement format), "Rule proposals" and "Show your port" through the web UI, in the owner's signed-in browser. The nightly posts to "Nightly results".
- **Reverts when:** nothing to revert.

## A-4 — Tier B uploads expire after 1 day, deleted explicitly after download

- **Book:** section 9: a short-lived Tier B upload bucket with lifecycle delete at 1 hour.
- **Reality:** R2 lifecycle rules work in whole days, so the shortest automatic expiry is 1 day.
- **What we do:** the upload bucket expires objects after 1 day as a backstop, and every Tier B job deletes its object explicitly right after downloading it. The 1-hour target is met by the explicit delete; the lifecycle rule only catches jobs that failed before deleting.
- **Reverts when:** R2 supports expiry in hours.

## A-5 — The Pages Worker configuration lives in `site/`

- **Book:** section 7 and section 9: `wrangler.toml` lives in `infra/`.
- **Reality:** `wrangler pages` reads its configuration from the project directory, which is `site/`.
- **What we do:** the Pages project's configuration and bindings are in `site/wrangler.toml`. The configurations of other Workers (board ingest, queue) live in `infra/`.
- **Reverts when:** wrangler can point Pages at a configuration file elsewhere. There is little reason to move it.

## A-6 — No Minecraft server runs until the owner accepts the EULA

- **Book:** section 6.1: the harness starts headless Fabric and NeoForge dedicated servers.
- **Reality:** running a Minecraft dedicated server requires accepting the Minecraft EULA. That is a terms acceptance, which only the owner can give (book section 2, stop condition 3).
- **What we do:** the harness refuses to start a server unless the owner's repository secret `MINECRAFT_EULA_ACCEPTED`, or the variable `MINECRAFT_EULA`, is `true`. The owner created that secret on 2026-11-03. Locally, a person who has accepted the EULA themselves sets the environment variable `MINECRAFT_EULA=true`. The harness writes `eula=true` into a server directory only when the gate is open. Until then, every harness run ends as "not run: EULA not accepted", so no mod can be green.
- **Reverts when:** the owner sets the variable (listed in `infra/TODO-OWNER.md`). The gate stays in the code either way.

## A-7 — The Pages project was created as a classic Pages project

- **Book:** section 9: a Cloudflare Pages project serving the site at `*.pages.dev`.
- **Reality:** by default, wrangler now hands new Pages projects to Workers with static assets, which are served under the account's `workers.dev` subdomain instead of `*.pages.dev`.
- **What we do:** the project `jarrow` was created as a classic Pages project with `--force`, to keep the address `https://jarrow.pages.dev` that `SITE_URL` and every link rely on.
- **Reverts when:** the project migrates to Workers with static assets. That is a deliberate change with its own decision entry, not a side effect.

## A-8 — API routes are Astro server routes, not a separate `functions/` Worker

- **Book:** section 4 puts the API routes (`/api/board`, `/api/queue`, `/api/auth/github`, `/api/badge/:mod.svg`) in a `functions/` Worker.
- **Reality:** the board has to work without JavaScript (section 12), so it is server-rendered. Its pages and the API share the same bindings.
- **What we do:** routes and server-rendered pages are Astro routes, built by `@astrojs/cloudflare` into one Pages worker (D-016).
- **Reverts when:** a route needs its own Worker, for example a cron. Such Workers live in `infra/workers/`.

## A-9 — Section 2 steps 6 and 7 (owner correction, 2026-11-03)

- **Book:** the owner's correction message of 2026-11-03 refers to section 2, steps 6 and 7, as "just added". The `BUILD-BOOK.md` committed at that time ended section 2 at step 5.
- **What we do:** the two steps were added to `BUILD-BOOK.md`, in the owner's words, so the next session reads them:
  - Step 6: "Needs the owner" is a proof of failure, not an estimate. An item goes there only after REST, GraphQL, the vendor CLI and the vendor API were tried with the credentials already held, and the exact command and error can be pasted.
  - Step 7: reporting never pauses work. The only owner-only item in M0 is the Minecraft EULA. Every other owner item goes under its milestone heading in `infra/TODO-OWNER.md`.
- **Reverts when:** never. This is the owner's rule.

## A-10 — Fabric `main` initializers run during `RegisterEvent`, not at common setup

- **Book:** section 5.3 runs `ModInitializer` at `FMLCommonSetupEvent`. Section 5.4 captures the `Registry.register` calls made during init and replays them into `RegisterEvent`.
- **Reality:** NeoForge fires `RegisterEvent` and freezes registries before `FMLCommonSetupEvent`, so registrations made at common setup are too late.
- **What we do:** run `main` initializers inside the first `RegisterEvent` with registration redirected to the bridge (D-023). The registry dump in the harness verifies it.
- **Reverts when:** never; this is how the two lifecycles line up.

## A-11 — `site/src/styles/tokens.css` and the Pages assembly step

- **Book:** section 8 names the tokens file `site/tokens.css`, and section 4 uses Astro on Pages directly.
- **Reality:** the generated tokens file lives at `site/src/styles/tokens.css`, where Astro's CSS pipeline imports it. The Cloudflare adapter now emits Workers output, which `site/scripts/pages-assemble.mjs` rearranges for Pages (D-025).
- **Reverts when:** the site moves to Workers static assets.
