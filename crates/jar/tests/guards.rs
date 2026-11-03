//! Hostile and malformed input: every case is an `Err`, never a panic and never a silently
//! dropped entry.

mod common;

use common::{End, Raw, build, build_with, local};
use jarrow_jar::{Entry, Error, Jar, Limit, Method, ReadOptions, UnsafeName, WriteOptions};

fn read(bytes: &[u8]) -> Result<Jar, Error> {
    Jar::read(bytes)
}

#[test]
fn path_traversal_names_are_errors() {
    let cases: &[(&str, UnsafeName)] = &[
        ("../evil.class", UnsafeName::ParentComponent),
        ("a/../../evil", UnsafeName::ParentComponent),
        ("assets/..", UnsafeName::ParentComponent),
        ("/etc/passwd", UnsafeName::Absolute),
        ("C:/Windows/evil.dll", UnsafeName::DriveLetter),
        ("C:evil", UnsafeName::DriveLetter),
        ("a\\..\\..\\evil", UnsafeName::Backslash),
        ("net\\example\\A.class", UnsafeName::Backslash),
        ("a/./b", UnsafeName::DotComponent),
        ("a//b", UnsafeName::EmptyComponent),
        ("", UnsafeName::Empty),
    ];
    for &(name, reason) in cases {
        // the hostile entry sits between two good ones: nothing may be dropped silently
        let bytes = build(&[
            Raw::stored("ok/first.txt", b"1"),
            Raw::stored(name, b"payload"),
            Raw::stored("ok/last.txt", b"2"),
        ]);
        assert_eq!(
            read(&bytes),
            Err(Error::UnsafeName {
                name: name.to_string(),
                reason
            }),
            "{name:?}"
        );
    }
}

#[test]
fn nul_in_name_is_an_error() {
    let bytes = build(&[Raw::name_bytes(b"a\0b", b"x")]);
    assert!(matches!(
        read(&bytes),
        Err(Error::UnsafeName {
            reason: UnsafeName::Nul,
            ..
        })
    ));
}

#[test]
fn dots_inside_names_are_fine() {
    let bytes = build(&[Raw::stored("assets/x/lang/en_us..json", b"{}")]);
    let jar = read(&bytes).unwrap();
    assert_eq!(jar.entries[0].name, "assets/x/lang/en_us..json");
}

#[test]
fn writer_refuses_unsafe_names() {
    for name in ["../x", "/x", "a\\b", "C:/x", "a//b", "./a", ""] {
        let jar = Jar::from_entries(vec![Entry::file(name, b"x".to_vec())]);
        assert!(
            matches!(
                jar.write(&WriteOptions::default()),
                Err(Error::UnsafeName { .. })
            ),
            "{name:?}"
        );
    }
}

#[test]
fn non_utf8_name_is_an_error() {
    let bytes = build(&[Raw::stored("ok", b""), Raw::name_bytes(b"caf\xe9", b"x")]);
    assert_eq!(read(&bytes), Err(Error::NameNotUtf8 { index: 1 }));
}

#[test]
fn duplicate_names_are_errors() {
    let bytes = build(&[Raw::stored("a.txt", b"1"), Raw::stored("a.txt", b"2")]);
    assert_eq!(
        read(&bytes),
        Err(Error::DuplicateName {
            name: "a.txt".into()
        })
    );
    let jar = Jar::from_entries(vec![
        Entry::file("a.txt", b"1".to_vec()),
        Entry::file("a.txt", b"2".to_vec()),
    ]);
    assert!(matches!(
        jar.write(&WriteOptions::default()),
        Err(Error::DuplicateName { .. })
    ));
}

#[test]
fn encrypted_entries_are_errors() {
    let mut raw = Raw::stored("secret.class", b"x");
    raw.flags = 1;
    assert_eq!(
        read(&build(&[raw])),
        Err(Error::Encrypted {
            name: "secret.class".into()
        })
    );
}

#[test]
fn unsupported_methods_are_errors() {
    let mut raw = Raw::stored("a.bz2", b"x");
    raw.method = 12;
    assert_eq!(
        read(&build(&[raw])),
        Err(Error::UnsupportedMethod {
            name: "a.bz2".into(),
            method: 12
        })
    );
}

#[test]
fn crc_mismatch_is_an_error() {
    let mut raw = Raw::deflated("a.txt", b"hello hello hello");
    raw.crc ^= 1;
    assert!(matches!(
        read(&build(&[raw])),
        Err(Error::CrcMismatch { .. })
    ));
}

