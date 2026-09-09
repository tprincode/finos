//! Parse FMS production xlsx templates into a FinanceClient seed document (no SQL).

use std::collections::{HashMap, HashSet};
use std::path::Path;

use application_core::contracts::{
    ImportCandidate, ProductionSeedAccount, ProductionSeedCharacteristic,
    ProductionSeedDisbursement, ProductionSeedDocument, ProductionSeedLot, ProductionSeedPlan,
    ProductionSeedSecurity, ProductionSeedTrendsWeek, ProductionSeedYieldBatch,
};
use calamine::{open_workbook, Data, Reader, Xlsx};
use chrono::{Duration, NaiveDate};

const MAX_SCALE: u8 = 8;
const YIELD_BATCH: usize = 250;

fn cell_string(v: &Data) -> String {
    match v {
        Data::Empty | Data::Error(_) => String::new(),
        Data::String(s) => s.trim().to_string(),
        Data::Float(n) => {
            if n.fract().abs() < 1e-9 {
                format!("{}", *n as i64)
            } else {
                format!("{n}")
            }
        }
        Data::Int(n) => n.to_string(),
        Data::Bool(b) => {
            if *b {
                "1".into()
            } else {
                "0".into()
            }
        }
        other => other.to_string().trim().to_string(),
    }
}

fn excel_serial_to_iso(serial: f64) -> String {
    let days = serial.trunc() as i64;
    let epoch = NaiveDate::from_ymd_opt(1899, 12, 30).expect("epoch");
    (epoch + Duration::days(days)).format("%Y-%m-%d").to_string()
}

fn looks_like_excel_date(s: &str) -> bool {
    s.parse::<f64>()
        .ok()
        .map(|n| (20_000.0..=60_000.0).contains(&n) && n.fract().abs() < 1e-9)
        .unwrap_or(false)
}

fn as_iso_date(raw: &str) -> String {
    if raw.contains('-') && raw.len() >= 10 {
        return raw.chars().take(10).collect();
    }
    if looks_like_excel_date(raw) {
        if let Ok(n) = raw.parse::<f64>() {
            return excel_serial_to_iso(n);
        }
    }
    raw.to_string()
}

fn read_data_rows(path: &Path) -> Result<Vec<HashMap<String, String>>, String> {
    let mut workbook: Xlsx<_> =
        open_workbook(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let range = workbook
        .worksheet_range("Data")
        .map_err(|e| format!("{} Data sheet: {e}", path.display()))?;
    let mut rows = range.rows();
    let header = rows
        .next()
        .ok_or_else(|| format!("{} empty", path.display()))?;
    let keys: Vec<String> = header.iter().map(cell_string).collect();
    let mut out = Vec::new();
    for row in rows {
        let mut map = HashMap::new();
        let mut any = false;
        for (i, key) in keys.iter().enumerate() {
            if key.is_empty() {
                continue;
            }
            let val = row.get(i).map(cell_string).unwrap_or_default();
            if !val.is_empty() {
                any = true;
            }
            map.insert(key.clone(), val);
        }
        if any {
            out.push(map);
        }
    }
    Ok(out)
}

fn map_account_kind(account_type: &str, name: &str) -> String {
    match account_type.trim().to_ascii_lowercase().as_str() {
        "taxable" => "taxable".into(),
        "ira" => "ira".into(),
        "hsa" => "hsa".into(),
        "roth" if name.eq_ignore_ascii_case("FI Roth") => "fi_roth".into(),
        "roth" => "fi_roth".into(),
        other => other.replace(' ', "_"),
    }
}

fn decimal_scale(raw: &str) -> u8 {
    let t = raw.trim().trim_start_matches('+').trim_start_matches('-');
    match t.split_once('.') {
        Some((_, frac)) => {
            let cleaned = frac.trim_end_matches('0');
            if cleaned.len() > MAX_SCALE as usize {
                MAX_SCALE
            } else {
                cleaned.len() as u8
            }
        }
        None => 0,
    }
}

fn to_minor(raw: &str, scale: u8) -> Result<i64, String> {
    let mut t = raw.trim().replace(',', "");
    // Tolerate spreadsheet typos like 12002..70
    while t.contains("..") {
        t = t.replace("..", ".");
    }
    if t.is_empty() {
        return Err("blank amount".into());
    }
    let neg = t.starts_with('-');
    let t = t.trim_start_matches('+').trim_start_matches('-');
    let (whole, frac) = match t.split_once('.') {
        Some((w, f)) => (w, f),
        None => (t, ""),
    };
    let frac: String = frac.chars().filter(|c| c.is_ascii_digit()).collect();
    let mut kept = frac.clone();
    let round_up = if kept.len() > scale as usize {
        let extra = kept.as_bytes()[scale as usize];
        kept.truncate(scale as usize);
        extra >= b'5'
    } else {
        while kept.len() < scale as usize {
            kept.push('0');
        }
        false
    };
    let combined = format!("{whole}{kept}");
    let n: i64 = combined
        .parse()
        .map_err(|_| format!("bad number {raw}"))?;
    let mut n = if neg { -n } else { n };
    if round_up {
        n += if neg { -1 } else { 1 };
    }
    Ok(n)
}

fn get<'a>(row: &'a HashMap<String, String>, key: &str) -> &'a str {
    row.get(key).map(|s| s.as_str()).unwrap_or("")
}

