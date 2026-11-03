//! The pins in `research-lock.json` that the harness provisions from.
//!
//! The lock is read leniently: only the fields the harness needs are looked up, by path, so
//! new loaders, game versions and fields can be added to it without breaking this build. A
//! pin the harness needs and cannot find is an error naming its path.

use std::fmt;

use serde_json::Value;

use crate::fetch::{Artifact, ExpectedHash, HashAlgorithm};
use crate::loader::Loader;

/// The repository's `research-lock.json`, embedded at build time. `--lock` overrides it.
pub const EMBEDDED_LOCK: &str = include_str!("../../../research-lock.json");

/// A pin that is missing or malformed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LockError {
    /// Not JSON.
    Json(String),
    /// A field the harness needs is absent.
    Missing(String),
    /// A field is present but unusable.
    Invalid {
        /// Dotted path of the field.
        path: String,
        /// Why.
        why: String,
    },
}

impl fmt::Display for LockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LockError::Json(e) => write!(f, "not valid JSON: {e}"),
            LockError::Missing(path) => write!(f, "no pin at {path}"),
            LockError::Invalid { path, why } => write!(f, "{path}: {why}"),
        }
    }
}

impl std::error::Error for LockError {}

/// The Fabric pins for one game version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FabricPins {
    /// Fabric Loader version.
    pub loader_version: String,
    /// Fabric installer version.
    pub installer_version: String,
    /// The hash-pinned installer (fallback when the launcher does not match).
    pub installer: Artifact,
    /// The meta.fabricmc.net server launcher, pinned by its observed SHA-256.
    pub server_launcher: Artifact,
    /// Fabric API, which every Fabric harness server loads.
    pub api: Artifact,
}

/// The NeoForge pins for one game version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NeoForgePins {
    /// NeoForge version.
    pub version: String,
    /// The installer jar.
    pub installer: Artifact,
}

/// A parsed `research-lock.json`.
#[derive(Debug, Clone, PartialEq)]
pub struct Lock {
    root: Value,
}

impl Lock {
    /// The lock embedded at build time.
    pub fn embedded() -> Result<Self, LockError> {
        Self::parse(EMBEDDED_LOCK)
    }

    /// Parses a lock file.
    pub fn parse(text: &str) -> Result<Self, LockError> {
        let root: Value = serde_json::from_str(text).map_err(|e| LockError::Json(e.to_string()))?;
        if !root.is_object() {
            return Err(LockError::Invalid {
                path: "(root)".to_owned(),
                why: "must be an object".to_owned(),
            });
        }
        Ok(Self { root })
    }

    /// The pinned JDK major version.
    pub fn jdk_major(&self) -> Option<u64> {
        self.root.pointer("/jdk/major").and_then(Value::as_u64)
    }

    /// Every pinned game version.
    pub fn game_versions(&self) -> Vec<String> {
        self.root
            .get("game_versions")
            .and_then(Value::as_object)
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default()
    }

    fn game(&self, game_version: &str) -> Result<&Value, LockError> {
        self.root
            .get("game_versions")
            .and_then(|g| g.get(game_version))
            .ok_or_else(|| {
                LockError::Missing(format!(
                    "game_versions.{game_version} (pinned: {})",
                    self.game_versions().join(", ")
                ))
            })
    }

    /// The loader version pinned for `loader` on `game_version`.
    pub fn loader_version(&self, loader: Loader, game_version: &str) -> Result<String, LockError> {
        let game = self.game(game_version)?;
        let (pointer, path) = match loader {
            Loader::Fabric => ("/fabric/loader/version", "fabric.loader.version"),
            Loader::NeoForge => ("/neoforge/version", "neoforge.version"),
            Loader::Forge => ("/forge/version", "forge.version"),
            Loader::Quilt => ("/quilt/loader/version", "quilt.loader.version"),
        };
        string_at(
            game,
            pointer,
            &format!("game_versions.{game_version}.{path}"),
        )
    }

    /// The vanilla dedicated server jar.
    pub fn mojang_server_jar(&self, game_version: &str) -> Result<Artifact, LockError> {
        let game = self.game(game_version)?;
        artifact(
            game,
            "/mojang/server_jar",
            &format!("game_versions.{game_version}.mojang.server_jar"),
            "minecraft-server",
            Some("server.jar"),
        )
    }

