//! `Jar::read` is the fuzzing entry point: it must return, never panic, on any input.
//! These property tests are a fast stand-in for a fuzzer in every `cargo test` run.

// Helpers outside #[test] fns unwrap like the tests do (clippy.toml allows it only there).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use jarrow_jar::{Entry, Jar, Manifest, ReadOptions, WriteOptions};
use proptest::prelude::*;

fn valid_archive() -> Vec<u8> {
    let mut nested = Jar::new();
    nested
        .entries
        .push(Entry::file("inner.txt", b"inner inner inner".to_vec()));
    let nested = nested.write(&WriteOptions::default()).unwrap();
    Jar::from_entries(vec![
        Entry::file(
            "META-INF/MANIFEST.MF",
            b"Manifest-Version: 1.0\r\n\r\nName: a\r\nSHA-256-Digest: x\r\n\r\n".to_vec(),
        ),
        Entry::directory("META-INF/jars"),
        Entry::file("META-INF/jars/n.jar", nested),
        Entry::file(
            "a",
            b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_vec(),
        ),
        Entry::file("META-INF/A.SF", b"sig".to_vec()),
    ])
    .write(&WriteOptions::default())
    .unwrap()
}

/// Everything a caller does with an untrusted jar, none of which may panic.
fn exercise(bytes: &[u8]) {
    let mut options = ReadOptions::default();
    options.max_total_size = 1 << 20;
    if let Ok(mut jar) = Jar::read_with(bytes, &options) {
        let _ = jar.manifest();
        let _ = jar.read_nested(&options);
        let _ = jar.strip_signatures();
        let _ = jar.write(&WriteOptions::default());
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn arbitrary_bytes(bytes in prop::collection::vec(any::<u8>(), 0..1024)) {
        exercise(&bytes);
    }

    #[test]
    fn arbitrary_bytes_with_an_end_record(
        mut bytes in prop::collection::vec(any::<u8>(), 0..512),
        tail in prop::collection::vec(any::<u8>(), 18),
    ) {
        bytes.extend_from_slice(b"PK\x05\x06");
        bytes.extend_from_slice(&tail);
        exercise(&bytes);
    }

    #[test]
    fn mutated_archives(
        edits in prop::collection::vec((any::<prop::sample::Index>(), any::<u8>()), 1..8),
        cut in any::<prop::sample::Index>(),
        truncate in any::<bool>(),
    ) {
        let mut bytes = valid_archive();
        for (index, value) in edits {
            let i = index.index(bytes.len());
            bytes[i] = value;
        }
        if truncate {
            bytes.truncate(cut.index(bytes.len()));
        }
        exercise(&bytes);
    }

    #[test]
    fn manifests(bytes in prop::collection::vec(any::<u8>(), 0..512)) {
        if let Ok(m) = Manifest::parse(&bytes) {
            let _ = m.to_bytes();
        }
    }
}
