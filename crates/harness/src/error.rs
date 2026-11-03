//! The harness error type: everything that stops the harness itself, as opposed to a mod
//! result (a red or not-run result is a value, not an error).

use std::fmt;
use std::path::Path;

use crate::diff::CompareError;
use crate::dump::DumpError;
use crate::fetch::DownloadError;
use crate::loader::Loader;
use crate::lock::LockError;

/// Something the harness could not do.
#[derive(Debug)]
pub enum HarnessError {
    /// Bad input from the caller (a missing file, a malformed version).
    Usage(String),
    /// `research-lock.json` lacks or garbles a pin the harness needs.
    Lock(LockError),
    /// A pinned download failed or did not match its publisher hash.
    Download(DownloadError),
    /// A file-system operation failed.
    Io {
        /// What the harness was doing.
        what: String,
        /// The error.
        error: std::io::Error,
    },
    /// No usable Java: `JAVA_HOME` unset, or the JDK is older than the pinned major.
    Java(String),
    /// A loader installer failed.
    Install(String),
    /// Provisioning for this loader is not built yet.
    NotBuiltYet {
        /// The loader.
        loader: Loader,
    },
    /// Two dumps, or a dump and a baseline, cannot be compared.
    Compare(CompareError),
    /// A dump could not be read.
    Dump {
        /// The file.
        path: String,
        /// Why.
        error: DumpError,
    },
}

impl HarnessError {
    /// An I/O error with what the harness was doing at the time.
    pub fn io(what: impl Into<String>, error: std::io::Error) -> Self {
        HarnessError::Io {
            what: what.into(),
            error,
        }
    }

    /// An I/O error on a path.
    pub fn io_at(verb: &str, path: &Path, error: std::io::Error) -> Self {
        Self::io(format!("{verb} {}", path.display()), error)
    }
}

impl fmt::Display for HarnessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HarnessError::Usage(message) => f.write_str(message),
            HarnessError::Lock(e) => write!(f, "research-lock.json: {e}"),
            HarnessError::Download(e) => write!(f, "{e}"),
            HarnessError::Io { what, error } => write!(f, "could not {what}: {error}"),
            HarnessError::Java(message) => f.write_str(message),
            HarnessError::Install(message) => f.write_str(message),
            HarnessError::NotBuiltYet { loader } => write!(
                f,
                "{loader} servers are not built yet: provisioning for {loader} will be driven by \
                 its loader profile (loaders/{loader}.toml), which this build does not read; \
                 fabric and neoforge are built"
            ),
            HarnessError::Compare(e) => write!(f, "{e}"),
            HarnessError::Dump { path, error } => write!(f, "{path}: {error}"),
        }
    }
}

impl std::error::Error for HarnessError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            HarnessError::Io { error, .. } => Some(error),
            _ => None,
        }
    }
}

impl From<LockError> for HarnessError {
    fn from(e: LockError) -> Self {
        HarnessError::Lock(e)
    }
}

impl From<DownloadError> for HarnessError {
    fn from(e: DownloadError) -> Self {
        HarnessError::Download(e)
    }
}

impl From<CompareError> for HarnessError {
    fn from(e: CompareError) -> Self {
        HarnessError::Compare(e)
    }
}
