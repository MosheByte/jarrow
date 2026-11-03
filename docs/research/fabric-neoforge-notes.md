# Fabric and NeoForge on 1.21.1 and 1.21.11: research notes

Research for the conversion engine (BUILD-BOOK section 5) and the harness (section 6.1). Read 2026-11-03. Every claim links to the page or source file it came from. Repository files are cited at a fixed commit or tag, so the links keep showing what was read. Everything is written in our own words. Nothing here comes from Sinytra Connector, Architectury or Forgified Fabric API code. Connector appears only as issue titles, which show where a known divergence bites.

The exact versions, URLs and hashes are in [`research-lock.json`](../../research-lock.json). `tools/research_lock.py --verify` re-checks them.

| | 1.21.1 | 1.21.11 |
| --- | --- | --- |
| Fabric Loader | 0.20.1 | 0.20.1 |
| Fabric API | 0.116.17+1.21.1 | 0.141.6+1.21.11 |
| NeoForge | 21.1.265 (FML 4.0.45) | 21.11.48 (FML 10.0.36) |
| Mixin shipped | Fabric: sponge-mixin 0.17.5 / NeoForge: 0.16.4 | Fabric: 0.17.5 / NeoForge: 0.16.5 |
| Java | 21 | 21 |
| Parchment | 2024.11.17 | 2025.12.20 |

Version facts come from [Fabric meta][fabric-meta-loader], [the Fabric Loader launcher JSON][loader-json], [the NeoForge maven API][nf-versions] and both NeoForge installers' `install_profile.json` and `version.json`. We opened the installers in a temp directory and deleted them afterwards. Their hashes are in the lock.

---

## 1. Names at runtime (mappings)

**Fabric uses intermediary names in production.** Outside a dev environment, Fabric Loader remaps the obfuscated game jar to intermediary and caches the result under `.fabric/remappedJars/`. Mods built for an obfuscated game are expected to ship intermediary names ([Fabric docs, Loader][fd-loader]). Intermediary gives each class, field and method an ID that stays the same across game versions ([Fabric 1.21.11 post][fb-12111]; [Fabric docs, mappings 1.21.11][fd-mappings-12111]).

