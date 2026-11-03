//! The process boundary: running installers to completion and starting servers.
//!
//! Everything the harness starts goes through [`ProcessHost`], so the runner can be tested
//! with a fake host that starts nothing. [`SystemHost`] is the real one.
//!
//! Child processes get a minimal environment ([`CHILD_ENV_ALLOWLIST`]). A server runs the mods
//! under test, which are third-party code; whatever secrets the harness's own environment
//! holds (CI tokens, the EULA variable) never reach them.

use std::ffi::OsString;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::time::{Duration, Instant};

/// Environment variables passed to child processes when the harness has them. Nothing else is
/// inherited.
pub const CHILD_ENV_ALLOWLIST: [&str; 16] = [
    "PATH",
    "SystemRoot",
    "SystemDrive",
    "windir",
    "ComSpec",
    "TEMP",
    "TMP",
    "TMPDIR",
    "HOME",
    "USERPROFILE",
    "APPDATA",
    "LOCALAPPDATA",
    "LANG",
    "LC_ALL",
    "TZ",
    "NUMBER_OF_PROCESSORS",
];

/// A command to run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSpec {
    /// The executable (an absolute path to `java`).
    pub program: PathBuf,
    /// Arguments.
    pub args: Vec<OsString>,
    /// Working directory.
    pub cwd: PathBuf,
    /// File that receives stdout and stderr, appended.
    pub log: PathBuf,
}

impl CommandSpec {
    /// The arguments as lossy strings, for records and tests.
    pub fn args_lossy(&self) -> Vec<String> {
        self.args
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect()
    }
}

/// How a command that ran to completion ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandOutcome {
    /// Exit code; `None` when it was killed or ended by a signal.
    pub exit_code: Option<i32>,
    /// Whether it was killed for running past the timeout.
    pub timed_out: bool,
}

/// A running server.
pub trait ServerProcess {
    /// The exit code once the process has ended (`-1` when ended by a signal), or `None` while
    /// it runs.
    fn try_wait(&mut self) -> io::Result<Option<i32>>;
    /// Writes one line to the server console (stdin).
    fn send_line(&mut self, line: &str) -> io::Result<()>;
    /// Kills the process and waits for it.
    fn kill(&mut self) -> io::Result<()>;
}

/// Starts processes.
pub trait ProcessHost {
    /// Runs a command to completion, killing it after `timeout`.
    fn run(&self, spec: &CommandSpec, timeout: Duration) -> io::Result<CommandOutcome>;
    /// Starts a server and returns a handle to it.
    fn spawn(&self, spec: &CommandSpec) -> io::Result<Box<dyn ServerProcess>>;
}

/// The real host: `std::process`.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemHost;

fn command(spec: &CommandSpec, log: &File) -> io::Result<Command> {
    let mut command = Command::new(&spec.program);
    command
        .args(&spec.args)
        .current_dir(&spec.cwd)
        .env_clear()
        .stdout(Stdio::from(log.try_clone()?))
        .stderr(Stdio::from(log.try_clone()?));
    for name in CHILD_ENV_ALLOWLIST {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    Ok(command)
}

fn open_log(path: &Path) -> io::Result<File> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    OpenOptions::new().create(true).append(true).open(path)
}

impl ProcessHost for SystemHost {
    fn run(&self, spec: &CommandSpec, timeout: Duration) -> io::Result<CommandOutcome> {
        let log = open_log(&spec.log)?;
        let mut child = command(spec, &log)?.stdin(Stdio::null()).spawn()?;
        let started = Instant::now();
        loop {
            if let Some(status) = child.try_wait()? {
                return Ok(CommandOutcome {
                    exit_code: status.code(),
                    timed_out: false,
                });
            }
            if started.elapsed() >= timeout {
                let _ = child.kill();
                let status = child.wait()?;
                return Ok(CommandOutcome {
                    exit_code: status.code(),
                    timed_out: true,
                });
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    fn spawn(&self, spec: &CommandSpec) -> io::Result<Box<dyn ServerProcess>> {
        let log = open_log(&spec.log)?;
        let mut child = command(spec, &log)?.stdin(Stdio::piped()).spawn()?;
        let stdin = child.stdin.take();
        Ok(Box::new(SystemProcess { child, stdin }))
    }
}

struct SystemProcess {
    child: Child,
    stdin: Option<ChildStdin>,
}

impl ServerProcess for SystemProcess {
    fn try_wait(&mut self) -> io::Result<Option<i32>> {
        Ok(self.child.try_wait()?.map(|s| s.code().unwrap_or(-1)))
    }

    fn send_line(&mut self, line: &str) -> io::Result<()> {
        let stdin = self
            .stdin
            .as_mut()
            .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "stdin is closed"))?;
        stdin.write_all(line.as_bytes())?;
        stdin.write_all(b"\n")?;
        stdin.flush()
    }

    fn kill(&mut self) -> io::Result<()> {
        if self.child.try_wait()?.is_none() {
            // Ignore "already exited" races; wait() below reaps the process either way.
            let _ = self.child.kill();
        }
        self.stdin = None;
        self.child.wait().map(|_| ())
    }
}

impl Drop for SystemProcess {
    fn drop(&mut self) {
        // A server is never left running behind the harness.
        if let Ok(None) = self.child.try_wait() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

/// The path to `java` inside a JDK home.
pub fn java_in(home: &Path) -> PathBuf {
    let name = if cfg!(windows) { "java.exe" } else { "java" };
    home.join("bin").join(name)
}

/// The version string `java -version` printed (`21.0.5`), from its output.
pub fn parse_java_version(output: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let start = line.find("version \"")? + "version \"".len();
        let rest = &line[start..];
        let end = rest.find('"')?;
        Some(rest[..end].to_owned())
    })
}

/// The major version of a Java version string (`21.0.5` is 21, `1.8.0_402` is 8).
pub fn java_major(version: &str) -> Option<u64> {
    let mut parts = version.split(['.', '_', '-', '+']);
    let first: u64 = parts.next()?.parse().ok()?;
    if first == 1 {
        parts.next()?.parse().ok()
    } else {
        Some(first)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn java_versions_parse() {
        let out = "openjdk version \"21.0.12.1\" 2026-08-18 LTS\nOpenJDK Runtime Environment Temurin-21.0.12.1+1\n";
        assert_eq!(parse_java_version(out).as_deref(), Some("21.0.12.1"));
        assert_eq!(java_major("21.0.12.1"), Some(21));
        assert_eq!(java_major("1.8.0_402"), Some(8));
        assert_eq!(java_major("25"), Some(25));
        assert_eq!(java_major("21-ea"), Some(21));
        assert_eq!(java_major("x"), None);
        assert_eq!(parse_java_version("no version here"), None);
    }

    #[test]
    fn java_lives_in_bin() {
        let java = java_in(Path::new("jdk"));
        assert!(java.starts_with(Path::new("jdk").join("bin")));
    }

    #[test]
    fn the_allowlist_holds_no_secret_or_harness_variables() {
        for name in CHILD_ENV_ALLOWLIST {
            let upper = name.to_ascii_uppercase();
            assert!(
                !upper.contains("TOKEN") && !upper.contains("KEY") && !upper.contains("SECRET")
            );
            assert!(
                !upper.contains("EULA")
                    && !upper.contains("GITHUB")
                    && !upper.contains("CLOUDFLARE")
            );
        }
    }
}
