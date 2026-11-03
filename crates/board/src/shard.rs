//! One shard of the nightly: one slice of a source loader's list, measured for one direction.
//! Download, verify, run the pipeline, delete, report.
//!
//! Shard assignment is a stable modulo: a mod belongs to shard `fnv1a64(project_id) % total`,
//! so a mod stays in the same shard however the list's order or length changes.

use std::fmt;
use std::fs::{self, File};
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Instant;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::board::{BoardResult, BoardRow, Engine, ShardStats, StepDurations, page_url};
use crate::gamever;
use crate::hash::Sha512Stream;
use crate::http::{Fetch, ResettableSink};
use crate::list::{BoardList, ListFile, ListMod};
use crate::loader::{Direction, Loader};
use crate::pipeline::{self, ModContext, ModPipeline};
use crate::report::{
    self, ClassifyStep, ConvertStep, DownloadStep, HarnessStep, InspectStep, ReasonKind, Report,
    ReportFile, RoundTripStep, StepStatus, Steps, board_outcome,
};
use crate::time::{self, Clock};
use crate::{Error, SCHEMA};

/// The largest JAR the board downloads, in bytes.
pub const MAX_FILE_BYTES: u64 = 512 * 1024 * 1024;

/// FNV-1a, 64 bit: a stable, dependency-free hash for shard assignment.
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// The 0-based bucket of a project among `total` shards.
pub fn bucket(project_id: &str, total: u32) -> u32 {
    let total = u64::from(total.max(1));
    u32::try_from(fnv1a64(project_id.as_bytes()) % total).unwrap_or(0)
}

/// `--shard N/M`: shard `N` (1-based) of `M`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ShardSpec {
    /// 1-based index.
    pub index: u32,
    /// Number of shards.
    pub total: u32,
}

impl ShardSpec {
    /// A shard spec; `1 <= index <= total <= 1000`.
    pub fn new(index: u32, total: u32) -> Result<Self, Error> {
        if total == 0 || total > 1000 || index == 0 || index > total {
            return Err(Error::invalid(format!(
                "shard {index}/{total} is out of range: use N/M with 1 <= N <= M <= 1000"
            )));
        }
        Ok(Self { index, total })
    }

    /// Whether the project belongs to this shard.
    pub fn contains(self, project_id: &str) -> bool {
        bucket(project_id, self.total) == self.index - 1
    }
}

impl fmt::Display for ShardSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.index, self.total)
    }
}

impl FromStr for ShardSpec {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bad = || Error::invalid(format!("shard {s:?} must look like 3/10"));
        let (n, m) = s.trim().split_once('/').ok_or_else(bad)?;
        let n = n.trim().parse::<u32>().map_err(|_| bad())?;
        let m = m.trim().parse::<u32>().map_err(|_| bad())?;
        ShardSpec::new(n, m)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ShardSpecJson {
    index: u32,
    total: u32,
}

impl Serialize for ShardSpec {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        ShardSpecJson {
            index: self.index,
            total: self.total,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ShardSpec {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = ShardSpecJson::deserialize(deserializer)?;
        ShardSpec::new(raw.index, raw.total).map_err(serde::de::Error::custom)
    }
}

/// The file name of a shard: `shard-<direction>-<n>.json`.
pub fn file_name(direction: Direction, spec: ShardSpec) -> String {
    format!("shard-{direction}-{}.json", spec.index)
}

/// `shard-<direction>-<n>.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShardFile {
    /// Always [`SCHEMA`].
    pub schema: u32,
    /// The run, for example `nightly-20261104`.
    pub run_id: String,
    /// The direction.
    pub direction: Direction,
    /// Its source loader.
    pub source_loader: Loader,
    /// Its target loader.
    pub target_loader: Loader,
    /// The pinned game versions, from the source list.
    pub game_versions: Vec<String>,
    /// When the source list was fetched.
    pub list_fetched_at: String,
    /// Which shard this is.
    pub shard: ShardSpec,
    /// The engine.
    pub engine: Engine,
    /// Content hash of the direction's rule tables, or `none`.
    pub rules_version: String,
    /// When the shard started.
    pub started_at: String,
    /// When the shard finished.
    pub finished_at: String,
    /// One row per mod of the shard.
    pub rows: Vec<BoardRow>,
    /// Counts and durations.
    pub stats: ShardStats,
}

impl ShardFile {
    /// Parses `shard-<direction>-<n>.json`.
    pub fn from_json(bytes: &[u8], what: &str) -> Result<Self, Error> {
        serde_json::from_slice(bytes).map_err(|e| Error::json(what.to_owned(), e))
    }

