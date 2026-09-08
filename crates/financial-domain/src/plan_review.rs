//! Plan-review over up to 12 issuer declarations (TR-C-7). Blank ≠ zero.

use crate::error::DomainError;

pub const DECLARATION_LOOKBACK: usize = 12;
const AVG6_NEEDED: usize = 6;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanReview {
    pub observation_count: usize,
    pub most_current_minor: Option<i64>,
    pub avg6_minor: Option<i64>,
    pub min_minor: Option<i64>,
    pub max_minor: Option<i64>,
    pub average_minor: Option<i64>,
    pub eighty_pct_of_avg_minor: Option<i64>,
    pub avg6_complete: bool,
    pub full_analysis_possible: bool,
    pub confirm_blocked: bool,
    pub incomplete_reason_required: bool,
}

/// `amounts_newest_first` may include None (no observation). Zeros are observations, not blanks.
pub fn plan_review(amounts_newest_first: &[Option<i64>]) -> PlanReview {
    let observed: Vec<i64> = amounts_newest_first.iter().copied().flatten().collect();
    let n = observed.len();
    let most_current_minor = observed.first().copied();
    let avg6_complete = n >= AVG6_NEEDED;
    let avg6_minor = if avg6_complete {
        Some(mean(&observed[..AVG6_NEEDED]))
    } else {
        None
    };
    let (min_minor, max_minor, average_minor, eighty) = if n == 0 {
        (None, None, None, None)
    } else {
        let min = *observed.iter().min().unwrap();
        let max = *observed.iter().max().unwrap();
        let avg = mean(&observed);
        (Some(min), Some(max), Some(avg), Some((avg * 80) / 100))
    };
    PlanReview {
        observation_count: n,
        most_current_minor,
        avg6_minor,
        min_minor,
        max_minor,
        average_minor,
        eighty_pct_of_avg_minor: if avg6_complete { eighty } else { None },
        avg6_complete,
        full_analysis_possible: avg6_complete,
        confirm_blocked: n == 0,
        incomplete_reason_required: n > 0 && n < AVG6_NEEDED,
    }
}

fn mean(vals: &[i64]) -> i64 {
    if vals.is_empty() {
        return 0;
    }
    let sum: i128 = vals.iter().map(|v| *v as i128).sum();
    (sum / vals.len() as i128) as i64
}

/// n=0 blocked. n=1–5 requires a non-empty incomplete reason. n≥6 always allowed.
/// Does not suggest or copy Avg 6 into Plan.
pub fn plan_confirm_gate(observation_count: usize, incomplete_reason: &str) -> Result<(), DomainError> {
    if observation_count == 0 {
        return Err(DomainError::PlanConfirmBlocked);
    }
    if observation_count < AVG6_NEEDED && incomplete_reason.trim().is_empty() {
        return Err(DomainError::IncompleteAnalysisRequired);
    }
    Ok(())
}

pub fn normalize_risk_tier(raw: &str) -> String {
    let n = raw.trim().to_ascii_lowercase().replace('_', " ");
    match n.as_str() {
        "highrisk" | "high risk" | "high-risk" => "Risk On".into(),
        "foundation" => "Foundation".into(),
        "core" => "Core".into(),
        "risk on" | "riskon" => "Risk On".into(),
        _ => raw.trim().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blanks_are_not_zero_and_avg6_needs_six() {
        let none = plan_review(&[None; 12]);
        assert_eq!(none.observation_count, 0);
        assert!(none.confirm_blocked);
        assert_eq!(none.most_current_minor, None);

        let three = plan_review(&[Some(10), None, Some(20), Some(30)]);
        assert_eq!(three.observation_count, 3);
        assert_eq!(three.most_current_minor, Some(10));
        assert!(!three.avg6_complete);
        assert!(three.incomplete_reason_required);
        assert_eq!(
            plan_confirm_gate(3, ""),
            Err(DomainError::IncompleteAnalysisRequired)
        );
        assert!(plan_confirm_gate(3, "short history").is_ok());

        let twelve: Vec<Option<i64>> = (1..=12).map(Some).collect();
        let full = plan_review(&twelve);
        assert!(full.full_analysis_possible);
        assert_eq!(full.avg6_minor, Some(mean(&[1, 2, 3, 4, 5, 6])));
        assert!(plan_confirm_gate(12, "").is_ok());
        assert_eq!(plan_confirm_gate(0, "x"), Err(DomainError::PlanConfirmBlocked));
    }

    #[test]
    fn high_risk_maps_to_risk_on() {
        assert_eq!(normalize_risk_tier("HighRisk"), "Risk On");
        assert_eq!(normalize_risk_tier("foundation"), "Foundation");
    }
}
