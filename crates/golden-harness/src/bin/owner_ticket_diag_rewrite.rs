//! Rewrite opaque open work-ticket reasons to Steps + Remediation.
//! Opens Profile A app-data (`%LOCALAPPDATA%\com.finos.desktop`).

use std::process::ExitCode;

use application_core::contracts::{CommandRequest, FINANCE_CLIENT_CONTRACT_VERSION};
use application_core::ports::canonical::Canonical;
use application_core::queries::execute_command_on;
use golden_harness::profile_a_app_dir;
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

#[tokio::main]
async fn main() -> ExitCode {
    let platform = match LocalPlatform::open(&profile_a_app_dir()).await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("open app-data failed: {e}");
            return ExitCode::FAILURE;
        }
    };
    let before = platform
        .work_ticket_list(None, Some("open".into()))
        .await
        .unwrap_or_default();
    println!("open_before={}", before.len());
    for t in &before {
        let opaque = !(t.reason.contains("Steps (")
            && t.reason.contains("Remediation:"));
        println!(
            "  {} {} opaque={} head={}",
            t.symbol,
            t.code,
            opaque,
            t.reason.lines().next().unwrap_or("")
        );
    }
    let result = execute_command_on(&platform, &platform, cmd("WorkTicketSyncMisses", serde_json::json!({}))).await;
    if !result.ok {
        eprintln!("WorkTicketSyncMisses failed: {:?}", result.error_code);
        return ExitCode::FAILURE;
    }
    let after = platform
        .work_ticket_list(None, Some("open".into()))
        .await
        .unwrap_or_default();
    println!("open_after={}", after.len());
    for t in &after {
        let ok = t.reason.contains("Steps (")
            && t.reason.contains("pass")
            && t.reason.contains("FAIL")
            && t.reason.contains("Remediation:");
        println!("--- {} {} diagnostics_ok={} ---", t.symbol, t.code, ok);
        println!("{}", t.reason);
    }
    ExitCode::SUCCESS
}
