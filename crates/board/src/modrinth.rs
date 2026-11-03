//! The parts of the Modrinth API v2 the board reads, and the rules for picking files.
//!
//! - Search: `GET /v2/search` with facets `project_type:mod`, `categories:<loader tag>` and
//!   `versions:<v1>` OR `versions:<v2>` ..., sorted by downloads, 100 per page.
//! - Versions: `GET /v2/project/<id>/version?loaders=[...]&game_versions=[...]&include_changelog=false`.
//!
//! The loader tag is the source loader's `modrinth_loader` from its profile
//! ([`Loader::modrinth_tag`]); a loader whose tag is not researched has no list.
//!
//! Field names follow <https://docs.modrinth.com/api/>. Unknown fields are ignored, so
//! Modrinth can add fields without breaking the board; missing required fields are errors.

use serde::Deserialize;

use crate::Error;
use crate::gamever;
use crate::hash;
use crate::loader::Loader;
use crate::time;

/// The production API base.
pub const API_BASE: &str = "https://api.modrinth.com/v2";

/// The only host the board downloads JARs from.
pub const CDN_PREFIX: &str = "https://cdn.modrinth.com/";

/// Projects per search page (Modrinth's maximum).
pub const PAGE_SIZE: u32 = 100;

/// One page of `GET /v2/search`.
#[derive(Debug, Clone, Deserialize)]
pub struct SearchPage {
    /// The projects on this page.
    pub hits: Vec<SearchHit>,
    /// The offset of the first hit.
    pub offset: u64,
    /// The page size Modrinth applied.
    pub limit: u64,
    /// Hits across all pages.
    pub total_hits: u64,
}

/// One project in a search page.
#[derive(Debug, Clone, Deserialize)]
pub struct SearchHit {
    /// Modrinth project id (base62).
    pub project_id: String,
    /// URL slug.
    pub slug: String,
    /// Display name.
    pub title: String,
    /// All-time downloads.
    pub downloads: u64,
    /// SPDX id or `LicenseRef-...`.
    #[serde(default)]
    pub license: Option<String>,
    /// `required`, `optional`, `unsupported` or `unknown`.
    #[serde(default)]
    pub client_side: Option<String>,
    /// `required`, `optional`, `unsupported` or `unknown`.
    #[serde(default)]
    pub server_side: Option<String>,
    /// Icon URL.
    #[serde(default)]
    pub icon_url: Option<String>,
    /// Categories and loader tags together, as Modrinth indexes them.
    #[serde(default)]
    pub categories: Vec<String>,
    /// Project type; the search facet asks for `mod`.
    #[serde(default)]
    pub project_type: Option<String>,
}

/// One version of a project.
#[derive(Debug, Clone, Deserialize)]
pub struct Version {
    /// Version id.
    pub id: String,
    /// The project it belongs to.
    pub project_id: String,
    /// The author's version string.
    pub version_number: String,
    /// `release`, `beta` or `alpha`.
    pub version_type: String,
    /// RFC 3339 publication time.
    pub date_published: String,
    /// Game versions this version declares.
    pub game_versions: Vec<String>,
    /// Loader tags this version declares.
    pub loaders: Vec<String>,
    /// The files; one is usually marked primary.
    pub files: Vec<VersionFile>,
}

/// One file of a version.
#[derive(Debug, Clone, Deserialize)]
pub struct VersionFile {
    /// Hashes Modrinth computed.
    pub hashes: FileHashes,
    /// Download URL.
    pub url: String,
    /// File name.
    pub filename: String,
    /// Whether this is the version's main file.
    #[serde(default)]
    pub primary: bool,
    /// Size in bytes.
    pub size: u64,
}

/// File hashes.
#[derive(Debug, Clone, Deserialize)]
pub struct FileHashes {
    /// SHA-512, hex.
    #[serde(default)]
    pub sha512: Option<String>,
}

