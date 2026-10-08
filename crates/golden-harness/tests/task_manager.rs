//! Household Task Manager is not collector work_ticket.

use application_core::contracts::{
    CommandRequest, QueryRequest, FINANCE_CLIENT_CONTRACT_VERSION,
};
use application_core::queries::{execute_command_on, execute_query_on};
use application_core::task::magi_week_title;
use golden_harness::repo_root;
use storage_sqlite::LocalPlatform;
use uuid::Uuid;

fn cmd(name: &str, body: serde_json::Value) -> CommandRequest {
    CommandRequest {
        contract_version: FINANCE_CLIENT_CONTRACT_VERSION.to_string(),
        command_name: name.to_string(),
        correlation_id: Uuid::new_v4(),
        body_json: Some(body.to_string()),
        expected_version: None,
    }
}

fn qry(name: &str, body: serde_json::Value) -> QueryRequest {
    QueryRequest {
        contract_version: FINANCE_CLIENT_CONTRACT_VERSION.to_string(),
        query_name: name.to_string(),
        correlation_id: Uuid::new_v4(),
        body_json: Some(body.to_string()),
    }
}

async fn must_cmd(platform: &LocalPlatform, name: &str, body: serde_json::Value) -> serde_json::Value {
    let result = execute_command_on(platform, platform, cmd(name, body)).await;
    assert!(result.ok, "{name} failed: {:?}", result.error_code);
    serde_json::from_str(result.body_json.as_deref().unwrap_or("{}")).unwrap()
}

async fn must_qry(platform: &LocalPlatform, name: &str, body: serde_json::Value) -> serde_json::Value {
    let result = execute_query_on(platform, platform, qry(name, body)).await;
    assert!(result.ok, "{name} failed: {:?}", result.error_code);
    serde_json::from_str(result.body_json.as_deref().unwrap_or("{}")).unwrap()
}

async fn empty_platform() -> (tempfile::TempDir, LocalPlatform) {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    (dir, platform)
}

#[test]
fn household_task_tables_are_not_work_ticket() {
    let root = repo_root();
    let ticket = std::fs::read_to_string(
        root.join("crates/storage-sqlite/migrations/0029_work_ticket.sql"),
    )
    .unwrap();
    let household = std::fs::read_to_string(
        root.join("crates/storage-sqlite/migrations/0062_household_task.sql"),
    )
    .unwrap();
    assert!(
        ticket.contains("security_id TEXT NOT NULL"),
        "collector work_ticket stays security-scoped"
    );
    assert!(
        household.contains("CREATE TABLE task_rule")
            && household.contains("CREATE TABLE task")
            && household.contains("magi_cliff_over")
            && !household.contains("work_ticket"),
        "0062 is household task, not work_ticket"
    );
    let domain = std::fs::read_to_string(root.join("crates/financial-domain/src/work_ticket.rs"))
        .unwrap();
    assert!(
        !domain.contains("magi_cliff_over") && !domain.contains("task_rule"),
        "work_ticket.rs is unchanged"
    );
}

