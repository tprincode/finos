//! Cash Management posting identity. Net is calculated; it is not a third stored amount.

use chrono::{Datelike, Duration, NaiveDate, Weekday};

use crate::error::DomainError;

/// Household Social Security retirement expected each month (owner lock). Never SSI.
pub const BARBARA_SSA_EXPECTED_MINOR: i64 = 133_100;
pub const TOM_SSA_EXPECTED_MINOR: i64 = 286_500;
pub const SSA_RETIREMENT_LABEL: &str = "Social Security retirement";
pub const SSA_VARIANCE_CODE: &str = "ssa_amount_variance";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SsaPayee {
    Barbara,
    Tom,
}

impl SsaPayee {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Barbara => "barbara",
            Self::Tom => "tom",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Barbara => "Barbara",
            Self::Tom => "Tom",
        }
    }

    pub fn expected_minor(self) -> i64 {
        match self {
            Self::Barbara => BARBARA_SSA_EXPECTED_MINOR,
            Self::Tom => TOM_SSA_EXPECTED_MINOR,
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "barbara" => Some(Self::Barbara),
            "tom" => Some(Self::Tom),
            _ => None,
        }
    }

    /// Element notes are "SSA Tom Deposit" / "SSA Barbara Deposit".
    pub fn from_element_note(note: &str) -> Option<Self> {
        let n = note.to_ascii_lowercase();
        if n.contains("barbara") {
            Some(Self::Barbara)
        } else if n.contains("tom") {
            Some(Self::Tom)
        } else {
            None
        }
    }

    pub fn all() -> [Self; 2] {
        [Self::Barbara, Self::Tom]
    }
}

/// Types Cash Management may post.
pub fn is_cash_distribution_type(activity_type: &str) -> bool {
    matches!(
        activity_type,
        "IRA_Distribution" | "Roth_Distribution" | "SSA" | "Withdrawal" | "HSA_Withdrawal"
    )
}

pub fn tom_ssa_idempotency_key(year: i32, month: u32) -> String {
    ssa_idempotency_key(SsaPayee::Tom, year, month)
}

pub fn ssa_idempotency_key(payee: SsaPayee, year: i32, month: u32) -> String {
    format!("ssa-{}-{year:04}-{month:02}", payee.as_str())
}

pub fn is_tom_ssa_amount(amount_minor: i64) -> bool {
    amount_minor == TOM_SSA_EXPECTED_MINOR
}

pub fn is_barbara_ssa_amount(amount_minor: i64) -> bool {
    amount_minor == BARBARA_SSA_EXPECTED_MINOR
}

/// Key prefix wins; otherwise exact expected amount. Seed keys like `seed-tom-june-5` classify by amount.
pub fn classify_ssa_row(idempotency_key: &str, amount_minor: i64) -> Option<SsaPayee> {
    let key = idempotency_key.to_ascii_lowercase();
    if key.starts_with("ssa-barbara-") {
        return Some(SsaPayee::Barbara);
    }
    if key.starts_with("ssa-tom-") {
        return Some(SsaPayee::Tom);
    }
    if is_barbara_ssa_amount(amount_minor) {
        return Some(SsaPayee::Barbara);
    }
    if is_tom_ssa_amount(amount_minor) {
        return Some(SsaPayee::Tom);
    }
    None
}

pub fn is_income_account_name(name: &str) -> bool {
    name.trim().eq_ignore_ascii_case("income")
}

pub fn is_ssa_account_name(name: &str) -> bool {
    name.trim().eq_ignore_ascii_case("external")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TomSsaStatus {
    Unconfirmed,
    Confirmed,
    Variance,
}

impl TomSsaStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unconfirmed => "unconfirmed",
            Self::Confirmed => "confirmed",
            Self::Variance => "variance",
        }
    }
}

/// Month status from expected-amount hits plus the optional `ssa-tom-YYYY-MM` confirm row.
/// A miss stays unconfirmed with no posted amount (never $0). Extra expected rows are audit.
pub fn tom_ssa_month_status(
    month_expected_count: usize,
    confirm_amount: Option<i64>,
) -> (TomSsaStatus, bool, Option<i64>) {
    ssa_payee_month_status(SsaPayee::Tom, month_expected_count, confirm_amount)
}

/// Per-payee month status. Two expected Tom rows in a month (June 5 and 26) are
/// locked facts. Extra is a third row for that payee, not the second.
pub fn ssa_payee_month_status(
    payee: SsaPayee,
    month_row_count: usize,
    confirm_amount: Option<i64>,
) -> (TomSsaStatus, bool, Option<i64>) {
    let extra_audit = month_row_count >= 3;
    let expected = payee.expected_minor();
    if month_row_count > 0 {
        let posted = confirm_amount
            .or_else(|| (month_row_count > 0).then_some(expected));
        if let Some(amount) = confirm_amount {
            if amount != expected {
                return (TomSsaStatus::Variance, extra_audit, Some(amount));
            }
        }
        return (TomSsaStatus::Confirmed, extra_audit, posted);
    }
    match confirm_amount {
        Some(amount) if amount == expected => (TomSsaStatus::Confirmed, false, Some(amount)),
        Some(amount) => (TomSsaStatus::Variance, false, Some(amount)),
        None => (TomSsaStatus::Unconfirmed, false, None),
    }
}

