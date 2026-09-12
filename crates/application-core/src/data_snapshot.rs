//! Household data snapshot: individual importable workbooks under raw-data/<date>/.
//! Sheets use the seed `Data` header row so `parse_production_templates` can reload them.
//! Only stored facts — no derived market-value or Plan rollups.

use std::path::{Path, PathBuf};

use rust_xlsxwriter::Workbook;

use crate::contracts::DataSnapshotExportBody;
use crate::ports::canonical::Canonical;
use crate::ports::platform::{Platform, PlatformError};

pub fn raw_data_dir(app_dir: &Path, as_of: &str) -> PathBuf {
    app_dir.join("raw-data").join(as_of)
}

pub async fn export_data_snapshot(
    platform: &dyn Platform,
    canonical: &dyn Canonical,
    as_of: &str,
) -> Result<DataSnapshotExportBody, PlatformError> {
    let folder = raw_data_dir(&platform.app_data_dir(), as_of);
    std::fs::create_dir_all(&folder).map_err(|e| {
        PlatformError::new("snapshot_write_failed", format!("create {}: {e}", folder.display()))
    })?;

    let accounts = canonical.account_list().await?;
    let securities = canonical.security_list().await?;
    let basis = canonical.basis_get().await?;
    let activities = canonical.activity_list().await?;
    let chars = canonical.position_characteristic_list().await?;
    let trends = canonical.trends_week_list().await?;
    let snapshots = canonical.account_balance_snapshot_list().await?;
    let plans = canonical.plan_history_list().await?;

    let acct_name = |id| {
        accounts
            .iter()
            .find(|a| a.account_id == id)
            .map(|a| a.name.clone())
            .unwrap_or_default()
    };
    let sym = |id| {
        securities
            .iter()
            .find(|s| s.security_id == id)
            .map(|s| s.symbol.clone())
            .unwrap_or_default()
    };

    let mut files = Vec::new();
    let mut counts: Vec<(String, u64)> = Vec::new();

    let account_rows: Vec<Vec<String>> = accounts
        .iter()
        .filter(|a| !a.name.eq_ignore_ascii_case("External"))
        .map(|a| {
            vec![
                a.name.clone(),
                account_type_label(&a.kind),
                financial_domain::account_value::account_custodian(&a.name).to_string(),
            ]
        })
        .collect();
    counts.push(("accounts".into(), account_rows.len() as u64));
    write_named(
        &folder,
        "Template_Accounts.xlsx",
        &["name", "account_type", "custodian"],
        &account_rows,
        &mut files,
    )?;

    let mut position_rows = Vec::new();
    for sec in &securities {
        let ch = chars.iter().find(|c| c.security_id == sec.security_id);
        let roc = ch.and_then(|c| c.roc_scale).unwrap_or(2);
        let plan = plans.iter().find(|p| {
            p.security_id == sec.security_id && p.effective_to.is_empty()
        });
        position_rows.push(vec![
            sec.symbol.clone(),
            ch.map(|c| c.underlying.clone()).unwrap_or_else(|| sec.name.clone()),
            ch.map(|c| c.payment_frequency.clone()).unwrap_or_default(),
            ch.map(|c| c.risk_tier.clone()).unwrap_or_default(),
            ch.map(|c| c.provider.clone()).unwrap_or_default(),
            ch.map(|c| money_opt(c.roc_pct_2025_actual_minor, roc))
                .unwrap_or_default(),
            ch.map(|c| money_opt(c.roc_pct_2026_estimate_minor, roc))
                .unwrap_or_default(),
            ch.map(|c| money_opt(c.roc_pct_2026_actual_minor, roc))
                .unwrap_or_default(),
            ch.map(|c| money_opt(c.roc_pct_2024_actual_minor, roc))
                .unwrap_or_default(),
            ch.map(|c| c.div_type.clone()).unwrap_or_default(),
            ch.map(|c| {
                if c.needs_roc_research {
                    "YES".into()
                } else {
                    "NO".into()
                }
            })
            .unwrap_or_else(|| "NO".into()),
            ch.map(|c| c.notes.clone()).unwrap_or_default(),
            ch.map(|c| if c.is_active { "YES".into() } else { "NO".into() })
                .unwrap_or_else(|| "YES".into()),
            plan.map(|p| money_str(p.amount_per_share_minor, p.amount_scale))
                .unwrap_or_default(),
        ]);
    }
    counts.push(("positions".into(), position_rows.len() as u64));
    write_named(
        &folder,
        "Template_Positions.xlsx",
        &[
            "symbol",
            "underlying",
            "payment_frequency",
            "risk_tier",
            "provider",
            "roc_pct_2025_actual",
            "roc_pct_2026_estimate",
            "roc_pct_2026_actual",
            "roc_pct_2024_actual",
            "div_type",
            "needs_roc_research",
            "notes",
            "is_active",
            "current_plan_per_share",
        ],
        &position_rows,
        &mut files,
    )?;

    let lot_rows: Vec<Vec<String>> = basis
        .lots
        .iter()
        .map(|lot| {
            let qty = money_str(lot.quantity_minor, lot.quantity_scale);
            let unit_orig = unit_cost(
                lot.performance_basis_minor,
                lot.quantity_minor,
                lot.quantity_scale,
                lot.scale,
            );
            let unit_tax = unit_cost(
                lot.tax_basis_minor,
                lot.quantity_minor,
                lot.quantity_scale,
                lot.scale,
            );
            vec![
                acct_name(lot.account_id),
                sym(lot.security_id),
                qty,
                unit_orig,
                unit_tax,
                lot.opened_on.clone(),
                if lot.origin.eq_ignore_ascii_case("drip") {
                    "drip".into()
                } else {
                    String::new()
                },
                if lot.remaining_quantity_minor > 0 {
                    "1".into()
                } else {
                    "0".into()
                },
            ]
        })
        .collect();
    counts.push(("lots".into(), lot_rows.len() as u64));
    write_named(
        &folder,
        "Template_Lots.xlsx",
        &[
            "account_name",
            "symbol",
            "quantity",
            "unit_cost_original",
            "unit_cost_tax",
            "purchase_date",
            "notes",
            "is_open",
        ],
        &lot_rows,
        &mut files,
    )?;

    let mut yield_rows = Vec::new();
    let mut disb_rows = Vec::new();
    for act in &activities {
        let account = acct_name(act.account_id);
        if account.eq_ignore_ascii_case("External") {
            continue;
        }
        let symbol = act.security_id.map(sym).unwrap_or_default();
        let amount = money_str(act.amount_minor, act.scale);
        if is_yield(&act.activity_type) {
            yield_rows.push(vec![
                account,
                symbol,
                amount,
                act.occurred_on.clone(),
            ]);
        } else {
            disb_rows.push(vec![
                account,
                amount,
                act.activity_type.clone(),
                act.occurred_on.clone(),
                act.occurred_on.chars().take(4).collect(),
                act.idempotency_key.clone(),
            ]);
        }
    }
    counts.push(("yields".into(), yield_rows.len() as u64));
    write_named(
        &folder,
        "Template_Transactions_Yield.xlsx",
        &["account_name", "symbol", "amount", "txn_date"],
        &yield_rows,
        &mut files,
    )?;
    counts.push(("disbursements".into(), disb_rows.len() as u64));
    write_named(
        &folder,
        "Template_Transactions_Disbursement.xlsx",
        &[
            "account_name",
            "amount_gross",
            "txn_type",
            "txn_date",
            "source_year",
            "source_ref",
        ],
        &disb_rows,
        &mut files,
    )?;

    let trend_rows: Vec<Vec<String>> = trends
        .iter()
        .map(|w| {
            let bal = |name: &str| {
                snapshots
                    .iter()
                    .find(|s| {
                        s.period_end == w.period_end
                            && accounts.iter().any(|a| {
                                a.account_id == s.account_id && a.name.eq_ignore_ascii_case(name)
                            })
                    })
                    .map(|s| money_str(s.balance_minor, s.scale))
                    .unwrap_or_default()
            };
            vec![
                w.period_end.clone(),
                money_str(w.profit_minor, w.scale),
                money_str(w.monthly_divs_minor, w.scale),
                money_str(w.fidelity_total_minor, w.scale),
                money_str(w.schwab_total_minor, w.scale),
                money_str(w.income_cash_minor, w.scale),
                money_str(w.acct9_cash_minor, w.scale),
                money_str(w.acct9_etf_value_minor, w.scale),
                bal("Car"),
                bal("Income"),
                bal("Health"),
                bal("FI Roth"),
                bal("Speculation"),
            ]
        })
        .collect();
    counts.push(("trends_weeks".into(), trend_rows.len() as u64));
    write_named(
        &folder,
        "Template_Trends_Weekly.xlsx",
        &[
            "week_end_friday",
            "profit",
            "monthly_divs",
            "fidelity_total",
            "schwab_total",
            "income_cash",
            "acct9_cash",
            "acct9_etf_value",
            "car_balance",
            "income_balance",
            "health_balance",
            "roth_balance",
            "speculation_balance",
        ],
        &trend_rows,
        &mut files,
    )?;

    let mut decl_rows = Vec::new();
    let mut pay_rows = Vec::new();
    let mut price_rows = Vec::new();
    let mut tmpl_rows = Vec::new();
    for sec in &securities {
        if let Ok(decls) = canonical.issuer_declaration_list(sec.security_id).await {
            for d in decls {
                decl_rows.push(vec![
                    sec.symbol.clone(),
                    money_opt(d.amount_per_share_minor, d.amount_scale),
                    d.payment_period.clone(),
                    d.source.clone(),
                    d.entered_at.clone(),
                ]);
            }
        }
        if let Ok(dates) = canonical.issuer_pay_date_list(sec.security_id).await {
            for p in dates {
                pay_rows.push(vec![
                    sec.symbol.clone(),
                    p.pay_on.clone(),
                    p.source.clone(),
                    p.recorded_at.clone(),
                ]);
            }
        }
        if let Ok(quotes) = canonical.price_quote_list(sec.security_id).await {
            for q in quotes {
                price_rows.push(vec![
                    sec.symbol.clone(),
                    money_str(q.price_minor, q.scale),
                    q.as_of_at.clone(),
                    q.source.clone(),
                    q.validation_status.clone(),
                ]);
            }
        }
        if let Ok(Some(t)) = canonical.retrieval_template_get(sec.security_id).await {
            tmpl_rows.push(vec![
                sec.symbol.clone(),
                t.declaration_source,
                t.source_url,
                t.roc_source_url,
                t.calendar_policy,
                t.inception_on,
                t.lookback_count.to_string(),
                if t.collector_enabled { "1".into() } else { "0".into() },
            ]);
        }
    }
    counts.push(("declarations".into(), decl_rows.len() as u64));
    write_named(
        &folder,
        "Template_Declarations.xlsx",
        &["symbol", "amount_per_share", "payment_period", "source", "entered_at"],
        &decl_rows,
        &mut files,
    )?;
    counts.push(("pay_dates".into(), pay_rows.len() as u64));
    write_named(
        &folder,
        "Template_PayDates.xlsx",
        &["symbol", "pay_on", "source", "recorded_at"],
        &pay_rows,
        &mut files,
    )?;
    counts.push(("last_prices".into(), price_rows.len() as u64));
    write_named(
        &folder,
        "Template_LastPrices.xlsx",
        &["symbol", "price", "as_of_at", "source", "validation_status"],
        &price_rows,
        &mut files,
    )?;
    counts.push(("retrieval_templates".into(), tmpl_rows.len() as u64));
    write_named(
        &folder,
        "Template_RetrievalTemplates.xlsx",
        &[
            "symbol",
            "declaration_source",
            "source_url",
            "roc_source_url",
            "calendar_policy",
            "inception_on",
            "lookback_count",
            "collector_enabled",
        ],
        &tmpl_rows,
        &mut files,
    )?;

    let yaml = write_plan_yaml(&securities, &plans);
    let yaml_name = "calculator-plan-seed.yaml";
    std::fs::write(folder.join(yaml_name), yaml).map_err(|e| {
        PlatformError::new("snapshot_write_failed", format!("{yaml_name}: {e}"))
    })?;
    files.push(yaml_name.into());

    let mut manifest = format!(
        "finos data snapshot\nas_of: {as_of}\nfolder: {}\n\nUncomputed source facts only. Reload via production seed parse of this folder.\n\n",
        folder.display()
    );
    for (name, n) in &counts {
        manifest.push_str(&format!("{name}: {n}\n"));
    }
    std::fs::write(folder.join("MANIFEST.md"), manifest).map_err(|e| {
        PlatformError::new("snapshot_write_failed", format!("MANIFEST.md: {e}"))
    })?;
    files.push("MANIFEST.md".into());

    Ok(DataSnapshotExportBody {
        as_of: as_of.to_string(),
        folder: folder.to_string_lossy().into_owned(),
        files,
        account_count: counts.iter().find(|(n, _)| n == "accounts").map(|(_, n)| *n).unwrap_or(0),
        lot_count: counts.iter().find(|(n, _)| n == "lots").map(|(_, n)| *n).unwrap_or(0),
        yield_count: counts.iter().find(|(n, _)| n == "yields").map(|(_, n)| *n).unwrap_or(0),
        note: "Individual workbooks under raw-data/<date>. Data sheets match seed import headers."
            .into(),
    })
}