#[test]
fn local_header_disagreements_are_errors() {
    let mut raw = Raw::stored("a.txt", b"x");
    raw.local_name = Some(b"b.txt".to_vec());
    assert_eq!(
        read(&build(&[raw])),
        Err(Error::LocalHeaderMismatch {
            name: "a.txt".into(),
            field: "name"
        })
    );
    let mut raw = Raw::stored("a.txt", b"x");
    raw.local_method = Some(8);
    assert_eq!(
        read(&build(&[raw])),
        Err(Error::LocalHeaderMismatch {
            name: "a.txt".into(),
            field: "compression method"
        })
    );
}

#[test]
fn directory_with_data_is_an_error() {
    let bytes = build(&[Raw::stored("dir/", b"data")]);
    assert_eq!(
        read(&bytes),
        Err(Error::DirectoryWithData {
            name: "dir/".into()
        })
    );
    let mut dir = Entry::directory("dir");
    dir.data = b"x".to_vec();
    assert!(matches!(
        Jar::from_entries(vec![dir]).write(&WriteOptions::default()),
        Err(Error::DirectoryWithData { .. })
    ));
}

#[test]
fn not_a_zip() {
    assert_eq!(read(b""), Err(Error::NotAZip));
    assert_eq!(read(b"PK"), Err(Error::NotAZip));
    assert_eq!(read(&[0u8; 100]), Err(Error::NotAZip));
    assert_eq!(
        read(b"just some text that is long enough"),
        Err(Error::NotAZip)
    );
}

#[test]
fn every_truncation_is_an_error() {
    let bytes = Jar::from_entries(vec![
        Entry::file(
            "META-INF/MANIFEST.MF",
            b"Manifest-Version: 1.0\r\n\r\n".to_vec(),
        ),
        Entry::directory("a"),
        Entry::file("a/b.txt", b"hello world, hello world".to_vec()),
    ])
    .write(&WriteOptions::default())
    .unwrap();
    for len in 0..bytes.len() {
        assert!(read(&bytes[..len]).is_err(), "prefix of {len} bytes");
    }
    assert!(read(&bytes).is_ok());
}

// ---- zip-bomb guards -------------------------------------------------------------------

fn limits(entries: usize, entry: u64, total: u64) -> ReadOptions {
    let mut options = ReadOptions::default();
    options.max_entries = entries;
    options.max_entry_size = entry;
    options.max_total_size = total;
    options
}

#[test]
fn size_lie_is_caught_while_inflating() {
    // 1 MiB of zeros deflates to about 1 KiB; the archive claims 100 bytes.
    let zeros = vec![0u8; 1 << 20];
    let mut raw = Raw::deflated("bomb.bin", &zeros);
    raw.usize = 100;
    assert_eq!(
        read(&build(&[raw])),
        Err(Error::SizeMismatch {
            name: "bomb.bin".into(),
            declared: 100,
            actual: 101
        })
    );
}

#[test]
fn size_lie_downwards_is_caught() {
    let mut raw = Raw::deflated("short.bin", b"twelve bytes");
    raw.usize = 50;
    assert_eq!(
        read(&build(&[raw])),
        Err(Error::SizeMismatch {
            name: "short.bin".into(),
            declared: 50,
            actual: 12
        })
    );
    let mut raw = Raw::stored("stored.bin", b"abc");
    raw.usize = 4;
    assert!(matches!(
        read(&build(&[raw])),
        Err(Error::SizeMismatch { .. })
    ));
}

#[test]
fn entry_size_limit_is_checked_before_inflating() {
    // The payload is not even valid deflate: the limit must fire first.
    let mut raw = Raw::stored("huge.bin", b"not deflate at all");
    raw.method = 8;
    raw.usize = 0xFFFF_FFF0;
    assert_eq!(
        read(&build(&[raw])),
        Err(Error::LimitExceeded {
            limit: Limit::EntrySize,
            requested: 0xFFFF_FFF0,
            max: ReadOptions::DEFAULT_MAX_ENTRY_SIZE
        })
    );
}

#[test]
fn total_size_limit() {
    let entries: Vec<Raw> = (0..10)
        .map(|i| Raw::deflated(&format!("f{i}.bin"), &[0u8; 1000]))
        .collect();
    let bytes = build(&entries);
    assert!(Jar::read_with(&bytes, &limits(100, 1000, 10_000)).is_ok());
    assert_eq!(
        Jar::read_with(&bytes, &limits(100, 1000, 9_999)),
        Err(Error::LimitExceeded {
            limit: Limit::TotalSize,
            requested: 10_000,
            max: 9_999
        })
    );
    assert!(matches!(
        Jar::read_with(&bytes, &limits(100, 999, 1 << 20)),
        Err(Error::LimitExceeded {
            limit: Limit::EntrySize,
            ..
        })
    ));
}

