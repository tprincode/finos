//! Classification review is a recorded human decision. It is not a MAGI oracle.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassificationReview {
    pub fact_key: String,
    pub classification: String,
    pub status: String,
}

pub fn prepare_review(
    fact_key: String,
    classification: String,
    status: String,
) -> ClassificationReview {
    ClassificationReview {
        fact_key,
        classification,
        status,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::magi::{project, MagiCompleteness, MagiDecision, MagiInputs, MagiRule};

    fn rule() -> MagiRule {
        MagiRule {
            threshold_minor: 8_460_000,
            safety_reserve_minor: 500_000,
            scale: 2,
        }
    }

    #[test]
    fn classification_review_does_not_change_magi_oracle() {
        let inputs = MagiInputs {
            actual_included_ytd_minor: 300_000,
            known_remaining_minor: 0,
            uncertain_amount_minor: 1_000_000,
            completeness: MagiCompleteness::PendingReview,
            include_trace: vec!["Classify G05-ROC-1".into()],
        };
        let before = project(rule(), inputs.clone(), &["Classify G05-ROC-1".into()]);
        let _review = prepare_review(
            "G05-ROC-1".into(),
            "return_of_capital".into(),
            "reviewed".into(),
        );
        let after = project(rule(), inputs, &["Classify G05-ROC-1".into()]);
        assert_eq!(before.decision, MagiDecision::Indeterminate);
        assert_eq!(after.decision, before.decision);
    }
}
