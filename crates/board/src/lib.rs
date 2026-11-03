//! The Jarrow nightly board pipeline.
//!
//! Three steps, each a subcommand of the `jarrow-board` binary and each a function here:
//!
//! 1. [`list::fetch_list`]: the most-downloaded mods of a direction's source loader on
//!    Modrinth, for the pinned game versions, with the file to test per version
//!    (`board-list.json`).
//! 2. [`shard::run_shard`]: one slice of that list. Each file is downloaded, checked against
//!    its sha512, run through the per-mod [`pipeline::ModPipeline`], and deleted. One report
//!    per mod and game version, plus `shard-<n>.json`.
//! 3. [`merge::merge`]: all shards into `board.json`, `board-summary.json` (the input of
//!    `tools/ledger.py`), `reports.tar.gz`, and the files the workflow writes to Cloudflare.
//!
//! The data contract is `docs/board.md`; the pipeline is described in
//! `docs/board-pipeline.md`.
//!
//! Milestone M1, wave 1: the classifier, the converter and the harness do not exist yet, so
//! every mod ends red with the reason "No converter yet (milestone M2)". Nothing on the board
//! is ever green without a harness result.
//!
//! Directions are inputs ([`loader::Direction`]), never assumptions: the source loader picks
//! the Modrinth facets and the files, the target loader names the conversion. Fabric to
//! NeoForge on 1.21.1 and 1.21.11 is the first column.

#![forbid(unsafe_code)]

pub mod board;
pub mod error;
pub mod gamever;
pub mod hash;
pub mod http;
pub mod list;
pub mod loader;
pub mod merge;
pub mod modrinth;
pub mod pipeline;
pub mod report;
pub mod shard;
pub mod time;

pub use error::Error;
pub use loader::{Direction, Loader};

/// Version of this engine, recorded in every shard, board and report.
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Schema version of `board-list.json`, `shard-<n>.json`, `board.json` and the reports.
pub const SCHEMA: u32 = 1;

/// The public board page, linked from the nightly Discussion.
pub const BOARD_URL: &str = "https://jarrow.pages.dev/board";
