# Sources for the rule tables

This file lists every source used to learn the Fabric and NeoForge APIs for Jarrow's rule tables (`rules/`), the research lock (`research-lock.json`) and the research notes ([`docs/research/fabric-neoforge-notes.md`](../docs/research/fabric-neoforge-notes.md) and [`docs/research/loaders.md`](../docs/research/loaders.md)). The Fabric and NeoForge sections also cover 1.20.1; Forge and Quilt have their own sections. Each entry gives its license, what it informs, and when it was read.

The format is fixed in [`docs/rules.md`](../docs/rules.md):
- Each source is a level-3 heading. The heading text is the id that rules cite in `sources = [...]`.
- Exactly four bullets follow: URL, License, Read, Used for.
- URLs are pinned to a tag or commit wherever the source is a repository.

The `rules` crate (M2) will reject a rule that cites an id missing from this file. No rule table exists yet.

## Clean-room statement

- **Rule tables describe API shapes, never mod bodies.** A table entry names public symbols: classes, method names and descriptors, event types, JSON keys, manifest fields. It says how a Fabric-side shape maps to a NeoForge-side shape. It never contains code from a mod, decompiled or otherwise, and never contains code from the loaders or their APIs.
- **No code from Sinytra Connector, Architectury or Forgified Fabric API** appears in `rules/` or `crates/`, and nobody on this project reads their source to write a rule. Their public issue trackers and docs may be cited; Connector's tracker is used for titles only, to locate known divergences. Those issues are listed below so rules can credit them.
- **The loaders are read, not copied.** Fabric Loader and Fabric API (Apache-2.0), NeoForge and FancyModLoader (LGPL-2.1), Forge (LGPL-2.1), and Quilt Loader, QSL and QFAPI (Apache-2.0) are read to learn their public API, lifecycle order and data formats. What we learn is restated in our own words and as symbol names. No source file, patch or doc paragraph is pasted into this repository.
- **Mapping files are never vendored.** Mojang's mappings are fetched from the version manifest at runtime, under their license header. Intermediary and Parchment (both CC0-1.0) are pinned by URL and hash in `research-lock.json`, not committed.
- **Rule tables are CC0-1.0** (see `rules/LICENSE`). So nothing under share-alike or non-commercial terms, such as the Fabric documentation (CC BY-NC-SA 4.0) or the Quilt RFCs (CC BY-SA 4.0), may be copied into them. Facts learned from those pages are restated, never quoted.

## Fabric

### fabric-loader/0.20.1
- URL: https://github.com/FabricMC/fabric-loader/tree/cb9523bc29ef775c50b6aef5e43bc56927cd8e67
- License: Apache-2.0
- Read: 2026-11-03
- Used for:
  - entrypoint interfaces and order (`Hooks`, `Knot`, `ModInitializer`, `PreLaunchEntrypoint`)
  - `fabric.mod.json` v1 parsing (`V1ModMetadataParser`) and the mod id pattern (`MetadataVerifier`)
  - side stripping (`Environment`, `EnvironmentStrippingData`)

### fabric-loader/launcher-json-0.20.1
- URL: https://maven.fabricmc.net/net/fabricmc/fabric-loader/0.20.1/fabric-loader-0.20.1.json
- License: Apache-2.0
- Read: 2026-11-03
- Used for: the Mixin and MixinExtras versions bundled with the pinned loader (`sponge-mixin 0.17.5+mixin.0.8.7`, `mixinextras-fabric 0.5.5`).

### fabric-api/1.21.1
- URL: https://github.com/FabricMC/fabric/tree/83c07162d88a703d15c8dae50cbbc12f4e9e63c3
- License: Apache-2.0
- Read: 2026-11-03
- Used for: the API surface for 1.21.1 (tag `0.116.17+1.21.1`):
  - events and phases (`Event`)
  - registry freezing (`BootstrapMixin`, `MainMixin`)
  - resource condition ids, fields and codec (`ResourceConditions`, `ResourceCondition`, `DefaultResourceConditionTypes`)
  - networking classes and handler thread (`ServerPlayNetworking`)
  - `UseBlockCallback`
  - the `fabric.mod.json` loader requirement

