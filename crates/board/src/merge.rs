//! Merging shards into the board, the ledger summary, the reports tarball and the Cloudflare
//! import files.
//!
//! The merge reads only its inputs: no clock, no environment, no hash-map order. The same
//! shard files and reports give byte-identical outputs, whatever order the shards are named
//! in.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::Path;

use jarrow_report::Outcome;
use serde::{Deserialize, Serialize};

use crate::board::{Board, BoardRow, BoardStats, DirectionBoard, Totals, top_reasons};
use crate::gamever;
use crate::hash;
use crate::loader::Direction;
use crate::report::{self, Report, board_outcome, check_verdict};
use crate::shard::{ShardFile, is_run_id};
use crate::time;
use crate::{Error, LEDGER_SCHEMA, SCHEMA};

/// Rows per `INSERT` statement in the D1 import (D1 caps one statement at 100 KB).
pub const D1_ROWS_PER_INSERT: usize = 50;

/// `board-summary.json`, in exactly the shape `tools/ledger.py` validates (its unknown keys
/// are errors). It describes one column: the first measured direction in the build order
/// (Fabric to NeoForge, the only column today). Every column's own numbers are in
/// `board.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LedgerSummary {
    /// Always [`LEDGER_SCHEMA`] (1).
    pub schema: u32,
    /// The column these numbers are for.
    pub direction: Direction,
    /// The run id.
    pub run: String,
    /// The run's date, `YYYY-MM-DD` (UTC, from when the first shard started).
    pub date: String,
    /// The commit the run built, 40 hex characters.
    pub commit: String,
    /// The game versions the column measured.
    pub game_versions: Vec<String>,
    /// Rows in the column.
    pub mods: u64,
    /// Green rows.
    pub green: u64,
    /// Yellow rows.
    pub yellow: u64,
    /// Red rows.
    pub red: u64,
    /// Always 0: "not run" is red on the board.
    pub not_run: u64,
    /// Harness runs.
    pub harness: LedgerHarness,
    /// Conversion areas the harness measured (none until the harness runs).
    pub areas: Vec<LedgerArea>,
    /// Reasons of red rows, by label.
    pub red_reasons: Vec<LedgerReason>,
}

/// `harness` in the ledger summary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LedgerHarness {
    /// Runs that produced a score.
    pub completed: u64,
    /// Runs attempted.
    pub attempted: u64,
}

/// One area in the ledger summary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LedgerArea {
    /// Area name.
    pub area: String,
    /// Mods that exercise it.
    pub measured: u64,
    /// Of those, green.
    pub green: u64,
    /// Why it is out of scope, if it is.
    pub unsupported: Option<String>,
}

/// One red reason in the ledger summary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LedgerReason {
    /// The reason's label.
    pub reason: String,
    /// Red rows with it.
    pub count: u64,
}

/// Everything the merge produces, in memory.
#[derive(Debug, Clone)]
pub struct MergeOutput {
    /// `board.json`.
    pub board: Board,
    /// `board-summary.json`.
    pub summary: LedgerSummary,
    /// Every report: `(path inside the tarball and the reports dir, canonical bytes)`,
    /// sorted by path.
    pub reports: Vec<(String, Vec<u8>)>,
}

fn check(cond: bool, message: impl FnOnce() -> String) -> Result<(), Error> {
    if cond {
        Ok(())
    } else {
        Err(Error::invalid(message()))
    }
}