fn security_is_crf(symbol: &str) -> bool {
    symbol.eq_ignore_ascii_case("CRF")
}

/// Robinhood BTC lots are crypto (Yahoo BTC-USD). Other BTC lots are Grayscale (Yahoo BTC).
fn data_symbol(account_name: &str, symbol: &str) -> String {
    if symbol.eq_ignore_ascii_case("BTC") && account_name.eq_ignore_ascii_case("Robinhood") {
        "BTC-USD".into()
    } else {
        symbol.to_string()
    }
}

/// Parse production xlsx templates into a document that `ProductionSeedLoad` can apply.
pub fn parse_production_templates(production_dir: &Path) -> Result<ProductionSeedDocument, String> {
    let accounts = read_data_rows(&production_dir.join("Template_Accounts.xlsx"))?;
    let positions = read_data_rows(&production_dir.join("Template_Positions.xlsx"))?;
    let lots = read_data_rows(&production_dir.join("Template_Lots.xlsx"))?;
    let yield_rows = read_data_rows(&production_dir.join("Template_Transactions_Yield.xlsx"))?;
    let disbursements =
        read_data_rows(&production_dir.join("Template_Transactions_Disbursement.xlsx"))?;
    let trends_weeks = parse_trends_weeks(production_dir)?;

    let mut account_rows = Vec::new();
    for row in &accounts {
        let name = get(row, "name").to_string();
        if name.is_empty() {
            continue;
        }
        account_rows.push(ProductionSeedAccount {
            name: name.clone(),
            kind: map_account_kind(get(row, "account_type"), &name),
        });
    }

    let mut seen_symbols: HashSet<String> = HashSet::new();
    let mut securities = Vec::new();
    let mut push_security = |symbol: &str, name: &str| {
        let key = symbol.to_ascii_uppercase();
        if key.is_empty() || !seen_symbols.insert(key) {
            return;
        }
        securities.push(ProductionSeedSecurity {
            symbol: symbol.to_string(),
            name: name.to_string(),
            crf: security_is_crf(symbol),
        });
    };
    for row in &positions {
        let symbol = get(row, "symbol");
        let name = if get(row, "underlying").is_empty() {
            symbol
        } else {
            get(row, "underlying")
        };
        push_security(symbol, name);
    }
    for row in lots.iter().chain(yield_rows.iter()) {
        let symbol = data_symbol(get(row, "account_name"), get(row, "symbol"));
        let name = if symbol.eq_ignore_ascii_case("BTC-USD") {
            "Bitcoin"
        } else {
            symbol.as_str()
        };
        push_security(&symbol, name);
    }

    let mut lot_rows = Vec::new();
    for (i, row) in lots.iter().enumerate() {
        let qty_raw = get(row, "quantity");
        let orig = get(row, "unit_cost_original");
        let tax = get(row, "unit_cost_tax");
        let qty_scale = decimal_scale(qty_raw);
        let money_scale = decimal_scale(orig).max(decimal_scale(tax)).max(2);
        let quantity_minor = to_minor(qty_raw, qty_scale)?;
        let unit_orig = to_minor(orig, money_scale)?;
        let unit_tax = to_minor(tax, money_scale)?;
        let div = 10i128.pow(qty_scale as u32);
        let perf = (quantity_minor as i128) * (unit_orig as i128) / div;
        let tax_basis = (quantity_minor as i128) * (unit_tax as i128) / div;
        lot_rows.push(ProductionSeedLot {
            account_name: get(row, "account_name").to_string(),
            symbol: data_symbol(get(row, "account_name"), get(row, "symbol")),
            opened_on: as_iso_date(get(row, "purchase_date")),
            origin: if get(row, "notes").to_ascii_lowercase().contains("drip") {
                "drip".into()
            } else {
                "purchase".into()
            },
            quantity_minor,
            quantity_scale: qty_scale,
            performance_basis_minor: perf as i64,
            tax_basis_minor: tax_basis as i64,
            scale: money_scale,
            is_open: matches!(get(row, "is_open").trim(), "1" | "true" | "TRUE" | "yes"),
            row_index: i as u64,
        });
    }

    let mut yield_batches = Vec::new();
    for (batch_idx, chunk) in yield_rows.chunks(YIELD_BATCH).enumerate() {
        let mut candidates = Vec::new();
        for row in chunk {
            let amount = get(row, "amount");
            if amount.is_empty() {
                return Err("yield blank amount (must stay unknown, not coerced)".into());
            }
            candidates.push(ImportCandidate {
                account_name: get(row, "account_name").to_string(),
                symbol: Some(data_symbol(get(row, "account_name"), get(row, "symbol")))
                    .filter(|s| !s.is_empty()),
                activity_type: "dividend".into(),
                amount_minor: Some(to_minor(amount, 2)?),
                scale: 2,
                occurred_on: as_iso_date(get(row, "txn_date")),
                candidate_id: None,
                validation: String::new(),
                issue: String::new(),
            });
        }
        yield_batches.push(ProductionSeedYieldBatch {
            source_id: format!("production-yield-{batch_idx}"),
            filename: format!("yield-{batch_idx}.xlsx"),
            content: format!("production-yield-{batch_idx}-{}-rows", chunk.len()),
            candidates,
        });
    }

    let mut disbursement_rows = Vec::new();
    for (i, row) in disbursements.iter().enumerate() {
        let gross = get(row, "amount_gross");
        if gross.is_empty() {
            return Err(format!(
                "disbursement {i} blank amount_gross (must stay unknown)"
            ));
        }
        let txn_type = get(row, "txn_type");
        disbursement_rows.push(ProductionSeedDisbursement {
            account_name: get(row, "account_name").to_string(),
            activity_type: if txn_type.is_empty() {
                "Withdrawal".into()
            } else {
                txn_type.to_string()
            },
            amount_minor: to_minor(gross, 2)?,
            scale: 2,
            occurred_on: as_iso_date(get(row, "txn_date")),
            idempotency_key: format!(
                "production-disb-{i}-{}-{}",
                get(row, "source_year"),
                get(row, "source_ref")
            ),
        });
    }

    Ok(ProductionSeedDocument {
        accounts: account_rows,
        securities,
        lots: lot_rows,
        yield_batches,
        disbursements: disbursement_rows,
        plans: merge_plans(production_dir, &positions)?,
        characteristics: parse_characteristics(&positions)?,
        trends_weeks,
    })
}