#[test]
fn tools_menu_opens_task_manager() {
    let root = repo_root();
    let app = std::fs::read_to_string(root.join("apps/desktop/src/App.tsx")).unwrap();
    let screen = std::fs::read_to_string(
        root.join("apps/desktop/src/features/task-manager/TaskManager.tsx"),
    )
    .unwrap();
    let ahead = std::fs::read_to_string(
        root.join("apps/desktop/src/features/cash/WeekAhead.tsx"),
    )
    .unwrap();
    let table = std::fs::read_to_string(
        root.join("apps/desktop/src/features/task-manager/ReminderTable.tsx"),
    )
    .unwrap();
    let catalog = std::fs::read_to_string(root.join("docs/architecture/ui-modules.json")).unwrap();
    assert!(
        catalog.contains("\"id\": \"task-manager\"")
            && catalog.contains("features/task-manager"),
        "Task Manager stays in the components catalog"
    );
    assert!(
        app.contains("WeekAheadPanel") || app.contains("task-manager"),
        "App still hosts Week Ahead (and Task Manager when mounted)"
    );
    assert!(
        screen.contains("aria-label=\"Task Manager\"") && screen.contains("<h2>Task Manager</h2>"),
        "owner must see the screen heading"
    );
    assert!(
        screen.contains("Open this week")
            && screen.contains("Snoozed")
            && screen.contains("Done (this week)")
            && screen.contains("aria-label=\"Add task\"")
            // The heading carries the nav anchor now, so match its text, not the bare tag.
            && screen.contains(">Add/View Task</h3>")
            && screen.contains("ReminderTable")
            && screen.contains("taskReminderRows")
            && screen.contains("taskRuleText")
            && screen.contains("aria-label=\"Built task rules\"")
            && screen.contains("magi_cliff_over")
            && screen.contains("plan_under")
            && screen.contains("plan_over")
            && screen.contains("3 periods")
            && screen.contains("5 periods")
            && screen.contains("<h3>Ticket functions</h3>")
            && screen.contains("Tickets are collector exceptions. Tasks are weekly reminders.")
            && screen.contains("aria-label=\"Ticket functions\"")
            && screen.contains("declaration_plan_mismatch")
            && screen.contains("declaration_amount_variation")
            && !screen.contains("onOpenWorkTicket")
            && !screen.contains("Snooze till next plan week")
            && !screen.contains("Enable magi_cliff_over"),
        "sections and Add task stay on the screen"
    );
    assert!(
        table.contains("function monthDay")
            && table.contains("return `${MONTHS[m - 1]} ${d}`")
            && table.contains("Dividend Plan")
            && table.contains("aria-label={`Open ${symbol} in Position Details`}")
            && table.contains("aria-label={snoozeAria ? snoozeAria(row.title) : `Snooze ${row.title}`}")
            && table.contains("Work ticket")
            && table.contains(
                "const ticketEnabled = magi ? Boolean(onOpenMagi) : Boolean(symbol && onOpenSymbol)"
            )
            && table.contains("disabled={!ticketEnabled}")
            && table.contains("onOpenSymbol(symbol)")
            && table.contains("onOpenMagi(row.taskId)"),
        "due date is month and day, plan domain reads Dividend Plan, the symbol opens Position Details, and MAGI opens its ticket"
    );
    assert!(
        app.contains("openPositionHub(symbol, \"plan\")"),
        "a Dividend Plan task opens Position Details on Plan Management"
    );
    assert!(
        app.contains("onAddElement={() => {")
            && app.contains("setElementEditorOpen(true)")
            && app.contains("goCmDesk(\"elements\", \"Element Management\")"),
        "Add element opens the existing element editor"
    );
    assert!(
        !screen.contains("<td>{row.taskId}</td>") && !screen.contains(">{row.taskId}<"),
        "do not show UUIDs as the owner label"
    );
    assert!(
        ahead.contains("aria-label=\"Week ahead\"")
            && ahead.contains("ReminderTable")
            && ahead.contains("taskReminderRows")
            && ahead.contains("<h3>Tasks</h3>")
            && ahead.contains("<h3>Elements</h3>")
            && ahead.contains("aria-label=\"Add element\"")
            && ahead.contains("onAddElement")
            && ahead.contains("till next plan week")
            && ahead.contains("Snooze till tomorrow")
            && !ahead.contains("Check tomorrow"),
        "Week Ahead uses the shared table; task snooze is the next plan week and element snooze is tomorrow"
    );
    assert_eq!(
        magi_week_title(248_580),
        "MAGI over cliff by $2,485.80 — resolve or snooze till next plan week."
    );
}

