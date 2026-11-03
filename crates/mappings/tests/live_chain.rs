//! The Fabric to NeoForge chain on the real, pinned mapping files.
//!
//! These tests are ignored by default: they need network access and `curl` on `PATH`. They read
//! the URLs and sha1 values from `research-lock.json`, download intermediary's v2 jar and
//! Mojang's server mappings into a fresh directory under the system temp directory (never the
//! repository), check both sha1 values, compose the chain and look up one class. The
//! directory is deleted afterwards. Run them with:
//!
//! ```sh
//! cargo test -p jarrow-mappings --test live_chain -- --ignored --nocapture
//! ```

// Helpers outside #[test] functions unwrap too: a failure there is a failing test.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use jarrow_mappings::{
    GlobalMemberMap, chain, intermediary, is_intermediary_member_name, ns, proguard,
};
use sha1::{Digest, Sha1};

/// A directory under the system temp directory, removed on drop.
struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "jarrow-mappings-live-{label}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        TempDir(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn research_lock() -> serde_json::Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research-lock.json");
    serde_json::from_slice(&fs::read(&path).unwrap()).unwrap()
}

fn sha1_hex(bytes: &[u8]) -> String {
    Sha1::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Downloads `url` to `dest` with curl (HTTPS only) and checks its sha1.
fn download(url: &str, sha1: &str, dest: &Path) -> Vec<u8> {
    assert!(url.starts_with("https://"), "{url} is not HTTPS");
    let status = Command::new("curl")
        .args([
            "--fail",
            "--silent",
            "--show-error",
            "--location",
            "--proto",
            "=https",
            "--retry",
            "3",
            "--output",
        ])
        .arg(dest)
        .arg(url)
        .status()
        .expect("curl must be on PATH for the live mapping tests");
    assert!(status.success(), "curl failed for {url}");
    let bytes = fs::read(dest).unwrap();
    assert_eq!(sha1_hex(&bytes), sha1, "sha1 mismatch for {url}");
    bytes
}

fn u16_at(b: &[u8], at: usize) -> usize {
    usize::from(u16::from_le_bytes([b[at], b[at + 1]]))
}

fn u32_at(b: &[u8], at: usize) -> usize {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]]) as usize
}

/// Reads one entry out of a zip archive (stored or deflated, no zip64): the job of the host,
/// done here in a few lines so the test needs no zip crate.
fn zip_entry(zip: &[u8], wanted: &str) -> Vec<u8> {
    let eocd = (0..=zip.len().saturating_sub(22))
        .rev()
        .find(|&i| zip[i..i + 4] == [0x50, 0x4b, 0x05, 0x06])
        .expect("no end of central directory");
    let count = u16_at(zip, eocd + 10);
    let mut at = u32_at(zip, eocd + 16);
    for _ in 0..count {
        assert_eq!(
            zip[at..at + 4],
            [0x50, 0x4b, 0x01, 0x02],
            "bad central directory"
        );
        let method = u16_at(zip, at + 10);
        let compressed = u32_at(zip, at + 20);
        let name_len = u16_at(zip, at + 28);
        let extra_len = u16_at(zip, at + 30);
        let comment_len = u16_at(zip, at + 32);
        let local = u32_at(zip, at + 42);
        let name = std::str::from_utf8(&zip[at + 46..at + 46 + name_len]).unwrap();
        if name == wanted {
            let data_at = local + 30 + u16_at(zip, local + 26) + u16_at(zip, local + 28);
            let data = &zip[data_at..data_at + compressed];
            return match method {
                0 => data.to_vec(),
                8 => miniz_oxide::inflate::decompress_to_vec(data).unwrap(),
                other => panic!("unsupported zip method {other}"),
            };
        }
        at += 46 + name_len + extra_len + comment_len;
    }
    panic!("{wanted} not found in the archive");
}

