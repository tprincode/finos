//! Position Details, Add Lot, and Holdings mount from features/.
//! An extracted catalog folder or a screen file with no importer fails.

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
        "apps/desktop/src/features/new-investment/NewInvestmentScreen.tsx",
        "apps/desktop/src/features/new-investment/index.ts",
        "apps/desktop/src/features/home/HomeScreen.tsx",
        "apps/desktop/src/features/home/index.ts",
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
    let wizard = std::fs::read_to_string(
        root.join("apps/desktop/src/features/new-investment/NewInvestmentScreen.tsx"),
    )
    .unwrap();
    assert!(
        app.contains("from \"./features/new-investment\"") && app.contains("<NewInvestmentScreen"),
        "App.tsx mounts Add Investment"
    );
    assert!(
        !app.contains("<h2>Add Investment</h2>") && !app.contains("<ReadinessChecklist"),
        "Add Investment markup must live in features/new-investment/, not App.tsx"
    );
    assert!(
        wizard.contains("<h2>Add Investment</h2>")
            && wizard.contains("aria-label=\"Add Investment\"")
            && wizard.contains("<ReadinessChecklist"),
        "Add Investment screen owns the wizard heading and the readiness checklist"
    );
    let catalog = std::fs::read_to_string(root.join("docs/architecture/ui-modules.json")).unwrap();
    assert!(
        app.contains("from \"./features/home\"") && app.contains("<HomeScreen"),
        "App.tsx mounts Home feature module"
    );
    assert!(
        !app.contains("aria-label=\"Portfolio summary\""),
        "Home markup must live in features/home/, not App.tsx"
    );
    let home = std::fs::read_to_string(
        root.join("apps/desktop/src/features/home/HomeScreen.tsx"),
    )
    .unwrap();
    assert!(
        home.contains("aria-label=\"Portfolio summary\"") && home.contains("<HomePaintMark"),
        "HomeScreen owns the portfolio board"
    );
    assert!(
        catalog.contains("apps/desktop/src/features/position-details/")
            && catalog.contains("\"id\": \"position-details\""),
        "catalog lists Position Details folder"
    );
    let home_mod = catalog
        .split("\"id\": \"home\"")
        .nth(1)
        .and_then(|s| s.split("\"id\":").next())
        .unwrap_or("");
    assert!(
        home_mod.contains("\"status\": \"extracted\"")
            && home_mod.contains("apps/desktop/src/features/home/"),
        "catalog marks home as extracted"
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

fn slash(path: &std::path::Path) -> String {
    let mut parts = Vec::new();
    for part in path.components() {
        match part {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                parts.pop();
            }
            other => parts.push(other.as_os_str().to_string_lossy().into_owned()),
        }
    }
    parts.join("/")
}

fn relative_imports(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    for quote in ['"', '\''] {
        let needle = format!("from {quote}");
        let mut rest = src;
        while let Some(at) = rest.find(&needle) {
            rest = &rest[at + needle.len()..];
            let Some(end) = rest.find(quote) else { break };
            let spec = &rest[..end];
            if spec.starts_with('.') {
                out.push(spec.to_string());
            }
            rest = &rest[end..];
        }
    }
    out
}

fn resolve_import(from: &std::path::Path, spec: &str) -> Option<std::path::PathBuf> {
    let raw = from.parent()?.join(spec);
    [
        raw.with_extension("tsx"),
        raw.with_extension("ts"),
        raw.join("index.tsx"),
        raw.join("index.ts"),
    ]
    .into_iter()
    .find(|path| path.is_file())
}

fn reachable_from(start: &std::path::Path) -> std::collections::HashSet<String> {
    let mut seen = std::collections::HashSet::new();
    let mut queue = vec![start.to_path_buf()];
    while let Some(file) = queue.pop() {
        let key = slash(&file);
        if !seen.insert(key) {
            continue;
        }
        let Ok(src) = std::fs::read_to_string(&file) else {
            continue;
        };
        for spec in relative_imports(&src) {
            if let Some(next) = resolve_import(&file, &spec) {
                queue.push(next);
            }
        }
    }
    seen
}

fn screens_under(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            screens_under(&path, out);
        } else if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with("Screen.tsx"))
        {
            out.push(path);
        }
    }
}

#[test]
fn extracted_modules_and_screen_files_are_imported() {
    let root = golden_harness::repo_root();
    let app_path = root.join("apps/desktop/src/App.tsx");
    let app = std::fs::read_to_string(&app_path).unwrap();
    assert!(
        app.contains("<HoldingsScreen") && app.contains("<AddLotScreen"),
        "Holdings and Add Lot open the extracted screens"
    );
    assert!(
        !app.contains("<h2>Holdings</h2>") && !app.contains("<h2>Add Lot</h2>"),
        "Holdings and Add Lot markup stays in the feature files"
    );
    let reached = reachable_from(&app_path);
    let mut screens = Vec::new();
    screens_under(&root.join("apps/desktop/src/features"), &mut screens);
    for screen in &screens {
        assert!(
            reached.contains(&slash(screen)),
            "screen file has no importer: {}",
            slash(screen)
        );
    }
    let catalog: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(root.join("docs/architecture/ui-modules.json")).unwrap(),
    )
    .unwrap();
    for module in catalog["modules"].as_array().unwrap() {
        if module["status"] != "extracted" {
            continue;
        }
        let folder = module["folder"].as_str().unwrap().trim_end_matches('/');
        if !folder.starts_with("apps/desktop/src/features/") {
            continue;
        }
        let prefix = folder.trim_end_matches(".tsx").trim_end_matches(".ts");
        assert!(
            reached.iter().any(|path| path.contains(prefix) || path.contains(folder)),
            "extracted {} is not imported from App",
            module["id"].as_str().unwrap_or(folder)
        );
    }
}
