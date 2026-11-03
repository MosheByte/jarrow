# The harness probe

**Status: built in M1, not yet run on a live server.** The probe compiles for Fabric and NeoForge on Minecraft 1.21.1 and 1.21.11, and its JSON writer, log capture and file handling have unit tests. It has never dumped a real server: starting one needs the owner's Minecraft EULA acceptance ([harness.md](harness.md), "The EULA gate"), and until the harness runs it in CI with the gate open, no dump exists and nothing is measured.

The probe is a small server-side mod, one jar per loader and game version, that the harness ([harness.md](harness.md), `crates/harness`) puts into each server it starts. When the server has started, the probe writes everything the harness compares into `jarrow-dump.json` and stops the server. The harness then diffs the dump of the original Fabric mod against the dump of the converted mod.

It lives in `harness/probe/`, is MIT licensed like the rest of Jarrow, and its mod id is `jarrow_probe`.

## What it dumps, and why

`jarrow-dump.json` is written into the server directory. It is one JSON object. Object keys are sorted (by Unicode code point, which is the byte order of their UTF-8 form, so Rust's `BTreeMap` reads them back in the same order), the file is UTF-8, pretty-printed with two-space indentation, and ends with a newline. The field names are those of `harness/dump-schema.json`, which is written together with `crates/harness`.

```json
{
  "schema": 1,
  "loader": "fabric",
  "loader_version": "0.20.1",
  "game_version": "1.21.1",
  "mods": [{ "id": "fabricloader", "version": "0.20.1" }],
  "registries": { "minecraft:block": ["minecraft:air", "minecraft:stone"] },
  "tags": { "minecraft:block": { "minecraft:logs": ["minecraft:acacia_log"] } },
  "recipes": { "minecraft:oak_planks": { "type": "minecraft:crafting_shapeless" } },
  "loot_tables": { "minecraft:blocks/stone": { "type": "minecraft:block" } },
  "advancements": { "minecraft:story/root": { "criteria": {} } },
  "worldgen": {
    "seed": 0,
    "chunks": [[0, 0], [8, 8], [-8, 8], [8, -8]],
    "block_histogram": { "minecraft:stone": 0 }
  },
  "log": { "errors": [], "warnings": [] },
  "errors": []
}
```

The values above show the shape only; they are not from a real run.

| Field | Content | Why the harness needs it |
| --- | --- | --- |
| `schema` | `1`, the format version | Readers refuse a format they do not know |
| `loader` | `fabric` or `neoforge` (later `forge`, `quilt`) | Which side of the diff this is |
| `loader_version`, `game_version` | As the loader reports them | The report records the exact versions used |
| `mods` | Every loaded mod as `{"id", "version"}`, sorted by id | Shows whether the mod under test loaded at all |
| `registries` | For every registry in `server.registryAccess()`, static and dynamic: the entry ids in the registry's iteration order (its id order) | Missing, extra and reordered blocks, items, entities, biomes and the rest |
| `tags` | For every registry (also those without tags): tag id to its member ids, sorted. Tag ids have no `#`. A member without a key is written `<direct holder>` | Tag contents decide recipes, loot and behaviour |
| `recipes` | Recipe id to the recipe as its codec encodes it (`Recipe.CODEC` with registry-aware JSON ops) | Data-driven content in its canonical form |
| `loot_tables` | Loot table id to `LootTable.DIRECT_CODEC` JSON | Same |
| `advancements` | Advancement id to `Advancement.CODEC` JSON | Same |
| `worldgen` | The overworld seed, the sampled chunks (chunk coordinates, in this order) and a histogram of block states over every section of those four chunks. Keys are block state strings such as `minecraft:oak_log[axis=y]` | Mods that add ores, features or biomes change the histogram |
| `log` | WARN lines in `warnings`, ERROR and FATAL lines in `errors`, each `[<logger>] <message>`, plus ` (<exception>)` when the event carried one | Load failures that do not crash the server |
| `errors` | Exception text for anything that failed while dumping, as `<section>: <id>: <message>` | A broken dump says so instead of looking empty |

Details that matter when reading a dump:

- **Failures are recorded, never hidden.** Each section is collected on its own. If one fails, it is left empty or partial and the exception goes into `errors`; the other sections are still written. A recipe, loot table or advancement that its codec cannot encode is left out of its map and listed in `errors`.
- **The log is what happened after the probe loaded.** The probe attaches a Log4j appender when the loader constructs it, so anything logged earlier (most of the loader's own start-up) is not in `log`. The harness reads the full server log for that. Timestamps and thread names are left out so two runs compare equal. Each list keeps at most 5,000 lines; the rest are counted in a final line. If log4j-core is not reachable from the probe, `log` stays empty and `errors` says why.
- **Numbers** are written as the codec produced them. JSON has no NaN or infinity; such a value is written as a string (`"NaN"`).
- **No stale dumps.** The probe deletes an existing `jarrow-dump.json` before it starts dumping, and writes the new one to `jarrow-dump.json.tmp` first and renames it, so the harness never reads a partial or old file.

What the probe does not dump: client-only state (rendering, screens, sounds), creative tabs and resolved data-pack conditions. [harness.md](harness.md) lists creative tabs and conditions as planned dump content; they are not built yet, and a report that depends on them says "not measured".

## When it runs, and how it stops

| | Fabric | NeoForge |
| --- | --- | --- |
| Loaded on | Dedicated servers only (`"environment": "server"`) | Dedicated servers only (`@Mod(dist = Dist.DEDICATED_SERVER)`) |
| Set-up | `ModInitializer.onInitialize` installs the log capture and registers the listener | The mod constructor does the same |
| Trigger | `ServerLifecycleEvents.SERVER_STARTED` (Fabric API `fabric-lifecycle-events-v1`) | `ServerStartedEvent` on `NeoForge.EVENT_BUS` |
| After the dump | `server.halt(false)` | `server.halt(false)` |

The dump runs on the server thread, before the first tick. Generating the four sample chunks takes a few seconds. The server is halted in every case, also when the dump failed, so the harness never waits for a server that does not stop. The world is saved by the normal shutdown.

The loader-specific code is about 40 lines per loader: it builds the `LoaderInfo` (loader id and version, game version, mod list) and calls `ProbeDump.onServerStarted`. Everything else is the same source on both loaders.

## Layout

```
harness/probe/
  settings.gradle          one build, one subproject per loader (-Ploaders picks them)
  build.gradle             reads research-lock.json; shared compiler and jar settings
  gradle.properties        Loom and ModDevGradle versions, probe version
  common/src/main/java     the dump: ProbeDump, Json, LogCapture, Log4jHook, LoaderInfo
  common/src/mc-1.21.1/java
  common/src/mc-1.21.11/java
                           GameShim: the few calls whose names differ between game versions
  common/src/test/java     JUnit tests (run by the Fabric subproject)
  fabric/                  Fabric Loom build, fabric.mod.json, JarrowProbeFabric
  neoforge/                ModDevGradle build, neoforge.mods.toml, JarrowProbeNeoForge
  ci/baseline.sh           the baseline step of harness.yml
```

Both loader builds add `common/src/main/java` and `common/src/mc-<game version>/java` as source directories, so the dump code is identical on both loaders. `GameShim` absorbs what Mojang renamed between 1.21.1 and 1.21.11: `ResourceLocation` became `Identifier` (and `ResourceKey.location()` became `identifier()`), `Registry.getTags()` returns named holder sets instead of pairs, recipe ids became resource keys, and loot tables are listed through the reloadable registries' lookup.

## Versions

Nothing version-specific is written into the build scripts. The game version is an input, `-PgameVersion`, and `build.gradle` reads the rest from `research-lock.json` at the repository root:

| Value | Lock path (under `game_versions.<game version>`) |
| --- | --- |
| Java release | `mojang.java_major` (falls back to `jdk.major`) |
| Fabric Loader | `fabric.loader.version` |
| Fabric API (only `fabric-lifecycle-events-v1` is compiled against) | `fabric.api.version` |
| NeoForge | `neoforge.version` |
| `neoforge.mods.toml` loader and dependency ranges | `neoforge.mods_toml.*` |

A game version without pins, or a loader without pins for that version, stops the build with a message that names what is missing. Parchment is not used: the probe needs no parameter names.

The build tools are pinned in `harness/probe/gradle.properties` and `harness.yml`:

| Tool | Version | Why this one |
| --- | --- | --- |
| Gradle | 9.5.1 | Fabric Loom 1.17.21 declares Gradle 9.5.0 as its minimum plugin API; ModDevGradle 2.0.148 runs on Gradle 9 (the NeoForge 1.21.11 MDK uses 9.2.1) |
| Fabric Loom | 1.17.21 (plugin `net.fabricmc.fabric-loom-remap`) | The newest Loom that runs on Java 21. Loom 1.18 and later are compiled for Java 25, and the project pins JDK 21 |
| ModDevGradle | 2.0.148 | The newest release; the version the NeoForge 1.21.11 MDK uses |
| JUnit | 6.1.3 | Tests only |

The Fabric probe is compiled against Mojang's official names (`loom.officialMojangMappings()`) and remapped to intermediary like any Fabric mod; NeoForge runs on Mojang's names, so its jar is used as compiled. The NeoForge build sets `disableRecompilation = true`, so ModDevGradle never decompiles the game. Mojang's mappings are downloaded by the build tools at build time and are never committed.

## Building locally

You need JDK 21 and Gradle 9.5.1. The repository has no Gradle wrapper, because the wrapper is a `.jar` and no `.jar` is ever committed. Download Gradle once and check its published SHA-256:

```sh
curl -fLO https://services.gradle.org/distributions/gradle-9.5.1-bin.zip
curl -fL https://services.gradle.org/distributions/gradle-9.5.1-bin.zip.sha256
sha256sum gradle-9.5.1-bin.zip   # must print bafc141b619ad6350fd975fc903156dd5c151998cc8b058e8c1044ab5f7b031f
unzip gradle-9.5.1-bin.zip
```

Then, from the repository root:

```sh
gradle -p harness/probe -PgameVersion=1.21.1 build
gradle -p harness/probe -PgameVersion=1.21.11 build
gradle -p harness/probe -PgameVersion=1.21.11 -Ploaders=neoforge build   # one loader only
```

The jars land in `harness/probe/<loader>/build/libs/jarrow-probe-<loader>-<game version>-<probe version>.jar`. Archives are built without file timestamps and in a fixed entry order; a clean rebuild of both 1.21.11 jars on one machine gave identical SHA-256 hashes (not yet compared across machines). `build` also runs the unit tests. The first build for a game version downloads Minecraft, the loader and the mappings (a few hundred MB into the Gradle user home); later builds take seconds.

Gradle properties (all `-P<name>=<value>`):

| Property | Required | Meaning |
| --- | --- | --- |
| `gameVersion` | yes | The Minecraft version to build for. It must be pinned in `research-lock.json` and have a `common/src/mc-<version>/java` directory |
| `loaders` | no | Comma-separated subprojects to build, from `fabric` and `neoforge`. Default: every loader with a probe directory. An unknown loader stops the build |

Running the probe means running a Minecraft server, which needs the EULA gate. Use the harness for that, not the probe on its own.

## Adding a game version or a loader

- **A game version:** pin it in `research-lock.json` (with a `D-` entry), add `common/src/mc-<version>/java/.../GameShim.java` with the same methods, add the version to `PROBE_GAME_VERSIONS` in `harness.yml`, and build it for every loader.
- **A loader** (Forge and Quilt are not built yet): add `harness/probe/<loader>/` with its build script, metadata and an entry class that builds a `LoaderInfo` and calls `ProbeDump.onServerStarted`, add the loader's pins to the lock, and add the directory name to the list in `settings.gradle`.

## The workflow: `.github/workflows/harness.yml`

Runs on every pull request, every push to `main`, and on `workflow_dispatch`. Top-level permissions are `contents: read`; only `harness-ok` gets `actions: read`. A newer run for the same pull request cancels the older one. Every third-party action is pinned to a commit SHA.

| Job | Runs when | What it does |
| --- | --- | --- |
| `changes` | Always | Sets `run=true` on push and `workflow_dispatch`, and on pull requests whose diff (`base...head`) touches `crates/**`, `harness/**`, `rules/**` or `.github/workflows/harness.yml`; otherwise `run=false`. Also outputs the game versions (`PROBE_GAME_VERSIONS`) |
| `probes (<game version>)` | `run=true`; one leg per game version (1.21.1, 1.21.11) | Ubuntu, Temurin JDK 21, Gradle 9.5.1 through `gradle/actions/setup-gradle` with `cache-provider: basic` (the open-source cache; the default provider is a commercial service with its own terms). Runs `gradle -p harness/probe -PgameVersion=<version> build` (both probes and the tests), then uploads both jars and a `SHA256SUMS` file as the artifact `probes-<version>` (kept 14 days) and lists the hashes in the job summary |
| `baseline` | After `probes` succeeded, `run=true`, and the run is not a fork pull request and not Dependabot (they get no secrets) | Ubuntu. If `crates/harness/Cargo.toml` exists, builds `jarrow-harness` (`cargo build --locked --release -p jarrow-harness`); if not, it prints `crates/harness not on main yet` and goes on. Downloads every `probes-*` artifact into `probes/`, then runs `harness/probe/ci/baseline.sh` with `MINECRAFT_EULA_ACCEPTED` from the repository secret in its environment (never echoed). Uploads `baseline/**/*.json`, `*.md` and `*.log`, never a jar, as the artifact `baseline` (kept 14 days), also when the step failed |
| `harness-ok` | Always, after the jobs above | Reads every job of this run attempt through the API, like `ci-ok` in `ci.yml`, lists their conclusions in the job summary, and fails if any job's conclusion is not `success` or `skipped` (a failed or cancelled job, including a single matrix leg). This is the check to make required |

### What the baseline step records

`ci/baseline.sh` takes these flags:

| Flag | Meaning |
| --- | --- |
| `--harness <path>` | The `jarrow-harness` binary; empty when it was not built |
| `--probes <dir>` | The directory holding `probes-<game version>/` with that version's jars |
| `--out <dir>` | Where each game version gets `<dir>/<game version>/` |
| `--versions "<v> ..."` | The game versions, space-separated |

For each game version it decides, in this order:

| Condition | Status | Reason | Workflow result |
| --- | --- | --- | --- |
| No `jarrow-harness` binary | `not-run` | `harness_not_built` | passes |
| `jarrow-harness baseline --help` fails | `not-run` | `no_baseline_command` | passes |
| The help text does not list `--game-version` and `--out` | `not-run` | `harness_interface_mismatch` (also a warning annotation) | passes |
| The EULA gate is closed | `not-run` | `eula_not_accepted` | passes |
| No probe jars for the version | `failed` | `probes_missing` | fails |
| `jarrow-harness baseline --game-version <v> --out <dir>/<v>` (plus `--probe-dir probes/probes-<v>` when its help lists that flag; `JARROW_PROBE_DIR` is always set) exits 0 | `ran` | `ok` | passes |
| The same exits non-zero | `failed` | `harness_failed` | fails |

The gate is open when `MINECRAFT_EULA_ACCEPTED` or `MINECRAFT_EULA` is `true`, `yes`, `1` or `accepted`, ignoring case: the harness's own rule ([harness.md](harness.md)). The script checks it so it never asks the harness to start a server with the gate closed; the harness enforces the gate again itself.

Each version gets `<out>/<game version>/baseline-status.json`:

```json
{
  "detail": "not run: EULA not accepted (MINECRAFT_EULA_ACCEPTED is not true, yes, 1 or accepted)",
  "exit_code": null,
  "game_version": "1.21.1",
  "reason": "eula_not_accepted",
  "schema": 1,
  "status": "not-run"
}
```

The job summary shows the gate state, whether `jarrow-harness` was built, and one row per game version. When the gate is closed it says that the baseline was not run, that this is not a failure of the workflow, and that nothing is green without a harness result. A `not-run` baseline is never reported as a pass of the harness itself: it means nothing was measured.

The dumps and the noise report are whatever `jarrow-harness baseline` writes into `--out`; their format belongs to the harness crate. The upload pattern keeps JSON, Markdown and log files and leaves out every `.jar`, `libraries/` and `versions/` directory, because a server directory can hold the Minecraft server jar, which is never redistributed.
