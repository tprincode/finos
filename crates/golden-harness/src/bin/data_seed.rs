//! One-time data load into the Profile A SQLite file. Not an owner screen.

use std::process::ExitCode;

use application_core::contracts::{
    QueryRequest, FINANCE_CLIENT_CONTRACT_VERSION,
};
use application_core::queries::execute_query_on;
use golden_harness::{
    load_production_expected, load_production_seed_via_commands, production_seed_actual_counts,
    production_seed_actual_totals, production_seed_plan_count, profile_a_app_dir, repo_root,
};
use storage_sqlite::LocalPlatform;
use uuid::Uuid;

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(msg) => {
            println!("{msg}");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("data-seed failed: {err}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<String, String> {
    let root = repo_root();
    let production = root.join("database/seed/production");
    if !production.join("Template_Accounts.xlsx").is_file() {
        return Err(format!(
            "templates missing at {}",
            production.display()
        ));
    }
    let expected = load_production_expected(&production.join("expected-production.yaml"))?;
    let app_dir = profile_a_app_dir();
    std::fs::create_dir_all(&app_dir).map_err(|e| e.to_string())?;
    let platform = LocalPlatform::open(&app_dir)
        .await
        .map_err(|e| e.to_string())?;
    let before = production_seed_actual_counts(&platform).await?;
    let plans_before = production_seed_plan_count(&platform).await?;
    load_production_seed_via_commands(&platform, &production).await?;
    let after = production_seed_actual_counts(&platform).await?;
    let trends = execute_query_on(
        &platform,
        &platform,
        QueryRequest {
            contract_version: FINANCE_CLIENT_CONTRACT_VERSION.to_string(),
            query_name: "TrendsGet".into(),
            correlation_id: Uuid::new_v4(),
            body_json: None,
        },
    )
    .await;
    let trends_weeks = if trends.ok {
        let val: serde_json::Value =
            serde_json::from_str(trends.body_json.as_deref().unwrap_or("{}")).unwrap_or_default();
        val["weeks"].as_array().map(|a| a.len()).unwrap_or(0)
    } else {
        0
    };
    let existing_profile =
        before.accounts > 0 && before.lots_total >= expected.counts.lots_total;
    if after != expected.counts {
        // Collectors / owner activity can add yield rows after the locked template gate.
        if existing_profile {
            eprintln!(
                "warning: counts {:?} != expected {:?} (existing Profile A DB; Trends weeks {trends_weeks})",
                after, expected.counts
            );
        } else {
            return Err(format!(
                "counts {:?} != expected {:?}",
                after, expected.counts
            ));
        }
    }
    let totals = production_seed_actual_totals(&platform).await?;
    if totals.yield_amount_minor != expected.totals.yield_amount_minor
        || totals.disbursement_gross_minor != expected.totals.disbursement_gross_minor
    {
        if existing_profile {
            eprintln!(
                "warning: money totals {:?} != expected yield {} gross {} (existing Profile A DB)",
                totals,
                expected.totals.yield_amount_minor,
                expected.totals.disbursement_gross_minor
            );
        } else {
            return Err(format!(
                "money totals {:?} != expected yield {} gross {}",
                totals,
                expected.totals.yield_amount_minor,
                expected.totals.disbursement_gross_minor
            ));
        }
    }
    let coverage = execute_query_on(
        &platform,
        &platform,
        QueryRequest {
            contract_version: FINANCE_CLIENT_CONTRACT_VERSION.to_string(),
            query_name: "PositionDetailsCoverageGet".into(),
            correlation_id: Uuid::new_v4(),
            body_json: None,
        },
    )
    .await;
    let coverage_note = if coverage.ok {
        let val: serde_json::Value =
            serde_json::from_str(coverage.body_json.as_deref().unwrap_or("{}")).unwrap_or_default();
        let n = val["rows"].as_array().map(|a| a.len()).unwrap_or(0);
        format!("; coverage {n} securities")
    } else {
        String::new()
    };
    if before == expected.counts && plans_before >= 40 {
        return Ok(format!(
            "data already loaded at {} (calculator/trends refreshed; Trends weeks {trends_weeks}{coverage_note})",
            platform.db_path().display()
        ));
    }
    Ok(format!(
        "data loaded at {} (8/75/1679/1250/5862/129; Trends weeks {trends_weeks}{coverage_note})",
        platform.db_path().display()
    ))
}
