//! Scoring a comparison into a harness report (decision D-024).
//!
//! | Score | Meaning | Outcome |
//! | --- | --- | --- |
//! | 1.0 | Identical once loader noise is subtracted | green |
//! | 0.75 | Only ordering or log-noise differences (registry order, new warnings) | green |
//! | 0.5 | Missing or extra entries under 1% of the mod's entries, or new error lines with no known cause | yellow |
//! | 0.25 | Loads, but entries differ materially: a changed entry, missing or extra entries at 1% or more, or a classified mixin, missing-class or registry error | red |
//! | 0 | Fails to load | red |
//!
//! A run that never started is `not-run` with no score, and so is a comparison whose
//! reference run failed: nothing was measured, so nothing is claimed.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fmt;

use jarrow_report::Outcome;
use serde::{Deserialize, Serialize};

use crate::diff::{Category, CompareError, DiffKind, Difference, diff_dumps};
use crate::dump::Dump;
use crate::loader::Loader;
use crate::logs::{Finding, ReasonKind, classify_messages, normalize_message, primary_reason};
use crate::noise::Noise;

/// The `record` value of a harness report.
pub const REPORT_RECORD: &str = "jarrow-harness-report";

/// The harness report format version.
pub const REPORT_SCHEMA_VERSION: u32 = 1;

/// Missing plus extra entries below this share of the mod's entries score 0.5; at or above
/// it they score 0.25.
pub const MINOR_ENTRY_RATIO: f64 = 0.01;

/// At most this many differences are listed in a report; the rest are counted.
pub const MAX_REPORTED_DIFFERENCES: usize = 1000;

/// The five score bands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Score {
    /// 0: the candidate did not load.
    FailedToLoad,
    /// 0.25: loads, entries differ materially.
    Material,
    /// 0.5: missing or extra entries under 1%.
    MinorEntries,
    /// 0.75: only ordering or log-noise differences.
    OrderOrNoise,
    /// 1.0: identical.
    Identical,
}

impl Score {
    /// The numeric score.
    pub const fn value(self) -> f64 {
        match self {
            Score::Identical => 1.0,
            Score::OrderOrNoise => 0.75,
            Score::MinorEntries => 0.5,
            Score::Material => 0.25,
            Score::FailedToLoad => 0.0,
        }
    }

    /// The board outcome for this score (D-024).
    pub const fn outcome(self) -> Outcome {
        match self {
            Score::Identical | Score::OrderOrNoise => Outcome::Green,
            Score::MinorEntries => Outcome::Yellow,
            Score::Material | Score::FailedToLoad => Outcome::Red,
        }
    }
}

impl fmt::Display for Score {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value())
    }
}

/// How one side of a comparison ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SideStatus {
    /// The server started and the probe wrote a valid dump.
    Completed,
    /// The server was started but no valid dump came out.
    Failed,
    /// The server was never started.
    NotRun,
}

/// One side of a comparison, as the runner left it.
#[derive(Debug, Clone, PartialEq)]
pub enum SideInput {
    /// A valid dump.
    Completed {
        /// The dump.
        dump: Box<Dump>,
        /// SHA-256 of the dump file, when read from one.
        dump_sha256: Option<String>,
    },
    /// The server started but produced no valid dump.
    Failed {
        /// Loader of the run.
        loader: Loader,
        /// Pinned loader version, when known.
        loader_version: Option<String>,
        /// Game version of the run.
        game_version: String,
        /// Classified reason.
        reason_kind: ReasonKind,
        /// What happened.
        reason: String,
        /// Log findings behind the reason.
        findings: Vec<Finding>,
    },
    /// The server never started.
    NotRun {
        /// Loader of the run.
        loader: Loader,
        /// Game version of the run.
        game_version: String,
        /// Why it did not start.
        reason_kind: ReasonKind,
        /// What happened.
        reason: String,
    },
}

impl SideInput {
    /// A side from a dump.
    pub fn from_dump(dump: Dump, dump_sha256: Option<String>) -> Self {
        SideInput::Completed {
            dump: Box::new(dump),
            dump_sha256,
        }
    }

