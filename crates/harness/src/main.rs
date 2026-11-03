//! `jarrow-harness`: provision loader servers, run them with the probe, learn loader noise,
//! and diff and score dumps. Every command and flag is documented in `docs/harness.md`.
//!
//! Exit status: 0 success; 1 a run failed (the server started but no valid dump came out) or
//! a dump is invalid; 2 a usage or harness error; 3 not run (the EULA gate is closed, or a
//! pinned download failed).

#![forbid(unsafe_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use clap::{Args, Parser, Subcommand};
use jarrow_harness::dump::validate_dump;
use jarrow_harness::fetch::{LazyHttpFetcher, sha256_hex};
use jarrow_harness::process::SystemHost;
use jarrow_harness::provision::{ProvisionContext, provision};
use jarrow_harness::runner::{
    BaselineRequest, DEFAULT_XMX, HarnessEnv, POLL_INTERVAL, RunRecord, RunRequest, STOP_GRACE,
    default_record_path, default_work_dir, find_probe, learn_noise, load_side, noise_path, run,
    run_baseline,
};
use jarrow_harness::{EulaGate, HarnessError, Loader, Lock, Noise, SideStatus, compare};

const EXIT_OK: u8 = 0;
const EXIT_FAILED: u8 = 1;
const EXIT_ERROR: u8 = 2;
const EXIT_NOT_RUN: u8 = 3;

/// Jarrow parity harness: headless loader servers, probe dumps, diff and score.
#[derive(Debug, Parser)]
#[command(
    name = "jarrow-harness",
    bin_name = "jarrow-harness",
    version,
    about = "Jarrow parity harness: provisions headless loader servers, runs them with the probe, \
             learns loader noise, and diffs and scores dumps (docs/harness.md).",
    after_help = "Exit status: 0 success, 1 run failed or dump invalid, 2 usage or harness error, \
                  3 not run (EULA gate closed or a pinned download failed).\n\
                  Servers start only when MINECRAFT_EULA_ACCEPTED or MINECRAFT_EULA is true, yes, 1 \
                  or accepted. Java comes from JAVA_HOME.",
    arg_required_else_help = true
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Download (hash-verified) and install the server for one loader and game version.
    Provision(ProvisionArgs),
    /// Run one server with the probe and the mods under test, and keep its dump.
    Run(RunArgs),
    /// Run two loaders with only their probe and learn the loader noise between them.
    Baseline(BaselineArgs),
    /// Compare a reference run with a candidate run, subtract the noise and score.
    Diff(DiffArgs),
    /// Check a dump against harness/dump-schema.json.
    Validate(ValidateArgs),
}

#[derive(Debug, Args)]
struct CacheArgs {
    /// Cache directory for downloads, installed servers and run directories.
    #[arg(
        long,
        value_name = "DIR",
        default_value = "target/jarrow-harness-cache"
    )]
    cache: PathBuf,
    /// research-lock.json to take pins from [default: the copy built into this binary].
    #[arg(long, value_name = "FILE")]
    lock: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct ServerArgs {
    /// Seconds a server may take to write its dump before it is stopped.
    #[arg(long, value_name = "SECONDS", default_value_t = 360)]
    timeout_secs: u64,
    /// Maximum Java heap for the server (-Xmx).
    #[arg(long, value_name = "SIZE", default_value = DEFAULT_XMX)]
    xmx: String,
}

#[derive(Debug, Args)]
struct ProvisionArgs {
    /// Loader: fabric, neoforge, forge or quilt (forge and quilt are not built yet).
    #[arg(long, value_name = "LOADER")]
    loader: Loader,
    /// Game version, as pinned in research-lock.json (for example 1.21.1).
    #[arg(long, value_name = "VERSION")]
    game_version: String,
    #[command(flatten)]
    cache: CacheArgs,
}

#[derive(Debug, Args)]
struct RunArgs {
    /// Loader: fabric, neoforge, forge or quilt (forge and quilt are not built yet).
    #[arg(long, value_name = "LOADER")]
    loader: Loader,
    /// Game version, as pinned in research-lock.json (for example 1.21.1).
    #[arg(long, value_name = "VERSION")]
    game_version: String,
    /// The probe mod jar for this loader.
    #[arg(long, value_name = "JAR")]
    probe: PathBuf,
    /// Mods under test, comma-separated or repeated. Omit for a probe-only run.
    #[arg(long, value_name = "JARS", value_delimiter = ',')]
    mods: Vec<PathBuf>,
    /// Where to write the dump.
    #[arg(long, value_name = "FILE")]
    out: PathBuf,
    /// Where to write the run record [default: the --out path with .run.json].
    #[arg(long, value_name = "FILE")]
    record: Option<PathBuf>,
    /// The server directory for this run, cleared first; it must be new, empty or made by
    /// jarrow-harness [default: <cache>/work/<loader>-<game version>].
    #[arg(long, value_name = "DIR")]
    work: Option<PathBuf>,
    #[command(flatten)]
    server: ServerArgs,
    #[command(flatten)]
    cache: CacheArgs,
}

