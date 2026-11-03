# The board pipeline

**Status: milestone M1, wave 1, one column.** The pipeline lists, downloads, verifies, inspects and reports, for one direction: Fabric → NeoForge. The classifier, the converter and the harness are not built yet, so every file is `not-run` and every mod ends red with the reason "No converter yet (milestone M2)". No mod is green, and none can be without a harness score. The other 11 directions have no column.

The pipeline is `crates/board` (package `jarrow-board`, binary `jarrow-board`) and `.github/workflows/board.yml`. The files it writes are defined in [board.md](board.md) (schema 2). `tools/board_summary.py` posts the nightly Discussion, and `tools/ledger.py` turns `board-summary.json` into `LEDGER.md`.

## Directions are inputs

A source list belongs to a loader, and a shard measures a direction, `<source>-<target>`: two different loader ids out of `fabric`, `neoforge`, `forge` and `quilt`. The source loader's Modrinth tag picks the facet (`categories:<tag>`) and the files (the version's `loaders` must include it), so every direction from that loader shares its list. The tag comes from the loader's profile (`modrinth_loader` in `loaders/<id>.toml`); only Fabric's is researched, so `list` refuses the other three loaders until theirs are. The target loader names the conversion and decides `target_build_exists`. Game versions are a list on the command line. Nothing else in the crate assumes Fabric, NeoForge or a particular game version, and the merge always lists all 12 directions in build order, with a column for each one it was given shards for. Fabric → NeoForge on 1.20.1, 1.21.1 and 1.21.11 is the first and only column.

## Steps

```text
jarrow-board list  --loader fabric --top 1000 --game-versions 1.20.1,1.21.1,1.21.11 --out board-list-fabric.json
jarrow-board shard --list board-list-fabric.json --direction fabric-neoforge --shard 3/10 \
                   --out shard-fabric-neoforge-3.json --reports-dir reports/
jarrow-board merge shard-*.json --reports-dir reports/ --out-dir out/
```

### `list`

Modrinth search (`GET /v2/search`) with the facets `project_type:mod`, `categories:<loader tag>` and the pinned versions OR-ed together, sorted by downloads, 100 per page. For each hit, one `GET /v2/project/<id>/version` filtered to the loader tag and the pinned versions, without changelogs. Per pinned version the newest release is picked, or the newest beta when there is no release; alphas never. Only a `.jar` on `https://cdn.modrinth.com/` with a sha512 counts, and the primary file wins. Hits without any such file go to `skipped`. Paging stops once `--top` mods with files are found or the results run out.

| Flag | Default | Meaning |
| --- | --- | --- |
| `--loader <id>` | required | The source loader; its profile's Modrinth tag picks facets and files (only `fabric` today) |
| `--top N` | `1000` | Mods to keep, counting only mods with at least one usable file (1 to 10,000) |
| `--game-versions V1,V2` | required | Pinned release versions, comma-separated; sorted and deduplicated |
| `--out FILE` | required | Where to write `board-list-<loader>.json` |
| `--api-base URL` | `https://api.modrinth.com/v2` | Modrinth API base |

### `shard`

A shard measures one direction over its source loader's list; the direction's source must be the list's loader. A mod belongs to shard `fnv1a64(project_id) % M` (1-based `N` is that bucket plus one), so a mod keeps its shard however the list changes. For every file of every mod in the shard, one after another:

1. Download into a fresh temporary directory, at most the listed size (and never above `--max-file-mib`), hashing while writing. A size or sha512 mismatch, an HTTP error after retries, or a file above the limit is a `not-run` result with `download_failed` (a red row), not an error.
2. Run the per-mod pipeline through the `ModPipeline` trait (`crates/board/src/pipeline.rs`):
   - **inspect**: the zip central directory only: size, entry count, and whether `fabric.mod.json`, `quilt.mod.json`, `META-INF/mods.toml` and `META-INF/neoforge.mods.toml` are present. An unreadable zip is a failed step, recorded in the report.
   - **classify**: not built; tier `null`, reason "classifier not built yet (M1 wave 2)".
   - **convert**: not built; `no_converter`, "No converter yet (milestone M2)".
   - **harness**: not run, because nothing was converted.
   - **round trips**: the hook for the classfile and manifest round trips; "not run yet" in wave 1.
3. Delete the JAR, then write the report (`<reports-dir>/<direction>/<project_id>/<game_version>.json`).

