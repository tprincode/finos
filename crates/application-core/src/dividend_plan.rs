//! Home Dividend Plan rollup. Calculator annual by account plus live market value.
//! Missing plan or last price stays unknown — never invented $0 — except Energy
//! and Robinhood, which do not generate income and roll $0 / 0%.

use std::collections::HashMap;

use crate::contracts::{
    DividendPlanHomeBody, DividendPlanRowBody, PlanHistoryRecord, PositionCharacteristicRecord,
};
use crate::ports::canonical::Canonical;
use crate::ports::platform::PlatformError;
use chrono::{Datelike, Months, NaiveDate};
use financial_domain::account_value::account_custodian;
use financial_domain::calculator::{plan_payment_cents, PaymentCadence};
use financial_domain::collector::calculator_view_includes;
use financial_domain::income_plan::map_income_plan_account;

/// Home grid averages: Account 9, Income, FI Roth, Car only.
const AVG_MONTHLY_INCOME_KEYS: [&str; 4] = ["Account 9", "Income", "Roth", "Car"];

const PLAN_ROWS: [(&str, &str); 8] = [
    ("Speculation", "Speculation"),
    ("Robinhood", "Robinhood"),
    ("Income", "Income"),
    ("Health", "Health"),
    ("Roth", "FI Roth"),
    ("Energy", "Energy"),
    ("Car", "Car"),
    ("Account 9", "9"),
];

pub fn plan_row_key(name: &str) -> Option<&'static str> {
    if let Some(key) = map_income_plan_account(name) {
        return Some(key);
    }
    let n = name.trim().to_ascii_lowercase();
    if n.contains("energy") {
        return Some("Energy");
    }
    if account_custodian(name) == "Direct" {
        return Some("Energy");
    }
    None
}

fn no_income_account(key: &str) -> bool {
    key == "Energy" || key == "Robinhood"
}

pub fn is_avg_monthly_income_account(name: &str) -> bool {
    map_income_plan_account(name).is_some_and(|key| AVG_MONTHLY_INCOME_KEYS.contains(&key))
}

/// Previous 12 complete calendar months: as-of 2026-09-12 → 2025-09-01 through 2026-08-31.
pub fn previous_twelve_complete_months(as_of: &str) -> Option<(String, String)> {
    let day = if as_of.len() >= 10 {
        &as_of[..10]
    } else {
        as_of
    };
    let d = NaiveDate::parse_from_str(day, "%Y-%m-%d").ok()?;
    let end = d.with_day(1)?.pred_opt()?;
    let start = end.with_day(1)?.checked_sub_months(Months::new(11))?;
    Some((
        start.format("%Y-%m-%d").to_string(),
        end.format("%Y-%m-%d").to_string(),
    ))
}

fn avg_monthly_plan_from_annuals(annuals: impl Iterator<Item = Option<i64>>) -> Option<i64> {
    monthly_of(sum_known(annuals))
}

fn monthly_of(annual: Option<i64>) -> Option<i64> {
    annual.map(|amt| amt / 12)
}

fn weekly_of(annual: Option<i64>) -> Option<i64> {
    annual.map(|amt| amt / 52)
}

/// Plan annual ÷ live market value, in bps (10000 = 100.00%). Zero plan is 0%.
fn effective_annual_bps(annual: Option<i64>, market_value: Option<i64>) -> Option<i64> {
    let annual = annual?;
    if annual == 0 {
        return Some(0);
    }
    let mv = market_value?;
    if mv <= 0 {
        return None;
    }
    Some(annual.saturating_mul(10_000) / mv)
}

fn sum_known(values: impl Iterator<Item = Option<i64>>) -> Option<i64> {
    let mut sum = 0i64;
    let mut any = false;
    for value in values {
        if let Some(v) = value {
            sum += v;
            any = true;
        }
    }
    any.then_some(sum)
}

fn all_known_sum(values: impl Iterator<Item = Option<i64>>) -> Option<i64> {
    let mut sum = 0i64;
    for value in values {
        sum += value?;
    }
    Some(sum)
}

fn bucket_row(
    key: &str,
    label: &str,
    annual: Option<i64>,
    market_value: Option<i64>,
) -> DividendPlanRowBody {
    let annual = if no_income_account(key) {
        Some(0)
    } else {
        annual
    };
    let monthly = monthly_of(annual);
    let (monthly_income, monthly_medical) = if key == "Health" {
        (None, monthly)
    } else {
        (monthly, None)
    };
    DividendPlanRowBody {
        account_name: label.to_string(),
        annual_dividend_minor: annual,
        market_value_minor: market_value,
        monthly_income_minor: monthly_income,
        monthly_reinvest_minor: None,
        monthly_medical_minor: monthly_medical,
        weekly_minor: weekly_of(annual),
        effective_annual_bps: effective_annual_bps(annual, market_value),
    }
}

