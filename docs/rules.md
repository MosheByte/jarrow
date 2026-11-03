# Rule tables

**Status: draft, implemented in M2.** No rule table exists yet, for any direction, and the `rules` crate (`crates/rules`) is not built. This page fixes a first draft of the format so that research, rule proposals and the crate start from the same shape. The format may change before M2 closes; when it does, this page changes in the same pull request.

## What a rule table is

A rule table says how one loader's API shape becomes another loader's shape, for one direction and one game version: which call, field, class or entrypoint to match in a mod's bytecode, what to replace it with, and how the two differ in behaviour. The `rules` crate loads and validates the tables; the `rewrite` crate applies them; every application becomes a report entry.

There is one engine for all 12 directions (D-029). The loaders' own facts (manifest, event model, runtime names and so on) live in the loader profiles ([loaders.md](loaders.md)); a rule table holds only what one API shape maps to in the other loader.

Rule tables describe API shapes only: class names, member names, descriptors, event timing, cancellation and return values, learned from public documentation and the open-source API repositories. They never contain code from a mod, and never code from Sinytra Connector, Architectury or Forgified Fabric API (see [CONTRIBUTING.md](../CONTRIBUTING.md), "Clean room"). Rule tables are CC0-1.0.

## Layout

```
rules/
  SOURCES.md                      every source any rule relies on, for every direction
  fabric-neoforge/                one directory per direction: <from>-<to>
    1.21.1/                       one directory per pinned game version
      lifecycle-events-v1.toml    one file per API area
      api-base.toml
      loader-entrypoints.toml
      registries.toml
      ...
    1.21.11/
      ...                         checked separately against that version's sources
  neoforge-fabric/                the inverse direction
    1.21.1/
      ...
```

- Tables live in `rules/<from>-<to>/<game-version>/<area>.toml`. `<from>` and `<to>` are loader ids with a profile in `loaders/`: `fabric`, `neoforge`, `forge`, `quilt`.
- One directory per pinned game version. A rule that holds for several versions is written in each directory, each checked against its own version's sources. Copying a rule across without checking is not allowed.
- One file per API area. In `fabric-neoforge`, the file stem is the Fabric API module name without the `fabric-` prefix: `fabric-lifecycle-events-v1` becomes `lifecycle-events-v1.toml`. Areas that are not a Fabric API module use a plain name: `loader-entrypoints.toml` for Fabric Loader entrypoints and `registries.toml` for vanilla registration. Every other direction sets its area names on its page in [directions/](directions/) before its first table.
- A direction's page ([directions/](directions/)) is written before any of its tables.

## Inverse pairs and the round trip

Tables come in inverse pairs: every A → B table has an inverse B → A table, or an explicit "not invertible" mark with a reason.

