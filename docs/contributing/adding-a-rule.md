# Adding a rule

A rule says how one API shape on one loader becomes a shape on another: a call, an event field, a class or an entrypoint. This page is the short version of [docs/rules.md](../rules.md), which has every key and the validation list.

**Status.** The format is a draft until the `rules` crate implements it in milestone M2. Until then nothing loads a rule table, so no rule can be run, tested by the harness or shown to work. A rule written today is checked by review against docs/rules.md and its sources.

## Where a rule lives

```
rules/<from>-<to>/<game-version>/<area>.toml
```

- `<from>-<to>` is the direction. Today the only direction is `fabric-neoforge`.
- `<game-version>` is `1.21.1` or `1.21.11`, the versions pinned in `research-lock.json`. A rule that holds on both is written in both directories and checked against each version's own sources; never copied across unchecked.
- `<area>` is the Fabric API module without its `fabric-` prefix (`fabric-lifecycle-events-v1` becomes `lifecycle-events-v1`), or `loader-entrypoints` and `registries` for areas that are not a Fabric API module.

## The table header and author credit

Every file starts with one `[table]`:

```toml
[table]
schema = 1
area = "lifecycle-events-v1"
game_version = "1.21.1"
from = "fabric"
to = "neoforge"
names = "official"
summary = "Server and world lifecycle and tick events from fabric-lifecycle-events-v1."
authors = ["@your-github-handle"]
```

| Key | Meaning |
| --- | --- |
| `schema` | Format version; `1` |
| `area` | Equals the file stem |
| `game_version` | Equals the directory name |
| `from`, `to` | The loaders of the direction: `"fabric"`, `"neoforge"` |
| `names` | `"official"`: Minecraft classes under Mojang's official names |
| `summary` | One line: the API area the file covers |
| `authors` | GitHub handles, each with `@`, of everyone who wrote or changed a rule in the file. Add yours when you add or change a rule; never remove one |

`authors` is newer than docs/rules.md, which does not list it yet. The `rules` crate will require it (M2) and read it to credit authors in the nightly Discussion and in the release notes.

## Every table has an inverse

Every table that maps A to B needs one of two things:

- the table that maps B to A, at the mirrored path with the same area stem:

  ```
  rules/fabric-neoforge/1.21.1/lifecycle-events-v1.toml
  rules/neoforge-fabric/1.21.1/lifecycle-events-v1.toml
  ```

- or a mark in its header that says why there can be none:

  ```toml
  [table]
  # ...the keys above...
  inverse = "not-invertible"
  inverse_reason = "One sentence: which B shapes have no A counterpart."
  ```

No `neoforge-fabric` table exists yet, so a new `fabric-neoforge` table either adds its mirror or carries the mark. The `rules` crate will refuse a table with neither (M2). These two keys are a draft like `authors`; when the crate fixes their names, this page and docs/rules.md change in the same pull request.

## A rule

Each rule is a `[[rule]]` entry:

| Key | Meaning |
| --- | --- |
| `id` | `<area>/<name>`, lowercase with hyphens. Never reused: reports refer to it |
| `kind` | `"event_bridge"`, `"call_rewrite"`, `"class_substitution"` or `"entrypoint"` |
| `summary` | One sentence: what becomes what |
| `confidence` | `"exact"`, `"equivalent"` (differences listed) or `"approximate"` (known differences that can matter) |
| `sources` | Ids of entries in `rules/SOURCES.md`, at least one |
| `[rule.match]` | What to find; the keys depend on `kind` |
| `[rule.target]` | What to produce; the keys depend on `kind` |
| `[[rule.difference]]` | Each behaviour difference: `topic` and `text`. Required for `event_bridge` |

| Kind | What it does | Issue label |
| --- | --- | --- |
| `event_bridge` | A Fabric `Event` field plus `register` becomes a NeoForge event-bus listener | `rule` |
| `call_rewrite` | A call to one method becomes a call to another | `rule` |
| `class_substitution` | A reference to one class becomes a reference to a class with the same members | `rule` |
| `entrypoint` | A `fabric.mod.json` entrypoint becomes a call from the generated `@Mod` class | `rule` |

The `match` and `target` keys for each kind are in docs/rules.md. Kinds for manifest keys and resources, the `good-first-rule` size, are not in the draft yet; they are added with the `rules` crate in M2.