fn annual_for_lots(
    lots: &[(uuid::Uuid, i64, u8)],
    securities: &HashMap<uuid::Uuid, String>,
    plans: &HashMap<uuid::Uuid, &PlanHistoryRecord>,
    characteristics: &HashMap<uuid::Uuid, &PositionCharacteristicRecord>,
) -> Option<i64> {
    let mut sum = 0i64;
    let mut any = false;
    for (security_id, qty, qty_scale) in lots {
        if *qty <= 0 {
            continue;
        }
        let symbol = securities
            .get(security_id)
            .map(String::as_str)
            .unwrap_or("");
        let ch = characteristics.get(security_id).copied();
        if !calculator_view_includes(
            ch.map(|c| c.div_type.as_str()).unwrap_or(""),
            symbol,
            ch.map(|c| c.payment_frequency.as_str()).unwrap_or(""),
        ) {
            continue;
        }
        let Some(plan) = plans.get(security_id) else {
            continue;
        };
        let periods = ch
            .and_then(|c| PaymentCadence::parse(&c.payment_frequency))
            .and_then(PaymentCadence::periods)
            .unwrap_or(0);
        if periods == 0 {
            continue;
        }
        sum += plan_payment_cents(
            *qty,
            *qty_scale,
            plan.amount_per_share_minor,
            plan.amount_scale,
        )
        .saturating_mul(periods as i64);
        any = true;
    }
    any.then_some(sum)
}

pub async fn dividend_plan_home_view(
    canonical: &dyn Canonical,
    as_of: &str,
) -> Result<DividendPlanHomeBody, PlatformError> {
    let home = crate::account_value::account_value_home_view(canonical, as_of).await?;
    let accounts = canonical.account_list().await?;
    let basis = canonical.basis_get().await?;
    let securities = canonical.security_list().await?;
    let plans = canonical.plan_history_list().await?;
    let characteristics = canonical.position_characteristic_list().await?;
    let symbol_by = securities
        .iter()
        .map(|s| (s.security_id, s.symbol.clone()))
        .collect::<HashMap<_, _>>();
    let plan_by = plans
        .iter()
        .map(|p| (p.security_id, p))
        .collect::<HashMap<_, _>>();
    let char_by = characteristics
        .iter()
        .map(|c| (c.security_id, c))
        .collect::<HashMap<_, _>>();
    let account_key = accounts
        .iter()
        .filter_map(|a| plan_row_key(&a.name).map(|key| (a.account_id, key)))
        .collect::<HashMap<_, _>>();
    let mut lots_by_key: HashMap<&str, Vec<(uuid::Uuid, i64, u8)>> = HashMap::new();
    for lot in &basis.lots {
        if lot.remaining_quantity_minor <= 0 {
            continue;
        }
        let Some(key) = account_key.get(&lot.account_id).copied() else {
            continue;
        };
        lots_by_key.entry(key).or_default().push((
            lot.security_id,
            lot.remaining_quantity_minor,
            lot.quantity_scale,
        ));
    }
    let mut mv_by_key: HashMap<&str, Option<i64>> = HashMap::new();
    for series in &home.accounts {
        let Some(key) = plan_row_key(&series.account_name) else {
            continue;
        };
        let mv = if series.current_complete {
            series.current_minor
        } else {
            None
        };
        mv_by_key.insert(key, mv);
    }
    let rows: Vec<DividendPlanRowBody> = PLAN_ROWS
        .iter()
        .map(|(key, label)| {
            let annual = annual_for_lots(
                lots_by_key.get(key).map(Vec::as_slice).unwrap_or(&[]),
                &symbol_by,
                &plan_by,
                &char_by,
            );
            bucket_row(key, label, annual, mv_by_key.get(key).copied().flatten())
        })
        .collect();
    let annual_total = sum_known(rows.iter().map(|r| r.annual_dividend_minor));
    let mv_total = all_known_sum(rows.iter().map(|r| r.market_value_minor));
    let total = DividendPlanRowBody {
        account_name: "Grand total".into(),
        annual_dividend_minor: annual_total,
        market_value_minor: mv_total,
        monthly_income_minor: sum_known(rows.iter().map(|r| r.monthly_income_minor)),
        monthly_reinvest_minor: None,
        monthly_medical_minor: sum_known(rows.iter().map(|r| r.monthly_medical_minor)),
        weekly_minor: sum_known(rows.iter().map(|r| r.weekly_minor)),
        effective_annual_bps: effective_annual_bps(annual_total, mv_total),
    };
    Ok(DividendPlanHomeBody {
        rows,
        total,
        scale: 2,
    })
}

