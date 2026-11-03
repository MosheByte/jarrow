# Board data contract

The nightly board produces three artefacts. The engine (`crates/board`), the site (`site/src/lib/board.ts`), `tools/board_summary.py` and `tools/ledger.py` all read them, so this file is the single definition. Change it only together with every reader.

## Mod list: `board-list.json`

Fetched weekly from the Modrinth API. It holds the top 1,000 projects with `project_type:mod`, `categories:fabric`, and at least one pinned game version (`versions:1.21.1` OR `versions:1.21.11`), ordered by downloads.

```json
{
  "schema": 1,
  "fetched_at": "2026-11-03T14:00:00Z",
  "game_versions": ["1.21.1", "1.21.11"],
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
      "neoforge_build_exists": true,
      "files": [
        { "game_version": "1.21.11", "version_id": "abc", "version_number": "0.6.0", "url": "https://cdn.modrinth.com/...", "sha512": "...", "size": 1234 }
      ]
    }
  ]
}
```

- `files` holds at most one entry per pinned game version: the newest Fabric release for that version, or the newest beta when no release exists.
- `neoforge_build_exists` is informational, taken from Modrinth's loader tags. It is not an author-verified "official port".
- JAR bytes are never stored. Each shard downloads, verifies `sha512`, processes, and deletes them.

## Board: `board.json`

```json
{
  "schema": 1,
  "generated_at": "2026-11-04T04:12:00Z",
  "run_id": "nightly-20261104",
  "engine": { "version": "0.1.0", "git_sha": "0123abcd" },
  "rules_version": "sha256 of the rules/ tree",
  "game_versions": ["1.21.1", "1.21.11"],
  "totals": { "mods": 1000, "green": 0, "yellow": 0, "red": 1000 },
  "top_reasons": [ { "kind": "no_converter", "count": 1000 } ],
  "rows": [
    {
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
      "neoforge_build_exists": true,
      "results": [
        {
          "game_version": "1.21.11",
          "version_id": "abc",
          "file_sha512": "...",
          "mod_id": "sodium",
          "mod_version": "0.6.0",
          "tier": "b",
          "tier_reasons": ["..."],
          "outcome": "red",
          "score": null,
          "reason_kind": "no_converter",
          "reason": "No converter yet (milestone M2)",
          "report_key": "reports/nightly-20261104/AANobbMI/1.21.11.json",
          "report_hash": "sha256:...",
          "duration_ms": 412
        }
      ]
    }
  ]
}
```

Rules:

- A mod's board `outcome` is the outcome on its primary game version, which is the newest pinned version it ships a Fabric file for. Every other version stays visible in `results`.
- `totals.mods == green + yellow + red`. "Not run" is not an outcome on the board: a mod the harness could not measure is red or yellow with the reason, never green.
- `outcome = green` requires a harness score of `1.0` or `0.75` ([harness.md](harness.md), D-024). Yellow is `0.5`, or `client_only_not_measured` when the mod loads but its content is client-only. Red covers everything else, including "harness not run" and "no converter yet".
- Tier values are `a`, `b`, `unsupported`, and outcome values are `green`, `yellow`, `red`, `not-run`, as serialised by `crates/report` (D-024). `site/src/lib/board.ts` and the D1 schema must use the same strings.
- `reason_kind` is a closed vocabulary defined in `crates/report` (`no_converter`, `unsupported_native`, `unsupported_transformer`, `unsupported_language_adapter`, `unsupported_obfuscated`, `license_personal_use`, `tier_b_required`, `harness_not_run_eula`, `harness_load_failure`, `class_not_found`, `mixin_failure`, `registry_conflict`, `crash`, `entries_differ`, `client_only_not_measured`, `download_failed`, `opt_out`). The board aggregates "top reasons" on it.
- Mods whose author opted out ("do not port") are removed from `rows` and counted in no total.
- The board links to the mod's own page (`page_url`) and never to a converted JAR.

## Reports

One JSON report per mod and game version, in the `crates/report` model, stored at `report_key`. The nightly also publishes all reports as the `reports.tar.gz` release asset. `report_hash` is the SHA-256 of the canonical report JSON; issue forms ask for it.

## Publication

1. `board.yml` (nightly, 02:00 UTC) splits the list into 10 shards. Each shard writes `shard-<n>.json` and its reports as workflow artifacts.
2. The merge job builds `board.json`, `board-summary.json` and `reports.tar.gz`, then writes them to Cloudflare directly with wrangler and the `CLOUDFLARE_API_TOKEN` secret (D-021):
   - KV `board.json` and the compact `board:rows` used for server rendering;
   - D1 `board_runs` and `board_rows`;
   - R2 reports at `report_key`.
3. The same files are uploaded to the GitHub release `board-latest` as a public, rebuildable mirror. A dated `board-YYYYMMDD` release keeps `board.json` as history.
4. `ledger/inputs/board-summary.json` and `LEDGER.md` are committed by the bot, which uses the `BOT_DEPLOY_KEY` deploy key that the `main` ruleset allows to bypass.
5. `tools/board_summary.py` posts the Discussion "Nightly YYYY-MM-DD: G green, Y yellow, R red; top reasons…" in the "Nightly results" category.
