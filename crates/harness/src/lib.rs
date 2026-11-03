//! The Jarrow parity harness.
//!
//! The harness decides whether a conversion works. It starts a headless dedicated server per
//! loader with a small probe mod, reads the probe's dump, diffs the dumps of the original and
//! the converted mod, subtracts the noise two loaders produce even with no mod at all, and
//! scores what is left (decision D-024). The design is documented in `docs/harness.md`.
//!
//! - [`loader`]: loader ids. The loader and the game version are always inputs.
//! - [`schema`] and [`dump`]: the probe dump format (`harness/dump-schema.json`).
//! - [`canon`]: canonical JSON.
//! - [`diff`], [`noise`] and [`score`]: the comparison and its score.
//! - [`logs`]: log parsing and the classified red reasons.

#![forbid(unsafe_code)]

pub mod canon;
pub mod diff;
pub mod dump;
pub mod loader;
pub mod logs;
pub mod noise;
pub mod schema;
pub mod score;

pub use diff::{Category, CompareError, DiffKind, Difference};
pub use dump::Dump;
pub use loader::Loader;
pub use logs::ReasonKind;
pub use noise::Noise;
pub use score::{HarnessReport, Score, SideInput, SideStatus, compare};
