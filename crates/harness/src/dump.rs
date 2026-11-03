//! The probe dump: what a probe mod writes to `jarrow-dump.json` (format in
//! `harness/dump-schema.json`, prose in `docs/harness.md`).

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::canon::canonicalize;
use crate::loader::Loader;
use crate::schema::{DUMP_SCHEMA_VERSION, Schema, SchemaError, Violation};

/// The file name the probe writes in the server's working directory.
pub const DUMP_FILE_NAME: &str = "jarrow-dump.json";

/// One loaded mod, as the loader lists it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ModEntry {
    /// Mod id.
    pub id: String,
    /// Mod version, as the loader reports it.
    pub version: String,
}

/// The worldgen sample.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Worldgen {
    /// Overworld seed.
    pub seed: i64,
    /// Chunk coordinates that were sampled.
    pub chunks: Vec<[i64; 2]>,
    /// Block state to count.
    pub block_histogram: BTreeMap<String, u64>,
}

/// Log events the probe captured.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DumpLog {
    /// ERROR and FATAL events.
    pub errors: Vec<String>,
    /// WARN events.
    pub warnings: Vec<String>,
}

/// A validated, canonicalised probe dump.
///
/// [`Dump::from_json`] is the only way the harness reads one: it validates against the
/// schema, then canonicalises (mods sorted by id, tag entries sorted, JSON values in
/// canonical form). Registry entry order is kept, because it is measured.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Dump {
    /// Format version; always [`DUMP_SCHEMA_VERSION`].
    pub schema: u64,
    /// Loader the server ran on.
    pub loader: Loader,
    /// Loader version as the loader reports it.
    pub loader_version: String,
    /// Minecraft version.
    pub game_version: String,
    /// Loaded mods, sorted by id.
    pub mods: Vec<ModEntry>,
    /// Registry id to entry ids, in raw id order.
    pub registries: BTreeMap<String, Vec<String>>,
    /// Registry id to tag id to sorted entry ids.
    pub tags: BTreeMap<String, BTreeMap<String, Vec<String>>>,
    /// Recipe id to canonical JSON.
    pub recipes: BTreeMap<String, Value>,
    /// Loot table id to canonical JSON.
    pub loot_tables: BTreeMap<String, Value>,
    /// Advancement id to canonical JSON.
    pub advancements: BTreeMap<String, Value>,
    /// Worldgen sample.
    pub worldgen: Worldgen,
    /// Captured log events.
    pub log: DumpLog,
    /// Problems the probe hit while dumping.
    pub errors: Vec<String>,
}

/// Why a dump could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DumpError {
    /// Not JSON at all.
    Json(String),
    /// JSON, but it breaks `harness/dump-schema.json`.
    Schema(Vec<Violation>),
    /// The embedded schema itself failed to compile (a build defect).
    BadSchema(SchemaError),
}

impl fmt::Display for DumpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DumpError::Json(e) => write!(f, "not valid JSON: {e}"),
            DumpError::Schema(violations) => {
                write!(
                    f,
                    "does not match harness/dump-schema.json ({} problem{})",
                    violations.len(),
                    if violations.len() == 1 { "" } else { "s" }
                )?;
                for v in violations.iter().take(10) {
                    write!(f, "; {v}")?;
                }
                if violations.len() > 10 {
                    write!(f, "; ...")?;
                }
                Ok(())
            }
            DumpError::BadSchema(e) => write!(f, "the embedded dump schema is invalid: {e}"),
        }
    }
}

impl std::error::Error for DumpError {}

impl Dump {
    /// Parses, validates and canonicalises a dump.
    pub fn from_json(text: &str) -> Result<Self, DumpError> {
        let value: Value =
            serde_json::from_str(text).map_err(|e| DumpError::Json(e.to_string()))?;
        Self::from_value(&value)
    }

