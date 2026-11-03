//! The per-mod pipeline: inspect, classify, convert, harness, round trips.
//!
//! [`ModPipeline`] is the seam where later milestones plug in the real crates:
//! `jarrow-classify` (M1 wave 2), `jarrow-engine` (M2), `jarrow-harness` (M1), and the
//! classfile and manifest round trips (M1 wave 2). [`WaveOne`] is what exists today: it reads
//! the zip central directory and records every other step as not built or not run, so every
//! mod is red with the reason "No converter yet (milestone M2)".

use std::fs::File;
use std::path::Path;

use jarrow_report::Outcome;

use crate::list::{ListFile, ListMod};
use crate::loader::Direction;
use crate::report::{
    BoardTier, ClassifyStep, ConvertStep, HarnessStep, InspectStep, Manifests, ReasonKind,
    RoundTripStep, StepStatus, Steps,
};

/// The classify reason while the classifier does not exist.
pub const CLASSIFIER_NOT_BUILT: &str = "classifier not built yet (M1 wave 2)";

/// The harness reason when nothing was converted.
pub const HARNESS_NOTHING_CONVERTED: &str = "not run: nothing was converted";

/// The round-trip reason in wave 1.
pub const ROUND_TRIP_NOT_RUN: &str =
    "not run yet: the classfile and manifest round trips are wired in M1 wave 2";

/// What a pipeline step knows about the mod it works on.
#[derive(Debug, Clone, Copy)]
pub struct ModContext<'a> {
    /// The direction being measured.
    pub direction: Direction,
    /// The project from the list.
    pub project: &'a ListMod,
    /// The file for this game version.
    pub file: &'a ListFile,
}

