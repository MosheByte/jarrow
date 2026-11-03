//! `META-INF/MANIFEST.MF`: parse and write.
//!
//! The format is the JAR File Specification's: a main section, then individual sections,
//! separated by blank lines. Each header is `Name: value`; a line holds at most 72 bytes,
//! and a longer header continues on lines that start with one space. Line endings may be
//! CR LF, LF or CR on input; the writer uses CR LF, as the JDK does.
//!
//! Parsing is lossless for everything the JVM reads: attribute order, repeated attributes
//! and repeated sections are kept as they are. Attribute names compare ASCII
//! case-insensitively, as in `java.util.jar.Attributes`.

use crate::error::{Error, Result};

/// Where the manifest lives in a JAR.
pub const MANIFEST_PATH: &str = "META-INF/MANIFEST.MF";

/// Longest line the writer emits, in bytes, line ending excluded.
pub const MAX_LINE_BYTES: usize = 72;

/// Longest attribute name the JAR specification allows.
const MAX_NAME_BYTES: usize = 70;

/// One `Name: value` header.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Attribute {
    /// The header name, for example `Main-Class`.
    pub name: String,
    /// The value, continuation lines already joined.
    pub value: String,
}

impl Attribute {
    /// A new attribute.
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Attribute {
        Attribute {
            name: name.into(),
            value: value.into(),
        }
    }
}

/// An individual section: the attributes for one entry, introduced by `Name: <path>`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct Section {
    /// The value of the section's `Name` header (an entry path or a package path).
    pub name: String,
    /// The attributes after the `Name` header, in order.
    pub attributes: Vec<Attribute>,
}

impl Section {
    /// A section with no attributes yet.
    pub fn new(name: impl Into<String>) -> Section {
        Section {
            name: name.into(),
            attributes: Vec::new(),
        }
    }

    /// The value of attribute `name` (last one wins, as in the JDK).
    pub fn get(&self, name: &str) -> Option<&str> {
        get(&self.attributes, name)
    }

    /// Sets attribute `name`, replacing the first existing one and dropping repeats.
    pub fn set(&mut self, name: &str, value: impl Into<String>) {
        set(&mut self.attributes, name, value.into());
    }

    /// Removes every attribute called `name`; returns whether one existed.
    pub fn remove(&mut self, name: &str) -> bool {
        remove(&mut self.attributes, name)
    }
}

/// A parsed `MANIFEST.MF`.
///
/// ```
/// use jarrow_jar::Manifest;
/// let text = b"Manifest-Version: 1.0\r\nMain-Class: net.example.Main\r\n\r\n";
/// let manifest = Manifest::parse(text).unwrap();
/// assert_eq!(manifest.get("main-class"), Some("net.example.Main"));
/// assert_eq!(manifest.to_bytes().unwrap(), text);
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct Manifest {
    /// Main attributes, in order.
    pub main: Vec<Attribute>,
    /// Individual sections, in order.
    pub sections: Vec<Section>,
}

fn get<'a>(attributes: &'a [Attribute], name: &str) -> Option<&'a str> {
    attributes
        .iter()
        .rev()
        .find(|a| a.name.eq_ignore_ascii_case(name))
        .map(|a| a.value.as_str())
}

fn set(attributes: &mut Vec<Attribute>, name: &str, value: String) {
    match attributes
        .iter()
        .position(|a| a.name.eq_ignore_ascii_case(name))
    {
        Some(first) => {
            attributes[first].value = value;
            let mut index = 0;
            attributes.retain(|a| {
                let keep = index <= first || !a.name.eq_ignore_ascii_case(name);
                index += 1;
                keep
            });
        }
        None => attributes.push(Attribute::new(name, value)),
    }
}

fn remove(attributes: &mut Vec<Attribute>, name: &str) -> bool {
    let before = attributes.len();
    attributes.retain(|a| !a.name.eq_ignore_ascii_case(name));
    attributes.len() != before
}

/// Whether `name` is a valid header name: 1 to 70 bytes of ASCII letters, digits, `-`, `_`.
pub fn is_valid_attribute_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= MAX_NAME_BYTES
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// A header line after continuation lines are joined, with the line it started on.
struct Logical {
    line: usize,
    bytes: Vec<u8>,
}