The verdict comes from the steps: a harness score decides when the harness ran (1.0 and 0.75 green, 0.5 yellow, 0.25 and 0 red, D-024); otherwise the result is `not-run` with the convert step's reason. The row takes its primary result's outcome, with `not-run` counted as red (yellow for `client_only_not_measured`). The shard fails only for infrastructure problems: an unreadable list, a list from another loader, an unwritable reports directory, a full disk.

| Flag | Default | Meaning |
| --- | --- | --- |
| `--list FILE` | required | `board-list-<loader>.json` from `list` |
| `--direction <source>-<target>` | required | The direction to measure; its source must be the list's loader |
| `--shard N/M` | required | Shard `N` of `M`, 1-based, `M` at most 1,000 |
| `--out FILE` | required | Where to write `shard-<direction>-N.json` |
| `--reports-dir DIR` | `reports` | Where the reports go |
| `--run-id ID` | `local-YYYYMMDD` | The run id every shard of the night shares (letters, digits, `-`, `_`, `.`) |
| `--git-sha SHA` | `$GITHUB_SHA`, else `unknown` | The commit being run; `merge` needs a 40-character sha for the ledger |
| `--rules-dir DIR` | `rules` | Rule-table root; the rules version hashes `<DIR>/<direction>/`, `none` without tables |
| `--work-dir DIR` | system temp | Parent of the temporary download directory |
| `--max-file-mib N` | `512` | Largest JAR to download, in MiB |

### `merge`

Reads every shard file (a directory argument means every `shard-*.json` in it), checks that they come from one run and one engine, that every shard `1..M` of every direction is present exactly once, that no project appears twice in a direction, that every result's outcome agrees with its score and every row with its primary result, and that every report exists with the hash its shard recorded and agrees with its row. Then it writes, under `--out-dir`:

| File | What |
| --- | --- |
| `board.json` | The board ([board.md](board.md)): all 12 directions, a column for each measured one |
| `board-summary.json` | The `tools/ledger.py` input: the first column in build order |
| `reports.tar.gz` | Every report, deterministic tarball |
| `cloudflare/kv-reports.json` | `wrangler kv bulk put` input: `[{"key": "report:<direction>:<project_id>:<game_version>", "value": "<report JSON>"}]` |
| `cloudflare/d1-board.sql` | `wrangler d1 execute --file` input in the schema of `infra/d1/migrations/0002_directions.sql`: deletes the run's earlier rows, inserts `board_runs`, then `board_rows` (one per mod, direction and game version) 50 per `INSERT` |

The merge reads no clock and no environment. The same shard files give byte-identical outputs whatever order they are named in (tested, and the tarball bytes are pinned so Linux, macOS and Windows must agree). `crates/board/tests/sample.rs` keeps a sample `board.json` and `board-summary.json` in `crates/board/tests/fixtures/`, and the tests of the site, `tools/board_summary.py` and `tools/ledger.py` read them, so the writer and every reader agree on one contract (regenerate with `JARROW_BLESS=1 cargo test -p jarrow-board --test sample`).

| Flag | Default | Meaning |
| --- | --- | --- |
| `SHARD...` | required | `shard-<direction>-N.json` files, or directories holding them |
| `--reports-dir DIR` | `reports` | Where every shard's reports are |
| `--out-dir DIR` | required | Output directory |

## HTTP

ureq over rustls with the ring provider and the operating system's root certificates (ureq's bundled roots are CDLA-Permissive-2.0, which `deny.toml` does not allow). Every request sends `User-Agent: MosheByte/jarrow-board/<version> (github.com/MosheByte/jarrow)` and nothing else of note: no cookies, no tokens. A 429 waits for `Retry-After` (or `X-Ratelimit-Reset`) and retries; when Modrinth reports `X-Ratelimit-Remaining: 0` the next request waits for the window to reset; 5xx and network errors back off 2, 4, 8 s. Four attempts per request, no single wait above two minutes. Bodies are capped (32 MiB for API responses, the listed size for JARs).

## The nightly: `.github/workflows/board.yml`

Scheduled at 02:00 UTC, and `workflow_dispatch` with the inputs `top` (default `1000`) and `refresh_list` (refetch even when the cached list is fresh). One direction per run for now (`DIRECTION: fabric-neoforge`, `SOURCE_LOADER: fabric`, `GAME_VERSIONS: 1.20.1,1.21.1,1.21.11`).