    /// Validates and canonicalises a dump that is already parsed.
    pub fn from_value(value: &Value) -> Result<Self, DumpError> {
        validate_dump(value)?;
        let mut dump: Dump = serde_json::from_value(value.clone()).map_err(|e| {
            // The schema admits every shape serde accepts; this only fires on numbers out of
            // range (a seed beyond i64, a count beyond u64).
            DumpError::Schema(vec![Violation {
                pointer: String::new(),
                message: e.to_string(),
            }])
        })?;
        dump.canonicalize();
        Ok(dump)
    }

    fn canonicalize(&mut self) {
        self.mods.sort();
        for tags in self.tags.values_mut() {
            for entries in tags.values_mut() {
                entries.sort();
                entries.dedup();
            }
        }
        for map in [
            &mut self.recipes,
            &mut self.loot_tables,
            &mut self.advancements,
        ] {
            for value in map.values_mut() {
                *value = canonicalize(value);
            }
        }
    }

    /// Number of measured entries: registry entries, tag members (an empty tag counts once),
    /// recipes, loot tables, advancements and worldgen block states. This is the unit the
    /// 1% band in the score is computed over.
    pub fn units(&self) -> usize {
        let registries: usize = self.registries.values().map(|e| e.len().max(1)).sum();
        let tags: usize = self
            .tags
            .values()
            .flat_map(BTreeMap::values)
            .map(|members| members.len().max(1))
            .sum();
        registries
            + tags
            + self.recipes.len()
            + self.loot_tables.len()
            + self.advancements.len()
            + self.worldgen.block_histogram.len()
    }
}

/// Validates a parsed dump against the embedded schema (no canonicalisation).
pub fn validate_dump(value: &Value) -> Result<(), DumpError> {
    let schema = Schema::dump().map_err(DumpError::BadSchema)?;
    schema.validate(value).map_err(DumpError::Schema)
}

/// The dump format version a dump declares, if it declares one. Lets callers tell an old or
/// newer probe from a broken one before validating.
pub fn declared_schema_version(value: &Value) -> Option<u64> {
    value.get("schema").and_then(Value::as_u64)
}

