//! Home portfolio board locks: metric one-row + Income through before Last Price.

use golden_harness::repo_root;

fn portfolio_summary(app: &str) -> &str {
    let Some(after) = app.split("className=\"portfolio-summary\"").nth(1) else {
        panic!("portfolio-summary missing from App.tsx");
    };
    after.split("</dl>").next().unwrap_or(after)
}

#[test]
fn home_portfolio_metrics_stay_in_ps_head_row() {
    let root = repo_root();
    let app = std::fs::read_to_string(root.join("apps/desktop/src/App.tsx")).expect("App.tsx");
    let css = std::fs::read_to_string(root.join("apps/desktop/src/App.css")).expect("App.css");

    let summary = portfolio_summary(&app);
    let Some(after_head) = summary.split("className=\"ps-head-row\"").nth(1) else {
        panic!("ps-head-row wrapper missing around Home metric cards");
    };
    let head = after_head
        .split("className=\"ps-cell ps-cash\"")
        .next()
        .unwrap_or(after_head);

    for needle in ["ps-mv", "ps-cost", "ps-tax", "ps-unrealized"] {
        assert!(head.contains(needle), "ps-head-row must contain {needle}");
    }
    let mv = head.find("ps-mv").expect("ps-mv");
    let cost = head.find("ps-cost").expect("ps-cost");
    let tax = head.find("ps-tax").expect("ps-tax");
    let unreal = head.find("ps-unrealized").expect("ps-unrealized");
    assert!(
        mv < cost && cost < tax && tax < unreal,
        "metric order must be mv → cost → tax → unrealized"
    );
    assert!(!head.contains("ps-cash"), "cash must stay outside ps-head-row");

    assert!(
        css.contains(".portfolio-summary .ps-head-row")
            && css.contains("grid-template-columns: repeat(4, minmax(0, 1fr))"),
        "App.css must keep .ps-head-row as a four-column row"
    );
}

#[test]
fn home_portfolio_income_through_before_last_price() {
    let app =
        std::fs::read_to_string(repo_root().join("apps/desktop/src/App.tsx")).expect("App.tsx");
    let summary = portfolio_summary(&app);
    let income = summary
        .find("<dt>Income through</dt>")
        .expect("Income through tile missing");
    let last_price = summary
        .find("<dt>Last Price all symbols</dt>")
        .expect("Last Price tile missing");
    assert!(
        income < last_price,
        "Owner lock: Income through must appear before Last Price (swap must not regress)"
    );
}
