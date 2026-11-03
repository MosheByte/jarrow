//! The whole pipeline offline: a list built in code, JARs built in memory and served by a
//! fake CDN, every shard, then the merge. Checks the shapes `docs/board.md` (schema 2) fixes,
//! merge determinism, and that nothing is ever green without a harness.

#![allow(
    clippy::unwrap_used,
    reason = "test helpers: a failed unwrap is a failed test"
)]

mod common;

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use common::{FakeFetch, jar};
use jarrow_board::board::Board;
use jarrow_board::hash::{sha256_hex, sha512_hex};
use jarrow_board::list::{BoardList, ListFile, ListMod};
use jarrow_board::merge::{self, LedgerSummary, merge, output_files};
use jarrow_board::pipeline::WaveOne;
use jarrow_board::report::{ReasonKind, Report, StepStatus};
use jarrow_board::shard::{self, ShardFile, ShardOptions, ShardSpec, run_shard, select};
use jarrow_board::time::FixedClock;
use jarrow_board::{Direction, Loader};
use jarrow_report::Outcome;

const SHA: &str = "0123456789abcdef0123456789abcdef01234567";
const SHARDS: u32 = 10;

struct Fixture {
    list: BoardList,
    cdn: FakeFetch,
}

fn fabric_neoforge() -> Direction {
    "fabric-neoforge".parse().unwrap()
}

fn file(project: &str, gv: &str, bytes: &[u8]) -> ListFile {
    let version_id = format!("V{}{}", &project[..4], gv.replace('.', ""));
    ListFile {
        game_version: gv.to_owned(),
        version_id: version_id.clone(),
        version_number: format!("1.0.0+{gv}"),
        version_type: "release".to_owned(),
        filename: format!("{project}-{gv}.jar"),
        url: format!(
            "https://cdn.modrinth.com/data/{project}/versions/{version_id}/{project}-{gv}.jar"
        ),
        sha512: sha512_hex(bytes),
        size: bytes.len() as u64,
    }
}

fn project(id: &str, downloads: u64, files: Vec<ListFile>) -> ListMod {
    ListMod {
        project_id: id.to_owned(),
        slug: id.to_ascii_lowercase(),
        title: format!("Project {id}"),
        downloads,
        license: Some("MIT".to_owned()),
        client_side: Some("optional".to_owned()),
        server_side: Some("optional".to_owned()),
        icon_url: None,
        loaders: vec!["fabric".to_owned()],
        files,
    }
}

/// 25 projects: 21 plain Fabric JARs (some for 1.20.1 or 1.21.1 as well), one multi-loader
/// JAR, one whose CDN bytes do not match the list, one that 404s, and one that is not a zip.
fn fixture() -> Fixture {
    let fabric = jar(&[
        "fabric.mod.json",
        "com/example/Mod.class",
        "assets/example/icon.png",
    ]);
    let multi = jar(&[
        "fabric.mod.json",
        "META-INF/neoforge.mods.toml",
        "META-INF/MANIFEST.MF",
    ]);
    let not_zip = b"this is not a zip archive".to_vec();
    let mut cdn = FakeFetch::default();
    let mut mods = Vec::new();
    for i in 0..21u64 {
        let id = format!("Fab{i:05}");
        let mut files = vec![file(&id, "1.21.11", &fabric)];
        if i % 3 == 0 {
            files.insert(0, file(&id, "1.21.1", &fabric));
        }
        if i % 7 == 0 {
            files.insert(0, file(&id, "1.20.1", &fabric));
        }
        for f in &files {
            cdn = cdn.with(&f.url, fabric.clone());
        }
        mods.push(project(&id, 10_000 - i * 100, files));
    }
    let multi_file = file("Neo00002", "1.21.1", &multi);
    cdn = cdn.with(&multi_file.url, multi.clone());
    let mut neo = project("Neo00002", 777, vec![multi_file]);
    neo.loaders = vec!["fabric".into(), "neoforge".into()];
    mods.push(neo);

    // Same length as the listed file, one byte different: only the sha512 can tell.
    let bad = file("Bad00003", "1.21.11", &fabric);
    let mut tampered = fabric.clone();
    tampered[40] ^= 0xff;
    cdn = cdn.with(&bad.url, tampered);
    mods.push(project("Bad00003", 500, vec![bad]));

    mods.push(project(
        "Mis00004",
        400,
        vec![file("Mis00004", "1.21.11", &fabric)],
    ));

    let nz = file("Zip00005", "1.21.11", &not_zip);
    cdn = cdn.with(&nz.url, not_zip.clone());
    mods.push(project("Zip00005", 300, vec![nz]));

    let list = BoardList {
        schema: 2,
        loader: Loader::Fabric,
        fetched_at: "2026-11-03T14:00:00Z".to_owned(),
        game_versions: vec!["1.20.1".into(), "1.21.1".into(), "1.21.11".into()],
        top: 1000,
        mods,
        skipped: vec![],
    }
    .normalized()
    .unwrap();
    Fixture { list, cdn }
}

