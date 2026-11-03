# NeoForge → Forge

**Status: not started.** No rule table exists for this direction yet, it has no board column, and nothing converts.

Newer research: [docs/research/loaders.md](../research/loaders.md), read on 2026-11-03, answers many of the questions this page marks as not researched yet, with sources. This page and the loader profiles in [`loaders/`](../../loaders/) have not been updated from it yet; that happens before this direction's first rule table.

| | |
| --- | --- |
| Position in the build order | 4 of 6, together with Forge → NeoForge |
| Inverse | [Forge → NeoForge](forge-neoforge.md), order 4 |
| Round trip | NeoForge → Forge → NeoForge, scored by the harness on a NeoForge server against the original mod ([harness.md](../harness.md#the-round-trip-check)) |
| Bar for v1 | At least 50% of this direction's board column green |
| Source list | The top 1,000 NeoForge mods on Modrinth for the pinned game versions ([board.md](../board.md)) |
| Rule tables | `rules/neoforge-forge/<game-version>/`. None exist yet |
| Loader profiles | [`loaders/neoforge.toml`](../../loaders/neoforge.toml), [`loaders/forge.toml`](../../loaders/forge.toml) |
| Game versions | Both loaders are pinned for 1.20.1, 1.21.1 and 1.21.11 in [`research-lock.json`](../../research-lock.json); their APIs are only partly researched (below) |

## Known differences

For the 1.21 pins (1.21.1 and 1.21.11). Facts marked "lock" come from [`research-lock.json`](../../research-lock.json), which records versions, install steps and what each installer ships; the others come from [fabric-neoforge-notes.md](../research/fabric-neoforge-notes.md). For NeoForge, the exact mod-id pattern and whether MixinExtras ships are not researched yet. For Forge, the mod-id rule, entrypoints, registration, the event model, mixin configs, access transformers, resource conditions and networking are not researched yet. So what the conversion has to do in each area is not known yet.

| Area | NeoForge | Forge | What the conversion has to do |
| --- | --- | --- | --- |
| Manifest | `META-INF/neoforge.mods.toml`, TOML ([§7](../research/fabric-neoforge-notes.md#7-neoforgemodstoml-format)) | `META-INF/mods.toml`, TOML; `loaderVersion` is a range over the Forge major version (`[52,)`, `[61,)`); the rest of the format is not researched yet ([lock](../../research-lock.json)) | not researched yet |
| Mod id | Lowercase letters, digits and `_`, 2 to 64 characters; exact pattern not researched ([§3](../research/fabric-neoforge-notes.md#3-fabricmodjson-schema-v1-and-the-neoforge-counterpart)) | not researched yet | not researched yet |
| Entrypoints | `@Mod` class, then lifecycle events on the mod bus ([§4](../research/fabric-neoforge-notes.md#4-neoforge-mod-event-buses-lifecycle)) | not researched yet | not researched yet |
| Registration | `RegisterEvent` or `DeferredRegister`; writable only before common setup ([§5](../research/fabric-neoforge-notes.md#5-registries-deferredregister-registerevent-freezing)) | not researched yet | not researched yet |
| Events | Game bus and one mod bus per mod; priorities; `ICancellableEvent` ([§4](../research/fabric-neoforge-notes.md#4-neoforge-mod-event-buses-lifecycle)) | EventBus 6.2.33 (1.21.1) and 7.0.5 (1.21.11) ship; the event model is not researched yet ([lock](../../research-lock.json)) | not researched yet |
| Mappings | `mojang` at runtime ([§1](../research/fabric-neoforge-notes.md#1-names-at-runtime-mappings)) | `mojang` at runtime ([lock](../../research-lock.json)) | Game classes keep their names: both run `mojang`. Rules still match in Mojang names ([rules.md](../rules.md#names-inside-a-rule)) |
| Mixins | sponge-mixin 0.16.4 (1.21.1) and 0.16.5 (1.21.11); MixinExtras not confirmed; `[[mixins]]` with no side key ([§8](../research/fabric-neoforge-notes.md#8-mixin-refmaps-and-loader-differences)) | Mixin 0.8.7 ships, and MixinExtras 0.5.4 on 1.21.11; configs not researched yet ([lock](../../research-lock.json)) | not researched yet |
| Access files | Access transformer, `mojang` names ([§9](../research/fabric-neoforge-notes.md#9-access-wideners-vs-access-transformers)) | not researched yet | not researched yet |
| Resources | `neoforge:conditions`, condition id in `type` ([§10](../research/fabric-neoforge-notes.md#10-resource-conditions)) | not researched yet | not researched yet |
| Networking | `RegisterPayloadHandlersEvent` with `PayloadRegistrar`; on 1.21.11 client handlers in `RegisterClientPayloadHandlersEvent` ([§11](../research/fabric-neoforge-notes.md#11-networking-payloads)) | not researched yet | not researched yet |
| API library | Built into NeoForge; mods depend on the `neoforge` mod id ([§3](../research/fabric-neoforge-notes.md#3-fabricmodjson-schema-v1-and-the-neoforge-counterpart)) | Built into Forge; mods depend on the `forge` mod id ([lock](../../research-lock.json)) | not researched yet |

## On 1.20.1

From [`research-lock.json`](../../research-lock.json):

- **NeoForge:** NeoForge 47.1.106, the Forge 47.1 fork published as `net.neoforged:forge`. It reads `META-INF/mods.toml`, mods depend on it as `forge`, and it runs on SRG names for fields and methods (Mojang names for classes). Mixin 0.8.5 and EventBus 6.0.5 ship.
- **Forge:** Forge 47.4.28. It reads `META-INF/mods.toml` and runs on SRG names for fields and methods (Mojang names for classes). Mixin 0.8.5 and EventBus 6.2.33 ship.

Everything else about 1.20.1 is not researched yet.

## Research needed before the first rule table

- NeoForge: every `"unknown"` field in [`loaders/neoforge.toml`](../../loaders/neoforge.toml), for each pinned game version. The comment above each field names the question.
- Forge: every `"unknown"` field in [`loaders/forge.toml`](../../loaders/forge.toml), for each pinned game version. The comment above each field names the question.
- Then this table again, with a source for every cell and the conversion step for every area.

## Rule tables

None exist yet. They will live in `rules/neoforge-forge/<game-version>/`, written after this page is complete. Each lands in the same pull request as its inverse in `rules/forge-neoforge/`, or with a "not invertible" mark and a reason ([rules.md](../rules.md#inverse-pairs-and-the-round-trip)). The area names for this direction are set on this page before its first table.

## Board column

None yet. It is built in the fixed order, and this direction is not offered on the site before its column exists.