/// Whether this build reads the dump format version `version`.
pub const fn reads_schema_version(version: u64) -> bool {
    version == DUMP_SCHEMA_VERSION
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use serde_json::json;

    /// A small, valid dump for tests. `loader` decides the loader-specific bits.
    pub(crate) fn sample(loader: Loader) -> Value {
        let (loader_version, loader_mods) = match loader {
            Loader::Fabric => (
                "0.20.1",
                vec![
                    ("fabric-api", "0.116.17+1.21.1"),
                    ("fabricloader", "0.20.1"),
                ],
            ),
            Loader::NeoForge => ("21.1.265", vec![("neoforge", "21.1.265")]),
            Loader::Forge => ("52.0.0", vec![("forge", "52.0.0")]),
            Loader::Quilt => ("0.29.0", vec![("quilt_loader", "0.29.0")]),
        };
        let mut mods: Vec<Value> = loader_mods
            .into_iter()
            .chain([("jarrow_probe", "0.1.0"), ("minecraft", "1.21.1")])
            .map(|(id, version)| json!({"id": id, "version": version}))
            .collect();
        mods.sort_by_key(|m| m["id"].as_str().unwrap_or_default().to_owned());
        json!({
            "schema": 1,
            "loader": loader.id(),
            "loader_version": loader_version,
            "game_version": "1.21.1",
            "mods": mods,
            "registries": {
                "minecraft:block": ["minecraft:air", "minecraft:stone", "minecraft:granite"],
                "minecraft:item": ["minecraft:air", "minecraft:stone", "minecraft:stick"],
                "minecraft:worldgen/biome": ["minecraft:plains", "minecraft:desert"]
            },
            "tags": {
                "minecraft:item": {
                    "minecraft:logs": ["minecraft:birch_log", "minecraft:oak_log"],
                    "c:rods/wooden": ["minecraft:stick"]
                }
            },
            "recipes": {
                "minecraft:stick": {"type": "minecraft:crafting_shaped", "pattern": ["#", "#"], "key": {"#": "#minecraft:planks"}, "result": {"id": "minecraft:stick", "count": 4}}
            },
            "loot_tables": {
                "minecraft:blocks/stone": {"type": "minecraft:block", "pools": [{"rolls": 1.0, "entries": [{"type": "minecraft:item", "name": "minecraft:cobblestone"}]}]}
            },
            "advancements": {
                "minecraft:story/root": {"criteria": {"crafting_table": {"trigger": "minecraft:inventory_changed"}}}
            },
            "worldgen": {
                "seed": 20261103,
                "chunks": [[0, 0], [1, 0]],
                "block_histogram": {"minecraft:air": 100, "minecraft:stone": 50, "minecraft:oak_log[axis=y]": 3}
            },
            "log": {"errors": [], "warnings": []},
            "errors": []
        })
    }

    #[test]
    fn a_sample_dump_is_valid_for_every_loader() {
        for loader in Loader::ALL {
            let dump = Dump::from_value(&sample(loader)).unwrap();
            assert_eq!(dump.loader, loader);
            assert_eq!(dump.units(), 3 + 3 + 2 + 2 + 1 + 1 + 1 + 1 + 3);
        }
    }

    #[test]
    fn canonicalisation_sorts_what_is_unordered_and_keeps_registry_order() {
        let mut value = sample(Loader::Fabric);
        value["mods"] = json!([{"id": "z", "version": "1"}, {"id": "a", "version": "2"}]);
        value["tags"]["minecraft:item"]["minecraft:logs"] =
            json!(["minecraft:oak_log", "minecraft:birch_log"]);
        value["registries"]["minecraft:item"] = json!(["minecraft:stick", "minecraft:air"]);
        value["recipes"]["minecraft:stick"]["result"] =
            json!({"count": 4.0, "id": "minecraft:stick"});
        let dump = Dump::from_value(&value).unwrap();
        assert_eq!(dump.mods[0].id, "a");
        assert_eq!(
            dump.tags["minecraft:item"]["minecraft:logs"],
            ["minecraft:birch_log", "minecraft:oak_log"]
        );
        assert_eq!(
            dump.registries["minecraft:item"],
            ["minecraft:stick", "minecraft:air"]
        );
        assert_eq!(dump.recipes["minecraft:stick"]["result"]["count"], json!(4));
    }

    #[test]
    fn schema_violations_are_reported_with_pointers() {
        let mut value = sample(Loader::NeoForge);
        value["loader"] = json!("rift");
        value["registries"]["minecraft:item"] = json!(["minecraft:stone", "minecraft:stone"]);
        value["worldgen"]["block_histogram"]["Minecraft:Stone"] = json!(-1);
        value["extra"] = json!(true);
        let Err(DumpError::Schema(violations)) = Dump::from_value(&value) else {
            panic!("expected schema violations");
        };
        let text: Vec<String> = violations.iter().map(ToString::to_string).collect();
        for expected in [
            "/loader: must be one of",
            "/registries/minecraft:item: items must be unique",
            "/worldgen/block_histogram/Minecraft:Stone: \"Minecraft:Stone\" does not match",
            "/worldgen/block_histogram/Minecraft:Stone: must be at least 0",
            "/extra: property is not allowed here",
        ] {
            assert!(
                text.iter().any(|t| t.starts_with(expected)),
                "{expected} not in {text:?}"
            );
        }
    }

    #[test]
    fn wrong_versions_and_missing_sections_are_refused() {
        let mut value = sample(Loader::Fabric);
        value["schema"] = json!(2);
        assert!(Dump::from_value(&value).is_err());
        let mut value = sample(Loader::Fabric);
        value.as_object_mut().unwrap().remove("worldgen");
        let error = Dump::from_value(&value).unwrap_err().to_string();
        assert!(
            error.contains("missing required property \"worldgen\""),
            "{error}"
        );
        assert!(matches!(
            Dump::from_json("{not json"),
            Err(DumpError::Json(_))
        ));
        let mut value = sample(Loader::Fabric);
        value["worldgen"]["seed"] = json!(1.0e30);
        assert!(
            Dump::from_value(&value).is_err(),
            "a seed beyond i64 is refused"
        );
    }
}
