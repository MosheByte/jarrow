//! One harness run: a fresh server directory, the probe and the mods under test, a fixed
//! `server.properties`, and a wait for the probe's dump.
//!
//! The EULA gate is checked before anything is downloaded or started. With the gate closed
//! the run ends as `not-run` (`harness_not_run_eula`), no server starts and no `eula.txt` is
//! written.

use std::ffi::OsString;
use std::fs;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::dump::{DUMP_FILE_NAME, Dump, Worldgen};
use crate::error::HarnessError;
use crate::eula::EulaGate;
use crate::fetch::{Fetcher, VerifiedFile, digest_file, sha256_hex};
use crate::loader::{Loader, is_game_version};
use crate::lock::Lock;
use crate::logs::{
    Finding, MAX_FINDINGS, ReasonKind, classify_crash_report, classify_log, primary_reason,
};
use crate::noise::Noise;
use crate::process::{CommandSpec, ProcessHost, java_in, java_major, parse_java_version};
use crate::provision::{
    InstalledServer, Launch, ProvisionContext, link_or_copy, profile, provision,
};
use crate::score::{SideInput, SideStatus};

/// The world seed of every harness server.
pub const HARNESS_SEED: i64 = 20_261_103;

/// How long a server may take to write its dump.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(6 * 60);

/// The default maximum heap.
pub const DEFAULT_XMX: &str = "3G";

/// How long a server may take to stop after `stop`.
pub const STOP_GRACE: Duration = Duration::from_secs(60);

/// How often the runner looks for the dump.
pub const POLL_INTERVAL: Duration = Duration::from_millis(500);

/// The `record` value of a run record.
pub const RUN_RECORD: &str = "jarrow-harness-run";

/// Marks a directory the runner created, so it may clear it next time.
pub const WORK_MARKER: &str = ".jarrow-harness-work";

/// The fixed `server.properties`. Only the port varies: it is a free loopback port, so
/// several runs can share a machine.
pub fn server_properties(port: u16) -> String {
    format!(
        "# Written by jarrow-harness; the same for every run (docs/harness.md).\n\
         difficulty=easy\n\
         enable-command-block=false\n\
         enable-query=false\n\
         enable-rcon=false\n\
         enforce-secure-profile=false\n\
         gamemode=survival\n\
         generate-structures=true\n\
         level-name=world\n\
         level-seed={HARNESS_SEED}\n\
         max-players=1\n\
         motd=jarrow-harness\n\
         online-mode=false\n\
         server-ip=127.0.0.1\n\
         server-port={port}\n\
         simulation-distance=4\n\
         spawn-protection=0\n\
         sync-chunk-writes=true\n\
         view-distance=4\n\
         white-list=false\n"
    )
}

/// What to run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunRequest {
    /// Loader.
    pub loader: Loader,
    /// Game version.
    pub game_version: String,
    /// The probe mod for this loader.
    pub probe: PathBuf,
    /// The mods under test.
    pub mods: Vec<PathBuf>,
    /// Cache directory.
    pub cache: PathBuf,
    /// The run's server directory (cleared first).
    pub work: PathBuf,
    /// Where the dump is copied.
    pub out: PathBuf,
    /// Where the run record is written.
    pub record: PathBuf,
    /// Time allowed until the dump appears.
    pub timeout: Duration,
    /// `-Xmx` value.
    pub xmx: String,
    /// The JDK (`JAVA_HOME`).
    pub java_home: Option<PathBuf>,
    /// Dump polling interval.
    pub poll: Duration,
    /// Time allowed to stop after `stop`.
    pub stop_grace: Duration,
}

impl RunRequest {
    /// A request with the default work directory, record path, timeout and heap.
    pub fn new(
        loader: Loader,
        game_version: &str,
        probe: PathBuf,
        out: PathBuf,
        cache: PathBuf,
    ) -> Self {
        Self {
            loader,
            game_version: game_version.to_owned(),
            probe,
            mods: Vec::new(),
            work: default_work_dir(&cache, loader, game_version),
            record: default_record_path(&out),
            out,
            cache,
            timeout: DEFAULT_TIMEOUT,
            xmx: DEFAULT_XMX.to_owned(),
            java_home: None,
            poll: POLL_INTERVAL,
            stop_grace: STOP_GRACE,
        }
    }
}

/// `<cache>/work/<loader>-<game version>`.
pub fn default_work_dir(cache: &Path, loader: Loader, game_version: &str) -> PathBuf {
    cache.join("work").join(format!("{loader}-{game_version}"))
}

/// `dump.json` gives `dump.run.json`.
pub fn default_record_path(out: &Path) -> PathBuf {
    out.with_extension("run.json")
}

/// What a run uses from the outside world.
pub struct HarnessEnv<'a> {
    /// Pins.
    pub lock: &'a Lock,
    /// Network.
    pub fetcher: &'a dyn Fetcher,
    /// Processes.
    pub host: &'a dyn ProcessHost,
    /// The EULA gate.
    pub eula: &'a EulaGate,
}

/// An input file of a run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputFile {
    /// `probe`, `mod` or `loader-mod`.
    pub role: String,
    /// File name in `mods/`.
    pub file: String,
    /// SHA-256.
    pub sha256: String,
    /// Bytes.
    pub size: u64,
}

/// The Java a run used.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JavaInfo {
    /// Path to `java`.
    pub path: String,
    /// What `java -version` reported.
    pub version: String,
}

/// What happened in one run. Written next to the dump as `<dump>.run.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunRecord {
    /// Always [`RUN_RECORD`].
    pub record: String,
    /// Run record format version (1).
    pub schema: u32,
    /// Loader.
    pub loader: Loader,
    /// Pinned loader version.
    pub loader_version: Option<String>,
    /// Game version.
    pub game_version: String,
    /// completed, failed or not-run.
    pub status: SideStatus,
    /// Why it failed or did not run.
    pub reason_kind: Option<ReasonKind>,
    /// One sentence.
    pub reason: String,
    /// The dump, relative to this record's directory (or absolute).
    pub dump: Option<String>,
    /// SHA-256 of the dump.
    pub dump_sha256: Option<String>,
    /// The probe, the mods under test and the loader's mods.
    pub inputs: Vec<InputFile>,
    /// The pinned downloads behind the server.
    pub server: Vec<VerifiedFile>,
    /// How the server was installed.
    pub method: Option<String>,
    /// The Java used.
    pub java: Option<JavaInfo>,
    /// The server's exit code, when it exited on its own or after `stop`.
    pub exit_code: Option<i32>,
    /// Wall time of the run.
    pub duration_ms: u64,
    /// The captured server output, relative to this record's directory.
    pub log: Option<String>,
    /// Classified log lines.
    pub findings: Vec<Finding>,
    /// Anything else worth knowing.
    pub notes: Vec<String>,
}

