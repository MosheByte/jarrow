//! Keeps `site/tokens.css` identical to `jarrow_report::tokens::css()`.
//!
//! Run with `JARROW_BLESS=1` to (re)write the file instead of checking it:
//!
//! ```text
//! JARROW_BLESS=1 cargo test -p jarrow-report --test tokens_current
//! ```

use std::error::Error;
use std::fs;
use std::path::PathBuf;

/// The file as named in messages, relative to the repository root.
const DISPLAY: &str = "site/tokens.css";

fn tokens_css_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("site")
        .join("tokens.css")
}

fn blessing() -> bool {
    std::env::var("JARROW_BLESS").is_ok_and(|v| v == "1")
}

#[test]
fn site_tokens_css_is_current() -> Result<(), Box<dyn Error>> {
    let path = tokens_css_path();
    let expected = jarrow_report::tokens::css();

    if blessing() {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        fs::write(&path, expected.as_bytes())?;
        eprintln!("blessed {DISPLAY}");
        return Ok(());
    }

    let actual = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) => {
            return Err(format!(
                "cannot read {DISPLAY}: {err}. Generate it with: \
                 JARROW_BLESS=1 cargo test -p jarrow-report --test tokens_current"
            )
            .into());
        }
    };
    // A Windows checkout with core.autocrlf=true turns LF into CRLF; the content is what counts.
    let actual = actual.replace("\r\n", "\n");
    assert!(
        actual == expected,
        "{DISPLAY} is out of date with crates/report/src/tokens.rs. Regenerate it with: \
         JARROW_BLESS=1 cargo test -p jarrow-report --test tokens_current"
    );
    Ok(())
}
