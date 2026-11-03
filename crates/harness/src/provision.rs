//! Installing a server per loader and game version into the cache.
//!
//! Each loader has a [`LoaderProfile`]. Fabric and NeoForge are built:
//!
//! - **Fabric**: the meta.fabricmc.net server launcher, checked against its observed SHA-256,
//!   with the Mojang server jar pre-placed (SHA-1 checked) and passed through
//!   `-Dfabric.installer.server.gameJar`. When the launcher's hash does not match, the profile
//!   falls back to the hash-pinned Fabric installer's server mode, as the lock prescribes.
//!   Fabric API (SHA-1 and SHA-512 checked) goes into every Fabric server's `mods/`.
//! - **NeoForge**: the hash-pinned installer with `--installServer`, then a launch through the
//!   `unix_args.txt` or `win_args.txt` argument file its `run.sh` and `run.bat` name.
//!
//! Forge and Quilt will be driven by loader profiles (`loaders/*.toml`); until then their
//! profiles return [`HarnessError::NotBuiltYet`].
//!
//! Layout under the cache directory:
//!
//! ```text
//! downloads/<algorithm>-<hash>/<file>        verified downloads, keyed by their pin
//! servers/<loader>-<version>-mc<game>/       an installed server, used as a template
//!     jarrow-install.json                    written last: how to launch it
//! work/<loader>-<game>/                      the default per-run directory
//! ```

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::error::HarnessError;
use crate::fetch::{
    Artifact, DownloadError, Fetcher, VerifiedFile, digest_file, download_verified,
};
use crate::loader::Loader;
use crate::lock::Lock;
use crate::process::{CommandSpec, ProcessHost};

/// The install record written into every installed server.
pub const INSTALL_RECORD_FILE: &str = "jarrow-install.json";

/// The `record` value of an install record.
pub const INSTALL_RECORD: &str = "jarrow-harness-install";

/// How long an installer may run.
pub const INSTALLER_TIMEOUT: Duration = Duration::from_secs(20 * 60);

/// How to start an installed server, relative to its directory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Launch {
    /// `java <jvm args> -jar <jar> nogui`.
    Jar {
        /// The jar, relative to the server directory.
        jar: String,
        /// Extra JVM arguments.
        jvm_args: Vec<String>,
    },
    /// `java @<argument file> nogui`, with the file for the current operating system.
    ArgFile {
        /// The Unix argument file, relative to the server directory.
        unix: Option<String>,
        /// The Windows argument file, relative to the server directory.
        windows: Option<String>,
    },
}

/// A server installed in the cache.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledServer {
    /// Always [`INSTALL_RECORD`].
    pub record: String,
    /// Install record format version (1).
    pub schema: u32,
    /// Loader.
    pub loader: Loader,
    /// Loader version.
    pub loader_version: String,
    /// Game version.
    pub game_version: String,
    /// How it was installed (`fabric-server-launcher`, `fabric-installer`,
    /// `neoforge-installer`).
    pub method: String,
    /// How to start it.
    pub launch: Launch,
    /// Mods every server of this loader loads besides the probe (Fabric API).
    pub loader_mods: Vec<VerifiedFile>,
    /// Every pinned download behind this install.
    pub files: Vec<VerifiedFile>,
    /// Anything worth knowing (a fallback that was taken).
    pub notes: Vec<String>,
    /// The server directory (not serialised; it is where the record was read from).
    #[serde(skip)]
    pub dir: PathBuf,
}

impl InstalledServer {
    /// The launch target that must exist for the install to be usable.
    fn launch_target(&self) -> Option<PathBuf> {
        match &self.launch {
            Launch::Jar { jar, .. } => Some(self.dir.join(jar)),
            Launch::ArgFile { unix, windows } => {
                let file = if cfg!(windows) { windows } else { unix };
                file.as_ref().map(|f| self.dir.join(f))
            }
        }
    }
}

