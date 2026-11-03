# The board pipeline

**Status: milestone M1, wave 1.** The pipeline lists, downloads, verifies, inspects and reports. The classifier, the converter and the harness are not built yet, so every mod ends red with the reason "No converter yet (milestone M2)". No mod is green, and none can be without a harness score.

The pipeline is `crates/board` (package `jarrow-board`, binary `jarrow-board`) and `.github/workflows/board.yml`. The files it writes are defined in [board.md](board.md). `tools/board_summary.py` posts the nightly Discussion, and `tools/ledger.py` turns `board-summary.json` into `LEDGER.md`.

## Directions are inputs

Every step takes a direction, `<source>-<target>`: two different loader ids out of `fabric`, `neoforge`, `forge` and `quilt`. The source loader picks the Modrinth facet (`categories:<source>`) and the files (the version's `loaders` must include it). The target loader names the conversion and decides `target_build_exists`. Game versions are a list on the command line. Nothing in the crate assumes Fabric, NeoForge or a particular game version. Fabric to NeoForge on 1.21.1 and 1.21.11 is the first column.

## Steps

```text
jarrow-board list  --direction fabric-neoforge --top 1000 --game-versions 1.21.1,1.21.11 --out board-list.json
jarrow-board shard --list board-list.json --shard 3/10 --out shard-3.json --reports-dir reports/
jarrow-board merge shard-*.json --reports-dir reports/ --out-dir out/
```

### `list`

Modrinth search (`GET /v2/search`) with the facets `project_type:mod`, `categories:<source loader>` and the pinned versions OR-ed together, sorted by downloads, 100 per page. For each hit, one `GET /v2/project/<id>/version` filtered to the source loader and the pinned versions, without changelogs. Per pinned version the newest release is picked, or the newest beta when there is no release; alphas never. Only a `.jar` on `https://cdn.modrinth.com/` with a sha512 counts, and the primary file wins. Hits without any such file go to `skipped`. Paging stops once `--top` mods with files are found or the results run out.

| Flag | Default | Meaning |
| --- | --- | --- |
| `--direction <source>-<target>` | required | The direction; its source loader picks facets and files |
| `--top N` | `1000` | Mods to keep, counting only mods with at least one usable file (1 to 10,000) |
| `--game-versions V1,V2` | required | Pinned release versions, comma-separated; sorted and deduplicated |
| `--out FILE` | required | Where to write `board-list.json` |
| `--api-base URL` | `https://api.modrinth.com/v2` | Modrinth API base |

### `shard`

A mod belongs to shard `fnv1a64(project_id) % M` (1-based `N` is that bucket plus one), so a mod keeps its shard however the list changes. For every file of every mod in the shard, one after another:

1. Download into a fresh temporary directory, at most the listed size (and never above `--max-file-mib`), hashing while writing. A size or sha512 mismatch, an HTTP error after retries, or a file above the limit is a red result with `download_failed`, not an error.
2. Run the per-mod pipeline through the `ModPipeline` trait (`crates/board/src/pipeline.rs`):
   - **inspect**: the zip central directory only: size, entry count, and whether `fabric.mod.json`, `quilt.mod.json`, `META-INF/mods.toml` and `META-INF/neoforge.mods.toml` are present. An unreadable zip is a failed step, recorded in the report.
   - **classify**: not built; tier `unknown`, reason "classifier not built yet (M1 wave 2)".
   - **convert**: not built; red, `no_converter`, "No converter yet (milestone M2)".
   - **harness**: not run, because nothing was converted.
   - **round trips**: the hook for the classfile and manifest round trips; "not run yet" in wave 1.
3. Delete the JAR, then write the report (`<reports-dir>/<direction>/<project_id>/<game_version>.json`).

The verdict comes from the steps: a harness score decides when the harness ran (1.0 and 0.75 green, 0.5 yellow, anything else red, D-024); otherwise the result is red with the convert step's reason. The shard fails only for infrastructure problems: an unreadable list, an unwritable reports directory, a full disk.

| Flag | Default | Meaning |
| --- | --- | --- |
| `--list FILE` | required | `board-list.json` from `list` |
| `--shard N/M` | required | Shard `N` of `M`, 1-based, `M` at most 1,000 |
| `--out FILE` | required | Where to write `shard-N.json` |
| `--reports-dir DIR` | `reports` | Where the reports go |
| `--run-id ID` | `local-YYYYMMDD` | The run id every shard of the night shares (letters, digits, `-`, `_`, `.`) |
| `--git-sha SHA` | `$GITHUB_SHA`, else `unknown` | The commit being run; `merge` needs a 40-character sha for the ledger |
| `--rules-dir DIR` | `rules` | Rule-table root; the rules version hashes `<DIR>/<direction>/`, `none` without tables |
| `--work-dir DIR` | system temp | Parent of the temporary download directory |
| `--max-file-mib N` | `512` | Largest JAR to download, in MiB |

### `merge`

Reads every shard file (a directory argument means every `shard-*.json` in it), checks that they come from one run and one engine, that every shard `1..M` of every direction is present exactly once, that no project appears twice, and that every report exists with the hash its shard recorded and agrees with its row. Then it writes, under `--out-dir`:

| File | What |
| --- | --- |
| `board.json` | The board ([board.md](board.md)) |
| `board-summary.json` | The `tools/ledger.py` input |
| `reports.tar.gz` | Every report, deterministic tarball |
| `cloudflare/kv-reports.json` | `wrangler kv bulk put` input: `[{"key": "report:<direction>:<project_id>:<game_version>", "value": "<report JSON>"}]` |
| `cloudflare/d1-board.sql` | `wrangler d1 execute --file` input: deletes the run's earlier rows, inserts `board_runs`, then `board_rows` 50 per `INSERT` |

The merge reads no clock and no environment. The same shard files give byte-identical outputs whatever order they are named in (tested, and the tarball bytes are pinned so Linux, macOS and Windows must agree).

| Flag | Default | Meaning |
| --- | --- | --- |
| `SHARD...` | required | `shard-N.json` files, or directories holding them |
| `--reports-dir DIR` | `reports` | Where every shard's reports are |
| `--out-dir DIR` | required | Output directory |

## HTTP

ureq over rustls with the ring provider and the operating system's root certificates (ureq's bundled roots are CDLA-Permissive-2.0, which `deny.toml` does not allow). Every request sends `User-Agent: MosheByte/jarrow-board/<version> (github.com/MosheByte/jarrow)` and nothing else of note: no cookies, no tokens. A 429 waits for `Retry-After` (or `X-Ratelimit-Reset`) and retries; when Modrinth reports `X-Ratelimit-Remaining: 0` the next request waits for the window to reset; 5xx and network errors back off 2, 4, 8 s. Four attempts per request, no single wait above two minutes. Bodies are capped (32 MiB for API responses, the listed size for JARs).

