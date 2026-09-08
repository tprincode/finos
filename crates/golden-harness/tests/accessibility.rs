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
        std::fs::read_to_string(repo_root().join("apps/desktop/src/TrendsCapture.tsx"))
            .expect("TrendsCapture.tsx");
    let trends_charts =
        std::fs::read_to_string(repo_root().join("apps/desktop/src/TrendsCharts.tsx"))
            .expect("TrendsCharts.tsx");
    let decl_chart =
        std::fs::read_to_string(repo_root().join("apps/desktop/src/DeclarationPaymentsChart.tsx"))
            .expect("DeclarationPaymentsChart.tsx");
    let import_wizard =
        std::fs::read_to_string(repo_root().join("apps/desktop/src/ImportWizard.tsx"))
            .expect("ImportWizard.tsx");
    let dividend_weeks =
        std::fs::read_to_string(repo_root().join("apps/desktop/src/DividendWeeks.tsx"))
            .expect("DividendWeeks.tsx");
    let sources = format!(
        "{app}\n{ui}\n{list}\n{trends_capture}\n{trends_charts}\n{decl_chart}\n{import_wizard}\n{dividend_weeks}"
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
        "aria-label=\"Dividend weeks\"",
        "aria-label=\"Dividend week table\"",
        "aria-label=\"Dividend performance period\"",
        "aria-label=\"Actual versus plan\"",
    ] {
        assert!(sources.contains(name), "missing import wizard name {name}");
    }
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
        "aria-label=\"Manual dividend\"",
        "aria-label=\"Manual dividend rows\"",
        "aria-label=\"Post manual dividends\"",
        "aria-label=\"Add manual dividend row\"",
        "aria-label=\"Income Plan\"",
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
        "aria-label=\"Save new investment facts\"",
        "aria-label=\"Save stored facts\"",
        "aria-label=\"Cancel position edits\"",
        "aria-label=\"Cancel new investment edits\"",
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
        "aria-label=\"Leave undecided\"",
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
        "aria-label=\"Current week\"",
        "aria-label=\"Previous week\"",
        "aria-label=\"Next week\"",
        "aria-label=\"Income plan by account\"",
        "aria-label=\"Income plan by position\"",
        "aria-label=\"Dividend weeks\"",
        "aria-label=\"Dividend week table\"",
        "aria-label=\"Dividend performance period\"",
        "aria-label=\"Actual versus plan\"",
        "aria-label=\"Filter holdings\"",
        "label=\"Filter calculator\"",
        "label=\"Filter position lots\"",
        "label=\"Filter dashboard\"",
        "aria-label=\"Trends weekly capture\"",
        "aria-label=\"Save Trends week\"",
        "aria-label=\"Correct Trends week\"",
        "aria-label=\"Close Trends week\"",
        "aria-label=\"Trends graphing period\"",
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
