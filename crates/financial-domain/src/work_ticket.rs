//! Work tickets: one open row per security+code, each with a fix tool.
//! Retry retrieve never remaps the assigned adapter.

use crate::div1::declaration_url_matches_source;

pub const TOOL_RETRY_RETRIEVE: &str = "retry_retrieve";
pub const TOOL_LOCK_CADENCE: &str = "lock_cadence";
pub const TOOL_SET_INCEPTION: &str = "set_inception";
pub const TOOL_AMOUNT_CONFIRM: &str = "amount_confirm";
pub const TOOL_ENTER_DECLARED_AMOUNT: &str = "enter_declared_amount";
pub const TOOL_PLAN_VS_DECL: &str = "plan_vs_declaration";
pub const TOOL_SET_DIV_TYPE: &str = "set_div_type";
pub const TOOL_SET_UNDERLYING: &str = "set_underlying";
pub const TOOL_SET_PROVIDER: &str = "set_provider";
pub const TOOL_SET_RISK: &str = "set_risk";
pub const TOOL_FIX_REMAINING_YEAR: &str = "fix_remaining_year";
pub const TOOL_RUN_ROC: &str = "run_roc";

pub const CODE_ADAPTER_URL_MISMATCH: &str = "adapter_url_mismatch";
pub const CODE_MISSING_SEED_URL: &str = "missing_seed_url";

/// Codes this slice can raise. Every entry must have a tool (`tool_for_code`).
pub const RAISEABLE_CODES: &[&str] = &[
    "declaration_retrieve_miss",
    "declaration_parse_unstable",
    "parse_miss",
    "declaration_stored_mismatch",
    "declaration_history_dropped",
    "declaration_lookback_short",
    "declaration_cadence_mismatch",
    "declaration_amount_variation",
    "declaration_plan_mismatch",
    "div_type",
    "frequency",
    "underlying",
    "provider",
    "risk_tier",
    "roc_estimate",
    "remaining_year",
    "paid_payable_supersede",
    crate::mlp_sec::CODE_PAYABLE_DATE_MOVED,
    crate::mlp_sec::CODE_SEC_403,
    crate::mlp_sec::CODE_OWNER_AMOUNT,
    CODE_ADAPTER_URL_MISMATCH,
    CODE_MISSING_SEED_URL,
];

pub fn tool_for_code(code: &str) -> Option<&'static str> {
    Some(match code.trim() {
        "declaration_retrieve_miss"
        | "declaration_parse_unstable"
        | "parse_miss"
        | "declaration_stored_mismatch"
        | "declaration_history_dropped"
        | crate::mlp_sec::CODE_SEC_403
        | CODE_ADAPTER_URL_MISMATCH
        | CODE_MISSING_SEED_URL => TOOL_RETRY_RETRIEVE,
        "declaration_lookback_short" => TOOL_SET_INCEPTION,
        "declaration_cadence_mismatch" | "frequency" => TOOL_LOCK_CADENCE,
        "declaration_amount_variation" => TOOL_AMOUNT_CONFIRM,
        crate::mlp_sec::CODE_OWNER_AMOUNT => TOOL_ENTER_DECLARED_AMOUNT,
        "declaration_plan_mismatch" => TOOL_PLAN_VS_DECL,
        "div_type" => TOOL_SET_DIV_TYPE,
        "underlying" => TOOL_SET_UNDERLYING,
        "provider" => TOOL_SET_PROVIDER,
        "risk_tier" => TOOL_SET_RISK,
        "remaining_year"
        | "paid_payable_supersede"
        | crate::mlp_sec::CODE_PAYABLE_DATE_MOVED => TOOL_FIX_REMAINING_YEAR,
        "roc_estimate" => TOOL_RUN_ROC,
        _ => return None,
    })
}

pub fn field_for_code(code: &str) -> &'static str {
    match code.trim() {
        "declaration_retrieve_miss"
        | "declaration_parse_unstable"
        | "parse_miss"
        | "declaration_stored_mismatch"
        | "declaration_history_dropped"
        | crate::mlp_sec::CODE_SEC_403
        | CODE_ADAPTER_URL_MISMATCH => "declaration_miss",
        CODE_MISSING_SEED_URL => "seed_url",
        "declaration_lookback_short" => "last_run",
        "declaration_cadence_mismatch" | "frequency" => "frequency",
        "declaration_amount_variation"
        | "declaration_plan_mismatch"
        | crate::mlp_sec::CODE_OWNER_AMOUNT => "last_run",
        "div_type" => "div_type",
        "underlying" => "underlying",
        "provider" => "provider",
        "risk_tier" => "risk_tier",
        "remaining_year"
        | "paid_payable_supersede"
        | crate::mlp_sec::CODE_PAYABLE_DATE_MOVED => "remaining_year",
        "roc_estimate" => "roc_estimate",
        _ => "last_run",
    }
}

