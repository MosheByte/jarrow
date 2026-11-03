# Board data contract

**Status: one column.** The nightly board (`.github/workflows/board.yml`) measures one direction, `fabric-neoforge`, with conversion stubbed: no converter exists yet (milestone M2), so every mod in that column is red with the reason `no_converter`. The other 11 directions have no column, no rows and zero counts. Before the first nightly has run, `board.json` does not exist and the site shows the empty board.

The nightly board produces three kinds of artefact: the source lists, `board.json` (with `board-summary.json`) and the reports. `crates/board` (`jarrow-board`, the writer), the site (`site/src/lib/board.ts`), `tools/board_summary.py` and `tools/ledger.py` read them, and the D1 schema (`infra/d1/migrations/0002_directions.sql`) stores them, so this file is the single definition. Change it only together with every reader. How the pipeline produces them is in [board-pipeline.md](board-pipeline.md).

This is schema 2: the board is per direction (D-029, D-042). Schema 1, from before the scope change, had no direction model and is no longer read or written.

## Directions and columns

The board has one column per direction: 12 directions between Fabric, NeoForge, Forge and Quilt (D-029), with the ids `<from>-<to>` from [directions/](directions/). A direction gets its column when the nightly starts running it, in the fixed build order. The first is `fabric-neoforge`, the only one today. A column holds a row for every mod in its source list, and its share of green is the direction's number against its v1 bar.

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

Each direction's source list is the top 1,000 mods of its source loader on Modrinth for the pinned game versions (1.20.1, 1.21.1 and 1.21.11). Directions with the same source loader share one list, so there is one file per source loader: `board-list-fabric.json`, `board-list-neoforge.json`, `board-list-forge.json`, `board-list-quilt.json`. A list is fetched only once one of its directions has a column, and only for a loader whose profile names its Modrinth loader tag (`modrinth_loader` in `loaders/<name>.toml`). Today only Fabric's is recorded, so `jarrow-board list` refuses the other three loaders, and only `board-list-fabric.json` exists.

Fetched weekly from the Modrinth API. It holds the top 1,000 projects with `project_type:mod`, `categories:<modrinth_loader>`, and at least one pinned game version (`versions:1.20.1` OR `versions:1.21.1` OR `versions:1.21.11`), ordered by downloads. A search hit without a usable file for any pinned version is listed under `skipped` with the reason and does not count towards the 1,000.

```json
{
  "schema": 2,
  "loader": "fabric",
  "fetched_at": "2026-11-03T14:00:00Z",
  "game_versions": ["1.20.1", "1.21.1", "1.21.11"],
  "top": 1000,
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
        { "game_version": "1.21.11", "version_id": "abc", "version_number": "0.6.0", "version_type": "release", "filename": "sodium-fabric-0.6.0.jar", "url": "https://cdn.modrinth.com/...", "sha512": "...", "size": 1234 }
      ]
    }
  ],
  "skipped": [
    { "project_id": "XXXXXXXX", "slug": "example", "reason": "no Fabric release or beta JAR on Modrinth for 1.20.1 or 1.21.1 or 1.21.11" }
  ]
}
```

- `loader` is the source loader id. `game_versions` is the pinned set, oldest first.
- `files` holds at most one entry per pinned game version, oldest version first: the newest release for this loader and that version, or the newest beta when no release exists. Alphas are never picked. Only a `.jar` on `https://cdn.modrinth.com/` with a sha512 counts, and the version's primary file wins.
- `loaders` lists the loader ids among the project's Modrinth tags. It is informational, and it is not an author-verified "official port". A row's `target_build_exists` comes from it.
- JAR bytes are never stored. Each shard downloads, verifies `sha512` and size, processes, and deletes them.

## Board: `board.json`

```json
{
  "schema": 2,
  "generated_at": "2026-11-04T04:12:00Z",
  "started_at": "2026-11-04T02:20:00Z",
  "run_id": "nightly-20261104",
  "engine": { "version": "0.1.0", "git_sha": "<40 hex>" },
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
      "game_versions": ["1.20.1", "1.21.1", "1.21.11"],
      "list_fetched_at": "2026-11-03T14:00:00Z",
      "rules_version": "none",
      "totals": { "mods": 1000, "green": 0, "yellow": 0, "red": 1000 },
      "top_reasons": [ { "kind": "no_converter", "label": "No converter yet (milestone M2)", "count": 1000 } ]
    },
    {
      "id": "neoforge-fabric",
      "from": "neoforge",
      "to": "fabric",
      "order": 2,
      "bar_percent": 50,
      "column": false,
      "game_versions": [],
      "list_fetched_at": null,
      "rules_version": null,
      "totals": { "mods": 0, "green": 0, "yellow": 0, "red": 0 },
      "top_reasons": []
    }
  ],
  "stats": {
    "shards": 10, "files": 2400, "downloaded_bytes": 3600000000, "download_failures": 0, "inspect_failures": 0,
    "max_shard_wall_ms": 600000, "total_shard_wall_ms": 5400000,
    "step_ms": { "download": 5000000, "inspect": 300000, "classify": 0, "convert": 0, "harness": 0, "round_trip": 0 }
  },
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
      "tier": null,
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
          "version_number": "0.6.0",
          "file_sha512": "...",
          "mod_id": null,
          "mod_version": "0.6.0",
          "tier": null,
          "tier_reasons": ["classifier not built yet (M1 wave 2)"],
          "outcome": "not-run",
          "score": null,
          "reason_kind": "no_converter",
          "reason": "No converter yet (milestone M2)",
          "report_key": "report:fabric-neoforge:AANobbMI:1.21.11",
          "report_hash": "sha256:...",
          "duration_ms": 412
        }
      ]
    }
  ]
}
```

