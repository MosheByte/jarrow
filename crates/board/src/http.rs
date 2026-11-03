//! HTTP for the board: a small [`Fetch`] trait and its ureq + rustls implementation.
//!
//! Every request carries the User-Agent `MosheByte/jarrow-board/<version>
//! (github.com/MosheByte/jarrow)`, as Modrinth asks of API clients. A `429 Too Many Requests`
//! waits for `Retry-After` (or `X-Ratelimit-Reset`) and retries; when Modrinth reports
//! `X-Ratelimit-Remaining: 0`, the next request waits for the window to reset. Server errors
//! and network errors are retried with a growing delay. Nothing is sent but the request line
//! and the User-Agent: no cookies, no tokens.

use std::fmt;
use std::io::{Read, Write};
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

/// The User-Agent every board request sends.
pub fn user_agent() -> String {
    format!(
        "MosheByte/jarrow-board/{} (github.com/MosheByte/jarrow)",
        crate::ENGINE_VERSION
    )
}

/// Why an HTTP request failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FetchErrorKind {
    /// The server answered with this status after every retry.
    Status(u16),
    /// The body was larger than allowed.
    TooLarge {
        /// The limit in bytes.
        limit: u64,
    },
    /// Connecting, TLS, reading or writing failed.
    Transport(String),
}

/// A failed request, with the URL it was for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchError {
    /// The URL (board URLs never carry credentials).
    pub url: String,
    /// What went wrong.
    pub kind: FetchErrorKind,
}

impl FetchError {
    /// A transport failure.
    pub fn transport(url: &str, message: impl Into<String>) -> Self {
        Self {
            url: url.to_owned(),
            kind: FetchErrorKind::Transport(message.into()),
        }
    }

    /// A final HTTP status.
    pub fn status(url: &str, status: u16) -> Self {
        Self {
            url: url.to_owned(),
            kind: FetchErrorKind::Status(status),
        }
    }

    /// Whether retrying could help: 429, 5xx and transport errors.
    pub fn is_transient(&self) -> bool {
        match self.kind {
            FetchErrorKind::Status(code) => code == 429 || code >= 500,
            FetchErrorKind::TooLarge { .. } => false,
            FetchErrorKind::Transport(_) => true,
        }
    }
}

impl fmt::Display for FetchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            FetchErrorKind::Status(code) => write!(f, "GET {} answered HTTP {code}", self.url),
            FetchErrorKind::TooLarge { limit } => {
                write!(f, "GET {} returned more than {limit} bytes", self.url)
            }
            FetchErrorKind::Transport(message) => write!(f, "GET {} failed: {message}", self.url),
        }
    }
}

impl std::error::Error for FetchError {}

/// What the board needs from HTTP. Tests implement it with canned responses.
pub trait Fetch {
    /// GETs `url` and returns the body, at most `max_bytes` long.
    fn get(&self, url: &str, max_bytes: u64) -> Result<Vec<u8>, FetchError>;

    /// GETs `url` and streams the body into `sink`, at most `max_bytes`. Returns the byte
    /// count. A retry starts the sink over, so `sink` must be rewound by the caller; the
    /// implementation calls `reset` before every attempt.
    fn download(
        &self,
        url: &str,
        sink: &mut dyn ResettableSink,
        max_bytes: u64,
    ) -> Result<u64, FetchError>;
}

/// A write target that can start over, for downloads that are retried.
pub trait ResettableSink: Write {
    /// Discards everything written so far.
    fn reset(&mut self) -> std::io::Result<()>;
}

impl ResettableSink for Vec<u8> {
    fn reset(&mut self) -> std::io::Result<()> {
        self.clear();
        Ok(())
    }
}

/// The response headers the retry policy reads.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RateHeaders {
    /// `Retry-After`, as sent.
    pub retry_after: Option<String>,
    /// `X-Ratelimit-Remaining`.
    pub remaining: Option<u64>,
    /// `X-Ratelimit-Reset`, seconds until the window resets.
    pub reset_secs: Option<u64>,
}

/// The longest the board ever waits before one retry.
pub const MAX_WAIT: Duration = Duration::from_secs(120);

/// How long to wait before retry number `attempt` (1-based) of a request that answered
/// `status`, or `None` when the status is final.
///
/// - 429: `Retry-After` seconds, else `X-Ratelimit-Reset`, else 60 s.
/// - 5xx: 2, 4, 8, ... seconds.
///
/// Capped at [`MAX_WAIT`]. An HTTP-date `Retry-After` is treated as 60 s.
pub fn retry_delay(status: u16, headers: &RateHeaders, attempt: u32) -> Option<Duration> {
    let backoff = || Duration::from_secs(2u64.saturating_pow(attempt.min(6)));
    let delay = match status {
        429 => {
            let from_retry_after = headers.retry_after.as_deref().map(|v| {
                v.trim()
                    .parse::<u64>()
                    .map(Duration::from_secs)
                    .unwrap_or(Duration::from_secs(60))
            });
            from_retry_after
                .or(headers.reset_secs.map(Duration::from_secs))
                .unwrap_or(Duration::from_secs(60))
        }
        500..=599 => backoff(),
        _ => return None,
    };
    Some(delay.min(MAX_WAIT))
}

