# Security policy

What you can and cannot expect from Jarrow's security, and why, is in the [assurance case](docs/security/assurance-case.md) (section 1, "Security requirements"). The most recent security review is [docs/security/review-2026-11-03.md](docs/security/review-2026-11-03.md).

## Supported versions

Jarrow has no versioned release yet. Until v0.1 (milestone M5), only the `main` branch and the newest nightly prerelease built from it are supported. Fixes land on `main` and reach users in the next nightly; there are no backports, and an older nightly is never patched.

| Version | Supported |
| --- | --- |
| `main` | Yes |
| The newest `nightly-YYYYMMDD` prerelease | Yes: the fix ships in the next nightly, built from `main` |
| Older nightlies | No: download the newest one ([docs/releases.md](docs/releases.md)) |
| v0.1 and later | Do not exist yet |

From v0.1 on, the policy will be: the newest minor release gets security fixes; the minor before it gets fixes for critical and high severity for 3 months after the newer one comes out; anything older is unsupported. Upgrading within a minor version never needs more than replacing the binary. This section changes with the first release.

## How to report

Use GitHub private vulnerability reporting, which is enabled for this repository:

https://github.com/jarrow-dev/jarrow/security/advisories/new

Please do not open a public issue, discussion or pull request for a vulnerability.

A good report says what is affected (a crate, a page, a route, a workflow), how to reproduce it, and what an attacker could do with it. A minimal proof of concept is welcome.

## What not to send

- **No mod JARs.** Not as attachments, not as links to copies you made. Link to the mod's public page (Modrinth or the author's site) instead.
- **No decompiled code** from any mod, even a few lines. Describe the behaviour, or name the class and method.
- **No secrets.** If you found a leaked token or key, tell us where it is (file, commit, log line) without copying its value.

## Scope

In scope:

- The conversion engine: every crate under `crates/`, including the WASM build that runs in the browser and the `jarrow` CLI.
- The site and its server routes on https://jarrow.pages.dev, and the Cloudflare Workers that serve or ingest board data.
- Tier B jobs: the GitHub Actions workflow that converts uploaded JARs, its upload and download path, and the rules that the workspace is wiped after each job and that no decompiled source reaches a log.
- The release pipeline once it exists: signing, SBOM, provenance and the packages published from this repository.
- Rule tables under `rules/`, where a rule could make the converter produce unsafe output.

Out of scope:

- Vulnerabilities in Minecraft, Fabric Loader, Fabric API, NeoForge or in a mod itself. Report those to their own projects.
- The GitHub and Cloudflare platforms themselves.
- Volume denial-of-service tests, social engineering, and physical attacks.

## What happens next

Reports are read by the project owner and by the AI agent that maintains Jarrow for the owner; both can see private vulnerability reports, nobody else can ([GOVERNANCE.md](GOVERNANCE.md)). These are targets, not promises:

| Step | Target |
| --- | --- |
| Acknowledge the report | 3 days |
| First assessment (valid or not, severity) | 7 days |
| Fix for critical or high severity | 30 days |
| Fix for medium or low severity | 90 days, or the next release, whichever comes first |
| Coordinated public disclosure | When a fix is out, or 90 days after the report, whichever comes first |

### The response process, step by step

1. **Receive.** A report arrives as a GitHub private vulnerability report (a draft security advisory, visible only to the reporter and the repository's admins). A report that arrives any other way, for example as a public issue, is moved into a private advisory, and the public copy is hidden or deleted with the reporter told why.
2. **Acknowledge** within 3 days: thank the reporter, confirm what was received, and ask whether and how they want to be credited.
3. **Assess** within 7 days: reproduce the problem on `main`, decide whether it is a vulnerability under the scope above, and rate it with CVSS v4.0 (critical, high, medium or low). Tell the reporter the result. A report that is not a vulnerability is closed with the reason; a bug that is not a vulnerability becomes a normal issue, with the reporter's agreement.
4. **Fix privately.** Develop the fix in a temporary private fork of the advisory, with a regression test that fails without the fix. Ask the reporter to check the fix when they can. Request a CVE through the advisory when the problem affects something users run or download (the CLI, the WASM build, the site, a release).
5. **Release.** Merge the fix through the normal pull request path ([docs/code-review.md](docs/code-review.md)); the next nightly carries it (from v0.1: a patch release). Rotate any secret the problem may have exposed, and check the logs for signs that it was used.
6. **Disclose.** Publish the GitHub security advisory when the fix is out, or 90 days after the report, whichever comes first, unless the reporter and the project agree on another date. The advisory names the affected versions, the fixed version, the CVE, workarounds and the credit.
7. **Learn.** Record what happened and what changes because of it: a `D-` entry in `DECISIONS.md` for a process change, an update to the [assurance case](docs/security/assurance-case.md) for a design change, and a note in the next [security review](docs/security/).

### Credit

Reporters are credited by name (or handle, with a link if they want one) in the published security advisory and in the release notes of the release that fixes the problem, unless they ask to stay anonymous. A reporter can change their mind about credit at any time before the advisory is published. No vulnerability report has been received yet (no security advisory exists). Weaknesses the project finds itself are listed in its security reviews in [docs/security/](docs/security/).

## Good-faith research

Research done in good faith is welcome. Do not access other people's data, do not keep data you come across, and do not degrade the service for others. Test against your own account and your own mods.
