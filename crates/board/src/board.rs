//! The rows and totals of `board.json` (`docs/board.md`). Shards write rows; the merge adds
//! totals, reasons and statistics.

use std::collections::BTreeMap;

use jarrow_report::Outcome;
use serde::{Deserialize, Serialize};

use crate::loader::{Direction, Loader};
use crate::report::{BoardTier, ReasonKind};

/// The engine that produced a shard or a board.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Engine {
    /// `jarrow-board` version.
    pub version: String,
    /// The commit of jarrow the run built, 40 hex characters (or `unknown` for a local run
    /// outside git).
    pub git_sha: String,
}

/// One game version's result for one mod.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoardResult {
    /// The game version.
    pub game_version: String,
    /// Modrinth version id.
    pub version_id: String,
    /// The author's version string on Modrinth.
    pub version_number: String,
    /// SHA-512 of the tested file.
    pub file_sha512: String,
    /// The mod id from the JAR's manifest; `null` until the manifest crate reads it (M1
    /// wave 2).
    pub mod_id: Option<String>,
    /// The mod's version; Modrinth's version number until the manifest crate reads the JAR's
    /// own (M1 wave 2).
    pub mod_version: Option<String>,
    /// The tier, `unknown` while the classifier is not built.
    pub tier: BoardTier,
    /// Why that tier.
    pub tier_reasons: Vec<String>,
    /// Green, yellow or red.
    pub outcome: Outcome,
    /// The harness score, when the harness ran.
    pub score: Option<f64>,
    /// The classified reason for yellow or red.
    pub reason_kind: Option<ReasonKind>,
    /// The reason in words.
    pub reason: Option<String>,
    /// The KV key of the full report.
    pub report_key: String,
    /// `sha256:<hex>` of the canonical report JSON.
    pub report_hash: String,
    /// Wall time for this file: download and every step, in milliseconds.
    pub duration_ms: u64,
}

/// One mod on the board, for one direction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoardRow {
    /// The direction this row measures.
    pub direction: Direction,
    /// Modrinth project id.
    pub project_id: String,
    /// URL slug.
    pub slug: String,
    /// Display name.
    pub title: String,
    /// All-time downloads when the list was fetched.
    pub downloads: u64,
    /// The mod's own Modrinth page. The board never links to a converted JAR.
    pub page_url: String,
    /// Icon URL.
    pub icon_url: Option<String>,
    /// SPDX id or `LicenseRef-...`, as Modrinth lists it.
    pub license: Option<String>,
    /// The newest pinned game version the mod ships a file for. The row's tier, outcome and
    /// reason are this version's.
    pub primary_game_version: String,
    /// Tier on the primary version.
    pub tier: BoardTier,
    /// Outcome on the primary version.
    pub outcome: Outcome,
    /// Reason kind on the primary version.
    pub reason_kind: Option<ReasonKind>,
    /// Reason on the primary version.
    pub reason: Option<String>,
    /// When this row's last file finished, RFC 3339 UTC.
    pub last_run: String,
    /// Whether Modrinth tags the project `neoforge` (informational).
    pub neoforge_build_exists: bool,
    /// Whether Modrinth tags the project with the direction's target loader (informational).
    pub target_build_exists: bool,
    /// Every pinned game version the mod ships a file for, oldest first.
    pub results: Vec<BoardResult>,
}

/// Mods per outcome. `mods == green + yellow + red` always holds.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Totals {
    /// Rows counted.
    pub mods: u64,
    /// Green rows.
    pub green: u64,
    /// Yellow rows.
    pub yellow: u64,
    /// Red rows.
    pub red: u64,
}

impl Totals {
    /// Counts the rows' outcomes. A `not-run` row counts as red: "not run" is not an outcome
    /// on the board.
    pub fn of<'a>(rows: impl IntoIterator<Item = &'a BoardRow>) -> Self {
        let mut t = Totals::default();
        for row in rows {
            t.mods += 1;
            match row.outcome {
                Outcome::Green => t.green += 1,
                Outcome::Yellow => t.yellow += 1,
                Outcome::Red | Outcome::NotRun => t.red += 1,
            }
        }
        t
    }

    /// The sum of two totals.
    pub fn plus(self, other: Totals) -> Totals {
        Totals {
            mods: self.mods + other.mods,
            green: self.green + other.green,
            yellow: self.yellow + other.yellow,
            red: self.red + other.red,
        }
    }
}

