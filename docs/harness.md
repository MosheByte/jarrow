# The parity harness

**Status: not built yet (milestone M1).** This page is the design. It becomes `crates/harness`. No mod has been measured in any direction, so no mod is green.

The harness is how Jarrow decides whether a conversion works. A conversion is green because the harness says so, never because the code compiled. It is built in M1, before the converter is good, so the converter is measured from its first day.

It serves all 12 directions between Fabric, NeoForge, Forge and Quilt (D-029). What differs per loader, such as how a server is installed, comes from the loader profiles in `loaders/` ([loaders.md](loaders.md)). The first direction it measures is Fabric → NeoForge.

## Servers for four loaders

The harness provisions a headless dedicated server for any of the four loaders, on any pinned game version, from that loader's profile (`harness` table) and the exact versions and hashes in `research-lock.json`:

| Loader | Install | Source |
| --- | --- | --- |
| Fabric | Fabric's server launcher with the verified Mojang server jar; fallback: the hash-pinned installer in server mode. Fabric API goes into `mods/` | [Research, section 13](research/fabric-neoforge-notes.md#13-harness-provisioning); steps per version in `research-lock.json` |
| NeoForge | The NeoForge installer with `--installServer`, then `run.sh` or `run.bat`. On 1.20.1 the installer is the Forge 47.1 fork's (`net.neoforged:forge`) | [Same](research/fabric-neoforge-notes.md#13-harness-provisioning) for 1.21; `research-lock.json` for 1.20.1 |
| Forge | The Forge installer with `--installServer`, then `run.sh` or `run.bat` | `research-lock.json` |
| Quilt | The Quilt installer in server mode, given the verified Mojang server jar as `server.jar`, then `quilt-server-launch.jar`. Its API library goes into `mods/`: Quilted Fabric API on 1.20.1, Fabric API on the 1.21 pins | `research-lock.json` |

Every loader is pinned, with its install steps, for all three game versions. Mojang's version JSON names Java 17 for 1.20.1 and Java 21 for the 1.21 pins. A profile field that is `"unknown"` stops the harness with that reason; it never borrows another loader's steps.

Every server uses the same world seed, `online-mode=false`, the same fixed `server.properties`, and no mods besides the mod under test, its loader's API library where that is separate, and the probe. The Minecraft server JAR is downloaded from Mojang's version manifest at run time and never committed.

## Probes, one per loader

A small probe mod for each loader, part of this repository and MIT licensed, runs after startup and writes one canonical dump format, whatever the loader:

- registries with ids and ordinal positions: blocks, items, entities, block entities, fluids, sounds, recipe serializers, loot functions, attributes, enchantments, biomes, features;
- recipes, tags, loot tables and advancements, as canonical JSON;
- data-pack and resource conditions, resolved;
- creative tabs and their contents;
- a worldgen sample: a fixed number of chunks at fixed coordinates, as a block-state histogram;
- the server log, with that loader's known noise filtered out.

| Probe | Built against | Status |
| --- | --- | --- |
| `probe-fabric` | Fabric Loader and Fabric API, a `main` entrypoint | Planned (M1) |
| `probe-neoforge` | NeoForge, an `@Mod` class | Planned (M1) |
| `probe-forge` | Forge | Waits for research into Forge's entrypoints and events |
| `probe-quilt` | Quilt | Waits for research into Quilt's entrypoints, and whether a Fabric probe already runs on it |

**Baselines.** Each loader adds content of its own (tags, registry entries, conditions), so two loaders never dump the same thing even without a mod. For every loader and game version, the harness first dumps a server with only the probe and the API library: the baseline. A mod is judged on what it adds over its loader's baseline. The probes themselves are checked by comparing baselines: entries that vanilla defines must dump identically on every loader, so a probe that drifts is caught before it scores a mod.

## One conversion

For one mod, one direction A → B and one pinned game version:

1. **Provision two servers.** Loader A with the original JAR; loader B with the converted JAR.
2. **Dump** both, and subtract each loader's baseline.
3. **Diff** what the mod added on A with what the converted mod added on B.
4. **Score** the result and attach a reason.

