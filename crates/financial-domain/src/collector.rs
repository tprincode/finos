//! Collector complete is not last_run_ok alone. A failed last run is still a gap.
//! Required Skip is not complete (F1).
//! Remaining-year prefers stored unoccurred dates vs the issuer list (D2).
//! Calendar `remaining_periods_to_year_end` is derive-once only.
//!
//! Runtime Fill-research-gaps holes are not the complete-gate list:
//! missing provider, frequency, DIV-1 (CASH counts as filled), or ROC estimate
//! (CASH / not-in-scope / BDC-1099 exempt). Underlying may be blank; it is recorded
//! and is not a hole.

use crate::calculator::PaymentCadence;
use crate::current_price::{is_cash, uses_cash_par};
use crate::declaration_lookback::{validate_paid_lookback, LookbackValidation};
use crate::div1::is_div1;
use crate::schedule::remaining_periods_to_year_end;

/// Required checklist fields. Skip on any of these is incomplete. Backtest is not in this list.
pub const REQUIRED_FIELDS: &[&str] = &[
    "div_type",
    "frequency",
    "underlying",
    "provider",
    "risk_tier",
    "roc_estimate",
    "paid_history",
    "remaining_year",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectorCompleteSpec<'a> {
    pub has_template: bool,
    pub symbol: &'a str,
    pub div_type: &'a str,
    pub payment_frequency: &'a str,
    pub underlying: &'a str,
    pub provider: &'a str,
    pub risk_tier: &'a str,
    pub paid_declaration_count: u8,
    pub inception_on: &'a str,
    pub as_of: &'a str,
    /// Persisted current-year estimate. `Some(0)` is filled when the notice said 0.
    pub roc_pct_minor: Option<i64>,
    /// Owner accepted (`RocPlanConfirm` / owner-typed). Propose-only is not enough.
    pub roc_owner_accepted: bool,
    /// Planned remaining payables already stored or derived for the current year.
    pub planned_remaining: Option<i64>,
    /// Unoccurred issuer-published dates through 31 Dec. `None` = vendor published none.
    pub issuer_remaining: Option<i64>,
    /// Required fields the owner skipped. Skip ≠ complete (F1). Backtest skip is omitted.
    pub skipped_required: &'a [&'a str],
    /// Owner Accept that the retrieved issuer table is the full paid series.
    /// Used when cadence × age would invent pays the issuer never published.
    pub lookback_owner_accepted: bool,
    /// `Some(false)` = last declaration retrieve missed. Never-run (`None`) is not this gap.
    pub last_run_ok: Option<bool>,
}

/// First-class ROC scope. Decide before any 19a-1 fetch (D1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RocScope {
    Cash,
    NotInScope,
    Bdc1099,
    InScope,
}

impl RocScope {
    pub fn skips_roc_estimate(self) -> bool {
        !matches!(self, RocScope::InScope)
    }

    pub fn skips_19a1_fetch(self) -> bool {
        !matches!(self, RocScope::InScope)
    }
}

