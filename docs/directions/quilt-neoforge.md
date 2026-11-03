# Quilt → NeoForge

**Status: not started.** No rule table exists for this direction yet, it has no board column, and nothing converts.

Newer research: [docs/research/loaders.md](../research/loaders.md), read on 2026-11-03, answers many of the questions this page marks as not researched yet, with sources. This page and the loader profiles in [`loaders/`](../../loaders/) have not been updated from it yet; that happens before this direction's first rule table.

| | |
| --- | --- |
| Position in the build order | 6 of 6, together with Quilt → Forge, Forge → Quilt, NeoForge → Quilt |
| Inverse | [NeoForge → Quilt](neoforge-quilt.md), order 6 |
| Round trip | Quilt → NeoForge → Quilt, scored by the harness on a Quilt server against the original mod ([harness.md](../harness.md#the-round-trip-check)) |
| Bar for v1 | At least 50% of this direction's board column green |
| Source list | The top 1,000 Quilt mods on Modrinth for the pinned game versions ([board.md](../board.md)) |
| Rule tables | `rules/quilt-neoforge/<game-version>/`. None exist yet |
| Loader profiles | [`loaders/quilt.toml`](../../loaders/quilt.toml), [`loaders/neoforge.toml`](../../loaders/neoforge.toml) |
| Game versions | Both loaders are pinned for 1.20.1, 1.21.1 and 1.21.11 in [`research-lock.json`](../../research-lock.json); their APIs are only partly researched (below) |

## Known differences

For the 1.21 pins (1.21.1 and 1.21.11). Facts marked "lock" come from [`research-lock.json`](../../research-lock.json), which records versions, install steps and what each installer ships; the others come from [fabric-neoforge-notes.md](../research/fabric-neoforge-notes.md). For Quilt, the manifest, mod-id rule, entrypoints, registration, events, mixin configs, access files, resource conditions and networking are not researched yet. For NeoForge, the exact mod-id pattern and whether MixinExtras ships are not researched yet. So what the conversion has to do in each area is not known yet.

| Area | Quilt | NeoForge | What the conversion has to do |
| --- | --- | --- | --- |
| Manifest | Not researched yet. Quilt Loader 0.30.2 provides `fabricloader` 0.19.5 to Fabric mods; how far it loads them as they are is not researched yet ([lock](../../research-lock.json)) | `META-INF/neoforge.mods.toml`, TOML ([§7](../research/fabric-neoforge-notes.md#7-neoforgemodstoml-format)) | not researched yet |
| Mod id | not researched yet | Lowercase letters, digits and `_`, 2 to 64 characters; exact pattern not researched ([§3](../research/fabric-neoforge-notes.md#3-fabricmodjson-schema-v1-and-the-neoforge-counterpart)) | not researched yet |
| Entrypoints | not researched yet | `@Mod` class, then lifecycle events on the mod bus ([§4](../research/fabric-neoforge-notes.md#4-neoforge-mod-event-buses-lifecycle)) | not researched yet |
| Registration | not researched yet | `RegisterEvent` or `DeferredRegister`; writable only before common setup ([§5](../research/fabric-neoforge-notes.md#5-registries-deferredregister-registerevent-freezing)) | not researched yet |
| Events | not researched yet | Game bus and one mod bus per mod; priorities; `ICancellableEvent` ([§4](../research/fabric-neoforge-notes.md#4-neoforge-mod-event-buses-lifecycle)) | not researched yet |
| Mappings | `intermediary` at runtime ([lock](../../research-lock.json)) | `mojang` at runtime ([§1](../research/fabric-neoforge-notes.md#1-names-at-runtime-mappings)) | Remap intermediary → mojang ([architecture.md](../architecture.md#conversion-data-flow)) |
| Mixins | sponge-mixin 0.17.4 and MixinExtras 0.5.5, bundled with the loader; configs not researched yet ([lock](../../research-lock.json)) | sponge-mixin 0.16.4 (1.21.1) and 0.16.5 (1.21.11); MixinExtras not confirmed; `[[mixins]]` with no side key ([§8](../research/fabric-neoforge-notes.md#8-mixin-refmaps-and-loader-differences)) | not researched yet |
| Access files | not researched yet | Access transformer, `mojang` names ([§9](../research/fabric-neoforge-notes.md#9-access-wideners-vs-access-transformers)) | not researched yet |
| Resources | not researched yet | `neoforge:conditions`, condition id in `type` ([§10](../research/fabric-neoforge-notes.md#10-resource-conditions)) | not researched yet |
| Networking | not researched yet | `RegisterPayloadHandlersEvent` with `PayloadRegistrar`; on 1.21.11 client handlers in `RegisterClientPayloadHandlersEvent` ([§11](../research/fabric-neoforge-notes.md#11-networking-payloads)) | not researched yet |
| API library | Fabric API (Modrinth lists no Quilt API release for 1.21.1 or 1.21.11) ([lock](../../research-lock.json)) | Built into NeoForge; mods depend on the `neoforge` mod id ([§3](../research/fabric-neoforge-notes.md#3-fabricmodjson-schema-v1-and-the-neoforge-counterpart)) | not researched yet |

## On 1.20.1

From [`research-lock.json`](../../research-lock.json):

- **Quilt:** Quilt Loader 0.30.2 with `intermediary` names, and Quilted Fabric API 7.7.0+0.92.2-1.20.1, which bundles the Quilt Standard Libraries 6.3.0 and Fabric API 0.92.2.
- **NeoForge:** NeoForge 47.1.106, the Forge 47.1 fork published as `net.neoforged:forge`. It reads `META-INF/mods.toml`, mods depend on it as `forge`, and it runs on SRG names for fields and methods (Mojang names for classes). Mixin 0.8.5 and EventBus 6.0.5 ship.

Everything else about 1.20.1 is not researched yet.

## Research needed before the first rule table

- Quilt: every `"unknown"` field in [`loaders/quilt.toml`](../../loaders/quilt.toml), for each pinned game version. The comment above each field names the question.
- NeoForge: every `"unknown"` field in [`loaders/neoforge.toml`](../../loaders/neoforge.toml), for each pinned game version. The comment above each field names the question.
- Then this table again, with a source for every cell and the conversion step for every area.

## Rule tables

None exist yet. They will live in `rules/quilt-neoforge/<game-version>/`, written after this page is complete. Each lands in the same pull request as its inverse in `rules/neoforge-quilt/`, or with a "not invertible" mark and a reason ([rules.md](../rules.md#inverse-pairs-and-the-round-trip)). The area names for this direction are set on this page before its first table.

## Board column

None yet. It is built in the fixed order, and this direction is not offered on the site before its column exists.
