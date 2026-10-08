//! Collector complete is not last_run_ok alone. A failed last run is still a gap.
//! Establish locks two retrieve templates: Template Dividend (owner seed URL) and
//! Template ROC (19.1 tax ROC search, then standing reusable URL). They stay separate.
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

/// Holdings only. Not on Income Plan. Never collectors — not Collectors grid,
/// not Run enabled, not recertify, not miss counts. Lots and history stay.
pub const NOT_A_COLLECTOR_SYMBOLS: &[&str] = &["MSTU", "SOXL", "TSLL"];
/// Same list. Prefer `NOT_A_COLLECTOR_SYMBOLS`.
pub const PARKED_LONG_HOLD_SYMBOLS: &[&str] = NOT_A_COLLECTOR_SYMBOLS;

/// Last-run-OK income fleet on the Profile A file (13 Sep 2026). Sorted.
pub const INCOME_FLEET_SYMBOLS: &[&str] = &[
    "AMDW", "AMDY", "AMZY", "BITO", "BTCI", "CEFS", "CLM", "CONY", "CRF", "EFC",
    "EPD", "ET", "FDRXX", "GLAD", "HAKY", "IGLD", "JEPQ", "MPLX", "MSTY", "NFLY",
    "NVDW", "ORC", "PLTW", "QDTE", "QDVO", "QQQI", "QYLD", "RDTE", "SPAXX", "SPYI",
    "SVOL", "SWVXX", "TOPW", "TRIN", "TSLW", "TSPY", "XDTE", "XPAY", "YBTC", "YMAX",
];

pub fn is_not_a_collector(symbol: &str) -> bool {
    let sym = symbol.trim().to_ascii_uppercase();
    NOT_A_COLLECTOR_SYMBOLS.iter().any(|s| *s == sym)
}

pub fn is_parked_long_hold(symbol: &str) -> bool {
    is_not_a_collector(symbol)
}

/// Required checklist fields. Skip on any of these is incomplete. Backtest is not in this list.
/// `template_dividend` and `template_roc` are the two locked retrieve templates (owner seed vs 19a-1 search).
pub const REQUIRED_FIELDS: &[&str] = &[
    "template_dividend",
    "template_roc",
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
    /// Template Dividend: standing declaration URL (owner seed). Empty = incomplete.
    pub has_dividend_template: bool,
    /// Template ROC: standing reusable 19a-1/tax URL. Required when ROC is in scope.
    pub has_roc_template: bool,
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
        "EPD" | "ET" | "MPLX" | "MLP1" | "TSLL" | "SOXL" | "MSTU" | "EFC" | "ORD1"
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

/// Process A establish checklist row status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EstablishRowStatus {
    Done,
    Open,
    Na,
}

impl EstablishRowStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Done => "done",
            Self::Open => "open",
            Self::Na => "na",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EstablishChecklistRow {
    pub id: &'static str,
    pub label: &'static str,
    pub status: EstablishRowStatus,
    /// When false, Open status blocks the establish footer Complete.
    pub blocks_complete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EstablishChecklist {
    pub rows: Vec<EstablishChecklistRow>,
    pub complete: bool,
    pub open_labels: Vec<String>,
}

fn gap_open(gaps: &[String], code: &str) -> bool {
    gaps.iter().any(|g| g.eq_ignore_ascii_case(code))
}

fn row(
    id: &'static str,
    label: &'static str,
    status: EstablishRowStatus,
    blocks_complete: bool,
) -> EstablishChecklistRow {
    EstablishChecklistRow {
        id,
        label,
        status,
        blocks_complete,
    }
}

/// Everything the readiness checklist needs that is not a collector gap code.
/// A struct, not more positional bools: a new MUST-satisfy step is a field, so adding one
/// is a compile error at every call site instead of a silently-skipped check.
#[derive(Debug, Clone, Copy, Default)]
pub struct ReadinessFacts<'a> {
    pub plan_known: bool,
    pub calculator_eligible: bool,
    pub price_ok: bool,
    pub has_open_lot: bool,
    pub roc_in_scope: bool,
    /// False for cash and holdings-only names: they are never collected, so the adapter
    /// and enabled rows are N/A rather than open forever.
    pub collectable: bool,
    /// Issuer adapter slug on the template. Blank or unregistered means no declaration is
    /// ever retrieved for this name — the silent failure behind a payer with no history.
    pub declaration_source: &'a str,
    /// Template row says Run enabled may collect this symbol.
    pub collector_enabled: bool,
    /// Periods the cart and Calculator annualise with. Zero means no rate can be formed,
    /// which is how a Twice monthly name shipped a rate built on the wrong period count.
    pub plan_periods_per_year: u8,
    /// Next unoccurred pay date, blank when none is known.
    pub next_pay_on: &'a str,
    /// A confirmed Plan window covers `next_pay_on`. A Plan that starts after the pay date
    /// leaves the week grid with no Plan $/sh even though the name looks established.
    pub plan_covers_next_pay: bool,
}