/// Scope from symbol, cash flag, provider, and issuer-page / structure keywords.
/// Does not guess 0%.
pub fn roc_scope(symbol: &str, div_type: &str, provider: &str, structure_text: &str) -> RocScope {
    if uses_cash_par(div_type, symbol) || symbol.trim().eq_ignore_ascii_case("CASH1") {
        return RocScope::Cash;
    }
    let sym = symbol.trim().to_ascii_uppercase();
    let blob = format!(
        "{} {} {}",
        sym,
        provider.trim(),
        structure_text.trim()
    )
    .to_ascii_lowercase();
    if matches!(
        sym.as_str(),
        "HAKY" | "QYLD" | "AMDW" | "TSPY" | "SVOL" | "PAY1" | "QDTE" | "RDTE" | "TOPW"
            | "XDTE" | "YBTC"
    ) || blob.contains("covered call")
        || blob.contains("19a-1")
        || blob.contains("1940 act")
        || blob.contains("weeklypay")
        || blob.contains("weekly pay")
        || (blob.contains("roundhill") && blob.contains("weekly"))
    {
        return RocScope::InScope;
    }
    if matches!(
        sym.as_str(),
        "EPD" | "MPLX" | "MLP1" | "TSLL" | "SOXL" | "MSTU" | "EFC" | "ORD1"
    ) || blob.contains("limited partnership")
        || blob.contains("k-1")
        || blob.contains(" k1")
        || (blob.contains("mlp") && !blob.contains("amplify"))
        || blob.contains("partnership")
        || payment_type_is_not_roc(&blob)
    {
        return RocScope::NotInScope;
    }
    if matches!(sym.as_str(), "GLAD") || blob.contains("bdc") {
        return RocScope::Bdc1099;
    }
    RocScope::InScope
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectorCompleteStatus {
    pub complete: bool,
    pub gaps: Vec<String>,
}

pub fn collector_is_complete(spec: &CollectorCompleteSpec<'_>) -> bool {
    collector_status(spec).complete
}

pub fn collector_status(spec: &CollectorCompleteSpec<'_>) -> CollectorCompleteStatus {
    let mut gaps = Vec::new();
    if !spec.has_template {
        gaps.push("template".into());
    }
    if spec.last_run_ok == Some(false) {
        gaps.push("last_run".into());
    }

    let cash = uses_cash_par(spec.div_type, spec.symbol);
    let div_ok = if cash {
        is_cash(spec.div_type) || uses_cash_par(spec.div_type, spec.symbol)
    } else {
        is_div1(spec.div_type)
    };
    if !div_ok {
        gaps.push("div_type".into());
    }

    if cash {
        push_required_skips(&mut gaps, spec.skipped_required, &["div_type"]);
        return CollectorCompleteStatus {
            complete: gaps.is_empty(),
            gaps,
        };
    }

    if PaymentCadence::parse(spec.payment_frequency)
        .and_then(PaymentCadence::periods)
        .is_none()
    {
        gaps.push("frequency".into());
    }
    if spec.underlying.trim().is_empty()
        || spec.underlying.eq_ignore_ascii_case(spec.symbol)
    {
        gaps.push("underlying".into());
    }
    if spec.provider.trim().is_empty() {
        gaps.push("provider".into());
    }
    if !risk_accepted(spec.risk_tier) {
        gaps.push("risk_tier".into());
    }

    let scope = roc_scope(spec.symbol, spec.div_type, spec.provider, "");
    if !scope.skips_roc_estimate() {
        let roc_filled = spec.roc_pct_minor.is_some() && spec.roc_owner_accepted;
        if !roc_filled {
            gaps.push("roc_estimate".into());
        }
    }

    match validate_paid_lookback(
        spec.paid_declaration_count,
        spec.inception_on,
        spec.as_of,
        spec.payment_frequency,
    ) {
        LookbackValidation::Complete | LookbackValidation::CompleteViaInception { .. } => {}
        LookbackValidation::ShortWithInception { paid, .. }
        | LookbackValidation::ShortWithoutInception { paid }
            if paid > 0 && spec.lookback_owner_accepted => {}
        _ => gaps.push("paid_history".into()),
    }

    if !remaining_year_matches(spec) {
        gaps.push("remaining_year".into());
    }

    push_required_skips(&mut gaps, spec.skipped_required, REQUIRED_FIELDS);

    CollectorCompleteStatus {
        complete: gaps.is_empty(),
        gaps,
    }
}

fn remaining_year_matches(spec: &CollectorCompleteSpec<'_>) -> bool {
    let Some(periods) = PaymentCadence::parse(spec.payment_frequency)
        .and_then(PaymentCadence::periods)
    else {
        return false;
    };
    let Some(planned) = spec.planned_remaining else {
        return false;
    };
    if planned < 0 {
        return false;
    }
    let planned_u = planned.min(i64::from(u8::MAX)) as u8;
    if planned_u > periods {
        return false;
    }
    match spec.issuer_remaining.filter(|n| *n > 0) {
        None => planned_u > 0 || remaining_periods_to_year_end(spec.as_of, periods) == Some(0),
        Some(issuer) => {
            let issuer_u = issuer.min(i64::from(u8::MAX)) as u8;
            if periods == 52 {
                planned_u.abs_diff(issuer_u) <= 1
            } else {
                planned_u == issuer_u
            }
        }
    }
}

/// True when both lists have unoccurred dates through 31 Dec and they differ.
pub fn remaining_year_dates_disagree(stored: &[&str], issuer: &[&str], as_of: &str) -> bool {
    let year = as_of.get(..4).unwrap_or("");
    if year.len() != 4 {
        return false;
    }
    let year_end = format!("{year}-12-31");
    let filt = |ons: &[&str]| -> std::collections::BTreeSet<String> {
        ons.iter()
            .filter_map(|on| {
                let p = on.trim();
                if p.len() >= 10 && p >= as_of && p <= year_end.as_str() {
                    Some(p[..10].to_string())
                } else {
                    None
                }
            })
            .collect()
    };
    let issuer_set = filt(issuer);
    !issuer_set.is_empty() && filt(stored) != issuer_set
}

fn risk_accepted(raw: &str) -> bool {
    crate::plan_review::owner_risk_accepted(raw)
}

/// Runtime Fill-research-gaps flags. `is_hole` ignores underlying.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeResearchGaps {
    pub provider_blank: bool,
    pub frequency_blank: bool,
    pub div_type_blank: bool,
    pub roc_estimate_blank: bool,
    pub underlying_blank: bool,
}

