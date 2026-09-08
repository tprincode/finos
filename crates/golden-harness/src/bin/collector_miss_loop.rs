//! Iterate live enabled collectors that last missed, post retrieve, print tickets.
//!
//! Usage:
//!   cargo run -p golden-harness --bin collector-miss-loop
//!   cargo run -p golden-harness --bin collector-miss-loop -- AMDW TRIN

use application_core::contracts::{CommandRequest, QueryRequest, FINANCE_CLIENT_CONTRACT_VERSION};
use application_core::queries::{execute_command_on, execute_query_on};
use golden_harness::profile_a_app_dir;
use import_engine::{collect_declarations_for, DeclarationTarget};
use serde_json::Value;
use storage_sqlite::LocalPlatform;
use uuid::Uuid;

fn cmd(name: &str, body: Value) -> CommandRequest {
    CommandRequest {
        contract_version: FINANCE_CLIENT_CONTRACT_VERSION.to_string(),
        command_name: name.to_string(),
        correlation_id: Uuid::new_v4(),
        body_json: Some(body.to_string()),
        expected_version: None,
    }
}

fn qry(name: &str, body: Value) -> QueryRequest {
    QueryRequest {
        contract_version: FINANCE_CLIENT_CONTRACT_VERSION.to_string(),
        query_name: name.to_string(),
        correlation_id: Uuid::new_v4(),
        body_json: Some(body.to_string()),
    }
}

async fn paid_declaration_context(
    platform: &LocalPlatform,
    security_id: &str,
) -> (u8, Vec<String>, Vec<(String, i64, u8)>) {
    let inv = execute_query_on(
        platform,
        platform,
        qry(
            "InvestmentGet",
            serde_json::json!({ "securityId": security_id }),
        ),
    )
    .await;
    if !inv.ok {
        return (0, Vec::new(), Vec::new());
    }
    let val: Value =
        serde_json::from_str(inv.body_json.as_deref().unwrap_or("{}")).unwrap_or_default();
    let mut paid = 0u8;
    let mut periods = Vec::new();
    let mut amounts = Vec::new();
    if let Some(arr) = val.get("declarations").and_then(|d| d.as_array()) {
        for row in arr {
            let amount = row
                .get("amountPerShareMinor")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            if amount <= 0 {
                continue;
            }
            paid = paid.saturating_add(1);
            if let Some(p) = row.get("paymentPeriod").and_then(|v| v.as_str()) {
                let p = p.trim();
                if !p.is_empty() {
                    periods.push(p.to_string());
                    let scale = row
                        .get("amountScale")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(2) as u8;
                    amounts.push((p.to_string(), amount, scale));
                }
            }
        }
    }
    (paid, periods, amounts)
}

