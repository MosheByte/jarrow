//! Minecraft release versions: parsing the `--game-versions` list and ordering versions.
//!
//! Only release versions (`1.21.1`, `1.21.11`, `1.20.1`) are accepted: snapshots and
//! pre-releases are never pinned. Versions compare numerically per component, so `1.21.11`
//! is newer than `1.21.1`, and `1.21.1` newer than `1.21`.

use std::cmp::Ordering;

use crate::Error;

/// Whether `s` is a release version: dot-separated numbers, at least two components.
pub fn is_release(s: &str) -> bool {
    let parts: Vec<&str> = s.split('.').collect();
    parts.len() >= 2
        && parts.len() <= 4
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.len() <= 4 && p.bytes().all(|b| b.is_ascii_digit()))
}

fn components(s: &str) -> Vec<u32> {
    s.split('.')
        .map(|p| p.parse::<u32>().unwrap_or(0))
        .collect()
}

/// Orders two release versions, oldest first. Missing components count as zero, so `1.21`
/// and `1.21.0` compare equal; the text breaks the tie, so the order is total.
pub fn compare(a: &str, b: &str) -> Ordering {
    let (ca, cb) = (components(a), components(b));
    let len = ca.len().max(cb.len());
    for i in 0..len {
        let x = ca.get(i).copied().unwrap_or(0);
        let y = cb.get(i).copied().unwrap_or(0);
        match x.cmp(&y) {
            Ordering::Equal => {}
            other => return other,
        }
    }
    a.cmp(b)
}

/// Sorts release versions oldest first and removes duplicates.
pub fn sort_versions(versions: &mut Vec<String>) {
    versions.sort_by(|a, b| compare(a, b));
    versions.dedup();
}

/// Parses a comma-separated list such as `1.21.1,1.21.11`: trimmed, validated, sorted
/// oldest first and deduplicated. An empty list is an error.
pub fn parse_list(s: &str) -> Result<Vec<String>, Error> {
    let mut out = Vec::new();
    for part in s.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if !is_release(part) {
            return Err(Error::invalid(format!(
                "game version {part:?} is not a release version such as 1.21.1"
            )));
        }
        out.push(part.to_owned());
    }
    if out.is_empty() {
        return Err(Error::invalid("at least one game version is required"));
    }
    sort_versions(&mut out);
    Ok(out)
}

/// The newest of the given versions, if any.
pub fn newest<'a>(versions: impl IntoIterator<Item = &'a str>) -> Option<&'a str> {
    versions.into_iter().max_by(|a, b| compare(a, b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_versions() {
        for ok in ["1.21.1", "1.21.11", "1.20.1", "1.21", "26.1"] {
            assert!(is_release(ok), "{ok}");
        }
        for bad in [
            "",
            "1",
            "1.21-pre1",
            "25w14a",
            "1..2",
            "1.21.1 ",
            "a.b",
            "1.2.3.4.5",
        ] {
            assert!(!is_release(bad), "{bad}");
        }
    }

    #[test]
    fn numeric_order() {
        assert_eq!(compare("1.21.1", "1.21.11"), Ordering::Less);
        assert_eq!(compare("1.21.11", "1.21.2"), Ordering::Greater);
        assert_eq!(compare("1.20.1", "1.21"), Ordering::Less);
        assert_eq!(compare("1.21", "1.21.1"), Ordering::Less);
        assert_eq!(compare("1.21.1", "1.21.1"), Ordering::Equal);
        assert_eq!(newest(["1.21.1", "1.21.11", "1.20.1"]), Some("1.21.11"));
        assert_eq!(newest(Vec::<&str>::new()), None);
    }

    #[test]
    fn parse_sorts_and_dedups() {
        assert_eq!(
            parse_list(" 1.21.11, 1.21.1,1.21.11 ,").unwrap(),
            vec!["1.21.1".to_owned(), "1.21.11".to_owned()]
        );
        assert_eq!(
            parse_list("1.21.1,1.20.1,1.21.11").unwrap(),
            vec!["1.20.1", "1.21.1", "1.21.11"]
        );
        assert!(parse_list("").is_err());
        assert!(parse_list(" , ").is_err());
        assert!(parse_list("1.21.1,24w33a").is_err());
    }
}
