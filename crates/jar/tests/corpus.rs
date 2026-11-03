//! Every jar in a local directory: read, nested read, manifest, signature strip, and a
//! deterministic write that reads back to the same entries. No jar is ever committed;
//! point the test at a directory of mods you have:
//!
//! ```text
//! JARROW_JAR_CORPUS=/path/to/mods cargo test -p jarrow-jar --test corpus -- --ignored --nocapture
//! ```
//!
//! Ignored by default because it needs that directory. No network.

use std::path::PathBuf;

use jarrow_jar::{Jar, NestedJar, ReadOptions, WriteOptions};

fn count(nested: &[NestedJar]) -> usize {
    nested.iter().map(NestedJar::count).sum()
}

#[test]
#[ignore = "needs JARROW_JAR_CORPUS; see the module docs for the command"]
fn every_jar_in_the_corpus() {
    let dir = PathBuf::from(
        std::env::var_os("JARROW_JAR_CORPUS").expect("JARROW_JAR_CORPUS names a directory"),
    );
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("jar")))
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "no .jar files in {}", dir.display());

    let options = ReadOptions::default();
    for path in paths {
        let label = path.file_name().unwrap().to_string_lossy().into_owned();
        let bytes = std::fs::read(&path).unwrap();
        let jar = Jar::read(&bytes).unwrap_or_else(|e| panic!("{label}: {e}"));
        let nested = jar
            .read_nested(&options)
            .unwrap_or_else(|e| panic!("{label}: {e}"));
        let manifest = jar.manifest().unwrap_or_else(|e| panic!("{label}: {e}"));

        let mut stripped = jar.clone();
        let report = stripped
            .strip_signatures()
            .unwrap_or_else(|e| panic!("{label}: {e}"));

        let written = jar.write(&WriteOptions::default()).unwrap();
        assert_eq!(
            written,
            jar.write(&WriteOptions::default()).unwrap(),
            "{label}"
        );
        let back = Jar::read(&written).unwrap();
        assert_eq!(back.entries.len(), jar.entries.len(), "{label}");
        for entry in &jar.entries {
            assert_eq!(back.get(&entry.name).unwrap().data, entry.data, "{label}");
        }
        assert_eq!(
            back.write(&WriteOptions::default()).unwrap(),
            written,
            "{label}"
        );

        println!(
            "{label}: {} entries, {} nested jars, manifest {}, signed {}, {} -> {} bytes",
            jar.entries.len(),
            count(&nested),
            if manifest.is_some() { "yes" } else { "no" },
            if report.files.is_empty() { "no" } else { "yes" },
            bytes.len(),
            written.len()
        );
    }
}
