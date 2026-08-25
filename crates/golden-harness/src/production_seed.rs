//! Load FMS production templates through the ProductionSeedLoad FinanceClient command.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use application_core::contracts::{
    CommandRequest, QueryRequest, FINANCE_CLIENT_CONTRACT_VERSION,
};
use application_core::queries::{execute_command_on, execute_query_on};
use serde::Deserialize;
use serde_json::Value;
use storage_sqlite::LocalPlatform;
use uuid::Uuid;

const EXTERNAL_ACCOUNT: &str = "External";

#[derive(Debug, Deserialize)]
pub struct ProductionExpected {
    pub counts: ProductionCounts,
    pub totals: ProductionTotals,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct ProductionCounts {
    pub accounts: u64,
    pub positions: u64,
    pub lots_total: u64,
    pub lots_open: u64,
    pub transactions_yield: u64,
    pub transactions_disbursement: u64,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct ProductionTotals {
    pub yield_amount_minor: i64,
    pub disbursement_gross_minor: i64,
    pub disbursement_net_minor: i64,
    #[serde(default)]
    pub open_performance_minor: i64,
    #[serde(default)]
    pub open_tax_minor: i64,
    #[serde(default = "scale_two")]
    pub scale: u8,
}

fn scale_two() -> u8 {
    2
}

/// Same directory the desktop Tauri host opens (`app_local_data_dir`, OneDrive skipped).
pub fn profile_a_app_dir() -> PathBuf {
    let local = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let identifier = local.join("com.finos.desktop");
    let s = identifier.to_string_lossy().to_lowercase();
    if s.contains("onedrive") || s.contains("dropbox") || s.contains("icloud") {
        local.join("finos")
    } else {
        identifier
    }
}

pub fn load_production_expected(path: &Path) -> Result<ProductionExpected, String> {
    let raw = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_yaml::from_str(&raw).map_err(|e| e.to_string())
}

fn cmd(name: &str, body: Value) -> CommandRequest {
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

async fn must(platform: &LocalPlatform, name: &str, body: Value) -> Result<Value, String> {
    let result = execute_command_on(platform, platform, cmd(name, body)).await;
    if !result.ok {
        return Err(format!(
            "{name}: {}",
            result.error_code.unwrap_or_else(|| "failed".into())
        ));
    }
    serde_json::from_str(result.body_json.as_deref().unwrap_or("{}")).map_err(|e| e.to_string())
}

/// Parse xlsx in import-engine, then apply through ProductionSeedLoad (no UI SQL).
pub async fn load_production_seed_via_commands(
    platform: &LocalPlatform,
    production_dir: &Path,
) -> Result<(), String> {
    let doc = import_engine::parse_production_templates(production_dir)?;
    let body = serde_json::to_value(&doc).map_err(|e| e.to_string())?;
    must(platform, "ProductionSeedLoad", body).await?;
    Ok(())
}

pub async fn production_seed_actual_counts(
    platform: &LocalPlatform,
) -> Result<ProductionCounts, String> {
    let accounts = execute_query_on(platform, platform, qry("AccountList")).await;
    if !accounts.ok {
        return Err("AccountList failed".into());
    }
    let account_val: Value =
        serde_json::from_str(accounts.body_json.as_deref().unwrap_or("[]")).map_err(|e| e.to_string())?;
    let details = execute_query_on(platform, platform, qry("PositionDetailsGet")).await;
    if !details.ok {
        return Err("PositionDetailsGet failed".into());
    }
    let details_val: Value =
        serde_json::from_str(details.body_json.as_deref().unwrap_or("{}")).map_err(|e| e.to_string())?;
    let mut position_symbols = HashSet::new();
    if let Some(lines) = details_val["positions"].as_array() {
        for line in lines {
            if let Some(symbol) = line["symbol"].as_str() {
                position_symbols.insert(symbol.to_string());
            }
        }
    }
    let basis = execute_query_on(platform, platform, qry("BasisGet")).await;
    if !basis.ok {
        return Err("BasisGet failed".into());
    }
    let basis_val: Value =
        serde_json::from_str(basis.body_json.as_deref().unwrap_or("{}")).map_err(|e| e.to_string())?;
    let lots = basis_val["lots"].as_array().cloned().unwrap_or_default();
    let lots_open = lots
        .iter()
        .filter(|l| l["remainingQuantityMinor"].as_i64().unwrap_or(0) > 0)
        .count() as u64;
    let activities = execute_query_on(platform, platform, qry("ActivityList")).await;
    if !activities.ok {
        return Err("ActivityList failed".into());
    }
    let activity_val: Value =
        serde_json::from_str(activities.body_json.as_deref().unwrap_or("[]")).map_err(|e| e.to_string())?;
    let acts = activity_val.as_array().cloned().unwrap_or_default();
    let yield_n = acts
        .iter()
        .filter(|a| {
            a["activityType"]
                .as_str()
                .unwrap_or("")
                .eq_ignore_ascii_case("dividend")
        })
        .count() as u64;
    let disbursement_n = acts
        .iter()
        .filter(|a| {
            matches!(
                a["activityType"].as_str().unwrap_or(""),
                "IRA_Distribution"
                    | "Withdrawal"
                    | "Form_1099"
                    | "SSA"
                    | "Roth_Distribution"
            )
        })
        .count() as u64;
    let household_accounts = account_val
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter(|a| a["name"].as_str() != Some(EXTERNAL_ACCOUNT))
                .count() as u64
        })
        .unwrap_or(0);
    Ok(ProductionCounts {
        accounts: household_accounts,
        positions: position_symbols.len() as u64,
        lots_total: lots.len() as u64,
        lots_open,
        transactions_yield: yield_n,
        transactions_disbursement: disbursement_n,
    })
}

fn is_disbursement_type(activity_type: &str) -> bool {
    matches!(
        activity_type,
        "IRA_Distribution" | "Withdrawal" | "Form_1099" | "SSA" | "Roth_Distribution"
    )
}

fn to_scale_2(amount_minor: i64, scale: u8) -> i64 {
    if scale == 2 {
        return amount_minor;
    }
    if scale > 2 {
        let div = 10i64.pow((scale - 2) as u32);
        let q = amount_minor / div;
        let r = (amount_minor % div).abs();
        if r * 2 >= div {
            q + if amount_minor >= 0 { 1 } else { -1 }
        } else {
            q
        }
    } else {
        amount_minor * 10i64.pow((2 - scale) as u32)
    }
}

pub async fn production_seed_actual_totals(
    platform: &LocalPlatform,
) -> Result<ProductionTotals, String> {
    let dividend = execute_query_on(platform, platform, qry("DividendGet")).await;
    if !dividend.ok {
        return Err("DividendGet failed".into());
    }
    let dividend_val: Value =
        serde_json::from_str(dividend.body_json.as_deref().unwrap_or("{}")).map_err(|e| e.to_string())?;
    let yield_amount_minor = dividend_val["actualTotalMinor"].as_i64().unwrap_or(0);

    let activities = execute_query_on(platform, platform, qry("ActivityList")).await;
    if !activities.ok {
        return Err("ActivityList failed".into());
    }
    let activity_val: Value =
        serde_json::from_str(activities.body_json.as_deref().unwrap_or("[]")).map_err(|e| e.to_string())?;
    let acts = activity_val.as_array().cloned().unwrap_or_default();
    let disbursement_gross_minor = acts
        .iter()
        .filter(|a| is_disbursement_type(a["activityType"].as_str().unwrap_or("")))
        .map(|a| {
            to_scale_2(
                a["amountMinor"].as_i64().unwrap_or(0),
                a["scale"].as_u64().unwrap_or(2) as u8,
            )
        })
        .sum();
    let basis = execute_query_on(platform, platform, qry("BasisGet")).await;
    if !basis.ok {
        return Err("BasisGet failed".into());
    }
    let basis_val: Value =
        serde_json::from_str(basis.body_json.as_deref().unwrap_or("{}")).map_err(|e| e.to_string())?;
    Ok(ProductionTotals {
        yield_amount_minor,
        disbursement_gross_minor,
        disbursement_net_minor: 0,
        open_performance_minor: basis_val["openPerformanceMinor"].as_i64().unwrap_or(0),
        open_tax_minor: basis_val["openTaxMinor"].as_i64().unwrap_or(0),
        scale: 2,
    })
}

pub async fn production_seed_plan_count(platform: &LocalPlatform) -> Result<u64, String> {
    let result = execute_query_on(platform, platform, qry("HouseholdSummaryGet")).await;
    if !result.ok {
        return Err("HouseholdSummaryGet failed".into());
    }
    let val: Value =
        serde_json::from_str(result.body_json.as_deref().unwrap_or("{}")).map_err(|e| e.to_string())?;
    Ok(val["planCount"].as_u64().unwrap_or(0))
}

/// Template money from the xlsx source facts (not from posted ledger).
pub fn production_template_totals(production_dir: &Path) -> Result<ProductionTotals, String> {
    let (yield_amount_minor, disbursement_gross_minor, disbursement_net_minor) =
        import_engine::production_template_totals(production_dir)?;
    let (open_performance_minor, open_tax_minor) =
        import_engine::production_template_basis_totals(production_dir)?;
    Ok(ProductionTotals {
        yield_amount_minor,
        disbursement_gross_minor,
        disbursement_net_minor,
        open_performance_minor,
        open_tax_minor,
        scale: 2,
    })
}
