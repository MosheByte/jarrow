//! Test support: an offline [`Fetch`] that answers from a map of URLs, and JARs built in
//! memory (tests never read or commit a real JAR).

#![allow(dead_code)]
#![allow(
    clippy::unwrap_used,
    reason = "test helpers: a failed unwrap is a failed test"
)]

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::PathBuf;

use jarrow_board::http::{Fetch, FetchError, FetchErrorKind, ResettableSink};

/// Answers GETs from a map; unknown URLs are 404. Records every URL asked for.
#[derive(Default)]
pub struct FakeFetch {
    pub routes: BTreeMap<String, Vec<u8>>,
    pub calls: RefCell<Vec<String>>,
}

impl FakeFetch {
    pub fn with(mut self, url: &str, body: impl Into<Vec<u8>>) -> Self {
        self.routes.insert(url.to_owned(), body.into());
        self
    }

    fn lookup(&self, url: &str) -> Result<Vec<u8>, FetchError> {
        self.calls.borrow_mut().push(url.to_owned());
        self.routes
            .get(url)
            .cloned()
            .ok_or_else(|| FetchError::status(url, 404))
    }
}

impl Fetch for FakeFetch {
    fn get(&self, url: &str, max_bytes: u64) -> Result<Vec<u8>, FetchError> {
        let body = self.lookup(url)?;
        if body.len() as u64 > max_bytes {
            return Err(FetchError {
                url: url.to_owned(),
                kind: FetchErrorKind::TooLarge { limit: max_bytes },
            });
        }
        Ok(body)
    }

    fn download(
        &self,
        url: &str,
        sink: &mut dyn ResettableSink,
        max_bytes: u64,
    ) -> Result<u64, FetchError> {
        let body = self.get(url, max_bytes)?;
        sink.reset()
            .map_err(|e| FetchError::transport(url, e.to_string()))?;
        sink.write_all(&body)
            .map_err(|e| FetchError::transport(url, e.to_string()))?;
        Ok(body.len() as u64)
    }
}

/// A zip with the given entry names (each holding its own name as content), stored, with
/// every header field fixed: DOS time 1980-01-01 00:00, "version made by" 2.0 on MS-DOS, no
/// attributes. The same bytes on every OS (the zip crate's writer stamps the host OS into each
/// central directory entry), so the hashes in `fixtures/board-sample.json` hold everywhere.
pub fn jar(entries: &[&str]) -> Vec<u8> {
    fn u16le(out: &mut Vec<u8>, v: usize) {
        out.extend_from_slice(&u16::try_from(v).unwrap().to_le_bytes());
    }
    fn u32le(out: &mut Vec<u8>, v: usize) {
        out.extend_from_slice(&u32::try_from(v).unwrap().to_le_bytes());
    }
    const VERSION: usize = 20;
    const DOS_DATE_1980_01_01: usize = 0x21;
    let mut out = Vec::new();
    let mut central = Vec::new();
    for name in entries {
        let data = name.as_bytes();
        let mut crc = flate2::Crc::new();
        crc.update(data);
        let crc = crc.sum() as usize;
        let offset = out.len();
        u32le(&mut out, 0x0403_4b50);
        for v in [VERSION, 0, 0, 0, DOS_DATE_1980_01_01] {
            u16le(&mut out, v);
        }
        for v in [crc, data.len(), data.len()] {
            u32le(&mut out, v);
        }
        u16le(&mut out, name.len());
        u16le(&mut out, 0);
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(data);

        u32le(&mut central, 0x0201_4b50);
        for v in [VERSION, VERSION, 0, 0, 0, DOS_DATE_1980_01_01] {
            u16le(&mut central, v);
        }
        for v in [crc, data.len(), data.len()] {
            u32le(&mut central, v);
        }
        for v in [name.len(), 0, 0, 0, 0] {
            u16le(&mut central, v);
        }
        u32le(&mut central, 0);
        u32le(&mut central, offset);
        central.extend_from_slice(name.as_bytes());
    }
    let central_offset = out.len();
    out.extend_from_slice(&central);
    u32le(&mut out, 0x0605_4b50);
    for v in [0, 0, entries.len(), entries.len()] {
        u16le(&mut out, v);
    }
    u32le(&mut out, central.len());
    u32le(&mut out, central_offset);
    u16le(&mut out, 0);
    out
}

/// Path of a fixture file.
pub fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

/// Bytes of a fixture file.
pub fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(fixture_path(name)).unwrap()
}

pub fn gv(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| (*s).to_owned()).collect()
}
