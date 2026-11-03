//! Nested jars, built in the test: listing, recursive reading, depth and budget limits.

// Helpers outside #[test] fns unwrap like the tests do (clippy.toml allows it only there).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use jarrow_jar::{Entry, Error, Jar, Limit, NestedDir, ReadOptions, WriteOptions};

fn jar_bytes(entries: Vec<Entry>) -> Vec<u8> {
    Jar::from_entries(entries)
        .write(&WriteOptions::default())
        .unwrap()
}

fn leaf(name: &str) -> Vec<u8> {
    jar_bytes(vec![Entry::file(
        format!("{name}/Marker.class"),
        name.as_bytes().to_vec(),
    )])
}

fn outer() -> Jar {
    let c = leaf("c");
    let b = jar_bytes(vec![
        Entry::file("fabric.mod.json", b"{}".to_vec()),
        Entry::file("META-INF/jars/c.jar", c),
    ]);
    Jar::from_entries(vec![
        Entry::file("fabric.mod.json", b"{}".to_vec()),
        Entry::directory("META-INF/jars"),
        Entry::file("META-INF/jars/a.jar", leaf("a")),
        Entry::file("META-INF/jarjar/metadata.json", b"{\"jars\":[]}".to_vec()),
        Entry::file("META-INF/jarjar/b.jar", b),
        Entry::file("libs/other.jar", leaf("other")),
    ])
}

#[test]
fn lists_nested_jars_in_both_directories() {
    let jar = outer();
    let listed: Vec<(NestedDir, &str)> = jar
        .nested_jar_entries()
        .into_iter()
        .map(|(dir, e)| (dir, e.name.as_str()))
        .collect();
    assert_eq!(
        listed,
        [
            (NestedDir::Jars, "META-INF/jars/a.jar"),
            (NestedDir::JarJar, "META-INF/jarjar/b.jar"),
        ]
    );
    assert_eq!(NestedDir::Jars.prefix(), "META-INF/jars/");
    assert_eq!(NestedDir::JarJar.prefix(), "META-INF/jarjar/");
}

#[test]
fn reads_nested_jars_recursively() {
    // through a real write and read, so the nested bytes are what an archive holds
    let jar = Jar::read(&outer().write(&WriteOptions::default()).unwrap()).unwrap();
    let nested = jar.read_nested(&ReadOptions::default()).unwrap();
    assert_eq!(nested.len(), 2);
    assert_eq!(nested[0].path, "META-INF/jars/a.jar");
    assert_eq!(nested[0].dir, NestedDir::Jars);
    assert_eq!(
        nested[0].jar.get("a/Marker.class").unwrap().data,
        b"a".to_vec()
    );
    assert!(nested[0].nested.is_empty());
    assert_eq!(nested[1].path, "META-INF/jarjar/b.jar");
    assert_eq!(nested[1].dir, NestedDir::JarJar);
    assert_eq!(nested[1].nested.len(), 1);
    assert_eq!(nested[1].nested[0].path, "META-INF/jars/c.jar");
    assert_eq!(
        nested[1].nested[0].jar.get("c/Marker.class").unwrap().data,
        b"c".to_vec()
    );
    assert_eq!(nested.iter().map(|n| n.count()).sum::<usize>(), 3);
}

#[test]
fn reads_a_jar_at_any_path() {
    let jar = outer();
    let other = jar
        .read_entry_as_jar("libs/other.jar", &ReadOptions::default())
        .unwrap();
    assert!(other.get("other/Marker.class").is_some());
    assert_eq!(
        jar.read_entry_as_jar("missing.jar", &ReadOptions::default()),
        Err(Error::NoSuchEntry {
            name: "missing.jar".into()
        })
    );
    assert!(matches!(
        jar.read_entry_as_jar("fabric.mod.json", &ReadOptions::default()),
        Err(Error::Nested { .. })
    ));
}

#[test]
fn a_broken_nested_jar_names_its_path() {
    let inner = jar_bytes(vec![Entry::file(
        "META-INF/jars/bad.jar",
        b"not a zip".to_vec(),
    )]);
    let jar = Jar::from_entries(vec![Entry::file("META-INF/jarjar/mid.jar", inner)]);
    assert_eq!(
        jar.read_nested(&ReadOptions::default()),
        Err(Error::Nested {
            path: "META-INF/jarjar/mid.jar".into(),
            source: Box::new(Error::Nested {
                path: "META-INF/jars/bad.jar".into(),
                source: Box::new(Error::NotAZip),
            }),
        })
    );
}

/// A chain of `depth` jars, each holding the next under META-INF/jars/.
fn chain(depth: usize) -> Jar {
    let mut bytes = leaf("bottom");
    for _ in 1..depth {
        bytes = jar_bytes(vec![Entry::file("META-INF/jars/next.jar", bytes)]);
    }
    Jar::from_entries(vec![Entry::file("META-INF/jars/next.jar", bytes)])
}

fn innermost(error: &Error) -> &Error {
    match error {
        Error::Nested { source, .. } => innermost(source),
        other => other,
    }
}

#[test]
fn nesting_depth_is_limited() {
    let jar = chain(4);
    let mut options = ReadOptions::default();
    options.max_nesting_depth = 4;
    let tree = jar.read_nested(&options).unwrap();
    assert_eq!(tree[0].count(), 4);

    options.max_nesting_depth = 3;
    let error = jar.read_nested(&options).unwrap_err();
    assert_eq!(
        innermost(&error),
        &Error::NestingTooDeep {
            path: "META-INF/jars/next.jar".into(),
            max: 3
        }
    );

    options.max_nesting_depth = 0;
    assert!(matches!(
        jar.read_nested(&options),
        Err(Error::NestingTooDeep { max: 0, .. })
    ));
}

#[test]
fn nesting_depth_has_a_hard_cap() {
    let jar = chain(jarrow_jar::HARD_MAX_NESTING_DEPTH as usize + 1);
    let mut options = ReadOptions::default();
    options.max_nesting_depth = u32::MAX;
    let error = jar.read_nested(&options).unwrap_err();
    assert!(matches!(
        innermost(&error),
        Error::NestingTooDeep { max, .. } if *max == jarrow_jar::HARD_MAX_NESTING_DEPTH
    ));
}

#[test]
fn limits_apply_to_the_whole_nested_tree() {
    // 30 nested jars, each a few hundred bytes but 64 KiB uncompressed: a nested bomb
    let inner = jar_bytes(vec![Entry::file("zeros.bin", vec![0u8; 64 << 10])]);
    let entries: Vec<Entry> = (0..30)
        .map(|i| Entry::file(format!("META-INF/jars/copy{i:02}.jar"), inner.clone()))
        .collect();
    let jar = Jar::from_entries(entries);
    assert!(jar.read_nested(&ReadOptions::default()).is_ok());

    let mut options = ReadOptions::default();
    options.max_total_size = 1 << 20; // each jar fits, all 30 together do not
    let error = jar.read_nested(&options).unwrap_err();
    assert!(matches!(
        innermost(&error),
        Error::LimitExceeded {
            limit: Limit::TotalSize,
            ..
        }
    ));

    let mut options = ReadOptions::default();
    options.max_entries = 29;
    let error = jar.read_nested(&options).unwrap_err();
    assert!(matches!(
        innermost(&error),
        Error::LimitExceeded {
            limit: Limit::Entries,
            ..
        }
    ));
}
