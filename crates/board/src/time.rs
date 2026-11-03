//! UTC timestamps without a date library: RFC 3339 formatting, validation and ordering.

use std::cmp::Ordering;
use std::time::{SystemTime, UNIX_EPOCH};

/// A source of the current time, so tests can fix it.
pub trait Clock {
    /// The current time as RFC 3339 UTC with second precision, `2026-11-04T02:00:00Z`.
    fn now(&self) -> String;
}

/// The system clock.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> String {
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        format_unix(secs)
    }
}

/// A clock that always answers the same time (tests and reproducible runs).
#[derive(Debug, Clone)]
pub struct FixedClock(pub String);

impl Clock for FixedClock {
    fn now(&self) -> String {
        self.0.clone()
    }
}

/// Days since 1970-01-01 to a civil date (proleptic Gregorian), Howard Hinnant's algorithm.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    // d is 1..=31 and m is 1..=12 by construction.
    (
        year,
        u32::try_from(m).unwrap_or(1),
        u32::try_from(d).unwrap_or(1),
    )
}

/// Seconds since the Unix epoch as RFC 3339 UTC, `2026-11-04T02:00:00Z`.
pub fn format_unix(secs: u64) -> String {
    let days = i64::try_from(secs / 86_400).unwrap_or(0);
    let rem = secs % 86_400;
    let (y, m, d) = civil_from_days(days);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

/// Whether `s` looks like `YYYY-MM-DDTHH:MM:SS` followed by optional fractional seconds and
/// `Z` or `+00:00`. Only the shape is checked, plus the ranges of month, day, hour, minute and
/// second.
pub fn is_rfc3339_utc(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() < 20 {
        return false;
    }
    let digits = |r: std::ops::Range<usize>| b[r].iter().all(u8::is_ascii_digit);
    let shape = digits(0..4)
        && b[4] == b'-'
        && digits(5..7)
        && b[7] == b'-'
        && digits(8..10)
        && b[10] == b'T'
        && digits(11..13)
        && b[13] == b':'
        && digits(14..16)
        && b[16] == b':'
        && digits(17..19);
    if !shape {
        return false;
    }
    let num = |r: std::ops::Range<usize>| s[r].parse::<u32>().unwrap_or(99);
    let ranges = (1..=12).contains(&num(5..7))
        && (1..=31).contains(&num(8..10))
        && num(11..13) <= 23
        && num(14..16) <= 59
        && num(17..19) <= 60;
    if !ranges {
        return false;
    }
    let mut rest = &s[19..];
    if let Some(frac) = rest.strip_prefix('.') {
        let n = frac.bytes().take_while(u8::is_ascii_digit).count();
        if n == 0 {
            return false;
        }
        rest = &frac[n..];
    }
    rest == "Z" || rest == "+00:00"
}

/// The `YYYY-MM-DD` date of an RFC 3339 timestamp.
pub fn date_of(ts: &str) -> Option<&str> {
    if is_rfc3339_utc(ts) {
        ts.get(0..10)
    } else {
        None
    }
}

/// `YYYYMMDD` for an RFC 3339 timestamp.
pub fn compact_date(ts: &str) -> Option<String> {
    date_of(ts).map(|d| d.replace('-', ""))
}

/// A sortable key for an RFC 3339 UTC timestamp: the seconds part plus the fraction padded to
/// nine digits, so `…:58.39Z` sorts after `…:58Z`. Plain string comparison gets that wrong.
pub fn sort_key(ts: &str) -> (String, String) {
    let head = ts.get(0..19).unwrap_or(ts).to_owned();
    let frac = ts
        .get(19..)
        .and_then(|rest| rest.strip_prefix('.'))
        .map(|f| f.bytes().take_while(u8::is_ascii_digit).count())
        .and_then(|n| ts.get(20..20 + n))
        .unwrap_or("");
    let mut frac = frac.chars().take(9).collect::<String>();
    while frac.len() < 9 {
        frac.push('0');
    }
    (head, frac)
}

/// Orders two RFC 3339 UTC timestamps, oldest first.
pub fn compare(a: &str, b: &str) -> Ordering {
    sort_key(a).cmp(&sort_key(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_known_instants() {
        assert_eq!(format_unix(0), "1970-01-01T00:00:00Z");
        assert_eq!(format_unix(951_782_400), "2000-02-29T00:00:00Z");
        assert_eq!(format_unix(1_793_671_200), "2026-11-03T02:00:00Z");
        assert_eq!(format_unix(4_107_542_399), "2100-02-28T23:59:59Z");
    }

    #[test]
    fn system_clock_is_well_formed() {
        assert!(is_rfc3339_utc(&SystemClock.now()));
    }

    #[test]
    fn validation() {
        for ok in [
            "2026-11-04T02:00:00Z",
            "2026-09-20T21:25:58.390320Z",
            "2026-10-15T17:51:36.063140+00:00",
        ] {
            assert!(is_rfc3339_utc(ok), "{ok}");
        }
        for bad in [
            "",
            "2026-11-04",
            "2026-11-04 02:00:00Z",
            "2026-13-04T02:00:00Z",
            "2026-11-04T25:00:00Z",
            "2026-11-04T02:00:00",
            "2026-11-04T02:00:00.Z",
            "2026-11-04T02:00:00+02:00",
        ] {
            assert!(!is_rfc3339_utc(bad), "{bad}");
        }
    }

    #[test]
    fn dates() {
        assert_eq!(date_of("2026-11-04T02:00:00Z"), Some("2026-11-04"));
        assert_eq!(compact_date("2026-11-04T02:00:00Z").unwrap(), "20261104");
        assert_eq!(date_of("yesterday"), None);
    }

    #[test]
    fn fractions_order_correctly() {
        assert_eq!(
            compare("2026-09-20T21:25:58Z", "2026-09-20T21:25:58.390320Z"),
            Ordering::Less
        );
        assert_eq!(
            compare("2026-09-20T21:25:58.39Z", "2026-09-20T21:25:58.390000Z"),
            Ordering::Equal
        );
        assert_eq!(
            compare("2026-09-21T00:00:00Z", "2026-09-20T23:59:59.999Z"),
            Ordering::Greater
        );
    }
}