    /// Pretty JSON with a trailing newline.
    pub fn to_json(&self) -> Result<Vec<u8>, Error> {
        let mut out = serde_json::to_vec_pretty(self).map_err(|e| Error::json("shard", e))?;
        out.push(b'\n');
        Ok(out)
    }
}

/// Settings of a shard run.
#[derive(Debug, Clone)]
pub struct ShardOptions {
    /// The run id all shards of the night share.
    pub run_id: String,
    /// The commit being run.
    pub git_sha: String,
    /// The engine version recorded in the shard and in every report. `jarrow-board` passes its
    /// own, [`crate::ENGINE_VERSION`]; tests that pin bytes pass a fixed one.
    pub engine_version: String,
    /// The rules version (see [`rules_version`]).
    pub rules_version: String,
    /// Where the reports go: `<dir>/<direction>/<project_id>/<game_version>.json`.
    pub reports_dir: PathBuf,
    /// Where JARs are downloaded (a fresh temporary directory inside it); the system temp
    /// directory when `None`.
    pub work_dir: Option<PathBuf>,
    /// Files larger than this are not downloaded.
    pub max_file_bytes: u64,
}

/// Whether `run_id` is safe as a file name part, a KV key part and a SQL literal.
pub fn is_run_id(run_id: &str) -> bool {
    !run_id.is_empty()
        && run_id.len() <= 64
        && run_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

/// The content hash of a direction's rule tables: SHA-256 over every file under
/// `<rules_dir>/<direction>/`, as sorted `path\0sha256\n` lines, or `none` when the
/// direction has no tables yet.
pub fn rules_version(rules_dir: &Path, direction: Direction) -> Result<String, Error> {
    let root = rules_dir.join(direction.id());
    if !root.is_dir() {
        return Ok("none".to_owned());
    }
    let mut files = Vec::new();
    let mut stack = vec![root.clone()];
    while let Some(dir) = stack.pop() {
        let entries = fs::read_dir(&dir).map_err(|e| Error::io_at("reading", &dir, e))?;
        for entry in entries {
            let entry = entry.map_err(|e| Error::io_at("reading", &dir, e))?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                let rel = path
                    .strip_prefix(&root)
                    .map(|p| p.to_string_lossy().replace('\\', "/"))
                    .unwrap_or_default();
                let bytes = fs::read(&path).map_err(|e| Error::io_at("reading", &path, e))?;
                files.push((rel, crate::hash::sha256_hex(&bytes)));
            }
        }
    }
    if files.is_empty() {
        return Ok("none".to_owned());
    }
    files.sort();
    let mut manifest = String::new();
    for (path, digest) in files {
        manifest.push_str(&path);
        manifest.push('\0');
        manifest.push_str(&digest);
        manifest.push('\n');
    }
    Ok(format!(
        "sha256:{}",
        crate::hash::sha256_hex(manifest.as_bytes())
    ))
}

/// A download target that hashes as it writes and can start over.
struct JarSink {
    file: File,
    hasher: Sha512Stream,
}

impl Write for JarSink {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let n = self.file.write(buf)?;
        self.hasher.update(&buf[..n]);
        Ok(n)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
}

impl ResettableSink for JarSink {
    fn reset(&mut self) -> std::io::Result<()> {
        self.file.set_len(0)?;
        self.file.seek(SeekFrom::Start(0))?;
        self.hasher = Sha512Stream::new();
        Ok(())
    }
}

fn millis(start: Instant) -> u64 {
    u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX)
}

