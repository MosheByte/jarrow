//! The Jarrow nightly board pipeline.
//!
//! Three steps, each a subcommand of the `jarrow-board` binary and each a function here:
//!
//! 1. [`list::fetch_list`]: the most-downloaded mods of one source loader on Modrinth, for the
//!    pinned game versions, with the file to test per version (`board-list-<loader>.json`).
//!    Every direction with that source loader shares the list.
//! 2. [`shard::run_shard`]: one slice of a list, measured for one direction. Each file is
//!    downloaded, checked against its sha512, run through the per-mod
//!    [`pipeline::ModPipeline`], and deleted. One report per mod and game version, plus
//!    `shard-<direction>-<n>.json`.
//! 3. [`merge::merge`]: all shards of every measured direction into `board.json` (all twelve
//!    directions, a column for each measured one), `board-summary.json` (the input of
//!    `tools/ledger.py`), `reports.tar.gz`, and the files the workflow writes to Cloudflare.
//!
//! The data contract is `docs/board.md` (schema 2); the pipeline is described in
//! `docs/board-pipeline.md`.
//!
//! Milestone M1, wave 1: the classifier, the converter and the harness do not exist yet, so
//! every file is `not-run` with the reason "No converter yet (milestone M2)" and every mod is
//! red on the board. Nothing on the board is ever green without a harness score.
//!
//! Directions are inputs ([`loader::Direction`]), never assumptions: the source loader picks
//! the Modrinth facets and the files, the target loader names the conversion. The nightly
//! measures one direction today, Fabric to NeoForge.

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

/// Schema version of `board-list-<loader>.json`, `shard-<direction>-<n>.json`, `board.json`
/// and the reports (`docs/board.md`).
pub const SCHEMA: u32 = 2;

/// Schema version of `board-summary.json`, the one `tools/ledger.py` accepts for every file
/// under `ledger/inputs/`.
pub const LEDGER_SCHEMA: u32 = 1;

/// The public board page, linked from the nightly Discussion.
pub const BOARD_URL: &str = "https://jarrow.pages.dev/board";