impl RunRecord {
    fn new(loader: Loader, game_version: &str) -> Self {
        Self {
            record: RUN_RECORD.to_owned(),
            schema: 1,
            loader,
            loader_version: None,
            game_version: game_version.to_owned(),
            status: SideStatus::NotRun,
            reason_kind: None,
            reason: String::new(),
            dump: None,
            dump_sha256: None,
            inputs: Vec::new(),
            server: Vec::new(),
            method: None,
            java: None,
            exit_code: None,
            duration_ms: 0,
            log: None,
            findings: Vec::new(),
            notes: Vec::new(),
        }
    }

    /// The record as one side of a comparison. A completed record's dump is read from
    /// `base` (the record's directory) and checked against the recorded hash.
    pub fn side_input(&self, base: &Path) -> Result<SideInput, HarnessError> {
        match self.status {
            SideStatus::Completed => {
                let dump = self.dump.as_ref().ok_or_else(|| {
                    HarnessError::Usage("a completed run record names no dump".to_owned())
                })?;
                let path = base.join(dump);
                let bytes = fs::read(&path).map_err(|e| HarnessError::io_at("read", &path, e))?;
                let sha = sha256_hex(&bytes);
                if self
                    .dump_sha256
                    .as_ref()
                    .is_some_and(|recorded| *recorded != sha)
                {
                    return Err(HarnessError::Usage(format!(
                        "{} changed after the run recorded it (sha256 {sha})",
                        path.display()
                    )));
                }
                let text = String::from_utf8_lossy(&bytes);
                let dump = Dump::from_json(&text).map_err(|error| HarnessError::Dump {
                    path: path.display().to_string(),
                    error,
                })?;
                Ok(SideInput::from_dump(dump, Some(sha)))
            }
            SideStatus::Failed => Ok(SideInput::Failed {
                loader: self.loader,
                loader_version: self.loader_version.clone(),
                game_version: self.game_version.clone(),
                reason_kind: self.reason_kind.unwrap_or(ReasonKind::HarnessLoadFailure),
                reason: self.reason.clone(),
                findings: self.findings.clone(),
            }),
            SideStatus::NotRun => Ok(SideInput::NotRun {
                loader: self.loader,
                game_version: self.game_version.clone(),
                reason_kind: self.reason_kind.ok_or_else(|| {
                    HarnessError::Usage("a not-run record names no reason_kind".to_owned())
                })?,
                reason: self.reason.clone(),
            }),
        }
    }

    fn write(&self, path: &Path) -> Result<(), HarnessError> {
        write_json(path, self)
    }
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), HarnessError> {
    let text = serde_json::to_string_pretty(value)
        .map_err(|e| HarnessError::Usage(format!("could not serialise {}: {e}", path.display())))?;
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent).map_err(|e| HarnessError::io_at("create", parent, e))?;
    }
    fs::write(path, text + "\n").map_err(|e| HarnessError::io_at("write", path, e))
}

/// Reads one side of a comparison from a dump or a run record.
pub fn load_side(path: &Path) -> Result<SideInput, HarnessError> {
    let bytes = fs::read(path).map_err(|e| HarnessError::io_at("read", path, e))?;
    let value: Value = serde_json::from_slice(&bytes).map_err(|e| HarnessError::Dump {
        path: path.display().to_string(),
        error: crate::dump::DumpError::Json(e.to_string()),
    })?;
    if value.get("record").and_then(Value::as_str) == Some(RUN_RECORD) {
        let record: RunRecord = serde_json::from_value(value).map_err(|e| {
            HarnessError::Usage(format!("{}: not a run record: {e}", path.display()))
        })?;
        let base = path.parent().unwrap_or_else(|| Path::new("."));
        return record.side_input(base);
    }
    let dump = Dump::from_value(&value).map_err(|error| HarnessError::Dump {
        path: path.display().to_string(),
        error,
    })?;
    Ok(SideInput::from_dump(dump, Some(sha256_hex(&bytes))))
}

fn relative_to_record(file: &Path, record: &Path) -> String {
    if file.parent() == record.parent()
        && let Some(name) = file.file_name()
    {
        return name.to_string_lossy().into_owned();
    }
    std::path::absolute(file)
        .unwrap_or_else(|_| file.to_path_buf())
        .display()
        .to_string()
}

fn file_name(path: &Path) -> Result<String, HarnessError> {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .ok_or_else(|| HarnessError::Usage(format!("{} is not a file path", path.display())))
}

fn input(role: &str, path: &Path) -> Result<InputFile, HarnessError> {
    if !path.is_file() {
        return Err(HarnessError::Usage(format!(
            "{role} {} does not exist",
            path.display()
        )));
    }
    let digests = digest_file(path).map_err(|e| HarnessError::io_at("read", path, e))?;
    Ok(InputFile {
        role: role.to_owned(),
        file: file_name(path)?,
        sha256: digests.sha256,
        size: digests.size,
    })
}

/// Runs one server. Writes the run record (and, when the run completed, the dump) and returns
/// the record. Errors are failures of the harness itself; a mod that fails to load is a
/// `failed` record, not an error.
pub fn run(req: &RunRequest, env: &HarnessEnv<'_>) -> Result<RunRecord, HarnessError> {
    let started = Instant::now();
    if !is_game_version(&req.game_version) {
        return Err(HarnessError::Usage(format!(
            "{:?} is not a game version (expected something like 1.21.1)",
            req.game_version
        )));
    }
    let mut record = RunRecord::new(req.loader, &req.game_version);
    record.inputs.push(input("probe", &req.probe)?);
    for m in &req.mods {
        record.inputs.push(input("mod", m)?);
    }
    let pre = ProvisionContext {
        lock: env.lock,
        game_version: &req.game_version,
        cache: &req.cache,
        fetcher: env.fetcher,
        host: env.host,
        java: None,
    };
    record.loader_version = Some(profile(req.loader).loader_version(&pre)?);

    if !env.eula.is_open() {
        record.reason_kind = Some(ReasonKind::HarnessNotRunEula);
        record.reason = EulaGate::closed_message().to_owned();
        return finish(record, req, started);
    }

    let java = resolve_java(req, env)?;
    let ctx = ProvisionContext {
        java: Some(Path::new(&java.path)),
        ..pre
    };
    record.java = Some(java.clone());
    let installed = match provision(req.loader, &ctx) {
        Ok(installed) => installed,
        Err(HarnessError::Download(e)) => {
            record.reason_kind = Some(ReasonKind::DownloadFailed);
            record.reason = format!("not run: {e}");
            return finish(record, req, started);
        }
        Err(other) => return Err(other),
    };
    record.method = Some(installed.method.clone());
    record.server = installed.files.clone();
    record.notes.extend(installed.notes.iter().cloned());
    for m in &installed.loader_mods {
        record.inputs.push(InputFile {
            role: "loader-mod".to_owned(),
            file: file_name(&m.path)?,
            sha256: m.sha256.clone(),
            size: m.size,
        });
    }

    let eula_txt = env
        .eula
        .eula_txt()
        .ok_or_else(|| HarnessError::Usage("the EULA gate closed during the run".to_owned()))?;
    prepare_work_dir(req, &installed, &eula_txt)?;
    let dump_path = req.work.join(DUMP_FILE_NAME);
    let spec = CommandSpec {
        program: PathBuf::from(&java.path),
        args: launch_args(&installed, &req.xmx, &dump_path)?,
        cwd: req.work.clone(),
        log: req.work.join("jarrow-server.log"),
    };
    let watch = watch_server(req, env.host, &spec, &dump_path)?;
    record.exit_code = watch.exit_code;
    record.notes.extend(watch.notes);

    let log_text = fs::read(&spec.log)
        .map(|b| String::from_utf8_lossy(&b).into_owned())
        .unwrap_or_default();
    let log_copy = req.record.with_extension("log");
    if spec.log.is_file() {
        if let Some(parent) = log_copy.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(parent).map_err(|e| HarnessError::io_at("create", parent, e))?;
        }
        fs::copy(&spec.log, &log_copy)
            .map_err(|e| HarnessError::io_at("copy the server log to", &log_copy, e))?;
        record.log = Some(relative_to_record(&log_copy, &req.record));
    }
    let mut findings = classify_log(&log_text, "server.log");
    findings.extend(crash_report_findings(&req.work));
    findings.truncate(MAX_FINDINGS);
    record.findings = findings;

    match watch.dump {
        Some(text) => accept_dump(&mut record, req, &text)?,
        None => {
            let kind = primary_reason(&record.findings).unwrap_or(ReasonKind::HarnessLoadFailure);
            let what = if watch.timed_out {
                format!(
                    "the server ran for {} s without writing {DUMP_FILE_NAME} and was stopped",
                    req.timeout.as_secs()
                )
            } else {
                format!(
                    "the server exited ({}) before writing {DUMP_FILE_NAME}",
                    watch
                        .exit_code
                        .map_or_else(|| "no exit code".to_owned(), |c| format!("code {c}"))
                )
            };
            let cause = record
                .findings
                .iter()
                .find(|f| f.kind == kind)
                .map(|f| format!(": {}", f.excerpt))
                .unwrap_or_default();
            record.status = SideStatus::Failed;
            record.reason_kind = Some(kind);
            record.reason = format!("{what}{cause}");
        }
    }
    finish(record, req, started)
}