/// Downloads `file` to `path`, checks size and sha512. On failure the partial file is
/// removed and the error is returned as text for the report.
fn download_verified(
    fetch: &dyn Fetch,
    file: &ListFile,
    path: &Path,
    max_file_bytes: u64,
) -> Result<u64, String> {
    if file.size > max_file_bytes {
        return Err(format!(
            "the file is {} bytes, above the board's {max_file_bytes}-byte limit",
            file.size
        ));
    }
    let handle = File::create(path).map_err(|e| format!("could not create the download: {e}"))?;
    let mut sink = JarSink {
        file: handle,
        hasher: Sha512Stream::new(),
    };
    let result = fetch
        .download(&file.url, &mut sink, file.size)
        .map_err(|e| format!("download failed: {e}"))
        .and_then(|n| {
            sink.flush()
                .map_err(|e| format!("could not write the download: {e}"))?;
            Ok(n)
        });
    let JarSink {
        file: handle,
        hasher,
    } = sink;
    drop(handle);
    let checked = result.and_then(|n| {
        if n != file.size {
            return Err(format!(
                "size mismatch: the list says {} bytes, the CDN sent {n}",
                file.size
            ));
        }
        let digest = hasher.finish_hex();
        if digest != file.sha512 {
            return Err(
                "sha512 mismatch: the downloaded file is not the one the list names".into(),
            );
        }
        Ok(n)
    });
    if checked.is_err() {
        // Best effort: the temporary directory is removed at the end of the shard anyway.
        let _ = fs::remove_file(path);
    }
    checked
}

/// What processing one file produced.
struct FileOutcome {
    report: Report,
    result: BoardResult,
    downloaded_bytes: u64,
    download_failed: bool,
    inspect_failed: bool,
    step_ms: StepDurations,
}

fn not_run_steps(download_error: String) -> Steps {
    let not_run = "not run: the file could not be downloaded";
    Steps {
        download: DownloadStep {
            status: StepStatus::Failed,
            sha512_verified: false,
            error: Some(download_error.clone()),
        },
        inspect: InspectStep {
            status: StepStatus::NotRun,
            size: None,
            entries: None,
            manifests: None,
            error: Some(not_run.to_owned()),
        },
        classify: ClassifyStep {
            status: StepStatus::NotRun,
            tier: None,
            reasons: vec!["not classified: the file could not be downloaded".to_owned()],
        },
        convert: ConvertStep {
            status: StepStatus::NotRun,
            reason_kind: ReasonKind::DownloadFailed,
            reason: download_error,
        },
        harness: HarnessStep {
            status: StepStatus::NotRun,
            score: None,
            reason_kind: None,
            reason: not_run.to_owned(),
        },
        round_trip: RoundTripStep {
            status: StepStatus::NotRun,
            reason: not_run.to_owned(),
        },
    }
}

