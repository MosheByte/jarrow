//! Write, then read: what comes back is what went in, and writing it again gives the same
//! bytes.

use std::collections::BTreeSet;

use jarrow_jar::{Compression, Entry, Jar, MANIFEST_PATH, Method, WriteOptions, crc32};
use proptest::prelude::*;

fn segment() -> impl Strategy<Value = String> {
    prop_oneof![
        "[a-z0-9_][a-zA-Z0-9_.$-]{0,9}",
        // non-ASCII names exercise the UTF-8 flag
        "[a-z][a-z\u{e9}\u{4e2d}\u{1f600}]{0,4}",
    ]
}

fn path() -> impl Strategy<Value = String> {
    prop::collection::vec(segment(), 1..4).prop_map(|parts| parts.join("/"))
}

fn entry() -> impl Strategy<Value = Entry> {
    prop_oneof![
        1 => path().prop_map(Entry::directory),
        4 => (
            path(),
            prop_oneof![
                prop::collection::vec(any::<u8>(), 0..600),
                // repetitive data, so deflate actually shrinks it
                (any::<u8>(), 0usize..3000).prop_map(|(b, n)| vec![b; n]),
            ],
            any::<bool>(),
        )
            .prop_map(|(name, data, stored)| {
                let mut e = Entry::file(name, data);
                if stored {
                    e.method = Method::Stored;
                }
                e
            }),
    ]
}

fn jar() -> impl Strategy<Value = Jar> {
    (
        prop::collection::vec(entry(), 0..24),
        prop::option::of((0usize..24, prop::collection::vec(any::<u8>(), 0..200))),
    )
        .prop_map(|(entries, manifest)| {
            let mut seen = BTreeSet::new();
            let mut entries: Vec<Entry> = entries
                .into_iter()
                .filter(|e| seen.insert(e.name.clone()))
                .collect();
            if let Some((at, data)) = manifest
                && seen.insert(MANIFEST_PATH.to_string())
            {
                let at = at.min(entries.len());
                entries.insert(at, Entry::file(MANIFEST_PATH, data));
            }
            Jar::from_entries(entries)
        })
}

fn options() -> impl Strategy<Value = WriteOptions> {
    (
        prop_oneof![
            Just(Compression::PerEntry),
            Just(Compression::Deflate),
            Just(Compression::Store)
        ],
        0u8..=9,
    )
        .prop_map(|(compression, level)| {
            let mut o = WriteOptions::default();
            o.compression = compression;
            o.deflate_level = level;
            o
        })
}

/// The order the writer promises: the manifest first, everything else as given.
fn written_order(jar: &Jar) -> Vec<&Entry> {
    let manifest = jar.entries.iter().filter(|e| e.name == MANIFEST_PATH);
    let rest = jar.entries.iter().filter(|e| e.name != MANIFEST_PATH);
    manifest.chain(rest).collect()
}

fn expected_method(entry: &Entry, options: &WriteOptions) -> Method {
    if entry.is_dir() || entry.data.is_empty() {
        return Method::Stored;
    }
    match options.compression {
        Compression::Deflate => Method::Deflated,
        Compression::Store => Method::Stored,
        _ => entry.method,
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(192))]

    #[test]
    fn write_then_read_returns_the_entries(jar in jar(), options in options()) {
        let bytes = jar.write(&options).unwrap();
        let back = Jar::read(&bytes).unwrap();
        let expected = written_order(&jar);
        prop_assert_eq!(back.entries.len(), expected.len());
        for (got, want) in back.entries.iter().zip(expected) {
            prop_assert_eq!(&got.name, &want.name);
            prop_assert_eq!(&got.data, &want.data);
            prop_assert_eq!(got.method, expected_method(want, &options));
            let original = got.original.unwrap();
            prop_assert_eq!(original.crc32, crc32(&want.data));
            prop_assert_eq!(original.uncompressed_size, want.data.len() as u64);
            prop_assert!(!original.has_extra_field);
            prop_assert_eq!(original.unix_mode, None);
            prop_assert!(!original.zip64);
        }
    }

    #[test]
    fn writing_what_was_read_gives_the_same_bytes(jar in jar(), options in options()) {
        let first = jar.write(&options).unwrap();
        prop_assert_eq!(&first, &jar.write(&options).unwrap());
        let back = Jar::read(&first).unwrap();
        let second = back.write(&options).unwrap();
        prop_assert_eq!(first, second);
    }
}

#[test]
fn manifest_is_written_first() {
    let jar = Jar::from_entries(vec![
        Entry::directory("net"),
        Entry::file("net/A.class", b"a".to_vec()),
        Entry::directory("META-INF"),
        Entry::file(MANIFEST_PATH, b"Manifest-Version: 1.0\r\n\r\n".to_vec()),
        Entry::file("z.txt", b"z".to_vec()),
    ]);
    let back = Jar::read(&jar.write(&WriteOptions::default()).unwrap()).unwrap();
    assert_eq!(
        back.names().collect::<Vec<_>>(),
        [MANIFEST_PATH, "net/", "net/A.class", "META-INF/", "z.txt"]
    );
}

#[test]
fn lowercase_manifest_name_is_also_moved_first() {
    let jar = Jar::from_entries(vec![
        Entry::file("a.txt", b"a".to_vec()),
        Entry::file(
            "meta-inf/manifest.mf",
            b"Manifest-Version: 1.0\r\n\r\n".to_vec(),
        ),
    ]);
    let back = Jar::read(&jar.write(&WriteOptions::default()).unwrap()).unwrap();
    assert_eq!(
        back.names().collect::<Vec<_>>(),
        ["meta-inf/manifest.mf", "a.txt"]
    );
}

#[test]
fn empty_jar_is_a_bare_end_record() {
    let bytes = Jar::new().write(&WriteOptions::default()).unwrap();
    assert_eq!(bytes.len(), 22);
    assert_eq!(&bytes[..4], b"PK\x05\x06");
    assert_eq!(Jar::read(&bytes).unwrap(), Jar::new());
}

#[test]
fn invalid_deflate_level_is_an_error() {
    let mut options = WriteOptions::default();
    options.deflate_level = 10;
    assert!(Jar::new().write(&options).is_err());
}

#[test]
fn put_get_remove() {
    let mut jar = Jar::new();
    assert!(jar.put(Entry::file("a", b"1".to_vec())).is_none());
    assert!(jar.put(Entry::file("b", b"2".to_vec())).is_none());
    let old = jar.put(Entry::file("a", b"3".to_vec())).unwrap();
    assert_eq!(old.data, b"1");
    assert_eq!(jar.names().collect::<Vec<_>>(), ["a", "b"]);
    assert_eq!(jar.get("a").unwrap().data, b"3");
    jar.get_mut("b").unwrap().data = b"4".to_vec();
    assert_eq!(jar.remove("b").unwrap().data, b"4");
    assert!(jar.remove("b").is_none());
    assert!(jar.get("b").is_none());
}
