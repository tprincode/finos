use golden_harness::repo_root;

#[test]
fn accessibility_primary_actions_have_accessible_names() {
    let app = std::fs::read_to_string(repo_root().join("apps/desktop/src/App.tsx"))
        .expect("App.tsx");
    let ui = std::fs::read_to_string(repo_root().join("packages/ui-components/src/index.tsx"))
        .expect("ui-components");
    let list = std::fs::read_to_string(repo_root().join("packages/ui-components/src/listTable.tsx"))
        .expect("listTable");
    let trends_capture =
        std::fs::read_to_string(repo_root().join("apps/desktop/src/features/graphing/TrendsCapture.tsx"))
            .expect("TrendsCapture.tsx");
        let trends_charts =
        std::fs::read_to_string(repo_root().join("apps/desktop/src/features/graphing/TrendsCharts.tsx"))
            .expect("TrendsCharts.tsx");
    let cash_week_desk =
        std::fs::read_to_string(repo_root().join("apps/desktop/src/features/cash/CashWeekDesk.tsx"))
            .expect("CashWeekDesk.tsx");
        let home_account_charts =
        std::fs::read_to_string(repo_root().join("apps/desktop/src/features/graphing/HomeAccountCharts.tsx"))
            .expect("HomeAccountCharts.tsx");
        let decl_chart =
        std::fs::read_to_string(repo_root().join("apps/desktop/src/features/graphing/DeclarationPaymentsChart.tsx"))
            .expect("DeclarationPaymentsChart.tsx");
    let import_wizard =
        std::fs::read_to_string(repo_root().join("apps/desktop/src/ImportWizard.tsx"))
            .expect("ImportWizard.tsx");
    let dividend_weeks =
        std::fs::read_to_string(repo_root().join("apps/desktop/src/features/graphing/DividendWeeks.tsx"))
            .expect("DividendWeeks.tsx");
    let cash_management =
        std::fs::read_to_string(repo_root().join("apps/desktop/src/CashManagement.tsx"))
            .expect("CashManagement.tsx");
    let household_income = std::fs::read_to_string(
        repo_root().join("apps/desktop/src/features/cash/HouseholdIncomeReport.tsx"),
    )
    .expect("HouseholdIncomeReport.tsx");
    let cash_coverage =
        std::fs::read_to_string(repo_root().join("apps/desktop/src/features/cash/CashCoverage.tsx"))
            .expect("CashCoverage.tsx");
    let home_dividend_plan =
        std::fs::read_to_string(repo_root().join("apps/desktop/src/features/home/HomeDividendPlan.tsx"))
            .expect("HomeDividendPlan.tsx");
    let plan_horizon =
        std::fs::read_to_string(
            repo_root().join("apps/desktop/src/features/income-plan/PlanHorizonPrompt.tsx"),
        )
        .expect("PlanHorizonPrompt.tsx");
    let income_plan_screen = std::fs::read_to_string(
        repo_root().join("apps/desktop/src/features/income-plan/IncomePlanScreen.tsx"),
    )
    .expect("IncomePlanScreen.tsx");
    let home_trend_focus =
        std::fs::read_to_string(repo_root().join("apps/desktop/src/features/cash/AccountCashFlow.tsx"))
            .expect("AccountCashFlow.tsx");
    let collectors_screen = std::fs::read_to_string(
        repo_root().join("apps/desktop/src/features/collectors/CollectorsScreen.tsx"),
    )
    .expect("CollectorsScreen.tsx");
    let collector_establish = std::fs::read_to_string(
        repo_root().join("apps/desktop/src/features/collectors/CollectorEstablishScreen.tsx"),
    )
    .expect("CollectorEstablishScreen.tsx");
    let cart_screen = std::fs::read_to_string(
        repo_root().join("apps/desktop/src/features/shopping-cart/ShoppingCartScreen.tsx"),
    )
    .expect("ShoppingCartScreen.tsx");
    let cart_execute = std::fs::read_to_string(
        repo_root().join("apps/desktop/src/features/shopping-cart/ExecutePlanPanel.tsx"),
    )
    .expect("ExecutePlanPanel.tsx");
    let cart_rail = std::fs::read_to_string(
        repo_root().join("apps/desktop/src/features/shopping-cart/StepRail.tsx"),
    )
    .expect("StepRail.tsx");
    let position_details = std::fs::read_to_string(
        repo_root().join("apps/desktop/src/features/position-details/PositionDetailsScreen.tsx"),
    )
    .expect("PositionDetailsScreen.tsx");
    let symbol_window = std::fs::read_to_string(
        repo_root().join("apps/desktop/src/features/market-impact/SymbolWindowRow.tsx"),
    )
    .expect("SymbolWindowRow.tsx");
    assert!(
        position_details.contains("aria-label=\"Plan Management\"")
            && position_details.contains("meanNewestPays(")
            && position_details.contains("recentPays")
            && !position_details.contains("lastPaidDeclarations("),
        "Plan Management shows Avg 6 from the calculator helper, not a second average"
    );
    let add_lot = std::fs::read_to_string(
        repo_root().join("apps/desktop/src/features/add-lot/AddLotScreen.tsx"),
    )
    .expect("AddLotScreen.tsx");
    let holdings = std::fs::read_to_string(
        repo_root().join("apps/desktop/src/features/holdings/HoldingsScreen.tsx"),
    )
    .expect("HoldingsScreen.tsx");
    let component_registry = std::fs::read_to_string(
        repo_root().join("apps/desktop/src/features/components/ComponentRegistry.tsx"),
    )
    .expect("ComponentRegistry.tsx");
    let sources = format!(
        "{app}\n{ui}\n{list}\n{trends_capture}\n{trends_charts}\n{cash_week_desk}\n{home_account_charts}\n{decl_chart}\n{import_wizard}\n{dividend_weeks}\n{cash_management}\n{household_income}\n{cash_coverage}\n{home_dividend_plan}\n{home_trend_focus}\n{plan_horizon}\n{income_plan_screen}\n{collectors_screen}\n{collector_establish}\n{cart_screen}\n{cart_execute}\n{cart_rail}\n{position_details}\n{symbol_window}\n{add_lot}\n{holdings}\n{component_registry}"
    );
    for name in [
        "aria-label=\"Import wizard\"",
        "aria-label=\"Import step\"",
        "aria-label=\"Validate step\"",
        "aria-label=\"Import transactions\"",
        "aria-label=\"Continue to validate\"",
        "aria-label=\"Continue last import\"",
        "aria-label=\"Load\"",
        "aria-label=\"Cancel\"",
        "aria-label=\"Plan versus declaration\"",
        "aria-label=\"Dividend week table\"",
        "aria-label=\"Dividend performance period\"",
        "aria-label=\"Plan versus declaration for selected period\"",
    ] {
        assert!(sources.contains(name), "missing import wizard name {name}");
    }
    for name in [
        "aria-label=\"finos\"",
        "aria-label=\"Save device name\"",
        "aria-label=\"Save data snapshot\"",
        "aria-label=\"Confirm save data snapshot\"",
        "aria-label=\"Cancel save data snapshot\"",
        "aria-label=\"Create snapshot\"",
        "aria-label=\"Restore published\"",
        "aria-label=\"Acknowledge review\"",
        "aria-label=\"Restart in progress\"",
        "aria-label=\"Exit\"",
        "aria-label=\"Import Fidelity or Schwab CSV\"",
        "aria-label=\"Check for updates\"",
        "aria-label=\"Exceptions\"",
        "aria-label=\"Manual dividend\"",
        "aria-label=\"Manual dividend rows\"",
        "aria-label=\"Post manual dividends\"",
        "aria-label=\"Add manual dividend row\"",
        "aria-label=\"Income Plan\"",
        "aria-label=\"Next-year plan dates\"",
        "aria-label=\"Confirm next-year plan dates\"",
        "aria-label=\"Calculator\"",
        "aria-label=\"Distribution history\"",
        "aria-label=\"Payment frequency filter\"",
        "aria-label=\"History period\"",
        "aria-label=\"History start date\"",
        "aria-label=\"History end date\"",
        "aria-label=\"Position Details\"",
        "aria-label=\"Dashboard\"",
        "aria-label=\"Holdings\"",
        "aria-label=\"Add Investment\"",
        "aria-label=\"Add Lot\"",
        "aria-label=\"Import\"",
        "aria-label=\"Collectors\"",
        "aria-label=\"Tickets\"",
        "aria-label=\"Settings\"",
        "aria-label=\"Run enabled collectors\"",
        "aria-label=\"Run misses only\"",
        "aria-label=\"Work tickets\"",
        "aria-label=\"Work ticket summary\"",
        "aria-label={`Recreate adapter ${symbol}`}",
        "aria-label={`Accept ${symbol} ROC change`}",
        "aria-label={`Reject ${symbol} ROC change`}",
        "aria-label={`ROC change action ${symbol}`}",
        "aria-label=\"Open exception log\"",
        "aria-label=\"Open process log\"",
        "aria-label=\"Capture process\"",
        "aria-label=\"Capture process lines\"",
        "aria-label=\"Dismiss capture process\"",
        "aria-label=\"Open in Position Details\"",
        "aria-label=\"Calculator snapshot\"",
        "aria-label=\"Plan and yields\"",
        "aria-label=\"Plan payment summary\"",
        "aria-label=\"Future plan payment dates\"",
        "aria-label=\"Holdings by account\"",
        "aria-label=\"Declarations\"",
        "aria-label=\"Lots by account\"",
        "aria-label=\"Ledger income\"",
        "aria-label=\"Position hub sections\"",
        "aria-label=\"Position hub summary\"",
        "aria-label=\"Collector action status\"",
        "aria-label=\"Collector run progress\"",
        "aria-label=\"Collector run log\"",
        "aria-label=\"Collector fleet\"",
        "aria-label=\"Collector footer grid\"",
        "aria-label=\"Collector statistics\"",
        "aria-label=\"Collect fresh distribution data for this symbol\"",
        "aria-label=\"Reevaluate collector\"",
        "aria-label=\"Establish collector fleet\"",
        "aria-label={`Establish collector ${row.symbol}`}",
        "aria-label={`Reevaluate collector ${row.symbol}`}",
        "aria-label={`Accept recommended ROC ${row.symbol}`}",
        "aria-label=\"Retrieve runs\"",
        "aria-label=\"Collector retrieve payload\"",
        "aria-label=\"Collector symbol page\"",
        "aria-label=\"Stored declarations\"",
        "aria-label=\"Collector plan\"",
        "aria-label=\"Assign lot\"",
        "aria-label=\"Retrieve declarations for this symbol\"",
        "aria-label=\"Mandatory data checklist\"",
        "aria-label=\"Dividend basis summary\"",
        "aria-label=\"Dividend statistics\"",
        "aria-label=\"Owner plan and tier actions\"",
        "aria-label=\"Last declarations\"",
        "aria-label=\"Remaining year dates\"",
        "aria-label=\"Provider decision\"",
        "aria-label=\"Use Most Current as Plan\"",
        "aria-label=\"Use Avg 6 as Plan\"",
        "aria-label=\"Plan Management\"",
        "aria-label=\"Save Process A research\"",
        "aria-label=\"Save stored facts\"",
        "aria-label=\"Position information\"",
        "aria-label=\"Position frequency\"",
        "aria-label=\"Cancel position edits\"",
        "aria-label=\"Research\"",
        "aria-label=\"Confirm add lot\"",
        "aria-label={`Confirm purchase ${row.symbol}`}",
        "aria-label=\"Completed register summary\"",
        "aria-label=\"Shopping cart steps\"",
        "aria-label=\"Cancel add lot edits\"",
        "aria-label=\"Cancel unsaved edits\"",
        "aria-label=\"Open screen with unsaved edits\"",
        "Save ${row.symbol} windows",
        "aria-label=\"Market impact windows\"",
        "aria-label=\"Position dossier\"",
        "aria-label=\"Confirm Plan\"",
        "aria-label=\"Complete research\"",
        "aria-label=\"Complete remaining details\"",
        "aria-label=\"Re-run research retrieval\"",
        "aria-label=\"Fill research gaps\"",
        "aria-label=\"Research progress\"",
        "aria-label=\"Research notes\"",
        "aria-label=\"Research result\"",
        "aria-label=\"Owner risk choice\"",
        "aria-label=\"Accept ROC estimate\"",
        "aria-label=\"Manual ROC percent\"",
        "aria-label=\"Store manual ROC percent\"",
        "aria-label=\"ROC research strip\"",
        "aria-label=\"Stored Template Dividend offer\"",
        "aria-label=\"Owner underlying\"",
        "aria-label=\"Owner risk tier\"",
        "aria-label=\"Save identity\"",
        "aria-label=\"Apply owner risk tier\"",
        "aria-label=\"Confirm Plan blocked reason\"",
        "aria-label=\"Confirm Plan ready\"",
        "aria-label=\"Plan typed but not stored\"",
        "aria-label=\"Investment details status\"",
        "investment-details-status is-complete",
        "investment-details-status is-missing",
        "Information still needed — Confirm Plan (Shopping Cart needs stored Plan / share)",
        "Confirm Plan (Shopping Cart needs stored Plan / share)",
        "aria-label=\"Add Investment completion status\"",
        "aria-label=\"What is saved versus optional\"",
        "research and Plan are saved in the book",
        "Calculator / Income Plan",
        "Calculator lists Plan with 0 shares",
        "aria-label=\"Researched without open lot\"",
        "aria-label=\"Incomplete analysis covered by inception\"",
        "aria-label=\"Incomplete analysis reason\"",
        "Recent inception date",
        "aria-label=\"Buy plan missing\"",
        "offerStoredTemplatesForSymbol",
        "PROCESS_A_DRAFT_KEY",
        "saveProcessAOwnerIdentity",
        "aria-label=\"Position theme strategy\"",
        "aria-label=\"Position primary risk driver\"",
        "aria-label=\"Position concentration\"",
        "aria-label=\"Position volatility proxy\"",
        "aria-label=\"Position tax character\"",
        "aria-label=\"Refresh last prices\"",
        "aria-label=\"Refresh declarations\"",
        "aria-label=\"Work Tickets\"",
        "aria-label=\"Income through transactions\"",
        "aria-label=\"Exit income through transactions\"",
        "aria-label=\"Income transaction period\"",
        "aria-label=\"Income transaction start\"",
        "aria-label=\"Income transaction end\"",
        "aria-label=\"Income transaction account\"",
        "aria-label=\"Core functions\"",
        "aria-label=\"Component Registry\"",
        "declaration-refresh-progress",
        "last-price-refresh-progress",
        "${formatCount(declarationProgress.current)} of ${formatCount(declarationProgress.total)}",
        "${formatCount(lastPriceProgress.current)} of ${formatCount(lastPriceProgress.total)}",
        "aria-label=\"Last price refresh progress\"",
        "aria-label=\"Declaration refresh progress\"",
        "aria-label=\"Refresh last price for this symbol\"",
        "aria-label=\"Retrieve declarations for this symbol\"",
        "aria-label=\"Complete research\"",
        "aria-label=\"Position ROC research status\"",
        "aria-label=\"Position symbols\"",
        "aria-label=\"Position information\"",
        "aria-label=\"Declaration graphing period\"",
        "aria-label=\"Issuer retrieve miss summary\"",
        "aria-label=\"Application\"",
        "aria-label=\"Data position totals\"",
        "aria-label=\"Apply issuer sources from provider\"",
        "aria-label=\"Investment type\"",
        "aria-label=\"Position div type\"",
        "aria-label=\"Accept div type\"",
        "DIV_TYPES",
        "normalizeDivType",
        "aria-label=\"Needs ROC research\"",
        "aria-label=\"Position is active\"",
        "aria-label=\"Expected tax handling\"",
        "aria-label=\"Declaration weekday\"",
        "aria-label=\"Ex-date weekday\"",
        "aria-label=\"Payday weekday\"",
        "aria-label=\"Position completeness\"",
        "aria-label=\"Position master\"",
        "aria-label={`Open ${row.symbol} dossier`}",
        "aria-label={`Filter ${label}`}",
        "aria-label={`Clear ${label} filter`}",
        "aria-label=\"Clear filters\"",
        "aria-label=\"Select week\"",
        "aria-label=\"Current week\"",
        "aria-label=\"Previous week\"",
        "aria-label=\"Next week\"",
        "aria-label=\"Income plan by account\"",
        "aria-label=\"Income plan by position\"",
        "Decl $ per share ${declShareTone",
        "aria-label=\"Plan versus declaration\"",
        "aria-label=\"Dividend week table\"",
        "aria-label=\"Dividend performance period\"",
        "aria-label=\"Plan versus declaration for selected period\"",
        "aria-label=\"Filter holdings\"",
        "label=\"Filter calculator\"",
        "label=\"Filter position lots\"",
        "label=\"Filter dashboard\"",
        "aria-label=\"Account values\"",
        "aria-label=\"Dividend Plan\"",
        "aria-label=\"Account cash flow projection\"",
        "aria-label=\"Ending plotted cash\"",
        "aria-label=\"Starting balance for period\"",
        "aria-label=\"Planned income for period\"",
        "aria-label=\"Planned withdrawals for period\"",
        "aria-label=\"Account trend account\"",
        "aria-label=\"Account trend duration\"",
        "aria-label=\"Account value legend\"",
        "Weekly actuals",
        "aria-label=\"Home graphing period\"",
        "aria-label=\"Risk Profile\"",
        "aria-label=\"Live by risk allocation\"",
        "aria-label=\"Symbol totals\"",
        "aria-label=\"Exit symbol totals\"",
        "aria-label=\"Saved cash weeks\"",
        "aria-label=\"Cash week overview\"",
        "aria-label=\"Trends weekly capture\"",
        "aria-label=\"Week capture grid\"",
        "aria-label=\"Account 9 70% ETF\"",
        "aria-label=\"Save Trends week\"",
        "aria-label=\"Edit Trends week\"",
        "aria-label=\"Correct Trends week\"",
        "aria-label=\"Close Trends week\"",
        "aria-label=\"Trends graphing period\"",
        "aria-label=\"Cash Management\"",
        "aria-label=\"Save cash distribution\"",
        "aria-label=\"Cancel cash distribution\"",
        "aria-label=\"Cash management week\"",
        "aria-label=\"Distribution account\"",
        "aria-label=\"Distribution gross\"",
        "aria-label=\"Federal withholding\"",
        "aria-label=\"State withholding\"",
        "aria-label=\"Saturday income draft\"",
        "aria-label=\"Confirm Tom Social Security retirement\"",
        "aria-label=\"Tom SSA received\"",
        "aria-label=\"Cancel Tom SSA confirm\"",
        "aria-label=\"Cash MAGI preview\"",
        "aria-label=\"Cash Management distributions YTD\"",
        "aria-label=\"Distribution account totals\"",
        "aria-label=\"All distribution accounts\"",
        "aria-label=\"Distribution tax sections\"",
        "aria-label=\"Cash Management tax and ACA monitor\"",
        "aria-label=\"Cash Management Tax Planning\"",
        "aria-label=\"Tax Planning income\"",
        "aria-label=\"Tax Planning MAGI\"",
        "aria-label=\"Income vs Expense planner\"",
        "aria-label=\"Coverage period\"",
        "aria-label=\"Coverage plan\"",
        "aria-label=\"Coverage income math\"",
        "aria-label=\"Coverage expense math\"",
        "aria-label=\"Portfolio summary\"",
        "aria-label={`Sort by ${label}`}",
    ] {
        assert!(sources.contains(name), "missing accessible name {name}");
    }
    assert!(
        sources.contains("REGISTERED_DECLARATION_SOURCES")
            || sources.contains("declarationSource")
                && sources.contains("Apply issuer sources from provider"),
        "declaration source select must list all registered issuer adapters"
    );
    assert!(
        !app.contains("Load household seed"),
        "owner UI must not mention household seed"
    );
    assert!(
        !app.contains("ProductionSeedLoad"),
        "owner UI must not call ProductionSeedLoad"
    );
    assert!(
        !app.contains("Data stays on this machine after seed"),
        "owner UI must not show seed process prose"
    );
    assert!(
        !app.contains("Checking data file"),
        "owner UI must not mention data file loading"
    );
    assert!(
        app.contains("saturdayOfWeek(new Date().toISOString().slice(0, 10))"),
        "Income Plan as-of must default to calendar this week, not last yield"
    );
}