/// How many yellow or red rows carry one reason kind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReasonCount {
    /// The reason kind.
    pub kind: ReasonKind,
    /// Its label.
    pub label: String,
    /// Rows.
    pub count: u64,
}

/// Counts the reason kinds of the non-green rows, most frequent first, then by kind id.
pub fn top_reasons<'a>(rows: impl IntoIterator<Item = &'a BoardRow>) -> Vec<ReasonCount> {
    let mut counts: BTreeMap<ReasonKind, u64> = BTreeMap::new();
    for row in rows {
        if row.outcome == Outcome::Green {
            continue;
        }
        if let Some(kind) = row.reason_kind {
            *counts.entry(kind).or_insert(0) += 1;
        }
    }
    let mut out: Vec<ReasonCount> = counts
        .into_iter()
        .map(|(kind, count)| ReasonCount {
            kind,
            label: kind.label().to_owned(),
            count,
        })
        .collect();
    out.sort_by(|a, b| {
        b.count
            .cmp(&a.count)
            .then_with(|| a.kind.id().cmp(b.kind.id()))
    });
    out
}

/// Milliseconds spent per step, summed over files.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StepDurations {
    /// Downloads and sha512 checks.
    pub download: u64,
    /// Zip central directory reads.
    pub inspect: u64,
    /// Classifier.
    pub classify: u64,
    /// Converter.
    pub convert: u64,
    /// Harness.
    pub harness: u64,
    /// Round trips.
    pub round_trip: u64,
}

impl StepDurations {
    /// Field-wise sum.
    pub fn plus(self, o: StepDurations) -> StepDurations {
        StepDurations {
            download: self.download + o.download,
            inspect: self.inspect + o.inspect,
            classify: self.classify + o.classify,
            convert: self.convert + o.convert,
            harness: self.harness + o.harness,
            round_trip: self.round_trip + o.round_trip,
        }
    }
}

/// Counts and durations of one shard.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShardStats {
    /// Mods processed.
    pub mods: u64,
    /// Files processed.
    pub files: u64,
    /// Bytes downloaded and verified.
    pub downloaded_bytes: u64,
    /// Files whose download failed or did not match its sha512.
    pub download_failures: u64,
    /// Files whose zip central directory could not be read.
    pub inspect_failures: u64,
    /// Wall time of the shard, in milliseconds.
    pub wall_ms: u64,
    /// Time per step, summed over files.
    pub step_ms: StepDurations,
}

/// One direction's column of the board.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirectionBoard {
    /// The direction.
    pub direction: Direction,
    /// Its source loader.
    pub source_loader: Loader,
    /// Its target loader.
    pub target_loader: Loader,
    /// The pinned game versions it measured.
    pub game_versions: Vec<String>,
    /// When its mod list was fetched from Modrinth.
    pub list_fetched_at: String,
    /// The content hash of its rule tables, or `none` while it has none.
    pub rules_version: String,
    /// Mods per outcome.
    pub totals: Totals,
    /// Reasons of its yellow and red rows.
    pub top_reasons: Vec<ReasonCount>,
}

/// Run-wide statistics (counts and durations), summed over shards.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoardStats {
    /// Shards merged.
    pub shards: u64,
    /// Files processed.
    pub files: u64,
    /// Bytes downloaded and verified.
    pub downloaded_bytes: u64,
    /// Files whose download failed or did not match its sha512.
    pub download_failures: u64,
    /// Files whose zip central directory could not be read.
    pub inspect_failures: u64,
    /// The slowest shard's wall time, in milliseconds.
    pub max_shard_wall_ms: u64,
    /// Shard wall times summed, in milliseconds.
    pub total_shard_wall_ms: u64,
    /// Time per step, summed over files.
    pub step_ms: StepDurations,
}