#[derive(Debug, Args)]
struct BaselineArgs {
    /// Game version, as pinned in research-lock.json (for example 1.21.1).
    #[arg(long, value_name = "VERSION")]
    game_version: String,
    /// The reference loader (the one mods are converted from).
    #[arg(long, value_name = "LOADER", default_value = "fabric")]
    from: Loader,
    /// The candidate loader (the one mods are converted to).
    #[arg(long, value_name = "LOADER", default_value = "neoforge")]
    to: Loader,
    /// The Fabric probe jar.
    #[arg(long, value_name = "JAR")]
    probe_fabric: Option<PathBuf>,
    /// The NeoForge probe jar.
    #[arg(long, value_name = "JAR")]
    probe_neoforge: Option<PathBuf>,
    /// The Forge probe jar.
    #[arg(long, value_name = "JAR")]
    probe_forge: Option<PathBuf>,
    /// The Quilt probe jar.
    #[arg(long, value_name = "JAR")]
    probe_quilt: Option<PathBuf>,
    /// A directory holding the probe jars as harness/probe names them
    /// (jarrow-probe-<loader>-<game version>-<version>.jar); used for every loader without its
    /// own --probe-<loader>.
    #[arg(long, value_name = "DIR")]
    probe_dir: Option<PathBuf>,
    /// The noise file (a path ending in .json), or a directory that receives noise.json; the
    /// baseline dumps and run records go next to it
    /// [default: target/jarrow-harness/noise-<from>-<to>-<game version>.json].
    #[arg(long, value_name = "FILE|DIR")]
    out: Option<PathBuf>,
    /// Learn from this existing reference baseline (a dump or a run record) instead of
    /// starting servers. Needs --candidate-dump.
    #[arg(long, value_name = "FILE", requires = "candidate_dump")]
    reference_dump: Option<PathBuf>,
    /// Learn from this existing candidate baseline (a dump or a run record) instead of
    /// starting servers. Needs --reference-dump.
    #[arg(long, value_name = "FILE", requires = "reference_dump")]
    candidate_dump: Option<PathBuf>,
    #[command(flatten)]
    server: ServerArgs,
    #[command(flatten)]
    cache: CacheArgs,
}

#[derive(Debug, Args)]
struct DiffArgs {
    /// The reference run: the original mod on its own loader (a dump or a run record).
    reference: PathBuf,
    /// The candidate run: the converted mod on the target loader (a dump or a run record).
    candidate: PathBuf,
    /// The noise file from `baseline` for these loaders and game version. Without it, nothing
    /// is subtracted (useful for two runs on the same loader).
    #[arg(long, value_name = "FILE")]
    baseline: Option<PathBuf>,
    /// Where to write the report.
    #[arg(long, value_name = "FILE")]
    out: PathBuf,
}

#[derive(Debug, Args)]
struct ValidateArgs {
    /// The dump to check.
    dump: PathBuf,
}

fn lock(args: &CacheArgs) -> Result<Lock, HarnessError> {
    match &args.lock {
        Some(path) => {
            let text =
                fs::read_to_string(path).map_err(|e| HarnessError::io_at("read", path, e))?;
            Ok(Lock::parse(&text)?)
        }
        None => Ok(Lock::embedded()?),
    }
}