fn options(reports: &Path, work: &Path) -> ShardOptions {
    ShardOptions {
        run_id: "nightly-20261104".to_owned(),
        git_sha: SHA.to_owned(),
        engine_version: jarrow_board::ENGINE_VERSION.to_owned(),
        rules_version: "none".to_owned(),
        reports_dir: reports.to_path_buf(),
        work_dir: Some(work.to_path_buf()),
        max_file_bytes: 64 * 1024 * 1024,
    }
}

fn run_direction(
    fx: &Fixture,
    direction: Direction,
    reports: &Path,
    work: &Path,
) -> Vec<ShardFile> {
    (1..=SHARDS)
        .map(|i| {
            let spec = ShardSpec::new(i, SHARDS).unwrap();
            let clock = FixedClock(format!("2026-11-04T02:{:02}:00Z", i));
            run_shard(
                &fx.cdn,
                &WaveOne,
                &fx.list,
                direction,
                spec,
                &options(reports, work),
                &clock,
                |_| {},
            )
            .unwrap()
        })
        .collect()
}

fn run_all(fx: &Fixture, reports: &Path, work: &Path) -> Vec<ShardFile> {
    run_direction(fx, fabric_neoforge(), reports, work)
}

fn keys(value: &serde_json::Value) -> BTreeSet<String> {
    value.as_object().unwrap().keys().cloned().collect()
}

fn set(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|s| (*s).to_owned()).collect()
}

#[test]
fn shards_partition_the_list() {
    let fx = fixture();
    let mut seen = BTreeSet::new();
    for i in 1..=SHARDS {
        for m in select(&fx.list, ShardSpec::new(i, SHARDS).unwrap()) {
            assert!(seen.insert(m.project_id.clone()), "{} twice", m.project_id);
        }
    }
    assert_eq!(seen.len(), fx.list.mods.len());
}