- **The pairing is written down twice.** The A → B table's header names its inverse, and the inverse's header names it back. Each rule names its inverse rule, or says why it has none.
- **A table and its inverse land in the same pull request.** The fixed build order decides when a direction gets its board column and its source list. It does not hold back an inverse table: the round trip needs both halves. So the first Fabric → NeoForge table arrives with its NeoForge → Fabric inverse, long before NeoForge → Fabric has a column of its own.
- **CI tests each pair by a round trip.** A mod built for loader A is converted A → B, then B → A, and the harness compares the result with the original on an A server and scores it with the usual scores ([harness.md](harness.md#the-round-trip-check)). The A → B half is also scored on its own, on a B server, as every conversion is.
- **"Not invertible" is a statement, not a gap.** A rule that is not invertible (Fabric event phases approximated with NeoForge priorities, for example) is listed in the round-trip report, so a round trip that is not green can be traced to it. A not-invertible mark never makes a round trip green.

## Names inside a rule

- Class names are JVM internal names: slashes for packages, `$` for nested classes (`net/fabricmc/fabric/api/event/lifecycle/v1/ServerTickEvents$EndTick`).
- Method and field types are JVM descriptors (`(Lnet/minecraft/server/MinecraftServer;)V`).
- Rules are matched after the first remap pass, so Minecraft classes appear under **Mojang names** (`net/minecraft/server/level/ServerLevel`), whatever the two loaders run on: never intermediary names (`net/minecraft/class_NNNN`), never SRG or Quilt hashed names, and never Yarn names. The engine remaps the source into Mojang names, applies the rules, and remaps into the target's runtime names last ([architecture.md](architecture.md#conversion-data-flow)). Loader and API classes are not remapped and appear as published.
- A **symbol** (used by `[[mark]]`) is written `owner` for a class, `owner.name:descriptor` for a field, and `owner.name(descriptor)` for a method, for example `net/fabricmc/fabric/api/event/Event.register(Ljava/lang/Object;)V`.

## File format

TOML. Unknown keys are errors, so a typo fails loudly instead of being ignored.

### The `[table]` header

| Key | Type | Required | Meaning |
| --- | --- | --- | --- |
| `schema` | integer | yes | Format version. This draft is `1`. The crate refuses a schema it does not know |
| `area` | string | yes | Must equal the file stem |
| `game_version` | string | yes | Must equal the directory name, and be one of the versions pinned in `research-lock.json` |
| `from` | string | yes | The source loader id, for example `"fabric"`. Must equal the first part of the direction directory |
| `to` | string | yes | The target loader id, for example `"neoforge"`. Must equal the second part |
| `names` | string | yes | `"mojang"`, the only namespace rules are written in. (An earlier draft called it `"official"`, which is the intermediary files' name for the obfuscated names, so it was renamed.) |
| `summary` | string | yes | One line: what API area the file covers |
| `inverse` | string | one of these two | The inverse table, as `<to>-<from>/<game-version>/<area>` |
| `not_invertible` | string | one of these two | Why this whole area has no inverse, in one sentence |

Loader and API versions are not repeated in the tables. They come from `research-lock.json` for the game version, through the loader profiles, so they can never drift apart.

### Keys every `[[rule]]` has

| Key | Type | Required | Meaning |
| --- | --- | --- | --- |
| `id` | string | yes | `<area>/<name>`, lowercase with hyphens. Unique within the game-version directory and never reused, because reports and telemetry refer to it |
| `kind` | string | yes | `"event_bridge"`, `"call_rewrite"`, `"class_substitution"` or `"entrypoint"` |
| `summary` | string | yes | One sentence: what becomes what |
| `confidence` | string | yes | `"exact"` (same behaviour by construction), `"equivalent"` (same behaviour per the sources, with the differences listed), `"approximate"` (known differences that can matter). Copied into each report entry |
| `sources` | array of strings | yes, at least one | Ids of entries in `rules/SOURCES.md` |
| `inverse` | string | one of these two | The `id` of the rule in the inverse table that undoes this one |
| `not_invertible` | string | one of these two | Why no rule can undo this one, in one sentence. Shown in round-trip reports |
| `[rule.match]` | table | yes | What to find. Keys depend on `kind` |
| `[rule.target]` | table | yes | What to produce. Keys depend on `kind` |
| `[[rule.difference]]` | array of tables | yes for `event_bridge`, at least one | Documented semantic differences |

Each `[[rule.difference]]` has:

| Key | Type | Meaning |
| --- | --- | --- |
| `topic` | string | `"timing"`, `"ordering"`, `"cancellation"`, `"return-value"`, `"arguments"`, `"side"`, `"phase"` or `"none"` |
| `text` | string | The difference in plain words, or for `"none"`, what was compared and found equal |

### Keys per kind

The match and target keys below are written for the first direction: a source whose events are held in callback fields and a target with an event bus (`events.model` in the two profiles), Fabric → NeoForge. Each other combination of models gets its keys here, and on its direction page, before its first table. Until then a table for it is refused.

### `kind = "event_bridge"`

A Fabric API `Event<T>` field plus a `register` call becomes a listener on a NeoForge event bus.

`[rule.match]`

| Key | Type | Meaning |
| --- | --- | --- |
| `owner` | string | Class that declares the static event field |
| `field` | string | Name of the static field of type `net/fabricmc/fabric/api/event/Event` |
| `callback` | string | The callback interface (the `T` in `Event<T>`) |
| `method` | string | The callback's single abstract method |
| `descriptor` | string | That method's descriptor |
| `register` | array of strings | `Event` methods that count as registration, as `name(descriptor)`. Draft v1 allows only `"register(Ljava/lang/Object;)V"`; phased registration is a `[[mark]]` |

`[rule.target]`

| Key | Type | Default | Meaning |
| --- | --- | --- | --- |
| `bus` | string | none | `"game"` for `NeoForge.EVENT_BUS`, `"mod"` for the mod's own event bus |
| `event` | string | none | The NeoForge event class |
| `priority` | string | `"NORMAL"` | `"HIGHEST"`, `"HIGH"`, `"NORMAL"`, `"LOW"` or `"LOWEST"` |
| `receive_canceled` | boolean | `false` | Whether the listener also receives cancelled events |
| `args` | array of strings | none | One entry per callback parameter, in order: a no-argument method on the event that supplies it, as `name(descriptor)` |
| `result` | string | none | How the callback's return value maps to the event. Draft v1 defines only `"void"` (the callback returns nothing). Kinds for callbacks that return a value (cancellation, interaction results) are added with the first rule that needs one |

What the rewriter will do with an `event_bridge` rule:

1. Generate, inside the converted mod, a functional interface with the same shape as `callback`, in a package derived from the mod id (`jarrow/gen/<modid>/`), and substitute it for the Fabric callback type everywhere in the mod: lambdas, method references and classes that implement it.
2. Replace the matched sequence (read the event field, then call a `register` method) with a call to a generated bridge method. The bridge calls `IEventBus.addListener(EventPriority, boolean, Class, Consumer)` on the chosen bus with a consumer that reads `args` from the event and calls the callback.
3. Emit a report entry: `kind = "event_bridge"`, the location (class, method, bytecode offset), `before` and `after` as symbol names, the rule `id` and its `confidence`.

When the rewriter finds a use it cannot match (the event stored in a variable first, a call to `invoker()`, a phased `register`), it does not guess. It emits an "unmatched" report entry with no rule id, and the classifier counts it toward Tier B.

### `kind = "call_rewrite"`

A call to a source-loader API method becomes a call to a target-loader or vanilla method.

| Table | Key | Type | Meaning |
| --- | --- | --- | --- |
| `[rule.match]` | `owner` | string | Class that declares the method |
| | `name` | string | Method name |
| | `descriptor` | string | Method descriptor |
| | `invoke` | string | `"static"`, `"virtual"`, `"interface"` or `"special"` |
| `[rule.target]` | `owner`, `name`, `descriptor`, `invoke` | as above | The replacement. Its descriptor must be compatible with the original after class substitutions; argument reordering or conversion needs a generated adapter, specified in M2 |

### `kind = "class_substitution"`

A reference to a source-loader API class becomes a reference to another class with the same shape.

| Table | Key | Type | Meaning |
| --- | --- | --- | --- |
| `[rule.match]` | `class` | string | The source-loader API class |
| `[rule.target]` | `class` | string | The replacement. Every member of the matched class that the mod uses must exist on the target with the same name and descriptor; the `rules` crate checks this against the target loader's API index |

### `kind = "entrypoint"`

A Fabric entrypoint becomes a call from the generated `@Mod` class.

| Table | Key | Type | Meaning |
| --- | --- | --- | --- |
| `[rule.match]` | `key` | string | The entrypoint key in `fabric.mod.json`: `"main"`, `"client"`, `"server"` or `"preLaunch"` |
| | `interface` | string | The entrypoint interface, for example `net/fabricmc/api/ModInitializer` |
| | `method` | string | Its method, for example `onInitialize` |
| | `descriptor` | string | Its descriptor, for example `()V` |
| `[rule.target]` | `stage` | string | `"construct"`, `"register"` (inside the first `RegisterEvent`, D-023), `"common-setup"`, `"client-setup"` or `"dedicated-server-setup"` |
| | `side` | string | `"both"`, `"client"` or `"server"` |

Custom entrypoint keys (mod menus, recipe viewers) have no rule. The report lists them as "plugin entrypoint for `<key>`, not translated".

### `[[mark]]`: symbols without a rule

Every public symbol in the source loader's API index maps to a rule, to a mark, or to nothing yet. A mark says why a symbol has no rule.

| Key | Type | Required | Meaning |
| --- | --- | --- | --- |
| `symbol` | string | yes | The symbol, in the format above |
| `status` | string | yes | `"tier-b"` (the Tier B agent may port it) or `"unsupported"` (no tier converts it) |
| `reason` | string | yes | Why, in one sentence. Shown in reports |
| `sources` | array of strings | yes | Ids in `rules/SOURCES.md` |

A mark has no inverse: nothing was rewritten, so there is nothing to undo.

## Example

This is a complete draft entry for Minecraft 1.21.1. It describes API shapes only: the Fabric API event `net.fabricmc.fabric.api.event.lifecycle.v1.ServerTickEvents` and the NeoForge event `net.neoforged.neoforge.event.tick.ServerTickEvent`. It is not active; no rule table exists until M2. The inverse names point at the NeoForge → Fabric table that would land with it; `event-tick` is an illustration, since that direction sets its area names on its page first.

`rules/fabric-neoforge/1.21.1/lifecycle-events-v1.toml`:

```toml
[table]
schema = 1
area = "lifecycle-events-v1"
game_version = "1.21.1"
from = "fabric"
to = "neoforge"
names = "mojang"
summary = "Server and world lifecycle and tick events from fabric-lifecycle-events-v1."
inverse = "neoforge-fabric/1.21.1/event-tick"

[[rule]]
id = "lifecycle-events-v1/server-tick-start"
kind = "event_bridge"
summary = "ServerTickEvents.START_SERVER_TICK becomes a game-bus listener for ServerTickEvent.Pre."
confidence = "equivalent"
sources = [
  "fabric-api/lifecycle-events-v1/ServerTickEvents",
  "neoforge/event/tick/ServerTickEvent",
  "neoforge-bus/IEventBus",
]
inverse = "event-tick/server-tick-pre"

[rule.match]
owner = "net/fabricmc/fabric/api/event/lifecycle/v1/ServerTickEvents"
field = "START_SERVER_TICK"
callback = "net/fabricmc/fabric/api/event/lifecycle/v1/ServerTickEvents$StartTick"
method = "onStartTick"
descriptor = "(Lnet/minecraft/server/MinecraftServer;)V"
register = ["register(Ljava/lang/Object;)V"]

[rule.target]
bus = "game"
event = "net/neoforged/neoforge/event/tick/ServerTickEvent$Pre"
priority = "NORMAL"
receive_canceled = false
args = ["getServer()Lnet/minecraft/server/MinecraftServer;"]
result = "void"

[[rule.difference]]
topic = "timing"
text = "Both fire once per server tick on the logical server, before the tick's work. Taken from the sources; the harness does not measure tick timing."

[[rule.difference]]
topic = "ordering"
text = "Fabric runs listeners by phase, then in registration order. NeoForge runs them by priority, then in registration order. A listener registered without a phase becomes NORMAL priority and keeps its order."

[[rule.difference]]
topic = "arguments"
text = "Fabric passes the MinecraftServer. NeoForge passes the event; getServer() returns the same server. The event also has hasTime(), which has no Fabric counterpart and is not used."

[[rule.difference]]
topic = "cancellation"
text = "Neither event can be cancelled."

[[rule]]
id = "lifecycle-events-v1/server-tick-end"
kind = "event_bridge"
summary = "ServerTickEvents.END_SERVER_TICK becomes a game-bus listener for ServerTickEvent.Post."
confidence = "equivalent"
sources = [
  "fabric-api/lifecycle-events-v1/ServerTickEvents",
  "neoforge/event/tick/ServerTickEvent",
  "neoforge-bus/IEventBus",
]
inverse = "event-tick/server-tick-post"

[rule.match]
owner = "net/fabricmc/fabric/api/event/lifecycle/v1/ServerTickEvents"
field = "END_SERVER_TICK"
callback = "net/fabricmc/fabric/api/event/lifecycle/v1/ServerTickEvents$EndTick"
method = "onEndTick"
descriptor = "(Lnet/minecraft/server/MinecraftServer;)V"
register = ["register(Ljava/lang/Object;)V"]

[rule.target]
bus = "game"
event = "net/neoforged/neoforge/event/tick/ServerTickEvent$Post"
priority = "NORMAL"
receive_canceled = false
args = ["getServer()Lnet/minecraft/server/MinecraftServer;"]
result = "void"

[[rule.difference]]
topic = "timing"
text = "Both fire once per server tick on the logical server, after the tick's work. Taken from the sources; the harness does not measure tick timing."

[[rule.difference]]
topic = "ordering"
text = "Same as server-tick-start: unphased Fabric listeners become NORMAL priority and keep registration order."

[[rule.difference]]
topic = "cancellation"
text = "Neither event can be cancelled."
```

The phased form of `register` gets a mark in `api-base.toml`, because Fabric phase ordering has no exact NeoForge counterpart:

```toml
[[mark]]
symbol = "net/fabricmc/fabric/api/event/Event.register(Lnet/minecraft/resources/ResourceLocation;Ljava/lang/Object;)V"
status = "tier-b"
reason = "Phased registration: Fabric phase ordering can only be approximated with NeoForge priorities."
sources = ["fabric-api/api-base/Event"]
```

## `rules/SOURCES.md`

One file for every direction. Each source is a level-3 heading whose text is the id that rules cite, followed by four bullets. The `rules` crate reads the headings and rejects a rule that cites an id that is not there. URLs are pinned to a commit or tag, so the source a rule was checked against cannot change underneath it.

```markdown
### fabric-api/lifecycle-events-v1/ServerTickEvents
- URL: https://github.com/FabricMC/fabric/blob/dcb9d1ca7f88e2456764ca0d99a392f510dc6018/fabric-lifecycle-events-v1/src/main/java/net/fabricmc/fabric/api/event/lifecycle/v1/ServerTickEvents.java
- License: Apache-2.0
- Read: 2026-11-03
- Used for: event field names, callback interfaces and methods, timing from the Javadoc.

### neoforge/event/tick/ServerTickEvent
- URL: https://github.com/neoforged/NeoForge/blob/53df0316e8ca9d67ef9227943efa7502d24d2f62/src/main/java/net/neoforged/neoforge/event/tick/ServerTickEvent.java
- License: LGPL-2.1
- Read: 2026-11-03
- Used for: Pre and Post event classes, getServer(), hasTime(), timing from the Javadoc, not cancellable.
```

A source that is an issue in Sinytra Connector's public tracker is cited the same way, and the rule credits it.

## Validation

The `rules` crate will refuse to load a table when:

- `area` differs from the file stem, or `game_version` from the directory, or the game version is not pinned;
- `from` or `to` differs from the direction directory, names a loader with no profile in `loaders/`, or `from` equals `to`;
- either loader's profile is `"not-researched"` for the game version ([loaders.md](loaders.md#validation));
- the combination of the two profiles' event models has no key set on this page yet;
- the header has both or neither of `inverse` and `not_invertible`, or a rule has both or neither;
- the inverse table or the inverse rule does not exist, or does not point back;
- an `id` is duplicated in the directory, or does not start with `<area>/`;
- a key is unknown, a required key is missing, or a value is outside its allowed set;
- a class name uses dots, or a descriptor is not a valid JVM descriptor;
- a cited source id is missing from `rules/SOURCES.md`;
- an `event_bridge` rule has no `[[rule.difference]]`;
- two rules match the same symbol.

## How rules reach the report and the ledger

- Every rule application is a report entry with `kind`, `location`, `before`, `after`, `rule` and `confidence`.
- Every use the rewriter could not match is an "unmatched" entry. The board aggregates them per direction into "top red reasons", which decides which rule to write next.
- The rule-table version used in a conversion is a content hash of `rules/<from>-<to>/<game-version>/`, so the same tables always give the same output (see "Determinism contract" in [architecture.md](architecture.md)).
- The ledger metric, per direction, is the share of the source loader's API index covered by rules, weighted by how often the mods in that direction's source list use each symbol.

## How to add a rule

1. Open a [rule proposal](https://github.com/MosheByte/jarrow/issues/new?template=rule-proposal.yml).
2. Check that the direction's page in [directions/](directions/) covers the API area. If it does not, update the page first.
3. Read both loaders' sides from public sources and add each source to `rules/SOURCES.md`.
4. Add the rule to each game-version directory it applies to, with every difference you found, and add its inverse to the inverse table, or mark it not invertible with a reason.
5. Add golden tests for both halves: a small fixture class written for the test (never taken from a mod), the expected rewritten output and the expected report entries.
6. Open a pull request. The harness smoke set and the round-trip check run on it. The rule counts as working when the harness says so.

## Open questions for M2

- **When the main entrypoint runs.** The first design placed `ModInitializer` at `FMLCommonSetupEvent`, but the registry bridge captures `register` calls made during the original initializer and replays them in `RegisterEvent`, which fires before common setup. D-023 (amendment A-10 in [spec-amendments.md](spec-amendments.md)) runs `main` inside the first `RegisterEvent` instead. M2 checks that stage with a harness measurement; if the measurement disagrees, a new decision and amendment follow.
- **Callbacks that return a value** (for example interaction callbacks that return a result). The `result` kinds are defined with the first rule that needs one.
- **Phased registration.** Marked Tier B for now; a later rule may approximate phases with priorities if the harness shows no difference.
- **Key sets for other event models.** NeoForge → Fabric needs the event-bus-to-callback-fields form of `event_bridge` and `entrypoint`. They are defined with the first NeoForge → Fabric table, which lands with its Fabric → NeoForge pair.
- **Namespaces other than intermediary and Mojang.** Rules stay in Mojang names. On 1.20.1, Forge and NeoForge run on SRG names for fields and methods (`research-lock.json`), so their 1.20.1 tables need the MCP config mappings that the installers merge, which are not pinned yet. No pinned loader runs on Quilt's hashed names.
