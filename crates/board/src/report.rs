//! The per-mod report the board writes for each mod and game version, and the board's
//! closed reason vocabulary.
//!
//! `crates/report` owns the report model ([`jarrow_report::ReportEntry`], `Tier`,
//! `Outcome`), and the report entries here are its type. It has no whole-report struct and
//! no `reason_kind` vocabulary yet, so both live here for now ([`Report`], [`ReasonKind`]),
//! with the strings `docs/board.md` fixes. They move to `crates/report` when it grows them.
//!
//! A report is deterministic: it holds no clock time and no duration, so the same input file,
//! engine version and pipeline give byte-identical JSON. Its SHA-256 is the `report_hash` on
//! the board.

use std::fmt;

use jarrow_report::{Confidence, EntryKind, Location, Outcome, ReportEntry, Tier};
use serde::{Deserialize, Serialize};

use crate::hash;
use crate::loader::{Direction, Loader};
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

/// A tier as the board records it: one of `crates/report`'s tiers, or `unknown` while the
/// classifier is not built (milestone M1, wave 2).
///
/// Serialised as `"a"`, `"b"`, `"unsupported"` or `"unknown"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BoardTier {
    /// Tier A.
    A,
    /// Tier B.
    B,
    /// Unsupported.
    Unsupported,
    /// Not classified: the classifier has not run on this file.
    Unknown,
}

impl BoardTier {
    /// Stable machine id, identical to the serialised form.
    pub const fn id(self) -> &'static str {
        match self {
            BoardTier::A => "a",
            BoardTier::B => "b",
            BoardTier::Unsupported => "unsupported",
            BoardTier::Unknown => "unknown",
        }
    }

    /// The value the D1 `board_rows.tier` column accepts (`infra/d1/migrations/0001_init.sql`
    /// spells Tier A and B in capitals).
    pub const fn d1_value(self) -> &'static str {
        match self {
            BoardTier::A => "A",
            BoardTier::B => "B",
            BoardTier::Unsupported => "unsupported",
            BoardTier::Unknown => "unknown",
        }
    }
}

impl From<Tier> for BoardTier {
    fn from(tier: Tier) -> Self {
        match tier {
            Tier::A => BoardTier::A,
            Tier::B => BoardTier::B,
            Tier::Unsupported => BoardTier::Unsupported,
        }
    }
}

/// The value the D1 `board_rows.outcome` column accepts (`not_run` with an underscore).
pub const fn outcome_d1_value(outcome: Outcome) -> &'static str {
    match outcome {
        Outcome::Green => "green",
        Outcome::Yellow => "yellow",
        Outcome::Red => "red",
        Outcome::NotRun => "not_run",
    }
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

/// Which loader manifests the JAR carries (wave 1 reads only the zip central directory).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifests {
    /// `fabric.mod.json`.
    pub fabric_mod_json: bool,
    /// `quilt.mod.json`.
    pub quilt_mod_json: bool,
    /// `META-INF/mods.toml` (Forge, and NeoForge before 20.5).
    pub mods_toml: bool,
    /// `META-INF/neoforge.mods.toml`.
    pub neoforge_mods_toml: bool,
}

impl Manifests {
    /// Whether the JAR carries the manifest of `loader`.
    pub fn has(self, loader: Loader) -> bool {
        match loader {
            Loader::Fabric => self.fabric_mod_json,
            Loader::Quilt => self.quilt_mod_json,
            Loader::Forge => self.mods_toml,
            Loader::NeoForge => self.neoforge_mods_toml,
        }
    }
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
    /// The tier, `unknown` until the classifier exists.
    pub tier: BoardTier,
    /// Ordered reasons.
    pub reasons: Vec<String>,
}

/// The convert step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConvertStep {
    /// Not built until milestone M2.
    pub status: StepStatus,
    /// The outcome this step forces (red while no converter exists).
    pub outcome: Outcome,
    /// The classified reason.
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
    /// The tier (from the classify step).
    pub tier: BoardTier,
    /// The result: never green without a harness score.
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

    /// Checks the honesty rules: green needs a harness score of 1.0 or 0.75, a red or yellow
    /// outcome needs a reason, and every entry carries the fields its kind needs.
    pub fn validate(&self) -> Result<(), Error> {
        let bad = |msg: &str| {
            Err(Error::invalid(format!(
                "report {}/{}/{}: {msg}",
                self.direction, self.project_id, self.game_version
            )))
        };
        if self.schema != SCHEMA {
            return bad("unknown schema");
        }
        match self.outcome {
            Outcome::Green => {
                if !matches!(self.score, Some(s) if s == 1.0 || s == 0.75) {
                    return bad("green needs a harness score of 1.0 or 0.75");
                }
            }
            Outcome::Yellow | Outcome::Red => {
                if self.reason_kind.is_none() || self.reason.as_deref().is_none_or(str::is_empty) {
                    return bad("a red or yellow outcome needs a reason");
                }
            }
            Outcome::NotRun => {
                return bad("the board never records not-run; it is red with a reason");
            }
        }
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
    fn tiers_serialise_like_crates_report_plus_unknown() {
        for tier in Tier::ALL {
            let board: BoardTier = tier.into();
            assert_eq!(
                serde_json::to_string(&board).unwrap(),
                serde_json::to_string(&tier).unwrap()
            );
            assert_eq!(board.id(), tier.id());
        }
        assert_eq!(
            serde_json::to_string(&BoardTier::Unknown).unwrap(),
            "\"unknown\""
        );
        assert_eq!(BoardTier::A.d1_value(), "A");
        assert_eq!(BoardTier::Unknown.d1_value(), "unknown");
        assert_eq!(outcome_d1_value(Outcome::NotRun), "not_run");
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