fn optional_money_minor(raw: &str) -> Result<Option<i64>, String> {
    if raw.trim().is_empty() {
        return Ok(None);
    }
    Ok(Some(to_minor(raw, 2)?))
}

fn parse_trends_weeks(production_dir: &Path) -> Result<Vec<ProductionSeedTrendsWeek>, String> {
    let path = production_dir.join("Template_Trends_Weekly.xlsx");
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let rows = read_data_rows(&path)?;
    let mut out = Vec::with_capacity(rows.len());
    for (i, row) in rows.iter().enumerate() {
        let period_end = as_iso_date(get(row, "week_end_friday"));
        if period_end.is_empty() {
            continue;
        }
        let profit = get(row, "profit");
        if profit.is_empty() {
            return Err(format!(
                "trends week {i} blank profit (must stay unknown, not coerced)"
            ));
        }
        out.push(ProductionSeedTrendsWeek {
            period_end,
            profit_minor: to_minor(profit, 2)?,
            monthly_divs_minor: to_minor(get(row, "monthly_divs"), 2)?,
            fidelity_total_minor: to_minor(get(row, "fidelity_total"), 2)?,
            schwab_total_minor: to_minor(get(row, "schwab_total"), 2)?,
            income_cash_minor: to_minor(get(row, "income_cash"), 2)?,
            acct9_cash_minor: to_minor(get(row, "acct9_cash"), 2)?,
            acct9_etf_value_minor: to_minor(get(row, "acct9_etf_value"), 2)?,
            car_balance_minor: optional_money_minor(get(row, "car_balance"))?,
            income_balance_minor: optional_money_minor(get(row, "income_balance"))?,
            health_balance_minor: optional_money_minor(get(row, "health_balance"))?,
            roth_balance_minor: optional_money_minor(get(row, "roth_balance"))?,
            speculation_balance_minor: optional_money_minor(get(row, "speculation_balance"))?,
            scale: 2,
        });
    }
    Ok(out)
}

