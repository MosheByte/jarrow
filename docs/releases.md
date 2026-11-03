# Releases

From milestone M1 on, the `jarrow` CLI ships as a nightly prerelease built from `main`. Signed, versioned releases start with v0.1 in milestone M5. This page covers both channels and how to check what you downloaded.

## Channels

| Channel | What you get | Where | Status |
| --- | --- | --- | --- |
| Nightly | A pre-alpha build of the CLI from `main`, once a day, for Linux, Windows and macOS | GitHub prereleases tagged `nightly-YYYYMMDD`: [all releases](https://github.com/MosheByte/jarrow/releases) | Running from M1 (`.github/workflows/nightly.yml`) |
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

### Verify a download

Check the archive against `SHA256SUMS` from the same release:

```sh
sha256sum --check --ignore-missing SHA256SUMS           # Linux
shasum -a 256 jarrow-aarch64-apple-darwin.tar.gz        # macOS: compare with the line in SHA256SUMS
Get-FileHash -Algorithm SHA256 jarrow-x86_64-pc-windows-msvc.zip   # Windows PowerShell
```

Check where it came from. Every archive and `SHA256SUMS` carries a Sigstore-signed SLSA build provenance attestation, made by `actions/attest-build-provenance` in the run that built it. With the GitHub CLI:

```sh
gh attestation verify jarrow-x86_64-unknown-linux-gnu.tar.gz --repo MosheByte/jarrow \
  --signer-workflow MosheByte/jarrow/.github/workflows/nightly.yml --source-ref refs/heads/main
```

That passes only for a file whose SHA-256 was attested by `nightly.yml` running on `main` of this repository, on a GitHub-hosted runner. The output names the commit and the workflow run. The publish job runs the same check on every asset before it publishes anything.

The binaries carry no Apple or Windows code signature and are not notarized, so macOS Gatekeeper and Windows SmartScreen may warn before the first run. Code signing comes with v0.1.

### When it runs

- **Every day at 05:30 UTC**, after the board run at 02:00 UTC.
- **By hand:** `gh workflow run nightly.yml --repo MosheByte/jarrow --ref main`.
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
| `publish` | Checks that exactly the four archives arrived, writes `SHA256SUMS` and the notes, attests and verifies every asset, creates or replaces the release, downloads it again to check the hashes and the tag, and prunes old nightlies | `contents: write`, `id-token: write`, `attestations: write` |

The publish job never runs code that was just built: the help text comes from the build job as a file.

## v0.1 and later (M5)

v0.1 is the first versioned release, planned for milestone M5 (see [ROADMAP.md](../ROADMAP.md)). It will be built with cargo-dist from a `v*` tag and published with signed artefacts on every channel: GitHub Releases with installers, a Homebrew tap, a Scoop bucket, winget, AUR, crates.io for the CLI and `jarrow-report` (D-012), and npm for the WASM package. The registry and distribution tokens those channels need are listed in `infra/INVENTORY.md`. The nightly keeps running beside it as the pre-release channel.

None of this exists yet. Until it does, the nightly is the only way to get a built CLI. To build the same code from source instead, use Rust 1.99 or newer: `cargo install --git https://github.com/MosheByte/jarrow --locked jarrow`.

## First nightly

The first run starts right after `nightly.yml` merges into `main`. Its release, run and attestation links are recorded here once it has finished.
