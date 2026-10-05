//! Mobile publish head + Week Ahead confirm outbox (cloud folder transport).

use application_core::contracts::{CommandRequest, QueryRequest, FINANCE_CLIENT_CONTRACT_VERSION};
use application_core::queries::{execute_command_on, execute_query_on};
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
    assert!(result.ok, "{name} failed: {:?} {:?}", result.error_code, result.body_json);
    serde_json::from_str(result.body_json.as_deref().unwrap_or("{}")).unwrap()
}

async fn must_qry(platform: &LocalPlatform, name: &str, body: serde_json::Value) -> serde_json::Value {
    let result = execute_query_on(platform, platform, qry(name, body)).await;
    assert!(result.ok, "{name} failed: {:?} {:?}", result.error_code, result.body_json);
    serde_json::from_str(result.body_json.as_deref().unwrap_or("{}")).unwrap()
}

#[tokio::test]
async fn mobile_publish_writes_head_and_outbox_drains_confirm() {
    let dir = tempfile::tempdir().unwrap();
    let publish = dir.path().join("cloud-transport");
    std::fs::create_dir_all(&publish).unwrap();
    std::env::set_var("FINOS_MOBILE_PUBLISH_DIR", &publish);

    let app_dir = dir.path().join("app-data");
    let platform = LocalPlatform::open(&app_dir).await.expect("open sqlite");

    must_cmd(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Income", "kind": "ira"}),
    )
    .await;
    must_cmd(
        &platform,
        "CashElementSave",
        serde_json::json!({
            "account": "Income",
            "name": "MobileTest",
            "kind": "Withdrawal",
            "cadence": "weekly",
            "weekdayOrMonthDay": "saturday",
            "amountMinor": 10000,
            "asOfDate": "2026-09-30",
            "startOn": "2026-09-01",
            "stopOn": "",
            "occurrences": []
        }),
    )
    .await;

    let published = must_cmd(
        &platform,
        "MobilePublish",
        serde_json::json!({"asOfDate": "2026-09-30"}),
    )
    .await;
    assert_eq!(published["asOf"], "2026-09-30");
    let head_path = published["headPath"].as_str().unwrap();
    assert!(std::path::Path::new(head_path).exists());

    let head = must_qry(&platform, "MobileHeadGet", serde_json::json!({})).await;
    assert_eq!(head["asOf"], "2026-09-30");
    assert!(head.get("weekAhead").is_some());
    assert!(head["magiJson"].as_str().unwrap().contains("applicableThresholdMinor"));

    let ahead = must_qry(
        &platform,
        "WeekAheadGet",
        serde_json::json!({"asOfDate": "2026-09-30"}),
    )
    .await;
    let rows = ahead["rows"].as_array().unwrap();
    assert!(
        !rows.is_empty(),
        "week ahead should list MobileTest: {ahead}"
    );
    let occurrence_id = rows
        .iter()
        .find(|r| r["note"] == "MobileTest")
        .or(rows.first())
        .unwrap()["occurrenceId"]
        .as_str()
        .unwrap();

    let put = must_cmd(
        &platform,
        "MobileOutboxPut",
        serde_json::json!({
            "occurrenceId": occurrence_id,
            "asOfDate": "2026-09-30",
            "note": "phone confirm"
        }),
    )
    .await;
    assert_eq!(put["kind"], "week_ahead_confirm");
    assert!(std::path::Path::new(put["path"].as_str().unwrap()).exists());

    let drained = must_cmd(&platform, "MobileOutboxDrain", serde_json::json!({})).await;
    assert_eq!(
        drained["applied"].as_array().unwrap().len(),
        1,
        "{drained}"
    );
    assert!(drained["failed"].as_array().unwrap().is_empty(), "{drained}");

    std::env::remove_var("FINOS_MOBILE_PUBLISH_DIR");
}

#[test]
fn web_phone_shell_reads_mobile_head() {
    let web = std::fs::read_to_string(
        golden_harness::repo_root().join("apps/web/src/App.tsx"),
    )
    .unwrap();
    assert!(
        web.contains("aria-label=\"Mobile week ahead\""),
        "apps/web must show the phone read set from published head"
    );
    assert!(
        web.contains("MobileOutboxPut"),
        "phone shell offers Week Ahead confirm intent path"
    );
}

#[test]
fn desktop_force_publish_mobile_menu_exists() {
    let lib = std::fs::read_to_string(
        golden_harness::repo_root().join("apps/desktop/src-tauri/src/lib.rs"),
    )
    .unwrap();
    let app = std::fs::read_to_string(
        golden_harness::repo_root().join("apps/desktop/src/App.tsx"),
    )
    .unwrap();
    assert!(
        lib.contains("mobile-publish") && lib.contains("Force publish mobile head"),
        "File menu must expose Force publish mobile head"
    );
    assert!(
        app.contains("MobilePublish") && app.contains("MobileOutboxDrain"),
        "desktop must run MobilePublish and drain the outbox"
    );
}
