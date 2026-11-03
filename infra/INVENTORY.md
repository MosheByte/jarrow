# Service inventory

Output of the environment discovery protocol (`AGENTS.md`, "Start of every session"). Re-run at the start of every session and update this file in the same PR as any change it causes.

Last run: 2026-11-03, session 1, by the build agent.

Every row was checked with one harmless real call on the date above. Secrets are listed by **name only**; values live in GitHub Actions secrets or Cloudflare bindings, never here.

Status columns:

- **Connected**: a read call succeeded end to end. Configured by the agent.
- **Self-configurable**: not connected, but the agent can create it with what it has.
- **Needs the owner**: a human step is required. The exact step is in [TODO-OWNER.md](TODO-OWNER.md) and the fallback is in use.

## Shell credentials and toolchains

| Service | Use in Jarrow | Evidence (2026-11-03) | Status | Notes |
| --- | --- | --- | --- | --- |
| GitHub (`gh` 2.102.0) | Repository, CI, releases, Discussions, Projects, Pages | `gh auth status`: logged in as `MosheByte`, classic token with `repo`, `workflow`, `admin:org`, `project`, `write:discussion`; `gh api user` succeeded | Connected | Repo [MosheByte/jarrow](https://github.com/MosheByte/jarrow) created and pushed. The org `jarrow` cannot be used (the name is a GitHub user). `POST admin/organizations` returned 404 (GHES only), so github.com offers no API to create an org. See D-001. |
| GitHub Pages | Rustdoc API reference | `POST repos/MosheByte/jarrow/pages build_type=workflow` returned `https://moshebyte.github.io/jarrow/` | Connected | Deployed by `pages.yml`. |
| Cloudflare (wrangler 4.157.0, OAuth) | Pages site, Workers, D1, KV, R2, Turnstile, Browser Rendering | `wrangler whoami`: one standard account, scopes include pages, workers, d1, workers_kv, browser, challenge-widgets. `d1 list`, `kv namespace list`, `r2 bucket list`, `pages project list` succeeded | Connected | Account id `e9b8acf2e89a1de289a1ee4abb0f3f8d` (repo variable `CLOUDFLARE_ACCOUNT_ID`). The account also hosts unrelated projects, so every Jarrow resource is prefixed `jarrow`. Jarrow never touches them. |
| Cloudflare API token for CI | Deploys, infra apply and board writes from GitHub Actions | The owner set the secret `CLOUDFLARE_API_TOKEN` on 2026-11-03 (a broad account token). Every workflow verifies it with `wrangler whoami` before use | Connected | Used by deploy-site.yml and infra.yml, and later by board.yml (D-021). |
| crates.io | Publish the `jarrow` crate | `~/.cargo/credentials.toml` absent. `GET crates.io/api/v1/crates/jarrow` returned "does not exist" (the name is free) | Needs the owner | Owner creates a token. A publish workflow (M5, not written yet) uses it (D-012). |
| npm | Publish the `jarrow` WASM package | `npm whoami` returned `ENEEDAUTH`. `GET registry.npmjs.org/jarrow` returned "Not found" (the name is free) | Needs the owner | Same flow as crates.io (D-012). |
| Rust 1.99.1, rustup 1.29.1 | Engine | `cargo --version`, `rustc --version` | Connected | Target `wasm32-unknown-unknown` comes from `rust-toolchain.toml`. |
| Node 24.20 / npm 11.19 | Site build | `node --version` | Connected | |
| Python 3.12 (`py -3.12`), uv 0.13 | `tools/` scripts and tests | `py -3.12 --version` | Connected | |
| JDK 21 (Temurin 21.0.12) | Harness, Tier B | `java -version` | Connected | CI uses `actions/setup-java`. |
| Docker 29.9 | Not required | CLI installed, daemon not running (`failed to connect to the docker API`) | Self-configurable | Not used. GitHub-hosted runners carry what the harness needs. |
| Self-hosted runner | Harness, Tier B | None registered | Needs the owner (optional) | Fallback: GitHub-hosted runners only, Tier B queue depth capped. |

## MCP connectors

| Service | Use in Jarrow | Evidence (2026-11-03) | Status | Notes |
| --- | --- | --- | --- | --- |
| Cloudflare Workers Bindings MCP (deprecated) and Cloudflare API MCP | Same account as wrangler | `workers_list`, `d1_databases_list`, `kv_namespaces_list`, `r2_buckets_list` succeeded. The API MCP `whoami` and `GET /accounts/{id}/pages/projects`, `.../challenges/widgets` and `.../browser-rendering/devtools/session` also succeeded | Connected | Secondary path. Wrangler is the primary tool. |
| Cloudflare Pages | Site at `https://jarrow.pages.dev` | Project `jarrow` created on 2026-11-03 (classic Pages, D-004) | Connected | Production branch `main`. Config in `site/wrangler.toml`. |
| Cloudflare D1 | Board rows, author flags, queue, error and event fallbacks | Database `jarrow` (`94b0f4c1-a071-4541-b2ec-f77abef9d445`) created | Connected | Migrations in `infra/d1/migrations/`. |
| Cloudflare KV | Cached `board.json`, rate limits | Namespace `jarrow-board` (`c04b064aa29d46468c8573ea7072cdf8`) created | Connected | Binding `BOARD`. |
| Cloudflare R2 | Reports, rule-table snapshots, mapping cache. Never JARs except the short-lived Tier B upload bucket | Bucket `jarrow-reports` created. R2 was already enabled on the account | Connected | `jarrow-uploads` (1-day lifecycle) is created in M4. |
| Cloudflare Turnstile | Tier B queue and author PR guard | `GET challenges/widgets` succeeded (the widgets listed belong to other projects) | Connected | Widget created in M4. The secret goes straight into a Pages secret. |
| Cloudflare Browser Rendering | OG images for board pages | `GET browser-rendering/devtools/session` succeeded | Connected | Used from M5. |
| `plugin:cloudflare` MCP | Not needed | Requires OAuth through `/mcp` | Needs the owner (optional) | The API MCP and wrangler already cover it. |
| Supabase MCP | Not used in v1 | `list_organizations`: 1 org. `list_projects`: 3 projects from other products, no Jarrow project | Connected | Not used: D1 is the v1 default and the org is at its free project limit, so a new project would be a payment (D-008). |
| Sentry MCP | Errors from the WASM bundle, Workers and Tier B | `find_organizations`: the connected Sentry organisation (us region). `find_projects`: 5 projects from other products | Connected | Projects `jarrow-site`, `jarrow-workers` and `jarrow-tierb` are created when their code exists (M3/M4, D-009). The source-map upload token is an owner item. |
| PostHog MCP | Product events and feature flags | `organizations-get` and `projects-get` succeeded: 1 org, 1 project that belongs to another product | Connected | A Jarrow project is created with the telemetry code in M3 (D-006). Its project key is public by design. |
| Resend MCP | Tier B completion emails | `list-domains`: none. `list-api-keys`: 2 keys from other products (names only) | Connected | No sending domain yet, so v1 sends no email (D-007). |
| Better Stack MCP (Uptime + Telemetry) | Status page, monitors, nightly heartbeat | `monitors`: 1 onboarding monitor (not Jarrow's). `status_pages`: none. `heartbeats`: none | Connected | Primary status page and monitors in M5 (D-005). |
| `plugin:betterstack` MCP | Not needed | Requires OAuth through `/mcp` | Needs the owner (optional) | The connected Better Stack MCP covers it. |
| UptimeRobot MCP | Second alert path | `get-user`: FREE plan, 0 of 50 monitors. `list-psps`: none. 1 email alert contact | Connected | Second monitors in M5 (D-005). |
| GitLab MCP | Repository mirror | `get_user`: active user. `list_projects`: 1 unrelated private project | Connected | Project and mirror job in M5. The mirror push needs a GitLab token secret (`GITLAB_MIRROR_TOKEN`), which is an owner item. `mirror.yml` is skipped until it is set. |
| `plugin:gitlab` MCP | Not needed | Requires OAuth through `/mcp` | Needs the owner (optional) | |
| Firecrawl | Research: Fabric and NeoForge docs, Modrinth pages | `firecrawl_search "NeoForge RegisterEvent documentation"` returned docs.neoforged.net pages | Connected | Sources are cited in `rules/SOURCES.md`. |
| Context7 | Research: library docs | `resolve-library-id NeoForge` returned `/neoforged/documentation` and 4 more | Connected | |
| Composio | Possible Discord route | Meta calls succeeded. Discord connection is user OAuth (read only). The Cloudflare connections fail with `6003 Invalid request headers` | Connected (Discord read only) | Cannot create webhooks: the bot toolkit has no connection. See Discord below. |
| Discord | Nightly, release and alert channels | No webhook URL. Composio's Discord connection can read guilds only | Needs the owner | Fallback: notifications go to GitHub Discussions and issues. Secret name: `DISCORD_WEBHOOK_URL`. |
| Claude Docs MCP | Not needed | `guide topic.index` succeeded | Connected | Not used. Project docs live in the repo. |
| Other connectors in the build environment | Not used by Jarrow | Present | — | Out of scope; Jarrow never uses them. |

## Other services in the project plan

| Service | Use in Jarrow | Evidence (2026-11-03) | Status | Notes |
| --- | --- | --- | --- | --- |
| Custom domain | Site URL | None owned | Needs the owner | Fallback: `https://jarrow.pages.dev`. Every link is built from `SITE_URL`. |
| GitHub OAuth app | Site sign-in, author mode | github.com has no API to register OAuth apps | Needs the owner | Exact fields in TODO-OWNER. Needed in M4. |
| GitHub Sponsors | Funding | Enrolment requires accepting terms and payout details | Needs the owner | `FUNDING.yml` is committed. The button works once the owner enrols. |
| Merge queue | Merging | Rulesets on user-owned repos do not support `merge_queue` | Needs the owner (org transfer) | Fallback: required check `ci-ok` plus auto-merge (D-010). |
| Discussion categories | Nightly results, rule proposals, ports | GraphQL `createDiscussionCategory` does not exist; REST returned 404. Created through the GitHub web UI in an authenticated maintainer browser session | Connected | "Nightly results", "Rule proposals" and "Show your port" exist (D-011). |
| Renovate | Dependency PRs | Needs the Renovate GitHub App installed | Needs the owner (optional) | Fallback: Dependabot for Cargo, npm, Actions and pip (D-014). |
| Codecov | Coverage reporting | No token | Needs the owner (optional) | Fallback: CI measures coverage with `cargo llvm-cov` and enforces the 80% floor itself. |
| OpenSSF Best Practices badge | Scorecard CII-Best-Practices, trust | Project 16297 created on 2026-11-03 through the maintainer's signed-in session; basics answered with repository evidence (25%) | Connected | https://www.bestpractices.dev/en/projects/16297. The remaining criteria are answered as milestones prove them. |
| Sigstore, SLSA provenance | Release signing | Keyless through GitHub OIDC | Self-configurable | Wired in `release.yml` (M5). |
| Homebrew tap, Scoop bucket | CLI distribution | Tap and bucket repos can be created under MosheByte | Self-configurable | Pushing from CI needs a token (`HOMEBREW_TAP_TOKEN`), which is an owner item. |
| winget, AUR | CLI distribution | winget needs PRs to microsoft/winget-pkgs with a token; AUR needs an account and SSH key | Needs the owner | M5. |
| Minecraft server EULA | Harness servers | The owner created the secret `MINECRAFT_EULA_ACCEPTED` on 2026-11-03 | Connected (owner accepted) | The harness gate reads that secret (D-013). |

## Secrets and variables (names only)

| Name | Kind | Where | Set by | Used in |
| --- | --- | --- | --- | --- |
| `CLOUDFLARE_ACCOUNT_ID` | Actions variable | MosheByte/jarrow | agent | deploy-site.yml, board ingest |
| `SITE_URL` | Actions variable | MosheByte/jarrow | agent | site build, board summary |
| `CLOUDFLARE_API_TOKEN` | Actions secret | MosheByte/jarrow | owner (set 2026-11-03) | deploy-site.yml, infra.yml, board.yml |
| `CARGO_REGISTRY_TOKEN` | Actions secret | MosheByte/jarrow | owner | release.yml (M5, not written yet) |
| `NPM_TOKEN` | Actions secret | MosheByte/jarrow | owner | release.yml (M5, not written yet) |
| `GITLAB_MIRROR_TOKEN` | Actions secret | MosheByte/jarrow | owner | mirror.yml |
| `GITLAB_MIRROR_PATH` | Actions variable | MosheByte/jarrow | agent (M5) | mirror.yml |
| `DISCORD_WEBHOOK_URL` | Actions secret | MosheByte/jarrow | owner | board.yml, release.yml |
| `MINECRAFT_EULA_ACCEPTED` | Actions secret | MosheByte/jarrow | owner (set 2026-11-03) | harness.yml, board.yml |
| `BOT_DEPLOY_KEY` | Actions secret (SSH deploy key, write) | MosheByte/jarrow | agent (generated 2026-11-03; rotate by replacing the deploy key) | Kept only for a push to a protected branch, which GITHUB_TOKEN cannot make; no workflow uses it. Bot commits (LEDGER.md, board data, the contributors list) are made as github-actions[bot] with GITHUB_TOKEN through tools/bot_pr.sh (docs/community-automation.md) |
| `HOMEBREW_TAP_TOKEN` | Actions secret | MosheByte/jarrow | owner | release.yml |
| `PROJECT_TOKEN` | Actions secret | MosheByte/jarrow | owner (optional) | roadmap project sync |
