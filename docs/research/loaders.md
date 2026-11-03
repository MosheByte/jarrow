# Fabric, NeoForge, Forge and Quilt on 1.20.1, 1.21.1 and 1.21.11: research notes

Research for the loader profiles ([`loaders/`](../../loaders/)), the engine and the harness. Read 2026-11-03. Every claim links to the page, file or API response it came from, and repository files are cited at a fixed commit or tag. Everything is in our own words. Nothing here comes from Sinytra Connector, Architectury or Forgified Fabric API code.

These notes describe the four loaders. They do not say that Jarrow converts anything. No direction converts yet, and no direction has a board column yet. A pairing between two loaders counts as working only when the harness says so.

The exact versions, URLs and hashes are in [`research-lock.json`](../../research-lock.json) (decision D-029 adds 1.20.1, Forge and Quilt). `tools/research_lock.py --verify` re-checks them. Fabric and NeoForge on 1.21.1 and 1.21.11 are covered in more depth in [`fabric-neoforge-notes.md`](fabric-neoforge-notes.md). This page repeats only what the loader profiles need and links to that page's sections for the rest.

| | 1.20.1 | 1.21.1 | 1.21.11 |
| --- | --- | --- | --- |
| Java (Mojang version JSON) | 17 | 21 | 21 |
| Fabric Loader / Fabric API | 0.20.1 / 0.92.12+1.20.1 | 0.20.1 / 0.116.17+1.21.1 | 0.20.1 / 0.141.6+1.21.11 |
| NeoForge | 47.1.106 (`net.neoforged:forge`, FML 47.2.2) | 21.1.265 (FML 4.0.45) | 21.11.48 (FML 10.0.36) |
| Forge (recommended build) | 47.4.28 | 52.1.20 | 61.2.4 |
| Quilt Loader / API library | 0.30.2 / QFAPI 7.7.0+0.92.2-1.20.1 | 0.30.2 / Fabric API 0.116.17+1.21.1 | 0.30.2 / Fabric API 0.141.6+1.21.11 |
| Names at runtime | Fabric, Quilt: intermediary. NeoForge, Forge: SRG members, Mojang classes | Fabric, Quilt: intermediary. NeoForge, Forge: Mojang | same as 1.21.1 |

