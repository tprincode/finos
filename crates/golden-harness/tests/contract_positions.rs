//! Contract positions Tools screen. Persists via ContractList / ContractCreate.
//! Roll yield still imports projectHold from interest-rate math.

use golden_harness::repo_root;
use std::path::PathBuf;

fn contracts_dir() -> PathBuf {
    repo_root().join("apps/desktop/src/features/contracts")
}

#[test]
fn tools_menu_opens_contract_positions() {
    let root = repo_root();
    let app = std::fs::read_to_string(root.join("apps/desktop/src/App.tsx")).unwrap();
    let screen = std::fs::read_to_string(contracts_dir().join("ContractPositions.tsx")).unwrap();
    let create = std::fs::read_to_string(contracts_dir().join("ContractCreateForm.tsx")).unwrap();
    assert!(
        app.contains("navButton(\"contract-positions\", \"Contract positions\")"),
        "Tools must list Contract positions next to Interest rate"
    );
    assert!(
        app.contains("<ContractPositions client={client}"),
        "App must mount Contract positions with the finance client"
    );
    assert!(
        screen.contains("aria-label=\"Contract positions\"")
            && screen.contains("<h2>Contract positions</h2>"),
        "owner must see the screen heading"
    );
    assert!(
        screen.contains("ContractList") && screen.contains("<ContractCreateForm"),
        "ContractList and the create form must stay on the screen"
    );
    assert!(
        create.contains("aria-label=\"OCC symbol\"")
            && create.contains("aria-label=\"Create contract\""),
        "create controls must stay on the create form"
    );
}

#[test]
fn parse_occ_documents_compact_and_osi_fixtures() {
    let parse = std::fs::read_to_string(contracts_dir().join("parseOcc.ts")).unwrap();
    assert!(
        parse.contains(".TSLL1270115C20.7")
            && parse.contains("2070")
            && parse.contains("strikeMinor")
            && parse.contains("AAPL  250117C00150000"),
        "parseOcc must document compact + OSI fixtures with strikeMinor cents"
    );
}

#[test]
fn roll_yield_imports_project_hold_from_interest_rate_math() {
    let roll = std::fs::read_to_string(contracts_dir().join("rollYield.ts")).unwrap();
    let screen = std::fs::read_to_string(contracts_dir().join("ContractPositions.tsx")).unwrap();
    assert!(
        roll.contains("from \"../interest-rate/math\"") && roll.contains("projectHold"),
        "rollYield must import projectHold from interest-rate math"
    );
    assert!(
        screen.contains("from \"../interest-rate/math\""),
        "ContractPositions must reuse interest-rate math helpers"
    );
    assert!(
        !contracts_dir().join("math.ts").is_file(),
        "do not copy interest-rate math.ts under features/contracts"
    );
}

#[test]
fn interest_rate_premium_table_unchanged_by_contract_positions() {
    let screen = std::fs::read_to_string(
        repo_root().join("apps/desktop/src/features/interest-rate/InterestRateCalculator.tsx"),
    )
    .unwrap();
    assert!(
        screen.contains("aria-label=\"Contract return projection\"")
            && screen.contains("aria-label=\"Covered call or put premium\"")
            && !screen.contains("parseOcc")
            && !screen.contains("ContractPositions"),
        "Interest rate calculator must keep its premium table and stay free of OCC rows"
    );
}
