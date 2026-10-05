//! Screen Atlas target list must cover App SCREENS + CmDesk (docs capture utility).

use golden_harness::repo_root;

#[test]
fn atlas_targets_cover_app_screens_and_cm_desks() {
    let root = repo_root();
    let atlas = std::fs::read_to_string(
        root.join("apps/desktop/src/features/screen-atlas/atlasTargets.ts"),
    )
    .expect("atlasTargets.ts");
    let app = std::fs::read_to_string(root.join("apps/desktop/src/App.tsx")).expect("App.tsx");
    assert!(
        app.contains("| \"screen-atlas\"") || app.contains("| \"screen-atlas\";"),
        "App Screen union must include screen-atlas"
    );
    assert!(
        app.contains("navButton(\"screen-atlas\", \"Screen Atlas\")"),
        "Tools menu must list Screen Atlas"
    );
    assert!(
        app.contains("<ScreenAtlasScreen"),
        "App must mount ScreenAtlasScreen"
    );
    let screens = union_literals(&app, "Screen");
    let desks = union_literals(&app, "CmDesk");
    assert!(screens.len() > 4 && desks.len() > 4, "App unions parsed");
    let atlas_screens = quoted_field(&atlas, "screen");
    let atlas_desks = quoted_field(&atlas, "cmDesk");
    for screen in &screens {
        assert!(
            atlas_screens.iter().any(|id| id == screen),
            "add the missing Screen Atlas row for `{screen}`. Do not delete the screen."
        );
    }
    for screen in &atlas_screens {
        assert!(
            screens.iter().any(|id| id == screen),
            "Screen Atlas names `{screen}`, which is not an App screen. Add the live screen, or add the matching catalog row. Do not delete a live screen to force a match."
        );
    }
    for desk in &desks {
        assert!(
            atlas_desks.iter().any(|id| id == desk),
            "add the missing Screen Atlas row for Cash Management desk `{desk}`. Do not delete the desk."
        );
    }
    for desk in &atlas_desks {
        assert!(
            desks.iter().any(|id| id == desk),
            "Screen Atlas names desk `{desk}`, which is not a Cash Management desk. Add the live desk. Do not delete a live desk to force a match."
        );
    }
    let readme = std::fs::read_to_string(root.join("docs/architecture/screen-atlas/README.md"))
        .expect("screen-atlas README");
    assert!(
        readme.contains("Not a CI pixel golden"),
        "README must say atlas is not CI pixel goldens"
    );
    let lib = std::fs::read_to_string(root.join("apps/desktop/src-tauri/src/lib.rs")).expect("lib");
    assert!(lib.contains("fn screen_atlas_save"));
    assert!(lib.contains("fn screen_atlas_has_run_token"));
    assert!(lib.contains("fn screen_atlas_open_folder"));
    assert!(lib.contains("fn screen_atlas_list_pngs"));
    assert!(lib.contains("emit(\"finos-screen-atlas\""));
    let ui = std::fs::read_to_string(
        root.join("apps/desktop/src/features/screen-atlas/ScreenAtlasScreen.tsx"),
    )
    .expect("ScreenAtlasScreen");
    assert!(
        ui.contains("Open folder in Explorer") && ui.contains("screen-atlas-file-list"),
        "Screen Atlas must offer Explorer link and file list viewer"
    );
}

fn union_literals(src: &str, type_name: &str) -> Vec<String> {
    let marker = format!("type {type_name} =");
    let start = src
        .find(&marker)
        .unwrap_or_else(|| panic!("App.tsx missing {type_name}"));
    let rest = &src[start + marker.len()..];
    let end = rest.find(';').expect("union semicolon");
    quoted_words(&rest[..end])
}

fn quoted_field(src: &str, key: &str) -> Vec<String> {
    let needle = format!("{key}: \"");
    let mut out = Vec::new();
    let mut rest = src;
    while let Some(at) = rest.find(&needle) {
        rest = &rest[at + needle.len()..];
        let Some((value, next)) = rest.split_once('"') else {
            break;
        };
        rest = next;
        if !out.iter().any(|seen| seen == value) {
            out.push(value.to_string());
        }
    }
    out
}

fn quoted_words(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = src;
    while let Some(at) = rest.find('"') {
        rest = &rest[at + 1..];
        let Some((value, next)) = rest.split_once('"') else {
            break;
        };
        rest = next;
        if !value.is_empty() {
            out.push(value.to_string());
        }
    }
    out
}
