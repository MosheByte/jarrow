//! Downloads with publisher-hash verification.
//!
//! Every file the harness provisions is named in `research-lock.json` with the hashes its
//! publisher gave (Mojang's version JSON, the Fabric and NeoForge maven sidecars, the Modrinth
//! API). [`download_verified`] streams a file into the cache, hashes it on the way, checks the
//! size and every pinned hash, and only then moves it into place. A cached file is re-hashed
//! before it is reused. A mismatch deletes the bytes and is an error; the harness never runs a
//! file whose hash it could not confirm.
//!
//! The network sits behind the [`Fetcher`] trait, so tests use an in-memory fetcher or a local
//! HTTP server and need no network.

use std::fmt;
use std::fs::{self, File};
use std::io::{self, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sha1::Sha1;
use sha2::{Digest, Sha256, Sha512};

/// Downloads without a pinned size stop at this many bytes.
pub const DEFAULT_LIMIT: u64 = 1 << 30;

/// The user agent the harness sends (the one `research-lock.json` names for research).
pub const USER_AGENT: &str = concat!(
    "MosheByte/jarrow-harness/",
    env!("CARGO_PKG_VERSION"),
    " (github.com/MosheByte/jarrow)"
);

/// A hash algorithm a publisher pins.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HashAlgorithm {
    /// SHA-1 (Mojang, maven sidecars).
    Sha1,
    /// SHA-256.
    Sha256,
    /// SHA-512 (Modrinth, maven sidecars).
    Sha512,
}

impl HashAlgorithm {
    /// Every algorithm, weakest first.
    pub const ALL: [HashAlgorithm; 3] = [
        HashAlgorithm::Sha1,
        HashAlgorithm::Sha256,
        HashAlgorithm::Sha512,
    ];

    /// Lower-case name, as in `research-lock.json`.
    pub const fn id(self) -> &'static str {
        match self {
            HashAlgorithm::Sha1 => "sha1",
            HashAlgorithm::Sha256 => "sha256",
            HashAlgorithm::Sha512 => "sha512",
        }
    }

    /// Length of the hex digest.
    pub const fn hex_len(self) -> usize {
        match self {
            HashAlgorithm::Sha1 => 40,
            HashAlgorithm::Sha256 => 64,
            HashAlgorithm::Sha512 => 128,
        }
    }
}

impl fmt::Display for HashAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id())
    }
}

/// A pinned hash.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExpectedHash {
    /// Algorithm.
    pub algorithm: HashAlgorithm,
    /// Lower-case hex digest.
    pub hex: String,
}

/// A file to download, as pinned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Artifact {
    /// What it is, for records and errors (`minecraft-server`, `fabric-api`, ...).
    pub role: String,
    /// HTTPS URL.
    pub url: String,
    /// File name to store it under.
    pub file_name: String,
    /// Pinned size in bytes, when the publisher gives one.
    pub size: Option<u64>,
    /// Every pinned hash; at least one.
    pub hashes: Vec<ExpectedHash>,
    /// Where the hashes come from (`mojang-version-json`, `maven-sidecar`, `observed`, ...).
    pub hash_source: String,
}

impl Artifact {
    /// The cache directory name for this artifact: its strongest pinned hash, so a pin change
    /// never reuses an old file.
    ///
    /// Only the first 16 hex digits name the directory, which keeps cache paths well under the
    /// 260-character limit Java still hits on Windows; the file in it is always checked against
    /// the full pins before use.
    pub fn cache_key(&self) -> String {
        self.hashes
            .iter()
            .max_by_key(|h| h.algorithm)
            .map(|h| format!("{}-{}", h.algorithm, h.hex.get(..16).unwrap_or(&h.hex)))
            .unwrap_or_else(|| "unpinned".to_owned())
    }
}

