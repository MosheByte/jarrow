# Directions

Jarrow's goal for v1 is any loader to any loader: Fabric, NeoForge, Forge and Quilt, every pair in both directions, on Minecraft 1.20.1, 1.21.1 and 1.21.11 (D-029). That is 12 directions. Each one has a page here, named `<from>-<to>.md` with the loader ids from [`loaders/`](../../loaders/).

**No direction converts today, and no direction has a board column.**

## What a direction page holds

- **Status**, in the words of the table below.
- **Position in the fixed build order**, and a link to its **inverse** direction.
- **The round trip** that tests the direction with its inverse: A → B → A, scored by the harness against the original mod ([harness.md](../harness.md#the-round-trip-check)).
- **The known differences** between the two loaders: manifest, mod id, entrypoints, registration, events, mappings, mixins, access files, resources and networking. Each cell has a source link, or says "not researched yet".
- **Its rule tables**, which do not exist yet for any direction.

A direction's page is written and complete before its first rule table. Rule tables live in `rules/<from>-<to>/<game-version>/` and come in inverse pairs ([rules.md](../rules.md#inverse-pairs-and-the-round-trip)). The differences on a page come from the two loader profiles ([loaders.md](../loaders.md)), which come from the research. Nothing on a page is guessed.

## The build order

The order is fixed. A direction gets its board column, its source list and its share of the work only after the steps before it. No direction is advertised on the site before its board column exists.

| Order | Directions | Bar for v1 |
| --- | --- | --- |
| 1 | Fabric → NeoForge | At least 60% of its column green |
| 2 | NeoForge → Fabric | At least 50% |
| 3 | Fabric → Quilt, Quilt → Fabric | At least 50% each |
| 4 | Forge → NeoForge, NeoForge → Forge | At least 50% each |
| 5 | Fabric → Forge, Forge → Fabric | At least 50% each |
| 6 | Quilt → Forge, Forge → Quilt, Quilt → NeoForge, NeoForge → Quilt | At least 50% each |

A column's share of green counts only harness results ([harness.md](../harness.md#scoring)). v1 is done when all 12 columns exist and each meets its bar.

## Status

| Direction | Order | Inverse | Status | Board column |
| --- | --- | --- | --- | --- |
| [Fabric → NeoForge](fabric-neoforge.md) | 1 | [NeoForge → Fabric](neoforge-fabric.md) | in progress: M1 (harness and board) next | none |
| [NeoForge → Fabric](neoforge-fabric.md) | 2 | [Fabric → NeoForge](fabric-neoforge.md) | not started | none |
| [Fabric → Quilt](fabric-quilt.md) | 3 | [Quilt → Fabric](quilt-fabric.md) | not started | none |
| [Quilt → Fabric](quilt-fabric.md) | 3 | [Fabric → Quilt](fabric-quilt.md) | not started | none |
| [Forge → NeoForge](forge-neoforge.md) | 4 | [NeoForge → Forge](neoforge-forge.md) | not started | none |
| [NeoForge → Forge](neoforge-forge.md) | 4 | [Forge → NeoForge](forge-neoforge.md) | not started | none |
| [Fabric → Forge](fabric-forge.md) | 5 | [Forge → Fabric](forge-fabric.md) | not started | none |
| [Forge → Fabric](forge-fabric.md) | 5 | [Fabric → Forge](fabric-forge.md) | not started | none |
| [Quilt → Forge](quilt-forge.md) | 6 | [Forge → Quilt](forge-quilt.md) | not started | none |
| [Forge → Quilt](forge-quilt.md) | 6 | [Quilt → Forge](quilt-forge.md) | not started | none |
| [Quilt → NeoForge](quilt-neoforge.md) | 6 | [NeoForge → Quilt](neoforge-quilt.md) | not started | none |
| [NeoForge → Quilt](neoforge-quilt.md) | 6 | [Quilt → NeoForge](quilt-neoforge.md) | not started | none |

Status words:

- **not started**: the page exists; no research beyond what the page cites, no rule table, no column.
- **in progress: _next step_**: work on the direction has begun; the next step is named.
- Once a direction has a board column, the "Board column" cell shows its share of green from the latest nightly `board.json`, with a link to the board, and nothing else. It is never typed by hand.

## Research coverage today

| Loader | 1.20.1 | 1.21.1 | 1.21.11 |
| --- | --- | --- | --- |
| Fabric | partial | researched | researched |
| NeoForge | partial | partial (two fields open) | partial (two fields open) |
| Forge | partial (lock only) | partial (lock only) | partial (lock only) |
| Quilt | partial (lock only) | partial (lock only) | partial (lock only) |

"Lock only" means [`research-lock.json`](../../research-lock.json) pins the build and records its runtime names, Mixin build, server install and API library, but the loader's mod-facing API is not researched yet ([loaders/README.md](../../loaders/README.md)). So only Fabric → NeoForge and NeoForge → Fabric have both sides researched, and only on 1.21.1 and 1.21.11. Every other page lists its open questions and points at the profile fields that need research.

These columns describe the loader profiles. [docs/research/loaders.md](../research/loaders.md), read on 2026-11-03, has since answered many of the open fields for Forge, Quilt and 1.20.1, with sources. The profiles and the direction pages are not updated from it yet, so this table still shows them as partial.
