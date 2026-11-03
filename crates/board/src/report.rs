//! The per-mod report the board writes for each mod and game version, and the board's
//! closed reason vocabulary.
//!
//! `crates/report` owns the report model ([`jarrow_report::ReportEntry`], `Tier`,
//! `Outcome`), and the report entries, tiers and outcomes here are its types (D-024). It has
//! no whole-report struct and no `reason_kind` vocabulary yet, so both live here for now
//! ([`Report`], [`ReasonKind`]), with the strings `docs/board.md` fixes. They move to
//! `crates/report` when it grows them.
//!
//! A result's outcome follows `crates/report`: only a harness score makes it green, yellow
//! or red, and a file the harness did not measure is `not-run`. The board row built from it
//! counts `not-run` as red (or yellow for `client_only_not_measured`), never green
//! ([`board_outcome`]).
//!
//! A report is deterministic: it holds no clock time and no duration, so the same input file,
//! engine version and pipeline give byte-identical JSON. Its SHA-256 is the `report_hash` on
//! the board.

use std::fmt;

use jarrow_report::{Confidence, EntryKind, Location, Outcome, ReportEntry, Tier};
use serde::{Deserialize, Serialize};

use crate::hash;
use crate::loader::Direction;
use crate::{Error, SCHEMA};

/// Why a mod is red or yellow on the board: the closed vocabulary of `docs/board.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasonKind {
    /// No converter exists yet (milestone M2).
    NoConverter,
    /// The mod ships native libraries.
    UnsupportedNative,
    /// The mod transforms bytecode outside Mixin.
    UnsupportedTransformer,
    /// The mod brings its own language adapter.
    UnsupportedLanguageAdapter,
    /// The JAR is obfuscated.
    UnsupportedObfuscated,
    /// The license allows personal use only.
    LicensePersonalUse,
    /// The mod needs the Tier B pipeline.
    TierBRequired,
    /// The harness did not run because the EULA gate is closed.
    HarnessNotRunEula,
    /// The converted mod failed to load.
    HarnessLoadFailure,
    /// A class was not found at runtime.
    ClassNotFound,
    /// A mixin failed to apply.
    MixinFailure,
    /// Two registrations collided.
    RegistryConflict,
    /// The server crashed.
    Crash,
    /// The harness dumps differ.
    EntriesDiffer,
    /// The mod loads but its content is client-only, so the harness cannot measure it.
    ClientOnlyNotMeasured,
    /// The file could not be downloaded or did not match its sha512.
    DownloadFailed,
    /// The author asked not to be ported.
    OptOut,
}

impl ReasonKind {
    /// Every kind, in declaration order.
    pub const ALL: [ReasonKind; 17] = [
        ReasonKind::NoConverter,
        ReasonKind::UnsupportedNative,
        ReasonKind::UnsupportedTransformer,
        ReasonKind::UnsupportedLanguageAdapter,
        ReasonKind::UnsupportedObfuscated,
        ReasonKind::LicensePersonalUse,
        ReasonKind::TierBRequired,
        ReasonKind::HarnessNotRunEula,
        ReasonKind::HarnessLoadFailure,
        ReasonKind::ClassNotFound,
        ReasonKind::MixinFailure,
        ReasonKind::RegistryConflict,
        ReasonKind::Crash,
        ReasonKind::EntriesDiffer,
        ReasonKind::ClientOnlyNotMeasured,
        ReasonKind::DownloadFailed,
        ReasonKind::OptOut,
    ];