/// Household extra-audit: a fourth row (June already has two Tom + one Barbara),
/// a third row for one payee, or an unclassified amount.
pub fn ssa_household_extra_audit(month_ssa_count: usize, duplicate_payee: bool, unclassified: bool) -> bool {
    month_ssa_count > 3 || duplicate_payee || unclassified
}

/// Saturday planned Income IRA draft stays open until one Income IRA posts that week.
pub fn saturday_draft_open(income_ira_posted_this_week: bool) -> bool {
    !income_ira_posted_this_week
}

/// Taxable gross this post would add to household MAGI. None = unknown / not this ticket.
/// Withholding is not an input (G-MAGI-06).
/// How a posted cash line is treated for tax. Amounts stay unknown until the owner fact is known.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CashTaxSection {
    IraOrdinary,
    Roth,
    Ssa,
}

impl CashTaxSection {
    pub fn id(self) -> &'static str {
        match self {
            Self::IraOrdinary => "ira",
            Self::Roth => "roth",
            Self::Ssa => "ssa",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::IraOrdinary => "IRA ordinary",
            Self::Roth => "Roth",
            Self::Ssa => "Social Security retirement",
        }
    }

    pub fn tax_note(self) -> &'static str {
        match self {
            Self::IraOrdinary => {
                "Income and Speculation IRA withdrawals are one ordinary income type."
            }
            Self::Roth => "Roth is not Marketplace MAGI.",
            Self::Ssa => "Social Security retirement. MAGI treatment stays unknown.",
        }
    }
}

/// Current-year cash-out groups only. Prior-year 1099 and Car withdrawals have no card —
/// Car character is ROC / ordinary / long-short on the Car ROC plan.
pub fn cash_tax_section(activity_type: &str, account_kind: &str) -> Option<CashTaxSection> {
    if crate::trends::is_prior_year_1099(activity_type) {
        return None;
    }
    let kind = account_kind.trim().to_ascii_lowercase();
    if activity_type == "SSA" {
        return Some(CashTaxSection::Ssa);
    }
    if activity_type == "Roth_Distribution" || kind == "roth" || kind == "fi_roth" {
        return Some(CashTaxSection::Roth);
    }
    if activity_type == "IRA_Distribution" || kind == "ira" {
        return Some(CashTaxSection::IraOrdinary);
    }
    None
}

pub fn magi_add_minor(activity_type: &str, gross_minor: Option<i64>) -> Option<i64> {
    match activity_type {
        "IRA_Distribution" => gross_minor,
        "Roth_Distribution" => Some(0),
        "SSA" => None,
        _ => None,
    }
}

/// Owner 2026 1099 job. Received total is MAGI-included ordinary earned.
pub const JOB_1099_2026_SOURCE: &str = "owner-1099-job-2026";
pub const JOB_1099_2026_MINOR: i64 = 1_462_500;

/// YTD / remaining for the 1099 job on this calendar year. 2026 remaining is 0.
pub fn job_1099_year_amounts(as_of: &str) -> (i64, i64) {
    if as_of.starts_with("2026") {
        (JOB_1099_2026_MINOR, 0)
    } else {
        (0, 0)
    }
}

pub fn tax_payment_credit_minor(federal_minor: i64, state_minor: i64) -> i64 {
    federal_minor + state_minor
}

/// Calendar month of an event date (TR-AC-06). Not a sum of Sat–Fri week cells.
/// Sat–Fri week, or SSA_2026 dated from Saturday minus 4 calendar days through Friday.
pub fn week_ahead_in_window(
    account: &str,
    occurred_on: &str,
    week_start: &str,
    week_end: &str,
) -> bool {
    if crate::income_plan::occurred_in_week(occurred_on, week_start, week_end) {
        return true;
    }
    if !account.eq_ignore_ascii_case("SSA_2026") {
        return false;
    }
    let Some(on) = crate::trends::parse_iso_date(occurred_on) else {
        return false;
    };
    let Some(sat) = crate::trends::parse_iso_date(week_start) else {
        return false;
    };
    let Some(fri) = crate::trends::parse_iso_date(week_end) else {
        return false;
    };
    let early = sat - Duration::days(4);
    on >= early && on <= fri
}

pub fn occurred_in_calendar_month(occurred_on: &str, year: i32, month: u32) -> bool {
    let Some(date) = crate::trends::parse_iso_date(occurred_on) else {
        return false;
    };
    date.year() == year && date.month() == month
}

pub fn net_minor(gross_minor: i64, federal_minor: i64, state_minor: i64) -> i64 {
    gross_minor - federal_minor - state_minor
}

/// Live seed kinds (13 Sep 2026): Income/Speculation/9 = ira; FI Roth = fi_roth;
/// Car/Robinhood/ENERGYX = taxable; Health = hsa; External = taxable (SSA by name).
pub fn cash_activity_allowed_for_account(
    activity_type: &str,
    account_name: &str,
    account_kind: &str,
) -> Result<(), DomainError> {
    if !is_cash_distribution_type(activity_type) {
        return Err(DomainError::CashDistributionType);
    }
    let kind = account_kind.trim().to_ascii_lowercase();
    let allowed = match activity_type {
        "SSA" => is_ssa_account_name(account_name),
        "IRA_Distribution" => kind == "ira",
        "Roth_Distribution" => kind == "roth" || kind == "fi_roth",
        "Withdrawal" => kind == "taxable" && !is_ssa_account_name(account_name),
        "HSA_Withdrawal" => kind == "hsa" && account_name.trim().eq_ignore_ascii_case("health"),
        _ => false,
    };
    if allowed {
        Ok(())
    } else {
        Err(DomainError::CashAccountKind)
    }
}