impl RuntimeResearchGaps {
    pub fn is_hole(&self) -> bool {
        self.provider_blank || self.frequency_blank || self.div_type_blank || self.roc_estimate_blank
    }
}

/// DIV-1 on create for income payers. Money-markets (SPAXX / FDRXX / SWVXX)
/// and existing or incoming CASH stay CASH. Never writes DIV-1 onto cash.
/// Empty `div_type` on a non-cash name becomes DIV-1.
pub fn div_type_on_create(symbol: &str, incoming: &str, existing: &str) -> String {
    if uses_cash_par(incoming, symbol) || is_cash(existing) {
        return "CASH".into();
    }
    let existing = existing.trim();
    if !existing.is_empty() {
        return existing.to_string();
    }
    let incoming = incoming.trim();
    if !incoming.is_empty() {
        return incoming.to_string();
    }
    "DIV-1".into()
}

/// Ordinary / qualified / MLP / K-1 / cash interest is not ROC. Providers do not print ROC=0.
pub fn payment_type_is_not_roc(text: &str) -> bool {
    roc_from_payment_type(text).is_some()
}

/// `Some((0, why))` when the payment type cannot be ROC. `None` = still need a 19a-1 percent.
pub fn roc_from_payment_type(text: &str) -> Option<(i64, &'static str)> {
    let t = text.to_ascii_lowercase();
    if t.contains("ordinary dividend")
        || t.contains("ordinary income")
        || t.contains("qualified dividend")
        || t.contains("non-qualified distribution")
        || t.contains("nonqualified distribution")
        || ((t.contains("equity-linked note") || t.contains("equity linked note") || t.contains(" eln"))
            && (t.contains("ordinary") || t.contains("taxed as")))
        || t.contains("does not list return of capital")
        || t.contains("does not list roc")
    {
        return Some((0, "ordinary/qualified dividend — not ROC"));
    }
    if t.contains("limited partnership")
        || t.contains("k-1")
        || t.contains(" k1")
        || (t.contains("mlp") && !t.contains("amplify"))
    {
        return Some((0, "MLP/K-1 — not ROC"));
    }
    None
}

