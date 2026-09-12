//! Catalog of owner-facing functions. Fail if a listed sentinel disappears.

use golden_harness::repo_root;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Sentinel {
    path: String,
    must_contain: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Item {
    id: String,
    menu_area: String,
    function: String,
    last_changed: String,
    last_verified: String,
    #[serde(default)]
    also_verify: Vec<String>,
    sentinels: Vec<Sentinel>,
}

#[derive(Debug, Deserialize)]
struct Catalog {
    items: Vec<Item>,
}

fn iso_day(value: &str) -> bool {
    let b = value.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter().enumerate().all(|(i, c)| {
            if i == 4 || i == 7 {
                true
            } else {
                c.is_ascii_digit()
            }
        })
}

#[test]
fn core_functions_catalog_sentinels_still_exist() {
    let root = repo_root();
    let raw = std::fs::read_to_string(root.join("docs/architecture/core-functions.json"))
        .expect("core-functions.json");
    let catalog: Catalog = serde_json::from_str(&raw).expect("parse catalog");
    assert!(
        !catalog.items.is_empty(),
        "core-functions.json must list at least one function"
    );
    for item in &catalog.items {
        assert!(!item.id.trim().is_empty(), "id required");
        assert!(!item.menu_area.trim().is_empty(), "{}: menuArea", item.id);
        assert!(!item.function.trim().is_empty(), "{}: function", item.id);
        assert!(
            iso_day(&item.last_changed),
            "{}: lastChanged must be YYYY-MM-DD, got {}",
            item.id,
            item.last_changed
        );
        assert!(
            iso_day(&item.last_verified),
            "{}: lastVerified must be YYYY-MM-DD, got {}",
            item.id,
            item.last_verified
        );
        assert!(
            !item.sentinels.is_empty(),
            "{}: at least one sentinel",
            item.id
        );
        for name in &item.also_verify {
            let test = root
                .join("crates/golden-harness/tests")
                .join(format!("{name}.rs"));
            assert!(
                test.is_file(),
                "{}: alsoVerify {name} must be crates/golden-harness/tests/{name}.rs",
                item.id
            );
        }
        for sentinel in &item.sentinels {
            let path = root.join(&sentinel.path);
            let body = std::fs::read_to_string(&path).unwrap_or_else(|e| {
                panic!("{}: read {}: {e}", item.id, path.display())
            });
            assert!(
                body.contains(&sentinel.must_contain),
                "{}: {} must contain {:?}",
                item.id,
                sentinel.path,
                sentinel.must_contain
            );
        }
    }
    let restart = catalog
        .items
        .iter()
        .find(|i| i.id == "file-restart-graceful")
        .expect("catalog must list file-restart-graceful");
    assert!(
        restart.function.contains("restart.token") && restart.function.contains("supervisor"),
        "file-restart-graceful must name restart.token and supervisor"
    );
    assert!(
        restart
            .sentinels
            .iter()
            .all(|s| !s.must_contain.contains("schtasks")),
        "file-restart-graceful sentinels must not lock schtasks"
    );
    for id in [
        "file-restart-graceful",
        "cash-management-post",
        "cash-management-ssa",
        "cash-management-magi",
        "cash-management-month",
        "settings-core-functions",
        "home-live-by-risk",
        "home-risk-symbol-popup",
        "home-graphing-period",
        "home-open-tickets",
        "collectors-stats-truth",
        "position-details-owner-facts",
        "save-unsaved-orange",
    ] {
        assert!(
            catalog.items.iter().any(|i| i.id == id),
            "catalog must list {id}"
        );
    }
}
