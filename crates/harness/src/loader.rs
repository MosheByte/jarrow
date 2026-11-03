//! Loader ids. The loader is always an input; nothing in the harness assumes Fabric or
//! NeoForge except the provisioning profiles that implement them.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// A mod loader the harness knows by name.
///
/// Serialised as its id: `"fabric"`, `"neoforge"`, `"forge"` or `"quilt"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Loader {
    /// Fabric Loader (with Fabric API on the server).
    Fabric,
    /// NeoForge.
    #[serde(rename = "neoforge")]
    NeoForge,
    /// Minecraft Forge.
    Forge,
    /// Quilt Loader.
    Quilt,
}

impl Loader {
    /// Every loader, in id order.
    pub const ALL: [Loader; 4] = [
        Loader::Fabric,
        Loader::NeoForge,
        Loader::Forge,
        Loader::Quilt,
    ];

    /// Stable machine id, identical to the serialised form and to the probe dump's `loader`.
    pub const fn id(self) -> &'static str {
        match self {
            Loader::Fabric => "fabric",
            Loader::NeoForge => "neoforge",
            Loader::Forge => "forge",
            Loader::Quilt => "quilt",
        }
    }
}

impl fmt::Display for Loader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id())
    }
}

/// A string that is not a loader id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownLoader(pub String);

impl fmt::Display for UnknownLoader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unknown loader {:?} (expected one of fabric, neoforge, forge, quilt)",
            self.0
        )
    }
}

impl std::error::Error for UnknownLoader {}

impl FromStr for Loader {
    type Err = UnknownLoader;

    /// Parses a loader id, case-insensitively.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Loader::ALL
            .into_iter()
            .find(|loader| loader.id().eq_ignore_ascii_case(s.trim()))
            .ok_or_else(|| UnknownLoader(s.to_owned()))
    }
}

/// Whether `version` looks like a Minecraft release id (`1.21.1`, `1.21.11`, `26.1`).
///
/// The harness never hard-codes the pinned versions; which ones exist is decided by
/// `research-lock.json`.
pub fn is_game_version(version: &str) -> bool {
    let parts: Vec<&str> = version.split('.').collect();
    (2..=3).contains(&parts.len())
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_round_trip_through_serde_and_from_str() {
        for loader in Loader::ALL {
            let json = serde_json::to_string(&loader).unwrap();
            assert_eq!(json, format!("\"{}\"", loader.id()));
            assert_eq!(serde_json::from_str::<Loader>(&json).unwrap(), loader);
            assert_eq!(loader.id().parse::<Loader>().unwrap(), loader);
        }
        assert_eq!(" NeoForge ".parse::<Loader>().unwrap(), Loader::NeoForge);
        assert!("neo-forge".parse::<Loader>().is_err());
        assert!(serde_json::from_str::<Loader>("\"neo_forge\"").is_err());
    }

    #[test]
    fn game_versions_are_shape_checked_only() {
        for ok in ["1.20.1", "1.21.1", "1.21.11", "26.1"] {
            assert!(is_game_version(ok), "{ok}");
        }
        for bad in ["", "1", "1.21.", "1.21.1-pre1", "24w10a", "1.2.3.4"] {
            assert!(!is_game_version(bad), "{bad}");
        }
    }
}