/// Establish checklist for Add Investment. Lot does not block Complete.
pub fn establish_checklist(
    status: &CollectorCompleteStatus,
    facts: ReadinessFacts<'_>,
) -> EstablishChecklist {
    let ReadinessFacts {
        plan_known,
        calculator_eligible,
        price_ok,
        has_open_lot,
        roc_in_scope,
        collectable,
        declaration_source,
        collector_enabled,
        plan_periods_per_year,
        next_pay_on,
        plan_covers_next_pay,
    } = facts;
    let gaps = &status.gaps;
    let mut rows = Vec::new();
    let done_or_open = |code: &str| {
        if gap_open(gaps, code) {
            EstablishRowStatus::Open
        } else {
            EstablishRowStatus::Done
        }
    };
    rows.push(row(
        "template_dividend",
        "Template Dividend URL",
        done_or_open("template_dividend"),
        true,
    ));
    rows.push(row(
        "template_roc",
        "Template ROC URL",
        if !roc_in_scope {
            EstablishRowStatus::Na
        } else {
            done_or_open("template_roc")
        },
        roc_in_scope,
    ));
    rows.push(row(
        "div_type",
        "DIV-1 or CASH",
        done_or_open("div_type"),
        true,
    ));
    rows.push(row(
        "frequency",
        "Payment frequency",
        done_or_open("frequency"),
        true,
    ));
    rows.push(row(
        "provider",
        "Provider",
        done_or_open("provider"),
        true,
    ));
    rows.push(row(
        "underlying",
        "Underlying",
        done_or_open("underlying"),
        true,
    ));
    rows.push(row(
        "risk_tier",
        "Risk tier",
        done_or_open("risk_tier"),
        true,
    ));
    rows.push(row(
        "paid_history",
        "Paid history / inception",
        done_or_open("paid_history"),
        true,
    ));
    rows.push(row(
        "remaining_year",
        "Remaining-year pay dates",
        done_or_open("remaining_year"),
        true,
    ));
    rows.push(row(
        "roc_estimate",
        "ROC estimate confirmed",
        if !roc_in_scope {
            EstablishRowStatus::Na
        } else {
            done_or_open("roc_estimate")
        },
        roc_in_scope,
    ));
    rows.push(row(
        "plan",
        "Confirm Plan",
        if plan_known {
            EstablishRowStatus::Done
        } else {
            EstablishRowStatus::Open
        },
        true,
    ));
    rows.push(row(
        "calculator",
        "Calculator / Cart eligible",
        if calculator_eligible {
            EstablishRowStatus::Done
        } else {
            EstablishRowStatus::Open
        },
        true,
    ));
    rows.push(row(
        "last_price",
        "Last price (non-$0)",
        if price_ok {
            EstablishRowStatus::Done
        } else {
            EstablishRowStatus::Open
        },
        true,
    ));
    rows.push(row(
        "declaration_adapter",
        "Issuer declaration adapter",
        if !collectable {
            EstablishRowStatus::Na
        } else if crate::div1::is_registered_declaration_source(declaration_source) {
            EstablishRowStatus::Done
        } else {
            EstablishRowStatus::Open
        },
        collectable,
    ));
    rows.push(row(
        "collector_enabled",
        "Collector enabled (Run enabled collects it)",
        if !collectable {
            EstablishRowStatus::Na
        } else if collector_enabled {
            EstablishRowStatus::Done
        } else {
            EstablishRowStatus::Open
        },
        collectable,
    ));
    rows.push(row(
        "plan_periods",
        "Annual periods for the rate",
        if plan_periods_per_year > 0 {
            EstablishRowStatus::Done
        } else {
            EstablishRowStatus::Open
        },
        true,
    ));
    rows.push(row(
        "plan_window",
        "Plan window covers the next pay date",
        if next_pay_on.trim().is_empty() {
            EstablishRowStatus::Na
        } else if plan_covers_next_pay {
            EstablishRowStatus::Done
        } else {
            EstablishRowStatus::Open
        },
        !next_pay_on.trim().is_empty(),
    ));
    rows.push(row(
        "first_lot",
        "First lot (optional — unlocks Collectors, Income Plan $, managed count)",
        if has_open_lot {
            EstablishRowStatus::Done
        } else {
            EstablishRowStatus::Na
        },
        false,
    ));

    let open_labels: Vec<String> = rows
        .iter()
        .filter(|r| r.blocks_complete && r.status == EstablishRowStatus::Open)
        .map(|r| r.label.to_string())
        .collect();
    let complete = open_labels.is_empty();
    EstablishChecklist {
        rows,
        complete,
        open_labels,
    }
}