**NeoForge uses Mojang's official names at runtime.** The NeoForge 20.2 announcement for 1.20.2 says SRG names were dropped for Mojang's mapping set everywhere ([NeoForge discussion #199][nf-202]). The pinned installers confirm it for both of our versions:
- **21.1.265** downloads Mojang's server mappings, merges them with NeoForm data, and renames the vanilla server with the result before patching.
- **21.11.48** does this in one `PROCESS_MINECRAFT_JAR` step, using the Mojang mappings as input.

Both installers resolve their mappings through a `DOWNLOAD_MOJMAPS` task (`install_profile.json`; installer URLs are in the lock). The book's "1.20.2+" claim (5.2) matches the 20.2 announcement. We found no first-party page that says in so many words which release first ran production on Mojang names. Jarrow only depends on the two pinned versions, and for those the installers settle it.

**The chain intermediary → official → Mojang works on both versions.** One example: in the intermediary tiny-v2 file, `net/minecraft/class_2960` maps to obfuscated `akr` on 1.21.1 and `amo` on 1.21.11. In Mojang's ProGuard file, `akr` is `net.minecraft.resources.ResourceLocation` (1.21.1) and `amo` is `net.minecraft.resources.Identifier` (1.21.11). The same chain holds for `class_7923` (`BuiltInRegistries`) and `class_8710` (`CustomPacketPayload`). We checked this by reading the pinned files in a temp directory.

**The namespace called `official` in intermediary is a trap.** It means Mojang's *obfuscated* names, not Mojang's mapped names. The header reads `tiny 2 0 official intermediary`. The `mappings` crate must use "obf" for that column and "mojang" for the ProGuard names.

**Mojang's mapping files carry their own license header.** It allows use for development and forbids redistributing the complete, unmodified file. So the mappings are fetched from the version manifest at runtime and never committed. This matches book 5.2 and section 12. We read the header in the pinned `server.txt`. Its URL and sha1 are in the lock.

**1.21.11 is the last obfuscated version.** 26.1 ships without obfuscation, so no intermediary exists after 1.21.11 ([Fabric 1.21.11 post][fb-12111]; [Fabric obfuscation post][fb-obf]; [NeoForge 2025 retrospective][nf-retro]). This has no effect on v1, but game-version translation (out of scope) would have to handle it.

**Reflection through the mapping resolver.** Fabric mods can translate names at runtime with `FabricLoader.getMappingResolver().mapClassName("intermediary", ...)` ([Fabric docs, Loader][fd-loader]). After conversion, the namespace is `mojang`. The rewrite has to give such calls a resolver that answers in Mojang names, or report them.

## 2. Fabric entrypoints

- `fabric.mod.json` → `entrypoints` has three standard keys:
  - `main` (`ModInitializer`)
  - `client` (`ClientModInitializer`, physical client only, after `main`)
  - `server` (`DedicatedServerModInitializer`, dedicated server only, after `main`)

  Other mods can define more keys, for example `fabric-datagen` ([Fabric docs, fabric.mod.json][fd-fmj]). An entry can name a class, a method (`Class::method`) or a static field ([same][fd-fmj]).
- The order is fixed in the loader. `Hooks.startClient` runs `main` and then `client`. `Hooks.startServer` runs `main` and then `server` ([Hooks.java @0.20.1][loader-hooks]).
- `preLaunch` (`PreLaunchEntrypoint`) runs from Knot before the game starts, earlier than any of those ([Knot.java @0.20.1][loader-knot]; [PreLaunchEntrypoint.java][loader-prelaunch]).
- The entrypoint interfaces are `net.fabricmc.api.ModInitializer`, `ClientModInitializer` and `DedicatedServerModInitializer`, plus `net.fabricmc.loader.api.entrypoint.PreLaunchEntrypoint` ([ModInitializer.java][loader-modinit]).
- `languageAdapters` lets a mod bring its own entrypoint language, such as Kotlin ([Fabric docs, fabric.mod.json][fd-fmj]). Book 5.10 treats custom language adapters as a classifier reason.
- Known divergence: Connector users report crashes at the entrypoint stage, for example "Could not execute entrypoint stage 'client'" ([Connector #2337][cn-2337]).

## 3. `fabric.mod.json` schema v1 and the NeoForge counterpart

Sources: [Fabric docs, fabric.mod.json][fd-fmj] and [V1ModMetadataParser @0.20.1][loader-v1parser] for Fabric; the NeoForge mods.toml pages for [1.21.1][nf-modfiles-1211] and [1.21.11][nf-modfiles-12111].

| Fabric key | Meaning | NeoForge target | Notes |
| --- | --- | --- | --- |
| `schemaVersion` | must be `1` | none | |
| `id` | `[a-z][a-z0-9-_]{1,63}` ([MetadataVerifier @0.20.1][loader-verifier]) | `[[mods]] modId` | NeoForge allows only lowercase letters, digits and underscores, 2 to 64 characters ([1.21.1][nf-modfiles-1211], [1.21.11][nf-modfiles-12111]). **A Fabric id with `-` cannot be used as is.** The manifest rule must rename it, rewrite every dependency on it, and report the change. |
| `version` | extended SemVer, otherwise a plain string | `version` | NeoForge recommends Maven-style versions. |
| `name` | display name | `displayName` | |
| `description` | text | `description` | |
| `authors`, `contributors` | arrays of strings or `{name, contact}` | `authors` (one string), `credits` | Lossy: contact objects are dropped and reported. |
| `contact.homepage` / `issues` / `sources` | URLs | `displayURL` / file-level `issueTrackerURL` / none | `sources` has no slot and goes under `[modproperties]`. |
| `license` | string or array (SPDX recommended) | file-level `license` (mandatory, one string) | An array becomes one string. The license gate (section 3) reads the original. |
| `icon` | path, or a map of sizes to paths | `logoFile` | 1.21.11 limits the characters allowed in `logoFile` ([1.21.11][nf-modfiles-12111]). |
| `environment` | `*`, `client` or `server` for the whole mod | no mod-level key | NeoForge loads every mod on both sides and expects client-only mods to do nothing on a server ([sides 1.21.1][nf-sides-1211], [sides 1.21.11][nf-sides-12111]). Map it to `@Mod(dist = …)` on the generated class and `side` on dependencies. |
| `entrypoints` | see section 2 | generated `@Mod` class | see section 4 |
| `jars` | nested jars | Jar-in-Jar `META-INF/jarjar/metadata.json` ([NeoForge JiJ][nf-jij]) | Nested Fabric mods would need converting too. Report them for now. |
| `languageAdapters` | custom adapters | none | classifier reason |
| `mixins` | list of paths or `{config, environment}` | `[[mixins]] config` | See section 8 for environment handling. |
| `accessWidener` | path | `META-INF/accesstransformer.cfg` or `[[accessTransformers]] file` | see section 7 |
| `provides` | alias ids | none found in the pinned docs | report |
| `depends` / `recommends` / `suggests` / `breaks` / `conflicts` | id → range or ranges | `[[dependencies.<id>]] type` = `required` / `optional` / (drop, or `optional`) / `incompatible` / `discouraged` | Fabric warns when a `recommends` is missing. NeoForge `optional` only checks the version when the mod is present ([1.21.11][nf-modfiles-12111]). |
| `custom` | free-form | `[modproperties.<id>]` | book 5.1 |

**Version ranges.** Fabric uses SemVer predicates such as `*`, `1.2.3`, `>=1.2`, `>1 <2`, `~1.2`, `^1.2`, `1.2.x`, or an array meaning "any of" ([Fabric docs, fabric.mod.json][fd-fmj]). NeoForge uses Maven ranges: `[1.2,)`, `(1,2)`, `[1.2.3]`, unions like `(,1.0],[1.2,)`, and `*` for any version ([Maven range spec][maven-ranges]; [1.21.11][nf-modfiles-12111]). Most predicates convert directly:

| Fabric | Maven |
| --- | --- |
| `>=A` | `[A,)` |
| `>A <B` | `(A,B)` |
| `^1.2` | `[1.2,2)` |
| `~1.2.3` | `[1.2.3,1.3)` |
| array | union |

Pre-release edges do not convert cleanly. Fabric's trailing `-` includes pre-releases, Maven orders them differently, and Fabric rewrites Minecraft snapshot versions into its own forms ([Fabric docs][fd-fmj]). The report flags any range that touches a pre-release.

**Dependency ids that disappear.** `fabricloader`, `fabric-api` and the `fabric-*` module ids have no NeoForge counterpart. The converted jar depends on `neoforge` instead. Jarrow's rule coverage stands in for Fabric API. `minecraft` keeps a range (the lock pins `[1.21.1]` / `[1.21.11]`). `java` can become `[features.<id>] javaVersion` ([1.21.11][nf-modfiles-12111]).

## 4. NeoForge: `@Mod`, event buses, lifecycle

- **Entry class.** The entry class is public and annotated `@Mod("<modid>")`. It has exactly one public constructor. That constructor can take any of `IEventBus` (the mod bus), `ModContainer`, `FMLModContainer` and `Dist`, in any order. `@Mod(value = …, dist = Dist.CLIENT)` limits a class to one physical side. One mod id can have several `@Mod` classes, for example common and client ([1.21.1][nf-modfiles-1211]; [1.21.11][nf-modfiles-12111]).
- **Two buses.** The game bus is `NeoForge.EVENT_BUS`. Each mod also gets its own mod bus, passed into its constructor. Events on the mod bus implement `IModBusEvent` ([events 1.21.1][nf-events-1211]; [events 1.21.11][nf-events-12111]).
- **`@EventBusSubscriber` changed inside 1.21.1.**
  - From 21.1.0 to 21.1.180, it registered static handlers on the game bus only. The mod bus needed `bus = Bus.MOD`.
  - In later 21.1 builds, and in 21.11, it registers on both buses and picks the right bus per event ([events 1.21.1][nf-events-1211]).

  Our 21.1.265 pin has the new behaviour. Generated code should still not depend on it: register explicitly on `modBus` and on `NeoForge.EVENT_BUS`.
- **Lifecycle order** ([events 1.21.11][nf-events-12111]):
  1. mod constructors
  2. `@EventBusSubscriber`
  3. `FMLConstructModEvent`
  4. registry events (`NewRegistryEvent`, `DataPackRegistryEvent.NewRegistry`, one `RegisterEvent` per registry)
  5. `FMLCommonSetupEvent`
  6. `FMLClientSetupEvent` or `FMLDedicatedServerSetupEvent`
  7. InterModComms
  8. `FMLLoadCompleteEvent`

  Many lifecycle events are dispatched in parallel. Work that has to run on the main thread goes through `enqueueWork` ([same][nf-events-12111]). The loader code shows the same order in both versions ([CommonModLoader 1.21.1][nf-cml-1211]; [CommonModLoader 1.21.11][nf-cml-12111]).
- **Cancellation and priority.** Events implementing `ICancellableEvent` can be cancelled, and cancelled events skip later handlers unless a handler asks for `receiveCanceled`. Priorities run from `HIGHEST` to `LOWEST` and are ignored for parallel events ([events 1.21.11][nf-events-12111]).

### Consequence for book 5.3 (entrypoints)

Book 5.3 says to call the original initializers "at the matching NeoForge lifecycle stage (`FMLCommonSetupEvent`, …)". That conflicts with the registry rules below. `onInitialize` almost always registers content, and by the time `FMLCommonSetupEvent` fires, the registries are frozen again (section 5). Setup events also run off the main thread, while Fabric runs `onInitialize` on the main thread ([Hooks.java][loader-hooks]).

What this means for the bridge:
- The registration part of `main` has to run inside the registry window.
- `client` and `server` can follow at `FMLClientSetupEvent` / `FMLDedicatedServerSetupEvent`, inside `enqueueWork` so they keep main-thread semantics.

**This needs a `D-` entry and a `docs/book-amendments.md` note from the lead.**

## 5. Registries: `DeferredRegister`, `RegisterEvent`, freezing

- **NeoForge.**
  - Registration happens through `RegisterEvent`, fired on the mod bus once per registry, or through `DeferredRegister`, which wraps that event and must be attached to the mod bus ([registries 1.21.1][nf-reg-1211]; [registries 1.21.11][nf-reg-12111]).
  - Lookups always go to the vanilla `Registry`.
  - Helper variants exist: `DeferredRegister.Blocks`, `.Items`, `.DataComponents` and `.Entities` ([registries 1.21.11][nf-reg-12111]).
  - Custom registries are created with `RegistryBuilder` and can sync integer ids with `.sync(true)` ([same][nf-reg-12111]).
- **NeoForge freezing.** `CommonModLoader.begin` constructs every mod first. It then calls `GameData.unfreezeData()`, fires all `RegisterEvent`s in registry order (`postRegisterEvents`) and calls `GameData.freezeData()`. Only after that come config loading, common setup and sided setup ([CommonModLoader 1.21.1][nf-cml-1211]; [GameData 1.21.1][nf-gamedata-1211]; [GameData 1.21.11][nf-gamedata-12111]).
  - **Registries are writable only inside that window.** NeoForge's patch to `MappedRegistry` adds `unfreeze()`, marks it internal, and points mods to the register events ([MappedRegistry patch 1.21.1][nf-mrpatch-1211]).
  - The patch also keeps the intrusive-holder map across repeated freeze and unfreeze ([same][nf-mrpatch-1211]; [1.21.11][nf-mrpatch-12111]).
  - If a `RegisterEvent` handler fails, NeoForge rolls every registry back to the vanilla snapshot and rethrows ([GameData 1.21.1][nf-gamedata-1211]).
- **Fabric freezing.** Fabric API's registry-sync module stops vanilla's bootstrap from freezing the built-in registries (`Registries.bootstrap()` is redirected to `Registries.init()`) ([BootstrapMixin @0.116.17][fapi-bootstrapmixin]). It freezes them only after mod initialisation. On a dedicated server that happens in `MainMixin`, after the entrypoints have run ([MainMixin @0.116.17][fapi-mainmixin]). So in Fabric, `Registry.register` works anywhere inside `onInitialize`.
- **What this means for the bridge (book 5.4).** Capture-and-replay is the right shape. But the replay has to happen inside `RegisterEvent`, and the capture must not need a writable registry earlier. Fabric code registers directly and often constructs blocks and items on the way. The safest design runs the original `main` entrypoint during the first `RegisterEvent` (all registries are unfrozen for the whole loop), with `Registry.register` redirected to the bridge, and verifies the order and ids with the harness registry dump.
- **Changes between 1.21.1 and 1.21.11.**
  - Since 1.21.2, block and item properties must carry their registry key (`setId`) before the object is constructed. Without it the game crashes with "Block id not set" or "Item id not set" ([Fabric 1.21.2 post][fb-1212]; [NeoForge primer 1.21.2][nf-primer-1212]).
  - 1.21.2 also renamed several registry lookup methods (`getOptional`, `getOrThrow`, `getValueOrThrow`) ([Fabric 1.21.2 post][fb-1212]).
  - In 1.21.11, game rules became registry entries ([NeoForge primer 1.21.11][nf-primer-12111]; [Fabric 1.21.11 post][fb-12111]).
  - From 1.21.5, data-pack files for a mod-created dynamic registry live in namespaced directories ([Fabric 1.21.5 post][fb-1215]).

  So the bridge and the rule tables are per game version and must not be shared.
- **Known divergence.** Connector reports a disconnect caused by a registry built with `FabricRegistryBuilder` that was not synced ([Connector #1355][cn-1355]). There is also an open request to translate item registries between loaders ([Connector #2076][cn-2076]). Custom Fabric registries need the sync attribute carried over to `RegistryBuilder.sync(true)`.

## 6. Events: Fabric callbacks vs NeoForge subscribers

- **Fabric.** An `Event<T>` holds callbacks, usually exposed as a static `EVENT` field on a callback interface, and `register(listener)` adds one ([Fabric docs, events 1.21.1][fd-events-1211]). Events have phases: `fabric:default` is the default phase, and `addPhaseOrdering(a, b)` orders two phases ([Event.java @0.116.17][fapi-event]).
- **NeoForge.** Handlers run by priority, may be cancelled, and some events carry a tri-state or result value ([events 1.21.11][nf-events-12111]).
- **Return values.** A Fabric callback such as `UseBlockCallback` returns an interaction result, and the first non-pass result wins. The NeoForge equivalent is a cancellable event with a result to set. Each event rule has to state how those map (book 5.5).
- **Changes between versions.** 1.21.2 merged the separate action-result classes into one ([Fabric 1.21.2 post][fb-1212]). The Fabric API source for 1.21.1 imports the Yarn class `net.minecraft.util.ActionResult`, while the 1.21.11 source imports Mojang's `net.minecraft.world.InteractionResult` ([UseBlockCallback @0.116.17][fapi-useblock-1211]; [UseBlockCallback @0.141.6][fapi-useblock-12111]). Both shipped jars use intermediary names, so the mapping set used in source never appears in a production jar. Rules match after the remap pass, so Minecraft symbols in rule tables are always written in Mojang names ([`docs/rules.md`](../rules.md)).
- **Present in both pinned Fabric APIs:** `ServerTickEvents`, `ServerLifecycleEvents`, `UseBlockCallback`, `AttackEntityCallback`, `PlayerBlockBreakEvents`, `ItemGroupEvents`, `FabricRegistryBuilder`, `PayloadTypeRegistry`, `ServerPlayNetworking` and `ClientPlayNetworking`. We checked this from the repository trees at both tags ([fabric @0.116.17+1.21.1][fapi-tag-1211]; [fabric @0.141.6+1.21.11][fapi-tag-12111]).

## 7. `neoforge.mods.toml` format

Sources: [mods.toml 1.21.1][nf-modfiles-1211], [mods.toml 1.21.11][nf-modfiles-12111], [MDK 1.21.1 template][mdk-1211-toml], [MDK 1.21.11 template][mdk-12111-toml]. The file lives at `META-INF/neoforge.mods.toml`. Since 20.5 it is the only mod metadata file NeoForge recognises ([NeoForge 20.5 post][nf-205]).

**File level:**

| Key | 1.21.1 | 1.21.11 |
| --- | --- | --- |
| `modLoader` | **mandatory**; `javafml` (or `lowcodefml`) | optional, defaults to `javafml`. `lowcodefml` was removed ([NeoForge 2025 retrospective][nf-retro]). |
| `loaderVersion` | **mandatory**; a Maven range over the language-loader version, currently `1`, so `[1,)` | optional, defaults to any version |

- The MDKs show the switch: the 1.21.1 template still has `modLoader` and `loaderVersion="${loader_version_range}"` set to `[1,)`, and the 1.21.11 template drops both ([MDK 1.21.1 properties][mdk-1211-props]; [MDK 1.21.11 properties][mdk-12111-props]).
- **Recommendation:** emit `modLoader="javafml"` and `loaderVersion="[1,)"` for both versions. Both are valid on both, and one code path stays deterministic.
- `license` is mandatory.
- Optional file-level keys: `showAsResourcePack`, `showAsDataPack`, `services`, `properties`, `issueTrackerURL`.

**`[[mods]]`:**
- `modId` is mandatory.
- The other keys: `namespace` (unused), `version`, `displayName`, `description`, `logoFile`, `logoBlur`, `updateJSONURL`, `modUrl`, `credits`, `authors`, `displayURL`, `enumExtensions`.
- `featureFlags` is documented only for 1.21.11.

**`[[dependencies.<modid>]]`:**

| Key | Values (default) |
| --- | --- |
| `modId` | mandatory |
| `type` | `required` (default), `optional`, `incompatible`, `discouraged` |
| `reason` | free text |
| `versionRange` | Maven range (default `*`) |
| `ordering` | `NONE` (default), `BEFORE`, `AFTER` |
| `side` | `BOTH` (default), `CLIENT`, `SERVER` |
| `referralUrl` | URL |

A `BEFORE`/`AFTER` cycle between two mods crashes the game.

**`[[mixins]]`:**

| Key | 1.21.1 | 1.21.11 |
| --- | --- | --- |
| `config` | yes | yes |
| `requiredMods` | no | yes |
| `behaviorVersion` | no | yes (a Fabric Mixin version whose behaviour to match) |

**`[[accessTransformers]]`:**
- `file` is mandatory once the block is present.
- Without the block, NeoForge looks for `META-INF/accesstransformer.cfg`.
- Several AT files apply in the order listed ([AT docs 1.21.11][nf-at-12111]).

**Feature and property tables:**
- `[features.<modid>]` has `javaVersion`. `openGLVersion` is documented for 1.21.1 only.
- `[modproperties.<modid>]` holds arbitrary key/values.

**Ranges written into converted jars** (all recorded in the lock):
- `neoforge`: lower bound is the pinned build, `[21.1.265,)` and `[21.11.48,)`, because the harness proved it on that build.
- `minecraft`: the exact version, as the MDK does: `[1.21.1]` and `[1.21.11]` ([MDK properties][mdk-1211-props]).

## 8. Mixin: refmaps and loader differences

- **Both loaders run Fabric's fork of Mixin.**
  - Fabric Loader 0.20.1 bundles `net.fabricmc:sponge-mixin:0.17.5+mixin.0.8.7` and MixinExtras ([loader launcher JSON][loader-json]).
  - NeoForge 21.1.265 ships 0.16.4 and 21.11.48 ships 0.16.5 (installer `version.json`).

  So a Fabric mod can rely on Mixin behaviour that is newer than what NeoForge provides. The converter should read each config's `minVersion` and `compatibilityLevel` and report anything above what NeoForge ships. 1.21.11's `behaviorVersion` exists to pin that behaviour ([mods.toml 1.21.11][nf-modfiles-12111]). Fabric Loader 0.17.0 already moved to Fabric Mixin 0.16.3 and a new MixinExtras release ([Fabric 1.21.9 post][fb-1219]). The post prints the MixinExtras version as "5.0.0", but the artifact versions are 0.5.x: the 0.20.1 launcher JSON lists `mixinextras-fabric:0.5.5` ([loader launcher JSON][loader-json]).
- **Config file keys:**

  | Key | Meaning |
  | --- | --- |
  | `package` | parent package of the mixin classes |
  | `mixins` | applied on both sides |
  | `client` | applied on the client only |
  | `server` | applied on the dedicated server only |
  | `refmap` | name of the reference map file |
  | `priority`, `plugin`, `required`, `minVersion` | ordering, companion plugin, failure handling, minimum Mixin version |

  ([Mixin wiki, the Mixin environment][mixin-env])
- **Declaring configs.**
  - Fabric lists configs in `fabric.mod.json`, each with an optional `environment` ([Fabric docs, fabric.mod.json][fd-fmj]).
  - NeoForge lists them in `[[mixins]]`, which has no side key ([mods.toml 1.21.11][nf-modfiles-12111]).

  A Fabric config marked `environment: client` therefore has to be rewritten: move its `mixins` entries into its `client` list, so it stays inert on a dedicated server.
- **Refmaps: two kinds of Fabric jar exist.**

  | Fabric Loom | Default | What the jar contains |
  | --- | --- | --- |
  | up to 1.11 | legacy annotation processor (`useLegacyMixinAp = true`) | a refmap JSON that maps the dev names in the mixin annotations to intermediary |
  | 1.12 and later | no annotation processor (`false`) | annotation strings rewritten to intermediary directly, no refmap needed |

  ([Loom dev/1.11][loom-111]; [Loom dev/1.12][loom-112]; [Loom dev/1.7][loom-17]). The board set for 1.21.1 and 1.21.11 will contain both kinds.

  NeoForge runs on Mojang names. For refmap jars, remap the refmap values to Mojang names (or regenerate the refmap, as book 5.6 says). For no-refmap jars, remap the annotation strings themselves. Either way the `@At` targets end up in Mojang names.
- **Known divergence.** Connector issues show injection failures where the log names the refmap in use ([Connector #1324][cn-1324]; [Connector #2294][cn-2294]).

## 9. Access wideners vs access transformers

| | Fabric access widener | NeoForge access transformer |
| --- | --- | --- |
| Header | `accessWidener v1\|v2 <namespace>`. The namespace must match the runtime namespace, which is `intermediary` in production ([AccessWidenerReader][aw-reader]). | none |
| Directives | `accessible`, `extendable`, `mutable`; in v2, `transitive-` prefixed forms ([AccessWidenerReader][aw-reader]; [Fabric docs, access widening 1.21.11][fd-aw-12111]) | access modifier `public` / `protected` / `default` / `private`, with optional `+f` or `-f` ([FMLAT spec][fmlat]; [AT docs 1.21.11][nf-at-12111]) |
| Class | `accessible class net/minecraft/Foo` (internal name) | `public net.minecraft.Foo` (dotted; inner classes use `$`) |
| Method | `… method <owner> <name> <descriptor>` | `public net.minecraft.Foo name(descriptor)` |
| Field | `… field <owner> <name> <descriptor>` (descriptor required) | `public-f net.minecraft.Foo name` (no descriptor) |
| Scope | Minecraft classes only ([Fabric docs, access widening][fd-aw-12111]) | classes outside the mod's control ([AT docs 1.21.11][nf-at-12111]) |

How each AW directive maps to an AT line:

| Fabric AW | NeoForge AT |
| --- | --- |
| `accessible` class | `public` |
| `accessible` field | `public` |
| `accessible` method | `public`. AW also makes an originally private method final ([Fabric docs][fd-aw-12111]), so emit `public+f` for those to keep dispatch the same. |
| `extendable` class | `public-f` |
| `extendable` method | `protected-f` |
| `mutable` field | `-f` on an access modifier. An AT line always needs a modifier, so a `mutable` field that is not also `accessible` needs its current access read from the vanilla class. |

All names go from intermediary to Mojang (book 5.7). NeoForge warns that an AT changes only the member it names, not overrides of it ([AT docs 1.21.11][nf-at-12111]).

Known divergence: an access widener does not apply to fields that the loader itself patched ([Connector #1894][cn-1894]). The harness should cover AW targets that sit in classes NeoForge patches.

## 10. Resource conditions

| | Fabric | NeoForge |
| --- | --- | --- |
| Key at the JSON root | `fabric:load_conditions` | `neoforge:conditions` |
| Value | one condition **or** an array, where an array means all must pass | array, all must pass |
| Condition id field | `condition` | `type` |
| Non-object payloads | n/a | 1.21.11 documents a `neoforge:value` wrapper |

Sources: [ResourceConditions @0.116.17][fapi-rc-1211]; [ResourceCondition codec][fapi-rcodec-1211]; [NeoForge conditions 1.21.1][nf-cond-1211]; [NeoForge conditions 1.21.11][nf-cond-12111]. The same Fabric ids exist in both pinned Fabric APIs ([DefaultResourceConditionTypes @0.116.17][fapi-drct-1211]; [@0.141.6][fapi-drct-12111]).

| Fabric (both versions) | NeoForge 1.21.1 | NeoForge 1.21.11 |
| --- | --- | --- |
| `fabric:true` | `neoforge:true` | `neoforge:always` |
| `fabric:not` (`value`) | `neoforge:not` (`value`) | same |
| `fabric:and` / `fabric:or` (`values`) | `neoforge:and` / `neoforge:or` (`values`) | same |
| `fabric:all_mods_loaded` (`values`) | `and` of `neoforge:mod_loaded` (`modid`) | same |
| `fabric:any_mods_loaded` (`values`) | `or` of `neoforge:mod_loaded` | same |
| `fabric:tags_populated` (`registry` default item, `values`) | `and` of `not` + `neoforge:tag_empty` (`tag`, items only) | `and` of `not` + `neoforge:tag_empty` (`registry`, `tag`) |
| `fabric:registry_contains` (`registry` default item, `values`) | `neoforge:item_exists` (`item`) for items only; other registries: report | `and` of `neoforge:registered` (`registry`, `value`) |
| `fabric:features_enabled` (`features`) | no built-in: report / Tier B | `neoforge:feature_flags_enabled` (`flags`) |

Fabric also reads a `fabric:overlays` key in `pack.mcmeta` for conditional overlays ([ResourceConditions @0.116.17][fapi-rc-1211]). We found no NeoForge counterpart in the pinned docs, so it is reported.

Pack formats differ between the two versions: data 48 and resource 34 on 1.21.1; data 94.1 and resource 75 on 1.21.11 ([misode/mcmeta summary][mcmeta]). They never change within one conversion, because conversion stays on one game version.

## 11. Networking payloads

- **Same base in both loaders.** Both build on vanilla `CustomPacketPayload` with a `Type` and a `StreamCodec`, the model since 1.20.5 ([NeoForge 20.5 post][nf-205]).
  - On Fabric, payload types are registered in `PayloadTypeRegistry` and receivers through `ServerPlayNetworking` / `ClientPlayNetworking`. Server handlers run on the server thread ([ServerPlayNetworking @0.116.17][fapi-spn-1211]).
  - On NeoForge, payloads are registered in `RegisterPayloadHandlersEvent` through a `PayloadRegistrar` (`playToClient`, `playToServer`, `playBidirectional`). Handlers run on the main thread by default, and `HandlerThread.NETWORK` opts out ([NeoForge 20.5 post][nf-205]; [payload docs 1.21.1][nf-payload-1211]).
- **1.21.11 splits client handlers out.** Client handlers are registered in a separate `RegisterClientPayloadHandlersEvent`, and the docs no longer use `DirectionalPayloadHandler` for bidirectional payloads ([payload docs 1.21.11][nf-payload-12111]; [NeoForge source tree 1.21.11][nf-tree-12111]). The 2025 retrospective dates this event to the 1.21.6 cycle ([NeoForge 2025 retrospective][nf-retro]).
- **Registration closes early on NeoForge.** NeoForge locks its network registry at the end of mod loading (`NetworkRegistry::setup` in `CommonModLoader.finish`) ([CommonModLoader 1.21.1][nf-cml-1211]). Fabric mods register payload types in `onInitialize`, which fits inside that window once section 4's bridge runs `main` early.
- **Fabric 1.21.11 adds two opt-in APIs** with no NeoForge equivalent in the pinned docs ([Fabric 1.21.11 post][fb-12111]):
  - a packet splitter (`registerLarge`)
  - a recipe-sync API

  Rule tables should mark both Tier B.
- **Known divergence.** A mod that reaches into Fabric API's internal `impl.networking` classes breaks under a shim ([Connector #2287][cn-2287]). Calls into `net.fabricmc.fabric.impl.*` are a classifier reason (book 5.10).

## 12. Other 1.21.x changes that matter for conversion

Each conversion stays on one game version, so these do not need translating. They do decide which rule tables apply to which version, and they explain why the tables live under `rules/fabric-neoforge/<game-version>/`.

| Change | Since | Effect on Jarrow | Source |
| --- | --- | --- | --- |
| `ResourceLocation` renamed `Identifier` in Mojang names, plus package moves (`util`, `advancements.criterion`, entity and model subpackages) | 1.21.11 | Every Mojang-named symbol in a 1.21.11 rule table differs from 1.21.1. Tables are never shared between versions. | [primer 1.21.11][nf-primer-12111]; [NeoForge 2025 retrospective][nf-retro] |
| Block and item properties must carry their id before construction | 1.21.2 | The registration bridge cannot assign ids late on 1.21.11. Mods already set them. | [Fabric 1.21.2 post][fb-1212]; [primer 1.21.2][nf-primer-1212] |
| Recipes became a data-pack registry; the server no longer sends full recipe data | 1.21.2 | Recipe-viewer integrations differ. NeoForge 1.21.4 added an opt-in recipe sync; Fabric 1.21.11 added its own. | [primer 1.21.2][nf-primer-1212]; [NeoForge 2025 retrospective][nf-retro]; [Fabric 1.21.11 post][fb-12111] |
| Client item definitions in `assets/<ns>/items/`; vanilla `ItemColors` tinting moved into those files | 1.21.4 | Resource copying is unchanged. Item-colour registration calls exist in 1.21.1 mods only, so their rules exist only in the 1.21.1 tables. | [Fabric 1.21.4 post][fb-1214]; [primer 1.21.4][nf-primer-1214] |
| NBT getters return `Optional`; data components can be read through `DataComponentGetter` | 1.21.5 | Vanilla API shape, the same on both loaders, no rule needed. Matters only for the Tier B agent's context. | [Fabric 1.21.5 post][fb-1215]; [primer 1.21.5][nf-primer-1215] |
| New data components (for example `USE_EFFECTS`) | 1.21.11 | The harness item and component dumps grow. No rule effect. | [primer 1.21.11][nf-primer-12111] |
| Fabric resource-loader API v1 (`ResourceLoader.registerReloader`) | Fabric 1.21.9 | 1.21.11 mods use a different Fabric API surface than 1.21.1 mods, so the API index (5.9) is per version. | [Fabric 1.21.9 post][fb-1219] |
| NeoForge condition ids renamed (`true`/`false` → `always`/`never`, `item_exists` → `registered`) | between the two pins | Condition rule tables are per version (section 10). | [conditions 1.21.1][nf-cond-1211]; [conditions 1.21.11][nf-cond-12111] |
| `@OnlyIn` side-stripping removed (1.21.6 cycle); FML no longer built on ModLauncher | NeoForge 1.21.6 to 1.21.11 | Fabric Loader strips members marked `@Environment` for the other side ([Environment.java][loader-env]; [EnvironmentStrippingData.java][loader-envstrip]). On 1.21.1 that maps to NeoForge's `@OnlyIn`. On 1.21.11 there is nothing to map to, so side-only members must stay unreachable on the server, or the mod is Tier B. The 21.11.48 installer's `version.json` main class is `net.neoforged.fml.startup.Client`, not BootstrapLauncher. | [NeoForge 2025 retrospective][nf-retro] |

## 13. Harness provisioning (section 6.1)

- **Fabric server:**
  - The meta endpoint `…/v2/versions/loader/<game>/<loader>/<installer>/server/jar` serves a launcher. It is the Fabric installer jar plus an `install.properties` naming the game and loader versions. It is generated on request, so it has no publisher hash. Its sha256 was identical across repeated fetches on 2026-11-03 and is pinned as observed.
  - On first start it downloads the vanilla server and checks its sha1 against Mojang's version JSON. The system property `fabric.installer.server.gameJar` points it at a server jar we already verified ([ServerLauncher.java][inst-launcher]; [MinecraftServerDownloader.java][inst-downloader]).
  - Fallback: the hash-pinned installer's server mode, `server -mcversion <v> -loader <v> -downloadMinecraft` ([ServerHandler.java][inst-serverhandler]; [Handler.java][inst-handler]).
- **NeoForge server:**
  - Install with `java -jar neoforge-<v>-installer.jar --installServer`, then start with `run.sh` or `run.bat`. `user_jvm_args.txt` holds the JVM flags ([NeoForge server guide][nf-userserver]).
  - The 21.11.48 install profile has a single `PROCESS_MINECRAFT_JAR` step. 21.1.265 runs separate extract, merge, rename and patch processors (installer `install_profile.json`).
- **Client-only mods.** A Fabric mod marked `environment: client` is never loaded by a Fabric dedicated server. After conversion, the NeoForge server sees the jar, but its client `@Mod` class is not constructed. Both dumps should therefore show no entries from that mod. The harness compares them on that basis, not as a failure.

## 14. Open questions the harness must answer

These are not settled by documentation. Each one becomes a harness probe or a `needs-proof` issue.

1. Does NeoForge accept direct writes to vanilla registries from inside one registry's `RegisterEvent` into a *different* registry? All registries are unfrozen for the whole loop, but nothing documents this as supported (section 5).
2. Does NeoForge 21.x provide MixinExtras at runtime? The installer library list names no MixinExtras artifact, so it may be jar-in-jar inside NeoForge. Unverified.
3. Does an AT line that would *narrow* access (for a `mutable`-only AW entry) get applied or ignored? The spec allows `private` and does not say.
4. Does Fabric's `provides` need emulating (alias ids) for dependents to resolve after conversion?

---

### Sources (all read 2026-11-03)

[fabric-meta-loader]: https://meta.fabricmc.net/v2/versions/loader
[loader-json]: https://maven.fabricmc.net/net/fabricmc/fabric-loader/0.20.1/fabric-loader-0.20.1.json
[nf-versions]: https://maven.neoforged.net/api/maven/versions/releases/net/neoforged/neoforge
[fd-loader]: https://docs.fabricmc.net/develop/loader/
[fd-fmj]: https://docs.fabricmc.net/develop/loader/fabric-mod-json
[fd-mappings-12111]: https://docs.fabricmc.net/1.21.11/develop/porting/mappings/
[fd-events-1211]: https://docs.fabricmc.net/1.21.1/develop/events
[fd-aw-12111]: https://docs.fabricmc.net/1.21.11/develop/class-tweakers/access-widening
[fb-obf]: https://fabricmc.net/2025/10/31/obfuscation.html
[fb-1212]: https://fabricmc.net/2024/10/14/1212.html
[fb-1214]: https://fabricmc.net/2024/12/02/1214.html
[fb-1215]: https://fabricmc.net/2025/03/24/1215.html
[fb-1219]: https://fabricmc.net/2025/09/23/1219.html
[fb-12111]: https://fabricmc.net/2025/12/05/12111.html
[loader-hooks]: https://github.com/FabricMC/fabric-loader/blob/0.20.1/minecraft/src/main/java/net/fabricmc/loader/impl/game/minecraft/Hooks.java
[loader-knot]: https://github.com/FabricMC/fabric-loader/blob/0.20.1/src/main/java/net/fabricmc/loader/impl/launch/knot/Knot.java
[loader-prelaunch]: https://github.com/FabricMC/fabric-loader/blob/0.20.1/src/main/java/net/fabricmc/loader/api/entrypoint/PreLaunchEntrypoint.java
[loader-modinit]: https://github.com/FabricMC/fabric-loader/blob/0.20.1/src/main/java/net/fabricmc/api/ModInitializer.java
[loader-v1parser]: https://github.com/FabricMC/fabric-loader/blob/0.20.1/src/main/java/net/fabricmc/loader/impl/metadata/V1ModMetadataParser.java
[loader-verifier]: https://github.com/FabricMC/fabric-loader/blob/0.20.1/src/main/java/net/fabricmc/loader/impl/metadata/MetadataVerifier.java
[loader-env]: https://github.com/FabricMC/fabric-loader/blob/0.20.1/src/main/java/net/fabricmc/api/Environment.java
[loader-envstrip]: https://github.com/FabricMC/fabric-loader/blob/0.20.1/src/main/java/net/fabricmc/loader/impl/transformer/EnvironmentStrippingData.java
[aw-reader]: https://github.com/FabricMC/access-widener/blob/30e2d0038d2307140f122eebc2f782f440250234/src/main/java/net/fabricmc/accesswidener/AccessWidenerReader.java
[loom-17]: https://github.com/FabricMC/fabric-loom/blob/dev/1.7/src/main/java/net/fabricmc/loom/extension/MixinExtensionApiImpl.java
[loom-111]: https://github.com/FabricMC/fabric-loom/blob/dev/1.11/src/main/java/net/fabricmc/loom/extension/MixinExtensionApiImpl.java
[loom-112]: https://github.com/FabricMC/fabric-loom/blob/dev/1.12/src/main/java/net/fabricmc/loom/extension/MixinExtensionApiImpl.java
[fapi-tag-1211]: https://github.com/FabricMC/fabric/tree/83c07162d88a703d15c8dae50cbbc12f4e9e63c3
[fapi-tag-12111]: https://github.com/FabricMC/fabric/tree/28e4019ae9dc6713d6797da19f60386debb25460
[fapi-bootstrapmixin]: https://github.com/FabricMC/fabric/blob/83c07162d88a703d15c8dae50cbbc12f4e9e63c3/fabric-registry-sync-v0/src/main/java/net/fabricmc/fabric/mixin/registry/sync/BootstrapMixin.java
[fapi-mainmixin]: https://github.com/FabricMC/fabric/blob/83c07162d88a703d15c8dae50cbbc12f4e9e63c3/fabric-registry-sync-v0/src/main/java/net/fabricmc/fabric/mixin/registry/sync/MainMixin.java
[fapi-event]: https://github.com/FabricMC/fabric/blob/83c07162d88a703d15c8dae50cbbc12f4e9e63c3/fabric-api-base/src/main/java/net/fabricmc/fabric/api/event/Event.java
[fapi-useblock-1211]: https://github.com/FabricMC/fabric/blob/83c07162d88a703d15c8dae50cbbc12f4e9e63c3/fabric-events-interaction-v0/src/main/java/net/fabricmc/fabric/api/event/player/UseBlockCallback.java
[fapi-useblock-12111]: https://github.com/FabricMC/fabric/blob/28e4019ae9dc6713d6797da19f60386debb25460/fabric-events-interaction-v0/src/main/java/net/fabricmc/fabric/api/event/player/UseBlockCallback.java
[fapi-rc-1211]: https://github.com/FabricMC/fabric/blob/83c07162d88a703d15c8dae50cbbc12f4e9e63c3/fabric-resource-conditions-api-v1/src/main/java/net/fabricmc/fabric/api/resource/conditions/v1/ResourceConditions.java
[fapi-rcodec-1211]: https://github.com/FabricMC/fabric/blob/83c07162d88a703d15c8dae50cbbc12f4e9e63c3/fabric-resource-conditions-api-v1/src/main/java/net/fabricmc/fabric/api/resource/conditions/v1/ResourceCondition.java
[fapi-drct-1211]: https://github.com/FabricMC/fabric/blob/83c07162d88a703d15c8dae50cbbc12f4e9e63c3/fabric-resource-conditions-api-v1/src/main/java/net/fabricmc/fabric/impl/resource/conditions/DefaultResourceConditionTypes.java
[fapi-drct-12111]: https://github.com/FabricMC/fabric/blob/28e4019ae9dc6713d6797da19f60386debb25460/fabric-resource-conditions-api-v1/src/main/java/net/fabricmc/fabric/impl/resource/conditions/DefaultResourceConditionTypes.java
[fapi-spn-1211]: https://github.com/FabricMC/fabric/blob/83c07162d88a703d15c8dae50cbbc12f4e9e63c3/fabric-networking-api-v1/src/main/java/net/fabricmc/fabric/api/networking/v1/ServerPlayNetworking.java
[inst-launcher]: https://github.com/FabricMC/fabric-installer/blob/6e7d1acd6a951a19a323060ac955f5e7bc51e3ce/src/main/java/net/fabricmc/installer/ServerLauncher.java
[inst-downloader]: https://github.com/FabricMC/fabric-installer/blob/6e7d1acd6a951a19a323060ac955f5e7bc51e3ce/src/main/java/net/fabricmc/installer/server/MinecraftServerDownloader.java
[inst-serverhandler]: https://github.com/FabricMC/fabric-installer/blob/6e7d1acd6a951a19a323060ac955f5e7bc51e3ce/src/main/java/net/fabricmc/installer/server/ServerHandler.java
[inst-handler]: https://github.com/FabricMC/fabric-installer/blob/6e7d1acd6a951a19a323060ac955f5e7bc51e3ce/src/main/java/net/fabricmc/installer/Handler.java
[nf-202]: https://github.com/neoforged/NeoForge/discussions/199
[nf-205]: https://neoforged.net/news/20.5release/
[nf-retro]: https://neoforged.net/news/2025-retrospection/
[nf-modfiles-1211]: https://docs.neoforged.net/docs/1.21.1/gettingstarted/modfiles/
[nf-modfiles-12111]: https://docs.neoforged.net/docs/1.21.11/gettingstarted/modfiles/
[nf-sides-1211]: https://docs.neoforged.net/docs/1.21.1/concepts/sides/
[nf-sides-12111]: https://docs.neoforged.net/docs/1.21.11/concepts/sides/
[nf-events-1211]: https://docs.neoforged.net/docs/1.21.1/concepts/events/
[nf-events-12111]: https://docs.neoforged.net/docs/1.21.11/concepts/events/
[nf-reg-1211]: https://docs.neoforged.net/docs/1.21.1/concepts/registries/
[nf-reg-12111]: https://docs.neoforged.net/docs/1.21.11/concepts/registries/
[nf-at-12111]: https://docs.neoforged.net/docs/1.21.11/advanced/accesstransformers/
[nf-cond-1211]: https://docs.neoforged.net/docs/1.21.1/resources/server/conditions/
[nf-cond-12111]: https://docs.neoforged.net/docs/1.21.11/resources/server/conditions/
[nf-payload-1211]: https://docs.neoforged.net/docs/1.21.1/networking/payload/
[nf-payload-12111]: https://docs.neoforged.net/docs/1.21.11/networking/payload/
[nf-jij]: https://docs.neoforged.net/toolchain/docs/dependencies/jarinjar
[nf-userserver]: https://docs.neoforged.net/user/docs/server
[nf-primer-1212]: https://docs.neoforged.net/primer/docs/1.21.2/
[nf-primer-1214]: https://docs.neoforged.net/primer/docs/1.21.4/
[nf-primer-1215]: https://docs.neoforged.net/primer/docs/1.21.5/
[nf-primer-12111]: https://docs.neoforged.net/primer/docs/1.21.11/
[nf-cml-1211]: https://github.com/neoforged/NeoForge/blob/624c929f49920c250166e2a8df7af87f16d220ef/src/main/java/net/neoforged/neoforge/internal/CommonModLoader.java
[nf-cml-12111]: https://github.com/neoforged/NeoForge/blob/0753a6a7dd9ed0bafc5441142206f33618f031b3/src/main/java/net/neoforged/neoforge/internal/CommonModLoader.java
[nf-gamedata-1211]: https://github.com/neoforged/NeoForge/blob/624c929f49920c250166e2a8df7af87f16d220ef/src/main/java/net/neoforged/neoforge/registries/GameData.java
[nf-gamedata-12111]: https://github.com/neoforged/NeoForge/blob/0753a6a7dd9ed0bafc5441142206f33618f031b3/src/main/java/net/neoforged/neoforge/registries/GameData.java
[nf-mrpatch-1211]: https://github.com/neoforged/NeoForge/blob/624c929f49920c250166e2a8df7af87f16d220ef/patches/net/minecraft/core/MappedRegistry.java.patch
[nf-mrpatch-12111]: https://github.com/neoforged/NeoForge/blob/0753a6a7dd9ed0bafc5441142206f33618f031b3/patches/net/minecraft/core/MappedRegistry.java.patch
[nf-tree-12111]: https://github.com/neoforged/NeoForge/tree/0753a6a7dd9ed0bafc5441142206f33618f031b3/src/client/java/net/neoforged/neoforge/client/network/event
[mdk-1211-props]: https://github.com/NeoForgeMDKs/MDK-1.21.1-ModDevGradle/blob/e93d7f09d4eec12c76d1d7be9bf4eaad0246e9e0/gradle.properties
[mdk-12111-props]: https://github.com/NeoForgeMDKs/MDK-1.21.11-ModDevGradle/blob/c3fd175261fd672799dca21ef03387bfdc4eba44/gradle.properties
[mdk-1211-toml]: https://github.com/NeoForgeMDKs/MDK-1.21.1-ModDevGradle/blob/e93d7f09d4eec12c76d1d7be9bf4eaad0246e9e0/src/main/templates/META-INF/neoforge.mods.toml
[mdk-12111-toml]: https://github.com/NeoForgeMDKs/MDK-1.21.11-ModDevGradle/blob/c3fd175261fd672799dca21ef03387bfdc4eba44/src/main/templates/META-INF/neoforge.mods.toml
[fmlat]: https://github.com/neoforged/AccessTransformers/blob/4c92e3a66f8a5dc530e3984ac969ef8d73970c38/FMLAT.md
[mixin-env]: https://github.com/SpongePowered/Mixin/wiki/Introduction-to-Mixins---The-Mixin-Environment
[maven-ranges]: https://maven.apache.org/enforcer/enforcer-rules/versionRanges.html
[mcmeta]: https://github.com/misode/mcmeta/blob/summary/versions/data.json
[cn-1324]: https://github.com/Sinytra/Connector/issues/1324
[cn-1355]: https://github.com/Sinytra/Connector/issues/1355
[cn-1894]: https://github.com/Sinytra/Connector/issues/1894
[cn-2076]: https://github.com/Sinytra/Connector/issues/2076
[cn-2287]: https://github.com/Sinytra/Connector/issues/2287
[cn-2294]: https://github.com/Sinytra/Connector/issues/2294
[cn-2337]: https://github.com/Sinytra/Connector/issues/2337
