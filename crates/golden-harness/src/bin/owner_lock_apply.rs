//! Apply owner cadence locks and AMDW 100% ROC from the cited Roundhill URL.
//! Does not invent 0%. Does not touch declaration last_run_ok.

use std::process::ExitCode;

use application_core::contracts::{
    CommandRequest, QueryRequest, FINANCE_CLIENT_CONTRACT_VERSION,
};
use application_core::queries::{execute_command_on, execute_query_on};
use golden_harness::profile_a_app_dir;
use storage_sqlite::LocalPlatform;
use uuid::Uuid;

const AMDW_ROC_URL: &str = "https://www.roundhillinvestments.com/social-disclosures";

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

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(msg) => {
            println!("{msg}");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("owner-lock-apply failed: {err}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<String, String> {
    let platform = LocalPlatform::open(&profile_a_app_dir())
        .await
        .map_err(|e| e.to_string())?;

    let set = execute_query_on(
        &platform,
        &platform,
        qry("CollectorSetGet", serde_json::json!({})),
    )
    .await;
    if !set.ok {
        return Err(set.error_code.unwrap_or_else(|| "collector_set_failed".into()));
    }
    let set_val: serde_json::Value =
        serde_json::from_str(set.body_json.as_deref().unwrap_or("{}")).unwrap_or_default();
    let items = set_val
        .get("items")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let weekly = ["AMDW", "QDTE", "RDTE", "TOPW", "XDTE", "YBTC"];
    let monthly = ["XPAY"];
    let mut lines = Vec::new();

    for (symbol, freq) in weekly
        .into_iter()
        .map(|s| (s, "Weekly"))
        .chain(monthly.into_iter().map(|s| (s, "Monthly")))
    {
        let item = items
            .iter()
            .find(|row| {
                row.get("symbol")
                    .and_then(|s| s.as_str())
                    .unwrap_or("")
                    .eq_ignore_ascii_case(symbol)
            })
            .ok_or_else(|| format!("{symbol} not in collector set"))?;
        let security_id = item
            .get("securityId")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let current = item
            .get("paymentFrequency")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if current != freq {
            return Err(format!(
                "{symbol} cadence is {current}; refused to replace lock with {freq}. Owner must name both the old and new value."
            ));
        }
        lines.push(format!("{symbol} cadence already {freq} (not rewritten)"));
    }

    let amdw = items
        .iter()
        .find(|row| {
            row.get("symbol")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .eq_ignore_ascii_case("AMDW")
        })
        .ok_or_else(|| "AMDW not in collector set".to_string())?;
    let amdw_id = amdw
        .get("securityId")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let today = chrono::Utc::now().date_naive().to_string();
    let roc = execute_command_on(
        &platform,
        &platform,
        cmd(
            "RocPlanConfirm",
            serde_json::json!({
                "securityId": amdw_id,
                "symbol": "AMDW",
                "rocPctMinor": 10000,
                "rocScale": 2,
                "ownerOverride": true,
                "source": "19a-1",
                "sourceUrl": AMDW_ROC_URL,
                "method": "owner-confirm",
                "establishedHow": "owner: 100% ROC from Roundhill social-disclosures",
                "asOfDate": today,
            }),
        ),
    )
    .await;
    if !roc.ok {
        return Err(format!(
            "AMDW ROC confirm failed: {}",
            roc.error_code.unwrap_or_default()
        ));
    }
    lines.push(format!("AMDW 100% ROC from {AMDW_ROC_URL}"));

    let tickets = execute_query_on(
        &platform,
        &platform,
        qry("WorkTicketList", serde_json::json!({})),
    )
    .await;
    if tickets.ok {
        let body: serde_json::Value =
            serde_json::from_str(tickets.body_json.as_deref().unwrap_or("{}")).unwrap_or_default();
        for item in body.get("items").and_then(|v| v.as_array()).cloned().unwrap_or_default()
        {
            let code = item.get("code").and_then(|v| v.as_str()).unwrap_or("");
            let status = item.get("status").and_then(|v| v.as_str()).unwrap_or("");
            let symbol = item.get("symbol").and_then(|v| v.as_str()).unwrap_or("");
            if status != "open" || code != "declaration_cadence_mismatch" {
                continue;
            }
            if !weekly.contains(&symbol) && !monthly.contains(&symbol) {
                continue;
            }
            let ticket_id = item.get("ticketId").and_then(|v| v.as_str()).unwrap_or("");
            if ticket_id.is_empty() {
                continue;
            }
            let freq = if weekly.contains(&symbol) {
                "Weekly"
            } else {
                "Monthly"
            };
            let filed = execute_command_on(
                &platform,
                &platform,
                cmd(
                    "WorkTicketFile",
                    serde_json::json!({
                        "ticketId": ticket_id,
                        "note": format!("Owner lock: {symbol} is {freq}. Infer must not replace lock.")
                    }),
                ),
            )
            .await;
            if !filed.ok {
                return Err(format!(
                    "{symbol} cadence ticket file failed: {}",
                    filed.error_code.unwrap_or_default()
                ));
            }
            lines.push(format!("{symbol} cadence ticket filed"));
        }
    }

    Ok(lines.join("\n"))
}