    /// Stable machine id, identical to the serialised form.
    pub const fn id(self) -> &'static str {
        match self {
            ReasonKind::NoConverter => "no_converter",
            ReasonKind::UnsupportedNative => "unsupported_native",
            ReasonKind::UnsupportedTransformer => "unsupported_transformer",
            ReasonKind::UnsupportedLanguageAdapter => "unsupported_language_adapter",
            ReasonKind::UnsupportedObfuscated => "unsupported_obfuscated",
            ReasonKind::LicensePersonalUse => "license_personal_use",
            ReasonKind::TierBRequired => "tier_b_required",
            ReasonKind::HarnessNotRunEula => "harness_not_run_eula",
            ReasonKind::HarnessLoadFailure => "harness_load_failure",
            ReasonKind::ClassNotFound => "class_not_found",
            ReasonKind::MixinFailure => "mixin_failure",
            ReasonKind::RegistryConflict => "registry_conflict",
            ReasonKind::Crash => "crash",
            ReasonKind::EntriesDiffer => "entries_differ",
            ReasonKind::ClientOnlyNotMeasured => "client_only_not_measured",
            ReasonKind::DownloadFailed => "download_failed",
            ReasonKind::OptOut => "opt_out",
        }
    }

    /// The one-line label the board, the ledger and the Discussion show for this kind.
    pub const fn label(self) -> &'static str {
        match self {
            ReasonKind::NoConverter => "No converter yet (milestone M2)",
            ReasonKind::UnsupportedNative => "Ships native libraries",
            ReasonKind::UnsupportedTransformer => "Transforms bytecode outside Mixin",
            ReasonKind::UnsupportedLanguageAdapter => "Uses its own language adapter",
            ReasonKind::UnsupportedObfuscated => "Obfuscated JAR",
            ReasonKind::LicensePersonalUse => "License allows personal use only",
            ReasonKind::TierBRequired => "Needs a Tier B conversion",
            ReasonKind::HarnessNotRunEula => "Harness not run: Minecraft EULA not accepted",
            ReasonKind::HarnessLoadFailure => "Converted mod failed to load",
            ReasonKind::ClassNotFound => "Class not found",
            ReasonKind::MixinFailure => "Mixin failure",
            ReasonKind::RegistryConflict => "Registry conflict",
            ReasonKind::Crash => "Server crash",
            ReasonKind::EntriesDiffer => "Harness dumps differ",
            ReasonKind::ClientOnlyNotMeasured => "Client-only content, not measured",
            ReasonKind::DownloadFailed => "Download failed or did not match its sha512",
            ReasonKind::OptOut => "Author opted out",
        }
    }
}

impl fmt::Display for ReasonKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id())
    }
}

/// The outcome a harness score maps to (decision D-024): 1.0 and 0.75 are green, 0.5 is
/// yellow, 0.25 and 0 are red. Any other value is not a harness score.
pub fn outcome_for_score(score: f64) -> Option<Outcome> {
    if score == 1.0 || score == 0.75 {
        Some(Outcome::Green)
    } else if score == 0.5 {
        Some(Outcome::Yellow)
    } else if score == 0.25 || score == 0.0 {
        Some(Outcome::Red)
    } else {
        None
    }
}

/// A row's outcome on the board, from its primary result: green, yellow and red stay as they
/// are; a result the harness did not measure (`not-run`) is red, or yellow when the reason is
/// `client_only_not_measured` (D-024). Never `not-run`, never green without a score.
pub fn board_outcome(outcome: Outcome, reason_kind: Option<ReasonKind>) -> Outcome {
    match outcome {
        Outcome::NotRun if reason_kind == Some(ReasonKind::ClientOnlyNotMeasured) => {
            Outcome::Yellow
        }
        Outcome::NotRun => Outcome::Red,
        other => other,
    }
}

/// Checks a result's outcome, score and reason against each other: a score decides the
/// outcome, no score means `not-run`, and anything but green carries a reason. `what` names
/// the result in the error.
pub fn check_verdict(
    outcome: Outcome,
    score: Option<f64>,
    reason_kind: Option<ReasonKind>,
    reason: Option<&str>,
    what: &str,
) -> Result<(), Error> {
    let bad = |msg: &str| Err(Error::invalid(format!("{what}: {msg}")));
    match score {
        Some(s) => match outcome_for_score(s) {
            None => return bad("the score is not a harness score (0, 0.25, 0.5, 0.75 or 1.0)"),
            Some(o) if o != outcome => {
                return bad("the outcome is not the one its harness score maps to (D-024)");
            }
            Some(_) => {}
        },
        None if outcome == Outcome::Green => {
            return bad("green needs a harness score of 1.0 or 0.75");
        }
        None if outcome != Outcome::NotRun => {
            return bad(
                "only a harness score makes a result yellow or red; without one it is not-run",
            );
        }
        None => {}
    }
    if outcome != Outcome::Green && (reason_kind.is_none() || reason.is_none_or(str::is_empty)) {
        return bad("a result that is not green needs a reason_kind and a reason");
    }
    Ok(())
}