#[test]
fn entry_count_limit() {
    let entries: Vec<Raw> = (0..5).map(|i| Raw::stored(&format!("f{i}"), b"")).collect();
    let bytes = build(&entries);
    assert!(Jar::read_with(&bytes, &limits(5, 1, 1)).is_ok());
    assert_eq!(
        Jar::read_with(&bytes, &limits(4, 1, 1)),
        Err(Error::LimitExceeded {
            limit: Limit::Entries,
            requested: 5,
            max: 4
        })
    );
}

#[test]
fn declared_entry_count_larger_than_directory_is_an_error() {
    let end = End {
        declared_entries: Some(60_000),
        ..End::default()
    };
    let bytes = build_with(&[Raw::stored("a", b"x")], &end);
    assert!(matches!(read(&bytes), Err(Error::Malformed { .. })));
}

#[test]
fn overlapping_entries_are_errors() {
    // The "better zip bomb" shape: entry B's local header and data sit inside entry A's
    // stored data, so one kernel is counted twice.
    let inner = Raw::stored("b.txt", b"kernel kernel kernel");
    let outer = Raw::stored("a.txt", &local(&inner));
    let mut b = inner.clone();
    b.write_local = false;
    b.offset = Some(30 + 5); // A's data starts after its 30-byte header and 5-byte name
    let bytes = build(&[outer, b]);
    assert_eq!(
        read(&bytes),
        Err(Error::OverlappingEntries {
            first: "a.txt".into(),
            second: "b.txt".into()
        })
    );
}

#[test]
fn two_records_for_one_local_header_are_errors() {
    let a = Raw::stored("a.txt", b"same bytes");
    let mut alias = Raw::stored("a.txt", b"same bytes");
    alias.write_local = false;
    alias.offset = Some(0);
    alias.name = b"b.txt".to_vec();
    let bytes = build(&[a, alias]);
    assert!(read(&bytes).is_err());
}

#[test]
fn data_running_into_the_directory_is_an_error() {
    let mut raw = Raw::stored("a.txt", b"abc");
    raw.csize = 500;
    raw.usize = 500;
    assert!(matches!(read(&build(&[raw])), Err(Error::Malformed { .. })));
}

#[test]
fn invalid_deflate_is_an_error() {
    let mut raw = Raw::stored("a.txt", &[0xFF; 16]);
    raw.method = 8;
    raw.usize = 10;
    assert_eq!(
        read(&build(&[raw])),
        Err(Error::Inflate {
            name: "a.txt".into()
        })
    );
}

// ---- tolerated real-world shapes ---------------------------------------------------------

#[test]
fn data_descriptors_prefix_comments_and_metadata_are_read() {
    let mut deflated = Raw::deflated("net/example/A.class", &[0xCA, 0xFE, 0xBA, 0xBE, 0, 0]);
    deflated.descriptor = true;
    deflated.local_extra = vec![0x55, 0x54, 0x01, 0x00, 0x00]; // "UT" timestamp field
    let mut unix = Raw::stored("run.sh", b"#!/bin/sh\n");
    unix.made_by = (3 << 8) | 30;
    unix.external = 0o100_755 << 16;
    let mut dir = Raw::deflated("net/", b"");
    dir.comment = b"a directory".to_vec();
    let end = End {
        prefix: b"#!/bin/sh\nexec java -jar \"$0\"\n".to_vec(),
        comment: b"archive comment".to_vec(),
        ..End::default()
    };
    let jar = Jar::read(&build_with(&[deflated, unix, dir], &end)).unwrap();
    assert_eq!(
        jar.names().collect::<Vec<_>>(),
        ["net/example/A.class", "run.sh", "net/"]
    );
    let class = &jar.entries[0];
    assert_eq!(class.data, [0xCA, 0xFE, 0xBA, 0xBE, 0, 0]);
    assert_eq!(class.method, Method::Deflated);
    let original = class.original.unwrap();
    assert!(original.has_extra_field);
    assert_eq!(original.unix_mode, None);
    assert_eq!(original.uncompressed_size, 6);
    let script = jar.entries[1].original.unwrap();
    assert_eq!(script.unix_mode, Some(0o100_755));
    assert!(!script.has_extra_field);
    assert!(jar.entries[2].is_dir());
    assert!(jar.entries[2].data.is_empty());
}
