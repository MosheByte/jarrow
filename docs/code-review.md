# Code review

How a change gets into Jarrow: what every pull request must pass, what a reviewer checks, and how to ask for a human review. It applies to everyone, the AI build agent and the bots included. The coding rules a reviewer holds a change to are in [coding-standards.md](coding-standards.md); how to open a pull request is in [CONTRIBUTING.md](../CONTRIBUTING.md).

## How a change merges

Every change reaches `main` through a pull request. Nothing is pushed to `main`, not by the owner, the agent or a bot: the `main` ruleset requires a pull request, allows squash merges only, keeps history linear and has no bypass actors.

1. **The author opens a pull request** with the template filled in: what and why, the milestone, and the proof (tests added, commands run, and a harness result for anything that claims a mod or a rule works).
2. **CI runs.** The required check is `ci-ok` ([`ci.yml`](../.github/workflows/ci.yml)). It passes only when every job of the run succeeded or was skipped: `fmt`, clippy and tests on Linux, macOS and Windows, `cargo deny`, the Python tests on three systems, the site's tests, type check and build on three systems, the WASM size budget (from M3), and `pr-ready` (`tools/pr_ready.py`: the agent trailer, no JARs or decompiled source, `research-lock.json` changes come with a `D-` entry, the `DECISIONS.md` format, `LEDGER.md` generated, every crate in the architecture doc, no secrets).
3. **Other checks run beside it** and a red one is a reason not to merge, even where the ruleset does not require it yet: `harness-ok` ([`harness.yml`](../.github/workflows/harness.yml): the probe build and, for same-repository pull requests, the harness baseline), `dco` ([`dco.yml`](../.github/workflows/dco.yml): every commit signed off), CodeQL, ClusterFuzzLite (`cflite-pr`), `reproducible` (for Rust changes) and the site preview deploy.
4. **Review** (below).
5. **The merge queue** runs `ci-ok` again on the exact result of merging onto the newest `main`, and squashes the pull request when it passes. Auto-merge puts a pull request in the queue once its checks pass.

**The harness decides green, not the author and not the reviewer.** A pull request that says a mod or a rule works must link a harness result. Without one it gets the `needs-proof` label and does not merge.

## What a reviewer checks

A reviewer reads the whole diff and the pull request description, and checks:

- **It does one thing, and the description is true.** The title and body describe what the diff does; nothing claims more than the tests or the harness show. Missing features say "not built yet (milestone Mx)".
- **Correctness.** The change does what it says, including edge cases and error paths. Library code returns errors instead of panicking.
- **Tests.** New behaviour has tests; a bug fix has a regression test that fails without the fix. Major new functionality comes with tests in the automated suite, or it does not merge ([CONTRIBUTING.md](../CONTRIBUTING.md), "Tests").
- **Standards.** The code follows [coding-standards.md](coding-standards.md). Formatting and lint findings are CI's job, but a reviewer rejects `#[allow]`, `# noqa` or `eslint-disable` without a stated reason.
- **Security.** Input from outside the project (JARs, downloads, pull request contents, issue and discussion text, Modrinth data) is validated and bounded; nothing untrusted reaches a shell through `${{ }}` in a workflow; new workflow triggers, permissions and secrets follow least privilege; no secret is printed. A change that moves a trust boundary updates the [assurance case](security/assurance-case.md) in the same pull request.
- **Clean room and legal.** No JAR, decompiled code, restricted mapping file or code copied from a mod, Sinytra Connector, Architectury or Forgified Fabric API. Rule tables cite their sources in `rules/SOURCES.md`.
- **Records.** A default the change chooses has a `D-` entry; a change of plan has an amendment in `docs/spec-amendments.md`; `AGENTS.md` and `docs/architecture.md` follow a changed crate, command or boundary; generated files (`LEDGER.md`, the contributors list, `tokens.css`) were regenerated, not edited.
- **Dependencies.** A new dependency is needed, maintained, license-compatible (`deny.toml`) and pinned by the lockfile; a new action is pinned to a full commit SHA with a version comment.
- **Site changes** also: the page works without JavaScript where the site promises that, follows the design tokens (green, amber and red only for results), meets the accessibility basics (headings, labels, contrast, keyboard use), and `npm run build` passes its CSP check.

Acceptable means: every required check passes, the reviewer has no unresolved objection, and every point above holds or the pull request says why it does not apply.

## Who reviews today

Honestly: there is one human on the project, the owner, and most pull requests are written by the AI build agent through the owner's account. GitHub does not let an account approve its own pull request, and the ruleset requires no approving review, so today a pull request merges on its checks. The agent reviews its own diffs against the list above, and the checks above are the gate that does not depend on anyone's attention. Until a second human maintainer exists ([GOVERNANCE.md](../GOVERNANCE.md)), the project does not meet the OpenSSF gold criterion that most changes are reviewed by someone other than their author.

A pull request from a fork merges only when someone with write access, today only the owner's account, enables auto-merge or merges it; whoever does that reviews it against the list above first.

## Asking for a human review

- **Anyone** can ask: request a review from `@MosheByte` on the pull request (`gh pr edit <number> --add-reviewer MosheByte`), or say "needs human review" in the description. Auto-merge is then left off until the review is in.
- **Ask for one** when a change touches a trust boundary, a secret, a workflow trigger or permission, `SECURITY.md`, `GOVERNANCE.md`, the license or legal pages, or anything an author or a user would want a person to have read.
- **When a second maintainer exists**, the ruleset will require one approving review from a code owner other than the author, and [`CODEOWNERS`](../.github/CODEOWNERS) will name both maintainers. This file changes with it.
