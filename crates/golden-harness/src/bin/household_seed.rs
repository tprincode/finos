//! One-time household load into the Profile A SQLite file. Not an owner screen.

use std::process::ExitCode;

use golden_harness::{
    load_production_expected, load_production_seed_via_commands, production_seed_actual_counts,
    production_seed_actual_totals, production_seed_plan_count, profile_a_app_dir, repo_root,
};
use storage_sqlite::LocalPlatform;

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(msg) => {
            println!("{msg}");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("household-seed failed: {err}");
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
    if before == expected.counts && plans_before >= 40 {
        return Ok(format!(
            "household already loaded at {} (no re-seed)",
            platform.db_path().display()
        ));
    }
    load_production_seed_via_commands(&platform, &production).await?;
    let after = production_seed_actual_counts(&platform).await?;
    if after != expected.counts {
        return Err(format!(
            "counts {:?} != expected {:?}",
            after, expected.counts
        ));
    }
    let totals = production_seed_actual_totals(&platform).await?;
    if totals.yield_amount_minor != expected.totals.yield_amount_minor
        || totals.disbursement_gross_minor != expected.totals.disbursement_gross_minor
    {
        return Err(format!(
            "money totals {:?} != expected yield {} gross {}",
            totals,
            expected.totals.yield_amount_minor,
            expected.totals.disbursement_gross_minor
        ));
    }
    Ok(format!(
        "household loaded at {} (8/74/1679/5862/129)",
        platform.db_path().display()
    ))
}