/// What a profile needs to install a server.
pub struct ProvisionContext<'a> {
    /// The pins.
    pub lock: &'a Lock,
    /// Game version.
    pub game_version: &'a str,
    /// Cache directory.
    pub cache: &'a Path,
    /// Network.
    pub fetcher: &'a dyn Fetcher,
    /// Processes (installers).
    pub host: &'a dyn ProcessHost,
    /// `java`, when one is available. Installers need it.
    pub java: Option<&'a Path>,
}

impl ProvisionContext<'_> {
    fn download(&self, artifact: &Artifact) -> Result<VerifiedFile, DownloadError> {
        let dest = self
            .cache
            .join("downloads")
            .join(artifact.cache_key())
            .join(&artifact.file_name);
        download_verified(self.fetcher, artifact, &dest)
    }

    fn java(&self, why: &str) -> Result<&Path, HarnessError> {
        self.java.ok_or_else(|| {
            HarnessError::Java(format!(
                "{why} needs Java: set JAVA_HOME to a JDK 21 installation"
            ))
        })
    }
}

/// Installs one loader's server.
pub trait LoaderProfile {
    /// The loader.
    fn loader(&self) -> Loader;
    /// The pinned loader version for the game version.
    fn loader_version(&self, ctx: &ProvisionContext<'_>) -> Result<String, HarnessError>;
    /// Installs into the empty directory `dir`.
    fn install(
        &self,
        ctx: &ProvisionContext<'_>,
        dir: &Path,
    ) -> Result<InstalledServer, HarnessError>;
}

/// The profile for `loader`.
pub fn profile(loader: Loader) -> Box<dyn LoaderProfile> {
    match loader {
        Loader::Fabric => Box::new(FabricProfile),
        Loader::NeoForge => Box::new(NeoForgeProfile),
        Loader::Forge | Loader::Quilt => Box::new(NotBuiltYet(loader)),
    }
}

/// The server directory in the cache for a loader version and game version.
pub fn server_dir(
    cache: &Path,
    loader: Loader,
    loader_version: &str,
    game_version: &str,
) -> PathBuf {
    let safe = |s: &str| -> String {
        s.chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') {
                    c
                } else {
                    '_'
                }
            })
            .collect()
    };
    cache.join("servers").join(format!(
        "{loader}-{}-mc{}",
        safe(loader_version),
        safe(game_version)
    ))
}

/// Installs the server for `loader`, or reuses the one already in the cache.
pub fn provision(
    loader: Loader,
    ctx: &ProvisionContext<'_>,
) -> Result<InstalledServer, HarnessError> {
    let profile = profile(loader);
    let loader_version = profile.loader_version(ctx)?;
    let dir = server_dir(ctx.cache, loader, &loader_version, ctx.game_version);
    if let Some(installed) = reusable(&dir, loader, &loader_version, ctx.game_version) {
        return Ok(installed);
    }
    if dir.exists() {
        fs::remove_dir_all(&dir)
            .map_err(|e| HarnessError::io_at("remove the incomplete install", &dir, e))?;
    }
    fs::create_dir_all(&dir).map_err(|e| HarnessError::io_at("create", &dir, e))?;
    let mut installed = profile.install(ctx, &dir)?;
    installed.dir = dir.clone();
    let text = serde_json::to_string_pretty(&installed)
        .map_err(|e| HarnessError::Install(format!("could not write the install record: {e}")))?;
    let record = dir.join(INSTALL_RECORD_FILE);
    fs::write(&record, text + "\n").map_err(|e| HarnessError::io_at("write", &record, e))?;
    Ok(installed)
}

