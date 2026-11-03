//! MANIFEST.MF: continuation lines, sections, line endings, errors, and the 72-byte writer.

use jarrow_jar::{
    Attribute, Entry, Error, Jar, MANIFEST_PATH, MAX_LINE_BYTES, Manifest, Section, WriteOptions,
};
use proptest::prelude::*;

const SAMPLE: &[u8] = b"Manifest-Version: 1.0\r\n\
Class-Path: lib/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\r\n bbbbbbbb.jar\r\n  lib/c.jar\r\n\
Main-Class: net.example.Main\r\n\
\r\n\
Name: net/example/\r\n\
Sealed: true\r\n\
\r\n\
Name: net/example/very/long/path/that/does/not/fit/on/one/line/of/the/manif\r\n est/A.class\r\n\
SHA-256-Digest: 47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU=\r\n\
\r\n";

#[test]
fn continuation_lines_are_joined() {
    let m = Manifest::parse(SAMPLE).unwrap();
    assert_eq!(m.get("Manifest-Version"), Some("1.0"));
    assert_eq!(
        m.get("class-path"),
        Some(
            "lib/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaabbbbbbbb.jar lib/c.jar"
        )
    );
    assert_eq!(m.get("Main-Class"), Some("net.example.Main"));
    assert_eq!(m.sections.len(), 2);
    assert_eq!(m.sections[0].name, "net/example/");
    assert_eq!(m.sections[0].get("Sealed"), Some("true"));
    let long = "net/example/very/long/path/that/does/not/fit/on/one/line/of/the/manifest/A.class";
    let section = m.section(long).unwrap();
    assert_eq!(
        section.get("SHA-256-Digest"),
        Some("47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU=")
    );
}

#[test]
fn every_line_ending_parses_the_same() {
    let crlf = Manifest::parse(SAMPLE).unwrap();
    let text = std::str::from_utf8(SAMPLE).unwrap();
    let lf = text.replace("\r\n", "\n");
    let cr = text.replace("\r\n", "\r");
    assert_eq!(Manifest::parse(lf.as_bytes()).unwrap(), crlf);
    assert_eq!(Manifest::parse(cr.as_bytes()).unwrap(), crlf);
}

#[test]
fn long_values_wrap_at_72_bytes_like_the_jdk() {
    let mut m = Manifest::new();
    m.set("Manifest-Version", "1.0");
    m.set("Long", "x".repeat(100));
    let bytes = m.to_bytes().unwrap();
    let mut expected = b"Manifest-Version: 1.0\r\nLong: ".to_vec();
    expected.extend_from_slice("x".repeat(66).as_bytes());
    expected.extend_from_slice(b"\r\n ");
    expected.extend_from_slice("x".repeat(34).as_bytes());
    expected.extend_from_slice(b"\r\n\r\n");
    assert_eq!(bytes, expected);
    assert_eq!(Manifest::parse(&bytes).unwrap(), m);
}

#[test]
fn a_header_of_exactly_72_bytes_is_one_line() {
    let mut m = Manifest::new();
    m.set("K", "y".repeat(72 - 3));
    let bytes = m.to_bytes().unwrap();
    assert_eq!(bytes.len(), 72 + 2 + 2);
    m.set("K", "y".repeat(72 - 2));
    let bytes = m.to_bytes().unwrap();
    assert!(bytes.windows(3).any(|w| w == b"\r\n "));
}

#[test]
fn wrapping_never_splits_a_utf8_sequence() {
    let mut m = Manifest::new();
    // "K: " + 68 ASCII bytes = 71 bytes, so the 2-byte 'é' would straddle the 72-byte mark
    let value = format!("{}\u{e9}\u{4e2d}{}", "a".repeat(68), "\u{1f600}".repeat(40));
    m.set("K", value.clone());
    let bytes = m.to_bytes().unwrap();
    for line in bytes.split(|&b| b == b'\n') {
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        assert!(line.len() <= MAX_LINE_BYTES, "{} bytes", line.len());
        assert!(std::str::from_utf8(line).is_ok());
    }
    assert_eq!(
        Manifest::parse(&bytes).unwrap().get("K"),
        Some(value.as_str())
    );
}

#[test]
fn manifest_version_is_written_first() {
    let mut m = Manifest::new();
    m.main.push(Attribute::new("Created-By", "Jarrow"));
    m.main.push(Attribute::new("Manifest-Version", "1.0"));
    assert_eq!(
        m.to_bytes().unwrap(),
        b"Manifest-Version: 1.0\r\nCreated-By: Jarrow\r\n\r\n"
    );
    let mut s = Manifest::new();
    s.main.push(Attribute::new("Created-By", "x"));
    s.main.push(Attribute::new("Signature-Version", "1.0"));
    assert!(
        s.to_bytes()
            .unwrap()
            .starts_with(b"Signature-Version: 1.0\r\n")
    );
}

#[test]
fn sections_are_written_with_blank_lines() {
    let mut m = Manifest::new();
    m.set("Manifest-Version", "1.0");
    let mut section = Section::new("net/example/");
    section.set("Sealed", "true");
    m.sections.push(section);
    m.sections.push(Section::new("empty/"));
    assert_eq!(
        m.to_bytes().unwrap(),
        b"Manifest-Version: 1.0\r\n\r\nName: net/example/\r\nSealed: true\r\n\r\nName: empty/\r\n\r\n"
    );
}