fn finish(
    mut record: RunRecord,
    req: &RunRequest,
    started: Instant,
) -> Result<RunRecord, HarnessError> {
    record.duration_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    record.write(&req.record)?;
    Ok(record)
}

fn accept_dump(record: &mut RunRecord, req: &RunRequest, text: &str) -> Result<(), HarnessError> {
    let problem = match Dump::from_json(text) {
        Err(e) => Some(format!("the probe wrote {DUMP_FILE_NAME}, but it {e}")),
        Ok(dump) if dump.loader != req.loader => Some(format!(
            "the dump says it ran on {}, not {}",
            dump.loader, req.loader
        )),
        Ok(dump) if dump.game_version != req.game_version => Some(format!(
            "the dump says game version {}, not {}",
            dump.game_version, req.game_version
        )),
        Ok(Dump {
            worldgen: Worldgen {
                seed: Some(seed), ..
            },
            ..
        }) if seed != HARNESS_SEED => Some(format!(
            "the dump's world seed is {seed}, not the harness seed {HARNESS_SEED}, so server.properties was not applied"
        )),
        Ok(dump) => {
            if dump.worldgen.seed.is_none() {
                record.notes.push(
                    "the probe could not sample worldgen (seed is null); the dump's errors say why"
                        .to_owned(),
                );
            }
            if record
                .loader_version
                .as_deref()
                .is_some_and(|v| v != dump.loader_version)
            {
                record.notes.push(format!(
                    "the loader reported version {}, the pin is {}",
                    dump.loader_version,
                    record.loader_version.as_deref().unwrap_or_default()
                ));
            }
            None
        }
    };
    if let Some(problem) = problem {
        record.status = SideStatus::Failed;
        record.reason_kind = Some(ReasonKind::HarnessLoadFailure);
        record.reason = problem;
        return Ok(());
    }
    if let Some(parent) = req.out.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent).map_err(|e| HarnessError::io_at("create", parent, e))?;
    }
    fs::write(&req.out, text).map_err(|e| HarnessError::io_at("write", &req.out, e))?;
    record.status = SideStatus::Completed;
    record.reason = "The server started and the probe wrote a valid dump.".to_owned();
    record.dump = Some(relative_to_record(&req.out, &req.record));
    record.dump_sha256 = Some(sha256_hex(text.as_bytes()));
    Ok(())
}

fn crash_report_findings(work: &Path) -> Vec<Finding> {
    let dir = work.join("crash-reports");
    let Ok(entries) = fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "txt"))
        .collect();
    paths.sort();
    paths
        .iter()
        .filter_map(|p| {
            let text = fs::read(p).ok()?;
            let name = p.file_name()?.to_string_lossy().into_owned();
            Some(classify_crash_report(
                &String::from_utf8_lossy(&text),
                &format!("crash-reports/{name}"),
            ))
        })
        .flatten()
        .collect()
}

fn resolve_java(req: &RunRequest, env: &HarnessEnv<'_>) -> Result<JavaInfo, HarnessError> {
    let home = req.java_home.as_ref().ok_or_else(|| {
        HarnessError::Java(
            "JAVA_HOME is not set; the harness runs servers with JDK 21 from JAVA_HOME".to_owned(),
        )
    })?;
    let java = java_in(home);
    if !java.is_file() {
        return Err(HarnessError::Java(format!(
            "JAVA_HOME is {}, but {} does not exist",
            home.display(),
            java.display()
        )));
    }
    let logs = req.cache.join("logs");
    fs::create_dir_all(&logs).map_err(|e| HarnessError::io_at("create", &logs, e))?;
    let log = logs.join(format!("java-version-{}.log", req.loader));
    if log.exists() {
        fs::remove_file(&log).map_err(|e| HarnessError::io_at("replace", &log, e))?;
    }
    let spec = CommandSpec {
        program: java.clone(),
        args: vec!["-version".into()],
        cwd: req.cache.clone(),
        log: log.clone(),
    };
    env.host
        .run(&spec, Duration::from_secs(60))
        .map_err(|e| HarnessError::io(format!("run {} -version", java.display()), e))?;
    let output = fs::read(&log)
        .map(|b| String::from_utf8_lossy(&b).into_owned())
        .unwrap_or_default();
    let version = parse_java_version(&output).ok_or_else(|| {
        HarnessError::Java(format!(
            "could not read the Java version from `{} -version`",
            java.display()
        ))
    })?;
    if let (Some(wanted), Some(found)) = (env.lock.jdk_major(), java_major(&version))
        && found < wanted
    {
        return Err(HarnessError::Java(format!(
            "JAVA_HOME has Java {version}; research-lock.json pins JDK {wanted}"
        )));
    }
    Ok(JavaInfo {
        path: java.display().to_string(),
        version,
    })
}

/// Files and directories of an installed server that a run never takes over.
const NOT_COPIED: [&str; 11] = [
    crate::provision::INSTALL_RECORD_FILE,
    "jarrow-install.log",
    DUMP_FILE_NAME,
    "eula.txt",
    "server.properties",
    "mods",
    "world",
    "logs",
    "config",
    "crash-reports",
    WORK_MARKER,
];

