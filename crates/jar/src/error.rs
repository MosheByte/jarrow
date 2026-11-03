//! The one error type of this crate.

use core::fmt;

/// Result alias for this crate.
pub type Result<T, E = Error> = core::result::Result<T, E>;

/// Why an entry name was refused.
///
/// Names are checked when an archive is read and again when one is written. A refused name
/// is an error, never a silently dropped entry.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnsafeName {
    /// The name is empty.
    Empty,
    /// The name starts with `/`.
    Absolute,
    /// The name starts with a drive letter such as `C:`.
    DriveLetter,
    /// The name contains a backslash, which Windows treats as a separator.
    Backslash,
    /// The name contains a NUL byte.
    Nul,
    /// A path component is `..`.
    ParentComponent,
    /// A path component is `.`, or consists only of dots (other than `..`).
    DotComponent,
    /// A path component is empty (`a//b`), other than the trailing `/` of a directory.
    EmptyComponent,
}

impl UnsafeName {
    /// A short English description, used in error messages.
    pub fn describe(self) -> &'static str {
        match self {
            UnsafeName::Empty => "the name is empty",
            UnsafeName::Absolute => "the name is an absolute path",
            UnsafeName::DriveLetter => "the name starts with a drive letter",
            UnsafeName::Backslash => "the name contains a backslash",
            UnsafeName::Nul => "the name contains a NUL byte",
            UnsafeName::ParentComponent => "the name contains a '..' component",
            UnsafeName::DotComponent => "the name contains a component made only of dots",
            UnsafeName::EmptyComponent => "the name contains an empty component",
        }
    }
}

/// Which configurable limit was exceeded (see [`crate::ReadOptions`]).
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Limit {
    /// [`crate::ReadOptions::max_entries`].
    Entries,
    /// [`crate::ReadOptions::max_entry_size`].
    EntrySize,
    /// [`crate::ReadOptions::max_total_size`].
    TotalSize,
}

impl Limit {
    fn describe(self) -> &'static str {
        match self {
            Limit::Entries => "entry count",
            Limit::EntrySize => "uncompressed entry size",
            Limit::TotalSize => "total uncompressed size",
        }
    }
}

/// Everything that can go wrong when reading or writing a JAR.
///
/// Malformed input always ends here; this crate never panics on bad bytes.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// No end-of-central-directory record: the input is not a zip archive.
    NotAZip,
    /// The archive structure is broken. `what` names the part.
    Malformed {
        /// What is broken, in English.
        what: String,
    },
    /// The archive spans several disks (split archives are not JARs).
    MultiDisk,
    /// An entry name could escape or alias the directory it is extracted to.
    UnsafeName {
        /// The refused name.
        name: String,
        /// Why it was refused.
        reason: UnsafeName,
    },
    /// An entry name is not valid UTF-8 (the JVM decodes entry names as UTF-8).
    NameNotUtf8 {
        /// Position of the entry in the central directory, from 0.
        index: usize,
    },
    /// Two entries have the same name, so tools disagree on which one is meant.
    DuplicateName {
        /// The repeated name.
        name: String,
    },
    /// The entry is encrypted.
    Encrypted {
        /// The entry name.
        name: String,
    },
    /// The entry uses a compression method other than stored (0) or deflated (8).
    UnsupportedMethod {
        /// The entry name.
        name: String,
        /// The method id from the archive.
        method: u16,
    },
    /// A configured limit was exceeded before anything was decompressed.
    LimitExceeded {
        /// Which limit.
        limit: Limit,
        /// The value that was asked for (declared by the archive, or summed so far).
        requested: u64,
        /// The configured maximum.
        max: u64,
    },
    /// The data of an entry does not have the size the central directory declares.
    SizeMismatch {
        /// The entry name.
        name: String,
        /// Size declared in the central directory.
        declared: u64,
        /// Size found (for decompressed data, capped at `declared + 1`).
        actual: u64,
    },
    /// The CRC-32 of an entry's data differs from the one the archive declares.
    CrcMismatch {
        /// The entry name.
        name: String,
        /// CRC-32 declared in the central directory.
        declared: u32,
        /// CRC-32 of the decompressed data.
        actual: u32,
    },
    /// The deflate stream of an entry is invalid or truncated.
    Inflate {
        /// The entry name.
        name: String,
    },
    /// Two entries' bytes overlap in the archive, the shape of an overlapping-file zip bomb.
    OverlappingEntries {
        /// The entry whose bytes start first.
        first: String,
        /// The entry that starts inside it.
        second: String,
    },
    /// An entry's local header disagrees with its central-directory record.
    LocalHeaderMismatch {
        /// The entry name from the central directory.
        name: String,
        /// Which field differs.
        field: &'static str,
    },
    /// A directory entry (name ending in `/`) carries data.
    DirectoryWithData {
        /// The entry name.
        name: String,
    },
    /// A write option is out of range.
    InvalidOption {
        /// Which option, and the allowed range.
        what: &'static str,
    },
    /// `META-INF/MANIFEST.MF` could not be parsed.
    Manifest {
        /// 1-based line number of the problem.
        line: usize,
        /// What is wrong, in English.
        what: String,
    },
    /// A manifest could not be written because a name or value cannot be represented.
    ManifestValue {
        /// What is wrong, in English.
        what: String,
    },
    /// Reading a nested jar failed.
    Nested {
        /// The nested jar's entry name in its parent.
        path: String,
        /// The error inside it.
        source: Box<Error>,
    },
    /// Nested jars go deeper than [`crate::ReadOptions::max_nesting_depth`].
    NestingTooDeep {
        /// The nested jar that would exceed the depth.
        path: String,
        /// The configured maximum depth.
        max: u32,
    },
    /// The entry asked for is not in the jar.
    NoSuchEntry {
        /// The name asked for.
        name: String,
    },
}