/// Gross must be known and positive. Withholding cannot exceed gross.
/// Roth refuses any withholding. Net = gross − fed − state.
pub fn validate_cash_distribution(
    activity_type: &str,
    gross_minor: Option<i64>,
    federal_minor: i64,
    state_minor: i64,
) -> Result<i64, DomainError> {
    if !is_cash_distribution_type(activity_type) {
        return Err(DomainError::CashDistributionType);
    }
    let Some(gross) = gross_minor else {
        return Err(DomainError::UnknownAmount);
    };
    if gross <= 0 {
        return Err(DomainError::CashDistributionIdentity);
    }
    if federal_minor < 0 || state_minor < 0 {
        return Err(DomainError::CashDistributionIdentity);
    }
    if activity_type == "Roth_Distribution" && (federal_minor != 0 || state_minor != 0) {
        return Err(DomainError::RothWithholdingNotAllowed);
    }
    let net = net_minor(gross, federal_minor, state_minor);
    if net < 0 {
        return Err(DomainError::CashDistributionIdentity);
    }
    Ok(net)
}

/// Capture accounts that may post `Cash_Adjust` (UI shows Account 9 for name `"9"`).
pub const CASH_ADJUST_ACCOUNT_NAMES: &[&str] =
    &["Income", "FI Roth", "Speculation", "Health", "Car", "9"];

pub fn is_cash_adjust_type(activity_type: &str) -> bool {
    activity_type == "Cash_Adjust"
}

pub fn is_cash_adjust_account(account_name: &str) -> bool {
    CASH_ADJUST_ACCOUNT_NAMES
        .iter()
        .any(|n| account_name.trim().eq_ignore_ascii_case(n))
}

/// Default money-market symbol when `account.cash_symbol` is empty.
pub fn default_cash_symbol(account_name: &str) -> Option<&'static str> {
    match account_name.trim() {
        "Income" | "FI Roth" | "Speculation" | "Car" => Some("SPAXX"),
        "Health" => Some("FDRXX"),
        "9" => Some("SWVXX"),
        _ => None,
    }
}

pub fn resolve_cash_symbol(account_name: &str, column: Option<&str>) -> Option<String> {
    if let Some(raw) = column.map(str::trim).filter(|s| !s.is_empty()) {
        return Some(raw.to_string());
    }
    default_cash_symbol(account_name).map(str::to_string)
}

pub fn cash_adjust_gap(typed_minor: i64, reference_minor: i64) -> i64 {
    typed_minor - reference_minor
}

pub fn cash_gap_is_material(gap_minor: i64) -> bool {
    gap_minor.abs() >= 1
}

pub fn cash_adjust_idempotency_key(account_id: uuid::Uuid, period_end: &str) -> String {
    format!("cash-adjust-{account_id}-{period_end}")
}

/// Reason note: `fee` | `split` | `other`, or `fee: detail`.
pub fn format_cash_adjust_note(reason_raw: &str) -> Result<String, DomainError> {
    let raw = reason_raw.trim();
    if raw.is_empty() {
        return Err(DomainError::CashAdjustReasonRequired);
    }
    let (kind, detail) = match raw.split_once(':') {
        Some((k, rest)) => (k.trim(), Some(rest.trim())),
        None => (raw, None),
    };
    let kind_l = kind.to_ascii_lowercase();
    if !matches!(kind_l.as_str(), "fee" | "split" | "other") {
        return Err(DomainError::CashAdjustReasonRequired);
    }
    match detail {
        Some(d) if !d.is_empty() => Ok(format!("{kind_l}: {d}")),
        _ => Ok(kind_l),
    }
}

/// Validate a Cash_Adjust post. Negatives allowed; withholding refused; not distribution rules.
pub fn validate_cash_adjust(
    activity_type: &str,
    account_name: &str,
    amount_minor: Option<i64>,
    federal_withholding_minor: i64,
    state_withholding_minor: i64,
    reason_raw: &str,
) -> Result<(i64, String), DomainError> {
    if !is_cash_adjust_type(activity_type) {
        return Err(DomainError::CashDistributionType);
    }
    if !is_cash_adjust_account(account_name) {
        return Err(DomainError::CashAdjustAccount);
    }
    if federal_withholding_minor != 0 || state_withholding_minor != 0 {
        return Err(DomainError::CashAdjustWithholdingNotAllowed);
    }
    let Some(amount) = amount_minor else {
        return Err(DomainError::UnknownAmount);
    };
    if amount == 0 {
        return Err(DomainError::CashAdjustAmount);
    }
    let note = format_cash_adjust_note(reason_raw)?;
    Ok((amount, note))
}

/// Register switcher books. Speculation is capture-only.
pub const REGISTER_BOOKS: &[&str] = &[
    "Income",
    "FI Roth",
    "Health",
    "Car",
    "Account 9",
    "SSA_2026",
];

