//! Modified UTF-8, the string encoding of `CONSTANT_Utf8` entries (JVMS 4.4.7).
//!
//! It differs from UTF-8 in two ways: U+0000 is written as the two bytes `C0 80`, and a
//! character outside the Basic Multilingual Plane is written as its UTF-16 surrogate pair,
//! three bytes per surrogate (six in all). Java strings are UTF-16, so a class file can also
//! hold a lone surrogate, which no Rust `str` can.
//!
//! Three levels of decoding:
//!
//! - [`decode`] is strict: it returns the text, or an error for a lone surrogate or for
//!   bytes that are not canonical Modified UTF-8.
//! - [`decode_lossless`] never fails: it splits the bytes into [`Piece::Text`] runs and
//!   [`Piece::Raw`] runs (lone surrogates and malformed bytes, kept as they were), and
//!   [`encode_pieces`] turns the pieces back into exactly the same bytes, for any input.
//! - [`decode_lossy`] replaces every raw run with U+FFFD, for display only.
//!
//! [`encode`] writes a `str` as canonical Modified UTF-8, and `decode(&encode(s)) == s`.

use std::borrow::Cow;
use std::fmt;

/// Why bytes are not a valid Modified UTF-8 string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Mutf8Error {
    /// A surrogate (U+D800..U+DFFF) that is not half of a high-low pair. It is valid in a
    /// class file but has no `str` form; [`decode_lossless`] keeps it as raw bytes.
    LoneSurrogate {
        /// Byte offset of the surrogate in the string.
        offset: usize,
    },
    /// Bytes that are not canonical Modified UTF-8: a zero byte, a stray continuation byte,
    /// a byte of `F0` or above, a truncated sequence, or an overlong encoding other than
    /// `C0 80`.
    Malformed {
        /// Byte offset of the first bad byte.
        offset: usize,
    },
}

impl fmt::Display for Mutf8Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LoneSurrogate { offset } => write!(f, "lone surrogate at byte {offset}"),
            Self::Malformed { offset } => write!(f, "malformed Modified UTF-8 at byte {offset}"),
        }
    }
}

impl std::error::Error for Mutf8Error {}

/// One run of a losslessly decoded string; see [`decode_lossless`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Piece {
    /// Characters, each of which was canonically encoded in the input.
    Text(String),
    /// Bytes kept exactly as they were: lone surrogates (three bytes each) and malformed
    /// sequences.
    Raw(Vec<u8>),
}

/// What one step of the scanner found.
enum Unit {
    /// A character and the number of bytes its canonical encoding took.
    Char(char, usize),
    /// A surrogate code unit, encoded canonically in three bytes.
    Surrogate(u16),
    /// A malformed sequence of this many bytes.
    Invalid(usize),
}

fn is_continuation(byte: Option<&u8>) -> Option<u32> {
    match byte {
        Some(&b) if b & 0xC0 == 0x80 => Some(u32::from(b & 0x3F)),
        _ => None,
    }
}

/// Scans the unit that starts at `bytes[0]`; `bytes` must not be empty.
fn scan(bytes: &[u8]) -> Unit {
    let Some(&b0) = bytes.first() else {
        return Unit::Invalid(1);
    };
    match b0 {
        0x01..=0x7F => Unit::Char(char::from(b0), 1),
        0xC0..=0xDF => {
            let Some(c1) = is_continuation(bytes.get(1)) else {
                return Unit::Invalid(1);
            };
            let value = (u32::from(b0 & 0x1F) << 6) | c1;
            match char::from_u32(value) {
                // U+0000 is the one character Modified UTF-8 writes in two bytes on purpose.
                Some(c) if value == 0 || value >= 0x80 => Unit::Char(c, 2),
                _ => Unit::Invalid(2),
            }
        }
        0xE0..=0xEF => {
            let (Some(c1), Some(c2)) =
                (is_continuation(bytes.get(1)), is_continuation(bytes.get(2)))
            else {
                return Unit::Invalid(1);
            };
            let value = (u32::from(b0 & 0x0F) << 12) | (c1 << 6) | c2;
            if value < 0x800 {
                Unit::Invalid(3)
            } else if (0xD800..=0xDFFF).contains(&value) {
                // value < 0x10000, so the conversion cannot fail.
                Unit::Surrogate(u16::try_from(value).unwrap_or(0xD800))
            } else {
                match char::from_u32(value) {
                    Some(c) => Unit::Char(c, 3),
                    None => Unit::Invalid(3),
                }
            }
        }
        // 0x00 (U+0000 must be C0 80), stray continuation bytes and 0xF0.. (four-byte UTF-8,
        // which Modified UTF-8 replaces with surrogate pairs).
        _ => Unit::Invalid(1),
    }
}