impl Error {
    pub(crate) fn malformed(what: impl Into<String>) -> Error {
        Error::Malformed { what: what.into() }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NotAZip => f.write_str("not a zip archive: no end-of-central-directory record"),
            Error::Malformed { what } => write!(f, "malformed archive: {what}"),
            Error::MultiDisk => f.write_str("multi-disk (split) archives are not supported"),
            Error::UnsafeName { name, reason } => {
                write!(f, "unsafe entry name {name:?}: {}", reason.describe())
            }
            Error::NameNotUtf8 { index } => {
                write!(f, "the name of entry #{index} is not valid UTF-8")
            }
            Error::DuplicateName { name } => write!(f, "duplicate entry name {name:?}"),
            Error::Encrypted { name } => write!(f, "entry {name:?} is encrypted"),
            Error::UnsupportedMethod { name, method } => {
                write!(
                    f,
                    "entry {name:?} uses unsupported compression method {method}"
                )
            }
            Error::LimitExceeded {
                limit,
                requested,
                max,
            } => write!(
                f,
                "{} limit exceeded: {requested} requested, at most {max} allowed",
                limit.describe()
            ),
            Error::SizeMismatch {
                name,
                declared,
                actual,
            } => write!(
                f,
                "entry {name:?} declares {declared} bytes but holds {actual}{}",
                if actual > declared { " or more" } else { "" }
            ),
            Error::CrcMismatch {
                name,
                declared,
                actual,
            } => write!(
                f,
                "entry {name:?} declares CRC-32 {declared:08x} but its data has {actual:08x}"
            ),
            Error::Inflate { name } => write!(f, "entry {name:?} has an invalid deflate stream"),
            Error::OverlappingEntries { first, second } => write!(
                f,
                "entry {second:?} starts inside the bytes of entry {first:?} (overlapping entries)"
            ),
            Error::LocalHeaderMismatch { name, field } => write!(
                f,
                "entry {name:?}: local header {field} differs from the central directory"
            ),
            Error::DirectoryWithData { name } => {
                write!(f, "directory entry {name:?} carries data")
            }
            Error::InvalidOption { what } => write!(f, "invalid option: {what}"),
            Error::Manifest { line, what } => write!(f, "manifest line {line}: {what}"),
            Error::ManifestValue { what } => write!(f, "cannot write manifest: {what}"),
            Error::Nested { path, source } => write!(f, "nested jar {path:?}: {source}"),
            Error::NestingTooDeep { path, max } => {
                write!(f, "nested jar {path:?} is nested deeper than {max} levels")
            }
            Error::NoSuchEntry { name } => write!(f, "no entry named {name:?}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Nested { source, .. } => Some(source.as_ref()),
            _ => None,
        }
    }
}