#[tokio::test]
async fn magi_cliff_sync_opens_refreshes_and_completes() {
    let (_dir, platform) = empty_platform().await;
    let rules = must_qry(&platform, "TaskRuleList", serde_json::json!({})).await;
    let codes: Vec<_> = rules["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["code"].as_str().unwrap().to_string())
        .collect();
    assert!(codes.contains(&"magi_cliff_over".to_string()));

    let opened = must_cmd(
        &platform,
        "MagiCliffTaskSync",
        serde_json::json!({
            "asOfDate": "2026-09-30",
            "overageMinor": 248580,
            "creditAtRiskMinor": 1805400,
            "suggestions": ["Cut remaining Traditional IRA draws by $2,485.80."]
        }),
    )
    .await;
    assert_eq!(opened["action"], "opened");
    let first_id = opened["task"]["taskId"].as_str().unwrap().to_string();

    let refreshed = must_cmd(
        &platform,
        "MagiCliffTaskSync",
        serde_json::json!({
            "asOfDate": "2026-09-30",
            "overageMinor": 250000,
            "creditAtRiskMinor": 1805400,
            "suggestions": ["Cut remaining Traditional IRA draws by $2,500.00."]
        }),
    )
    .await;
    assert_eq!(refreshed["action"], "refreshed");
    assert_eq!(refreshed["task"]["taskId"], first_id);

    let listed = must_qry(
        &platform,
        "TaskList",
        serde_json::json!({"status": "open", "weekStart": "2026-09-26"}),
    )
    .await;
    assert_eq!(listed["items"].as_array().unwrap().len(), 1);

    let ahead = must_qry(
        &platform,
        "WeekAheadGet",
        serde_json::json!({"asOfDate": "2026-09-30"}),
    )
    .await;
    let tasks = ahead["tasks"].as_array().unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(
        tasks[0]["title"],
        "MAGI over cliff by $2,500.00 — resolve or snooze till next plan week."
    );

    let done = must_cmd(
        &platform,
        "MagiCliffTaskSync",
        serde_json::json!({
            "asOfDate": "2026-09-30",
            "overageMinor": 0,
            "creditAtRiskMinor": 0,
            "suggestions": []
        }),
    )
    .await;
    assert_eq!(done["action"], "completed");
    assert_eq!(done["task"]["status"], "done");

    let open_after = must_qry(
        &platform,
        "TaskList",
        serde_json::json!({"status": "open", "weekStart": "2026-09-26"}),
    )
    .await;
    assert!(open_after["items"].as_array().unwrap().is_empty());

    let skip_done = must_cmd(
        &platform,
        "MagiCliffTaskSync",
        serde_json::json!({
            "asOfDate": "2026-09-30",
            "overageMinor": 248580,
            "creditAtRiskMinor": 1805400,
            "suggestions": []
        }),
    )
    .await;
    assert_eq!(skip_done["action"], "skipped");
}

#[tokio::test]
async fn ignore_hides_until_next_saturday_then_can_raise() {
    let (_dir, platform) = empty_platform().await;
    let opened = must_cmd(
        &platform,
        "MagiCliffTaskSync",
        serde_json::json!({
            "asOfDate": "2026-09-30",
            "overageMinor": 248580,
            "creditAtRiskMinor": 1805400,
            "suggestions": []
        }),
    )
    .await;
    let task_id = opened["task"]["taskId"].as_str().unwrap();
    must_cmd(
        &platform,
        "TaskResolve",
        serde_json::json!({
            "taskId": task_id,
            "how": "ignored_until",
            "ignoreUntil": "2026-10-03"
        }),
    )
    .await;

    let skip = must_cmd(
        &platform,
        "MagiCliffTaskSync",
        serde_json::json!({
            "asOfDate": "2026-09-30",
            "overageMinor": 248580,
            "creditAtRiskMinor": 1805400,
            "suggestions": []
        }),
    )
    .await;
    assert_eq!(skip["action"], "skipped");

    let ahead = must_qry(
        &platform,
        "WeekAheadGet",
        serde_json::json!({"asOfDate": "2026-09-30"}),
    )
    .await;
    assert!(ahead["tasks"].as_array().unwrap().is_empty());

    let next_week = must_cmd(
        &platform,
        "MagiCliffTaskSync",
        serde_json::json!({
            "asOfDate": "2026-10-03",
            "overageMinor": 248580,
            "creditAtRiskMinor": 1805400,
            "suggestions": []
        }),
    )
    .await;
    assert_eq!(next_week["action"], "opened");
    assert_eq!(next_week["task"]["weekStart"], "2026-10-03");

    let added = must_cmd(
        &platform,
        "TaskAdd",
        serde_json::json!({
            "title": "Call the CPA",
            "dueOn": "2026-10-02",
            "note": ""
        }),
    )
    .await;
    assert_eq!(added["domain"], "manual");
    assert_eq!(added["title"], "Call the CPA");
    assert!(!added["code"].as_str().unwrap().is_empty());
}

#[test]
fn magi_cliff_over_is_one_household_task() {
    let src = std::fs::read_to_string(repo_root().join("crates/application-core/src/task.rs")).unwrap();
    assert!(src.contains("pub const CODE_MAGI_CLIFF_OVER: &str = \"magi_cliff_over\""));
    let magi = src
        .split("pub async fn magi_cliff_task_sync")
        .nth(1)
        .expect("magi_cliff_task_sync")
        .split("fn magi_payload")
        .next()
        .unwrap();
    assert!(
        magi.contains("title: \"Resolve MAGI cliff gap\".into()"),
        "MAGI title stays the household sentence"
    );
    assert!(
        magi.contains("code: CODE_MAGI_CLIFF_OVER.into()"),
        "MAGI code is the one household code"
    );
    assert!(
        !magi.contains("security_id") && !magi.contains("symbol") && !magi.contains("plan_under"),
        "magi_cliff_task_sync has no symbol and no per-security code"
    );
}

const PLAN_MINOR: i64 = 100;
const AS_OF: &str = "2026-10-03";

async fn seed_quarterly_pays(
    platform: &LocalPlatform,
    symbol: &str,
    pays: &[(&str, Option<i64>)],
) -> String {
    let security = must_cmd(
        platform,
        "SecurityRegister",
        serde_json::json!({"symbol": symbol, "name": symbol}),
    )
    .await;
    let security_id = security["securityId"].as_str().unwrap().to_string();
    must_cmd(
        platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": security_id,
            "paymentFrequency": "Quarterly",
            "divType": "DIV-1",
            "riskTier": "HighRisk"
        }),
    )
    .await;
    for (period, minor) in pays {
        let mut body = serde_json::json!({
            "securityId": security_id,
            "amountScale": 2,
            "paymentPeriod": period,
            "source": "provider-site",
            "enteredAt": "2026-10-03"
        });
        if let Some(amount) = minor {
            body["amountPerShareMinor"] = serde_json::json!(amount);
        } else {
            body["amountPerShareMinor"] = serde_json::Value::Null;
        }
        must_cmd(platform, "IssuerDeclarationRecord", body).await;
    }
    must_cmd(
        platform,
        "PlanHistoryConfirm",
        serde_json::json!({
            "securityId": security_id,
            "amountPerShareMinor": PLAN_MINOR,
            "amountScale": 2,
            "planningPeriodsPerYear": 4,
            "effectiveFrom": "2025-01-01",
            "decisionReason": "owner",
            "incompleteAnalysisReason": "Fewer than six observations"
        }),
    )
    .await;
    security_id
}