#[test]
fn cart_confirm_returns_to_cart_then_confirm_cash() {
    let app = std::fs::read_to_string(repo_root().join("apps/desktop/src/App.tsx")).unwrap();
    let cart = std::fs::read_to_string(
        repo_root().join("apps/desktop/src/features/shopping-cart/ShoppingCartScreen.tsx"),
    )
    .unwrap();
    let execute = std::fs::read_to_string(
        repo_root().join("apps/desktop/src/features/shopping-cart/ExecutePlanPanel.tsx"),
    )
    .unwrap();
    let rail = std::fs::read_to_string(
        repo_root().join("apps/desktop/src/features/shopping-cart/StepRail.tsx"),
    )
    .unwrap();
    // Hard lock: after LotOpen from a cart buy, screen must return to shopping-cart.
    // A weak "opens Add Lot" OR previously let this regress four times.
    assert!(
        app.contains("setScreen(fromCart ? \"shopping-cart\"")
            && app.contains("resumeScenarioId={cartResumeScenarioId}")
            && app.contains("writeCartResume("),
        "LotOpen from cart must setScreen shopping-cart and keep resumeScenarioId — opening wizard is a defect"
    );
    assert!(
        app.contains("let fromCart = false")
            && app.contains("fromCart = true")
            && app.contains("pendingCartBuyRef")
            && app.contains("CartExecuteBuyStep"),
        "cart buy path must mark fromCart and record CartExecuteBuyStep"
    );
    assert!(
        app.contains("addLotReturnScreenRef.current = \"shopping-cart\"")
            && app.contains("cartRefreshKey={cartRefreshKey}"),
        "Confirm purchase must set return screen + refresh cart on return"
    );
    assert!(
        cart.contains("CartScenarioGet")
            && cart.contains("resumingCart")
            && cart.contains("Returning to in-progress cart"),
        "Shopping Cart remount must restore the executing scenario, not CartStartWizard"
    );
    assert!(
        !app.contains("Enter qty and unit $ again for another lot"),
        "cart Confirm must not keep the owner on Add Lot for a second lot"
    );
    assert!(
        !app.contains("aria-label=\"Save add lot\""),
        "Add Lot commit is Confirm, not Save"
    );
    assert!(
        !app.contains("Type the next qty and unit $"),
        "Add Lot must not stay for a second Save"
    );
    assert!(
        rail.contains("Confirm purchase") && rail.contains("Confirm cash") && !rail.contains("Open lot"),
        "step rail is Confirm purchase then Confirm cash"
    );
    assert!(
        execute.contains("Confirm cash") && !execute.contains("Align cash"),
        "finish step is Confirm cash"
    );
    assert!(
        execute.contains("Confirm cash preview") && execute.contains("Confirm cash calculation"),
        "Confirm cash must show leftover dollars, not only a button"
    );
    assert!(
        cart.contains("align leftover cash after the last purchase")
            && execute.contains("Completed register summary")
            && execute.contains("Net dividend change")
            && !execute.contains("Ending {cashSymbol} $"),
        "last purchase aligns leftover cash and shows one completion row"
    );
    // periodsForPlan lives in cartPlan.ts; the screen must still route through it.
    let cart_plan = std::fs::read_to_string(
        repo_root().join("apps/desktop/src/features/shopping-cart/cartPlan.ts"),
    )
    .unwrap();
    assert!(
        cart.contains("Frequency wins over a stale Calculator planningPeriodsPerYear")
            && cart.contains("periodsForPlan(")
            && cart_plan.contains("Twice monthly")
            && cart_plan.contains("return 24"),
        "cart yield must prefer Twice monthly = 24 over stale planningPeriodsPerYear"
    );
    assert!(
        cart.contains("agreedScene ? (") && cart.contains("<ExecutePlanPanel"),
        "after Agree the cart is the execute panel only"
    );
}

