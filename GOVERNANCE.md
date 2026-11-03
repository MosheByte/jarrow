# Governance

How Jarrow is run: who does what, how decisions are made, who holds which access, and what happens if someone disappears. This page describes the project as it is on 2026-11-03, not as it should be. Where the two differ, it says so.

Jarrow is a young project with one human: the owner. An AI agent does most of the engineering, and bots run the automation. The model below is "benevolent dictator with a public decision log": the owner decides, everything is decided in the open, and every default the agent takes is written down where anyone can read it and argue with it.

## Roles

| Role | Who today | Responsibilities | Can | Cannot |
| --- | --- | --- | --- | --- |
| **Owner** | [MosheByte](https://github.com/MosheByte) (the one human with admin rights) | Sets direction; accepts terms and makes payments for the project; holds every account, token and key; reviews and reverses decisions; handles vulnerability reports with the agent ([SECURITY.md](SECURITY.md)); appoints maintainers | Everything an organization owner can do on GitHub and an account owner on Cloudflare | Push to `main` directly: the ruleset has no bypass actors, so the owner's changes go through pull requests like everyone's |
| **Maintainer** | Nobody yet besides the owner | Reviews and merges pull requests; triages issues and discussions; labels rule proposals `accepted`; runs releases; answers security reports | Write or maintain access to the repository, granted by the owner | Change organization settings, secrets or Cloudflare, unless the owner also makes them an admin there |
| **AI build agent** | Claude (Anthropic), working for the owner | Writes the code, docs, tests and infrastructure; opens pull requests and lets CI merge them; keeps `DECISIONS.md`, `docs/spec-amendments.md`, `infra/INVENTORY.md` and `infra/TODO-OWNER.md` current; runs the environment discovery each session ([AGENTS.md](AGENTS.md)) | Act through the owner's GitHub account and tools the owner connected. Every commit it writes says so in a `Co-Authored-By: Claude ...` trailer | Accept terms, pay, create accounts, redistribute someone else's mod, or destroy something it did not create ([AGENTS.md](AGENTS.md), "When you may stop"). It writes what it needs into `infra/TODO-OWNER.md` instead. It is not a maintainer of its own: it has no account, and it stops if the owner's access goes |
| **Contributors** | Anyone | Propose changes by pull request; propose rules in Discussions; report bugs, red mods and opt-outs through the issue forms | Open issues, discussions and pull requests from a fork | Merge, or see a secret: fork pull requests run without secrets ([assurance case](docs/security/assurance-case.md)) |
| **Bots and automation** | GitHub Actions workflows, `github-actions[bot]`, `jarrow-bot` (an SSH deploy key), Dependabot, the Renovate app (installed; its onboarding PR is open), Socket Security, CodeQL, `github-merge-queue[bot]` | CI, the nightly board and CLI prerelease, the contributors list, PR labels, first-PR comments, red-mod intake, dependency updates, code scanning, merging queued pull requests | What their workflow's `permissions:` block grants, and `jarrow-bot` can push `bot/` branches | Push to `main`; any change they make goes through a pull request and `ci-ok` like everyone's |

The generated list of everyone who has worked on the repository is in [docs/team.md](docs/team.md).

## How decisions are made

1. **Small, reversible choices** (a library, a default, a naming question) are made by whoever does the work, usually the agent, and recorded in [DECISIONS.md](DECISIONS.md) as a numbered `D-<n>` entry with the date, the decision, why, the reversal cost and links. `tools/pr_ready.py` checks the format and numbering on every pull request. The agent does not stop to ask about these; it decides, logs and continues.
2. **When the plan meets reality** and the plan has to change, the change is a `D-` entry plus an amendment in [docs/spec-amendments.md](docs/spec-amendments.md).
3. **What only the owner can decide** (terms, payments, accounts, legal boundaries, anything destructive to what the agent did not create) waits in [infra/TODO-OWNER.md](infra/TODO-OWNER.md) with the exact steps, and the project uses a fallback meanwhile.
4. **The owner can reverse any decision.** A reversal is a new `D-` entry that names the one it reverses.
5. **Anyone can challenge a decision** by opening an issue or a discussion that names the `D-` entry. The owner decides; the outcome is a new `D-` entry or a comment explaining why not.
6. **Rule proposals** start in the Discussions category "Rule proposals". A maintainer labels an accepted proposal `accepted`, and a workflow turns it into an issue ([docs/community-automation.md](docs/community-automation.md)).
7. **Every change to the repository** is a pull request that passes `ci-ok` and merges through the merge queue ([docs/code-review.md](docs/code-review.md)). Nothing is pushed to `main`.

## Who has which access

Checked on 2026-11-03 with `gh api` and the Cloudflare inventory ([infra/INVENTORY.md](infra/INVENTORY.md)).

| Access | Who | Notes |
| --- | --- | --- |
| GitHub organization `jarrow-dev`, owner role | MosheByte | The only member and the only owner. The organization requires 2FA (`two_factor_requirement_enabled` is `true`, checked 2026-11-03), and the owner's second factor is TOTP or a passkey, not SMS |
| Repository `jarrow-dev/jarrow`, admin | MosheByte | No other collaborator |
| `main` ruleset | Nobody can bypass it | Pull request required, linear history, squash only, `ci-ok` required, merge queue |
| Actions secrets (`CLOUDFLARE_API_TOKEN`, `BOT_DEPLOY_KEY`, `MINECRAFT_EULA_ACCEPTED`, and the planned ones in the inventory) | Set by the owner (the deploy key by the agent, on the owner's account) | Values are never in the repository or a log. Names only in [infra/INVENTORY.md](infra/INVENTORY.md) |
| Deploy key `jarrow-bot` | GitHub Actions, through `BOT_DEPLOY_KEY` | Write access; used to push `bot/` branches |
| Cloudflare account (Pages project `jarrow`, D1, KV, R2) | MosheByte | The account also hosts unrelated projects; the CI token is broader than it should be ([#24](https://github.com/jarrow-dev/jarrow/issues/24)) |
| GitHub private vulnerability reports | Repository admins: MosheByte | Read by the owner and the agent |
| OpenSSF Best Practices entry [16297](https://www.bestpractices.dev/en/projects/16297) | MosheByte | |
| Package registries (crates.io, npm) | Nobody yet | Names not claimed yet; tokens are owner items |
| Domain | None | The site uses `jarrow.pages.dev`, which belongs to the Cloudflare Pages project |

## Access continuity and bus factor

The bus factor is 1. One person, the owner, holds every admin right: the GitHub organization, the repository, the Cloudflare account, every secret and the badge entry. The AI agent does not change that: it has no account of its own, works only through the owner's account and tools, and stops if the owner's access goes. The project never counts the agent, a bot or a second account of the owner as another person.

**Access continuity** (OpenSSF silver `access_continuity`) follows the route the criterion gives a project run by one person: keys in a lockbox and a will that grants the legal rights. The lockbox holds, by name, what someone needs to create and close issues, accept pull requests and publish a release within a week:

1. the owner role of the GitHub organization `jarrow-dev`, with its 2FA recovery codes;
2. the Cloudflare account that holds the Pages project `jarrow`, D1, KV and R2;
3. the crates.io and npm accounts that own the package names, once claimed;
4. the OpenSSF Best Practices entry and the GitLab mirror.

Secrets in GitHub Actions need no copy: an organization owner can replace every one of them. Everything else is rebuildable from the repository itself. The lockbox and the will are the owner's to set up ([infra/TODO-OWNER.md](infra/TODO-OWNER.md)). Until the owner confirms both exist, the badge entry marks `access_continuity` as not met.

**Bus factor** (silver SHOULD, gold MUST), gold **`contributors_unassociated`** and gold **`two_person_review`** stay not met until people from outside the project contribute and review. The way in for them is the `good-first-rule` issues and the path in [CONTRIBUTING.md](CONTRIBUTING.md). When a contributor has made significant contributions and the owner trusts them, they become a maintainer with organization and Cloudflare admin, and this section changes.

The public parts survive regardless: the code is MIT, the rule tables are CC0, and the repository is mirrored to GitLab ([.github/workflows/mirror.yml](.github/workflows/mirror.yml)). Anyone may fork and continue.

## Changing this document

Governance changes like any other change: a pull request, with a `D-` entry when it changes a role, a right or how decisions are made. The owner has the final say.
