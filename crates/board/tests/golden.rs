//! Golden bytes: the merge's binary output must be identical on Linux, macOS and Windows.

use jarrow_board::hash::sha256_hex;
use jarrow_board::merge::reports_tarball;

#[test]
fn reports_tarball_bytes_are_pinned() {
    let reports = vec![
        (
            "reports/fabric-neoforge/AAAAAAA1/1.21.11.json".to_owned(),
            br#"{"schema":1,"outcome":"red"}"#.to_vec(),
        ),
        (
            "reports/fabric-neoforge/AAAAAAA1/1.21.1.json".to_owned(),
            br#"{"schema":1,"outcome":"red","reason_kind":"no_converter"}"#.to_vec(),
        ),
    ];
    let bytes = reports_tarball(&reports).unwrap();
    assert_eq!(
        sha256_hex(&bytes),
        "89eb877f26773e0f855089059ac6d7a8ba28033e04b3cb66a261e4b7d4b46896"
    );
}
