//! Tiers and harness outcomes.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Where a mod can be converted, decided by the tier classifier (book section 5.10).
///
/// Serialised as `"a"`, `"b"` or `"unsupported"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Tier {
    /// Covered by rule tables; converted in the browser (WASM) or the CLI.
    A,
    /// Needs a deep conversion in a GitHub Actions job (decompile, rules, agent, recompile).
    B,
    /// Cannot be converted; the classifier names the reason.
    Unsupported,
}

impl Tier {
    /// Every tier, in display order.
    pub const ALL: [Tier; 3] = [Tier::A, Tier::B, Tier::Unsupported];

    /// Stable machine id, identical to the serialised form.
    pub const fn id(self) -> &'static str {
        match self {
            Tier::A => "a",
            Tier::B => "b",
            Tier::Unsupported => "unsupported",
        }
    }

    /// Human label for badges and tables.
    pub const fn label(self) -> &'static str {
        match self {
            Tier::A => "Tier A",
            Tier::B => "Tier B",
            Tier::Unsupported => "Unsupported",
        }
    }

    /// The one sentence every result page shows to explain the tier.
    pub const fn summary(self) -> &'static str {
        match self {
            Tier::A => {
                "Tier A covers what the rule tables translate mechanically (manifest, mappings, \
                 entrypoints, registries, resources, mixins, access wideners and Fabric API calls \
                 with a rule), and it runs in your browser or the CLI."
            }
            Tier::B => {
                "Tier B needs a deep conversion (decompile, rules, agent fixes, recompile) that \
                 runs in a GitHub Actions job and must pass the full parity harness before a \
                 result is returned."
            }
            Tier::Unsupported => {
                "Unsupported means Jarrow will not convert this mod (for example native \
                 libraries, bytecode transformers, deep rendering hooks, obfuscated JARs or a \
                 license that forbids modification), and the reason is named."
            }
        }
    }
}

impl fmt::Display for Tier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

impl FromStr for Tier {
    type Err = ParseError;

    /// Parses the machine id (`a`, `b`, `unsupported`), case-insensitively.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Tier::ALL
            .into_iter()
            .find(|tier| tier.id().eq_ignore_ascii_case(s.trim()))
            .ok_or_else(|| ParseError::new("tier", s))
    }
}

/// The harness result for one conversion (book section 6).
///
/// Only the parity harness produces [`Outcome::Green`], [`Outcome::Yellow`] or
/// [`Outcome::Red`]. Anything without a harness result is [`Outcome::NotRun`].
///
/// Serialised as `"green"`, `"yellow"`, `"red"` or `"not-run"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Outcome {
    /// The harness found the converted mod behaves like the original.
    Green,
    /// The harness found differences that need a look; the report lists them.
    Yellow,
    /// The conversion or the harness failed; a classified reason is attached.
    Red,
    /// No harness result exists, so nothing is claimed.
    NotRun,
}

impl Outcome {
    /// Every outcome, in display order.
    pub const ALL: [Outcome; 4] = [
        Outcome::Green,
        Outcome::Yellow,
        Outcome::Red,
        Outcome::NotRun,
    ];

    /// Stable machine id, identical to the serialised form.
    pub const fn id(self) -> &'static str {
        match self {
            Outcome::Green => "green",
            Outcome::Yellow => "yellow",
            Outcome::Red => "red",
            Outcome::NotRun => "not-run",
        }
    }

    /// Human label for badges and tables.
    pub const fn label(self) -> &'static str {
        match self {
            Outcome::Green => "green",
            Outcome::Yellow => "yellow",
            Outcome::Red => "red",
            Outcome::NotRun => "not run",
        }
    }

    /// The one sentence a result page shows to explain the outcome.
    pub const fn summary(self) -> &'static str {
        match self {
            Outcome::Green => {
                "Verified by the harness: the converted mod matched the original on every \
                 measured dump."
            }
            Outcome::Yellow => {
                "The harness found differences that need a look; the report lists each one."
            }
            Outcome::Red => {
                "The conversion or the harness failed; the reason is attached to the report."
            }
            Outcome::NotRun => "The harness has not run on this conversion, so nothing is claimed.",
        }
    }

    /// Whether this outcome comes from a harness run. Only these may be shown as a result.
    pub const fn has_harness_result(self) -> bool {
        !matches!(self, Outcome::NotRun)
    }
}

impl fmt::Display for Outcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

impl FromStr for Outcome {
    type Err = ParseError;

    /// Parses the machine id (`green`, `yellow`, `red`, `not-run`), case-insensitively.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Outcome::ALL
            .into_iter()
            .find(|outcome| outcome.id().eq_ignore_ascii_case(s.trim()))
            .ok_or_else(|| ParseError::new("outcome", s))
    }
}

/// A string that is not a known [`Tier`] or [`Outcome`] id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    what: &'static str,
    input: String,
}

impl ParseError {
    fn new(what: &'static str, input: &str) -> Self {
        Self {
            what,
            input: input.to_owned(),
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown {} {:?}", self.what, self.input)
    }
}

impl std::error::Error for ParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tier_ids_match_serde() {
        for tier in Tier::ALL {
            let json = serde_json::to_string(&tier).unwrap();
            assert_eq!(json, format!("\"{}\"", tier.id()));
            let back: Tier = serde_json::from_str(&json).unwrap();
            assert_eq!(back, tier);
            assert_eq!(tier.id().parse::<Tier>().unwrap(), tier);
        }
    }

    #[test]
    fn outcome_ids_match_serde() {
        for outcome in Outcome::ALL {
            let json = serde_json::to_string(&outcome).unwrap();
            assert_eq!(json, format!("\"{}\"", outcome.id()));
            let back: Outcome = serde_json::from_str(&json).unwrap();
            assert_eq!(back, outcome);
            assert_eq!(outcome.id().parse::<Outcome>().unwrap(), outcome);
        }
    }

    #[test]
    fn parsing_is_case_insensitive_and_rejects_unknown() {
        assert_eq!(" A ".parse::<Tier>().unwrap(), Tier::A);
        assert_eq!("NOT-RUN".parse::<Outcome>().unwrap(), Outcome::NotRun);
        let err = "c".parse::<Tier>().unwrap_err();
        assert_eq!(err.to_string(), "unknown tier \"c\"");
        assert!("amber".parse::<Outcome>().is_err());
        assert!(serde_json::from_str::<Outcome>("\"not_run\"").is_err());
    }

    #[test]
    fn only_harness_outcomes_count_as_results() {
        assert!(Outcome::Green.has_harness_result());
        assert!(Outcome::Yellow.has_harness_result());
        assert!(Outcome::Red.has_harness_result());
        assert!(!Outcome::NotRun.has_harness_result());
    }

    #[test]
    fn summaries_are_single_sentences_without_promises() {
        let summaries = Tier::ALL
            .iter()
            .map(|t| t.summary())
            .chain(Outcome::ALL.iter().map(|o| o.summary()));
        for summary in summaries {
            assert!(summary.ends_with('.'), "{summary}");
            assert_eq!(summary.matches(". ").count(), 0, "one sentence: {summary}");
            assert!(!summary.to_lowercase().contains("guarantee"), "{summary}");
        }
    }

    #[test]
    fn display_uses_labels() {
        assert_eq!(Tier::Unsupported.to_string(), "Unsupported");
        assert_eq!(Outcome::NotRun.to_string(), "not run");
    }
}
