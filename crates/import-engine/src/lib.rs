//! Import pipeline: stage, validate, approve, post (Capture component group).

mod broker;
mod gap;
mod production;
mod retrieve;

pub use broker::{
    classify_action, crf_drip_class, detect_broker, find_broker_header, parse_broker_csv,
    parse_broker_csv_detail, parse_broker_day, resolve_account_name, ActionClass, BrokerLayout,
    BrokerParse, CrfDripClass, DEFAULT_ACCOUNT_ALIASES,
};
pub use gap::{
    dividend_gap, is_account_9, parse_roi_dividend_update, parse_yield_sheet_csv, DividendFact,
    GapKind, GapRow,
};
pub use production::{
    audit_yield_template, parse_production_templates, production_template_basis_totals,
    production_template_totals, YieldTemplateAudit,
};
pub use retrieve::{
    collect_declaration_candidates_for, collect_declarations_for, collect_from_fetched_page,
    collect_last_price_quotes, collect_last_price_quotes_for, declaration_candidates,
    enrich_collector_quote_only, enrich_retrieve_body, is_registered_declaration_source, live_market_snapshot, live_price_quote,
    live_research_identity, live_roc_candidates, live_roc_candidates_for, looks_like_roc_notice_url,
    page_content_hash, parse_19a1_notice, parse_amplify_distribution_pack, parse_amplify_distributions,
    pdf_notice_text, should_invent_dated_19a1_filenames,
    parse_search_result_urls, rank_roc_search_urls, roc_19a1_search_query, roc_estimate_from_search_hits,
    inception_search_query, inception_on_from_search_hits,
    ftvest_history_years, parse_cornerstone_press, parse_div1_distributions, parse_edgar_offering_as_of, parse_edgar_offering_price,
    parse_generic_distributions,
    parse_jpmorgan_distributions, jpmorgan_cusip_from_seed, parse_globalx_distribution_history,
    parse_moneymarket_distributions, parse_nasdaq_dividends, parse_neos_distributions,
    parse_proshares_distribution_summary, parse_roundhill_distribution_api,
    rexshares_calendar_covers_inception,
    parse_roundhill_distributions, parse_saba_distributions, parse_simplify_distributions,
    parse_yahoo_daily_closes,
    parse_yieldmax_distributions,
    price_quote_candidates, profile_from_vendor_htmls, registered_declaration_sources,
    retrieve_result, roundhill_fund_page, DeclarationCollectOutcome, DeclarationTarget,
    LastPriceTarget, LiveRocFill,
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