fn reusable(
    dir: &Path,
    loader: Loader,
    loader_version: &str,
    game_version: &str,
) -> Option<InstalledServer> {
    let text = fs::read_to_string(dir.join(INSTALL_RECORD_FILE)).ok()?;
    let mut installed: InstalledServer = serde_json::from_str(&text).ok()?;
    installed.dir = dir.to_path_buf();
    let same = installed.record == INSTALL_RECORD
        && installed.loader == loader
        && installed.loader_version == loader_version
        && installed.game_version == game_version;
    let launchable = installed.launch_target().is_some_and(|t| t.is_file());
    // Loader mods live in the download cache; re-hash them so a damaged cache is noticed.
    let mods_intact = installed
        .loader_mods
        .iter()
        .all(|m| digest_file(&m.path).is_ok_and(|d| d.sha256 == m.sha256));
    (same && launchable && mods_intact).then_some(installed)
}

/// Hard-links `src` to `dst`, or copies it when the file system cannot link.
pub fn link_or_copy(src: &Path, dst: &Path) -> Result<(), HarnessError> {
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent).map_err(|e| HarnessError::io_at("create", parent, e))?;
    }
    if dst.exists() {
        fs::remove_file(dst).map_err(|e| HarnessError::io_at("replace", dst, e))?;
    }
    if fs::hard_link(src, dst).is_ok() {
        return Ok(());
    }
    fs::copy(src, dst)
        .map(|_| ())
        .map_err(|e| HarnessError::io(format!("copy {} to {}", src.display(), dst.display()), e))
}

struct FabricProfile;

impl LoaderProfile for FabricProfile {
    fn loader(&self) -> Loader {
        Loader::Fabric
    }

    fn loader_version(&self, ctx: &ProvisionContext<'_>) -> Result<String, HarnessError> {
        Ok(ctx.lock.fabric(ctx.game_version)?.loader_version)
    }

    fn install(
        &self,
        ctx: &ProvisionContext<'_>,
        dir: &Path,
    ) -> Result<InstalledServer, HarnessError> {
        let pins = ctx.lock.fabric(ctx.game_version)?;
        let server = ctx.download(&ctx.lock.mojang_server_jar(ctx.game_version)?)?;
        let api = ctx.download(&pins.api)?;
        link_or_copy(&server.path, &dir.join("server.jar"))?;
        let mut files = vec![server];
        let mut notes = Vec::new();
        let (method, launch) = match ctx.download(&pins.server_launcher) {
            Ok(launcher) => {
                link_or_copy(&launcher.path, &dir.join("fabric-server-launcher.jar"))?;
                files.push(launcher);
                (
                    "fabric-server-launcher",
                    Launch::Jar {
                        jar: "fabric-server-launcher.jar".to_owned(),
                        jvm_args: vec!["-Dfabric.installer.server.gameJar=server.jar".to_owned()],
                    },
                )
            }
            Err(DownloadError::Mismatch { actual, .. }) => {
                notes.push(format!(
                    "the meta.fabricmc.net server launcher hashed to {actual}, not the observed \
                     sha256 in research-lock.json, so the hash-pinned installer {} was used",
                    pins.installer_version
                ));
                let installer = ctx.download(&pins.installer)?;
                let java = ctx.java("the Fabric installer")?;
                let spec = CommandSpec {
                    program: java.to_path_buf(),
                    args: vec![
                        "-jar".into(),
                        installer.path.clone().into_os_string(),
                        "server".into(),
                        "-dir".into(),
                        dir.as_os_str().to_owned(),
                        "-mcversion".into(),
                        ctx.game_version.into(),
                        "-loader".into(),
                        pins.loader_version.clone().into(),
                    ],
                    cwd: dir.to_path_buf(),
                    log: dir.join("jarrow-install.log"),
                };
                run_installer(ctx, &spec, "the Fabric installer")?;
                let launch_jar = dir.join("fabric-server-launch.jar");
                if !launch_jar.is_file() {
                    return Err(HarnessError::Install(format!(
                        "the Fabric installer wrote no fabric-server-launch.jar; see {}",
                        spec.log.display()
                    )));
                }
                let properties = dir.join("fabric-server-launcher.properties");
                fs::write(&properties, "serverJar=server.jar\n")
                    .map_err(|e| HarnessError::io_at("write", &properties, e))?;
                files.push(installer);
                (
                    "fabric-installer",
                    Launch::Jar {
                        jar: "fabric-server-launch.jar".to_owned(),
                        jvm_args: Vec::new(),
                    },
                )
            }
            Err(other) => return Err(other.into()),
        };
        files.push(api.clone());
        Ok(InstalledServer {
            record: INSTALL_RECORD.to_owned(),
            schema: 1,
            loader: Loader::Fabric,
            loader_version: pins.loader_version,
            game_version: ctx.game_version.to_owned(),
            method: method.to_owned(),
            launch,
            loader_mods: vec![api],
            files,
            notes,
            dir: dir.to_path_buf(),
        })
    }
}