/// Merges shard files. Every shard of every measured direction must be present exactly once,
/// every shard must come from the same run and engine, and every result's report must be in
/// `reports_dir` with the hash the shard recorded. The board lists all twelve directions; the
/// ones the shards measured get a column.
pub fn merge(shards: &[ShardFile], reports_dir: &Path) -> Result<MergeOutput, Error> {
    let first = shards
        .first()
        .ok_or_else(|| Error::invalid("merge needs at least one shard file"))?;
    let run_id = first.run_id.clone();
    let engine = first.engine.clone();
    check(is_run_id(&run_id), || {
        format!("run id {run_id:?} is malformed")
    })?;

    let mut by_direction: BTreeMap<Direction, Vec<&ShardFile>> = BTreeMap::new();
    for shard in shards {
        check(shard.schema == SCHEMA, || {
            format!("shard {} has schema {}", shard.shard, shard.schema)
        })?;
        check(shard.run_id == run_id, || {
            format!(
                "shards come from different runs: {run_id} and {}",
                shard.run_id
            )
        })?;
        check(shard.engine == engine, || {
            format!(
                "shards were built by different engines: {}@{} and {}@{}",
                engine.version, engine.git_sha, shard.engine.version, shard.engine.git_sha
            )
        })?;
        check(
            shard.source_loader == shard.direction.source
                && shard.target_loader == shard.direction.target,
            || {
                format!(
                    "shard {} names loaders that are not its direction's",
                    shard.shard
                )
            },
        )?;
        check(
            time::is_rfc3339_utc(&shard.started_at) && time::is_rfc3339_utc(&shard.finished_at),
            || format!("shard {} has a malformed start or finish time", shard.shard),
        )?;
        by_direction.entry(shard.direction).or_default().push(shard);
    }

    let mut rows: Vec<BoardRow> = Vec::new();
    let mut columns: BTreeMap<Direction, DirectionBoard> = BTreeMap::new();
    let mut stats = BoardStats::default();
    let mut reports: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    for (direction, group) in &by_direction {
        let head = group[0];
        let total = head.shard.total;
        let mut seen = BTreeSet::new();
        for shard in group {
            check(shard.shard.total == total, || {
                format!("{direction}: shards disagree on the shard count")
            })?;
            check(seen.insert(shard.shard.index), || {
                format!("{direction}: shard {} is given twice", shard.shard)
            })?;
            check(
                shard.game_versions == head.game_versions
                    && shard.list_fetched_at == head.list_fetched_at,
                || format!("{direction}: shards were run from different lists"),
            )?;
            check(shard.rules_version == head.rules_version, || {
                format!("{direction}: shards disagree on the rules version")
            })?;
        }
        let missing: Vec<String> = (1..=total)
            .filter(|i| !seen.contains(i))
            .map(|i| format!("{i}/{total}"))
            .collect();
        check(missing.is_empty(), || {
            format!("{direction}: missing shards {}", missing.join(", "))
        })?;

        let mut direction_rows = Vec::new();
        let mut projects = BTreeSet::new();
        for shard in group {
            stats.shards += 1;
            stats.files += shard.stats.files;
            stats.downloaded_bytes += shard.stats.downloaded_bytes;
            stats.download_failures += shard.stats.download_failures;
            stats.inspect_failures += shard.stats.inspect_failures;
            stats.max_shard_wall_ms = stats.max_shard_wall_ms.max(shard.stats.wall_ms);
            stats.total_shard_wall_ms += shard.stats.wall_ms;
            stats.step_ms = stats.step_ms.plus(shard.stats.step_ms);
            for row in &shard.rows {
                check(row.direction == *direction, || {
                    format!(
                        "{direction}: row {} names another direction",
                        row.project_id
                    )
                })?;
                check(projects.insert(row.project_id.clone()), || {
                    format!("{direction}: project {} appears twice", row.project_id)
                })?;
                check_row(row, &head.game_versions)?;
                for result in &row.results {
                    let rel =
                        report::report_path(*direction, &row.project_id, &result.game_version);
                    let bytes = read_report(reports_dir, &rel)?;
                    let digest = format!("sha256:{}", hash::sha256_hex(&bytes));
                    check(digest == result.report_hash, || {
                        format!(
                            "{rel}: hash {digest} does not match the shard's {}",
                            result.report_hash
                        )
                    })?;
                    let parsed: Report = serde_json::from_slice(&bytes)
                        .map_err(|e| Error::json(format!("report {rel}"), e))?;
                    parsed.validate()?;
                    check(
                        parsed.direction == *direction
                            && parsed.project_id == row.project_id
                            && parsed.game_version == result.game_version
                            && parsed.tier == result.tier
                            && parsed.outcome == result.outcome
                            && parsed.score == result.score
                            && parsed.reason_kind == result.reason_kind,
                        || format!("{rel}: the report does not match its board row"),
                    )?;
                    reports.insert(format!("reports/{rel}"), bytes);
                }
                direction_rows.push(row.clone());
            }
        }
        columns.insert(
            *direction,
            DirectionBoard {
                column: true,
                game_versions: head.game_versions.clone(),
                list_fetched_at: Some(head.list_fetched_at.clone()),
                rules_version: Some(head.rules_version.clone()),
                totals: Totals::of(&direction_rows),
                top_reasons: top_reasons(&direction_rows),
                ..DirectionBoard::without_column(*direction)
            },
        );
        rows.extend(direction_rows);
    }

    rows.sort_by(|a, b| {
        a.direction
            .cmp(&b.direction)
            .then_with(|| b.downloads.cmp(&a.downloads))
            .then_with(|| a.project_id.cmp(&b.project_id))
    });
    let directions: Vec<DirectionBoard> = Direction::all()
        .into_iter()
        .map(|d| {
            columns
                .remove(&d)
                .unwrap_or_else(|| DirectionBoard::without_column(d))
        })
        .collect();
    let mut game_versions: Vec<String> = directions
        .iter()
        .flat_map(|d| d.game_versions.iter().cloned())
        .collect();
    gamever::sort_versions(&mut game_versions);
    let generated_at = shards
        .iter()
        .map(|s| s.finished_at.as_str())
        .max_by(|a, b| time::compare(a, b))
        .unwrap_or_default()
        .to_owned();
    let started_at = shards
        .iter()
        .map(|s| s.started_at.as_str())
        .min_by(|a, b| time::compare(a, b))
        .unwrap_or_default()
        .to_owned();
    let totals = directions
        .iter()
        .fold(Totals::default(), |acc, d| acc.plus(d.totals));
    let board = Board {
        schema: SCHEMA,
        generated_at,
        started_at,
        run_id,
        engine,
        game_versions,
        totals,
        directions,
        stats,
        rows,
    };
    let summary = ledger_summary(&board)?;
    Ok(MergeOutput {
        board,
        summary,
        reports: reports.into_iter().collect(),
    })
}

