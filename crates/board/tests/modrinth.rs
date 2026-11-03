//! Modrinth parsing and file picking against hand-written JSON fixtures, and the list fetch
//! against an offline fake of the API.

#![allow(
    clippy::unwrap_used,
    reason = "test helpers: a failed unwrap is a failed test"
)]

mod common;

use common::{FakeFetch, fixture, gv};
use jarrow_board::list::{BoardList, ListOptions, fetch_list};
use jarrow_board::modrinth::{self, API_BASE, pick_file, pick_files};
use jarrow_board::{Direction, Loader};

const PINNED: &[&str] = &["1.21.1", "1.21.11"];

fn fabric_neoforge() -> Direction {
    "fabric-neoforge".parse().unwrap()
}

#[test]
fn search_pages_parse() {
    let page = modrinth::parse_search(&fixture("search-page-1.json")).unwrap();
    assert_eq!(page.total_hits, 4);
    assert_eq!(page.offset, 0);
    assert_eq!(page.limit, 100);
    assert_eq!(page.hits.len(), 3);
    let alpha = &page.hits[0];
    assert_eq!(alpha.project_id, "AAAAAAA1");
    assert_eq!(alpha.slug, "alpha-lib");
    assert_eq!(alpha.downloads, 5000);
    assert_eq!(alpha.license.as_deref(), Some("MIT"));
    assert_eq!(
        modrinth::loader_tags(&alpha.categories),
        gv(&["fabric", "neoforge"])
    );
    // A null icon and missing optional fields are fine.
    assert_eq!(page.hits[1].icon_url, None);
    assert_eq!(page.hits[2].project_type.as_deref(), Some("mod"));
}

#[test]
fn malformed_pages_are_errors_not_panics() {
    for body in [
        &b""[..],
        b"[]",
        b"{\"hits\": 3}",
        b"{\"hits\":[{\"project_id\":\"X\"}],\"offset\":0,\"limit\":1,\"total_hits\":1}",
        b"\xff\xfe",
    ] {
        assert!(modrinth::parse_search(body).is_err());
    }
    assert!(modrinth::parse_versions(b"{}").is_err());
    assert!(modrinth::parse_versions(b"[{\"id\":\"A\"}]").is_err());
}

#[test]
fn release_beats_beta_and_the_newest_release_wins() {
    let versions = modrinth::parse_versions(&fixture("versions-alpha.json")).unwrap();
    // 1.21.1: two releases published in the same second; the later fraction wins.
    let picked = pick_file(&versions, Loader::Fabric, "1.21.1").unwrap();
    assert_eq!(picked.version_id, "AV000002");
    assert_eq!(picked.version_type, "release");
    assert_eq!(picked.size, 1002);
    // 1.21.11: the only Fabric release is NeoForge-only, the alpha is never picked, so the
    // beta wins. Its upper-case sha512 is normalised.
    let picked = pick_file(&versions, Loader::Fabric, "1.21.11").unwrap();
    assert_eq!(picked.version_id, "AV000003");
    assert_eq!(picked.version_type, "beta");
    assert_eq!(picked.sha512, picked.sha512.to_ascii_lowercase());
    // The NeoForge release is found when NeoForge is the source loader.
    let picked = pick_file(&versions, Loader::NeoForge, "1.21.11").unwrap();
    assert_eq!(picked.version_id, "AV000005");
    // Quilt-tagged versions count for Quilt.
    assert_eq!(
        pick_file(&versions, Loader::Quilt, "1.21.11")
            .unwrap()
            .version_id,
        "AV000003"
    );
    assert!(pick_file(&versions, Loader::Forge, "1.21.1").is_none());
    assert!(pick_file(&versions, Loader::Fabric, "1.20.1").is_none());
}

#[test]
fn primary_cdn_jars_only() {
    let versions = modrinth::parse_versions(&fixture("versions-beta.json")).unwrap();
    // The newest release points outside Modrinth's CDN, so the older one is used, and its
    // primary file, not the sources JAR listed first.
    let picked = pick_file(&versions, Loader::Fabric, "1.21.11").unwrap();
    assert_eq!(picked.version_id, "BV000002");
    assert_eq!(picked.filename, "beta-tweaks-1.4.2.jar");
    assert!(picked.url.starts_with(modrinth::CDN_PREFIX));
}

#[test]
fn one_version_can_serve_several_pins() {
    let versions = modrinth::parse_versions(&fixture("versions-delta.json")).unwrap();
    let files = pick_files(&versions, Loader::Fabric, &gv(&["1.21.11", "1.21.1"]));
    let got: Vec<(&str, &str)> = files
        .iter()
        .map(|f| (f.game_version.as_str(), f.version_id.as_str()))
        .collect();
    assert_eq!(got, vec![("1.21.1", "DV000001"), ("1.21.11", "DV000001")]);
}

