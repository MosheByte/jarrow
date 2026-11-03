# Security assurance case

This is the argument that Jarrow meets its security requirements, with the evidence for each claim. It covers what exists on `main` today (milestone M1: the JAR reader, the class-file parser, mappings, the harness, the board pipeline, the site and the CI) and states the design for the parts that are planned (Tier B in M4, signed releases in M5). Where something is not built yet, it says so.

It is kept current: a pull request that changes a trust boundary, adds a secret, adds an input from outside the project or adds a workflow trigger updates this file in the same pull request ([code review standards](../code-review.md)). The last full [security review](review-2026-11-03.md) checked the claims below against the code on 2026-11-03.

Links point at `main` of https://github.com/jarrow-dev/jarrow.

## 1. Security requirements: what you can and cannot expect

What Jarrow promises:

1. **Reading a JAR is safe.** Any byte sequence given to the JAR reader or the class-file parser ends in a result or an error, never in a crash, memory corruption, unbounded memory use or a file written outside where the caller asked. This holds in the browser (WASM), the CLI, the board and, from M4, Tier B.
2. **Your JAR stays yours.** Tier A runs in your browser or on your machine, and no network request carries the JAR. Tier B (M4) holds an upload only for the job that converts it. The project never stores, hosts or links to a converted JAR, and never stores a JAR in its own storage ([legal.md](../legal.md)).
3. **What CI builds is what the source says.** Release binaries come from `main`, built by GitHub-hosted runners from a clean checkout, with SLSA build provenance signed through Sigstore ([releases.md](../releases.md)); the build is reproducible bit for bit (`reproducible.yml`).
4. **Pull requests cannot reach secrets.** Code from a pull request never runs in a job that holds a secret or a write token, unless the pull request comes from a person with write access to the repository.
5. **The site does not run code it did not ship.** Pages are encoded on output and served with a strict Content-Security-Policy, HSTS and anti-framing headers.
6. **Downloads are verified.** Everything the harness downloads (Minecraft server, loaders, APIs, mappings) is checked against hashes pinned in `research-lock.json` before use.

What Jarrow does not promise:

- **A converted mod is not sandboxed.** The converter rewrites bytecode; it does not make a mod safe. A malicious Fabric mod converts into a malicious NeoForge mod. Only run mods you would run unconverted.
- **The harness runs mods.** `jarrow-harness` starts real Minecraft servers with the mods it is given. Run it only on mods you trust, or in a throwaway machine or CI runner, as the project does.
- **No release is signed with a code-signing certificate yet.** Nightly binaries have provenance attestations but no Apple or Windows signature (planned for v0.1, M5).
- **Pre-alpha.** Nothing converts yet; the security of the converter's output is not established by anything but the requirements above.

## 2. Threat model

### Assets

| Asset | Why it matters |
| --- | --- |
| The machines that read JARs: browser tabs, CLI users, CI runners | Requirement 1 |
| Users' JARs and their contents (possibly unpublished mods) | Requirement 2 |
| `main`, tags, releases and their attestations | What users download and run (requirement 3) |
| Secrets: `CLOUDFLARE_API_TOKEN`, `BOT_DEPLOY_KEY`, `GITLAB_MIRROR_TOKEN`; the owner's EULA acceptance `MINECRAFT_EULA_ACCEPTED` | Write access to the site, Cloudflare storage, `bot/` branches and the mirror; a terms acceptance only the owner gives |
| Cloudflare resources: Pages project, D1 `jarrow`, KV `jarrow-board`, R2 `jarrow-reports` | The public board and reports |
| Site visitors' browsers | Requirement 5 |
| Maintainer accounts on GitHub and Cloudflare | Control over all of the above |

### Attackers

| Attacker | Can | Wants |
| --- | --- | --- |
| A mod author or anyone who can hand a JAR to Jarrow | Craft any bytes: zip bombs, overlapping entries, lying sizes, traversal names, deep nesting, malformed class files | Crash or exhaust the converter, the board or Tier B; write files outside the work directory; execute code |
| A pull request author (fork) | Change any file in the pull request, including workflows, tests, build scripts and dependencies; control branch names, titles and artifacts of their runs | Read a secret, get a write token, push to `main`, plant code in a release |
| A supply-chain attacker | Publish a malicious version of a crate, npm package, action or base image | Code execution in CI with whatever that job holds |
| An issue, discussion or Modrinth project author | Control text and URLs the bots read and quote | Make a bot post links or mentions in the project's name; break a bot |
| A network attacker | Intercept or redirect traffic | Substitute a downloaded artifact; read traffic |
| A malicious web page | Frame the site, load its resources | Clickjacking; injecting script |
| Someone who takes over a maintainer account | Anything that account can do | Everything above at once |