pub fn register_book_label(raw: &str) -> Option<&'static str> {
    match raw.trim() {
        "Income" => Some("Income"),
        "FI Roth" | "Roth" => Some("FI Roth"),
        "Health" => Some("Health"),
        "Car" => Some("Car"),
        "Account 9" | "9" => Some("Account 9"),
        "SSA_2026" => Some("SSA_2026"),
        _ => None,
    }
}

/// Income net / fed / state history reads seed IRA parents. Week-ahead
/// slices are already on confirmed occurrences — skip those keys.
/// Parent `amount_minor` is gross; net is gross − federal − state.
pub fn income_element_activity_slice(
    note: &str,
    activity_type: &str,
    amount_minor: i64,
    federal_withholding_minor: i64,
    state_withholding_minor: i64,
    idempotency_key: &str,
) -> Option<i64> {
    if !activity_type.eq_ignore_ascii_case("IRA_Distribution") {
        return None;
    }
    if idempotency_key.starts_with("week-ahead-") {
        return None;
    }
    match note.trim().to_ascii_lowercase().as_str() {
        "fed" | "federal" => (federal_withholding_minor > 0).then_some(federal_withholding_minor),
        "state" => (state_withholding_minor > 0).then_some(state_withholding_minor),
        "net" => {
            let net = amount_minor - federal_withholding_minor - state_withholding_minor;
            (net > 0).then_some(net)
        }
        _ => None,
    }
}

/// Posted ledger slice for Element history. Week-ahead keys stay on
/// confirmed occs. Unconfirmed leftover plans are not this function.
pub fn element_history_posted_slice(
    account: &str,
    note: &str,
    activity_type: &str,
    amount_minor: i64,
    federal_withholding_minor: i64,
    state_withholding_minor: i64,
    idempotency_key: &str,
) -> Option<i64> {
    if idempotency_key.starts_with("week-ahead-") {
        return None;
    }
    match register_book_label(account) {
        Some("Income") => income_element_activity_slice(
            note,
            activity_type,
            amount_minor,
            federal_withholding_minor,
            state_withholding_minor,
            idempotency_key,
        ),
        Some("Car") => activity_type
            .eq_ignore_ascii_case("Withdrawal")
            .then_some(amount_minor.abs()),
        Some("SSA_2026") => {
            if !activity_type.eq_ignore_ascii_case("SSA") {
                return None;
            }
            let payee = SsaPayee::from_element_note(note)?;
            (amount_minor.abs() == payee.expected_minor()).then_some(amount_minor.abs())
        }
        Some("Health") => activity_type
            .eq_ignore_ascii_case("HSA_Withdrawal")
            .then_some(amount_minor.abs()),
        Some("FI Roth") => activity_type
            .eq_ignore_ascii_case("Roth_Distribution")
            .then_some(amount_minor.abs()),
        _ => None,
    }
}

pub fn register_ledger_account(book: &str) -> &'static str {
    match register_book_label(book).unwrap_or("") {
        "SSA_2026" => "External",
        "Account 9" => "9",
        "FI Roth" => "FI Roth",
        "Health" => "Health",
        "Car" => "Car",
        _ => "Income",
    }
}

pub fn register_period_bounds(
    period: &str,
    as_of: NaiveDate,
) -> Result<(NaiveDate, NaiveDate), DomainError> {
    match period.trim() {
        "2W" => {
            let week = crate::week::week_containing(as_of);
            Ok((week.start - Duration::days(7), week.end + Duration::days(7)))
        }
        "1M" => {
            let start = NaiveDate::from_ymd_opt(as_of.year(), as_of.month(), 1)
                .ok_or(DomainError::UnknownAmount)?;
            let end = last_of_month(as_of.year(), as_of.month()).ok_or(DomainError::UnknownAmount)?;
            Ok((start, end))
        }
        "3M" => {
            let end = last_of_month(as_of.year(), as_of.month()).ok_or(DomainError::UnknownAmount)?;
            let start_month = as_of
                .checked_sub_months(chrono::Months::new(2))
                .ok_or(DomainError::UnknownAmount)?;
            let start = NaiveDate::from_ymd_opt(start_month.year(), start_month.month(), 1)
                .ok_or(DomainError::UnknownAmount)?;
            Ok((start, end))
        }
        "1Y" => {
            let start = NaiveDate::from_ymd_opt(as_of.year(), 1, 1).ok_or(DomainError::UnknownAmount)?;
            let end = NaiveDate::from_ymd_opt(as_of.year(), 12, 31).ok_or(DomainError::UnknownAmount)?;
            Ok((start, end))
        }
        _ => Err(DomainError::UnknownAmount),
    }
}