fn check_row(row: &BoardRow, game_versions: &[String]) -> Result<(), Error> {
    let id = &row.project_id;
    check(!row.results.is_empty(), || {
        format!("row {id} has no results")
    })?;
    check(row.outcome != Outcome::NotRun, || {
        format!("row {id} is not-run; the board counts a file without a harness score as red")
    })?;
    let newest = gamever::newest(row.results.iter().map(|r| r.game_version.as_str()));
    check(newest == Some(row.primary_game_version.as_str()), || {
        format!("row {id}: primary_game_version is not its newest result")
    })?;
    let mut versions = BTreeSet::new();
    for r in &row.results {
        check(r.direction == row.direction, || {
            format!("row {id} has a result for another direction")
        })?;
        check(game_versions.contains(&r.game_version), || {
            format!(
                "row {id} has a result for unpinned version {}",
                r.game_version
            )
        })?;
        check(versions.insert(r.game_version.as_str()), || {
            format!("row {id} has two results for {}", r.game_version)
        })?;
        check_verdict(
            r.outcome,
            r.score,
            r.reason_kind,
            r.reason.as_deref(),
            &format!("row {id}, {}", r.game_version),
        )?;
    }
    let primary = row
        .results
        .iter()
        .find(|r| r.game_version == row.primary_game_version);
    check(
        primary.is_some_and(|p| {
            row.outcome == board_outcome(p.outcome, p.reason_kind)
                && p.reason_kind == row.reason_kind
                && p.tier == row.tier
        }),
        || format!("row {id}: tier, outcome and reason must be the primary version's"),
    )
}

fn read_report(reports_dir: &Path, rel: &str) -> Result<Vec<u8>, Error> {
    let path = reports_dir.join(rel);
    fs::read(&path).map_err(|e| Error::io_at("reading report", &path, e))
}

