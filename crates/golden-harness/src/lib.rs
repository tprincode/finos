//! Golden Business Outcome Harness runner (ADR-0013).
//! Production-path commands only. Expected MAGI oracles are never generated here.

use std::fs;
use std::path::{Path, PathBuf};

use chrono::Datelike;

use application_core::contracts::{
    CommandRequest, QueryRequest, ReconcileCounts, FINANCE_CLIENT_CONTRACT_VERSION,
};
use application_core::ports::canonical::Canonical;
use application_core::queries::{execute_command_on, execute_query_on};
use serde::{Deserialize, Serialize};
use storage_sqlite::LocalPlatform;
use uuid::Uuid;

mod magi_run;
mod production_seed;

pub use magi_run::{compare_magi_pack, compare_magi_pack_postgres, magi_pack_run};
pub use production_seed::{
    load_production_expected, load_production_seed_via_commands, production_seed_actual_counts,
    production_seed_actual_totals, production_seed_plan_count, production_template_totals,
    profile_a_app_dir, ProductionCounts, ProductionExpected, ProductionTotals,
};

pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MagiPackVerdict {
    BlockedPendingOwner,
    Passed,
    Failed,
}

#[derive(Debug, Deserialize)]
struct ScenarioFile {
    scenario_id: String,
    approval: ApprovalBlock,
    #[serde(default)]
    input_facts: serde_yaml::Value,
}

#[derive(Debug, Deserialize)]
struct ApprovalBlock {
    status: String,
}