/// Lookback window for Element history. `all` is unbounded. Month chips are
/// calendar months back from as-of through as-of, not the Register 1M month.
pub fn element_history_period_bounds(
    duration: &str,
    as_of: NaiveDate,
) -> Result<(Option<NaiveDate>, Option<NaiveDate>), DomainError> {
    let key = duration.trim().to_ascii_lowercase();
    let key = key
        .replace(" months", "m")
        .replace(" month", "m")
        .replace(' ', "");
    match key.as_str() {
        "all" => Ok((None, None)),
        "ytd" => {
            let start = NaiveDate::from_ymd_opt(as_of.year(), 1, 1).ok_or(DomainError::UnknownAmount)?;
            Ok((Some(start), Some(as_of)))
        }
        "1m" => {
            let start = as_of
                .checked_sub_months(chrono::Months::new(1))
                .ok_or(DomainError::UnknownAmount)?;
            Ok((Some(start), Some(as_of)))
        }
        "3m" => {
            let start = as_of
                .checked_sub_months(chrono::Months::new(3))
                .ok_or(DomainError::UnknownAmount)?;
            Ok((Some(start), Some(as_of)))
        }
        "6m" => {
            let start = as_of
                .checked_sub_months(chrono::Months::new(6))
                .ok_or(DomainError::UnknownAmount)?;
            Ok((Some(start), Some(as_of)))
        }
        _ => Err(DomainError::UnknownAmount),
    }
}

/// Stop date before as-of is retired. Empty stop is current. Stop on as-of is still current.
pub fn element_is_retired(stop_on: &str, as_of: NaiveDate) -> bool {
    crate::trends::parse_iso_date(stop_on.trim()).is_some_and(|stop| stop < as_of)
}

fn last_of_month(year: i32, month: u32) -> Option<NaiveDate> {
    let first = NaiveDate::from_ymd_opt(year, month, 1)?;
    if month == 12 {
        NaiveDate::from_ymd_opt(year, 12, 31)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1).map(|d| d - Duration::days(1))
    }
    .or_else(|| Some(first))
}

fn parse_weekday_token(raw: &str) -> Weekday {
    match raw.trim().to_ascii_lowercase().as_str() {
        "mon" | "monday" => Weekday::Mon,
        "tue" | "tuesday" => Weekday::Tue,
        "wed" | "wednesday" => Weekday::Wed,
        "thu" | "thursday" => Weekday::Thu,
        "fri" | "friday" => Weekday::Fri,
        "sun" | "sunday" => Weekday::Sun,
        _ => Weekday::Sat,
    }
}

fn clamp_month_day(year: i32, month: u32, day: u32) -> Option<NaiveDate> {
    NaiveDate::from_ymd_opt(year, month, day)
        .or_else(|| last_of_month(year, month))
}

/// Cadence dates inside `[start, end]` for horizon fill. One-time yields none.
pub fn element_horizon_dates(
    cadence: &str,
    weekday_or_month_day: &str,
    start: NaiveDate,
    end: NaiveDate,
) -> Vec<String> {
    let mut out = Vec::new();
    match cadence.trim().to_ascii_lowercase().as_str() {
        "weekly" => {
            let want = parse_weekday_token(weekday_or_month_day);
            let mut d = start;
            while d <= end {
                if d.weekday() == want {
                    out.push(d.format("%Y-%m-%d").to_string());
                }
                d += Duration::days(1);
            }
        }
        "monthly" => {
            let day = weekday_or_month_day.trim().parse::<u32>().unwrap_or(1);
            let mut y = start.year();
            let mut m = start.month();
            loop {
                if let Some(on) = clamp_month_day(y, m, day) {
                    if on >= start && on <= end {
                        out.push(on.format("%Y-%m-%d").to_string());
                    }
                }
                if y > end.year() || (y == end.year() && m >= end.month()) {
                    break;
                }
                if m == 12 {
                    y += 1;
                    m = 1;
                } else {
                    m += 1;
                }
            }
        }
        "annual" => {
            let (month, day) = if let Some((a, b)) = weekday_or_month_day.split_once('-') {
                (
                    a.parse::<u32>().unwrap_or(start.month()),
                    b.parse::<u32>().unwrap_or(1),
                )
            } else {
                (
                    start.month(),
                    weekday_or_month_day.trim().parse::<u32>().unwrap_or(1),
                )
            };
            for year in start.year()..=end.year() {
                if let Some(on) = clamp_month_day(year, month, day) {
                    if on >= start && on <= end {
                        out.push(on.format("%Y-%m-%d").to_string());
                    }
                }
            }
        }
        _ => {}
    }
    out
}

/// Deposit increases running cash; Withdrawal decreases. Unknown start stays —.
pub fn fold_running_cash(start: Option<i64>, deltas: &[i64]) -> Vec<Option<i64>> {
    let Some(mut running) = start else {
        return deltas.iter().map(|_| None).collect();
    };
    deltas
        .iter()
        .map(|d| {
            running += *d;
            Some(running)
        })
        .collect()
}

pub fn register_cash_qty_to_minor(remaining_quantity_minor: i64, quantity_scale: u8) -> i64 {
    if quantity_scale >= 2 {
        let div = 10i64.pow(u32::from(quantity_scale - 2));
        if div == 0 {
            remaining_quantity_minor
        } else {
            remaining_quantity_minor / div
        }
    } else {
        remaining_quantity_minor * 10i64.pow(u32::from(2 - quantity_scale))
    }
}

