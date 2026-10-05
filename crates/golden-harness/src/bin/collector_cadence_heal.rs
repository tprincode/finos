//! Heal standing pay-date twins for every enabled collector (Weekly/Monthly/Quarterly).
//! Opens Profile A app-data (`%LOCALAPPDATA%\com.finos.desktop`).

use std::process::ExitCode;

use application_core::contracts::{CommandRequest, FINANCE_CLIENT_CONTRACT_VERSION};
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
    let as_of = chrono::Local::now().format("%Y-%m-%d").to_string();
    let result = execute_command_on(
        &platform,
        &platform,
        cmd(
            "CollectorCadenceHeal",
            serde_json::json!({ "asOfDate": as_of }),
        ),
    )
    .await;
    if !result.ok {
        eprintln!(
            "CollectorCadenceHeal failed: {:?}",
            result.error_code
        );
        return ExitCode::FAILURE;
    }
    println!("{}", result.body_json.as_deref().unwrap_or("{}"));
    ExitCode::SUCCESS
}