## The round-trip check

Rule tables come in inverse pairs, A → B with B → A ([rules.md](rules.md#inverse-pairs-and-the-round-trip)). CI tests each pair with a round trip:

1. Convert a mod built for A with the A → B tables, then convert the result back with the B → A tables.
2. Run the original and the round-tripped JAR, each on its own loader A server.
3. Diff the two dumps and score them with the same table as any conversion.

Both halves are needed: a round trip is only run for a pair whose two tables exist. The round-trip report lists every rule that was applied and every rule marked "not invertible", so a round trip that is not green can be traced to the rules behind it. A round-trip score never stands in for a direction's own score: the A → B half is always scored on its own, on a B server, too.

## Scoring

| Score | Meaning | Board colour |
| --- | --- | --- |
| `1.0` | Dumps are identical | Green |
| `0.75` | Only ordering or log-noise differences | Green |
| `0.5` | Missing or extra entries, under 1% of entries | Yellow |
| `0.25` | The mod loads but entries differ materially | Red |
| `0` | The converted mod fails to load | Red |

This mapping is decision D-024, the same for every direction and for round trips. Red results carry a reason taken from the log: class not found, mixin failure, registry conflict, crash, or another classified cause. The board aggregates those reasons per direction into "top red reasons", which is the work queue for that direction's rule tables.

A report for a mod the harness has not run on says `not-run`, never green. On the board such a mod counts as red, with the reason why it was not run (for example `no_converter` or `harness_not_run_eula`), so a column's totals always add up to its number of mods (D-024, [board.md](board.md)).

## What it cannot measure

The harness compares what a server can dump. It does not measure client-only behaviour such as rendering, screens, sounds as heard, or tick-by-tick timing. Where a result depends on those, the docs and the report say "not measured".

An optional client smoke test is planned for M4: a headless client of the target loader with a software renderer joins the server and takes a screenshot of the creative tab. The screenshot is kept as evidence. It is not scored. It starts with NeoForge, the first direction's target.

## The EULA gate

Running a Minecraft dedicated server requires accepting the Minecraft EULA. That is a terms acceptance only the project owner can give (amendment A-6 in [spec-amendments.md](spec-amendments.md)).

- The harness refuses to start any server, for any loader, unless `MINECRAFT_EULA_ACCEPTED` or `MINECRAFT_EULA` is `true`, `yes`, `1` or `accepted` (case-insensitive).
- In CI the value comes from the repository secret `MINECRAFT_EULA_ACCEPTED`, which the project owner created on 2026-11-03. The agent never sets it.
- Locally, a person who has read and accepted the [Minecraft EULA](https://www.minecraft.net/en-us/eula) sets the environment variable `MINECRAFT_EULA_ACCEPTED=true` for their own runs.
- The harness writes `eula=true` into a server directory only when the gate is open. It never writes it otherwise.
- With the gate closed, the harness exits with a clear message and the result is "not run: EULA not accepted", which is never green (decision D-013). No mod can be green until the owner opens the gate.

## Where it runs

- **Pull requests** that touch `crates/`, `rules/` or `loaders/` run the harness on a 25-mod smoke set for each direction that has a board column, plus the round-trip check for each rule-table pair the pull request touches (`.github/workflows/harness.yml`).
- **The nightly board** runs it on every mod of every direction column, sharded (see [architecture.md](architecture.md), "Board data flow").
- **Tier B jobs** run it before returning a result (M4).
- **Locally**, with network access and the Java version Mojang's version JSON names: 21 for the 1.21 pins, 17 for 1.20.1 (`research-lock.json`). The command line is settled in M1 and will be documented here.

## Outputs

Each run produces a structured result in the shared report format from `crates/report`: the direction, the score, the per-category differences, the classified reason, the exact versions used, and the hashes of the inputs. A round trip's result also names the pair and lists the rules marked "not invertible" that it applied. The result never contains decompiled code or the JAR itself.
