//! Mod loaders and conversion directions.
//!
//! A direction is `<source>-<target>`, for example `fabric-neoforge`. The source loader decides
//! which Modrinth projects and files the board tests; the target loader names what the
//! converter will produce. Every pair of distinct loaders is a valid direction, so nothing in
//! the pipeline assumes Fabric or NeoForge.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::Error;

/// A mod loader Jarrow knows (v1 scope).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Loader {
    /// Fabric (fabricmc.net).
    Fabric,
    /// NeoForge (neoforged.net).
    NeoForge,
    /// Minecraft Forge (minecraftforge.net).
    Forge,
    /// Quilt (quiltmc.org).
    Quilt,
}

impl Loader {
    /// Every loader, in a fixed order.
    pub const ALL: [Loader; 4] = [
        Loader::Fabric,
        Loader::NeoForge,
        Loader::Forge,
        Loader::Quilt,
    ];

    /// The loader id used on the command line, in file names and in JSON.
    pub const fn id(self) -> &'static str {
        match self {
            Loader::Fabric => "fabric",
            Loader::NeoForge => "neoforge",
            Loader::Forge => "forge",
            Loader::Quilt => "quilt",
        }
    }

    /// Display name.
    pub const fn name(self) -> &'static str {
        match self {
            Loader::Fabric => "Fabric",
            Loader::NeoForge => "NeoForge",
            Loader::Forge => "Forge",
            Loader::Quilt => "Quilt",
        }
    }

    /// The loader tag Modrinth uses in `categories` facets and in version `loaders`.
    pub const fn modrinth_tag(self) -> &'static str {
        // Modrinth's tags are the same lowercase ids today; kept separate so a rename on
        // Modrinth's side touches one function.
        self.id()
    }

    /// The manifest a JAR for this loader carries, as a path inside the JAR.
    ///
    /// Forge and NeoForge before 20.5 share `META-INF/mods.toml`; NeoForge 20.5 and later
    /// (all pinned versions) use `META-INF/neoforge.mods.toml`.
    pub const fn manifest_path(self) -> &'static str {
        match self {
            Loader::Fabric => "fabric.mod.json",
            Loader::NeoForge => "META-INF/neoforge.mods.toml",
            Loader::Forge => "META-INF/mods.toml",
            Loader::Quilt => "quilt.mod.json",
        }
    }
}

impl fmt::Display for Loader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id())
    }
}

impl FromStr for Loader {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Loader::ALL
            .into_iter()
            .find(|loader| loader.id() == s.trim().to_ascii_lowercase())
            .ok_or_else(|| {
                Error::invalid(format!(
                    "unknown loader {s:?}; expected one of fabric, neoforge, forge, quilt"
                ))
            })
    }
}

/// A conversion direction: from one loader's JAR to another's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Direction {
    /// The loader the original mod is built for.
    pub source: Loader,
    /// The loader the converted mod targets.
    pub target: Loader,
}

impl Direction {
    /// A direction between two different loaders.
    pub fn new(source: Loader, target: Loader) -> Result<Self, Error> {
        if source == target {
            return Err(Error::invalid(format!(
                "a direction needs two different loaders, got {source}-{target}"
            )));
        }
        Ok(Self { source, target })
    }

    /// Every valid direction (twelve in v1).
    pub fn all() -> Vec<Direction> {
        let mut out = Vec::new();
        for source in Loader::ALL {
            for target in Loader::ALL {
                if source != target {
                    out.push(Direction { source, target });
                }
            }
        }
        out
    }

    /// The id, `<source>-<target>`.
    pub fn id(self) -> String {
        format!("{}-{}", self.source.id(), self.target.id())
    }

    /// Human label, "Fabric to NeoForge".
    pub fn label(self) -> String {
        format!("{} to {}", self.source.name(), self.target.name())
    }
}

impl fmt::Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}-{}", self.source.id(), self.target.id())
    }
}

impl FromStr for Direction {
    type Err = Error;

    /// Parses `<source>-<target>`, for example `fabric-neoforge`.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (source, target) = s.trim().split_once('-').ok_or_else(|| {
            Error::invalid(format!(
                "direction {s:?} must be <source>-<target>, for example fabric-neoforge"
            ))
        })?;
        Direction::new(source.parse()?, target.parse()?)
    }
}

impl Serialize for Direction {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.id())
    }
}

impl<'de> Deserialize<'de> for Direction {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loader_ids_round_trip() {
        for loader in Loader::ALL {
            assert_eq!(loader.id().parse::<Loader>().unwrap(), loader);
            let json = serde_json::to_string(&loader).unwrap();
            assert_eq!(json, format!("\"{}\"", loader.id()));
            assert_eq!(serde_json::from_str::<Loader>(&json).unwrap(), loader);
        }
        assert_eq!(" NeoForge ".parse::<Loader>().unwrap(), Loader::NeoForge);
        assert!("liteloader".parse::<Loader>().is_err());
    }

    #[test]
    fn directions_parse_and_print() {
        let d: Direction = "fabric-neoforge".parse().unwrap();
        assert_eq!(d.source, Loader::Fabric);
        assert_eq!(d.target, Loader::NeoForge);
        assert_eq!(d.id(), "fabric-neoforge");
        assert_eq!(d.label(), "Fabric to NeoForge");
        assert_eq!(
            serde_json::to_string(&d).unwrap(),
            "\"fabric-neoforge\"".to_owned()
        );
        assert_eq!(
            serde_json::from_str::<Direction>("\"quilt-forge\"").unwrap(),
            Direction::new(Loader::Quilt, Loader::Forge).unwrap()
        );
    }

    #[test]
    fn every_direction_is_valid_and_distinct() {
        let all = Direction::all();
        assert_eq!(all.len(), 12);
        for d in &all {
            assert_eq!(d.id().parse::<Direction>().unwrap(), *d);
        }
    }

    #[test]
    fn bad_directions_are_errors() {
        for bad in [
            "fabric",
            "fabric-fabric",
            "fabric-",
            "-neoforge",
            "fabric-rift",
            "",
        ] {
            assert!(bad.parse::<Direction>().is_err(), "{bad}");
        }
        assert!(serde_json::from_str::<Direction>("\"forge-forge\"").is_err());
    }

    #[test]
    fn manifests_are_distinct() {
        let mut paths: Vec<_> = Loader::ALL.iter().map(|l| l.manifest_path()).collect();
        paths.sort_unstable();
        paths.dedup();
        assert_eq!(paths.len(), 4);
    }
}
