//! Mapping formats and composition for Jarrow.
//!
//! A mod JAR built for one loader names Minecraft's classes, fields and methods in that
//! loader's runtime namespace: Fabric and Quilt ship intermediary names, NeoForge runs on
//! Mojang's names, Forge on SRG or Mojang's names depending on the game version. Converting
//! a mod means rewriting every such name, and this crate provides the tables to do it.
//!
//! - [`MappingSet`]: classes, fields, methods, parameters, local variables and packages with
//!   one name per namespace. Lookups by primary name ([`MappingSet::class`],
//!   [`MappingSet::field`], [`MappingSet::method`]) and from any namespace to any other
//!   ([`MappingSet::mapper`], [`Mapper`]).
//! - Formats: [`tiny1`], [`tiny2`], [`proguard`] (Mojang's `client.txt` and `server.txt`),
//!   [`tsrg`] (TSRG and TSRG2), each with a parser and a writer, and [`intermediary::load`]
//!   for the `mappings/mappings.tiny` entry of an intermediary jar.
//! - Transformations: [`compose`] through a shared namespace, [`MappingSet::invert`],
//!   [`MappingSet::project`], [`MappingSet::filter`], [`MappingSet::merge`], and [`chain`],
//!   which finds and composes the sets that lead from one namespace to another.
//! - [`desc`]: descriptor and generic-signature remapping, and the Java source types of
//!   ProGuard files.
//! - [`GlobalMemberMap`]: member names that are unique on their own (intermediary
//!   `field_N`, `method_N`), with conflict detection.
//!
//! Which namespace a loader runs on, for which game version, is an input here, never an
//! assumption: callers pass namespace names (see [`ns`] for the usual ones).
//!
//! The crate does no I/O and builds for `wasm32-unknown-unknown`. Hosts fetch mapping files
//! from the URLs pinned in `research-lock.json` and pass their bytes in. Mojang's mappings
//! are never stored in the repository.
//!
//! The Fabric to NeoForge chain:
//!
//! ```
//! use jarrow_mappings::{chain, intermediary, proguard};
//!
//! let intermediary = intermediary::load(
//!     b"tiny\t2\t0\tofficial\tintermediary\nc\ta\tnet/minecraft/class_1\n",
//! )?;
//! let mojang = proguard::parse_mojang("net.minecraft.demo.Thing -> a:\n")?;
//! let set = chain("intermediary", "mojang", &[&intermediary, &mojang])?;
//! let mapper = set.mapper("intermediary", "mojang")?;
//! assert_eq!(mapper.map_class("net/minecraft/class_1"), Some("net/minecraft/demo/Thing"));
//! # Ok::<(), jarrow_mappings::Error>(())
//! ```

#![forbid(unsafe_code)]

mod compose;
pub mod desc;
mod error;
mod global;
pub mod intermediary;
mod mapper;
mod model;
pub mod ns;
pub mod proguard;
mod text;
pub mod tiny1;
pub mod tiny2;
pub mod tsrg;

use std::fmt;

pub use compose::{chain, compose};
pub use error::{Error, Result};
pub use global::{
    ConflictTarget, GlobalLookup, GlobalMemberMap, MemberConflict, MemberKind,
    is_intermediary_member_name,
};
pub use mapper::Mapper;
pub use model::{
    ClassMapping, Entry, FieldMapping, InlineFrame, LineRange, MappingSet, MethodMapping, Names,
    PackageMapping, ParamMapping, Stats, VarMapping, names,
};

/// The mapping file formats this crate reads and writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Format {
    /// Tiny v1 (`v1\t...`).
    TinyV1,
    /// Tiny v2 (`tiny\t2\t0\t...`).
    TinyV2,
    /// ProGuard (`original -> obfuscated:`), Mojang's format.
    Proguard,
    /// TSRG, Forge's two-column SRG format.
    Tsrg,
    /// TSRG2 (`tsrg2 ...`).
    Tsrg2,
}

impl Format {
    /// Every format.
    pub const ALL: [Format; 5] = [
        Format::TinyV1,
        Format::TinyV2,
        Format::Proguard,
        Format::Tsrg,
        Format::Tsrg2,
    ];

    /// Stable machine id: `tiny-v1`, `tiny-v2`, `proguard`, `tsrg`, `tsrg2`.
    pub const fn id(self) -> &'static str {
        match self {
            Format::TinyV1 => "tiny-v1",
            Format::TinyV2 => "tiny-v2",
            Format::Proguard => "proguard",
            Format::Tsrg => "tsrg",
            Format::Tsrg2 => "tsrg2",
        }
    }

    /// Human label.
    pub const fn label(self) -> &'static str {
        match self {
            Format::TinyV1 => "tiny v1",
            Format::TinyV2 => "tiny v2",
            Format::Proguard => "ProGuard",
            Format::Tsrg => "TSRG",
            Format::Tsrg2 => "TSRG2",
        }
    }
}

impl fmt::Display for Format {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// Guesses the format of a mapping file from its first meaningful line. Returns `None` when
/// nothing matches. TSRG (v1) and ProGuard have no header, so they are told apart by shape.
pub fn detect(text: &str) -> Option<Format> {
    let text = text::strip_bom(text);
    if text.starts_with("tiny\t2\t") {
        return Some(Format::TinyV2);
    }
    if text.starts_with("v1\t") {
        return Some(Format::TinyV1);
    }
    if text.starts_with("tsrg2 ") {
        return Some(Format::Tsrg2);
    }
    let line = text
        .lines()
        .map(str::trim_end)
        .find(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))?;
    if line.contains(" -> ") && line.ends_with(':') {
        return Some(Format::Proguard);
    }
    if !line.starts_with(char::is_whitespace) && line.split_whitespace().count() == 2 {
        return Some(Format::Tsrg);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_ids() {
        let ids: Vec<_> = Format::ALL.iter().map(|f| f.id()).collect();
        assert_eq!(ids, ["tiny-v1", "tiny-v2", "proguard", "tsrg", "tsrg2"]);
        assert_eq!(Format::Proguard.to_string(), "ProGuard");
    }

    #[test]
    fn detection() {
        assert_eq!(detect("tiny\t2\t0\ta\tb\n"), Some(Format::TinyV2));
        assert_eq!(detect("v1\ta\tb\n"), Some(Format::TinyV1));
        assert_eq!(detect("tsrg2 obf srg\n"), Some(Format::Tsrg2));
        assert_eq!(detect("# header\n\na.B -> c:\n"), Some(Format::Proguard));
        assert_eq!(detect("a net/minecraft/A\n\tb f_1\n"), Some(Format::Tsrg));
        assert_eq!(detect(""), None);
        assert_eq!(detect("hello there world\n"), None);
    }
}