#[test]
fn lenient_where_the_jdk_is() {
    // no final newline, several blank lines, repeated attributes and sections
    let text = b"Manifest-Version: 1.0\nA: 1\na: 2\n\n\n\nName: x\nB: 3\n\nName: x\nC: 4";
    let m = Manifest::parse(text).unwrap();
    assert_eq!(m.main.len(), 3);
    assert_eq!(m.get("A"), Some("2"));
    assert_eq!(m.sections.len(), 2);
    assert_eq!(m.sections[1].get("C"), Some("4"));
    assert_eq!(Manifest::parse(b"").unwrap(), Manifest::new());
    assert_eq!(Manifest::parse(b"\r\n").unwrap(), Manifest::new());
    // a value keeps its spaces; "name:  x" has the value " x"
    assert_eq!(
        Manifest::parse(b"N:  x \r\n").unwrap().get("N"),
        Some(" x ")
    );
}

fn parse_error(text: &[u8]) -> (usize, String) {
    match Manifest::parse(text) {
        Err(Error::Manifest { line, what }) => (line, what),
        other => panic!("expected a manifest error, got {other:?}"),
    }
}

#[test]
fn malformed_manifests_are_errors() {
    assert_eq!(parse_error(b"Manifest-Version 1.0\r\n").0, 1);
    assert_eq!(parse_error(b"Manifest-Version: 1.0\r\nA:b\r\n").0, 2);
    assert_eq!(parse_error(b" starts with a continuation\r\n").0, 1);
    assert_eq!(parse_error(b"A: 1\r\n\r\n continuation\r\n").0, 3);
    let (line, what) = parse_error(b"A: 1\r\n\r\nSealed: true\r\n");
    assert_eq!(line, 3);
    assert!(what.contains("Name"), "{what}");
    assert_eq!(parse_error(b"Bad Name: x\r\n").0, 1);
    assert_eq!(parse_error(b"A.B: x\r\n").0, 1);
    assert_eq!(parse_error(b": x\r\n").0, 1);
    assert_eq!(parse_error(b"A: caf\xe9\r\n").0, 1);
}

#[test]
fn unwritable_values_are_errors() {
    let mut m = Manifest::new();
    m.set("A", "line\nbreak");
    assert!(matches!(m.to_bytes(), Err(Error::ManifestValue { .. })));
    let mut m = Manifest::new();
    m.main.push(Attribute::new("not valid", "x"));
    assert!(matches!(m.to_bytes(), Err(Error::ManifestValue { .. })));
    let mut m = Manifest::new();
    m.sections.push(Section::new("a\rb"));
    assert!(matches!(m.to_bytes(), Err(Error::ManifestValue { .. })));
}

#[test]
fn jar_manifest_helpers() {
    let mut jar = Jar::from_entries(vec![Entry::file("a.txt", b"a".to_vec())]);
    assert_eq!(jar.manifest().unwrap(), None);
    let mut m = Manifest::new();
    m.set("Manifest-Version", "1.0");
    jar.set_manifest(&m).unwrap();
    assert_eq!(jar.entries[0].name, MANIFEST_PATH);
    assert_eq!(jar.manifest().unwrap(), Some(m.clone()));
    m.set("Main-Class", "net.example.Main");
    jar.set_manifest(&m).unwrap();
    assert_eq!(jar.entries.len(), 2);
    let back = Jar::read(&jar.write(&WriteOptions::default()).unwrap()).unwrap();
    assert_eq!(
        back.manifest().unwrap().unwrap().get("Main-Class"),
        Some("net.example.Main")
    );
}

fn attribute() -> impl Strategy<Value = Attribute> {
    ("[A-Za-z0-9_-]{1,70}", "[^\r\n\0]{0,160}").prop_map(|(n, v)| Attribute::new(n, v))
}

fn manifest() -> impl Strategy<Value = Manifest> {
    (
        "[0-9.]{1,6}",
        prop::collection::vec(attribute(), 0..8),
        prop::collection::vec(
            ("[^\r\n\0]{0,120}", prop::collection::vec(attribute(), 0..4)),
            0..5,
        ),
    )
        .prop_map(|(version, rest, sections)| {
            let mut m = Manifest::new();
            m.main.push(Attribute::new("Manifest-Version", version));
            m.main.extend(rest);
            m.sections = sections
                .into_iter()
                .map(|(name, attributes)| Section { name, attributes })
                .collect();
            m
        })
}

proptest! {
    #[test]
    fn write_then_parse_round_trips(m in manifest()) {
        let bytes = m.to_bytes().unwrap();
        for line in bytes.split(|&b| b == b'\n') {
            let line = line.strip_suffix(b"\r").unwrap_or(line);
            prop_assert!(line.len() <= MAX_LINE_BYTES);
        }
        let back = Manifest::parse(&bytes).unwrap();
        prop_assert_eq!(&back, &m);
        prop_assert_eq!(back.to_bytes().unwrap(), bytes);
    }

    #[test]
    fn parse_never_panics(bytes in prop::collection::vec(any::<u8>(), 0..400)) {
        let _ = Manifest::parse(&bytes);
    }
}
