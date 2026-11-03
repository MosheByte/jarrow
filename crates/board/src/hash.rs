//! SHA-256 and SHA-512 as lowercase hex.

use sha2::{Digest, Sha256, Sha512};

/// Lowercase hex of `bytes`.
pub fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(char::from(DIGITS[usize::from(byte >> 4)]));
        out.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    out
}

/// SHA-256 of `data`, lowercase hex.
pub fn sha256_hex(data: &[u8]) -> String {
    hex(&Sha256::digest(data))
}

/// SHA-512 of `data`, lowercase hex.
pub fn sha512_hex(data: &[u8]) -> String {
    hex(&Sha512::digest(data))
}

/// Whether `s` is lowercase hex of exactly `len` characters.
pub fn is_hex(s: &str, len: usize) -> bool {
    s.len() == len && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// A SHA-512 computed while bytes stream through it.
#[derive(Default)]
pub struct Sha512Stream {
    inner: Sha512,
}

impl Sha512Stream {
    /// A new, empty hasher.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds bytes.
    pub fn update(&mut self, data: &[u8]) {
        self.inner.update(data);
    }

    /// The digest as lowercase hex.
    pub fn finish_hex(self) -> String {
        hex(&self.inner.finalize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_digests() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            sha512_hex(b"abc"),
            "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a\
             2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f"
        );
    }

    #[test]
    fn streaming_matches_one_shot() {
        let mut stream = Sha512Stream::new();
        stream.update(b"a");
        stream.update(b"bc");
        assert_eq!(stream.finish_hex(), sha512_hex(b"abc"));
    }

    #[test]
    fn hex_checks() {
        assert!(is_hex(&sha256_hex(b"x"), 64));
        assert!(!is_hex("ABCDEF", 6));
        assert!(!is_hex("abc", 4));
        assert_eq!(hex(&[0x00, 0x0f, 0xf0, 0xff]), "000ff0ff");
    }
}