#[allow(clippy::too_many_arguments)]
fn process_file(
    fetch: &dyn Fetch,
    pipeline: &dyn ModPipeline,
    direction: Direction,
    project: &ListMod,
    file: &ListFile,
    work: &Path,
    max_file_bytes: u64,
    engine_version: &str,
) -> Result<FileOutcome, Error> {
    let started = Instant::now();
    let mut step_ms = StepDurations::default();
    let jar = work.join(format!("{}-{}.jar", project.project_id, file.game_version));
    let ctx = ModContext {
        direction,
        project,
        file,
    };

    let t = Instant::now();
    let downloaded = download_verified(fetch, file, &jar, max_file_bytes);
    step_ms.download = millis(t);

    let (steps, downloaded_bytes, download_failed) = match downloaded {
        Err(error) => (not_run_steps(error), 0, true),
        Ok(bytes) => {
            let t = Instant::now();
            let inspect = pipeline.inspect(&ctx, &jar);
            step_ms.inspect = millis(t);
            let t = Instant::now();
            let classify = pipeline.classify(&ctx, &jar, &inspect);
            step_ms.classify = millis(t);
            let t = Instant::now();
            let convert = pipeline.convert(&ctx, &jar, &classify);
            step_ms.convert = millis(t);
            let t = Instant::now();
            let harness = pipeline.harness(&ctx, &convert);
            step_ms.harness = millis(t);
            let t = Instant::now();
            let round_trip = pipeline.round_trip(&ctx, &jar);
            step_ms.round_trip = millis(t);
            fs::remove_file(&jar).map_err(|e| Error::io_at("deleting", &jar, e))?;
            let steps = Steps {
                download: DownloadStep {
                    status: StepStatus::Ok,
                    sha512_verified: true,
                    error: None,
                },
                inspect,
                classify,
                convert,
                harness,
                round_trip,
            };
            (steps, bytes, false)
        }
    };
    let inspect_failed = steps.inspect.status == StepStatus::Failed;
    let verdict = pipeline::decide(&steps);
    let reason_for_entry = verdict
        .reason
        .clone()
        .unwrap_or_else(|| "not converted".to_owned());
    let report = Report {
        schema: SCHEMA,
        engine_version: engine_version.to_owned(),
        direction,
        project_id: project.project_id.clone(),
        slug: project.slug.clone(),
        title: project.title.clone(),
        game_version: file.game_version.clone(),
        file: ReportFile {
            version_id: file.version_id.clone(),
            version_number: file.version_number.clone(),
            filename: file.filename.clone(),
            url: file.url.clone(),
            sha512: file.sha512.clone(),
            size: file.size,
        },
        tier: steps.classify.tier,
        outcome: verdict.outcome,
        score: verdict.score,
        reason_kind: verdict.reason_kind,
        reason: verdict.reason.clone(),
        entries: if verdict.outcome == jarrow_report::Outcome::Green {
            Vec::new()
        } else {
            vec![report::not_converted_entry(&reason_for_entry)]
        },
        steps,
    };
    report.validate()?;
    let result = BoardResult {
        direction,
        game_version: file.game_version.clone(),
        version_id: file.version_id.clone(),
        version_number: file.version_number.clone(),
        file_sha512: file.sha512.clone(),
        mod_id: None,
        mod_version: Some(file.version_number.clone()),
        tier: report.tier,
        tier_reasons: report.steps.classify.reasons.clone(),
        outcome: report.outcome,
        score: report.score,
        reason_kind: report.reason_kind,
        reason: report.reason.clone(),
        report_key: report::report_key(direction, &project.project_id, &file.game_version),
        report_hash: report.hash()?,
        duration_ms: millis(started),
    };
    Ok(FileOutcome {
        report,
        result,
        downloaded_bytes,
        download_failed,
        inspect_failed,
        step_ms,
    })
}

/// Writes a report's canonical JSON under `reports_dir`.
pub fn write_report(reports_dir: &Path, report: &Report) -> Result<PathBuf, Error> {
    let rel = report::report_path(report.direction, &report.project_id, &report.game_version);
    let path = reports_dir.join(&rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| Error::io_at("creating", parent, e))?;
    }
    fs::write(&path, report.canonical_json()?).map_err(|e| Error::io_at("writing", &path, e))?;
    Ok(path)
}

/// The mods of `list` that belong to `spec`, in list order.
pub fn select(list: &BoardList, spec: ShardSpec) -> Vec<&ListMod> {
    list.mods
        .iter()
        .filter(|m| spec.contains(&m.project_id))
        .collect()
}

