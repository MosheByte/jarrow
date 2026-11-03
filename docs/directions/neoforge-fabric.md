# NeoForge → Fabric

**Status: not started.** No rule table exists for this direction yet, it has no board column, and nothing converts.

Newer research: [docs/research/loaders.md](../research/loaders.md), read on 2026-11-03, answers many of the questions this page marks as not researched yet, with sources. This page and the loader profiles in [`loaders/`](../../loaders/) have not been updated from it yet; that happens before this direction's first rule table.

| | |
| --- | --- |
| Position in the build order | 2 of 6, after Fabric → NeoForge |
| Inverse | [Fabric → NeoForge](fabric-neoforge.md), order 1 |
| Round trip | NeoForge → Fabric → NeoForge, scored by the harness on a NeoForge server against the original mod ([harness.md](../harness.md#the-round-trip-check)) |
| Bar for v1 | At least 50% of this direction's board column green |
| Source list | The top 1,000 NeoForge mods on Modrinth for the pinned game versions ([board.md](../board.md)) |
| Rule tables | `rules/neoforge-fabric/<game-version>/`. None exist yet |
| Loader profiles | [`loaders/neoforge.toml`](../../loaders/neoforge.toml), [`loaders/fabric.toml`](../../loaders/fabric.toml) |
| Game versions | 1.21.1 and 1.21.11 are researched for both loaders. 1.20.1 is pinned for both in [`research-lock.json`](../../research-lock.json) and only partly researched ([below](#on-1201)) |

The facts below come from the same research as the inverse direction, [fabric-neoforge-notes.md](../research/fabric-neoforge-notes.md), read in the other direction. That research was written for Fabric → NeoForge, so where this direction needs something it did not cover, the table says "not researched yet".

## Known differences

| Area | NeoForge | Fabric | What the conversion has to do |
| --- | --- | --- | --- |
| Manifest | `META-INF/neoforge.mods.toml`, TOML ([§7][s7]) | `fabric.mod.json`, JSON, `schemaVersion` 1 ([§3][s3]) | Translate each key by the table in [§3][s3], read backwards. Convert Maven ranges to SemVer predicates; a union becomes an array. Replace the `neoforge` dependency with `fabricloader` and the Fabric API modules the rule tables need. Which NeoForge-only keys (`enumExtensions`, `featureFlags`, `[features]`) have no Fabric slot: not researched yet |
| Mod id | Lowercase letters, digits and `_`, 2 to 64 characters ([§3][s3]) | `^[a-z][a-z0-9-_]{1,63}$` ([§3][s3]) | Every such id fits Fabric's character set. Whether NeoForge allows a digit or `_` as the first character, which Fabric does not: not researched yet |
| Sides | `@Mod(dist = …)` per entry class; `@OnlyIn` on 1.21.1 only ([§4][s4], [§12][s12]) | `environment` for the whole mod; `@Environment` ([§3][s3], [§12][s12]) | A client-only `@Mod` class becomes a `client` entrypoint. `@OnlyIn` on 1.21.1 becomes `@Environment` |
| Entrypoints | `@Mod` class, constructor taking any of `IEventBus`, `ModContainer`, `FMLModContainer`, `Dist`; then lifecycle events ([§4][s4]) | `main`, `client`, `server` entrypoints on the main thread ([§2][s2]) | Generate entrypoints that construct the `@Mod` class. The constructor's arguments need Fabric-side stand-ins, and each lifecycle event needs a point in Fabric's order. That design is not researched yet |
| Registration | `RegisterEvent` or `DeferredRegister` inside the register window ([§5][s5]) | Direct `Registry.register` during `onInitialize` ([§5][s5]) | Replay `DeferredRegister` entries and `RegisterEvent` handlers as direct registrations during `main`, in registry order. Custom registries built with `RegistryBuilder.sync(true)` need Fabric's registry sync ([§5][s5]) |
| Events | Game bus and mod bus; priorities; `ICancellableEvent` ([§4][s4]) | `Event<T>` fields; phases; return values ([§6][s6]) | One rule per NeoForge event that has a Fabric API counterpart. Which NeoForge events have one is not researched yet. Priorities can only be approximated with phases, and cancellation becomes a return value |
| Mappings | `mojang` at runtime ([§1][s1]) | `intermediary` at runtime ([§1][s1]) | Remap `mojang` → `obf` → `intermediary`, the chain of the inverse direction run backwards |
| Mixins | sponge-mixin 0.16.4 (1.21.1) and 0.16.5 (1.21.11); `[[mixins]]` with no side key ([§8][s8]) | sponge-mixin 0.17.5 and MixinExtras 0.5.5; configs in `fabric.mod.json` ([§8][s8]) | List each config in `fabric.mod.json`. Remap targets to intermediary and write a refmap or rewrite the annotation strings. Whether Fabric's newer Mixin build changes the behaviour a NeoForge mod relies on: not researched yet (1.21.11's `behaviorVersion` exists for that question) |
| Access files | Access transformer, Mojang names; may narrow access ([§9][s9]) | Access widener, intermediary names; widens only ([§9][s9]) | Map each line to `accessible`, `extendable` or `mutable` and remap the names. An AT field line has no descriptor, so it is read from the vanilla class. A line that narrows access has no access-widener form and is reported |
| Resources | `neoforge:conditions`, field `type` ([§10][s10]) | `fabric:load_conditions`, field `condition` ([§10][s10]) | Translate each condition by the table in [§10][s10], read backwards. The NeoForge ids differ between 1.21.1 and 1.21.11. NeoForge conditions without a Fabric counterpart are reported; which those are, beyond the table, is not researched yet |
| Networking | `RegisterPayloadHandlersEvent` with `PayloadRegistrar`; `HandlerThread.NETWORK` opt-out; on 1.21.11, `RegisterClientPayloadHandlersEvent` ([§11][s11]) | `PayloadTypeRegistry` with `ServerPlayNetworking` and `ClientPlayNetworking`; server handlers on the server thread ([§11][s11]) | Register payload types and receivers during `main`. Handlers that opt into the network thread have no counterpart in the research: not researched yet |
| NeoForge additions to vanilla | NeoForge patches vanilla classes; the research covers only the `MappedRegistry` patch ([§5][s5]) | None | Calls to methods NeoForge adds to vanilla classes need a rule each or are Tier B. Which methods mods call: not researched yet |
| API library | Built into NeoForge ([§3][s3]) | Fabric API, a separate mod: 0.116.17+1.21.1 and 0.141.6+1.21.11 ([lock](../../research-lock.json)) | The converted mod depends on Fabric API, and on the modules its rules use |

## What changes between 1.21.1 and 1.21.11

The same version differences as the inverse direction apply ([§12][s12]), so the tables for the two versions are separate and never copied across.

## On 1.20.1

From [`research-lock.json`](../../research-lock.json):

- **NeoForge:** NeoForge 47.1.106, the Forge 47.1 fork published as `net.neoforged:forge`. It reads `META-INF/mods.toml`, mods depend on it as `forge`, and it runs on SRG names for fields and methods (Mojang names for classes). Mixin 0.8.5 and EventBus 6.0.5 ship.
- **Fabric:** Fabric Loader 0.20.1, the same loader as on 1.21, with Fabric API 0.92.12+1.20.1 and `intermediary` names. Fabric API's events, registry sync, resource conditions and networking on 1.20.1 are not researched yet.

Everything else about NeoForge on 1.20.1 is not researched yet, and no 1.20.1 table is written before it is.

## Rule tables

None exist yet. They will live in `rules/neoforge-fabric/1.21.1/` and `rules/neoforge-fabric/1.21.11/`. The area names for this direction are set on this page before its first table. Tables written as inverses of Fabric → NeoForge tables land in the same pull request as their pair ([rules.md](../rules.md#inverse-pairs-and-the-round-trip)).

## Board column

None yet. It is built after the Fabric → NeoForge column, in the fixed order, and this direction is not offered on the site before its column exists.

[s1]: ../research/fabric-neoforge-notes.md#1-names-at-runtime-mappings
[s2]: ../research/fabric-neoforge-notes.md#2-fabric-entrypoints
[s3]: ../research/fabric-neoforge-notes.md#3-fabricmodjson-schema-v1-and-the-neoforge-counterpart
[s4]: ../research/fabric-neoforge-notes.md#4-neoforge-mod-event-buses-lifecycle
[s5]: ../research/fabric-neoforge-notes.md#5-registries-deferredregister-registerevent-freezing
[s6]: ../research/fabric-neoforge-notes.md#6-events-fabric-callbacks-vs-neoforge-subscribers
[s7]: ../research/fabric-neoforge-notes.md#7-neoforgemodstoml-format
[s8]: ../research/fabric-neoforge-notes.md#8-mixin-refmaps-and-loader-differences
[s9]: ../research/fabric-neoforge-notes.md#9-access-wideners-vs-access-transformers
[s10]: ../research/fabric-neoforge-notes.md#10-resource-conditions
[s11]: ../research/fabric-neoforge-notes.md#11-networking-payloads
[s12]: ../research/fabric-neoforge-notes.md#12-other-121x-changes-that-matter-for-conversion
