//! Live checks against the real publishers. They need the network (and, for provisioning, a
//! JDK in `JAVA_HOME` and a few hundred megabytes), so they are ignored by default. None of
//! them starts a Minecraft server: that needs the owner's EULA acceptance, and the harness
//! workflow does it with the probe mods (docs/harness.md).

use std::error::Error;
use std::path::PathBuf;

use jarrow_harness::Loader;
use jarrow_harness::fetch::{HttpFetcher, download_verified};
use jarrow_harness::lock::Lock;
use jarrow_harness::process::{SystemHost, java_in};
use jarrow_harness::provision::{ProvisionContext, provision};

fn cache() -> PathBuf {
    std::env::var_os("JARROW_HARNESS_CACHE")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/jarrow-harness-cache")
        })
}

/// The meta.fabricmc.net server launcher still hashes to the observed SHA-256 in
/// research-lock.json, for every pinned game version (about 180 KB each). Run with:
/// `cargo test -p jarrow-harness --test live -- --ignored fabric_launchers_match_the_lock`
#[test]
#[ignore = "network: downloads the Fabric server launchers"]
fn fabric_launchers_match_the_lock() -> Result<(), Box<dyn Error>> {
    let lock = Lock::embedded()?;
    let fetcher = HttpFetcher::new()?;
    for game in lock.game_versions() {
        let launcher = lock.fabric(&game)?.server_launcher;
        let dest = cache().join("live").join(&game).join(&launcher.file_name);
        let file = download_verified(&fetcher, &launcher, &dest)?;
        println!("{game}: {} ok ({} bytes)", launcher.url, file.size);
    }
    Ok(())
}

/// Provisions Fabric and NeoForge for every pinned game version into the cache, with every
/// publisher hash checked. Needs `JAVA_HOME` (JDK 21) and downloads several hundred MB. Run with:
/// `cargo test -p jarrow-harness --test live -- --ignored provision_every_pinned_server`
#[test]
#[ignore = "network and JDK: downloads and installs Fabric and NeoForge servers"]
fn provision_every_pinned_server() -> Result<(), Box<dyn Error>> {
    let lock = Lock::embedded()?;
    let fetcher = HttpFetcher::new()?;
    let java = std::env::var_os("JAVA_HOME")
        .map(|home| java_in(&PathBuf::from(home)))
        .ok_or("JAVA_HOME is not set")?;
    let cache = cache();
    for game in lock.game_versions() {
        for loader in [Loader::Fabric, Loader::NeoForge] {
            let ctx = ProvisionContext {
                lock: &lock,
                game_version: &game,
                cache: &cache,
                fetcher: &fetcher,
                host: &SystemHost,
                java: Some(&java),
            };
            let installed = provision(loader, &ctx)?;
            println!(
                "{loader} {} for {game}: {} ({})",
                installed.loader_version,
                installed.dir.display(),
                installed.method
            );
        }
    }
    Ok(())
}