/// A file on disk whose hashes were checked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifiedFile {
    /// What it is.
    pub role: String,
    /// Where it is.
    pub path: PathBuf,
    /// Where it came from.
    pub url: String,
    /// Size in bytes.
    pub size: u64,
    /// SHA-256 (always computed, whatever was pinned).
    pub sha256: String,
    /// The pinned algorithms that matched.
    pub verified: Vec<HashAlgorithm>,
    /// Where the pinned hashes come from.
    pub hash_source: String,
}

/// The network boundary.
pub trait Fetcher {
    /// Streams the body of `url` into `sink`, at most `limit` bytes, and returns the number of
    /// bytes written. A non-success HTTP status is an error.
    fn fetch(&self, url: &str, sink: &mut dyn Write, limit: u64) -> Result<u64, FetchError>;
}

/// A failed fetch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchError {
    /// The URL.
    pub url: String,
    /// What went wrong.
    pub message: String,
}

impl fmt::Display for FetchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "download of {} failed: {}", self.url, self.message)
    }
}

impl std::error::Error for FetchError {}

/// Why a verified download failed.
#[derive(Debug)]
pub enum DownloadError {
    /// The fetch itself failed.
    Fetch(FetchError),
    /// The bytes did not match a pinned hash. They were deleted.
    Mismatch {
        /// URL.
        url: String,
        /// Algorithm that failed.
        algorithm: HashAlgorithm,
        /// Pinned digest.
        expected: String,
        /// Digest of what arrived.
        actual: String,
    },
    /// The size did not match the pin. The bytes were deleted.
    Size {
        /// URL.
        url: String,
        /// Pinned size.
        expected: u64,
        /// Size of what arrived.
        actual: u64,
    },
    /// Writing the cache failed.
    Io {
        /// Path.
        path: PathBuf,
        /// Error.
        error: io::Error,
    },
}

impl fmt::Display for DownloadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DownloadError::Fetch(e) => write!(f, "{e}"),
            DownloadError::Mismatch {
                url,
                algorithm,
                expected,
                actual,
            } => write!(
                f,
                "{url} does not match its pinned {algorithm}: expected {expected}, got {actual}; \
                 the download was deleted"
            ),
            DownloadError::Size {
                url,
                expected,
                actual,
            } => write!(
                f,
                "{url} is {actual} bytes, the pin says {expected}; the download was deleted"
            ),
            DownloadError::Io { path, error } => {
                write!(f, "could not write {}: {error}", path.display())
            }
        }
    }
}

impl std::error::Error for DownloadError {}

/// Hex digests and size of a byte stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Digests {
    /// SHA-1.
    pub sha1: String,
    /// SHA-256.
    pub sha256: String,
    /// SHA-512.
    pub sha512: String,
    /// Bytes.
    pub size: u64,
}

impl Digests {
    /// The digest for `algorithm`.
    pub fn get(&self, algorithm: HashAlgorithm) -> &str {
        match algorithm {
            HashAlgorithm::Sha1 => &self.sha1,
            HashAlgorithm::Sha256 => &self.sha256,
            HashAlgorithm::Sha512 => &self.sha512,
        }
    }
}

/// A writer that hashes what passes through it.
struct HashingWriter<W> {
    inner: W,
    sha1: Sha1,
    sha256: Sha256,
    sha512: Sha512,
    size: u64,
}

impl<W: Write> HashingWriter<W> {
    fn new(inner: W) -> Self {
        Self {
            inner,
            sha1: Sha1::new(),
            sha256: Sha256::new(),
            sha512: Sha512::new(),
            size: 0,
        }
    }

    fn finish(self) -> (W, Digests) {
        let digests = Digests {
            sha1: hex(&self.sha1.finalize()),
            sha256: hex(&self.sha256.finalize()),
            sha512: hex(&self.sha512.finalize()),
            size: self.size,
        };
        (self.inner, digests)
    }
}

impl<W: Write> Write for HashingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = self.inner.write(buf)?;
        let written = &buf[..n];
        self.sha1.update(written);
        self.sha256.update(written);
        self.sha512.update(written);
        self.size += n as u64;
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(char::from(DIGITS[usize::from(b >> 4)]));
        out.push(char::from(DIGITS[usize::from(b & 0x0f)]));
    }
    out
}