### Out of scope

The security of Minecraft, Fabric, NeoForge or a mod itself; the GitHub and Cloudflare platforms; denial of service by volume ([SECURITY.md](../../SECURITY.md), "Scope").

## 3. Trust boundaries

```
 untrusted                                   |  Jarrow
-------------------------------------------- + ------------------------------------------------
 B1  JAR and class-file bytes                 -> crates/jar, crates/classfile (WASM, CLI, board, Tier B)
 B2  Internet downloads (Mojang, Fabric,      -> crates/harness fetch (hash pins), crates/board http
     NeoForge, Modrinth)
 B3  Pull request code and its run artifacts  -> CI runners without secrets (pull_request), and
                                                 pull_request_target / workflow_run jobs that never run it
 B4  Issue, discussion and Modrinth text      -> bot comments (welcome, red-mod-intake, proposal-to-issue)
 B5  Jobs on main and schedules               -> secrets (only these, and same-repository PRs by people)
 B6  board.json in KV (written by the nightly) -> server-rendered pages -> visitors' browsers
 B7  Tier B upload (M4, planned)              -> one ephemeral Actions job, wiped afterwards
 B8  Maintainer accounts                      -> GitHub organization, Cloudflare account
```

| Boundary | What crosses it | What guards it |
| --- | --- | --- |
| B1 | Bytes from anyone | The limits and checks of section 5, `forbid(unsafe_code)`, fuzzing |
| B2 | Files from upstream servers | HTTPS with certificate verification (rustls, OS roots); every artifact pinned by hash in `research-lock.json`, checked before use; size limits |
| B3 | Code, branch names, artifacts | `pull_request` runs get a read-only token and no secrets; jobs with secrets skip forks and bots; `pull_request_target` (labeler) and `workflow_run` (welcome) never check out or run pull request code |
| B4 | Text and URLs | Values reach scripts through `env:`, not `${{ }}`; output is defused (mentions, hidden markers, table cells) and URLs are percent-encoded before they go into Markdown |
| B5 | Secrets | Step-level `env:`; only `main`, schedules and same-repository human PRs; tokens never echoed |
| B6 | Board data | `site/src/lib/board.ts` validates the document; Astro escapes output; links must be `https:`; CSP |
| B7 | A user's JAR (M4) | Planned: presigned R2 upload deleted after download, 1-day bucket expiry, 30-minute job, workspace wiped, no decompiled source in logs ([architecture.md](../architecture.md), "Tier B pipeline") |
| B8 | Admin rights | 2FA on the one admin account; see [GOVERNANCE.md](../../GOVERNANCE.md) for the single-admin risk |

## 4. Secure design principles applied

The principles of Saltzer and Schroeder, as the OpenSSF criteria name them, with where each one shows in the code and configuration.

