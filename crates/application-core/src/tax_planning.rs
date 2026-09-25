//! Household Tax Planning: YTD / projected income by tax type.
//! Car ordinary/ROC/gains stay on CarRocPlanGet. Car cash withdrawals are
//! Register facts only — they are not a tax row and must not join All income
//! sources (those dollars are already ROC + ordinary). This rollup does not
//! rewrite MAGI oracles.

use crate::cash_ytd::cash_ytd_get;
use crate::contracts::{CarRocPlanBody, TaxPlanningBody, TaxPlanningGroup, TaxPlanningRow};
use crate::ports::canonical::Canonical;
use crate::ports::platform::PlatformError;
use chrono::Datelike;
use financial_domain::cash_management::{
    is_cash_distribution_type, job_1099_year_amounts, JOB_1099_2026_MINOR, JOB_1099_2026_SOURCE,
};
use financial_domain::trends::parse_iso_date;

fn year_bounds(as_of: &str) -> Result<(String, String), PlatformError> {
    let day = parse_iso_date(as_of)
        .ok_or_else(|| PlatformError::new("bad_date", format!("invalid asOfDate {as_of}")))?;
    let year = day.year();
    Ok((format!("{year}-01-01"), format!("{year}-12-31")))
}

fn add_opt(left: Option<i64>, right: Option<i64>) -> Option<i64> {
    Some(left? + right?)
}

fn sum_opts(values: &[Option<i64>]) -> Option<i64> {
    let mut total = 0i64;
    for v in values {
        total += (*v)?;
    }
    Some(total)
}

fn row_total(ytd: Option<i64>, projected: Option<i64>) -> Option<i64> {
    add_opt(ytd, projected)
}

fn ytd_row(label: &str, rows: &[crate::contracts::CashYtdRow]) -> (i64, Option<i64>) {
    rows.iter()
        .find(|r| r.label == label)
        .map(|r| (r.actual_minor, r.remaining_minor))
        .unwrap_or((0, None))
}

async fn speculation_ira(
    canonical: &dyn Canonical,
    jan1: &str,
    as_of: &str,
    dec31: &str,
) -> Result<(i64, Option<i64>), PlatformError> {
    let accounts = canonical.account_list().await?;
    let Some(acct) = accounts
        .iter()
        .find(|a| a.name.eq_ignore_ascii_case("Speculation"))
        .cloned()
    else {
        return Ok((0, None));
    };
    let activities = canonical
        .activity_list_in_range(Some(acct.account_id), jan1, as_of)
        .await
        .unwrap_or_default();
    let occs = canonical.planned_occurrence_list().await?;
    let mut ytd = 0i64;
    for act in &activities {
        if act.account_id != acct.account_id {
            continue;
        }
        if !is_cash_distribution_type(&act.activity_type) {
            continue;
        }
        ytd += act.amount_minor.abs();
    }
    let mut projected = 0i64;
    let mut projected_known = false;
    for o in &occs {
        if !o.account.eq_ignore_ascii_case("Speculation") {
            continue;
        }
        if o.note.eq_ignore_ascii_case("dividend") {
            continue;
        }
        if o.confirmed_at.is_none()
            && o.occurred_on.as_str() > as_of
            && o.occurred_on.as_str() <= dec31
        {
            projected += o.amount_minor.abs();
            projected_known = true;
        }
    }
    Ok((ytd, if projected_known { Some(projected) } else { None }))
}

fn push_row(
    rows: &mut Vec<TaxPlanningRow>,
    key: &str,
    label: &str,
    ytd: Option<i64>,
    projected: Option<i64>,
    magi_impact: &str,
) {
    rows.push(TaxPlanningRow {
        key: key.into(),
        label: label.into(),
        ytd_minor: ytd,
        projected_minor: projected,
        total_minor: row_total(ytd, projected),
        magi_impact: magi_impact.into(),
    });
}

fn group(label: &str, rows: &[&TaxPlanningRow]) -> TaxPlanningGroup {
    TaxPlanningGroup {
        label: label.into(),
        ytd_minor: sum_opts(&rows.iter().map(|r| r.ytd_minor).collect::<Vec<_>>()),
        projected_minor: sum_opts(&rows.iter().map(|r| r.projected_minor).collect::<Vec<_>>()),
        total_minor: sum_opts(&rows.iter().map(|r| r.total_minor).collect::<Vec<_>>()),
    }
}

