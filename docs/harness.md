# The parity harness

**Status: M1, `crates/harness` exists for Fabric and NeoForge.** The dump format, provisioning for Fabric and NeoForge, the EULA-gated runner, baselines, diff and score are built and tested offline. Forge and Quilt servers are not built yet (they will be driven by loader profiles), and the round-trip check is a design. The probe mods (`harness/probe`, [probes.md](probes.md)) and `.github/workflows/harness.yml` are a separate package. No live server has run through the harness yet, and until the converter exists (M2) no converted mod has been measured in any direction, so no mod is green.

The harness is how Jarrow decides whether a conversion works. A conversion is green because the harness says so, never because the code compiled. It is built in M1, before the converter is good, so the converter is measured from its first day.

It serves all 12 directions between Fabric, NeoForge, Forge and Quilt (D-029). What differs per loader, such as how a server is installed, comes from the loader profiles in `loaders/` ([loaders.md](loaders.md)). The first direction it measures is Fabric → NeoForge.

## Servers for four loaders

The harness is designed to provision a headless dedicated server for any of the four loaders, on any pinned game version, from that loader's profile (`harness` table) and the exact versions and hashes in `research-lock.json`:

| Loader | Install | Source |
| --- | --- | --- |
| Fabric | Fabric's server launcher with the verified Mojang server jar; fallback: the hash-pinned installer in server mode. Fabric API goes into `mods/` | [Research, section 13](research/fabric-neoforge-notes.md#13-harness-provisioning); steps per version in `research-lock.json` |
| NeoForge | The NeoForge installer with `--installServer`, then `run.sh` or `run.bat`. On 1.20.1 the installer is the Forge 47.1 fork's (`net.neoforged:forge`) | [Same](research/fabric-neoforge-notes.md#13-harness-provisioning) for 1.21; `research-lock.json` for 1.20.1 |
| Forge | The Forge installer with `--installServer`, then `run.sh` or `run.bat` | `research-lock.json` |
| Quilt | The Quilt installer in server mode, given the verified Mojang server jar as `server.jar`, then `quilt-server-launch.jar`. Its API library goes into `mods/`: Quilted Fabric API on 1.20.1, Fabric API on the 1.21 pins | `research-lock.json` |