| Principle | Where it is applied | Evidence |
| --- | --- | --- |
| Economy of mechanism | One Rust workspace is the product; the site, the CLI and the Actions jobs are thin shells around it. The JAR reader depends only on `miniz_oxide` and does no I/O | [architecture.md](../architecture.md), [`crates/jar/Cargo.toml`](../../crates/jar/Cargo.toml) |
| Fail-safe defaults | The harness refuses to start a server unless the EULA gate is open; a download without a pinned hash is refused; the reader refuses (rather than skips) unsafe names, encryption, size lies and CRC errors; the board never shows green without a harness score; the CSP starts from `default-src 'none'` | [`eula.rs`](../../crates/harness/src/eula.rs), [`fetch.rs`](../../crates/harness/src/fetch.rs) `download_verified`, [`read.rs`](../../crates/jar/src/read.rs), [`pipeline.rs`](../../crates/board/src/pipeline.rs) `decide`, [`headers.ts`](../../site/src/lib/headers.ts) |
| Complete mediation | Every entry name is checked when read and again when written; every archive offset and size is checked against the data before use; every board row is validated before it is rendered | [`name.rs`](../../crates/jar/src/name.rs) `check_name`, [`write.rs`](../../crates/jar/src/write.rs), [`board.ts`](../../site/src/lib/board.ts) |
| Open design | Everything is public: code, workflows, rule tables, this document. No security depends on secrecy except the tokens themselves | This repository |
| Separation of privilege | A change reaches `main` only through a pull request whose `ci-ok` check passes, run again by the merge queue on the exact merge result. The `main` ruleset has no bypass actors, so this holds for the owner too (an admin could edit the ruleset itself; the organization audit log records that). Terms acceptances (the EULA) and tokens are the owner's alone, never the agent's | [`ci.yml`](../../.github/workflows/ci.yml) `ci-ok`, [code-review.md](../code-review.md), [GOVERNANCE.md](../../GOVERNANCE.md) |
| Least privilege | `permissions: contents: read` at the top of every workflow, write scopes per job; `persist-credentials: false` on every checkout; secrets on steps, not jobs; child processes start with a cleared environment and an allowlist | [`.github/workflows/`](../../.github/workflows/), [`process.rs`](../../crates/harness/src/process.rs) `CHILD_ENV_ALLOWLIST` |
| Least common mechanism | Fork pull requests share no secret-bearing job with trusted code; the bot that comments on first PRs runs in a job with no checkout | [`deploy-site.yml`](../../.github/workflows/deploy-site.yml), [`welcome.yml`](../../.github/workflows/welcome.yml) |
| Psychological acceptability | Safe paths are the easy ones: `pr_ready.py` runs every merge check locally, the DCO check prints the exact command to fix a commit, the CSP check prints the hashes to paste | [`pr_ready.py`](../../tools/pr_ready.py), [`dco_check.py`](../../tools/dco_check.py), [`csp-check.mjs`](../../site/scripts/csp-check.mjs) |
| Limited attack surface | No server-side accounts or uploads before M4; three server routes; the site stores zero JAR bytes; the CLI has no network code | [`site/src/pages/`](../../site/src/pages/), [legal.md](../legal.md) |
| Input validation with allowlists | Entry names, compression methods (stored and deflate only), mirror paths, branch names for preview deploys, Modrinth slugs and hosts in the red-mod bot, `https:` for every link the site renders | [`read.rs`](../../crates/jar/src/read.rs), [`mirror.yml`](../../.github/workflows/mirror.yml), [`red-mod-intake.yml`](../../.github/workflows/red-mod-intake.yml) `modrinth_slug`, [`board.ts`](../../site/src/lib/board.ts) `isHttpsUrl` |

### The concrete mechanisms

- **The JAR reader's limits** ([`ReadOptions`](../../crates/jar/src/read.rs)): at most 100,000 entries, 256 MiB per entry and 1 GiB in total, checked on the declared sizes before anything is decompressed; the inflater is capped at the declared size plus one byte, so an archive that lies fails instead of filling memory; overlapping entries are refused; nested jars share one budget and stop at depth 8 (never more than 64). Tests: [`guards.rs`](../../crates/jar/tests/guards.rs), [`no_panic.rs`](../../crates/jar/tests/no_panic.rs), [`zip64.rs`](../../crates/jar/tests/zip64.rs), [`nested.rs`](../../crates/jar/tests/nested.rs).
- **No unsafe code**: `unsafe_code = "forbid"` for the whole workspace and `#![forbid(unsafe_code)]` in every crate root; clippy denies `unwrap` and `expect` in library code, so bad input becomes an error, not a panic ([`Cargo.toml`](../../Cargo.toml), [`clippy.toml`](../../clippy.toml)).
- **Fuzzing**: ClusterFuzzLite fuzzes `jar_read` and `classfile_parse` with AddressSanitizer for five minutes on every pull request that touches Rust and for an hour every week, built with debug assertions and overflow checks ([`cflite_pr.yml`](../../.github/workflows/cflite_pr.yml), [`cflite_batch.yml`](../../.github/workflows/cflite_batch.yml), [`fuzz/`](../../fuzz/)).
- **The EULA gate**: no Minecraft server starts unless `MINECRAFT_EULA_ACCEPTED` (or `MINECRAFT_EULA`) is true, and only the owner sets that secret ([`eula.rs`](../../crates/harness/src/eula.rs), [`baseline.sh`](../../harness/probe/ci/baseline.sh)).
- **Fork pull requests get no secrets**: `deploy-site.yml` and the harness `baseline` job run only for same-repository pull requests whose actor and author are not bots; everything else on `pull_request` has a read-only token ([`deploy-site.yml`](../../.github/workflows/deploy-site.yml), [`harness.yml`](../../.github/workflows/harness.yml)).
- **SHA-pinned actions and images**: every `uses:` names a full commit SHA with its version as a comment; the ClusterFuzzLite and devcontainer base images are pinned by digest; Python test dependencies install with `--require-hashes`; npm installs with `npm ci` from a lockfile; Dependabot proposes updates weekly ([`dependabot.yml`](../../.github/dependabot.yml)).
- **Secret scanning**: GitHub secret scanning and push protection are on for the repository, and `tools/pr_ready.py` (part of `ci-ok`) fails on any tracked file that looks like a GitHub, npm, GitLab, Slack or AWS token, a private key, a webhook URL or a literal Cloudflare token, without printing the value.
- **No JAR storage**: `.gitignore` blocks `*.jar`, and `pr_ready.py` fails on any tracked JAR, class file, archive holding classes, or decompiled source; the board downloads each JAR to a temporary directory, deletes it after processing and fails the shard if one is left; R2 holds reports only ([`board.yml`](../../.github/workflows/board.yml) `shard`).
- **Site headers**: the CSP, HSTS and the other headers in [`headers.ts`](../../site/src/lib/headers.ts), checked against every built page by [`csp-check.mjs`](../../site/scripts/csp-check.mjs) in `npm run build`.
- **Static analysis**: CodeQL for Actions, JavaScript/TypeScript, Python and Rust on every pull request and weekly ([`codeql.yml`](../../.github/workflows/codeql.yml)); `cargo deny` for advisories, licenses and sources ([`deny.toml`](../../deny.toml)); OpenSSF Scorecard ([`scorecard.yml`](../../.github/workflows/scorecard.yml)); clippy with warnings denied.

