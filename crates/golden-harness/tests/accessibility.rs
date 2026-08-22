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
        "aria-label=\"Import Fidelity or Schwab CSV\"",
        "aria-label=\"Check for updates\"",
        "aria-label=\"Exceptions\"",
        "aria-label=\"Income Plan\"",
        "aria-label=\"Calculator\"",
        "aria-label=\"Position Details\"",
        "aria-label=\"Dashboard\"",
        "aria-label=\"Holdings\"",
        "aria-label=\"New Investment\"",
        "aria-label=\"Add Lot\"",
        "aria-label=\"Import\"",
        "aria-label=\"Settings\"",
        "aria-label=\"Assign lot\"",
        "aria-label=\"Retrieve from market\"",
        "aria-label=\"Mandatory data checklist\"",
        "aria-label=\"Use Most Current as Plan\"",
        "aria-label=\"Save new investment facts\"",
        "aria-label=\"Save stored facts\"",
        "aria-label=\"Confirm Plan\"",
        "aria-label=\"Confirm stored Plan\"",
        "aria-label=\"Open first lot\"",
        "aria-label=\"Record last price\"",
        "aria-label=\"Retrieve declarations\"",
        "aria-label=\"Validate import\"",
        "aria-label=\"Approve import\"",
        "aria-label=\"Post import\"",
        "aria-label=\"Previous week\"",
        "aria-label=\"Next week\"",
        "aria-label=\"Filter holdings\"",
    ] {
        assert!(app.contains(name), "missing accessible name {name}");
    }
    assert!(
        !app.contains("Load household seed"),
        "owner UI must not mention household seed"
    );
    assert!(
        !app.contains("ProductionSeedLoad"),
        "owner UI must not call ProductionSeedLoad"
    );
}
