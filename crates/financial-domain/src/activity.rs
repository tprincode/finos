//! Immutable activity events. Corrections link; they do not erase. No FIFO lot picker.

use crate::error::DomainError;
use crate::money::{require_known_amount, Money};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedActivity {
    pub account_id: Uuid,
    pub security_id: Option<Uuid>,
    pub activity_type: String,
    pub amount: Money,
    pub occurred_on: String,
    pub corrects_activity_id: Option<Uuid>,
    pub lot_id: Option<Uuid>,
}

/// Build a postable event. Unknown amounts fail. Lot is never auto-assigned (no FIFO).
pub fn prepare_activity(
    account_id: Uuid,
    security_id: Option<Uuid>,
    activity_type: String,
    amount_minor: Option<i64>,
    scale: u8,
    occurred_on: String,
    corrects_activity_id: Option<Uuid>,
) -> Result<PreparedActivity, DomainError> {
    let amount = require_known_amount(amount_minor, scale)?;
    Ok(PreparedActivity {
        account_id,
        security_id,
        activity_type,
        amount,
        occurred_on,
        corrects_activity_id,
        lot_id: None,
    })
}

pub fn idempotency_key(source: &str, content_hash: &str, candidate_index: u32) -> String {
    format!("{source}:{content_hash}:{candidate_index}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_amount_is_not_converted_to_zero() {
        let err = prepare_activity(
            Uuid::nil(),
            None,
            "deposit".into(),
            None,
            2,
            "2026-01-10".into(),
            None,
        )
        .unwrap_err();
        assert_eq!(err, DomainError::UnknownAmount);
    }

    #[test]
    fn posting_does_not_assign_a_fifo_lot() {
        let prepared = prepare_activity(
            Uuid::nil(),
            None,
            "buy".into(),
            Some(1_000),
            2,
            "2026-01-10".into(),
            None,
        )
        .unwrap();
        assert!(prepared.lot_id.is_none());
    }
}
