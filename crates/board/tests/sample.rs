//! The sample `board.json` and `board-summary.json` that every reader's tests read: the site
//! (`site/src/lib/board.test.ts`), `tools/board_summary.py` and `tools/ledger.py`. This test
//! fails when the files differ from what `jarrow-board merge` writes, so the writer and the
//! readers cannot drift apart. Regenerate them with:
//!
//! ```text
//! JARROW_BLESS=1 cargo test -p jarrow-board --test sample
//! ```
//!
//! The sample is one nightly of wave 1 for the one column that exists, Fabric to NeoForge,
//! over four made-up projects: every result `not-run`, every row red. Durations are set to
//! zero, because they are the only values that vary between runs, and the engine version is
//! the fixed `0.0.0-sample`, so a workspace version bump does not change the sample.

#![allow(
    clippy::unwrap_used,
    reason = "test helpers: a failed unwrap is a failed test"
)]

mod common;

use std::fs;
use std::path::PathBuf;

use common::{FakeFetch, jar};
use jarrow_board::board::{BoardStats, StepDurations};
use jarrow_board::hash::sha512_hex;
use jarrow_board::list::{BoardList, ListFile, ListMod};
use jarrow_board::merge::{merge, output_files};
use jarrow_board::pipeline::WaveOne;
use jarrow_board::shard::{ShardOptions, ShardSpec, run_shard};
use jarrow_board::time::FixedClock;
use jarrow_board::{Direction, Loader};

fn file(project: &str, gv: &str, bytes: &[u8]) -> ListFile {
    let version_id = format!("V{}{}", &project[..4], gv.replace('.', ""));
    ListFile {
        game_version: gv.to_owned(),
        version_id: version_id.clone(),
        version_number: format!("2.0.0+{gv}"),
        version_type: "release".to_owned(),
        filename: format!("{project}-{gv}.jar"),
        url: format!(
            "https://cdn.modrinth.com/data/{project}/versions/{version_id}/{project}-{gv}.jar"
        ),
        sha512: sha512_hex(bytes),
        size: bytes.len() as u64,
    }
}

fn project(
    id: &str,
    slug: &str,
    downloads: u64,
    loaders: &[&str],
    files: Vec<ListFile>,
) -> ListMod {
    ListMod {
        project_id: id.to_owned(),
        slug: slug.to_owned(),
        title: format!("Sample {slug}"),
        downloads,
        license: Some("MIT".to_owned()),
        client_side: Some("optional".to_owned()),
        server_side: Some("optional".to_owned()),
        icon_url: None,
        loaders: loaders.iter().map(|l| (*l).to_owned()).collect(),
        files,
    }
}

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

#[test]
fn board_samples_are_current() {
    let fabric = jar(&["fabric.mod.json", "com/example/Sample.class"]);
    let both = jar(&["fabric.mod.json", "META-INF/neoforge.mods.toml"]);
    let mut cdn = FakeFetch::default();
    let three = vec![
        file("SmpAAAA1", "1.20.1", &fabric),
        file("SmpAAAA1", "1.21.1", &fabric),
        file("SmpAAAA1", "1.21.11", &fabric),
    ];
    for f in &three {
        cdn = cdn.with(&f.url, fabric.clone());
    }
    let multi = file("SmpBBBB2", "1.21.1", &both);
    cdn = cdn.with(&multi.url, both.clone());
    let old = file("SmpCCCC3", "1.20.1", &fabric);
    cdn = cdn.with(&old.url, fabric.clone());
    // Listed but never served: the download fails, the result is not-run with download_failed.
    let gone = file("SmpDDDD4", "1.21.11", &fabric);
    let list = BoardList {
        schema: 2,
        loader: Loader::Fabric,
        fetched_at: "2026-11-03T14:00:00Z".to_owned(),
        game_versions: vec!["1.20.1".into(), "1.21.1".into(), "1.21.11".into()],
        top: 4,
        mods: vec![
            project(
                "SmpAAAA1",
                "sample-three",
                4000,
                &["fabric", "quilt"],
                three,
            ),
            project(
                "SmpBBBB2",
                "sample-both",
                3000,
                &["fabric", "neoforge"],
                vec![multi],
            ),
            project("SmpCCCC3", "sample-old", 2000, &["fabric"], vec![old]),
            project("SmpDDDD4", "sample-gone", 1000, &["fabric"], vec![gone]),
        ],
        skipped: vec![],
    }
    .normalized()
    .unwrap();

    let tmp = tempfile::tempdir().unwrap();
    let reports = tmp.path().join("reports");
    let direction: Direction = "fabric-neoforge".parse().unwrap();
    let shards: Vec<_> = (1..=2)
        .map(|i| {
            run_shard(
                &cdn,
                &WaveOne,
                &list,
                direction,
                ShardSpec::new(i, 2).unwrap(),
                &ShardOptions {
                    run_id: "nightly-20261104".to_owned(),
                    git_sha: "0123456789abcdef0123456789abcdef01234567".to_owned(),
                    engine_version: "0.0.0-sample".to_owned(),
                    rules_version: "none".to_owned(),
                    reports_dir: reports.clone(),
                    work_dir: Some(tmp.path().join("work")),
                    max_file_bytes: 1024 * 1024,
                },
                &FixedClock(format!("2026-11-04T02:2{i}:00Z")),
                |_| {},
            )
            .unwrap()
        })
        .collect();
    let mut out = merge(&shards, &reports).unwrap();
    for row in &mut out.board.rows {
        for result in &mut row.results {
            result.duration_ms = 0;
        }
    }
    out.board.stats = BoardStats {
        max_shard_wall_ms: 0,
        total_shard_wall_ms: 0,
        step_ms: StepDurations::default(),
        ..out.board.stats
    };
    let files = output_files(&out).unwrap();
    let bless = std::env::var_os("JARROW_BLESS").is_some();
    for (name, sample) in [
        ("board.json", "board-sample.json"),
        ("board-summary.json", "board-summary-sample.json"),
    ] {
        let bytes = &files.iter().find(|(n, _)| n == name).unwrap().1;
        let path = fixtures().join(sample);
        if bless {
            fs::write(&path, bytes).unwrap();
            continue;
        }
        let current = fs::read(&path).unwrap_or_default();
        let normalise = |b: &[u8]| String::from_utf8_lossy(b).replace("\r\n", "\n");
        assert!(
            normalise(&current) == normalise(bytes),
            "{} is not what jarrow-board merge writes; run \
             JARROW_BLESS=1 cargo test -p jarrow-board --test sample",
            path.display()
        );
    }
}

/// The in-memory JARs are byte-identical on every OS, so the sample's hashes are too.
#[test]
fn sample_jars_are_the_same_everywhere() {
    let bytes = jar(&["fabric.mod.json", "META-INF/neoforge.mods.toml"]);
    let archive = zip::ZipArchive::new(std::io::Cursor::new(bytes.clone())).unwrap();
    assert_eq!(archive.len(), 2);
    assert_eq!(
        jarrow_board::hash::sha256_hex(&bytes),
        "ba196f69b6d2032a0e66a8024eac198635baec2377efcf3d348631f88a0ea356"
    );
}
