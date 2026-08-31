//! Lifetime distributions and ROC research completeness. Unknown stays unknown.

use crate::error::DomainError;
use crate::money::{rescale, to_usd_cents};

/// Plan YOC uses the locked cadence 52 / 12 / 4 (AC-PD-06). Unknown is 0, never a default 12.
pub fn planning_periods_per_year(frequency: &str) -> u8 {
    crate::calculator::PaymentCadence::parse(frequency)
        .and_then(crate::calculator::PaymentCadence::periods)
        .unwrap_or(0)
}

/// Account-line remaining qty must sum to open-lot qty per security (AC-PD-04).
pub fn account_qty_matches_open_lots(
    account_lines: &[(uuid::Uuid, i64, u8)],
    open_lots: &[(uuid::Uuid, i64, u8)],
) -> Result<(), DomainError> {
    fn summed(rows: &[(uuid::Uuid, i64, u8)]) -> std::collections::HashMap<uuid::Uuid, (i64, u8)> {
        let mut out = std::collections::HashMap::new();
        for (security_id, qty, scale) in rows {
            let entry = out.entry(*security_id).or_insert((0, *scale));
            let qty_scale = entry.1.max(*scale);
            entry.0 = rescale(entry.0, entry.1, qty_scale) + rescale(*qty, *scale, qty_scale);
            entry.1 = qty_scale;
        }
        out
    }
    let by_line = summed(account_lines);
    let by_lot = summed(open_lots);
    let mut ids: std::collections::HashSet<uuid::Uuid> = by_line.keys().copied().collect();
    ids.extend(by_lot.keys().copied());
    for id in ids {
        let line = by_line.get(&id).copied().unwrap_or((0, 0));
        let lot = by_lot.get(&id).copied().unwrap_or((0, 0));
        let scale = line.1.max(lot.1);
        let line_q = rescale(line.0, line.1, scale);
        let lot_q = rescale(lot.0, lot.1, scale);
        if line_q != lot_q {
            return Err(DomainError::QtyReconcileMismatch);
        }
    }
    Ok(())
}

/// Cost recovery = total distributions / original economic cost, in bps.
/// Combining total distributions with a ROC-reduced denominator is forbidden (AC-PD-20).
/// Zero or missing cost stays unknown, never a silent 0 bps.
pub fn cost_recovery_bps(
    total_distributions_minor: i64,
    original_economic_cost_minor: i64,
    denominator_is_roc_reduced: bool,
) -> Result<Option<i64>, DomainError> {
    if denominator_is_roc_reduced {
        return Err(DomainError::CostRecoveryRocReducedDenominator);
    }
    if original_economic_cost_minor <= 0 {
        return Ok(None);
    }
    Ok(Some(
        total_distributions_minor.saturating_mul(10_000) / original_economic_cost_minor,
    ))
}

fn cents(amount_minor: i64, scale: u8) -> i64 {
    to_usd_cents(amount_minor, scale)
}

/// Opened original economic cost of lots (never remaining tax / ROC-reduced).
pub fn lifetime_original_economic_cost_cents<'a, I>(lots: I) -> i64
where
    I: IntoIterator<Item = &'a (i64, u8)>,
{
    lots.into_iter()
        .map(|(amount, scale)| cents(*amount, *scale))
        .sum()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RocObservationView<'a> {
    pub tax_year: &'a str,
    pub kind: &'a str,
    pub source: &'a str,
    pub roc_pct_minor: Option<i64>,
    pub established_how: &'a str,
}

fn year_eq(tax_year: &str, year: &str) -> bool {
    tax_year.trim() == year || tax_year.trim().starts_with(year)
}

fn is_actual(kind: &str) -> bool {
    kind.eq_ignore_ascii_case("actual")
}

fn is_estimate(kind: &str) -> bool {
    let k = kind.to_ascii_lowercase();
    k == "estimate" || k.contains("19a")
}

fn has_source(source: &str) -> bool {
    !source.trim().is_empty()
}

