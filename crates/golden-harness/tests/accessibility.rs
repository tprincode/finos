use golden_harness::repo_root;

#[test]
fn accessibility_primary_actions_have_accessible_names() {
    let app = std::fs::read_to_string(repo_root().join("apps/desktop/src/App.tsx"))
        .expect("App.tsx");
    for name in [
        "aria-label=\"finos\"",
        "aria-label=\"Save device name\"",
        "aria-label=\"Create snapshot\"",
        "aria-label=\"Restore published\"",
        "aria-label=\"Acknowledge review\"",
        "aria-label=\"Exit\"",
        "aria-label=\"Register sample account\"",
        "aria-label=\"Import sample Fidelity dividend\"",
        "aria-label=\"Check for updates\"",
    ] {
        assert!(app.contains(name), "missing accessible name {name}");
    }
}