/// `board.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Board {
    /// Always 1.
    pub schema: u32,
    /// When the last shard finished, RFC 3339 UTC.
    pub generated_at: String,
    /// When the first shard started, RFC 3339 UTC.
    pub started_at: String,
    /// The run, for example `nightly-20261104`.
    pub run_id: String,
    /// The engine.
    pub engine: Engine,
    /// The rules version when every direction shares one, otherwise `per-direction`.
    pub rules_version: String,
    /// Every pinned game version of every direction, oldest first.
    pub game_versions: Vec<String>,
    /// All directions together.
    pub totals: Totals,
    /// All directions together.
    pub top_reasons: Vec<ReasonCount>,
    /// One entry per direction, sorted by direction id.
    pub directions: Vec<DirectionBoard>,
    /// Counts and durations.
    pub stats: BoardStats,
    /// Rows, sorted by direction, downloads (most first) and project id.
    pub rows: Vec<BoardRow>,
}

/// The Modrinth page of a project: by slug when the slug is plain, else by id.
pub fn page_url(slug: &str, project_id: &str) -> String {
    let plain = !slug.is_empty()
        && slug
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'));
    let key = if plain { slug } else { project_id };
    format!("https://modrinth.com/mod/{key}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(outcome: Outcome, kind: Option<ReasonKind>) -> BoardRow {
        BoardRow {
            direction: "fabric-neoforge".parse().unwrap(),
            project_id: "AAAAAAAA".into(),
            slug: "a".into(),
            title: "A".into(),
            downloads: 1,
            page_url: page_url("a", "AAAAAAAA"),
            icon_url: None,
            license: None,
            primary_game_version: "1.21.11".into(),
            tier: BoardTier::Unknown,
            outcome,
            reason_kind: kind,
            reason: kind.map(|k| k.label().to_owned()),
            last_run: "2026-11-04T02:00:00Z".into(),
            neoforge_build_exists: false,
            target_build_exists: false,
            results: vec![],
        }
    }

    #[test]
    fn totals_always_add_up() {
        let rows = vec![
            row(Outcome::Red, Some(ReasonKind::NoConverter)),
            row(Outcome::Red, Some(ReasonKind::DownloadFailed)),
            row(Outcome::NotRun, Some(ReasonKind::NoConverter)),
            row(Outcome::Green, None),
            row(Outcome::Yellow, Some(ReasonKind::ClientOnlyNotMeasured)),
        ];
        let t = Totals::of(&rows);
        assert_eq!(
            t,
            Totals {
                mods: 5,
                green: 1,
                yellow: 1,
                red: 3
            }
        );
        assert_eq!(t.mods, t.green + t.yellow + t.red);
        assert_eq!(t.plus(t).mods, 10);
    }

    #[test]
    fn top_reasons_are_sorted_and_skip_green() {
        let rows = vec![
            row(Outcome::Red, Some(ReasonKind::NoConverter)),
            row(Outcome::Red, Some(ReasonKind::DownloadFailed)),
            row(Outcome::Red, Some(ReasonKind::NoConverter)),
            row(Outcome::Red, Some(ReasonKind::Crash)),
            row(Outcome::Green, Some(ReasonKind::Crash)),
        ];
        let reasons = top_reasons(&rows);
        let kinds: Vec<(&str, u64)> = reasons.iter().map(|r| (r.kind.id(), r.count)).collect();
        assert_eq!(
            kinds,
            vec![("no_converter", 2), ("crash", 1), ("download_failed", 1)]
        );
        assert_eq!(reasons[0].label, "No converter yet (milestone M2)");
    }

    #[test]
    fn page_urls() {
        assert_eq!(
            page_url("sodium", "AANobbMI"),
            "https://modrinth.com/mod/sodium"
        );
        assert_eq!(
            page_url("we ird", "AANobbMI"),
            "https://modrinth.com/mod/AANobbMI"
        );
        assert_eq!(
            page_url("", "AANobbMI"),
            "https://modrinth.com/mod/AANobbMI"
        );
    }
}
