//! Jar signatures: detection and stripping.
//!
//! A converted jar's bytes differ from the signed ones, so its signature can only fail.
//! Stripping removes the signature files and the per-entry digests they sign, and reports
//! exactly what it removed.

use crate::Jar;
use crate::error::Result;
use crate::manifest::Manifest;
use crate::name::{ends_with_ignore_case, starts_with_ignore_case};
use crate::write::manifest_index;

const META_INF: &str = "META-INF/";
const SIGNATURE_SUFFIXES: [&str; 4] = [".SF", ".RSA", ".DSA", ".EC"];

/// One digest attribute removed from a manifest section.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RemovedDigest {
    /// The section's `Name` (the entry the digest was for).
    pub section: String,
    /// The attribute name, for example `SHA-256-Digest`.
    pub attribute: String,
}

/// What [`Jar::strip_signatures`] removed, for the report.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct StrippedSignatures {
    /// Signature files removed, in archive order.
    pub files: Vec<String>,
    /// Digest attributes removed from the manifest's individual sections, in manifest order.
    pub digests: Vec<RemovedDigest>,
    /// Individual sections dropped because they held nothing but digests.
    pub sections: Vec<String>,
}

impl StrippedSignatures {
    /// Whether nothing was removed.
    pub fn is_empty(&self) -> bool {
        self.files.is_empty() && self.digests.is_empty() && self.sections.is_empty()
    }
}

/// Whether `name` is a signature file: directly in `META-INF/` (any case), ending in
/// `.SF`, `.RSA`, `.DSA` or `.EC`, or starting with `SIG-` (names the JAR specification
/// reserves for signatures). Matched ignoring ASCII case, as the JDK's verifier does.
pub fn is_signature_file(name: &str) -> bool {
    if !starts_with_ignore_case(name, META_INF) {
        return false;
    }
    let Some(rest) = name.get(META_INF.len()..) else {
        return false;
    };
    if rest.is_empty() || rest.contains('/') {
        return false;
    }
    SIGNATURE_SUFFIXES
        .iter()
        .any(|suffix| ends_with_ignore_case(rest, suffix))
        || starts_with_ignore_case(rest, "SIG-")
}

/// Whether `name` is a per-entry digest attribute (`SHA-256-Digest`, `SHA1-Digest`, ...).
fn is_digest_attribute(name: &str) -> bool {
    ends_with_ignore_case(name, "-Digest")
}

/// Removes digest attributes from `manifest`; returns whether it changed.
fn strip_digests(manifest: &mut Manifest, report: &mut StrippedSignatures) -> bool {
    let mut changed = false;
    let mut emptied = Vec::with_capacity(manifest.sections.len());
    for section in &mut manifest.sections {
        let before = section.attributes.len();
        let name = &section.name;
        section.attributes.retain(|a| {
            let digest = is_digest_attribute(&a.name);
            if digest {
                report.digests.push(RemovedDigest {
                    section: name.clone(),
                    attribute: a.name.clone(),
                });
            }
            !digest
        });
        let removed = section.attributes.len() != before;
        changed |= removed;
        emptied.push(removed && section.attributes.is_empty());
    }
    let mut index = 0;
    manifest.sections.retain(|section| {
        let drop = emptied.get(index).copied().unwrap_or(false);
        index += 1;
        if drop {
            report.sections.push(section.name.clone());
        }
        !drop
    });
    changed
}

impl Jar {
    /// Whether the jar holds any signature file (see [`is_signature_file`]).
    pub fn is_signed(&self) -> bool {
        self.entries
            .iter()
            .any(|e| !e.is_dir() && is_signature_file(&e.name))
    }

    /// Removes the signature files (`META-INF/*.SF`, `*.RSA`, `*.DSA`, `*.EC`, `SIG-*`) and
    /// the `*-Digest` attributes in the manifest's individual sections, dropping sections
    /// left empty. Other attributes and sections stay. The manifest is rewritten only when a
    /// digest was removed. Returns what was removed.
    ///
    /// Errors when the manifest cannot be parsed or rewritten; the jar is then unchanged.
    pub fn strip_signatures(&mut self) -> Result<StrippedSignatures> {
        let mut report = StrippedSignatures::default();
        let mut new_manifest = None;
        if let Some(index) = manifest_index(&self.entries) {
            let mut manifest = Manifest::parse(&self.entries[index].data)?;
            if strip_digests(&mut manifest, &mut report) {
                new_manifest = Some((index, manifest.to_bytes()?));
            }
        }
        if let Some((index, bytes)) = new_manifest {
            self.entries[index].data = bytes;
        }
        self.entries.retain(|e| {
            if !e.is_dir() && is_signature_file(&e.name) {
                report.files.push(e.name.clone());
                false
            } else {
                true
            }
        });
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_file_names() {
        for name in [
            "META-INF/MOJANG.SF",
            "META-INF/MOJANG.RSA",
            "META-INF/key.dsa",
            "META-INF/X.EC",
            "meta-inf/a.sf",
            "META-INF/SIG-ABC",
        ] {
            assert!(is_signature_file(name), "{name}");
        }
        for name in [
            "META-INF/MANIFEST.MF",
            "META-INF/sub/A.SF",
            "META-INF/",
            "A.SF",
            "META-INF/services/x.RSA",
            "META-INF/notes.SFX",
        ] {
            assert!(!is_signature_file(name), "{name}");
        }
    }

    #[test]
    fn digest_attribute_names() {
        assert!(is_digest_attribute("SHA-256-Digest"));
        assert!(is_digest_attribute("sha1-digest"));
        assert!(is_digest_attribute("MD5-Digest"));
        assert!(!is_digest_attribute("Digest-Algorithms"));
        assert!(!is_digest_attribute("Sealed"));
    }
}
