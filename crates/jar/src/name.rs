//! Entry-name safety checks, shared by the reader and the writer.

use crate::error::UnsafeName;

/// Checks that `name` cannot escape or alias the directory a JAR is extracted to.
///
/// A name ending in `/` is a directory; that one trailing slash is allowed. Components are
/// separated by `/` only. Refused: empty names, absolute paths, drive letters, backslashes,
/// NUL bytes, `..` components, components made only of dots, and empty components.
/// Dots inside a component (`lang/en_us..json`) are fine.
pub fn check_name(name: &str) -> Result<(), UnsafeName> {
    if name.is_empty() {
        return Err(UnsafeName::Empty);
    }
    if name.contains('\0') {
        return Err(UnsafeName::Nul);
    }
    if name.contains('\\') {
        return Err(UnsafeName::Backslash);
    }
    if name.starts_with('/') {
        return Err(UnsafeName::Absolute);
    }
    let bytes = name.as_bytes();
    if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        return Err(UnsafeName::DriveLetter);
    }
    let body = name.strip_suffix('/').unwrap_or(name);
    for component in body.split('/') {
        if component.is_empty() {
            return Err(UnsafeName::EmptyComponent);
        }
        if component == ".." {
            return Err(UnsafeName::ParentComponent);
        }
        if component.bytes().all(|b| b == b'.') {
            return Err(UnsafeName::DotComponent);
        }
    }
    Ok(())
}

/// ASCII case-insensitive prefix test, for `META-INF/` paths (the JVM matches those
/// case-insensitively).
pub(crate) fn starts_with_ignore_case(name: &str, prefix: &str) -> bool {
    name.len() >= prefix.len()
        && name.as_bytes()[..prefix.len()].eq_ignore_ascii_case(prefix.as_bytes())
}

/// ASCII case-insensitive suffix test.
pub(crate) fn ends_with_ignore_case(name: &str, suffix: &str) -> bool {
    name.len() >= suffix.len()
        && name.as_bytes()[name.len() - suffix.len()..].eq_ignore_ascii_case(suffix.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_ordinary_names() {
        for name in [
            "a",
            "META-INF/",
            "META-INF/MANIFEST.MF",
            "net/example/Mod.class",
            "assets/example/lang/en_us..json",
            "a..b/c",
            ".hidden",
            "dir/.gitkeep",
            "unicode/\u{00e9}t\u{00e9}.txt",
        ] {
            assert_eq!(check_name(name), Ok(()), "{name}");
        }
    }

    #[test]
    fn refuses_unsafe_names() {
        let cases = [
            ("", UnsafeName::Empty),
            ("/etc/passwd", UnsafeName::Absolute),
            ("/", UnsafeName::Absolute),
            ("C:/Windows/x", UnsafeName::DriveLetter),
            ("c:x", UnsafeName::DriveLetter),
            ("a\\..\\b", UnsafeName::Backslash),
            ("a\\b", UnsafeName::Backslash),
            ("a\0b", UnsafeName::Nul),
            ("../evil", UnsafeName::ParentComponent),
            ("a/../../evil", UnsafeName::ParentComponent),
            ("a/..", UnsafeName::ParentComponent),
            ("a/../", UnsafeName::ParentComponent),
            ("./a", UnsafeName::DotComponent),
            ("a/./b", UnsafeName::DotComponent),
            ("a/.../b", UnsafeName::DotComponent),
            ("a//b", UnsafeName::EmptyComponent),
            ("a//", UnsafeName::EmptyComponent),
        ];
        for (name, reason) in cases {
            assert_eq!(check_name(name), Err(reason), "{name:?}");
        }
    }

    #[test]
    fn case_insensitive_helpers() {
        assert!(starts_with_ignore_case("meta-inf/x", "META-INF/"));
        assert!(!starts_with_ignore_case("META", "META-INF/"));
        assert!(ends_with_ignore_case("META-INF/a.rsa", ".RSA"));
        assert!(!ends_with_ignore_case("SA", ".RSA"));
    }
}
