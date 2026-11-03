//! `jarrow`: the command-line shell around the Jarrow engine.
//!
//! Built so far: `jarrow --version` and `jarrow tokens` (CSS, or JSON with `--json`). The `check` command arrives with the
//! classifier (milestone M1) and `convert` with the engine (milestone M2); until then they do
//! not exist, rather than pretending to work.

#![forbid(unsafe_code)]

use std::io::{self, Write};
use std::process::ExitCode;

use clap::{Parser, Subcommand};

const NOT_BUILT_YET: &str = "\
Not built yet:
  jarrow check     classify a JAR into Tier A, Tier B or unsupported (milestone M1)
  jarrow convert   convert a Fabric JAR to NeoForge with a report (milestone M2)

Progress: https://github.com/jarrow-dev/jarrow/blob/main/LEDGER.md";

/// Jarrow converts Minecraft Fabric mod JARs to NeoForge and proves the result with a
/// behaviour harness. This build has no converter yet.
#[derive(Debug, Parser)]
#[command(
    name = "jarrow",
    bin_name = "jarrow",
    version,
    about = "Fabric to NeoForge mod JAR converter with a parity report (in development; no converter yet).",
    after_help = NOT_BUILT_YET,
    arg_required_else_help = true
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Print the design tokens as CSS (the source of site/tokens.css).
    Tokens {
        /// Print them as JSON instead (the tokens.json of the npm package `jarrow`).
        #[arg(long)]
        json: bool,
    },
}

fn run(cli: Cli) -> io::Result<()> {
    match cli.command {
        Command::Tokens { json } => {
            let text = if json {
                jarrow_report::tokens::json()
            } else {
                jarrow_report::tokens::css()
            };
            let mut stdout = io::stdout().lock();
            stdout.write_all(text.as_bytes())?;
            stdout.flush()
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        // `jarrow tokens | head` closes the pipe early; that is not an error.
        Err(err) if err.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("jarrow: {err}");
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
    fn only_built_commands_exist() {
        let names: Vec<String> = Cli::command()
            .get_subcommands()
            .map(|c| c.get_name().to_owned())
            .collect();
        assert_eq!(names, ["tokens"]);
        assert!(Cli::try_parse_from(["jarrow", "convert", "x.jar"]).is_err());
        assert!(Cli::try_parse_from(["jarrow", "check", "x.jar"]).is_err());
    }
}