    /// The Fabric pins.
    pub fn fabric(&self, game_version: &str) -> Result<FabricPins, LockError> {
        let game = self.game(game_version)?;
        let base = format!("game_versions.{game_version}.fabric");
        let installer = artifact(
            game,
            "/fabric/installer",
            &format!("{base}.installer"),
            "fabric-installer",
            None,
        )?;
        let launcher_path = format!("{base}.server_launcher");
        let launcher = game
            .pointer("/fabric/server_launcher")
            .ok_or_else(|| LockError::Missing(launcher_path.clone()))?;
        let observed = string_at(
            launcher,
            "/observed_sha256",
            &format!("{launcher_path}.observed_sha256"),
        )?;
        let server_launcher = Artifact {
            role: "fabric-server-launcher".to_owned(),
            url: url_at(launcher, &launcher_path)?,
            file_name: "fabric-server-launcher.jar".to_owned(),
            size: launcher.get("size").and_then(Value::as_u64),
            hashes: vec![hash(
                HashAlgorithm::Sha256,
                &observed,
                &format!("{launcher_path}.observed_sha256"),
            )?],
            hash_source: launcher
                .get("hash_source")
                .and_then(Value::as_str)
                .unwrap_or("observed")
                .to_owned(),
        };
        Ok(FabricPins {
            loader_version: string_at(
                game,
                "/fabric/loader/version",
                &format!("{base}.loader.version"),
            )?,
            installer_version: string_at(
                game,
                "/fabric/installer/version",
                &format!("{base}.installer.version"),
            )?,
            installer,
            server_launcher,
            api: artifact(
                game,
                "/fabric/api",
                &format!("{base}.api"),
                "fabric-api",
                None,
            )?,
        })
    }

    /// The NeoForge pins.
    pub fn neoforge(&self, game_version: &str) -> Result<NeoForgePins, LockError> {
        let game = self.game(game_version)?;
        let base = format!("game_versions.{game_version}.neoforge");
        Ok(NeoForgePins {
            version: string_at(game, "/neoforge/version", &format!("{base}.version"))?,
            installer: artifact(
                game,
                "/neoforge/installer",
                &format!("{base}.installer"),
                "neoforge-installer",
                None,
            )?,
        })
    }
}

fn string_at(value: &Value, pointer: &str, path: &str) -> Result<String, LockError> {
    match value.pointer(pointer) {
        None => Err(LockError::Missing(path.to_owned())),
        Some(Value::String(s)) if !s.trim().is_empty() => Ok(s.clone()),
        Some(_) => Err(LockError::Invalid {
            path: path.to_owned(),
            why: "must be a non-empty string".to_owned(),
        }),
    }
}

fn url_at(value: &Value, path: &str) -> Result<String, LockError> {
    let url = string_at(value, "/url", &format!("{path}.url"))?;
    if !url.starts_with("https://") {
        return Err(LockError::Invalid {
            path: format!("{path}.url"),
            why: "pinned downloads must be https".to_owned(),
        });
    }
    Ok(url)
}

fn hash(algorithm: HashAlgorithm, hex: &str, path: &str) -> Result<ExpectedHash, LockError> {
    let hex = hex.trim().to_ascii_lowercase();
    if hex.len() != algorithm.hex_len() || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(LockError::Invalid {
            path: path.to_owned(),
            why: format!("not a {algorithm} hex digest"),
        });
    }
    Ok(ExpectedHash { algorithm, hex })
}

fn artifact(
    game: &Value,
    pointer: &str,
    path: &str,
    role: &str,
    file_name: Option<&str>,
) -> Result<Artifact, LockError> {
    let value = game
        .pointer(pointer)
        .ok_or_else(|| LockError::Missing(path.to_owned()))?;
    let url = url_at(value, path)?;
    let mut hashes = Vec::new();
    for algorithm in HashAlgorithm::ALL {
        if let Some(hex) = value.get(algorithm.id()).and_then(Value::as_str) {
            hashes.push(hash(algorithm, hex, &format!("{path}.{algorithm}"))?);
        }
    }
    if hashes.is_empty() {
        return Err(LockError::Invalid {
            path: path.to_owned(),
            why: "no sha1, sha256 or sha512 pinned, so it cannot be verified".to_owned(),
        });
    }
    let file_name = match file_name {
        Some(name) => name.to_owned(),
        None => value
            .get("filename")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .unwrap_or_else(|| file_name_from_url(&url)),
    };
    if file_name.is_empty() || file_name.contains(['/', '\\']) || file_name.starts_with('.') {
        return Err(LockError::Invalid {
            path: path.to_owned(),
            why: format!("unusable file name {file_name:?}"),
        });
    }
    Ok(Artifact {
        role: role.to_owned(),
        url,
        file_name,
        size: value.get("size").and_then(Value::as_u64),
        hashes,
        hash_source: value
            .get("hash_source")
            .and_then(Value::as_str)
            .unwrap_or("unspecified")
            .to_owned(),
    })
}