fn prepare_work_dir(
    req: &RunRequest,
    installed: &InstalledServer,
    eula_txt: &str,
) -> Result<(), HarnessError> {
    let work = &req.work;
    if work.exists() {
        let ours = work.join(WORK_MARKER).is_file();
        let empty = fs::read_dir(work)
            .map_err(|e| HarnessError::io_at("read", work, e))?
            .next()
            .is_none();
        if !ours && !empty {
            return Err(HarnessError::Usage(format!(
                "refusing to clear {}: it is not empty and was not created by jarrow-harness \
                 (pass an empty or new directory to --work)",
                work.display()
            )));
        }
        fs::remove_dir_all(work).map_err(|e| HarnessError::io_at("clear", work, e))?;
    }
    fs::create_dir_all(work).map_err(|e| HarnessError::io_at("create", work, e))?;
    let marker = work.join(WORK_MARKER);
    fs::write(
        &marker,
        "This directory is cleared by every jarrow-harness run.\n",
    )
    .map_err(|e| HarnessError::io_at("write", &marker, e))?;
    link_tree(&installed.dir, work, true)?;

    let mods = work.join("mods");
    fs::create_dir_all(&mods).map_err(|e| HarnessError::io_at("create", &mods, e))?;
    let mut names = Vec::new();
    let sources = installed
        .loader_mods
        .iter()
        .map(|m| (m.path.as_path(), true))
        .chain(std::iter::once((req.probe.as_path(), false)))
        .chain(req.mods.iter().map(|m| (m.as_path(), false)));
    for (source, link) in sources {
        let name = file_name(source)?;
        if names.contains(&name) {
            return Err(HarnessError::Usage(format!(
                "two inputs are both named {name}; mods/ needs distinct file names"
            )));
        }
        let dest = mods.join(&name);
        if link {
            link_or_copy(source, &dest)?;
        } else {
            fs::copy(source, &dest)
                .map_err(|e| HarnessError::io_at("copy into mods/", source, e))?;
        }
        names.push(name);
    }
    let port = free_port()?;
    let properties = work.join("server.properties");
    fs::write(&properties, server_properties(port))
        .map_err(|e| HarnessError::io_at("write", &properties, e))?;
    let eula = work.join("eula.txt");
    fs::write(&eula, eula_txt).map_err(|e| HarnessError::io_at("write", &eula, e))
}

fn link_tree(src: &Path, dst: &Path, top: bool) -> Result<(), HarnessError> {
    let entries = fs::read_dir(src).map_err(|e| HarnessError::io_at("read", src, e))?;
    for entry in entries {
        let entry = entry.map_err(|e| HarnessError::io_at("read", src, e))?;
        let name = entry.file_name();
        if top && NOT_COPIED.iter().any(|n| name == *n) {
            continue;
        }
        let kind = entry
            .file_type()
            .map_err(|e| HarnessError::io_at("read", &entry.path(), e))?;
        let target = dst.join(&name);
        if kind.is_dir() {
            fs::create_dir_all(&target).map_err(|e| HarnessError::io_at("create", &target, e))?;
            link_tree(&entry.path(), &target, false)?;
        } else if kind.is_file() {
            link_or_copy(&entry.path(), &target)?;
        }
        // Symbolic links are not followed.
    }
    Ok(())
}

fn free_port() -> Result<u16, HarnessError> {
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .map_err(|e| HarnessError::io("find a free loopback port", e))?;
    let port = listener
        .local_addr()
        .map_err(|e| HarnessError::io("find a free loopback port", e))?
        .port();
    Ok(port)
}

/// The `java` arguments for an installed server.
pub fn launch_args(
    installed: &InstalledServer,
    xmx: &str,
    dump_path: &Path,
) -> Result<Vec<OsString>, HarnessError> {
    if xmx.is_empty() || !xmx.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return Err(HarnessError::Usage(format!(
            "--xmx {xmx:?} is not a heap size such as 3G"
        )));
    }
    let mut dump_property = OsString::from("-Djarrow.dump=");
    dump_property.push(std::path::absolute(dump_path).unwrap_or_else(|_| dump_path.to_path_buf()));
    let mut args: Vec<OsString> = vec![
        format!("-Xmx{xmx}").into(),
        "-Djava.awt.headless=true".into(),
        dump_property,
    ];
    match &installed.launch {
        Launch::Jar { jar, jvm_args } => {
            args.extend(jvm_args.iter().map(OsString::from));
            args.push("-jar".into());
            args.push(jar.into());
        }
        Launch::ArgFile { unix, windows } => {
            let file = if cfg!(windows) { windows } else { unix };
            let file = file.as_ref().ok_or_else(|| {
                HarnessError::Install(format!(
                    "the {} install has no argument file for this operating system",
                    installed.loader
                ))
            })?;
            args.push(format!("@{file}").into());
        }
    }
    args.push("nogui".into());
    Ok(args)
}

struct Watch {
    dump: Option<String>,
    exit_code: Option<i32>,
    timed_out: bool,
    notes: Vec<String>,
}

fn ready_dump(path: &Path) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str::<serde::de::IgnoredAny>(&text).ok()?;
    Some(text)
}

fn watch_server(
    req: &RunRequest,
    host: &dyn ProcessHost,
    spec: &CommandSpec,
    dump_path: &Path,
) -> Result<Watch, HarnessError> {
    let mut process = host
        .spawn(spec)
        .map_err(|e| HarnessError::io(format!("start {}", spec.program.display()), e))?;
    let started = Instant::now();
    let mut watch = Watch {
        dump: None,
        exit_code: None,
        timed_out: false,
        notes: Vec::new(),
    };
    loop {
        if let Some(text) = ready_dump(dump_path) {
            watch.dump = Some(text);
            break;
        }
        if let Some(code) = process
            .try_wait()
            .map_err(|e| HarnessError::io("watch the server", e))?
        {
            watch.exit_code = Some(code);
            watch.dump = ready_dump(dump_path);
            return Ok(watch);
        }
        if started.elapsed() >= req.timeout {
            watch.timed_out = true;
            process
                .kill()
                .map_err(|e| HarnessError::io("stop the server", e))?;
            return Ok(watch);
        }
        std::thread::sleep(req.poll);
    }
    // The dump is in; ask the server to stop, and make sure it does.
    if process.send_line("stop").is_err() {
        watch
            .notes
            .push("the server console was closed before `stop` could be sent".to_owned());
    }
    let asked = Instant::now();
    loop {
        if let Some(code) = process
            .try_wait()
            .map_err(|e| HarnessError::io("watch the server", e))?
        {
            watch.exit_code = Some(code);
            break;
        }
        if asked.elapsed() >= req.stop_grace {
            process
                .kill()
                .map_err(|e| HarnessError::io("stop the server", e))?;
            watch.notes.push(format!(
                "the server did not stop within {} s of `stop` and was killed",
                req.stop_grace.as_secs()
            ));
            break;
        }
        std::thread::sleep(req.poll);
    }
    Ok(watch)
}

