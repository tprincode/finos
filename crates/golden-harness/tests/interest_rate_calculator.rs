//! Interest rate calculator is a Tools scratch pad. Compound 365-day year.
//! Spreadsheet lock: 3.50% monthly → 51.11% annual. Hold uses start (today) and end date.

use golden_harness::repo_root;

#[test]
fn tools_menu_opens_interest_rate_calculator() {
    let root = repo_root();
    let app = std::fs::read_to_string(root.join("apps/desktop/src/App.tsx")).unwrap();
    let screen = std::fs::read_to_string(
        root.join("apps/desktop/src/features/interest-rate/InterestRateCalculator.tsx"),
    )
    .unwrap();
    let math =
        std::fs::read_to_string(root.join("apps/desktop/src/features/interest-rate/math.ts"))
            .unwrap();
    assert!(
        app.contains("navButton(\"interest-rate\", \"Interest rate calculator\")"),
        "Tools must list Interest rate calculator"
    );
    assert!(
        app.contains("<InterestRateCalculator"),
        "App must mount the extracted calculator"
    );
    assert!(
        screen.contains("aria-label=\"Interest rate calculator\"")
            && screen.contains("<h2>Interest rate calculator</h2>"),
        "owner must see the screen heading"
    );
    assert!(
        screen.contains("aria-label=\"Hold start date\"")
            && screen.contains("aria-label=\"Hold end date\"")
            && screen.contains("Enter the end date"),
        "contract rows must take today as start and prompt for end date"
    );
    assert!(
        screen.contains("aria-label=\"Contract amount\"")
            && screen.contains("aria-label=\"Covered call or put premium\""),
        "contract $ and premium $ are the inputs"
    );
    assert!(
        math.contains("daily: 365")
            && math.contains("monthly: 12")
            && math.contains("** fromPerYear")
            && math.contains("** (365 / days)"),
        "conversion is compound on a 365-day year"
    );
}

#[test]
fn spreadsheet_monthly_3_50_compounds_to_51_11_annual() {
    let annual = (1.0_f64 + 0.035).powi(12) - 1.0;
    let shown = (annual * 10000.0).round() / 100.0;
    assert!(
        (shown - 51.11).abs() < 0.001,
        "sheet monthly 3.50% must still be 51.11% annual, got {shown}"
    );
}
