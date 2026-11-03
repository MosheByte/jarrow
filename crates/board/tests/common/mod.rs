//! Test support: an offline [`Fetch`] that answers from a map of URLs, and JARs built in
//! memory (tests never read or commit a real JAR).

#![allow(dead_code)]
#![allow(
    clippy::unwrap_used,
    reason = "test helpers: a failed unwrap is a failed test"
)]

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io::Write;
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

/// A zip with the given entry names (each holding its own name as content).
pub fn jar(entries: &[&str]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for name in entries {
        writer.start_file(*name, options).unwrap();
        writer.write_all(name.as_bytes()).unwrap();
    }
    writer.finish().unwrap().into_inner()
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
