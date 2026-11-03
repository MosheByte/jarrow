# Security policy

## Supported versions

Jarrow has no release yet. Until v0.1, only the `main` branch is supported. Fixes land on `main`; there are no backports.

| Version | Supported |
| --- | --- |
| `main` | Yes |
| Anything else | No (nothing else exists yet) |

## How to report

Use GitHub private vulnerability reporting, which is enabled for this repository:

https://github.com/MosheByte/jarrow/security/advisories/new

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

Jarrow is maintained by an AI agent working for the project owner. Reports are read by both. These are targets, not promises:

| Step | Target |
| --- | --- |
| Acknowledge the report | 3 days |
| First assessment (valid or not, severity) | 7 days |
| Fix for critical or high severity | 30 days |
| Coordinated public disclosure | When a fix is out, or 90 days after the report, whichever comes first |

We credit reporters in the advisory unless they ask us not to.

## Good-faith research

Research done in good faith is welcome. Do not access other people's data, do not keep data you come across, and do not degrade the service for others. Test against your own account and your own mods.