/// Fallback search when the standing `roc_source_url` fails.
/// Template: `19.1 tax ROC (TICKER) (Provider) website data source`.
pub fn roc_fallback_search_query(ticker: &str, provider: &str) -> String {
    let ticker = ticker.trim().to_ascii_uppercase();
    let provider = provider.trim();
    if provider.is_empty() {
        format!("19.1 tax ROC {ticker} website data source")
    } else {
        format!("19.1 tax ROC {ticker} {provider} website data source")
    }
}

/// CASH / not-in-scope / BDC-1099 do not need an ROC estimate or 19a-1 hunt.
pub fn needs_roc_research_on_create(div_type: &str, symbol: &str) -> bool {
    !roc_scope(symbol, div_type, "", "").skips_roc_estimate()
}

/// DIV-1 income names and CASH (DIV-2 nickname). Equities stay off the URL sheet.
pub fn is_div1_or_cash(div_type: &str, symbol: &str) -> bool {
    uses_cash_par(div_type, symbol) || is_div1(div_type)
}

/// Calculator / Home plan count list DIV-1 and CASH only. `None` cadence stays stored, not shown.
pub fn calculator_view_includes(div_type: &str, symbol: &str, payment_frequency: &str) -> bool {
    if crate::calculator::is_non_paying(payment_frequency) {
        return false;
    }
    is_div1_or_cash(div_type, symbol)
}

/// Failing DIV-1/CASH collector with an empty seed URL. Never-run may probe once.
pub fn needs_owner_seed_url(
    div_type: &str,
    symbol: &str,
    source_url: &str,
    last_run_ok: Option<bool>,
    has_retrieve_miss_ticket: bool,
) -> bool {
    if !is_div1_or_cash(div_type, symbol) {
        return false;
    }
    if !source_url.trim().is_empty() {
        return false;
    }
    last_run_ok == Some(false) || has_retrieve_miss_ticket
}

/// Shortest operator gap labels for the Collectors footer grid.
/// Complete Y/N still uses `collector_status`. Underlying appears only when N.
pub fn fleet_gap_labels(gaps: &[String], complete: bool) -> Vec<String> {
    const ORDER: &[(&str, &str)] = &[
        ("seed_url", "seed URL"),
        ("roc_estimate", "ROC"),
        ("div_type", "DIV-1"),
        ("frequency", "frequency"),
        ("provider", "provider"),
        ("paid_history", "inception"),
        ("last_run", "last run"),
        ("underlying", "underlying"),
    ];
    let mut out = Vec::new();
    for (code, label) in ORDER {
        if *code == "underlying" && complete {
            continue;
        }
        if gaps.iter().any(|g| g.eq_ignore_ascii_case(code)) {
            out.push((*label).to_string());
        }
    }
    for gap in gaps {
        if ORDER.iter().any(|(code, _)| gap.eq_ignore_ascii_case(code)) {
            continue;
        }
        if !out.iter().any(|l| l.eq_ignore_ascii_case(gap)) {
            out.push(gap.clone());
        }
    }
    out
}

/// Home / standing order: one issuer retrieve per enabled collector **today**.
/// This is not lookback row count and not remaining-year pay dates.
pub fn declaration_daily_retrieve_current(
    last_run_ok: Option<bool>,
    last_run_at: &str,
    today: &str,
) -> bool {
    let day = today.trim();
    if day.len() < 10 {
        return false;
    }
    last_run_at.trim().starts_with(day) && last_run_ok == Some(true)
}

/// Unoccurred stored pay dates through 31 Dec. Does not rebuild a year.
pub fn stored_remaining_planned(pay_ons: &[&str], as_of: &str) -> i64 {
    let year = as_of.get(..4).unwrap_or("");
    if year.len() != 4 {
        return 0;
    }
    let year_end = format!("{year}-12-31");
    pay_ons
        .iter()
        .filter(|on| {
            let p = on.trim();
            p >= as_of && p <= year_end.as_str()
        })
        .count() as i64
}