/// Physical lines (terminator removed), 1-based line numbers.
fn split_lines(bytes: &[u8]) -> Vec<(usize, &[u8])> {
    let mut lines = Vec::new();
    let mut start = 0;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\n' => {
                lines.push((lines.len() + 1, &bytes[start..i]));
                i += 1;
                start = i;
            }
            b'\r' => {
                lines.push((lines.len() + 1, &bytes[start..i]));
                i += if bytes.get(i + 1) == Some(&b'\n') {
                    2
                } else {
                    1
                };
                start = i;
            }
            _ => i += 1,
        }
    }
    if start < bytes.len() {
        lines.push((lines.len() + 1, &bytes[start..]));
    }
    lines
}

fn parse_header(logical: &Logical) -> Result<Attribute> {
    let err = |what: &str| Error::Manifest {
        line: logical.line,
        what: what.to_string(),
    };
    let colon = logical
        .bytes
        .iter()
        .position(|&b| b == b':')
        .ok_or_else(|| err("header has no ':'"))?;
    if logical.bytes.get(colon + 1) != Some(&b' ') {
        return Err(err("':' in a header must be followed by a space"));
    }
    let name = core::str::from_utf8(&logical.bytes[..colon])
        .map_err(|_| err("header name is not UTF-8"))?;
    if !is_valid_attribute_name(name) {
        return Err(err(&format!(
            "invalid header name {name:?} (1 to 70 ASCII letters, digits, '-' or '_')"
        )));
    }
    let value = core::str::from_utf8(&logical.bytes[colon + 2..])
        .map_err(|_| err("header value is not UTF-8"))?;
    Ok(Attribute::new(name, value))
}

fn write_wrapped(out: &mut Vec<u8>, line: &str) {
    let bytes = line.as_bytes();
    let mut start = 0;
    let mut room = MAX_LINE_BYTES;
    while bytes.len() - start > room {
        let mut cut = start + room;
        // Never split a UTF-8 sequence, so every line stays valid UTF-8 on its own.
        while !line.is_char_boundary(cut) {
            cut -= 1;
        }
        out.extend_from_slice(&bytes[start..cut]);
        out.extend_from_slice(b"\r\n ");
        start = cut;
        room = MAX_LINE_BYTES - 1;
    }
    out.extend_from_slice(&bytes[start..]);
    out.extend_from_slice(b"\r\n");
}

fn check_value(what: &str, value: &str) -> Result<()> {
    if value.contains(['\r', '\n', '\0']) {
        return Err(Error::ManifestValue {
            what: format!("{what} contains a line break or NUL"),
        });
    }
    Ok(())
}

fn write_attribute(out: &mut Vec<u8>, attribute: &Attribute) -> Result<()> {
    if !is_valid_attribute_name(&attribute.name) {
        return Err(Error::ManifestValue {
            what: format!("invalid header name {:?}", attribute.name),
        });
    }
    check_value(
        &format!("the value of {:?}", attribute.name),
        &attribute.value,
    )?;
    write_wrapped(out, &format!("{}: {}", attribute.name, attribute.value));
    Ok(())
}

impl Manifest {
    /// An empty manifest (no attributes, no sections).
    pub fn new() -> Manifest {
        Manifest::default()
    }

    /// Parses manifest bytes.
    ///
    /// Errors: a header without `": "`, an invalid header name, a continuation line with
    /// no header before it, a section that does not start with `Name`, or text that is not
    /// UTF-8 (checked after continuation lines are joined, as the JDK joins bytes).
    pub fn parse(bytes: &[u8]) -> Result<Manifest> {
        // groups[0] is the main section; every blank line closes the current group.
        let mut groups: Vec<Vec<Logical>> = vec![Vec::new()];
        let mut open = true;
        for (line, text) in split_lines(bytes) {
            if text.is_empty() {
                open = false;
                continue;
            }
            if !open {
                groups.push(Vec::new());
                open = true;
            }
            let Some(group) = groups.last_mut() else {
                continue; // never empty: it starts with the main group
            };
            if let Some(rest) = text.strip_prefix(b" ") {
                match group.last_mut() {
                    Some(previous) => previous.bytes.extend_from_slice(rest),
                    None => {
                        return Err(Error::Manifest {
                            line,
                            what: "continuation line with no header before it".to_string(),
                        });
                    }
                }
            } else {
                group.push(Logical {
                    line,
                    bytes: text.to_vec(),
                });
            }
        }

        let mut manifest = Manifest::new();
        let mut groups = groups.iter();
        for logical in groups.next().into_iter().flatten() {
            manifest.main.push(parse_header(logical)?);
        }
        for group in groups {
            let mut headers = group.iter();
            let Some(first) = headers.next() else {
                continue;
            };
            let first_header = parse_header(first)?;
            if !first_header.name.eq_ignore_ascii_case("Name") {
                return Err(Error::Manifest {
                    line: first.line,
                    what: format!(
                        "an individual section must start with 'Name', not {:?}",
                        first_header.name
                    ),
                });
            }
            let mut section = Section::new(first_header.value);
            for logical in headers {
                section.attributes.push(parse_header(logical)?);
            }
            manifest.sections.push(section);
        }
        Ok(manifest)
    }