#[derive(Debug, Deserialize)]
struct OracleFile {
    scenario_id: String,
    status: String,
    #[serde(default)]
    applicable_threshold_minor: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct MagiRuleFile {
    threshold_minor: i64,
    safety_reserve_minor: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MagiPackInventory {
    pub scenario_ids: Vec<String>,
    pub oracle_ids: Vec<String>,
    pub all_pending_owner: bool,
    pub all_oracles_proposed: bool,
    pub all_scenarios_approved: bool,
    pub all_oracles_approved: bool,
    pub facts_present: bool,
    pub threshold_minor: i64,
    pub safety_reserve_minor: i64,
}

fn yaml_mapping_nonempty(value: &serde_yaml::Value) -> bool {
    match value {
        serde_yaml::Value::Mapping(map) => !map.is_empty(),
        serde_yaml::Value::Sequence(seq) => !seq.is_empty(),
        _ => false,
    }
}

/// Inventory of the MAGI pack. Does not approve oracles and does not run MAGI math.
pub fn magi_pack_inventory(root: &Path) -> Result<MagiPackInventory, String> {
    let scenarios_dir = root.join("tests/golden/scenarios");
    let oracle_dir = root.join("tests/golden/oracle");
    let rule: MagiRuleFile = serde_yaml::from_str(
        &fs::read_to_string(root.join("tests/golden/rules/marketplace-magi-2026.yaml"))
            .map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let mut scenario_ids = Vec::new();
    let mut all_pending_owner = true;
    let mut all_scenarios_approved = true;
    let mut facts_present = true;
    let entries = fs::read_dir(&scenarios_dir).map_err(|e| e.to_string())?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with("G-MAGI-") || !name.ends_with(".yaml") {
            continue;
        }
        let parsed: ScenarioFile = serde_yaml::from_str(
            &fs::read_to_string(entry.path()).map_err(|e| e.to_string())?,
        )
        .map_err(|e| format!("{name}: {e}"))?;
        if parsed.approval.status != "pending-owner" {
            all_pending_owner = false;
        }
        if parsed.approval.status != "approved" {
            all_scenarios_approved = false;
        }
        if !yaml_mapping_nonempty(&parsed.input_facts) {
            facts_present = false;
        }
        scenario_ids.push(parsed.scenario_id);
    }
    scenario_ids.sort();
    let mut oracle_ids = Vec::new();
    let mut all_oracles_proposed = true;
    let mut all_oracles_approved = true;
    let oracle_entries = fs::read_dir(&oracle_dir).map_err(|e| e.to_string())?;
    for entry in oracle_entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with("G-MAGI-") || !name.ends_with(".yaml") {
            continue;
        }
        let parsed: OracleFile = serde_yaml::from_str(
            &fs::read_to_string(entry.path()).map_err(|e| e.to_string())?,
        )
        .map_err(|e| format!("{name}: {e}"))?;
        if parsed.status != "proposed" {
            all_oracles_proposed = false;
        }
        if parsed.status != "approved" {
            all_oracles_approved = false;
        }
        if parsed.applicable_threshold_minor != Some(rule.threshold_minor) {
            return Err(format!(
                "{} threshold must equal rule threshold {}",
                parsed.scenario_id, rule.threshold_minor
            ));
        }
        oracle_ids.push(parsed.scenario_id);
    }
    oracle_ids.sort();
    Ok(MagiPackInventory {
        scenario_ids,
        oracle_ids,
        all_pending_owner,
        all_oracles_proposed,
        all_scenarios_approved,
        all_oracles_approved,
        facts_present,
        threshold_minor: rule.threshold_minor,
        safety_reserve_minor: rule.safety_reserve_minor,
    })
}

pub fn magi_pack_verdict(scenarios_dir: &Path) -> MagiPackVerdict {
    let mut pending = false;
    let mut found = 0u32;
    let Ok(entries) = fs::read_dir(scenarios_dir) else {
        return MagiPackVerdict::Failed;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with("G-MAGI-") || !name.ends_with(".yaml") {
            continue;
        }
        found += 1;
        let raw = fs::read_to_string(entry.path()).unwrap_or_default();
        let parsed: ScenarioFile = serde_yaml::from_str(&raw).unwrap_or(ScenarioFile {
            scenario_id: name.to_string(),
            approval: ApprovalBlock {
                status: "invalid".into(),
            },
            input_facts: serde_yaml::Value::Null,
        });
        if parsed.approval.status != "approved" {
            pending = true;
        }
        let _ = parsed.scenario_id;
    }
    if found == 0 {
        return MagiPackVerdict::Failed;
    }
    if pending {
        MagiPackVerdict::BlockedPendingOwner
    } else {
        // Owner-approved oracles still require a production-path comparator (M5).
        MagiPackVerdict::Failed
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceReport {
    pub scenario_id: String,
    pub scenario_version: String,
    pub rule_set_id: String,
    pub calculation_version: String,
    pub git_commit: Option<String>,
    pub adapter: String,
    pub platform: String,
    pub recorded_at: String,
    pub verdict: String,
    pub diffs: Vec<String>,
}

pub fn write_evidence_report(dir: &Path, report: &EvidenceReport) -> std::io::Result<PathBuf> {
    fs::create_dir_all(dir)?;
    let path = dir.join(format!("{}-evidence.json", report.scenario_id));
    fs::write(&path, serde_json::to_vec_pretty(report).unwrap())?;
    Ok(path)
}

#[derive(Debug, Deserialize)]
struct SeedFixture {
    accounts: Vec<NamedAccount>,
    securities: Vec<NamedSecurity>,
    import: SeedImport,
}

#[derive(Debug, Deserialize)]
struct NamedAccount {
    name: String,
    kind: String,
}

#[derive(Debug, Deserialize)]
struct NamedSecurity {
    symbol: String,
    name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SeedImport {
    source_id: String,
    filename: String,
    content: String,
    candidates: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct ExpectedCounts {
    pub accounts: u64,
    pub securities: u64,
    pub evidence: u64,
    pub import_batches: u64,
    pub posted_activities: u64,
    pub amount_minor_sum: i64,
    pub scale: u8,
    pub audit_records: u64,
    pub exceptions_open: u64,
}

impl From<ReconcileCounts> for ExpectedCounts {
    fn from(c: ReconcileCounts) -> Self {
        Self {
            accounts: c.accounts,
            securities: c.securities,
            evidence: c.evidence,
            import_batches: c.import_batches,
            posted_activities: c.posted_activities,
            amount_minor_sum: c.amount_minor_sum,
            scale: c.scale,
            audit_records: c.audit_records,
            exceptions_open: c.exceptions_open,
        }
    }
}

/// Fill required collector fields so the first LotOpen is allowed.
/// Required Skip is not complete; remaining-year dates match periods through 31 Dec.
pub async fn complete_collector_for_first_lot(
    platform: &LocalPlatform,
    security_id: &str,
    symbol: &str,
) -> Result<(), String> {
    complete_collector_for_first_lot_as(platform, security_id, symbol, "Monthly", true).await
}

pub async fn complete_collector_for_first_lot_as(
    platform: &LocalPlatform,
    security_id: &str,
    symbol: &str,
    cadence: &str,
    replace_dates: bool,
) -> Result<(), String> {
    let as_of = chrono::Utc::now().date_naive();
    let as_of_s = as_of.to_string();
    let periods_per_year = financial_domain::calculator::PaymentCadence::parse(cadence)
        .and_then(financial_domain::calculator::PaymentCadence::periods)
        .unwrap_or(12);
    let periods = financial_domain::schedule::remaining_periods_to_year_end(&as_of_s, periods_per_year)
        .unwrap_or(1);
    let mut dates = Vec::new();
    match periods_per_year {
        12 => {
            for month in as_of.month()..=12 {
                let pay_on = if month == 12 {
                    chrono::NaiveDate::from_ymd_opt(as_of.year(), 12, 31)
                } else {
                    chrono::NaiveDate::from_ymd_opt(as_of.year(), month + 1, 1)
                        .and_then(|d| d.pred_opt())
                };
                if let Some(d) = pay_on {
                    dates.push(serde_json::json!({ "payOn": d.to_string(), "source": "test" }));
                }
                if dates.len() >= usize::from(periods) {
                    break;
                }
            }
        }
        4 => {
            let start_q = ((as_of.month() - 1) / 3) + 1;
            for q in start_q..=4 {
                let month = q * 3;
                let pay_on = if month == 12 {
                    chrono::NaiveDate::from_ymd_opt(as_of.year(), 12, 31)
                } else {
                    chrono::NaiveDate::from_ymd_opt(as_of.year(), month + 1, 1)
                        .and_then(|d| d.pred_opt())
                };
                if let Some(d) = pay_on {
                    dates.push(serde_json::json!({ "payOn": d.to_string(), "source": "test" }));
                }
            }
        }
        _ => {
            let year_end = chrono::NaiveDate::from_ymd_opt(as_of.year(), 12, 31).unwrap();
            let mut week = financial_domain::week::week_containing(as_of);
            while week.start <= year_end && dates.len() < usize::from(periods) {
                if week.end >= as_of {
                    dates.push(serde_json::json!({
                        "payOn": week.end.to_string(),
                        "source": "test"
                    }));
                }
                week.start += chrono::Duration::days(7);
                week.end += chrono::Duration::days(7);
            }
        }
    }
    let mut steps = Vec::new();
    let existing = execute_query_on(
        platform,
        platform,
        qry_json(
            "InvestmentGet",
            serde_json::json!({ "securityId": security_id, "asOfDate": as_of_s }),
        ),
    )
    .await;
    let existing_json: serde_json::Value = serde_json::from_str(
        existing.body_json.as_deref().unwrap_or("{}"),
    )
    .unwrap_or_else(|_| serde_json::json!({}));
    let tpl = existing_json.get("template").cloned().unwrap_or_default();
    steps.push((
        "RetrievalTemplateSet",
        serde_json::json!({
            "securityId": security_id,
            "priceSource": tpl.get("priceSource").and_then(|v| v.as_str()).unwrap_or("public"),
            "sourceSymbol": tpl.get("sourceSymbol").and_then(|v| v.as_str()).unwrap_or(symbol),
            "declarationSource": tpl.get("declarationSource").and_then(|v| v.as_str()).unwrap_or("issuer"),
            "sourceUrl": tpl.get("sourceUrl").and_then(|v| v.as_str()).unwrap_or(""),
            "calendarPolicy": tpl.get("calendarPolicy").and_then(|v| v.as_str()).unwrap_or("issuer_calendar"),
            "collectorEnabled": tpl.get("collectorEnabled").and_then(|v| v.as_bool()).unwrap_or(true),
            "lookbackCount": tpl.get("lookbackCount").and_then(|v| v.as_u64()).unwrap_or(12),
            "inceptionOn": as_of_s
        }),
    ));
    steps.push((
            "PositionCharacteristicUpsert",
            serde_json::json!({
                "securityId": security_id,
                "paymentFrequency": cadence,
                "replaceCadence": true,
                "divType": "DIV-1",
                "underlying": "HACK",
                "provider": "Amplify",
                "riskTier": "Foundation",
                "rocPct2026EstimateMinor": 5000,
                "rocScale": 2,
                "needsRocResearch": false
            }),
    ));
    if replace_dates && !dates.is_empty() {
        steps.push((
            "IssuerPayDateReplace",
            serde_json::json!({
                "securityId": security_id,
                "asOfDate": as_of_s,
                "dates": dates
            }),
        ));
    }
    for (name, body) in steps {
        let result = execute_command_on(platform, platform, cmd(name, body)).await;
        if !result.ok {
            return Err(format!(
                "{name}: {}",
                result.error_code.unwrap_or_else(|| "failed".into())
            ));
        }
    }
    if let Ok(sid) = Uuid::parse_str(security_id) {
        let prior = as_of
            .pred_opt()
            .unwrap_or(as_of)
            .format("%Y-%m-%d")
            .to_string();
        let _ = platform
            .retrieval_template_touch_run(
                sid,
                true,
                "test collector complete".into(),
                prior,
                String::new(),
                "",
            )
            .await;
    }
    Ok(())
}

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

fn qry_json(name: &str, body: serde_json::Value) -> QueryRequest {
    QueryRequest {
        contract_version: FINANCE_CLIENT_CONTRACT_VERSION.to_string(),
        query_name: name.to_string(),
        correlation_id: Uuid::new_v4(),
        body_json: Some(body.to_string()),
    }
}

/// Load the M2 seed through the same command dispatch as the desktop host.
pub async fn load_seed_via_commands(
    platform: &LocalPlatform,
    fixture_path: &Path,
) -> Result<(), String> {
    let raw = fs::read_to_string(fixture_path).map_err(|e| e.to_string())?;
    let fixture: SeedFixture = serde_yaml::from_str(&raw).map_err(|e| e.to_string())?;
    for account in fixture.accounts {
        let result = execute_command_on(
            platform,
            platform,
            cmd(
                "AccountRegister",
                serde_json::json!({"name": account.name, "kind": account.kind}),
            ),
        )
        .await;
        if !result.ok {
            return Err(result.error_code.unwrap_or_else(|| "account".into()));
        }
    }
    for security in fixture.securities {
        let result = execute_command_on(
            platform,
            platform,
            cmd(
                "SecurityRegister",
                serde_json::json!({"symbol": security.symbol, "name": security.name}),
            ),
        )
        .await;
        if !result.ok {
            return Err(result.error_code.unwrap_or_else(|| "security".into()));
        }
    }
    let stage_body = serde_json::json!({
        "sourceId": fixture.import.source_id,
        "filename": fixture.import.filename,
        "content": fixture.import.content,
        "candidates": fixture.import.candidates,
    });
    let staged = execute_command_on(platform, platform, cmd("ImportStage", stage_body)).await;
    if !staged.ok {
        return Err(staged.error_code.unwrap_or_else(|| "stage".into()));
    }
    let batch: serde_json::Value =
        serde_json::from_str(staged.body_json.as_deref().unwrap_or("{}")).map_err(|e| e.to_string())?;
    let batch_id = batch
        .get("batchId")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "missing batchId".to_string())?;
    for name in ["ImportValidate", "ImportApprove", "ImportPost"] {
        let result = execute_command_on(
            platform,
            platform,
            cmd(name, serde_json::json!({"batchId": batch_id})),
        )
        .await;
        if !result.ok {
            return Err(format!(
                "{name}: {}",
                result.error_code.unwrap_or_else(|| "failed".into())
            ));
        }
    }
    let _ = execute_query_on(platform, platform, qry("ReconcileCountsGet")).await;
    Ok(())
}

pub fn load_expected(path: &Path) -> Result<ExpectedCounts, String> {
    let raw = fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_yaml::from_str(&raw).map_err(|e| e.to_string())
}

pub fn core_crate_tomls_forbid_sqlx_and_tauri(root: &Path) -> Result<(), String> {
    for rel in ["crates/application-core/Cargo.toml", "crates/financial-domain/Cargo.toml"] {
        let text = fs::read_to_string(root.join(rel)).map_err(|e| e.to_string())?;
        for forbidden in ["sqlx", "tauri"] {
            for line in text.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with('#') {
                    continue;
                }
                if trimmed.starts_with(forbidden)
                    || trimmed.starts_with(&format!("{forbidden}."))
                    || trimmed.starts_with(&format!("{forbidden} ="))
                {
                    return Err(format!("{rel} must not depend on {forbidden}"));
                }
            }
        }
    }
    Ok(())
}

pub fn desktop_ui_contains_no_sql(src_dir: &Path) -> Result<(), String> {
    fn walk(dir: &Path, hits: &mut Vec<String>) -> std::io::Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                walk(&path, hits)?;
                continue;
            }
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if !matches!(ext, "ts" | "tsx" | "js" | "jsx") {
                continue;
            }
            let text = fs::read_to_string(&path)?;
            for (i, line) in text.lines().enumerate() {
                let lower = line.to_ascii_lowercase();
                if (lower.contains("select ") && !lower.contains("select all"))
                    || lower.contains("insert into")
                    || lower.contains("delete from")
                    || lower.contains("sqlite")
                    || lower.contains("executesql")
                {
                    hits.push(format!("{}:{}: {line}", path.display(), i + 1));
                }
            }
        }
        Ok(())
    }
    let mut hits = Vec::new();
    walk(src_dir, &mut hits).map_err(|e| e.to_string())?;
    if hits.is_empty() {
        Ok(())
    } else {
        Err(hits.join("\n"))
    }
}
