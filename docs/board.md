# Board data contract

**Status: no board has run yet.** No direction has a board column, so `board.json` has no rows and every count is zero.

The nightly board produces three kinds of artefact: the source lists, `board.json` and the reports. The engine (`crates/board`, the writer), the site (`site/src/lib/board.ts`), `tools/board_summary.py` and `tools/ledger.py` all read them, and the D1 schema (`infra/d1/migrations/`) stores them, so this file is the single definition. Change it only together with every reader.

## Directions and columns

The board has one column per direction: 12 directions between Fabric, NeoForge, Forge and Quilt (D-029), with the ids `<from>-<to>` from [directions/](directions/). A direction gets its column when the nightly starts running it, in the fixed build order. The first is `fabric-neoforge` in milestone M1. A column holds a row for every mod in its source list, and its share of green is the direction's number against its v1 bar.

| Order | Direction ids | Bar (`bar_percent`) |
| --- | --- | --- |
| 1 | `fabric-neoforge` | 60 |
| 2 | `neoforge-fabric` | 50 |
| 3 | `fabric-quilt`, `quilt-fabric` | 50 |
| 4 | `forge-neoforge`, `neoforge-forge` | 50 |
| 5 | `fabric-forge`, `forge-fabric` | 50 |
| 6 | `quilt-forge`, `forge-quilt`, `quilt-neoforge`, `neoforge-quilt` | 50 |

No direction is shown on the site before its column exists.

## Source lists: `board-list-<loader>.json`

Each direction's source list is the top 1,000 mods of its source loader on Modrinth for the pinned game versions. Directions with the same source loader share one list, so there is one file per source loader: `board-list-fabric.json`, `board-list-neoforge.json`, `board-list-forge.json`, `board-list-quilt.json`. A list is fetched only once one of its directions has a column, and only for a loader whose profile names its Modrinth loader tag (`modrinth_loader` in `loaders/<name>.toml`; today only Fabric's is recorded).

Fetched weekly from the Modrinth API. It holds the top 1,000 projects with `project_type:mod`, `categories:<modrinth_loader>`, and at least one pinned game version (`versions:1.20.1` OR `versions:1.21.1` OR `versions:1.21.11`), ordered by downloads.

```json
{
  "schema": 2,
  "loader": "fabric",
  "fetched_at": "2026-11-03T14:00:00Z",
  "game_versions": ["1.20.1", "1.21.1", "1.21.11"],
  "mods": [
    {
      "project_id": "AANobbMI",
      "slug": "sodium",
      "title": "Sodium",
      "downloads": 123456789,
      "license": "LicenseRef-Polyform-Shield-1.0.0",
      "client_side": "required",
      "server_side": "unsupported",
      "icon_url": "https://cdn.modrinth.com/...",
      "loaders": ["fabric", "neoforge", "quilt"],
      "files": [
        { "game_version": "1.21.11", "version_id": "abc", "version_number": "0.6.0", "url": "https://cdn.modrinth.com/...", "sha512": "...", "size": 1234 }
      ]
    }
  ]
}
```

- `loader` is a loader id. `game_versions` is the pinned set.
- `files` holds at most one entry per pinned game version: the newest release for this loader and that version, or the newest beta when no release exists.
- `loaders` lists the loader tags Modrinth shows for the project. It is informational, taken from Modrinth's loader tags. It is not an author-verified "official port".
- JAR bytes are never stored. Each shard downloads, verifies `sha512`, processes, and deletes them.

## Board: `board.json`

```json
{
  "schema": 2,
  "generated_at": "2026-11-04T04:12:00Z",
  "run_id": "nightly-20261104",
  "engine": { "version": "0.1.0", "git_sha": "0123abcd" },
  "game_versions": ["1.20.1", "1.21.1", "1.21.11"],
  "totals": { "mods": 1000, "green": 0, "yellow": 0, "red": 1000 },
  "directions": [
    {
      "id": "fabric-neoforge",
      "from": "fabric",
      "to": "neoforge",
      "order": 1,
      "bar_percent": 60,
      "column": true,
      "rules_version": "sha256 of the rules/fabric-neoforge/ tree",
      "totals": { "mods": 1000, "green": 0, "yellow": 0, "red": 1000 },
      "top_reasons": [ { "kind": "no_converter", "count": 1000 } ]
    },
    {
      "id": "neoforge-fabric",
      "from": "neoforge",
      "to": "fabric",
      "order": 2,
      "bar_percent": 50,
      "column": false,
      "rules_version": null,
      "totals": { "mods": 0, "green": 0, "yellow": 0, "red": 0 },
      "top_reasons": []
    }
  ],
  "rows": [
    {
      "direction": "fabric-neoforge",
      "project_id": "AANobbMI",
      "slug": "sodium",
      "title": "Sodium",
      "downloads": 123456789,
      "page_url": "https://modrinth.com/mod/sodium",
      "icon_url": "https://cdn.modrinth.com/...",
      "license": "LicenseRef-Polyform-Shield-1.0.0",
      "primary_game_version": "1.21.11",
      "tier": "b",
      "outcome": "red",
      "reason_kind": "no_converter",
      "reason": "No converter yet (milestone M2)",
      "last_run": "2026-11-04T03:55:00Z",
      "report_url": null,
      "target_build_exists": true,
      "results": [
        {
          "direction": "fabric-neoforge",
          "game_version": "1.21.11",
          "version_id": "abc",
          "file_sha512": "...",
          "mod_id": "sodium",
          "mod_version": "0.6.0",
          "tier": "b",
          "tier_reasons": ["..."],
          "outcome": "not-run",
          "score": null,
          "reason_kind": "no_converter",
          "reason": "No converter yet (milestone M2)",
          "report_key": "reports/nightly-20261104/fabric-neoforge/AANobbMI/1.21.11.json",
          "report_hash": "sha256:...",
          "duration_ms": 412
        }
      ]
    }
  ]
}
```

