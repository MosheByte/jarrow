//! Mod loaders and conversion directions.
//!
//! A direction is `<source>-<target>`, for example `fabric-neoforge`. The source loader decides
//! which Modrinth projects and files the board tests; the target loader names what the
//! converter will produce. Every pair of distinct loaders is a valid direction, so nothing in
//! the pipeline assumes Fabric or NeoForge.
//!
//! The twelve directions have a fixed build order and a v1 bar each (decision D-029,
//! `docs/directions/README.md`). [`Direction::all`] lists them in that order, and directions
//! sort by it.

use std::cmp::Ordering;
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

    /// The loader tag Modrinth uses in `categories` facets and in version `loaders`, as the
    /// loader's profile records it (`modrinth_loader` in `loaders/<id>.toml`), or `None` while
    /// the profile says `"unknown"`. A loader without a researched tag gets no source list:
    /// code that meets `"unknown"` stops (decision D-038). A test keeps this function in step
    /// with the profiles.
    pub const fn modrinth_tag(self) -> Option<&'static str> {
        match self {
            Loader::Fabric => Some("fabric"),
            Loader::NeoForge | Loader::Forge | Loader::Quilt => None,
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

/// The twelve directions in the fixed build order: source, target, build step, and the share
/// of the direction's board column that must be green for v1 (`docs/board.md`, D-029).
const BUILD_ORDER: [(Loader, Loader, u8, u8); 12] = [
    (Loader::Fabric, Loader::NeoForge, 1, 60),
    (Loader::NeoForge, Loader::Fabric, 2, 50),
    (Loader::Fabric, Loader::Quilt, 3, 50),
    (Loader::Quilt, Loader::Fabric, 3, 50),
    (Loader::Forge, Loader::NeoForge, 4, 50),
    (Loader::NeoForge, Loader::Forge, 4, 50),
    (Loader::Fabric, Loader::Forge, 5, 50),
    (Loader::Forge, Loader::Fabric, 5, 50),
    (Loader::Quilt, Loader::Forge, 6, 50),
    (Loader::Forge, Loader::Quilt, 6, 50),
    (Loader::Quilt, Loader::NeoForge, 6, 50),
    (Loader::NeoForge, Loader::Quilt, 6, 50),
];

/// A conversion direction: from one loader's JAR to another's. Directions compare by their
/// place in the build order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

    /// Every valid direction (twelve in v1), in the build order.
    pub fn all() -> Vec<Direction> {
        BUILD_ORDER
            .iter()
            .map(|&(source, target, _, _)| Direction { source, target })
            .collect()
    }

    /// `(index in the build order, build step, bar)`. Every valid direction is in the table.
    fn entry(self) -> (usize, u8, u8) {
        BUILD_ORDER
            .iter()
            .enumerate()
            .find(|(_, (s, t, _, _))| *s == self.source && *t == self.target)
            .map_or((usize::MAX, 0, 0), |(i, &(_, _, order, bar))| {
                (i, order, bar)
            })
    }

    /// The direction's index in [`Direction::all`], 0 to 11.
    pub fn position(self) -> usize {
        self.entry().0
    }

    /// Its step in the build order, 1 to 6.
    pub fn order(self) -> u8 {
        self.entry().1
    }

    /// The share of its board column that must be green for v1, in percent.
    pub fn bar_percent(self) -> u8 {
        self.entry().2
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

impl PartialOrd for Direction {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Direction {
    fn cmp(&self, other: &Self) -> Ordering {
        self.position().cmp(&other.position())
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
        for (i, d) in all.iter().enumerate() {
            assert_eq!(d.id().parse::<Direction>().unwrap(), *d);
            assert_eq!(d.position(), i);
        }
        for source in Loader::ALL {
            for target in Loader::ALL {
                if source != target {
                    assert!(all.contains(&Direction::new(source, target).unwrap()));
                }
            }
        }
    }

    #[test]
    fn build_order_and_bars_follow_d029() {
        let all = Direction::all();
        let ids: Vec<String> = all.iter().map(|d| d.id()).collect();
        assert_eq!(
            ids,
            [
                "fabric-neoforge",
                "neoforge-fabric",
                "fabric-quilt",
                "quilt-fabric",
                "forge-neoforge",
                "neoforge-forge",
                "fabric-forge",
                "forge-fabric",
                "quilt-forge",
                "forge-quilt",
                "quilt-neoforge",
                "neoforge-quilt",
            ]
        );
        let orders: Vec<u8> = all.iter().map(|d| d.order()).collect();
        assert_eq!(orders, [1, 2, 3, 3, 4, 4, 5, 5, 6, 6, 6, 6]);
        for d in &all {
            let bar = if d.id() == "fabric-neoforge" { 60 } else { 50 };
            assert_eq!(d.bar_percent(), bar, "{d}");
        }
        let mut sorted = all.clone();
        sorted.reverse();
        sorted.sort();
        assert_eq!(sorted, all);
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

    /// `modrinth_tag` says what `loaders/<id>.toml` records: the tag, or `None` for
    /// `"unknown"`.
    #[test]
    fn modrinth_tags_match_the_loader_profiles() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../loaders");
        for loader in Loader::ALL {
            let path = root.join(format!("{}.toml", loader.id()));
            let text = std::fs::read_to_string(&path).unwrap();
            let recorded = text
                .lines()
                .find_map(|line| {
                    let rest = line.trim().strip_prefix("modrinth_loader")?;
                    let value = rest.trim_start().strip_prefix('=')?.trim_start();
                    let value = value.strip_prefix('"')?;
                    Some(value[..value.find('"')?].to_owned())
                })
                .unwrap_or_else(|| panic!("{} has no modrinth_loader", path.display()));
            let expected = (recorded != "unknown").then_some(recorded.as_str());
            assert_eq!(loader.modrinth_tag(), expected, "{}", path.display());
        }
    }
}
