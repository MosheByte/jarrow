//! Shared helpers for the text formats.

use crate::error::{Error, Result};

/// UTF-8 text from bytes, without a leading byte-order mark.
pub(crate) fn utf8(bytes: &[u8]) -> Result<&str> {
    let text = std::str::from_utf8(bytes).map_err(|e| Error::NotUtf8 {
        offset: e.valid_up_to(),
    })?;
    Ok(strip_bom(text))
}

pub(crate) fn strip_bom(text: &str) -> &str {
    text.strip_prefix('\u{feff}').unwrap_or(text)
}

/// Lines with 1-based numbers, `\r\n` and `\n` both accepted. A final newline does not make
/// an extra line.
pub(crate) fn numbered_lines(text: &str) -> impl Iterator<Item = (usize, &str)> {
    let text = strip_bom(text);
    let text = text.strip_suffix('\n').unwrap_or(text);
    let empty = text.is_empty();
    text.split('\n')
        .enumerate()
        .filter(move |_| !empty)
        .map(|(i, line)| (i + 1, line.strip_suffix('\r').unwrap_or(line)))
}

/// The number of leading tab characters.
pub(crate) fn tab_depth(line: &str) -> usize {
    line.bytes().take_while(|&b| b == b'\t').count()
}

/// Parses a non-negative decimal integer that fits in `u32`.
pub(crate) fn parse_u32(s: &str) -> Option<u32> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_and_numbers() {
        let lines: Vec<_> = numbered_lines("\u{feff}a\r\nb\n\nc\n").collect();
        assert_eq!(lines, [(1, "a"), (2, "b"), (3, ""), (4, "c")]);
        assert_eq!(numbered_lines("").count(), 0);
        assert_eq!(numbered_lines("\n").count(), 0);
        assert_eq!(tab_depth("\t\tx"), 2);
        assert_eq!(parse_u32("42"), Some(42));
        assert_eq!(parse_u32("-1"), None);
        assert_eq!(parse_u32("+1"), None);
        assert_eq!(parse_u32("99999999999"), None);
        assert!(utf8(&[0xff]).is_err());
        assert_eq!(utf8(b"\xef\xbb\xbfok").unwrap(), "ok");
    }
}