Every loader is pinned, with its install steps, for all three game versions. `crates/harness` builds the Fabric and NeoForge installs today ([Provisioning](#provisioning-and-the-cache)); for Forge and Quilt it answers "not built yet". Mojang's version JSON names Java 17 for 1.20.1 and Java 21 for the 1.21 pins. A profile field that is `"unknown"` stops the harness with that reason; it never borrows another loader's steps.

Every server uses the same fixed `server.properties` (seed `20261103`, `online-mode=false`), and no mods besides the mod under test, its loader's API library where that is separate, and the probe. The Minecraft server JAR comes from Mojang's URL in the lock, SHA-1 checked, at run time. It is never committed.

## Probes, one per loader

A small probe mod for each loader, part of this repository and MIT licensed, runs after startup and writes one canonical dump, `jarrow-dump.json`, whatever the loader ([The probe dump](#the-probe-dump)):

- every static and dynamic registry, with entries in raw id order;
- tags, resolved and sorted;
- recipes, loot tables and advancements, encoded with the game's own codecs;
- a worldgen sample: the seed, the chunks sampled, and a block-state histogram;
- the server log's errors and warnings, and the probe's own errors.

| Probe | Built against | Status |
| --- | --- | --- |
| `probe-fabric` | Fabric Loader and Fabric API, a `main` entrypoint | Built (M1) for 1.21.1 and 1.21.11, `harness/probe` ([probes.md](probes.md)); not run on a live server yet |
| `probe-neoforge` | NeoForge, an `@Mod` class | Built (M1) for 1.21.1 and 1.21.11; not run on a live server yet |
| `probe-forge` | Forge | Waits for research into Forge's entrypoints and events |
| `probe-quilt` | Quilt | Waits for research into Quilt's entrypoints, and whether a Fabric probe already runs on it |

No probe is built for 1.20.1 yet.

**Baselines.** Each loader adds content of its own (tags, registry entries, conditions), so two loaders never dump the same thing even without a mod. For every loader and game version, the harness first dumps a server with only the probe and the API library: the baseline. A mod is judged on what it adds over its loader's baseline. The probes themselves are checked by comparing baselines: entries that vanilla defines must dump identically on every loader, so a probe that drifts is caught before it scores a mod. How `crates/harness` learns and subtracts the noise between two loaders today is in [Loader noise and baselines](#loader-noise-and-baselines).

## One conversion

For one mod, one direction A → B and one pinned game version, with the loaders and the game version always given as inputs:

1. **Provision two headless dedicated servers** from the pins in `research-lock.json` ([Provisioning](#provisioning-and-the-cache)).
   - Reference: the original mod on loader A (for the first direction: Fabric Loader, Fabric API and the original JAR).
   - Candidate: the converted mod on loader B (for the first direction: NeoForge and the converted JAR).
2. **Dump** both with the probe.
3. **Diff** the canonical dumps of the two servers, then **subtract the loader noise** learned from a baseline pair: the same two loaders with only the probe ([Loader noise](#loader-noise-and-baselines)).
4. **Score** what is left and attach a reason ([Scoring](#scoring)).

## The round-trip check

Rule tables come in inverse pairs, A → B with B → A ([rules.md](rules.md#inverse-pairs-and-the-round-trip)). CI tests each pair with a round trip:

1. Convert a mod built for A with the A → B tables, then convert the result back with the B → A tables.
2. Run the original and the round-tripped JAR, each on its own loader A server.
3. Diff the two dumps and score them with the same table as any conversion.

Both halves are needed: a round trip is only run for a pair whose two tables exist. The round-trip report lists every rule that was applied and every rule marked "not invertible", so a round trip that is not green can be traced to the rules behind it. A round-trip score never stands in for a direction's own score: the A → B half is always scored on its own, on a B server, too.

## Scoring

| Score | Meaning | Board colour |
| --- | --- | --- |
| `1.0` | Dumps are identical once the loader noise is subtracted | Green |
| `0.75` | Only ordering or noise differences (registry order, worldgen drift, new warnings) | Green |
| `0.5` | Missing or extra entries, under 1% of entries | Yellow |
| `0.25` | The mod loads but entries differ materially | Red |
| `0` | The converted mod fails to load | Red |

This mapping is decision D-024, the same for every direction and for round trips. Red results carry a reason taken from the log: class not found, mixin failure, registry conflict, crash, or another classified cause. The board aggregates those reasons per direction into "top red reasons", which is the work queue for that direction's rule tables.

A report for a mod the harness has not run on says `not-run`, never green. On the board such a mod counts as red, with the reason why it was not run (for example `no_converter` or `harness_not_run_eula`), so a column's totals always add up to its number of mods (D-024, [board.md](board.md)).

How `crates/harness` (`score.rs`) decides the band, after canonicalisation and noise subtraction:

| Band | Exactly when |
| --- | --- |
| `0` | The candidate run failed (no valid dump), or a mod under test that loaded on the reference loader is missing from the candidate's mod list. Mod ids match after lower-casing and turning `-` into `_`, because NeoForge ids cannot contain `-`. |
| `0.25` | Any entry changed (a recipe, loot table or advancement with different content, a worldgen count that moved beyond drift), or missing plus extra entries are 1% or more of the mod's entries, or a new candidate error line is classified as `mixin_failure`, `class_not_found` or `registry_conflict`. |
| `0.5` | Missing or extra entries under 1% of the mod's entries, or new candidate error lines or probe errors with no classified cause. |
| `0.75` | Only registries whose shared entries are in a different order, worldgen drift, or new warnings. |
| `1.0` | Nothing left. |

- **Entries** are registry entries, tag members (an empty tag counts once), recipes, loot tables, advancements and worldgen block states.
- **The mod's entries** are the reference dump's entries minus the reference baseline's. Measured against the whole dump, vanilla's tens of thousands of entries would hide a mod that lost most of its own content. Without a baseline, the whole reference dump is the denominator.
- **Order** compares only the entries both registries hold, so entries a loader appends are not an order difference.
- **Worldgen drift.** The worldgen sample is not exactly repeatable. Two runs of the same Fabric server, same seed and same probe (the two `harness.yml` baselines of PR #14 on 2026-11-03) gave histograms that differed in 4 block states on 1.21.1 and 15 on 1.21.11: at most 32 blocks (granite, 3,768 against 3,800 of 393,216 sampled) or 10 on a small count (clay, 33 against 23), with a handful of mushrooms present in one run only. Every other part of the dumps was identical. A block-state count that moves by no more than 16 blocks, or 2% of the larger count, is therefore `drift` (0.75 band), also when the state is absent on one side. Beyond that it is a missing, extra or changed entry. A mod's worldgen change smaller than the drift band is not measured.
- **Log lines** are compared after normalisation (first line only, no log prefix, numbers and long hex ids replaced by `#`). Fewer lines in the candidate are never a difference.
- When the **reference** run failed or did not start, nothing was measured: the report is `not-run` with no score (`harness_load_failure` for a failed reference, the reference's own reason otherwise). Many mods need a library mod the harness does not provide; that must not count against the converter.

### Red reasons

The reasons are the board's closed `reason_kind` vocabulary ([board.md](board.md)). The harness produces these:

| `reason_kind` | When |
| --- | --- |
| `harness_not_run_eula` | Not run: the EULA gate is closed |
| `download_failed` | Not run: a pinned download failed or did not match its publisher hash |
| `mixin_failure` | Mixin markers (`MixinApplyError`, `InvalidInjectionException`, "Mixin apply failed", "Critical injection failure", the Mixin throwables packages) |
| `class_not_found` | `ClassNotFoundException`, `NoClassDefFoundError`, `NoSuchMethodError`, `NoSuchFieldError` |
| `registry_conflict` | "Registry is already frozen", "Adding duplicate key/value", "Unbound values in registry", "Duplicate registration", "already registered" |
| `harness_load_failure` | Loader load failures ("Mod loading has failed", "Incompatible mods found", missing mandatory dependencies), a dump that never appeared or broke the schema, a timeout, a mod under test that did not load |
| `crash` | Crash reports, "Exception in server tick loop", "Failed to start the minecraft server", JVM fatal errors, `OutOfMemoryError` |
| `entries_differ` | The server loaded and the dumps differ (0.5 and 0.25 without a classified log cause) |

When a log matches several, the most specific wins: `mixin_failure`, then `class_not_found`, `registry_conflict`, `harness_load_failure`, `crash`. Only ERROR and FATAL events, output written outside the logger (for example `Exception in thread "main"`) and crash reports are classified. A `ClassNotFoundException` logged at WARN or DEBUG, which loaders and mods do on purpose when they probe for optional classes, never makes a run red. The markers are hand-written substrings, tested on hand-written log excerpts in `crates/harness/src/logs.rs`.

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

In `crates/harness` the gate is checked before anything is downloaded, installed or started. A closed gate gives a `not-run` run record with `harness_not_run_eula` and exit status 3; nothing is downloaded, no Java runs, no server starts and no `eula.txt` exists anywhere. The tests in `runner.rs` and `tests/cli.rs` check exactly that. `provision` never starts a server, so it works with the gate closed.

## Where it runs

- **Pull requests** that touch `crates/`, `rules/` or `loaders/` run the harness on a 25-mod smoke set for each direction that has a board column, plus the round-trip check for each rule-table pair the pull request touches (`.github/workflows/harness.yml`, owned by the probe-mod package; the invocations it should run are [below](#what-harnessyml-runs)).
- **The nightly board** runs it on every mod of every direction column, sharded (see [architecture.md](architecture.md), "Board data flow"). It is not wired into the board yet: today every result there is `not-run` because no converter exists.
- **Tier B jobs** run it before returning a result (M4).
- **Locally**, with network access, the EULA variable set by a person who accepted it, and the Java version Mojang's version JSON names in `JAVA_HOME`: 21 for the 1.21 pins, 17 for 1.20.1 (D-034). `crates/harness` checks for at least the lock's `jdk.major`, 21, and has not run a 1.20.1 server yet. The commands are [below](#command-line).

## Outputs

Each run produces a structured result: the direction, the score, the per-category differences, the classified reason, the exact versions used, and the hashes of the inputs. A round trip's result also names the pair and lists the rules marked "not invertible" that it applied. The result never contains decompiled code or the JAR itself.

`crates/harness` writes three kinds of files. None of them holds a timestamp except the run record's duration, so the same inputs give the same report bytes.

| File | `record` | Written by | Holds |
| --- | --- | --- | --- |
| Run record (`<dump>.run.json`) | `jarrow-harness-run` | `run`, `baseline` | `status` (`completed`, `failed`, `not-run`), `reason_kind`, `reason`, the dump's path and SHA-256, every input's name and SHA-256 (probe, mods, loader mods), every pinned server download with its URL, SHA-256 and the hashes verified, the install method, the Java version, the exit code, the duration, the copied server log, classified findings, notes |
| Noise file (`schema` 2) | `jarrow-harness-noise` | `baseline` | The game version, each side's loader, loader version, mod ids and entry count, every difference between the two baselines, the differing JSON Pointers of every changed recipe, loot table and advancement, and every normalised log line either produced |
| Report | `jarrow-harness-report` | `diff` | `score` (or `null`), `outcome` (`green`, `yellow`, `red`, `not-run`, from `crates/report`), `reason_kind`, `reason`, both sides' status and dump hashes, the baseline's SHA-256, the counts behind the score, per-category counts, mods that did not load, the remaining differences (first 1,000, sorted, with the rest counted) and the classified findings |

The report types live in `crates/harness` (`HarnessReport`) and use `crates/report`'s `Outcome`; they move into the full report model when `crates/report` grows it.

## Command line

The binary is `jarrow-harness` (`cargo build --release -p jarrow-harness`, then `target/release/jarrow-harness`). `jarrow-harness --help` and `jarrow-harness <command> --help` print the same flags as below.

Exit status, for every command: `0` success; `1` a run failed (the server started but no valid dump came out), or `validate` found the dump invalid; `2` a usage or harness error (bad flags, missing files, a missing pin, no Java, an installer failure, inputs that cannot be compared, a baseline for other loaders); `3` not run (the EULA gate is closed, or a pinned download failed or did not match its hash).

Environment: `JAVA_HOME` (the JDK whose `bin/java` runs installers and servers; it must be at least the lock's `jdk.major`, 21), `MINECRAFT_EULA_ACCEPTED` or `MINECRAFT_EULA` (the gate). Nothing else is read.

Flags shared by `provision`, `run` and `baseline`:

| Flag | Default | Meaning |
| --- | --- | --- |
| `--cache <DIR>` | `target/jarrow-harness-cache` | Downloads, installed servers and run directories ([layout](#provisioning-and-the-cache)) |
| `--lock <FILE>` | the copy of `research-lock.json` built into the binary | The pins to provision from |

Flags shared by `run` and `baseline`:

| Flag | Default | Meaning |
| --- | --- | --- |
| `--timeout-secs <SECONDS>` | `360` | How long a server may take to write its dump before it is killed |
| `--xmx <SIZE>` | `3G` | The server's maximum heap (`-Xmx`) |

### `provision`

```sh
jarrow-harness provision --loader <LOADER> --game-version <VERSION> [--cache DIR] [--lock FILE]
```

Downloads the pinned files with hash verification and installs the server into the cache. It starts no server and does not need the EULA gate. NeoForge, and Fabric's fallback path, need `JAVA_HOME` to run the installer.

| Flag | Meaning |
| --- | --- |
| `--loader <LOADER>` | `fabric`, `neoforge`, `forge` or `quilt`. `forge` and `quilt` exit 2 with "not built yet" |
| `--game-version <VERSION>` | A game version pinned in the lock, for example `1.21.1` |

### `run`

```sh
jarrow-harness run --loader <LOADER> --game-version <VERSION> --probe <JAR> [--mods a.jar,b.jar] \
    --out <DUMP> [--record FILE] [--work DIR] [--timeout-secs N] [--xmx SIZE] [--cache DIR] [--lock FILE]
```

Runs one server with the probe and the mods under test and copies its dump to `--out`.

| Flag | Default | Meaning |
| --- | --- | --- |
| `--loader <LOADER>` | | `fabric`, `neoforge`, `forge` or `quilt` (`forge`, `quilt`: not built yet) |
| `--game-version <VERSION>` | | A pinned game version |
| `--probe <JAR>` | | The probe mod for this loader |
| `--mods <JARS>` | none | Mods under test, comma-separated or repeated. Omit for a probe-only run. A path that contains a comma must be given with its own `--mods` |
| `--out <FILE>` | | Where the dump is written (only when the run completed) |
| `--record <FILE>` | `--out` with `.run.json` | Where the run record is written (always). The server log is copied next to it as `<record>.log` |
| `--work <DIR>` | `<cache>/work/<loader>-<game version>` | The server directory. It is cleared before the run, so it must be new, empty, or one `jarrow-harness` made (it holds a `.jarrow-harness-work` marker); anything else is refused |

What a run does, in order: hash the inputs; read the pinned loader version; check the EULA gate (closed: write a `not-run` record and stop); check `JAVA_HOME` and `java -version`; provision (a download failure gives a `not-run` record with `download_failed`); build a fresh server directory from the installed template (hard links, falling back to copies); put the loader's own mods (Fabric API on Fabric), the probe and the mods under test into `mods/`; write the fixed `server.properties` and `eula.txt`; start `java -Xmx3G -Djava.awt.headless=true -Djarrow.dump=<absolute path of jarrow-dump.json> <launch> nogui`; wait for a complete `jarrow-dump.json`; send `stop` on the console (kill after 60 s); classify the log and any crash reports; validate the dump against the schema and check that its loader, game version and seed are the run's.

The fixed `server.properties` (only the port changes, to a free loopback port, so runs can share a machine): `difficulty=easy`, `enable-command-block=false`, `enable-query=false`, `enable-rcon=false`, `enforce-secure-profile=false`, `gamemode=survival`, `generate-structures=true`, `level-name=world`, `level-seed=20261103`, `max-players=1`, `motd=jarrow-harness`, `online-mode=false`, `server-ip=127.0.0.1`, `server-port=<free port>`, `simulation-distance=4`, `spawn-protection=0`, `sync-chunk-writes=true`, `view-distance=4`, `white-list=false`.

The server process gets a cleared environment with only `PATH`, `SystemRoot`, `SystemDrive`, `windir`, `ComSpec`, `TEMP`, `TMP`, `TMPDIR`, `HOME`, `USERPROFILE`, `APPDATA`, `LOCALAPPDATA`, `LANG`, `LC_ALL`, `TZ` and `NUMBER_OF_PROCESSORS`. Mods under test are third-party code; the harness's own secrets, the EULA variable included, never reach them. They do run with the harness user's file-system and network access, so CI must run them in a job that holds no other secrets.

### `baseline`

```sh
jarrow-harness baseline --game-version <VERSION> --probe-fabric <JAR> --probe-neoforge <JAR> \
    [--from fabric] [--to neoforge] [--out FILE|DIR] [--timeout-secs N] [--xmx SIZE] [--cache DIR] [--lock FILE]
jarrow-harness baseline --game-version <VERSION> --probe-dir <DIR> [--out FILE|DIR] ...
jarrow-harness baseline --game-version <VERSION> --reference-dump <FILE> --candidate-dump <FILE> [--out FILE|DIR]
```

Runs the `--from` loader and the `--to` loader with only their probe, one after the other, and learns the noise between them. The second form learns it from two existing baseline dumps (or completed run records) without starting anything, so a workflow can run the two loaders in separate jobs.

| Flag | Default | Meaning |
| --- | --- | --- |
| `--game-version <VERSION>` | | A pinned game version |
| `--from <LOADER>` | `fabric` | The reference loader (converted from) |
| `--to <LOADER>` | `neoforge` | The candidate loader (converted to) |
| `--probe-fabric <JAR>`, `--probe-neoforge <JAR>`, `--probe-forge <JAR>`, `--probe-quilt <JAR>` | | The probe for each loader. The `--from` and `--to` loaders each need one, here or through `--probe-dir` |
| `--probe-dir <DIR>` | | A directory with the probe jars as `harness/probe` names them, `jarrow-probe-<loader>-<game version>-<version>.jar` (source and javadoc jars are ignored). Used for each loader without its own `--probe-<loader>`; anything but exactly one match is an error |
| `--out <FILE\|DIR>` | `target/jarrow-harness/noise-<from>-<to>-<game version>.json` | The noise file when the path ends in `.json` and is not a directory; otherwise a directory that receives `noise.json`. The baseline dumps and run records are written next to the noise file as `<noise stem>.<loader>.json` and `<noise stem>.<loader>.run.json` (with `.run.log`); the runs use `<cache>/work/baseline-<loader>-<game version>` |
| `--reference-dump <FILE>` | | An existing reference baseline (dump or run record); needs `--candidate-dump` |
| `--candidate-dump <FILE>` | | An existing candidate baseline (dump or run record); needs `--reference-dump` |

Exit status: `0` when the noise file was written; otherwise `3` if either run did not start and `1` if one failed.

### `diff`

```sh
jarrow-harness diff <REFERENCE> <CANDIDATE> [--baseline <NOISE>] --out <REPORT>
```

Compares the reference run (the original mod on its loader) with the candidate run (the converted mod on the target loader), subtracts the noise and writes the report. Each side is a dump or a run record; a run record of a failed or not-run run gives a `0` or `not-run` report instead of a comparison, so a workflow can always pass the two run records. Prints `score <s> (<outcome>): <reason>` (or `not-run: <reason>`) and exits 0 when the report is written, whatever the score.

| Flag | Meaning |
| --- | --- |
| `<REFERENCE>`, `<CANDIDATE>` | Dumps or run records. A completed run record's dump is read relative to the record and must still have the recorded SHA-256 |
| `--baseline <FILE>` | The noise file for these two loaders, their exact versions and this game version; any mismatch exits 2 with "re-run `jarrow-harness baseline`". Without it, nothing is subtracted (for two runs on the same loader) |
| `--out <FILE>` | Where the report is written |

### `validate`

```sh
jarrow-harness validate <DUMP>
```

Checks a dump against `harness/dump-schema.json` and lists every problem with its JSON Pointer. Exit 0 when valid, 1 when not. Probe authors can run it on a dump without starting the rest of the harness.

### What `harness.yml` runs

The workflow belongs to the probe-mod package ([probes.md](probes.md)). Its `baseline` job builds `jarrow-harness` (`cargo build --locked --release -p jarrow-harness`), sets up Temurin 21 (which sets `JAVA_HOME`), downloads the probe jars into `probes/probes-<game version>/`, and runs `harness/probe/ci/baseline.sh` with `MINECRAFT_EULA_ACCEPTED` from the repository secret. When the gate is open, that script runs, per game version:

```sh
jarrow-harness baseline --game-version "$GV" --out "baseline/$GV" --probe-dir "probes/probes-$GV"
```

which provisions both servers into `target/jarrow-harness-cache`, runs Fabric and then NeoForge with only their probe, and writes `baseline/$GV/noise.json` with `noise.fabric.json`, `noise.neoforge.json`, their `.run.json` records and `.run.log` server logs next to it. Exit 0 means the noise file was written; the script counts anything else as a failed baseline. With the gate closed the script does not call the harness at all.

Once there is a smoke set and a converter (M2), these are the invocations per mod, after the baseline above (`$P_FABRIC` and `$P_NEO` are the probe jars):

```sh
H=target/release/jarrow-harness
OUT=baseline/$GV
$H run --loader fabric   --game-version "$GV" --probe "$P_FABRIC" --mods "mods/$MOD.jar"      --out "$OUT/$MOD/fabric.json"
$H run --loader neoforge --game-version "$GV" --probe "$P_NEO"    --mods "converted/$MOD.jar" --out "$OUT/$MOD/neoforge.json"
$H diff "$OUT/$MOD/fabric.run.json" "$OUT/$MOD/neoforge.run.json" --baseline "$OUT/noise.json" --out "$OUT/$MOD/report.json"
```

- Until the converter exists there is no converted JAR: skip the NeoForge mod run and the diff, and record `no_converter`.
- Treat exit statuses 1 and 3 from `run` as results (the run record says what happened), not as job failures; fail the job on 2. `diff` exits 0 whenever it wrote a report.
- Give `MINECRAFT_EULA_ACCEPTED` only to the steps that start servers, never on pull requests from forks and never when `github.actor == 'dependabot[bot]'`. Without it those steps exit 3 and every report is `not-run`, which is correct. Those steps must hold no other secret, because they run third-party mods.
- `provision` needs no EULA and can run first, so a cache of `target/jarrow-harness-cache` keyed on `hashFiles('research-lock.json')` saves the downloads.
- Upload the output directory as evidence: dumps, run records, server logs and reports. Never upload `target/jarrow-harness-cache`, which holds the Minecraft server jar.

## The probe dump

`harness/dump-schema.json` (JSON Schema 2020-12) is the contract between the probe mods and `crates/harness`; `jarrow-harness validate` checks a dump against it. [probes.md](probes.md) describes what the probe in `harness/probe` writes. What the schema cannot say, and what the harness relies on:

- **Where.** `jarrow-dump.json` in the server's working directory. The harness also passes `-Djarrow.dump=<absolute path>`, which names the same file, for a probe that prefers a property. The probe writes `jarrow-dump.json.tmp` and renames it; the harness also waits until the file parses as JSON, so a partial file is never read.
- **When.** After the server has started. The probe halts the server itself (`server.halt(false)`); the harness also sends `stop` on the console once the dump is complete, and kills the server 60 s later if it is still running.
- **`mods`**: every mod the loader lists, sorted by id, with the version the loader reports.
- **`registries`**: every registry of the server's registry access, static and dynamic, entry ids in iteration order (raw id order). Duplicates break the schema. The harness compares order only on shared entries.
- **`tags`**: per registry, every tag (id without `#`) with its resolved members, sorted. A member without a key is the literal `<direct holder>`; the harness sorts again and removes repeats, so a sorting slip does not change a score.
- **`recipes`, `loot_tables`, `advancements`**: id to the entry encoded with the game's own codec as JSON. The harness canonicalises (sorted keys, `1.0` equals `1`), so the codec's key order does not matter; array order does. A value JSON cannot hold (NaN) arrives as a string and is compared as one.
- **`worldgen`**: the overworld seed, the chunk coordinates sampled (a fixed list in the probe; the diff refuses two dumps whose chunk sets differ) and a histogram of block states written as `namespace:path` or `namespace:path[property=value,...]`. The harness fixes the seed to `20261103`; a dump with another seed fails the run. `seed` is `null` when the probe could not sample worldgen; the run still counts, the record notes it, and the empty histogram shows up in the diff as missing worldgen entries.
- **`log`**: one string per WARN (`warnings`) or ERROR and FATAL (`errors`) event after the probe loaded, written `[<logger>] <message>` plus ` (<exception>)`. The harness compares the first line after normalisation and classifies the text. For what happened before the probe loaded, the harness reads the full server log of a failed run.
- **`errors`**: problems the probe itself hit, as `<section>: <id>: <message>`. An empty list means the dump is complete.
- **`schema`** is `1`. Any incompatible change bumps it, together with `harness/dump-schema.json`, `crates/harness` and the probe.

## Provisioning and the cache

Every download is pinned in `research-lock.json` with its publisher's hashes. A file is streamed into the cache, hashed on the way, checked against its size and every pinned hash (SHA-1, SHA-256, SHA-512, whichever the publisher gives), and only then moved into place. A cached file is re-hashed before reuse and replaced if it no longer matches. A mismatch deletes the bytes and stops provisioning. Plain HTTP is refused except to the loopback interface (local test servers).

- **Fabric**: the Mojang server JAR (SHA-1 from Mojang's version JSON) is placed as `server.jar`; the meta.fabricmc.net server launcher is checked against the SHA-256 the lock records as observed (the publisher gives no hash file) and started with `-Dfabric.installer.server.gameJar=server.jar`, so it never downloads the game again. If the launcher's hash differs, the hash-pinned Fabric installer runs `server -dir <dir> -mcversion <v> -loader <v>` instead and the server starts from its `fabric-server-launch.jar`; the install record notes it. Fabric API (SHA-1 and SHA-512 from Modrinth) goes into every Fabric server's `mods/`. The launcher downloads Fabric Loader's own libraries from maven.fabricmc.net when the server starts; those are not pinned in the lock.
- **NeoForge**: the installer (SHA-1, SHA-256 and SHA-512 from the maven sidecars) runs `java -jar <installer> --installServer` in the server directory (the command in the lock and NeoForge's server guide). The server starts through the argument file that `run.sh` (`unix_args.txt`) or `run.bat` (`win_args.txt`) names, with the conventional `libraries/net/neoforged/neoforge/<version>/` path as the fallback. The installer downloads the game and its libraries itself and checks them against its own install profile.
- **Forge, Quilt**: not built yet. Their provisioning will come from loader profiles (`loaders/*.toml`); `provision` and `run` say so and exit 2.

```text
<cache>/downloads/<algorithm>-<16 hex>/<file>  verified downloads, keyed by the first 16 hex digits of their strongest pin
<cache>/servers/<loader>-<version>-mc<game>/    an installed server, the template for runs
    jarrow-install.json                          written last: launch method, files and hashes
<cache>/work/<loader>-<game>/                   the default run directory
<cache>/logs/                                    java -version output
```

An install is reused when its record matches the loader, loader version and game version, its launch target exists, and its loader mods still hash as recorded; otherwise it is removed and installed again.

## Loader noise and baselines

Two loaders differ even with no mod at all: NeoForge adds its own registries and attributes, Fabric API and NeoForge ship different conventional tags, and each loader logs its own warnings. A baseline pair (each loader with only the probe, same game version) shows those differences. `baseline` records them, with each side's mod list and entry count, in the noise file.

`diff` recognises a difference as noise when the baseline pair had the same difference (same category, kind and path), and a log line as noise when either baseline logged it. Noise is subtracted before scoring and counted in the report (`noise_subtracted`). A noise file applies only to the loaders, loader versions and game version it was learned on; anything else is refused rather than misapplied.

Changed recipes, loot tables and advancements are matched more finely. The first live baseline (PR #14, probe only) showed NeoForge encoding 1,138 of 1.21.1's vanilla loot tables and 302 of its recipes differently from Fabric: it names every loot pool (`"name": "main"`) and rewrites vanilla ingredients to conventional tags (`neoforge:difference` of `c:chests/wooden` and `c:chests/trapped`). Matching those entries by id alone would hide any change a mod makes to the same vanilla entries, for example a pool its converted version lost. So the noise file keeps, per changed entry, the JSON Pointers that differ, with array indices written `*` (`/pools/*/name`). A changed entry in a mod comparison is noise only when every pointer that differs in it is one of those. A pool on one side only adds `/pools#length` and is never noise.

The first live numbers, from the same baseline (Fabric 0.20.1 against NeoForge 21.1.265 and 21.11.48, probe only):

| | 1.21.1 | 1.21.11 |
| --- | --- | --- |
| Entries (Fabric / NeoForge) | 20,034 / 20,008 | 22,953 / 22,830 |
| Loot tables encoded differently | 1,138 | 1,281 |
| Recipes encoded differently | 302 | 487 |
| Registry entries only on NeoForge | 60 | 72 |
| Tag members only on Fabric / only on NeoForge | 153 / 67 | 264 / 68 |
| Worldgen block states that differ (all within drift) | 13 | 0 |

Without a baseline, the probe-only pair scores 0.25 on 1.21.1, so a cross-loader comparison is only meaningful with the noise subtracted.
