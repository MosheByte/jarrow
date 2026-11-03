//! Fuzz target: arbitrary bytes through the JAR reader, the nested-jar reader, the manifest
//! parser, the class-file parser for every `.class` entry, and the deterministic writer.
//!
//! Beyond "no panic", it checks two promises: a class entry that parses writes back to its
//! own bytes, and a jar that was read and written reads back with the same entries, with
//! the writer giving the same bytes twice.

#![no_main]

use jarrow_classfile::ClassFile;
use jarrow_jar::{Entry, Jar, ReadOptions, WriteOptions};
use libfuzzer_sys::fuzz_target;

/// Tighter than the defaults, so a small zip bomb cannot spend the fuzzer's time budget on
/// inflating hundreds of megabytes.
fn options() -> ReadOptions {
    let mut options = ReadOptions::default();
    options.max_entries = 4096;
    options.max_entry_size = 4 << 20;
    options.max_total_size = 8 << 20;
    options.max_nesting_depth = 3;
    options
}

fn contents(jar: &Jar) -> Vec<(String, bool, Vec<u8>)> {
    let mut list: Vec<_> = jar
        .entries
        .iter()
        .map(|e: &Entry| (e.name.clone(), e.is_dir(), e.data.clone()))
        .collect();
    list.sort();
    list
}

fuzz_target!(|data: &[u8]| {
    let options = options();
    let Ok(jar) = Jar::read_with(data, &options) else {
        return;
    };
    let _ = jar.read_nested(&options);
    let _ = jar.manifest();
    for entry in &jar.entries {
        if entry.name.ends_with(".class")
            && let Ok(class) = ClassFile::parse(&entry.data)
        {
            let written = class.to_bytes().expect("a parsed class always writes");
            assert_eq!(
                written, entry.data,
                "{}: parse -> write must reproduce the entry",
                entry.name
            );
        }
    }
    let write = WriteOptions::default();
    if let Ok(written) = jar.write(&write) {
        assert_eq!(
            jar.write(&write).ok().as_deref(),
            Some(written.as_slice()),
            "write is deterministic"
        );
        let back = Jar::read_with(&written, &options).expect("a jar Jarrow wrote reads back");
        assert_eq!(
            contents(&back),
            contents(&jar),
            "read -> write -> read keeps every entry"
        );
    }
});
