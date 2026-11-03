# Coding standards

The style and quality rules for each language in Jarrow, and the tool that enforces each one. Contributions must follow them; a pull request that does not pass the checks below does not merge ([code-review.md](code-review.md)). Where a rule is not machine-checked yet, this page says so.

Run everything CI runs before you push: `py -3.12 tools/pr_ready.py` (or `python tools/pr_ready.py`) plus the commands in each section.

## Rust (the product: `crates/`)

Style guide: the [Rust Style Guide](https://doc.rust-lang.org/nightly/style-guide/) as implemented by `rustfmt` (style edition 2024), and the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) for public APIs.

| Rule | Where it is set | Enforced by |
| --- | --- | --- |
| Formatting: `rustfmt`, style edition 2024, field-init and `?` shorthands | [`rustfmt.toml`](../rustfmt.toml) | CI job `fmt`: `cargo fmt --all -- --check` |
| No compiler or clippy warning, on every target including tests | | CI jobs `rust (3 OS)`: `cargo clippy --workspace --all-targets --locked -- -D warnings` |
| No `unsafe` code, anywhere | `[workspace.lints.rust] unsafe_code = "forbid"` in [`Cargo.toml`](../Cargo.toml), and `#![forbid(unsafe_code)]` in every crate root | The compiler (a `forbid` cannot be overridden). The only planned exception is the WASM bindings crate (M3), if `wasm-bindgen` makes it unavoidable |
| Library code does not panic on bad input: no `unwrap()`, `expect()`, `todo!()`, `unimplemented!()` or `dbg!()` | `[workspace.lints.clippy]` in [`Cargo.toml`](../Cargo.toml); tests may unwrap ([`clippy.toml`](../clippy.toml)) | clippy with `-D warnings` |
| Every public item is documented | `missing_docs = "warn"`, which `-D warnings` makes an error | clippy; rustdoc with `RUSTDOCFLAGS="-D warnings"` in [`pages.yml`](../.github/workflows/pages.yml) |
| Edition 2024, the toolchain pinned in `rust-toolchain.toml` (MSRV equals the pin) | [`rust-toolchain.toml`](../rust-toolchain.toml), [`Cargo.toml`](../Cargo.toml) | Every CI job installs the pinned toolchain |
| Dependencies: crates.io only, allowed licenses, no known advisory, no unmaintained or yanked crate, no wildcard version | [`deny.toml`](../deny.toml) | CI job `deny`: `cargo deny check` |
| Untrusted input is bounded: counts, sizes and offsets checked before use; limits documented | [assurance case](security/assurance-case.md) | Review; fuzzing in `cflite-pr` |
| Determinism: no clock, host name, locale, environment or hash-map order in any output | [architecture.md](architecture.md), "Determinism contract" | Tests that compare outputs byte for byte; `reproducible.yml` for the CLI binary |

## Python (`tools/`)

Style guide: [PEP 8](https://peps.python.org/pep-0008/), with lines up to 120 characters, type hints on public functions, and `from __future__ import annotations`. The tools use the standard library only; test dependencies are pinned with hashes in [`tools/requirements-dev.txt`](../tools/requirements-dev.txt).

| Rule | Where it is set | Enforced by |
| --- | --- | --- |
| Pyflakes (unused or undefined names, unused imports, shadowing) and the pycodestyle errors and warnings that do not depend on line length | [`ruff.toml`](../ruff.toml) (`E4`, `E7`, `E9`, `F`, `W`) | CI job `python (3 OS)`: `python -m ruff check` |
| Python 3.12 | `target-version` in `ruff.toml`, `setup-python` in CI | CI |
| Every tool has tests in `tools/tests/`, run on Linux, macOS and Windows | | CI: `python -m pytest tools/tests` |
| Inline Python in workflows (`shell: python` steps) follows the same rules and is tested by cutting it out of the YAML | [`test_community_workflows.py`](../tools/tests/test_community_workflows.py) | CI: pytest |

Not machine-checked yet: line length and formatting (`ruff format` would rewrite most files; it will be turned on in a pull request of its own).

## TypeScript and Astro (`site/`)

Style guide: TypeScript in strict mode (`astro/tsconfigs/strict`), ES modules, two-space indentation, single quotes, and no `any` where a type is known. Components follow the existing files in `site/src/components/`.

| Rule | Where it is set | Enforced by |
| --- | --- | --- |
| Type checking in strict mode, Astro components included | [`site/tsconfig.json`](../site/tsconfig.json) | CI job `site (3 OS)`: `npm run check` (`astro check`) |
| Tests for logic in `src/lib/` and `scripts/` | | CI: `npm test` (`node --test`) |
| No inline script the Content-Security-Policy does not list, no inline event handler, no `javascript:` URL, no script from another origin | [`site/src/lib/headers.ts`](../site/src/lib/headers.ts) | CI: `npm run build` ends with `scripts/csp-check.mjs` |
| Colours only from the design tokens; green, amber and red only for results | [`crates/report/src/tokens.rs`](../crates/report/src/tokens.rs) | `tokens_current` test (the CSS is generated); review for usage |

Not machine-checked yet: formatting (indentation, quotes, line breaks). A formatter check (Prettier with the Astro plugin) is planned once the site rewrite that is in progress has landed.

## GitHub Actions workflows (`.github/workflows/`)

| Rule | Enforced by |
| --- | --- |
| Valid syntax, expressions, contexts, `needs`, action inputs and `permissions` | CI job `actionlint` (actionlint 1.7.12, checksum-verified download) |
| Top-level `permissions: contents: read`; each job asks only for what it uses | Review; `test_workflow_rules` for the community workflows |
| Every `uses:` pinned to a full commit SHA with a `# vX` comment | Review; `test_workflow_rules` for the community workflows; Dependabot keeps the pins current |
| `persist-credentials: false` on every checkout | Review |
| Untrusted values (titles, bodies, branch names, artifacts) reach scripts through `env:`, never through `${{ }}` inside `run:` | Review; CodeQL for Actions (`codeql.yml`) |
| Secrets on the step that needs them, never on a job; never echoed | Review; `test_secret_jobs_skip_pull_requests_from_bots` for the Cloudflare token |
| Workflows that react to other people's content (`issues`, `discussion`, `workflow_run`, `pull_request_target`) never check out or run pull request code | `test_workflow_rules`; review |

## Everywhere

- Line endings are LF ([`.gitattributes`](../.gitattributes)), so byte-level tests agree on every OS.
- No JAR, class file or decompiled source is ever committed, and no secret: `tools/pr_ready.py` checks every tracked file.
- Commit messages: imperative, a short first line, details below; agent commits carry the `Co-Authored-By` trailer (checked by `pr_ready.py`), and from 2026-11-03 every commit carries a DCO sign-off (checked by `dco.yml`, see [CONTRIBUTING.md](../CONTRIBUTING.md)).
- Project copy never promises an outcome, and nothing is called green, working or supported without a harness result ([AGENTS.md](../AGENTS.md)).