fn write_named(
    folder: &Path,
    name: &str,
    headers: &[&str],
    rows: &[Vec<String>],
    files: &mut Vec<String>,
) -> Result<(), PlatformError> {
    let bytes = write_data_workbook(headers, rows)
        .map_err(|e| PlatformError::new("snapshot_write_failed", format!("{name}: {e}")))?;
    std::fs::write(folder.join(name), bytes)
        .map_err(|e| PlatformError::new("snapshot_write_failed", format!("{name}: {e}")))?;
    files.push(name.to_string());
    Ok(())
}

pub fn write_data_workbook(headers: &[&str], rows: &[Vec<String>]) -> Result<Vec<u8>, String> {
    let mut wb = Workbook::new();
    let sheet = wb.add_worksheet();
    sheet.set_name("Data").map_err(|e| e.to_string())?;
    for (c, h) in headers.iter().enumerate() {
        sheet
            .write_string(0, c as u16, *h)
            .map_err(|e| e.to_string())?;
    }
    for (r, row) in rows.iter().enumerate() {
        for (c, cell) in row.iter().enumerate() {
            sheet
                .write_string((r + 1) as u32, c as u16, cell)
                .map_err(|e| e.to_string())?;
        }
    }
    wb.save_to_buffer().map_err(|e| e.to_string())
}