#[derive(Debug, serde::Deserialize)]
struct CalculatorPlanSeedFile {
    effective_from: String,
    decision_reason: String,
    symbols: Vec<CalculatorPlanSeedRow>,
}

#[derive(Debug, serde::Deserialize)]
struct CalculatorPlanSeedRow {
    symbol: String,
    plan: String,
    periods: u8,
}

fn optional_minor(raw: &str) -> Result<Option<(i64, u8)>, String> {
    if raw.trim().is_empty() {
        return Ok(None);
    }
    let scale = decimal_scale(raw).max(1);
    Ok(Some((to_minor(raw, scale)?, scale)))
}

fn parse_characteristics(
    positions: &[HashMap<String, String>],
) -> Result<Vec<ProductionSeedCharacteristic>, String> {
    let mut rows = Vec::new();
    for row in positions {
        let symbol = get(row, "symbol").to_string();
        if symbol.is_empty() {
            continue;
        }
        let p2025 = optional_minor(get(row, "roc_pct_2025_actual"))?;
        let p2026e = optional_minor(get(row, "roc_pct_2026_estimate"))?;
        let p2026a = optional_minor(get(row, "roc_pct_2026_actual"))?;
        let p2024 = optional_minor(get(row, "roc_pct_2024_actual"))?;
        let roc_scale = [p2025, p2026e, p2026a, p2024]
            .into_iter()
            .flatten()
            .map(|(_, s)| s)
            .max();
        let to_scaled = |pair: Option<(i64, u8)>| -> Option<i64> {
            let (minor, scale) = pair?;
            let target = roc_scale.unwrap_or(scale);
            if target == scale {
                Some(minor)
            } else if target > scale {
                Some(minor * 10i64.pow((target - scale) as u32))
            } else {
                Some(minor / 10i64.pow((scale - target) as u32))
            }
        };
        rows.push(ProductionSeedCharacteristic {
            symbol,
            payment_frequency: {
                let raw = get(row, "payment_frequency").trim();
                if raw.is_empty() {
                    "None".into()
                } else {
                    raw.to_string()
                }
            },
            risk_tier: get(row, "risk_tier").to_string(),
            provider: get(row, "provider").to_string(),
            underlying: get(row, "underlying").to_string(),
            roc_pct_2025_actual_minor: to_scaled(p2025),
            roc_pct_2026_estimate_minor: to_scaled(p2026e),
            roc_pct_2026_actual_minor: to_scaled(p2026a),
            roc_pct_2024_actual_minor: to_scaled(p2024),
            roc_scale,
            div_type: get(row, "div_type").to_string(),
            needs_roc_research: matches!(
                get(row, "needs_roc_research").trim().to_ascii_uppercase().as_str(),
                "YES" | "1" | "TRUE"
            ),
            notes: get(row, "notes").to_string(),
            is_active: !matches!(
                get(row, "is_active").trim().to_ascii_uppercase().as_str(),
                "NO" | "0" | "FALSE" | "INACTIVE"
            ),
        });
    }
    Ok(rows)
}