Version facts come from [Fabric meta][fabric-meta-1201], [the NeoForge maven API for 1.20.1][nf-legacy-versions], [Forge's promotions file][forge-promos], [Quilt meta][quilt-meta-loader] and [Modrinth][qfapi-modrinth]. We downloaded the four Forge-family installers (Forge 47.4.28, 52.1.20 and 61.2.4, and NeoForge 47.1.106) to a temp directory, checked each against its sha256 sidecar, read `install_profile.json` and `version.json`, and deleted them. Their URLs and hashes are in the lock.

---

## 1. Fabric

The loader is the same build on all three versions. Only Fabric API differs.

| Topic | 1.20.1 | 1.21.1 and 1.21.11 |
| --- | --- | --- |
| Manifest | `fabric.mod.json`, schema version 1, read by the loader's V1 parser ([Fabric docs][fd-fmj]; [V1ModMetadataParser @0.20.1][loader-v1parser]). Mod ids match `[a-z][a-z0-9-_]{1,63}` ([MetadataVerifier @0.20.1][loader-verifier]). Fabric API for 1.20.1 asks for `fabricloader >=0.16.10` and `java >=17` ([fabric.mod.json @0.92.12][fapi-1201-fmj]). | Same file and parser. See [notes, section 3](fabric-neoforge-notes.md#3-fabricmodjson-schema-v1-and-the-neoforge-counterpart). |
| Entrypoints | `main`, then `client` on a client or `server` on a dedicated server ([Hooks.java @0.20.1][loader-hooks]); `preLaunch` runs earlier, from Knot ([Knot.java @0.20.1][loader-knot]). | Same loader, same order ([notes, section 2](fabric-neoforge-notes.md#2-fabric-entrypoints)). |
| Events and lifecycle | Fabric API `Event<T>` with phases; the default phase is `fabric:default` ([Event.java @0.92.12][fapi-1201-event]). | Same model ([notes, section 6](fabric-neoforge-notes.md#6-events-fabric-callbacks-vs-neoforge-subscribers)). |
| Registration | Mods call vanilla `Registry.register` during `main`. Fabric API's registry sync redirects the vanilla registry bootstrap so the built-in registries stay open for mod init ([BootstrapMixin @0.92.12][fapi-1201-bootstrap]). | Same redirect; the dedicated server freezes after the entrypoints ([notes, section 5](fabric-neoforge-notes.md#5-registries-deferredregister-registerevent-freezing)). |
| Names at runtime | intermediary: the loader remaps the game to intermediary outside a dev environment ([Fabric docs, Loader][fd-loader]). Fabric meta pairs loader 0.20.1 with `net.fabricmc:intermediary:1.20.1` ([Fabric meta][fabric-meta-1201]). | intermediary ([notes, section 1](fabric-neoforge-notes.md#1-names-at-runtime-mappings)). |
| Mixin | `net.fabricmc:sponge-mixin:0.17.5+mixin.0.8.7` and MixinExtras ship with loader 0.20.1 ([launcher JSON][loader-json]). | Same loader, same Mixin. |
| Access file | Access widener, header `accessWidener v1\|v2 <namespace>`, with `accessible`, `extendable`, `mutable` and the `transitive-` forms ([AccessWidenerReader][aw-reader]). | Same reader; on 1.21.11 the Fabric docs file access wideners under "class tweakers" ([Fabric docs 1.21.11][fd-aw-12111]). |
| Resource conditions | Root key `fabric:load_conditions` holds an **array** of condition objects, each naming its condition in `condition` ([ResourceConditions @0.92.12][fapi-1201-rc]). | Same key; one condition or an array, codec based ([ResourceCondition @0.116.17][fapi-rcodec-1211]; [notes, section 10](fabric-neoforge-notes.md#10-resource-conditions)). |
| Networking | Channels named by an `Identifier` carry a `PacketByteBuf`; the typed form is `FabricPacket` with a `PacketType` ([ServerPlayNetworking @0.92.12][fapi-1201-spn]). There is no `PayloadTypeRegistry` on 1.20.1 ([networking module @0.92.12][fapi-1201-networking]). | Vanilla `CustomPacketPayload` registered in `PayloadTypeRegistry` ([notes, section 11](fabric-neoforge-notes.md#11-networking-payloads)). |
| Dedicated server | The meta server launcher for `1.20.1/0.20.1/1.1.2`, or the pinned installer's `server` mode ([ServerLauncher.java][inst-launcher]; [ServerHandler.java][inst-serverhandler]). | Same ([notes, section 13](fabric-neoforge-notes.md#13-harness-provisioning)). |

## 2. NeoForge

NeoForge for 1.20.1 is a different product line from NeoForge for 1.21.x, so the two are described separately.

### 2.1 NeoForge 47.1.106 on 1.20.1

- **What it is.** It is published as `net.neoforged:forge` with Maven versions `1.20.1-47.1.N` ([NeoForge maven API][nf-legacy-versions]). Its source keeps Forge's `net.minecraftforge` packages ([NeoForge 1.20.1 tree][nf-1201-tree]). Its installer ships FancyModLoader 47.2.2 together with `net.minecraftforge:eventbus:6.0.5` and `cpw.mods:modlauncher:10.0.9` ([installer 47.1.106][nf-installer-1201], `version.json`). The NeoForge docs have no 1.20.1 version: their versioned docs start at 1.20.4 ([versioned_docs][nf-docs-versions]). So for this line we read the NeoForge 1.20.1 MDK and source, and the Forge 1.20.1 docs where they describe the shared Forge 47 base.
- **Manifest.** `META-INF/mods.toml`, `modLoader="javafml"`, `loaderVersion` from `loader_version_range=[47,)` ([MDK 1.20.1 properties][nf-mdk-1201-props]). Mods depend on the mod id **`forge`**, not `neoforge` ([MDK 1.20.1 mods.toml][nf-mdk-1201-toml]). Mod ids match `^[a-z][a-z0-9_]{1,63}$` ([Forge docs 1.20.1, mod files][forge-docs-1201-modfiles]).
- **Entrypoint.** A class annotated `@Mod(MODID)`; its constructor gets the mod bus from `FMLJavaModLoadingContext.get().getModEventBus()` and registers itself on `MinecraftForge.EVENT_BUS` ([MDK 1.20.1 ExampleMod][nf-mdk-1201-mod]).
- **Events and lifecycle.** Two buses: `MinecraftForge.EVENT_BUS` and the mod bus. Priorities run `HIGHEST` to `LOWEST` ([Forge docs 1.20.1, events][forge-docs-1201-events]). Registry events come after mod construction and run one after another; most other lifecycle events run in parallel and use `enqueueWork` for main-thread work ([Forge docs 1.20.1, lifecycle][forge-docs-1201-lifecycle]).
- **Registration.** `DeferredRegister` on the mod bus, or `RegisterEvent` ([MDK 1.20.1 ExampleMod][nf-mdk-1201-mod]; [Forge docs 1.20.1, registries][forge-docs-1201-registries]).
- **Names at runtime: SRG.** The installer merges MCP config `1.20.1-20230612.114412` with Mojang's server mappings and renames the vanilla server into its `MC_SRG` jar before patching. Classes get Mojang names, fields and methods SRG names ([installer 47.1.106][nf-installer-1201], `install_profile.json`). This matches NeoForge's statement that SRG was dropped only with 20.2 for 1.20.2 ([NeoForge discussion #199][nf-202]).
- **Mixin.** `org.spongepowered:mixin:0.8.5` (upstream Mixin, not Fabric's fork) ([installer 47.1.106][nf-installer-1201], `version.json`). Mixin finds a mod's configs through the `MixinConfigs` attribute of the jar manifest ([Mixin 0.8.5 Constants][mixin-085-constants]; [MixinPlatformAgentDefault][mixin-085-agent]).
- **Access file.** Access transformers, by default `META-INF/accesstransformer.cfg`. Fields and methods of Minecraft classes are named with SRG names ([Forge docs 1.20.1, access transformers][forge-docs-1201-at]).
- **Resource conditions.** Root key `forge:conditions` ([ICondition @NeoForge 1.20.1][nf-1201-cond]). Condition types include `forge:true`, `forge:not`, `forge:and`, `forge:or`, `forge:mod_loaded`, `forge:item_exists` and `forge:tag_empty` ([Forge docs 1.20.1, conditional][forge-docs-1201-cond]).
- **Networking.** `SimpleChannel` from `NetworkRegistry.newSimpleChannel` ([NetworkRegistry @NeoForge 1.20.1][nf-1201-net]).
- **Dedicated server.** `java -jar forge-1.20.1-47.1.106-installer.jar --installServer` writes `run.sh` and `run.bat`, which start `java @user_jvm_args.txt @libraries/net/neoforged/forge/1.20.1-47.1.106/unix_args.txt` ([installer 47.1.106][nf-installer-1201], `data/run.sh`).

### 2.2 NeoForge 21.1.265 and 21.11.48

Covered in [`fabric-neoforge-notes.md`](fabric-neoforge-notes.md). In short:
- `META-INF/neoforge.mods.toml`, with `modLoader` and `loaderVersion` mandatory on 1.21.1 and optional on 1.21.11 ([mods.toml 1.21.1][nf-modfiles-1211]; [mods.toml 1.21.11][nf-modfiles-12111]).
- An `@Mod` class with an `IEventBus` constructor parameter, and two buses ([events 1.21.11][nf-events-12111]).
- `DeferredRegister` or `RegisterEvent`, inside the registry window ([CommonModLoader 1.21.1][nf-cml-1211]).
- Mojang names at runtime; Fabric's Mixin fork 0.16.4 and 0.16.5 (installers, [notes, section 8](fabric-neoforge-notes.md#8-mixin-refmaps-and-loader-differences)).
- Access transformers ([AT docs 1.21.11][nf-at-12111]).
- `neoforge:conditions` ([conditions 1.21.1][nf-cond-1211]).
- Payloads through `RegisterPayloadHandlersEvent` ([payloads 1.21.1][nf-payload-1211]).
- `--installServer` ([server guide][nf-userserver]).

## 3. Forge (MinecraftForge)

The pins are the recommended builds from Forge's promotions file on 2026-11-03: 47.4.28, 52.1.20 and 61.2.4 ([promotions_slim.json][forge-promos]). The Forge docs have a 1.20.1 branch and a 1.21.x branch. The 1.21.x pages still describe 1.21.1-era APIs, and some of their statements no longer match the 1.21.1 source (noted below), so the source at each build's commit wins.

| Topic | 47.4.28 (1.20.1) | 52.1.20 (1.21.1) | 61.2.4 (1.21.11) |
| --- | --- | --- | --- |
| Manifest | `META-INF/mods.toml`; `modLoader` and `loaderVersion` are mandatory; for `javafml` the loader version is the Forge major version ([docs 1.20.1][forge-docs-1201-modfiles]). Mods depend on mod id `forge` ([MDK mods.toml @47.4.28][forge-mdk-1201-toml]). | Same file; adds `clientSideOnly`, which skips the mod file on a dedicated server ([docs 1.21.x][forge-docs-121-modfiles]). MDK: `loaderVersion="[<spec version>,)"` ([MDK mods.toml @52.1.20][forge-mdk-1211-toml]). | Same template ([MDK mods.toml @61.2.4][forge-mdk-12111-toml]). |
| Mod id | `^[a-z][a-z0-9_]{1,63}$`; no `-` ([docs 1.20.1][forge-docs-1201-modfiles]) | Same ([docs 1.21.x][forge-docs-121-modfiles]) | Not documented separately; the 1.21.x page is the latest ([docs 1.21.x][forge-docs-121-modfiles]) |
| Entrypoint | `@Mod` class; mod bus from `FMLJavaModLoadingContext.get().getModEventBus()` ([docs 1.20.1][forge-docs-1201-modfiles]) | `@Mod` class whose constructor takes `FMLJavaModLoadingContext` and reads the mod bus from it ([docs 1.21.x][forge-docs-121-modfiles]; [MDK ExampleMod @52.1.20][forge-mdk-1211-mod]) | `FMLJavaModLoadingContext.getModBusGroup()` returns a `BusGroup` ([FMLJavaModLoadingContext @61.2.4][forge-12111-ctx]) |
| Events | EventBus 6.2.33 ([installer][forge-installer-1201]); `MinecraftForge.EVENT_BUS` plus the mod bus, `@SubscribeEvent`, `@Cancelable`, five priorities ([docs 1.20.1][forge-docs-1201-events]) | EventBus 6.2.33 ([installer][forge-installer-1211]); same model, plus a `MONITOR` priority that runs last and must not change the event ([docs 1.21.x][forge-docs-121-events]) | **EventBus 7.0.5** ([installer][forge-installer-12111]): each event type has its own `EventBus<T>`, and events can be records ([EventBus 7.0 README][eventbus7]). Game events expose a static `BUS`, for example `TickEvent.ServerTickEvent.Pre.BUS` ([TickEvent @61.2.4][forge-12111-tick]); mod-bus events are reached through `getBus(modBusGroup)` ([FMLCommonSetupEvent @61.2.4][forge-12111-setup]) |
| Lifecycle | Construction, then `NewRegistryEvent`, `DataPackRegistryEvent.NewRegistry` and `RegisterEvent` in sequence, then common setup and sided setup in parallel ([docs 1.20.1][forge-docs-1201-lifecycle]) | Same order ([docs 1.21.x][forge-docs-121-lifecycle]) | `RegisterEvent` is a mod-bus event read through `getBus(modBusGroup)` ([RegisterEvent @61.2.4][forge-12111-register]) |
| Registration | `DeferredRegister` or `RegisterEvent`; `RegisterEvent` fires per registry after the mod constructors and before configs load ([docs 1.20.1][forge-docs-1201-registries]) | Same ([docs 1.21.x][forge-docs-121-registries]) | `DeferredRegister` still exists ([source tree @61.2.4][forge-12111-tree]) |
| Names at runtime | **SRG** members, Mojang classes: MCP config merged with Mojang's mappings, server renamed into `MC_SRG` ([installer][forge-installer-1201], `install_profile.json`) | **Mojang**: Mojang's server mappings applied by `ForgeAutoRenamingTool --reverse` into `MC_OFF` ([installer][forge-installer-1211]) | **Mojang**, same steps ([installer][forge-installer-12111]) |
| Mixin | `org.spongepowered:mixin:0.8.5` ([installer][forge-installer-1201]) | `org.spongepowered:mixin:0.8.7` ([installer][forge-installer-1211]) | `org.spongepowered:mixin:0.8.7` plus `mixinextras-forge:0.5.4` ([installer][forge-installer-12111]) |
| Mixin configs | Listed in the jar manifest's `MixinConfigs` attribute ([MDKExamples, mixins-only][forge-mdkexamples-mixin]; [Mixin Constants][mixin-085-constants]). Forge's example set says obfuscated (SRG) mod jars are needed for 1.20.4 and older ([MDKExamples README][forge-mdkexamples-readme]). | Same manifest attribute ([MDKExamples][forge-mdkexamples-mixin]) | Same |
| Access file | Access transformers: `META-INF/accesstransformer.cfg` by default, or the `accessTransformers` list in `mods.toml` ([ModFile @47.4.28][forge-1201-modfile]); SRG names for Minecraft members ([docs 1.20.1][forge-docs-1201-at]) | Same lookup ([ModFile @52.1.20][forge-1211-modfile]). The 1.21.x docs page still says SRG names ([docs 1.21.x][forge-docs-121-at]), but the game runs on Mojang names on this build (row above); which names an AT line needs here is an open question (section 6) | Same as 52.1.20 ([ModFile @61.2.4][forge-12111-modfile]) |
| Resource conditions | Root key `forge:conditions`, an array ([ICondition @47.4.28][forge-1201-cond]); types `forge:true`, `forge:not`, `forge:and`, `forge:or`, `forge:mod_loaded`, `forge:item_exists`, `forge:tag_empty` ([docs 1.20.1][forge-docs-1201-cond]) | Root key `forge:condition`, **one** condition ([ICondition @52.1.20][forge-1211-cond]) | `forge:condition` ([ICondition @61.2.4][forge-12111-cond]) |
| Networking | `SimpleChannel` from `NetworkRegistry.newSimpleChannel` ([NetworkRegistry @47.4.28][forge-1201-net]) | `ChannelBuilder.named(...)` then `simpleChannel()`, `eventNetworkChannel()` or `payloadChannel()` for vanilla `CustomPacketPayload` ([ChannelBuilder @52.1.20][forge-1211-net]). The 1.21.x docs page still shows `newSimpleChannel` ([docs 1.21.x][forge-docs-121-net]) | Same builder ([ChannelBuilder @61.2.4][forge-12111-net]) |
| Dedicated server | `--installServer` writes `run.sh` and `run.bat`: `java @user_jvm_args.txt @libraries/net/minecraftforge/forge/1.20.1-47.4.28/unix_args.txt`; main class `cpw.mods.bootstraplauncher.BootstrapLauncher` ([installer][forge-installer-1201]) | `run.sh` first runs `forge-1.21.1-52.1.20-shim.jar --onlyCheckJava`, then the args file; the args file starts the shim jar; main class `net.minecraftforge.bootstrap.ForgeBootstrap` ([installer][forge-installer-1211]) | Same as 52.1.20 with the 61.2.4 shim ([installer][forge-installer-12111]) |

## 4. Quilt

Quilt Loader 0.30.2 is the newest version without a pre-release suffix; Quilt meta lists it for all three game versions ([Quilt meta][quilt-meta-loader]). Quilt meta has no stable flag, so "stable" here means "no `-beta.N`".

- **Manifest.** `quilt.mod.json` with `schema_version: 1` and a `quilt_loader` object ([RFC 0002][rfc-qmj]). Mod ids match `^[a-z][a-z0-9-_]{1,63}$`, so `-` is allowed, as on Fabric ([RFC 0002][rfc-qmj]). Quilt Loader also reads `fabric.mod.json` ([FabricModMetadataReader @0.30.2][ql-fabricreader]) and presents itself to Fabric mods as `fabricloader` 0.19.5 ([quilt.mod.json @0.30.2][ql-qmj]). That meets Fabric API's `fabricloader` requirement on all three versions (>=0.16.10 on 1.20.1 per [fabric.mod.json @0.92.12][fapi-1201-fmj]; >=0.15.11 and >=0.17.3 on 1.21.x per the lock notes).
- **Entrypoints.** The loader itself runs the Fabric keys: `main`, then `client` or `server` ([Hooks.java @0.30.2][ql-hooks]). An entrypoint can name a class, a static field or a method ([RFC 0002][rfc-qmj]). With QSL (1.20.1 only, see below), Quilt mods use `init`, `client_init` and `server_init`, and the initializer receives its `ModContainer` ([ModInitializer @QSL 6.3.0][qsl-modinit]; [ClientModInitializer][qsl-client]; [DedicatedServerModInitializer][qsl-server]).
- **Which API library.**
  - On 1.20.1: QFAPI 7.7.0+0.92.2-1.20.1, which bundles QSL 6.3.0 and Fabric API 0.92.2 ([Modrinth][qfapi-modrinth]; [QFAPI gradle.properties @7.7.0][qfapi-props]).
  - On 1.21.1 and 1.21.11: Modrinth lists no QFAPI or QSL release, only 1.21 alphas ([Modrinth][qfapi-modrinth]). Since 1.20.6 QSL ships only `qsl_base` and `crash_info`, and Quilt tells modders to move to the Fabric API equivalents ([Quilt blog, 2024-07-03][quilt-blog-2024-07]). So on those versions a Quilt mod uses Fabric API, and everything in section 1 for Fabric API applies.
- **Events.** On 1.20.1, QSL has its own `Event<T>` with phases; the default phase is `quilt:default` ([Event @QSL 6.3.0][qsl-event]). QFAPI also carries Fabric API's modules, including lifecycle events ([QFAPI tree @7.7.0][qfapi-tree]). On 1.21.x, Fabric API's `Event<T>` (section 1).
- **Registration.** Vanilla `Registry.register`, as on Fabric. QSL 6.3.0 adds registry events such as an entry-added event per registry ([RegistryEvents @QSL 6.3.0][qsl-registry-events]), and QFAPI carries Fabric's registry sync ([QFAPI tree @7.7.0][qfapi-tree]).
- **Names at runtime: intermediary.** Outside development the loader targets the `intermediary` namespace ([MappingConfigurationImpl @0.30.2][ql-mapping]), and Quilt meta pairs 0.30.2 with `net.fabricmc:intermediary` for each version ([Quilt meta 1.20.1][quilt-meta-1201]). The RFC's default for a mod's `intermediate_mappings` is `org.quiltmc:hashed`, with `net.fabricmc:intermediary` as the other supported value ([RFC 0002][rfc-qmj]). We have not checked which of the two mod jars on the board declare.
- **Mixin.** `net.fabricmc:sponge-mixin:0.17.4+mixin.0.8.7`, Fabric's fork ([Quilt meta 1.20.1][quilt-meta-1201], launcherMeta; [gradle.properties @0.30.2][ql-props]). The loader's own `quilt.mod.json` depends on `mixinextras` ([quilt.mod.json @0.30.2][ql-qmj]), and its build pins MixinExtras 0.5.5 ([gradle.properties @0.30.2][ql-props]). Configs are listed in the `mixin` field, each with an optional `environment` ([RFC 0002][rfc-qmj]).
- **Access file.** The `access_widener` field lists access widener files ([RFC 0002][rfc-qmj]); the loader builds against FabricMC's `class_tweaker` 0.3.0-beta.2 ([gradle.properties @0.30.2][ql-props]). The environment field uses `client`, `dedicated_server` or `*` ([RFC 0002][rfc-qmj]), not Fabric's `server`.
- **Resource conditions.** QSL 6.3.0 has no conditions module of its own ([QSL tree @6.3.0][qsl-tree]). QFAPI 7.7.0 carries Fabric's `fabric-resource-conditions-api-v1` ([QFAPI tree @7.7.0][qfapi-tree]), so the key is `fabric:load_conditions` on 1.20.1, and on 1.21.x as well, through Fabric API.
- **Networking.** On 1.20.1, QSL's `ServerPlayNetworking` works on channels named by an `Identifier` with a `PacketByteBuf`, like Fabric's 1.20.1 API ([ServerPlayNetworking @QSL 6.3.0][qsl-networking]), and QFAPI carries Fabric's networking module too ([QFAPI tree @7.7.0][qfapi-tree]). On 1.21.x, Fabric API networking (section 1).
- **Dedicated server.** `java -jar quilt-installer-0.15.1.jar install server <game> <loader> --install-dir=<dir>` writes `quilt-server-launch.jar`; `--download-server` also fetches `server.jar` ([CliInstaller @0.15.1][qi-cli]; [InstallServer @0.15.1][qi-server]). The launcher takes the game jar from `-Dloader.gameJarPath`, else from `serverJar` in `quilt-server-launcher.properties`, which defaults to `server.jar` ([QuiltServerLauncher @0.30.2][ql-serverlauncher]). So the harness can place the sha1-checked Mojang jar itself and skip `--download-server`.

## 5. Side by side

| | Fabric | NeoForge | Forge | Quilt |
| --- | --- | --- | --- | --- |
| Manifest | `fabric.mod.json` | 1.20.1: `META-INF/mods.toml`; 1.21.x: `META-INF/neoforge.mods.toml` | `META-INF/mods.toml` | `quilt.mod.json` (also reads `fabric.mod.json`) |
| Mod id | `-` allowed | no `-` | no `-` | `-` allowed |
| Loader dependency id | `fabricloader` | 1.20.1: `forge`; 1.21.x: `neoforge` | `forge` | `quilt_loader` (and `fabricloader` 0.19.5 provided) |
| Entry | `main` / `client` / `server` | `@Mod` class | `@Mod` class | Fabric keys; `init` / `client_init` / `server_init` with QSL |
| Events | `Event<T>`, phases | 1.20.1: Forge EventBus 6; 1.21.x: NeoForge bus | EventBus 6 (1.20.1, 1.21.1), EventBus 7 (1.21.11) | QSL `Event<T>` (1.20.1), Fabric `Event<T>` (1.21.x) |
| Runtime names | intermediary | 1.20.1: SRG members; 1.21.x: Mojang | 1.20.1: SRG members; 1.21.x: Mojang | intermediary |
| Mixin | Fabric fork 0.17.5 | 1.20.1: upstream 0.8.5; 1.21.x: Fabric fork 0.16.4 / 0.16.5 | upstream 0.8.5 / 0.8.7 / 0.8.7 + MixinExtras 0.5.4 | Fabric fork 0.17.4 |
| Mixin config list | `fabric.mod.json` `mixins` | 1.20.1: manifest `MixinConfigs`; 1.21.x: `[[mixins]]` | manifest `MixinConfigs` | `quilt.mod.json` `mixin` |
| Access file | access widener | access transformer | access transformer | access widener |
| Conditions key | `fabric:load_conditions` | 1.20.1: `forge:conditions`; 1.21.x: `neoforge:conditions` | 1.20.1: `forge:conditions`; 1.21.x: `forge:condition` | `fabric:load_conditions` |
| Server start | meta launcher or installer | installer `--installServer`, `run.sh` | installer `--installServer`, `run.sh` | quilt-installer, `quilt-server-launch.jar` |

Every cell repeats a claim from sections 1 to 4, where its source is linked. The NeoForge 1.21.x Mixin config list is from [mods.toml 1.21.11][nf-modfiles-12111], and the Modrinth loader tags are `fabric`, `neoforge`, `forge` and `quilt` ([Modrinth loader tags][modrinth-loader-tags]).

## 6. Open questions the harness must answer

None of these is settled by documentation. Each becomes a harness probe or a `needs-proof` issue.

1. Forge 52.1.20 and 61.2.4 run on Mojang names, but the Forge 1.21.x access transformer page still asks for SRG names. Which names does an AT line need on those builds?
2. Does Forge 47.4.28 or 52.1.20 provide MixinExtras at runtime? Only the 61.2.4 installer lists it.
3. Does a Fabric API mod for 1.21.1 or 1.21.11 load unchanged on Quilt Loader 0.30.2? The loader provides `fabricloader` 0.19.5, but nothing here measures it.
4. Which `intermediate_mappings` value do Quilt mod jars on the board declare, and does Quilt Loader 0.30.2 still accept `org.quiltmc:hashed`?
5. Does NeoForge 47.1.106 behave like Forge 47.4.28 for the same mod? They share a base, but 47.4.28 is about two years newer.
6. Do the 1.20.1 servers start on JDK 21? The lock pins Java 17 for 1.20.1 and does not assume more.

---

### Sources (all read 2026-11-03)

[fabric-meta-1201]: https://meta.fabricmc.net/v2/versions/loader/1.20.1/0.20.1
[nf-legacy-versions]: https://maven.neoforged.net/api/maven/versions/releases/net/neoforged/forge
[forge-promos]: https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json
[quilt-meta-loader]: https://meta.quiltmc.org/v3/versions/loader
[quilt-meta-1201]: https://meta.quiltmc.org/v3/versions/loader/1.20.1
[qfapi-modrinth]: https://api.modrinth.com/v2/project/qvIfYCYJ/version
[modrinth-loader-tags]: https://api.modrinth.com/v2/tag/loader
[fd-fmj]: https://docs.fabricmc.net/develop/loader/fabric-mod-json
[fd-loader]: https://docs.fabricmc.net/develop/loader/
[fd-aw-12111]: https://docs.fabricmc.net/1.21.11/develop/class-tweakers/access-widening
[loader-v1parser]: https://github.com/FabricMC/fabric-loader/blob/0.20.1/src/main/java/net/fabricmc/loader/impl/metadata/V1ModMetadataParser.java
[loader-verifier]: https://github.com/FabricMC/fabric-loader/blob/0.20.1/src/main/java/net/fabricmc/loader/impl/metadata/MetadataVerifier.java
[loader-hooks]: https://github.com/FabricMC/fabric-loader/blob/0.20.1/minecraft/src/main/java/net/fabricmc/loader/impl/game/minecraft/Hooks.java
[loader-knot]: https://github.com/FabricMC/fabric-loader/blob/0.20.1/src/main/java/net/fabricmc/loader/impl/launch/knot/Knot.java
[loader-json]: https://maven.fabricmc.net/net/fabricmc/fabric-loader/0.20.1/fabric-loader-0.20.1.json
[aw-reader]: https://github.com/FabricMC/access-widener/blob/30e2d0038d2307140f122eebc2f782f440250234/src/main/java/net/fabricmc/accesswidener/AccessWidenerReader.java
[fapi-1201-fmj]: https://github.com/FabricMC/fabric/blob/ba3f3da8411fb8dd27fa5eb3551d9ff775bb2c0a/src/main/resources/fabric.mod.json
[fapi-1201-event]: https://github.com/FabricMC/fabric/blob/ba3f3da8411fb8dd27fa5eb3551d9ff775bb2c0a/fabric-api-base/src/main/java/net/fabricmc/fabric/api/event/Event.java
[fapi-1201-bootstrap]: https://github.com/FabricMC/fabric/blob/ba3f3da8411fb8dd27fa5eb3551d9ff775bb2c0a/fabric-registry-sync-v0/src/main/java/net/fabricmc/fabric/mixin/registry/sync/BootstrapMixin.java
[fapi-1201-rc]: https://github.com/FabricMC/fabric/blob/ba3f3da8411fb8dd27fa5eb3551d9ff775bb2c0a/fabric-resource-conditions-api-v1/src/main/java/net/fabricmc/fabric/api/resource/conditions/v1/ResourceConditions.java
[fapi-1201-spn]: https://github.com/FabricMC/fabric/blob/ba3f3da8411fb8dd27fa5eb3551d9ff775bb2c0a/fabric-networking-api-v1/src/main/java/net/fabricmc/fabric/api/networking/v1/ServerPlayNetworking.java
[fapi-1201-networking]: https://github.com/FabricMC/fabric/tree/ba3f3da8411fb8dd27fa5eb3551d9ff775bb2c0a/fabric-networking-api-v1/src/main/java/net/fabricmc/fabric/api/networking/v1
[fapi-rcodec-1211]: https://github.com/FabricMC/fabric/blob/83c07162d88a703d15c8dae50cbbc12f4e9e63c3/fabric-resource-conditions-api-v1/src/main/java/net/fabricmc/fabric/api/resource/conditions/v1/ResourceCondition.java
[inst-launcher]: https://github.com/FabricMC/fabric-installer/blob/6e7d1acd6a951a19a323060ac955f5e7bc51e3ce/src/main/java/net/fabricmc/installer/ServerLauncher.java
[inst-serverhandler]: https://github.com/FabricMC/fabric-installer/blob/6e7d1acd6a951a19a323060ac955f5e7bc51e3ce/src/main/java/net/fabricmc/installer/server/ServerHandler.java
[nf-1201-tree]: https://github.com/neoforged/NeoForge/tree/a3317fc07a2b36793b8b87c19dd4b05265135d7c/src/main/java/net/minecraftforge
[nf-1201-cond]: https://github.com/neoforged/NeoForge/blob/a3317fc07a2b36793b8b87c19dd4b05265135d7c/src/main/java/net/minecraftforge/common/crafting/conditions/ICondition.java
[nf-1201-net]: https://github.com/neoforged/NeoForge/blob/a3317fc07a2b36793b8b87c19dd4b05265135d7c/src/main/java/net/minecraftforge/network/NetworkRegistry.java
[nf-installer-1201]: https://maven.neoforged.net/releases/net/neoforged/forge/1.20.1-47.1.106/forge-1.20.1-47.1.106-installer.jar
[nf-docs-versions]: https://github.com/neoforged/Documentation/tree/4a680cdd4d15013b65fd84998d1f4cd7e2885ae5/versioned_docs
[nf-mdk-1201-props]: https://github.com/neoforged/MDK/blob/80aef8cefff5d43d54341d007376a2ed5150a74e/gradle.properties
[nf-mdk-1201-toml]: https://github.com/neoforged/MDK/blob/80aef8cefff5d43d54341d007376a2ed5150a74e/src/main/templates/META-INF/mods.toml
[nf-mdk-1201-mod]: https://github.com/neoforged/MDK/blob/80aef8cefff5d43d54341d007376a2ed5150a74e/src/main/java/com/example/examplemod/ExampleMod.java
[nf-202]: https://github.com/neoforged/NeoForge/discussions/199
[nf-modfiles-1211]: https://docs.neoforged.net/docs/1.21.1/gettingstarted/modfiles/
[nf-modfiles-12111]: https://docs.neoforged.net/docs/1.21.11/gettingstarted/modfiles/
[nf-events-12111]: https://docs.neoforged.net/docs/1.21.11/concepts/events/
[nf-cml-1211]: https://github.com/neoforged/NeoForge/blob/624c929f49920c250166e2a8df7af87f16d220ef/src/main/java/net/neoforged/neoforge/internal/CommonModLoader.java
[nf-at-12111]: https://docs.neoforged.net/docs/1.21.11/advanced/accesstransformers/
[nf-cond-1211]: https://docs.neoforged.net/docs/1.21.1/resources/server/conditions/
[nf-payload-1211]: https://docs.neoforged.net/docs/1.21.1/networking/payload/
[nf-userserver]: https://docs.neoforged.net/user/docs/server
[forge-docs-1201-modfiles]: https://github.com/MinecraftForge/Documentation/blob/2bd2b1e3cfdc899057ba57b992e157cdc2094ded/docs/gettingstarted/modfiles.md
[forge-docs-1201-events]: https://github.com/MinecraftForge/Documentation/blob/2bd2b1e3cfdc899057ba57b992e157cdc2094ded/docs/concepts/events.md
[forge-docs-1201-lifecycle]: https://github.com/MinecraftForge/Documentation/blob/2bd2b1e3cfdc899057ba57b992e157cdc2094ded/docs/concepts/lifecycle.md
[forge-docs-1201-registries]: https://github.com/MinecraftForge/Documentation/blob/2bd2b1e3cfdc899057ba57b992e157cdc2094ded/docs/concepts/registries.md
[forge-docs-1201-at]: https://github.com/MinecraftForge/Documentation/blob/2bd2b1e3cfdc899057ba57b992e157cdc2094ded/docs/advanced/accesstransformers.md
[forge-docs-1201-cond]: https://github.com/MinecraftForge/Documentation/blob/2bd2b1e3cfdc899057ba57b992e157cdc2094ded/docs/resources/server/conditional.md
[forge-docs-121-modfiles]: https://github.com/MinecraftForge/Documentation/blob/ac05e12aae903703cb8a9908a153623ce476a509/docs/gettingstarted/modfiles.md
[forge-docs-121-events]: https://github.com/MinecraftForge/Documentation/blob/ac05e12aae903703cb8a9908a153623ce476a509/docs/concepts/events.md
[forge-docs-121-lifecycle]: https://github.com/MinecraftForge/Documentation/blob/ac05e12aae903703cb8a9908a153623ce476a509/docs/concepts/lifecycle.md
[forge-docs-121-registries]: https://github.com/MinecraftForge/Documentation/blob/ac05e12aae903703cb8a9908a153623ce476a509/docs/concepts/registries.md
[forge-docs-121-at]: https://github.com/MinecraftForge/Documentation/blob/ac05e12aae903703cb8a9908a153623ce476a509/docs/advanced/accesstransformers.md
[forge-docs-121-net]: https://github.com/MinecraftForge/Documentation/blob/ac05e12aae903703cb8a9908a153623ce476a509/docs/networking/simpleimpl.md
[forge-mdk-1201-toml]: https://github.com/MinecraftForge/MinecraftForge/blob/d13dd327bdc75c010f3358992e80e7e92f51e030/mdk/src/main/resources/META-INF/mods.toml
[forge-mdk-1211-toml]: https://github.com/MinecraftForge/MinecraftForge/blob/8199135df7b0ce1e321c5a59ac3e4e982f301812/mdk/src/main/resources/META-INF/mods.toml
[forge-mdk-12111-toml]: https://github.com/MinecraftForge/MinecraftForge/blob/94f2c3df41b209c9c0680d5c4cedc83916a5e1c3/mdk/src/main/resources/META-INF/mods.toml
[forge-mdk-1211-mod]: https://github.com/MinecraftForge/MinecraftForge/blob/8199135df7b0ce1e321c5a59ac3e4e982f301812/mdk/src/main/java/com/example/examplemod/ExampleMod.java
[forge-1201-modfile]: https://github.com/MinecraftForge/MinecraftForge/blob/d13dd327bdc75c010f3358992e80e7e92f51e030/fmlloader/src/main/java/net/minecraftforge/fml/loading/moddiscovery/ModFile.java
[forge-1211-modfile]: https://github.com/MinecraftForge/MinecraftForge/blob/8199135df7b0ce1e321c5a59ac3e4e982f301812/fmlloader/src/main/java/net/minecraftforge/fml/loading/moddiscovery/ModFile.java
[forge-12111-modfile]: https://github.com/MinecraftForge/MinecraftForge/blob/94f2c3df41b209c9c0680d5c4cedc83916a5e1c3/fmlloader/src/main/java/net/minecraftforge/fml/loading/moddiscovery/ModFile.java
[forge-1201-cond]: https://github.com/MinecraftForge/MinecraftForge/blob/d13dd327bdc75c010f3358992e80e7e92f51e030/src/main/java/net/minecraftforge/common/crafting/conditions/ICondition.java
[forge-1211-cond]: https://github.com/MinecraftForge/MinecraftForge/blob/8199135df7b0ce1e321c5a59ac3e4e982f301812/src/main/java/net/minecraftforge/common/crafting/conditions/ICondition.java
[forge-12111-cond]: https://github.com/MinecraftForge/MinecraftForge/blob/94f2c3df41b209c9c0680d5c4cedc83916a5e1c3/src/main/java/net/minecraftforge/common/crafting/conditions/ICondition.java
[forge-1201-net]: https://github.com/MinecraftForge/MinecraftForge/blob/d13dd327bdc75c010f3358992e80e7e92f51e030/src/main/java/net/minecraftforge/network/NetworkRegistry.java
[forge-1211-net]: https://github.com/MinecraftForge/MinecraftForge/blob/8199135df7b0ce1e321c5a59ac3e4e982f301812/src/main/java/net/minecraftforge/network/ChannelBuilder.java
[forge-12111-net]: https://github.com/MinecraftForge/MinecraftForge/blob/94f2c3df41b209c9c0680d5c4cedc83916a5e1c3/src/main/java/net/minecraftforge/network/ChannelBuilder.java
[forge-12111-tick]: https://github.com/MinecraftForge/MinecraftForge/blob/94f2c3df41b209c9c0680d5c4cedc83916a5e1c3/src/main/java/net/minecraftforge/event/TickEvent.java
[forge-12111-ctx]: https://github.com/MinecraftForge/MinecraftForge/blob/94f2c3df41b209c9c0680d5c4cedc83916a5e1c3/javafmllanguage/src/main/java/net/minecraftforge/fml/javafmlmod/FMLJavaModLoadingContext.java
[forge-12111-setup]: https://github.com/MinecraftForge/MinecraftForge/blob/94f2c3df41b209c9c0680d5c4cedc83916a5e1c3/src/main/java/net/minecraftforge/fml/event/lifecycle/FMLCommonSetupEvent.java
[forge-12111-register]: https://github.com/MinecraftForge/MinecraftForge/blob/94f2c3df41b209c9c0680d5c4cedc83916a5e1c3/src/main/java/net/minecraftforge/registries/RegisterEvent.java
[forge-12111-tree]: https://github.com/MinecraftForge/MinecraftForge/tree/94f2c3df41b209c9c0680d5c4cedc83916a5e1c3/src/main/java/net/minecraftforge/registries
[forge-installer-1201]: https://maven.minecraftforge.net/net/minecraftforge/forge/1.20.1-47.4.28/forge-1.20.1-47.4.28-installer.jar
[forge-installer-1211]: https://maven.minecraftforge.net/net/minecraftforge/forge/1.21.1-52.1.20/forge-1.21.1-52.1.20-installer.jar
[forge-installer-12111]: https://maven.minecraftforge.net/net/minecraftforge/forge/1.21.11-61.2.4/forge-1.21.11-61.2.4-installer.jar
[forge-mdkexamples-mixin]: https://github.com/MinecraftForge/MDKExamples/blob/c0a20b210732654a261f9c040af9fe115d32137e/mixins-only/fg7/build.gradle
[forge-mdkexamples-readme]: https://github.com/MinecraftForge/MDKExamples/blob/c0a20b210732654a261f9c040af9fe115d32137e/README.md
[eventbus7]: https://github.com/MinecraftForge/EventBus/blob/cab319a5fea8b205d8c8f205d6d55af2ba16126d/README.md
[mixin-085-constants]: https://github.com/SpongePowered/Mixin/blob/155314e6e91465dad727e621a569906a410cd6f4/src/main/java/org/spongepowered/asm/util/Constants.java
[mixin-085-agent]: https://github.com/SpongePowered/Mixin/blob/155314e6e91465dad727e621a569906a410cd6f4/src/main/java/org/spongepowered/asm/launch/platform/MixinPlatformAgentDefault.java
[rfc-qmj]: https://github.com/QuiltMC/rfcs/blob/ca458de2fef50da15d892e96be1165ce775255f2/specification/0002-quilt.mod.json.md
[ql-props]: https://github.com/QuiltMC/quilt-loader/blob/c36b50f62eebd9cd653a1f065596d3018700d9f0/gradle.properties
[ql-qmj]: https://github.com/QuiltMC/quilt-loader/blob/c36b50f62eebd9cd653a1f065596d3018700d9f0/src/main/resources/quilt.mod.json
[ql-mapping]: https://github.com/QuiltMC/quilt-loader/blob/c36b50f62eebd9cd653a1f065596d3018700d9f0/src/main/java/org/quiltmc/loader/impl/game/MappingConfigurationImpl.java
[ql-hooks]: https://github.com/QuiltMC/quilt-loader/blob/c36b50f62eebd9cd653a1f065596d3018700d9f0/minecraft/src/main/java/org/quiltmc/loader/impl/game/minecraft/Hooks.java
[ql-fabricreader]: https://github.com/QuiltMC/quilt-loader/blob/c36b50f62eebd9cd653a1f065596d3018700d9f0/src/main/java/org/quiltmc/loader/impl/fabric/metadata/FabricModMetadataReader.java
[ql-serverlauncher]: https://github.com/QuiltMC/quilt-loader/blob/c36b50f62eebd9cd653a1f065596d3018700d9f0/src/main/java/org/quiltmc/loader/impl/launch/server/QuiltServerLauncher.java
[qsl-tree]: https://github.com/QuiltMC/quilt-standard-libraries/tree/e6cbb6c9c1160d33bdf758572f4e325f051a7a03/library
[qsl-modinit]: https://github.com/QuiltMC/quilt-standard-libraries/blob/e6cbb6c9c1160d33bdf758572f4e325f051a7a03/library/core/qsl_base/src/main/java/org/quiltmc/qsl/base/api/entrypoint/ModInitializer.java
[qsl-client]: https://github.com/QuiltMC/quilt-standard-libraries/blob/e6cbb6c9c1160d33bdf758572f4e325f051a7a03/library/core/qsl_base/src/main/java/org/quiltmc/qsl/base/api/entrypoint/client/ClientModInitializer.java
[qsl-server]: https://github.com/QuiltMC/quilt-standard-libraries/blob/e6cbb6c9c1160d33bdf758572f4e325f051a7a03/library/core/qsl_base/src/main/java/org/quiltmc/qsl/base/api/entrypoint/server/DedicatedServerModInitializer.java
[qsl-event]: https://github.com/QuiltMC/quilt-standard-libraries/blob/e6cbb6c9c1160d33bdf758572f4e325f051a7a03/library/core/qsl_base/src/main/java/org/quiltmc/qsl/base/api/event/Event.java
[qsl-registry-events]: https://github.com/QuiltMC/quilt-standard-libraries/blob/e6cbb6c9c1160d33bdf758572f4e325f051a7a03/library/core/registry/src/main/java/org/quiltmc/qsl/registry/api/event/RegistryEvents.java
[qsl-networking]: https://github.com/QuiltMC/quilt-standard-libraries/blob/e6cbb6c9c1160d33bdf758572f4e325f051a7a03/library/core/networking/src/main/java/org/quiltmc/qsl/networking/api/ServerPlayNetworking.java
[qfapi-props]: https://github.com/QuiltMC/quilted-fabric-api/blob/9a6fea07586d32d4519b3fdbc0d79f2d8672c8b5/gradle.properties
[qfapi-tree]: https://github.com/QuiltMC/quilted-fabric-api/tree/9a6fea07586d32d4519b3fdbc0d79f2d8672c8b5
[quilt-blog-2024-07]: https://quiltmc.org/en/blog/2024-07-03-qfapi-moving-forward/
[qi-cli]: https://github.com/QuiltMC/quilt-installer/blob/4916835f50ec8c29be25838188fcf1667738810e/src/main/java/org/quiltmc/installer/CliInstaller.java
[qi-server]: https://github.com/QuiltMC/quilt-installer/blob/4916835f50ec8c29be25838188fcf1667738810e/src/main/java/org/quiltmc/installer/action/InstallServer.java