/// Decodes the unit at the start of `bytes`: the character, or `None` for a raw run, and
/// the number of bytes consumed. A high surrogate followed by a low one is one character.
fn next(bytes: &[u8]) -> (Option<char>, usize) {
    match scan(bytes) {
        Unit::Char(c, len) => (Some(c), len),
        Unit::Surrogate(high @ 0xD800..=0xDBFF) => {
            if let Some(rest) = bytes.get(3..)
                && let Unit::Surrogate(low @ 0xDC00..=0xDFFF) = scan_nonempty(rest)
                && let Some(c) = char::from_u32(
                    0x10000 + ((u32::from(high) - 0xD800) << 10) + (u32::from(low) - 0xDC00),
                )
            {
                return (Some(c), 6);
            }
            (None, 3)
        }
        Unit::Surrogate(_) => (None, 3),
        Unit::Invalid(len) => (None, len),
    }
}

fn scan_nonempty(bytes: &[u8]) -> Unit {
    if bytes.is_empty() {
        Unit::Invalid(0)
    } else {
        scan(bytes)
    }
}

/// Splits `bytes` into text and raw runs without losing a byte.
///
/// `encode_pieces(&decode_lossless(b)) == b` for every byte string `b`. Adjacent runs of
/// the same kind are merged and no run is empty, so canonical input without lone
/// surrogates gives at most one [`Piece::Text`].
pub fn decode_lossless(bytes: &[u8]) -> Vec<Piece> {
    let mut pieces: Vec<Piece> = Vec::new();
    let mut pos = 0;
    while let Some(rest) = bytes.get(pos..).filter(|r| !r.is_empty()) {
        let (ch, len) = next(rest);
        let len = len.clamp(1, rest.len());
        match (ch, pieces.last_mut()) {
            (Some(c), Some(Piece::Text(text))) => text.push(c),
            (Some(c), _) => pieces.push(Piece::Text(c.to_string())),
            (None, Some(Piece::Raw(raw))) => raw.extend_from_slice(rest.get(..len).unwrap_or(rest)),
            (None, _) => pieces.push(Piece::Raw(rest.get(..len).unwrap_or(rest).to_vec())),
        }
        pos += len;
    }
    pieces
}

/// Joins pieces from [`decode_lossless`] back into Modified UTF-8 bytes.
pub fn encode_pieces(pieces: &[Piece]) -> Vec<u8> {
    let mut out = Vec::new();
    for piece in pieces {
        match piece {
            Piece::Text(text) => encode_into(text, &mut out),
            Piece::Raw(raw) => out.extend_from_slice(raw),
        }
    }
    out
}

/// Decodes canonical Modified UTF-8.
///
/// Borrows when the bytes are already the UTF-8 form of the same text (no U+0000 and no
/// supplementary characters), which covers almost every class, method and descriptor name.
pub fn decode(bytes: &[u8]) -> Result<Cow<'_, str>, Mutf8Error> {
    // Valid UTF-8 without a zero byte and without four-byte sequences is canonical Modified
    // UTF-8 with the same meaning: from_utf8 already rejects C0 80, overlongs and surrogates.
    if !bytes.iter().any(|&b| b == 0 || b >= 0xF0)
        && let Ok(text) = std::str::from_utf8(bytes)
    {
        return Ok(Cow::Borrowed(text));
    }
    let mut out = String::with_capacity(bytes.len());
    let mut pos = 0;
    while let Some(rest) = bytes.get(pos..).filter(|r| !r.is_empty()) {
        match next(rest) {
            (Some(c), len) => {
                out.push(c);
                pos += len;
            }
            (None, _) => {
                return Err(match scan(rest) {
                    Unit::Surrogate(_) => Mutf8Error::LoneSurrogate { offset: pos },
                    _ => Mutf8Error::Malformed { offset: pos },
                });
            }
        }
    }
    Ok(Cow::Owned(out))
}

