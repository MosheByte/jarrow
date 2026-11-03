//! Interop with the JDK's own zip code, through the `jar` tool.
//!
//! Ignored by default because it needs a JDK. Run it with:
//!
//! ```text
//! cargo test -p jarrow-jar --test jdk_interop -- --ignored
//! ```
//!
//! It uses `$JAVA_HOME/bin/jar` when `JAVA_HOME` is set, otherwise `jar` on `PATH`
//! (JDK 21, as pinned in research-lock.json). No network.

// Helpers outside #[test] fns unwrap like the tests do (clippy.toml allows it only there).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};
use std::process::Command;

use jarrow_jar::{Entry, Jar, MANIFEST_PATH, Method, WriteOptions};

fn jar_tool() -> PathBuf {
    match std::env::var_os("JAVA_HOME") {
        Some(home) => {
            let exe = if cfg!(windows) { "jar.exe" } else { "jar" };
            Path::new(&home).join("bin").join(exe)
        }
        None => PathBuf::from("jar"),
    }
}

fn run(args: &[&str], dir: &Path) -> String {
    let output = Command::new(jar_tool())
        // print entry names as UTF-8 whatever the console code page is (JDK 19+)
        .arg("-J-Dstdout.encoding=UTF-8")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("the jar tool runs (set JAVA_HOME or put a JDK on PATH)");
    assert!(
        output.status.success(),
        "jar {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("jarrow-jar-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
#[ignore = "needs a JDK; see the module docs for the command"]
fn the_jdk_reads_what_we_write() {
    let dir = scratch("write");
    let mut stored = Entry::file("net/example/Stored.bin", (0..=255u8).collect::<Vec<u8>>());
    stored.method = Method::Stored;
    let nested = Jar::from_entries(vec![Entry::file("inner.txt", b"inner".to_vec())])
        .write(&WriteOptions::default())
        .unwrap();
    let jar = Jar::from_entries(vec![
        Entry::directory("META-INF"),
        Entry::file(
            MANIFEST_PATH,
            b"Manifest-Version: 1.0\r\nCreated-By: jarrow-jar test\r\n\r\n".to_vec(),
        ),
        Entry::directory("net/example"),
        Entry::file("net/example/Text.txt", "hello\n".repeat(200).into_bytes()),
        stored,
        Entry::file("net/example/\u{e9}t\u{e9}.txt", b"utf-8 name".to_vec()),
        Entry::file("empty.txt", Vec::new()),
        Entry::file("META-INF/jars/nested.jar", nested),
    ]);
    let bytes = jar.write(&WriteOptions::default()).unwrap();
    std::fs::write(dir.join("ours.jar"), &bytes).unwrap();

    let listing = run(&["--list", "--file", "ours.jar"], &dir);
    let listed: Vec<&str> = listing.lines().collect();
    let mut expected: Vec<&str> = vec![MANIFEST_PATH];
    expected.extend(jar.names().filter(|n| *n != MANIFEST_PATH));
    assert_eq!(listed, expected);

    let out = dir.join("x");
    std::fs::create_dir_all(&out).unwrap();
    run(&["--extract", "--file", "../ours.jar"], &out);
    for entry in jar.entries.iter().filter(|e| !e.is_dir()) {
        let on_disk = std::fs::read(out.join(&entry.name)).unwrap();
        assert_eq!(on_disk, entry.data, "{}", entry.name);
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
#[ignore = "needs a JDK; see the module docs for the command"]
fn we_read_what_the_jdk_writes() {
    let dir = scratch("read");
    let src = dir.join("src");
    std::fs::create_dir_all(src.join("net/example")).unwrap();
    let text = "repetitive text ".repeat(300);
    std::fs::write(src.join("net/example/Text.txt"), &text).unwrap();
    std::fs::write(src.join("net/example/B.bin"), [0u8, 1, 2, 3, 255]).unwrap();
    std::fs::write(
        dir.join("manifest.txt"),
        format!(
            "Main-Class: net.example.Main\nClass-Path: {}\n",
            "lib/x.jar ".repeat(20)
        ),
    )
    .unwrap();

    for (file, extra) in [
        ("deflated.jar", None),
        ("stored.jar", Some("--no-compress")),
    ] {
        let mut args = vec!["--create", "--file", file, "--manifest", "manifest.txt"];
        args.extend(extra);
        args.extend(["-C", "src", "."]);
        run(&args, &dir);

        let bytes = std::fs::read(dir.join(file)).unwrap();
        let jar = Jar::read(&bytes).unwrap();
        assert_eq!(
            jar.get("net/example/Text.txt").unwrap().data,
            text.as_bytes(),
            "{file}"
        );
        assert_eq!(
            jar.get("net/example/B.bin").unwrap().data,
            [0u8, 1, 2, 3, 255],
            "{file}"
        );
        let manifest = jar.manifest().unwrap().unwrap();
        assert_eq!(manifest.get("Main-Class"), Some("net.example.Main"));
        // the JDK wrapped this 210-byte header over several 72-byte lines
        assert_eq!(
            manifest.get("Class-Path"),
            Some("lib/x.jar ".repeat(20).as_str()),
            "{file}: continuation lines joined"
        );
        assert!(manifest.get("Created-By").is_some(), "{file}");
        // and our writer turns it into a deterministic archive the JDK can read back
        let ours = jar.write(&WriteOptions::default()).unwrap();
        assert_eq!(Jar::read(&ours).unwrap().names().count(), jar.entries.len());
    }
    std::fs::remove_dir_all(&dir).unwrap();
}