/// The owner called out a wrong MUIB rate repeatedly. Two locks: the cart reads Plan only,
/// and every rate shows the Plan $ and periods it came from so a wrong one is readable.
#[test]
fn cart_rate_is_plan_only_and_shows_its_plan_basis() {
    let folder = repo_root().join("apps/desktop/src/features/shopping-cart");
    let cart = std::fs::read_to_string(folder.join("ShoppingCartScreen.tsx")).unwrap();
    let sheets = std::fs::read_to_string(folder.join("PlanSheets.tsx")).unwrap();
    let plan = std::fs::read_to_string(folder.join("cartPlan.ts")).unwrap();
    assert!(
        plan.contains("export function planBasisText") && plan.contains("Plan $"),
        "cartPlan owns the Plan $ x periods label"
    );
    assert!(
        cart.contains("function planBasisForSymbol")
            && cart.matches("planBasis:").count() >= 2
            && cart.contains("planBasisForSymbol(calculator, master, line.symbol)"),
        "sell and buy sheet rows both carry the Plan basis"
    );
    assert!(
        sheets.contains("planBasis?: string") && sheets.matches("row.planBasis").count() >= 2,
        "the sheet renders the Plan basis next to the rate"
    );
    for entry in std::fs::read_dir(&folder).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("tsx")
            && path.extension().and_then(|e| e.to_str()) != Some("ts")
        {
            continue;
        }
        let body = std::fs::read_to_string(&path).unwrap();
        for needle in [
            "declPerShareMinor",
            "declaredAmountMinor",
            "amountPerShareMinor",
            "IssuerDeclarationList",
            "declFwdYieldBps",
        ] {
            assert!(
                !body.contains(needle),
                "{} must not read {needle} — the cart uses Plan amounts for every rate and \
                 projected income comparison",
                path.display()
            );
        }
    }
}