/// Plan annual ÷ 12 and trailing-12-month actual ÷ 12 for 9 / Income / FI Roth / Car.
pub async fn home_avg_monthly_income(
    canonical: &dyn Canonical,
    as_of: &str,
) -> Result<(Option<i64>, Option<i64>), PlatformError> {
    let plan = dividend_plan_home_view(canonical, as_of).await?;
    let plan_monthly = avg_monthly_plan_from_annuals(plan.rows.iter().filter_map(|row| {
        is_avg_monthly_income_account(&row.account_name).then_some(row.annual_dividend_minor)
    }));
    let Some((start, end)) = previous_twelve_complete_months(as_of) else {
        return Ok((plan_monthly, None));
    };
    let accounts = canonical.account_list().await?;
    let allowed: std::collections::HashSet<_> = accounts
        .iter()
        .filter(|a| is_avg_monthly_income_account(&a.name))
        .map(|a| a.account_id)
        .collect();
    let dividend = canonical.dividend_get().await?;
    let total = dividend
        .actuals
        .iter()
        .filter(|a| {
            allowed.contains(&a.account_id)
                && a.occurred_on.as_str() >= start.as_str()
                && a.occurred_on.as_str() <= end.as_str()
        })
        .map(|a| financial_domain::money::to_usd_cents(a.amount_minor, a.scale))
        .sum::<i64>();
    Ok((plan_monthly, Some(total / 12)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn energyx_maps_to_energy_row() {
        assert_eq!(plan_row_key("EnergyX"), Some("Energy"));
        assert_eq!(plan_row_key("ENERGYX"), Some("Energy"));
        assert_eq!(plan_row_key("Energy"), Some("Energy"));
        assert_eq!(plan_row_key("9"), Some("Account 9"));
        assert_eq!(plan_row_key("FI Roth"), Some("Roth"));
        assert_eq!(
            financial_domain::income_plan::display_account_label("Roth"),
            "FI Roth"
        );
    }

    #[test]
    fn weekly_is_annual_over_fifty_two() {
        assert_eq!(weekly_of(Some(12_000)), Some(230));
        assert_eq!(monthly_of(Some(12_000)), Some(1_000));
        let health = bucket_row("Health", "Health", Some(12_000), Some(5_000));
        assert_eq!(health.monthly_medical_minor, Some(1_000));
        assert_eq!(health.monthly_income_minor, None);
        assert_eq!(health.monthly_reinvest_minor, None);
        let income = bucket_row("Income", "Income", Some(12_000), Some(5_000));
        assert_eq!(income.monthly_income_minor, Some(1_000));
        assert_eq!(income.monthly_medical_minor, None);
        assert_eq!(income.weekly_minor, Some(230));
        assert_eq!(income.effective_annual_bps, Some(24_000));
        let rh = bucket_row("Robinhood", "Robinhood", None, Some(25_986));
        assert_eq!(rh.annual_dividend_minor, Some(0));
        assert_eq!(rh.monthly_income_minor, Some(0));
        assert_eq!(rh.weekly_minor, Some(0));
        assert_eq!(rh.effective_annual_bps, Some(0));
        let energy = bucket_row("Energy", "Energy", None, Some(260_000));
        assert_eq!(energy.annual_dividend_minor, Some(0));
        assert_eq!(energy.effective_annual_bps, Some(0));
    }

    #[test]
    fn previous_twelve_complete_months_ends_last_month() {
        assert_eq!(
            previous_twelve_complete_months("2026-09-12"),
            Some(("2025-09-01".into(), "2026-08-31".into()))
        );
        assert_eq!(
            previous_twelve_complete_months("2026-01-01"),
            Some(("2025-01-01".into(), "2025-12-31".into()))
        );
        assert!(is_avg_monthly_income_account("Income"));
        assert!(is_avg_monthly_income_account("FI Roth"));
        assert!(is_avg_monthly_income_account("9"));
        assert!(is_avg_monthly_income_account("Car"));
        assert!(!is_avg_monthly_income_account("Health"));
        assert!(!is_avg_monthly_income_account("Speculation"));
        assert!(!is_avg_monthly_income_account("Energy"));
        assert_eq!(
            avg_monthly_plan_from_annuals([Some(12_000), Some(12_000), None].into_iter()),
            Some(2_000)
        );
        assert_eq!(
            avg_monthly_plan_from_annuals([None, None].into_iter()),
            None
        );
    }
}
