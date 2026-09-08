//! Fill current-year 19a-1 estimate on the Profile A SQLite file via RocResearchRetrieve.
//! Does not invent 0%. Does not write 2025 actual. Does not touch declaration last_run_ok.

use std::process::ExitCode;

use application_core::contracts::{
    CommandRequest, IssuerPayDateRecord, QueryRequest, FINANCE_CLIENT_CONTRACT_VERSION,
};
use application_core::ports::canonical::Canonical;
use application_core::queries::{execute_command_on, execute_query_on};
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
            eprintln!("roc-19a1-fill failed: {err}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<String, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let owner_zero = args.iter().any(|a| a == "--owner-zero");
    let confirm_ordinary = args.iter().any(|a| a == "--confirm-ordinary");
    let accept_lookback = args.iter().any(|a| a == "--accept-lookback");
    let except_variation = args.iter().any(|a| a == "--except-variation");
    let fleet_status = args.iter().any(|a| a == "--fleet-status");
    let set_row = args.iter().any(|a| a == "--set-row");
    let dump_set = args.iter().any(|a| a == "--dump-set");
    let close_complete = args.iter().any(|a| a == "--close-complete-tickets");
    let keep_stored_roc = args.iter().any(|a| a == "--keep-stored-roc");
    let frequency = args
        .iter()
        .position(|a| a == "--frequency")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let inception_on = args
        .iter()
        .position(|a| a == "--inception-on")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let set_source_url = args
        .iter()
        .position(|a| a == "--source-url")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let risk_tier = args
        .iter()
        .position(|a| a == "--risk")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let remaining_only = args
        .iter()
        .position(|a| a == "--remaining-only")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let roc_url = args
        .iter()
        .position(|a| a == "--roc-url")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let owner_pct = args
        .iter()
        .position(|a| a == "--owner-pct")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse::<i64>().ok());
    let roc_scale = args
        .iter()
        .position(|a| a == "--roc-scale")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse::<u8>().ok())
        .unwrap_or(1);
    let skip_vals = args
        .iter()
        .enumerate()
        .filter_map(|(i, a)| {
            if a == "--owner-pct"
                || a == "--roc-scale"
                || a == "--roc-url"
                || a == "--frequency"
                || a == "--inception-on"
                || a == "--source-url"
                || a == "--risk"
                || a == "--remaining-only"
            {
                Some(i + 1)
            } else {
                None
            }
        })
        .collect::<std::collections::HashSet<_>>();
    let symbol = args
        .iter()
        .enumerate()
        .find(|(i, a)| !a.starts_with('-') && !skip_vals.contains(i))
        .map(|(_, a)| a.clone())
        .unwrap_or_else(|| "HAKY".into())
        .to_ascii_uppercase();
    let app_dir = profile_a_app_dir();
    let platform = LocalPlatform::open(&app_dir)
        .await
        .map_err(|e| e.to_string())?;

    let set = execute_query_on(
        &platform,
        &platform,
        qry("CollectorSetGet", serde_json::json!({})),
    )
    .await;
    if !set.ok {
        return Err(set.error_code.unwrap_or_else(|| "query_failed".into()));
    }
    let set_val: serde_json::Value =
        serde_json::from_str(set.body_json.as_deref().unwrap_or("{}")).unwrap_or_default();
    if set_row {
        for row in set_val
            .get("items")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
        {
            if row.get("symbol").and_then(|s| s.as_str()) != Some(symbol.as_str()) {
                continue;
            }
            let complete = row.get("complete").and_then(|v| v.as_bool()).unwrap_or(false);
            let gaps = row
                .get("gaps")
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|g| g.as_str())
                        .collect::<Vec<_>>()
                        .join(",")
                })
                .unwrap_or_default();
            let last_ok = row.get("lastRunOk");
            let paid = row.get("paidDeclarationCount");
            let rem = row.get("remainingPlanned");
            let tix = row.get("openTicketCount");
            let roc = row.get("rocEstimateMinor");
            let url = row.get("sourceUrl").and_then(|s| s.as_str()).unwrap_or("");
            let roc_url = row.get("rocSourceUrl").and_then(|s| s.as_str()).unwrap_or("");
            return Ok(format!(
                "{symbol} complete={complete} gaps=[{gaps}] last_run_ok={last_ok:?} paid={paid:?} rem={rem:?} tickets={tix:?} roc={roc:?} url={url} roc_url={roc_url}"
            ));
        }
        return Err(format!("{symbol} not in CollectorSetGet"));
    }
    if dump_set {
        let tickets = execute_query_on(
            &platform,
            &platform,
            qry("WorkTicketList", serde_json::json!({ "status": "open" })),
        )
        .await;
        let tbody: serde_json::Value =
            serde_json::from_str(tickets.body_json.as_deref().unwrap_or("{}")).unwrap_or_default();
        let mut out = Vec::new();
        for row in set_val
            .get("items")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
        {
            if !row.get("collectorEnabled").and_then(|v| v.as_bool()).unwrap_or(false) {
                continue;
            }
            let src = row.get("declarationSource").and_then(|s| s.as_str()).unwrap_or("");
            if src.trim().is_empty() {
                continue;
            }
            let sid = row.get("securityId").and_then(|s| s.as_str()).unwrap_or("");
            let codes: Vec<String> = tbody
                .get("items")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter(|t| t.get("securityId").and_then(|v| v.as_str()) == Some(sid))
                .filter_map(|t| t.get("code").and_then(|c| c.as_str()).map(|s| s.to_string()))
                .collect();
            let mut obj = row;
            obj["openTicketCodes"] = serde_json::json!(codes);
            out.push(obj);
        }
        return Ok(serde_json::to_string_pretty(&out).unwrap_or_else(|_| "[]".into()));
    }
    if fleet_status {
        let tickets = execute_query_on(
            &platform,
            &platform,
            qry("WorkTicketList", serde_json::json!({ "status": "open" })),
        )
        .await;
        let tbody: serde_json::Value =
            serde_json::from_str(tickets.body_json.as_deref().unwrap_or("{}")).unwrap_or_default();
        let mut lines = Vec::new();
        for row in set_val
            .get("items")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
        {
            let complete = row.get("complete").and_then(|v| v.as_bool()).unwrap_or(false);
            if !complete {
                continue;
            }
            let sid = row.get("securityId").and_then(|v| v.as_str()).unwrap_or("");
            let sym = row.get("symbol").and_then(|v| v.as_str()).unwrap_or("?");
            let gaps = row
                .get("gaps")
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|g| g.as_str())
                        .collect::<Vec<_>>()
                        .join(",")
                })
                .unwrap_or_default();
            let open: Vec<String> = tbody
                .get("items")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter(|t| t.get("securityId").and_then(|v| v.as_str()) == Some(sid))
                .filter_map(|t| {
                    let code = t.get("code").and_then(|c| c.as_str())?;
                    let reason = t.get("reason").and_then(|r| r.as_str()).unwrap_or("");
                    Some(format!("{code}: {reason}"))
                })
                .collect();
            if open.is_empty() {
                continue;
            }
            lines.push(format!(
                "{sym} complete gaps=[{gaps}] tickets={}",
                open.join(" | ")
            ));
        }
        if lines.is_empty() {
            return Ok("No completed collectors have open tickets.".into());
        }
        return Ok(lines.join("\n"));
    }
    if close_complete {
        let tickets = execute_query_on(
            &platform,
            &platform,
            qry("WorkTicketList", serde_json::json!({ "status": "open" })),
        )
        .await;
        let tbody: serde_json::Value =
            serde_json::from_str(tickets.body_json.as_deref().unwrap_or("{}")).unwrap_or_default();
        let complete_ids: std::collections::HashSet<String> = set_val
            .get("items")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|row| row.get("complete").and_then(|v| v.as_bool()).unwrap_or(false))
            .filter_map(|row| {
                row.get("securityId")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
            .collect();
        let mut lines = Vec::new();
        for t in tbody
            .get("items")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
        {
            let sid = t.get("securityId").and_then(|v| v.as_str()).unwrap_or("");
            if !complete_ids.contains(sid) {
                continue;
            }
            let code = t.get("code").and_then(|c| c.as_str()).unwrap_or("");
            let sym = t.get("symbol").and_then(|s| s.as_str()).unwrap_or("?");
            let Some(ticket_id) = t.get("ticketId").and_then(|v| v.as_str()) else {
                continue;
            };
            if !sym.eq_ignore_ascii_case(&symbol)
                && args.iter().any(|a| {
                    !a.starts_with('-') && a.eq_ignore_ascii_case(&symbol)
                })
            {
                continue;
            }
            if financial_domain::work_ticket::is_retrieve_failure_code(code) {
                lines.push(format!("{sym} {code} left open (retrieve still missed)"));
                continue;
            }
            let (cmd_name, body) = if code == "declaration_amount_variation" {
                (
                    "WorkTicketResolve",
                    serde_json::json!({
                        "ticketId": ticket_id,
                        "tool": "amount_confirm",
                        "action": "except",
                        "note": "excepted: issuer download amounts kept"
                    }),
                )
            } else {
                let note = match code {
                    "declaration_lookback_short" => {
                        "filed: owner accepted issuer calendar as the full paid series"
                    }
                    "paid_payable_supersede" => {
                        "filed: occurred/mid-month date kept; vendor month-end added as second pay"
                    }
                    _ => "filed: verified complete",
                };
                (
                    "WorkTicketFile",
                    serde_json::json!({ "ticketId": ticket_id, "note": note }),
                )
            };
            let result = execute_command_on(&platform, &platform, cmd(cmd_name, body)).await;
            if result.ok {
                lines.push(format!("{sym} {code} closed"));
            } else {
                lines.push(format!(
                    "{sym} {code} FAIL {}",
                    result.error_code.unwrap_or_else(|| "error".into())
                ));
            }
        }
        if lines.is_empty() {
            return Ok("No completed-collector tickets to close.".into());
        }
        return Ok(lines.join("\n"));
    }
    let item = set_val
        .get("items")
        .and_then(|v| v.as_array())
        .and_then(|items| {
            items.iter().find(|row| {
                row.get("symbol")
                    .and_then(|s| s.as_str())
                    .unwrap_or("")
                    .eq_ignore_ascii_case(&symbol)
            })
        })
        .cloned()
        .ok_or_else(|| format!("{symbol} not in collector set"))?;
    let security_id = item
        .get("securityId")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let declaration_source = item
        .get("declarationSource")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let source_url = item
        .get("sourceUrl")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let last_run_ok_before = item.get("lastRunOk").cloned();
    if let Some(pay_on) = remaining_only {
        let sid = Uuid::parse_str(&security_id).map_err(|e| e.to_string())?;
        let as_of = chrono::Utc::now().date_naive().to_string();
        let decls = platform
            .issuer_declaration_list(sid)
            .await
            .map_err(|e| e.code.clone())?;
        let mut dropped = 0u32;
        for d in decls {
            if d.amount_per_share_minor.unwrap_or(0) <= 0 {
                continue;
            }
            if financial_domain::schedule::period_has_occurred(&d.payment_period, &as_of) {
                continue;
            }
            dropped += platform
                .issuer_declaration_supersede_period(sid, d.payment_period)
                .await
                .map_err(|e| e.code.clone())? as u32;
        }
        platform
            .issuer_pay_date_replace(
                sid,
                as_of.clone(),
                vec![IssuerPayDateRecord {
                    pay_date_id: Uuid::new_v4(),
                    security_id: sid,
                    pay_on: pay_on.clone(),
                    source: "derived_walk".into(),
                    recorded_at: as_of,
                }],
            )
            .await
            .map_err(|e| e.code.clone())?;
        return Ok(format!(
            "{symbol} remaining {pay_on} (derive once); dropped {dropped} unoccurred paid row(s). last_run_ok unchanged."
        ));
    }
    if security_id.is_empty() {
        return Err(format!("{symbol} missing securityId"));
    }

    if frequency.is_some() || inception_on.is_some() || set_source_url.is_some() || risk_tier.is_some() {
        let sid = Uuid::parse_str(&security_id).map_err(|e| e.to_string())?;
        let mut notes = Vec::new();
        if frequency.is_some() || risk_tier.is_some() {
            let list = platform
                .position_characteristic_list()
                .await
                .map_err(|e| e.code.clone())?;
            let Some(mut rec) = list.into_iter().find(|c| c.security_id == sid) else {
                return Err(format!("{symbol} has no position characteristic"));
            };
            if let Some(freq) = frequency.as_ref() {
                rec.payment_frequency = freq.clone();
                notes.push(format!("frequency={freq}"));
                if financial_domain::calculator::is_non_paying(freq) {
                    rec.div_type.clear();
                    notes.push("div_type= (not a payer)".into());
                } else {
                    rec.div_type = "DIV-1".into();
                }
            } else if risk_tier.is_some() {
                rec.div_type = "DIV-1".into();
            }
            if let Some(risk) = risk_tier {
                rec.risk_tier = financial_domain::plan_review::normalize_risk_tier(&risk);
                notes.push(format!("risk_tier={}", rec.risk_tier));
            }
            platform
                .position_characteristic_upsert(rec)
                .await
                .map_err(|e| e.code.clone())?;
        }
        let park_non_payer = frequency
            .as_deref()
            .is_some_and(financial_domain::calculator::is_non_paying);
        if inception_on.is_some() || set_source_url.is_some() || park_non_payer {
            let mut rec = platform
                .retrieval_template_get(sid)
                .await
                .map_err(|e| e.code.clone())?
                .ok_or_else(|| format!("{symbol} has no retrieval template"))?;
            if let Some(on) = inception_on {
                rec.inception_on = on.clone();
                notes.push(format!("inception_on={on}"));
            }
            if let Some(url) = set_source_url {
                rec.source_url = url.clone();
                if !financial_domain::current_price::is_cash_par_symbol(&symbol)
                    && (rec.roc_source_url.trim().is_empty()
                        || rec.roc_source_url.contains("mstr-etfs")
                        || rec.roc_source_url.contains("ellingtonfinancial"))
                {
                    rec.roc_source_url = url.clone();
                }
                notes.push(format!("source_url={url}"));
            }
            if park_non_payer {
                rec.collector_enabled = false;
                rec.declaration_source.clear();
                rec.calendar_policy = "none".into();
                notes.push("collector_enabled=false".into());
                notes.push("declaration_source cleared".into());
            }
            platform
                .retrieval_template_set(rec)
                .await
                .map_err(|e| e.code.clone())?;
        }
        if park_non_payer {
            let tickets = execute_query_on(
                &platform,
                &platform,
                qry(
                    "WorkTicketList",
                    serde_json::json!({ "securityId": security_id, "status": "open" }),
                ),
            )
            .await;
            if tickets.ok {
                let body: serde_json::Value = serde_json::from_str(
                    tickets.body_json.as_deref().unwrap_or("{}"),
                )
                .unwrap_or_default();
                for t in body
                    .get("items")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default()
                {
                    if t.get("code").and_then(|c| c.as_str())
                        != Some("declaration_retrieve_miss")
                    {
                        continue;
                    }
                    let Some(ticket_id) = t.get("ticketId").and_then(|v| v.as_str()) else {
                        continue;
                    };
                    let result = execute_command_on(
                        &platform,
                        &platform,
                        cmd(
                            "WorkTicketFile",
                            serde_json::json!({
                                "ticketId": ticket_id,
                                "note": "filed: long-hold, not a dividend payer"
                            }),
                        ),
                    )
                    .await;
                    notes.push(if result.ok {
                        "declaration_retrieve_miss closed".into()
                    } else {
                        format!(
                            "declaration_retrieve_miss FAIL {}",
                            result.error_code.unwrap_or_else(|| "error".into())
                        )
                    });
                }
            }
        }
        return Ok(format!("{symbol} owner facts: {}", notes.join("; ")));
    }

    if let Some(url) = roc_url {
        let sid = Uuid::parse_str(&security_id).map_err(|e| e.to_string())?;
        let mut rec = platform
            .retrieval_template_get(sid)
            .await
            .map_err(|e| e.code.clone())?
            .ok_or_else(|| format!("{symbol} has no retrieval template"))?;
        rec.roc_source_url = url.clone();
        platform
            .retrieval_template_set(rec)
            .await
            .map_err(|e| e.code.clone())?;
        return Ok(format!(
            "{symbol} roc_source_url set; last_run_ok unchanged."
        ));
    }

    if keep_stored_roc {
        let sid = Uuid::parse_str(&security_id).map_err(|e| e.to_string())?;
        let list = platform
            .position_characteristic_list()
            .await
            .map_err(|e| e.code.clone())?;
        let Some(mut rec) = list.into_iter().find(|c| c.security_id == sid) else {
            return Err(format!("{symbol} has no position characteristic"));
        };
        rec.needs_roc_research = false;
        platform
            .position_characteristic_upsert(rec)
            .await
            .map_err(|e| e.code.clone())?;
        return Ok(format!(
            "{symbol} kept stored ROC estimate; needs_roc_research=false. last_run_ok unchanged."
        ));
    }

    if except_variation {
        let tickets = execute_query_on(
            &platform,
            &platform,
            qry(
                "WorkTicketList",
                serde_json::json!({ "securityId": security_id, "status": "open" }),
            ),
        )
        .await;
        if !tickets.ok {
            return Err(tickets
                .error_code
                .unwrap_or_else(|| "ticket_list_failed".into()));
        }
        let body: serde_json::Value =
            serde_json::from_str(tickets.body_json.as_deref().unwrap_or("{}")).unwrap_or_default();
        let mut n = 0u32;
        for t in body
            .get("items")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
        {
            if t.get("code").and_then(|c| c.as_str()) != Some("declaration_amount_variation") {
                continue;
            }
            let Some(ticket_id) = t.get("ticketId").and_then(|v| v.as_str()) else {
                continue;
            };
            let result = execute_command_on(
                &platform,
                &platform,
                cmd(
                    "WorkTicketResolve",
                    serde_json::json!({
                        "ticketId": ticket_id,
                        "tool": "amount_confirm",
                        "action": "except",
                        "note": "excepted: vendor download amounts kept"
                    }),
                ),
            )
            .await;
            if !result.ok {
                return Err(result
                    .error_code
                    .unwrap_or_else(|| "except_failed".into()));
            }
            n += 1;
        }
        return Ok(format!("{symbol} excepted {n} amount-variation ticket(s)."));
    }

    if let Some(pct) = owner_pct {
        let today = chrono::Utc::now().date_naive().to_string();
        let result = execute_command_on(
            &platform,
            &platform,
            cmd(
                "RocPlanConfirm",
                serde_json::json!({
                    "securityId": security_id,
                    "symbol": symbol,
                    "rocPctMinor": pct,
                    "rocScale": roc_scale,
                    "ownerOverride": true,
                    "source": "19a-1",
                    "sourceUrl": source_url,
                    "method": "19a-1-current-year",
                    "establishedHow": "owner accepted ROC percent from 19a-1 notices",
                    "asOfDate": today,
                }),
            ),
        )
        .await;
        if !result.ok {
            return Err(result
                .error_code
                .unwrap_or_else(|| "roc_plan_confirm_failed".into()));
        }
        return Ok(format!(
            "{symbol} owner ROC {pct} scale {roc_scale}. last_run_ok unchanged."
        ));
    }

    if accept_lookback {
        let result = execute_command_on(
            &platform,
            &platform,
            cmd(
                "CollectorFieldDecisionSet",
                serde_json::json!({
                    "securityId": security_id,
                    "field": "paid_history",
                    "decision": "accept"
                }),
            ),
        )
        .await;
        if !result.ok {
            return Err(result
                .error_code
                .unwrap_or_else(|| "lookback_accept_failed".into()));
        }
        return Ok(format!(
            "{symbol} owner Accept: issuer calendar is the full paid series. last_run_ok unchanged."
        ));
    }

    if confirm_ordinary {
        let today = chrono::Utc::now().date_naive().to_string();
        let result = execute_command_on(
            &platform,
            &platform,
            cmd(
                "RocPlanConfirm",
                serde_json::json!({
                    "securityId": security_id,
                    "symbol": symbol,
                    "rocPctMinor": 0,
                    "rocScale": 2,
                    "ownerOverride": true,
                    "source": "payment-type",
                    "sourceUrl": source_url,
                    "method": "payment-type",
                    "establishedHow": "ordinary/qualified dividend — not ROC",
                    "asOfDate": today,
                }),
            ),
        )
        .await;
        if !result.ok {
            return Err(result
                .error_code
                .unwrap_or_else(|| "roc_plan_confirm_failed".into()));
        }
        return Ok(format!(
            "{symbol} 0% ROC — ordinary dividend is not ROC. last_run_ok unchanged."
        ));
    }

    if owner_zero {
        if !matches!(symbol.as_str(), "TSLL" | "MPLX" | "MSTU" | "SOXL") {
            return Err(format!(
                "{symbol} is not on the owner lock-zero list (TSLL, MPLX, MSTU, SOXL)"
            ));
        }
        let today = chrono::Utc::now().date_naive().to_string();
        let result = execute_command_on(
            &platform,
            &platform,
            cmd(
                "RocPlanConfirm",
                serde_json::json!({
                    "securityId": security_id,
                    "symbol": symbol,
                    "rocPctMinor": 0,
                    "rocScale": 2,
                    "ownerOverride": true,
                    "source": "owner-override",
                    "sourceUrl": source_url,
                    "method": "owner-override",
                    "establishedHow": "owner lock: not ROC — 0%",
                    "asOfDate": today,
                }),
            ),
        )
        .await;
        if !result.ok {
            return Err(result
                .error_code
                .unwrap_or_else(|| "roc_plan_confirm_failed".into()));
        }
        return Ok(format!(
            "{symbol} owner lock 0% ROC (not a 19a-1). last_run_ok unchanged."
        ));
    }

    let today = chrono::Utc::now().date_naive().to_string();
    let mut body = serde_json::json!({
        "securityId": security_id,
        "symbol": symbol,
        "declarationSource": declaration_source,
        "sourceUrl": source_url,
        "asOfDate": today,
    });
    import_engine::enrich_retrieve_body("RocResearchRetrieve", &mut body);
    let cands = body
        .get("candidates")
        .and_then(|c| c.as_array())
        .cloned()
        .unwrap_or_default();
    if cands.is_empty() {
        return Err(format!(
            "{symbol} 19a-1 parse miss — unknown, not 0%. probes={}",
            body.get("rocProbes").map(|p| p.to_string()).unwrap_or_default()
        ));
    }

    let result = execute_command_on(
        &platform,
        &platform,
        cmd("RocResearchRetrieve", body.clone()),
    )
    .await;
    if !result.ok {
        return Err(result.error_code.unwrap_or_else(|| "retrieve_failed".into()));
    }

    let inv = execute_query_on(
        &platform,
        &platform,
        qry(
            "InvestmentGet",
            serde_json::json!({ "securityId": security_id, "asOfDate": today }),
        ),
    )
    .await;
    if !inv.ok {
        return Err(inv.error_code.unwrap_or_else(|| "investment_get_failed".into()));
    }
    let inv_val: serde_json::Value =
        serde_json::from_str(inv.body_json.as_deref().unwrap_or("{}")).unwrap_or_default();
    let pct = inv_val.get("rocPct2026EstimateMinor");
    let y2025 = inv_val.get("rocPct2025ActualMinor");
    let completed = inv_val
        .get("rocResearchCompletedAt")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let url = inv_val
        .get("rocEstimateSourceUrl")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let after_set = execute_query_on(
        &platform,
        &platform,
        qry("CollectorSetGet", serde_json::json!({})),
    )
    .await;
    let after_val: serde_json::Value =
        serde_json::from_str(after_set.body_json.as_deref().unwrap_or("{}")).unwrap_or_default();
    let last_run_ok_after = after_val
        .get("items")
        .and_then(|v| v.as_array())
        .and_then(|items| {
            items.iter().find(|row| {
                row.get("securityId")
                    .and_then(|s| s.as_str())
                    == Some(security_id.as_str())
            })
        })
        .and_then(|row| row.get("lastRunOk"))
        .cloned();
    if last_run_ok_after != last_run_ok_before {
        return Err(format!(
            "declaration last_run_ok changed: {last_run_ok_before:?} -> {last_run_ok_after:?}"
        ));
    }
    if y2025.map(|v| !v.is_null()).unwrap_or(false) {
        return Err("2025 actual must stay N/A".into());
    }
    Ok(format!(
        "{symbol} 2026 estimate {} sourceUrl={url} lastUpdate={completed} last_run_ok={last_run_ok_after:?}",
        pct.map(|v| v.to_string()).unwrap_or_else(|| "null".into())
    ))
}