/// How a pipeline step ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StepStatus {
    /// The step ran and succeeded.
    Ok,
    /// The step ran and failed; the step says why.
    Failed,
    /// The step does not exist yet; the step names the milestone.
    NotBuilt,
    /// The step exists but did not run; the step says why.
    NotRun,
}

/// The download step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DownloadStep {
    /// Ok, or failed.
    pub status: StepStatus,
    /// Whether the downloaded bytes matched the list's sha512 and size.
    pub sha512_verified: bool,
    /// Why it failed, when it did.
    pub error: Option<String>,
}

/// Which manifest files the JAR carries (wave 1 reads only the zip central directory). Which
/// loader reads which file, on which game version, is a loader-profile fact
/// (`loaders/<id>.toml`), so this records only the files.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifests {
    /// `fabric.mod.json`.
    pub fabric_mod_json: bool,
    /// `quilt.mod.json`.
    pub quilt_mod_json: bool,
    /// `META-INF/mods.toml`.
    pub mods_toml: bool,
    /// `META-INF/neoforge.mods.toml`.
    pub neoforge_mods_toml: bool,
}

/// The inspect step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InspectStep {
    /// Ok, failed or not run (when the download failed).
    pub status: StepStatus,
    /// The JAR's size in bytes.
    pub size: Option<u64>,
    /// Entries in the zip central directory.
    pub entries: Option<u64>,
    /// Which loader manifests are present.
    pub manifests: Option<Manifests>,
    /// Why it failed or did not run.
    pub error: Option<String>,
}

/// The classify step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClassifyStep {
    /// Not built in wave 1.
    pub status: StepStatus,
    /// The tier, or `null` when the classifier did not run (not built yet, or no file).
    pub tier: Option<Tier>,
    /// Ordered reasons; when the tier is `null`, why.
    pub reasons: Vec<String>,
}

/// The convert step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConvertStep {
    /// Not built until milestone M2.
    pub status: StepStatus,
    /// The classified reason a file without a conversion carries.
    pub reason_kind: ReasonKind,
    /// The reason in words.
    pub reason: String,
}

/// The harness step.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HarnessStep {
    /// Not run while nothing is converted.
    pub status: StepStatus,
    /// The harness score (`docs/harness.md`), when it ran.
    pub score: Option<f64>,
    /// The classified reason when the score is not green.
    pub reason_kind: Option<ReasonKind>,
    /// Why it did not run, or what it found.
    pub reason: String,
}

/// The round-trip checks (classfile and manifest parse and re-write the input unchanged).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoundTripStep {
    /// Not run yet in wave 1.
    pub status: StepStatus,
    /// Why it did not run, or what it found.
    pub reason: String,
}

/// Every step of the per-mod pipeline, in order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Steps {
    /// Download and sha512 check.
    pub download: DownloadStep,
    /// Zip central directory.
    pub inspect: InspectStep,
    /// Tier classifier.
    pub classify: ClassifyStep,
    /// Converter.
    pub convert: ConvertStep,
    /// Parity harness.
    pub harness: HarnessStep,
    /// Classfile and manifest round trips.
    pub round_trip: RoundTripStep,
}

/// The file a report is about.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReportFile {
    /// Modrinth version id.
    pub version_id: String,
    /// The author's version string.
    pub version_number: String,
    /// File name.
    pub filename: String,
    /// CDN URL.
    pub url: String,
    /// SHA-512 from the list.
    pub sha512: String,
    /// Size from the list.
    pub size: u64,
}

