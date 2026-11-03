---
title: Directions
description: The 12 conversion directions planned for Jarrow v1, the order they are built in, the bar each must reach, and their status today. None is available yet.
---

:::note[Status]
No direction is available yet. No direction has a board column, and nothing converts.
:::

The goal for v1 is any loader to any loader: Fabric, NeoForge, Forge and Quilt, every pair in both directions, on Minecraft 1.20.1, 1.21.1 and 1.21.11. Four loaders make 12 directions.

## Build order and status

The order is fixed. Each direction gets its own column on the nightly board, and its bar is the share of that column the parity harness marks green. A direction is offered on this site only once its column exists.

| Order | Direction | Bar for v1 | Board column | Status |
| --- | --- | --- | --- | --- |
| 1 | Fabric → NeoForge | At least 60% green | Not yet | In progress: milestone M0 now, then M1 (the harness and the board) |
| 2 | NeoForge → Fabric | At least 50% green | Not yet | Not started |
| 3 | Fabric → Quilt | At least 50% green | Not yet | Not started |
| 3 | Quilt → Fabric | At least 50% green | Not yet | Not started |
| 4 | Forge → NeoForge | At least 50% green | Not yet | Not started |
| 4 | NeoForge → Forge | At least 50% green | Not yet | Not started |
| 5 | Fabric → Forge | At least 50% green | Not yet | Not started |
| 5 | Forge → Fabric | At least 50% green | Not yet | Not started |
| 6 | Quilt → Forge | At least 50% green | Not yet | Not started |
| 6 | Forge → Quilt | At least 50% green | Not yet | Not started |
| 6 | Quilt → NeoForge | At least 50% green | Not yet | Not started |
| 6 | NeoForge → Quilt | At least 50% green | Not yet | Not started |

v1 is done when all 12 columns exist and each one is at or above its bar, together with the other items in the [definition of done](/roadmap/).

## How a direction is built

- **One engine.** Every direction runs through the same Rust engine. What differs between loaders is described in one profile per loader, `loaders/<name>.toml` in the repository.
- **Rule tables in inverse pairs.** The rules for A → B come with their inverse for B → A. A mod converted from A to B and back to A is run through the harness against the original, so each half of the pair checks the other.
- **A page before the tables.** Each direction gets its own page in the repository, `docs/directions/<from>-<to>.md`, written before any of its rule tables.
- **The harness decides.** A direction's column counts only harness results, the same way the board does for every mod. Nothing is called working without one.

## The first direction: Fabric → NeoForge

The milestones M0 to M6 on the [roadmap](/roadmap/) build the engine and Fabric → NeoForge. The project is in M0, which sets up the repository, the CI and this site. M1 comes next: the parity harness and the board, run against the 1,000 most-downloaded Fabric mods with conversion stubbed, so every mod will be red with the reason "no converter yet" until the converter exists (M2).

The other 11 directions follow in the order above.

## Where this table lives

The source of this table is [docs/directions/](https://github.com/MosheByte/jarrow/tree/main/docs/directions) in the repository, and this page follows it. The [roadmap](/roadmap/) has the same order.
