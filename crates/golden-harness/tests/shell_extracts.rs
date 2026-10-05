//! Position Details lives in features/; Add Lot / Holdings folders are extract targets
//! (markup may still mount from App until those binders cut over).

#[test]
fn position_details_lives_in_feature_module() {
    let root = golden_harness::repo_root();
    for rel in [
        "apps/desktop/src/features/add-lot/AddLotScreen.tsx",
        "apps/desktop/src/features/add-lot/index.ts",
        "apps/desktop/src/features/holdings/HoldingsScreen.tsx",
        "apps/desktop/src/features/holdings/index.ts",
        "apps/desktop/src/features/position-details/PositionDetailsScreen.tsx",
        "apps/desktop/src/features/position-details/index.ts",
    ] {
        assert!(root.join(rel).is_file(), "missing {rel}");
    }
    let app = std::fs::read_to_string(root.join("apps/desktop/src/App.tsx")).unwrap();
    let pd = std::fs::read_to_string(
        root.join("apps/desktop/src/features/position-details/PositionDetailsScreen.tsx"),
    )
    .unwrap();
    assert!(
        app.contains("from \"./features/position-details\"")
            && app.contains("<PositionDetailsScreen"),
        "App.tsx mounts Position Details feature module"
    );
    assert!(
        !app.contains("aria-label=\"Position Details\"")
            && !app.contains("<h2>Position Details</h2>"),
        "Position Details markup must live in features/position-details/, not App.tsx"
    );
    assert!(
        pd.contains("aria-label=\"Position Details\"") && pd.contains("<h2>Position Details</h2>"),
        "Position Details screen owns the hub heading"
    );
    let catalog = std::fs::read_to_string(root.join("docs/architecture/ui-modules.json")).unwrap();
    assert!(
        catalog.contains("apps/desktop/src/features/position-details/")
            && catalog.contains("\"id\": \"position-details\""),
        "catalog lists Position Details folder"
    );
    let pd_mod = catalog
        .split("\"id\": \"position-details\"")
        .nth(1)
        .and_then(|s| s.split("\"id\":").next())
        .unwrap_or("");
    assert!(
        pd_mod.contains("\"status\": \"extracted\""),
        "catalog marks position-details as extracted"
    );
}