    /// The loader of the run.
    pub fn loader(&self) -> Loader {
        match self {
            SideInput::Completed { dump, .. } => dump.loader,
            SideInput::Failed { loader, .. } | SideInput::NotRun { loader, .. } => *loader,
        }
    }

    /// The game version of the run.
    pub fn game_version(&self) -> &str {
        match self {
            SideInput::Completed { dump, .. } => &dump.game_version,
            SideInput::Failed { game_version, .. } | SideInput::NotRun { game_version, .. } => {
                game_version
            }
        }
    }

    fn report_side(&self) -> ReportSide {
        match self {
            SideInput::Completed { dump, dump_sha256 } => ReportSide {
                loader: dump.loader,
                loader_version: Some(dump.loader_version.clone()),
                status: SideStatus::Completed,
                reason_kind: None,
                dump_sha256: dump_sha256.clone(),
            },
            SideInput::Failed {
                loader,
                loader_version,
                reason_kind,
                ..
            } => ReportSide {
                loader: *loader,
                loader_version: loader_version.clone(),
                status: SideStatus::Failed,
                reason_kind: Some(*reason_kind),
                dump_sha256: None,
            },
            SideInput::NotRun {
                loader,
                reason_kind,
                ..
            } => ReportSide {
                loader: *loader,
                loader_version: None,
                status: SideStatus::NotRun,
                reason_kind: Some(*reason_kind),
                dump_sha256: None,
            },
        }
    }
}

/// One side as the report records it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportSide {
    /// Loader.
    pub loader: Loader,
    /// Loader version (from the dump, or the pin when the run failed).
    pub loader_version: Option<String>,
    /// How the run ended.
    pub status: SideStatus,
    /// Why it failed or did not run.
    pub reason_kind: Option<ReasonKind>,
    /// SHA-256 of the dump file.
    pub dump_sha256: Option<String>,
}

/// The baseline a report subtracted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaselineInfo {
    /// SHA-256 of the noise file, filled in by whoever read it.
    pub sha256: Option<String>,
    /// Reference loader version of the baseline.
    pub reference_loader_version: String,
    /// Candidate loader version of the baseline.
    pub candidate_loader_version: String,
    /// Content and order differences the baseline knows.
    pub known_differences: usize,
    /// Normalised log lines the baseline knows.
    pub known_log_lines: usize,
}

/// Difference counts per kind.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct KindCounts {
    /// Missing.
    pub missing: usize,
    /// Extra.
    pub extra: usize,
    /// Changed.
    pub changed: usize,
    /// Order.
    pub order: usize,
}

impl KindCounts {
    fn add(&mut self, kind: DiffKind) {
        match kind {
            DiffKind::Missing => self.missing += 1,
            DiffKind::Extra => self.extra += 1,
            DiffKind::Changed => self.changed += 1,
            DiffKind::Order => self.order += 1,
        }
    }
}

/// The numbers behind a score.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Counts {
    /// Entries in the reference dump.
    pub reference_units: usize,
    /// Entries in the candidate dump.
    pub candidate_units: usize,
    /// The mod's own entries: reference entries minus the reference baseline's. Without a
    /// baseline, every reference entry.
    pub mod_units: usize,
    /// Missing entries after noise.
    pub missing: usize,
    /// Extra entries after noise.
    pub extra: usize,
    /// Changed entries after noise.
    pub changed: usize,
    /// Registries whose shared entries are in a different order, after noise.
    pub order: usize,
    /// (missing + extra) / mod_units (a mod with no entries counts as one).
    pub entry_ratio: f64,
    /// New ERROR or FATAL log lines in the candidate, after noise.
    pub new_log_errors: usize,
    /// New WARN log lines in the candidate, after noise.
    pub new_log_warnings: usize,
    /// New probe errors in the candidate, after noise.
    pub new_probe_errors: usize,
    /// Differences recognised as loader noise and subtracted.
    pub noise_subtracted: usize,
}

