//! Signature stripping: signature files and per-entry digests go, everything else stays,
//! and the report says exactly what was removed.

use jarrow_jar::{Entry, Error, Jar, MANIFEST_PATH, RemovedDigest, WriteOptions};

const SIGNED_MANIFEST: &[u8] = b"Manifest-Version: 1.0\r\n\
Created-By: 21.0.5 (Eclipse Adoptium)\r\n\
\r\n\
Name: net/example/A.class\r\n\
SHA-256-Digest: 47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU=\r\n\
\r\n\
Name: net/example/\r\n\
Sealed: true\r\n\
\r\n\
Name: net/example/B.class\r\n\
SHA-256-Digest: ypeBEsobvcr6wjGzmiPcTaeG7/gUfE5yuYB3ha/uSLs=\r\n\
SHA1-Digest: 2jmj7l5rSw0yVb/vlWAYkK/YBwk=\r\n\
Implementation-Title: kept\r\n\
\r\n";

fn signed_jar() -> Jar {
    Jar::from_entries(vec![
        Entry::directory("META-INF"),
        Entry::file(MANIFEST_PATH, SIGNED_MANIFEST.to_vec()),
        Entry::file(
            "META-INF/SIGNER.SF",
            b"Signature-Version: 1.0\r\n\r\n".to_vec(),
        ),
        Entry::file("META-INF/SIGNER.RSA", vec![0x30, 0x82]),
        Entry::file("META-INF/other.dsa", vec![0x30]),
        Entry::file("META-INF/K.EC", vec![0x30]),
        Entry::file("META-INF/SIG-EXTRA", vec![0x30]),
        Entry::file("META-INF/sub/KEEP.SF", b"not a signature".to_vec()),
        Entry::file("META-INF/services/net.example.Service", b"impl".to_vec()),
        Entry::directory("net/example"),
        Entry::file("net/example/A.class", vec![0xCA, 0xFE]),
        Entry::file("net/example/B.class", vec![0xCA, 0xFE]),
    ])
}

#[test]
fn strips_files_and_digests_and_reports_them() {
    let mut jar = signed_jar();
    assert!(jar.is_signed());
    let report = jar.strip_signatures().unwrap();
    assert_eq!(
        report.files,
        [
            "META-INF/SIGNER.SF",
            "META-INF/SIGNER.RSA",
            "META-INF/other.dsa",
            "META-INF/K.EC",
            "META-INF/SIG-EXTRA"
        ]
    );
    let digest = |section: &str, attribute: &str| RemovedDigest {
        section: section.into(),
        attribute: attribute.into(),
    };
    assert_eq!(
        report.digests,
        [
            digest("net/example/A.class", "SHA-256-Digest"),
            digest("net/example/B.class", "SHA-256-Digest"),
            digest("net/example/B.class", "SHA1-Digest"),
        ]
    );
    assert_eq!(report.sections, ["net/example/A.class"]);
    assert!(!report.is_empty());
    assert!(!jar.is_signed());

    assert_eq!(
        jar.names().collect::<Vec<_>>(),
        [
            "META-INF/",
            MANIFEST_PATH,
            "META-INF/sub/KEEP.SF",
            "META-INF/services/net.example.Service",
            "net/example/",
            "net/example/A.class",
            "net/example/B.class",
        ]
    );
    assert_eq!(
        jar.get(MANIFEST_PATH).unwrap().data,
        b"Manifest-Version: 1.0\r\n\
Created-By: 21.0.5 (Eclipse Adoptium)\r\n\
\r\n\
Name: net/example/\r\n\
Sealed: true\r\n\
\r\n\
Name: net/example/B.class\r\n\
Implementation-Title: kept\r\n\
\r\n"
    );
}

#[test]
fn stripping_twice_removes_nothing_more() {
    let mut jar = signed_jar();
    jar.strip_signatures().unwrap();
    let after_first = jar.clone();
    let report = jar.strip_signatures().unwrap();
    assert!(report.is_empty());
    assert_eq!(jar, after_first);
}

#[test]
fn unsigned_manifest_bytes_are_left_alone() {
    // LF line endings and no Manifest-Version: rewriting would change both
    let manifest = b"Created-By: hand\nName-Like: x\n".to_vec();
    let mut jar = Jar::from_entries(vec![
        Entry::file(MANIFEST_PATH, manifest.clone()),
        Entry::file("a.txt", b"a".to_vec()),
    ]);
    assert!(!jar.is_signed());
    assert!(jar.strip_signatures().unwrap().is_empty());
    assert_eq!(jar.get(MANIFEST_PATH).unwrap().data, manifest);
}

#[test]
fn signature_files_without_a_manifest_are_still_removed() {
    let mut jar = Jar::from_entries(vec![
        Entry::file("META-INF/A.SF", b"x".to_vec()),
        Entry::file("a.txt", b"a".to_vec()),
    ]);
    let report = jar.strip_signatures().unwrap();
    assert_eq!(report.files, ["META-INF/A.SF"]);
    assert_eq!(jar.names().collect::<Vec<_>>(), ["a.txt"]);
}

#[test]
fn malformed_manifest_is_an_error_and_changes_nothing() {
    let mut jar = Jar::from_entries(vec![
        Entry::file(MANIFEST_PATH, b"no colon here\r\n".to_vec()),
        Entry::file("META-INF/A.SF", b"x".to_vec()),
    ]);
    let before = jar.clone();
    assert!(matches!(
        jar.strip_signatures(),
        Err(Error::Manifest { line: 1, .. })
    ));
    assert_eq!(jar, before);
}

#[test]
fn stripped_jar_writes_and_reads_back_unsigned() {
    let mut jar = signed_jar();
    jar.strip_signatures().unwrap();
    let back = Jar::read(&jar.write(&WriteOptions::default()).unwrap()).unwrap();
    assert!(!back.is_signed());
    assert_eq!(back.entries[0].name, MANIFEST_PATH);
    let manifest = back.manifest().unwrap().unwrap();
    assert!(manifest.section("net/example/A.class").is_none());
    assert_eq!(
        manifest
            .section("net/example/B.class")
            .unwrap()
            .get("Implementation-Title"),
        Some("kept")
    );
}