async fn saturday(platform: &LocalPlatform) -> serde_json::Value {
    must_cmd(
        platform,
        "PlanSaturdayTaskSync",
        serde_json::json!({"asOfDate": AS_OF}),
    )
    .await
}

async fn open_tasks(platform: &LocalPlatform) -> Vec<serde_json::Value> {
    let listed = must_qry(
        platform,
        "TaskList",
        serde_json::json!({"status": "open"}),
    )
    .await;
    listed["items"].as_array().unwrap().clone()
}

fn plan_tasks<'a>(items: &'a [serde_json::Value], prefix: &str) -> Vec<&'a serde_json::Value> {
    items
        .iter()
        .filter(|row| row["code"].as_str().unwrap_or("").starts_with(prefix))
        .collect()
}

#[tokio::test]
async fn two_periods_under_plan_open_no_task() {
    let (_dir, platform) = empty_platform().await;
    let security_id = seed_quarterly_pays(
        &platform,
        "PLN1",
        &[("2026-04-15", Some(80)), ("2026-07-15", Some(80))],
    )
    .await;
    let synced = saturday(&platform).await;
    assert_eq!(synced["opened"], 0);
    let open = open_tasks(&platform).await;
    assert!(
        plan_tasks(&open, &format!("plan_under:{security_id}")).is_empty(),
        "two unders stay under the threshold of 3"
    );
}