/// The result of comparing a candidate run with its reference.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HarnessReport {
    /// Always [`REPORT_RECORD`].
    pub record: String,
    /// Always [`REPORT_SCHEMA_VERSION`].
    pub schema: u32,
    /// Game version of both runs.
    pub game_version: String,
    /// The original mod on its own loader.
    pub reference: ReportSide,
    /// The converted mod on the target loader.
    pub candidate: ReportSide,
    /// The baseline subtracted, if any.
    pub baseline: Option<BaselineInfo>,
    /// 1.0, 0.75, 0.5, 0.25 or 0; `None` when nothing was measured.
    pub score: Option<f64>,
    /// green, yellow, red or not-run.
    pub outcome: Outcome,
    /// Why the outcome is not green, or why nothing was measured.
    pub reason_kind: Option<ReasonKind>,
    /// One or two sentences.
    pub reason: String,
    /// The numbers behind the score; `None` unless both runs completed.
    pub counts: Option<Counts>,
    /// Remaining differences per category.
    pub categories: BTreeMap<Category, KindCounts>,
    /// Mods under test that loaded on the reference loader but not on the candidate.
    pub mods_not_loaded: Vec<String>,
    /// Remaining differences, sorted, at most [`MAX_REPORTED_DIFFERENCES`].
    pub differences: Vec<Difference>,
    /// Remaining differences not listed.
    pub differences_truncated: usize,
    /// Classified log findings on the candidate side.
    pub findings: Vec<Finding>,
}

impl HarnessReport {
    fn new(reference: &SideInput, candidate: &SideInput) -> Self {
        Self {
            record: REPORT_RECORD.to_owned(),
            schema: REPORT_SCHEMA_VERSION,
            game_version: reference.game_version().to_owned(),
            reference: reference.report_side(),
            candidate: candidate.report_side(),
            baseline: None,
            score: None,
            outcome: Outcome::NotRun,
            reason_kind: None,
            reason: String::new(),
            counts: None,
            categories: BTreeMap::new(),
            mods_not_loaded: Vec::new(),
            differences: Vec::new(),
            differences_truncated: 0,
            findings: Vec::new(),
        }
    }

    fn scored(&mut self, score: Score, reason_kind: Option<ReasonKind>, reason: String) {
        self.score = Some(score.value());
        self.outcome = score.outcome();
        self.reason_kind = reason_kind;
        self.reason = reason;
    }

    fn not_measured(&mut self, reason_kind: ReasonKind, reason: String) {
        self.score = None;
        self.outcome = Outcome::NotRun;
        self.reason_kind = Some(reason_kind);
        self.reason = reason;
    }

    /// The report as pretty JSON. It holds no timestamps, so the same inputs give the same
    /// bytes.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        Ok(serde_json::to_string_pretty(self)? + "\n")
    }

    /// One line for a terminal.
    pub fn summary(&self) -> String {
        match self.score {
            Some(score) => format!("score {score} ({}): {}", self.outcome.id(), self.reason),
            None => format!("{}: {}", self.outcome.id(), self.reason),
        }
    }
}

