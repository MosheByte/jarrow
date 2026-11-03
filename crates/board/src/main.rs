//! `jarrow-board`: the nightly board pipeline's command line (see `docs/board-pipeline.md`).
//!
//! ```text
//! jarrow-board list  --direction fabric-neoforge --top 1000 --game-versions 1.21.1,1.21.11 --out board-list.json
//! jarrow-board shard --list board-list.json --shard 3/10 --out shard-3.json --reports-dir reports/
//! jarrow-board merge shard-*.json --reports-dir reports/ --out-dir out/
//! ```

#![forbid(unsafe_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use jarrow_board::http::HttpClient;
use jarrow_board::list::{BoardList, ListOptions, fetch_list};
use jarrow_board::merge::{merge, write_outputs};
use jarrow_board::modrinth::API_BASE;
use jarrow_board::pipeline::WaveOne;
use jarrow_board::shard::{
    MAX_FILE_BYTES, ShardFile, ShardOptions, ShardSpec, rules_version, run_shard,
};
use jarrow_board::time::{Clock, SystemClock, compact_date};
use jarrow_board::{Direction, Error, gamever};

/// The nightly board: Modrinth list, sharded per-mod pipeline, merged board.
#[derive(Debug, Parser)]
#[command(
    name = "jarrow-board",
    bin_name = "jarrow-board",
    version,
    about = "The Jarrow nightly board pipeline (milestone M1: no converter yet, every mod is red).",
    arg_required_else_help = true
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Fetch the most-downloaded mods of the direction's source loader from Modrinth.
    List {
        /// Conversion direction, `<source>-<target>`, for example fabric-neoforge.
        #[arg(long)]
        direction: Direction,
        /// How many mods to keep (each with at least one usable file).
        #[arg(long, default_value_t = 1000)]
        top: u32,
        /// Pinned game versions, comma-separated.
        #[arg(long, value_name = "VERSIONS")]
        game_versions: String,
        /// Where to write board-list.json.
        #[arg(long, value_name = "FILE")]
        out: PathBuf,
        /// Modrinth API base URL.
        #[arg(long, default_value = API_BASE, value_name = "URL")]
        api_base: String,
    },
    /// Run one shard of a list: download, verify, pipeline, delete, report.
    Shard {
        /// board-list.json from `list`.
        #[arg(long, value_name = "FILE")]
        list: PathBuf,
        /// Which shard, N/M (1-based).
        #[arg(long, value_name = "N/M")]
        shard: ShardSpec,
        /// Where to write shard-N.json.
        #[arg(long, value_name = "FILE")]
        out: PathBuf,
        /// Directory for the per-mod reports.
        #[arg(long, value_name = "DIR", default_value = "reports")]
        reports_dir: PathBuf,
        /// Run id shared by every shard of the night (default: local-YYYYMMDD).
        #[arg(long, value_name = "ID")]
        run_id: Option<String>,
        /// Commit being run (default: $GITHUB_SHA, else "unknown").
        #[arg(long, value_name = "SHA")]
        git_sha: Option<String>,
        /// Rule-table root; the rules version hashes <DIR>/<direction>/.
        #[arg(long, value_name = "DIR", default_value = "rules")]
        rules_dir: PathBuf,
        /// Where JARs are downloaded (a temporary directory inside it; default: system temp).
        #[arg(long, value_name = "DIR")]
        work_dir: Option<PathBuf>,
        /// Largest JAR to download, in MiB.
        #[arg(long, value_name = "MIB", default_value_t = MAX_FILE_BYTES / (1024 * 1024))]
        max_file_mib: u64,
    },
    /// Merge shard files into board.json, board-summary.json, reports.tar.gz and the
    /// Cloudflare import files.
    Merge {
        /// shard-N.json files, or directories holding them.
        #[arg(required = true, value_name = "SHARD")]
        shards: Vec<PathBuf>,
        /// Directory holding every shard's reports.
        #[arg(long, value_name = "DIR", default_value = "reports")]
        reports_dir: PathBuf,
        /// Output directory.
        #[arg(long, value_name = "DIR")]
        out_dir: PathBuf,
    },
}

fn write_file(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent).map_err(|e| Error::io_at("creating", parent, e))?;
    }
    fs::write(path, bytes).map_err(|e| Error::io_at("writing", path, e))
}

fn read_file(path: &Path) -> Result<Vec<u8>, Error> {
    fs::read(path).map_err(|e| Error::io_at("reading", path, e))
}

fn shard_paths(inputs: &[PathBuf]) -> Result<Vec<PathBuf>, Error> {
    let mut out = Vec::new();
    for input in inputs {
        if input.is_dir() {
            let mut found: Vec<PathBuf> = fs::read_dir(input)
                .map_err(|e| Error::io_at("reading", input, e))?
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|p| {
                    p.file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|n| n.starts_with("shard-") && n.ends_with(".json"))
                })
                .collect();
            found.sort();
            out.extend(found);
        } else {
            out.push(input.clone());
        }
    }
    if out.is_empty() {
        return Err(Error::invalid("no shard files found"));
    }
    Ok(out)
}

