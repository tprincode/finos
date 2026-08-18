//! Marketplace MAGI projection (V1.1 §11.4). No FIFO, no unknown-to-zero.

use crate::money::Money;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MagiCompleteness {
    Complete,
    Incomplete,
    PendingReview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MagiDecision {
    Safe,
    Watch,
    LikelyOver,
    Over,
    Indeterminate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MagiRule {
    pub threshold_minor: i64,
    pub safety_reserve_minor: i64,
    pub scale: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MagiInputs {
    pub actual_included_ytd_minor: i64,
    pub known_remaining_minor: i64,
    pub uncertain_amount_minor: i64,
    pub completeness: MagiCompleteness,
    pub include_trace: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MagiSnapshot {
    pub threshold: Money,
    pub actual_included_ytd: Money,
    pub known_remaining: Money,
    pub base_forecast: Money,
    pub conservative_forecast: Money,
    pub uncertain_amount: Money,
    pub raw_headroom: Money,
    pub protected_headroom: Money,
    pub completeness: MagiCompleteness,
    pub decision: MagiDecision,
    pub warnings: Vec<String>,
    pub trace: Vec<String>,
}

fn money(amount_minor: i64, scale: u8) -> Money {
    Money {
        amount_minor,
        scale,
    }
}

pub fn project(rule: MagiRule, inputs: MagiInputs, extra_warnings: &[String]) -> MagiSnapshot {
    let scale = rule.scale;
    let base = inputs.actual_included_ytd_minor + inputs.known_remaining_minor;
    // Incomplete coverage keeps unapproved amounts out of the MAGI forecast (G-MAGI-09).
    // Pending-review still folds uncertain into conservative (G-MAGI-05).
    let conservative = if matches!(inputs.completeness, MagiCompleteness::Incomplete) {
        base
    } else {
        base + inputs.uncertain_amount_minor
    };
    let raw = rule.threshold_minor - base;
    let protected = rule.threshold_minor - conservative - rule.safety_reserve_minor;
    let incomplete = matches!(
        inputs.completeness,
        MagiCompleteness::Incomplete | MagiCompleteness::PendingReview
    );
    let decision = if incomplete {
        MagiDecision::Indeterminate
    } else if inputs.actual_included_ytd_minor >= rule.threshold_minor {
        MagiDecision::Over
    } else if conservative > rule.threshold_minor {
        MagiDecision::LikelyOver
    } else if protected <= 0 || inputs.uncertain_amount_minor > 0 {
        MagiDecision::Watch
    } else {
        MagiDecision::Safe
    };
    let mut warnings = extra_warnings.to_vec();
    if decision == MagiDecision::Indeterminate {
        warnings.retain(|w| !w.is_empty());
    }
    let mut trace = vec![
        format!(
            "threshold = {} (scale {})",
            rule.threshold_minor, rule.scale
        ),
        format!("actual included ytd = {}", inputs.actual_included_ytd_minor),
        format!("known remaining = {}", inputs.known_remaining_minor),
        format!("uncertain = {}", inputs.uncertain_amount_minor),
        format!("base = {base}"),
        format!("conservative = {conservative}"),
        format!("raw headroom = {raw}"),
        format!("protected headroom = {protected}"),
    ];
    for id in &inputs.include_trace {
        trace.push(format!("included source {id}"));
    }
    MagiSnapshot {
        threshold: money(rule.threshold_minor, scale),
        actual_included_ytd: money(inputs.actual_included_ytd_minor, scale),
        known_remaining: money(inputs.known_remaining_minor, scale),
        base_forecast: money(base, scale),
        conservative_forecast: money(conservative, scale),
        uncertain_amount: money(inputs.uncertain_amount_minor, scale),
        raw_headroom: money(raw, scale),
        protected_headroom: money(protected, scale),
        completeness: inputs.completeness,
        decision,
        warnings,
        trace,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule() -> MagiRule {
        MagiRule {
            threshold_minor: 8_460_000,
            safety_reserve_minor: 500_000,
            scale: 2,
        }
    }

    fn inputs(actual: i64, remaining: i64, uncertain: i64, completeness: MagiCompleteness) -> MagiInputs {
        MagiInputs {
            actual_included_ytd_minor: actual,
            known_remaining_minor: remaining,
            uncertain_amount_minor: uncertain,
            completeness,
            include_trace: vec![],
        }
    }

    #[test]
    fn g01_safe_below_threshold() {
        let snap = project(
            rule(),
            inputs(4_000_000, 1_000_000, 0, MagiCompleteness::Complete),
            &[],
        );
        assert_eq!(snap.decision, MagiDecision::Safe);
        assert_eq!(snap.raw_headroom.amount_minor, 3_460_000);
        assert_eq!(snap.protected_headroom.amount_minor, 2_960_000);
    }

    #[test]
    fn g02_cent_boundary() {
        let below = project(
            rule(),
            inputs(8_459_999, 0, 0, MagiCompleteness::Complete),
            &[],
        );
        let at = project(
            rule(),
            inputs(8_460_000, 0, 0, MagiCompleteness::Complete),
            &[],
        );
        let above = project(
            rule(),
            inputs(8_460_001, 0, 0, MagiCompleteness::Complete),
            &[],
        );
        assert_eq!(below.decision, MagiDecision::Watch);
        assert_eq!(at.decision, MagiDecision::Over);
        assert_eq!(above.decision, MagiDecision::Over);
    }

    #[test]
    fn unresolved_classification_is_indeterminate() {
        let snap = project(
            rule(),
            inputs(300_000, 0, 1_000_000, MagiCompleteness::PendingReview),
            &["Classify G05-ROC-1".into()],
        );
        assert_eq!(snap.decision, MagiDecision::Indeterminate);
        assert_ne!(snap.decision, MagiDecision::Safe);
    }

    #[test]
    fn incomplete_does_not_fold_uncertain_into_conservative() {
        let snap = project(
            rule(),
            MagiInputs {
                actual_included_ytd_minor: 4_000_000,
                known_remaining_minor: 0,
                uncertain_amount_minor: 500_000,
                completeness: MagiCompleteness::Incomplete,
                include_trace: vec!["G09-DIV-1".into()],
            },
            &[],
        );
        assert_eq!(snap.decision, MagiDecision::Indeterminate);
        assert_eq!(snap.conservative_forecast.amount_minor, 4_000_000);
        assert_eq!(snap.uncertain_amount.amount_minor, 500_000);
        assert_eq!(snap.protected_headroom.amount_minor, 3_960_000);
    }
}