pub fn book_matches_income_plan_account(book: &str, account_name: &str) -> bool {
    let book = register_book_label(book).unwrap_or(book);
    let label = crate::income_plan::display_account_label(
        crate::income_plan::map_income_plan_account(account_name).unwrap_or(account_name),
    );
    let want = match book {
        "Account 9" => "9",
        "FI Roth" => "FI Roth",
        other => other,
    };
    label.eq_ignore_ascii_case(want) || account_name.eq_ignore_ascii_case(book)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prove_subtotal_identity() {
        assert_eq!(validate_cash_distribution("IRA_Distribution", Some(100_000), 18_000, 4_500).unwrap(), 77_500);
        assert_eq!(net_minor(100_000, 18_000, 4_500), 77_500);
    }

    #[test]
    fn unknown_gross_is_not_zero() {
        assert_eq!(
            validate_cash_distribution("IRA_Distribution", None, 0, 0).unwrap_err(),
            DomainError::UnknownAmount
        );
    }

    #[test]
    fn withholding_cannot_exceed_gross() {
        assert_eq!(
            validate_cash_distribution("IRA_Distribution", Some(100), 80, 30).unwrap_err(),
            DomainError::CashDistributionIdentity
        );
    }

    #[test]
    fn roth_refuses_withholding() {
        assert_eq!(
            validate_cash_distribution("Roth_Distribution", Some(10_000), 1, 0).unwrap_err(),
            DomainError::RothWithholdingNotAllowed
        );
        assert_eq!(
            validate_cash_distribution("Roth_Distribution", Some(10_000), 0, 0).unwrap(),
            10_000
        );
    }

    #[test]
    fn withdrawal_posts_like_ira() {
        assert_eq!(
            validate_cash_distribution("Withdrawal", Some(10_000), 1_000, 500).unwrap(),
            8_500
        );
    }

    #[test]
    fn ssa_posts_as_social_security_retirement() {
        assert_eq!(
            validate_cash_distribution("SSA", Some(TOM_SSA_EXPECTED_MINOR), 0, 0).unwrap(),
            TOM_SSA_EXPECTED_MINOR
        );
        assert_eq!(SSA_RETIREMENT_LABEL, "Social Security retirement");
        assert!(!SSA_RETIREMENT_LABEL.contains("SSI"));
    }

    #[test]
    fn tom_ssa_miss_is_unconfirmed_not_zero() {
        let (status, extra, posted) = tom_ssa_month_status(0, None);
        assert_eq!(status, TomSsaStatus::Unconfirmed);
        assert!(!extra);
        assert_eq!(posted, None);
    }

    #[test]
    fn tom_ssa_expected_confirms_and_extra_is_audit() {
        let (status, extra, posted) = tom_ssa_month_status(2, None);
        assert_eq!(status, TomSsaStatus::Confirmed);
        assert!(!extra);
        assert_eq!(posted, Some(TOM_SSA_EXPECTED_MINOR));
        let (status3, extra3, _) = tom_ssa_month_status(3, None);
        assert_eq!(status3, TomSsaStatus::Confirmed);
        assert!(extra3);
    }

    #[test]
    fn barbara_and_tom_are_separate_expected_amounts() {
        assert_eq!(SsaPayee::Barbara.expected_minor(), 133_100);
        assert_eq!(SsaPayee::Tom.expected_minor(), 286_500);
        assert_eq!(
            ssa_idempotency_key(SsaPayee::Barbara, 2026, 7),
            "ssa-barbara-2026-07"
        );
        assert_eq!(classify_ssa_row("seed-tom-june-5", 286_500), Some(SsaPayee::Tom));
        assert_eq!(
            classify_ssa_row("barb-jul-3", 133_100),
            Some(SsaPayee::Barbara)
        );
        assert!(!ssa_household_extra_audit(3, false, false));
        assert!(ssa_household_extra_audit(4, false, false));
        assert!(!ssa_household_extra_audit(2, false, false));
        let (status, extra, posted) = ssa_payee_month_status(SsaPayee::Barbara, 1, Some(133_100));
        assert_eq!(status, TomSsaStatus::Confirmed);
        assert!(!extra);
        assert_eq!(posted, Some(133_100));
    }

    #[test]
    fn tom_ssa_variance_keeps_received_amount() {
        let (status, extra, posted) = tom_ssa_month_status(0, Some(280_000));
        assert_eq!(status, TomSsaStatus::Variance);
        assert!(!extra);
        assert_eq!(posted, Some(280_000));
    }

    #[test]
    fn saturday_draft_closes_after_income_ira() {
        assert!(saturday_draft_open(false));
        assert!(!saturday_draft_open(true));
    }

    #[test]
    fn cash_tax_sections_follow_owner_account_types() {
        assert_eq!(
            cash_tax_section("IRA_Distribution", "ira"),
            Some(CashTaxSection::IraOrdinary)
        );
        assert_eq!(
            cash_tax_section("IRA_Distribution", "ira"),
            cash_tax_section("IRA_Distribution", "IRA")
        );
        assert_eq!(
            cash_tax_section("Roth_Distribution", "roth"),
            Some(CashTaxSection::Roth)
        );
        assert_eq!(
            cash_tax_section("Roth_Distribution", "fi_roth"),
            Some(CashTaxSection::Roth)
        );
        assert_eq!(cash_tax_section("Withdrawal", "taxable"), None);
        assert_eq!(cash_tax_section("Form_1099", "taxable"), None);
        assert_eq!(
            cash_tax_section("SSA", "external"),
            Some(CashTaxSection::Ssa)
        );
        assert!(CashTaxSection::IraOrdinary.tax_note().contains("Speculation"));
    }

    #[test]
    fn cash_type_must_match_the_account_the_owner_sees() {
        assert!(cash_activity_allowed_for_account("IRA_Distribution", "Income", "ira").is_ok());
        assert!(cash_activity_allowed_for_account("IRA_Distribution", "Speculation", "ira").is_ok());
        assert!(cash_activity_allowed_for_account("Roth_Distribution", "FI Roth", "fi_roth").is_ok());
        assert!(cash_activity_allowed_for_account("Roth_Distribution", "Roth", "roth").is_ok());
        assert!(cash_activity_allowed_for_account("Withdrawal", "Car", "taxable").is_ok());
        assert!(cash_activity_allowed_for_account("Withdrawal", "Robinhood", "taxable").is_ok());
        assert!(cash_activity_allowed_for_account("SSA", "External", "taxable").is_ok());
        assert!(cash_activity_allowed_for_account("SSA", "External", "external").is_ok());
        assert_eq!(
            cash_activity_allowed_for_account("Withdrawal", "Income", "ira").unwrap_err(),
            DomainError::CashAccountKind
        );
        assert_eq!(
            cash_activity_allowed_for_account("IRA_Distribution", "Car", "taxable").unwrap_err(),
            DomainError::CashAccountKind
        );
        assert_eq!(
            cash_activity_allowed_for_account("Roth_Distribution", "Income", "ira").unwrap_err(),
            DomainError::CashAccountKind
        );
        assert_eq!(
            cash_activity_allowed_for_account("SSA", "Car", "taxable").unwrap_err(),
            DomainError::CashAccountKind
        );
        assert_eq!(
            cash_activity_allowed_for_account("Withdrawal", "External", "taxable").unwrap_err(),
            DomainError::CashAccountKind
        );
        assert_eq!(
            cash_activity_allowed_for_account("Withdrawal", "Health", "hsa").unwrap_err(),
            DomainError::CashAccountKind
        );
        assert!(cash_activity_allowed_for_account("HSA_Withdrawal", "Health", "hsa").is_ok());
        assert_eq!(
            cash_activity_allowed_for_account("HSA_Withdrawal", "Car", "taxable").unwrap_err(),
            DomainError::CashAccountKind
        );
    }

    #[test]
    fn week_ahead_ssa_lookback_is_four_calendar_days() {
        assert!(week_ahead_in_window(
            "Income",
            "2026-09-12",
            "2026-09-12",
            "2026-09-18"
        ));
        assert!(week_ahead_in_window(
            "SSA_2026",
            "2026-09-08",
            "2026-09-12",
            "2026-09-18"
        ));
        assert!(!week_ahead_in_window(
            "SSA_2026",
            "2026-09-07",
            "2026-09-12",
            "2026-09-18"
        ));
        assert!(!week_ahead_in_window(
            "Income",
            "2026-09-08",
            "2026-09-12",
            "2026-09-18"
        ));
    }

    #[test]
    fn withholding_does_not_change_magi_add() {
        let add = magi_add_minor("IRA_Distribution", Some(100_000));
        assert_eq!(add, Some(100_000));
        assert_eq!(tax_payment_credit_minor(18_000, 4_500), 22_500);
        assert_eq!(magi_add_minor("IRA_Distribution", Some(100_000)), add);
        assert_eq!(magi_add_minor("Roth_Distribution", Some(10_000)), Some(0));
        assert_eq!(magi_add_minor("SSA", Some(TOM_SSA_EXPECTED_MINOR)), None);
        assert_eq!(job_1099_year_amounts("2026-09-22"), (JOB_1099_2026_MINOR, 0));
        assert_eq!(job_1099_year_amounts("2025-12-31"), (0, 0));
        assert_eq!(
            SsaPayee::from_element_note("SSA Tom Deposit"),
            Some(SsaPayee::Tom)
        );
        assert_eq!(
            SsaPayee::from_element_note("SSA Barbara Deposit"),
            Some(SsaPayee::Barbara)
        );
        assert_eq!(
            element_history_posted_slice(
                "Car",
                "car",
                "Withdrawal",
                60_000,
                0,
                0,
                "owner-car-wd-2026-01-23"
            ),
            Some(60_000)
        );
        assert_eq!(
            element_history_posted_slice(
                "Car",
                "car",
                "Withdrawal",
                85_000,
                0,
                0,
                "week-ahead-abc"
            ),
            None
        );
        assert_eq!(
            element_history_posted_slice(
                "SSA_2026",
                "SSA Tom Deposit",
                "SSA",
                TOM_SSA_EXPECTED_MINOR,
                0,
                0,
                "ssa-tom-2026-09"
            ),
            Some(TOM_SSA_EXPECTED_MINOR)
        );
        assert_eq!(
            element_history_posted_slice(
                "SSA_2026",
                "SSA Barbara Deposit",
                "SSA",
                TOM_SSA_EXPECTED_MINOR,
                0,
                0,
                "ssa-tom-2026-09"
            ),
            None
        );
    }

    #[test]
    fn july_third_is_july_not_june_week_sum() {
        assert!(occurred_in_calendar_month("2026-07-03", 2026, 7));
        assert!(!occurred_in_calendar_month("2026-07-03", 2026, 6));
    }

    #[test]
    fn register_1m_is_calendar_month() {
        let as_of = NaiveDate::from_ymd_opt(2026, 9, 18).unwrap();
        let (start, end) = register_period_bounds("1M", as_of).unwrap();
        assert_eq!(start.to_string(), "2026-09-01");
        assert_eq!(end.to_string(), "2026-09-30");
    }

    #[test]
    fn register_1y_is_calendar_year() {
        let as_of = NaiveDate::from_ymd_opt(2026, 9, 18).unwrap();
        let (start, end) = register_period_bounds("1Y", as_of).unwrap();
        assert_eq!(start.to_string(), "2026-01-01");
        assert_eq!(end.to_string(), "2026-12-31");
    }

    #[test]
    fn element_history_durations_are_lookback_from_as_of() {
        let as_of = NaiveDate::from_ymd_opt(2026, 9, 22).unwrap();
        let (start, end) = element_history_period_bounds("ytd", as_of).unwrap();
        assert_eq!(start.unwrap().to_string(), "2026-01-01");
        assert_eq!(end.unwrap().to_string(), "2026-09-22");
        let (start, end) = element_history_period_bounds("1 month", as_of).unwrap();
        assert_eq!(start.unwrap().to_string(), "2026-08-22");
        assert_eq!(end.unwrap().to_string(), "2026-09-22");
        let (start, end) = element_history_period_bounds("6m", as_of).unwrap();
        assert_eq!(start.unwrap().to_string(), "2026-03-22");
        assert_eq!(end.unwrap().to_string(), "2026-09-22");
        let (start, end) = element_history_period_bounds("all", as_of).unwrap();
        assert_eq!(start, None);
        assert_eq!(end, None);
    }

    #[test]
    fn income_element_slice_matches_locked_footer_math() {
        assert_eq!(
            income_element_activity_slice(
                "fed",
                "IRA_Distribution",
                25_000,
                17_750,
                7_000,
                "production-disb-102",
            ),
            Some(17_750)
        );
        assert_eq!(
            income_element_activity_slice(
                "state",
                "IRA_Distribution",
                25_000,
                17_750,
                7_000,
                "production-disb-102",
            ),
            Some(7_000)
        );
        assert_eq!(
            income_element_activity_slice(
                "net",
                "IRA_Distribution",
                25_000,
                17_750,
                7_000,
                "production-disb-102",
            ),
            Some(250)
        );
        assert_eq!(
            income_element_activity_slice(
                "fed",
                "IRA_Distribution",
                17_750,
                0,
                0,
                "week-ahead-abc",
            ),
            None
        );
        assert_eq!(
            income_element_activity_slice("car", "IRA_Distribution", 25_000, 17_750, 7_000, "x"),
            None
        );
        assert_eq!(
            3_998_68 + 95_750,
            495_618,
            "seed fed + Aug/Sep confirmed = locked $4,956.18"
        );
        assert_eq!(
            16_350_99 + 492_250,
            2_127_349,
            "seed net (gross−fed−state) + Aug/Sep confirmed = locked $21,273.49"
        );
    }

    #[test]
    fn element_retired_only_after_stop() {
        let as_of = NaiveDate::from_ymd_opt(2026, 9, 22).unwrap();
        assert!(!element_is_retired("", as_of));
        assert!(!element_is_retired("2026-12-25", as_of));
        assert!(!element_is_retired("2026-09-22", as_of));
        assert!(element_is_retired("2026-08-01", as_of));
    }

    #[test]
    fn weekly_sat_horizon_lists_saturdays() {
        let start = NaiveDate::from_ymd_opt(2026, 9, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 9, 30).unwrap();
        let days = element_horizon_dates("weekly", "Sat", start, end);
        assert!(days.contains(&"2026-09-12".into()));
        assert!(days.contains(&"2026-09-19".into()));
        assert!(!days.iter().any(|d| d == "2026-09-11"));
    }

    #[test]
    fn monthly_first_horizon() {
        let start = NaiveDate::from_ymd_opt(2026, 9, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 10, 31).unwrap();
        let days = element_horizon_dates("monthly", "1", start, end);
        assert_eq!(days, vec!["2026-09-01".to_string(), "2026-10-01".to_string()]);
    }

    #[test]
    fn running_fold_unknown_stays_none() {
        assert_eq!(fold_running_cash(None, &[100, -40]), vec![None, None]);
        assert_eq!(
            fold_running_cash(Some(1_000), &[100, -40]),
            vec![Some(1_100), Some(1_060)]
        );
    }

    #[test]
    fn cash_adjust_allows_negative_refuses_withholding() {
        let (amt, note) =
            validate_cash_adjust("Cash_Adjust", "Car", Some(-825), 0, 0, "fee").unwrap();
        assert_eq!(amt, -825);
        assert_eq!(note, "fee");
        assert_eq!(
            validate_cash_adjust("Cash_Adjust", "Car", Some(-825), 1, 0, "fee").unwrap_err(),
            DomainError::CashAdjustWithholdingNotAllowed
        );
        assert_eq!(
            validate_cash_adjust("Cash_Adjust", "Car", Some(-825), 0, 0, "").unwrap_err(),
            DomainError::CashAdjustReasonRequired
        );
    }
}