/// Owner must Except (keep vendor amount) or Reject (discard it). Does not mean the retrieve missed.
pub fn is_amount_confirm_code(code: &str) -> bool {
    matches!(
        code.trim(),
        "declaration_amount_variation" | "declaration_plan_mismatch"
    )
}

/// First history parse fail prompts a second URL once. Not lookback-short.
pub fn is_history_parse_fail(code: &str) -> bool {
    matches!(
        code.trim(),
        "declaration_retrieve_miss"
            | "declaration_parse_unstable"
            | "parse_miss"
            | "declaration_stored_mismatch"
            | "no_declaration_adapter"
            | "retrieve_parse_failed"
            | "retrieve_failed"
    )
}

/// Retrieve actually failed to parse/store. Auto-closed when a later retrieve is ok.
pub fn is_retrieve_failure_code(code: &str) -> bool {
    matches!(
        code.trim(),
        "declaration_retrieve_miss"
            | "declaration_parse_unstable"
            | "parse_miss"
            | "declaration_stored_mismatch"
            | crate::mlp_sec::CODE_SEC_403
            | CODE_ADAPTER_URL_MISMATCH
            | CODE_MISSING_SEED_URL
    )
}

/// Closed when retrieve is ok. Confirm tickets (variation) stay open for Except/Reject.
pub fn is_auto_file_on_ok_code(code: &str) -> bool {
    is_retrieve_failure_code(code) || code.trim() == "declaration_history_dropped"
}

/// Pre-D2 leftover: calendar stub year forced to 4/12/52. Close when lists agree.
pub fn is_stale_expected_4_remaining_year(reason: &str) -> bool {
    reason.to_ascii_lowercase().contains("expected 4")
}

pub fn is_declaration_ticket_code(code: &str) -> bool {
    matches!(
        code.trim(),
        "declaration_retrieve_miss"
            | "declaration_parse_unstable"
            | "parse_miss"
            | "declaration_stored_mismatch"
            | "declaration_history_dropped"
            | "declaration_lookback_short"
            | "declaration_cadence_mismatch"
            | "declaration_amount_variation"
            | "declaration_plan_mismatch"
            | crate::mlp_sec::CODE_OWNER_AMOUNT
            | CODE_ADAPTER_URL_MISMATCH
            | CODE_MISSING_SEED_URL
    )
}

/// Retry URL must be that assigned adapter's own host. Never switch Amplify → Roundhill.
pub fn retry_url_matches_assigned_adapter(assigned_source: &str, url: &str) -> bool {
    declaration_url_matches_source(assigned_source, url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_raiseable_code_has_a_tool() {
        for code in RAISEABLE_CODES {
            assert!(
                tool_for_code(code).is_some(),
                "ticket_missing_tool: {code}"
            );
        }
    }

    #[test]
    fn stale_expected_4_matches_leftover_calendar_text() {
        assert!(is_stale_expected_4_remaining_year(
            "Remaining-year count is 3, expected 4 periods through 31 Dec."
        ));
        assert!(!is_stale_expected_4_remaining_year(
            "Remaining-year stored dates disagree with the issuer list through 31 Dec."
        ));
    }

    #[test]
    fn amplify_retry_rejects_roundhill_url() {
        assert!(retry_url_matches_assigned_adapter(
            "amplify",
            "https://amplifyetfs.com/haky/#distributions"
        ));
        assert!(!retry_url_matches_assigned_adapter(
            "amplify",
            "https://www.roundhillinvestments.com/etf/topw/"
        ));
        assert!(!retry_url_matches_assigned_adapter(
            "amplify",
            "https://dividendhistory.org/payout/HAKY/"
        ));
        assert!(retry_url_matches_assigned_adapter(
            "roundhill",
            "https://www.roundhillinvestments.com/etf/topw/"
        ));
    }
}
