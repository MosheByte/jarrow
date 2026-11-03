# Releases

From milestone M1 on, the `jarrow` CLI ships as a nightly prerelease built from `main`. Pre-alpha versions `0.1.0-alpha.N` hold the name `jarrow` on crates.io and npm. Signed, versioned releases start with v0.1 in milestone M5. This page covers these channels and how to check what you downloaded.

## Channels

| Channel | What you get | Where | Status |
| --- | --- | --- | --- |
| Nightly | A pre-alpha build of the CLI from `main`, once a day, for Linux, Windows and macOS | GitHub prereleases tagged `nightly-YYYYMMDD`: [all releases](https://github.com/jarrow-dev/jarrow/releases) | Running from M1 (`.github/workflows/nightly.yml`) |
| crates.io, pre-alpha | The CLI (`jarrow`) and the report model (`jarrow-report`) as source crates, version `0.1.0-alpha.1`. The CLI has `--version` and `tokens` only | [crates.io/crates/jarrow](https://crates.io/crates/jarrow), [crates.io/crates/jarrow-report](https://crates.io/crates/jarrow-report) | Not published yet: the first run of `.github/workflows/publish.yml` stopped on an owner step (see [First run](#first-run); D-012) |
| npm, pre-alpha | The package `jarrow`, version `0.1.0-alpha.1`, under the dist-tag `alpha`: the design tokens (`tokens.css`, `tokens.json`), with npm provenance. No JavaScript and no engine yet; the WASM engine arrives in M3 | [npmjs.com/package/jarrow](https://www.npmjs.com/package/jarrow) | Not published yet: the first run of `.github/workflows/publish.yml` stopped on an owner step (see [First run](#first-run); D-012) |
| v0.1 and later | Versioned releases built with cargo-dist, signed on every channel | GitHub Releases, Homebrew, Scoop, winget, AUR, crates.io, and npm for the WASM package | Planned (M5) |

A nightly is not a release. It can break from one day to the next, and it can only do what its own help text says. Each nightly's notes include the output of `jarrow --help` and of every subcommand's `--help` for that exact commit, so the notes always match the binary. Anything the help lists as not built yet is not in that build.

## Nightly

### What a nightly contains

Each nightly is a GitHub prerelease tagged `nightly-YYYYMMDD` and titled `Nightly YYYY-MM-DD (<short sha>)`. It is never marked "Latest". Its assets:

| Asset | Target | Built on | Notes |
| --- | --- | --- | --- |
| `jarrow-x86_64-unknown-linux-gnu.tar.gz` | Linux x86_64 | `ubuntu-latest` | Needs glibc of at least the version printed in the release notes (the newest `GLIBC_` symbol version the binary links against) |
| `jarrow-x86_64-pc-windows-msvc.zip` | Windows x86_64 | `windows-latest` | The C runtime is linked statically (`-C target-feature=+crt-static`), so no Visual C++ redistributable is needed |
| `jarrow-x86_64-apple-darwin.tar.gz` | macOS on Intel | `macos-15-intel` | |
| `jarrow-aarch64-apple-darwin.tar.gz` | macOS on Apple silicon | `macos-15-intel`, cross-compiled | arm64 runners could not be scheduled for this repository (D-027), so the binary is built with `rustup target add aarch64-apple-darwin` on the Intel image. CI checks its architecture with `lipo` but cannot run it |
| `SHA256SUMS` | | | SHA-256 of every archive, in `sha256sum` format |

Every archive holds one directory, `jarrow-<target>/`, with the binary (`jarrow`, or `jarrow.exe` on Windows), `LICENSE` and `README.md`. The archives are written with fixed timestamps (the commit time), fixed owners and modes, and sorted entries, so packaging the same binary twice gives the same bytes.

Each binary is built with `cargo build --release -p jarrow --locked --target <target>` from a clean checkout, with no build cache, and with the toolchain pinned in `rust-toolchain.toml`. Before packaging, each native binary runs with `--version` and `--help`. That smoke run is all the nightly checks. The tests run in `ci.yml` on every pull request and every push to `main`.

### What the release notes say

Each nightly's notes start with what the build is and the one check it had, then two lists written by `tools/release_notes.py` in the `changes` job:

- **Changes since `<previous nightly tag>`:** every pull request merged into `main` after the previous nightly's commit, up to this build's commit, with its number, title and author's handle, then any commit in that range that no pull request holds (`main` took direct pushes during M0). The previous nightly is the newest `nightly-YYYYMMDD` tag older than this run's own tag, so a same-day rerun still compares with the nightly before it. With no older nightly, the list starts after the repository's first commit, and the heading says "Changes since the first commit".
- **Contributors:** every handle that authored or co-authored those changes: the pull requests' authors, the commits' authors from `git log` (bots included, such as `dependabot[bot]`), and every `Co-Authored-By` trailer. People are listed by GitHub handle, bots by account name and agents by name (Claude Opus 5.5). A person GitHub has no account for is listed by the name in the commit; no e-mail address is printed.

After them come the `--help` output, the downloads and how to verify a download.

The pull requests come from `gh pr list --state merged --base main --search "merged:>=<day before the previous nightly's commit>"`, kept only when their merge commit is in the range. A commit in the range that none of them accounts for is looked up with `gh api repos/<repo>/commits/<sha>/pulls` before it is listed on its own, and handles behind commit addresses that are not GitHub no-reply addresses come from GitHub's record of the commit's authors (GraphQL). If git or any of those lookups fails, the `changes` job fails and nothing is published, so a nightly never ships a partial list; run it again. Dry runs write both lists too (for a pull request, up to its head commit) and show them in the `changes` job's summary.

### Verify a download

Check the archive against `SHA256SUMS` from the same release:

```sh
sha256sum --check --ignore-missing SHA256SUMS           # Linux
shasum -a 256 jarrow-aarch64-apple-darwin.tar.gz        # macOS: compare with the line in SHA256SUMS
Get-FileHash -Algorithm SHA256 jarrow-x86_64-pc-windows-msvc.zip   # Windows PowerShell
```

Check where it came from. Every archive and `SHA256SUMS` carries a Sigstore-signed SLSA build provenance attestation, made by `actions/attest-build-provenance` in the run that built it. With the GitHub CLI:

```sh
gh attestation verify jarrow-x86_64-unknown-linux-gnu.tar.gz --repo jarrow-dev/jarrow \
  --signer-workflow jarrow-dev/jarrow/.github/workflows/nightly.yml --source-ref refs/heads/main
```

That passes only for a file whose SHA-256 was attested by `nightly.yml` running on `main` of this repository, on a GitHub-hosted runner. The output names the commit and the workflow run. The publish job runs the same check on every asset before it publishes anything.

The repository moved from `MosheByte/jarrow` to `jarrow-dev/jarrow` on 2026-11-03, and an attestation names the repository its run belonged to. `nightly-20261103` was built before the move and its attestations name `MosheByte/jarrow`, so verify its assets with `--repo MosheByte/jarrow --signer-workflow MosheByte/jarrow/.github/workflows/nightly.yml`. Every later nightly takes the command above.

The binaries carry no Apple or Windows code signature and are not notarized, so macOS Gatekeeper and Windows SmartScreen may warn before the first run. Code signing comes with v0.1.

### When it runs

- **Every day at 05:30 UTC**, after the board run at 02:00 UTC.
- **By hand:** `gh workflow run nightly.yml --repo jarrow-dev/jarrow --ref main`.
- **The day** in the tag and title is the UTC date the run was created. Re-running an old run replaces that run's own release; it never creates a release for another day.
- **Same day, again:** a second run on the same day replaces that day's release. The `nightly-YYYYMMDD` tag moves to the new commit, the title and notes are rewritten, every asset is replaced (`gh release upload --clobber`), and any asset the new run does not produce is deleted. Two runs never overlap: the workflow's concurrency group makes a second run wait for the first.
- **Retention:** after publishing, the workflow keeps the 14 newest `nightly-YYYYMMDD` prereleases and deletes older ones together with their tags. It also deletes a `nightly-YYYYMMDD` tag whose release no longer exists. It never deletes any other release or tag: only names matching `^nightly-[0-9]{8}$` are touched, never `board-latest`, `board-YYYYMMDD` or `v*`, and never the tag it just published.
- **Dry runs:** a pull request that changes `nightly.yml`, or a manual run on any branch other than `main`, builds, packages and smoke-tests all four targets and skips the publish job. Only `main` publishes.

### Jobs and permissions

The workflow's default token permission is `contents: read`. No repository secret is used.

| Job | Runs | Token permissions |
| --- | --- | --- |
| `meta` | Names the day, the tag and the short SHA from the run's creation time | `actions: read` |
| `build` (4 legs) | Builds, smoke-tests and packages one target; the Linux leg also records the help text and the glibc floor from the packaged binary | `contents: read` |
| `changes` | Writes the "Changes since" and "Contributors" sections with `tools/release_notes.py` from the full history and the `nightly-*` tags, and shows them in its summary | `contents: read`, `pull-requests: read` |
| `publish` | Checks that exactly the four archives arrived, writes `SHA256SUMS` and the notes, attests and verifies every asset, creates or replaces the release, downloads it again to check the hashes and the tag, and prunes old nightlies | `contents: write`, `id-token: write`, `attestations: write` |

The publish job never runs code that was just built: the help text comes from the build job as a file.

## crates.io and npm (pre-alpha)

The first releases on both registries hold the name `jarrow` and ship real, working code from this repository, never an empty placeholder (D-012). They are pre-releases: the workspace version is `0.1.0-alpha.1`, and `0.1.0` stays reserved for the signed M5 release.

| Registry | Package | Contents today | Install |
| --- | --- | --- | --- |
| crates.io | `jarrow` | The CLI: `jarrow --version`, `jarrow tokens` (CSS) and `jarrow tokens --json`. `check` (M1) and `convert` (M2) are not built yet | `cargo install jarrow --version 0.1.0-alpha.1 --locked` (Cargo skips pre-releases unless the version is named; Rust 1.99 or newer) |
| crates.io | `jarrow-report` | Tiers, outcomes, report entries and the design tokens | `jarrow-report = "=0.1.0-alpha.1"` |
| npm | `jarrow` | `tokens.css` and `tokens.json`, written by `jarrow tokens` and `jarrow tokens --json` at publish time and never committed. The WASM engine arrives in M3 | `npm install jarrow@alpha` |

Only `crates/report` and `crates/cli` set `publish = true`; every other crate inherits `publish = false` from the workspace. Nothing is published from a maintainer machine.

### How a version is published

`.github/workflows/publish.yml` runs only by hand, only from `main`, with an input `what` (`crates`, `npm` or `both`):

```sh
gh workflow run publish.yml --repo jarrow-dev/jarrow --ref main -f what=both
```

| Job | Runs | Token permissions | Secret |
| --- | --- | --- | --- |
| `crates` | Packages and builds both crates (`cargo package -p jarrow-report -p jarrow`), publishes `jarrow-report`, polls the crates.io API and the sparse index until it is served, then publishes `jarrow` and waits for it the same way | `contents: read` | `CARGO_REGISTRY_TOKEN`, in the publishing step only |
| `npm tokens` | Builds the CLI and writes `packages/npm/tokens.css` and `tokens.json`; checks the CSS equals `site/src/styles/tokens.css` | `contents: read` | none |
| `npm` | Publishes `packages/npm` with `npm publish --provenance --access public --tag alpha` (npm refuses a pre-release without `--tag`). It runs no code built in this run | `contents: read`, `id-token: write` | `NPM_TOKEN` as `NODE_AUTH_TOKEN`, in the publishing step only |

A version that a registry already has is skipped, so a run that failed halfway can be run again. Versions are never deleted; a broken one is yanked (`cargo yank`) or deprecated (`npm deprecate`), and the next one is `0.1.0-alpha.2`. To publish a new pre-release, bump `version` in the root `Cargo.toml` (and `jarrow-report` in `[workspace.dependencies]`) and in `packages/npm/package.json` together; `tools/tests/test_publish.py` fails when they differ.

Check the npm package's provenance with `npm audit signatures` in a project that installed it, or on its npmjs.com page, which links the workflow run that built it.

### First run

The first run, https://github.com/jarrow-dev/jarrow/actions/runs/41691082030 (`what=both`, on `main` at `e2f6b4e`), published nothing. Both registries refused the upload for account reasons that only the owner can fix (`infra/TODO-OWNER.md`, M5):

- `crates`: both crates packaged and built. crates.io answered the upload of `jarrow-report` with "the remote server responded with an error (status 400 Bad Request): A verified email address is required to publish crates to crates.io. Visit https://crates.io/settings/profile to set and verify your email address." `jarrow` was not attempted.
- `npm tokens` passed. `npm` packed the five files and signed the provenance statement, then npm answered the upload with "403 Forbidden ... Two-factor authentication or granular access token with bypass 2fa enabled is required to publish packages."

After the owner's fix, the same command runs it again and skips anything already published: `gh workflow run publish.yml --repo jarrow-dev/jarrow --ref main -f what=both`.

## v0.1 and later (M5)

v0.1 is the first versioned release, planned for milestone M5 (see [ROADMAP.md](../ROADMAP.md)). It will be built with cargo-dist from a `v*` tag and published with signed artefacts on every channel: GitHub Releases with installers, a Homebrew tap, a Scoop bucket, winget, AUR, crates.io for the CLI and `jarrow-report` (D-012), and npm for the WASM package. The registry and distribution tokens those channels need are listed in `infra/INVENTORY.md`. The nightly keeps running beside it as the pre-release channel.

None of this exists yet. Until it does, the nightly is the only way to get a prebuilt CLI. To build it from source, use Rust 1.99 or newer: `cargo install jarrow --version 0.1.0-alpha.1 --locked` for the pre-alpha on crates.io, or `cargo install --git https://github.com/jarrow-dev/jarrow --locked jarrow` for `main`.

## First nightly

`nightly.yml` merged on 2026-11-03 as [`ea0af33`](https://github.com/MosheByte/jarrow/commit/ea0af33aa4f245d05d026e4a1f424c9be0574b4d) and was then run twice by hand on `main` that day. The second run checked the same-day path: it moved the tag (to the same commit), rewrote the notes and replaced all five assets.

| | Link |
| --- | --- |
| Release | [nightly-20261103](https://github.com/MosheByte/jarrow/releases/tag/nightly-20261103), "Nightly 2026-11-03 (ea0af33)" |
| First run (created the release) | https://github.com/MosheByte/jarrow/actions/runs/41670170206 |
| Same-day rerun (replaced it) | https://github.com/MosheByte/jarrow/actions/runs/41670924062 |
| Attestation, first run | https://github.com/MosheByte/jarrow/attestations/63185312 |
| Attestation, rerun (the current assets) | https://github.com/MosheByte/jarrow/attestations/63187171 |

Checked from a maintainer machine after the runs:

- `gh release view nightly-20261103 --repo MosheByte/jarrow`: a prerelease, not a draft and not "Latest", tag on `ea0af33`, with the four archives and `SHA256SUMS`.
- `sha256sum --check --ignore-missing SHA256SUMS` passed for the downloaded `jarrow-x86_64-pc-windows-msvc.zip` and `jarrow-x86_64-unknown-linux-gnu.tar.gz`.
- `gh attestation verify <archive> --repo MosheByte/jarrow --signer-workflow MosheByte/jarrow/.github/workflows/nightly.yml --source-ref refs/heads/main` exited 0 for both. The verified SLSA v1 provenance names `ea0af33`, `refs/heads/main` and the run that built the file.

The two runs built the same commit on fresh runners. The Linux and both macOS archives came out byte-identical, so `gh attestation verify` on the Linux archive finds both runs' attestations. The Windows zip differed between the runs. Only 24 bytes of `jarrow.exe` differ, all written by the MSVC linker: the link timestamp in the PE header and debug directory, and the PDB GUID.

## Test coverage

The `coverage` job in `.github/workflows/ci.yml` measures the line coverage of the Rust workspace on every pull request, in the merge queue and on every push to `main`. On Ubuntu, with the JDK, it runs the same tests as the `rust` job under `cargo llvm-cov --workspace --locked --lcov` (cargo-llvm-cov 0.9.1, installed by `taiki-e/install-action` pinned by SHA). It writes the total line coverage, and each crate's, into its job summary, and uploads `lcov.info` to Codecov with `codecov/codecov-action` (pinned by SHA) and the `CODECOV_TOKEN` secret: [app.codecov.io/gh/jarrow-dev/jarrow](https://app.codecov.io/gh/jarrow-dev/jarrow). Pull requests from forks and from Dependabot get no repository secrets, so they skip the upload; the job still measures and summarises. Codecov counts partially covered lines on its own terms, so its number can differ a little from the job summary's.

Coverage is informational for now. Nothing fails on the number: `codecov.yml` marks the project status (target 80% of `crates/`) and the patch status `informational: true`, an upload error does not fail the job (`fail_ci_if_error: false`), and Codecov posts no pull request comment. The job fails only when a test fails, as the `rust` job would, and `ci-ok` needs it like every other job.

**The floor: 80% line coverage of `crates/`.** It becomes enforced once the numbers settle, that is once the M1 crates have landed and the total on `main` stops moving by more than a point or two between merges. Enforcing it is one pull request with its own `D-` entry: drop `informational: true` from the project status in `codecov.yml`, and make the coverage job fail when the total for `crates/` in its summary is below 80%, so the gate does not depend on Codecov being up.
