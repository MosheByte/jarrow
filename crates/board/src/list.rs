//! `board-list.json`: the mods a direction's board tests, fetched from Modrinth.
//!
//! The list holds the top `top` projects of the source loader, by downloads, that ship at
//! least one usable file for a pinned game version. Projects the search returns without such
//! a file are recorded under `skipped`, with the reason, and do not count towards `top`.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::gamever;
use crate::http::Fetch;
use crate::loader::{Direction, Loader};
use crate::modrinth::{self, PickedFile, SearchHit};
use crate::time;
use crate::{Error, SCHEMA};

/// Largest search page or version list the board accepts, in bytes.
pub const MAX_API_BODY: u64 = 32 * 1024 * 1024;

/// The largest `--top` the pipeline accepts.
pub const MAX_TOP: u32 = 10_000;

/// `board-list.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoardList {
    /// Always [`SCHEMA`].
    pub schema: u32,
    /// When the list was fetched, RFC 3339 UTC.
    pub fetched_at: String,
    /// The direction, `<source>-<target>`.
    pub direction: Direction,
    /// The direction's source loader (repeated for readers that do not parse `direction`).
    pub source_loader: Loader,
    /// The direction's target loader.
    pub target_loader: Loader,
    /// The pinned game versions, oldest first.
    pub game_versions: Vec<String>,
    /// How many mods were asked for.
    pub top: u32,
    /// The mods, most downloaded first.
    pub mods: Vec<ListMod>,
    /// Search hits that were left out, with the reason.
    pub skipped: Vec<ListSkip>,
}

/// One mod of the list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListMod {
    /// Modrinth project id.
    pub project_id: String,
    /// URL slug.
    pub slug: String,
    /// Display name.
    pub title: String,
    /// All-time downloads when the list was fetched.
    pub downloads: u64,
    /// SPDX id or `LicenseRef-...`, as Modrinth lists it.
    pub license: Option<String>,
    /// Modrinth's client-side flag.
    pub client_side: Option<String>,
    /// Modrinth's server-side flag.
    pub server_side: Option<String>,
    /// Icon URL.
    pub icon_url: Option<String>,
    /// The project's loader tags on Modrinth, sorted.
    pub loaders: Vec<String>,
    /// Whether Modrinth tags the project with `neoforge`. Informational: not an
    /// author-verified port.
    pub neoforge_build_exists: bool,
    /// Whether Modrinth tags the project with the direction's target loader.
    pub target_build_exists: bool,
    /// At most one file per pinned game version, oldest version first.
    pub files: Vec<ListFile>,
}

/// The file the board tests for one game version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListFile {
    /// The game version.
    pub game_version: String,
    /// Modrinth version id.
    pub version_id: String,
    /// The author's version string.
    pub version_number: String,
    /// `release` or `beta`.
    pub version_type: String,
    /// File name.
    pub filename: String,
    /// CDN URL.
    pub url: String,
    /// SHA-512, lowercase hex.
    pub sha512: String,
    /// Size in bytes.
    pub size: u64,
}

/// A search hit the list leaves out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListSkip {
    /// Modrinth project id.
    pub project_id: String,
    /// URL slug.
    pub slug: String,
    /// Why it is not on the list.
    pub reason: String,
}

impl From<PickedFile> for ListFile {
    fn from(p: PickedFile) -> Self {
        Self {
            game_version: p.game_version,
            version_id: p.version_id,
            version_number: p.version_number,
            version_type: p.version_type,
            filename: p.filename,
            url: p.url,
            sha512: p.sha512,
            size: p.size,
        }
    }
}

/// What to fetch.
#[derive(Debug, Clone)]
pub struct ListOptions {
    /// The direction; its source loader picks the facets and the files.
    pub direction: Direction,
    /// The pinned game versions.
    pub game_versions: Vec<String>,
    /// How many mods to keep.
    pub top: u32,
    /// The API base, normally [`modrinth::API_BASE`].
    pub api_base: String,
}

/// Builds a list entry from a search hit and the files picked for it.
pub fn list_mod(hit: &SearchHit, direction: Direction, files: Vec<PickedFile>) -> ListMod {
    let loaders = modrinth::loader_tags(&hit.categories);
    let tagged = |loader: Loader| loaders.iter().any(|l| l == loader.id());
    ListMod {
        project_id: hit.project_id.clone(),
        slug: hit.slug.clone(),
        title: hit.title.clone(),
        downloads: hit.downloads,
        license: hit.license.clone(),
        client_side: hit.client_side.clone(),
        server_side: hit.server_side.clone(),
        icon_url: hit.icon_url.clone(),
        neoforge_build_exists: tagged(Loader::NeoForge),
        target_build_exists: tagged(direction.target),
        loaders,
        files: files.into_iter().map(ListFile::from).collect(),
    }
}