#[test]
fn end_to_end_board() {
    let fx = fixture();
    let tmp = tempfile::tempdir().unwrap();
    let reports = tmp.path().join("reports");
    let work = tmp.path().join("work");
    let shards = run_all(&fx, &reports, &work);

    // Every JAR was deleted: the work directory holds nothing after the shards.
    assert_eq!(fs::read_dir(&work).unwrap().count(), 0, "JARs left behind");

    let out = merge(&shards, &reports).unwrap();
    let board = &out.board;
    let n = fx.list.mods.len() as u64;
    assert_eq!(board.schema, 2);
    assert_eq!(board.totals.mods, n);
    assert_eq!(board.totals.green, 0);
    assert_eq!(board.totals.yellow, 0);
    assert_eq!(board.totals.red, n);

    // All twelve directions in the build order; only the measured one has a column.
    let ids: Vec<Direction> = board.directions.iter().map(|d| d.id).collect();
    assert_eq!(ids, Direction::all());
    let column = &board.directions[0];
    assert!(column.column);
    assert_eq!(column.id, fabric_neoforge());
    assert_eq!((column.order, column.bar_percent), (1, 60));
    assert_eq!(column.totals, board.totals);
    assert_eq!(
        column.list_fetched_at.as_deref(),
        Some("2026-11-03T14:00:00Z")
    );
    assert_eq!(column.rules_version.as_deref(), Some("none"));
    assert_eq!(column.game_versions, fx.list.game_versions);
    for d in &board.directions[1..] {
        assert!(!d.column, "{}", d.id);
        assert_eq!(d.totals.mods, 0);
        assert!(d.top_reasons.is_empty() && d.rules_version.is_none());
        assert!(d.game_versions.is_empty() && d.list_fetched_at.is_none());
    }
    assert_eq!(board.game_versions, fx.list.game_versions);
    let reasons: Vec<(ReasonKind, u64)> = column
        .top_reasons
        .iter()
        .map(|r| (r.kind, r.count))
        .collect();
    assert_eq!(
        reasons,
        vec![
            (ReasonKind::NoConverter, n - 2),
            (ReasonKind::DownloadFailed, 2)
        ]
    );
    assert_eq!(board.started_at, "2026-11-04T02:01:00Z");
    assert_eq!(board.generated_at, "2026-11-04T02:10:00Z");
    assert_eq!(board.stats.shards, u64::from(SHARDS));
    assert_eq!(board.stats.download_failures, 2);
    assert_eq!(board.stats.inspect_failures, 1);
    let files: usize = fx.list.mods.iter().map(|m| m.files.len()).sum();
    assert_eq!(board.stats.files, files as u64);
    assert_eq!(out.reports.len(), files);

    // Rows: sorted by downloads, red on the board, every result not-run (no harness ran),
    // outcome and reason from the primary version.
    let downloads: Vec<u64> = board.rows.iter().map(|r| r.downloads).collect();
    let mut sorted = downloads.clone();
    sorted.sort_by(|a, b| b.cmp(a));
    assert_eq!(downloads, sorted);
    for row in &board.rows {
        assert_eq!(row.direction, fabric_neoforge());
        assert_eq!(row.outcome, Outcome::Red);
        assert_eq!(row.tier, None);
        assert_eq!(row.report_url, None);
        assert!(row.page_url.starts_with("https://modrinth.com/mod/"));
        let newest = row.results.last().unwrap();
        assert_eq!(row.primary_game_version, newest.game_version);
        for r in &row.results {
            assert_eq!(r.direction, row.direction);
            assert_eq!(r.outcome, Outcome::NotRun);
            assert_eq!(r.score, None);
            assert_eq!(r.tier, None);
            assert!(r.report_key.starts_with("report:fabric-neoforge:"));
            assert!(r.report_hash.starts_with("sha256:"));
        }
    }
    let three = board
        .rows
        .iter()
        .find(|r| r.project_id == "Fab00000")
        .unwrap();
    let versions: Vec<&str> = three
        .results
        .iter()
        .map(|r| r.game_version.as_str())
        .collect();
    assert_eq!(versions, ["1.20.1", "1.21.1", "1.21.11"]);
    assert_eq!(three.primary_game_version, "1.21.11");
    let bad = board
        .rows
        .iter()
        .find(|r| r.project_id == "Bad00003")
        .unwrap();
    assert_eq!(bad.reason_kind, Some(ReasonKind::DownloadFailed));
    assert!(bad.reason.as_deref().unwrap().contains("sha512 mismatch"));
    let missing = board
        .rows
        .iter()
        .find(|r| r.project_id == "Mis00004")
        .unwrap();
    assert!(missing.reason.as_deref().unwrap().contains("HTTP 404"));
    let neo = board
        .rows
        .iter()
        .find(|r| r.project_id == "Neo00002")
        .unwrap();
    assert!(neo.target_build_exists);
    assert!(!three.target_build_exists);
    assert_eq!(neo.primary_game_version, "1.21.1");

    // Reports: one per result, canonical, hashed, honest about every step.
    let (path, bytes) = out
        .reports
        .iter()
        .find(|(p, _)| p == "reports/fabric-neoforge/Neo00002/1.21.1.json")
        .unwrap();
    assert_eq!(
        format!("sha256:{}", sha256_hex(bytes)),
        neo.results[0].report_hash,
        "{path}"
    );
    let report: Report = serde_json::from_slice(bytes).unwrap();
    assert_eq!(report.schema, 2);
    assert_eq!(report.steps.download.status, StepStatus::Ok);
    assert!(report.steps.download.sha512_verified);
    assert_eq!(report.steps.inspect.status, StepStatus::Ok);
    assert_eq!(report.steps.inspect.entries, Some(3));
    let manifests = report.steps.inspect.manifests.unwrap();
    assert!(manifests.fabric_mod_json && manifests.neoforge_mods_toml);
    assert!(!manifests.quilt_mod_json && !manifests.mods_toml);
    assert_eq!(report.steps.classify.status, StepStatus::NotBuilt);
    assert_eq!(report.steps.classify.tier, None);
    assert_eq!(
        report.steps.classify.reasons,
        vec!["classifier not built yet (M1 wave 2)"]
    );
    assert_eq!(report.steps.convert.status, StepStatus::NotBuilt);
    assert_eq!(
        report.steps.convert.reason,
        "No converter yet (milestone M2)"
    );
    assert_eq!(report.steps.harness.status, StepStatus::NotRun);
    assert_eq!(report.steps.round_trip.status, StepStatus::NotRun);
    assert!(report.steps.round_trip.reason.starts_with("not run yet"));
    assert_eq!(report.outcome, Outcome::NotRun);
    assert_eq!(report.tier, None);
    assert_eq!(report.entries.len(), 1);

    let zip = out
        .reports
        .iter()
        .find(|(p, _)| p.contains("/Zip00005/"))
        .map(|(_, b)| serde_json::from_slice::<Report>(b).unwrap())
        .unwrap();
    assert_eq!(zip.steps.inspect.status, StepStatus::Failed);
    assert_eq!(zip.reason_kind, Some(ReasonKind::NoConverter));

    let failed = out
        .reports
        .iter()
        .find(|(p, _)| p.contains("/Bad00003/"))
        .map(|(_, b)| serde_json::from_slice::<Report>(b).unwrap())
        .unwrap();
    assert_eq!(failed.steps.download.status, StepStatus::Failed);
    assert_eq!(failed.steps.inspect.status, StepStatus::NotRun);
    assert_eq!(failed.steps.convert.reason_kind, ReasonKind::DownloadFailed);
    assert_eq!(failed.outcome, Outcome::NotRun);

    // The ledger summary describes the column.
    assert_eq!(out.summary.direction, fabric_neoforge());
    assert_eq!(out.summary.mods, n);
    assert_eq!(out.summary.red, n);
}

