//! Fabric's intermediary mappings, as published in the `intermediary-<version>-v2.jar` layout.
//!
//! The jar holds one file, [`JAR_ENTRY`], in tiny v2 with the header
//! `tiny 2 0 official intermediary`. This crate does no I/O and no unzipping: the host reads
//! that entry out of the jar (whose URL and hashes are pinned in `research-lock.json`) and
//! passes its bytes to [`load`].
//!
//! Fabric calls the obfuscated column `official`, which here would be easy to confuse with
//! Mojang's official (deobfuscated) names. [`load`] renames it to [`ns::OBF`], so the set's
//! namespaces are `(obf, intermediary)`.

use crate::Format;
use crate::error::{Error, Result};
use crate::model::MappingSet;
use crate::ns;
use crate::text::utf8;
use crate::{tiny1, tiny2};

/// The path of the mappings file inside an intermediary jar.
pub const JAR_ENTRY: &str = "mappings/mappings.tiny";

/// The column name Fabric uses for Mojang's obfuscated names.
pub const FABRIC_OBFUSCATED_NAMESPACE: &str = "official";

/// Loads the bytes of `mappings/mappings.tiny` from an intermediary jar (tiny v2, or tiny v1
/// for old jars) into a set with the namespaces `(obf, intermediary)`.
pub fn load(bytes: &[u8]) -> Result<MappingSet> {
    let text = utf8(bytes)?;
    let mut set = if text.starts_with("tiny\t") {
        tiny2::parse(text)?
    } else if text.starts_with("v1\t") {
        tiny1::parse(text)?
    } else {
        return Err(Error::syntax(
            Format::TinyV2,
            1,
            "not an intermediary mappings file (expected a tiny v2 or v1 header)",
        ));
    };
    if set.namespace_index(FABRIC_OBFUSCATED_NAMESPACE).is_some() {
        set.rename_namespace(FABRIC_OBFUSCATED_NAMESPACE, ns::OBF)?;
    }
    for required in [ns::OBF, ns::INTERMEDIARY] {
        set.require_namespace(required)?;
    }
    if set.primary_namespace() != ns::OBF {
        set = set.with_primary(ns::OBF)?;
    }
    Ok(set)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renames_official_to_obf() {
        let set =
            load(b"tiny\t2\t0\tofficial\tintermediary\nc\ta\tnet/minecraft/class_1\n").unwrap();
        assert_eq!(set.namespaces(), ["obf", "intermediary"]);
        let v1 = load(b"v1\tofficial\tintermediary\nCLASS\ta\tnet/minecraft/class_1\n").unwrap();
        assert_eq!(v1, set);
        let reordered =
            load(b"tiny\t2\t0\tintermediary\tofficial\nc\tnet/minecraft/class_1\ta\n").unwrap();
        assert_eq!(reordered, set);
    }

    #[test]
    fn rejects_other_files() {
        assert!(load(b"").is_err());
        assert!(load(b"a -> b:\n").is_err());
        assert!(load(b"tiny\t2\t0\tofficial\tnamed\n").is_err());
        assert!(load(&[0xff, 0xfe]).is_err());
    }
}
