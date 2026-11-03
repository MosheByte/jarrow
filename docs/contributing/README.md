# Contributing in ten minutes

The shortest path from nothing to a pull request. You need a GitHub account and a browser. You do not need Rust, Python, Node or Java on your machine.

## 1. Open a Codespace

[Open in GitHub Codespaces](https://codespaces.new/MosheByte/jarrow)

The Codespace opens the project's prebuilt image, `ghcr.io/moshebyte/jarrow-devcontainer`, which [`devcontainer.yml`](../../.github/workflows/devcontainer.yml) builds from [`.devcontainer/Dockerfile`](../../.devcontainer/Dockerfile). The image already has:

- Rust 1.99.1 with rustfmt, clippy and the `wasm32-unknown-unknown` target, plus cargo-fuzz
- Python 3.12 with the test tools from `tools/requirements-dev.txt`
- Node 24, JDK 21 and the GitHub CLI (`gh`)

On first start the container fetches the Rust crates, runs `npm ci` in `site/` and checks the Python packages. Port 4321 (the site's dev server) is forwarded.

Codespaces count against the free monthly allowance of the GitHub account that creates them. To work on your own machine instead, open the repository in VS Code with the Dev Containers extension and Docker, then choose "Reopen in Container". It pulls the same image (linux/amd64).

Inside the container:

| Task | Command |
| --- | --- |
| Python tests | `python -m pytest tools/tests` |
| Rust build and tests | `cargo test --workspace` |
| Site dev server | `cd site && npm run dev` |
| The checks CI runs before a merge | `python tools/pr_ready.py --all` |

## 2. Pick an issue

Issues sized for a first contribution carry one of two labels:

- [`good-first-rule`](https://github.com/MosheByte/jarrow/issues?q=is%3Aissue%20is%3Aopen%20label%3Agood-first-rule): one rule for a manifest key or a resource, with no bytecode involved.
- [`good first issue`](https://github.com/MosheByte/jarrow/issues?q=is%3Aissue%20is%3Aopen%20label%3A%22good%20first%20issue%22): GitHub's standard label, for small changes of any kind.

Larger rule work is labelled [`rule`](https://github.com/MosheByte/jarrow/issues?q=is%3Aissue%20is%3Aopen%20label%3Arule) (an API call rewrite) or [`needs-proof`](https://github.com/MosheByte/jarrow/issues?q=is%3Aissue%20is%3Aopen%20label%3Aneeds-proof) (needs harness work first). Comment on the issue you take, so two people do not write the same rule.

Until the board opens issues by itself (milestone M1), these labels are set by hand, and the lists can be short or empty.

## 3. Make the change

For a rule, follow [adding-a-rule.md](adding-a-rule.md). For anything else, [CONTRIBUTING.md](../../CONTRIBUTING.md) has the build, test, commit and pull request rules, including the clean-room rule.

When your pull request merges, the nightly contributors run lists you under People in the README's "Who works on Jarrow" section.

## What exists today

| Piece | Status |
| --- | --- |
| Codespace and devcontainer (`.devcontainer/`) | Exists |
| Labels `good-first-rule`, `good first issue`, `rule`, `needs-proof` | Exist; set by hand until the board opens issues |
| `tools/pr_ready.py`, the checks CI runs before a merge | Exists |
| Credit in "Who works on Jarrow" (README, `docs/team.md`) | Exists; refreshed nightly by `contributors.yml` |
| Rule-table format ([docs/rules.md](../rules.md)) | Draft; fixed when the `rules` crate lands (M2) |
| Rule tables under `rules/` | None yet. A table can be written today; nothing loads it until the `rules` crate (M2) |
| An issue for each top red reason on the board, closed when the reason turns green | Not built yet (milestone M1, with the board) |
| `jarrow rule new <from> <to> <symbol>` and `jarrow rule check` | Not built yet (milestone M2, with the `rules` crate and the converter) |
| The harness result posted as a comment on rule pull requests | Not built yet (milestone M2, with the `rules` crate and the converter) |
