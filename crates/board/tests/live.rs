//! Live checks against the real Modrinth API. They need the network, so they are ignored by
//! default. Run them with:
//!
//! ```text
//! cargo test -p jarrow-board --test live -- --ignored
//! ```

use jarrow_board::http::HttpClient;
use jarrow_board::list::{ListOptions, fetch_list};
use jarrow_board::modrinth::API_BASE;
use jarrow_board::time::{Clock, SystemClock};

/// Fetches the top 3 Fabric mods for the pinned versions and checks the list's shape.
///
/// `cargo test -p jarrow-board --test live -- --ignored`
#[test]
#[ignore = "needs the network: cargo test -p jarrow-board --test live -- --ignored"]
fn modrinth_top_three() {
    let client = HttpClient::new().unwrap();
    let opts = ListOptions {
        direction: "fabric-neoforge".parse().unwrap(),
        game_versions: vec!["1.21.1".to_owned(), "1.21.11".to_owned()],
        top: 3,
        api_base: API_BASE.to_owned(),
    };
    let list = fetch_list(&client, &opts, SystemClock.now(), |line| {
        eprintln!("{line}")
    })
    .unwrap();
    assert_eq!(list.mods.len(), 3);
    for m in &list.mods {
        assert!(!m.files.is_empty());
        assert!(m.loaders.iter().any(|l| l == "fabric"));
    }
    list.validate().unwrap();
}