#[test]
fn dividend_weeks_lives_on_income_plan_not_trends_middle() {
    let app = std::fs::read_to_string(repo_root().join("apps/desktop/src/App.tsx")).unwrap();
    let trends = app
        .split("{screen === \"trends\" ? (")
        .nth(1)
        .and_then(|rest| rest.split("{screen === \"shopping-cart\" ? (").next())
        .unwrap_or("");
    assert!(
        !trends.contains("DividendWeeksPanel"),
        "Plan vs Decl must not sit on the Trends capture page"
    );
    let income = std::fs::read_to_string(
        repo_root().join("apps/desktop/src/features/income-plan/IncomePlanScreen.tsx"),
    )
    .unwrap();
    assert!(
        income.contains("DividendWeeksPanel")
            && income.contains("aria-label=\"Income Plan\""),
        "Plan vs Decl belongs at the bottom of Income Plan"
    );
    assert!(
        app.contains("<IncomePlanScreen")
            && !app.contains("className=\"income-plan-page\""),
        "Income Plan screen lives in features/income-plan, not App.tsx"
    );
    let charts = std::fs::read_to_string(
        repo_root().join("apps/desktop/src/features/graphing/TrendsCharts.tsx"),
    )
    .unwrap();
    let cash_desk = std::fs::read_to_string(
        repo_root().join("apps/desktop/src/features/cash/CashWeekDesk.tsx"),
    )
    .unwrap();
    let cash = std::fs::read_to_string(
        repo_root().join("apps/desktop/src/CashManagement.tsx"),
    )
    .unwrap();
    assert!(
        !trends.contains("TrendsCapturePanel"),
        "week capture must not sit on Trends"
    );
    assert!(
        !charts.contains("Saved Trends weeks") && !charts.contains("FID+SCH"),
        "Trends must not show the week desk table or FID+SCH cards"
    );
    let cash_screen = app
        .split("{screen === \"cash-management\" ? (")
        .nth(1)
        .and_then(|rest| rest.split("{screen === \"holdings\" ? (").next())
        .unwrap_or("");
    assert!(
        cash_screen.contains("TrendsCapturePanel") && cash.contains("CashWeekDesk"),
        "Cash Management hosts week capture and the saved-weeks desk"
    );
    assert!(
        cash_desk.contains("Saved cash weeks"),
        "Cash Management must list week rows including gaps"
    );
    assert!(
        cash_desk.contains("trendsTableSaturdays"),
        "cash week table must enumerate Sat–Fri gaps through today"
    );
    assert!(
        cash_desk.contains("plannedWeekIncomeMinor")
            && cash_desk.contains("reportedWeekIncomeMinor"),
        "Week income splits planned (Decl $) from reported (paid actuals)"
    );
    assert!(
        cash_desk.contains("Planned weekly income")
            && cash_desk.contains("Reported weekly income"),
        "Cash week desk has Planned and Reported weekly income columns"
    );
    assert!(
        !cash_desk.contains("<th className=\"numeric\">Profit</th>")
            && !cash_desk.contains("Profit {"),
        "Cash week desk does not show seed Profit"
    );
    assert!(
        charts.contains("Declared vs Plan"),
        "Trends must keep the weekly Plan vs Declaration chart"
    );
    assert!(
        charts.contains("plannedWeekIncomeMinor")
            && charts.contains("reportedWeekIncomeMinor")
            && charts.contains("aria-label=\"Planned weekly income\"")
            && charts.contains("aria-label=\"Reported weekly income\""),
        "Weekly charts plot Planned and Reported weekly income, not stored seed Profit"
    );
    assert!(
        !charts.contains("key: \"profitMinor\""),
        "Trends metric charts must not use seed profitMinor"
    );
    assert!(
        charts.contains("filterPerfByPeriod") && charts.contains("onGraphPeriodChange"),
        "Decl vs Plan and Trends charts must follow Trends graphing period only"
    );
    let decl_chart = std::fs::read_to_string(
        repo_root().join("apps/desktop/src/features/graphing/DividendWeeks.tsx"),
    )
    .unwrap();
    assert!(
        decl_chart.contains("name: \"Declared\"")
            && decl_chart.contains("name: \"Plan\"")
            && !decl_chart.contains("Decl — solid")
            && !decl_chart.contains("Plan — dashed")
            && decl_chart.contains("type: \"dashed\"")
            && decl_chart.contains("symbol: \"none\""),
        "Declared vs Plan: Declared and Plan labels, dashed Plan line, no circles"
    );
    assert!(
        charts.contains("week_not_saved"),
        "charts must ignore week_not_saved quality noise"
    );
    let capture = std::fs::read_to_string(
        repo_root().join("apps/desktop/src/features/graphing/TrendsCapture.tsx"),
    )
    .unwrap();
    assert!(
        !capture.contains("<dt>Profit</dt>")
            && capture.contains("<dt>Planned weekly income</dt>")
            && capture.contains("<dt>Reported weekly income</dt>"),
        "week review shows Planned and Reported weekly income, not seed Profit"
    );
    assert!(
        capture.contains("onWizardActive"),
        "Trends wizard must tell App when it is in progress"
    );
    assert!(
        app.contains("Finish the week on Review"),
        "leaving mid-wizard must stay blocked until Accept"
    );
}

