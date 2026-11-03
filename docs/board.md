# Board data contract

The nightly board produces three artefacts. The engine (`crates/board`), the site (`site/src/lib/board.ts`), `tools/board_summary.py` and `tools/ledger.py` all read them, so this file is the single definition. Change it only together with every reader. How the pipeline produces them is in [board-pipeline.md](board-pipeline.md).

The board is per **direction**: `<source>-<target>`, for example `fabric-neoforge`, where both are loader ids (`fabric`, `neoforge`, `forge`, `quilt`) and differ. The source loader decides which Modrinth projects and files are tested. Fabric to NeoForge is the first column; other directions use the same files and fields.

## Mod list: `board-list.json`

Fetched weekly from the Modrinth API, one file per direction. It holds the top 1,000 projects with `project_type:mod`, `categories:<source loader>`, and at least one pinned game version (`versions:1.21.1` OR `versions:1.21.11`), ordered by downloads. A search hit without a usable file for any pinned version is listed under `skipped` with the reason and does not count towards the 1,000.

```json
{
  "schema": 1,
  "fetched_at": "2026-11-03T14:00:00Z",
  "direction": "fabric-neoforge",
  "source_loader": "fabric",
  "target_loader": "neoforge",
  "game_versions": ["1.21.1", "1.21.11"],
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
      "neoforge_build_exists": true,
      "target_build_exists": true,
      "files": [
        { "game_version": "1.21.11", "version_id": "abc", "version_number": "0.6.0", "version_type": "release", "filename": "sodium-fabric-0.6.0.jar", "url": "https://cdn.modrinth.com/...", "sha512": "...", "size": 1234 }
      ]
    }
  ],
  "skipped": [
    { "project_id": "XXXXXXXX", "slug": "example", "reason": "no Fabric release or beta JAR on Modrinth for 1.21.1 or 1.21.11" }
  ]
}
```

- `files` holds at most one entry per pinned game version, oldest version first: the newest release of the source loader for that version, or the newest beta when no release exists. Alphas are never picked. Only a `.jar` on `https://cdn.modrinth.com/` with a sha512 counts, and the version's primary file wins.
- `loaders` are the project's loader tags on Modrinth. `neoforge_build_exists` (the tag `neoforge`) and `target_build_exists` (the direction's target loader) are informational. Neither is an author-verified "official port".
- JAR bytes are never stored. Each shard downloads, verifies `sha512` and size, processes, and deletes them.

## Board: `board.json`