fn money_str(minor: i64, scale: u8) -> String {
    let div = 10i64.pow(scale as u32);
    let neg = minor < 0;
    let n = minor.abs();
    let whole = n / div;
    let frac = n % div;
    let mut s = if scale == 0 {
        whole.to_string()
    } else {
        format!("{whole}.{frac:0width$}", width = scale as usize)
    };
    if neg {
        s.insert(0, '-');
    }
    s
}

fn money_opt(minor: Option<i64>, scale: u8) -> String {
    minor.map(|n| money_str(n, scale)).unwrap_or_default()
}

fn unit_cost(basis_minor: i64, qty_minor: i64, qty_scale: u8, money_scale: u8) -> String {
    if qty_minor == 0 {
        return String::new();
    }
    let unit = (basis_minor as i128) * 10i128.pow(qty_scale as u32) / (qty_minor as i128);
    money_str(unit as i64, money_scale)
}

fn write_plan_yaml(
    securities: &[crate::contracts::SecurityRecord],
    plans: &[crate::contracts::PlanHistoryRecord],
) -> String {
    let effective = plans
        .iter()
        .find(|p| !p.effective_from.is_empty())
        .map(|p| p.effective_from.as_str())
        .unwrap_or("2000-01-01");
    let reason = plans
        .iter()
        .find(|p| !p.decision_reason.is_empty())
        .map(|p| p.decision_reason.as_str())
        .unwrap_or("snapshot");
    let mut rows = String::new();
    for plan in plans {
        if !plan.effective_to.is_empty() {
            continue;
        }
        let Some(sec) = securities.iter().find(|s| s.security_id == plan.security_id) else {
            continue;
        };
        rows.push_str(&format!(
            "  - symbol: {}\n    plan: \"{}\"\n    periods: {}\n",
            sec.symbol,
            money_str(plan.amount_per_share_minor, plan.amount_scale),
            plan.planning_periods_per_year
        ));
    }
    if rows.is_empty() {
        format!("effective_from: \"{effective}\"\ndecision_reason: \"{reason}\"\nsymbols: []\n")
    } else {
        format!("effective_from: \"{effective}\"\ndecision_reason: \"{reason}\"\nsymbols:\n{rows}")
    }
}

fn is_yield(activity_type: &str) -> bool {
    let t = activity_type.to_ascii_lowercase();
    t.contains("dividend") || t == "yield"
}

fn account_type_label(kind: &str) -> String {
    match kind {
        "fi_roth" => "roth".into(),
        other => other.replace('_', " "),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn money_round_trips_cents() {
        assert_eq!(money_str(1250, 2), "12.50");
        assert_eq!(money_str(-40, 2), "-0.40");
        assert_eq!(money_str(100, 0), "100");
    }

    #[test]
    fn data_workbook_has_data_sheet_headers() {
        let bytes = write_data_workbook(&["name", "account_type"], &[vec!["Income".into(), "taxable".into()]])
            .expect("xlsx");
        assert!(bytes.len() > 32);
    }
}
