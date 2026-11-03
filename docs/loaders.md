# Loader profiles

**Status: draft, read by code from milestone M1.** Four profiles exist as data in [`loaders/`](../loaders/). No crate reads them yet. Fabric is fully researched for 1.21.1 and 1.21.11, and NeoForge almost fully. For Forge, Quilt and every loader on 1.20.1, [`research-lock.json`](../research-lock.json) pins the versions and records the runtime names, the Mixin build and how a server is installed; [research/loaders.md](research/loaders.md) now has sourced answers for much of the rest, but they are not copied into the profiles yet, so those fields still say `"unknown"`. The table in [`loaders/README.md`](../loaders/README.md) shows the coverage.

Jarrow has one engine for all 12 directions (D-029). What differs between loaders is data, and it lives in one profile per loader: `loaders/fabric.toml`, `loaders/neoforge.toml`, `loaders/forge.toml` and `loaders/quilt.toml`. The engine reads the source profile and the target profile of a direction and never hard-codes a loader. This page defines the file format.

## What a profile records

For each loader and each pinned game version:

| Area | What it says | Used by |
| --- | --- | --- |
| Manifest | Path and format of the mod metadata file | `crates/manifest`, `crates/classify` |
| Mod id | Which ids the loader accepts | `crates/manifest` |
| Entrypoints | How the loader finds and starts a mod's code | `crates/rewrite` |
| Events | The event model: callback fields or an event bus, ordering, cancellation | `crates/rules`, `crates/rewrite` |
| Registration | When and how content is registered, and when registries freeze | `crates/rewrite` |
| Mappings | The names the game classes carry at runtime | `crates/remap` |
| Mixin | Who provides Mixin, which build, how configs are declared | `crates/manifest`, `crates/classify` |
| Access file | Access widener or access transformer, path and names | `crates/manifest` |
| Resources | The resource-condition key in data-pack JSON | `crates/manifest` |
| Networking | The payload API family | `crates/rules` |
| Harness | How the harness installs and starts a server | `crates/harness` |
| API library | The loader's API library, if it is separate from the loader | `crates/classify`, `crates/harness` |

## Where the facts come from

A profile never introduces a fact. Every value is copied from [`research-lock.json`](../research-lock.json) or from a research note in [`research/`](research/), and each table names its source. When the research changes, the profile changes in the same pull request.

A fact that has not been researched is written as the string `"unknown"`, with a TOML comment beside it that names the research still needed. A field is never left out and never guessed. Code that meets `"unknown"` stops and reports it; it never falls back to another loader's value.

## Format

TOML, one file per loader, named after the loader id. Unknown keys are errors.

### Top level

| Key | Type | Meaning |
| --- | --- | --- |
| `schema` | integer | Format version. This draft is `1` |
| `id` | string | Loader id: `fabric`, `neoforge`, `forge` or `quilt`. Must equal the file stem |
| `name` | string | Display name |
| `modrinth_loader` | string | The Modrinth loader tag the board's source list filters on ([board.md](board.md)), or `"unknown"`. A loader whose tag is unknown gets no source list |
| `lock_section` | string | The key under `game_versions.<version>` in `research-lock.json` that pins this loader |
| `research` | array of strings | Research notes this profile is built from |

### `[versions."<game-version>"]`

One table per pinned game version: `"1.20.1"`, `"1.21.1"` and `"1.21.11"`. All three are always present.

| Key | Type | Meaning |
| --- | --- | --- |
| `status` | string | `"researched"` (no field is unknown), `"partial"` (some are) or `"not-researched"` (all are) |
| `loader` | string | The pinned loader version, from `research-lock.json` |
| `java` | string | Java major version the pinned loader runs on |

Every leaf value below may be the string `"unknown"` instead of its usual type. Each sub-table has a `source` key: a repository path, with a heading anchor where one exists, or `"unknown"`.

### `[versions."<v>".manifest]`

| Key | Type | Meaning |
| --- | --- | --- |
| `path` | string | Path of the metadata file inside the JAR |
| `format` | string | `"json"` or `"toml"` |
| `schema` | string | Schema version or notes on mandatory keys for this version |

### `[versions."<v>".mod_id]`

| Key | Type | Meaning |
| --- | --- | --- |
| `rule` | string | The id rule in plain words |
| `pattern` | string | The exact pattern, as a regular expression, when the research has it |

### `[versions."<v>".entrypoints]`

| Key | Type | Meaning |
| --- | --- | --- |
| `model` | string | `"manifest-keys"` (the manifest names entrypoint classes) or `"mod-annotation"` (an annotated class is the entry) |
| `kinds` | array of strings | The entrypoint kinds the loader runs, in its own names |
| `order` | array of strings | The order the loader runs them, or its lifecycle stages |