/// How long to wait before the next request, given the last response's rate headers: the
/// reset time when no requests are left in the window, else nothing.
pub fn pacing_delay(headers: &RateHeaders) -> Option<Duration> {
    match headers.remaining {
        Some(0) => Some(Duration::from_secs(headers.reset_secs.unwrap_or(60).max(1)).min(MAX_WAIT)),
        _ => None,
    }
}

/// The real HTTP client: ureq over rustls (ring provider), with the operating system's root
/// certificates.
pub struct HttpClient {
    agent: ureq::Agent,
    max_attempts: u32,
    pause: Mutex<Option<Duration>>,
    sleep: fn(Duration),
}

impl fmt::Debug for HttpClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HttpClient")
            .field("max_attempts", &self.max_attempts)
            .finish_non_exhaustive()
    }
}

fn header_text(headers: &ureq::http::HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned)
}

fn rate_headers(headers: &ureq::http::HeaderMap) -> RateHeaders {
    let number = |name: &str| header_text(headers, name).and_then(|v| v.trim().parse().ok());
    RateHeaders {
        retry_after: header_text(headers, "retry-after"),
        remaining: number("x-ratelimit-remaining"),
        reset_secs: number("x-ratelimit-reset"),
    }
}

impl HttpClient {
    /// A client with the board's User-Agent, a 10-minute ceiling per request and four
    /// attempts per request.
    pub fn new() -> Result<Self, FetchError> {
        let roots = rustls_native_certs::load_native_certs();
        let certs: Vec<ureq::tls::Certificate<'static>> = roots
            .certs
            .iter()
            .map(|der| ureq::tls::Certificate::from_der(der.as_ref()).to_owned())
            .collect();
        if certs.is_empty() {
            return Err(FetchError::transport(
                "https://api.modrinth.com/",
                "no root certificates found in the operating system's store",
            ));
        }
        let tls = ureq::tls::TlsConfig::builder()
            .provider(ureq::tls::TlsProvider::Rustls)
            .root_certs(ureq::tls::RootCerts::Specific(Arc::new(certs)))
            .unversioned_rustls_crypto_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .build();
        let config = ureq::Agent::config_builder()
            .user_agent(user_agent())
            .http_status_as_error(false)
            .max_redirects(5)
            .timeout_connect(Some(Duration::from_secs(30)))
            .timeout_global(Some(Duration::from_secs(600)))
            .tls_config(tls)
            .build();
        Ok(Self {
            agent: config.into(),
            max_attempts: 4,
            pause: Mutex::new(None),
            sleep: std::thread::sleep,
        })
    }

    fn wait_for_window(&self) {
        let pending = self.pause.lock().ok().and_then(|mut p| p.take());
        if let Some(delay) = pending {
            (self.sleep)(delay);
        }
    }

    fn remember_pacing(&self, headers: &RateHeaders) {
        if let Ok(mut pause) = self.pause.lock() {
            *pause = pacing_delay(headers);
        }
    }

    /// Runs one GET with retries and hands a successful body reader to `read`.
    fn request<T>(
        &self,
        url: &str,
        mut read: impl FnMut(&mut dyn Read) -> Result<T, FetchError>,
    ) -> Result<T, FetchError> {
        let mut attempt: u32 = 0;
        loop {
            attempt += 1;
            self.wait_for_window();
            let (err, delay) = match self.agent.get(url).call() {
                Ok(response) => {
                    let status = response.status().as_u16();
                    let headers = rate_headers(response.headers());
                    self.remember_pacing(&headers);
                    if (200..300).contains(&status) {
                        let mut body = response.into_body();
                        let mut reader = body.as_reader();
                        match read(&mut reader) {
                            Ok(value) => return Ok(value),
                            Err(err) => (err, None),
                        }
                    } else {
                        (
                            FetchError::status(url, status),
                            retry_delay(status, &headers, attempt),
                        )
                    }
                }
                Err(err) => (FetchError::transport(url, err.to_string()), None),
            };
            if !err.is_transient() || attempt >= self.max_attempts {
                return Err(err);
            }
            let backoff = Duration::from_secs(2u64.saturating_pow(attempt.min(6)));
            (self.sleep)(delay.unwrap_or(backoff).min(MAX_WAIT));
        }
    }
}