#[tokio::main]
async fn main() {
    let filter: Vec<String> = std::env::args()
        .skip(1)
        .map(|s| s.to_ascii_uppercase())
        .collect();
    let app = profile_a_app_dir();
    eprintln!("Opening {}", app.display());
    let platform = LocalPlatform::open(&app).await.unwrap_or_else(|e| {
        eprintln!("open live sqlite failed: {e}");
        std::process::exit(1);
    });
    if filter
        .iter()
        .any(|f| f == "--ALIGN-FUTURE-PLAN" || f == "ALIGN-FUTURE-PLAN")
    {
        let align = execute_command_on(
            &platform,
            &platform,
            cmd(
                "CollectorAlignFutureToPlan",
                serde_json::json!({
                    "asOfDate": chrono::Local::now().format("%Y-%m-%d").to_string()
                }),
            ),
        )
        .await;
        eprintln!(
            "CollectorAlignFutureToPlan ok={} body={}",
            align.ok,
            align.body_json.as_deref().unwrap_or("")
        );
        return;
    }
    let set = execute_query_on(&platform, &platform, qry("CollectorSetGet", serde_json::json!({})))
        .await;
    if !set.ok {
        eprintln!("CollectorSetGet failed: {:?}", set.error_code);
        std::process::exit(1);
    }
    let body: Value = serde_json::from_str(set.body_json.as_deref().unwrap_or("{}")).unwrap();
    let mut items: Vec<Value> = body["items"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|r| r["collectorEnabled"].as_bool().unwrap_or(false))
        .filter(|r| !r["declarationSource"].as_str().unwrap_or("").trim().is_empty())
        .collect();
    if !filter.is_empty() {
        items.retain(|r| {
            filter
                .iter()
                .any(|f| r["symbol"].as_str().unwrap_or("").eq_ignore_ascii_case(f))
        });
    } else {
        items.retain(|r| r["lastRunOk"].as_bool() != Some(true));
    }
    items.sort_by(|a, b| {
        a["symbol"]
            .as_str()
            .unwrap_or("")
            .cmp(b["symbol"].as_str().unwrap_or(""))
    });
    eprintln!(
        "Miss/stale enabled collectors: {}  as_of={}",
        items.len(),
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
    );

    let mut ok_n = 0u32;
    let mut miss_n = 0u32;
    let mut ticket_fail = 0u32;
    for row in &items {
        let symbol = row["symbol"].as_str().unwrap_or("?").to_string();
        let security_id = row["securityId"].as_str().unwrap_or("").to_string();
        let source = row["declarationSource"].as_str().unwrap_or("").to_string();
        eprint!("{symbol} via {source}… ");
        let (paid_count, known_payment_periods, known_declaration_amounts) =
            paid_declaration_context(&platform, &security_id).await;
        let target = DeclarationTarget {
            security_id: security_id.clone(),
            symbol: symbol.clone(),
            declaration_source: source.clone(),
            source_symbol: row["sourceSymbol"]
                .as_str()
                .unwrap_or(&symbol)
                .to_string(),
            source_url: row["sourceUrl"].as_str().unwrap_or("").to_string(),
            force_refresh: true,
            last_run_ok: false,
            inception_on: row["inceptionOn"].as_str().unwrap_or("").to_string(),
            payment_frequency: row["paymentFrequency"].as_str().unwrap_or("").to_string(),
            div_type: row["divType"].as_str().unwrap_or("DIV-1").to_string(),
            paid_count,
            known_payment_periods,
            known_declaration_amounts,
            ..Default::default()
        };
        let outcome = collect_declarations_for(vec![target]);
        let retrieve = execute_command_on(
            &platform,
            &platform,
            cmd(
                "CollectorRetrieve",
                serde_json::json!({
                    "securityId": security_id,
                    "symbol": symbol,
                    "declarationSource": source,
                    "candidates": outcome.candidates,
                    "pagePaid": if outcome.page_paid.is_empty() {
                        outcome.candidates.clone()
                    } else {
                        outcome.page_paid
                    },
                    "upcomingPays": outcome.pay_dates,
                    "misses": outcome.misses,
                    "fetchedSourceUrl": outcome.fetched_source_url,
                    "forceRefresh": true
                }),
            ),
        )
        .await;
        if !retrieve.ok {
            miss_n += 1;
            eprintln!("command fail {:?}", retrieve.error_code);
            continue;
        }
        let retrieved: Value =
            serde_json::from_str(retrieve.body_json.as_deref().unwrap_or("{}")).unwrap();
        let run_ok = retrieved["ok"].as_bool().unwrap_or(false);
        let code = retrieved["code"].as_str().unwrap_or("");
        let message = retrieved["message"].as_str().unwrap_or("");
        if run_ok {
            ok_n += 1;
        } else {
            miss_n += 1;
        }

        let tickets = execute_query_on(
            &platform,
            &platform,
            qry(
                "WorkTicketList",
                serde_json::json!({ "securityId": security_id, "status": "open" }),
            ),
        )
        .await;
        let ticket_body: Value =
            serde_json::from_str(tickets.body_json.as_deref().unwrap_or("{}")).unwrap();
        let open: Vec<String> = ticket_body["items"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter(|t| t["status"] == "open" && t["symbol"] == symbol)
            .map(|t| t["code"].as_str().unwrap_or("?").to_string())
            .collect();
        let ticket_ok = if run_ok {
            !open.iter().any(|c| {
                matches!(
                    c.as_str(),
                    "declaration_retrieve_miss"
                        | "declaration_parse_unstable"
                        | "parse_miss"
                        | "declaration_stored_mismatch"
                )
            })
        } else {
            !open.is_empty()
        };
        if !ticket_ok {
            ticket_fail += 1;
        }
        eprintln!(
            "{} {} — {} tickets=[{}] ticket_process={}",
            if run_ok { "OK" } else { "MISS" },
            if code.is_empty() { "—" } else { code },
            message.chars().take(90).collect::<String>(),
            open.join(","),
            if ticket_ok { "ok" } else { "FAIL" }
        );
    }

    let all = execute_query_on(&platform, &platform, qry("CollectorSetGet", serde_json::json!({})))
        .await;
    let all_body: Value = serde_json::from_str(all.body_json.as_deref().unwrap_or("{}")).unwrap();
    let tickets = execute_query_on(&platform, &platform, qry("WorkTicketList", serde_json::json!({})))
        .await;
    let ticket_body: Value =
        serde_json::from_str(tickets.body_json.as_deref().unwrap_or("{}")).unwrap();
    let mut swept = 0u32;
    for t in ticket_body["items"].as_array().cloned().unwrap_or_default() {
        if t["status"] != "open" {
            continue;
        }
        let code = t["code"].as_str().unwrap_or("");
        if !matches!(
            code,
            "declaration_retrieve_miss"
                | "declaration_parse_unstable"
                | "parse_miss"
                | "declaration_stored_mismatch"
                | "declaration_history_dropped"
        ) {
            continue;
        }
        let sid = t["securityId"].as_str().unwrap_or("");
        let row = all_body["items"].as_array().and_then(|rows| {
            rows.iter().find(|r| r["securityId"].as_str() == Some(sid))
        });
        if row.and_then(|r| r["lastRunOk"].as_bool()) != Some(true) {
            continue;
        }
        let filed = execute_command_on(
            &platform,
            &platform,
            cmd(
                "WorkTicketFile",
                serde_json::json!({
                    "ticketId": t["ticketId"],
                    "note": "auto: latest declaration retrieve is ok"
                }),
            ),
        )
        .await;
        if filed.ok {
            swept += 1;
            eprintln!(
                "swept stale {} {} — latest retrieve ok",
                t["symbol"].as_str().unwrap_or("?"),
                code
            );
        }
    }

    println!(
        "\ncollector-miss-loop {}  ok={} miss={} ticket_process_fail={} stale_tickets_filed={} of {} misses",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
        ok_n,
        miss_n,
        ticket_fail,
        swept,
        items.len()
    );
    if miss_n > 0 || ticket_fail > 0 {
        std::process::exit(2);
    }
}