/// Hashes a file.
pub fn digest_file(path: &Path) -> io::Result<Digests> {
    let mut reader = BufReader::new(File::open(path)?);
    let mut writer = HashingWriter::new(io::sink());
    io::copy(&mut reader, &mut writer)?;
    Ok(writer.finish().1)
}

/// Hashes bytes.
pub fn digest_bytes(bytes: &[u8]) -> Digests {
    let mut writer = HashingWriter::new(io::sink());
    // Writing to a sink cannot fail.
    let _ = writer.write_all(bytes);
    writer.finish().1
}

/// SHA-256 of bytes, as lower-case hex.
pub fn sha256_hex(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

/// Checks digests against an artifact's pins: the size first, then every hash.
fn check(artifact: &Artifact, digests: &Digests) -> Result<Vec<HashAlgorithm>, DownloadError> {
    if let Some(expected) = artifact.size
        && expected != digests.size
    {
        return Err(DownloadError::Size {
            url: artifact.url.clone(),
            expected,
            actual: digests.size,
        });
    }
    let mut verified = Vec::new();
    for pin in &artifact.hashes {
        let actual = digests.get(pin.algorithm);
        if !actual.eq_ignore_ascii_case(&pin.hex) {
            return Err(DownloadError::Mismatch {
                url: artifact.url.clone(),
                algorithm: pin.algorithm,
                expected: pin.hex.clone(),
                actual: actual.to_owned(),
            });
        }
        verified.push(pin.algorithm);
    }
    verified.sort();
    verified.dedup();
    Ok(verified)
}

fn verified_file(
    artifact: &Artifact,
    dest: &Path,
    digests: &Digests,
    verified: Vec<HashAlgorithm>,
) -> VerifiedFile {
    VerifiedFile {
        role: artifact.role.clone(),
        path: dest.to_path_buf(),
        url: artifact.url.clone(),
        size: digests.size,
        sha256: digests.sha256.clone(),
        verified,
        hash_source: artifact.hash_source.clone(),
    }
}

/// Downloads `artifact` to `dest`, or reuses `dest` when it is already there with the pinned
/// hashes. See the module docs.
pub fn download_verified(
    fetcher: &dyn Fetcher,
    artifact: &Artifact,
    dest: &Path,
) -> Result<VerifiedFile, DownloadError> {
    let io_err = |path: &Path, error: io::Error| DownloadError::Io {
        path: path.to_path_buf(),
        error,
    };
    if artifact.hashes.is_empty() {
        return Err(DownloadError::Fetch(FetchError {
            url: artifact.url.clone(),
            message: "the pin has no hash, so the file cannot be verified".to_owned(),
        }));
    }
    if dest.is_file() {
        let digests = digest_file(dest).map_err(|e| io_err(dest, e))?;
        if let Ok(verified) = check(artifact, &digests) {
            return Ok(verified_file(artifact, dest, &digests, verified));
        }
        // A cached file that no longer matches is replaced, never used.
        fs::remove_file(dest).map_err(|e| io_err(dest, e))?;
    }
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| io_err(parent, e))?;
    }
    let mut part = dest.as_os_str().to_owned();
    part.push(".part");
    let part = PathBuf::from(part);
    let file = File::create(&part).map_err(|e| io_err(&part, e))?;
    let mut writer = HashingWriter::new(io::BufWriter::new(file));
    let limit = artifact.size.map_or(DEFAULT_LIMIT, |s| s.saturating_add(1));
    let fetched = fetcher.fetch(&artifact.url, &mut writer, limit);
    let flushed = writer.flush();
    let (_, digests) = writer.finish();
    let outcome = fetched
        .map_err(DownloadError::Fetch)
        .and_then(|_| flushed.map_err(|e| io_err(&part, e)))
        .and_then(|()| check(artifact, &digests));
    match outcome {
        Ok(verified) => {
            fs::rename(&part, dest).map_err(|e| io_err(dest, e))?;
            Ok(verified_file(artifact, dest, &digests, verified))
        }
        Err(e) => {
            // Best effort: the error that matters is the one being returned.
            let _ = fs::remove_file(&part);
            Err(e)
        }
    }
}