#[test]
fn board_json_has_the_contract_shape() {
    let fx = fixture();
    let tmp = tempfile::tempdir().unwrap();
    let shards = run_all(&fx, &tmp.path().join("reports"), &tmp.path().join("work"));
    let out = merge(&shards, &tmp.path().join("reports")).unwrap();
    let files = output_files(&out).unwrap();
    let names: Vec<&str> = files.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(
        names,
        vec![
            "board.json",
            "board-summary.json",
            "reports.tar.gz",
            "cloudflare/kv-reports.json",
            "cloudflare/d1-board.sql"
        ]
    );

    let board: serde_json::Value = serde_json::from_slice(&files[0].1).unwrap();
    assert_eq!(
        keys(&board),
        set(&[
            "schema",
            "generated_at",
            "started_at",
            "run_id",
            "engine",
            "game_versions",
            "totals",
            "directions",
            "stats",
            "rows",
        ])
    );
    assert_eq!(board["schema"], 2);
    assert_eq!(keys(&board["engine"]), set(&["version", "git_sha"]));
    assert_eq!(
        keys(&board["totals"]),
        set(&["mods", "green", "yellow", "red"])
    );
    assert_eq!(board["directions"].as_array().unwrap().len(), 12);
    for d in board["directions"].as_array().unwrap() {
        assert_eq!(
            keys(d),
            set(&[
                "id",
                "from",
                "to",
                "order",
                "bar_percent",
                "column",
                "game_versions",
                "list_fetched_at",
                "rules_version",
                "totals",
                "top_reasons",
            ])
        );
    }
    let first = &board["directions"][0];
    assert_eq!(first["id"], "fabric-neoforge");
    assert_eq!(first["from"], "fabric");
    assert_eq!(first["to"], "neoforge");
    assert_eq!(first["order"], 1);
    assert_eq!(first["bar_percent"], 60);
    assert_eq!(first["column"], true);
    assert_eq!(
        keys(&first["top_reasons"][0]),
        set(&["kind", "label", "count"])
    );
    let second = &board["directions"][1];
    assert_eq!(second["id"], "neoforge-fabric");
    assert_eq!(second["column"], false);
    assert!(second["rules_version"].is_null() && second["list_fetched_at"].is_null());
    let row = &board["rows"][0];
    assert_eq!(
        keys(row),
        set(&[
            "direction",
            "project_id",
            "slug",
            "title",
            "downloads",
            "page_url",
            "icon_url",
            "license",
            "primary_game_version",
            "tier",
            "outcome",
            "reason_kind",
            "reason",
            "last_run",
            "report_url",
            "target_build_exists",
            "results",
        ])
    );
    assert!(row["tier"].is_null());
    assert_eq!(row["outcome"], "red");
    assert_eq!(row["reason_kind"], "no_converter");
    assert!(row["report_url"].is_null());
    assert_eq!(
        keys(&row["results"][0]),
        set(&[
            "direction",
            "game_version",
            "version_id",
            "version_number",
            "file_sha512",
            "mod_id",
            "mod_version",
            "tier",
            "tier_reasons",
            "outcome",
            "score",
            "reason_kind",
            "reason",
            "report_key",
            "report_hash",
            "duration_ms",
        ])
    );
    let result = &row["results"][0];
    assert_eq!(result["direction"], "fabric-neoforge");
    assert_eq!(result["outcome"], "not-run");
    assert!(result["score"].is_null());
    assert!(result["tier"].is_null());
    assert!(result["mod_id"].is_null());

    // board-summary.json carries exactly the keys tools/ledger.py accepts.
    let summary: serde_json::Value = serde_json::from_slice(&files[1].1).unwrap();
    assert_eq!(
        keys(&summary),
        set(&[
            "schema",
            "direction",
            "run",
            "date",
            "commit",
            "game_versions",
            "mods",
            "green",
            "yellow",
            "red",
            "not_run",
            "harness",
            "areas",
            "red_reasons",
        ])
    );
    assert_eq!(summary["schema"], 1);
    assert_eq!(summary["direction"], "fabric-neoforge");
    let parsed: LedgerSummary = serde_json::from_value(summary.clone()).unwrap();
    assert_eq!(parsed.date, "2026-11-04");
    assert_eq!(parsed.commit, SHA);
    assert_eq!(
        parsed.mods,
        parsed.green + parsed.yellow + parsed.red + parsed.not_run
    );
    assert_eq!(parsed.harness.attempted, 0);
    assert_eq!(
        parsed.red_reasons[0].reason,
        "No converter yet (milestone M2)"
    );
    assert!(parsed.red_reasons.iter().map(|r| r.count).sum::<u64>() <= parsed.red);

    // KV bulk file: sorted keys, each value a report.
    let kv: Vec<serde_json::Value> = serde_json::from_slice(&files[3].1).unwrap();
    assert_eq!(kv.len(), out.reports.len());
    let kv_keys: Vec<&str> = kv.iter().map(|p| p["key"].as_str().unwrap()).collect();
    let mut sorted = kv_keys.clone();
    sorted.sort_unstable();
    assert_eq!(kv_keys, sorted);
    for pair in &kv {
        let report: Report = serde_json::from_str(pair["value"].as_str().unwrap()).unwrap();
        assert_eq!(
            pair["key"],
            format!(
                "report:fabric-neoforge:{}:{}",
                report.project_id, report.game_version
            )
        );
    }

    // D1, in the schema of migration 0002: the run once, its earlier rows deleted first, one
    // row per mod, direction and game version, batched, with the D-024 strings.
    let sql = String::from_utf8(files[4].1.clone()).unwrap();
    assert!(sql.contains("DELETE FROM board_rows WHERE run_id = 'nightly-20261104';"));
    assert!(sql.contains("DELETE FROM board_runs WHERE id = 'nightly-20261104';"));
    assert!(sql.contains(
        "INSERT INTO board_runs (id, started_at, finished_at, engine_version, game_versions, \
         directions, totals) VALUES ('nightly-20261104', "
    ));
    assert!(sql.contains(r#"'["fabric-neoforge"]'"#));
    assert!(sql.contains(
        r#""fabric-neoforge":{"mods":25,"green":0,"yellow":0,"red":25,"rules_version":"none"}"#
    ));
    assert_eq!(
        sql.matches("INSERT INTO board_rows (run_id, direction, project_id")
            .count(),
        1
    );
    let rows: Vec<&str> = sql
        .lines()
        .filter(|l| l.starts_with("('nightly-20261104', 'fabric-neoforge', "))
        .collect();
    assert_eq!(rows.len(), out.reports.len());
    assert_eq!(
        rows.iter()
            .filter(|l| l.ends_with(", 1, 'red'),") || l.ends_with(", 1, 'red');"))
            .count(),
        fx.list.mods.len()
    );
    assert_eq!(
        rows.iter()
            .filter(|l| l.contains(", NULL, 'not-run', NULL, "))
            .count(),
        out.reports.len()
    );
    assert!(!sql.contains("'unknown'") && !sql.contains("'not_run'") && !sql.contains("'A'"));
}

#[test]
fn two_directions_from_one_source_list() {
    // Fabric -> NeoForge and Fabric -> Quilt share the Fabric list; each gets its own column.
    let fx = fixture();
    let tmp = tempfile::tempdir().unwrap();
    let reports = tmp.path().join("reports");
    let work = tmp.path().join("work");
    let quilt: Direction = "fabric-quilt".parse().unwrap();
    let mut shards = run_direction(&fx, quilt, &reports, &work);
    shards.extend(run_all(&fx, &reports, &work));
    let out = merge(&shards, &reports).unwrap();
    let board = &out.board;
    let measured: Vec<String> = board
        .directions
        .iter()
        .filter(|d| d.column)
        .map(|d| d.id.id())
        .collect();
    assert_eq!(measured, ["fabric-neoforge", "fabric-quilt"]);
    let n = fx.list.mods.len() as u64;
    assert_eq!(board.totals.mods, 2 * n, "one entry per mod per column");
    assert_eq!(board.rows.len() as u64, 2 * n);
    // Rows follow the build order: every fabric-neoforge row before every fabric-quilt row.
    let split = board
        .rows
        .iter()
        .position(|r| r.direction == quilt)
        .unwrap();
    assert!(
        board.rows[..split]
            .iter()
            .all(|r| r.direction == fabric_neoforge())
    );
    assert!(board.rows[split..].iter().all(|r| r.direction == quilt));
    let neo = board
        .rows
        .iter()
        .find(|r| r.direction == quilt && r.project_id == "Neo00002")
        .unwrap();
    assert!(!neo.target_build_exists, "tagged neoforge, not quilt");
    // The ledger summary is the first column's.
    assert_eq!(out.summary.direction, fabric_neoforge());
    assert_eq!(out.summary.mods, n);
    let sql = merge::d1_sql(board).unwrap();
    assert!(sql.contains(r#"'["fabric-neoforge","fabric-quilt"]'"#));
    assert!(sql.contains("('nightly-20261104', 'fabric-quilt', "));
}

#[test]
fn merge_is_deterministic_and_order_independent() {
    let fx = fixture();
    let tmp = tempfile::tempdir().unwrap();
    let reports = tmp.path().join("reports");
    let shards = run_all(&fx, &reports, &tmp.path().join("work"));
    let forward = output_files(&merge(&shards, &reports).unwrap()).unwrap();
    let mut reversed_shards = shards.clone();
    reversed_shards.reverse();
    let backward = output_files(&merge(&reversed_shards, &reports).unwrap()).unwrap();
    assert_eq!(forward, backward);
    // Writing to disk gives the same bytes.
    let a = tmp.path().join("out-a");
    let b = tmp.path().join("out-b");
    merge::write_outputs(&merge(&shards, &reports).unwrap(), &a).unwrap();
    merge::write_outputs(&merge(&reversed_shards, &reports).unwrap(), &b).unwrap();
    for (name, _) in &forward {
        assert_eq!(
            fs::read(a.join(name)).unwrap(),
            fs::read(b.join(name)).unwrap(),
            "{name}"
        );
    }
    // board.json round-trips through its own type.
    let board: Board = serde_json::from_slice(&forward[0].1).unwrap();
    assert_eq!(board.rows.len(), fx.list.mods.len());
}

#[test]
fn reports_do_not_depend_on_the_clock_or_the_shard() {
    let fx = fixture();
    let tmp = tempfile::tempdir().unwrap();
    let a = tmp.path().join("a");
    let b = tmp.path().join("b");
    let spec = ShardSpec::new(1, 1).unwrap();
    let first = run_shard(
        &fx.cdn,
        &WaveOne,
        &fx.list,
        fabric_neoforge(),
        spec,
        &options(&a, &tmp.path().join("w")),
        &FixedClock("2026-11-04T02:00:00Z".into()),
        |_| {},
    )
    .unwrap();
    let second = run_shard(
        &fx.cdn,
        &WaveOne,
        &fx.list,
        fabric_neoforge(),
        spec,
        &options(&b, &tmp.path().join("w")),
        &FixedClock("2026-12-25T23:59:59Z".into()),
        |_| {},
    )
    .unwrap();
    let hashes = |s: &ShardFile| -> Vec<String> {
        s.rows
            .iter()
            .flat_map(|r| r.results.iter().map(|x| x.report_hash.clone()))
            .collect()
    };
    assert_eq!(hashes(&first), hashes(&second));
    let rel = "fabric-neoforge/Fab00000/1.21.11.json";
    assert_eq!(
        fs::read(a.join(rel)).unwrap(),
        fs::read(b.join(rel)).unwrap()
    );
    assert_eq!(
        shard::file_name(first.direction, first.shard),
        "shard-fabric-neoforge-1.json"
    );
}

#[test]
fn merge_refuses_incomplete_or_inconsistent_input() {
    let fx = fixture();
    let tmp = tempfile::tempdir().unwrap();
    let reports = tmp.path().join("reports");
    let shards = run_all(&fx, &reports, &tmp.path().join("work"));

    let err = merge(&shards[1..], &reports).unwrap_err().to_string();
    assert!(err.contains("missing shards 1/10"), "{err}");

    let mut doubled = shards.clone();
    doubled.push(shards[0].clone());
    assert!(
        merge(&doubled, &reports)
            .unwrap_err()
            .to_string()
            .contains("twice")
    );

    let mut other_run = shards.clone();
    other_run[3].run_id = "nightly-20261105".into();
    assert!(
        merge(&other_run, &reports)
            .unwrap_err()
            .to_string()
            .contains("different runs")
    );

    let mut green = shards.clone();
    let row = green.iter_mut().find_map(|s| s.rows.first_mut()).unwrap();
    row.outcome = Outcome::Green;
    row.results.last_mut().unwrap().outcome = Outcome::Green;
    let err = merge(&green, &reports).unwrap_err().to_string();
    assert!(err.contains("green needs a harness score"), "{err}");

    let mut not_run = shards.clone();
    let row = not_run.iter_mut().find_map(|s| s.rows.first_mut()).unwrap();
    row.outcome = Outcome::NotRun;
    let err = merge(&not_run, &reports).unwrap_err().to_string();
    assert!(err.contains("not-run"), "{err}");

    let mut red_without_score = shards.clone();
    let row = red_without_score
        .iter_mut()
        .find_map(|s| s.rows.first_mut())
        .unwrap();
    row.results.last_mut().unwrap().outcome = Outcome::Red;
    let err = merge(&red_without_score, &reports).unwrap_err().to_string();
    assert!(err.contains("only a harness score"), "{err}");

    assert!(merge(&[], &reports).is_err());

    // A report that changed after the shard wrote it is caught by its hash.
    let victim = shards.iter().find_map(|s| s.rows.first()).unwrap();
    let path = reports.join(format!(
        "fabric-neoforge/{}/{}.json",
        victim.project_id, victim.primary_game_version
    ));
    fs::write(&path, b"{}").unwrap();
    assert!(
        merge(&shards, &reports)
            .unwrap_err()
            .to_string()
            .contains("does not match")
    );
}

#[test]
fn a_shard_measures_only_directions_from_the_lists_loader() {
    let fx = fixture();
    let tmp = tempfile::tempdir().unwrap();
    let err = run_shard(
        &fx.cdn,
        &WaveOne,
        &fx.list,
        "quilt-neoforge".parse().unwrap(),
        ShardSpec::new(1, 1).unwrap(),
        &options(&tmp.path().join("r"), &tmp.path().join("w")),
        &FixedClock("2026-11-04T02:00:00Z".into()),
        |_| {},
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("board-list-fabric.json"), "{err}");
}

#[test]
fn directions_are_inputs() {
    // The same JARs listed as Quilt mods and measured as Quilt to NeoForge: nothing in the
    // shard assumes Fabric.
    let mut fx = fixture();
    fx.list.loader = Loader::Quilt;
    let d: Direction = "quilt-neoforge".parse().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let reports = tmp.path().join("reports");
    let shards = run_direction(&fx, d, &reports, &tmp.path().join("work"));
    let out = merge(&shards, &reports).unwrap();
    let column = out.board.directions.iter().find(|x| x.column).unwrap();
    assert_eq!(column.id, d);
    assert_eq!(column.from, Loader::Quilt);
    assert_eq!(column.order, 6);
    assert!(
        out.reports
            .iter()
            .all(|(p, _)| p.starts_with("reports/quilt-neoforge/"))
    );
    assert!(out.board.rows.iter().all(|r| {
        r.results
            .iter()
            .all(|x| x.report_key.starts_with("report:quilt-neoforge:"))
    }));
    assert_eq!(out.summary.direction, d);
}
