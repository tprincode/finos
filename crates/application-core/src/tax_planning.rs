//! Household Tax Planning: YTD / projected income by tax type.
//! Car ordinary/ROC/gains stay on CarRocPlanGet. Car cash withdrawals are
//! Register facts only — they are not a tax row and must not join All income
//! sources (those dollars are already ROC + ordinary). This rollup does not
//! rewrite MAGI oracles.

use crate::cash_ytd::cash_ytd_get;
use crate::contracts::{
    CarRocPlanBody, TaxPlanningBody, TaxPlanningGroup, TaxPlanningRow, TaxWithholdingRow,
};
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
        let on = o.occurred_on.get(..10).unwrap_or(o.occurred_on.as_str());
        let already_posted = activities.iter().any(|act| {
            act.account_id == acct.account_id
                && is_cash_distribution_type(&act.activity_type)
                && act.occurred_on.get(..10).unwrap_or(act.occurred_on.as_str()) == on
        });
        if o.confirmed_at.is_none() && on >= as_of && on <= dec31 && !already_posted {
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

fn withholding_key(note: &str) -> Option<&'static str> {
    match note.trim().to_ascii_lowercase().as_str() {
        "fed" | "federal" => Some("fed"),
        "state" => Some("state"),
        _ => None,
    }
}