/// What a baseline needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaselineRequest {
    /// Game version.
    pub game_version: String,
    /// The reference loader and its probe.
    pub reference: (Loader, PathBuf),
    /// The candidate loader and its probe.
    pub candidate: (Loader, PathBuf),
    /// The noise file to write.
    pub out: PathBuf,
    /// Cache directory.
    pub cache: PathBuf,
    /// Per-run timeout.
    pub timeout: Duration,
    /// `-Xmx` value.
    pub xmx: String,
    /// The JDK.
    pub java_home: Option<PathBuf>,
    /// Dump polling interval.
    pub poll: Duration,
    /// Time allowed to stop after `stop`.
    pub stop_grace: Duration,
}

/// The two runs of a baseline and, when both completed, the noise learned from them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaselineOutcome {
    /// The reference run.
    pub reference: RunRecord,
    /// The candidate run.
    pub candidate: RunRecord,
    /// The noise, written to the request's `out`.
    pub noise: Option<Noise>,
}

/// `noise.json` and `fabric` give `noise.fabric.json`.
pub fn sibling(out: &Path, tag: &str) -> PathBuf {
    let stem = out
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    out.with_file_name(format!("{stem}.{tag}.json"))
}

/// The noise file a baseline writes. `out` is the file itself when it ends in `.json` and is not
/// a directory; otherwise it is a directory that receives `noise.json` (and, next to it, the
/// baseline dumps and run records). Without `out`:
/// `target/jarrow-harness/noise-<from>-<to>-<game version>.json`.
pub fn noise_path(out: Option<&Path>, from: Loader, to: Loader, game_version: &str) -> PathBuf {
    match out {
        None => PathBuf::from("target/jarrow-harness")
            .join(format!("noise-{from}-{to}-{game_version}.json")),
        Some(path) if path.is_dir() || path.extension().is_none_or(|e| e != "json") => {
            path.join("noise.json")
        }
        Some(path) => path.to_path_buf(),
    }
}

