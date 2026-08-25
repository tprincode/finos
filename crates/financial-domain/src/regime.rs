//! Owner-dated bull/bear windows. The domain does not pick the period (PD-BR-03).
//! Unknown stays unknown; zeros are not substituted. Guarded denominators.

use crate::error::DomainError;

pub const TIER_RULESET: &str = "pd-tier-2026.1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PricePoint {
    pub on: String,
    pub price_minor: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegimeInputs {
    pub start_on: String,
    pub end_on: String,
    pub kind: String,
    pub method: String,
    pub reason: String,
    pub prices: Vec<PricePoint>,
    pub benchmark_prices: Vec<PricePoint>,
    /// Distribution cash in window, same scale as prices (typically 2).
    pub distribution_minor: Option<i64>,
    pub planned_income_minor: Option<i64>,
    pub observed_income_minor: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegimeResult {
    pub price_return_bps: Option<i64>,
    pub total_return_bps: Option<i64>,
    pub cushion_bps: Option<i64>,
    pub max_drawdown_bps: Option<i64>,
    pub recovery_ratio_bps: Option<i64>,
    pub recovery_days: Option<i64>,
    pub income_reliability_bps: Option<i64>,
    pub bear_relative_bps: Option<i64>,
    pub downside_capture_bps: Option<i64>,
    pub upside_capture_bps: Option<i64>,
    pub completeness: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceDimensions {
    pub income_reliability: Option<i64>,
    pub downside_resilience: Option<i64>,
    pub recovery_upside: Option<i64>,
    pub nav_persistence: Option<i64>,
    pub diversification: Option<i64>,
    pub data_confidence: i64,
    pub known_components: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TierSuggestion {
    pub suggested_tier: String,
    pub ruleset: String,
    pub reason: String,
    pub complete: bool,
}

/// Split-adjusted close when a split occurred. The domain does not branch on method.
pub const REGIME_PRICE_METHOD: &str = "adjusted";

/// Period is owner-complete when dates and kind exist. Method is locked; reason is implied by kind.
pub fn period_ready(start_on: &str, end_on: &str, kind: &str) -> bool {
    !start_on.trim().is_empty()
        && !end_on.trim().is_empty()
        && matches!(
            kind.trim().to_ascii_lowercase().as_str(),
            "bull" | "bear" | "recovery" | "stress"
        )
}

pub fn locked_selection_reason(kind: &str) -> String {
    format!("dates named as {} period", kind.trim())
}

pub fn calculate(inputs: &RegimeInputs) -> Result<RegimeResult, DomainError> {
    if !period_ready(&inputs.start_on, &inputs.end_on, &inputs.kind) {
        return Err(DomainError::RegimePeriodIncomplete);
    }
    let path = in_window(&inputs.prices, &inputs.start_on, &inputs.end_on);
    if path.len() < 2 {
        return Ok(empty_result("incomplete"));
    }
    let first = path[0].price_minor;
    let last = path[path.len() - 1].price_minor;
    if first <= 0 {
        return Ok(empty_result("incomplete"));
    }
    let price_return_bps = Some(bps(last - first, first));
    let total_return_bps = inputs
        .distribution_minor
        .map(|dist| bps(last - first + dist, first));
    let cushion_bps = match (total_return_bps, price_return_bps) {
        (Some(total), Some(price)) => Some(total - price),
        _ => None,
    };
    let max_drawdown_bps = max_drawdown(&path);
    let (recovery_ratio_bps, recovery_days) = recovery(&path);
    let income_reliability_bps = match (inputs.planned_income_minor, inputs.observed_income_minor) {
        (Some(plan), Some(obs)) if plan > 0 => Some(bps(obs, plan)),
        _ => None,
    };
    let bench = in_window(&inputs.benchmark_prices, &inputs.start_on, &inputs.end_on);
    let (bear_relative_bps, downside_capture_bps, upside_capture_bps) =
        relative_and_capture(price_return_bps, &bench, &inputs.kind);
    let completeness = if income_reliability_bps.is_some() && bench.len() >= 2 {
        "complete"
    } else {
        "partial"
    };
    Ok(RegimeResult {
        price_return_bps,
        total_return_bps,
        cushion_bps,
        max_drawdown_bps,
        recovery_ratio_bps,
        recovery_days,
        income_reliability_bps,
        bear_relative_bps,
        downside_capture_bps,
        upside_capture_bps,
        completeness: completeness.into(),
    })
}

pub fn dimensions(result: &RegimeResult, diversification: Option<i64>) -> EvidenceDimensions {
    let income = result.income_reliability_bps.map(clamp_score);
    let downside = result.max_drawdown_bps.map(|dd| clamp_score(10_000 + dd));
    let recovery = result.recovery_ratio_bps.map(clamp_score);
    let nav = match (result.price_return_bps, result.total_return_bps) {
        (Some(p), Some(t)) => Some(clamp_score(5_000 + (p - (t - p).min(t)) / 2)),
        _ => None,
    };
    let parts = [
        income,
        downside,
        recovery,
        nav,
        diversification,
    ];
    let known = parts.iter().filter(|p| p.is_some()).count() as u8;
    EvidenceDimensions {
        income_reliability: income,
        downside_resilience: downside,
        recovery_upside: recovery,
        nav_persistence: nav,
        diversification,
        data_confidence: ((known as i64) * 100) / 6,
        known_components: known,
    }
}

/// Expected declaration/cash observations in an owner window. None if frequency is unknown.
pub fn expected_observation_count(
    start_on: &str,
    end_on: &str,
    periods_per_year: u8,
) -> Option<i64> {
    if periods_per_year == 0 {
        return None;
    }
    let days = days_between(start_on, end_on)? + 1;
    if days <= 0 {
        return None;
    }
    Some(((periods_per_year as i64) * days / 365).max(1))
}

/// Suggestion only. Never writes an effective tier.
pub fn suggest_tier(dims: &EvidenceDimensions, kind: &str) -> TierSuggestion {
    let complete = dims.known_components >= 3;
    if !complete {
        return TierSuggestion {
            suggested_tier: String::new(),
            ruleset: TIER_RULESET.into(),
            reason: "incomplete evidence; no tier suggestion".into(),
            complete: false,
        };
    }
    let downside = dims.downside_resilience.unwrap_or(50);
    let income = dims.income_reliability.unwrap_or(50);
    let nav = dims.nav_persistence.unwrap_or(50);
    let bearish = matches!(kind.to_ascii_lowercase().as_str(), "bear" | "stress");
    let (tier, reason) = if bearish && downside < 40 && income < 50 {
        (
            "Risk On",
            "weak downside resilience and income reliability in a stress window",
        )
    } else if income >= 80 && downside >= 60 && nav >= 50 {
        (
            "Foundation",
            "income reliability and downside resilience support Foundation",
        )
    } else {
        ("Core", "mixed evidence supports Core")
    };
    TierSuggestion {
        suggested_tier: tier.into(),
        ruleset: TIER_RULESET.into(),
        reason: reason.into(),
        complete: true,
    }
}

fn empty_result(completeness: &str) -> RegimeResult {
    RegimeResult {
        price_return_bps: None,
        total_return_bps: None,
        cushion_bps: None,
        max_drawdown_bps: None,
        recovery_ratio_bps: None,
        recovery_days: None,
        income_reliability_bps: None,
        bear_relative_bps: None,
        downside_capture_bps: None,
        upside_capture_bps: None,
        completeness: completeness.into(),
    }
}

fn in_window<'a>(prices: &'a [PricePoint], start: &str, end: &str) -> Vec<&'a PricePoint> {
    let mut v: Vec<&PricePoint> = prices
        .iter()
        .filter(|p| p.on.as_str() >= start && p.on.as_str() <= end && p.price_minor > 0)
        .collect();
    v.sort_by(|a, b| a.on.cmp(&b.on));
    v
}

fn bps(num: i64, den: i64) -> i64 {
    if den == 0 {
        return 0;
    }
    (num.saturating_mul(10_000)) / den
}

fn max_drawdown(path: &[&PricePoint]) -> Option<i64> {
    let mut peak = path.first()?.price_minor;
    let mut worst = 0i64;
    for p in path {
        if p.price_minor > peak {
            peak = p.price_minor;
        }
        if peak > 0 {
            let dd = bps(p.price_minor - peak, peak);
            if dd < worst {
                worst = dd;
            }
        }
    }
    Some(worst)
}

fn recovery(path: &[&PricePoint]) -> (Option<i64>, Option<i64>) {
    if path.len() < 3 {
        return (None, None);
    }
    let mut peak = path[0].price_minor;
    let mut trough = path[0].price_minor;
    let mut trough_idx = 0usize;
    let mut peak_before = path[0].price_minor;
    for (i, p) in path.iter().enumerate() {
        if p.price_minor > peak {
            peak = p.price_minor;
        }
        if p.price_minor < trough {
            trough = p.price_minor;
            trough_idx = i;
            peak_before = peak.max(trough);
        }
    }
    if trough_idx + 1 >= path.len() || peak_before <= trough {
        return (None, None);
    }
    let mut later_peak = trough;
    for p in path.iter().skip(trough_idx + 1) {
        if p.price_minor > later_peak {
            later_peak = p.price_minor;
        }
    }
    let drop = peak_before - trough;
    if drop <= 0 {
        return (None, None);
    }
    let ratio = Some(bps(later_peak - trough, drop));
    let days = days_between(&path[trough_idx].on, &path.last().unwrap().on);
    (ratio, days)
}

fn days_between(a: &str, b: &str) -> Option<i64> {
    let pa = chrono::NaiveDate::parse_from_str(&a[..a.len().min(10)], "%Y-%m-%d").ok()?;
    let pb = chrono::NaiveDate::parse_from_str(&b[..b.len().min(10)], "%Y-%m-%d").ok()?;
    Some((pb - pa).num_days())
}

fn relative_and_capture(
    position_bps: Option<i64>,
    bench: &[&PricePoint],
    kind: &str,
) -> (Option<i64>, Option<i64>, Option<i64>) {
    let Some(pos) = position_bps else {
        return (None, None, None);
    };
    if bench.len() < 2 || bench[0].price_minor <= 0 {
        return (None, None, None);
    }
    let bench_bps = bps(
        bench[bench.len() - 1].price_minor - bench[0].price_minor,
        bench[0].price_minor,
    );
    let relative = Some(pos - bench_bps);
    if bench_bps == 0 {
        return (relative, None, None);
    }
    let capture = Some(bps(pos, bench_bps));
    let k = kind.to_ascii_lowercase();
    if k == "bear" || k == "stress" {
        (relative, capture, None)
    } else {
        (relative, None, capture)
    }
}

fn clamp_score(bps_like: i64) -> i64 {
    (bps_like / 100).clamp(0, 100)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pt(on: &str, price: i64) -> PricePoint {
        PricePoint {
            on: on.into(),
            price_minor: price,
        }
    }

    fn ready_inputs(prices: Vec<PricePoint>) -> RegimeInputs {
        RegimeInputs {
            start_on: "2026-01-02".into(),
            end_on: "2026-04-07".into(),
            kind: "Bear".into(),
            method: REGIME_PRICE_METHOD.into(),
            reason: "owner stress window".into(),
            prices,
            benchmark_prices: vec![pt("2026-01-02", 10_000), pt("2026-04-07", 8_000)],
            distribution_minor: Some(500),
            planned_income_minor: Some(1_000),
            observed_income_minor: Some(900),
        }
    }

    #[test]
    fn incomplete_period_is_blocked() {
        let mut inputs = ready_inputs(vec![pt("2026-01-02", 10_000), pt("2026-04-07", 8_000)]);
        inputs.start_on.clear();
        assert_eq!(calculate(&inputs), Err(DomainError::RegimePeriodIncomplete));
    }

    #[test]
    fn method_and_reason_are_not_a_gate() {
        let mut inputs = ready_inputs(vec![pt("2026-01-02", 10_000), pt("2026-04-07", 8_000)]);
        inputs.method.clear();
        inputs.reason.clear();
        assert!(calculate(&inputs).is_ok());
    }

    #[test]
    fn thin_path_is_incomplete_not_zero() {
        let out = calculate(&ready_inputs(vec![pt("2026-01-02", 10_000)])).unwrap();
        assert_eq!(out.completeness, "incomplete");
        assert_eq!(out.price_return_bps, None);
        assert_eq!(out.cushion_bps, None);
    }

    #[test]
    fn cushion_is_total_minus_price() {
        let out = calculate(&ready_inputs(vec![
            pt("2026-01-02", 10_000),
            pt("2026-02-01", 9_000),
            pt("2026-04-07", 8_000),
        ]))
        .unwrap();
        assert_eq!(out.price_return_bps, Some(-2_000));
        assert_eq!(out.total_return_bps, Some(-1_500));
        assert_eq!(out.cushion_bps, Some(500));
        assert_eq!(
            out.cushion_bps.unwrap(),
            out.total_return_bps.unwrap() - out.price_return_bps.unwrap()
        );
        assert!(out.max_drawdown_bps.unwrap() < 0);
        assert_eq!(out.income_reliability_bps, Some(9_000));
        assert_eq!(out.downside_capture_bps, Some(10_000));
    }

    #[test]
    fn suggest_does_not_imply_a_write() {
        let dims = dimensions(
            &RegimeResult {
                price_return_bps: Some(-2_000),
                total_return_bps: Some(-1_500),
                cushion_bps: Some(500),
                max_drawdown_bps: Some(-8_000),
                recovery_ratio_bps: Some(4_000),
                recovery_days: Some(40),
                income_reliability_bps: Some(3_000),
                bear_relative_bps: Some(-100),
                downside_capture_bps: Some(12_000),
                upside_capture_bps: None,
                completeness: "partial".into(),
            },
            None,
        );
        let s = suggest_tier(&dims, "Bear");
        assert!(s.complete);
        assert_eq!(s.ruleset, TIER_RULESET);
        assert_eq!(s.suggested_tier, "Risk On");
    }

    #[test]
    fn missing_distribution_leaves_cushion_unknown() {
        let mut inputs = ready_inputs(vec![pt("2026-01-02", 10_000), pt("2026-04-07", 8_000)]);
        inputs.distribution_minor = None;
        let out = calculate(&inputs).unwrap();
        assert_eq!(out.price_return_bps, Some(-2_000));
        assert_eq!(out.total_return_bps, None);
        assert_eq!(out.cushion_bps, None);
    }

    #[test]
    fn capture_blocked_when_benchmark_flat() {
        let mut inputs = ready_inputs(vec![pt("2026-01-02", 10_000), pt("2026-04-07", 9_000)]);
        inputs.benchmark_prices = vec![pt("2026-01-02", 5_000), pt("2026-04-07", 5_000)];
        let out = calculate(&inputs).unwrap();
        assert!(out.bear_relative_bps.is_some());
        assert_eq!(out.downside_capture_bps, None);
    }

    #[test]
    fn ac_pd_11_missing_evidence_lowers_confidence() {
        let thin = calculate(&ready_inputs(vec![pt("2026-01-02", 10_000)])).unwrap();
        let full = calculate(&ready_inputs(vec![
            pt("2026-01-02", 10_000),
            pt("2026-02-01", 9_000),
            pt("2026-04-07", 8_000),
        ]))
        .unwrap();
        let thin_d = dimensions(&thin, None);
        let full_d = dimensions(&full, None);
        assert!(thin_d.data_confidence < full_d.data_confidence);
        assert!(thin_d.known_components < full_d.known_components);
        assert!(thin.price_return_bps.is_none());
        assert!(thin_d.income_reliability.is_none() || thin_d.known_components < 6);
    }
}