fn run_installer(
    ctx: &ProvisionContext<'_>,
    spec: &CommandSpec,
    what: &str,
) -> Result<(), HarnessError> {
    let outcome = ctx
        .host
        .run(spec, INSTALLER_TIMEOUT)
        .map_err(|e| HarnessError::io(format!("start {what}"), e))?;
    if outcome.timed_out {
        return Err(HarnessError::Install(format!(
            "{what} ran longer than {} minutes and was stopped; see {}",
            INSTALLER_TIMEOUT.as_secs() / 60,
            spec.log.display()
        )));
    }
    if outcome.exit_code != Some(0) {
        return Err(HarnessError::Install(format!(
            "{what} exited with {}; see {}",
            outcome
                .exit_code
                .map_or_else(|| "no exit code".to_owned(), |c| format!("code {c}")),
            spec.log.display()
        )));
    }
    Ok(())
}

struct NeoForgeProfile;

impl LoaderProfile for NeoForgeProfile {
    fn loader(&self) -> Loader {
        Loader::NeoForge
    }

    fn loader_version(&self, ctx: &ProvisionContext<'_>) -> Result<String, HarnessError> {
        Ok(ctx.lock.neoforge(ctx.game_version)?.version)
    }

    fn install(
        &self,
        ctx: &ProvisionContext<'_>,
        dir: &Path,
    ) -> Result<InstalledServer, HarnessError> {
        let pins = ctx.lock.neoforge(ctx.game_version)?;
        let installer = ctx.download(&pins.installer)?;
        let java = ctx.java("the NeoForge installer")?;
        let spec = CommandSpec {
            program: java.to_path_buf(),
            args: vec![
                "-jar".into(),
                installer.path.clone().into_os_string(),
                "--installServer".into(),
            ],
            cwd: dir.to_path_buf(),
            log: dir.join("jarrow-install.log"),
        };
        run_installer(ctx, &spec, "the NeoForge installer")?;
        let unix = find_arg_file(dir, "run.sh", &pins.version, "unix_args.txt");
        let windows = find_arg_file(dir, "run.bat", &pins.version, "win_args.txt");
        let wanted = if cfg!(windows) { &windows } else { &unix };
        if wanted.is_none() {
            return Err(HarnessError::Install(format!(
                "the NeoForge installer finished but wrote no {} argument file; see {}",
                if cfg!(windows) {
                    "win_args.txt"
                } else {
                    "unix_args.txt"
                },
                spec.log.display()
            )));
        }
        Ok(InstalledServer {
            record: INSTALL_RECORD.to_owned(),
            schema: 1,
            loader: Loader::NeoForge,
            loader_version: pins.version,
            game_version: ctx.game_version.to_owned(),
            method: "neoforge-installer".to_owned(),
            launch: Launch::ArgFile { unix, windows },
            loader_mods: Vec::new(),
            files: vec![installer],
            notes: Vec::new(),
            dir: dir.to_path_buf(),
        })
    }
}