/// The probe jar for `loader` in `dir`, as `harness/probe` names it:
/// `jarrow-probe-<loader>-<game version>-<probe version>.jar`. Source and javadoc jars are
/// ignored; anything but exactly one match is an error.
pub fn find_probe(dir: &Path, loader: Loader, game_version: &str) -> Result<PathBuf, HarnessError> {
    let prefix = format!("jarrow-probe-{loader}-{game_version}-");
    let entries =
        fs::read_dir(dir).map_err(|e| HarnessError::io_at("read the probe directory", dir, e))?;
    let mut found: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .filter(|p| {
            p.file_name().and_then(|n| n.to_str()).is_some_and(|name| {
                name.starts_with(&prefix)
                    && name.ends_with(".jar")
                    && !name.ends_with("-sources.jar")
                    && !name.ends_with("-javadoc.jar")
            })
        })
        .collect();
    found.sort();
    match found.len() {
        1 => Ok(found.remove(0)),
        0 => Err(HarnessError::Usage(format!(
            "no {prefix}*.jar in {}",
            dir.display()
        ))),
        _ => Err(HarnessError::Usage(format!(
            "several {loader} probes for {game_version} in {}: {}",
            dir.display(),
            found
                .iter()
                .filter_map(|p| p.file_name())
                .map(|n| n.to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join(", ")
        ))),
    }
}

/// Runs both loaders with only their probe and learns the noise between them.
pub fn run_baseline(
    req: &BaselineRequest,
    env: &HarnessEnv<'_>,
) -> Result<BaselineOutcome, HarnessError> {
    let side = |(loader, probe): &(Loader, PathBuf)| -> Result<RunRecord, HarnessError> {
        let out = sibling(&req.out, loader.id());
        let mut run_req = RunRequest::new(
            *loader,
            &req.game_version,
            probe.clone(),
            out,
            req.cache.clone(),
        );
        run_req.work = req
            .cache
            .join("work")
            .join(format!("baseline-{loader}-{}", req.game_version));
        run_req.timeout = req.timeout;
        run_req.xmx = req.xmx.clone();
        run_req.java_home = req.java_home.clone();
        run_req.poll = req.poll;
        run_req.stop_grace = req.stop_grace;
        run(&run_req, env)
    };
    if req.reference.0 == req.candidate.0 {
        return Err(HarnessError::Usage(
            "a baseline compares two different loaders (--from and --to are the same)".to_owned(),
        ));
    }
    let reference = side(&req.reference)?;
    let candidate = side(&req.candidate)?;
    let noise =
        if reference.status == SideStatus::Completed && candidate.status == SideStatus::Completed {
            Some(learn_noise(
                &sibling(&req.out, &format!("{}.run", req.reference.0)),
                &sibling(&req.out, &format!("{}.run", req.candidate.0)),
                &req.out,
            )?)
        } else {
            None
        };
    Ok(BaselineOutcome {
        reference,
        candidate,
        noise,
    })
}

/// Learns noise from two existing baseline dumps (or completed run records) and writes it.
pub fn learn_noise(reference: &Path, candidate: &Path, out: &Path) -> Result<Noise, HarnessError> {
    let dump = |path: &Path| -> Result<Dump, HarnessError> {
        match load_side(path)? {
            SideInput::Completed { dump, .. } => Ok(*dump),
            _ => Err(HarnessError::Usage(format!(
                "{} is not a completed run, so it cannot be a baseline",
                path.display()
            ))),
        }
    };
    let noise = Noise::learn(&dump(reference)?, &dump(candidate)?)?;
    write_json(out, &noise)?;
    Ok(noise)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::io;
    use std::rc::Rc;

    use crate::dump::tests::sample;
    use crate::eula::EulaGate;
    use crate::process::{CommandOutcome, ServerProcess};
    use crate::provision::tests::{InstallerHost, fake_lock_and_fetcher};
    use crate::testutil::TempDir;

    #[derive(Clone)]
    enum Behaviour {
        /// Writes this dump (with `{LOADER}` filled in) and runs until `stop`.
        Dump(Value),
        /// Logs this and exits with this code.
        Crash(&'static str, i32),
        /// Never writes anything and never exits.
        Hang,
    }

    #[derive(Default)]
    struct Events {
        ran: Vec<CommandSpec>,
        spawned: Vec<CommandSpec>,
        stopped: usize,
        killed: usize,
    }

    struct FakeHost {
        installers: InstallerHost,
        behaviour: Behaviour,
        events: Rc<RefCell<Events>>,
    }

    impl FakeHost {
        fn new(behaviour: Behaviour) -> Self {
            Self {
                installers: InstallerHost::default(),
                behaviour,
                events: Rc::default(),
            }
        }
    }

    impl ProcessHost for FakeHost {
        fn run(&self, spec: &CommandSpec, timeout: Duration) -> io::Result<CommandOutcome> {
            self.events.borrow_mut().ran.push(spec.clone());
            if spec.args_lossy() == ["-version"] {
                fs::write(&spec.log, "openjdk version \"21.0.5\" 2024-10-15 LTS\n")?;
                return Ok(CommandOutcome {
                    exit_code: Some(0),
                    timed_out: false,
                });
            }
            self.installers.run(spec, timeout)
        }

        fn spawn(&self, spec: &CommandSpec) -> io::Result<Box<dyn ServerProcess>> {
            self.events.borrow_mut().spawned.push(spec.clone());
            let mut exited = None;
            match &self.behaviour {
                Behaviour::Dump(value) => {
                    fs::write(&spec.log, "[10:00:00] [Server thread/INFO]: Done (1.0s)!\n")?;
                    fs::write(spec.cwd.join(DUMP_FILE_NAME), value.to_string())?;
                }
                Behaviour::Crash(log, code) => {
                    fs::write(&spec.log, log)?;
                    exited = Some(*code);
                }
                Behaviour::Hang => {}
            }
            Ok(Box::new(FakeServer {
                events: Rc::clone(&self.events),
                exited,
            }))
        }
    }

    struct FakeServer {
        events: Rc<RefCell<Events>>,
        exited: Option<i32>,
    }

    impl ServerProcess for FakeServer {
        fn try_wait(&mut self) -> io::Result<Option<i32>> {
            Ok(self.exited)
        }

        fn send_line(&mut self, line: &str) -> io::Result<()> {
            if line == "stop" {
                self.events.borrow_mut().stopped += 1;
                self.exited = Some(0);
            }
            Ok(())
        }

        fn kill(&mut self) -> io::Result<()> {
            self.events.borrow_mut().killed += 1;
            self.exited = Some(-1);
            Ok(())
        }
    }

    struct Setup {
        tmp: TempDir,
        lock: Lock,
        fetcher: crate::fetch::tests::MemoryFetcher,
        java_home: PathBuf,
    }

    fn setup(tag: &str) -> Setup {
        let tmp = TempDir::new(tag);
        let (lock, fetcher) = fake_lock_and_fetcher(b"launcher");
        let java_home = tmp.path().join("jdk");
        fs::create_dir_all(java_home.join("bin")).unwrap();
        fs::write(java_in(&java_home), b"").unwrap();
        fs::write(tmp.path().join("probe.jar"), b"probe").unwrap();
        fs::write(tmp.path().join("examplemod.jar"), b"mod").unwrap();
        Setup {
            tmp,
            lock,
            fetcher,
            java_home,
        }
    }

    impl Setup {
        fn request(&self, loader: Loader) -> RunRequest {
            let root = self.tmp.path();
            let mut req = RunRequest::new(
                loader,
                "9.9",
                root.join("probe.jar"),
                root.join("out/dump.json"),
                root.join("cache"),
            );
            req.mods = vec![root.join("examplemod.jar")];
            req.java_home = Some(self.java_home.clone());
            req.poll = Duration::from_millis(1);
            req.timeout = Duration::from_millis(200);
            req.stop_grace = Duration::from_millis(50);
            req
        }

        fn run(
            &self,
            req: &RunRequest,
            host: &FakeHost,
            eula: &EulaGate,
        ) -> Result<RunRecord, HarnessError> {
            let env = HarnessEnv {
                lock: &self.lock,
                fetcher: &self.fetcher,
                host,
                eula,
            };
            run(req, &env)
        }
    }

    fn open_gate() -> EulaGate {
        EulaGate::from_lookup(|name| (name == "MINECRAFT_EULA_ACCEPTED").then(|| "true".to_owned()))
    }

    fn dump_for(loader: Loader) -> Value {
        let mut value = sample(loader);
        value["game_version"] = Value::from("9.9");
        value
    }

    fn files_named(dir: &Path, name: &str) -> Vec<PathBuf> {
        let mut found = Vec::new();
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    found.extend(files_named(&path, name));
                } else if path.file_name().is_some_and(|n| n == name) {
                    found.push(path);
                }
            }
        }
        found
    }

    #[test]
    fn a_closed_gate_starts_nothing_downloads_nothing_and_writes_no_eula() {
        let s = setup("run-eula");
        let host = FakeHost::new(Behaviour::Dump(dump_for(Loader::Fabric)));
        for gate in [
            EulaGate::closed(),
            EulaGate::from_lookup(|_| Some("false".to_owned())),
            EulaGate::from_lookup(|_| Some(String::new())),
        ] {
            let req = s.request(Loader::Fabric);
            let record = s.run(&req, &host, &gate).unwrap();
            assert_eq!(record.status, SideStatus::NotRun);
            assert_eq!(record.reason_kind, Some(ReasonKind::HarnessNotRunEula));
            assert!(record.reason.contains("MINECRAFT_EULA_ACCEPTED"));
            assert!(req.record.is_file(), "the not-run record is written");
            assert!(!req.out.exists(), "no dump");
        }
        assert!(
            host.events.borrow().spawned.is_empty(),
            "no server was started"
        );
        assert!(
            host.events.borrow().ran.is_empty(),
            "no installer and no java was run"
        );
        assert!(
            s.fetcher.calls.borrow().is_empty(),
            "nothing was downloaded"
        );
        assert!(
            files_named(s.tmp.path(), "eula.txt").is_empty(),
            "no eula.txt anywhere"
        );
    }

    #[test]
    fn an_open_gate_runs_the_server_and_keeps_its_dump() {
        let s = setup("run-ok");
        let host = FakeHost::new(Behaviour::Dump(dump_for(Loader::Fabric)));
        let req = s.request(Loader::Fabric);
        let record = s.run(&req, &host, &open_gate()).unwrap();
        assert_eq!(record.status, SideStatus::Completed, "{}", record.reason);
        assert_eq!(record.dump.as_deref(), Some("dump.json"));
        assert_eq!(record.exit_code, Some(0));
        assert_eq!(record.java.as_ref().unwrap().version, "21.0.5");
        assert_eq!(record.method.as_deref(), Some("fabric-server-launcher"));
        let roles: Vec<&str> = record.inputs.iter().map(|i| i.role.as_str()).collect();
        assert_eq!(roles, ["probe", "mod", "loader-mod"]);

        let events = host.events.borrow();
        assert_eq!(events.spawned.len(), 1);
        assert_eq!(events.stopped, 1, "stop was sent once the dump appeared");
        assert_eq!(events.killed, 0);
        let spec = &events.spawned[0];
        let args = spec.args_lossy();
        assert_eq!(args[0], "-Xmx3G");
        assert!(
            args.iter()
                .any(|a| a.starts_with("-Djarrow.dump=") && a.ends_with(DUMP_FILE_NAME))
        );
        assert!(args.contains(&"-Dfabric.installer.server.gameJar=server.jar".to_owned()));
        assert_eq!(
            args[args.len() - 3..],
            ["-jar", "fabric-server-launcher.jar", "nogui"].map(String::from)
        );
        assert_eq!(spec.program, java_in(&s.java_home));

        let work = &req.work;
        assert_eq!(
            fs::read_to_string(work.join("eula.txt"))
                .unwrap()
                .lines()
                .last(),
            Some("eula=true")
        );
        let properties = fs::read_to_string(work.join("server.properties")).unwrap();
        for line in [
            "online-mode=false",
            "level-seed=20261103",
            "spawn-protection=0",
            "view-distance=4",
            "simulation-distance=4",
            "server-ip=127.0.0.1",
        ] {
            assert!(
                properties.lines().any(|l| l == line),
                "{line} in {properties}"
            );
        }
        let mut mods: Vec<String> = fs::read_dir(work.join("mods"))
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        mods.sort();
        assert_eq!(
            mods,
            ["examplemod.jar", "fabric-api-1+9.9.jar", "probe.jar"]
        );
        assert_eq!(
            fs::read(work.join("server.jar")).unwrap(),
            b"vanilla server"
        );
        assert!(!work.join(crate::provision::INSTALL_RECORD_FILE).exists());

        let side = load_side(&req.record).unwrap();
        assert!(matches!(side, SideInput::Completed { .. }));
        assert!(matches!(
            load_side(&req.out).unwrap(),
            SideInput::Completed { .. }
        ));
    }

    #[test]
    fn neoforge_launches_through_its_argument_file() {
        let s = setup("run-neo");
        let host = FakeHost::new(Behaviour::Dump(dump_for(Loader::NeoForge)));
        let mut req = s.request(Loader::NeoForge);
        req.mods.clear();
        let record = s.run(&req, &host, &open_gate()).unwrap();
        assert_eq!(record.status, SideStatus::Completed, "{}", record.reason);
        let args = host.events.borrow().spawned[0].args_lossy();
        let expected = if cfg!(windows) {
            "win_args.txt"
        } else {
            "unix_args.txt"
        };
        let argfile = &args[args.len() - 2];
        assert!(
            argfile.starts_with("@libraries/") && argfile.ends_with(expected),
            "{argfile}"
        );
        let mods: Vec<_> = fs::read_dir(req.work.join("mods")).unwrap().collect();
        assert_eq!(mods.len(), 1, "only the probe: NeoForge has no loader mods");
    }

    #[test]
    fn a_crash_is_a_failed_record_with_a_classified_reason() {
        let s = setup("run-crash");
        let log = "[10:00:00] [main/ERROR] (Minecraft) Failed to start the minecraft server\n\
                   java.lang.RuntimeException: Could not execute entrypoint stage 'main'\n\
                   Caused by: java.lang.NoClassDefFoundError: net/fabricmc/fabric/api/example/v1/ExampleEvents\n";
        let host = FakeHost::new(Behaviour::Crash(log, 1));
        let req = s.request(Loader::Fabric);
        let record = s.run(&req, &host, &open_gate()).unwrap();
        assert_eq!(record.status, SideStatus::Failed);
        assert_eq!(record.reason_kind, Some(ReasonKind::ClassNotFound));
        assert!(
            record.reason.contains("exited (code 1)"),
            "{}",
            record.reason
        );
        assert_eq!(record.exit_code, Some(1));
        assert_eq!(record.log.as_deref(), Some("dump.run.log"));
        assert!(req.record.with_extension("log").is_file());
        assert!(!req.out.exists());
        assert!(matches!(
            load_side(&req.record).unwrap(),
            SideInput::Failed {
                reason_kind: ReasonKind::ClassNotFound,
                ..
            }
        ));
    }

    #[test]
    fn a_server_that_never_writes_a_dump_is_stopped_at_the_timeout() {
        let s = setup("run-hang");
        let host = FakeHost::new(Behaviour::Hang);
        let req = s.request(Loader::Fabric);
        let record = s.run(&req, &host, &open_gate()).unwrap();
        assert_eq!(record.status, SideStatus::Failed);
        assert_eq!(record.reason_kind, Some(ReasonKind::HarnessLoadFailure));
        assert!(
            record.reason.contains("without writing jarrow-dump.json"),
            "{}",
            record.reason
        );
        assert_eq!(host.events.borrow().killed, 1);
    }

    #[test]
    fn an_invalid_or_mismatched_dump_is_a_failed_record() {
        let s = setup("run-invalid");
        let mut bad = dump_for(Loader::Fabric);
        bad["registries"] = Value::from(3);
        let host = FakeHost::new(Behaviour::Dump(bad));
        let record = s
            .run(&s.request(Loader::Fabric), &host, &open_gate())
            .unwrap();
        assert_eq!(record.status, SideStatus::Failed);
        assert!(
            record
                .reason
                .contains("does not match harness/dump-schema.json"),
            "{}",
            record.reason
        );

        let mut seed = dump_for(Loader::Fabric);
        seed["worldgen"]["seed"] = Value::from(1);
        let host = FakeHost::new(Behaviour::Dump(seed));
        let record = s
            .run(&s.request(Loader::Fabric), &host, &open_gate())
            .unwrap();
        assert!(
            record.reason.contains("server.properties was not applied"),
            "{}",
            record.reason
        );

        // The probe writes a null seed when worldgen sampling failed: the run still counts.
        let mut no_seed = dump_for(Loader::Fabric);
        no_seed["worldgen"]["seed"] = Value::Null;
        let host = FakeHost::new(Behaviour::Dump(no_seed));
        let record = s
            .run(&s.request(Loader::Fabric), &host, &open_gate())
            .unwrap();
        assert_eq!(record.status, SideStatus::Completed, "{}", record.reason);
        assert!(record.notes.iter().any(|n| n.contains("seed is null")));

        let host = FakeHost::new(Behaviour::Dump(dump_for(Loader::NeoForge)));
        let record = s
            .run(&s.request(Loader::Fabric), &host, &open_gate())
            .unwrap();
        assert!(
            record.reason.contains("ran on neoforge, not fabric"),
            "{}",
            record.reason
        );
    }

    #[test]
    fn a_foreign_work_directory_is_never_cleared() {
        let s = setup("run-foreign");
        let host = FakeHost::new(Behaviour::Dump(dump_for(Loader::Fabric)));
        let mut req = s.request(Loader::Fabric);
        req.work = s.tmp.path().join("precious");
        fs::create_dir_all(&req.work).unwrap();
        fs::write(req.work.join("notes.txt"), b"keep me").unwrap();
        let err = s.run(&req, &host, &open_gate()).unwrap_err();
        assert!(err.to_string().contains("refusing to clear"), "{err}");
        assert_eq!(fs::read(req.work.join("notes.txt")).unwrap(), b"keep me");
        assert!(host.events.borrow().spawned.is_empty());
        // A directory the harness created is cleared and reused.
        let req = s.request(Loader::Fabric);
        s.run(&req, &host, &open_gate()).unwrap();
        fs::write(req.work.join("leftover.txt"), b"x").unwrap();
        s.run(&req, &host, &open_gate()).unwrap();
        assert!(!req.work.join("leftover.txt").exists());
    }

    #[test]
    fn bad_inputs_are_usage_errors() {
        let s = setup("run-usage");
        let host = FakeHost::new(Behaviour::Hang);
        let mut req = s.request(Loader::Fabric);
        req.mods.push(s.tmp.path().join("missing.jar"));
        assert!(matches!(
            s.run(&req, &host, &open_gate()),
            Err(HarnessError::Usage(_))
        ));
        let mut req = s.request(Loader::Fabric);
        req.game_version = "latest".into();
        assert!(matches!(
            s.run(&req, &host, &open_gate()),
            Err(HarnessError::Usage(_))
        ));
        let mut req = s.request(Loader::Fabric);
        req.java_home = None;
        assert!(matches!(
            s.run(&req, &host, &open_gate()),
            Err(HarnessError::Java(_))
        ));
        let req = s.request(Loader::Forge);
        assert!(matches!(
            s.run(&req, &host, &open_gate()),
            Err(HarnessError::NotBuiltYet { .. })
        ));
        assert!(host.events.borrow().spawned.is_empty());
    }

    #[test]
    fn a_failed_download_is_a_not_run_record() {
        let s = setup("run-download");
        let host = FakeHost::new(Behaviour::Hang);
        let mut fetcher = crate::fetch::tests::MemoryFetcher::default();
        fetcher.files = s.fetcher.files.clone();
        fetcher.files.insert(
            "https://mojang.invalid/server.jar".into(),
            b"not the pinned bytes".to_vec(),
        );
        let env = HarnessEnv {
            lock: &s.lock,
            fetcher: &fetcher,
            host: &host,
            eula: &open_gate(),
        };
        let record = run(&s.request(Loader::Fabric), &env).unwrap();
        assert_eq!(record.status, SideStatus::NotRun);
        assert_eq!(record.reason_kind, Some(ReasonKind::DownloadFailed));
        assert!(record.reason.contains("server.jar"), "{}", record.reason);
        assert!(host.events.borrow().spawned.is_empty());
    }

    #[test]
    fn a_baseline_learns_noise_from_both_loaders() {
        let s = setup("baseline");
        let fabric_host = FakeHost::new(Behaviour::Dump(dump_for(Loader::Fabric)));
        let out = s.tmp.path().join("noise/noise.json");
        let req = BaselineRequest {
            game_version: "9.9".into(),
            reference: (Loader::Fabric, s.tmp.path().join("probe.jar")),
            candidate: (Loader::NeoForge, s.tmp.path().join("probe.jar")),
            out: out.clone(),
            cache: s.tmp.path().join("cache"),
            timeout: Duration::from_millis(200),
            xmx: "3G".into(),
            java_home: Some(s.java_home.clone()),
            poll: Duration::from_millis(1),
            stop_grace: Duration::from_millis(50),
        };
        // The fake host writes one dump for every server, so both sides claim to be Fabric and
        // the NeoForge side is refused as mismatched.
        let env = HarnessEnv {
            lock: &s.lock,
            fetcher: &s.fetcher,
            host: &fabric_host,
            eula: &open_gate(),
        };
        let outcome = run_baseline(&req, &env).unwrap();
        assert_eq!(outcome.reference.status, SideStatus::Completed);
        assert_eq!(outcome.candidate.status, SideStatus::Failed);
        assert!(outcome.noise.is_none());
        assert!(!out.exists());

        // From two dumps on disk, the noise is learned without starting anything.
        let neo = s.tmp.path().join("noise/noise.neoforge.json");
        fs::write(&neo, dump_for(Loader::NeoForge).to_string()).unwrap();
        let noise = learn_noise(
            &s.tmp.path().join("noise/noise.fabric.run.json"),
            &neo,
            &out,
        )
        .unwrap();
        assert_eq!(noise.candidate.loader, Loader::NeoForge);
        assert_eq!(
            Noise::from_json(&fs::read_to_string(&out).unwrap()).unwrap(),
            noise
        );

        let same = BaselineRequest {
            candidate: (Loader::Fabric, s.tmp.path().join("probe.jar")),
            ..req
        };
        assert!(run_baseline(&same, &env).is_err());
    }

    #[test]
    fn paths_derive_from_the_output() {
        assert_eq!(
            default_record_path(Path::new("a/dump.json")),
            Path::new("a/dump.run.json")
        );
        assert_eq!(
            sibling(Path::new("a/noise.json"), "fabric"),
            Path::new("a/noise.fabric.json")
        );
        assert_eq!(
            default_work_dir(Path::new("cache"), Loader::NeoForge, "1.21.1"),
            Path::new("cache/work/neoforge-1.21.1")
        );
        assert!(server_properties(25565).contains("\nserver-port=25565\n"));
    }

    #[test]
    fn the_noise_out_is_a_file_or_a_directory() {
        let tmp = TempDir::new("noise-out");
        let (f, n) = (Loader::Fabric, Loader::NeoForge);
        assert_eq!(
            noise_path(None, f, n, "1.21.1"),
            Path::new("target/jarrow-harness/noise-fabric-neoforge-1.21.1.json")
        );
        assert_eq!(
            noise_path(Some(Path::new("out/n.json")), f, n, "1.21.1"),
            Path::new("out/n.json")
        );
        // harness/probe/ci/baseline.sh passes a directory per game version.
        assert_eq!(
            noise_path(Some(Path::new("baseline/1.21.1")), f, n, "1.21.1"),
            Path::new("baseline/1.21.1/noise.json")
        );
        let dir = tmp.path().join("looks.json");
        fs::create_dir_all(&dir).unwrap();
        assert_eq!(
            noise_path(Some(&dir), f, n, "1.21.1"),
            dir.join("noise.json")
        );
    }

    #[test]
    fn probes_are_found_by_the_names_harness_probe_builds() {
        let tmp = TempDir::new("probe-dir");
        let dir = tmp.path();
        for name in [
            "jarrow-probe-fabric-1.21.1-0.1.0.jar",
            "jarrow-probe-fabric-1.21.1-0.1.0-sources.jar",
            "jarrow-probe-neoforge-1.21.1-0.1.0.jar",
            "jarrow-probe-neoforge-1.21.11-0.1.0.jar",
            "SHA256SUMS",
        ] {
            fs::write(dir.join(name), b"x").unwrap();
        }
        assert!(
            find_probe(dir, Loader::Fabric, "1.21.1")
                .unwrap()
                .ends_with("jarrow-probe-fabric-1.21.1-0.1.0.jar")
        );
        assert!(
            find_probe(dir, Loader::NeoForge, "1.21.11")
                .unwrap()
                .ends_with("jarrow-probe-neoforge-1.21.11-0.1.0.jar")
        );
        let err = find_probe(dir, Loader::Forge, "1.21.1").unwrap_err();
        assert!(
            err.to_string()
                .contains("no jarrow-probe-forge-1.21.1-*.jar"),
            "{err}"
        );
        fs::write(dir.join("jarrow-probe-fabric-1.21.1-0.2.0.jar"), b"x").unwrap();
        let err = find_probe(dir, Loader::Fabric, "1.21.1").unwrap_err();
        assert!(err.to_string().contains("several fabric probes"), "{err}");
        assert!(find_probe(&dir.join("missing"), Loader::Fabric, "1.21.1").is_err());
    }
}