## 5. Common weaknesses countered

| CWE | Weakness | Where it could occur | How it is countered | Evidence |
| --- | --- | --- | --- | --- |
| [CWE-22](https://cwe.mitre.org/data/definitions/22.html) | Path traversal ("zip slip") | Entry names in JARs | `check_name` refuses `..`, absolute paths, drive letters, backslashes, NUL, empty and dot-only components, on read and on write | [`name.rs`](../../crates/jar/src/name.rs) tests |
| [CWE-409](https://cwe.mitre.org/data/definitions/409.html) | Highly compressed data (zip bomb) | Deflate entries, overlapping entries, nested jars | Declared-size limits before inflating, inflater capped at the declared size, overlap refused, one budget for nested jars | [`read.rs`](../../crates/jar/src/read.rs), [`guards.rs`](../../crates/jar/tests/guards.rs) |
| [CWE-770](https://cwe.mitre.org/data/definitions/770.html) | Allocation without limits | Counts from archives and class files | Entry-count limit; list capacities capped by the bytes left (`capacity()`); switch tables checked against the code length | [`bytes.rs`](../../crates/classfile/src/bytes.rs), [`insn.rs`](../../crates/classfile/src/insn.rs) |
| [CWE-674](https://cwe.mitre.org/data/definitions/674.html) | Uncontrolled recursion | Nested annotations, nested jars | `MAX_NESTING` in annotation parsing; `HARD_MAX_NESTING_DEPTH` for jars | [`annotation.rs`](../../crates/classfile/src/annotation.rs), [`nested.rs`](../../crates/jar/src/nested.rs) |
| [CWE-125](https://cwe.mitre.org/data/definitions/125.html), [CWE-787](https://cwe.mitre.org/data/definitions/787.html) | Out-of-bounds read or write | All parsers | Safe Rust only (`forbid(unsafe_code)`); bounds-checked cursors; fuzzing with AddressSanitizer | [`Cargo.toml`](../../Cargo.toml), [`fuzz/`](../../fuzz/) |
| [CWE-190](https://cwe.mitre.org/data/definitions/190.html) | Integer overflow | Offsets and sizes | `checked_add`/`checked_mul` and `try_from` on every untrusted value; fuzz builds with overflow checks | [`read.rs`](../../crates/jar/src/read.rs), [`.clusterfuzzlite/build.sh`](../../.clusterfuzzlite/build.sh) |
| [CWE-20](https://cwe.mitre.org/data/definitions/20.html) | Improper input validation | Archive structure | CRC, sizes, local/central header agreement, methods, encryption flags all checked; errors instead of silent skips | [`read.rs`](../../crates/jar/src/read.rs) |
| [CWE-78](https://cwe.mitre.org/data/definitions/78.html), [CWE-94](https://cwe.mitre.org/data/definitions/94.html) | Command or code injection | Workflows reading branch names, titles, bodies; harness starting Java | Untrusted values reach shells only through `env:`; `Command::new` with an argument list, never a shell | [security review](review-2026-11-03.md), [`process.rs`](../../crates/harness/src/process.rs) |
| [CWE-117](https://cwe.mitre.org/data/definitions/117.html) | Log injection | `pr_ready.py` printing file names | CR and LF escaped, annotations escaped | [`pr_ready.py`](../../tools/pr_ready.py) `_one_line` |
| [CWE-116](https://cwe.mitre.org/data/definitions/116.html) | Improper output encoding | Bot comments quoting other people's text and URLs | Mentions and markers defused, table cells escaped, URLs percent-encoded | [`welcome.yml`](../../.github/workflows/welcome.yml), [`red-mod-intake.yml`](../../.github/workflows/red-mod-intake.yml), [`proposal-to-issue.yml`](../../.github/workflows/proposal-to-issue.yml) |
| [CWE-79](https://cwe.mitre.org/data/definitions/79.html) | Cross-site scripting | Board rows on server-rendered pages | Astro output escaping, no `set:html`, `https:`-only links, CSP without `unsafe-inline` scripts | [`board.ts`](../../site/src/lib/board.ts), [`headers.ts`](../../site/src/lib/headers.ts) |
| [CWE-1021](https://cwe.mitre.org/data/definitions/1021.html) | Clickjacking | The site | `frame-ancestors 'none'` and `X-Frame-Options: DENY` | [`_headers`](../../site/public/_headers) |
| [CWE-319](https://cwe.mitre.org/data/definitions/319.html) | Cleartext transmission | Downloads, the site | HTTPS only (plain HTTP to loopback for tests), redirects off HTTPS refused in `research_lock.py`; HSTS on the site | [`fetch.rs`](../../crates/harness/src/fetch.rs) `url_allowed`, [`research_lock.py`](../../tools/research_lock.py) |
| [CWE-494](https://cwe.mitre.org/data/definitions/494.html) | Download of code without an integrity check | Minecraft server, loaders, actions, dependencies | Hash pins verified before use; actions pinned by SHA; images by digest; pip by hash; npm by lockfile | [`fetch.rs`](../../crates/harness/src/fetch.rs), [`research-lock.json`](../../research-lock.json) |
| [CWE-829](https://cwe.mitre.org/data/definitions/829.html) | Functionality from an untrusted control sphere | Third-party actions | SHA pins, Dependabot review, CodeQL for Actions | [`.github/workflows/`](../../.github/workflows/) |
| [CWE-798](https://cwe.mitre.org/data/definitions/798.html), [CWE-532](https://cwe.mitre.org/data/definitions/532.html) | Hard-coded credentials; secrets in logs | Repository files, workflow logs | Secret scanning with push protection, `pr_ready.py` secret patterns, secrets in Actions secrets only, `wrangler whoami` output kept out of logs, key file removed by a trap | [`pr_ready.py`](../../tools/pr_ready.py), [`bot_pr.sh`](../../tools/bot_pr.sh) |
| [CWE-250](https://cwe.mitre.org/data/definitions/250.html) | Execution with unnecessary privileges | Workflow tokens | Read-only default token, per-job write scopes | [`.github/workflows/`](../../.github/workflows/) |

## 6. Residual risks

Known and accepted for now, each with its tracking:

- One person holds every admin right; see [GOVERNANCE.md](../../GOVERNANCE.md), "Access continuity".
- The Cloudflare token is broader than it needs to be ([#24](https://github.com/jarrow-dev/jarrow/issues/24)), and the organization does not yet require 2FA ([#25](https://github.com/jarrow-dev/jarrow/issues/25)).
- Mojang artifacts are pinned by SHA-1 only, because that is what Mojang publishes ([#28](https://github.com/jarrow-dev/jarrow/issues/28)).
- Same-repository pull requests by people with write access run with secrets in `deploy-site.yml` and the harness baseline. Write access is the trust boundary; today only the owner and the build agent working through the owner's account have it.
- `style-src` allows inline styles (Starlight's style attributes); styles cannot run script.
- Tier B (M4) is designed but not built; its boundary (B7) is reviewed when it is.
