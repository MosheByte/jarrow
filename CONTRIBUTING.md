# Contributing to Jarrow

Thanks for looking. Jarrow is pre-alpha (milestone M0): nothing converts yet. The most useful help right now is research and rule proposals, described below.

Read [AGENTS.md](AGENTS.md) for the map of the repository and [docs/architecture.md](docs/architecture.md) for how the parts fit. Both are kept current.

## Without installing anything

Open a [GitHub Codespace](https://codespaces.new/MosheByte/jarrow) on this repository: the devcontainer in `.devcontainer/` has every tool below preinstalled. Then pick an issue labelled `good-first-rule` or `good first issue` and follow [docs/contributing/](docs/contributing/README.md), which also says which parts of that path exist today and which arrive in M1 and M2.

## Build and test

You need Rust stable (the version is pinned in `rust-toolchain.toml`), Python 3.12, Node 24 and, from milestone M1 on, JDK 21 for the harness.

```sh
# Rust workspace
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

# Python tools
python -m pytest tools/tests          # on Windows: py -3.12 -m pytest tools/tests

# Site
cd site && npm ci && npm run build

# Everything CI checks before a merge
python tools/pr_ready.py              # on Windows: py -3.12 tools/pr_ready.py
```

CI runs the same checks on Linux, macOS and Windows. If they pass locally on one system and fail on another in CI, CI wins.

## Pull requests

1. Make a branch. Nobody pushes to `main`.
2. Open a pull request with the template filled in. Keep it small: one change, one reason.
3. CI runs. Pull requests that touch `crates/` or `rules/` also run the harness smoke set (from milestone M1).
4. When the required check (`ci-ok`) passes, auto-merge squashes the PR onto `main`. History stays linear: squash merges only, no merge commits, no force-pushes to `main`. (A merge queue replaces auto-merge once the repository moves to an organisation; see [docs/spec-amendments.md](docs/spec-amendments.md), A-2.)

A pull request that claims a mod or a rule "works" must link a harness result. Without one it gets the `needs-proof` label and does not merge. The harness decides green, not the author and not the reviewer.

## Commits

Write commit messages in the imperative, short first line, details below.

Commits made by the agent that builds Jarrow end with this trailer, without exception:

```
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

If you are a person contributing by hand, do not add that trailer to your own work. If an AI tool helped you, say which one in the pull request description. Honesty about who wrote what is a project rule.

Every commit made from 2026-11-03 18:30 UTC carries a Developer Certificate of Origin sign-off: a `Signed-off-by: Your Name <you@example.com>` trailer whose e-mail matches the commit's author, which says you agree to the [DCO 1.1](https://developercertificate.org/). `git commit -s` adds it, and `git rebase --signoff main` adds it to commits you already made. The `dco` check on every pull request runs `tools/dco_check.py`; bot commits and merge commits are exempt.

## Contributing a rule

Rule tables are TOML files under `rules/fabric-neoforge/<game-version>/`. The format is described in [docs/rules.md](docs/rules.md) (draft, implemented in M2).

1. Open a [rule proposal](https://github.com/MosheByte/jarrow/issues/new?template=rule-proposal.yml) first. Name the Fabric API symbol, its NeoForge counterpart, the semantic differences, and your public sources.
2. Add the rule to the table for each game version it applies to, and add every source to `rules/SOURCES.md`.
3. Add a golden test for the rule.
4. The harness measures the result in CI and in the nightly run.

Rule tables are dedicated to the public domain under CC0-1.0. By contributing a rule you agree to that.

### Clean room

This is the rule that protects the project. Break it and the contribution is reverted.

- Describe API shapes only: class names, method names, descriptors, event timing, cancellation, return values. Learn them from public documentation and from the open-source API repositories of Fabric and NeoForge.
- Never copy code from a mod. Not into `rules/`, not into `crates/`, not into an issue.
- Never copy code from Sinytra Connector, Architectury or Forgified Fabric API. You may read their public issue trackers to learn about known differences, and you must cite them when you do. You may not reproduce their code or port it.
- Never paste decompiled code anywhere in this project.
- Never commit a JAR, a decompiled source file, or a mapping file whose terms restrict redistribution (Mojang's mappings are fetched at runtime, never committed).

## Contributing code

Code contributions are licensed under MIT, the same as the project. The project's quality bars apply to everyone: `cargo fmt`, clippy with warnings denied, no `unsafe` (except where the WASM bindings make it unavoidable), no `unwrap()` in library crates, tests for what you add.

## Reporting

- A conversion that looks wrong: the [red mod report](https://github.com/MosheByte/jarrow/issues/new?template=red-mod-report.yml) form.
- A bug in Jarrow itself: the [bug report](https://github.com/MosheByte/jarrow/issues/new?template=bug.yml) form.
- A mod author who wants their mod left alone: the [author opt-out](https://github.com/MosheByte/jarrow/issues/new?template=author-opt-out.yml) form.
- A security problem: [SECURITY.md](SECURITY.md), never a public issue.
- A question: [Discussions Q&A](https://github.com/MosheByte/jarrow/discussions/categories/q-a).

Everyone follows the [Code of Conduct](CODE_OF_CONDUCT.md).