## The nightly: `.github/workflows/board.yml`

Scheduled at 02:00 UTC, and `workflow_dispatch` with the inputs `top` (default `1000`) and `refresh_list` (refetch even when the cached list is fresh). One direction per run for now (`DIRECTION: fabric-neoforge`, `GAME_VERSIONS: 1.21.1,1.21.11`).

| Job | Permissions | What |
| --- | --- | --- |
| `build` | `contents: read` | `cargo build --release --locked -p jarrow-board`, uploaded as an artifact; names the run `nightly-YYYYMMDD` |
| `list` | `contents: read` | Reuses `board-list.json` from the `board-list` release when it is younger than 7 days and was fetched for the same direction, versions and `top`; otherwise fetches it from Modrinth |
| `list-release` | `contents: write` | Only after a refetch, on `main`: keeps the list as the `board-list` release asset |
| `shard` (10) | `contents: read` | One shard each; checks no JAR is left behind; uploads `shard-N.json` and its reports |
| `merge` | `contents: read` | Merges the shards and writes a step summary |
| `publish-cloudflare` | `contents: read` | `wrangler whoami`, then D1 (`d1 execute --remote --file`), KV reports (`kv bulk put --remote`), R2 tarball (`r2 object put --remote` at `reports/<run_id>.tar.gz`), and KV `board.json` last (`kv key put --remote`) |
| `publish-release` | `contents: write` | Replaces the assets of `board-latest` (creates it if missing) and creates `board-YYYYMMDD` with `board.json` |
| `publish-ledger` | `contents: read` | Copies `board-summary.json` to `ledger/inputs/`, runs `tools/ledger.py`, commits both as jarrow-bot and pushes with `BOT_DEPLOY_KEY` |
| `publish-discussion` | `contents: read`, `discussions: write` | `tools/board_summary.py` with `GH_TOKEN` |

Publishing runs only on `main` and never for Dependabot. The workflow has no `pull_request` trigger, so a fork never sees a secret. `CLOUDFLARE_API_TOKEN` is set only on the wrangler steps and reaches only Cloudflare; `wrangler whoami` output stays out of the log. The deploy key goes to a 0600 file in `RUNNER_TEMP`, removed by a trap, used only through `GIT_SSH_COMMAND`, with GitHub's host keys from `gh api meta`. The bot's commit message ends with the `Co-Authored-By` trailer and carries `Jarrow-Generated: board`.

**Time budget.** The job timeouts on the longest path add up to 4 h 30 min (build 30, list 60, shard 120, merge 20, publish 30, Discussion 10), inside the 5-hour limit. A real shard of about 100 mods and 180 files takes minutes: the live dry run below downloaded 40 files in 6 s.

## Running it locally

```sh
cargo build -p jarrow-board
jarrow-board list --direction fabric-neoforge --top 20 --game-versions 1.21.1,1.21.11 --out board-list.json
jarrow-board shard --list board-list.json --shard 1/1 --out shards/shard-1.json --reports-dir reports \
  --git-sha "$(git rev-parse HEAD)"
jarrow-board merge shards --reports-dir reports --out-dir out
python tools/board_summary.py --board out/board.json --dry-run
```

Dry run on 2026-11-03 against the live Modrinth API (`--top 20`, one shard of all 20): the list took 5 s (1 search page, 20 version requests) and found 20 mods with 40 files (every mod ships for both 1.21.1 and 1.21.11), none skipped. The shard downloaded and verified 61,112,986 bytes in 6.4 s with no download or inspect failure; all 40 JARs carry `fabric.mod.json` and none of the other manifests. The merge wrote 40 reports: 20 mods, 0 green, 0 yellow, 20 red, all `no_converter`. `tools/ledger.py` accepted the summary, and the generated D1 SQL ran twice against a local D1 with migration 0001 and left 40 rows.

Tests: `cargo test -p jarrow-board` runs offline (hand-written Modrinth fixtures, JARs built in memory, a fake CDN). The live check is ignored by default: `cargo test -p jarrow-board --test live -- --ignored`.

## Not built yet

- The classifier (M1 wave 2), the converter (M2) and the harness run in the shard: their steps are recorded as not built or not run.
- The classfile and manifest round trips (M1 wave 2): the `round_trip` hook records "not run yet".
- Reading `mod_id` and the mod's own version from the JAR's manifest (M1 wave 2).
- Author opt-outs (`opt_out`): no opt-out list is read yet, because none exists.
- More than one direction per nightly: the merge, the board and the Discussion handle several, the workflow runs one.
