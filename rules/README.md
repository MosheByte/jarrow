# Rule tables

Rule tables are the knowledge Jarrow uses to turn a Fabric mod into a NeoForge mod. Each entry says how one Fabric API shape (a call, an event, an entrypoint, a class) becomes a NeoForge shape, and how the two behave differently. The `rules` crate loads the tables and the `rewrite` crate applies them. Every application becomes an entry in the conversion report.

**Status: no rule table exists yet.** The tables and the `rules` crate arrive in milestone M2. Until then this directory holds only the source register and the license.

## Layout

```
rules/
  README.md          this file
  LICENSE            CC0-1.0 for everything under rules/
  SOURCES.md         every source a rule relies on, with license and access date
  fabric-neoforge/
    <game-version>/  one directory per pinned game version: 1.21.1, 1.21.11
      <area>.toml    one file per API area
```

- **One directory per game version.** The pinned game versions come from [`research-lock.json`](../research-lock.json). A rule that holds on both versions is written in both directories and checked against each version's own sources. Mojang renamed many classes in 1.21.11 (for example `ResourceLocation` became `Identifier`), so tables are never shared or copied across versions without checking. The reasons are in [`docs/research/fabric-neoforge-notes.md`](../docs/research/fabric-neoforge-notes.md).
- **One file per API area.** The file name is the Fabric API module without its `fabric-` prefix, for example `lifecycle-events-v1.toml`. Areas that are not a Fabric API module use plain names, such as `loader-entrypoints.toml` and `registries.toml`.
- **The format** is TOML. The table header, rule kinds, marks for symbols without a rule, validation and the full workflow for adding a rule are in **[`docs/rules.md`](../docs/rules.md)**.

## What may go in a table

- Public API shapes only: class names, member names, descriptors, event timing, cancellation and return-value semantics, JSON keys and manifest fields. They are learned from public documentation and the open-source API repositories listed in [`SOURCES.md`](SOURCES.md).
- Every rule cites at least one source id from `SOURCES.md`.
- Never code from a mod, decompiled or not.
- Never code from Sinytra Connector, Architectury or Forgified Fabric API. Connector's issue tracker may be cited by title to locate a known divergence, and the rule credits it.
- Never text copied from sources under share-alike or non-commercial terms (for example the Fabric documentation, CC BY-NC-SA 4.0). Restate the fact instead.

## License

Everything under `rules/` is dedicated to the public domain under **CC0 1.0 Universal** (SPDX `CC0-1.0`), so anyone can reuse the knowledge. See [`LICENSE`](LICENSE) and the legal code at https://creativecommons.org/publicdomain/zero/1.0/legalcode. The rest of the repository is MIT.

## Changing what the tables are checked against

The tables are checked against the loader, Fabric API and NeoForge versions pinned in `research-lock.json`. Run `py -3.12 tools/research_lock.py --verify` to re-check those pins against their publishers. Changing a pin needs a `D-` entry in `DECISIONS.md` (`tools/pr_ready.py` enforces this), and every table for that game version must be re-checked.
