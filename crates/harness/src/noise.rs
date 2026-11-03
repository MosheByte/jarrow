//! Loader noise, learned from a baseline pair.
//!
//! A baseline pair is two runs with nothing but the probe: one on the reference loader, one on
//! the candidate loader, same game version. Whatever differs between them (NeoForge's own
//! registries, Fabric API's tags, loader log lines) is a property of the loaders, not of any
//! mod. [`Noise::learn`] records those differences; [`Noise::is_noise`] recognises them in a
//! mod comparison so they can be subtracted before scoring.
//!
//! The noise file also keeps each side's mod list and entry count, so the scorer can tell the
//! mods under test from the loader's own mods and the mod's own entries from vanilla's.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::diff::{Category, CompareError, DiffKind, Difference, diff_dumps};
use crate::dump::Dump;
use crate::loader::Loader;
use crate::logs::normalize_message;

/// The `record` value of a noise file.
pub const NOISE_RECORD: &str = "jarrow-harness-noise";

/// The noise file format version.
pub const NOISE_SCHEMA_VERSION: u32 = 1;

/// One side of a baseline pair.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoiseSide {
    /// The loader.
    pub loader: Loader,
    /// The loader version the baseline ran.
    pub loader_version: String,
    /// The mod ids the loader loads with only the probe (the loader, its API, the probe,
    /// `minecraft`, ...).
    pub mods: Vec<String>,
    /// [`Dump::units`] of the baseline dump.
    pub units: usize,
}

impl NoiseSide {
    fn of(dump: &Dump) -> Self {
        Self {
            loader: dump.loader,
            loader_version: dump.loader_version.clone(),
            mods: dump.mods.iter().map(|m| m.id.clone()).collect(),
            units: dump.units(),
        }
    }
}

/// The identity of a difference, without its human detail.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct NoiseItem {
    /// Category.
    pub category: Category,
    /// Kind.
    pub kind: DiffKind,
    /// Path.
    pub path: Vec<String>,
}

impl From<&Difference> for NoiseItem {
    fn from(d: &Difference) -> Self {
        Self {
            category: d.category,
            kind: d.kind,
            path: d.path.clone(),
        }
    }
}

/// Loader noise for one (reference loader, candidate loader, game version).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Noise {
    /// Always [`NOISE_RECORD`].
    pub record: String,
    /// Always [`NOISE_SCHEMA_VERSION`].
    pub schema: u32,
    /// The game version of both baseline runs.
    pub game_version: String,
    /// The reference loader's baseline.
    pub reference: NoiseSide,
    /// The candidate loader's baseline.
    pub candidate: NoiseSide,
    /// Every content and order difference between the two baselines.
    pub differences: BTreeSet<NoiseItem>,
    /// Every normalised log message and probe error either baseline produced. A mod run that
    /// logs one of these is not blamed for it.
    pub log: BTreeSet<String>,
}

impl Noise {
    /// Learns the noise from a baseline pair.
    pub fn learn(reference: &Dump, candidate: &Dump) -> Result<Self, CompareError> {
        let differences = diff_dumps(reference, candidate)?
            .iter()
            .filter(|d| !matches!(d.category, Category::Log | Category::ProbeErrors))
            .map(NoiseItem::from)
            .collect();
        let log = [reference, candidate]
            .into_iter()
            .flat_map(|d| d.log.errors.iter().chain(&d.log.warnings).chain(&d.errors))
            .map(|m| normalize_message(m))
            .collect();
        Ok(Self {
            record: NOISE_RECORD.to_owned(),
            schema: NOISE_SCHEMA_VERSION,
            game_version: reference.game_version.clone(),
            reference: NoiseSide::of(reference),
            candidate: NoiseSide::of(candidate),
            differences,
            log,
        })
    }

    /// Reads a noise file.
    pub fn from_json(text: &str) -> Result<Self, String> {
        let noise: Noise =
            serde_json::from_str(text).map_err(|e| format!("not a noise file: {e}"))?;
        if noise.record != NOISE_RECORD {
            return Err(format!("not a noise file: record is {:?}", noise.record));
        }
        if noise.schema != NOISE_SCHEMA_VERSION {
            return Err(format!(
                "noise file schema {} is not supported (this build reads {NOISE_SCHEMA_VERSION})",
                noise.schema
            ));
        }
        Ok(noise)
    }

