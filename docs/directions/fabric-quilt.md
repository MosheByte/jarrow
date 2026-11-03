# Fabric → Quilt

**Status: not started.** No rule table exists for this direction yet, it has no board column, and nothing converts.

Newer research: [docs/research/loaders.md](../research/loaders.md), read on 2026-11-03, answers many of the questions this page marks as not researched yet, with sources. This page and the loader profiles in [`loaders/`](../../loaders/) have not been updated from it yet; that happens before this direction's first rule table.

| | |
| --- | --- |
| Position in the build order | 3 of 6, together with Quilt → Fabric |
| Inverse | [Quilt → Fabric](quilt-fabric.md), order 3 |
| Round trip | Fabric → Quilt → Fabric, scored by the harness on a Fabric server against the original mod ([harness.md](../harness.md#the-round-trip-check)) |
| Bar for v1 | At least 50% of this direction's board column green |
| Source list | The top 1,000 Fabric mods on Modrinth for the pinned game versions ([board.md](../board.md)) |
| Rule tables | `rules/fabric-quilt/<game-version>/`. None exist yet |
| Loader profiles | [`loaders/fabric.toml`](../../loaders/fabric.toml), [`loaders/quilt.toml`](../../loaders/quilt.toml) |
| Game versions | Both loaders are pinned for 1.20.1, 1.21.1 and 1.21.11 in [`research-lock.json`](../../research-lock.json); their APIs are only partly researched (below) |

## Known differences

For the 1.21 pins (1.21.1 and 1.21.11). Facts marked "lock" come from [`research-lock.json`](../../research-lock.json), which records versions, install steps and what each installer ships; the others come from [fabric-neoforge-notes.md](../research/fabric-neoforge-notes.md). For Quilt, the manifest, mod-id rule, entrypoints, registration, events, mixin configs, access files, resource conditions and networking are not researched yet. So what the conversion has to do in each area is not known yet.

| Area | Fabric | Quilt | What the conversion has to do |
| --- | --- | --- | --- |
| Manifest | `fabric.mod.json`, JSON, `schemaVersion` 1 ([§3](../research/fabric-neoforge-notes.md#3-fabricmodjson-schema-v1-and-the-neoforge-counterpart)) | Not researched yet. Quilt Loader 0.30.2 provides `fabricloader` 0.19.5 to Fabric mods; how far it loads them as they are is not researched yet ([lock](../../research-lock.json)) | not researched yet |
| Mod id | `^[a-z][a-z0-9-_]{1,63}$` ([§3](../research/fabric-neoforge-notes.md#3-fabricmodjson-schema-v1-and-the-neoforge-counterpart)) | not researched yet | not researched yet |
| Entrypoints | `main`, `client`, `server` and `preLaunch` keys in `fabric.mod.json` ([§2](../research/fabric-neoforge-notes.md#2-fabric-entrypoints)) | not researched yet | not researched yet |
| Registration | Direct `Registry.register` during `onInitialize`; frozen after mod init ([§5](../research/fabric-neoforge-notes.md#5-registries-deferredregister-registerevent-freezing)) | not researched yet | not researched yet |
| Events | `Event<T>` fields with `register`; ordered by phase; results by return value ([§6](../research/fabric-neoforge-notes.md#6-events-fabric-callbacks-vs-neoforge-subscribers)) | not researched yet | not researched yet |
| Mappings | `intermediary` at runtime ([§1](../research/fabric-neoforge-notes.md#1-names-at-runtime-mappings)) | `intermediary` at runtime ([lock](../../research-lock.json)) | Game classes keep their names: both run `intermediary`. Rules still match in Mojang names ([rules.md](../rules.md#names-inside-a-rule)) |
| Mixins | sponge-mixin 0.17.5 and MixinExtras 0.5.5, bundled with the loader; configs in `fabric.mod.json` with an optional `environment` ([§8](../research/fabric-neoforge-notes.md#8-mixin-refmaps-and-loader-differences)) | sponge-mixin 0.17.4 and MixinExtras 0.5.5, bundled with the loader; configs not researched yet ([lock](../../research-lock.json)) | not researched yet |
| Access files | Access widener, `intermediary` names ([§9](../research/fabric-neoforge-notes.md#9-access-wideners-vs-access-transformers)) | not researched yet | not researched yet |
| Resources | `fabric:load_conditions`, condition id in `condition` ([§10](../research/fabric-neoforge-notes.md#10-resource-conditions)) | not researched yet | not researched yet |
| Networking | `PayloadTypeRegistry` with `ServerPlayNetworking` and `ClientPlayNetworking` ([§11](../research/fabric-neoforge-notes.md#11-networking-payloads)) | not researched yet | not researched yet |
| API library | Fabric API (0.116.17+1.21.1, 0.141.6+1.21.11), a separate mod ([lock](../../research-lock.json)) | Fabric API (Modrinth lists no Quilt API release for 1.21.1 or 1.21.11) ([lock](../../research-lock.json)) | not researched yet |

## On 1.20.1

From [`research-lock.json`](../../research-lock.json):

- **Fabric:** Fabric Loader 0.20.1, the same loader as on 1.21, with Fabric API 0.92.12+1.20.1 and `intermediary` names. Fabric API's events, registry sync, resource conditions and networking on 1.20.1 are not researched yet.
- **Quilt:** Quilt Loader 0.30.2 with `intermediary` names, and Quilted Fabric API 7.7.0+0.92.2-1.20.1, which bundles the Quilt Standard Libraries 6.3.0 and Fabric API 0.92.2.

Everything else about 1.20.1 is not researched yet.

## Research needed before the first rule table

- Quilt: every `"unknown"` field in [`loaders/quilt.toml`](../../loaders/quilt.toml), for each pinned game version. The comment above each field names the question.
- Fabric: the 1.20.1 fields that depend on Fabric API 0.92.12 (events, registration, resource conditions, networking).
- Then this table again, with a source for every cell and the conversion step for every area.

## Rule tables

None exist yet. They will live in `rules/fabric-quilt/<game-version>/`, written after this page is complete. Each lands in the same pull request as its inverse in `rules/quilt-fabric/`, or with a "not invertible" mark and a reason ([rules.md](../rules.md#inverse-pairs-and-the-round-trip)). The area names for this direction are set on this page before its first table.

## Board column

None yet. It is built in the fixed order, and this direction is not offered on the site before its column exists.