/// One report: one mod, one game version, one direction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    /// Always [`SCHEMA`].
    pub schema: u32,
    /// The engine version that wrote it.
    pub engine_version: String,
    /// The direction.
    pub direction: Direction,
    /// Modrinth project id.
    pub project_id: String,
    /// URL slug.
    pub slug: String,
    /// Display name.
    pub title: String,
    /// The game version.
    pub game_version: String,
    /// The file.
    pub file: ReportFile,
    /// What each step did.
    pub steps: Steps,
    /// The tier (from the classify step), `null` when the classifier did not run.
    pub tier: Option<Tier>,
    /// The result: green, yellow or red from a harness score, otherwise `not-run`.
    pub outcome: Outcome,
    /// The harness score, when the harness ran.
    pub score: Option<f64>,
    /// The classified reason for a yellow or red outcome.
    pub reason_kind: Option<ReasonKind>,
    /// The reason in words.
    pub reason: Option<String>,
    /// Structured entries in the `crates/report` model.
    pub entries: Vec<ReportEntry>,
}

impl Report {
    /// The canonical JSON: compact, fields in declaration order. These bytes are what the
    /// board stores and what [`Report::hash`] covers.
    pub fn canonical_json(&self) -> Result<Vec<u8>, Error> {
        serde_json::to_vec(self).map_err(|e| Error::json("report", e))
    }

    /// `sha256:<hex>` of the canonical JSON.
    pub fn hash(&self) -> Result<String, Error> {
        Ok(format!(
            "sha256:{}",
            hash::sha256_hex(&self.canonical_json()?)
        ))
    }

    /// Checks the honesty rules: only a harness score makes a result green, yellow or red
    /// (green needs 1.0 or 0.75), a result without one is `not-run`, anything but green needs a
    /// reason, and every entry carries the fields its kind needs.
    pub fn validate(&self) -> Result<(), Error> {
        let what = format!(
            "report {}/{}/{}",
            self.direction, self.project_id, self.game_version
        );
        if self.schema != SCHEMA {
            return Err(Error::invalid(format!("{what}: unknown schema")));
        }
        if self.tier != self.steps.classify.tier {
            return Err(Error::invalid(format!(
                "{what}: the tier is not the classify step's"
            )));
        }
        check_verdict(
            self.outcome,
            self.score,
            self.reason_kind,
            self.reason.as_deref(),
            &what,
        )?;
        for entry in &self.entries {
            entry
                .validate()
                .map_err(|e| Error::invalid(format!("report entry: {e}")))?;
        }
        Ok(())
    }
}

/// The entry every wave-1 report carries: the JAR as a whole was not translated, and why.
pub fn not_converted_entry(reason: &str) -> ReportEntry {
    ReportEntry::new(EntryKind::NotTranslated, Location::jar(), Confidence::Exact)
        .with_note(format!("{reason}: nothing in this JAR was converted"))
}

/// The KV key of a report: `report:<direction>:<project_id>:<game_version>`.
pub fn report_key(direction: Direction, project_id: &str, game_version: &str) -> String {
    format!("report:{direction}:{project_id}:{game_version}")
}