pub fn collector_is_complete(spec: &CollectorCompleteSpec<'_>) -> bool {
    collector_status(spec).complete
}

pub fn collector_status(spec: &CollectorCompleteSpec<'_>) -> CollectorCompleteStatus {
    let mut gaps = Vec::new();
    if !spec.has_dividend_template {
        gaps.push("template_dividend".into());
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
        if !spec.has_roc_template {
            gaps.push("template_roc".into());
        }
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
                return planned_u.abs_diff(issuer_u) <= 1;
            }
            // Equality made every name permanently incomplete whenever the issuer had
            // published fewer months ahead than the derived walk filled — a monthly payer
            // that posts one month at a time could never be complete. The real rules are
            // that no published date is missing, and that the walk has not invented more
            // pays than the year has left.
            let year_cap = remaining_periods_to_year_end(spec.as_of, periods).unwrap_or(periods);
            planned_u >= issuer_u && planned_u <= year_cap.max(issuer_u)
        }
    }
}

/// True when both lists have unoccurred dates through 31 Dec and the **sets** differ.
/// Prefer [`remaining_year_authoritative_disagree`] when derived fillers may remain.
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

/// True when **authoritative** (non-derived) stored future dates disagree with the
/// vendor list for the same cadence period through 31 Dec.
///
/// Leftover `derived_walk` / invented fillers for periods the vendor has not posted
/// must **not** be passed in `stored_authoritative` — they are not a ticket.
pub fn remaining_year_authoritative_disagree(
    stored_authoritative: &[&str],
    vendor: &[&str],
    as_of: &str,
    payment_frequency: &str,
) -> bool {
    let year = as_of.get(..4).unwrap_or("");
    if year.len() != 4 {
        return false;
    }
    let year_end = format!("{year}-12-31");
    let in_window = |on: &str| -> Option<String> {
        let p = on.trim();
        if p.len() >= 10 && p >= as_of && p <= year_end.as_str() {
            Some(p[..10].to_string())
        } else {
            None
        }
    };
    let stored: Vec<String> = stored_authoritative
        .iter()
        .filter_map(|on| in_window(on))
        .collect();
    let vendor: Vec<String> = vendor.iter().filter_map(|on| in_window(on)).collect();
    if vendor.is_empty() {
        return false;
    }
    for v in &vendor {
        let same: Vec<&String> = stored
            .iter()
            .filter(|s| {
                crate::schedule::vendor_payables_same_period(payment_frequency, s, v)
            })
            .collect();
        if same.is_empty() {
            return true;
        }
        if !same.iter().any(|s| s.as_str() == v.as_str()) {
            return true;
        }
    }
    for s in &stored {
        let covered = vendor.iter().any(|v| {
            crate::schedule::vendor_payables_same_period(payment_frequency, s, v)
        });
        if !covered {
            return true;
        }
    }
    false
}