fn java_home() -> Option<PathBuf> {
    std::env::var_os("JAVA_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

fn status_code(status: SideStatus) -> u8 {
    match status {
        SideStatus::Completed => EXIT_OK,
        SideStatus::Failed => EXIT_FAILED,
        SideStatus::NotRun => EXIT_NOT_RUN,
    }
}

fn print_record(record: &RunRecord, path: &Path) {
    let status = match record.status {
        SideStatus::Completed => "completed",
        SideStatus::Failed => "failed",
        SideStatus::NotRun => "not run",
    };
    let kind = record
        .reason_kind
        .map(|k| format!(" [{k}]"))
        .unwrap_or_default();
    println!(
        "{} {} on {}: {status}{kind}: {}",
        record.loader,
        record.loader_version.as_deref().unwrap_or("?"),
        record.game_version,
        record.reason
    );
    println!("  record: {}", path.display());
}

fn dispatch(cli: Cli) -> Result<u8, HarnessError> {
    let fetcher = LazyHttpFetcher::default();
    let host = SystemHost;
    match cli.command {
        Command::Provision(args) => {
            let lock = lock(&args.cache)?;
            let java = java_home().map(|home| jarrow_harness::process::java_in(&home));
            let ctx = ProvisionContext {
                lock: &lock,
                game_version: &args.game_version,
                cache: &args.cache.cache,
                fetcher: &fetcher,
                host: &host,
                java: java.as_deref(),
            };
            let installed = provision(args.loader, &ctx)?;
            println!(
                "{} {} for {}: installed in {} ({})",
                installed.loader,
                installed.loader_version,
                installed.game_version,
                installed.dir.display(),
                installed.method
            );
            for note in &installed.notes {
                println!("  note: {note}");
            }
            Ok(EXIT_OK)
        }
        Command::Run(args) => {
            let lock = lock(&args.cache)?;
            let eula = EulaGate::from_env();
            let env = HarnessEnv {
                lock: &lock,
                fetcher: &fetcher,
                host: &host,
                eula: &eula,
            };
            let mut req = RunRequest::new(
                args.loader,
                &args.game_version,
                args.probe,
                args.out.clone(),
                args.cache.cache.clone(),
            );
            req.mods = args.mods;
            req.record = args
                .record
                .unwrap_or_else(|| default_record_path(&args.out));
            req.work = args.work.unwrap_or_else(|| {
                default_work_dir(&args.cache.cache, args.loader, &args.game_version)
            });
            req.timeout = Duration::from_secs(args.server.timeout_secs);
            req.xmx = args.server.xmx;
            req.java_home = java_home();
            let record = run(&req, &env)?;
            print_record(&record, &req.record);
            if record.status == SideStatus::Completed {
                println!("  dump: {}", req.out.display());
            }
            Ok(status_code(record.status))
        }
        Command::Baseline(args) => baseline(args, &fetcher, &host),
        Command::Diff(args) => {
            let reference = load_side(&args.reference)?;
            let candidate = load_side(&args.candidate)?;
            let noise = match &args.baseline {
                Some(path) => {
                    let text = fs::read_to_string(path)
                        .map_err(|e| HarnessError::io_at("read", path, e))?;
                    let noise = Noise::from_json(&text)
                        .map_err(|e| HarnessError::Usage(format!("{}: {e}", path.display())))?;
                    Some((noise, sha256_hex(text.as_bytes())))
                }
                None => None,
            };
            let mut report = compare(&reference, &candidate, noise.as_ref().map(|(n, _)| n))?;
            if let (Some(info), Some((_, sha))) = (report.baseline.as_mut(), noise.as_ref()) {
                info.sha256 = Some(sha.clone());
            }
            let text = report
                .to_json()
                .map_err(|e| HarnessError::Usage(format!("could not serialise the report: {e}")))?;
            if let Some(parent) = args.out.parent().filter(|p| !p.as_os_str().is_empty()) {
                fs::create_dir_all(parent).map_err(|e| HarnessError::io_at("create", parent, e))?;
            }
            fs::write(&args.out, text).map_err(|e| HarnessError::io_at("write", &args.out, e))?;
            println!("{}", report.summary());
            println!("  report: {}", args.out.display());
            Ok(EXIT_OK)
        }
        Command::Validate(args) => {
            let text = fs::read_to_string(&args.dump)
                .map_err(|e| HarnessError::io_at("read", &args.dump, e))?;
            let value: serde_json::Value = match serde_json::from_str(&text) {
                Ok(value) => value,
                Err(e) => {
                    println!("{}: not valid JSON: {e}", args.dump.display());
                    return Ok(EXIT_FAILED);
                }
            };
            match validate_dump(&value) {
                Ok(()) => {
                    println!("{}: valid (dump schema 1)", args.dump.display());
                    Ok(EXIT_OK)
                }
                Err(e) => {
                    println!("{}: {e}", args.dump.display());
                    Ok(EXIT_FAILED)
                }
            }
        }
    }
}

fn baseline(
    args: BaselineArgs,
    fetcher: &LazyHttpFetcher,
    host: &SystemHost,
) -> Result<u8, HarnessError> {
    let out = noise_path(args.out.as_deref(), args.from, args.to, &args.game_version);
    if let (Some(reference), Some(candidate)) = (&args.reference_dump, &args.candidate_dump) {
        let noise = learn_noise(reference, candidate, &out)?;
        println!(
            "noise for {} {} -> {} {} on {}: {} differences, {} log lines",
            noise.reference.loader,
            noise.reference.loader_version,
            noise.candidate.loader,
            noise.candidate.loader_version,
            noise.game_version,
            noise.differences.len(),
            noise.log.len()
        );
        println!("  noise: {}", out.display());
        return Ok(EXIT_OK);
    }
    let probe_for = |loader: Loader| -> Result<PathBuf, HarnessError> {
        let probe = match loader {
            Loader::Fabric => &args.probe_fabric,
            Loader::NeoForge => &args.probe_neoforge,
            Loader::Forge => &args.probe_forge,
            Loader::Quilt => &args.probe_quilt,
        };
        match (probe, &args.probe_dir) {
            (Some(probe), _) => Ok(probe.clone()),
            (None, Some(dir)) => find_probe(dir, loader, &args.game_version),
            (None, None) => Err(HarnessError::Usage(format!(
                "baseline needs --probe-{loader} or --probe-dir for the {loader} side"
            ))),
        }
    };
    let lock = lock(&args.cache)?;
    let eula = EulaGate::from_env();
    let env = HarnessEnv {
        lock: &lock,
        fetcher,
        host,
        eula: &eula,
    };
    let req = BaselineRequest {
        game_version: args.game_version.clone(),
        reference: (args.from, probe_for(args.from)?),
        candidate: (args.to, probe_for(args.to)?),
        out: out.clone(),
        cache: args.cache.cache.clone(),
        timeout: Duration::from_secs(args.server.timeout_secs),
        xmx: args.server.xmx,
        java_home: java_home(),
        poll: POLL_INTERVAL,
        stop_grace: STOP_GRACE,
    };
    let outcome = run_baseline(&req, &env)?;
    for (record, loader) in [
        (&outcome.reference, args.from),
        (&outcome.candidate, args.to),
    ] {
        let path = jarrow_harness::runner::sibling(&out, &format!("{loader}.run"));
        print_record(record, &path);
    }
    match &outcome.noise {
        Some(noise) => {
            println!(
                "noise: {} differences, {} log lines -> {}",
                noise.differences.len(),
                noise.log.len(),
                out.display()
            );
            Ok(EXIT_OK)
        }
        None => {
            println!("no noise file was written: both baseline runs must complete");
            let statuses = [outcome.reference.status, outcome.candidate.status];
            Ok(if statuses.contains(&SideStatus::NotRun) {
                EXIT_NOT_RUN
            } else {
                EXIT_FAILED
            })
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match dispatch(cli) {
        Ok(code) => ExitCode::from(code),
        Err(err) => {
            eprintln!("jarrow-harness: {err}");
            ExitCode::from(EXIT_ERROR)
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
    fn the_four_harness_commands_and_validate_exist() {
        let names: Vec<String> = Cli::command()
            .get_subcommands()
            .map(|c| c.get_name().to_owned())
            .collect();
        assert_eq!(names, ["provision", "run", "baseline", "diff", "validate"]);
    }

    #[test]
    fn mods_split_on_commas() {
        let cli = Cli::try_parse_from([
            "jarrow-harness",
            "run",
            "--loader",
            "fabric",
            "--game-version",
            "1.21.1",
            "--probe",
            "p.jar",
            "--mods",
            "a.jar,b.jar",
            "--mods",
            "c.jar",
            "--out",
            "dump.json",
        ])
        .unwrap();
        let Command::Run(args) = cli.command else {
            panic!("expected run");
        };
        assert_eq!(
            args.mods,
            [PathBuf::from("a.jar"), "b.jar".into(), "c.jar".into()]
        );
        assert_eq!(args.server.timeout_secs, 360);
        assert_eq!(args.server.xmx, "3G");
        assert_eq!(
            args.cache.cache,
            PathBuf::from("target/jarrow-harness-cache")
        );
    }

    #[test]
    fn unknown_loaders_and_half_offline_baselines_are_refused() {
        assert!(
            Cli::try_parse_from([
                "jarrow-harness",
                "provision",
                "--loader",
                "rift",
                "--game-version",
                "1.21.1"
            ])
            .is_err()
        );
        assert!(
            Cli::try_parse_from([
                "jarrow-harness",
                "baseline",
                "--game-version",
                "1.21.1",
                "--reference-dump",
                "a.json"
            ])
            .is_err()
        );
    }
}