/// The ledger input for `tools/ledger.py`: the numbers of the first column in the build
/// order.
pub fn ledger_summary(board: &Board) -> Result<LedgerSummary, Error> {
    let date = time::date_of(&board.started_at)
        .ok_or_else(|| Error::invalid("the board's started_at is malformed"))?
        .to_owned();
    check(hash::is_hex(&board.engine.git_sha, 40), || {
        format!(
            "engine.git_sha {:?} is not a 40-character commit; tools/ledger.py needs one \
             (pass --git-sha to jarrow-board shard)",
            board.engine.git_sha
        )
    })?;
    let column = board
        .directions
        .iter()
        .find(|d| d.column)
        .ok_or_else(|| Error::invalid("the board has no column to summarise"))?;
    let rows: Vec<&BoardRow> = board
        .rows
        .iter()
        .filter(|r| r.direction == column.id)
        .collect();
    let mut red: BTreeMap<&'static str, u64> = BTreeMap::new();
    for row in rows.iter().filter(|r| r.outcome == Outcome::Red) {
        if let Some(kind) = row.reason_kind {
            *red.entry(kind.label()).or_insert(0) += 1;
        }
    }
    let mut red_reasons: Vec<LedgerReason> = red
        .into_iter()
        .map(|(reason, count)| LedgerReason {
            reason: reason.to_owned(),
            count,
        })
        .collect();
    red_reasons.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.reason.cmp(&b.reason)));
    let attempted = rows
        .iter()
        .flat_map(|r| &r.results)
        .filter(|r| r.score.is_some())
        .count() as u64;
    Ok(LedgerSummary {
        schema: LEDGER_SCHEMA,
        direction: column.id,
        run: board.run_id.clone(),
        date,
        commit: board.engine.git_sha.clone(),
        game_versions: column.game_versions.clone(),
        mods: column.totals.mods,
        green: column.totals.green,
        yellow: column.totals.yellow,
        red: column.totals.red,
        not_run: 0,
        harness: LedgerHarness {
            completed: attempted,
            attempted,
        },
        areas: Vec::new(),
        red_reasons,
    })
}

/// `reports.tar.gz`: every report under `reports/`, sorted, with fixed metadata (mode 0644,
/// owner 0, mtime 0) and a fixed gzip header, so the bytes depend only on the reports.
pub fn reports_tarball(reports: &[(String, Vec<u8>)]) -> Result<Vec<u8>, Error> {
    let gz = flate2::GzBuilder::new()
        .mtime(0)
        .operating_system(255)
        .write(Vec::new(), flate2::Compression::new(6));
    let mut tar = tar::Builder::new(gz);
    tar.mode(tar::HeaderMode::Deterministic);
    let mut sorted: Vec<&(String, Vec<u8>)> = reports.iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    for (path, bytes) in sorted {
        let mut header = tar::Header::new_ustar();
        header
            .set_path(path)
            .map_err(|e| Error::io(format!("tar path {path}"), e))?;
        header.set_size(bytes.len() as u64);
        header.set_mode(0o644);
        header.set_uid(0);
        header.set_gid(0);
        header.set_mtime(0);
        header.set_entry_type(tar::EntryType::Regular);
        header.set_cksum();
        tar.append(&header, bytes.as_slice())
            .map_err(|e| Error::io(format!("tar entry {path}"), e))?;
    }
    let gz = tar
        .into_inner()
        .map_err(|e| Error::io("finishing reports.tar.gz", e))?;
    gz.finish()
        .map_err(|e| Error::io("compressing reports.tar.gz", e))
}

#[derive(Serialize)]
struct KvPair<'a> {
    key: String,
    value: &'a str,
}

/// The `wrangler kv bulk put` file for the reports: `[{"key", "value"}]`, sorted by key.
pub fn kv_reports_json(board: &Board, reports: &[(String, Vec<u8>)]) -> Result<Vec<u8>, Error> {
    let by_path: BTreeMap<&str, &[u8]> = reports
        .iter()
        .map(|(p, b)| (p.as_str(), b.as_slice()))
        .collect();
    let mut pairs = Vec::new();
    for row in &board.rows {
        for result in &row.results {
            let rel = format!(
                "reports/{}",
                report::report_path(row.direction, &row.project_id, &result.game_version)
            );
            let bytes = by_path
                .get(rel.as_str())
                .ok_or_else(|| Error::invalid(format!("no report for {rel}")))?;
            let value = std::str::from_utf8(bytes)
                .map_err(|_| Error::invalid(format!("{rel} is not UTF-8")))?;
            pairs.push(KvPair {
                key: result.report_key.clone(),
                value,
            });
        }
    }
    pairs.sort_by(|a, b| a.key.cmp(&b.key));
    let mut out = serde_json::to_vec(&pairs).map_err(|e| Error::json("kv-reports.json", e))?;
    out.push(b'\n');
    Ok(out)
}