fn check_chain(game_version: &str, expected: &str) {
    let lock = research_lock();
    let pins = &lock["game_versions"][game_version];
    let inter = &pins["fabric"]["intermediary"];
    let server = &pins["mojang"]["server_mappings"];
    let field = |v: &serde_json::Value, key: &str| v[key].as_str().unwrap().to_owned();

    let dir = TempDir::new(game_version);
    let jar = download(
        &field(inter, "url"),
        &field(inter, "sha1"),
        &dir.0.join("intermediary-v2.jar"),
    );
    let txt = download(
        &field(server, "url"),
        &field(server, "sha1"),
        &dir.0.join("server.txt"),
    );

    let started = Instant::now();
    let intermediary = intermediary::load(&zip_entry(&jar, intermediary::JAR_ENTRY)).unwrap();
    let loaded = started.elapsed();
    let mojang = proguard::parse_bytes(&txt, ns::MOJANG, ns::OBF).unwrap();
    let parsed = started.elapsed();
    let set = chain(ns::INTERMEDIARY, ns::MOJANG, &[&intermediary, &mojang]).unwrap();
    let chained = started.elapsed();
    let mapper = set.mapper(ns::INTERMEDIARY, ns::MOJANG).unwrap();
    assert_eq!(mapper.map_class("net/minecraft/class_2960"), Some(expected));

    // The writers reproduce the real files' content (in memory only; nothing is saved).
    let tiny = jarrow_mappings::tiny2::write(&intermediary).unwrap();
    assert_eq!(jarrow_mappings::tiny2::parse(&tiny).unwrap(), intermediary);
    let txt = proguard::write_mojang(&mojang).unwrap();
    assert_eq!(proguard::parse_mojang(&txt).unwrap(), mojang);

    let global = GlobalMemberMap::build_filtered(
        &set,
        ns::INTERMEDIARY,
        ns::MOJANG,
        is_intermediary_member_name,
    )
    .unwrap();
    println!(
        "{game_version}: intermediary {:?}, mojang server {:?}, chain {:?}",
        intermediary.stats(),
        mojang.stats(),
        set.stats()
    );
    println!(
        "{game_version}: load {loaded:?}, + proguard {parsed:?}, + chain {chained:?}; global map {} fields, {} methods, {} conflicts",
        global.field_count(),
        global.method_count(),
        global.conflicts().len()
    );
    for conflict in global.conflicts().iter().take(5) {
        println!("  conflict: {conflict:?}");
    }
}

fn unnamed_classes(set: &jarrow_mappings::MappingSet) -> usize {
    set.classes().iter().filter(|c| c.name(1).is_none()).count()
}

/// Mojang's server file covers only the dedicated server's classes; merging the client file
/// (the same obfuscated names) gives the remapper the client-only classes too.
fn check_client_and_server(game_version: &str) {
    let lock = research_lock();
    let pins = &lock["game_versions"][game_version];
    let field = |v: &serde_json::Value, key: &str| v[key].as_str().unwrap().to_owned();
    let inter = &pins["fabric"]["intermediary"];
    let server = &pins["mojang"]["server_mappings"];
    let client = &pins["mojang"]["client_mappings"];

    let dir = TempDir::new(&format!("{game_version}-merge"));
    let jar = download(
        &field(inter, "url"),
        &field(inter, "sha1"),
        &dir.0.join("intermediary-v2.jar"),
    );
    let server = download(
        &field(server, "url"),
        &field(server, "sha1"),
        &dir.0.join("server.txt"),
    );
    let client = download(
        &field(client, "url"),
        &field(client, "sha1"),
        &dir.0.join("client.txt"),
    );

    let intermediary = intermediary::load(&zip_entry(&jar, intermediary::JAR_ENTRY)).unwrap();
    let server = proguard::parse_bytes(&server, ns::MOJANG, ns::OBF).unwrap();
    let mut mojang = proguard::parse_bytes(&client, ns::MOJANG, ns::OBF).unwrap();
    let server_only = chain(ns::INTERMEDIARY, ns::MOJANG, &[&intermediary, &server]).unwrap();
    mojang.merge(&server).unwrap();
    let merged = chain(ns::INTERMEDIARY, ns::MOJANG, &[&intermediary, &mojang]).unwrap();
    println!(
        "{game_version}: intermediary classes without a Mojang name: {} with the server file, {} with client and server merged",
        unnamed_classes(&server_only),
        unnamed_classes(&merged)
    );
    assert!(unnamed_classes(&server_only) > 0);
    assert_eq!(unnamed_classes(&merged), 0);
}

/// `cargo test -p jarrow-mappings --test live_chain -- --ignored --nocapture`
#[test]
#[ignore = "network: downloads the pinned 1.21.1 intermediary jar and both Mojang mapping files"]
fn client_and_server_merge_1_21_1() {
    check_client_and_server("1.21.1");
}

/// `cargo test -p jarrow-mappings --test live_chain -- --ignored --nocapture`
#[test]
#[ignore = "network: downloads the pinned 1.21.1 intermediary jar and Mojang server mappings"]
fn intermediary_to_mojang_1_21_1() {
    check_chain("1.21.1", "net/minecraft/resources/ResourceLocation");
}

/// `cargo test -p jarrow-mappings --test live_chain -- --ignored --nocapture`
#[test]
#[ignore = "network: downloads the pinned 1.21.11 intermediary jar and Mojang server mappings"]
fn intermediary_to_mojang_1_21_11() {
    check_chain("1.21.11", "net/minecraft/resources/Identifier");
}
