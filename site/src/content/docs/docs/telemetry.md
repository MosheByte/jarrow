---
title: Telemetry
description: What Jarrow collects (nothing yet) and what it plans to collect.
---

**Nothing is collected today.** This site loads no analytics script, sets no cookies, and has no accounts. The only data it serves is the public board, which holds no information about visitors. Two kinds of request go elsewhere, both to GitHub: the home page shows the CI status badge, which your browser loads from github.com, and the [About](/about/) page shows the avatars of the people, agents and bots that work on Jarrow, which your browser loads from GitHub without sending a referrer.

## What is planned

When the converter exists (milestone M3), it will send anonymous product events so the project can see whether conversions work in practice. The rules:

- Anonymous by default. No IP addresses are stored.
- An opt-out toggle on the site.
- Mod ids are public data. Mod bytes, decompiled source and file contents are never sent to any analytics or error service.
- Error reports scrub stack traces of user class names before they leave the job.

## Planned events

| Event | Fields |
| --- | --- |
| `conversion_started` | mod id, tier |
| `conversion_completed` | mod id, tier, duration, success, rule-hit counts |

Session replay will be off. The canonical list of every event and field lives in [docs/telemetry.md](https://github.com/MosheByte/jarrow/blob/main/docs/telemetry.md) in the repository, and this page will follow it.