/// Posted Income withholding is YTD. Unconfirmed Income `fed` / `state` Elements
/// through 31 Dec are the remaining plan. Confirmed Element Saturdays count as collected.
async fn withholding_rows(
    canonical: &dyn Canonical,
    as_of: &str,
    jan1: &str,
    dec31: &str,
) -> Result<Vec<TaxWithholdingRow>, PlatformError> {
    if as_of <= dec31 {
        crate::week_ahead::ensure_horizon(canonical, as_of, dec31).await?;
    }
    let accounts = canonical.account_list().await.unwrap_or_default();
    let income_id = accounts
        .iter()
        .find(|a| a.name.eq_ignore_ascii_case("Income"))
        .map(|a| a.account_id);
    let elements = canonical.cash_element_list().await.unwrap_or_default();
    let occs = canonical.planned_occurrence_list().await.unwrap_or_default();
    let posted = if let Some(account_id) = income_id {
        canonical
            .activity_list_in_range(Some(account_id), jan1, as_of)
            .await
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let mut ytd = std::collections::HashMap::from([("fed", 0i64), ("state", 0i64)]);
    let mut remaining = std::collections::HashMap::from([("fed", 0i64), ("state", 0i64)]);
    for act in &posted {
        for key in ["fed", "state"] {
            if let Some(amount) = financial_domain::cash_management::income_element_activity_slice(
                key,
                &act.activity_type,
                act.amount_minor,
                act.federal_withholding_minor,
                act.state_withholding_minor,
                &act.idempotency_key,
            ) {
                *ytd.entry(key).or_insert(0) += amount;
            }
        }
    }
    for element in &elements {
        if !element.account.eq_ignore_ascii_case("Income") {
            continue;
        }
        let Some(key) = withholding_key(&element.note) else {
            continue;
        };
        for occ in occs.iter().filter(|o| o.element_id == element.element_id) {
            if occ.is_cancelled {
                continue;
            }
            let on = occ.occurred_on.as_str();
            if occ.confirmed_at.is_some() {
                if on >= jan1 && on <= as_of {
                    *ytd.entry(key).or_insert(0) += occ.amount_minor.abs();
                }
            } else if on >= as_of && on <= dec31 {
                *remaining.entry(key).or_insert(0) += occ.amount_minor.abs();
            }
        }
    }
    Ok(vec![
        TaxWithholdingRow {
            key: "fed".into(),
            label: "Fed tax".into(),
            ytd_minor: ytd["fed"],
            remaining_minor: remaining["fed"],
        },
        TaxWithholdingRow {
            key: "state".into(),
            label: "State tax".into(),
            ytd_minor: ytd["state"],
            remaining_minor: remaining["state"],
        },
    ])
}

fn group(label: &str, rows: &[&TaxPlanningRow]) -> TaxPlanningGroup {
    TaxPlanningGroup {
        label: label.into(),
        ytd_minor: sum_opts(&rows.iter().map(|r| r.ytd_minor).collect::<Vec<_>>()),
        projected_minor: sum_opts(&rows.iter().map(|r| r.projected_minor).collect::<Vec<_>>()),
        total_minor: sum_opts(&rows.iter().map(|r| r.total_minor).collect::<Vec<_>>()),
    }
}

fn is_capital_gain_key(key: &str) -> bool {
    key == "ltcg" || key == "stcg"
}

/// MAGI takes a capital gain whole, but a net capital loss only to the 1040 limit.
/// The rows keep the uncapped net; the cap lands here, once per column.
fn magi_column(
    rows: &[&TaxPlanningRow],
    pick: impl Fn(&TaxPlanningRow) -> Option<i64>,
) -> Option<i64> {
    let mut plain = 0i64;
    let mut gains: Vec<i64> = Vec::new();
    for row in rows {
        let value = pick(row)?;
        if is_capital_gain_key(&row.key) {
            gains.push(value);
        } else {
            plain += value;
        }
    }
    Some(plain + financial_domain::roc::net_capital_gain_for_magi(&gains).magi_minor)
}

fn magi_group(label: &str, rows: &[&TaxPlanningRow]) -> TaxPlanningGroup {
    TaxPlanningGroup {
        label: label.into(),
        ytd_minor: magi_column(rows, |r| r.ytd_minor),
        projected_minor: magi_column(rows, |r| r.projected_minor),
        total_minor: magi_column(rows, |r| r.total_minor),
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
    // No withdrawal element means nothing left to withdraw. A missing plan is
    // zero, not the dividend forecast.
    let ira_proj = Some(income_proj.unwrap_or(0) + nine_proj.unwrap_or(0) + spec_proj.unwrap_or(0));
    let roth_proj = Some(roth_proj.unwrap_or(0));
    let hsa_proj = Some(hsa_proj.unwrap_or(0));

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
    // No lot sale is a decided $0 — this section reports status, and nothing sold
    // is nothing gained. A sale we could not place in a term stays unknown.
    let gain_ytd = |stored: Option<i64>| {
        if car.lot_sale_count == 0 {
            stored.or(Some(0))
        } else {
            stored
        }
    };
    let long_ytd = gain_ytd(car.ytd_long_term_gain_minor);
    let short_ytd = gain_ytd(car.ytd_short_term_gain_minor);
    // Nothing plans a future lot sale, so a known YTD means $0 still to come.
    push_row(
        &mut rows,
        "ltcg",
        "Long Term Capital Gains",
        long_ytd,
        long_ytd.map(|_| 0),
        "magi_ltcg",
    );
    push_row(
        &mut rows,
        "stcg",
        "Short Term Capital Gains",
        short_ytd,
        short_ytd.map(|_| 0),
        "magi",
    );

    let withholding = withholding_rows(canonical, as_of, &jan1, &dec31).await?;
    let ira_contribution_minor = ira_contribution_year_minor(canonical, as_of).await?;
    let magi_rows: Vec<&TaxPlanningRow> = rows
        .iter()
        .filter(|r| r.magi_impact == "magi" || r.magi_impact == "magi_ltcg")
        .collect();
    let not_magi_rows: Vec<&TaxPlanningRow> = rows.iter().filter(|r| r.magi_impact == "none").collect();
    let all_rows: Vec<&TaxPlanningRow> = rows.iter().collect();
    let net_gain = rows
        .iter()
        .filter(|r| is_capital_gain_key(&r.key))
        .map(|r| r.total_minor)
        .collect::<Option<Vec<i64>>>()
        .map(|parts| financial_domain::roc::net_capital_gain_for_magi(&parts));

    Ok(TaxPlanningBody {
        as_of_date: as_of.to_string(),
        magi_included: magi_group("MAGI-included", &magi_rows),
        not_magi: group("No MAGI impact", &not_magi_rows),
        all_sources: group("All income sources", &all_rows),
        rows,
        scale: 2,
        withholding,
        car: Some(car.clone()),
        ytd: Some(ytd),
        ira_contribution_minor,
        net_capital_gain_minor: net_gain.map(|n| n.net_minor),
        capital_gain_magi_minor: net_gain.map(|n| n.magi_minor),
        capital_loss_carryforward_minor: net_gain.map(|n| n.carryforward_minor),
    })
}

async fn ira_contribution_year_minor(
    canonical: &dyn Canonical,
    as_of: &str,
) -> Result<i64, PlatformError> {
    let year = as_of.get(..4).unwrap_or("");
    if year.len() != 4 {
        return Ok(0);
    }
    let prefix = format!("{year}-");
    let activities = canonical.activity_list().await?;
    Ok(activities
        .iter()
        .filter(|row| {
            row.activity_type.eq_ignore_ascii_case("ira-contribution")
                && row.occurred_on.starts_with(&prefix)
        })
        .map(|row| row.amount_minor)
        .sum())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(key: &str, impact: &str, total: Option<i64>) -> TaxPlanningRow {
        TaxPlanningRow {
            key: key.into(),
            label: key.into(),
            ytd_minor: total,
            projected_minor: total.map(|_| 0),
            total_minor: total,
            magi_impact: impact.into(),
        }
    }

    #[test]
    fn magi_group_takes_the_capped_slice_not_the_whole_loss() {
        // $20,000.00 of ordinary income against a $10,000.00 long-term loss.
        let rows = vec![
            row("ira", "magi", Some(2_000_000)),
            row("ltcg", "magi_ltcg", Some(-1_000_000)),
            row("stcg", "magi", Some(0)),
        ];
        let refs: Vec<&TaxPlanningRow> = rows.iter().collect();
        let group = magi_group("MAGI-included", &refs);
        assert_eq!(group.total_minor, Some(2_000_000 - 300_000));
        assert_ne!(group.total_minor, Some(2_000_000 - 1_000_000));
        assert_eq!(group.ytd_minor, Some(1_700_000));
        assert_eq!(group.projected_minor, Some(0));
    }

    #[test]
    fn a_gain_reaches_the_magi_group_whole() {
        let rows = vec![
            row("ira", "magi", Some(2_000_000)),
            row("ltcg", "magi_ltcg", Some(500_000)),
            row("stcg", "magi", Some(0)),
        ];
        let refs: Vec<&TaxPlanningRow> = rows.iter().collect();
        assert_eq!(
            magi_group("MAGI-included", &refs).total_minor,
            Some(2_500_000)
        );
    }

    #[test]
    fn one_unknown_row_keeps_the_magi_group_unknown() {
        let rows = vec![
            row("ira", "magi", Some(2_000_000)),
            row("ordinary", "magi", None),
            row("ltcg", "magi_ltcg", Some(-1_000_000)),
            row("stcg", "magi", Some(0)),
        ];
        let refs: Vec<&TaxPlanningRow> = rows.iter().collect();
        assert_eq!(magi_group("MAGI-included", &refs).total_minor, None);
    }

    #[test]
    fn all_income_sources_keeps_the_uncapped_loss() {
        let rows = vec![
            row("ira", "magi", Some(2_000_000)),
            row("ltcg", "magi_ltcg", Some(-1_000_000)),
            row("stcg", "magi", Some(0)),
        ];
        let refs: Vec<&TaxPlanningRow> = rows.iter().collect();
        assert_eq!(
            group("All income sources", &refs).total_minor,
            Some(1_000_000)
        );
    }
}