#[tokio::test]
async fn three_consecutive_unders_open_one_adjust_task() {
    let (_dir, platform) = empty_platform().await;
    let security_id = seed_quarterly_pays(
        &platform,
        "PLN1",
        &[
            ("2026-01-15", Some(80)),
            ("2026-04-15", Some(80)),
            ("2026-07-15", Some(80)),
        ],
    )
    .await;
    let synced = saturday(&platform).await;
    assert_eq!(synced["opened"], 1);
    let ahead = must_qry(
        &platform,
        "WeekAheadGet",
        serde_json::json!({"asOfDate": AS_OF}),
    )
    .await;
    let ahead_titles: Vec<_> = ahead["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["title"].as_str().unwrap().to_string())
        .collect();
    assert!(
        ahead_titles
            .iter()
            .any(|title| title == "PLN1 — Plan amount is wrong and must be adjusted."),
        "week ahead names the symbol in front of the stored sentence: {ahead_titles:?}"
    );
    let open = open_tasks(&platform).await;
    let unders = plan_tasks(&open, "plan_under:");
    assert_eq!(unders.len(), 1);
    assert_eq!(unders[0]["code"], format!("plan_under:{security_id}"));
    assert_eq!(unders[0]["title"], "Plan amount is wrong and must be adjusted.");
    assert_eq!(unders[0]["domain"], "plan");
    let payload: serde_json::Value = serde_json::from_str(unders[0]["payloadJson"].as_str().unwrap()).unwrap();
    assert_eq!(payload["symbol"], "PLN1");
    assert_eq!(payload["streak"], serde_json::json!(3));
    let task_id = unders[0]["taskId"].as_str().unwrap().to_string();

    let again = saturday(&platform).await;
    assert_eq!(again["opened"], 0);
    assert_eq!(again["refreshed"], 1);
    let still = open_tasks(&platform).await;
    assert_eq!(plan_tasks(&still, "plan_under:")[0]["taskId"], task_id);

    must_cmd(
        &platform,
        "IssuerDeclarationRecord",
        serde_json::json!({
            "securityId": security_id,
            "amountPerShareMinor": PLAN_MINOR,
            "amountScale": 2,
            "paymentPeriod": "2026-10-15",
            "source": "provider-site",
            "enteredAt": "2026-10-03"
        }),
    )
    .await;
    let closed = saturday(&platform).await;
    assert_eq!(closed["completed"], 1);
    let after = open_tasks(&platform).await;
    assert!(plan_tasks(&after, "plan_under:").is_empty());
}

#[tokio::test]
async fn pay_at_plan_after_two_unders_opens_no_task() {
    let (_dir, platform) = empty_platform().await;
    seed_quarterly_pays(
        &platform,
        "PLN1",
        &[
            ("2026-01-15", Some(80)),
            ("2026-04-15", Some(80)),
            ("2026-07-15", Some(PLAN_MINOR)),
        ],
    )
    .await;
    let synced = saturday(&platform).await;
    assert_eq!(synced["opened"], 0);
    let open = open_tasks(&platform).await;
    assert!(plan_tasks(&open, "plan_under:").is_empty());
    assert!(plan_tasks(&open, "plan_over:").is_empty());
}

#[tokio::test]
async fn blank_pay_does_not_count_as_under() {
    let (_dir, platform) = empty_platform().await;
    seed_quarterly_pays(
        &platform,
        "PLN1",
        &[
            ("2026-01-15", Some(80)),
            ("2026-04-15", Some(80)),
            ("2026-07-15", None),
        ],
    )
    .await;
    let synced = saturday(&platform).await;
    assert_eq!(synced["opened"], 0);
    let open = open_tasks(&platform).await;
    assert!(
        plan_tasks(&open, "plan_under:").is_empty(),
        "a missing pay is blank, not $0, and breaks the under run"
    );
}

#[tokio::test]
async fn five_consecutive_overs_open_one_validate_task() {
    let (_dir, platform) = empty_platform().await;
    let security_id = seed_quarterly_pays(
        &platform,
        "PLN1",
        &[
            ("2025-07-15", Some(150)),
            ("2025-10-15", Some(150)),
            ("2026-01-15", Some(150)),
            ("2026-04-15", Some(150)),
            ("2026-07-15", Some(150)),
        ],
    )
    .await;
    let synced = saturday(&platform).await;
    assert_eq!(synced["opened"], 1);
    let open = open_tasks(&platform).await;
    assert!(plan_tasks(&open, "plan_under:").is_empty());
    let overs = plan_tasks(&open, "plan_over:");
    assert_eq!(overs.len(), 1);
    assert_eq!(overs[0]["code"], format!("plan_over:{security_id}"));
    assert_eq!(
        overs[0]["title"],
        "Validate Plan amount — paid above Plan for 5 periods."
    );
    let payload: serde_json::Value = serde_json::from_str(overs[0]["payloadJson"].as_str().unwrap()).unwrap();
    assert_eq!(payload["symbol"], "PLN1");
    assert_eq!(payload["streak"], serde_json::json!(5));
}

#[tokio::test]
async fn three_unders_and_magi_cliff_are_two_tasks() {
    let (_dir, platform) = empty_platform().await;
    let security_id = seed_quarterly_pays(
        &platform,
        "PLN1",
        &[
            ("2026-01-15", Some(80)),
            ("2026-04-15", Some(80)),
            ("2026-07-15", Some(80)),
        ],
    )
    .await;
    saturday(&platform).await;
    must_cmd(
        &platform,
        "MagiCliffTaskSync",
        serde_json::json!({
            "asOfDate": AS_OF,
            "overageMinor": 248580,
            "creditAtRiskMinor": 1805400,
            "suggestions": []
        }),
    )
    .await;
    let open = open_tasks(&platform).await;
    assert_eq!(open.len(), 2);
    let magi: Vec<_> = open
        .iter()
        .filter(|row| row["code"] == "magi_cliff_over")
        .collect();
    assert_eq!(magi.len(), 1);
    assert_eq!(magi[0]["title"], "Resolve MAGI cliff gap");
    assert!(
        !magi[0]["code"].as_str().unwrap().contains("PLN1")
            && !magi[0]["code"].as_str().unwrap().contains(':'),
        "MAGI stays the single household code"
    );
    let unders = plan_tasks(&open, "plan_under:");
    assert_eq!(unders.len(), 1);
    assert_eq!(unders[0]["code"], format!("plan_under:{security_id}"));
}