/// The argument file a NeoForge start script names (`@libraries/.../unix_args.txt`), or the
/// conventional path when the script names none. Only files that exist are returned.
pub fn find_arg_file(
    dir: &Path,
    script: &str,
    version: &str,
    conventional: &str,
) -> Option<String> {
    let from_script = fs::read_to_string(dir.join(script)).ok().and_then(|text| {
        text.split_whitespace()
            .map(|token| token.trim_matches(|c| c == '"' || c == '\''))
            .filter_map(|token| token.strip_prefix('@'))
            .find(|path| path.ends_with("args.txt") && !path.contains("user_jvm_args"))
            .map(|path| path.replace('\\', "/"))
    });
    let fallback = format!("libraries/net/neoforged/neoforge/{version}/{conventional}");
    from_script
        .into_iter()
        .chain(std::iter::once(fallback))
        .find(|path| !path.contains("..") && dir.join(path).is_file())
}

struct NotBuiltYet(Loader);

impl LoaderProfile for NotBuiltYet {
    fn loader(&self) -> Loader {
        self.0
    }

    fn loader_version(&self, _: &ProvisionContext<'_>) -> Result<String, HarnessError> {
        Err(HarnessError::NotBuiltYet { loader: self.0 })
    }

    fn install(&self, _: &ProvisionContext<'_>, _: &Path) -> Result<InstalledServer, HarnessError> {
        Err(HarnessError::NotBuiltYet { loader: self.0 })
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::io;

    use crate::fetch::tests::MemoryFetcher;
    use crate::fetch::{HashAlgorithm, sha256_hex};
    use crate::process::{CommandOutcome, ServerProcess};
    use crate::testutil::TempDir;

    /// A host that records commands and imitates the installers.
    #[derive(Default)]
    pub(crate) struct InstallerHost {
        pub(crate) runs: RefCell<Vec<CommandSpec>>,
        pub(crate) exit_code: Option<i32>,
    }

    impl ProcessHost for InstallerHost {
        fn run(&self, spec: &CommandSpec, _: Duration) -> io::Result<CommandOutcome> {
            self.runs.borrow_mut().push(spec.clone());
            let args = spec.args_lossy();
            if args.iter().any(|a| a == "--installServer") {
                let lib = spec.cwd.join("libraries/net/neoforged/neoforge/9.9.9");
                fs::create_dir_all(&lib)?;
                fs::write(lib.join("unix_args.txt"), "-p libraries/x.jar\n")?;
                fs::write(lib.join("win_args.txt"), "-p libraries/x.jar\n")?;
                fs::write(
                    spec.cwd.join("run.sh"),
                    "#!/usr/bin/env sh\njava @user_jvm_args.txt @libraries/net/neoforged/neoforge/9.9.9/unix_args.txt \"$@\"\n",
                )?;
                fs::write(
                    spec.cwd.join("run.bat"),
                    "java @user_jvm_args.txt @libraries/net/neoforged/neoforge/9.9.9/win_args.txt %*\r\n",
                )?;
            } else if args.iter().any(|a| a == "server") {
                fs::write(spec.cwd.join("fabric-server-launch.jar"), b"launch")?;
            }
            Ok(CommandOutcome {
                exit_code: self.exit_code.or(Some(0)),
                timed_out: false,
            })
        }

        fn spawn(&self, _: &CommandSpec) -> io::Result<Box<dyn ServerProcess>> {
            Err(io::Error::other("installer host starts no servers"))
        }
    }

    fn sha(algorithm: HashAlgorithm, bytes: &[u8]) -> String {
        use sha1::Sha1;
        use sha2::{Digest, Sha256, Sha512};
        let digest: Vec<u8> = match algorithm {
            HashAlgorithm::Sha1 => Sha1::digest(bytes).to_vec(),
            HashAlgorithm::Sha256 => Sha256::digest(bytes).to_vec(),
            HashAlgorithm::Sha512 => Sha512::digest(bytes).to_vec(),
        };
        digest.iter().map(|b| format!("{b:02x}")).collect()
    }

    /// A lock for game version 9.9 whose pins match the bytes `fetcher` serves.
    pub(crate) fn fake_lock_and_fetcher(launcher_bytes: &[u8]) -> (Lock, MemoryFetcher) {
        let (lock, fetcher) = fake_lock_json(launcher_bytes);
        (Lock::parse(&lock.to_string()).unwrap(), fetcher)
    }

    /// [`fake_lock_and_fetcher`], with the lock still as JSON so a test can change a pin.
    pub(crate) fn fake_lock_json(launcher_bytes: &[u8]) -> (serde_json::Value, MemoryFetcher) {
        let server = b"vanilla server".as_slice();
        let api = b"fabric api".as_slice();
        let installer = b"fabric installer".as_slice();
        let neo = b"neoforge installer".as_slice();
        let lock = serde_json::json!({
            "jdk": {"major": 21},
            "game_versions": {"9.9": {
                "mojang": {"server_jar": {"url": "https://mojang.invalid/server.jar", "sha1": sha(HashAlgorithm::Sha1, server), "size": server.len(), "hash_source": "mojang-version-json"}},
                "fabric": {
                    "loader": {"version": "0.20.1"},
                    "installer": {"version": "1.1.2", "url": "https://fabric.invalid/fabric-installer-1.1.2.jar", "sha1": sha(HashAlgorithm::Sha1, installer), "sha256": sha(HashAlgorithm::Sha256, installer), "sha512": sha(HashAlgorithm::Sha512, installer)},
                    "server_launcher": {"url": "https://meta.fabric.invalid/server/jar", "observed_sha256": sha(HashAlgorithm::Sha256, b"launcher"), "hash_source": "observed"},
                    "api": {"url": "https://cdn.invalid/fabric-api-1%2B9.9.jar", "filename": "fabric-api-1+9.9.jar", "sha1": sha(HashAlgorithm::Sha1, api), "sha512": sha(HashAlgorithm::Sha512, api)}
                },
                "neoforge": {
                    "version": "9.9.9",
                    "installer": {"url": "https://neoforged.invalid/neoforge-9.9.9-installer.jar", "sha1": sha(HashAlgorithm::Sha1, neo), "sha256": sha(HashAlgorithm::Sha256, neo), "sha512": sha(HashAlgorithm::Sha512, neo)}
                }
            }}
        });
        let fetcher = MemoryFetcher::with(&[
            ("https://mojang.invalid/server.jar", server),
            (
                "https://fabric.invalid/fabric-installer-1.1.2.jar",
                installer,
            ),
            ("https://meta.fabric.invalid/server/jar", launcher_bytes),
            ("https://cdn.invalid/fabric-api-1%2B9.9.jar", api),
            (
                "https://neoforged.invalid/neoforge-9.9.9-installer.jar",
                neo,
            ),
        ]);
        (lock, fetcher)
    }

    fn ctx<'a>(
        lock: &'a Lock,
        cache: &'a Path,
        fetcher: &'a MemoryFetcher,
        host: &'a InstallerHost,
        java: Option<&'a Path>,
    ) -> ProvisionContext<'a> {
        ProvisionContext {
            lock,
            game_version: "9.9",
            cache,
            fetcher,
            host,
            java,
        }
    }

    #[test]
    fn fabric_uses_the_launcher_with_a_preplaced_server_jar() {
        let tmp = TempDir::new("prov-fabric");
        let (lock, fetcher) = fake_lock_and_fetcher(b"launcher");
        let host = InstallerHost::default();
        let installed = provision(
            Loader::Fabric,
            &ctx(&lock, tmp.path(), &fetcher, &host, None),
        )
        .unwrap();
        assert_eq!(installed.method, "fabric-server-launcher");
        assert_eq!(
            installed.launch,
            Launch::Jar {
                jar: "fabric-server-launcher.jar".into(),
                jvm_args: vec!["-Dfabric.installer.server.gameJar=server.jar".into()],
            }
        );
        assert_eq!(
            fs::read(installed.dir.join("server.jar")).unwrap(),
            b"vanilla server"
        );
        assert_eq!(installed.loader_mods[0].sha256, sha256_hex(b"fabric api"));
        assert!(
            installed.loader_mods[0]
                .path
                .ends_with("fabric-api-1+9.9.jar")
        );
        assert!(
            host.runs.borrow().is_empty(),
            "no installer runs on the launcher path"
        );
        assert!(installed.dir.join(INSTALL_RECORD_FILE).is_file());

        // A second provision reuses the install and downloads nothing.
        let calls = fetcher.calls.borrow().len();
        let again = provision(
            Loader::Fabric,
            &ctx(&lock, tmp.path(), &fetcher, &host, None),
        )
        .unwrap();
        assert_eq!(again, installed);
        assert_eq!(fetcher.calls.borrow().len(), calls);
    }

    #[test]
    fn fabric_falls_back_to_the_pinned_installer_when_the_launcher_hash_differs() {
        let tmp = TempDir::new("prov-fallback");
        let (lock, fetcher) = fake_lock_and_fetcher(b"a different launcher");
        let host = InstallerHost::default();
        let java = PathBuf::from("/jdk/bin/java");
        let installed = provision(
            Loader::Fabric,
            &ctx(&lock, tmp.path(), &fetcher, &host, Some(&java)),
        )
        .unwrap();
        assert_eq!(installed.method, "fabric-installer");
        assert!(installed.notes[0].contains("hash-pinned installer"));
        let runs = host.runs.borrow();
        let args = runs[0].args_lossy();
        assert_eq!(args[2..4], ["server".to_owned(), "-dir".to_owned()]);
        assert_eq!(
            args[5..],
            ["-mcversion", "9.9", "-loader", "0.20.1"].map(String::from)
        );
        assert_eq!(
            fs::read_to_string(installed.dir.join("fabric-server-launcher.properties")).unwrap(),
            "serverJar=server.jar\n"
        );
        assert!(
            !installed.dir.join("fabric-server-launcher.jar").exists(),
            "the unverified launcher is not used"
        );
    }

    #[test]
    fn neoforge_runs_the_installer_and_finds_the_argument_files() {
        let tmp = TempDir::new("prov-neo");
        let (lock, fetcher) = fake_lock_and_fetcher(b"launcher");
        let host = InstallerHost::default();
        let java = PathBuf::from("/jdk/bin/java");
        let installed = provision(
            Loader::NeoForge,
            &ctx(&lock, tmp.path(), &fetcher, &host, Some(&java)),
        )
        .unwrap();
        assert_eq!(
            installed.launch,
            Launch::ArgFile {
                unix: Some("libraries/net/neoforged/neoforge/9.9.9/unix_args.txt".into()),
                windows: Some("libraries/net/neoforged/neoforge/9.9.9/win_args.txt".into()),
            }
        );
        let runs = host.runs.borrow();
        assert_eq!(runs[0].program, java);
        assert_eq!(
            runs[0].args_lossy().last().map(String::as_str),
            Some("--installServer")
        );
        assert_eq!(runs[0].cwd, installed.dir);
    }

    #[test]
    fn neoforge_needs_java_and_a_successful_installer() {
        let tmp = TempDir::new("prov-neo-fail");
        let (lock, fetcher) = fake_lock_and_fetcher(b"launcher");
        let host = InstallerHost::default();
        let err = provision(
            Loader::NeoForge,
            &ctx(&lock, tmp.path(), &fetcher, &host, None),
        )
        .unwrap_err();
        assert!(matches!(err, HarnessError::Java(_)), "{err}");
        let failing = InstallerHost {
            exit_code: Some(1),
            ..InstallerHost::default()
        };
        let java = PathBuf::from("java");
        let err = provision(
            Loader::NeoForge,
            &ctx(&lock, tmp.path(), &fetcher, &failing, Some(&java)),
        )
        .unwrap_err();
        assert!(err.to_string().contains("exited with code 1"), "{err}");
        assert!(
            !server_dir(tmp.path(), Loader::NeoForge, "9.9.9", "9.9")
                .join(INSTALL_RECORD_FILE)
                .exists()
        );
    }

    #[test]
    fn a_download_that_breaks_its_pin_stops_provisioning() {
        let tmp = TempDir::new("prov-tamper");
        let (mut lock, fetcher) = fake_lock_json(b"launcher");
        lock["game_versions"]["9.9"]["mojang"]["server_jar"]["sha1"] =
            serde_json::json!("1".repeat(40));
        let lock = Lock::parse(&lock.to_string()).unwrap();
        let host = InstallerHost::default();
        let err = provision(
            Loader::Fabric,
            &ctx(&lock, tmp.path(), &fetcher, &host, None),
        )
        .unwrap_err();
        match &err {
            HarnessError::Download(DownloadError::Mismatch { algorithm, .. }) => {
                assert_eq!(*algorithm, HashAlgorithm::Sha1);
            }
            other => panic!("unexpected {other}"),
        }
        let dir = server_dir(tmp.path(), Loader::Fabric, "0.20.1", "9.9");
        assert!(!dir.join(INSTALL_RECORD_FILE).exists());
        assert!(!dir.join("server.jar").exists());
        assert!(host.runs.borrow().is_empty());
    }

    #[test]
    fn forge_and_quilt_are_not_built_yet() {
        let tmp = TempDir::new("prov-forge");
        let (lock, fetcher) = fake_lock_and_fetcher(b"launcher");
        let host = InstallerHost::default();
        for loader in [Loader::Forge, Loader::Quilt] {
            let err =
                provision(loader, &ctx(&lock, tmp.path(), &fetcher, &host, None)).unwrap_err();
            assert!(matches!(err, HarnessError::NotBuiltYet { .. }));
            assert!(err.to_string().contains("not built yet"), "{err}");
            assert_eq!(profile(loader).loader(), loader);
        }
        assert!(fetcher.calls.borrow().is_empty());
    }

    #[test]
    fn argument_files_come_from_the_start_script_or_the_conventional_path() {
        let tmp = TempDir::new("argfile");
        let dir = tmp.path();
        assert_eq!(find_arg_file(dir, "run.sh", "1", "unix_args.txt"), None);
        fs::create_dir_all(dir.join("libraries/net/neoforged/neoforge/1")).unwrap();
        fs::write(
            dir.join("libraries/net/neoforged/neoforge/1/unix_args.txt"),
            "",
        )
        .unwrap();
        assert_eq!(
            find_arg_file(dir, "run.sh", "1", "unix_args.txt").as_deref(),
            Some("libraries/net/neoforged/neoforge/1/unix_args.txt")
        );
        fs::create_dir_all(dir.join("other")).unwrap();
        fs::write(dir.join("other/server_args.txt"), "").unwrap();
        fs::write(
            dir.join("run.sh"),
            "java @user_jvm_args.txt \"@other/server_args.txt\" \"$@\"",
        )
        .unwrap();
        assert_eq!(
            find_arg_file(dir, "run.sh", "1", "unix_args.txt").as_deref(),
            Some("other/server_args.txt")
        );
        fs::write(dir.join("run.sh"), "java @../../escape/args.txt").unwrap();
        assert_eq!(
            find_arg_file(dir, "run.sh", "1", "unix_args.txt").as_deref(),
            Some("libraries/net/neoforged/neoforge/1/unix_args.txt"),
            "paths leaving the server directory are ignored"
        );
    }
}