fn actual_2025_with_source(obs: &[RocObservationView<'_>]) -> bool {
    obs.iter().any(|o| {
        year_eq(o.tax_year, "2025")
            && is_actual(o.kind)
            && has_source(o.source)
            && (o.roc_pct_minor.is_some() || !o.established_how.trim().is_empty())
    })
}

fn actual_or_estimate_2026(obs: &[RocObservationView<'_>]) -> bool {
    obs.iter().any(|o| {
        year_eq(o.tax_year, "2026")
            && has_source(o.source)
            && (is_actual(o.kind) || is_estimate(o.kind))
            && (o.roc_pct_minor.is_some() || is_estimate(o.kind))
    })
}

fn estimate_2026_or_19a1(obs: &[RocObservationView<'_>]) -> bool {
    obs.iter().any(|o| {
        (year_eq(o.tax_year, "2026") && is_estimate(o.kind) && o.roc_pct_minor.is_some())
            || o.kind.to_ascii_lowercase().contains("19a") && o.roc_pct_minor.is_some()
    })
}

/// In-scope if needs_roc_research, any stored ROC year percent, or open lots on Car.
pub fn roc_in_scope(
    needs_roc_research: bool,
    any_roc_year_percent: bool,
    has_open_car_lots: bool,
) -> bool {
    needs_roc_research || any_roc_year_percent || has_open_car_lots
}

/// True when any lot opened on or before 31 Dec of `year` (YYYY).
pub fn held_in_calendar_year(opened_on: &[&str], year: i32) -> bool {
    let y = format!("{year:04}");
    opened_on.iter().any(|on| {
        let stamp = on.trim();
        stamp.len() >= 4 && &stamp[..4] <= y.as_str()
    })
}

/// Derived research strip. `needsRocResearch` alone is never complete.
/// 1099 actual for year Y is only missing when the name was held in Y.
/// A year not held is N/A — never missing-1099. 2026 actual is not required in 2026.
pub fn roc_research_status(
    in_scope: bool,
    observations: &[RocObservationView<'_>],
    held_in_2025: bool,
) -> &'static str {
    if !in_scope {
        return "not-in-scope";
    }
    let has_2025 = actual_2025_with_source(observations);
    let has_2026 = actual_or_estimate_2026(observations) || estimate_2026_or_19a1(observations);
    if held_in_2025 && has_2025 && has_2026 {
        return "complete";
    }
    if estimate_2026_or_19a1(observations) {
        if !held_in_2025 {
            return "estimate-only · N/A 2025";
        }
        if !has_2025 {
            return "estimate-only · missing-1099";
        }
        return "estimate-only";
    }
    if !held_in_2025 {
        return "N/A 2025";
    }
    "missing-1099"
}

pub fn declaration_freshness(
    entered_or_paid_on: &[&str],
    today: &str,
    last_run_at: &str,
    last_run_ok: Option<bool>,
) -> &'static str {
    let run_today = !last_run_at.is_empty()
        && (last_run_at.starts_with(today) || last_run_at == today);
    let stamp_today = entered_or_paid_on
        .iter()
        .any(|stamp| stamp.starts_with(today) || *stamp == today);
    if run_today || stamp_today {
        return "current";
    }
    let has_paid = entered_or_paid_on.iter().any(|s| !s.is_empty());
    if last_run_ok != Some(true) && !has_paid {
        return "unavailable";
    }
    "stale"
}

pub fn recorded_status(is_active: bool, open_lots: bool, had_lots: bool, has_characteristics: bool) -> &'static str {
    if open_lots && is_active {
        "included"
    } else if open_lots && !is_active {
        "inactive"
    } else if had_lots && !open_lots {
        "closed_no_open_lots"
    } else if has_characteristics {
        "characteristics_only"
    } else {
        "characteristics_only"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn ac_pd_20_rejects_roc_reduced_denominator() {
        assert_eq!(
            cost_recovery_bps(1_000, 10_000, true),
            Err(DomainError::CostRecoveryRocReducedDenominator)
        );
        assert_eq!(cost_recovery_bps(1_000, 10_000, false), Ok(Some(1_000)));
        assert_eq!(cost_recovery_bps(1_000, 0, false), Ok(None));
    }

    #[test]
    fn ac_pd_04_qty_mismatch_is_named() {
        let a = Uuid::from_u128(1);
        let ok = account_qty_matches_open_lots(&[(a, 10, 0), (a, 5, 0)], &[(a, 15, 0)]);
        assert!(ok.is_ok());
        let bad = account_qty_matches_open_lots(&[(a, 10, 0)], &[(a, 15, 0)]);
        assert_eq!(bad, Err(DomainError::QtyReconcileMismatch));
    }

    #[test]
    fn plan_periods_are_52_12_4() {
        assert_eq!(planning_periods_per_year("Weekly"), 52);
        assert_eq!(planning_periods_per_year("monthly"), 12);
        assert_eq!(planning_periods_per_year("Quarterly"), 4);
        assert_eq!(planning_periods_per_year("12"), 12);
        assert_eq!(planning_periods_per_year("None"), 0);
        assert_eq!(planning_periods_per_year(""), 0);
    }

    #[test]
    fn roc_strip_needs_source_and_years() {
        let none = roc_research_status(false, &[], true);
        assert_eq!(none, "not-in-scope");
        let missing = roc_research_status(true, &[], true);
        assert_eq!(missing, "missing-1099");
        let na_prior = roc_research_status(true, &[], false);
        assert_eq!(na_prior, "N/A 2025");
        let estimate = [RocObservationView {
            tax_year: "2026",
            kind: "estimate",
            source: "template-positions",
            roc_pct_minor: Some(8_000),
            established_how: "",
        }];
        assert_eq!(
            roc_research_status(true, &estimate, false),
            "estimate-only · N/A 2025"
        );
        assert_eq!(
            roc_research_status(true, &estimate, true),
            "estimate-only · missing-1099"
        );
        let complete = [
            RocObservationView {
                tax_year: "2025",
                kind: "actual",
                source: "template-positions",
                roc_pct_minor: Some(9_000),
                established_how: "",
            },
            RocObservationView {
                tax_year: "2026",
                kind: "estimate",
                source: "template-positions",
                roc_pct_minor: Some(8_000),
                established_how: "",
            },
        ];
        assert_eq!(roc_research_status(true, &complete, true), "complete");
        assert_eq!(
            roc_research_status(true, &complete, false),
            "estimate-only · N/A 2025"
        );
        let no_source = [RocObservationView {
            tax_year: "2025",
            kind: "actual",
            source: "",
            roc_pct_minor: Some(9_000),
            established_how: "",
        }];
        assert_eq!(roc_research_status(true, &no_source, true), "missing-1099");
        assert!(!held_in_calendar_year(&["2026-08-15"], 2025));
        assert!(held_in_calendar_year(&["2026-08-15"], 2026));
    }

    #[test]
    fn declaration_freshness_follows_last_run_and_stamps() {
        assert_eq!(declaration_freshness(&[], "2026-08-25", "", None), "unavailable");
        assert_eq!(
            declaration_freshness(&["2026-08-01"], "2026-08-25", "2026-08-25", Some(true)),
            "current"
        );
        assert_eq!(
            declaration_freshness(&["2026-08-01"], "2026-08-25", "2026-08-24", Some(true)),
            "stale"
        );
        assert_eq!(
            declaration_freshness(&[], "2026-08-25", "2026-08-25", Some(false)),
            "current"
        );
    }
}