#[test]
fn dividend_weeks_newest_first_empty_not_na() {
    let src = std::fs::read_to_string(
        repo_root().join("apps/desktop/src/features/graphing/DividendWeeks.tsx"),
    )
    .expect("DividendWeeks.tsx");
    assert!(
        src.contains("useListSort(\"end\", \"desc\")"),
        "dividend weeks must list newest first"
    );
    assert!(
        !src.contains("missing stays N/A"),
        "dividend weeks must not lecture about N/A"
    );
    assert!(
        !src.contains("Past Saturday"),
        "dividend weeks must not lead with Sat–Fri explainer"
    );
    assert!(
        !src.contains("Week start") && !src.contains("Week ending"),
        "one Week column, not start and ending"
    );
    assert!(
        src.contains("moneyOrEmpty") && src.contains("pctOrEmpty"),
        "missing plan/decl/% must render empty, not N/A"
    );
    assert!(
        src.contains("Plan vs Decl") && src.contains("dividend-weeks-head"),
        "Plan vs Decl must open as its own headed section"
    );
    assert!(
        src.contains("pctOfPlanHeat") && src.contains("pct-heat"),
        "% of Plan must use miss/exceed heat color"
    );
    assert!(
        src.contains("dividend-weeks-summary-table")
            && src.contains("Selected weeks"),
        "period summary must be a table tied to the selected weeks"
    );
    assert!(
        !src.contains("ReactECharts") && !src.contains("dividend-weeks-chart"),
        "Plan vs Decl chart stays on Trends, not under the Income Plan table"
    );
}