A symbol that should have no rule gets a `[[mark]]` instead, with `symbol`, `status` (`"tier-b"` or `"unsupported"`), `reason` and `sources`.

Names inside a rule:

- Classes are JVM internal names: slashes for packages, `$` for nested classes (`net/fabricmc/fabric/api/event/lifecycle/v1/ServerTickEvents$EndTick`).
- Types are JVM descriptors (`(Lnet/minecraft/server/MinecraftServer;)V`).
- Minecraft classes use Mojang's official names, never intermediary or Yarn names.
- A symbol is `owner` for a class, `owner.name:descriptor` for a field and `owner.name(descriptor)` for a method.

## Example

A complete table with one rule, adapted from the example in docs/rules.md, with the `authors` key added:

```toml
[table]
schema = 1
area = "lifecycle-events-v1"
game_version = "1.21.1"
from = "fabric"
to = "neoforge"
names = "official"
summary = "Server and world lifecycle and tick events from fabric-lifecycle-events-v1."
authors = ["@your-github-handle"]

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
text = "Fabric runs listeners by phase, then in registration order. NeoForge runs them by priority, then in registration order. A listener registered without a phase becomes NORMAL priority and keeps its order."

[[rule.difference]]
topic = "cancellation"
text = "Neither event can be cancelled."
```

Two things are still missing before this could be a real table: its inverse (the mirrored `neoforge-fabric` table, or the mark), and the three source ids it cites, which are not in `rules/SOURCES.md` yet (step 4 below).

## The commands that arrive in M2

```sh
jarrow rule new <from> <to> <symbol>   # not built yet (milestone M2)
jarrow rule check                      # not built yet (milestone M2)
```

- `jarrow rule new fabric neoforge 'net/fabricmc/fabric/api/event/lifecycle/v1/ServerTickEvents.END_SERVER_TICK:Lnet/fabricmc/fabric/api/event/Event;'` will add a rule stub for the symbol to the right table, with your handle in `authors`, and scaffold the fixture for its test.
- `jarrow rule check` will run the differential test on your machine: the harness on the fixture before and after conversion, and the difference between the two.
- With them, a pull request that changes a rule table gets the harness result as a comment automatically. The target is within 15 minutes of the push.

Until then, `rule` is not a command of the `jarrow` binary.

## The steps that work today

1. Open the Codespace ([README.md](README.md)) and make a branch: `git switch -c rule/<area>-<name>`.
2. Copy the example above into `rules/fabric-neoforge/<game-version>/<area>.toml`, or add your `[[rule]]` to the file if it exists. Change it to your symbol, put your handle in `authors`, and add the mirrored table or the `not-invertible` mark.
3. Describe API shapes only, learned from public documentation and the open-source API repositories. Never code from a mod, never decompiled code, never code from Sinytra Connector, Architectury or Forgified Fabric API ([CONTRIBUTING.md](../../CONTRIBUTING.md), "Clean room"). Rule tables are CC0-1.0.
4. Add every source you used to `rules/SOURCES.md`: a level-3 heading with the id the rule cites, then four bullets: URL (pinned to a tag or commit), License, Read (the date), Used for.
5. Check that the file is valid TOML (no output means it parsed; nothing checks the rule keys until M2):

   ```sh
   python -c "import sys, tomllib; tomllib.load(open(sys.argv[1], 'rb'))" rules/fabric-neoforge/1.21.1/lifecycle-events-v1.toml
   ```

6. Run the checks CI runs before a merge:

   ```sh
   python tools/pr_ready.py --all          # outside the container on Windows: py -3.12 tools/pr_ready.py --all
   ```

7. Commit, push and open a pull request (`gh pr create`). If you cannot push to the repository, the Codespace offers to create a fork for you when you push. In the template's Proof section, say that no harness result exists yet because nothing loads rule tables before M2. Do not describe the rule as working.

Known problem: CI also runs `tools/pr_ready.py` in range mode, whose trailer check expects every commit to carry the `Co-Authored-By` trailer that the project's agent adds to its own commits. It fails on commits made by people. Do not add that trailer to your own commits ([CONTRIBUTING.md](../../CONTRIBUTING.md), "Commits"); the failure is in the check, not in your pull request.