/// A SQL string literal: single quotes doubled. NUL is refused (SQLite would cut the text).
pub fn sql_text(value: &str) -> Result<String, Error> {
    if value.contains('\0') {
        return Err(Error::invalid("a value for D1 contains a NUL character"));
    }
    Ok(format!("'{}'", value.replace('\'', "''")))
}

fn sql_opt(value: Option<&str>) -> Result<String, Error> {
    value.map_or_else(|| Ok("NULL".to_owned()), sql_text)
}

/// One column's entry in `board_runs.totals`.
#[derive(Serialize)]
struct RunTotals<'a> {
    mods: u64,
    green: u64,
    yellow: u64,
    red: u64,
    rules_version: Option<&'a str>,
}

/// The D1 import for `wrangler d1 execute jarrow --remote --file`, in the schema of
/// `infra/d1/migrations/0002_directions.sql`: delete the run's earlier rows (so a re-run
/// replaces them), insert the run into `board_runs` with its measured directions and their
/// totals, then one `board_rows` row per mod, direction and game version, with `is_primary`
/// and `board_outcome` on the entry that decides the row, [`D1_ROWS_PER_INSERT`] rows per
/// statement. Tiers and outcomes use the `board.json` strings (D-024).
pub fn d1_sql(board: &Board) -> Result<String, Error> {
    let mut sql = String::new();
    sql.push_str("-- Generated by jarrow-board merge from board.json. Do not edit.\n");
    sql.push_str(&format!("-- run {}\n", board.run_id));
    let id_sql = sql_text(&board.run_id)?;
    sql.push_str(&format!(
        "DELETE FROM board_rows WHERE run_id = {id_sql};\n"
    ));
    sql.push_str(&format!("DELETE FROM board_runs WHERE id = {id_sql};\n"));
    let measured: Vec<&DirectionBoard> = board.directions.iter().filter(|d| d.column).collect();
    let ids: Vec<Direction> = measured.iter().map(|d| d.id).collect();
    let totals: BTreeMap<String, RunTotals<'_>> = measured
        .iter()
        .map(|d| {
            (
                d.id.id(),
                RunTotals {
                    mods: d.totals.mods,
                    green: d.totals.green,
                    yellow: d.totals.yellow,
                    red: d.totals.red,
                    rules_version: d.rules_version.as_deref(),
                },
            )
        })
        .collect();
    sql.push_str(&format!(
        "INSERT INTO board_runs (id, started_at, finished_at, engine_version, game_versions, \
         directions, totals) VALUES ({id_sql}, {}, {}, {}, {}, {}, {});\n",
        sql_text(&board.started_at)?,
        sql_text(&board.generated_at)?,
        sql_text(&format!(
            "{}+{}",
            board.engine.version, board.engine.git_sha
        ))?,
        sql_text(
            &serde_json::to_string(&board.game_versions)
                .map_err(|e| Error::json("game_versions", e))?
        )?,
        sql_text(&serde_json::to_string(&ids).map_err(|e| Error::json("directions", e))?)?,
        sql_text(&serde_json::to_string(&totals).map_err(|e| Error::json("totals", e))?)?,
    ));
    let mut values = Vec::new();
    for row in &board.rows {
        for r in &row.results {
            let primary = r.game_version == row.primary_game_version;
            let score = r
                .score
                .map_or_else(|| "NULL".to_owned(), |s| format!("{s}"));
            values.push(format!(
                "({id_sql}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {score}, {}, {}, {}, {}, \
                 {}, {}, {})",
                sql_text(&row.direction.id())?,
                sql_text(&row.project_id)?,
                sql_text(&row.slug)?,
                sql_text(&r.game_version)?,
                row.downloads,
                sql_text(&r.version_id)?,
                sql_text(&r.file_sha512)?,
                sql_opt(r.mod_id.as_deref())?,
                sql_opt(r.mod_version.as_deref())?,
                sql_opt(r.tier.map(|t| t.id()))?,
                sql_text(r.outcome.id())?,
                sql_opt(r.reason_kind.map(|k| k.id()))?,
                sql_opt(r.reason.as_deref())?,
                sql_text(&r.report_key)?,
                sql_text(&r.report_hash)?,
                r.duration_ms,
                u8::from(primary),
                sql_opt(primary.then(|| row.outcome.id()))?,
            ));
        }
    }
    for chunk in values.chunks(D1_ROWS_PER_INSERT) {
        sql.push_str(
            "INSERT INTO board_rows (run_id, direction, project_id, slug, game_version, \
             downloads, version_id, file_sha512, mod_id, mod_version, tier, outcome, score, \
             reason_kind, reason, report_key, report_hash, duration_ms, is_primary, \
             board_outcome) VALUES\n",
        );
        sql.push_str(&chunk.join(",\n"));
        sql.push_str(";\n");
    }
    Ok(sql)
}