#[test]
fn g1_g6_slice1b_capture_grid_one_table() {
    let capture = std::fs::read_to_string(
        repo_root().join("apps/desktop/src/features/graphing/TrendsCapture.tsx"),
    )
    .unwrap();
    // G1: six rows visible at once after the week dropdown.
    assert!(
        capture.contains("aria-label=\"Week capture grid\"")
            && capture.contains("label: \"Income\"")
            && capture.contains("label: \"FI Roth\"")
            && capture.contains("label: \"Speculation\"")
            && capture.contains("label: \"Health\"")
            && capture.contains("label: \"Car\"")
            && capture.contains("label: \"Account 9\""),
        "G1: one table lists Income, FI Roth, Speculation, Health, Car, Account 9"
    );
    assert!(
        !capture.contains("const STEPS")
            && !capture.contains("Next Trends step")
            && !capture.contains("setStep("),
        "G1: no Week → account → Review rail and no Next between accounts"
    );
    // G2: typed ETF 10000 → read-only 70% is 7000.
    assert!(
        capture.contains("G2: 10000 → 7000")
            && capture.contains("Math.round((total * 70) / 100)"),
        "G2: Account 9 70% is typed ETF total × 0.70"
    );
    // G3: blank ETF is — not $0.
    assert!(
        capture.contains("etfSeventy == null ? \"—\"")
            && capture.contains("seventyFromEtfTotal"),
        "G3: blank ETF total is not $0"
    );
    // G4: Edit refills the same table.
    assert!(
        capture.contains("typedDraftRef")
            && capture.contains("Edit Trends week")
            && !capture.contains("setStep(\"Income\")")
            && !capture.contains("setStep(\"Capture\")"),
        "G4: Edit keeps values on the same table"
    );
    // G5: no Adjust reason when cash matches.
    assert!(
        capture.contains("aria-label=\"Week cash recon\"")
            && capture.contains("material ?")
            && capture.contains("cash adjust reason"),
        "G5: Adjust reason only when the cash gap is material"
    );
    // G6
    assert!(
        capture.contains("accountName: \"Speculation\""),
        "G6: Speculation is a row"
    );
    assert!(
        capture.contains("placeholder=\"skip\"")
            && capture.contains("Blank cash = skip this week")
            && capture.contains("draft[row.totalKey].trim() !== \"\""),
        "blank cash is allowed on input and Accept; only totals are required"
    );
    assert!(
        capture.contains("const weekIncome = capture.suggestedMonthlyDivsMinor")
            && capture.contains("monthlyDivsMinor: weekIncome")
            && capture.contains("aria-label=\"Accept blocked reason\""),
        "Accept must send suggestedMonthlyDivsMinor; blocked clicks must say why"
    );
    assert!(
        capture.contains("Fidelity week-to-week")
            && capture.contains("Schwab week-to-week")
            && capture.contains("fidEntered ? formatUsd(fidChange, scale) : \"TBD\"")
            && capture.contains("schwabEntered ? formatUsd(schChange, scale) : \"TBD\""),
        "week-to-week stays TBD until that broker's totals are entered"
    );
    let css = std::fs::read_to_string(repo_root().join("apps/desktop/src/App.css")).unwrap();
    assert!(
        css.contains(".trends-capture-review")
            && css.contains("repeat(7, minmax(0, 1fr))"),
        "the seven week-review figures stay on one row"
    );
    let app = std::fs::read_to_string(repo_root().join("apps/desktop/src/App.tsx")).unwrap();
    assert!(
        app.contains("WeekCaptureAccept"),
        "Accept uses WeekCaptureAccept from PR #10"
    );
    assert!(
        app.contains("declarationRefreshedOn === summary.declarationAsOf"),
        "DeclarationRefresh on open runs once per local date"
    );
    assert!(
        app.contains("inSchedule === true")
            && app.contains("refreshDeclarations(true)")
            && app.contains("force ? { force: true }"),
        "auto collectors share weekday 9-4; Refresh declarations is force"
    );
}

