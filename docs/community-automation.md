# Community automation

The bots and workflows that turn issues, discussions and pull requests from contributors into work and credit. Each section says what triggers it, what it posts, and what it may do. None of them checks out or runs code from a pull request.

| What | Trigger | What it does | Token permissions |
| --- | --- | --- | --- |
| `tools/bot_pr.sh` | Called by a workflow | Commits generated files as `github-actions[bot]` on a `bot/<prefix>-<date>` branch, pushes it and opens a pull request with the workflow token, turns on auto-merge, starts `ci.yml` and `harness.yml` on the branch | The caller's: `contents: write`, `pull-requests: write`, `actions: write` |
| `contributors.yml` | Nightly, 02:30 UTC; manual | Regenerates "Who works on Jarrow" (with every member's commit count) and the contributor numbers in `LEDGER.md`, through `bot_pr.sh` | `contents: write`, `pull-requests: write`, `actions: write`, `checks: read` |
| `deps-automerge.yml` | `pull_request_target` (opened, reopened, synchronize) from `dependabot[bot]` or `renovate[bot]` | Turns on auto-merge, so the pull request merges through the merge queue once `ci-ok` is green | `contents: write`, `pull-requests: write` |
| `welcome.yml` | A `ci` or `harness` run on a pull request finishes | One comment on a first-time contributor's pull request: the check results and the next rule to take | `pull-requests: write`, `issues: read`, `actions: read` |
| `proposal-to-issue.yml` | The label `accepted` is added to a discussion in "Rule proposals" | Opens an issue labelled `rule` from the discussion and links it back | `issues: write`, `discussions: write` |
| `red-mod-intake.yml` | An issue labelled `red-mod` is opened, edited or labelled | One comment with what Modrinth and the board say about the mod | `issues: write`, `contents: read` |

Every workflow has `permissions: contents: read` at the top and grants more per job only. Actions are pinned to full commit SHAs.

## Bot pull requests: `tools/bot_pr.sh`

Every change to `main` goes through a pull request, the bot's included. A workflow that regenerates files does not push to `main`; it calls:

```sh
bash tools/bot_pr.sh <branch-prefix> <title> <body-file> -- <path>...
```

1. If none of the paths changed, it prints "nothing changed" and exits 0. No branch, no pull request.
2. It commits the paths on a new branch `bot/<prefix>-<UTC date>` (with the time appended when that branch already exists). Author and committer are `github-actions[bot] <41898282+github-actions[bot]@users.noreply.github.com>`, GitHub Actions' own bot account, so the commit counts for GitHub Actions in GitHub's contributor list and in "Who works on Jarrow". The squash commit on `main` counts the same way, because a squash merge is authored by the pull request's author. The commit message is the title, the body file, and the trailers `Jarrow-Generated: <prefix>` and `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
3. It pushes the branch over HTTPS with `GH_TOKEN`, the job's `GITHUB_TOKEN`. The token reaches git only through a credential helper that the script sets in the environment for its own run, scoped to the GitHub server. It is never written to a file, a remote URL or a command line, and never printed. The checkout keeps `persist-credentials: false`.
4. It opens the pull request with the same token, then runs `gh pr merge --auto --squash --match-head-commit <sha>` with the same subject and body, so the squash commit on `main` keeps both trailers.
5. Events the workflow token causes start no workflow, so neither the push nor the new pull request starts CI. The script starts `ci.yml` and `harness.yml` on the branch with `gh workflow run <file> --ref <branch>`; `workflow_dispatch` is the documented exception that does start a run. That `ci` run reports `ci-ok` on the head commit, which the `main` ruleset requires, and auto-merge puts the pull request in the merge queue. The queue then runs its `merge_group` checks as for any pull request. Both workflows have a `workflow_dispatch` trigger for this, and `ci.yml` no longer runs on pushes to `bot/**`.
6. Older open pull requests from `bot/<prefix>-*` branches are closed with "Superseded by #N" and their branches deleted: they regenerated the same files from an older `main`.
7. It prints the pull request URL on its last line.

`tools/tests/test_bot_pr.py` runs the script against a local bare repository with a fake `gh`, and checks the credential helper with `git credential fill`.

What the job needs:

- `GH_TOKEN: ${{ github.token }}` with `contents: write` (push the `bot/` branch, turn on auto-merge), `pull-requests: write` (open and close pull requests) and `actions: write` (`gh workflow run`). The `main` ruleset accepts changes to `main` only through a pull request, so the token cannot push to `main` itself.
- The job behind `if: github.ref == 'refs/heads/main'`, so only `main`'s copy of the workflow opens bot pull requests.
- The repository setting **Settings > Actions > General > Workflow permissions > "Allow GitHub Actions to create and approve pull requests"**. It is on (checked 2026-11-03, `can_approve_pull_request_reviews: true`). While it is off, `gh pr create` is refused; the script then fails with a message that names the setting, and the pushed branch stays so a person can open the pull request by hand.
- "Allow auto-merge" in the repository settings (on).

The site picks up a new `site/src/data/team.json` with its next deploy.

Two workflows call it: `contributors.yml` (prefix `contributors`) and the `publish-ledger` job of `board.yml` (prefix `board`, for `ledger/inputs/board-summary.json` and `LEDGER.md`).

### The deploy key

`BOT_DEPLOY_KEY`, the `jarrow-bot` deploy key with write access, stays configured, but only for a push to a protected branch, which the workflow token cannot make. No workflow uses it today: `contributors.yml` and `board.yml` commit as `github-actions[bot]` through `bot_pr.sh`, and their `bot/` branches are not protected. A future branch that is not protected either, such as a `pr-assets` branch for pull request screenshots, uses the workflow token as well.

A workflow that does need the key reads it in one step of `main`'s copy only, writes it to a `0600` file under `RUNNER_TEMP` that an exit trap removes, and hands it to git only through `GIT_SSH_COMMAND`, with GitHub's SSH host keys from `gh api meta`. For a push to `main` itself the key would also need a bypass entry in the `main` ruleset. On 2026-11-03 the ruleset had no bypass actors.

## Dependency updates: Dependabot, Renovate and `deps-automerge.yml`

Dependency pull requests merge on their own when CI is green, through the merge queue like every other pull request.

- **Dependabot** (`.github/dependabot.yml`) keeps Cargo, npm (`site/`), GitHub Actions and pip (`tools/`), weekly on Monday. TypeScript 7 and later are ignored until `@astrojs/check` accepts them (D-025).
- **Renovate** (`renovate.json`) covers what Dependabot does not, so no update arrives twice: the devcontainer base image (tag and digest together), `rust-toolchain.toml` with the MSRV in `Cargo.toml` (one pull request), Gradle (`GRADLE_VERSION` in `harness.yml`) with the probe build's plugins and test libraries (`harness/probe`), and npm tools pinned inside workflow steps (`npx --yes wrangler@x.y.z`). Its cargo, npm, GitHub Actions and pip managers are off, and the ClusterFuzzLite builder image is left out. Fabric Loom stays below 1.18 (compiled for Java 25; the project pins JDK 21) and Gradle below 10, because `harness-ok`, which builds the probes, is not a required check. TypeScript stays below 7, as for Dependabot. Renovate runs on Monday mornings (UTC), with `automerge` and `platformAutomerge` on.
- **`deps-automerge.yml`** runs on `pull_request_target` (opened, reopened, synchronize) for pull requests that `dependabot[bot]` or `renovate[bot]` opened from a branch of this repository. It runs `gh pr merge <n> --auto` with the workflow token and nothing else: no checkout, no build, no code from the pull request. The pull request joins the merge queue once `ci-ok` passes; a red `ci-ok` leaves it open for a person.

Only `ci-ok` gates the merge. An update that only `harness.yml`, `devcontainer.yml` or the ClusterFuzzLite workflows would catch still merges when `ci-ok` is green, which is why the Renovate rules above hold back the versions known to break the probe build.

## Nightly contributors: `contributors.yml`

Runs `tools/contributors.py` (the README section between the `ALL-CONTRIBUTORS-LIST` markers, `.all-contributorsrc`, `docs/team.md`, `site/src/data/team.json`), then `tools/contributors.py --ledger` and `tools/ledger.py`, and hands the changed files to `bot_pr.sh contributors`.

Every person, agent and bot in the list shows its commit count on `main` next to its name, 0 included. An agent's count is the commits whose `Co-Authored-By` trailers name it. A bot's count is the commits it authored, by author name (`github-actions[bot]`, `dependabot[bot]`, `renovate[bot]`, ...); a squash merge is authored by the pull request's author, so each merged bot pull request counts once for its bot. The board's ledger commits (`Jarrow-Generated: board`) count for GitHub Actions.

The refresh commits carry `Jarrow-Generated: contributors`. `tools/contributors.py` leaves a refresh commit out while it is the newest commit on `main`, so the list it wrote stays up to date and a refresh never causes another one. Once another commit lands on top of it, it counts like any other commit, for GitHub Actions and for the agent its trailers name, so the numbers follow GitHub's own count with at most the latest refresh missing.

### Contributor numbers in `LEDGER.md`

`tools/contributors.py --ledger` writes `ledger/inputs/contributors.json`; `tools/ledger.py` renders it as the "Contributors" section, with "No data yet" when the file is absent.

- **Contributors (30 days):** people and agents counted separately, over the as-of day (UTC) and the 29 days before it. People are the authors of merged pull requests (from `gh pr list --state merged`) plus the human authors and co-authors of commits on `main` in the window, by GitHub login. Agents are the names in `Co-Authored-By` trailers of those commits. Bots are not counted, and neither are commits a bot authored.
- **First-time contributors (30 days):** people whose first merged pull request in the repository falls in the window. When the pull request lookup fails, this number is shown as not measured, never as zero.
- The file keeps its old `as_of` when nothing else would change, so the nightly opens a pull request only when a number or a login changes.

The same people, agents and bots are listed on the site at [/contributors](https://jarrow.pages.dev/contributors/) and on [/about](https://jarrow.pages.dev/about/), from `site/src/data/team.json`.

## First pull request comment: `welcome.yml`

Triggered by `workflow_run` (`completed`) of the workflows named `ci` and `harness`, for runs whose event was `pull_request`.

1. It finds the open pull request whose head is the run's commit: from the run's pull request list, else by head branch (`pulls?head=<owner>:<branch>`, which covers forks), else by commit.
2. It acts only for a first-time contributor: `author_association` is `FIRST_TIME_CONTRIBUTOR` or `FIRST_TIMER`, or the author has no merged pull request in the repository. Bot accounts are skipped.
3. It posts one comment that starts with the hidden marker `<!-- jarrow:welcome -->`. When another check finishes or the contributor pushes again, it edits that comment instead of posting a second one.

What the comment says, and nothing else:

- One line naming the author and the commit.
- For `ci` and `harness`: the newest run on that commit, its result, and each job with its result and a link to its log.
- The harness summary, from the first of these that exists: the file `summary.md` in the run's artifact named `harness-summary` (up to 256 KiB read, 6,000 characters quoted); a table of the `<game version>/baseline-status.json` files (game version, status, reason, detail) in the run's `baseline` artifact, which `harness.yml` uploads today ([probes.md](probes.md)); a job's check-run summary. Without any of them it says the run left no summary. Without a harness run at all it says "the harness did not run on this PR". While the harness is still running it says so.
- "Next rule you could take": the oldest open, unassigned issue labelled `good-first-rule`, falling back to `good first issue` and then `help wanted` (the oldest assigned one when all are taken), with its title and link. With none open: "no open good-first-rule issue right now".

Job names and the harness summary come from the pull request's own run, so a pull request can change them. The workflow treats them as text: `@` mentions are defused, HTML comment markers are neutralised, and the summary is capped.

GitHub offers no API for a job's step summary, so the comment cannot quote the harness job summary itself. A harness run that wants its own words in the comment uploads them as `summary.md` in an artifact named `harness-summary`.

## Rule proposals become issues: `proposal-to-issue.yml`

Triggered by `discussion` `labeled`, only when the label is `accepted` and the discussion is in the category "Rule proposals" (slug `rule-proposals`).

1. It reads the discussion through the GraphQL API.
2. It opens an issue with the discussion's title, the label `rule`, and a body that starts with the hidden marker `<!-- jarrow:proposal-discussion=<number> -->`, a line that links the discussion and names its author, and then the discussion's body.
3. It comments on the discussion: "Accepted. This proposal is now issue #N: <link>", with the marker `<!-- jarrow:proposal-issue -->`.

It is idempotent: when an issue labelled `rule` already carries the discussion's marker, no second issue is opened, and when the bot's linking comment is already on the discussion, no second comment is posted. A rerun after a partial failure completes only the missing step.

The `accepted` label has to exist before anyone can add it. It was created once, on 2026-11-03, with:

```sh
gh label create accepted --repo MosheByte/jarrow --color 0e8a16 --description "Rule proposal accepted: becomes an issue"
```

## Red mod intake: `red-mod-intake.yml`

Triggered by `issues` `opened`, `edited` and `labeled`, for issues that carry the label `red-mod` (the red mod report form adds it).

1. It reads the form's labels from `.github/ISSUE_TEMPLATE/red-mod-report.yml` on the default branch, through the API, and takes the value of the field with id `source-mod-url` or `mod-url` (both accepted while the field is renamed). Without that field it takes the first Modrinth link in the issue body.
2. If the value is not a Modrinth project URL (`https://modrinth.com/<mod|plugin|datapack|shader|resourcepack|modpack|project>/<slug or id>`), the comment says so and asks for the mod's page.
3. Otherwise it reads `https://api.modrinth.com/v2/project/<slug>` and `.../project/<id>/version` with the User-Agent `MosheByte/jarrow red-mod-intake (+https://github.com/MosheByte/jarrow)`, and `https://jarrow.pages.dev/api/board.json`.
4. It posts one comment with the marker `<!-- jarrow:red-mod-intake -->`, and edits it on later runs:
   - mod id and slug, title, license, client and server side, loaders, how many game versions are listed and which pinned ones (1.20.1, 1.21.1, 1.21.11), downloads;
   - whether a NeoForge build exists for a pinned version, from Modrinth's loader tags (not an author-verified port);
   - per pinned game version, the newest file (primary file, version number, release type, loaders, date) and whether a NeoForge build exists for that version;
   - the board's current row for the mod (result, tier, reason, game version, last run, report link) when the board has one, with a link to `https://jarrow.pages.dev/board/#mod-<Modrinth project id>`; otherwise that the mod is not on the board, or that the board has no rows yet, or that it could not be read.

The board row link expects the board page to give each row the id `mod-<Modrinth project id>`. Until it does, the link opens the board at the top.

### Pre-filled reports from the board and the site

GitHub issue forms take a field's value from a query parameter named after the field's id. The board and the site link to the form with the mod filled in:

```text
https://github.com/MosheByte/jarrow/issues/new?template=red-mod-report.yml&source-mod-url=<url-encoded Modrinth URL>&title=<url-encoded title>
```

For example, for Sodium:

```text
https://github.com/MosheByte/jarrow/issues/new?template=red-mod-report.yml&source-mod-url=https%3A%2F%2Fmodrinth.com%2Fmod%2Fsodium&title=%5Bred-mod%5D%20Sodium
```

The parameter name must be the id the form on `main` uses: `source-mod-url` once the form's field is renamed, `mod-url` until then (`...&mod-url=https%3A%2F%2Fmodrinth.com%2Fmod%2Fsodium&...`). The intake reads either.

## Labels the automation uses

| Label | Used by |
| --- | --- |
| `good-first-rule` | The next rule offered in the welcome comment; manifest and resource rules with no bytecode |
| `rule` | Issues made from accepted rule proposals; API call rewrites |
| `needs-proof` | Changes that need harness work before they can merge |
| `good first issue`, `help wanted` | GitHub's standard names, so GitHub's own discovery lists them; fallbacks in the welcome comment |
| `accepted` | Added by a maintainer to a rule proposal discussion |
| `red-mod` | Added by the red mod report form; starts the intake |

## Not built yet

These parts of the contributor pipeline depend on code that does not exist yet. Each lands with the milestone named, matching [docs/contributing/](contributing/README.md).

- **Red reasons become issues** (milestone M1, with the board). A nightly job opens one issue per distinct top red reason, titled by the blocking API symbol or manifest key, with the affected mods, the harness diff excerpt, the rule-table file and a rule stub, labelled `good-first-rule`, `rule` or `needs-proof`; it keeps at least 25 `good-first-rule` issues open and closes an issue when the board turns its reason green. Until then the labels are set by hand.
- **`jarrow rule new <from> <to> <symbol>` and `jarrow rule check`** (milestone M2, with the `rules` crate and the converter). The Codespace and devcontainer exist today.
- **The harness result as a comment on rule pull requests** (milestone M2). `harness.yml` already runs on every pull request, and the welcome comment quotes what it leaves for a first-time contributor.
- **Credit in rule file headers** (milestone M2, with the rule tables) **and in the nightly Discussion** ("3 new rules this week by @x, @y", ending with the open `good-first-rule` count; milestone M1, posted by `tools/board_summary.py` with the board).
- **Release notes that list every contributor by handle** (milestone M1, in the nightly prerelease notes of `nightly.yml`). The handles are in `ledger/inputs/contributors.json` and `site/src/data/team.json`; the notes do not read them yet.
