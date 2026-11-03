# Research

What Jarrow knows about its inputs before any converter code exists: which game versions to support, which exact loader, API and tool versions the harness runs, and how Fabric and NeoForge differ on those versions. Everything here was produced on 2026-11-03.

| File | What it is | How it is produced |
| --- | --- | --- |
| [`game-versions.json`](game-versions.json) | Ranking of the 1.21 minors by Fabric download volume on Modrinth. The top two are pinned: **1.21.1** and **1.21.11** | `py -3.12 tools/research_versions.py --out docs/research/game-versions.json` (Modrinth API, top 200 Fabric mods) |
| [`../../research-lock.json`](../../research-lock.json) | The single pinned source of truth per game version. It holds URLs plus publisher-provided hashes (sha1, sha256, sha512 as available) and the Java version for: the Mojang version JSON, server jar and mappings; Fabric Loader, intermediary, Fabric API, installer and server launcher; NeoForge; Parchment. Also Vineflower | Built once from live publisher metadata; nothing in it was typed by hand |
| [`fabric-neoforge-notes.md`](fabric-neoforge-notes.md) | Research notes for the engine and the harness, with a source link for every claim: entrypoints, `fabric.mod.json`, `@Mod` and event buses, registries and freezing, `neoforge.mods.toml`, access wideners versus access transformers, Mixin, resource conditions, mappings at runtime, networking, and the 1.21.x changes between the two pins | Written from official docs and the projects' public repositories (sources in [`rules/SOURCES.md`](../../rules/SOURCES.md)) |

## Checking the lock

```sh
py -3.12 tools/research_lock.py            # offline schema check (default, same as --check)
py -3.12 tools/research_lock.py --print    # what is pinned, per game version
py -3.12 tools/research_lock.py --verify   # re-fetch publisher metadata and compare every pin
py -3.12 tools/research_lock.py --verify --strict --json   # for CI: warnings fail, JSON output
```

`--verify` does four things:
- re-reads Mojang's version manifest and the pinned version JSONs, Fabric meta, the Fabric and NeoForge mavens, the Modrinth API, the Parchment maven and Maven Central
- checks that every pinned URL answers with the pinned size (HEAD, or a 1-byte ranged GET)
- checks every hash against the publisher's own `.sha*` file, manifest field or API value
- downloads only metadata and the roughly 180 KB generated Fabric server launcher, never a game jar or installer

What it reports:
- **WARN** (not a failure) for manifest drift, when Mojang re-publishes a version JSON under a new sha1.
- **WARN** if the generated launcher's observed hash changes.
- **INFO** when a newer upstream release exists.

The tests (`tools/tests/test_research_lock.py`) check the schema and run the `--verify` logic against fake publishers. They use no network.

## Rules for changing anything here

- `research-lock.json` changes only together with a `D-` entry in `DECISIONS.md`. `tools/pr_ready.py` refuses the pull request otherwise.
- After a re-lock:
  - run `--verify`
  - re-check the notes sections that cite the old versions
  - re-check every rule table under `rules/fabric-neoforge/<game-version>/`
- No mod jar, game jar, installer, mapping file or decompiled source is ever committed. Research that needs to look inside one does it in a temporary directory outside the repository and deletes it afterwards.
- Notes are written in our own words with a link per claim. Nothing is copied from Sinytra Connector, Architectury or Forgified Fabric API; Connector issue titles may be cited.
