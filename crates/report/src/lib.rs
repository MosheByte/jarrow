//! The Jarrow report model.
//!
//! One vocabulary for every surface that shows a conversion: the browser, the CLI, the
//! Tier B job and the nightly board.
//!
//! - [`Tier`]: where a mod can be converted (A, B) or why it cannot (unsupported).
//! - [`Outcome`]: the harness result for a conversion (green, yellow, red), or
//!   [`Outcome::NotRun`] when no harness result exists, in which case nothing is claimed.
//! - [`ReportEntry`]: one structured line of a report (`kind`, `location`, `before`, `after`,
//!   `rule`, `confidence`, plus a `note` that says why), so the board can aggregate the
//!   reasons mods are red.
//! - [`tokens`]: the design tokens. The site's `tokens.css` and the badge colours are both
//!   generated from this module, so they cannot drift apart.

#![forbid(unsafe_code)]

mod entry;
mod outcome;
pub mod tokens;

pub use entry::{Confidence, EntryError, EntryKind, Location, ReportEntry};
pub use outcome::{Outcome, ParseError, Tier};