/// The path of a report inside `--reports-dir` and inside `reports.tar.gz`:
/// `<direction>/<project_id>/<game_version>.json`.
pub fn report_path(direction: Direction, project_id: &str, game_version: &str) -> String {
    format!("{direction}/{project_id}/{game_version}.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reason_ids_match_serde_and_the_contract() {
        let contract = [
            "no_converter",
            "unsupported_native",
            "unsupported_transformer",
            "unsupported_language_adapter",
            "unsupported_obfuscated",
            "license_personal_use",
            "tier_b_required",
            "harness_not_run_eula",
            "harness_load_failure",
            "class_not_found",
            "mixin_failure",
            "registry_conflict",
            "crash",
            "entries_differ",
            "client_only_not_measured",
            "download_failed",
            "opt_out",
        ];
        let ids: Vec<&str> = ReasonKind::ALL.iter().map(|k| k.id()).collect();
        assert_eq!(ids, contract);
        for kind in ReasonKind::ALL {
            let json = serde_json::to_string(&kind).unwrap();
            assert_eq!(json, format!("\"{}\"", kind.id()));
            assert_eq!(serde_json::from_str::<ReasonKind>(&json).unwrap(), kind);
            assert!(!kind.label().is_empty());
            assert!(!kind.label().to_lowercase().contains("guarantee"));
        }
    }

    #[test]
    fn tiers_and_outcomes_serialise_as_d024_says() {
        let tiers: Vec<String> = Tier::ALL
            .iter()
            .map(|t| serde_json::to_string(&Some(*t)).unwrap())
            .collect();
        assert_eq!(tiers, ["\"a\"", "\"b\"", "\"unsupported\""]);
        assert_eq!(serde_json::to_string(&None::<Tier>).unwrap(), "null");
        let outcomes: Vec<&str> = Outcome::ALL.iter().map(|o| o.id()).collect();
        assert_eq!(outcomes, ["green", "yellow", "red", "not-run"]);
    }

    #[test]
    fn scores_map_as_d024_says() {
        assert_eq!(outcome_for_score(1.0), Some(Outcome::Green));
        assert_eq!(outcome_for_score(0.75), Some(Outcome::Green));
        assert_eq!(outcome_for_score(0.5), Some(Outcome::Yellow));
        assert_eq!(outcome_for_score(0.25), Some(Outcome::Red));
        assert_eq!(outcome_for_score(0.0), Some(Outcome::Red));
        assert_eq!(outcome_for_score(0.3), None);
    }

    #[test]
    fn not_run_is_red_on_the_board_unless_client_only() {
        let no = Some(ReasonKind::NoConverter);
        assert_eq!(board_outcome(Outcome::NotRun, no), Outcome::Red);
        assert_eq!(
            board_outcome(Outcome::NotRun, Some(ReasonKind::ClientOnlyNotMeasured)),
            Outcome::Yellow
        );
        assert_eq!(board_outcome(Outcome::NotRun, None), Outcome::Red);
        for o in [Outcome::Green, Outcome::Yellow, Outcome::Red] {
            assert_eq!(board_outcome(o, no), o);
        }
    }

    #[test]
    fn verdicts_need_a_score_for_a_colour() {
        let reason = Some("No converter yet (milestone M2)");
        let kind = Some(ReasonKind::NoConverter);
        check_verdict(Outcome::NotRun, None, kind, reason, "x").unwrap();
        check_verdict(Outcome::Green, Some(0.75), None, None, "x").unwrap();
        check_verdict(Outcome::Red, Some(0.25), kind, reason, "x").unwrap();
        for (outcome, score) in [
            (Outcome::Green, None),
            (Outcome::Green, Some(0.5)),
            (Outcome::Red, None),
            (Outcome::Yellow, None),
            (Outcome::NotRun, Some(0.0)),
            (Outcome::Red, Some(0.3)),
        ] {
            assert!(check_verdict(outcome, score, kind, reason, "x").is_err());
        }
        assert!(check_verdict(Outcome::NotRun, None, None, reason, "x").is_err());
        assert!(check_verdict(Outcome::NotRun, None, kind, Some(""), "x").is_err());
    }

    #[test]
    fn keys_and_paths() {
        let d: Direction = "fabric-neoforge".parse().unwrap();
        assert_eq!(
            report_key(d, "AANobbMI", "1.21.11"),
            "report:fabric-neoforge:AANobbMI:1.21.11"
        );
        assert_eq!(
            report_path(d, "AANobbMI", "1.21.11"),
            "fabric-neoforge/AANobbMI/1.21.11.json"
        );
    }

    #[test]
    fn the_not_converted_entry_is_valid() {
        let entry = not_converted_entry(ReasonKind::NoConverter.label());
        entry.validate().unwrap();
        assert_eq!(entry.kind, EntryKind::NotTranslated);
    }
}