fn fake_api() -> FakeFetch {
    let pins = gv(PINNED);
    let page = |offset| modrinth::search_url(API_BASE, Loader::Fabric, &pins, offset);
    let versions = |id: &str| modrinth::versions_url(API_BASE, id, Loader::Fabric, &pins).unwrap();
    FakeFetch::default()
        .with(&page(0), fixture("search-page-1.json"))
        .with(&page(3), fixture("search-page-2.json"))
        .with(&versions("AAAAAAA1"), fixture("versions-alpha.json"))
        .with(&versions("BBBBBBB2"), fixture("versions-beta.json"))
        .with(&versions("CCCCCCC3"), fixture("versions-forge-only.json"))
        .with(&versions("DDDDDDD4"), fixture("versions-delta.json"))
}

fn options(top: u32) -> ListOptions {
    ListOptions {
        direction: fabric_neoforge(),
        game_versions: gv(PINNED),
        top,
        api_base: API_BASE.to_owned(),
    }
}

#[test]
fn the_list_pages_skips_and_stops_at_top() {
    let api = fake_api();
    let list = fetch_list(&api, &options(1000), "2026-11-03T14:00:00Z".into(), |_| {}).unwrap();
    let ids: Vec<&str> = list.mods.iter().map(|m| m.project_id.as_str()).collect();
    assert_eq!(ids, vec!["AAAAAAA1", "BBBBBBB2", "DDDDDDD4"]);
    assert_eq!(list.skipped.len(), 1);
    assert_eq!(list.skipped[0].project_id, "CCCCCCC3");
    assert!(list.skipped[0].reason.contains("no Fabric release or beta"));
    assert_eq!(list.direction, fabric_neoforge());
    assert_eq!(list.source_loader, Loader::Fabric);
    assert_eq!(list.target_loader, Loader::NeoForge);
    assert_eq!(list.game_versions, gv(PINNED));

    let alpha = &list.mods[0];
    assert!(alpha.neoforge_build_exists);
    assert!(alpha.target_build_exists);
    assert_eq!(alpha.files.len(), 2);
    assert_eq!(alpha.files[0].game_version, "1.21.1");
    assert_eq!(alpha.files[1].game_version, "1.21.11");
    let beta = &list.mods[1];
    assert!(!beta.neoforge_build_exists);
    assert_eq!(beta.loaders, gv(&["fabric", "quilt"]));
    assert_eq!(beta.files.len(), 1);

    // Two search pages and one version request per project; nothing else.
    let calls = api.calls.borrow();
    assert_eq!(calls.iter().filter(|u| u.contains("/search?")).count(), 2);
    assert_eq!(calls.iter().filter(|u| u.contains("/version?")).count(), 4);
    assert!(
        calls
            .iter()
            .all(|u| u.contains("include_changelog=false") || u.contains("/search?"))
    );

    // The output validates and round-trips.
    let json = list.to_json().unwrap();
    assert_eq!(BoardList::from_json(&json).unwrap(), list);
}

#[test]
fn top_counts_only_mods_with_files() {
    let api = fake_api();
    let list = fetch_list(&api, &options(2), "2026-11-03T14:00:00Z".into(), |_| {}).unwrap();
    let ids: Vec<&str> = list.mods.iter().map(|m| m.project_id.as_str()).collect();
    assert_eq!(ids, vec!["AAAAAAA1", "BBBBBBB2"]);
    // It stopped before asking for the third project or the second page.
    assert_eq!(api.calls.borrow().len(), 3);
}

#[test]
fn list_errors() {
    let api = fake_api();
    assert!(fetch_list(&api, &options(0), "2026-11-03T14:00:00Z".into(), |_| {}).is_err());
    let missing = FakeFetch::default();
    let err = fetch_list(&missing, &options(5), "2026-11-03T14:00:00Z".into(), |_| {}).unwrap_err();
    assert!(err.to_string().contains("HTTP 404"), "{err}");
}

#[test]
fn list_validation_rejects_contract_breaks() {
    let api = fake_api();
    let list = fetch_list(&api, &options(1000), "2026-11-03T14:00:00Z".into(), |_| {}).unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&list.to_json().unwrap()).unwrap();
    value["mods"][0]["files"][0]["url"] = "https://example.invalid/x.jar".into();
    assert!(BoardList::from_json(value.to_string().as_bytes()).is_err());
    let mut value: serde_json::Value = serde_json::from_slice(&list.to_json().unwrap()).unwrap();
    value["mods"][1]["project_id"] = "AAAAAAA1".into();
    assert!(BoardList::from_json(value.to_string().as_bytes()).is_err());
    let mut value: serde_json::Value = serde_json::from_slice(&list.to_json().unwrap()).unwrap();
    value["unexpected"] = 1.into();
    assert!(BoardList::from_json(value.to_string().as_bytes()).is_err());
    let mut value: serde_json::Value = serde_json::from_slice(&list.to_json().unwrap()).unwrap();
    value["target_loader"] = "forge".into();
    assert!(BoardList::from_json(value.to_string().as_bytes()).is_err());
}

#[test]
fn the_source_loader_decides_the_facets() {
    let pins = gv(&["1.20.1"]);
    for loader in Loader::ALL {
        let url = modrinth::search_url(API_BASE, loader, &pins, 0);
        assert!(
            url.contains(&format!("categories%3A{}", loader.id())),
            "{url}"
        );
    }
}