| Job | Permissions | What |
| --- | --- | --- |
| `build` | `contents: read` | `cargo build --release --locked -p jarrow-board`, uploaded as an artifact; names the run `nightly-YYYYMMDD` |
| `list` | `contents: read` | Reuses `board-list-<loader>.json` from the `board-list` release when it is younger than 7 days and was fetched with schema 2 for the same loader, versions and `top`; otherwise fetches it from Modrinth |
| `list-release` | `contents: write` | Only after a refetch, on `main`: keeps the list as a `board-list` release asset |
| `shard` (10) | `contents: read` | One shard each; checks no JAR is left behind; uploads `shard-<direction>-N.json` and its reports |
| `merge` | `contents: read` | Merges the shards and writes a step summary |
| `publish-cloudflare` | `contents: read` | `wrangler whoami`, then D1 (`d1 execute --remote --file`, into the tables of migration 0002, which `infra.yml` applies), KV reports (`kv bulk put --remote`), R2 tarball (`r2 object put --remote` at `reports/<run_id>.tar.gz`), and KV `board.json` last (`kv key put --remote`) |
| `publish-release` | `contents: write` | Replaces the assets of `board-latest` (creates it if missing) and creates `board-YYYYMMDD` with `board.json` |
| `publish-ledger` | `contents: read` | Copies `board-summary.json` to `ledger/inputs/`, runs `tools/ledger.py`, commits both as jarrow-bot and pushes with `BOT_DEPLOY_KEY` |
| `publish-discussion` | `contents: read`, `discussions: write` | `tools/board_summary.py` with `GH_TOKEN` |

Publishing runs only on `main` and never for Dependabot. The workflow has no `pull_request` trigger, so a fork never sees a secret. `CLOUDFLARE_API_TOKEN` is set only on the wrangler steps and reaches only Cloudflare; `wrangler whoami` output stays out of the log. The deploy key goes to a 0600 file in `RUNNER_TEMP`, removed by a trap, used only through `GIT_SSH_COMMAND`, with GitHub's host keys from `gh api meta`. The bot's commit message ends with the `Co-Authored-By` trailer and carries `Jarrow-Generated: board`.

**Time budget.** The job timeouts on the longest path add up to 4 h 30 min (build 30, list 60, shard 120, merge 20, publish 30, Discussion 10), inside the 5-hour limit. A real shard of about 100 mods and up to 300 files (three pinned versions) takes minutes: the live dry run below downloaded 40 files in 6 s.

## Running it locally

```sh
cargo build -p jarrow-board
jarrow-board list --loader fabric --top 20 --game-versions 1.20.1,1.21.1,1.21.11 --out board-list-fabric.json
jarrow-board shard --list board-list-fabric.json --direction fabric-neoforge --shard 1/1 \
  --out shards/shard-fabric-neoforge-1.json --reports-dir reports --git-sha "$(git rev-parse HEAD)"
jarrow-board merge shards --reports-dir reports --out-dir out
python tools/board_summary.py --board out/board.json --dry-run
```

Dry run on 2026-11-03 against the live Modrinth API, with the schema-1 pipeline before the direction model (`--top 20`, one shard of all 20, 1.21.1 and 1.21.11 only): the list took 5 s (1 search page, 20 version requests) and found 20 mods with 40 files (every mod ships for both 1.21.1 and 1.21.11), none skipped. The shard downloaded and verified 61,112,986 bytes in 6.4 s with no download or inspect failure; all 40 JARs carry `fabric.mod.json` and none of the other manifests. The merge wrote 40 reports: 20 mods, 0 green, 0 yellow, 20 red, all `no_converter`. `tools/ledger.py` accepted the summary, and the generated D1 SQL ran twice against a local D1 with migration 0001 and left 40 rows. The schema-2 SQL is checked the same way offline: the test fixture's SQL, run twice against SQLite with migrations 0001 and 0002, leaves one row per mod, direction and game version, and the tables' checks refuse a green without a score, the tier `unknown` and the outcome `not_run`.

Tests: `cargo test -p jarrow-board` runs offline (hand-written Modrinth fixtures, JARs built in memory, a fake CDN). The live check is ignored by default: `cargo test -p jarrow-board --test live -- --ignored`.

## Not built yet

- The classifier (M1 wave 2), the converter (M2) and the harness run in the shard: their steps are recorded as not built or not run.
- The classfile and manifest round trips (M1 wave 2): the `round_trip` hook records "not run yet".
- Reading `mod_id` and the mod's own version from the JAR's manifest (M1 wave 2).
- Author opt-outs (`opt_out`): no opt-out list is read yet, because none exists.
- More than one direction per nightly: the merge, the board and the Discussion handle several (tested with two columns from one list), the workflow runs one.
- Source lists for NeoForge, Forge and Quilt: their profiles do not record a Modrinth tag yet (`modrinth_loader = "unknown"`), so `list` refuses them.