#[test]
fn holdings_assign_picks_sell_not_uuid() {
    let holdings = std::fs::read_to_string(
        repo_root().join("apps/desktop/src/features/holdings/HoldingsScreen.tsx"),
    )
    .unwrap();
    let picker = std::fs::read_to_string(
        repo_root().join("apps/desktop/src/features/shared/pickers/UnassignedSellTable.tsx"),
    )
    .unwrap();
    let host = std::fs::read_to_string(repo_root().join("crates/application-core/src/cart.rs"))
        .unwrap();
    assert!(
        !holdings.contains("Sell activity id") && !holdings.contains("assignActivityId"),
        "Holdings must not ask the owner to type a sell activity UUID"
    );
    assert!(
        holdings.contains("if (!lotId || !selectedSell) return;")
            && holdings.contains("disabled={busy || writesBlocked || !lotId || !selectedSell}"),
        "Assign lot stays off until an unassigned sell is picked"
    );
    assert!(
        holdings.contains("No unassigned sell for this symbol")
            && holdings.contains("Cart Confirm sell assigns the lot in the same step"),
        "Holdings leftover copy names cart as the assigned-sale path"
    );
    assert!(
        picker.contains("aria-label=\"Unassigned sells\"")
            && picker.contains("Choose sell ${sell.occurredOn}")
            && !picker.contains("Choose sell ${sell.activityId}"),
        "unassigned sell labels are date/account/amount, not UUIDs"
    );
    assert!(
        host.contains("lot_assign(\n                sell.lot_id,")
            && host.contains("cart_execute_step_add("),
        "Cart Confirm sell still assigns in the same command"
    );
}