### fabric-api/1.21.11
- URL: https://github.com/FabricMC/fabric/tree/28e4019ae9dc6713d6797da19f60386debb25460
- License: Apache-2.0
- Read: 2026-11-03
- Used for: the same files as fabric-api/1.21.1 at tag `0.141.6+1.21.11`, compared version against version.

### fabric-api/1.20.1
- URL: https://github.com/FabricMC/fabric/tree/ba3f3da8411fb8dd27fa5eb3551d9ff775bb2c0a
- License: Apache-2.0
- Read: 2026-11-03
- Used for: the API surface for 1.20.1 (tag `0.92.12+1.20.1`):
  - the `fabric.mod.json` requirements (`fabricloader >=0.16.10`, `java >=17`)
  - event phases (`Event`, default phase `fabric:default`)
  - the registry bootstrap redirect (`BootstrapMixin`)
  - resource conditions as an array of objects keyed by `condition` (`ResourceConditions`)
  - channel networking with `PacketByteBuf`, `FabricPacket` and `PacketType` (`ServerPlayNetworking`)

### fabric-docs
- URL: https://github.com/FabricMC/fabric-docs/tree/2e637f701af37e6a5305f3ed11bbe3472073b0b0
- License: CC BY-NC-SA 4.0 (facts only, never copied into rules)
- Read: 2026-11-03
- Used for (pages rendered at https://docs.fabricmc.net):
  - the `fabric.mod.json` fields and version range syntax
  - Fabric Loader and mapping namespaces
  - access widening (1.21.11)
  - events (1.21.1)
  - porting and mappings (1.21.11)
  - resource conditions

### fabric-blog
- URL: https://fabricmc.net/blog/
- License: no license stated (all rights reserved; facts only)
- Read: 2026-11-03
- Used for: modder-facing changes per release, from these posts:
  - 1.21 / 1.21.1: https://fabricmc.net/2024/05/31/121.html
  - 1.21.2: https://fabricmc.net/2024/10/14/1212.html
  - 1.21.4: https://fabricmc.net/2024/12/02/1214.html
  - 1.21.5: https://fabricmc.net/2025/03/24/1215.html
  - 1.21.9: https://fabricmc.net/2025/09/23/1219.html
  - obfuscation: https://fabricmc.net/2025/10/31/obfuscation.html
  - 1.21.11: https://fabricmc.net/2025/12/05/12111.html

### fabric-meta
- URL: https://meta.fabricmc.net/v2/versions/loader
- License: service; each artefact carries its own license
- Read: 2026-11-03
- Used for:
  - stable loader, installer and intermediary versions per game version
  - the server launcher endpoint `/v2/versions/loader/{game}/{loader}/{installer}/server/jar`

### fabric-meta/1.20.1
- URL: https://meta.fabricmc.net/v2/versions/loader/1.20.1/0.20.1
- License: service; each artefact carries its own license
- Read: 2026-11-03
- Used for: the 1.20.1 launch profile for Fabric Loader 0.20.1 (intermediary 1.20.1, minimum Java 8) and the 1.20.1 server launcher endpoint.

### fabric-maven
- URL: https://maven.fabricmc.net/
- License: service; each artefact carries its own license
- Read: 2026-11-03
- Used for: the `.sha1` / `.sha256` / `.sha512` sidecars pinned in `research-lock.json` (loader, intermediary, installer, Fabric API).

### intermediary
- URL: https://github.com/FabricMC/intermediary
- License: CC0-1.0
- Read: 2026-11-03
- Used for:
  - intermediary names for 1.21.1 and 1.21.11 (tiny-v2 jars, by hash in the lock)
  - the `official` column means Mojang's obfuscated names

### fabric-access-widener
- URL: https://github.com/FabricMC/access-widener/blob/30e2d0038d2307140f122eebc2f782f440250234/src/main/java/net/fabricmc/accesswidener/AccessWidenerReader.java
- License: Apache-2.0
- Read: 2026-11-03
- Used for:
  - the access widener header (`accessWidener v1|v2 <namespace>`)
  - the directives `accessible`, `extendable`, `mutable`, and the `transitive-` prefix

### fabric-loom/mixin-ap-default
- URL: https://github.com/FabricMC/fabric-loom/blob/dev/1.12/src/main/java/net/fabricmc/loom/extension/MixinExtensionApiImpl.java
- License: MIT
- Read: 2026-11-03
- Used for: which mod jars carry a mixin refmap. The legacy annotation processor is on by default up to Loom 1.11 (`dev/1.7`, `dev/1.11`) and off from 1.12.

### fabric-installer
- URL: https://github.com/FabricMC/fabric-installer/tree/6e7d1acd6a951a19a323060ac955f5e7bc51e3ce
- License: Apache-2.0
- Read: 2026-11-03
- Used for:
  - server launcher behaviour: `install.properties`, the vanilla server sha1 check, the `fabric.installer.server.gameJar` property
  - server install flags (`-mcversion`, `-loader`, `-downloadMinecraft`)

## NeoForge

### neoforge/1.21.1
- URL: https://github.com/neoforged/NeoForge/tree/624c929f49920c250166e2a8df7af87f16d220ef
- License: LGPL-2.1
- Read: 2026-11-03
- Used for:
  - the registry freeze window and register order (`CommonModLoader`, `GameData`, `patches/.../MappedRegistry.java.patch`)
  - lifecycle stage order
  - payload registration events

### neoforge/1.21.11
- URL: https://github.com/neoforged/NeoForge/tree/0753a6a7dd9ed0bafc5441142206f33618f031b3
- License: LGPL-2.1
- Read: 2026-11-03
- Used for: the same files as neoforge/1.21.1, compared version against version, plus `RegisterClientPayloadHandlersEvent`.

### neoforge/1.20.1
- URL: https://github.com/neoforged/NeoForge/tree/a3317fc07a2b36793b8b87c19dd4b05265135d7c
- License: LGPL-2.1
- Read: 2026-11-03
- Used for: NeoForge for 1.20.1 (head of the `1.20.1` branch, last commit 2024-05-28; 47.1.106 is the newest build):
  - the `net.minecraftforge` package layout kept from Forge 47
  - the `forge:conditions` key (`ICondition`)
  - `NetworkRegistry.newSimpleChannel`

### neoforge-installer/1.20.1-47.1.106
- URL: https://maven.neoforged.net/releases/net/neoforged/forge/1.20.1-47.1.106/forge-1.20.1-47.1.106-installer.jar
- License: LGPL-2.1
- Read: 2026-11-03 (opened in a temp directory and deleted)
- Used for: from `install_profile.json`, `version.json` and `data/run.sh`:
  - the SRG rename of the vanilla server (MCP config merged with Mojang's mappings)
  - FancyModLoader 47.2.2, `eventbus 6.0.5`, `mixin 0.8.5`
  - the `run.sh` start line and the `BootstrapLauncher` main class

### neoforge-installer/21.1.265
- URL: https://maven.neoforged.net/releases/net/neoforged/neoforge/21.1.265/neoforge-21.1.265-installer.jar
- License: LGPL-2.1
- Read: 2026-11-03 (opened in a temp directory and deleted)
- Used for: from `install_profile.json` and `version.json`:
  - Mojang mappings applied to the vanilla server
  - FML 4.0.45
  - `sponge-mixin 0.16.4+mixin.0.8.7`
  - the launcher main class

### neoforge-installer/21.11.48
- URL: https://maven.neoforged.net/releases/net/neoforged/neoforge/21.11.48/neoforge-21.11.48-installer.jar
- License: LGPL-2.1
- Read: 2026-11-03 (opened in a temp directory and deleted)
- Used for: from `install_profile.json` and `version.json`:
  - the `PROCESS_MINECRAFT_JAR` step with Mojang mappings
  - FML 10.0.36
  - `sponge-mixin 0.16.5+mixin.0.8.7`
  - main class `net.neoforged.fml.startup.Client`

### neoforge-maven-api
- URL: https://maven.neoforged.net/api/maven/versions/releases/net/neoforged/neoforge
- License: service
- Read: 2026-11-03
- Used for: the NeoForge version list, the stable versus beta split, and the newest stable build per game version.

### neoforge-maven-api/forge
- URL: https://maven.neoforged.net/api/maven/versions/releases/net/neoforged/forge
- License: service
- Read: 2026-11-03
- Used for: the 1.20.1 build list (`1.20.1-47.1.N`), its newest build 47.1.106, and the stray `47.1.82` entry that has no artifact.

### neoforge-docs
- URL: https://github.com/neoforged/Documentation/tree/4a680cdd4d15013b65fd84998d1f4cd7e2885ae5
- License: MIT
- Read: 2026-11-03
- Used for: versioned docs 1.21.1 and 1.21.11 (rendered at https://docs.neoforged.net):
  - `neoforge.mods.toml`, `@Mod`
  - event buses and lifecycle
  - registries
  - access transformers
  - data load conditions
  - payloads
  - sides
  - Jar-in-Jar
  - server install
  - the list of versioned docs, which starts at 1.20.4 (no 1.20.1 version)

### neoforge-primers
- URL: https://github.com/neoforged/.github/tree/2bb6f9f3bdaec6efe2b69c3241b15fb47b64fc53/primers
- License: Unlicense
- Read: 2026-11-03
- Used for: vanilla changes from 1.21.2 to 1.21.11, including the 1.21.11 rename of `ResourceLocation` to `Identifier`.

### neoforge/discussion-199
- URL: https://github.com/neoforged/NeoForge/discussions/199
- License: no license stated (facts only)
- Read: 2026-11-03
- Used for: the 20.2 (1.20.2) announcement that SRG names were replaced by Mojang's mapping set.

### neoforged-news/20.5
- URL: https://neoforged.net/news/20.5release/
- License: no license stated (facts only)
- Read: 2026-11-03
- Used for:
  - `neoforge.mods.toml` became the only metadata file
  - data components
  - the payload, `StreamCodec` and `PayloadRegistrar` model
  - handlers run on the main thread by default

### neoforged-news/2025-retrospection
- URL: https://neoforged.net/news/2025-retrospection/
- License: no license stated (facts only)
- Read: 2026-11-03
- Used for:
  - 1.21.11 as the last obfuscated version and the `ResourceLocation` to `Identifier` rename
  - `javafml` as the default loader, `lowcodefml` removed
  - `@OnlyIn` stripping removed
  - the client payload handler event
  - recipe sync in 1.21.4

### neoforge-mdk/1.20.1
- URL: https://github.com/neoforged/MDK/tree/80aef8cefff5d43d54341d007376a2ed5150a74e
- License: no repository license detected; the template ships `TEMPLATE_LICENSE.txt` (MIT) for mods made from it (facts only)
- Read: 2026-11-03
- Used for: branch `1.20.1-legacy`: the `mods.toml` template (dependency on mod id `forge`), `loader_version_range=[47,)`, Parchment 2023.09.03, and the `@Mod` constructor with `FMLJavaModLoadingContext.get().getModEventBus()` and `DeferredRegister`.

### neoforge-mdk/1.21.1
- URL: https://github.com/NeoForgeMDKs/MDK-1.21.1-ModDevGradle/tree/e93d7f09d4eec12c76d1d7be9bf4eaad0246e9e0
- License: no license file detected (facts only)
- Read: 2026-11-03
- Used for: the reference `neoforge.mods.toml` template, `loader_version_range=[1,)`, the Minecraft and NeoForge ranges, and Parchment 2024.11.17.

### neoforge-mdk/1.21.11
- URL: https://github.com/NeoForgeMDKs/MDK-1.21.11-ModDevGradle/tree/c3fd175261fd672799dca21ef03387bfdc4eba44
- License: no license file detected (facts only)
- Read: 2026-11-03
- Used for: the reference `neoforge.mods.toml` template without `modLoader` and `loaderVersion`, the version ranges, and Parchment 2025.12.20.

### neoforge-accesstransformers/fmlat
- URL: https://github.com/neoforged/AccessTransformers/blob/4c92e3a66f8a5dc530e3984ac969ef8d73970c38/FMLAT.md
- License: MIT
- Read: 2026-11-03
- Used for: access transformer syntax, `+f` / `-f`, and the class, method and field directive forms.

## Forge

### forge/1.20.1-47.4.28
- URL: https://github.com/MinecraftForge/MinecraftForge/tree/d13dd327bdc75c010f3358992e80e7e92f51e030
- License: LGPL-2.1 (LICENSE.txt: LGPL 2.1 unless noted otherwise)
- Read: 2026-11-03
- Used for: the commit of the 47.4.28 changelog entry on branch `1.20.1`:
  - the MDK `mods.toml` template (`modLoader`, `loaderVersion`, dependency on `forge`)
  - access transformer lookup (`ModFile`: `accessTransformers` list, default `META-INF/accesstransformer.cfg`)
  - the `forge:conditions` key (`ICondition`)
  - `NetworkRegistry.newSimpleChannel`

### forge/1.21.1-52.1.20
- URL: https://github.com/MinecraftForge/MinecraftForge/tree/8199135df7b0ce1e321c5a59ac3e4e982f301812
- License: LGPL-2.1
- Read: 2026-11-03
- Used for: the commit of the 52.1.20 changelog entry on branch `1.21.1`:
  - the MDK `mods.toml` template and `ExampleMod` (constructor takes `FMLJavaModLoadingContext`)
  - access transformer lookup (`ModFile`)
  - the single `forge:condition` field (`ICondition`)
  - `ChannelBuilder` (`simpleChannel`, `eventNetworkChannel`, `payloadChannel`)

### forge/1.21.11-61.2.4
- URL: https://github.com/MinecraftForge/MinecraftForge/tree/94f2c3df41b209c9c0680d5c4cedc83916a5e1c3
- License: LGPL-2.1
- Read: 2026-11-03
- Used for: the commit of the 61.2.4 changelog entry on branch `1.21.11`:
  - EventBus 7 shapes: record events with a static `BUS` (`TickEvent`), `getModBusGroup()` (`FMLJavaModLoadingContext`), `getBus(modBusGroup)` (`FMLCommonSetupEvent`, `RegisterEvent`)
  - the MDK `mods.toml` template, `ModFile`, `ICondition`, `ChannelBuilder`, `DeferredRegister`

### forge-installer/1.20.1-47.4.28
- URL: https://maven.minecraftforge.net/net/minecraftforge/forge/1.20.1-47.4.28/forge-1.20.1-47.4.28-installer.jar
- License: LGPL-2.1
- Read: 2026-11-03 (opened in a temp directory and deleted)
- Used for: from `install_profile.json`, `version.json` and `data/run.sh`: the SRG rename of the vanilla server, `eventbus 6.2.33`, `mixin 0.8.5`, the `run.sh` start line and the `BootstrapLauncher` main class.

### forge-installer/1.21.1-52.1.20
- URL: https://maven.minecraftforge.net/net/minecraftforge/forge/1.21.1-52.1.20/forge-1.21.1-52.1.20-installer.jar
- License: LGPL-2.1
- Read: 2026-11-03 (opened in a temp directory and deleted)
- Used for: from `install_profile.json`, `version.json` and `data/run.sh`: the rename with Mojang's server mappings, `eventbus 6.2.33`, `mixin 0.8.7`, the shim jar start and the `ForgeBootstrap` main class.

### forge-installer/1.21.11-61.2.4
- URL: https://maven.minecraftforge.net/net/minecraftforge/forge/1.21.11-61.2.4/forge-1.21.11-61.2.4-installer.jar
- License: LGPL-2.1
- Read: 2026-11-03 (opened in a temp directory and deleted)
- Used for: from `install_profile.json`, `version.json` and `data/run.sh`: the rename with Mojang's server mappings, `eventbus 7.0.5`, `mixin 0.8.7`, `mixinextras-forge 0.5.4`, the shim jar start and the `ForgeBootstrap` main class.

### forge-promotions
- URL: https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json
- License: service
- Read: 2026-11-03
- Used for: the `recommended` and `latest` builds per game version (47.4.28, 52.1.20 and 61.2.4 recommended).

### forge-maven
- URL: https://maven.minecraftforge.net/
- License: service; each artefact carries its own license
- Read: 2026-11-03
- Used for: the installer `.sha1` / `.sha256` / `.sha512` sidecars pinned in `research-lock.json`, and the per-build changelogs that tie each build to a commit.

### forge-docs/1.20.1
- URL: https://github.com/MinecraftForge/Documentation/tree/2bd2b1e3cfdc899057ba57b992e157cdc2094ded
- License: MIT
- Read: 2026-11-03
- Used for: branch `1.20.1` (rendered at https://docs.minecraftforge.net): `mods.toml` keys and the mod id pattern, events and priorities, the lifecycle order, registries, access transformers (SRG names), conditional recipes. Also read for NeoForge 1.20.1, which shares the Forge 47 base.

### forge-docs/1.21.x
- URL: https://github.com/MinecraftForge/Documentation/tree/ac05e12aae903703cb8a9908a153623ce476a509
- License: MIT
- Read: 2026-11-03
- Used for: branch `1.21.x`: `clientSideOnly`, the `FMLJavaModLoadingContext` constructor parameter, the `MONITOR` priority, the lifecycle and registries. Its access transformer and SimpleImpl pages still describe SRG names and `newSimpleChannel`, which the 52.1.20 source and installer contradict.

### forge-eventbus/7.0
- URL: https://github.com/MinecraftForge/EventBus/tree/cab319a5fea8b205d8c8f205d6d55af2ba16126d
- License: LGPL-2.1
- Read: 2026-11-03
- Used for: the EventBus 7 model from the README at tag `7.0`: one `EventBus<T>` per event type, `RecordEvent`, compile-time validation.

### forge-mdkexamples
- URL: https://github.com/MinecraftForge/MDKExamples/tree/c0a20b210732654a261f9c040af9fe115d32137e
- License: no license file detected (facts only)
- Read: 2026-11-03
- Used for: mixin configs listed in the jar manifest's `MixinConfigs` attribute (`mixins-only/fg7/build.gradle`), and the README's note that obfuscated mod jars are needed for 1.20.4 and older.

## Quilt

### quilt-loader/0.30.2
- URL: https://github.com/QuiltMC/quilt-loader/tree/c36b50f62eebd9cd653a1f065596d3018700d9f0
- License: Apache-2.0
- Read: 2026-11-03
- Used for: tag `0.30.2`:
  - `gradle.properties`: `sponge-mixin 0.17.4+mixin.0.8.7`, MixinExtras 0.5.5, `class_tweaker 0.3.0-beta.2`
  - `quilt.mod.json`: provides `fabricloader` 0.19.5, depends on `mixinextras`
  - the production namespace `intermediary` (`MappingConfigurationImpl`)
  - the `main` / `client` / `server` entrypoint order (`Hooks`) and `fabric.mod.json` support (`FabricModMetadataReader`)
  - the server launcher's game jar lookup (`QuiltServerLauncher`)

### quilt-rfc/0002-quilt.mod.json
- URL: https://github.com/QuiltMC/rfcs/blob/ca458de2fef50da15d892e96be1165ce775255f2/specification/0002-quilt.mod.json.md
- License: CC BY-SA 4.0 (facts only, never copied into rules)
- Read: 2026-11-03
- Used for: the `quilt.mod.json` fields: `schema_version`, the mod id pattern, entrypoint notation, `intermediate_mappings`, `mixin`, `access_widener` and `minecraft.environment`.

### qsl/6.3.0+1.20.1
- URL: https://github.com/QuiltMC/quilt-standard-libraries/tree/e6cbb6c9c1160d33bdf758572f4e325f051a7a03
- License: Apache-2.0
- Read: 2026-11-03
- Used for: the commit that sets `QSL_VERSION = "6.3.0"` on branch `1.20`:
  - the `init` / `client_init` / `server_init` entrypoints (`qsl_base`)
  - `Event` with the `quilt:default` phase
  - `RegistryEvents`, `ServerPlayNetworking`
  - the module list, which has no resource conditions module

### qfapi/7.7.0+0.92.2-1.20.1
- URL: https://github.com/QuiltMC/quilted-fabric-api/tree/9a6fea07586d32d4519b3fdbc0d79f2d8672c8b5
- License: Apache-2.0
- Read: 2026-11-03
- Used for: the release commit on branch `1.20.1`: `gradle.properties` (version 7.7.0, upstream Fabric API 0.92.2, QSL 6.3.0+1.20.1) and the bundled Fabric modules (resource conditions, networking, registry sync, lifecycle events).

### quilt-installer/0.15.1
- URL: https://github.com/QuiltMC/quilt-installer/tree/4916835f50ec8c29be25838188fcf1667738810e
- License: Apache-2.0
- Read: 2026-11-03
- Used for: tag `0.15.1`: the `install server <game> <loader>` command line with `--install-dir` and `--download-server` (`CliInstaller`), and the `quilt-server-launch.jar` it writes (`InstallServer`).

### quilt-meta
- URL: https://meta.quiltmc.org/v3/versions/loader
- License: service; each artefact carries its own license
- Read: 2026-11-03
- Used for: Quilt Loader versions with sizes and hashes, the per-game profiles (`/v3/versions/loader/<game>`) that pair a loader with intermediary, and the installer list (`/v3/versions/installer`).

### quilt-maven
- URL: https://maven.quiltmc.org/repository/release/
- License: service; each artefact carries its own license
- Read: 2026-11-03
- Used for: the `.sha1` / `.sha256` / `.sha512` sidecars of Quilt Loader 0.30.2 and the Quilt installer 0.15.1, and the QFAPI maven artifact, which is a different file from the Modrinth one.

### quilt-blog/2024-07-03
- URL: https://quiltmc.org/en/blog/2024-07-03-qfapi-moving-forward/
- License: Apache-2.0 (the quiltmc.org site repository's LICENSE.md; facts only)
- Read: 2026-11-03
- Used for: from 1.20.6 on, QSL ships only `qsl_base` and `crash_info`, and Quilt modders are told to use the Fabric API equivalents.

## Shared tooling and data

### mojang/version-manifest
- URL: https://piston-meta.mojang.com/mc/game/version_manifest_v2.json
- License: Mojang / Microsoft terms (https://www.minecraft.net/en-us/eula)
- Read: 2026-11-03
- Used for:
  - the version JSON, server jar and mapping URLs, sha1s and sizes
  - the required Java version for 1.21.1 and 1.21.11

### mojang/version-1.20.1
- URL: https://piston-meta.mojang.com/v1/packages/72ae93b0934df5541860e3e6b72cf883c74da491/1.20.1.json
- License: Mojang / Microsoft terms (https://www.minecraft.net/en-us/eula)
- Read: 2026-11-03
- Used for: the 1.20.1 server jar and mapping URLs, sha1s and sizes, and Java 17.

### mojang/mappings
- URL: https://piston-data.mojang.com/v1/objects/03f8985492bda0afc0898465341eb0acef35f570/server.txt
- License: the Mojang mappings header plus the Minecraft EULA. The header allows use for development and forbids redistributing the complete unmodified file, so Jarrow fetches mappings at runtime and never commits them.
- Read: 2026-11-03 (1.21.1 and 1.21.11 server mappings, read in a temp directory and deleted)
- Used for: the intermediary to Mojang name chain, for example `class_2960` is `ResourceLocation` on 1.21.1 and `Identifier` on 1.21.11.

### mixin/wiki-environment
- URL: https://github.com/SpongePowered/Mixin/wiki/Introduction-to-Mixins---The-Mixin-Environment
- License: MIT (Mixin project)
- Read: 2026-11-03
- Used for: the Mixin config keys `package`, `mixins`, `client`, `server`, `refmap`, `priority`, `plugin`, `required`, `minVersion`.

### mixin/0.8.5
- URL: https://github.com/SpongePowered/Mixin/tree/155314e6e91465dad727e621a569906a410cd6f4
- License: MIT
- Read: 2026-11-03
- Used for: tag `releases/0.8.5`: the `MixinConfigs` manifest attribute (`Constants`) that the platform agent reads from each jar (`MixinPlatformAgentDefault`), which is how Forge and NeoForge 1.20.1 mods list their configs.

### modrinth/api
- URL: https://docs.modrinth.com/api/
- License: Modrinth Terms of Service (https://modrinth.com/legal/terms). The API requires a uniquely identifying User-Agent and allows 300 requests per minute.
- Read: 2026-11-03
- Used for:
  - the Fabric API version and hashes (project `P7dR8mSH`)
  - the game-version ranking in `docs/research/game-versions.json`
  - the board's mod list ([`docs/board.md`](../docs/board.md))

### modrinth/qfapi
- URL: https://api.modrinth.com/v2/project/qvIfYCYJ/version
- License: Modrinth Terms of Service (https://modrinth.com/legal/terms)
- Read: 2026-11-03
- Used for: QFAPI 7.7.0+0.92.2-1.20.1 and its hashes, and the absence of any QFAPI or QSL release for 1.21.1 and 1.21.11.

### modrinth/loader-tags
- URL: https://api.modrinth.com/v2/tag/loader
- License: Modrinth Terms of Service (https://modrinth.com/legal/terms)
- Read: 2026-11-03
- Used for: the loader tags `fabric`, `neoforge`, `forge` and `quilt`.

### parchment
- URL: https://maven.parchmentmc.org/org/parchmentmc/data/
- License: CC0-1.0
- Read: 2026-11-03
- Used for: Parchment 2024.11.17 (1.21.1) and 2025.12.20 (1.21.11), for parameter names in Tier B only.

### parchment/1.20.1
- URL: https://maven.parchmentmc.org/org/parchmentmc/data/parchment-1.20.1/maven-metadata.xml
- License: CC0-1.0
- Read: 2026-11-03
- Used for: Parchment 2023.09.03 for 1.20.1 (the release in maven-metadata.xml), for parameter names in Tier B only.

### vineflower
- URL: https://repo1.maven.org/maven2/org/vineflower/vineflower/1.12.0/
- License: Apache-2.0
- Read: 2026-11-03
- Used for: the Tier B decompiler pin (1.12.0) and its Maven Central hashes.

### maven/version-ranges
- URL: https://maven.apache.org/enforcer/enforcer-rules/versionRanges.html
- License: Apache-2.0
- Read: 2026-11-03
- Used for: the version range syntax used by `neoforge.mods.toml` (bounds, exact versions, unions).

### misode/mcmeta
- URL: https://github.com/misode/mcmeta/blob/summary/versions/data.json
- License: no license declared (facts only)
- Read: 2026-11-03
- Used for: pack format numbers, for reference only (1.21.1 data 48 / resource 34; 1.21.11 data 94.1 / resource 75).

## Known divergences (Connector issue titles only)

Sinytra Connector's source was not read. These issue titles point at places where Fabric and NeoForge behave differently in practice. Rules that address one cite it here and credit Connector.

### connector-issue/1324
- URL: https://github.com/Sinytra/Connector/issues/1324
- License: title only, no content reused
- Read: 2026-11-03
- Used for: a Mixin injection failure while a refmap is in use.

### connector-issue/2294
- URL: https://github.com/Sinytra/Connector/issues/2294
- License: title only, no content reused
- Read: 2026-11-03
- Used for: a Mixin injection that finds no target, with no refmap.

### connector-issue/1355
- URL: https://github.com/Sinytra/Connector/issues/1355
- License: title only, no content reused
- Read: 2026-11-03
- Used for: a custom registry from `FabricRegistryBuilder` that was not synced.

### connector-issue/2076
- URL: https://github.com/Sinytra/Connector/issues/2076
- License: title only, no content reused
- Read: 2026-11-03
- Used for: translating item registries between loaders.

### connector-issue/1894
- URL: https://github.com/Sinytra/Connector/issues/1894
- License: title only, no content reused
- Read: 2026-11-03
- Used for: an access widener that does not apply to fields the loader patched.

### connector-issue/2287
- URL: https://github.com/Sinytra/Connector/issues/2287
- License: title only, no content reused
- Read: 2026-11-03
- Used for: a mod that uses Fabric API internal networking classes.

### connector-issue/2337
- URL: https://github.com/Sinytra/Connector/issues/2337
- License: title only, no content reused
- Read: 2026-11-03
- Used for: an entrypoint stage failure on 1.21.1.
