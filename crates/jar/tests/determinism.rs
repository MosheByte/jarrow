//! The writer's output is a pure function of the entries and the options.

mod common;

use common::{Raw, build, u16_at, u32_at};
use jarrow_jar::{Compression, Entry, Jar, Method, WriteOptions};

fn sample() -> Jar {
    Jar::from_entries(vec![
        Entry::directory("META-INF"),
        Entry::file(
            "META-INF/MANIFEST.MF",
            b"Manifest-Version: 1.0\r\nCreated-By: Jarrow\r\n\r\n".to_vec(),
        ),
        Entry::directory("net/example"),
        Entry::file(
            "net/example/Mod.class",
            vec![0xCA, 0xFE, 0xBA, 0xBE, 0, 0, 0, 65],
        ),
        Entry::file("assets/example/lang/en_us.json", br#"{"a":"b"}"#.repeat(40)),
        Entry::file("empty.txt", Vec::new()),
    ])
}

#[test]
fn two_writes_are_identical() {
    let jar = sample();
    for compression in [
        Compression::PerEntry,
        Compression::Deflate,
        Compression::Store,
    ] {
        let mut options = WriteOptions::default();
        options.compression = compression;
        let a = jar.write(&options).unwrap();
        let b = jar.clone().write(&options).unwrap();
        assert_eq!(a, b, "{compression:?}");
    }
}

/// The exact bytes of a small stored jar, field by field, so any change to the format the
/// writer emits shows up here (APPNOTE.TXT 4.3.7, 4.3.12 and 4.3.16).
#[test]
fn stored_output_is_byte_exact() {
    let mut file = Entry::file("a.txt", b"hi".to_vec());
    file.method = Method::Stored;
    let jar = Jar::from_entries(vec![Entry::directory("d"), file]);
    let bytes = jar.write(&WriteOptions::default()).unwrap();

    let crc_hi = jarrow_jar::crc32(b"hi").to_le_bytes();
    let mut expected: Vec<u8> = Vec::new();
    // local header "d/": version 20, no flags, stored, 1980-01-01 00:00:00, CRC 0, sizes 0
    expected.extend_from_slice(b"PK\x03\x04\x14\x00\x00\x00\x00\x00\x00\x00\x21\x00");
    expected.extend_from_slice(&[0; 12]);
    expected.extend_from_slice(b"\x02\x00\x00\x00d/");
    // local header "a.txt": version 10, stored, CRC of "hi", sizes 2
    expected.extend_from_slice(b"PK\x03\x04\x0a\x00\x00\x00\x00\x00\x00\x00\x21\x00");
    expected.extend_from_slice(&crc_hi);
    expected.extend_from_slice(b"\x02\x00\x00\x00\x02\x00\x00\x00\x05\x00\x00\x00a.txthi");
    let cd_offset = expected.len() as u32;
    // central record "d/": made by and needed 20, MS-DOS directory attribute, offset 0
    expected.extend_from_slice(b"PK\x01\x02\x14\x00\x14\x00\x00\x00\x00\x00\x00\x00\x21\x00");
    expected.extend_from_slice(&[0; 12]);
    expected.extend_from_slice(b"\x02\x00\x00\x00\x00\x00\x00\x00\x00\x00\x10\x00\x00\x00");
    expected.extend_from_slice(b"\x00\x00\x00\x00d/");
    // central record "a.txt": offset 32
    expected.extend_from_slice(b"PK\x01\x02\x0a\x00\x0a\x00\x00\x00\x00\x00\x00\x00\x21\x00");
    expected.extend_from_slice(&crc_hi);
    expected.extend_from_slice(b"\x02\x00\x00\x00\x02\x00\x00\x00\x05\x00\x00\x00\x00\x00");
    expected.extend_from_slice(b"\x00\x00\x00\x00\x00\x00\x00\x00\x20\x00\x00\x00a.txt");
    let cd_size = expected.len() as u32 - cd_offset;
    // end of central directory: 2 entries, no comment
    expected.extend_from_slice(b"PK\x05\x06\x00\x00\x00\x00\x02\x00\x02\x00");
    expected.extend_from_slice(&cd_size.to_le_bytes());
    expected.extend_from_slice(&cd_offset.to_le_bytes());
    expected.extend_from_slice(b"\x00\x00");

    assert_eq!(bytes, expected);
}

#[test]
fn every_header_has_the_fixed_timestamp_and_no_extra_field() {
    let bytes = sample().write(&WriteOptions::default()).unwrap();
    let mut pos = 0;
    let mut locals = 0;
    while u32_at(&bytes, pos) == 0x0403_4b50 {
        assert_eq!(u16_at(&bytes, pos + 10), 0, "DOS time");
        assert_eq!(u16_at(&bytes, pos + 12), 0x0021, "DOS date 1980-01-01");
        assert_eq!(u16_at(&bytes, pos + 6) & (1 << 3), 0, "no data descriptor");
        assert_eq!(u16_at(&bytes, pos + 28), 0, "no local extra field");
        let name = usize::from(u16_at(&bytes, pos + 26));
        let csize = u32_at(&bytes, pos + 18) as usize;
        pos += 30 + name + csize;
        locals += 1;
    }
    assert_eq!(locals, 6);
    for _ in 0..6 {
        assert_eq!(u32_at(&bytes, pos), 0x0201_4b50);
        assert_eq!(u16_at(&bytes, pos + 12), 0, "DOS time");
        assert_eq!(u16_at(&bytes, pos + 14), 0x0021, "DOS date");
        assert_eq!(u16_at(&bytes, pos + 30), 0, "no central extra field");
        assert_eq!(u16_at(&bytes, pos + 32), 0, "no comment");
        assert_eq!(bytes[pos + 5], 0, "made by MS-DOS: no Unix mode");
        pos += 46 + usize::from(u16_at(&bytes, pos + 28));
    }
    assert_eq!(u32_at(&bytes, pos), 0x0605_4b50);
    assert_eq!(bytes.len(), pos + 22);
}

#[test]
fn input_metadata_never_reaches_the_output() {
    // The same names, data and methods, once with timestamps, Unix modes, extra fields,
    // comments, data descriptors and a prefix, once fresh: the output must be identical.
    let mut a = Raw::deflated("net/A.class", b"class bytes class bytes");
    a.descriptor = true;
    a.local_extra = vec![0x55, 0x54, 0x01, 0x00, 0x00];
    a.central_extra = vec![0x55, 0x54, 0x01, 0x00, 0x00];
    a.made_by = (3 << 8) | 30;
    a.external = 0o100_644 << 16;
    a.comment = b"entry comment".to_vec();
    let mut b = Raw::stored("b.txt", b"bee");
    b.time = 0x7d1c;
    b.date = 0x5b8e;
    let end = common::End {
        prefix: b"prefix bytes".to_vec(),
        comment: b"archive comment".to_vec(),
        ..common::End::default()
    };
    let read = Jar::read(&common::build_with(&[a, b], &end)).unwrap();

    let mut fresh_b = Entry::file("b.txt", b"bee".to_vec());
    fresh_b.method = Method::Stored;
    let fresh = Jar::from_entries(vec![
        Entry::file("net/A.class", b"class bytes class bytes".to_vec()),
        fresh_b,
    ]);
    let options = WriteOptions::default();
    assert_eq!(
        read.write(&options).unwrap(),
        fresh.write(&options).unwrap()
    );
}

#[test]
fn output_does_not_depend_on_how_the_input_was_compressed() {
    let data = b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_vec();
    let stored = Jar::read(&build(&[Raw::stored("x", &data)])).unwrap();
    let deflated = Jar::read(&build(&[Raw::deflated("x", &data)])).unwrap();
    let mut options = WriteOptions::default();
    options.compression = Compression::Deflate;
    assert_eq!(
        stored.write(&options).unwrap(),
        deflated.write(&options).unwrap()
    );
}

#[test]
fn more_than_65534_entries_use_a_zip64_end_record() {
    let entries: Vec<Entry> = (0..70_000)
        .map(|i| {
            let mut e = Entry::file(format!("f/{i:05}"), Vec::new());
            e.method = Method::Stored;
            e
        })
        .collect();
    let jar = Jar::from_entries(entries);
    let bytes = jar.write(&WriteOptions::default()).unwrap();
    assert_eq!(bytes, jar.write(&WriteOptions::default()).unwrap());
    // EOCD entry counts are the 0xFFFF marker; the zip64 record and locator precede it
    let eocd = bytes.len() - 22;
    assert_eq!(u16_at(&bytes, eocd + 10), 0xFFFF);
    assert_eq!(u32_at(&bytes, eocd - 20), 0x0706_4b50);
    assert_eq!(u32_at(&bytes, eocd - 20 - 56), 0x0606_4b50);
    let back = Jar::read(&bytes).unwrap();
    assert_eq!(back.entries.len(), 70_000);
    assert_eq!(back.entries[69_999].name, "f/69999");
    assert_eq!(back.write(&WriteOptions::default()).unwrap(), bytes);
}