fn merge_plans(
    production_dir: &Path,
    positions: &[HashMap<String, String>],
) -> Result<Vec<ProductionSeedPlan>, String> {
    let seed_path = production_dir.join("calculator-plan-seed.yaml");
    let seed: CalculatorPlanSeedFile = if seed_path.is_file() {
        let raw = std::fs::read_to_string(&seed_path).map_err(|e| e.to_string())?;
        serde_yaml::from_str(&raw).map_err(|e| e.to_string())?
    } else {
        return Err("calculator-plan-seed.yaml missing".into());
    };
    let mut by_symbol: HashMap<String, CalculatorPlanSeedRow> = HashMap::new();
    for row in seed.symbols {
        by_symbol.insert(row.symbol.to_ascii_uppercase(), row);
    }
    let mut freq_by_symbol: HashMap<String, String> = HashMap::new();
    let mut template_plan: HashMap<String, String> = HashMap::new();
    for row in positions {
        let symbol = get(row, "symbol").to_ascii_uppercase();
        if symbol.is_empty() {
            continue;
        }
        freq_by_symbol.insert(symbol.clone(), get(row, "payment_frequency").to_string());
        let plan = get(row, "current_plan_per_share");
        if !plan.is_empty() {
            template_plan.insert(symbol, plan.to_string());
        }
    }
    let mut out = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut consider: Vec<String> = by_symbol.keys().cloned().collect();
    consider.extend(template_plan.keys().cloned());
    consider.sort();
    for symbol in consider {
        if !seen.insert(symbol.clone()) {
            continue;
        }
        let yaml = by_symbol.get(&symbol);
        let plan_raw = template_plan
            .get(&symbol)
            .cloned()
            .or_else(|| yaml.map(|r| r.plan.clone()));
        let Some(plan_raw) = plan_raw else {
            continue;
        };
        let freq = freq_by_symbol.get(&symbol).map(|s| s.as_str()).unwrap_or("");
        let periods = financial_domain::calculator::periods_from_frequency(freq)
            .or_else(|| yaml.map(|r| r.periods))
            .unwrap_or(0);
        if periods == 0 {
            continue;
        }
        let scale = decimal_scale(&plan_raw).max(1);
        out.push(ProductionSeedPlan {
            symbol,
            amount_per_share_minor: to_minor(&plan_raw, scale)?,
            amount_scale: scale,
            planning_periods_per_year: periods,
            effective_from: seed.effective_from.clone(),
            decision_reason: seed.decision_reason.clone(),
        });
    }
    Ok(out)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YieldTemplateAudit {
    pub row_count: u64,
    pub unique_count: u64,
    pub unique_amount_minor: i64,
    pub extra_duplicate_rows: Vec<String>,
}

/// Exact account+symbol+date+amount copies in the locked yield template.
/// Import posts one row per key; extras are skipped, not invented as $0.
pub fn audit_yield_template(production_dir: &Path) -> Result<YieldTemplateAudit, String> {
    let yield_rows = read_data_rows(&production_dir.join("Template_Transactions_Yield.xlsx"))?;
    let mut seen = HashSet::new();
    let mut extra = Vec::new();
    let mut unique_amount_minor = 0i64;
    for row in &yield_rows {
        let amount = get(row, "amount");
        if amount.is_empty() {
            return Err("yield blank amount (must stay unknown, not coerced)".into());
        }
        let amount_minor = to_minor(amount, 2)?;
        let key = format!(
            "{}|{}|{}|{}",
            get(row, "account_name"),
            data_symbol(get(row, "account_name"), get(row, "symbol")),
            as_iso_date(get(row, "txn_date")),
            amount_minor
        );
        if seen.insert(key.clone()) {
            unique_amount_minor += amount_minor;
        } else {
            extra.push(key);
        }
    }
    extra.sort();
    Ok(YieldTemplateAudit {
        row_count: yield_rows.len() as u64,
        unique_count: seen.len() as u64,
        unique_amount_minor,
        extra_duplicate_rows: extra,
    })
}

/// Template money from the xlsx source facts (not from posted ledger).
pub fn production_template_totals(
    production_dir: &Path,
) -> Result<(i64, i64, i64), String> {
    let yield_rows = read_data_rows(&production_dir.join("Template_Transactions_Yield.xlsx"))?;
    let disbursements =
        read_data_rows(&production_dir.join("Template_Transactions_Disbursement.xlsx"))?;
    let mut yield_amount_minor: i64 = 0;
    for row in &yield_rows {
        let amount = get(row, "amount");
        if amount.is_empty() {
            return Err("yield blank amount (must stay unknown, not coerced)".into());
        }
        yield_amount_minor += to_minor(amount, 2)?;
    }
    let mut disbursement_gross_minor: i64 = 0;
    let mut withheld_minor: i64 = 0;
    for row in &disbursements {
        let gross = get(row, "amount_gross");
        if gross.is_empty() {
            return Err("disbursement blank amount_gross (must stay unknown)".into());
        }
        disbursement_gross_minor += to_minor(gross, 2)?;
        if !get(row, "fed_tax_withheld").is_empty() {
            withheld_minor += to_minor(get(row, "fed_tax_withheld"), 2)?;
        }
        if !get(row, "state_tax_withheld").is_empty() {
            withheld_minor += to_minor(get(row, "state_tax_withheld"), 2)?;
        }
    }
    Ok((
        yield_amount_minor,
        disbursement_gross_minor,
        disbursement_gross_minor - withheld_minor,
    ))
}

/// Open-lot original cost and tax basis from the locked lots template, in USD cents.
pub fn production_template_basis_totals(production_dir: &Path) -> Result<(i64, i64), String> {
    let doc = parse_production_templates(production_dir)?;
    let mut open_performance_minor = 0i64;
    let mut open_tax_minor = 0i64;
    for lot in &doc.lots {
        if !lot.is_open {
            continue;
        }
        open_performance_minor +=
            financial_domain::money::to_usd_cents(lot.performance_basis_minor, lot.scale);
        open_tax_minor += financial_domain::money::to_usd_cents(lot.tax_basis_minor, lot.scale);
    }
    Ok((open_performance_minor, open_tax_minor))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn production_template_basis_totals_from_locked_xlsx() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../database/seed/production");
        let (open_performance_minor, open_tax_minor) =
            production_template_basis_totals(&dir).expect("locked lots template");
        assert_eq!(open_performance_minor, 46_694_666);
        assert_eq!(open_tax_minor, 46_135_629);
    }

    #[test]
    fn robinhood_btc_parses_as_btc_usd_grayscale_stays_btc() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../database/seed/production");
        let doc = parse_production_templates(&dir).expect("locked templates");
        assert!(
            doc.lots
                .iter()
                .any(|l| l.symbol == "BTC-USD" && l.account_name.eq_ignore_ascii_case("Robinhood")),
            "Robinhood crypto lots must use BTC-USD"
        );
        assert!(
            doc.lots
                .iter()
                .any(|l| l.symbol == "BTC" && !l.account_name.eq_ignore_ascii_case("Robinhood")),
            "Grayscale BTC lots must stay BTC"
        );
        assert!(
            !doc.lots
                .iter()
                .any(|l| l.symbol.eq_ignore_ascii_case("BTC")
                    && l.account_name.eq_ignore_ascii_case("Robinhood")),
            "Robinhood must not keep a BTC symbol after remap"
        );
        assert!(doc.securities.iter().any(|s| s.symbol == "BTC"));
        assert!(doc.securities.iter().any(|s| s.symbol == "BTC-USD"));
    }

    #[test]
    fn seed_payment_frequencies_are_cadence_or_empty() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../database/seed/production");
        let doc = parse_production_templates(&dir).expect("locked templates");
        let mut bad = Vec::new();
        for row in &doc.characteristics {
            if financial_domain::calculator::PaymentCadence::parse(&row.payment_frequency).is_none()
            {
                bad.push((row.symbol.clone(), row.payment_frequency.clone()));
            }
        }
        assert!(
            bad.is_empty(),
            "seed payment_frequency must be Weekly/Monthly/Quarterly/None: {bad:?}"
        );
        assert_eq!(
            doc.characteristics
                .iter()
                .find(|r| r.symbol == "ENERGYX")
                .map(|r| r.payment_frequency.as_str()),
            Some("None")
        );
    }

    #[test]
    fn locked_yield_template_names_exact_duplicate_rows() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../database/seed/production");
        let audit = audit_yield_template(&dir).expect("yield audit");
        assert_eq!(audit.row_count, 5862, "locked yield template row count");
        assert_eq!(audit.unique_count, 5857, "unique account|symbol|date|amount");
        assert_eq!(audit.unique_amount_minor, 16_763_298);
        assert_eq!(
            audit.extra_duplicate_rows.len(),
            5,
            "exact copies skipped on post: {:?}",
            audit.extra_duplicate_rows
        );
        eprintln!("exact yield copies skipped: {:?}", audit.extra_duplicate_rows);
    }
}