fn pretty(value: &impl Serialize, what: &str) -> Result<Vec<u8>, Error> {
    let mut out = serde_json::to_vec_pretty(value).map_err(|e| Error::json(what.to_owned(), e))?;
    out.push(b'\n');
    Ok(out)
}

/// The files the merge writes, relative to `--out-dir`, with their bytes.
pub fn output_files(out: &MergeOutput) -> Result<Vec<(String, Vec<u8>)>, Error> {
    Ok(vec![
        ("board.json".to_owned(), pretty(&out.board, "board.json")?),
        (
            "board-summary.json".to_owned(),
            pretty(&out.summary, "board-summary.json")?,
        ),
        ("reports.tar.gz".to_owned(), reports_tarball(&out.reports)?),
        (
            "cloudflare/kv-reports.json".to_owned(),
            kv_reports_json(&out.board, &out.reports)?,
        ),
        (
            "cloudflare/d1-board.sql".to_owned(),
            d1_sql(&out.board)?.into_bytes(),
        ),
    ])
}

/// Writes [`output_files`] under `out_dir`.
pub fn write_outputs(out: &MergeOutput, out_dir: &Path) -> Result<Vec<String>, Error> {
    let files = output_files(out)?;
    let mut written = Vec::new();
    for (rel, bytes) in files {
        let path = out_dir.join(&rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| Error::io_at("creating", parent, e))?;
        }
        let mut file = fs::File::create(&path).map_err(|e| Error::io_at("creating", &path, e))?;
        file.write_all(&bytes)
            .map_err(|e| Error::io_at("writing", &path, e))?;
        written.push(rel);
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sql_literals_escape_quotes() {
        assert_eq!(sql_text("it's").unwrap(), "'it''s'");
        assert_eq!(sql_text("").unwrap(), "''");
        assert!(sql_text("a\0b").is_err());
        assert_eq!(sql_opt(None).unwrap(), "NULL");
    }

    #[test]
    fn tarball_is_deterministic_and_sorted() {
        let a = vec![
            ("reports/b.json".to_owned(), b"{\"b\":1}".to_vec()),
            ("reports/a.json".to_owned(), b"{\"a\":1}".to_vec()),
        ];
        let b = vec![a[1].clone(), a[0].clone()];
        let ta = reports_tarball(&a).unwrap();
        let tb = reports_tarball(&b).unwrap();
        assert_eq!(ta, tb);
        // gzip header: magic, deflate, no flags, mtime 0, OS 255.
        assert_eq!(&ta[..4], &[0x1f, 0x8b, 8, 0]);
        assert_eq!(&ta[4..8], &[0, 0, 0, 0]);
        assert_eq!(ta[9], 255);
        let mut names = Vec::new();
        let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(ta.as_slice()));
        for entry in archive.entries().unwrap() {
            let entry = entry.unwrap();
            assert_eq!(entry.header().mtime().unwrap(), 0);
            assert_eq!(entry.header().mode().unwrap(), 0o644);
            names.push(entry.path().unwrap().to_string_lossy().into_owned());
        }
        assert_eq!(names, vec!["reports/a.json", "reports/b.json"]);
    }
}
