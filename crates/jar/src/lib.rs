//! Reading and writing JARs for the Jarrow engine.
//!
//! - [`Jar::read`] parses a zip archive from bytes. Entries come back in central-directory
//!   order, decompressed, with how they looked in the input ([`Original`]: method, CRC,
//!   sizes, extra-field presence, Unix mode). Zip64 is supported. Unsafe names (`..`,
//!   absolute paths, drive letters, backslashes and the like), duplicates, encryption,
//!   overlapping entries, size lies and CRC mismatches are errors, never silently dropped
//!   entries; [`ReadOptions`] bounds entry count and decompressed sizes (zip-bomb guards).
//! - [`Jar::write`] is deterministic: the same entries and [`WriteOptions`] always give the
//!   same bytes (fixed DOS timestamp 1980-01-01 00:00:00, entries in the order given with
//!   `META-INF/MANIFEST.MF` first, a fixed deflate level, no extra fields).
//! - [`Jar::strip_signatures`] removes jar signatures and reports what it removed.
//! - [`Manifest`] parses and writes `META-INF/MANIFEST.MF` (72-byte lines, continuations,
//!   main and individual sections).
//! - [`Jar::read_nested`] reads jar-in-jar files under `META-INF/jars/` and
//!   `META-INF/jarjar/`, recursively, within the same limits.
//!
//! The crate builds for `wasm32-unknown-unknown`: it does no I/O, uses no threads and
//! depends only on `miniz_oxide` (pure-Rust DEFLATE). [`Jar::read`] never panics on any
//! input, which makes it the fuzzing entry point.
//!
//! ```
//! use jarrow_jar::{Entry, Jar, WriteOptions};
//!
//! let mut jar = Jar::new();
//! jar.entries.push(Entry::file("net/example/Hello.txt", b"hello".to_vec()));
//! let bytes = jar.write(&WriteOptions::default()).unwrap();
//! assert_eq!(bytes, jar.write(&WriteOptions::default()).unwrap());
//!
//! let back = Jar::read(&bytes).unwrap();
//! assert_eq!(back.get("net/example/Hello.txt").unwrap().data, b"hello");
//! ```

#![forbid(unsafe_code)]

mod crc;
mod entry;
mod error;
mod manifest;
mod name;
mod nested;
mod read;
mod signature;
mod write;

pub use crc::crc32;
pub use entry::{Entry, Method, Original};
pub use error::{Error, Limit, Result, UnsafeName};
pub use manifest::{
    Attribute, MANIFEST_PATH, MAX_LINE_BYTES, Manifest, Section, is_valid_attribute_name,
};
pub use name::check_name;
pub use nested::{NestedDir, NestedJar};
pub use read::{HARD_MAX_NESTING_DEPTH, ReadOptions};
pub use signature::{RemovedDigest, StrippedSignatures, is_signature_file};
pub use write::{Compression, WriteOptions};

/// A JAR (or any zip archive) in memory: its entries, in order.
///
/// `entries` is public so callers can add, reorder and edit entries freely; the writer
/// checks names, duplicates and directories again before it writes anything.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Jar {
    /// Files and directories, in archive order.
    pub entries: Vec<Entry>,
}

impl Jar {
    /// An empty jar.
    pub fn new() -> Jar {
        Jar::default()
    }

    /// A jar holding `entries`, in that order.
    pub fn from_entries(entries: Vec<Entry>) -> Jar {
        Jar { entries }
    }

    /// Reads a zip archive with the default [`ReadOptions`].
    ///
    /// Never panics: malformed or hostile input returns an [`Error`].
    pub fn read(bytes: &[u8]) -> Result<Jar> {
        Jar::read_with(bytes, &ReadOptions::default())
    }

    /// Reads a zip archive with explicit limits.
    pub fn read_with(bytes: &[u8], options: &ReadOptions) -> Result<Jar> {
        let mut budget = read::Budget::new(options);
        read::read_archive(bytes, options, &mut budget)
    }

    /// Writes the jar deterministically (see [`WriteOptions`] and the crate docs).
    ///
    /// Errors on unsafe or duplicate names, a directory with data, or an invalid option.
    pub fn write(&self, options: &WriteOptions) -> Result<Vec<u8>> {
        write::write_archive(self, options, write::STANDARD)
    }

    /// The entry called `name`, if any.
    pub fn get(&self, name: &str) -> Option<&Entry> {
        self.entries.iter().find(|e| e.name == name)
    }

    /// The entry called `name`, mutably.
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Entry> {
        self.entries.iter_mut().find(|e| e.name == name)
    }

    /// Adds `entry`, replacing an entry of the same name in place (returned) or appending.
    pub fn put(&mut self, entry: Entry) -> Option<Entry> {
        match self.get_mut(&entry.name) {
            Some(slot) => Some(core::mem::replace(slot, entry)),
            None => {
                self.entries.push(entry);
                None
            }
        }
    }

    /// Removes and returns the entry called `name`.
    pub fn remove(&mut self, name: &str) -> Option<Entry> {
        let index = self.entries.iter().position(|e| e.name == name)?;
        Some(self.entries.remove(index))
    }

    /// Entry names, in order.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.entries.iter().map(|e| e.name.as_str())
    }

    /// The parsed manifest, or `None` when the jar has none. The manifest is
    /// `META-INF/MANIFEST.MF`, or the first entry matching that name ignoring ASCII case.
    pub fn manifest(&self) -> Result<Option<Manifest>> {
        match write::manifest_index(&self.entries) {
            Some(index) => Manifest::parse(&self.entries[index].data).map(Some),
            None => Ok(None),
        }
    }

    /// Writes `manifest` into the existing manifest entry, or adds `META-INF/MANIFEST.MF`
    /// as the first entry.
    pub fn set_manifest(&mut self, manifest: &Manifest) -> Result<()> {
        let bytes = manifest.to_bytes()?;
        match write::manifest_index(&self.entries) {
            Some(index) => self.entries[index].data = bytes,
            None => self.entries.insert(0, Entry::file(MANIFEST_PATH, bytes)),
        }
        Ok(())
    }
}