```json
{
  "schema": 1,
  "generated_at": "2026-11-04T04:12:00Z",
  "started_at": "2026-11-04T02:20:00Z",
  "run_id": "nightly-20261104",
  "engine": { "version": "0.1.0", "git_sha": "0123abcd..." },
  "rules_version": "none",
  "game_versions": ["1.21.1", "1.21.11"],
  "totals": { "mods": 1000, "green": 0, "yellow": 0, "red": 1000 },
  "top_reasons": [ { "kind": "no_converter", "label": "No converter yet (milestone M2)", "count": 1000 } ],
  "directions": [
    {
      "direction": "fabric-neoforge",
      "source_loader": "fabric",
      "target_loader": "neoforge",
      "game_versions": ["1.21.1", "1.21.11"],
      "list_fetched_at": "2026-11-03T14:00:00Z",
      "rules_version": "none",
      "totals": { "mods": 1000, "green": 0, "yellow": 0, "red": 1000 },
      "top_reasons": [ { "kind": "no_converter", "label": "No converter yet (milestone M2)", "count": 1000 } ]
    }
  ],
  "stats": {
    "shards": 10, "files": 1800, "downloaded_bytes": 2700000000, "download_failures": 0, "inspect_failures": 0,
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
      "tier": "unknown",
      "outcome": "red",
      "reason_kind": "no_converter",
      "reason": "No converter yet (milestone M2)",
      "last_run": "2026-11-04T03:55:00Z",
      "neoforge_build_exists": true,
      "target_build_exists": true,
      "results": [
        {
          "game_version": "1.21.11",
          "version_id": "abc",
          "version_number": "0.6.0",
          "file_sha512": "...",
          "mod_id": null,
          "mod_version": "0.6.0",
          "tier": "unknown",
          "tier_reasons": ["classifier not built yet (M1 wave 2)"],
          "outcome": "red",
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

Rules:

- A row is one mod in one direction. `rows` are sorted by direction, then downloads (most first), then project id.
- A mod's board `outcome` is the outcome on its primary game version, which is the newest pinned version it ships a file for. Every other version stays visible in `results`.
- `totals.mods == green + yellow + red`, per direction and over all directions. "Not run" is not an outcome on the board: a mod the harness could not measure is red or yellow with the reason, never green.
- `outcome = green` requires a harness score of `1.0` or `0.75` ([harness.md](harness.md), D-024). Yellow is `0.5`, or `client_only_not_measured` when the mod loads but its content is client-only. Red covers everything else, including "harness not run" and "no converter yet".
- Tier values are `a`, `b`, `unsupported` (as serialised by `crates/report`, D-024), and `unknown` while the classifier is not built (milestone M1, wave 2). Outcome values are `green`, `yellow`, `red`, `not-run`. `site/src/lib/board.ts` uses the same strings. The D1 schema spells two of them differently (`A`, `B`, `not_run`); the D1 import maps them.
- `reason_kind` is a closed vocabulary (`no_converter`, `unsupported_native`, `unsupported_transformer`, `unsupported_language_adapter`, `unsupported_obfuscated`, `license_personal_use`, `tier_b_required`, `harness_not_run_eula`, `harness_load_failure`, `class_not_found`, `mixin_failure`, `registry_conflict`, `crash`, `entries_differ`, `client_only_not_measured`, `download_failed`, `opt_out`). It is defined in `crates/board` (`ReasonKind`) until `crates/report` grows it. `top_reasons` counts the yellow and red rows per kind, most frequent first, with the kind's `label`.
- `mod_id` is `null` and `mod_version` is Modrinth's version number until the manifest crate reads the JAR's own (M1 wave 2).
- `rules_version` is `sha256:<hex>` over the direction's rule tables under `rules/<direction>/`, or `none` while it has none; the top-level value is `per-direction` when directions differ.
- `stats` holds counts and durations in milliseconds. They vary between runs; nothing else in the merge does.
- Mods whose author opted out ("do not port") are removed from `rows` and counted in no total (not built yet: no opt-out exists).
- The board links to the mod's own page (`page_url`) and never to a converted JAR.

## Board summary: `board-summary.json`

The input of `tools/ledger.py`, which the bot commits as `ledger/inputs/board-summary.json`. Its shape is the one `tools/ledger.py` validates (unknown keys are errors there), summed over all directions; per-direction numbers and durations are in `board.json`.

```json
{
  "schema": 1, "run": "nightly-20261104", "date": "2026-11-04", "commit": "<40 hex>",
  "game_versions": ["1.21.1", "1.21.11"],
  "mods": 1000, "green": 0, "yellow": 0, "red": 1000, "not_run": 0,
  "harness": { "completed": 0, "attempted": 0 },
  "areas": [],
  "red_reasons": [ { "reason": "No converter yet (milestone M2)", "count": 1000 } ]
}
```

`date` is the UTC date the first shard started. `not_run` is always 0 (see the rules above). `areas` stays empty until the harness measures conversion areas.

## Reports

One JSON report per mod, game version and direction, written by `crates/board` (`Report`; `crates/report` has no whole-report type yet). Its `entries` are `crates/report` `ReportEntry` values. It holds every pipeline step (`download`, `inspect`, `classify`, `convert`, `harness`, `round_trip`), each with a `status` of `ok`, `failed`, `not-built` or `not-run` and the reason. A report holds no clock time and no duration, so the same file and engine give the same bytes.

- The canonical JSON is compact, with fields in a fixed order. `report_hash` is its SHA-256; issue forms ask for it.
- KV key: `report_key`, `report:<direction>:<project_id>:<game_version>`.
- In `reports.tar.gz` (a release asset and an R2 object): `reports/<direction>/<project_id>/<game_version>.json`. The tarball is deterministic: sorted entries, mode 0644, owner 0, mtime 0, fixed gzip header.

## Publication

1. `board.yml` (nightly, 02:00 UTC) splits the list into 10 shards. Each shard writes `shard-<n>.json` and its reports as workflow artifacts.
2. The merge job builds `board.json`, `board-summary.json`, `reports.tar.gz` and the Cloudflare import files, then the workflow writes Cloudflare directly with wrangler and the `CLOUDFLARE_API_TOKEN` secret (D-021):
   - D1 `board_runs` and `board_rows`. The tables have no direction column, so the D1 run id is `<run_id>:<direction>`, for example `nightly-20261104:fabric-neoforge`. A rerun deletes that run's rows first. `board_rows.report_key` holds the KV key;
   - KV: every report under its `report_key`, then `board.json` under `board.json` (last, so the site switches only once the reports are in place);
   - R2: `reports.tar.gz` at `reports/<run_id>.tar.gz` in `jarrow-reports`.
3. The same files are uploaded to the GitHub release `board-latest` as a public, rebuildable mirror, replacing the previous assets. A dated `board-YYYYMMDD` release keeps `board.json` as history.
4. `ledger/inputs/board-summary.json` and `LEDGER.md` are committed by the bot, which uses the `BOT_DEPLOY_KEY` deploy key that the `main` ruleset allows to bypass.
5. `tools/board_summary.py` posts one Discussion per direction in the "Nightly results" category (or "Announcements" when it is missing), titled "Nightly YYYY-MM-DD: G green, Y yellow, R red" (with the direction in the title when the board has several), once per date and direction.
