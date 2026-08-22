//! Import pipeline: stage, validate, approve, post (Capture component group).

mod broker;
mod production;
mod retrieve;

pub use broker::{detect_broker, parse_broker_csv, BrokerLayout};
pub use production::{parse_production_templates, production_template_totals};
pub use retrieve::{
    declaration_candidates, enrich_retrieve_body, live_market_snapshot, price_quote_candidates,
    retrieve_result,
};

use financial_domain::error::DomainError;
use financial_domain::money::require_known_amount;

/// Stable idempotency key for a source document (ADR-0009).
pub fn document_key(source_id: &str, content_hash: &str) -> String {
    format!("{source_id}:{content_hash}")
}

/// Candidate amount must be known before post. Unknown is not zero.
pub fn require_candidate_amount(amount_minor: Option<i64>, scale: u8) -> Result<i64, DomainError> {
    Ok(require_known_amount(amount_minor, scale)?.amount_minor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_source_and_hash_share_a_key() {
        assert_eq!(
            document_key("seed", "abc"),
            document_key("seed", "abc")
        );
        assert_ne!(document_key("seed", "abc"), document_key("seed", "def"));
    }

    #[test]
    fn missing_amount_does_not_become_zero() {
        assert!(require_candidate_amount(None, 2).is_err());
        assert_eq!(require_candidate_amount(Some(0), 2).unwrap(), 0);
    }
}
