//! End-to-end tests of the `jarrow` binary.

use std::error::Error;
use std::process::Command;

fn jarrow(args: &[&str]) -> Result<std::process::Output, Box<dyn Error>> {
    Ok(Command::new(env!("CARGO_BIN_EXE_jarrow"))
        .args(args)
        .output()?)
}

#[test]
fn version_prints_the_package_version() -> Result<(), Box<dyn Error>> {
    let out = jarrow(&["--version"])?;
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout)?;
    assert_eq!(
        stdout.trim_end(),
        format!("jarrow {}", env!("CARGO_PKG_VERSION"))
    );
    Ok(())
}

#[test]
fn tokens_prints_the_generated_css() -> Result<(), Box<dyn Error>> {
    let out = jarrow(&["tokens"])?;
    assert!(out.status.success());
    assert_eq!(String::from_utf8(out.stdout)?, jarrow_report::tokens::css());
    Ok(())
}

#[test]
fn tokens_json_prints_the_generated_json() -> Result<(), Box<dyn Error>> {
    let out = jarrow(&["tokens", "--json"])?;
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout)?;
    assert_eq!(stdout, jarrow_report::tokens::json());
    assert!(stdout.starts_with('{'), "{stdout}");
    assert!(stdout.contains(r#""name": "jw-accent""#), "{stdout}");
    Ok(())
}

#[test]
fn tokens_rejects_unknown_flags() -> Result<(), Box<dyn Error>> {
    let out = jarrow(&["tokens", "--yaml"])?;
    assert!(!out.status.success());
    assert!(out.stdout.is_empty());
    Ok(())
}

#[test]
fn help_says_what_is_not_built() -> Result<(), Box<dyn Error>> {
    let out = jarrow(&["--help"])?;
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout)?;
    assert!(stdout.contains("no converter yet"), "{stdout}");
    assert!(stdout.contains("(milestone M1)"), "{stdout}");
    assert!(stdout.contains("(milestone M2)"), "{stdout}");
    assert!(!stdout.to_lowercase().contains("guarantee"));
    Ok(())
}

#[test]
fn unbuilt_commands_fail() -> Result<(), Box<dyn Error>> {
    for args in [["convert", "mod.jar"], ["check", "mod.jar"]] {
        let out = jarrow(&args)?;
        assert!(!out.status.success(), "{args:?} must not pretend to work");
        assert!(out.stdout.is_empty());
    }
    Ok(())
}

#[test]
fn no_arguments_prints_help_and_fails() -> Result<(), Box<dyn Error>> {
    let out = jarrow(&[])?;
    assert!(!out.status.success());
    let stderr = String::from_utf8(out.stderr)?;
    assert!(stderr.contains("Usage: jarrow"), "{stderr}");
    Ok(())
}
