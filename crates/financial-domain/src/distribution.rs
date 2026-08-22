//! Distribution characterization (ROC / ordinary / qualified). Does not post cash.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistributionCharacterization {
    pub category: String,
    pub amount_minor: i64,
    pub scale: u8,
}

/// Record a tax-category characterization. Unknown amounts are refused.
pub fn prepare_characterization(
    category: String,
    amount_minor: i64,
    scale: u8,
) -> DistributionCharacterization {
    DistributionCharacterization {
        category,
        amount_minor,
        scale,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::activity::prepare_activity;
    use uuid::Uuid;

    #[test]
    fn characterization_is_not_a_posted_activity() {
        let posted = prepare_activity(
            Uuid::nil(),
            None,
            "dividend".into(),
            Some(50_000),
            2,
            "2026-06-15".into(),
            None,
        )
        .unwrap();
        let _char = prepare_characterization("return_of_capital".into(), 10_000, 2);
        assert_eq!(posted.amount.amount_minor, 50_000);
        assert!(posted.lot_id.is_none());
    }
}
