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
    #[serde(default)]
    notes: Vec<String>,
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
        catalog.items.len() >= 28,
        "catalog row count must not fall below the 11 Sep 2026 count of 28, got {}",
        catalog.items.len()
    );
    assert!(
        catalog
            .notes
            .iter()
            .any(|n| n.contains("Export") && n.contains("Cash Management")),
        "catalog must name the Export vs Cash Management drift"
    );
    for item in &catalog.items {
        if matches!(
            item.id.as_str(),
            "home-open-tickets"
                | "home-refresh-declarations"
                | "home-dividend-plan-panel"
                | "home-live-by-risk"
                | "file-restart-graceful"
                | "settings-core-functions"
                | "shopping-cart-swap"
                | "save-unsaved-orange"
        ) {
            assert!(
                !item.also_verify.is_empty(),
                "{}: alsoVerify must stay non-empty",
                item.id
            );
        }
    }
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
        let verified_ok = iso_day(&item.last_verified)
            || (item.id == "file-restart-graceful"
                && item.last_verified == "UNVERIFIED-owner-windows-installed");
        assert!(
            verified_ok,
            "{}: lastVerified must be YYYY-MM-DD (or UNVERIFIED-owner-windows-installed for file-restart-graceful), got {}",
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
        restart.function.contains("Installed release")
            && restart.function.contains("current_exe")
            && restart.function.contains("does not rely on app.restart() alone"),
        "file-restart-graceful must lock the installed-release spawn-before-exit contract"
    );
    assert!(
        restart.function.contains("restart-owner-gate"),
        "file-restart-graceful must name the restart-owner-gate attestation contract"
    );
    assert!(
        restart.last_verified == "UNVERIFIED-owner-windows-installed"
            || {
                let attest = std::fs::read_to_string(
                    root.join("docs/architecture/restart-owner-attestation.md"),
                )
                .unwrap_or_default();
                let status = attest.split("## STATUS").nth(1).unwrap_or("");
                status.contains("OWNER_CONFIRMED_INSTALLED_RESTART: true")
            },
        "lastVerified must stay UNVERIFIED-owner-windows-installed until owner STATUS attestation confirms installed Restart"
    );
    assert!(
        !restart.function.to_ascii_lowercase().contains("household"),
        "file-restart-graceful must not use household slang"
    );
    assert!(
        restart
            .sentinels
            .iter()
            .any(|s| s.must_contain.contains(
                "installed_release_restart_must_spawn_exe_before_exit"
            )),
        "file-restart-graceful must sentinel the desktop_menu hard gate"
    );
    assert!(
        restart
            .sentinels
            .iter()
            .all(|s| !s.must_contain.contains("schtasks")),
        "file-restart-graceful sentinels must not lock schtasks"
    );
    let contracts = std::fs::read_to_string(root.join("docs/architecture/component-contracts.md"))
        .expect("component-contracts.md");
    assert!(
        !contracts.contains("implementations land in later milestones"),
        "contracts must not say implementations are still future work"
    );
    assert!(
        !contracts.contains("parked until the owner names SC-1"),
        "Shopping Cart scenario commands are shipped"
    );
    for name in [
        "CashDistributionPost",
        "SsaConfirm",
        "CashAdjustPost",
        "WeekCaptureAccept",
        "CashManagementWeekGet",
        "CashManagementRemindersGet",
        "CashManagementMonthGet",
        "TrendsWeekSave",
        "IncomePlanWeekGet",
        "IncomePlanGridGet",
        "IncomePlanExportGet",
        "DashboardBurndownGet",
        "LastPriceAutoWindowGet",
        "CashPileGet",
        "WorkTicketList",
        "DividendPerformanceGet",
    ] {
        assert!(
            contracts.contains(name),
            "component-contracts.md must name {name}"
        );
    }
    assert!(
        contracts.contains("one `Canonical` port") || contracts.contains("one Canonical port"),
        "contracts must restate isolation as a port/review rule"
    );

    for id in [
        "file-restart-graceful",
        "cash-management-post",
        "cash-management-ssa",
        "cash-management-magi",
        "cash-management-month",
        "cash-management-import",
        "settings-core-functions",
        "home-live-by-risk",
        "home-risk-symbol-popup",
        "home-graphing-period",
        "home-weekly-actuals",
        "shopping-cart-swap",
        "home-open-tickets",
        "home-dividend-plan-panel",
        "collectors-stats-truth",
        "position-details-owner-facts",
        "save-unsaved-orange",
        "last-price-auto-window",
        "tools-component-registry",
    ] {
        assert!(
            catalog.items.iter().any(|i| i.id == id),
            "catalog must list {id}"
        );
    }
}