/// Compares a candidate run with its reference run, subtracting `noise` when given.
pub fn compare(
    reference: &SideInput,
    candidate: &SideInput,
    noise: Option<&Noise>,
) -> Result<HarnessReport, CompareError> {
    if reference.game_version() != candidate.game_version() {
        return Err(CompareError::Incomparable(format!(
            "game version {} against {}",
            reference.game_version(),
            candidate.game_version()
        )));
    }
    let mut report = HarnessReport::new(reference, candidate);
    match (reference, candidate) {
        (
            SideInput::NotRun {
                reason_kind,
                reason,
                loader,
                ..
            },
            _,
        ) => {
            report.not_measured(
                *reason_kind,
                format!("Not measured: the reference run on {loader} did not start ({reason})."),
            );
        }
        (
            _,
            SideInput::NotRun {
                reason_kind,
                reason,
                loader,
                ..
            },
        ) => {
            report.not_measured(
                *reason_kind,
                format!("Not measured: the candidate run on {loader} did not start ({reason})."),
            );
        }
        (
            SideInput::Failed {
                reason_kind,
                loader,
                ..
            },
            _,
        ) => {
            report.not_measured(
                ReasonKind::HarnessLoadFailure,
                format!(
                    "Not measured: the reference run on {loader} did not produce a dump \
                     ({reason_kind}), so there is nothing to compare the conversion with."
                ),
            );
        }
        (
            SideInput::Completed { .. },
            SideInput::Failed {
                reason_kind,
                reason,
                findings,
                loader,
                ..
            },
        ) => {
            report.findings = findings.clone();
            report.scored(
                Score::FailedToLoad,
                Some(*reason_kind),
                format!("The converted mod did not load on {loader}: {reason}"),
            );
        }
        (SideInput::Completed { dump: a, .. }, SideInput::Completed { dump: b, .. }) => {
            score_dumps(&mut report, a, b, noise)?;
        }
    }
    Ok(report)
}

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// Mod ids are matched after lower-casing and turning `-` into `_`, because NeoForge and
/// Forge ids cannot contain `-` and the manifest conversion renames them.
fn mod_key(id: &str) -> String {
    id.to_ascii_lowercase().replace('-', "_")
}

