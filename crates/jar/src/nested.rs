//! Nested jars (jar-in-jar).
//!
//! Loaders ship library and module jars inside a mod jar. Two directories hold them by
//! convention, and this module lists and reads what is in them. Which loader declares a
//! nested jar where (for example the `jars` list of `fabric.mod.json`, or
//! `META-INF/jarjar/metadata.json`) is metadata, read by the manifest crate; this module
//! only knows the archive layout. [`Jar::read_entry_as_jar`] reads a jar at any path.

use crate::Jar;
use crate::entry::Entry;
use crate::error::{Error, Result};
use crate::name::ends_with_ignore_case;
use crate::read::{Budget, HARD_MAX_NESTING_DEPTH, ReadOptions, read_archive};

/// A directory that holds nested jars by convention.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NestedDir {
    /// `META-INF/jars/`: where Fabric Loom puts jar-in-jar files for Fabric Loader (Quilt
    /// builds use the same directory).
    Jars,
    /// `META-INF/jarjar/`: NeoForge's (and Forge's) JarJar directory, next to its
    /// `metadata.json`.
    JarJar,
}

impl NestedDir {
    /// Every known directory.
    pub const ALL: [NestedDir; 2] = [NestedDir::Jars, NestedDir::JarJar];

    /// The directory's path prefix, with its trailing `/`.
    pub fn prefix(self) -> &'static str {
        match self {
            NestedDir::Jars => "META-INF/jars/",
            NestedDir::JarJar => "META-INF/jarjar/",
        }
    }

    /// The directory a nested jar named `name` sits in: a file ending in `.jar` (any case)
    /// anywhere below one of the prefixes (which are case-sensitive, as loaders read them).
    pub fn of(name: &str) -> Option<NestedDir> {
        if name.ends_with('/') || !ends_with_ignore_case(name, ".jar") {
            return None;
        }
        NestedDir::ALL
            .into_iter()
            .find(|dir| name.starts_with(dir.prefix()))
    }
}

/// A nested jar, read, with the jars nested in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NestedJar {
    /// The nested jar's entry name in its parent.
    pub path: String,
    /// The directory it was found in.
    pub dir: NestedDir,
    /// Its contents.
    pub jar: Jar,
    /// Jars nested in it, in entry order.
    pub nested: Vec<NestedJar>,
}

impl NestedJar {
    /// How many nested jars this tree holds, itself included.
    pub fn count(&self) -> usize {
        1 + self.nested.iter().map(NestedJar::count).sum::<usize>()
    }
}

fn nest(path: &str, error: Error) -> Error {
    Error::Nested {
        path: path.to_string(),
        source: Box::new(error),
    }
}

fn read_level(
    jar: &Jar,
    options: &ReadOptions,
    budget: &mut Budget,
    depth: u32,
) -> Result<Vec<NestedJar>> {
    let max = options.max_nesting_depth.min(HARD_MAX_NESTING_DEPTH);
    let mut out = Vec::new();
    for (dir, entry) in jar.nested_jar_entries() {
        if depth > max {
            return Err(Error::NestingTooDeep {
                path: entry.name.clone(),
                max,
            });
        }
        let inner = read_archive(&entry.data, options, budget).map_err(|e| nest(&entry.name, e))?;
        let nested =
            read_level(&inner, options, budget, depth + 1).map_err(|e| nest(&entry.name, e))?;
        out.push(NestedJar {
            path: entry.name.clone(),
            dir,
            jar: inner,
            nested,
        });
    }
    Ok(out)
}

impl Jar {
    /// The entries that are nested jars (see [`NestedDir::of`]), in entry order.
    pub fn nested_jar_entries(&self) -> Vec<(NestedDir, &Entry)> {
        self.entries
            .iter()
            .filter_map(|e| NestedDir::of(&e.name).map(|dir| (dir, e)))
            .collect()
    }

    /// Reads the entry called `name` as a jar, with `options` as its limits.
    pub fn read_entry_as_jar(&self, name: &str, options: &ReadOptions) -> Result<Jar> {
        let entry = self.get(name).ok_or_else(|| Error::NoSuchEntry {
            name: name.to_string(),
        })?;
        Jar::read_with(&entry.data, options).map_err(|e| nest(name, e))
    }

    /// Reads every nested jar (see [`Jar::nested_jar_entries`]), recursively.
    ///
    /// The entry-count and total-size limits of `options` apply to all nested jars together
    /// (this jar is already read and not counted), and nesting stops with
    /// [`Error::NestingTooDeep`] past [`ReadOptions::max_nesting_depth`] levels (this jar is
    /// level 0). An error inside a nested jar comes back as [`Error::Nested`], one level per
    /// jar, so the path to the broken jar is in the error.
    pub fn read_nested(&self, options: &ReadOptions) -> Result<Vec<NestedJar>> {
        let mut budget = Budget::new(options);
        read_level(self, options, &mut budget, 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_dir_of() {
        assert_eq!(NestedDir::of("META-INF/jars/a.jar"), Some(NestedDir::Jars));
        assert_eq!(
            NestedDir::of("META-INF/jars/sub/a.JAR"),
            Some(NestedDir::Jars)
        );
        assert_eq!(
            NestedDir::of("META-INF/jarjar/b.jar"),
            Some(NestedDir::JarJar)
        );
        assert_eq!(NestedDir::of("META-INF/jarjar/metadata.json"), None);
        assert_eq!(NestedDir::of("META-INF/jars/"), None);
        assert_eq!(NestedDir::of("META-INF/jars/x.jar/"), None);
        assert_eq!(NestedDir::of("libs/a.jar"), None);
        assert_eq!(NestedDir::of("meta-inf/jars/a.jar"), None);
    }
}