fn copy_limited(
    url: &str,
    reader: &mut dyn Read,
    sink: &mut dyn Write,
    max_bytes: u64,
) -> Result<u64, FetchError> {
    let mut buf = vec![0u8; 64 * 1024];
    let mut total: u64 = 0;
    loop {
        let n = reader
            .read(&mut buf)
            .map_err(|e| FetchError::transport(url, e.to_string()))?;
        if n == 0 {
            return Ok(total);
        }
        total += n as u64;
        if total > max_bytes {
            return Err(FetchError {
                url: url.to_owned(),
                kind: FetchErrorKind::TooLarge { limit: max_bytes },
            });
        }
        sink.write_all(&buf[..n])
            .map_err(|e| FetchError::transport(url, format!("writing the download: {e}")))?;
    }
}

impl Fetch for HttpClient {
    fn get(&self, url: &str, max_bytes: u64) -> Result<Vec<u8>, FetchError> {
        self.request(url, |reader| {
            let mut out = Vec::new();
            copy_limited(url, reader, &mut out, max_bytes)?;
            Ok(out)
        })
    }

    fn download(
        &self,
        url: &str,
        sink: &mut dyn ResettableSink,
        max_bytes: u64,
    ) -> Result<u64, FetchError> {
        self.request(url, |reader| {
            sink.reset()
                .map_err(|e| FetchError::transport(url, format!("resetting the download: {e}")))?;
            copy_limited(url, reader, &mut *sink, max_bytes)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers(
        retry_after: Option<&str>,
        remaining: Option<u64>,
        reset: Option<u64>,
    ) -> RateHeaders {
        RateHeaders {
            retry_after: retry_after.map(str::to_owned),
            remaining,
            reset_secs: reset,
        }
    }

    #[test]
    fn user_agent_names_the_project() {
        let ua = user_agent();
        assert!(ua.starts_with("MosheByte/jarrow-board/"), "{ua}");
        assert!(ua.ends_with(" (github.com/MosheByte/jarrow)"), "{ua}");
        assert!(ua.contains(crate::ENGINE_VERSION));
    }

    #[test]
    fn too_many_requests_respects_retry_after() {
        assert_eq!(
            retry_delay(429, &headers(Some("7"), None, Some(30)), 1),
            Some(Duration::from_secs(7))
        );
        assert_eq!(
            retry_delay(429, &headers(None, Some(0), Some(30)), 1),
            Some(Duration::from_secs(30))
        );
        assert_eq!(
            retry_delay(
                429,
                &headers(Some("Wed, 21 Oct 2026 07:28:00 GMT"), None, None),
                1
            ),
            Some(Duration::from_secs(60))
        );
        assert_eq!(
            retry_delay(429, &headers(Some("100000"), None, None), 1),
            Some(MAX_WAIT)
        );
    }

    #[test]
    fn server_errors_back_off_and_client_errors_do_not_retry() {
        assert_eq!(
            retry_delay(503, &RateHeaders::default(), 1),
            Some(Duration::from_secs(2))
        );
        assert_eq!(
            retry_delay(500, &RateHeaders::default(), 3),
            Some(Duration::from_secs(8))
        );
        assert_eq!(
            retry_delay(502, &RateHeaders::default(), 30),
            Some(Duration::from_secs(64))
        );
        for status in [200, 301, 400, 403, 404, 410] {
            assert_eq!(
                retry_delay(status, &RateHeaders::default(), 1),
                None,
                "{status}"
            );
        }
    }

    #[test]
    fn pacing_waits_only_when_the_window_is_empty() {
        assert_eq!(pacing_delay(&headers(None, Some(10), Some(40))), None);
        assert_eq!(
            pacing_delay(&headers(None, Some(0), Some(40))),
            Some(Duration::from_secs(40))
        );
        assert_eq!(
            pacing_delay(&headers(None, Some(0), Some(0))),
            Some(Duration::from_secs(1))
        );
        assert_eq!(pacing_delay(&RateHeaders::default()), None);
    }

    #[test]
    fn transient_errors() {
        assert!(FetchError::status("u", 429).is_transient());
        assert!(FetchError::status("u", 503).is_transient());
        assert!(!FetchError::status("u", 404).is_transient());
        assert!(FetchError::transport("u", "reset").is_transient());
        let too_large = FetchError {
            url: "u".into(),
            kind: FetchErrorKind::TooLarge { limit: 1 },
        };
        assert!(!too_large.is_transient());
        assert_eq!(
            FetchError::status("https://x/", 404).to_string(),
            "GET https://x/ answered HTTP 404"
        );
    }

    #[test]
    fn copy_limited_enforces_the_cap() {
        let data = vec![7u8; 1000];
        let mut out = Vec::new();
        assert_eq!(
            copy_limited("u", &mut data.as_slice(), &mut out, 1000).unwrap(),
            1000
        );
        let mut out = Vec::new();
        let err = copy_limited("u", &mut data.as_slice(), &mut out, 999).unwrap_err();
        assert_eq!(err.kind, FetchErrorKind::TooLarge { limit: 999 });
    }
}
