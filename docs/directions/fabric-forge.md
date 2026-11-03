# Fabric → Forge

**Status: not started.** No rule table exists for this direction yet, it has no board column, and nothing converts.

Newer research: [docs/research/loaders.md](../research/loaders.md), read on 2026-11-03, answers many of the questions this page marks as not researched yet, with sources. This page and the loader profiles in [`loaders/`](../../loaders/) have not been updated from it yet; that happens before this direction's first rule table.

| | |
| --- | --- |
| Position in the build order | 5 of 6, together with Forge → Fabric |
| Inverse | [Forge → Fabric](forge-fabric.md), order 5 |
| Round trip | Fabric → Forge → Fabric, scored by the harness on a Fabric server against the original mod ([harness.md](../harness.md#the-round-trip-check)) |
| Bar for v1 | At least 50% of this direction's board column green |
| Source list | The top 1,000 Fabric mods on Modrinth for the pinned game versions ([board.md](../board.md)) |
| Rule tables | `rules/fabric-forge/<game-version>/`. None exist yet |
| Loader profiles | [`loaders/fabric.toml`](../../loaders/fabric.toml), [`loaders/forge.toml`](../../loaders/forge.toml) |
| Game versions | Both loaders are pinned for 1.20.1, 1.21.1 and 1.21.11 in [`research-lock.json`](../../research-lock.json); their APIs are only partly researched (below) |

## Known differences

For the 1.21 pins (1.21.1 and 1.21.11). Facts marked "lock" come from [`research-lock.json`](../../research-lock.json), which records versions, install steps and what each installer ships; the others come from [fabric-neoforge-notes.md](../research/fabric-neoforge-notes.md). For Forge, the mod-id rule, entrypoints, registration, the event model, mixin configs, access transformers, resource conditions and networking are not researched yet. So what the conversion has to do in each area is not known yet.

| Area | Fabric | Forge | What the conversion has to do |
| --- | --- | --- | --- |
| Manifest | `fabric.mod.json`, JSON, `schemaVersion` 1 ([§3](../research/fabric-neoforge-notes.md#3-fabricmodjson-schema-v1-and-the-neoforge-counterpart)) | `META-INF/mods.toml`, TOML; `loaderVersion` is a range over the Forge major version (`[52,)`, `[61,)`); the rest of the format is not researched yet ([lock](../../research-lock.json)) | not researched yet |
| Mod id | `^[a-z][a-z0-9-_]{1,63}$` ([§3](../research/fabric-neoforge-notes.md#3-fabricmodjson-schema-v1-and-the-neoforge-counterpart)) | not researched yet | not researched yet |
| Entrypoints | `main`, `client`, `server` and `preLaunch` keys in `fabric.mod.json` ([§2](../research/fabric-neoforge-notes.md#2-fabric-entrypoints)) | not researched yet | not researched yet |
| Registration | Direct `Registry.register` during `onInitialize`; frozen after mod init ([§5](../research/fabric-neoforge-notes.md#5-registries-deferredregister-registerevent-freezing)) | not researched yet | not researched yet |
| Events | `Event<T>` fields with `register`; ordered by phase; results by return value ([§6](../research/fabric-neoforge-notes.md#6-events-fabric-callbacks-vs-neoforge-subscribers)) | EventBus 6.2.33 (1.21.1) and 7.0.5 (1.21.11) ship; the event model is not researched yet ([lock](../../research-lock.json)) | not researched yet |
| Mappings | `intermediary` at runtime ([§1](../research/fabric-neoforge-notes.md#1-names-at-runtime-mappings)) | `mojang` at runtime ([lock](../../research-lock.json)) | Remap intermediary → mojang ([architecture.md](../architecture.md#conversion-data-flow)) |
| Mixins | sponge-mixin 0.17.5 and MixinExtras 0.5.5, bundled with the loader; configs in `fabric.mod.json` with an optional `environment` ([§8](../research/fabric-neoforge-notes.md#8-mixin-refmaps-and-loader-differences)) | Mixin 0.8.7 ships, and MixinExtras 0.5.4 on 1.21.11; configs not researched yet ([lock](../../research-lock.json)) | not researched yet |
| Access files | Access widener, `intermediary` names ([§9](../research/fabric-neoforge-notes.md#9-access-wideners-vs-access-transformers)) | not researched yet | not researched yet |
| Resources | `fabric:load_conditions`, condition id in `condition` ([§10](../research/fabric-neoforge-notes.md#10-resource-conditions)) | not researched yet | not researched yet |
| Networking | `PayloadTypeRegistry` with `ServerPlayNetworking` and `ClientPlayNetworking` ([§11](../research/fabric-neoforge-notes.md#11-networking-payloads)) | not researched yet | not researched yet |
| API library | Fabric API (0.116.17+1.21.1, 0.141.6+1.21.11), a separate mod ([lock](../../research-lock.json)) | Built into Forge; mods depend on the `forge` mod id ([lock](../../research-lock.json)) | not researched yet |

## On 1.20.1

From [`research-lock.json`](../../research-lock.json):

- **Fabric:** Fabric Loader 0.20.1, the same loader as on 1.21, with Fabric API 0.92.12+1.20.1 and `intermediary` names. Fabric API's events, registry sync, resource conditions and networking on 1.20.1 are not researched yet.
- **Forge:** Forge 47.4.28. It reads `META-INF/mods.toml` and runs on SRG names for fields and methods (Mojang names for classes). Mixin 0.8.5 and EventBus 6.2.33 ship.

Everything else about 1.20.1 is not researched yet.

## Research needed before the first rule table

- Forge: every `"unknown"` field in [`loaders/forge.toml`](../../loaders/forge.toml), for each pinned game version. The comment above each field names the question.
- Fabric: the 1.20.1 fields that depend on Fabric API 0.92.12 (events, registration, resource conditions, networking).
- Then this table again, with a source for every cell and the conversion step for every area.

## Rule tables

None exist yet. They will live in `rules/fabric-forge/<game-version>/`, written after this page is complete. Each lands in the same pull request as its inverse in `rules/forge-fabric/`, or with a "not invertible" mark and a reason ([rules.md](../rules.md#inverse-pairs-and-the-round-trip)). The area names for this direction are set on this page before its first table.

## Board column

None yet. It is built in the fixed order, and this direction is not offered on the site before its column exists.
