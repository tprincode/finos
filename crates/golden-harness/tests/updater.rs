use std::fs;
use std::path::Path;

use application_core::contracts::{
    CommandRequest, QueryRequest, FINANCE_CLIENT_CONTRACT_VERSION,
};
use application_core::queries::{execute_command_on, execute_query_on};
use golden_harness::repo_root;
use serde_json::Value;
use storage_sqlite::LocalPlatform;
use uuid::Uuid;

const GITHUB_LATEST_JSON: &str =
    "https://github.com/EVTom/finos/releases/latest/download/latest.json";

fn cmd(name: &str, body: serde_json::Value) -> CommandRequest {
    CommandRequest {
        contract_version: FINANCE_CLIENT_CONTRACT_VERSION.to_string(),
        command_name: name.to_string(),
        correlation_id: Uuid::new_v4(),
        body_json: Some(body.to_string()),
        expected_version: None,
    }
}

fn qry(name: &str) -> QueryRequest {
    QueryRequest {
        contract_version: FINANCE_CLIENT_CONTRACT_VERSION.to_string(),
        query_name: name.to_string(),
        correlation_id: Uuid::new_v4(),
        body_json: None,
    }
}

async fn must_ok(platform: &LocalPlatform, name: &str, body: serde_json::Value) -> serde_json::Value {
    let result = execute_command_on(platform, platform, cmd(name, body)).await;
    assert!(result.ok, "{name} failed: {:?}", result.error_code);
    serde_json::from_str(result.body_json.as_deref().unwrap_or("{}")).unwrap()
}

async fn query_json(platform: &LocalPlatform, name: &str) -> serde_json::Value {
    let result = execute_query_on(platform, platform, qry(name)).await;
    assert!(result.ok, "{name} failed: {:?}", result.error_code);
    serde_json::from_str(result.body_json.as_deref().unwrap_or("{}")).unwrap()
}

fn skip_dir(name: &str) -> bool {
    matches!(
        name,
        "target" | "node_modules" | ".git" | "updater-keys" | "dist" | "data"
    )
}

fn looks_like_minisign_secret(text: &str) -> bool {
    let prefix = "untrusted comment: minisign ";
    text.contains(&format!("{prefix}encrypted secret key"))
        || text.contains(&format!("{prefix}secret key"))
}

#[test]
fn updater_config_has_github_endpoint_and_pubkey_without_private_key() {
    let root = repo_root();
    let gitignore = fs::read_to_string(root.join(".gitignore")).expect(".gitignore");
    assert!(
        gitignore.contains("apps/desktop/src-tauri/updater-keys/"),
        "private updater key directory must be gitignored"
    );

    let raw = fs::read_to_string(root.join("apps/desktop/src-tauri/tauri.conf.json"))
        .expect("tauri.conf.json");
    let conf: Value = serde_json::from_str(&raw).expect("json");
    let updater = &conf["plugins"]["updater"];
    let endpoints = updater["endpoints"]
        .as_array()
        .expect("plugins.updater.endpoints array");
    assert!(
        endpoints
            .iter()
            .any(|e| e.as_str() == Some(GITHUB_LATEST_JSON)),
        "updater endpoint must be GitHub latest.json for this repo"
    );
    let pubkey = updater["pubkey"].as_str().expect("plugins.updater.pubkey");
    assert!(
        pubkey.len() > 32,
        "updater pubkey must be embedded public key material"
    );
    assert!(
        !looks_like_minisign_secret(pubkey),
        "tauri.conf.json must not contain a minisign private key"
    );
    assert!(
        !raw.contains("TAURI_SIGNING_PRIVATE_KEY"),
        "private key env must not be stored in tauri.conf.json"
    );

    let mut secret_hits = Vec::new();
    walk_for_secrets(&root, &mut secret_hits).expect("walk repo");
    assert!(
        secret_hits.is_empty(),
        "minisign private key must not be in the tree:\n{}",
        secret_hits.join("\n")
    );
}

fn walk_for_secrets(dir: &Path, hits: &mut Vec<String>) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if path.is_dir() {
            if skip_dir(name) {
                continue;
            }
            walk_for_secrets(&path, hits)?;
            continue;
        }
        if name.ends_with(".key") {
            hits.push(format!("key file {}", path.display()));
            continue;
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !matches!(ext, "json" | "toml" | "md" | "rs" | "ts" | "tsx" | "js" | "env" | "txt" | "yml" | "yaml") {
            continue;
        }
        if let Ok(text) = fs::read_to_string(&path) {
            if looks_like_minisign_secret(&text) {
                hits.push(format!("{}: minisign secret key material", path.display()));
            }
        }
    }
    Ok(())
}

#[tokio::test]
async fn updater_check_does_not_post_dividend_facts() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let account = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Taxable Brokerage", "kind": "taxable"}),
    )
    .await;
    let account_id = account["accountId"].as_str().unwrap();
    must_ok(
        &platform,
        "DividendActualRecord",
        serde_json::json!({
            "accountId": account_id,
            "occurredOn": "2026-06-15",
            "amountMinor": 50_000,
            "scale": 2,
            "idempotencyKey": "updater-div-1"
        }),
    )
    .await;
    let before = query_json(&platform, "DividendGet").await;
    assert_eq!(before["actualTotalMinor"].as_i64().unwrap(), 50_000);

    let check = query_json(&platform, "UpdaterCheckGet").await;
    assert_eq!(check["applied"].as_bool().unwrap(), false);
    assert_eq!(check["posted"].as_bool().unwrap(), false);
    assert_eq!(check["status"].as_str().unwrap(), "fail-closed");

    let after = query_json(&platform, "DividendGet").await;
    assert_eq!(after["actualTotalMinor"].as_i64().unwrap(), 50_000);
}
