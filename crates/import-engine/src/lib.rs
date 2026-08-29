//! Import pipeline: stage, validate, approve, post (Capture component group).

mod broker;
mod production;
mod retrieve;

pub use broker::{detect_broker, parse_broker_csv, BrokerLayout};
pub use production::{
    parse_production_templates, production_template_basis_totals, production_template_totals,
};
pub use retrieve::{
    collect_declaration_candidates_for, collect_declarations_for, collect_from_fetched_page,
    collect_last_price_quotes, collect_last_price_quotes_for, declaration_candidates,
    enrich_collector_quote_only, enrich_retrieve_body, is_registered_declaration_source, live_market_snapshot, live_price_quote,
    live_roc_candidates, page_content_hash, parse_19a1_notice, parse_amplify_distributions,
    parse_edgar_offering_as_of, parse_edgar_offering_price, parse_generic_distributions,
    parse_moneymarket_distributions, parse_nasdaq_dividends, parse_neos_distributions,
    parse_proshares_distribution_summary, parse_roundhill_distribution_api,
    parse_roundhill_distributions, parse_saba_distributions, parse_simplify_distributions,
    parse_yahoo_daily_closes,
    parse_yieldmax_distributions,
    price_quote_candidates, profile_from_vendor_htmls, registered_declaration_sources,
    retrieve_result, roundhill_fund_page, DeclarationCollectOutcome, DeclarationTarget,
    LastPriceTarget,
};
pub use financial_domain::div1::{div1_adapter_missing, is_div1};

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
