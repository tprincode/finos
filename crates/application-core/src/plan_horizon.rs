//! June include-next-year holes. Vendor issuer dates are not invented here.

use uuid::Uuid;

use crate::contracts::{
    AssumedPayDateRecord, PlanHorizonGetBody, PlanHorizonNameBody,
};
use crate::ports::canonical::Canonical;
use crate::ports::platform::PlatformError;

fn cadence_label(frequency: &str, periods: u8) -> String {
    if !frequency.trim().is_empty() {
        return frequency.to_string();
    }
    match periods {
        52 => "Weekly".into(),
        12 => "Monthly".into(),
        4 => "Quarterly".into(),
        _ => String::new(),
    }
}

fn current_plan_periods(
    plans: &[crate::contracts::PlanHistoryRecord],
    security_id: Uuid,
) -> Option<u8> {
    plans
        .iter()
        .find(|p| p.security_id == security_id && p.effective_to.trim().is_empty())
        .map(|p| p.planning_periods_per_year)
}

async fn holes_for_security(
    canonical: &dyn Canonical,
    security_id: Uuid,
    as_of: &str,
    cadence: &str,
    horizon_start: &str,
    horizon_end: &str,
) -> Result<Vec<String>, PlatformError> {
    let decls = canonical
        .issuer_declaration_list(security_id)
        .await
        .unwrap_or_default();
    let paid: Vec<String> = decls
        .iter()
        .filter(|d| d.amount_per_share_minor.unwrap_or(0) > 0)
        .map(|d| d.payment_period.clone())
        .collect();
    let paid_refs: Vec<&str> = paid.iter().map(String::as_str).collect();
    let derived = financial_domain::schedule::derive_horizon_pay_ons(
        as_of,
        cadence,
        &paid_refs,
        horizon_start,
        horizon_end,
    );
    if derived.is_empty() {
        return Ok(Vec::new());
    }
    let issuer = canonical
        .issuer_pay_date_list(security_id)
        .await
        .unwrap_or_default();
    Ok(derived
        .into_iter()
        .filter(|pay_on| {
            !issuer.iter().any(|row| {
                row.pay_on == *pay_on
                    || financial_domain::schedule::vendor_payables_same_period(
                        cadence, &row.pay_on, pay_on,
                    )
            })
        })
        .collect())
}

pub async fn plan_horizon_preview(
    canonical: &dyn Canonical,
    as_of: &str,
) -> Result<PlanHorizonGetBody, PlatformError> {
    let Some(assume_year) = financial_domain::schedule::assume_calendar_year(as_of) else {
        return Err(PlatformError::new("invalid_as_of", "as-of is not a date"));
    };
    let Some((horizon_start, horizon_end)) = financial_domain::schedule::assume_year_bounds(as_of)
    else {
        return Err(PlatformError::new("invalid_as_of", "as-of is not a date"));
    };
    let securities = canonical.security_list().await.unwrap_or_default();
    let plans = canonical.plan_history_list().await.unwrap_or_default();
    let chars = canonical
        .position_characteristic_list()
        .await
        .unwrap_or_default();
    let mut names = Vec::new();
    let mut hole_count = 0u32;
    for sec in securities {
        let Some(periods) = current_plan_periods(&plans, sec.security_id) else {
            continue;
        };
        let freq = chars
            .iter()
            .find(|c| c.security_id == sec.security_id)
            .map(|c| c.payment_frequency.as_str())
            .unwrap_or("");
        let cadence = cadence_label(freq, periods);
        if cadence.is_empty() {
            continue;
        }
        let holes = holes_for_security(
            canonical,
            sec.security_id,
            as_of,
            &cadence,
            &horizon_start,
            &horizon_end,
        )
        .await?;
        if holes.is_empty() {
            continue;
        }
        hole_count = hole_count.saturating_add(holes.len() as u32);
        names.push(PlanHorizonNameBody {
            security_id: sec.security_id,
            symbol: sec.symbol,
            holes,
        });
    }
    names.sort_by(|a, b| a.symbol.cmp(&b.symbol));
    Ok(PlanHorizonGetBody {
        assume_year,
        hole_count,
        names,
    })
}

pub async fn plan_horizon_assume(
    canonical: &dyn Canonical,
    as_of: &str,
) -> Result<PlanHorizonGetBody, PlatformError> {
    let preview = plan_horizon_preview(canonical, as_of).await?;
    for name in &preview.names {
        let cadence = {
            let chars = canonical
                .position_characteristic_list()
                .await
                .unwrap_or_default();
            chars
                .iter()
                .find(|c| c.security_id == name.security_id)
                .map(|c| c.payment_frequency.clone())
                .unwrap_or_else(|| "Monthly".into())
        };
        for pay_on in &name.holes {
            let rec = AssumedPayDateRecord {
                assumed_pay_date_id: Uuid::new_v4(),
                security_id: name.security_id,
                pay_on: pay_on.clone(),
                cadence: cadence.clone(),
                provenance: "assumed_next_year".into(),
                assumed_on: as_of.to_string(),
            };
            let _ = canonical.assumed_pay_date_insert(rec).await;
        }
    }
    Ok(preview)
}