/// Runs one shard of `list` for `direction`: every file of every mod in the shard, one after
/// another. The direction's source loader must be the list's loader.
///
/// Per file: download into a fresh temporary directory, verify size and sha512, run the
/// pipeline, delete the JAR, write the report. A failed download is a `not-run` result with
/// `download_failed` (a red row), not an error; an error stops the shard (a full disk, an
/// unwritable reports directory, a malformed list).
#[allow(clippy::too_many_arguments)]
pub fn run_shard(
    fetch: &dyn Fetch,
    pipeline: &dyn ModPipeline,
    list: &BoardList,
    direction: Direction,
    spec: ShardSpec,
    opts: &ShardOptions,
    clock: &dyn Clock,
    mut progress: impl FnMut(&str),
) -> Result<ShardFile, Error> {
    list.validate()?;
    if direction.source != list.loader {
        return Err(Error::invalid(format!(
            "{direction} measures {} mods, but {} lists {} mods",
            direction.source.name(),
            crate::list::file_name(list.loader),
            list.loader.name()
        )));
    }
    if !is_run_id(&opts.run_id) {
        return Err(Error::invalid(format!(
            "run id {:?} may use only letters, digits, '-', '_' and '.' (at most 64)",
            opts.run_id
        )));
    }
    let started_at = clock.now();
    let wall = Instant::now();
    let work_parent = opts.work_dir.clone().unwrap_or_else(std::env::temp_dir);
    fs::create_dir_all(&work_parent).map_err(|e| Error::io_at("creating", &work_parent, e))?;
    let work = tempfile::Builder::new()
        .prefix("jarrow-board-")
        .tempdir_in(&work_parent)
        .map_err(|e| Error::io_at("creating a temporary directory in", &work_parent, e))?;
    fs::create_dir_all(&opts.reports_dir)
        .map_err(|e| Error::io_at("creating", &opts.reports_dir, e))?;

    let mods = select(list, spec);
    let mut rows = Vec::with_capacity(mods.len());
    let mut stats = ShardStats::default();
    for (i, project) in mods.iter().enumerate() {
        let mut results = Vec::with_capacity(project.files.len());
        for file in &project.files {
            let outcome = process_file(
                fetch,
                pipeline,
                direction,
                project,
                file,
                work.path(),
                opts.max_file_bytes,
                &opts.engine_version,
            )?;
            write_report(&opts.reports_dir, &outcome.report)?;
            stats.files += 1;
            stats.downloaded_bytes += outcome.downloaded_bytes;
            stats.download_failures += u64::from(outcome.download_failed);
            stats.inspect_failures += u64::from(outcome.inspect_failed);
            stats.step_ms = stats.step_ms.plus(outcome.step_ms);
            progress(&format!(
                "[{}/{}] {} {}: {} ({})",
                i + 1,
                mods.len(),
                project.slug,
                file.game_version,
                outcome.result.outcome.id(),
                outcome.result.reason_kind.map_or("no reason", |k| k.id()),
            ));
            results.push(outcome.result);
        }
        results.sort_by(|a, b| gamever::compare(&a.game_version, &b.game_version));
        let Some(primary) = results.last().cloned() else {
            continue;
        };
        stats.mods += 1;
        rows.push(BoardRow {
            direction,
            project_id: project.project_id.clone(),
            slug: project.slug.clone(),
            title: project.title.clone(),
            downloads: project.downloads,
            page_url: page_url(&project.slug, &project.project_id),
            icon_url: project.icon_url.clone(),
            license: project.license.clone(),
            primary_game_version: primary.game_version.clone(),
            tier: primary.tier,
            outcome: board_outcome(primary.outcome, primary.reason_kind),
            reason_kind: primary.reason_kind,
            reason: primary.reason.clone(),
            last_run: clock.now(),
            report_url: None,
            target_build_exists: project.tagged(direction.target),
            results,
        });
    }
    work.close()
        .map_err(|e| Error::io("removing the shard's temporary directory", e))?;
    stats.wall_ms = millis(wall);
    let finished_at = clock.now();
    if !time::is_rfc3339_utc(&started_at) || !time::is_rfc3339_utc(&finished_at) {
        return Err(Error::invalid("the clock returned a malformed time"));
    }
    Ok(ShardFile {
        schema: SCHEMA,
        run_id: opts.run_id.clone(),
        direction,
        source_loader: direction.source,
        target_loader: direction.target,
        game_versions: list.game_versions.clone(),
        list_fetched_at: list.fetched_at.clone(),
        shard: spec,
        engine: Engine {
            version: opts.engine_version.clone(),
            git_sha: opts.git_sha.clone(),
        },
        rules_version: opts.rules_version.clone(),
        started_at,
        finished_at,
        rows,
        stats,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv_known_values() {
        // Reference values of FNV-1a 64 (draft-eastlake-fnv).
        assert_eq!(fnv1a64(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a64(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(fnv1a64(b"foobar"), 0x8594_4171_f739_67e8);
    }

    #[test]
    fn specs_parse_and_validate() {
        assert_eq!(
            "3/10".parse::<ShardSpec>().unwrap(),
            ShardSpec::new(3, 10).unwrap()
        );
        assert_eq!(" 1 / 1 ".parse::<ShardSpec>().unwrap().to_string(), "1/1");
        for bad in [
            "0/10", "11/10", "1/0", "a/b", "3", "3/10/1", "-1/10", "1/1001",
        ] {
            assert!(bad.parse::<ShardSpec>().is_err(), "{bad}");
        }
        let json = serde_json::to_string(&ShardSpec::new(2, 10).unwrap()).unwrap();
        assert_eq!(json, r#"{"index":2,"total":10}"#);
        assert!(serde_json::from_str::<ShardSpec>(r#"{"index":0,"total":10}"#).is_err());
    }

    #[test]
    fn every_project_lands_in_exactly_one_shard() {
        let ids: Vec<String> = (0..2000).map(|i| format!("P{i:07}")).collect();
        let specs: Vec<ShardSpec> = (1..=10).map(|i| ShardSpec::new(i, 10).unwrap()).collect();
        let mut sizes = [0usize; 10];
        for id in &ids {
            let owners: Vec<usize> = specs
                .iter()
                .enumerate()
                .filter(|(_, s)| s.contains(id))
                .map(|(i, _)| i)
                .collect();
            assert_eq!(owners.len(), 1, "{id}");
            sizes[owners[0]] += 1;
        }
        // A stable hash spreads 2000 ids roughly evenly: no shard is empty or holds a third.
        assert!(sizes.iter().all(|&n| n > 100 && n < 700), "{sizes:?}");
    }

    #[test]
    fn assignment_is_stable() {
        // Pinned values: changing the hash would move every mod to another shard.
        assert_eq!(bucket("AANobbMI", 10), (fnv1a64(b"AANobbMI") % 10) as u32);
        assert_eq!(bucket("P7dR8mSH", 10), bucket("P7dR8mSH", 10));
        assert_eq!(bucket("anything", 1), 0);
    }

    #[test]
    fn file_names() {
        let d: Direction = "fabric-neoforge".parse().unwrap();
        assert_eq!(
            file_name(d, ShardSpec::new(3, 10).unwrap()),
            "shard-fabric-neoforge-3.json"
        );
    }

    #[test]
    fn run_ids() {
        assert!(is_run_id("nightly-20261104"));
        assert!(is_run_id("local-20261103.2"));
        for bad in ["", "a b", "x'y", "a/b", "a:b"] {
            assert!(!is_run_id(bad), "{bad}");
        }
    }

    #[test]
    fn rules_version_is_none_without_tables_and_a_hash_with_them() {
        let d: Direction = "fabric-neoforge".parse().unwrap();
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(rules_version(dir.path(), d).unwrap(), "none");
        fs::create_dir_all(dir.path().join("fabric-neoforge/1.21.1")).unwrap();
        assert_eq!(rules_version(dir.path(), d).unwrap(), "none");
        fs::write(dir.path().join("fabric-neoforge/1.21.1/a.toml"), b"x").unwrap();
        let v1 = rules_version(dir.path(), d).unwrap();
        assert!(v1.starts_with("sha256:") && v1.len() == 71, "{v1}");
        fs::write(dir.path().join("fabric-neoforge/1.21.1/a.toml"), b"y").unwrap();
        assert_ne!(rules_version(dir.path(), d).unwrap(), v1);
    }
}