    /// Writes the manifest: CR LF line endings, lines of at most [`MAX_LINE_BYTES`] bytes
    /// (never splitting a UTF-8 sequence), `Manifest-Version` (or, without it,
    /// `Signature-Version`) first in the main section as the specification orders it,
    /// everything else in order, and a blank line after every section.
    ///
    /// Errors: an invalid attribute name, or a name or value with CR, LF or NUL in it.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        let version = self
            .main
            .iter()
            .position(|a| a.name.eq_ignore_ascii_case("Manifest-Version"))
            .or_else(|| {
                self.main
                    .iter()
                    .position(|a| a.name.eq_ignore_ascii_case("Signature-Version"))
            });
        if let Some(index) = version {
            write_attribute(&mut out, &self.main[index])?;
        }
        for (index, attribute) in self.main.iter().enumerate() {
            if Some(index) != version {
                write_attribute(&mut out, attribute)?;
            }
        }
        out.extend_from_slice(b"\r\n");
        for section in &self.sections {
            check_value("a section name", &section.name)?;
            write_wrapped(&mut out, &format!("Name: {}", section.name));
            for attribute in &section.attributes {
                write_attribute(&mut out, attribute)?;
            }
            out.extend_from_slice(b"\r\n");
        }
        Ok(out)
    }

    /// The value of main attribute `name` (last one wins, as in the JDK).
    pub fn get(&self, name: &str) -> Option<&str> {
        get(&self.main, name)
    }

    /// Sets main attribute `name`, replacing the first existing one and dropping repeats.
    pub fn set(&mut self, name: &str, value: impl Into<String>) {
        set(&mut self.main, name, value.into());
    }

    /// Removes every main attribute called `name`; returns whether one existed.
    pub fn remove(&mut self, name: &str) -> bool {
        remove(&mut self.main, name)
    }

    /// The first individual section called `name` (section names are case-sensitive paths).
    pub fn section(&self, name: &str) -> Option<&Section> {
        self.sections.iter().find(|s| s.name == name)
    }

    /// The first individual section called `name`, mutably.
    pub fn section_mut(&mut self, name: &str) -> Option<&mut Section> {
        self.sections.iter_mut().find(|s| s.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_lines_handles_every_terminator() {
        let lines: Vec<_> = split_lines(b"a\r\nb\nc\rd")
            .into_iter()
            .map(|(n, l)| (n, l.to_vec()))
            .collect();
        assert_eq!(
            lines,
            [
                (1, b"a".to_vec()),
                (2, b"b".to_vec()),
                (3, b"c".to_vec()),
                (4, b"d".to_vec())
            ]
        );
        assert_eq!(split_lines(b"a\r\n\r\n").len(), 2);
    }

    #[test]
    fn set_replaces_first_and_drops_repeats() {
        let mut m = Manifest::new();
        m.main.push(Attribute::new("A", "1"));
        m.main.push(Attribute::new("B", "2"));
        m.main.push(Attribute::new("a", "3"));
        assert_eq!(m.get("A"), Some("3"));
        m.set("A", "4");
        assert_eq!(m.main, [Attribute::new("A", "4"), Attribute::new("B", "2")]);
        assert!(m.remove("b"));
        assert!(!m.remove("b"));
        m.set("C", "5");
        assert_eq!(m.main.last(), Some(&Attribute::new("C", "5")));
    }

    #[test]
    fn attribute_names() {
        assert!(is_valid_attribute_name("Manifest-Version"));
        assert!(is_valid_attribute_name("SHA-256-Digest"));
        assert!(is_valid_attribute_name("a_b"));
        assert!(!is_valid_attribute_name(""));
        assert!(!is_valid_attribute_name("a b"));
        assert!(!is_valid_attribute_name("a.b"));
        assert!(!is_valid_attribute_name(&"x".repeat(71)));
        assert!(is_valid_attribute_name(&"x".repeat(70)));
    }
}