/// The last path segment of a URL, percent-decoded (`fabric-api-0.116.17%2B1.21.1.jar`
/// becomes `fabric-api-0.116.17+1.21.1.jar`).
pub fn file_name_from_url(url: &str) -> String {
    let path = url.split(['?', '#']).next().unwrap_or_default();
    let segment = path.rsplit('/').next().unwrap_or_default();
    let bytes = segment.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let Some(byte) = std::str::from_utf8(&bytes[i + 1..i + 3])
                .ok()
                .and_then(|h| u8::from_str_radix(h, 16).ok())
        {
            out.push(byte);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_embedded_lock_has_every_pin_the_harness_needs() {
        let lock = Lock::embedded().unwrap();
        assert_eq!(lock.jdk_major(), Some(21));
        for game in ["1.21.1", "1.21.11"] {
            let server = lock.mojang_server_jar(game).unwrap();
            assert_eq!(server.file_name, "server.jar");
            assert!(
                server
                    .hashes
                    .iter()
                    .any(|h| h.algorithm == HashAlgorithm::Sha1)
            );
            let fabric = lock.fabric(game).unwrap();
            assert_eq!(fabric.loader_version, "0.20.1");
            assert_eq!(
                fabric.server_launcher.hashes[0].algorithm,
                HashAlgorithm::Sha256
            );
            assert_eq!(fabric.server_launcher.hash_source, "observed");
            assert!(
                fabric.api.file_name.starts_with("fabric-api-"),
                "{}",
                fabric.api.file_name
            );
            assert!(fabric.api.hashes.len() >= 2);
            assert_eq!(fabric.installer.hashes.len(), 3);
            let neoforge = lock.neoforge(game).unwrap();
            assert!(neoforge.installer.file_name.ends_with("-installer.jar"));
            assert_eq!(neoforge.installer.hashes.len(), 3);
            assert_eq!(
                lock.loader_version(Loader::NeoForge, game).unwrap(),
                neoforge.version
            );
        }
        assert!(lock.game_versions().contains(&"1.21.1".to_owned()));
    }

    #[test]
    fn missing_pins_name_their_path() {
        let lock = Lock::embedded().unwrap();
        let err = lock.fabric("1.7.10").unwrap_err();
        assert!(
            err.to_string()
                .starts_with("no pin at game_versions.1.7.10"),
            "{err}"
        );
        let err = lock.loader_version(Loader::Quilt, "1.21.1").unwrap_err();
        assert_eq!(
            err,
            LockError::Missing("game_versions.1.21.1.quilt.loader.version".into())
        );
    }

    #[test]
    fn bad_pins_are_refused() {
        let lock = Lock::parse(
            r#"{"game_versions": {"9.9": {
                "mojang": {"server_jar": {"url": "http://insecure.example/server.jar", "sha1": "00"}},
                "neoforge": {"version": "1", "installer": {"url": "https://x.example/i.jar"}}
            }}}"#,
        )
        .unwrap();
        assert!(
            lock.mojang_server_jar("9.9")
                .unwrap_err()
                .to_string()
                .contains("https")
        );
        assert!(
            lock.neoforge("9.9")
                .unwrap_err()
                .to_string()
                .contains("cannot be verified")
        );
        let lock = Lock::parse(
            r#"{"game_versions": {"9.9": {"mojang": {"server_jar": {"url": "https://x.example/s.jar", "sha1": "zz"}}}}}"#,
        )
        .unwrap();
        assert!(
            lock.mojang_server_jar("9.9")
                .unwrap_err()
                .to_string()
                .contains("sha1")
        );
        assert!(Lock::parse("[]").is_err());
        assert!(Lock::parse("{").is_err());
    }

    #[test]
    fn file_names_are_decoded_from_urls() {
        assert_eq!(
            file_name_from_url("https://cdn.example/data/x/fabric-api-0.116.17%2B1.21.1.jar?x=1"),
            "fabric-api-0.116.17+1.21.1.jar"
        );
        assert_eq!(file_name_from_url("https://x.example/a/b%"), "b%");
        assert_eq!(file_name_from_url("https://x.example/a/b%2"), "b%2");
    }
}