/// The file the board tests for one game version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickedFile {
    /// The game version.
    pub game_version: String,
    /// The version it comes from.
    pub version_id: String,
    /// The author's version string.
    pub version_number: String,
    /// `release` or `beta`.
    pub version_type: String,
    /// RFC 3339 publication time of the version.
    pub date_published: String,
    /// File name.
    pub filename: String,
    /// CDN URL.
    pub url: String,
    /// SHA-512, lowercase hex.
    pub sha512: String,
    /// Size in bytes.
    pub size: u64,
}

/// Percent-encodes a query value (everything but RFC 3986 unreserved characters).
pub fn encode_query_value(value: &str) -> String {
    let mut out = String::with_capacity(value.len() * 3);
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(char::from(byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

fn json_string_array(items: &[String]) -> String {
    serde_json::Value::from(items.to_vec()).to_string()
}

/// The search facets for a source loader's Modrinth tag and the pinned versions, as
/// Modrinth's JSON:
/// `[["project_type:mod"],["categories:fabric"],["versions:1.21.1","versions:1.21.11"]]`.
/// Inner arrays are OR, outer arrays AND.
pub fn search_facets(tag: &str, game_versions: &[String]) -> String {
    let versions: Vec<String> = game_versions
        .iter()
        .map(|v| format!("versions:{v}"))
        .collect();
    let facets = vec![
        vec!["project_type:mod".to_owned()],
        vec![format!("categories:{tag}")],
        versions,
    ];
    serde_json::Value::from(facets).to_string()
}

/// `GET /v2/search` for one page, sorted by downloads.
pub fn search_url(api_base: &str, tag: &str, game_versions: &[String], offset: u64) -> String {
    format!(
        "{}/search?facets={}&index=downloads&offset={offset}&limit={PAGE_SIZE}",
        api_base.trim_end_matches('/'),
        encode_query_value(&search_facets(tag, game_versions)),
    )
}

/// Whether `id` looks like a Modrinth id (base62), so it is safe in a URL path.
pub fn is_modrinth_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.bytes().all(|b| b.is_ascii_alphanumeric())
}

/// `GET /v2/project/<id>/version`, filtered to the loader tag and the pinned versions, without
/// changelogs.
pub fn versions_url(
    api_base: &str,
    project_id: &str,
    tag: &str,
    game_versions: &[String],
) -> Result<String, Error> {
    if !is_modrinth_id(project_id) {
        return Err(Error::invalid(format!(
            "{project_id:?} is not a Modrinth project id"
        )));
    }
    Ok(format!(
        "{}/project/{project_id}/version?loaders={}&game_versions={}&include_changelog=false",
        api_base.trim_end_matches('/'),
        encode_query_value(&json_string_array(&[tag.to_owned()])),
        encode_query_value(&json_string_array(game_versions)),
    ))
}

/// Parses a search page.
pub fn parse_search(body: &[u8]) -> Result<SearchPage, Error> {
    serde_json::from_slice(body).map_err(|e| Error::json("Modrinth search page", e))
}

/// Parses a project's version list.
pub fn parse_versions(body: &[u8]) -> Result<Vec<Version>, Error> {
    serde_json::from_slice(body).map_err(|e| Error::json("Modrinth version list", e))
}

/// The loader ids among a project's categories, sorted (Modrinth puts loaders and categories
/// in one list). Informational only (the list's `loaders` and the rows'
/// `target_build_exists`): it matches the category strings against Jarrow's loader ids and
/// decides nothing the board measures.
pub fn loader_tags(categories: &[String]) -> Vec<String> {
    let mut tags: Vec<String> = Loader::ALL
        .iter()
        .filter(|l| categories.iter().any(|c| c == l.id()))
        .map(|l| l.id().to_owned())
        .collect();
    tags.sort();
    tags
}

fn usable_file(version: &Version) -> Option<(&VersionFile, String)> {
    let candidates = version
        .files
        .iter()
        .filter(|f| f.primary)
        .chain(version.files.iter().filter(|f| !f.primary));
    for file in candidates {
        let Some(sha512) = file.hashes.sha512.as_deref().map(str::to_ascii_lowercase) else {
            continue;
        };
        let ok = hash::is_hex(&sha512, 128)
            && file.url.starts_with(CDN_PREFIX)
            && file.size > 0
            && file.filename.to_ascii_lowercase().ends_with(".jar");
        if ok {
            return Some((file, sha512));
        }
    }
    None
}

/// Picks the file to test for one game version: the newest `release` that declares the
/// loader tag and the game version, or the newest `beta` when there is no release. Alphas are
/// never picked. Within a version, the primary file wins, and only a `.jar` on Modrinth's CDN
/// with a sha512 counts. Ties on the publication time break on the version id, so the pick is
/// deterministic.
pub fn pick_file(versions: &[Version], tag: &str, game_version: &str) -> Option<PickedFile> {
    let matching = |kind: &str| {
        versions
            .iter()
            .filter(|v| v.version_type == kind)
            .filter(|v| v.loaders.iter().any(|l| l == tag))
            .filter(|v| v.game_versions.iter().any(|g| g == game_version))
            .filter(|v| time::is_rfc3339_utc(&v.date_published))
            .filter_map(|v| usable_file(v).map(|(file, sha)| (v, file, sha)))
            .max_by(|a, b| {
                time::compare(&a.0.date_published, &b.0.date_published)
                    .then_with(|| a.0.id.cmp(&b.0.id))
            })
    };
    let (version, file, sha512) = matching("release").or_else(|| matching("beta"))?;
    Some(PickedFile {
        game_version: game_version.to_owned(),
        version_id: version.id.clone(),
        version_number: version.version_number.clone(),
        version_type: version.version_type.clone(),
        date_published: version.date_published.clone(),
        filename: file.filename.clone(),
        url: file.url.clone(),
        sha512,
        size: file.size,
    })
}

/// Picks one file per pinned game version (see [`pick_file`]), oldest version first.
pub fn pick_files(versions: &[Version], tag: &str, game_versions: &[String]) -> Vec<PickedFile> {
    let mut sorted = game_versions.to_vec();
    gamever::sort_versions(&mut sorted);
    sorted
        .iter()
        .filter_map(|gv| pick_file(versions, tag, gv))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gv(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn facets_follow_the_loader_tag() {
        assert_eq!(
            search_facets("fabric", &gv(&["1.21.1", "1.21.11"])),
            r#"[["project_type:mod"],["categories:fabric"],["versions:1.21.1","versions:1.21.11"]]"#
        );
        assert_eq!(
            search_facets("forge", &gv(&["1.20.1"])),
            r#"[["project_type:mod"],["categories:forge"],["versions:1.20.1"]]"#
        );
    }

    #[test]
    fn urls_are_encoded() {
        let url = search_url(
            "https://api.modrinth.com/v2/",
            "quilt",
            &gv(&["1.21.1"]),
            200,
        );
        assert_eq!(
            url,
            "https://api.modrinth.com/v2/search?facets=%5B%5B%22project_type%3Amod%22%5D%2C\
             %5B%22categories%3Aquilt%22%5D%2C%5B%22versions%3A1.21.1%22%5D%5D\
             &index=downloads&offset=200&limit=100"
        );
        let url = versions_url(API_BASE, "AANobbMI", "neoforge", &gv(&["1.21.1"])).unwrap();
        assert_eq!(
            url,
            "https://api.modrinth.com/v2/project/AANobbMI/version?loaders=%5B%22neoforge%22%5D\
             &game_versions=%5B%221.21.1%22%5D&include_changelog=false"
        );
        assert!(versions_url(API_BASE, "../admin", "fabric", &gv(&["1.21.1"])).is_err());
        assert!(versions_url(API_BASE, "", "fabric", &gv(&["1.21.1"])).is_err());
    }

    #[test]
    fn loader_tags_come_from_categories() {
        let cats = gv(&["optimization", "quilt", "fabric", "neoforge", "library"]);
        assert_eq!(loader_tags(&cats), gv(&["fabric", "neoforge", "quilt"]));
        assert!(loader_tags(&gv(&["library"])).is_empty());
    }
}
