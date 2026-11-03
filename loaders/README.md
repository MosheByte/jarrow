# Loader profiles

One TOML file per mod loader. Jarrow's one engine reads the profile of a direction's source loader and the profile of its target loader, so no loader is hard-coded in the code (D-029). The format is defined in [docs/loaders.md](../docs/loaders.md).

| File | Loader | 1.20.1 | 1.21.1 | 1.21.11 |
| --- | --- | --- | --- | --- |
| [`fabric.toml`](fabric.toml) | Fabric | partial | researched | researched |
| [`neoforge.toml`](neoforge.toml) | NeoForge | partial | partial | partial |
| [`forge.toml`](forge.toml) | Forge | partial | partial | partial |
| [`quilt.toml`](quilt.toml) | Quilt | partial | partial | partial |

"Partial" means some fields are still `"unknown"`, and how much differs a lot:

- **NeoForge on 1.21.1 and 1.21.11:** only the exact mod-id pattern and whether MixinExtras ships.
- **Fabric on 1.20.1:** the loader is the same 0.20.1, so its own facts carry over; what depends on Fabric API 0.92.12 (events, registration, resource conditions, networking) is not researched yet.
- **Forge everywhere, NeoForge and Quilt on 1.20.1, Quilt on the 1.21 pins:** [`research-lock.json`](../research-lock.json) gives the pinned build, the runtime names, the Mixin build, the server install and the API library. The mod-facing API (manifest details, entrypoints, events, registration, access files, resource conditions, networking) is not researched yet.

[`docs/research/loaders.md`](../docs/research/loaders.md), read on 2026-11-03, has sourced answers for many of these `"unknown"` fields (mod-id rules, entrypoints, event buses, registration, Mixin configs, the Modrinth loader tags). They are not copied into the profiles yet, so the profiles still say `"unknown"` there.

## Rules

- **No fact without research.** Every value comes from [`research-lock.json`](../research-lock.json) or [`docs/research/`](../docs/research/), and each table names its source.
- **"unknown" is a value.** A fact that has not been researched is written as `"unknown"`, with a comment naming the research still needed. Nothing is guessed and no field is left out.
- **One table per pinned game version.** Even where two versions agree, each is checked against its own sources, as with the rule tables.
- **Profiles are data, not code.** No crate reads them yet. The planned `crates/loaders` (milestone M1) validates them as described in [docs/loaders.md](../docs/loaders.md#validation).

To fill a field: write the fact with its source in `docs/research/`, copy it here, set `source`, update `status`, and update the direction pages in [`docs/directions/`](../docs/directions/) that use the loader.
