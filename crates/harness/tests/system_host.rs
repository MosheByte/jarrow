//! Real processes through `SystemHost`, with this test binary as the child: the console, the
//! captured output, the exit code, the timeout, `kill`, and the cleared environment.

use std::error::Error;
use std::fs;
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use jarrow_harness::process::{CommandSpec, ProcessHost, SystemHost};

/// The child reads this file in its working directory to know what to do. Without it the
/// child does nothing, so running the ignored test directly is harmless.
const MARKER: &str = "jarrow-harness-child-mode";

/// The child side of the tests below; they start it through `SystemHost`. On its own it does
/// nothing: `cargo test -p jarrow-harness --test system_host -- --ignored child_console`.
#[test]
#[ignore = "child process of the SystemHost tests; they start it"]
fn child_console() {
    let Ok(mode) = fs::read_to_string(MARKER) else {
        return;
    };
    println!("child: ready");
    println!(
        "child: CARGO_MANIFEST_DIR visible: {}",
        std::env::var_os("CARGO_MANIFEST_DIR").is_some()
    );
    match mode.trim() {
        "exit7" => std::process::exit(7),
        "hang" => loop {
            std::thread::sleep(Duration::from_secs(1));
        },
        _ => {
            for line in std::io::stdin().lock().lines() {
                if line.unwrap() == "stop" {
                    println!("child: stopping");
                    return;
                }
            }
        }
    }
}

fn child(tag: &str, mode: &str) -> Result<CommandSpec, Box<dyn Error>> {
    let dir =
        std::env::temp_dir().join(format!("jarrow-harness-child-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir)?;
    fs::write(dir.join(MARKER), mode)?;
    Ok(CommandSpec {
        program: std::env::current_exe()?,
        args: [
            "child_console",
            "--exact",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ]
        .map(Into::into)
        .to_vec(),
        log: dir.join("child.log"),
        cwd: dir,
    })
}

fn wait_for_log(log: &Path, needle: &str) -> Result<String, Box<dyn Error>> {
    let started = Instant::now();
    loop {
        let text = fs::read_to_string(log).unwrap_or_default();
        if text.contains(needle) {
            return Ok(text);
        }
        if started.elapsed() > Duration::from_secs(60) {
            return Err(format!("{needle:?} never appeared in {}: {text}", log.display()).into());
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn cleanup(spec: &CommandSpec) {
    let _ = fs::remove_dir_all(&spec.cwd);
}

#[test]
fn a_server_reads_its_console_and_its_output_is_captured() -> Result<(), Box<dyn Error>> {
    let spec = child("console", "console")?;
    let mut process = SystemHost.spawn(&spec)?;
    let text = wait_for_log(&spec.log, "child: ready")?;
    assert!(
        wait_for_log(&spec.log, "CARGO_MANIFEST_DIR visible:")?.contains("visible: false"),
        "the child must not inherit the harness's environment: {text}"
    );
    assert_eq!(process.try_wait()?, None);
    process.send_line("stop")?;
    let started = Instant::now();
    let code = loop {
        if let Some(code) = process.try_wait()? {
            break code;
        }
        assert!(
            started.elapsed() < Duration::from_secs(60),
            "the child did not stop"
        );
        std::thread::sleep(Duration::from_millis(20));
    };
    assert_eq!(code, 0);
    wait_for_log(&spec.log, "child: stopping")?;
    cleanup(&spec);
    Ok(())
}

#[test]
fn run_reports_the_exit_code() -> Result<(), Box<dyn Error>> {
    let spec = child("exit", "exit7")?;
    let outcome = SystemHost.run(&spec, Duration::from_secs(60))?;
    assert_eq!(outcome.exit_code, Some(7));
    assert!(!outcome.timed_out);
    assert!(fs::read_to_string(&spec.log)?.contains("child: ready"));
    cleanup(&spec);
    Ok(())
}

#[test]
fn run_kills_at_the_timeout_and_kill_stops_a_server() -> Result<(), Box<dyn Error>> {
    let spec = child("hang-run", "hang")?;
    let outcome = SystemHost.run(&spec, Duration::from_millis(1500))?;
    assert!(outcome.timed_out);
    cleanup(&spec);

    let spec = child("hang-spawn", "hang")?;
    let mut process = SystemHost.spawn(&spec)?;
    wait_for_log(&spec.log, "child: ready")?;
    process.kill()?;
    assert!(process.try_wait()?.is_some());
    cleanup(&spec);
    Ok(())
}

#[test]
fn a_missing_program_is_an_error() {
    let spec = CommandSpec {
        program: PathBuf::from("this-program-does-not-exist-jarrow"),
        args: Vec::new(),
        cwd: std::env::temp_dir(),
        log: std::env::temp_dir()
            .join(format!("jarrow-harness-missing-{}.log", std::process::id())),
    };
    assert!(SystemHost.spawn(&spec).is_err());
    assert!(SystemHost.run(&spec, Duration::from_secs(1)).is_err());
    let _ = fs::remove_file(&spec.log);
}