/// Whether `url` may be fetched: HTTPS anywhere, plain HTTP only to the loopback interface
/// (local test servers). Hash pins protect the content either way; this keeps the transport
/// private too.
pub fn url_allowed(url: &str) -> bool {
    if url.starts_with("https://") {
        return true;
    }
    let Some(rest) = url.strip_prefix("http://") else {
        return false;
    };
    let host = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let host = match host.rsplit_once(':') {
        Some((h, port)) if !h.is_empty() && port.bytes().all(|b| b.is_ascii_digit()) => h,
        _ => host,
    };
    matches!(host, "127.0.0.1" | "localhost" | "[::1]")
}

/// The real fetcher: HTTPS through `ureq` and `rustls`, trusting the operating system's root
/// certificates.
pub struct HttpFetcher {
    agent: ureq::Agent,
}

impl HttpFetcher {
    /// Builds a fetcher. Fails when the platform has no root certificates to trust.
    pub fn new() -> Result<Self, FetchError> {
        let loaded = rustls_native_certs::load_native_certs();
        let roots: Vec<ureq::tls::Certificate<'static>> = loaded
            .certs
            .iter()
            .map(|der| ureq::tls::Certificate::from_der(der.as_ref()).to_owned())
            .collect();
        if roots.is_empty() {
            return Err(FetchError {
                url: String::new(),
                message: format!(
                    "no root certificates found in the operating system store ({} errors)",
                    loaded.errors.len()
                ),
            });
        }
        let tls = ureq::tls::TlsConfig::builder()
            .provider(ureq::tls::TlsProvider::Rustls)
            .root_certs(ureq::tls::RootCerts::new_with_certs(&roots))
            .unversioned_rustls_crypto_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .build();
        let agent = ureq::Agent::config_builder()
            .tls_config(tls)
            .user_agent(USER_AGENT)
            .timeout_connect(Some(Duration::from_secs(30)))
            .timeout_recv_response(Some(Duration::from_secs(60)))
            .timeout_per_read(Some(Duration::from_secs(60)))
            .timeout_recv_body(Some(Duration::from_secs(15 * 60)))
            .build()
            .new_agent();
        Ok(Self { agent })
    }
}

impl Fetcher for HttpFetcher {
    fn fetch(&self, url: &str, sink: &mut dyn Write, limit: u64) -> Result<u64, FetchError> {
        let fail = |message: String| FetchError {
            url: url.to_owned(),
            message,
        };
        if !url_allowed(url) {
            return Err(fail(
                "only https URLs are fetched (plain http only to loopback)".to_owned(),
            ));
        }
        let mut response = self
            .agent
            .get(url)
            .call()
            .map_err(|e| fail(e.to_string()))?;
        let mut reader = response.body_mut().with_config().limit(limit).reader();
        let mut buf = vec![0u8; 64 * 1024];
        let mut total = 0u64;
        loop {
            let n = reader.read(&mut buf).map_err(|e| fail(e.to_string()))?;
            if n == 0 {
                break;
            }
            sink.write_all(&buf[..n])
                .map_err(|e| fail(format!("writing the download: {e}")))?;
            total += n as u64;
        }
        Ok(total)
    }
}

/// An [`HttpFetcher`] built on first use, so commands that end before any download (a closed
/// EULA gate, a cached install) never touch the certificate store.
#[derive(Default)]
pub struct LazyHttpFetcher {
    inner: std::sync::OnceLock<Result<HttpFetcher, FetchError>>,
}