fn score_dumps(
    report: &mut HarnessReport,
    a: &Dump,
    b: &Dump,
    noise: Option<&Noise>,
) -> Result<(), CompareError> {
    if let Some(noise) = noise {
        noise.check_applies(a, b)?;
        report.baseline = Some(BaselineInfo {
            sha256: None,
            reference_loader_version: noise.reference.loader_version.clone(),
            candidate_loader_version: noise.candidate.loader_version.clone(),
            known_differences: noise.differences.len(),
            known_log_lines: noise.log.len(),
        });
    }
    let all = diff_dumps(a, b)?;
    let total = all.len();
    let remaining: Vec<Difference> = all
        .into_iter()
        .filter(|d| !noise.is_some_and(|n| n.is_noise(d)))
        .collect();
    let noise_subtracted = total - remaining.len();

    let mut entries = KindCounts::default();
    let (mut new_errors, mut new_warnings, mut new_probe_errors) = (0, 0, 0);
    let mut new_error_lines = BTreeSet::new();
    for d in &remaining {
        report.categories.entry(d.category).or_default().add(d.kind);
        if d.category.holds_entries() {
            entries.add(d.kind);
        }
        match (d.category, d.path.first().map(String::as_str)) {
            (Category::Log, Some("error")) => {
                new_errors += 1;
                if let Some(line) = d.path.last() {
                    new_error_lines.insert(line.clone());
                }
            }
            (Category::Log, Some("warning")) => new_warnings += 1,
            (Category::ProbeErrors, _) => new_probe_errors += 1,
            _ => {}
        }
    }

    let new_error_messages: Vec<&str> = b
        .log
        .errors
        .iter()
        .filter(|m| new_error_lines.contains(&normalize_message(m)))
        .map(String::as_str)
        .collect();
    report.findings = classify_messages(new_error_messages, "dump.log.errors");
    let log_reason = primary_reason(&report.findings);

    let mods_not_loaded = noise.map(|n| mods_not_loaded(a, b, n)).unwrap_or_default();
    let mod_units = match noise {
        Some(n) => a.units().saturating_sub(n.reference.units),
        None => a.units(),
    };
    let differing = entries.missing + entries.extra;
    let entry_ratio = differing as f64 / mod_units.max(1) as f64;
    report.counts = Some(Counts {
        reference_units: a.units(),
        candidate_units: b.units(),
        mod_units,
        missing: entries.missing,
        extra: entries.extra,
        changed: entries.changed,
        order: entries.order,
        entry_ratio,
        new_log_errors: new_errors,
        new_log_warnings: new_warnings,
        new_probe_errors,
        noise_subtracted,
    });
    report.differences_truncated = remaining.len().saturating_sub(MAX_REPORTED_DIFFERENCES);
    report.differences = remaining
        .into_iter()
        .take(MAX_REPORTED_DIFFERENCES)
        .collect();

    let subtracted = match noise {
        Some(n) => format!(
            " once the {} and {} baseline is subtracted",
            n.reference.loader, n.candidate.loader
        ),
        None => " (no baseline was subtracted)".to_owned(),
    };
    let entry_summary = format!(
        "{} missing, {} extra, {} changed ({:.2}% of the mod's {})",
        entries.missing,
        entries.extra,
        entries.changed,
        entry_ratio * 100.0,
        plural(mod_units, "entry", "entries")
    );
    let materially_logged = matches!(
        log_reason,
        Some(ReasonKind::MixinFailure | ReasonKind::ClassNotFound | ReasonKind::RegistryConflict)
    );

    if !mods_not_loaded.is_empty() {
        let kind = log_reason.unwrap_or(ReasonKind::HarnessLoadFailure);
        let reason = format!(
            "{} loaded on {} but not on {}. {}",
            mods_not_loaded.join(", "),
            a.loader,
            b.loader,
            kind.sentence()
        );
        report.mods_not_loaded = mods_not_loaded;
        report.scored(Score::FailedToLoad, Some(kind), reason);
    } else if entries.changed > 0 || (differing > 0 && entry_ratio >= MINOR_ENTRY_RATIO) {
        let kind = if materially_logged {
            log_reason.unwrap_or(ReasonKind::EntriesDiffer)
        } else {
            ReasonKind::EntriesDiffer
        };
        report.scored(
            Score::Material,
            Some(kind),
            format!("Entries differ materially{subtracted}: {entry_summary}."),
        );
    } else if let Some(kind) = log_reason.filter(|_| materially_logged) {
        let excerpt = report
            .findings
            .iter()
            .find(|f| f.kind == kind)
            .map(|f| f.excerpt.clone())
            .unwrap_or_default();
        report.scored(
            Score::Material,
            Some(kind),
            format!("{} The candidate logged: {excerpt}", kind.sentence()),
        );
    } else if differing > 0 || new_errors > 0 || new_probe_errors > 0 {
        let mut parts = Vec::new();
        if differing > 0 {
            parts.push(entry_summary);
        }
        if new_errors > 0 {
            parts.push(plural(new_errors, "new error line", "new error lines"));
        }
        if new_probe_errors > 0 {
            parts.push(plural(
                new_probe_errors,
                "new probe error",
                "new probe errors",
            ));
        }
        report.scored(
            Score::MinorEntries,
            Some(ReasonKind::EntriesDiffer),
            format!(
                "Small differences remain{subtracted}: {}.",
                parts.join("; ")
            ),
        );
    } else if entries.order > 0 || new_warnings > 0 {
        let mut parts = Vec::new();
        if entries.order > 0 {
            parts.push(format!(
                "{} in a different order",
                plural(entries.order, "registry", "registries")
            ));
        }
        if new_warnings > 0 {
            parts.push(plural(new_warnings, "new warning", "new warnings"));
        }
        report.scored(
            Score::OrderOrNoise,
            None,
            format!(
                "Verified by the harness: only ordering or log-noise differences remain{subtracted} ({}).",
                parts.join(", ")
            ),
        );
    } else {
        report.scored(
            Score::Identical,
            None,
            format!("Verified by the harness: the dumps match{subtracted}."),
        );
    }
    Ok(())
}

