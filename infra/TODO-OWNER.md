# Owner to-do

An item goes here only after the agent has tried REST, GraphQL, the vendor CLI and the vendor API with the credentials it holds, and can paste the exact command and error (BUILD-BOOK.md section 2, step 6). Items are grouped by the milestone that needs them. Each one has a working fallback, so the build never waits.

Run the commands from any shell where `gh` is logged in as MosheByte. `gh secret set` prompts for the value, so it never lands in shell history.

## M0 — Ground

### Accept the Minecraft EULA for the harness — done by the owner

The owner created the repository secret `MINECRAFT_EULA_ACCEPTED` on 2026-11-03 at 14:09 UTC. The harness opens its gate when that secret, or the variable `MINECRAFT_EULA`, is `true` (case-insensitive; `yes`, `1` and `accepted` also count). The agent cannot read secret values. If the first harness run reports "EULA not accepted", set the secret to `true`:

```bash
gh secret set MINECRAFT_EULA_ACCEPTED --repo MosheByte/jarrow --body true
```

### Cloudflare API token for CI (tried; every route refused)

Fallback in use: the agent deploys the site from the maintainer machine with wrangler. The nightly publishes `board.json` as a GitHub release asset, and a scheduled Worker ingests it (D-003).

Evidence, 2026-11-03:

1. REST with the wrangler OAuth token. `GET` and `POST` on `/user/tokens`, `/user/tokens/permission_groups`, `/accounts/e9b8acf2e89a1de289a1ee4abb0f3f8d/tokens` and `/accounts/e9b8acf2e89a1de289a1ee4abb0f3f8d/tokens/permission_groups` all returned:
   `HTTP 403 {"code": 9109, "message": "Unauthorized to access requested resource"}`
2. Vendor CLI. `npx wrangler login --scopes-list` offers no API-token scope (its scopes are account, user, zone, workers, pages, d1 and similar), and wrangler has no token-creation command.
3. Vendor API through the Cloudflare API MCP. `POST /accounts/e9b8acf2e89a1de289a1ee4abb0f3f8d/tokens` with Pages, Workers Scripts, KV, D1 and R2 write permission groups returned:
   `HTTP 403 (Ray ID a54bfd5aeb1cc746-IAD) {"code": 9109, "message": "Unauthorized to access requested resource"} — This connection was not granted a scope this endpoint accepts. Reconnect and grant one of: account-api-tokens.write (Account API Tokens Write).`
   `POST /user/tokens` returned:
   `HTTP 403 (Ray ID a54bfd5ce810c746-IAD) — No OAuth scope covers this endpoint, so OAuth connections cannot use it. Connect with an API token that has one of these permissions instead: API Tokens Write.`
4. Composio's Cloudflare connection. `GET /user/tokens/permission_groups` returned:
   `HTTP 400 {"code": 6003, "message": "Invalid request headers", "error_chain": [{"code": 6103, "message": "Invalid format for X-Auth-Key header"}]}`

Either of these unblocks it:

- **Dashboard, 1 minute (recommended, the value never leaves you):** dash.cloudflare.com → Manage Account → Account API Tokens → Create Token → Custom. Grant Cloudflare Pages Edit, Workers Scripts Edit, Workers KV Storage Edit, D1 Edit and Workers R2 Storage Edit on this account, then run:

  ```bash
  gh secret set CLOUDFLARE_API_TOKEN --repo MosheByte/jarrow
  ```

- **Reconnect the Cloudflare API MCP** and grant `account-api-tokens.write`. The agent then mints a scoped token through `POST /accounts/{id}/tokens`.

`CLOUDFLARE_ACCOUNT_ID` is already set as a repository variable. `deploy-site.yml` starts deploying from Actions on the first push after the secret exists.

## M4 — Deep

### GitHub OAuth app (sign-in and author mode)

To be attempted through the API in M4. If GitHub refuses, the error will be pasted here. The fields will be:

- Homepage: `https://jarrow.pages.dev`
- Callback: `https://jarrow.pages.dev/api/auth/github/callback`

## M5 — Public

These items are attempted through every route when M5 starts. They move here with the pasted error only if all routes fail:

- the crates.io and npm first publish (the names were free on 2026-11-03; the `claim-names.yml` workflow is ready)
- the org `jarrow-dev` and repository transfer (needed for the merge queue)
- Sponsors enrolment
- the Discord webhook
- Homebrew, Scoop, winget and AUR credentials
- the GitLab mirror token
- the Sentry source-map token
- Codecov
- Renovate

Owner-only by nature (payment or legal):

- a trademark check on "Jarrow" before launch
- buying a domain (`jarrow.dev`, `getjarrow.dev` or `jarrow.app`)
