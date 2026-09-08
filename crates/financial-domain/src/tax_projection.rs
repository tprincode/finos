//! Tax projection is a decision-support view over MAGI. It is not a posted MAGI fact.

use crate::magi::{MagiCompleteness, MagiDecision, MagiSnapshot};
use crate::money::Money;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaxProjectionView {
    pub decision: MagiDecision,
    pub actual_included_ytd: Money,
    pub applicable_threshold: Money,
    pub completeness: MagiCompleteness,
}

/// Copy MAGI decision fields into a projection view. Does not change the snapshot.
pub fn tax_projection_from_magi(snapshot: &MagiSnapshot) -> TaxProjectionView {
    TaxProjectionView {
        decision: snapshot.decision,
        actual_included_ytd: snapshot.actual_included_ytd,
        applicable_threshold: snapshot.threshold,
        completeness: snapshot.completeness,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::magi::{project, MagiCompleteness, MagiDecision, MagiInputs, MagiRule};

    #[test]
    fn tax_projection_does_not_change_magi_decision() {
        let rule = MagiRule {
            threshold_minor: 8_460_000,
            safety_reserve_minor: 500_000,
            scale: 2,
        };
        let snapshot = project(
            rule,
            MagiInputs {
                actual_included_ytd_minor: 300_000,
                known_remaining_minor: 0,
                uncertain_amount_minor: 0,
                completeness: MagiCompleteness::Complete,
                include_trace: vec!["w2".into()],
            },
            &[],
        );
        let before = snapshot.decision;
        let view = tax_projection_from_magi(&snapshot);
        assert_eq!(view.decision, MagiDecision::Safe);
        assert_eq!(view.decision, before);
        assert_eq!(snapshot.decision, before);
        assert_eq!(view.actual_included_ytd.amount_minor, 300_000);
        assert_eq!(view.applicable_threshold.amount_minor, 8_460_000);
    }
}