/// Pay-date `source` values that are projections — overwritten when vendor posts.
pub fn is_derived_pay_source(source: &str) -> bool {
    crate::mlp_sec::is_invented_horizon_source(source)
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

/// Fallback search when the standing Template ROC URL fails.
/// Template ROC: `19.1 tax ROC (TICKER) (Provider) website data source`.
/// Template Dividend is the owner seed declaration URL — not this search.
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

/// Calculator / Home plan count list DIV-1 only. Cash stays on Home and Cash Management.
/// `None` cadence stays stored, not shown.
pub fn calculator_view_includes(div_type: &str, _symbol: &str, payment_frequency: &str) -> bool {
    if crate::calculator::is_non_paying(payment_frequency) {
        return false;
    }
    is_div1(div_type)
}

/// A held position or a stored Plan stays on the Calculator sheet unless the cadence is None.
/// Home plan count still uses `calculator_view_includes`.
pub fn calculator_row_listed(payment_frequency: &str) -> bool {
    !crate::calculator::is_non_paying(payment_frequency)
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
        ("template_dividend", "Template Dividend"),
        ("template_roc", "Template ROC"),
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

/// Locked `ExpectedPaymentPattern.declaration_weekday` matches `today` (Monday / Mon).
pub fn is_declaration_weekday_today(today: &str, weekday: &str) -> bool {
    let raw = weekday.trim();
    if raw.is_empty() {
        return false;
    }
    let Ok(d) = chrono::NaiveDate::parse_from_str(today.trim(), "%Y-%m-%d") else {
        return false;
    };
    let full = d.format("%A").to_string();
    let short = d.format("%a").to_string();
    raw.eq_ignore_ascii_case(&full) || raw.eq_ignore_ascii_case(&short)
}

/// R5 same-day skip. Blocked on the locked declaration weekday so a later
/// issuer post is fetched. Blank weekday keeps the skip.
pub fn same_day_retrieve_skip_allowed(
    last_run_ok: Option<bool>,
    last_run_at: &str,
    today: &str,
    declaration_weekday: &str,
) -> bool {
    declaration_daily_retrieve_current(last_run_ok, last_run_at, today)
        && !is_declaration_weekday_today(today, declaration_weekday)
}

/// After the fleet run: failed names (not current today) must each have an
/// open retrieve-failure ticket. Extra ROC / amount tickets do not count.
pub fn declaration_fail_ticket_parity(
    failed_symbols: impl IntoIterator<Item = impl AsRef<str>>,
    open_miss_ticket_symbols: impl IntoIterator<Item = impl AsRef<str>>,
) -> bool {
    use std::collections::BTreeSet;
    let fails: BTreeSet<String> = failed_symbols
        .into_iter()
        .map(|s| s.as_ref().trim().to_ascii_uppercase())
        .filter(|s| !s.is_empty())
        .collect();
    let tickets: BTreeSet<String> = open_miss_ticket_symbols
        .into_iter()
        .map(|s| s.as_ref().trim().to_ascii_uppercase())
        .filter(|s| !s.is_empty())
        .collect();
    fails == tickets
}

/// Unoccurred stored pay dates through 31 Dec. Unique days only — duplicate
/// rows (same payable stored twice) must not fail the remaining-year gate.
pub fn stored_remaining_planned(pay_ons: &[&str], as_of: &str) -> i64 {
    let year = as_of.get(..4).unwrap_or("");
    if year.len() != 4 {
        return 0;
    }
    let year_end = format!("{year}-12-31");
    let mut seen = std::collections::BTreeSet::new();
    for on in pay_ons {
        let p = on.trim();
        if p.len() >= 10 && p >= as_of && p <= year_end.as_str() {
            seen.insert(&p[..10]);
        }
    }
    seen.len() as i64
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
            has_dividend_template: true,
            has_roc_template: true,
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
    fn establish_requires_both_retrieve_templates() {
        let mut no_div = base(&[]);
        no_div.has_dividend_template = false;
        let d = collector_status(&no_div);
        assert!(!d.complete);
        assert!(d.gaps.iter().any(|g| g == "template_dividend"));

        let mut no_roc = base(&[]);
        no_roc.has_roc_template = false;
        let r = collector_status(&no_roc);
        assert!(!r.complete);
        assert!(r.gaps.iter().any(|g| g == "template_roc"));

        let mut cash = base(&[]);
        cash.symbol = "SPAXX";
        cash.div_type = "CASH";
        cash.has_roc_template = false;
        cash.payment_frequency = "";
        cash.underlying = "";
        cash.provider = "";
        cash.risk_tier = "";
        cash.roc_pct_minor = None;
        cash.roc_owner_accepted = false;
        cash.planned_remaining = None;
        assert!(collector_is_complete(&cash));
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
        // Derived fillers + vendor subset must not ticket when authoritative matches.
        assert!(!remaining_year_authoritative_disagree(
            &["2026-10-15"],
            &["2026-10-15"],
            "2026-09-06",
            "Monthly",
        ));
        assert!(!remaining_year_authoritative_disagree(
            &["2026-10-15"],
            &["2026-10-15"],
            "2026-09-06",
            "Monthly",
        ));
        // Same month, different day → authoritative conflict.
        assert!(remaining_year_authoritative_disagree(
            &["2026-10-01"],
            &["2026-10-15"],
            "2026-09-06",
            "Monthly",
        ));
        // Extra authoritative date vendor did not list.
        assert!(remaining_year_authoritative_disagree(
            &["2026-10-15", "2026-11-15"],
            &["2026-10-15"],
            "2026-09-06",
            "Monthly",
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
        assert_eq!(roc_scope("ET", "DIV-1", "", ""), RocScope::NotInScope);
        assert_eq!(roc_scope("EPD", "DIV-1", "", ""), RocScope::NotInScope);
        assert_eq!(roc_scope("MPLX", "DIV-1", "", ""), RocScope::NotInScope);
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
            has_dividend_template: true,
            has_roc_template: false,
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

    /// A fully wired payer: every readiness field satisfied. Tests below break one field
    /// at a time so each MUST-satisfy step is pinned to exactly one failure.
    fn wired<'a>() -> ReadinessFacts<'a> {
        ReadinessFacts {
            plan_known: true,
            calculator_eligible: true,
            price_ok: true,
            has_open_lot: true,
            roc_in_scope: false,
            collectable: true,
            declaration_source: "amplify",
            collector_enabled: true,
            plan_periods_per_year: 24,
            next_pay_on: "2026-10-16",
            plan_covers_next_pay: true,
        }
    }

    fn row_status(list: &EstablishChecklist, id: &str) -> EstablishRowStatus {
        list.rows
            .iter()
            .find(|r| r.id == id)
            .unwrap_or_else(|| panic!("no readiness row {id}"))
            .status
    }

    /// BITO's live shape: ProShares had posted only October, so the walk supplied Nov and
    /// Dec. Demanding stored == published made that name incomplete forever.
    #[test]
    fn derived_fillers_past_the_published_horizon_are_not_a_remaining_year_gap() {
        let mut spec = base(&[]);
        spec.as_of = "2026-10-03";
        spec.payment_frequency = "Monthly";
        spec.planned_remaining = Some(3);
        spec.issuer_remaining = Some(1);
        assert!(
            !collector_status(&spec).gaps.iter().any(|g| g == "remaining_year"),
            "one published month plus two fallbacks is the normal shape"
        );

        // A published date that is missing from the stored series is still a gap.
        spec.planned_remaining = Some(1);
        spec.issuer_remaining = Some(3);
        assert!(
            collector_status(&spec).gaps.iter().any(|g| g == "remaining_year"),
            "stored must cover every date the issuer published"
        );

        // And the walk may not invent more pays than the year has left.
        spec.planned_remaining = Some(9);
        spec.issuer_remaining = Some(1);
        assert!(
            collector_status(&spec).gaps.iter().any(|g| g == "remaining_year"),
            "9 remaining monthly pays cannot fit between October and December"
        );
    }

    #[test]
    fn a_fully_wired_payer_has_no_open_readiness_rows() {
        let ok = CollectorCompleteStatus {
            complete: true,
            gaps: Vec::new(),
        };
        let list = establish_checklist(&ok, wired());
        assert!(
            list.complete,
            "every field satisfied must be Complete, open: {:?}",
            list.open_labels
        );
    }

    /// The silent killer: a blank or unregistered adapter means no declaration is ever
    /// retrieved, so the name looks established and never pays history.
    #[test]
    fn missing_declaration_adapter_blocks_complete() {
        let ok = CollectorCompleteStatus {
            complete: true,
            gaps: Vec::new(),
        };
        let mut facts = wired();
        facts.declaration_source = "";
        let blank = establish_checklist(&ok, facts);
        assert_eq!(row_status(&blank, "declaration_adapter"), EstablishRowStatus::Open);
        assert!(!blank.complete);

        facts.declaration_source = "a-provider-we-never-registered";
        let unknown = establish_checklist(&ok, facts);
        assert_eq!(
            row_status(&unknown, "declaration_adapter"),
            EstablishRowStatus::Open,
            "an unregistered slug is as dead as a blank one"
        );

        // Cash and holdings-only names are never collected: N/A, not open forever.
        facts.collectable = false;
        facts.declaration_source = "";
        facts.collector_enabled = false;
        let cash = establish_checklist(&ok, facts);
        assert_eq!(row_status(&cash, "declaration_adapter"), EstablishRowStatus::Na);
        assert_eq!(row_status(&cash, "collector_enabled"), EstablishRowStatus::Na);
        assert!(cash.complete);
    }

    #[test]
    fn collector_not_enabled_blocks_complete() {
        let ok = CollectorCompleteStatus {
            complete: true,
            gaps: Vec::new(),
        };
        let mut facts = wired();
        facts.collector_enabled = false;
        let list = establish_checklist(&ok, facts);
        assert_eq!(row_status(&list, "collector_enabled"), EstablishRowStatus::Open);
        assert!(!list.complete);
    }

    /// MUIB shipped a cart rate built on the wrong period count. Zero periods means no
    /// rate can be formed at all, so it is a blocking gap, not a display detail.
    #[test]
    fn zero_annual_periods_blocks_complete() {
        let ok = CollectorCompleteStatus {
            complete: true,
            gaps: Vec::new(),
        };
        let mut facts = wired();
        facts.plan_periods_per_year = 0;
        let list = establish_checklist(&ok, facts);
        assert_eq!(row_status(&list, "plan_periods"), EstablishRowStatus::Open);
        assert!(!list.complete);
    }

    /// HAKY's symptom: a Plan that starts after the pay date leaves the week grid with no
    /// Plan $/sh while every establish field reads done.
    #[test]
    fn plan_that_misses_the_next_pay_blocks_complete() {
        let ok = CollectorCompleteStatus {
            complete: true,
            gaps: Vec::new(),
        };
        let mut facts = wired();
        facts.plan_covers_next_pay = false;
        let list = establish_checklist(&ok, facts);
        assert_eq!(row_status(&list, "plan_window"), EstablishRowStatus::Open);
        assert!(!list.complete);

        // No published pay date yet is unknown, not a failure.
        facts.next_pay_on = "";
        let unknown = establish_checklist(&ok, facts);
        assert_eq!(row_status(&unknown, "plan_window"), EstablishRowStatus::Na);
        assert!(unknown.complete);
    }

    /// The owner's rule: a researched name with zero lots is valid.
    #[test]
    fn no_lot_is_still_complete() {
        let ok = CollectorCompleteStatus {
            complete: true,
            gaps: Vec::new(),
        };
        let mut facts = wired();
        facts.has_open_lot = false;
        let list = establish_checklist(&ok, facts);
        assert_eq!(row_status(&list, "first_lot"), EstablishRowStatus::Na);
        assert!(list.complete, "no lot required to be established");
    }

    #[test]
    fn income_fleet_is_forty_and_excludes_parked_long_hold() {
        assert_eq!(INCOME_FLEET_SYMBOLS.len(), 40);
        assert_eq!(PARKED_LONG_HOLD_SYMBOLS.len(), 3);
        assert!(INCOME_FLEET_SYMBOLS.windows(2).all(|w| w[0] < w[1]));
        assert!(is_not_a_collector("MSTU"));
        assert!(is_not_a_collector("tsll"));
        assert!(is_not_a_collector("SOXL"));
        assert!(!is_not_a_collector("JEPQ"));
        for symbol in PARKED_LONG_HOLD_SYMBOLS {
            assert!(!INCOME_FLEET_SYMBOLS.contains(symbol), "{symbol}");
        }
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
    fn declaration_weekday_blocks_same_day_skip() {
        assert!(is_declaration_weekday_today("2026-09-14", "Monday"));
        assert!(is_declaration_weekday_today("2026-09-14", "Mon"));
        assert!(!is_declaration_weekday_today("2026-09-14", "Wednesday"));
        assert!(!is_declaration_weekday_today("2026-09-14", ""));
        assert!(same_day_retrieve_skip_allowed(
            Some(true),
            "2026-09-14T07:07:01",
            "2026-09-14",
            "",
        ));
        assert!(!same_day_retrieve_skip_allowed(
            Some(true),
            "2026-09-14T07:07:01",
            "2026-09-14",
            "Monday",
        ));
    }

    #[test]
    fn fail_count_equals_miss_ticket_symbols() {
        assert!(declaration_fail_ticket_parity(["EFC"], ["EFC"]));
        assert!(declaration_fail_ticket_parity(
            std::iter::empty::<&str>(),
            std::iter::empty::<&str>(),
        ));
        assert!(!declaration_fail_ticket_parity(["EFC"], std::iter::empty::<&str>()));
        assert!(!declaration_fail_ticket_parity(["EFC"], ["QYLD"]));
        assert!(declaration_fail_ticket_parity(["EFC", "TOPW"], ["topw", "efc"]));
    }

    #[test]
    fn stored_remaining_planned_counts_unoccurred_through_year_end() {
        let dates = ["2026-08-31", "2026-09-30", "2026-12-31", "2027-01-31"];
        let refs: Vec<&str> = dates.iter().copied().collect();
        assert_eq!(stored_remaining_planned(&refs, "2026-09-05"), 2);
        assert_eq!(stored_remaining_planned(&refs, "2026-08-22"), 3);
        let dupes = ["2026-11-19", "2026-11-19", "2026-11-18"];
        let dupe_refs: Vec<&str> = dupes.iter().copied().collect();
        assert_eq!(stored_remaining_planned(&dupe_refs, "2026-09-14"), 2);
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
        assert!(
            !calculator_view_includes("CASH", "SPAXX", "Monthly"),
            "cash stays off the Calculator"
        );
        assert!(!calculator_view_includes("", "BTC-USD", ""));
        assert!(!calculator_view_includes("", "SOXL", "None"));
        assert!(!calculator_view_includes("DIV-1", "MSTU", "None"));
        assert!(calculator_row_listed("Monthly"));
        assert!(calculator_row_listed(""));
        assert!(!calculator_row_listed("None"));
    }
}