impl Fetcher for LazyHttpFetcher {
    fn fetch(&self, url: &str, sink: &mut dyn Write, limit: u64) -> Result<u64, FetchError> {
        match self.inner.get_or_init(HttpFetcher::new) {
            Ok(fetcher) => fetcher.fetch(url, sink, limit),
            Err(e) => Err(FetchError {
                url: url.to_owned(),
                message: e.message.clone(),
            }),
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::BTreeMap;

    use crate::testutil::TempDir;

    /// An in-memory fetcher that counts calls.
    #[derive(Default)]
    pub(crate) struct MemoryFetcher {
        pub(crate) files: BTreeMap<String, Vec<u8>>,
        pub(crate) calls: RefCell<Vec<String>>,
    }

    impl MemoryFetcher {
        pub(crate) fn with(files: &[(&str, &[u8])]) -> Self {
            Self {
                files: files
                    .iter()
                    .map(|(u, b)| ((*u).to_owned(), b.to_vec()))
                    .collect(),
                calls: RefCell::new(Vec::new()),
            }
        }
    }

    impl Fetcher for MemoryFetcher {
        fn fetch(&self, url: &str, sink: &mut dyn Write, limit: u64) -> Result<u64, FetchError> {
            self.calls.borrow_mut().push(url.to_owned());
            let bytes = self.files.get(url).ok_or_else(|| FetchError {
                url: url.to_owned(),
                message: "404 Not Found".to_owned(),
            })?;
            let take = bytes
                .len()
                .min(usize::try_from(limit).unwrap_or(usize::MAX));
            sink.write_all(&bytes[..take]).map_err(|e| FetchError {
                url: url.to_owned(),
                message: e.to_string(),
            })?;
            Ok(take as u64)
        }
    }

    pub(crate) fn pinned(
        role: &str,
        url: &str,
        bytes: &[u8],
        algorithms: &[HashAlgorithm],
    ) -> Artifact {
        let mut writer = HashingWriter::new(io::sink());
        writer.write_all(bytes).unwrap();
        let digests = writer.finish().1;
        Artifact {
            role: role.to_owned(),
            url: url.to_owned(),
            file_name: url.rsplit('/').next().unwrap().to_owned(),
            size: Some(bytes.len() as u64),
            hashes: algorithms
                .iter()
                .map(|a| ExpectedHash {
                    algorithm: *a,
                    hex: digests.get(*a).to_owned(),
                })
                .collect(),
            hash_source: "test".to_owned(),
        }
    }

    #[test]
    fn known_digests() {
        let mut writer = HashingWriter::new(io::sink());
        writer.write_all(b"abc").unwrap();
        let d = writer.finish().1;
        assert_eq!(d.sha1, "a9993e364706816aba3e25717850c26c9cd0d89d");
        assert_eq!(
            d.sha256,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert!(d.sha512.starts_with("ddaf35a193617aba"));
        assert_eq!(d.size, 3);
        assert_eq!(sha256_hex(b"abc"), d.sha256);
    }

    #[test]
    fn a_matching_download_lands_and_is_reused_from_the_cache() {
        let tmp = TempDir::new("fetch-ok");
        let url = "https://example.invalid/server.jar";
        let fetcher = MemoryFetcher::with(&[(url, b"server bytes")]);
        let artifact = pinned(
            "minecraft-server",
            url,
            b"server bytes",
            &[HashAlgorithm::Sha1],
        );
        let dest = tmp.path().join("cache/server.jar");
        let file = download_verified(&fetcher, &artifact, &dest).unwrap();
        assert_eq!(fs::read(&dest).unwrap(), b"server bytes");
        assert_eq!(file.verified, [HashAlgorithm::Sha1]);
        assert_eq!(file.sha256, sha256_hex(b"server bytes"));
        assert!(!tmp.path().join("cache/server.jar.part").exists());
        download_verified(&fetcher, &artifact, &dest).unwrap();
        assert_eq!(
            fetcher.calls.borrow().len(),
            1,
            "the second call is a cache hit"
        );
    }

    #[test]
    fn every_pinned_hash_is_checked_and_a_mismatch_leaves_nothing() {
        let tmp = TempDir::new("fetch-bad");
        let url = "https://example.invalid/api.jar";
        let fetcher = MemoryFetcher::with(&[(url, b"tampered")]);
        let mut artifact = pinned(
            "fabric-api",
            url,
            b"tampered",
            &[HashAlgorithm::Sha1, HashAlgorithm::Sha512],
        );
        artifact.hashes[1].hex = "0".repeat(128);
        let dest = tmp.path().join("api.jar");
        let err = download_verified(&fetcher, &artifact, &dest).unwrap_err();
        match &err {
            DownloadError::Mismatch {
                algorithm,
                expected,
                ..
            } => {
                assert_eq!(*algorithm, HashAlgorithm::Sha512);
                assert_eq!(expected, &"0".repeat(128));
            }
            other => panic!("unexpected {other:?}"),
        }
        assert!(err.to_string().contains("deleted"));
        assert!(!dest.exists());
        assert!(!tmp.path().join("api.jar.part").exists());
    }

    #[test]
    fn a_size_mismatch_is_refused() {
        let tmp = TempDir::new("fetch-size");
        let url = "https://example.invalid/a.jar";
        let fetcher = MemoryFetcher::with(&[(url, b"123456")]);
        let mut artifact = pinned("a", url, b"123456", &[HashAlgorithm::Sha256]);
        artifact.size = Some(5);
        let err = download_verified(&fetcher, &artifact, &tmp.path().join("a.jar")).unwrap_err();
        assert!(
            matches!(
                err,
                DownloadError::Size {
                    expected: 5,
                    actual: 6,
                    ..
                }
            ),
            "{err:?}"
        );
    }

    #[test]
    fn a_corrupted_cache_entry_is_replaced() {
        let tmp = TempDir::new("fetch-corrupt");
        let url = "https://example.invalid/b.jar";
        let fetcher = MemoryFetcher::with(&[(url, b"good")]);
        let artifact = pinned("b", url, b"good", &[HashAlgorithm::Sha1]);
        let dest = tmp.path().join("b.jar");
        fs::write(&dest, b"evil").unwrap();
        download_verified(&fetcher, &artifact, &dest).unwrap();
        assert_eq!(fs::read(&dest).unwrap(), b"good");
        assert_eq!(fetcher.calls.borrow().len(), 1);
    }

    #[test]
    fn fetch_errors_and_unpinned_artifacts_are_errors() {
        let tmp = TempDir::new("fetch-404");
        let fetcher = MemoryFetcher::default();
        let artifact = pinned(
            "c",
            "https://example.invalid/c.jar",
            b"x",
            &[HashAlgorithm::Sha1],
        );
        assert!(matches!(
            download_verified(&fetcher, &artifact, &tmp.path().join("c.jar")),
            Err(DownloadError::Fetch(_))
        ));
        let mut unpinned = artifact;
        unpinned.hashes.clear();
        let err = download_verified(&fetcher, &unpinned, &tmp.path().join("c.jar")).unwrap_err();
        assert!(err.to_string().contains("no hash"));
        assert_eq!(
            fetcher.calls.borrow().len(),
            1,
            "unpinned files are never fetched"
        );
    }

    #[test]
    fn only_https_or_loopback_http_is_allowed() {
        assert!(url_allowed(
            "https://piston-data.mojang.com/v1/objects/x/server.jar"
        ));
        assert!(url_allowed("http://127.0.0.1:8080/file"));
        assert!(url_allowed("http://localhost/file"));
        assert!(!url_allowed("http://maven.fabricmc.net/file"));
        assert!(!url_allowed("http://127.0.0.1.evil.example/file"));
        assert!(!url_allowed("ftp://127.0.0.1/file"));
        assert!(!url_allowed("file:///etc/passwd"));
    }

    #[test]
    fn cache_keys_follow_the_strongest_pin() {
        let a = pinned(
            "a",
            "https://x.invalid/a.jar",
            b"a",
            &[HashAlgorithm::Sha1, HashAlgorithm::Sha512],
        );
        assert!(a.cache_key().starts_with("sha512-"));
        assert_eq!(
            a.cache_key().len(),
            "sha512-".len() + 16,
            "short enough for Windows paths"
        );
    }
}