### `[versions."<v>".events]`

| Key | Type | Meaning |
| --- | --- | --- |
| `model` | string | `"callback-fields"` (event objects held in static fields, listeners added with `register`) or `"event-bus"` (listeners on buses, by priority) |
| `ordering` | string | How listeners are ordered |
| `cancellation` | string | How a listener cancels or overrides a result |
| `summary` | string | The model in one or two sentences |

### `[versions."<v>".registration]`

| Key | Type | Meaning |
| --- | --- | --- |
| `model` | string | `"direct"` (register calls during init) or `"register-event"` (register inside the loader's registry events) |
| `freeze` | string | When registries stop accepting entries |
| `summary` | string | The model in one or two sentences |

### `[versions."<v>".mappings]`

| Key | Type | Meaning |
| --- | --- | --- |
| `runtime` | string | The namespace game classes carry at runtime, from the list below |

Namespace ids, shared by `crates/mappings` and `crates/remap`:

| Id | Names |
| --- | --- |
| `obf` | Mojang's obfuscated names. Intermediary files call this column `official`, which is not Mojang's mapped names ([research, section 1](research/fabric-neoforge-notes.md#1-names-at-runtime-mappings)) |
| `intermediary` | Fabric's intermediary names |
| `mojang` | Mojang's mapped names, from the ProGuard files in the version manifest |
| `srg` | SRG names for fields and methods, with Mojang names for classes, as the 1.20.1 Forge-family installers rename the game |
| `quilt-hashed` | Quilt's hashed names |

Which loader runs which namespace on which version is a profile fact:

| Loader | 1.20.1 | 1.21.1 and 1.21.11 |
| --- | --- | --- |
| Fabric | `intermediary` | `intermediary` |
| NeoForge | `srg` | `mojang` |
| Forge | `srg` | `mojang` |
| Quilt | `intermediary` | `intermediary` |

Sources: the research notes for Fabric and NeoForge on the 1.21 pins, and the `runtime_names` entries in `research-lock.json` for the rest. No pinned loader runs on `quilt-hashed`; whether any mod JAR ships those names is not researched yet.

### `[versions."<v>".mixin]`

| Key | Type | Meaning |
| --- | --- | --- |
| `service` | string | Who provides Mixin at runtime |
| `artifact` | string | The Mixin artifact and version that ships |
| `mixinextras` | string | The MixinExtras build that ships, or `"none"` |
| `configs` | string | How a mod declares its mixin configs |

### `[versions."<v>".access]`

| Key | Type | Meaning |
| --- | --- | --- |
| `format` | string | `"access-widener"` or `"access-transformer"` |
| `path` | string | Where the file lives or how the manifest names it |
| `namespace` | string | The namespace the file's names are in |

### `[versions."<v>".resources]`

| Key | Type | Meaning |
| --- | --- | --- |
| `condition_key` | string | The key at the JSON root that holds load conditions |
| `condition_id_field` | string | The field that names a condition's type |

### `[versions."<v>".networking]`

| Key | Type | Meaning |
| --- | --- | --- |
| `family` | string | Short id of the payload API family |
| `summary` | string | The registration and handler model |

### `[versions."<v>".harness]`

| Key | Type | Meaning |
| --- | --- | --- |
| `install` | string | `"server-launcher"` or `"installer"` |
| `lock` | string | The path in `research-lock.json` that holds the exact steps and hashes |
| `summary` | string | The install and start steps in one sentence |

### `[versions."<v>".api]`

| Key | Type | Meaning |
| --- | --- | --- |
| `name` | string | The API library's name |
| `separate` | boolean | `true` when it is a separate download from the loader |
| `artifact` | string | Maven coordinates of the pinned build |

## Validation

The planned `crates/loaders` (milestone M1) will refuse a profile when:

- `id` differs from the file stem, or `schema` is unknown to it;
- a pinned game version is missing, or a version that is not pinned appears;
- a table or key is missing, or a key is not in this page;
- a value is outside its allowed set and is not `"unknown"`;
- `status` disagrees with the values (`"researched"` with an unknown field, for example);
- `loader` differs from the pin in `research-lock.json` under `lock_section`.

The engine refuses a direction on a game version where either profile is `"not-researched"`. The harness refuses to provision a server whose `harness.install` is `"unknown"`. Both give the reason; neither guesses.

## Changing a profile

1. Research the fact and write it, with its source, in `docs/research/` (and pin any version in `research-lock.json` with a `D-` entry).
2. Copy it into the profile, set `source`, and update `status`.
3. Re-read the direction pages in [`directions/`](directions/) that use this loader, and update their tables of differences.