The example is shortened: a real `board.json` lists all 12 directions.

### Directions

- `directions` always lists all 12 directions, once each, in build order. `id`, `from`, `to`, `order` and `bar_percent` repeat the table above, and readers check them against it.
- `column` is `true` once the nightly measured the direction. A direction with `column: false` has zero totals, no `top_reasons`, `rules_version: null` and no rows.
- `rules_version` is the content hash of the direction's `rules/<from>-<to>/` tree used in the run.
- Each direction's `totals.mods == green + yellow + red`, and equals the number of rows with that direction. `top_reasons` aggregates the direction's `reason_kind` values.

### Totals

- `totals` at the top is the sum of every direction's totals. It counts entries, one per mod per column: a mod in two columns counts twice. While only one column exists, it equals that column's totals. A reader that shows a direction reads that direction's `totals`.
- `totals.mods == green + yellow + red` holds at the top too. "Not run" is not an outcome on the board: a mod the harness could not measure is red or yellow with the reason, never green.

### Rows

- One row per mod per direction column. `direction` names the column; the same Modrinth project can have a row in several columns.
- A row's `outcome` is the outcome on its `primary_game_version`, the newest pinned version the mod ships a source-loader file for. Every version stays visible in `results`, one entry per game version, each naming its `direction` and `game_version`.
- A row's `outcome` is `green`, `yellow` or `red`. `outcome = green` requires a harness score of `1.0` or `0.75` on the primary game version ([harness.md](harness.md#scoring), D-024). Yellow is `0.5`, or `client_only_not_measured` when the mod loads but its content is client-only. Red covers everything else, including "harness not run" and "no converter yet".
- A result's `outcome` may also be `not-run`, when no harness result exists for that version. Its `score` is then `null`.
- `tier` is `a`, `b` or `unsupported`, or `null` when the classifier could not run (for example `download_failed`).
- Every yellow or red row carries a `reason_kind`.
- `report_url` is the site page of the primary report, or `null` before one exists. `report_key` is the R2 key of each report.
- `target_build_exists` is informational: whether Modrinth lists the target loader's tag for the project.
- Mods whose author opted out ("do not port") are removed from `rows` and counted in no total.
- The board links to the mod's own page (`page_url`) and never to a converted JAR.

### Vocabulary

- Tier values are `a`, `b`, `unsupported`, and outcome values are `green`, `yellow`, `red`, `not-run`, as serialised by `crates/report` (D-024). `site/src/lib/board.ts` and the D1 schema use the same strings.
- Loader ids are `fabric`, `neoforge`, `forge`, `quilt`; direction ids are `<from>-<to>`.
- Harness scores are `0`, `0.25`, `0.5`, `0.75`, `1.0` or `null`.
- `reason_kind` is a closed vocabulary, to be defined in `crates/report`: `no_converter`, `unsupported_native`, `unsupported_transformer`, `unsupported_language_adapter`, `unsupported_obfuscated`, `license_personal_use`, `tier_b_required`, `harness_not_run_eula`, `harness_load_failure`, `class_not_found`, `mixin_failure`, `registry_conflict`, `crash`, `entries_differ`, `client_only_not_measured`, `download_failed`, `opt_out`. The board aggregates "top reasons" per direction on it.

## Reports

One JSON report per mod, direction and game version, in the `crates/report` model, stored at `report_key` (`reports/<run_id>/<direction>/<project_id>/<game_version>.json`). The nightly also publishes all reports as the `reports.tar.gz` release asset. `report_hash` is the SHA-256 of the canonical report JSON; issue forms ask for it.

## Storage

D1 (`infra/d1/migrations/0002_directions.sql`) keeps one `board_runs` row per nightly, with the measured directions and the per-direction totals, and one `board_rows` row per mod, direction and game version: the `results` entries, with `is_primary` and `board_outcome` marking the entry that decides the row's outcome.

## Publication

1. `board.yml` (nightly, 02:00 UTC) splits each column's source list into shards. Each shard writes `shard-<direction>-<n>.json` and its reports as workflow artifacts.
2. The merge job builds `board.json`, `board-summary.json` and `reports.tar.gz`, then writes them to Cloudflare directly with wrangler and the `CLOUDFLARE_API_TOKEN` secret (D-021):
   - KV `board.json` and the compact `board:rows` used for server rendering;
   - D1 `board_runs` and `board_rows`;
   - R2 reports at `report_key`.
3. The same files are uploaded to the GitHub release `board-latest` as a public, rebuildable mirror. A dated `board-YYYYMMDD` release keeps `board.json` as history.
4. `ledger/inputs/board-summary.json` and `LEDGER.md` are committed by the bot, which uses the `BOT_DEPLOY_KEY` deploy key that the `main` ruleset allows to bypass. `board-summary.json` describes the first column today; it gains a direction key before a second column exists.
5. `tools/board_summary.py` posts the Discussion "Nightly YYYY-MM-DD: G green, Y yellow, R red; top reasons…" per column, in the "Nightly results" category.
