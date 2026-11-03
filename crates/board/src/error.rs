//! The one error type of the board pipeline.

use std::fmt;
use std::path::Path;

use crate::http::FetchError;

/// Everything that can stop a board step. Malformed input is an error, never a panic.
#[derive(Debug)]
pub enum Error {
    /// Reading or writing a file failed.
    Io {
        /// What was being done, including the path.
        context: String,
        /// The underlying error.
        source: std::io::Error,
    },
    /// A JSON document could not be read or written.
    Json {
        /// What was being read, including the path or URL.
        context: String,
        /// The underlying error.
        source: serde_json::Error,
    },
    /// An HTTP request failed after its retries.
    Fetch(FetchError),
    /// The input is well formed but breaks a rule of the contract (`docs/board.md`).
    Invalid(String),
}

impl Error {
    /// An I/O error with a context line.
    pub fn io(context: impl Into<String>, source: std::io::Error) -> Self {
        Error::Io {
            context: context.into(),
            source,
        }
    }

    /// An I/O error about a path.
    pub fn io_at(action: &str, path: &Path, source: std::io::Error) -> Self {
        Error::io(format!("{action} {}", path.display()), source)
    }

    /// A JSON error with a context line.
    pub fn json(context: impl Into<String>, source: serde_json::Error) -> Self {
        Error::Json {
            context: context.into(),
            source,
        }
    }

    /// A contract violation.
    pub fn invalid(message: impl Into<String>) -> Self {
        Error::Invalid(message.into())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io { context, source } => write!(f, "{context}: {source}"),
            Error::Json { context, source } => write!(f, "{context}: {source}"),
            Error::Fetch(err) => write!(f, "{err}"),
            Error::Invalid(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io { source, .. } => Some(source),
            Error::Json { source, .. } => Some(source),
            Error::Fetch(err) => Some(err),
            Error::Invalid(_) => None,
        }
    }
}

impl From<FetchError> for Error {
    fn from(err: FetchError) -> Self {
        Error::Fetch(err)
    }
}
