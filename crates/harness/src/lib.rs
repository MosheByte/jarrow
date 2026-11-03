//! The Jarrow parity harness.
//!
//! The harness decides whether a conversion works. It starts a headless dedicated server per
//! loader with a small probe mod, reads the probe's dump, diffs the dumps of the original and
//! the converted mod, and scores what is left (decision D-024). The design is documented in
//! `docs/harness.md`.
//!
//! - [`loader`]: loader ids. The loader and the game version are always inputs.
//! - [`schema`] and [`dump`]: the probe dump format (`harness/dump-schema.json`).
//! - [`canon`]: canonical JSON.

#![forbid(unsafe_code)]

pub mod canon;
pub mod dump;
pub mod loader;
pub mod schema;

pub use dump::Dump;
pub use loader::Loader;