/// Mods under test (reference mods the baseline does not have) with no counterpart among the
/// candidate's mods under test.
fn mods_not_loaded(a: &Dump, b: &Dump, noise: &Noise) -> Vec<String> {
    let base_a: BTreeSet<&str> = noise.reference.mods.iter().map(String::as_str).collect();
    let base_b: BTreeSet<&str> = noise.candidate.mods.iter().map(String::as_str).collect();
    let loaded_b: BTreeSet<String> = b
        .mods
        .iter()
        .filter(|m| !base_b.contains(m.id.as_str()))
        .map(|m| mod_key(&m.id))
        .collect();
    a.mods
        .iter()
        .filter(|m| !base_a.contains(m.id.as_str()))
        .filter(|m| !loaded_b.contains(&mod_key(&m.id)))
        .map(|m| m.id.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dump::tests::sample;
    use serde_json::{Value, json};

    fn dump(value: &Value) -> Dump {
        Dump::from_value(value).unwrap()
    }

    fn side(value: &Value) -> SideInput {
        SideInput::from_dump(dump(value), None)
    }

    /// A baseline pair, and a mod that adds `n` items plus a recipe on both loaders.
    struct World {
        noise: Noise,
        reference: Value,
        candidate: Value,
    }

    fn world(n: usize) -> World {
        let fabric = sample(Loader::Fabric);
        let mut neoforge = sample(Loader::NeoForge);
        neoforge["registries"]["neoforge:fluid_type"] = json!(["neoforge:empty"]);
        neoforge["tags"]["minecraft:item"]["c:rods/wooden"] =
            json!(["minecraft:bamboo", "minecraft:stick"]);
        neoforge["log"]["warnings"] = json!(["NeoForge config loaded in 31 ms"]);
        let noise = Noise::learn(&dump(&fabric), &dump(&neoforge)).unwrap();
        let add_mod = |mut v: Value, id: &str| {
            let items = v["registries"]["minecraft:item"].as_array_mut().unwrap();
            for i in 0..n {
                items.push(json!(format!("examplemod:widget_{i}")));
            }
            v["recipes"]["examplemod:widget_0"] = json!({"type": "minecraft:crafting_shapeless", "ingredients": ["minecraft:stick"], "result": {"id": "examplemod:widget_0", "count": 1}});
            let mods = v["mods"].as_array_mut().unwrap();
            mods.push(json!({"id": id, "version": "1.0.0"}));
            v
        };
        World {
            noise,
            reference: add_mod(fabric, "example-mod"),
            candidate: add_mod(neoforge, "example_mod"),
        }
    }

    fn score(w: &World) -> HarnessReport {
        compare(&side(&w.reference), &side(&w.candidate), Some(&w.noise)).unwrap()
    }

    #[test]
    fn identical_after_noise_is_one_and_green() {
        let w = world(99);
        let report = score(&w);
        assert_eq!(report.score, Some(1.0), "{}", report.reason);
        assert_eq!(report.outcome, Outcome::Green);
        assert_eq!(report.reason_kind, None);
        let counts = report.counts.as_ref().unwrap();
        assert!(counts.noise_subtracted >= 4, "{counts:?}");
        assert_eq!(counts.mod_units, 100, "99 items and one recipe");
        // Only the renamed mod id is listed, for information; it is not an entry.
        assert!(
            report
                .differences
                .iter()
                .all(|d| d.category == Category::Mods),
            "{:?}",
            report.differences
        );
        assert!(report.reason.starts_with("Verified by the harness"));
        assert!(
            report.mods_not_loaded.is_empty(),
            "example-mod matches example_mod"
        );
    }

    #[test]
    fn same_loader_without_baseline_is_identical() {
        let a = sample(Loader::Fabric);
        let report = compare(&side(&a), &side(&a), None).unwrap();
        assert_eq!(report.score, Some(1.0));
        assert!(report.reason.contains("no baseline"));
        assert_eq!(report.counts.unwrap().mod_units, dump(&a).units());
    }

    #[test]
    fn order_or_new_warnings_are_point_seven_five_and_green() {
        let mut w = world(10);
        let items = w.candidate["registries"]["minecraft:item"]
            .as_array_mut()
            .unwrap();
        items.swap(3, 4);
        let report = score(&w);
        assert_eq!(report.score, Some(0.75), "{}", report.reason);
        assert_eq!(report.outcome, Outcome::Green);
        assert!(
            report.reason.contains("1 registry in a different order"),
            "{}",
            report.reason
        );

        let mut w = world(10);
        w.candidate["log"]["warnings"] = json!([
            "NeoForge config loaded in 5 ms",
            "examplemod: deprecated call"
        ]);
        let report = score(&w);
        assert_eq!(report.score, Some(0.75), "{}", report.reason);
        assert_eq!(
            report.counts.unwrap().new_log_warnings,
            1,
            "the baseline warning is noise"
        );
    }

    #[test]
    fn a_few_missing_entries_are_point_five_and_yellow() {
        let mut w = world(199);
        w.candidate["registries"]["minecraft:item"]
            .as_array_mut()
            .unwrap()
            .pop();
        let report = score(&w);
        assert_eq!(report.score, Some(0.5), "{}", report.reason);
        assert_eq!(report.outcome, Outcome::Yellow);
        assert_eq!(report.reason_kind, Some(ReasonKind::EntriesDiffer));
        let counts = report.counts.unwrap();
        assert_eq!((counts.missing, counts.mod_units), (1, 200));
        assert!((counts.entry_ratio - 0.005).abs() < 1e-12);
        assert!(
            report.reason.contains("0.50% of the mod's 200 entries"),
            "{}",
            report.reason
        );
    }

    #[test]
    fn unexplained_new_errors_are_point_five() {
        let mut w = world(10);
        w.candidate["log"]["errors"] = json!(["examplemod: could not read config, using defaults"]);
        let report = score(&w);
        assert_eq!(report.score, Some(0.5), "{}", report.reason);
        assert!(report.reason.contains("1 new error line"));
    }

    #[test]
    fn one_percent_or_changed_content_is_point_two_five_and_red() {
        let mut w = world(99);
        w.candidate["registries"]["minecraft:item"]
            .as_array_mut()
            .unwrap()
            .pop();
        let report = score(&w);
        assert_eq!(
            report.score,
            Some(0.25),
            "1 of 100 is 1%, not under it: {}",
            report.reason
        );
        assert_eq!(report.outcome, Outcome::Red);
        assert_eq!(report.reason_kind, Some(ReasonKind::EntriesDiffer));

        let mut w = world(500);
        w.candidate["recipes"]["examplemod:widget_0"]["result"]["count"] = json!(2);
        let report = score(&w);
        assert_eq!(report.score, Some(0.25), "{}", report.reason);
        let changed = report
            .differences
            .iter()
            .find(|d| d.category == Category::Recipes)
            .unwrap();
        assert_eq!(changed.kind, DiffKind::Changed);
        assert_eq!(
            changed.detail.as_deref(),
            Some("/result/count: 1 in the reference, 2 in the candidate")
        );
    }

    #[test]
    fn a_classified_mixin_error_is_red_even_when_entries_match() {
        let mut w = world(10);
        w.candidate["log"]["errors"] = json!([
            "mixin: Mixin apply failed examplemod.mixins.json:LevelMixin -> net.minecraft.server.level.ServerLevel"
        ]);
        let report = score(&w);
        assert_eq!(report.score, Some(0.25), "{}", report.reason);
        assert_eq!(report.reason_kind, Some(ReasonKind::MixinFailure));
        assert_eq!(report.findings.len(), 1);
    }

    #[test]
    fn a_mod_that_did_not_load_scores_zero() {
        let mut w = world(10);
        let mods = w.candidate["mods"].as_array_mut().unwrap();
        mods.retain(|m| m["id"] != "example_mod");
        let report = score(&w);
        assert_eq!(report.score, Some(0.0), "{}", report.reason);
        assert_eq!(report.outcome, Outcome::Red);
        assert_eq!(report.reason_kind, Some(ReasonKind::HarnessLoadFailure));
        assert_eq!(report.mods_not_loaded, ["example-mod"]);
    }

    #[test]
    fn a_failed_candidate_scores_zero_with_its_classified_reason() {
        let w = world(10);
        let failed = SideInput::Failed {
            loader: Loader::NeoForge,
            loader_version: Some("21.1.265".into()),
            game_version: "1.21.1".into(),
            reason_kind: ReasonKind::ClassNotFound,
            reason: "the server stopped before writing jarrow-dump.json.".into(),
            findings: Vec::new(),
        };
        let report = compare(&side(&w.reference), &failed, Some(&w.noise)).unwrap();
        assert_eq!(report.score, Some(0.0));
        assert_eq!(report.reason_kind, Some(ReasonKind::ClassNotFound));
        assert_eq!(report.candidate.status, SideStatus::Failed);
        assert!(report.counts.is_none());
    }

    #[test]
    fn nothing_measured_is_not_run_without_a_score() {
        let w = world(10);
        let eula = SideInput::NotRun {
            loader: Loader::NeoForge,
            game_version: "1.21.1".into(),
            reason_kind: ReasonKind::HarnessNotRunEula,
            reason: "the EULA gate is closed".into(),
        };
        let report = compare(&side(&w.reference), &eula, Some(&w.noise)).unwrap();
        assert_eq!(report.score, None);
        assert_eq!(report.outcome, Outcome::NotRun);
        assert_eq!(report.reason_kind, Some(ReasonKind::HarnessNotRunEula));

        let broken_reference = SideInput::Failed {
            loader: Loader::Fabric,
            loader_version: None,
            game_version: "1.21.1".into(),
            reason_kind: ReasonKind::HarnessLoadFailure,
            reason: "missing dependency".into(),
            findings: Vec::new(),
        };
        let report = compare(&broken_reference, &side(&w.candidate), Some(&w.noise)).unwrap();
        assert_eq!(
            report.outcome,
            Outcome::NotRun,
            "a broken reference measures nothing"
        );
        assert_eq!(report.score, None);
        assert!(!report.outcome.has_harness_result());
    }

    #[test]
    fn mismatched_inputs_are_errors_not_scores() {
        let w = world(10);
        let mut other = w.candidate.clone();
        other["game_version"] = json!("1.21.11");
        assert!(compare(&side(&w.reference), &side(&other), Some(&w.noise)).is_err());
        let mut stale = w.candidate.clone();
        stale["loader_version"] = json!("21.1.1");
        assert!(matches!(
            compare(&side(&w.reference), &side(&stale), Some(&w.noise)),
            Err(CompareError::BaselineMismatch(_))
        ));
    }

    #[test]
    fn reports_are_deterministic_and_capped() {
        let mut w = world(1500);
        w.candidate["registries"]["minecraft:item"] = json!(["minecraft:air"]);
        let a = score(&w);
        let b = score(&w);
        assert_eq!(a.to_json().unwrap(), b.to_json().unwrap());
        assert_eq!(a.differences.len(), MAX_REPORTED_DIFFERENCES);
        // 1502 missing items plus the two halves of the renamed mod id.
        assert_eq!(a.differences_truncated, 1504 - MAX_REPORTED_DIFFERENCES);
        assert!(
            a.summary().starts_with("score 0.25 (red): "),
            "{}",
            a.summary()
        );
        assert!(!a.to_json().unwrap().to_lowercase().contains("guarantee"));
    }

    #[test]
    fn bands_map_to_d024_outcomes() {
        assert_eq!(Score::Identical.outcome(), Outcome::Green);
        assert_eq!(Score::OrderOrNoise.outcome(), Outcome::Green);
        assert_eq!(Score::MinorEntries.outcome(), Outcome::Yellow);
        assert_eq!(Score::Material.outcome(), Outcome::Red);
        assert_eq!(Score::FailedToLoad.outcome(), Outcome::Red);
        let values: Vec<f64> = [
            Score::Identical,
            Score::OrderOrNoise,
            Score::MinorEntries,
            Score::Material,
            Score::FailedToLoad,
        ]
        .iter()
        .map(|s| s.value())
        .collect();
        assert_eq!(values, [1.0, 0.75, 0.5, 0.25, 0.0]);
    }
}
