# The parity harness

**Status: not built yet (milestone M1).** This page is the design from [BUILD-BOOK.md](../BUILD-BOOK.md) section 6.1. It becomes `crates/harness`. No mod has been measured, so no mod is green.

The harness is how Jarrow decides whether a conversion works. A conversion is green because the harness says so, never because the code compiled. It is built in M1, before the converter is good, so the converter is measured from its first day.

## What it does

For one mod and one pinned game version (1.21.1 or 1.21.11):

1. **Provision two headless dedicated servers.**
   - Fabric: Fabric Loader + Fabric API + the original JAR.
   - NeoForge: NeoForge + the converted JAR.
   - Loader, API and NeoForge versions come from `research-lock.json`. The Minecraft server JAR is downloaded from Mojang's version manifest at run time and never committed.
   - Both servers use the same world seed, `online-mode=false`, no other mods, and the same fixed `server.properties`.
2. **Dump.** A small probe mod, one per loader, part of this repository and MIT licensed, runs after startup and writes:
   - registries with ids and ordinal positions: blocks, items, entities, block entities, fluids, sounds, recipe serializers, loot functions, attributes, enchantments, biomes, features;
   - recipes, tags, loot tables and advancements, as canonical JSON;
   - data-pack and resource conditions, resolved;
   - creative tabs and their contents;
   - a worldgen sample: a fixed number of chunks at fixed coordinates, as a block-state histogram;
   - the server log, with loader-specific noise filtered out.
3. **Diff** the canonical dumps of the two servers.
4. **Score** the result and attach a reason.

## Scoring

| Score | Meaning | Board colour |
| --- | --- | --- |
| `1.0` | Dumps are identical | Green |
| `0.75` | Only ordering or log-noise differences | Green |
| `0.5` | Missing or extra entries, under 1% of entries | Yellow |
| `0.25` | The mod loads but entries differ materially | Red |
| `0` | The converted mod fails to load | Red |

This mapping is decision D-024. Red results carry a reason taken from the log: class not found, mixin failure, registry conflict, crash, or another classified cause. The board aggregates those reasons into "top red reasons", which is the work queue for the rule tables.

A report for a mod the harness has not run on says `not-run`, never green. On the board such a mod counts as red, with the reason why it was not run (for example `no_converter` or `harness_not_run`), so the totals always add up to the number of mods (D-024).

## What it cannot measure

The harness compares what a server can dump. It does not measure client-only behaviour such as rendering, screens, sounds as heard, or tick-by-tick timing. Where a result depends on those, the docs and the report say "not measured".

An optional client smoke test is planned for M4: a headless NeoForge client with a software renderer joins the server and takes a screenshot of the creative tab. The screenshot is kept as evidence. It is not scored.

## The EULA gate

Running a Minecraft dedicated server requires accepting the Minecraft EULA. That is a terms acceptance only the project owner can give (amendment A-6 in [book-amendments.md](book-amendments.md)).

- The harness refuses to start any server unless `MINECRAFT_EULA_ACCEPTED` or `MINECRAFT_EULA` is `true`, `yes`, `1` or `accepted` (case-insensitive).
- In CI the value comes from the repository secret `MINECRAFT_EULA_ACCEPTED`, which the owner created on 2026-11-03. The agent never sets it.
- Locally, a person who has read and accepted the [Minecraft EULA](https://www.minecraft.net/en-us/eula) sets the environment variable `MINECRAFT_EULA_ACCEPTED=true` for their own runs.
- The harness writes `eula=true` into a server directory only when the gate is open. It never writes it otherwise.
- With the gate closed, the harness exits with a clear message and the result is "not run: EULA not accepted", which is never green (decision D-013). No mod can be green until the owner opens the gate.

## Where it runs

- **Pull requests** that touch `crates/` or `rules/` run the harness on a 25-mod smoke set (`.github/workflows/harness.yml`).
- **The nightly board** runs it on all 1,000 mods, sharded (see [architecture.md](architecture.md), "Board data flow").
- **Tier B jobs** run it before returning a result (M4).
- **Locally**, with JDK 21 and network access. The command line is settled in M1 and will be documented here.

## Outputs

Each run produces a structured result in the shared report format from `crates/report`: the score, the per-category differences, the classified reason, the exact versions used, and the hashes of the inputs. The result never contains decompiled code or the JAR itself.
