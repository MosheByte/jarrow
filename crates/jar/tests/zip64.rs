//! Zip64 on the read side, from archives built field by field. The writer's zip64 output
//! is covered by the 70,000-entry test in determinism.rs and by a unit test that lowers
//! the zip64 thresholds.

mod common;

use common::{End, Raw, build, build_with, zip64_extra};
use jarrow_jar::{Error, Jar};

#[test]
fn zip64_sizes_in_the_extra_field() {
    let data = b"zip64 sizes live in the extra field";
    let mut raw = Raw::deflated("big.bin", data);
    let csize = u64::from(raw.csize);
    // as a zip64 writer lays it out: both headers marked, the real sizes in extra fields
    raw.central_extra = zip64_extra(&[data.len() as u64, csize]);
    raw.local_extra = zip64_extra(&[data.len() as u64, csize]);
    raw.csize = 0xFFFF_FFFF;
    raw.usize = 0xFFFF_FFFF;

    let jar = Jar::read(&build(&[raw])).unwrap();
    assert_eq!(jar.entries[0].data, data);
    let original = jar.entries[0].original.unwrap();
    assert!(original.zip64);
    assert!(original.has_extra_field);
    assert_eq!(original.uncompressed_size, data.len() as u64);
    assert_eq!(original.compressed_size, csize);
}

#[test]
fn zip64_offset_in_the_extra_field() {
    let first = Raw::stored("first.txt", b"first");
    let mut second = Raw::stored("second.txt", b"second");
    let second_offset = common::local(&first).len() as u64;
    second.offset = Some(0xFFFF_FFFF);
    second.central_extra = zip64_extra(&[second_offset]);
    let jar = Jar::read(&build(&[first, second])).unwrap();
    assert_eq!(jar.entries[1].data, b"second");
    assert!(jar.entries[1].original.unwrap().zip64);
}

#[test]
fn marked_values_without_a_zip64_field_are_errors() {
    let mut raw = Raw::stored("a.txt", b"x");
    raw.offset = Some(0xFFFF_FFFF);
    assert!(matches!(
        Jar::read(&build(&[raw.clone()])),
        Err(Error::Malformed { .. })
    ));
    // a zip64 field that is too short for the marked values
    raw.central_extra = vec![0x01, 0x00, 0x04, 0x00, 0, 0, 0, 0];
    assert!(matches!(
        Jar::read(&build(&[raw.clone()])),
        Err(Error::Malformed { .. })
    ));
    // a broken extra block
    raw.central_extra = vec![0x01, 0x00, 0x40];
    assert!(matches!(
        Jar::read(&build(&[raw])),
        Err(Error::Malformed { .. })
    ));
}

#[test]
fn zip64_end_record_is_read() {
    let end = End {
        zip64: true,
        ..End::default()
    };
    let bytes = build_with(
        &[
            Raw::stored("a.txt", b"a"),
            Raw::deflated("b.txt", b"bbbbbbbb"),
        ],
        &end,
    );
    let jar = Jar::read(&bytes).unwrap();
    assert_eq!(jar.names().collect::<Vec<_>>(), ["a.txt", "b.txt"]);
    assert_eq!(jar.entries[1].data, b"bbbbbbbb");
}

#[test]
fn broken_zip64_end_records_are_errors() {
    let end = End {
        zip64: true,
        ..End::default()
    };
    let good = build_with(&[Raw::stored("a.txt", b"a")], &end);
    let eocd = good.len() - 22;
    let locator = eocd - 20;
    let record = locator - 56;

    let mut bad_signature = good.clone();
    bad_signature[record] ^= 0xFF;
    assert!(matches!(
        Jar::read(&bad_signature),
        Err(Error::Malformed { .. })
    ));

    let mut bad_offset = good.clone();
    bad_offset[locator + 8..locator + 16].copy_from_slice(&u64::MAX.to_le_bytes());
    assert!(Jar::read(&bad_offset).is_err());

    let mut multi_disk = good.clone();
    multi_disk[locator + 16..locator + 20].copy_from_slice(&5u32.to_le_bytes());
    assert_eq!(Jar::read(&multi_disk), Err(Error::MultiDisk));

    let mut huge_count = good;
    huge_count[record + 32..record + 40].copy_from_slice(&u64::MAX.to_le_bytes());
    huge_count[record + 24..record + 32].copy_from_slice(&u64::MAX.to_le_bytes());
    assert!(Jar::read(&huge_count).is_err());
}

#[test]
fn split_archives_are_errors() {
    let mut bytes = build(&[Raw::stored("a.txt", b"a")]);
    let eocd = bytes.len() - 22;
    bytes[eocd + 4] = 1; // number of this disk
    assert_eq!(Jar::read(&bytes), Err(Error::MultiDisk));
}