The example is shortened: a real `board.json` lists all 12 directions. Before the first nightly the site serves the empty board: `generated_at`, `started_at`, `run_id`, `engine` and `stats` are `null`, `game_versions` is the pinned set, every direction has `column: false`, and `rows` is empty.

### Directions

- `directions` always lists all 12 directions, once each, in build order. `id`, `from`, `to`, `order` and `bar_percent` repeat the table above, and readers check them against it.
- `column` is `true` once the nightly measured the direction in this run. A direction with `column: false` has `game_versions: []`, `list_fetched_at: null`, `rules_version: null`, zero totals, no `top_reasons` and no rows.
- `game_versions` are the pinned versions the column measured, and `list_fetched_at` is when its source list was fetched.
- `rules_version` is `sha256:<hex>` over the direction's rule tables under `rules/<from>-<to>/`, or `none` while it has none (every direction today).
- Each direction's `totals.mods == green + yellow + red`, and equals the number of rows with that direction. `top_reasons` counts the direction's yellow and red rows per `reason_kind`, most frequent first, with the kind's `label`.

### Totals

- `totals` at the top is the sum of every direction's totals. It counts entries, one per mod per column: a mod in two columns counts twice. While only one column exists, it equals that column's totals. A reader that shows a direction reads that direction's `totals`.
- `totals.mods == green + yellow + red` holds at the top too. "Not run" is not an outcome on the board: a mod the harness could not measure is red or yellow with the reason, never green.
- `game_versions` at the top is every version some column measured, oldest first.

### Rows

- One row per mod per direction column. `direction` names the column; the same Modrinth project can have a row in several columns. `rows` are sorted by direction (build order), then downloads (most first), then project id.
- A row's `outcome` is decided by its primary game version, `primary_game_version`: the newest pinned version the mod ships a source-loader file for. Its `tier`, `reason_kind` and `reason` are that result's. Every version stays visible in `results`, one entry per game version, each naming its `direction` and `game_version`.
- A result's `outcome` follows `crates/report` (D-024): only a harness score makes it `green` (1.0 or 0.75), `yellow` (0.5) or `red` (0.25 or 0) ([harness.md](harness.md#scoring)). A result without a harness score is `not-run`, with `score: null` and a `reason_kind`; while no converter exists, every result is `not-run` with `no_converter` (or `download_failed`).
- A row's `outcome` is `green`, `yellow` or `red`, never `not-run`: it is the primary result's outcome, with `not-run` counted as `red`, or as `yellow` when the reason is `client_only_not_measured` (the mod loads but its content is client-only). So `outcome = green` requires a harness score of `1.0` or `0.75` on the primary game version, and red covers everything else, including "harness not run" and "no converter yet".
- `tier` is `a`, `b` or `unsupported`, or `null` when the classifier did not run: it is not built yet (M1 wave 2), or there was no file to classify (`download_failed`). `tier_reasons` say why.
- Every result and row that is not green carries a `reason_kind` and a `reason`.
- `mod_id` is `null` and `mod_version` is Modrinth's version number until the manifest crate reads the JAR's own (M1 wave 2).
- `report_url` is the site page of the primary report, or `null` until the site has report pages (every row today). `report_key` is the KV key of each result's report.
- `target_build_exists` is informational: whether Modrinth tags the project with the direction's target loader.
- Mods whose author opted out ("do not port") are removed from `rows` and counted in no total (not built yet: no opt-out list exists).
- The board links to the mod's own page (`page_url`) and never to a converted JAR.
- `stats` holds counts and durations in milliseconds. They vary between runs; nothing else in the merge does.

### Vocabulary

