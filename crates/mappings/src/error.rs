//! The one error type of the crate.

use std::fmt;

use crate::Format;

/// Everything that can go wrong while reading, writing or combining mappings.
///
/// Malformed input is always an `Err`, never a panic. Parse errors carry the 1-based line
/// number of the offending line.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// The input is not valid UTF-8.
    NotUtf8 {
        /// Byte offset of the first invalid sequence.
        offset: usize,
    },
    /// A line of a mapping file does not follow its format.
    Syntax {
        /// The format being read.
        format: Format,
        /// 1-based line number.
        line: usize,
        /// What is wrong, in plain words.
        message: String,
    },
    /// A namespace was named that the mapping set does not have.
    UnknownNamespace(String),
    /// A namespace appears twice where each must be unique.
    DuplicateNamespace(String),
    /// A namespace list was empty, or an entry had the wrong number of names.
    NamespaceCount {
        /// How many names were expected.
        expected: usize,
        /// How many were given.
        found: usize,
    },
    /// The primary (first) namespace of a class, field or method has no name.
    MissingPrimaryName {
        /// What kind of entry, for example `class` or `method`.
        what: &'static str,
    },
    /// A class or method position does not exist in the set.
    OutOfRange {
        /// `class` or `method`.
        what: &'static str,
        /// The position that was asked for.
        index: usize,
    },
    /// Two entries with the same key disagree on a name.
    Conflict {
        /// What kind of entry, for example `class` or `field`.
        what: &'static str,
        /// The entry's key in the primary namespace (`owner.name desc` for members).
        key: String,
        /// The namespace in which the names differ.
        namespace: String,
        /// The name already recorded.
        existing: String,
        /// The other name.
        other: String,
    },
    /// A field or method descriptor, or a generic signature, is malformed.
    Descriptor {
        /// The text that failed to parse.
        input: String,
        /// Byte offset of the problem inside `input`.
        offset: usize,
        /// What is wrong.
        message: &'static str,
    },
    /// A writer cannot express this mapping set in its format.
    Unrepresentable {
        /// The target format.
        format: Format,
        /// What cannot be written, and why.
        message: String,
    },
    /// No chain of mapping sets connects the two namespaces.
    NoPath {
        /// The namespace the chain starts from.
        source: String,
        /// The namespace the chain should reach.
        target: String,
    },
}

impl Error {
    pub(crate) fn syntax(format: Format, line: usize, message: impl Into<String>) -> Self {
        Error::Syntax {
            format,
            line,
            message: message.into(),
        }
    }

    pub(crate) fn unrepresentable(format: Format, message: impl Into<String>) -> Self {
        Error::Unrepresentable {
            format,
            message: message.into(),
        }
    }

    pub(crate) fn descriptor(input: &str, offset: usize, message: &'static str) -> Self {
        Error::Descriptor {
            input: input.to_owned(),
            offset,
            message,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NotUtf8 { offset } => write!(f, "input is not UTF-8 (byte {offset})"),
            Error::Syntax {
                format,
                line,
                message,
            } => write!(f, "{format} line {line}: {message}"),
            Error::UnknownNamespace(ns) => write!(f, "unknown namespace {ns:?}"),
            Error::DuplicateNamespace(ns) => write!(f, "namespace {ns:?} appears twice"),
            Error::NamespaceCount { expected, found } => {
                write!(f, "expected {expected} names, found {found}")
            }
            Error::MissingPrimaryName { what } => {
                write!(f, "a {what} has no name in the primary namespace")
            }
            Error::OutOfRange { what, index } => write!(f, "no {what} at position {index}"),
            Error::Conflict {
                what,
                key,
                namespace,
                existing,
                other,
            } => write!(
                f,
                "{what} {key} maps to both {existing:?} and {other:?} in namespace {namespace:?}"
            ),
            Error::Descriptor {
                input,
                offset,
                message,
            } => write!(f, "bad descriptor {input:?} at byte {offset}: {message}"),
            Error::Unrepresentable { format, message } => {
                write!(f, "cannot write {format}: {message}")
            }
            Error::NoPath { source, target } => write!(
                f,
                "no chain of mapping sets connects namespace {source:?} to {target:?}"
            ),
        }
    }
}

impl std::error::Error for Error {}

/// Shorthand for results of this crate.
pub type Result<T, E = Error> = std::result::Result<T, E>;
