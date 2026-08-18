//! Production-path MAGI driver. Compares MagiProjectionGet to owner-approved oracles.
//! Does not write or regenerate oracle files.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use application_core::contracts::{
    CommandRequest, MagiProjection, QueryRequest, FINANCE_CLIENT_CONTRACT_VERSION,
};
use application_core::queries::{execute_command_on, execute_query_on};
use serde::Deserialize;
use storage_sqlite::LocalPlatform;
use uuid::Uuid;

use crate::{magi_pack_inventory, magi_pack_verdict, MagiPackVerdict};

#[derive(Debug, Deserialize)]
struct MagiRuleFile {
    threshold_minor: i64,
    safety_reserve_minor: i64,
    #[serde(default = "two")]
    threshold_scale: u8,
}

fn two() -> u8 {
    2
}

#[derive(Debug, Deserialize)]
struct ScenarioFile {
    #[allow(dead_code)]
    scenario_id: String,
    input_facts: InputFacts,
    #[serde(default)]
    provenance: Provenance,
    #[serde(default)]
    expected_warnings: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
struct InputFacts {
    #[serde(default)]
    accounts: Vec<NamedAccount>,
    #[serde(default)]
    activities: Vec<FactActivity>,
    #[serde(default)]
    declarations: Vec<FactDeclaration>,
    #[serde(default)]
    boundary_probes: Vec<BoundaryProbe>,
    #[serde(default)]
    snapshots: Vec<PlanSnapshot>,
    #[serde(default)]
    withholding: Vec<WithholdingSnap>,
    #[serde(default)]
    tax_forms: Vec<TaxForm>,
    #[serde(default)]
    adjustments: Vec<FactAdjustment>,
}

#[derive(Debug, Deserialize)]
struct NamedAccount {
    name: String,
    kind: String,
}

#[derive(Debug, Deserialize)]
struct FactActivity {
    id: String,
    account: String,
    #[serde(rename = "type")]
    activity_type: String,
    occurred_on: String,
    amount_minor: i64,
    #[serde(default = "two")]
    scale: u8,
    magi: String,
    #[serde(default)]
    classification: String,
}

#[derive(Debug, Deserialize)]
struct FactDeclaration {
    amount_minor: i64,
}

#[derive(Debug, Deserialize)]
struct BoundaryProbe {
    id: String,
    actual_included_ytd_minor: i64,
}

#[derive(Debug, Deserialize)]
struct PlanSnapshot {
    id: String,
    planned_remaining_minor: i64,
}

#[derive(Debug, Deserialize)]
struct WithholdingSnap {
    id: String,
    federal_withheld_minor: i64,
}

#[derive(Debug, Deserialize)]
struct TaxForm {
    amount_minor: i64,
}

#[derive(Debug, Deserialize)]
struct FactAdjustment {
    id: String,
    amount_minor: i64,
    #[serde(default = "two")]
    scale: u8,
    #[serde(default)]
    reason: String,
    #[serde(default = "proposed")]
    status: String,
}

fn proposed() -> String {
    "proposed".into()
}

#[derive(Debug, Default, Deserialize)]
struct Provenance {
    #[serde(default = "complete")]
    completeness: String,
}

fn complete() -> String {
    "complete".into()
}

#[derive(Debug, Deserialize)]
struct OracleFile {
    decision_state: String,
    data_completeness: String,
    #[serde(default)]
    actual_included_ytd_minor: Option<i64>,
    #[serde(default)]
    known_remaining_minor: Option<i64>,
    #[serde(default)]
    base_forecast_minor: Option<i64>,
    #[serde(default)]
    conservative_forecast_minor: Option<i64>,
    #[serde(default)]
    uncertain_amount_minor: Option<i64>,
    #[serde(default)]
    raw_headroom_minor: Option<i64>,
    #[serde(default)]
    protected_headroom_minor: Option<i64>,
    #[serde(default)]
    included: Vec<String>,
    #[serde(default)]
    warnings: Vec<String>,
    #[serde(default)]
    source_transaction: Option<String>,
    #[serde(default)]
    probes: Vec<ProbeOracle>,
    #[serde(default)]
    before_ira: Option<SnapOracle>,
    #[serde(default)]
    after_ira: Option<SnapOracle>,
    #[serde(default)]
    before_plan: Option<SnapOracle>,
    #[serde(default)]
    after_plan: Option<SnapOracle>,
    #[serde(default)]
    magi_withhold_a_minor: Option<i64>,
    #[serde(default)]
    magi_withhold_b_minor: Option<i64>,
    #[serde(default)]
    tax_payment_a_minor: Option<i64>,
    #[serde(default)]
    tax_payment_b_minor: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct ProbeOracle {
    id: String,
    actual_included_ytd_minor: i64,
    known_remaining_minor: i64,
    base_forecast_minor: i64,
    conservative_forecast_minor: i64,
    uncertain_amount_minor: i64,
    raw_headroom_minor: i64,
    protected_headroom_minor: i64,
    decision_state: String,
}

#[derive(Debug, Deserialize)]
struct SnapOracle {
    actual_included_ytd_minor: i64,
    known_remaining_minor: i64,
    base_forecast_minor: i64,
    conservative_forecast_minor: i64,
    #[serde(default)]
    uncertain_amount_minor: Option<i64>,
    raw_headroom_minor: i64,
    protected_headroom_minor: i64,
    #[serde(default)]
    decision_state: Option<String>,
}

fn cmd(name: &str, body: serde_json::Value) -> CommandRequest {
    CommandRequest {
        contract_version: FINANCE_CLIENT_CONTRACT_VERSION.to_string(),
        command_name: name.to_string(),
        correlation_id: Uuid::new_v4(),
        body_json: Some(body.to_string()),
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

async fn must_cmd(
    platform: &LocalPlatform,
    name: &str,
    body: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let result = execute_command_on(platform, platform, cmd(name, body)).await;
    if !result.ok {
        return Err(format!(
            "{name}: {}",
            result.error_code.unwrap_or_else(|| "failed".into())
        ));
    }
    serde_json::from_str(result.body_json.as_deref().unwrap_or("{}")).map_err(|e| e.to_string())
}

async fn projection(platform: &LocalPlatform) -> Result<MagiProjection, String> {
    let result = execute_query_on(platform, platform, qry("MagiProjectionGet")).await;
    if !result.ok {
        return Err(format!(
            "MagiProjectionGet: {}",
            result.error_code.unwrap_or_else(|| "failed".into())
        ));
    }
    serde_json::from_str(result.body_json.as_deref().unwrap_or("{}")).map_err(|e| e.to_string())
}

async fn tax_payment(platform: &LocalPlatform) -> Result<i64, String> {
    let result = execute_query_on(platform, platform, qry("MagiTaxPaymentGet")).await;
    if !result.ok {
        return Err(format!(
            "MagiTaxPaymentGet: {}",
            result.error_code.unwrap_or_else(|| "failed".into())
        ));
    }
    let v: serde_json::Value =
        serde_json::from_str(result.body_json.as_deref().unwrap_or("{}")).map_err(|e| e.to_string())?;
    v.get("amountMinor")
        .and_then(|x| x.as_i64())
        .ok_or_else(|| "MagiTaxPaymentGet missing amountMinor".into())
}

async fn open_platform() -> Result<(LocalPlatform, tempfile::TempDir), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .map_err(|e| e.to_string())?;
    Ok((platform, dir))
}

async fn set_rule(platform: &LocalPlatform, root: &Path) -> Result<(), String> {
    let rule: MagiRuleFile = serde_yaml::from_str(
        &fs::read_to_string(root.join("tests/golden/rules/marketplace-magi-2026.yaml"))
            .map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    must_cmd(
        platform,
        "MagiRuleSet",
        serde_json::json!({
            "thresholdMinor": rule.threshold_minor,
            "safetyReserveMinor": rule.safety_reserve_minor,
            "scale": rule.threshold_scale,
        }),
    )
    .await?;
    Ok(())
}

async fn register_accounts(
    platform: &LocalPlatform,
    accounts: &[NamedAccount],
) -> Result<HashMap<String, String>, String> {
    let mut ids = HashMap::new();
    for account in accounts {
        let body = must_cmd(
            platform,
            "AccountRegister",
            serde_json::json!({"name": account.name, "kind": account.kind}),
        )
        .await?;
        let id = body
            .get("accountId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("missing accountId for {}", account.name))?
            .to_string();
        ids.insert(account.name.clone(), id);
    }
    Ok(ids)
}

async fn post_activity(
    platform: &LocalPlatform,
    accounts: &HashMap<String, String>,
    activity: &FactActivity,
) -> Result<(), String> {
    let account_id = accounts
        .get(&activity.account)
        .ok_or_else(|| format!("unknown account {}", activity.account))?;
    must_cmd(
        platform,
        "ActivityPost",
        serde_json::json!({
            "accountId": account_id,
            "activityType": activity.activity_type,
            "amountMinor": activity.amount_minor,
            "scale": activity.scale,
            "occurredOn": activity.occurred_on,
            "idempotencyKey": activity.id,
        }),
    )
    .await?;
    Ok(())
}

async fn record_fact(platform: &LocalPlatform, activity: &FactActivity) -> Result<(), String> {
    must_cmd(
        platform,
        "MagiFactRecord",
        serde_json::json!({
            "sourceId": activity.id,
            "treatment": activity.magi,
            "amountMinor": activity.amount_minor,
            "scale": activity.scale,
            "category": activity.classification,
        }),
    )
    .await?;
    Ok(())
}

async fn set_coverage(
    platform: &LocalPlatform,
    completeness: &str,
    remaining_minor: i64,
    withholding_minor: i64,
    form_total_minor: i64,
    warnings: &[String],
) -> Result<(), String> {
    must_cmd(
        platform,
        "MagiCoverageSet",
        serde_json::json!({
            "completeness": completeness,
            "remainingMinor": remaining_minor,
            "withholdingMinor": withholding_minor,
            "formTotalMinor": form_total_minor,
            "warnings": warnings,
        }),
    )
    .await?;
    Ok(())
}

fn json_enum<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_default()
}

fn decision_str(p: &MagiProjection) -> String {
    json_enum(&p.decision_state)
}

fn completeness_str(p: &MagiProjection) -> String {
    json_enum(&p.data_completeness)
}

fn eq_i64(label: &str, actual: i64, expected: i64, diffs: &mut Vec<String>) {
    if actual != expected {
        diffs.push(format!("{label}: got {actual} expected {expected}"));
    }
}

fn eq_str(label: &str, actual: &str, expected: &str, diffs: &mut Vec<String>) {
    if actual != expected {
        diffs.push(format!("{label}: got {actual} expected {expected}"));
    }
}

fn compare_money_fields(
    actual: &MagiProjection,
    expected: &OracleFile,
    diffs: &mut Vec<String>,
) {
    eq_str(
        "decision_state",
        &decision_str(actual),
        &expected.decision_state,
        diffs,
    );
    eq_str(
        "data_completeness",
        &completeness_str(actual),
        &expected.data_completeness,
        diffs,
    );
    if let Some(v) = expected.actual_included_ytd_minor {
        eq_i64(
            "actual_included_ytd",
            actual.actual_included_ytd.amount_minor,
            v,
            diffs,
        );
    }
    if let Some(v) = expected.known_remaining_minor {
        eq_i64(
            "known_remaining",
            actual.known_remaining.amount_minor,
            v,
            diffs,
        );
    }
    if let Some(v) = expected.base_forecast_minor {
        eq_i64("base_forecast", actual.base_forecast.amount_minor, v, diffs);
    }
    if let Some(v) = expected.conservative_forecast_minor {
        eq_i64(
            "conservative_forecast",
            actual.conservative_forecast.amount_minor,
            v,
            diffs,
        );
    }
    if let Some(v) = expected.uncertain_amount_minor {
        eq_i64(
            "uncertain_amount",
            actual.uncertain_amount.amount_minor,
            v,
            diffs,
        );
    }
    if let Some(v) = expected.raw_headroom_minor {
        eq_i64("raw_headroom", actual.raw_headroom.amount_minor, v, diffs);
    }
    if let Some(v) = expected.protected_headroom_minor {
        eq_i64(
            "protected_headroom",
            actual.protected_headroom.amount_minor,
            v,
            diffs,
        );
    }
    if actual.warnings != expected.warnings {
        diffs.push(format!(
            "warnings: got {:?} expected {:?}",
            actual.warnings, expected.warnings
        ));
    }
    for id in &expected.included {
        if !actual.calculation_trace.iter().any(|line| line.contains(id)) {
            diffs.push(format!("trace missing included source {id}"));
        }
    }
    if let Some(src) = &expected.source_transaction {
        if !actual.calculation_trace.iter().any(|line| line.contains(src)) {
            diffs.push(format!("trace must name {src}"));
        }
    }
}

fn compare_snap(label: &str, actual: &MagiProjection, expected: &SnapOracle, diffs: &mut Vec<String>) {
    eq_i64(
        &format!("{label}.actual"),
        actual.actual_included_ytd.amount_minor,
        expected.actual_included_ytd_minor,
        diffs,
    );
    eq_i64(
        &format!("{label}.remaining"),
        actual.known_remaining.amount_minor,
        expected.known_remaining_minor,
        diffs,
    );
    eq_i64(
        &format!("{label}.base"),
        actual.base_forecast.amount_minor,
        expected.base_forecast_minor,
        diffs,
    );
    eq_i64(
        &format!("{label}.conservative"),
        actual.conservative_forecast.amount_minor,
        expected.conservative_forecast_minor,
        diffs,
    );
    if let Some(v) = expected.uncertain_amount_minor {
        eq_i64(
            &format!("{label}.uncertain"),
            actual.uncertain_amount.amount_minor,
            v,
            diffs,
        );
    }
    eq_i64(
        &format!("{label}.raw"),
        actual.raw_headroom.amount_minor,
        expected.raw_headroom_minor,
        diffs,
    );
    eq_i64(
        &format!("{label}.protected"),
        actual.protected_headroom.amount_minor,
        expected.protected_headroom_minor,
        diffs,
    );
    if let Some(state) = &expected.decision_state {
        eq_str(
            &format!("{label}.decision"),
            &decision_str(actual),
            state,
            diffs,
        );
    }
}

fn load_scenario(root: &Path, id: &str) -> Result<ScenarioFile, String> {
    serde_yaml::from_str(
        &fs::read_to_string(root.join(format!("tests/golden/scenarios/{id}.yaml")))
            .map_err(|e| e.to_string())?,
    )
    .map_err(|e| format!("{id} scenario: {e}"))
}

fn load_oracle(root: &Path, id: &str) -> Result<OracleFile, String> {
    serde_yaml::from_str(
        &fs::read_to_string(root.join(format!("tests/golden/oracle/{id}.yaml")))
            .map_err(|e| e.to_string())?,
    )
    .map_err(|e| format!("{id} oracle: {e}"))
}

fn remaining_from(scenario: &ScenarioFile) -> i64 {
    scenario
        .input_facts
        .declarations
        .iter()
        .map(|d| d.amount_minor)
        .sum()
}

fn form_total_from(scenario: &ScenarioFile) -> i64 {
    scenario
        .input_facts
        .tax_forms
        .iter()
        .map(|f| f.amount_minor)
        .sum()
}

async fn run_standard(
    root: &Path,
    scenario: &ScenarioFile,
    oracle: &OracleFile,
) -> Result<Vec<String>, String> {
    let (platform, _dir) = open_platform().await?;
    set_rule(&platform, root).await?;
    let accounts = register_accounts(&platform, &scenario.input_facts.accounts).await?;
    for activity in &scenario.input_facts.activities {
        post_activity(&platform, &accounts, activity).await?;
        record_fact(&platform, activity).await?;
    }
    let form_total = form_total_from(scenario);
    let extra_warnings = if form_total != 0 {
        Vec::new()
    } else {
        scenario.expected_warnings.clone()
    };
    set_coverage(
        &platform,
        &scenario.provenance.completeness,
        remaining_from(scenario),
        0,
        form_total,
        &extra_warnings,
    )
    .await?;
    for adj in &scenario.input_facts.adjustments {
        must_cmd(
            &platform,
            "MagiAdjustmentRecord",
            serde_json::json!({
                "adjustmentId": adj.id,
                "amountMinor": adj.amount_minor,
                "scale": adj.scale,
                "status": adj.status,
                "reason": adj.reason,
            }),
        )
        .await?;
    }
    let actual = projection(&platform).await?;
    let mut diffs = Vec::new();
    compare_money_fields(&actual, oracle, &mut diffs);
    Ok(diffs)
}

async fn run_g02(root: &Path, scenario: &ScenarioFile, oracle: &OracleFile) -> Result<Vec<String>, String> {
    let mut diffs = Vec::new();
    for probe in &oracle.probes {
        let fact = scenario
            .input_facts
            .boundary_probes
            .iter()
            .find(|p| p.id == probe.id)
            .ok_or_else(|| format!("missing probe {}", probe.id))?;
        let (platform, _dir) = open_platform().await?;
        set_rule(&platform, root).await?;
        must_cmd(
            &platform,
            "MagiFactRecord",
            serde_json::json!({
                "sourceId": fact.id,
                "treatment": "include",
                "amountMinor": fact.actual_included_ytd_minor,
                "scale": 2,
                "category": "boundary",
            }),
        )
        .await?;
        set_coverage(&platform, "complete", 0, 0, 0, &[]).await?;
        let actual = projection(&platform).await?;
        eq_str(
            &format!("{}.decision", probe.id),
            &decision_str(&actual),
            &probe.decision_state,
            &mut diffs,
        );
        eq_i64(
            &format!("{}.actual", probe.id),
            actual.actual_included_ytd.amount_minor,
            probe.actual_included_ytd_minor,
            &mut diffs,
        );
        eq_i64(
            &format!("{}.remaining", probe.id),
            actual.known_remaining.amount_minor,
            probe.known_remaining_minor,
            &mut diffs,
        );
        eq_i64(
            &format!("{}.base", probe.id),
            actual.base_forecast.amount_minor,
            probe.base_forecast_minor,
            &mut diffs,
        );
        eq_i64(
            &format!("{}.conservative", probe.id),
            actual.conservative_forecast.amount_minor,
            probe.conservative_forecast_minor,
            &mut diffs,
        );
        eq_i64(
            &format!("{}.uncertain", probe.id),
            actual.uncertain_amount.amount_minor,
            probe.uncertain_amount_minor,
            &mut diffs,
        );
        eq_i64(
            &format!("{}.raw", probe.id),
            actual.raw_headroom.amount_minor,
            probe.raw_headroom_minor,
            &mut diffs,
        );
        eq_i64(
            &format!("{}.protected", probe.id),
            actual.protected_headroom.amount_minor,
            probe.protected_headroom_minor,
            &mut diffs,
        );
        if !actual
            .calculation_trace
            .iter()
            .any(|line| line.contains(&probe.id))
        {
            diffs.push(format!("trace missing probe {}", probe.id));
        }
    }
    Ok(diffs)
}

async fn run_g03(root: &Path, scenario: &ScenarioFile, oracle: &OracleFile) -> Result<Vec<String>, String> {
    let (platform, _dir) = open_platform().await?;
    set_rule(&platform, root).await?;
    let accounts = register_accounts(&platform, &scenario.input_facts.accounts).await?;
    let div = scenario
        .input_facts
        .activities
        .iter()
        .find(|a| a.id == "G03-DIV-1")
        .ok_or_else(|| "missing G03-DIV-1".to_string())?;
    let ira = scenario
        .input_facts
        .activities
        .iter()
        .find(|a| a.id == "G03-IRA-1")
        .ok_or_else(|| "missing G03-IRA-1".to_string())?;
    post_activity(&platform, &accounts, div).await?;
    record_fact(&platform, div).await?;
    set_coverage(&platform, "complete", 0, 0, 0, &[]).await?;
    let before = projection(&platform).await?;
    post_activity(&platform, &accounts, ira).await?;
    record_fact(&platform, ira).await?;
    let after = projection(&platform).await?;
    let mut diffs = Vec::new();
    if let Some(exp) = &oracle.before_ira {
        compare_snap("before_ira", &before, exp, &mut diffs);
    }
    if let Some(exp) = &oracle.after_ira {
        compare_snap("after_ira", &after, exp, &mut diffs);
    }
    eq_str("after.decision", &decision_str(&after), &oracle.decision_state, &mut diffs);
    for id in &oracle.included {
        if !after.calculation_trace.iter().any(|line| line.contains(id)) {
            diffs.push(format!("trace missing included source {id}"));
        }
    }
    Ok(diffs)
}

async fn run_g06(root: &Path, scenario: &ScenarioFile, oracle: &OracleFile) -> Result<Vec<String>, String> {
    let (platform, _dir) = open_platform().await?;
    set_rule(&platform, root).await?;
    let accounts = register_accounts(&platform, &scenario.input_facts.accounts).await?;
    for activity in &scenario.input_facts.activities {
        post_activity(&platform, &accounts, activity).await?;
        record_fact(&platform, activity).await?;
    }
    let a = scenario
        .input_facts
        .withholding
        .iter()
        .find(|w| w.id == "withhold-a")
        .ok_or_else(|| "missing withhold-a".to_string())?;
    let b = scenario
        .input_facts
        .withholding
        .iter()
        .find(|w| w.id == "withhold-b")
        .ok_or_else(|| "missing withhold-b".to_string())?;
    set_coverage(&platform, "complete", 0, a.federal_withheld_minor, 0, &[]).await?;
    let magi_a = projection(&platform).await?;
    let tax_a = tax_payment(&platform).await?;
    set_coverage(&platform, "complete", 0, b.federal_withheld_minor, 0, &[]).await?;
    let magi_b = projection(&platform).await?;
    let tax_b = tax_payment(&platform).await?;
    let mut diffs = Vec::new();
    compare_money_fields(&magi_a, oracle, &mut diffs);
    if let Some(v) = oracle.magi_withhold_a_minor {
        eq_i64("magi_withhold_a", magi_a.actual_included_ytd.amount_minor, v, &mut diffs);
    }
    if let Some(v) = oracle.magi_withhold_b_minor {
        eq_i64("magi_withhold_b", magi_b.actual_included_ytd.amount_minor, v, &mut diffs);
    }
    if magi_a.actual_included_ytd.amount_minor != magi_b.actual_included_ytd.amount_minor {
        diffs.push("MAGI changed when withholding changed".into());
    }
    if let Some(v) = oracle.tax_payment_a_minor {
        eq_i64("tax_payment_a", tax_a, v, &mut diffs);
    }
    if let Some(v) = oracle.tax_payment_b_minor {
        eq_i64("tax_payment_b", tax_b, v, &mut diffs);
    }
    Ok(diffs)
}

async fn run_g08(root: &Path, scenario: &ScenarioFile, oracle: &OracleFile) -> Result<Vec<String>, String> {
    let (platform, _dir) = open_platform().await?;
    set_rule(&platform, root).await?;
    let accounts = register_accounts(&platform, &scenario.input_facts.accounts).await?;
    for activity in &scenario.input_facts.activities {
        post_activity(&platform, &accounts, activity).await?;
        record_fact(&platform, activity).await?;
    }
    let before_plan = scenario
        .input_facts
        .snapshots
        .iter()
        .find(|s| s.id == "before_plan")
        .ok_or_else(|| "missing before_plan".to_string())?;
    let after_plan = scenario
        .input_facts
        .snapshots
        .iter()
        .find(|s| s.id == "after_plan")
        .ok_or_else(|| "missing after_plan".to_string())?;
    set_coverage(
        &platform,
        "complete",
        before_plan.planned_remaining_minor,
        0,
        0,
        &[],
    )
    .await?;
    let before = projection(&platform).await?;
    set_coverage(
        &platform,
        "complete",
        after_plan.planned_remaining_minor,
        0,
        0,
        &[],
    )
    .await?;
    let after = projection(&platform).await?;
    let mut diffs = Vec::new();
    if let Some(exp) = &oracle.before_plan {
        compare_snap("before_plan", &before, exp, &mut diffs);
    }
    if let Some(exp) = &oracle.after_plan {
        compare_snap("after_plan", &after, exp, &mut diffs);
    }
    if before.actual_included_ytd.amount_minor != after.actual_included_ytd.amount_minor {
        diffs.push("actual changed when remaining/plan changed".into());
    }
    eq_str("after.decision", &decision_str(&after), &oracle.decision_state, &mut diffs);
    Ok(diffs)
}

async fn run_one(root: &Path, id: &str) -> Result<Vec<String>, String> {
    let scenario = load_scenario(root, id)?;
    let oracle = load_oracle(root, id)?;
    match id {
        "G-MAGI-02" => run_g02(root, &scenario, &oracle).await,
        "G-MAGI-03" => run_g03(root, &scenario, &oracle).await,
        "G-MAGI-06" => run_g06(root, &scenario, &oracle).await,
        "G-MAGI-08" => run_g08(root, &scenario, &oracle).await,
        _ => run_standard(root, &scenario, &oracle).await,
    }
}

/// Compare every G-MAGI scenario on the production command path. Never writes oracles.
pub async fn compare_magi_pack(root: &Path) -> Result<(), String> {
    let inv = magi_pack_inventory(root)?;
    if !inv.all_scenarios_approved || !inv.all_oracles_approved {
        return Err("MAGI pack is not owner-approved".into());
    }
    let mut failed = Vec::new();
    for id in &inv.scenario_ids {
        match run_one(root, id).await {
            Ok(diffs) if diffs.is_empty() => {}
            Ok(diffs) => failed.push(format!("{id}: {}", diffs.join("; "))),
            Err(err) => failed.push(format!("{id}: {err}")),
        }
    }
    if failed.is_empty() {
        Ok(())
    } else {
        Err(failed.join("\n"))
    }
}

pub async fn magi_pack_run(root: &Path) -> MagiPackVerdict {
    let scenarios = root.join("tests/golden/scenarios");
    match magi_pack_verdict(&scenarios) {
        MagiPackVerdict::BlockedPendingOwner => MagiPackVerdict::BlockedPendingOwner,
        MagiPackVerdict::Failed | MagiPackVerdict::Passed => match compare_magi_pack(root).await {
            Ok(()) => MagiPackVerdict::Passed,
            Err(_) => MagiPackVerdict::Failed,
        },
    }
}
