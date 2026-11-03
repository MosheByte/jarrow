//! The Jarrow parity harness.
//!
//! The harness decides whether a conversion works. It starts a headless dedicated server per
//! loader with a small probe mod, reads the probe's dump, diffs the dumps of the original and
//! the converted mod, subtracts the noise two loaders produce even with no mod at all, and
//! scores what is left (decision D-024). The design and the command line are documented in
//! `docs/harness.md`.
//!
//! - [`loader`]: loader ids. The loader and the game version are always inputs.
//! - [`lock`]: the pins in `research-lock.json` the harness provisions from.
//! - [`fetch`]: downloads with publisher-hash verification.
//! - [`provision`]: installing a server per loader into the cache (Fabric and NeoForge built;
//!   Forge and Quilt return "not built yet").
//! - [`eula`]: the Minecraft EULA gate. Nothing starts while it is closed.
//! - [`process`]: the process boundary (real processes, or fakes in tests).
//! - [`runner`]: one server run, from a fresh directory to a validated dump, and baselines.
//! - [`schema`] and [`dump`]: the probe dump format (`harness/dump-schema.json`).
//! - [`canon`]: canonical JSON.
//! - [`diff`], [`noise`] and [`score`]: the comparison and its score.
//! - [`logs`]: log parsing and the classified red reasons.

#![forbid(unsafe_code)]

pub mod canon;
pub mod diff;
pub mod dump;
pub mod error;
pub mod eula;
pub mod fetch;
pub mod loader;
pub mod lock;
pub mod logs;
pub mod noise;
pub mod process;
pub mod provision;
pub mod runner;
pub mod schema;
pub mod score;

#[cfg(test)]
mod testutil;

pub use diff::{Category, CompareError, DiffKind, Difference};
pub use dump::Dump;
pub use error::HarnessError;
pub use eula::EulaGate;
pub use loader::Loader;
pub use lock::Lock;
pub use logs::ReasonKind;
pub use noise::Noise;
pub use runner::{RunRecord, RunRequest};
pub use score::{HarnessReport, Score, SideInput, SideStatus, compare};