fn run(cli: Cli) -> Result<(), Error> {
    match cli.command {
        Command::List {
            direction,
            top,
            game_versions,
            out,
            api_base,
        } => {
            let opts = ListOptions {
                direction,
                game_versions: gamever::parse_list(&game_versions)?,
                top,
                api_base,
            };
            let client = HttpClient::new()?;
            let list = fetch_list(&client, &opts, SystemClock.now(), |line| {
                eprintln!("{line}")
            })?;
            write_file(&out, &list.to_json()?)?;
            let files: usize = list.mods.iter().map(|m| m.files.len()).sum();
            println!(
                "{}: {} mods, {files} files, {} skipped search hits ({direction}, {})",
                out.display(),
                list.mods.len(),
                list.skipped.len(),
                list.game_versions.join(", ")
            );
            Ok(())
        }
        Command::Shard {
            list,
            shard,
            out,
            reports_dir,
            run_id,
            git_sha,
            rules_dir,
            work_dir,
            max_file_mib,
        } => {
            let board_list = BoardList::from_json(&read_file(&list)?)?;
            let clock = SystemClock;
            let run_id = match run_id {
                Some(id) => id,
                None => format!(
                    "local-{}",
                    compact_date(&clock.now()).unwrap_or_else(|| "00000000".into())
                ),
            };
            let git_sha = git_sha
                .or_else(|| std::env::var("GITHUB_SHA").ok())
                .unwrap_or_else(|| "unknown".to_owned());
            let opts = ShardOptions {
                run_id,
                git_sha,
                rules_version: rules_version(&rules_dir, board_list.direction)?,
                reports_dir,
                work_dir,
                max_file_bytes: max_file_mib.saturating_mul(1024 * 1024),
            };
            let client = HttpClient::new()?;
            let result = run_shard(
                &client,
                &WaveOne,
                &board_list,
                shard,
                &opts,
                &clock,
                |line| {
                    eprintln!("{line}");
                },
            )?;
            write_file(&out, &result.to_json()?)?;
            println!(
                "{}: shard {shard}, {} mods, {} files, {} bytes downloaded, {} download failures, {} ms",
                out.display(),
                result.stats.mods,
                result.stats.files,
                result.stats.downloaded_bytes,
                result.stats.download_failures,
                result.stats.wall_ms
            );
            Ok(())
        }
        Command::Merge {
            shards,
            reports_dir,
            out_dir,
        } => {
            let mut parsed = Vec::new();
            for path in shard_paths(&shards)? {
                parsed.push(ShardFile::from_json(
                    &read_file(&path)?,
                    &path.display().to_string(),
                )?);
            }
            let output = merge(&parsed, &reports_dir)?;
            let written = write_outputs(&output, &out_dir)?;
            let t = output.board.totals;
            println!(
                "{}: {} ({} mods: {} green, {} yellow, {} red; {} reports)",
                out_dir.display(),
                written.join(", "),
                t.mods,
                t.green,
                t.yellow,
                t.red,
                output.reports.len()
            );
            for d in &output.board.directions {
                let reasons: Vec<String> = d
                    .top_reasons
                    .iter()
                    .map(|r| format!("{} {}", r.kind, r.count))
                    .collect();
                println!(
                    "  {}: {} mods, {} green, {} yellow, {} red; reasons: {}",
                    d.direction,
                    d.totals.mods,
                    d.totals.green,
                    d.totals.yellow,
                    d.totals.red,
                    if reasons.is_empty() {
                        "none".to_owned()
                    } else {
                        reasons.join(", ")
                    }
                );
            }
            Ok(())
        }
    }
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("jarrow-board: {err}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn subcommands_are_list_shard_merge() {
        let names: Vec<String> = Cli::command()
            .get_subcommands()
            .map(|c| c.get_name().to_owned())
            .collect();
        assert_eq!(names, ["list", "shard", "merge"]);
    }

    #[test]
    fn arguments_parse() {
        let cli = Cli::try_parse_from([
            "jarrow-board",
            "shard",
            "--list",
            "board-list.json",
            "--shard",
            "3/10",
            "--out",
            "shard-3.json",
        ])
        .unwrap();
        match cli.command {
            Command::Shard {
                shard, reports_dir, ..
            } => {
                assert_eq!(shard.to_string(), "3/10");
                assert_eq!(reports_dir, PathBuf::from("reports"));
            }
            other => panic!("parsed {other:?}"),
        }
        assert!(
            Cli::try_parse_from([
                "jarrow-board",
                "list",
                "--direction",
                "fabric-fabric",
                "--game-versions",
                "1.21.1",
                "--out",
                "x"
            ])
            .is_err()
        );
    }
}
