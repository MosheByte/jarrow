# Fabric → NeoForge

**Status: in progress: M1 (harness and board) next.** No rule table exists for this direction yet, and nothing converts. It has the board's only column, where every mod is red until a converter exists. Milestones M0 to M6 in [ROADMAP.md](../../ROADMAP.md) build the shared engine together with this direction.

Newer research: [docs/research/loaders.md](../research/loaders.md), read on 2026-11-03, answers many of the questions this page marks as not researched yet, with sources. This page and the loader profiles in [`loaders/`](../../loaders/) have not been updated from it yet; that happens before this direction's first rule table.

| | |
| --- | --- |
| Position in the build order | 1 of 6, the first direction |
| Inverse | [NeoForge → Fabric](neoforge-fabric.md), order 2 |
| Round trip | Fabric → NeoForge → Fabric, scored by the harness on a Fabric server against the original mod ([harness.md](../harness.md#the-round-trip-check)) |
| Bar for v1 | At least 60% of this direction's board column green |
| Source list | The top 1,000 Fabric mods on Modrinth for the pinned game versions ([board.md](../board.md)) |
| Rule tables | `rules/fabric-neoforge/<game-version>/`. None exist yet |
| Loader profiles | [`loaders/fabric.toml`](../../loaders/fabric.toml), [`loaders/neoforge.toml`](../../loaders/neoforge.toml) |
| Game versions | 1.21.1 and 1.21.11 are researched for both loaders. 1.20.1 is pinned for both in [`research-lock.json`](../../research-lock.json) and only partly researched ([below](#on-1201)) |

Everything on this page comes from the research notes, [fabric-neoforge-notes.md](../research/fabric-neoforge-notes.md), read on 2026-11-03 for Fabric Loader 0.20.1, Fabric API 0.116.17+1.21.1 and 0.141.6+1.21.11, and NeoForge 21.1.265 and 21.11.48. Section links below point into that file.

## Known differences

| Area | Fabric | NeoForge | What the conversion has to do |
| --- | --- | --- | --- |
| Manifest | `fabric.mod.json`, JSON, `schemaVersion` 1 ([§3][s3]) | `META-INF/neoforge.mods.toml`, TOML. `modLoader` and `loaderVersion` are mandatory on 1.21.1 and optional on 1.21.11; `license` is mandatory ([§7][s7]) | Translate each key by the table in [§3][s3]. Emit `modLoader="javafml"` and `loaderVersion="[1,)"` on both versions, so one code path serves both. Convert SemVer predicates to Maven ranges and flag any range that touches a pre-release. Replace `fabricloader`, `fabric-api` and `fabric-*` dependencies with `neoforge`. `contact.sources` and `custom` go under `[modproperties]`. Report what does not fit: contact objects, `provides`, nested `jars` |
| Mod id | `^[a-z][a-z0-9-_]{1,63}$` ([§3][s3]) | Lowercase letters, digits and `_`, 2 to 64 characters ([§3][s3]) | An id with `-` cannot be used as is. Rename it, rewrite every dependency on it, and report the rename |
| Sides | `environment` (`*`, `client`, `server`) for the whole mod; `@Environment` strips members on the other side ([§3][s3], [§12][s12]) | Every mod loads on both sides; `@Mod(dist = …)` limits one entry class. `@OnlyIn` exists on 1.21.1 and is gone on 1.21.11 ([§3][s3], [§12][s12]) | Map `environment` to `@Mod(dist = …)` and dependency `side`. On 1.21.1, `@Environment` becomes `@OnlyIn`. On 1.21.11 there is no counterpart, so side-only members must stay unreachable on the server, or the mod is Tier B |
| Entrypoints | `main`, `client`, `server`, `preLaunch`; `main` then `client` or `server`, on the main thread ([§2][s2]) | One public `@Mod` class per entry, constructor taking any of `IEventBus`, `ModContainer`, `FMLModContainer`, `Dist`; then the lifecycle events, many dispatched in parallel ([§4][s4]) | Generate one `@Mod` class. Run `main` inside the first `RegisterEvent` (D-023, amendment A-10). Run `client` and `server` at `FMLClientSetupEvent` and `FMLDedicatedServerSetupEvent`, through `enqueueWork` so they keep main-thread semantics. Report custom entrypoint keys as "plugin entrypoint for `<key>`, not translated". No stage is chosen for `preLaunch` yet |
| Registration | Direct `Registry.register` anywhere inside `onInitialize`; Fabric API freezes registries after mod init ([§5][s5]) | `RegisterEvent` on the mod bus, or `DeferredRegister`; registries are writable only inside the register window, before common setup ([§5][s5]) | A registration bridge: run `main` in the first `RegisterEvent` with `Registry.register` redirected to the bridge, keep order and ids, and verify with the harness registry dump. Carry a custom registry's sync attribute over to `RegistryBuilder.sync(true)` |
| Events | `Event<T>` fields with `register`; ordered by phase, then registration; results by return value ([§6][s6]) | A game bus and one mod bus per mod; ordered by priority, then registration; `ICancellableEvent` ([§4][s4], [§6][s6]) | One `event_bridge` rule per event, each with its documented differences ([rules.md](../rules.md)). Unphased listeners become `NORMAL` priority. Phased `register` is Tier B for now. Each callback that returns a value needs its own `result` mapping |
| Mappings | `intermediary` at runtime ([§1][s1]) | `mojang` at runtime ([§1][s1]) | Remap intermediary → `obf` → `mojang`, using the version's intermediary file and Mojang's ProGuard file fetched at runtime. The intermediary column named `official` holds the obfuscated names. Calls to Fabric's mapping resolver get a resolver that answers in Mojang names, or are reported |
| Mixins | sponge-mixin 0.17.5 and MixinExtras 0.5.5, bundled with the loader; configs in `fabric.mod.json` with an optional `environment` ([§8][s8]) | sponge-mixin 0.16.4 on 1.21.1 and 0.16.5 on 1.21.11; MixinExtras at runtime not confirmed; `[[mixins]]` has no side key; 1.21.11 adds `requiredMods` and `behaviorVersion` ([§8][s8], [§7][s7]) | Remap mixin targets to Mojang names, in the refmap (Loom up to 1.11) or in the annotation strings (Loom 1.12 and later). Move a client-only config's `mixins` entries into its `client` list. Report a `minVersion` or `compatibilityLevel` above what NeoForge ships |
| Access files | Access widener, header names the `intermediary` namespace ([§9][s9]) | Access transformer in Mojang names, `META-INF/accesstransformer.cfg` or `[[accessTransformers]]` ([§9][s9]) | Map each directive by the table in [§9][s9] and remap every name. An `accessible` private method becomes `public+f`. A `mutable` field that is not also `accessible` needs its current access read from the vanilla class |
| Resources | `fabric:load_conditions`, condition id in `condition`; one condition or an array ([§10][s10]) | `neoforge:conditions`, condition id in `type`; always an array ([§10][s10]) | Translate each condition by the table in [§10][s10]. The NeoForge ids differ between 1.21.1 and 1.21.11 (`true` became `always`, `item_exists` became `registered`). `fabric:features_enabled` has no built-in on 1.21.1, and `fabric:overlays` in `pack.mcmeta` has no counterpart: both are reported |
| Networking | `PayloadTypeRegistry` with `ServerPlayNetworking` and `ClientPlayNetworking` over vanilla `CustomPacketPayload` ([§11][s11]) | `RegisterPayloadHandlersEvent` with a `PayloadRegistrar`; handlers on the main thread by default; on 1.21.11 client handlers register in `RegisterClientPayloadHandlersEvent` ([§11][s11]) | Payload registration fits the register window once `main` runs early. Fabric 1.21.11's packet splitter (`registerLarge`) and recipe-sync API have no counterpart in the pinned docs and are Tier B. Calls into `net.fabricmc.fabric.impl.*` are a classifier reason |
| API library | Fabric API, a separate mod: 0.116.17+1.21.1 and 0.141.6+1.21.11 ([lock](../../research-lock.json)) | Built into NeoForge; mods depend on the `neoforge` mod id ([§3][s3]) | Rule coverage stands in for Fabric API. Calls to Fabric API symbols without a rule or a mark count toward Tier B |

## What changes between 1.21.1 and 1.21.11

The tables for the two versions are separate and never copied across ([§12][s12]):

- `ResourceLocation` is `Identifier` in Mojang names on 1.21.11, with package moves, so every Mojang-named symbol differs.
- Since 1.21.2, block and item properties carry their registry key before construction.
- Recipes became a data-pack registry in 1.21.2; recipe-viewer integrations differ.
- Item-colour registration exists in 1.21.1 mods only.
- NeoForge renamed condition ids between the two pins.
- `@OnlyIn` side stripping is gone on NeoForge 1.21.11.
- Fabric's resource-loader API v1 (Fabric 1.21.9) means 1.21.11 mods use a different Fabric API surface.

## On 1.20.1

From [`research-lock.json`](../../research-lock.json):

- **Fabric:** Fabric Loader 0.20.1, the same loader as on 1.21, with Fabric API 0.92.12+1.20.1 and `intermediary` names. So the loader-side rows above hold for 1.20.1 too: manifest, mod id, entrypoints, Mixin, access widener. Fabric API's events, registry sync, resource conditions and networking on 1.20.1 are not researched yet.
- **NeoForge:** NeoForge 47.1.106, the Forge 47.1 fork published as `net.neoforged:forge`. It reads `META-INF/mods.toml`, not `neoforge.mods.toml`, mods depend on it as `forge`, and it runs on SRG names for fields and methods (Mojang names for classes). Mixin 0.8.5 and EventBus 6.0.5 ship.

So on 1.20.1 the target side differs from the 1.21 pins in almost every row: a different manifest file and dependency id, and a remap from `intermediary` to SRG member names, which needs the MCP config mappings the installer merges (not pinned yet). Everything else about NeoForge on 1.20.1 is not researched yet, and no 1.20.1 table is written before it is.

## Questions only the harness can answer

From [§14][s14]. Each becomes a harness probe or a `needs-proof` issue:

1. Can a `RegisterEvent` handler for one registry write into a different vanilla registry?
2. Does NeoForge 21.x provide MixinExtras at runtime?
3. Is an access transformer line that narrows access applied or ignored?
4. Does Fabric's `provides` need alias ids for dependents to resolve?

## Rule tables

None exist yet. They will live in `rules/fabric-neoforge/1.21.1/` and `rules/fabric-neoforge/1.21.11/`, one file per API area, named after the Fabric API module without the `fabric-` prefix (`lifecycle-events-v1.toml`), or `loader-entrypoints.toml` and `registries.toml` ([rules.md](../rules.md#layout)). Each table lands in the same pull request as its inverse in `rules/neoforge-fabric/`, or with a "not invertible" mark and a reason.

## Board column

The nightly board (`.github/workflows/board.yml`, milestone M1 wave 1) measures this direction over the Fabric source list on 1.20.1, 1.21.1 and 1.21.11, with conversion stubbed, so every mod in it is red with the reason `no_converter` ([board.md](../board.md)). A share of green can only come from harness results after the converter exists (M2).

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
[s14]: ../research/fabric-neoforge-notes.md#14-open-questions-the-harness-must-answer