    /// Writes the noise file as pretty JSON with sorted contents.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        Ok(serde_json::to_string_pretty(self)? + "\n")
    }

    /// Fails unless this baseline was learned for the loaders, loader versions and game version
    /// of the two dumps. A stale baseline would hide real differences or invent some.
    pub fn check_applies(&self, reference: &Dump, candidate: &Dump) -> Result<(), CompareError> {
        let mismatch = |what: String| Err(CompareError::BaselineMismatch(what));
        if self.game_version != reference.game_version
            || self.game_version != candidate.game_version
        {
            return mismatch(format!(
                "baseline is for game version {}, the dumps are {} and {}",
                self.game_version, reference.game_version, candidate.game_version
            ));
        }
        for (side, dump, role) in [
            (&self.reference, reference, "reference"),
            (&self.candidate, candidate, "candidate"),
        ] {
            if side.loader != dump.loader || side.loader_version != dump.loader_version {
                return mismatch(format!(
                    "baseline {role} is {} {}, the {role} dump ran {} {}; re-run `jarrow-harness baseline`",
                    side.loader, side.loader_version, dump.loader, dump.loader_version
                ));
            }
        }
        Ok(())
    }

    /// Whether `difference` is loader noise.
    pub fn is_noise(&self, difference: &Difference) -> bool {
        match difference.category {
            Category::Log | Category::ProbeErrors => difference
                .path
                .last()
                .is_some_and(|message| self.log.contains(message)),
            _ => self.differences.contains(&NoiseItem::from(difference)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dump::tests::sample;
    use serde_json::json;

    fn baseline_pair() -> (Dump, Dump) {
        let fabric = sample(Loader::Fabric);
        let mut neoforge = sample(Loader::NeoForge);
        neoforge["registries"]["neoforge:fluid_type"] = json!(["neoforge:empty", "neoforge:water"]);
        neoforge["tags"]["minecraft:item"]["c:rods/wooden"] =
            json!(["minecraft:bamboo", "minecraft:stick"]);
        neoforge["log"]["warnings"] =
            json!(["[03Nov2026 10:00:00.000] [main/WARN] [net.neoforged.fml/]: Config took 31 ms"]);
        (
            Dump::from_value(&fabric).unwrap(),
            Dump::from_value(&neoforge).unwrap(),
        )
    }

    #[test]
    fn learns_differences_mods_units_and_log_lines() {
        let (a, b) = baseline_pair();
        let noise = Noise::learn(&a, &b).unwrap();
        assert_eq!(noise.game_version, "1.21.1");
        assert_eq!(noise.reference.loader, Loader::Fabric);
        assert_eq!(noise.candidate.loader_version, "21.1.265");
        assert_eq!(
            noise.reference.mods,
            ["fabric-api", "fabricloader", "jarrow_probe", "minecraft"]
        );
        assert_eq!(noise.candidate.units, a.units() + 3);
        assert!(noise.log.contains("Config took # ms"));
        let paths: Vec<Vec<&str>> = noise
            .differences
            .iter()
            .map(|i| i.path.iter().map(String::as_str).collect())
            .collect();
        assert!(paths.contains(&vec!["neoforge:fluid_type", "neoforge:water"]));
        assert!(paths.contains(&vec!["minecraft:item", "c:rods/wooden", "minecraft:bamboo"]));
        assert!(paths.contains(&vec!["fabric-api"]));
    }

    #[test]
    fn round_trips_through_json_and_checks_its_header() {
        let (a, b) = baseline_pair();
        let noise = Noise::learn(&a, &b).unwrap();
        let text = noise.to_json().unwrap();
        assert_eq!(Noise::from_json(&text).unwrap(), noise);
        assert_eq!(noise.to_json().unwrap(), text, "deterministic");
        let wrong = text.replace(NOISE_RECORD, "something-else");
        assert!(Noise::from_json(&wrong).unwrap_err().contains("record"));
        let newer = text.replace("\"schema\": 1", "\"schema\": 9");
        assert!(Noise::from_json(&newer).unwrap_err().contains("schema 9"));
        assert!(Noise::from_json("[]").is_err());
    }

    #[test]
    fn applies_only_to_the_same_loaders_and_versions() {
        let (a, b) = baseline_pair();
        let noise = Noise::learn(&a, &b).unwrap();
        assert!(noise.check_applies(&a, &b).is_ok());
        assert!(noise.check_applies(&b, &a).is_err(), "sides swapped");
        let mut newer = sample(Loader::NeoForge);
        newer["loader_version"] = json!("21.1.300");
        let err = noise
            .check_applies(&a, &Dump::from_value(&newer).unwrap())
            .unwrap_err();
        assert!(
            err.to_string().contains("re-run `jarrow-harness baseline`"),
            "{err}"
        );
        let mut other = sample(Loader::NeoForge);
        other["game_version"] = json!("1.21.11");
        assert!(
            noise
                .check_applies(&a, &Dump::from_value(&other).unwrap())
                .is_err()
        );
    }
}