/// Fetches the list: search pages of 100, sorted by downloads, until `top` mods with a usable
/// file are found or the results run out. One version request per project.
pub fn fetch_list(
    fetch: &dyn Fetch,
    opts: &ListOptions,
    fetched_at: String,
    mut progress: impl FnMut(&str),
) -> Result<BoardList, Error> {
    if opts.top == 0 || opts.top > MAX_TOP {
        return Err(Error::invalid(format!(
            "--top must be between 1 and {MAX_TOP}, got {}",
            opts.top
        )));
    }
    let mut game_versions = opts.game_versions.clone();
    gamever::sort_versions(&mut game_versions);
    if game_versions.is_empty() {
        return Err(Error::invalid("at least one game version is required"));
    }
    let source = opts.direction.source;
    let mut mods = Vec::new();
    let mut skipped = Vec::new();
    let mut seen = BTreeSet::new();
    let mut offset: u64 = 0;
    let target = usize::try_from(opts.top).unwrap_or(usize::MAX);
    'pages: loop {
        let url = modrinth::search_url(&opts.api_base, source, &game_versions, offset);
        let page = modrinth::parse_search(&fetch.get(&url, MAX_API_BODY)?)?;
        progress(&format!(
            "search offset {offset}: {} hits of {}",
            page.hits.len(),
            page.total_hits
        ));
        if page.hits.is_empty() {
            break;
        }
        for hit in &page.hits {
            if mods.len() >= target {
                break 'pages;
            }
            if !modrinth::is_modrinth_id(&hit.project_id) || !seen.insert(hit.project_id.clone()) {
                continue;
            }
            let url =
                modrinth::versions_url(&opts.api_base, &hit.project_id, source, &game_versions)?;
            let versions = modrinth::parse_versions(&fetch.get(&url, MAX_API_BODY)?)?;
            let files = modrinth::pick_files(&versions, source, &game_versions);
            if files.is_empty() {
                skipped.push(ListSkip {
                    project_id: hit.project_id.clone(),
                    slug: hit.slug.clone(),
                    reason: format!(
                        "no {} release or beta JAR on Modrinth for {}",
                        source.name(),
                        game_versions.join(" or ")
                    ),
                });
                continue;
            }
            mods.push(list_mod(hit, opts.direction, files));
        }
        offset += page.hits.len() as u64;
        if mods.len() >= target || offset >= page.total_hits {
            break;
        }
    }
    let list = BoardList {
        schema: SCHEMA,
        fetched_at,
        direction: opts.direction,
        source_loader: opts.direction.source,
        target_loader: opts.direction.target,
        game_versions,
        top: opts.top,
        mods,
        skipped,
    };
    list.normalized()
}

impl BoardList {
    /// Sorts mods (downloads, most first, then project id) and skips (project id), and
    /// checks the list against the contract.
    pub fn normalized(mut self) -> Result<Self, Error> {
        self.mods.sort_by(|a, b| {
            b.downloads
                .cmp(&a.downloads)
                .then_with(|| a.project_id.cmp(&b.project_id))
        });
        self.skipped.sort_by(|a, b| a.project_id.cmp(&b.project_id));
        self.validate()?;
        Ok(self)
    }

    /// Checks the rules of `docs/board.md` that a reader relies on.
    pub fn validate(&self) -> Result<(), Error> {
        let bad = |msg: String| Err(Error::invalid(format!("board-list.json: {msg}")));
        if self.schema != SCHEMA {
            return bad(format!("schema must be {SCHEMA}, got {}", self.schema));
        }
        if !time::is_rfc3339_utc(&self.fetched_at) {
            return bad(format!(
                "fetched_at {:?} is not an RFC 3339 UTC time",
                self.fetched_at
            ));
        }
        if self.source_loader != self.direction.source
            || self.target_loader != self.direction.target
        {
            return bad("source_loader and target_loader must match direction".to_owned());
        }
        if self.game_versions.is_empty()
            || !self.game_versions.iter().all(|v| gamever::is_release(v))
        {
            return bad("game_versions must be a non-empty list of release versions".to_owned());
        }
        let mut ids = BTreeSet::new();
        for m in &self.mods {
            if !modrinth::is_modrinth_id(&m.project_id) {
                return bad(format!("{:?} is not a Modrinth project id", m.project_id));
            }
            if !ids.insert(m.project_id.as_str()) {
                return bad(format!("project {} is listed twice", m.project_id));
            }
            if m.files.is_empty() {
                return bad(format!("project {} has no files", m.project_id));
            }
            let mut versions = BTreeSet::new();
            for f in &m.files {
                if !self.game_versions.contains(&f.game_version) {
                    return bad(format!(
                        "project {} has a file for unpinned version {}",
                        m.project_id, f.game_version
                    ));
                }
                if !versions.insert(f.game_version.as_str()) {
                    return bad(format!(
                        "project {} has two files for {}",
                        m.project_id, f.game_version
                    ));
                }
                if !crate::hash::is_hex(&f.sha512, 128) {
                    return bad(format!("project {} has a malformed sha512", m.project_id));
                }
                if !f.url.starts_with(modrinth::CDN_PREFIX) {
                    return bad(format!(
                        "project {} has a file outside {}",
                        m.project_id,
                        modrinth::CDN_PREFIX
                    ));
                }
                if !modrinth::is_modrinth_id(&f.version_id) {
                    return bad(format!(
                        "project {} has a malformed version id",
                        m.project_id
                    ));
                }
            }
        }
        Ok(())
    }

    /// Parses and validates `board-list.json`.
    pub fn from_json(bytes: &[u8]) -> Result<Self, Error> {
        let list: BoardList =
            serde_json::from_slice(bytes).map_err(|e| Error::json("board-list.json", e))?;
        list.validate()?;
        Ok(list)
    }

    /// The list as pretty JSON with a trailing newline.
    pub fn to_json(&self) -> Result<Vec<u8>, Error> {
        let mut out =
            serde_json::to_vec_pretty(self).map_err(|e| Error::json("board-list.json", e))?;
        out.push(b'\n');
        Ok(out)
    }
}