pub async fn tax_planning_get(
    canonical: &dyn Canonical,
    as_of: &str,
    car: &CarRocPlanBody,
) -> Result<TaxPlanningBody, PlatformError> {
    let (jan1, dec31) = year_bounds(as_of)?;
    let ytd = cash_ytd_get(canonical, as_of, "account").await?;
    let (income_ytd, income_proj) = ytd_row("Income", &ytd.rows);
    let (nine_ytd, nine_proj) = ytd_row("Account 9", &ytd.rows);
    let (roth_ytd, roth_proj) = ytd_row("FI Roth", &ytd.rows);
    let (hsa_ytd, hsa_proj) = ytd_row("Health", &ytd.rows);
    let (ssa_ytd, ssa_proj) = ytd_row("SSA_2026", &ytd.rows);
    let (spec_ytd, spec_proj) = speculation_ira(canonical, &jan1, as_of, &dec31).await?;
    let (job_ytd, job_proj) = job_1099_year_amounts(as_of);
    if job_ytd != 0 {
        let _ = canonical
            .magi_fact_record(
                JOB_1099_2026_SOURCE.into(),
                "include".into(),
                JOB_1099_2026_MINOR,
                2,
                "1099-job".into(),
            )
            .await;
    }
    let ira_ytd = income_ytd + nine_ytd + spec_ytd;
    let ira_proj = match (income_proj, nine_proj, spec_proj) {
        (Some(a), Some(b), Some(c)) => Some(a + b + c),
        (Some(a), Some(b), None) => Some(a + b),
        (Some(a), None, Some(c)) => Some(a + c),
        (None, Some(b), Some(c)) => Some(b + c),
        (Some(a), None, None) => Some(a),
        (None, Some(b), None) => Some(b),
        (None, None, Some(c)) => Some(c),
        (None, None, None) => None,
    };

    let mut rows = Vec::new();
    push_row(
        &mut rows,
        "ira",
        "IRA withdrawals",
        Some(ira_ytd),
        ira_proj,
        "magi",
    );
    push_row(
        &mut rows,
        "roth",
        "Roth withdrawals",
        Some(roth_ytd),
        roth_proj,
        "none",
    );
    push_row(
        &mut rows,
        "hsa",
        "HSA withdrawals",
        Some(hsa_ytd),
        hsa_proj,
        "none",
    );
    push_row(
        &mut rows,
        "roc",
        "ROC",
        car.ytd_roc_minor,
        car.remaining_roc_minor,
        "none",
    );
    push_row(
        &mut rows,
        "ordinary",
        "Ordinary income",
        car.ytd_ordinary_minor,
        car.remaining_ordinary_minor,
        "magi",
    );
    push_row(
        &mut rows,
        "job1099",
        "1099 job",
        Some(job_ytd),
        Some(job_proj),
        "magi",
    );
    push_row(
        &mut rows,
        "ssa",
        "SSA",
        Some(ssa_ytd),
        ssa_proj,
        "magi",
    );
    push_row(
        &mut rows,
        "ltcg",
        "Long Term Capital Gains",
        car.ytd_long_term_gain_minor.or(Some(0)),
        Some(0),
        "magi_ltcg",
    );
    push_row(
        &mut rows,
        "stcg",
        "Short Term Capital Gains",
        car.ytd_short_term_gain_minor.or(Some(0)),
        Some(0),
        "magi",
    );

    let magi_rows: Vec<&TaxPlanningRow> = rows
        .iter()
        .filter(|r| r.magi_impact == "magi" || r.magi_impact == "magi_ltcg")
        .collect();
    let not_magi_rows: Vec<&TaxPlanningRow> = rows.iter().filter(|r| r.magi_impact == "none").collect();
    let all_rows: Vec<&TaxPlanningRow> = rows.iter().collect();

    Ok(TaxPlanningBody {
        as_of_date: as_of.to_string(),
        magi_included: group("MAGI-included", &magi_rows),
        not_magi: group("No MAGI impact", &not_magi_rows),
        all_sources: group("All income sources", &all_rows),
        rows,
        scale: 2,
        car: Some(car.clone()),
        ytd: Some(ytd),
    })
}