/// The steps of the per-mod pipeline. Each step gets what the earlier steps found and never
/// panics: a failure is a step result.
pub trait ModPipeline {
    /// Reads the JAR's structure.
    fn inspect(&self, ctx: &ModContext<'_>, jar: &Path) -> InspectStep;
    /// Decides the tier.
    fn classify(&self, ctx: &ModContext<'_>, jar: &Path, inspect: &InspectStep) -> ClassifyStep;
    /// Converts the JAR to the direction's target loader.
    fn convert(&self, ctx: &ModContext<'_>, jar: &Path, classify: &ClassifyStep) -> ConvertStep;
    /// Runs the parity harness on the conversion.
    fn harness(&self, ctx: &ModContext<'_>, convert: &ConvertStep) -> HarnessStep;
    /// Parses and re-writes the input's classes and manifest and checks the bytes match.
    fn round_trip(&self, ctx: &ModContext<'_>, jar: &Path) -> RoundTripStep;
}

/// Reads the zip central directory of `jar`: size, entry count, and which loader manifests
/// are present. Nothing is decompressed.
pub fn inspect_jar(jar: &Path) -> InspectStep {
    let failed = |error: String| InspectStep {
        status: StepStatus::Failed,
        size: None,
        entries: None,
        manifests: None,
        error: Some(error),
    };
    let file = match File::open(jar) {
        Ok(file) => file,
        Err(e) => return failed(format!("could not open the JAR: {e}")),
    };
    let size = match file.metadata() {
        Ok(meta) => meta.len(),
        Err(e) => return failed(format!("could not read the JAR's size: {e}")),
    };
    let archive = match zip::ZipArchive::new(file) {
        Ok(archive) => archive,
        Err(e) => {
            let mut step = failed(format!("not a readable zip archive: {e}"));
            step.size = Some(size);
            return step;
        }
    };
    let mut manifests = Manifests::default();
    for name in archive.file_names_raw() {
        match name {
            b"fabric.mod.json" => manifests.fabric_mod_json = true,
            b"quilt.mod.json" => manifests.quilt_mod_json = true,
            b"META-INF/mods.toml" => manifests.mods_toml = true,
            b"META-INF/neoforge.mods.toml" => manifests.neoforge_mods_toml = true,
            _ => {}
        }
    }
    InspectStep {
        status: StepStatus::Ok,
        size: Some(size),
        entries: Some(archive.len() as u64),
        manifests: Some(manifests),
        error: None,
    }
}

/// The outcome a harness score maps to (decision D-024): 1.0 and 0.75 are green, 0.5 is
/// yellow, anything else is red.
pub fn outcome_for_score(score: f64) -> Outcome {
    if score == 1.0 || score == 0.75 {
        Outcome::Green
    } else if score == 0.5 {
        Outcome::Yellow
    } else {
        Outcome::Red
    }
}

/// The board's verdict for one file, derived from the steps.
#[derive(Debug, Clone, PartialEq)]
pub struct Verdict {
    /// Green, yellow or red; never not-run.
    pub outcome: Outcome,
    /// The harness score, when it ran.
    pub score: Option<f64>,
    /// The classified reason for yellow or red.
    pub reason_kind: Option<ReasonKind>,
    /// The reason in words.
    pub reason: Option<String>,
}

/// Decides the outcome: the harness score when the harness ran and scored, otherwise the
/// convert step's red outcome and reason. A file without a harness score is never green.
pub fn decide(steps: &Steps) -> Verdict {
    if steps.harness.status == StepStatus::Ok
        && let Some(score) = steps.harness.score
    {
        let outcome = outcome_for_score(score);
        let (reason_kind, reason) = if outcome == Outcome::Green {
            (None, None)
        } else {
            (
                Some(
                    steps
                        .harness
                        .reason_kind
                        .unwrap_or(ReasonKind::EntriesDiffer),
                ),
                Some(steps.harness.reason.clone()),
            )
        };
        return Verdict {
            outcome,
            score: Some(score),
            reason_kind,
            reason,
        };
    }
    // Without a harness score nothing is claimed: whatever the convert step says, only red
    // reaches the board.
    let kind = steps.convert.reason_kind;
    Verdict {
        outcome: Outcome::Red,
        score: None,
        reason_kind: Some(kind),
        reason: Some(steps.convert.reason.clone()),
    }
}

/// The pipeline as it exists in milestone M1, wave 1.
#[derive(Debug, Clone, Copy, Default)]
pub struct WaveOne;

impl ModPipeline for WaveOne {
    fn inspect(&self, _ctx: &ModContext<'_>, jar: &Path) -> InspectStep {
        inspect_jar(jar)
    }

    fn classify(&self, _ctx: &ModContext<'_>, _jar: &Path, _inspect: &InspectStep) -> ClassifyStep {
        ClassifyStep {
            status: StepStatus::NotBuilt,
            tier: BoardTier::Unknown,
            reasons: vec![CLASSIFIER_NOT_BUILT.to_owned()],
        }
    }

    fn convert(&self, _ctx: &ModContext<'_>, _jar: &Path, _classify: &ClassifyStep) -> ConvertStep {
        ConvertStep {
            status: StepStatus::NotBuilt,
            outcome: Outcome::Red,
            reason_kind: ReasonKind::NoConverter,
            reason: ReasonKind::NoConverter.label().to_owned(),
        }
    }

    fn harness(&self, _ctx: &ModContext<'_>, _convert: &ConvertStep) -> HarnessStep {
        HarnessStep {
            status: StepStatus::NotRun,
            score: None,
            reason_kind: None,
            reason: HARNESS_NOTHING_CONVERTED.to_owned(),
        }
    }

    fn round_trip(&self, _ctx: &ModContext<'_>, _jar: &Path) -> RoundTripStep {
        RoundTripStep {
            status: StepStatus::NotRun,
            reason: ROUND_TRIP_NOT_RUN.to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::DownloadStep;

    fn wave_one_steps() -> Steps {
        Steps {
            download: DownloadStep {
                status: StepStatus::Ok,
                sha512_verified: true,
                error: None,
            },
            inspect: InspectStep {
                status: StepStatus::Ok,
                size: Some(1),
                entries: Some(1),
                manifests: Some(Manifests::default()),
                error: None,
            },
            classify: ClassifyStep {
                status: StepStatus::NotBuilt,
                tier: BoardTier::Unknown,
                reasons: vec![CLASSIFIER_NOT_BUILT.to_owned()],
            },
            convert: ConvertStep {
                status: StepStatus::NotBuilt,
                outcome: Outcome::Red,
                reason_kind: ReasonKind::NoConverter,
                reason: ReasonKind::NoConverter.label().to_owned(),
            },
            harness: HarnessStep {
                status: StepStatus::NotRun,
                score: None,
                reason_kind: None,
                reason: HARNESS_NOTHING_CONVERTED.to_owned(),
            },
            round_trip: RoundTripStep {
                status: StepStatus::NotRun,
                reason: ROUND_TRIP_NOT_RUN.to_owned(),
            },
        }
    }

    #[test]
    fn scores_map_as_d024_says() {
        assert_eq!(outcome_for_score(1.0), Outcome::Green);
        assert_eq!(outcome_for_score(0.75), Outcome::Green);
        assert_eq!(outcome_for_score(0.5), Outcome::Yellow);
        assert_eq!(outcome_for_score(0.25), Outcome::Red);
        assert_eq!(outcome_for_score(0.0), Outcome::Red);
    }

    #[test]
    fn no_harness_score_means_red_with_the_convert_reason() {
        let verdict = decide(&wave_one_steps());
        assert_eq!(verdict.outcome, Outcome::Red);
        assert_eq!(verdict.reason_kind, Some(ReasonKind::NoConverter));
        assert_eq!(
            verdict.reason.as_deref(),
            Some("No converter yet (milestone M2)")
        );
        assert_eq!(verdict.score, None);
    }

    #[test]
    fn a_convert_step_cannot_claim_green_without_the_harness() {
        let mut steps = wave_one_steps();
        steps.convert.outcome = Outcome::Green;
        assert_eq!(decide(&steps).outcome, Outcome::Red);
        steps.harness.status = StepStatus::Ok;
        assert_eq!(decide(&steps).outcome, Outcome::Red, "ok without a score");
    }

    #[test]
    fn a_harness_score_decides_when_present() {
        let mut steps = wave_one_steps();
        steps.harness.status = StepStatus::Ok;
        steps.harness.score = Some(0.75);
        let verdict = decide(&steps);
        assert_eq!(verdict.outcome, Outcome::Green);
        assert_eq!(verdict.reason_kind, None);
        steps.harness.score = Some(0.25);
        steps.harness.reason_kind = Some(ReasonKind::MixinFailure);
        steps.harness.reason = "mixin target not found".into();
        let verdict = decide(&steps);
        assert_eq!(verdict.outcome, Outcome::Red);
        assert_eq!(verdict.reason_kind, Some(ReasonKind::MixinFailure));
    }

    #[test]
    fn inspect_reports_unreadable_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("not-a.jar");
        std::fs::write(&path, b"definitely not a zip").unwrap();
        let step = inspect_jar(&path);
        assert_eq!(step.status, StepStatus::Failed);
        assert_eq!(step.size, Some(20));
        assert!(step.error.unwrap().contains("zip"));
        let missing = inspect_jar(&dir.path().join("missing.jar"));
        assert_eq!(missing.status, StepStatus::Failed);
        assert_eq!(missing.size, None);
    }
}
