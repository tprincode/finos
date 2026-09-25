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
    let sources = format!(
        "{app}\n{ui}\n{list}\n{trends_capture}\n{trends_charts}\n{cash_week_desk}\n{home_account_charts}\n{decl_chart}\n{import_wizard}\n{dividend_weeks}\n{cash_management}\n{cash_coverage}\n{home_dividend_plan}\n{home_trend_focus}\n{plan_horizon}\n{income_plan_screen}\n{collectors_screen}\n{collector_establish}"
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
        "aria-label=\"Add Position\"",
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
        "aria-label=\"Position fleet compare\"",
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
        "aria-label=\"Use Most Current as Plan\"",
        "aria-label=\"Save Process A research\"",
        "aria-label=\"Save stored facts\"",
        "aria-label=\"Position information\"",
        "aria-label=\"Position risk\"",
        "aria-label=\"Position frequency\"",
        "aria-label=\"Cancel position edits\"",
        "aria-label=\"Research\"",
        "aria-label=\"Cancel add lot edits\"",
        "aria-label=\"Cancel unsaved edits\"",
        "aria-label=\"Open screen with unsaved edits\"",
        "aria-label=\"Save owner period\"",
        "aria-label=\"Calculate window\"",
        "aria-label={`Apply ${tier}`}",
        "aria-label=\"Position dossier\"",
        "aria-label=\"Confirm Plan\"",
        "aria-label=\"Complete research\"",
        "aria-label=\"Fill research gaps\"",
        "aria-label=\"Research progress\"",
        "aria-label=\"Research notes\"",
        "aria-label=\"Research result\"",
        "aria-label=\"Owner risk choice\"",
        "aria-label=\"Set risk\"",
        "aria-label=\"Confirm ROC plan\"",
        "aria-label=\"Next wizard step\"",
        "aria-label=\"Previous wizard step\"",
        "aria-label=\"Wizard step guidance\"",
        "aria-label=\"Source research\"",
        "aria-label=\"Issuer site attempts\"",
        "aria-label=\"What we maintain\"",
        "aria-label=\"Maintenance strategy\"",
        "aria-label=\"Standing retrieval template\"",
        "aria-label=\"Source analytics\"",
        "aria-label=\"Future declaration strategy\"",
        "aria-label=\"New investment underlying\"",
        "aria-label=\"Look-through research\"",
        "aria-label=\"New investment theme strategy\"",
        "aria-label=\"New investment primary risk driver\"",
        "aria-label=\"New investment concentration\"",
        "aria-label=\"New investment volatility proxy\"",
        "aria-label=\"New investment tax character\"",
        "aria-label=\"Look-through risk suggestion\"",
        "aria-label=\"Position theme strategy\"",
        "aria-label=\"Position primary risk driver\"",
        "aria-label=\"Position concentration\"",
        "aria-label=\"Position volatility proxy\"",
        "aria-label=\"Position tax character\"",
        "aria-label=\"Open first lot\"",
        "aria-label=\"Save remaining payment dates\"",
        "aria-label=\"Remaining-year payment dates\"",
        "aria-label=\"Next payment date\"",
        "aria-label={`Remaining pay date ${pay.originalPayOn}`}",
        "aria-label=\"Record bull period\"",
        "aria-label=\"Record bear period\"",
        "aria-label=\"Bull start\"",
        "aria-label=\"Bull end\"",
        "aria-label=\"Bear start\"",
        "aria-label=\"Bear end\"",
        "aria-label=\"Record last price\"",
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
        "aria-label=\"Component registry\"",
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
        "aria-label=\"Issuer retrieve\"",
        "aria-label=\"Issuer retrieve miss summary\"",
        "aria-label=\"Application\"",
        "aria-label=\"Data position totals\"",
        "aria-label=\"Apply issuer sources from provider\"",
        "aria-label=\"Position div type\"",
        "aria-label=\"Needs ROC research\"",
        "aria-label=\"Position is active\"",
        "aria-label=\"Expected tax handling\"",
        "aria-label=\"Declaration weekday\"",
        "aria-label=\"Ex-date weekday\"",
        "aria-label=\"Payday weekday\"",
        "aria-label=\"Price source\"",
        "aria-label=\"Declaration source\"",
        "aria-label=\"Lookback count\"",
        "aria-label=\"Position completeness\"",
        "aria-label=\"Position master\"",
        "aria-label={`Open ${row.symbol} dossier`}",
        "aria-label={`Filter ${label}`}",
        "aria-label={`Clear ${label} filter`}",
        "aria-label=\"Clear filters\"",
        "aria-label=\"Retrieve declarations\"",
        "aria-label=\"Select week\"",
        "aria-label=\"Loading Data\"",
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
        "aria-label=\"Fidelity total\"",
        "aria-label=\"Schwab total\"",
        "aria-label=\"Finish this week on Cash Management\"",
        "aria-label=\"Cash week follow-up\"",
        "aria-label=\"Add cash activity\"",
        "aria-label=\"Saved cash weeks\"",
        "aria-label=\"Cash week overview\"",
        "aria-label=\"Trends weekly capture\"",
        "aria-label=\"Week capture grid\"",
        "aria-label=\"Income Total Balance\"",
        "aria-label=\"FI Roth Total Balance\"",
        "aria-label=\"Speculation Total Balance\"",
        "aria-label=\"Health Total Balance\"",
        "aria-label=\"Car Total Balance\"",
        "aria-label=\"Account 9 Total Balance\"",
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
        "aria-label=\"Cash management month\"",
        "aria-label=\"Cash Management distributions YTD\"",
        "aria-label=\"Distribution account totals\"",
        "aria-label=\"All distribution accounts\"",
        "aria-label=\"Distribution tax sections\"",
        "aria-label=\"Cash Management tax and ACA monitor\"",
        "aria-label=\"Cash Management Tax Planning\"",
        "aria-label=\"Tax Planning income\"",
        "aria-label=\"Tax Planning MAGI\"",
        "aria-label=\"Cash Management Coverage\"",
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
        app.contains("REGISTERED_DECLARATION_SOURCES"),
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
        charts.contains("Weekly Decl vs Plan"),
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
        decl_chart.contains("Decl — solid")
            && decl_chart.contains("Plan — dashed")
            && decl_chart.contains("type: \"dashed\"")
            && decl_chart.contains("symbol: \"none\""),
        "Decl vs Plan: solid Decl, dashed Plan, no circles"
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