/// Fill-research-gaps holes for one open-lot payer. Not collector-complete.
pub fn runtime_research_gaps(
    symbol: &str,
    provider: &str,
    payment_frequency: &str,
    div_type: &str,
    roc_pct_minor: Option<i64>,
    underlying: &str,
) -> RuntimeResearchGaps {
    let cash = uses_cash_par(div_type, symbol);
    let frequency_ok = PaymentCadence::parse(payment_frequency)
        .and_then(PaymentCadence::periods)
        .is_some();
    let div_ok = if cash {
        true
    } else {
        is_div1(div_type)
    };
    RuntimeResearchGaps {
        provider_blank: provider.trim().is_empty(),
        frequency_blank: !frequency_ok,
        div_type_blank: !div_ok,
        roc_estimate_blank: !roc_scope(symbol, div_type, provider, "").skips_roc_estimate()
            && roc_pct_minor.is_none(),
        underlying_blank: underlying.trim().is_empty(),
    }
}

fn push_required_skips(gaps: &mut Vec<String>, skipped: &[&str], required: &[&str]) {
    for field in skipped {
        let n = field.trim();
        if n.eq_ignore_ascii_case("backtest") {
            continue;
        }
        if required.iter().any(|r| r.eq_ignore_ascii_case(n))
            && !gaps.iter().any(|g| g.eq_ignore_ascii_case(n))
        {
            gaps.push(n.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base<'a>(skipped: &'a [&'a str]) -> CollectorCompleteSpec<'a> {
        CollectorCompleteSpec {
            has_template: true,
            symbol: "PAY1",
            div_type: "DIV-1",
            payment_frequency: "Monthly",
            underlying: "UNDR",
            provider: "Issuer",
            risk_tier: "Foundation",
            paid_declaration_count: 12,
            inception_on: "",
            as_of: "2026-08-22",
            roc_pct_minor: Some(9_000),
            roc_owner_accepted: true,
            planned_remaining: Some(5),
            issuer_remaining: None,
            skipped_required: skipped,
            lookback_owner_accepted: false,
            last_run_ok: None,
        }
    }

    #[test]
    fn last_run_miss_is_not_complete() {
        let mut spec = base(&[]);
        spec.last_run_ok = Some(false);
        let status = collector_status(&spec);
        assert!(!status.complete);
        assert!(status.gaps.iter().any(|g| g == "last_run"));
        spec.last_run_ok = Some(true);
        assert!(collector_is_complete(&spec));
    }

    #[test]
    fn complete_when_required_fields_accepted() {
        assert!(collector_is_complete(&base(&[])));
        assert!(collector_status(&base(&[])).gaps.is_empty());
    }

    #[test]
    fn f1_required_skip_is_not_complete() {
        let spec = base(&["roc_estimate"]);
        let status = collector_status(&spec);
        assert!(!status.complete);
        assert!(status.gaps.iter().any(|g| g == "roc_estimate"));
    }

    #[test]
    fn f1_backtest_skip_does_not_block() {
        assert!(collector_is_complete(&base(&["backtest"])));
    }

    #[test]
    fn f1_roc_propose_only_is_not_complete() {
        let mut spec = base(&[]);
        spec.roc_owner_accepted = false;
        assert!(!collector_is_complete(&spec));
        spec.roc_pct_minor = Some(0);
        spec.roc_owner_accepted = true;
        assert!(collector_is_complete(&spec));
    }

    #[test]
    fn f2_monthly_august_expects_five_not_twelve() {
        assert_eq!(remaining_periods_to_year_end("2026-08-22", 12), Some(5));
        let mut spec = base(&[]);
        spec.planned_remaining = Some(5);
        spec.issuer_remaining = None;
        assert!(collector_is_complete(&spec));
        spec.planned_remaining = Some(13);
        assert!(!collector_is_complete(&spec));
        spec.planned_remaining = Some(3);
        spec.as_of = "2026-09-06";
        spec.issuer_remaining = Some(4);
        assert!(!collector_is_complete(&spec));
        spec.issuer_remaining = Some(3);
        assert!(collector_is_complete(&spec));
    }

    #[test]
    fn mlp1_and_ord1_complete_without_roc_pct() {
        for symbol in ["MLP1", "ORD1"] {
            let spec = CollectorCompleteSpec {
                symbol,
                roc_pct_minor: None,
                roc_owner_accepted: false,
                planned_remaining: Some(3),
                issuer_remaining: None,
                ..base(&[])
            };
            let status = collector_status(&spec);
            assert!(status.complete, "{symbol} {status:?}");
            assert!(!status.gaps.iter().any(|g| g == "roc_estimate"), "{symbol}");
            assert!(!needs_roc_research_on_create("DIV-1", symbol));
            assert!(!runtime_research_gaps(symbol, "Issuer", "Monthly", "DIV-1", None, "UNDR")
                .roc_estimate_blank);
        }
    }

    #[test]
    fn weekly_derived_remaining_when_vendor_has_no_upcoming() {
        let mut spec = base(&[]);
        spec.symbol = "AMZY";
        spec.payment_frequency = "Weekly";
        spec.as_of = "2026-09-08";
        spec.planned_remaining = Some(16);
        spec.issuer_remaining = Some(0);
        assert!(collector_is_complete(&spec));
        assert!(!collector_status(&spec)
            .gaps
            .iter()
            .any(|g| g == "remaining_year"));
    }

    #[test]
    fn pay1_three_remaining_without_december_is_not_a_gap() {
        let mut spec = base(&[]);
        spec.as_of = "2026-09-06";
        spec.planned_remaining = Some(3);
        spec.issuer_remaining = None;
        assert!(collector_is_complete(&spec));
        assert!(!collector_status(&spec)
            .gaps
            .iter()
            .any(|g| g == "remaining_year"));
        let stored = ["2026-10-15", "2026-11-13"];
        assert_eq!(stored_remaining_planned(&stored, "2026-09-06"), 2);
        assert!(!remaining_year_dates_disagree(&stored, &[], "2026-09-06"));
        assert!(remaining_year_dates_disagree(
            &stored,
            &["2026-10-15", "2026-11-13", "2026-12-15"],
            "2026-09-06"
        ));
    }

    #[test]
    fn roc_fallback_search_is_ticker_provider_website_data_source() {
        assert_eq!(
            roc_fallback_search_query("haky", "amplify"),
            "19.1 tax ROC HAKY amplify website data source"
        );
        assert_eq!(
            roc_fallback_search_query("HAKY", ""),
            "19.1 tax ROC HAKY website data source"
        );
    }

    #[test]
    fn roc_scope_skips_cash_mlp_ordinary_and_bdc() {
        assert_eq!(roc_scope("CASH1", "CASH", "", ""), RocScope::Cash);
        assert_eq!(roc_scope("MLP1", "DIV-1", "", ""), RocScope::NotInScope);
        assert_eq!(roc_scope("ORD1", "DIV-1", "", ""), RocScope::NotInScope);
        assert_eq!(roc_scope("GLAD", "DIV-1", "", ""), RocScope::Bdc1099);
        assert_eq!(roc_scope("PAY1", "DIV-1", "", ""), RocScope::InScope);
        assert_eq!(
            roc_scope("FOO", "DIV-1", "", "limited partnership K-1"),
            RocScope::NotInScope
        );
        assert_eq!(
            roc_from_payment_type("JPMorgan Nasdaq Equity Premium Income ETF ordinary dividend"),
            Some((0, "ordinary/qualified dividend — not ROC"))
        );
        assert_eq!(
            roc_scope("JEPQ", "DIV-1", "JPMorgan", "ordinary dividend ELN ordinary income"),
            RocScope::NotInScope
        );
        assert_eq!(
            roc_scope("FOO", "DIV-1", "", "covered call 19a-1 1940 Act ETF"),
            RocScope::InScope
        );
        assert_eq!(roc_scope("MSTU", "DIV-1", "", ""), RocScope::NotInScope);
        assert_eq!(roc_scope("EFC", "DIV-1", "", ""), RocScope::NotInScope);
        assert!(!runtime_research_gaps("GLAD", "Gladstone", "Monthly", "DIV-1", None, "UNDR")
            .roc_estimate_blank);
    }

    #[test]
    fn owner_accept_clears_short_lookback_without_inventing_pays() {
        let spec = CollectorCompleteSpec {
            symbol: "MSTU",
            payment_frequency: "Quarterly",
            paid_declaration_count: 1,
            inception_on: "2024-09-18",
            as_of: "2026-09-08",
            roc_pct_minor: Some(0),
            roc_owner_accepted: true,
            planned_remaining: Some(1),
            issuer_remaining: None,
            lookback_owner_accepted: true,
            ..base(&[])
        };
        let status = collector_status(&spec);
        assert!(status.complete, "{status:?}");
        assert!(!status.gaps.iter().any(|g| g == "paid_history"), "{status:?}");
        let mut no = spec;
        no.lookback_owner_accepted = false;
        assert!(collector_status(&no).gaps.iter().any(|g| g == "paid_history"));
    }

    #[test]
    fn f2_quarterly_and_weekly_cap() {
        assert_eq!(remaining_periods_to_year_end("2026-08-22", 4), Some(2));
        assert_eq!(remaining_periods_to_year_end("2026-01-15", 12), Some(12));
        assert!(remaining_periods_to_year_end("2026-01-03", 52).is_some_and(|n| n <= 52));
    }

    #[test]
    fn cash_skips_declaration_and_roc() {
        let spec = CollectorCompleteSpec {
            has_template: true,
            symbol: "SPAXX",
            div_type: "CASH",
            payment_frequency: "",
            underlying: "",
            provider: "",
            risk_tier: "",
            paid_declaration_count: 0,
            inception_on: "",
            as_of: "2026-08-22",
            roc_pct_minor: None,
            roc_owner_accepted: false,
            planned_remaining: None,
            issuer_remaining: None,
            skipped_required: &[],
            lookback_owner_accepted: false,
            last_run_ok: None,
        };
        assert!(collector_is_complete(&spec));
    }

    #[test]
    fn empty_div_type_is_incomplete() {
        let mut spec = base(&[]);
        spec.div_type = "";
        assert!(!collector_is_complete(&spec));
    }

    #[test]
    fn fill_gaps_hole_is_provider_frequency_div1_or_roc() {
        let filled = runtime_research_gaps("PAY1", "Issuer", "Monthly", "DIV-1", Some(5000), "UNDR");
        assert!(!filled.is_hole());

        let no_underlying = runtime_research_gaps("PAY1", "Issuer", "Monthly", "DIV-1", Some(5000), "");
        assert!(!no_underlying.is_hole());
        assert!(no_underlying.underlying_blank);

        assert!(runtime_research_gaps("PAY1", "", "Monthly", "DIV-1", Some(5000), "UNDR").is_hole());
        assert!(runtime_research_gaps("PAY1", "Issuer", "", "DIV-1", Some(5000), "UNDR").is_hole());
        assert!(runtime_research_gaps("PAY1", "Issuer", "Monthly", "", Some(5000), "UNDR").is_hole());
        assert!(runtime_research_gaps("PAY1", "Issuer", "Monthly", "DIV-1", None, "UNDR").is_hole());
    }

    #[test]
    fn fill_gaps_cash_does_not_require_roc_or_div1() {
        let cash = runtime_research_gaps("CASH1", "Broker", "", "CASH", None, "");
        assert!(!cash.div_type_blank);
        assert!(!cash.roc_estimate_blank);
        assert!(cash.frequency_blank);
        assert!(cash.is_hole());
        let cash_ready = runtime_research_gaps("CASH1", "Broker", "Monthly", "CASH", None, "");
        assert!(!cash_ready.is_hole());
    }

    #[test]
    fn fleet_gap_labels_put_underlying_only_when_incomplete() {
        let status = collector_status(&base(&[]));
        assert!(status.complete);
        assert!(fleet_gap_labels(&status.gaps, true).is_empty());
        let mut spec = base(&[]);
        spec.underlying = "";
        spec.div_type = "";
        spec.roc_owner_accepted = false;
        let status = collector_status(&spec);
        let labels = fleet_gap_labels(&status.gaps, status.complete);
        assert!(labels.iter().any(|g| g == "DIV-1"));
        assert!(labels.iter().any(|g| g == "ROC"));
        assert!(labels.iter().any(|g| g == "underlying"));
        assert!(!status.complete);
    }

    #[test]
    fn daily_retrieve_current_is_today_and_explicit_ok() {
        assert!(declaration_daily_retrieve_current(
            Some(true),
            "2026-09-08T14:00:00",
            "2026-09-08"
        ));
        assert!(!declaration_daily_retrieve_current(
            None,
            "2026-09-08T14:00:00",
            "2026-09-08"
        ));
        assert!(!declaration_daily_retrieve_current(
            Some(false),
            "2026-09-08T14:00:00",
            "2026-09-08"
        ));
        assert!(!declaration_daily_retrieve_current(
            Some(true),
            "2026-09-07T14:00:00",
            "2026-09-08"
        ));
        assert!(!declaration_daily_retrieve_current(None, "", "2026-09-08"));
    }

    #[test]
    fn stored_remaining_planned_counts_unoccurred_through_year_end() {
        let dates = ["2026-08-31", "2026-09-30", "2026-12-31", "2027-01-31"];
        let refs: Vec<&str> = dates.iter().copied().collect();
        assert_eq!(stored_remaining_planned(&refs, "2026-09-05"), 2);
        assert_eq!(stored_remaining_planned(&refs, "2026-08-22"), 3);
    }

    #[test]
    fn div_type_on_create_sets_div1_for_income_and_cash_for_mm() {
        assert_eq!(div_type_on_create("PAY1", "", ""), "DIV-1");
        assert_eq!(div_type_on_create("NEW1", "", ""), "DIV-1");
        assert_eq!(div_type_on_create("SPAXX", "", ""), "CASH");
        assert_eq!(div_type_on_create("CASH1", "CASH", ""), "CASH");
        assert_eq!(div_type_on_create("PAY1", "DIV-1", ""), "DIV-1");
        assert_eq!(div_type_on_create("PAY1", "DIV-1", "CASH"), "CASH");
        assert!(!needs_roc_research_on_create("CASH", "CASH1"));
        assert!(needs_roc_research_on_create("DIV-1", "PAY1"));
    }

    #[test]
    fn needs_owner_seed_url_only_after_fail_on_div1_or_cash() {
        assert!(!needs_owner_seed_url("DIV-1", "PAY1", "", None, false));
        assert!(needs_owner_seed_url(
            "DIV-1",
            "PAY1",
            "",
            Some(false),
            false
        ));
        assert!(!needs_owner_seed_url(
            "DIV-1",
            "PAY1",
            "https://example.test/pay1",
            Some(false),
            false
        ));
        assert!(needs_owner_seed_url("CASH", "CASH1", "", Some(false), true));
        assert!(!needs_owner_seed_url("", "NEW1", "", Some(false), true));
        assert!(is_div1_or_cash("DIV-1", "PAY1"));
        assert!(is_div1_or_cash("CASH", "CASH1"));
        assert!(!is_div1_or_cash("", "NEW1"));
        assert!(calculator_view_includes("DIV-1", "PAY1", "Weekly"));
        assert!(calculator_view_includes("CASH", "SPAXX", "Monthly"));
        assert!(!calculator_view_includes("", "BTC-USD", ""));
        assert!(!calculator_view_includes("", "SOXL", "None"));
        assert!(!calculator_view_includes("DIV-1", "MSTU", "None"));
    }
}