- Tier values are `a`, `b`, `unsupported` (or `null`), and outcome values are `green`, `yellow`, `red`, `not-run`, as serialised by `crates/report` (D-024). `crates/board`, `site/src/lib/board.ts`, `tools/board_summary.py` and the D1 schema use the same strings.
- Loader ids are `fabric`, `neoforge`, `forge`, `quilt`; direction ids are `<from>-<to>`.
- Harness scores are `0`, `0.25`, `0.5`, `0.75`, `1.0` or `null`.
- `reason_kind` is a closed vocabulary, defined in `crates/board` (`ReasonKind`) until `crates/report` grows it: `no_converter`, `unsupported_native`, `unsupported_transformer`, `unsupported_language_adapter`, `unsupported_obfuscated`, `license_personal_use`, `tier_b_required`, `harness_not_run_eula`, `harness_load_failure`, `class_not_found`, `mixin_failure`, `registry_conflict`, `crash`, `entries_differ`, `client_only_not_measured`, `download_failed`, `opt_out`. Each kind has one `label`, which the board, the ledger and the Discussion show.

## Board summary: `board-summary.json`

The input of `tools/ledger.py`, which the bot commits as `ledger/inputs/board-summary.json`. It describes one column, named by `direction`: the first measured direction in build order, which is `fabric-neoforge` today. Its shape is the one `tools/ledger.py` validates (unknown keys are errors there, and `schema` is 1 like every ledger input); every column's own numbers are in `board.json`.

```json
{
  "schema": 1, "direction": "fabric-neoforge", "run": "nightly-20261104", "date": "2026-11-04", "commit": "<40 hex>",
  "game_versions": ["1.20.1", "1.21.1", "1.21.11"],
  "mods": 1000, "green": 0, "yellow": 0, "red": 1000, "not_run": 0,
  "harness": { "completed": 0, "attempted": 0 },
  "areas": [],
  "red_reasons": [ { "reason": "No converter yet (milestone M2)", "count": 1000 } ]
}
```

`date` is the UTC date the first shard started. `not_run` is always 0 (see the rules above). `areas` stays empty until the harness measures conversion areas.

## Reports

One JSON report per mod, direction and game version, written by `crates/board` (`Report`, schema 2; `crates/report` has no whole-report type yet). Its `entries` are `crates/report` `ReportEntry` values. It holds every pipeline step (`download`, `inspect`, `classify`, `convert`, `harness`, `round_trip`), each with a `status` of `ok`, `failed`, `not-built` or `not-run` and the reason, plus the result's `tier`, `outcome`, `score` and reason. A report holds no clock time and no duration, so the same file and engine give the same bytes.

- The canonical JSON is compact, with fields in a fixed order. `report_hash` is its SHA-256; issue forms ask for it.
- KV key: `report_key`, `report:<direction>:<project_id>:<game_version>`.
- In `reports.tar.gz` (a release asset and an R2 object): `reports/<direction>/<project_id>/<game_version>.json`. The tarball is deterministic: sorted entries, mode 0644, owner 0, mtime 0, fixed gzip header.

## Storage

D1 (`infra/d1/migrations/0002_directions.sql`, D-044) keeps one `board_runs` row per nightly, with the measured directions (`directions`, a JSON array) and the per-direction totals and `rules_version` (`totals`, a JSON object keyed by direction id). It keeps one `board_rows` row per mod, direction and game version: the `results` entries, with the same tier and outcome strings as `board.json`, `is_primary = 1` on the primary result and `board_outcome` holding the row's outcome there. `board_rows.report_key` is the KV key.

## Publication

1. `board.yml` (nightly, 02:00 UTC) fetches `board-list-<loader>.json` for each measured direction's source loader (or reuses the copy on the `board-list` release while it is younger than 7 days), then splits each column's list into 10 shards. Each shard writes `shard-<direction>-<n>.json` and its reports as workflow artifacts.
2. The merge job builds `board.json`, `board-summary.json`, `reports.tar.gz` and the Cloudflare import files, then the workflow writes Cloudflare directly with wrangler and the `CLOUDFLARE_API_TOKEN` secret (D-021):
   - D1 `board_runs` and `board_rows`, keyed by the run id. A rerun deletes that run's rows first;
   - KV: every report under its `report_key`, then `board.json` under `board.json` (last, so the site switches only once the reports are in place);
   - R2: `reports.tar.gz` at `reports/<run_id>.tar.gz` in `jarrow-reports`.
3. The same files are uploaded to the GitHub release `board-latest` as a public, rebuildable mirror, replacing the previous assets. A dated `board-YYYYMMDD` release keeps `board.json` as history.
4. `ledger/inputs/board-summary.json` and `LEDGER.md` are committed by the bot, which uses the `BOT_DEPLOY_KEY` deploy key that the `main` ruleset allows to bypass.
5. `tools/board_summary.py` posts one Discussion per column in the "Nightly results" category (or "Announcements" when it is missing), titled "Nightly YYYY-MM-DD: G green, Y yellow, R red" (with the direction in the title when the board has several columns), once per date and direction. Directions without a column get no post.