/// Decodes for display: like [`decode_lossless`], with each raw run shown as U+FFFD.
pub fn decode_lossy(bytes: &[u8]) -> Cow<'_, str> {
    if let Ok(text) = decode(bytes) {
        return text;
    }
    let mut out = String::with_capacity(bytes.len());
    for piece in decode_lossless(bytes) {
        match piece {
            Piece::Text(text) => out.push_str(&text),
            Piece::Raw(_) => out.push(char::REPLACEMENT_CHARACTER),
        }
    }
    Cow::Owned(out)
}

/// Encodes `text` as canonical Modified UTF-8.
pub fn encode(text: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(text.len());
    encode_into(text, &mut out);
    out
}

/// Appends the canonical Modified UTF-8 form of `text` to `out`.
pub fn encode_into(text: &str, out: &mut Vec<u8>) {
    for c in text.chars() {
        let value = u32::from(c);
        match value {
            0 => out.extend_from_slice(&[0xC0, 0x80]),
            0x01..=0x7F => out.push(value as u8),
            0x80..=0x7FF => {
                out.push(0xC0 | (value >> 6) as u8);
                out.push(0x80 | (value & 0x3F) as u8);
            }
            0x800..=0xFFFF => push_three(out, value),
            _ => {
                let v = value - 0x10000;
                push_three(out, 0xD800 | (v >> 10));
                push_three(out, 0xDC00 | (v & 0x3FF));
            }
        }
    }
}

fn push_three(out: &mut Vec<u8>, unit: u32) {
    out.push(0xE0 | ((unit >> 12) & 0x0F) as u8);
    out.push(0x80 | ((unit >> 6) & 0x3F) as u8);
    out.push(0x80 | (unit & 0x3F) as u8);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nul_and_supplementary_characters() {
        assert_eq!(encode("\0"), [0xC0, 0x80]);
        assert_eq!(encode("\u{1F600}"), [0xED, 0xA0, 0xBD, 0xED, 0xB8, 0x80]);
        assert_eq!(
            decode(&[0xED, 0xA0, 0xBD, 0xED, 0xB8, 0x80]).unwrap(),
            "\u{1F600}"
        );
        assert_eq!(decode(&[0x41, 0xC0, 0x80, 0x42]).unwrap(), "A\0B");
    }

    #[test]
    fn plain_names_borrow() {
        assert!(matches!(
            decode(b"java/lang/Object").unwrap(),
            Cow::Borrowed(_)
        ));
        assert!(matches!(
            decode("caf\u{e9}".as_bytes()).unwrap(),
            Cow::Borrowed(_)
        ));
    }

    #[test]
    fn lone_surrogates_are_kept_raw() {
        let high = [0xED, 0xA0, 0x80];
        assert_eq!(decode(&high), Err(Mutf8Error::LoneSurrogate { offset: 0 }));
        let mixed = [b'a', 0xED, 0xB0, 0x80, b'b'];
        let pieces = decode_lossless(&mixed);
        assert_eq!(
            pieces,
            vec![
                Piece::Text("a".into()),
                Piece::Raw(vec![0xED, 0xB0, 0x80]),
                Piece::Text("b".into())
            ]
        );
        assert_eq!(encode_pieces(&pieces), mixed);
        assert_eq!(decode_lossy(&mixed), "a\u{FFFD}b");
    }

    #[test]
    fn malformed_bytes_are_errors_but_round_trip() {
        for bad in [
            &[0x00][..],
            &[0x80],
            &[0xF0, 0x9F, 0x98, 0x80],
            &[0xC1, 0x81],
            &[0xE0, 0x80],
        ] {
            assert!(
                matches!(decode(bad), Err(Mutf8Error::Malformed { .. })),
                "{bad:?}"
            );
            assert_eq!(encode_pieces(&decode_lossless(bad)), bad);
        }
    }
}