#[test]
fn plan_management_shows_typed_change_and_shares_are_not_a_missing_lot() {
    let screen = std::fs::read_to_string(
        repo_root().join("apps/desktop/src/features/position-details/PositionDetailsScreen.tsx"),
    )
    .unwrap();
    let app = std::fs::read_to_string(repo_root().join("apps/desktop/src/App.tsx")).unwrap();
    let ui = std::fs::read_to_string(repo_root().join("packages/ui-components/src/index.tsx")).unwrap();
    let plan_rs =
        std::fs::read_to_string(repo_root().join("crates/storage-sqlite/src/plan.rs")).unwrap();

    let snap = screen
        .split("id=\"hub-calculator\"")
        .nth(1)
        .unwrap()
        .split("id=\"hub-plan\"")
        .next()
        .unwrap();
    let shares = snap.find("sharesMinor > 0").expect("shares check");
    let does_not_pay = snap
        .find("Cadence is does not pay")
        .expect("cadence omission");
    let neither = snap
        .find("neither open shares nor a Plan")
        .expect("shares and plan omission");
    let no_lot = snap.find("needs Plan + first lot").expect("no-share copy");
    assert!(
        does_not_pay < shares && shares < neither && neither < no_lot,
        "holdings shares must not produce the missing-lot sentence"
    );
    assert!(
        !snap.contains("is not on the Calculator list"),
        "an omitted row names the failed check"
    );

    let plan = screen.split("id=\"hub-plan\"").nth(1).unwrap();
    assert!(plan.contains("aria-label=\"Plan amount\""));
    assert!(plan.contains("FWD at this amount"));
    assert!(plan.contains("planFwdAtAmountBps("));
    assert!(plan.contains("meanNewestPays(pays, 3)"));
    assert!(plan.contains("${Math.min(payCount, 3)} of 3, unknown"));
    assert!(plan.contains("minPaidDeclaration("));
    assert!(plan.contains("planDecisionImpact("));
    assert!(plan.contains("formatScale6(impact.perShareDeltaUnits)"));
    assert!(plan.contains("formatScale6(impact.paymentDeltaUnits)"));
    assert!(plan.contains("formatScale6(impact.annualDeltaUnits)"));
    assert!(plan.contains("${formatUsd(avg, 2)} (${Math.min(payCount, 6)} of 6)"));
    let plan_section = plan.split("id=\"hub-identity\"").next().unwrap();
    assert!(plan_section.contains("aria-label=\"Stored Plan decision reason\""));
    assert!(plan_section.contains("planReason === \"Match Most Current\""));
    assert!(plan_section.contains("planReason === \"Match Avg 6 (owner typed)\" && avg != null"));
    assert!(!screen.contains("Use Most Current as Plan"));
    assert!(!screen.contains("Use Avg 6 as Plan"));
    let identity = screen
        .split("id=\"hub-identity\"")
        .nth(1)
        .unwrap()
        .split("id=\"hub-calculator\"")
        .next()
        .unwrap();
    assert!(identity.contains("two-col-facts"));
    let yields = screen
        .split("id=\"hub-plan-yields\"")
        .nth(1)
        .unwrap()
        .split("id=\"hub-accounts\"")
        .next()
        .unwrap();
    assert!(yields.contains("two-col-facts"));
    assert!(
        screen.contains("className=\"two-col-facts\" aria-label=\"Position dossier\"")
            && plan_section.contains("aria-label=\"Stored incomplete analysis reason\"")
    );
    assert!(screen.contains("It does not store the Plan."));
    assert!(screen.contains("aria-label=\"Collector gaps\""));
    assert!(app.contains("aria-label=\"Collector gaps\""));
    let fwd = ui
        .split("export function planFwdAtAmountBps")
        .nth(1)
        .unwrap()
        .split("function paidInViewCents")
        .next()
        .unwrap();
    assert!(fwd.contains("amountMinor * periods * 1_000_000"));
    assert!(
        !fwd.contains("cellCents"),
        "FWD must not round the typed per-share to a cent first"
    );

    let impact = ui
        .split("export function planDecisionImpact")
        .nth(1)
        .unwrap()
        .split("export function formatScale6")
        .next()
        .unwrap();
    assert!(impact.contains("moneyUnits(input.nextMinor, input.nextScale)"));
    assert!(impact.contains("Math.round(perShareDeltaUnits * quantity)"));
    assert!(
        !impact.contains("cellCents"),
        "the typed change must not round each share to a cent first"
    );
    fn units(minor: i64, scale: u32) -> i64 {
        minor * 10i64.pow(6 - scale)
    }
    let per_share = units(23_461, 5) - units(2_300, 4);
    let payment = per_share * 20;
    assert_ne!(per_share, 0);
    assert_ne!(payment, 0);

    let upsert = plan_rs
        .split("pub async fn position_characteristic_upsert")
        .nth(1)
        .unwrap();
    let keep = upsert.find("stored_owner_tier").expect("tier keep");
    let branch = upsert
        .find("payment_frequency.trim().is_empty()")
        .expect("both upsert paths");
    assert!(
        keep < branch,
        "a blank tier is replaced before either characteristic write"
    );
}

#[test]
fn stored_tier_selects_on_open_and_roc_url_is_a_link() {
    let screen = std::fs::read_to_string(
        repo_root().join("apps/desktop/src/features/position-details/PositionDetailsScreen.tsx"),
    )
    .unwrap();
    let app = std::fs::read_to_string(repo_root().join("apps/desktop/src/App.tsx")).unwrap();
    let queries =
        std::fs::read_to_string(repo_root().join("crates/application-core/src/queries.rs")).unwrap();
    let load = app.split("const applyInvestment").nth(1).expect("load");
    assert!(
        load.contains("RISK_TIERS.includes(body.riskTier)")
            && load.contains("setOwnerRiskChoice(body.riskTier)"),
        "opening a position selects the stored tier"
    );
    assert!(
        app.contains("RISK_TIERS.includes(storedTier)"),
        "complete research keeps a stored tier ahead of a suggestion"
    );
    assert!(screen.contains("Tier is stored as ${storedTier}."));
    assert!(screen.contains("<a href={rocUrl}>ROC URL Data</a>"));
    assert!(screen.contains("aria-label=\"Assigned tier\""));
    assert!(screen.contains("aria-label=\"ROC\""));
    assert!(screen.contains("ROC previous year"));
    assert!(screen.contains("ROC current year"));
    assert!(screen.contains("roc-facts"));
    assert!(!screen.contains("research-notes-overview"));
    assert!(!screen.contains("aria-label=\"Set risk\""));
    assert!(!screen.contains("aria-label=\"Position risk\""));
    assert!(screen.contains("className=\"fact-strip\""));
    assert!(screen.contains("className=\"fact-grid\""));
    assert!(
        !queries.contains("roc source:"),
        "confirming ROC must not replace owner notes"
    );
}
