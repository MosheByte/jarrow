# Forge → Quilt

**Status: not started.** No rule table exists for this direction yet, it has no board column, and nothing converts.

Newer research: [docs/research/loaders.md](../research/loaders.md), read on 2026-11-03, answers many of the questions this page marks as not researched yet, with sources. This page and the loader profiles in [`loaders/`](../../loaders/) have not been updated from it yet; that happens before this direction's first rule table.

| | |
| --- | --- |
| Position in the build order | 6 of 6, together with Quilt → Forge, Quilt → NeoForge, NeoForge → Quilt |
| Inverse | [Quilt → Forge](quilt-forge.md), order 6 |
| Round trip | Forge → Quilt → Forge, scored by the harness on a Forge server against the original mod ([harness.md](../harness.md#the-round-trip-check)) |
| Bar for v1 | At least 50% of this direction's board column green |
| Source list | The top 1,000 Forge mods on Modrinth for the pinned game versions ([board.md](../board.md)) |
| Rule tables | `rules/forge-quilt/<game-version>/`. None exist yet |
| Loader profiles | [`loaders/forge.toml`](../../loaders/forge.toml), [`loaders/quilt.toml`](../../loaders/quilt.toml) |
| Game versions | Both loaders are pinned for 1.20.1, 1.21.1 and 1.21.11 in [`research-lock.json`](../../research-lock.json); their APIs are only partly researched (below) |

## Known differences

For the 1.21 pins (1.21.1 and 1.21.11). Facts marked "lock" come from [`research-lock.json`](../../research-lock.json), which records versions, install steps and what each installer ships; the others come from [fabric-neoforge-notes.md](../research/fabric-neoforge-notes.md). For Forge, the mod-id rule, entrypoints, registration, the event model, mixin configs, access transformers, resource conditions and networking are not researched yet. For Quilt, the manifest, mod-id rule, entrypoints, registration, events, mixin configs, access files, resource conditions and networking are not researched yet. So what the conversion has to do in each area is not known yet.

| Area | Forge | Quilt | What the conversion has to do |
| --- | --- | --- | --- |
| Manifest | `META-INF/mods.toml`, TOML; `loaderVersion` is a range over the Forge major version (`[52,)`, `[61,)`); the rest of the format is not researched yet ([lock](../../research-lock.json)) | Not researched yet. Quilt Loader 0.30.2 provides `fabricloader` 0.19.5 to Fabric mods; how far it loads them as they are is not researched yet ([lock](../../research-lock.json)) | not researched yet |
| Mod id | not researched yet | not researched yet | not researched yet |
| Entrypoints | not researched yet | not researched yet | not researched yet |
| Registration | not researched yet | not researched yet | not researched yet |
| Events | EventBus 6.2.33 (1.21.1) and 7.0.5 (1.21.11) ship; the event model is not researched yet ([lock](../../research-lock.json)) | not researched yet | not researched yet |
| Mappings | `mojang` at runtime ([lock](../../research-lock.json)) | `intermediary` at runtime ([lock](../../research-lock.json)) | Remap mojang → intermediary ([architecture.md](../architecture.md#conversion-data-flow)) |
| Mixins | Mixin 0.8.7 ships, and MixinExtras 0.5.4 on 1.21.11; configs not researched yet ([lock](../../research-lock.json)) | sponge-mixin 0.17.4 and MixinExtras 0.5.5, bundled with the loader; configs not researched yet ([lock](../../research-lock.json)) | not researched yet |
| Access files | not researched yet | not researched yet | not researched yet |
| Resources | not researched yet | not researched yet | not researched yet |
| Networking | not researched yet | not researched yet | not researched yet |
| API library | Built into Forge; mods depend on the `forge` mod id ([lock](../../research-lock.json)) | Fabric API (Modrinth lists no Quilt API release for 1.21.1 or 1.21.11) ([lock](../../research-lock.json)) | not researched yet |

## On 1.20.1

From [`research-lock.json`](../../research-lock.json):

- **Forge:** Forge 47.4.28. It reads `META-INF/mods.toml` and runs on SRG names for fields and methods (Mojang names for classes). Mixin 0.8.5 and EventBus 6.2.33 ship.
- **Quilt:** Quilt Loader 0.30.2 with `intermediary` names, and Quilted Fabric API 7.7.0+0.92.2-1.20.1, which bundles the Quilt Standard Libraries 6.3.0 and Fabric API 0.92.2.

Everything else about 1.20.1 is not researched yet.

## Research needed before the first rule table

- Forge: every `"unknown"` field in [`loaders/forge.toml`](../../loaders/forge.toml), for each pinned game version. The comment above each field names the question.
- Quilt: every `"unknown"` field in [`loaders/quilt.toml`](../../loaders/quilt.toml), for each pinned game version. The comment above each field names the question.
- Then this table again, with a source for every cell and the conversion step for every area.

## Rule tables

None exist yet. They will live in `rules/forge-quilt/<game-version>/`, written after this page is complete. Each lands in the same pull request as its inverse in `rules/quilt-forge/`, or with a "not invertible" mark and a reason ([rules.md](../rules.md#inverse-pairs-and-the-round-trip)). The area names for this direction are set on this page before its first table.

## Board column

None yet. It is built in the fixed order, and this direction is not offered on the site before its column exists.
