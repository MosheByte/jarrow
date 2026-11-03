# Owner to-do

An item goes here only after every route (REST, GraphQL, the vendor CLI and the vendor API) was tried with the credentials already held and failed with a recorded error; the exact command and error are pasted with the item (D-019 in `DECISIONS.md`, amendment A-9 in `docs/spec-amendments.md`). Items are grouped by the milestone that needs them. Each one has a working fallback, so the build never waits.

Run the commands from any shell where `gh` is logged in as MosheByte. `gh secret set` prompts for the value, so it never lands in shell history.

## M0 — Ground

### Accept the Minecraft EULA for the harness — done by the owner

The owner created the repository secret `MINECRAFT_EULA_ACCEPTED` on 2026-11-03 at 14:09 UTC. The harness opens its gate when that secret, or the variable `MINECRAFT_EULA`, is `true` (case-insensitive; `yes`, `1` and `accepted` also count). The agent cannot read secret values. If the first harness run reports "EULA not accepted", set the secret to `true`:

```bash
gh secret set MINECRAFT_EULA_ACCEPTED --repo MosheByte/jarrow --body true
```

### Cloudflare API token for CI — done by the owner

The owner set `CLOUDFLARE_API_TOKEN` as an Actions secret on 2026-11-03 at 14:27 UTC. It is a broad account token on purpose. Cloudflare is Connected (D-021), and no Cloudflare item goes on this list again.

### Ask GitHub Support to purge the pre-rewrite commits (optional)

`main`'s history was replaced once (D-026), but GitHub still serves the old commits by direct SHA.
Evidence, 2026-11-03: after the force-push, `gh api repos/MosheByte/jarrow/commits/03b5d9a --jq .sha` still returned `03b5d9ad4e73710988781ecefffa657af821dfb0`.
No REST or GraphQL endpoint, and no `gh` command, deletes cached commits or garbage-collects a repository. Only GitHub Support can do it, and only for the account owner. Open a request at https://support.github.com/request → "Remove sensitive data". Name the repository `MosheByte/jarrow` and the old commits `884864c`, `e8d672f`, `ba958d2`, `9e28d6c` and `03b5d9a`.

## M4 — Deep

### GitHub OAuth app (sign-in and author mode)

To be attempted through the API in M4. If GitHub refuses, the error will be pasted here. The fields will be:

- Homepage: `https://jarrow.pages.dev`
- Callback: `https://jarrow.pages.dev/api/auth/github/callback`

## M5 — Public

### OpenSSF Best Practices badge — done

The owner signed in at bestpractices.dev, and the agent created project 16297 (https://www.bestpractices.dev/en/projects/16297) and answered the basics with repository evidence.

### crates.io and npm tokens for the first publish — done by the owner

The owner set the Actions secrets `CARGO_REGISTRY_TOKEN` (2026-11-03, 16:28 UTC) and `NPM_TOKEN` (16:31 UTC) on `jarrow-dev/jarrow`. `.github/workflows/publish.yml` uses them to publish `jarrow` and `jarrow-report` to crates.io and `jarrow` to npm (D-012, docs/releases.md). The agent cannot read secret values and never publishes from a machine.

If a publish fails with 401 or 403 (or `EOTP` on npm), the token is missing, expired or lacks a permission. Replace it; `gh secret set` prompts for the value:

- crates.io: a token from https://crates.io/settings/tokens with the scopes `publish-new` and `publish-update`, for all crates or for `jarrow` and `jarrow-report`. Then `gh secret set CARGO_REGISTRY_TOKEN --repo jarrow-dev/jarrow`.
- npm: a granular access token (npmjs.com, Access Tokens, Generate New Token) with read and write on all packages (`jarrow` does not exist until the first publish) and "Bypass two-factor authentication" enabled, since CI cannot answer a one-time password. Then `gh secret set NPM_TOKEN --repo jarrow-dev/jarrow`.

These items are attempted through every route when M5 starts. They move here with the pasted error only if all routes fail:

- the org `jarrow-dev` and repository transfer (needed for the merge queue)
- Sponsors enrolment
- the Discord webhook
- Homebrew, Scoop, winget and AUR credentials
- the GitLab mirror token
- the Sentry source-map token
- Renovate

Owner-only by nature (payment or legal):

- a trademark check on "Jarrow" before launch
- buying a domain (`jarrow.dev`, `getjarrow.dev` or `jarrow.app`)
