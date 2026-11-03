---
title: Status
description: The current milestone and what works today.
---

**Current milestone: M0 — Ground.**

## What works today

Nothing converts yet. There is no converter, no classifier, no harness and no board run. The board on this site is empty because the first nightly has not run.

What exists in M0 is the ground the rest is built on: the public repository, its CI, this site and these docs. The milestone closes when `ci.yml` is green on Linux, macOS and Windows, the site is reachable, and every external service the project uses is classified in the inventory.

## Directions

No direction is available. The goal for v1 is any loader to any loader: Fabric, NeoForge, Forge and Quilt, all 12 directions, on Minecraft 1.20.1, 1.21.1 and 1.21.11. They are built in a fixed order. The first, Fabric → NeoForge, is in progress: the milestones below build it. The other 11 have not started, and none has a board column yet. [Directions](/docs/directions/) has the order and the bar for each.

## What comes next

The milestones build the engine and the first direction, Fabric → NeoForge.

| Milestone | Delivers |
| --- | --- |
| M1 — Measure | The parity harness and the nightly board, run against the 1,000 most-downloaded Fabric mods with conversion stubbed, so every mod is red with the reason "no converter yet" |
| M2 — Convert | The conversion engine: manifest, mappings, entrypoints, registries, resources, mixins, access wideners |
| M3 — Browser | The converter page, running in the browser |
| M4 — Deep | Tier B conversions, GitHub sign-in, author mode |
| M5 — Public | Complete docs, badges, status page, monitoring, release v0.1 |
| M6 — Done | Rule-table growth until at least 60% of the Fabric → NeoForge column is green |
| After M6 | The other 11 directions, in the fixed order, each until its own column is at least 50% green |

The [roadmap](/roadmap/) has the exit criterion of each milestone, rendered from the repository's `ROADMAP.md`. Live tracking is on the [roadmap project](https://github.com/users/MosheByte/projects/1).

## How "works" is decided

A milestone closes only when its exit criterion is green on the `main` branch, checked by CI or by the nightly run. A conversion counts as working only when the parity harness says so. Where the harness cannot measure something, these docs will say "not measured".
