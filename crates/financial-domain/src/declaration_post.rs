//! Post-process checks after a declaration retrieve is stored.
//! Integer math only. Unknown stays unknown — never invent $0 or a cadence.

use crate::calculator::{infer_payment_cadence, PaymentCadence};

/// Owner-named ceiling: a new paid amount may not move more than this percent vs the prior paid.
pub const AMOUNT_VARIATION_PCT: i64 = 30;

/// Ticket amount changes when stored paid history already exists.
/// Never a license to overwrite those rows. Zero stored pays = first write, no ticket.
pub fn amount_variation_applies(stored_paid_count: u8) -> bool {
    stored_paid_count > 0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaidDeclarationView<'a> {
    pub payment_period: &'a str,
    pub amount_per_share_minor: Option<i64>,
    pub amount_scale: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostCheckIssue {
    pub code: &'static str,
    pub message: String,
}

/// `|new - prev| / prev > pct/100` using integers: `|Δ| * 100 > pct * prev`.
/// `None` when prev ≤ 0 (unknown — do not invent a miss or a pass).
pub fn amount_exceeds_variation_pct(new_minor: i64, prev_minor: i64, pct: i64) -> Option<bool> {
    if prev_minor <= 0 || pct < 0 {
        return None;
    }
    let delta = (new_minor as i128 - prev_minor as i128).abs();
    Some(delta.saturating_mul(100) > (prev_minor as i128).saturating_mul(pct as i128))
}

fn period_key(raw: &str) -> String {
    let s = raw.trim();
    if s.len() >= 10 {
        s[..10].to_string()
    } else {
        s.to_string()
    }
}

fn paid_sorted<'a>(rows: &[PaidDeclarationView<'a>]) -> Vec<(String, i64, u8)> {
    let mut out: Vec<(String, i64, u8)> = rows
        .iter()
        .filter_map(|r| {
            let amt = r.amount_per_share_minor.filter(|a| *a > 0)?;
            let period = period_key(r.payment_period);
            if !crate::schedule::is_plausible_payment_period(&period) {
                return None;
            }
            Some((period, amt, r.amount_scale))
        })
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn format_per_share(minor: i64, scale: u8) -> String {
    let scale = scale.min(6);
    let factor = 10i64.pow(u32::from(scale));
    let whole = minor / factor;
    let frac = (minor % factor).unsigned_abs();
    if scale == 0 {
        format!("${whole}")
    } else {
        format!("${whole}.{frac:0width$}", width = scale as usize)
    }
}

fn amount_confirm_message(period: &str, new_minor: i64, new_scale: u8, prior_period: &str, prior_minor: i64, prior_scale: u8, pct: i64) -> String {
    format!(
        "Paid {}/unit on {period} is more than {pct}% away from the prior paid {}/unit on {prior_period}. Except keeps the new issuer amount. Reject leaves the stored amount.",
        format_per_share(new_minor, new_scale),
        format_per_share(prior_minor, prior_scale),
    )
}

/// Short increment pages keep stored pays. Dates in SQLite and absent from this
/// page are not a retrieve miss and must not wipe history.
pub fn previous_periods_retained(
    _stored_before: &[PaidDeclarationView<'_>],
    _reparsed: &[PaidDeclarationView<'_>],
) -> Vec<PostCheckIssue> {
    Vec::new()
}

/// Overlapping payable dates whose page amount differs from stored. Ticket only.
/// Does not overwrite. Runtime window only — months-old page reprints stay locked.
pub fn overlap_amount_change_issues(
    stored: &[PaidDeclarationView<'_>],
    page: &[PaidDeclarationView<'_>],
    as_of: &str,
) -> Vec<PostCheckIssue> {
    let mut issues = Vec::new();
    for row in page {
        let Some(page_amt) = row.amount_per_share_minor.filter(|a| *a > 0) else {
            continue;
        };
        let period = period_key(row.payment_period);
        if period.is_empty() || !crate::schedule::is_plausible_payment_period(&period) {
            continue;
        }
        if !crate::schedule::runtime_declaration_verify_period(&period, as_of) {
            continue;
        }
        let Some(stored_row) = stored.iter().find(|s| {
            s.amount_per_share_minor.map(|a| a > 0).unwrap_or(false)
                && period_key(s.payment_period) == period
        }) else {
            continue;
        };
        let Some(stored_amt) = stored_row.amount_per_share_minor else {
            continue;
        };
        if !crate::money::amounts_equal(
            stored_amt,
            stored_row.amount_scale,
            page_amt,
            row.amount_scale,
        ) {
            issues.push(PostCheckIssue {
                code: "declaration_amount_variation",
                message: format!(
                    "Issuer page {}/unit on {period} does not match stored {}/unit. Except keeps the new issuer amount. Reject leaves the stored amount.",
                    format_per_share(page_amt, row.amount_scale),
                    format_per_share(stored_amt, stored_row.amount_scale),
                ),
            });
        }
    }
    issues
}

/// Locked cadence vs inferred median gap of the newest 12 paid dates.
/// Older quarterly history must not hide a recent monthly switch (TRIN 2026).
pub fn cadence_matches_locked(
    paid_periods: &[&str],
    locked_frequency: &str,
) -> Vec<PostCheckIssue> {
    let Some(locked) = PaymentCadence::parse(locked_frequency).filter(|c| c.periods().is_some())
    else {
        return Vec::new();
    };
    let mut recent: Vec<&str> = paid_periods.to_vec();
    recent.sort_unstable();
    let start = recent.len().saturating_sub(12);
    let recent = &recent[start..];
    let Some(inferred) = infer_payment_cadence(recent, None) else {
        return Vec::new();
    };
    if inferred == locked {
        return Vec::new();
    }
    vec![PostCheckIssue {
        code: "declaration_cadence_mismatch",
        message: format!(
            "Collector paid dates infer {}. Locked cadence is {}. Keeping {} would hide the discovered {} history and can drop or re-group stored paid dates. Change cadence on Position Details (Save replaces cadence) only if the lock is wrong.",
            inferred.label(),
            locked.label(),
            locked.label(),
            inferred.label()
        ),
    }]
}

/// Minimum days between consecutive pays for a locked cadence (band lower bound).
/// Weekly 5–9, Monthly 25–35, Quarterly 90–100.
pub fn cadence_spacing_floor_days(locked_frequency: &str) -> Option<i64> {
    cadence_gap_band(locked_frequency).map(|(lo, _)| lo)
}

/// Inclusive consecutive-gap band for future plan / spacing guards.
pub fn cadence_gap_band(locked_frequency: &str) -> Option<(i64, i64)> {
    match PaymentCadence::parse(locked_frequency)? {
        PaymentCadence::Weekly => Some((5, 9)),
        PaymentCadence::TwiceMonthly => Some((11, 20)),
        PaymentCadence::Monthly => Some((25, 35)),
        PaymentCadence::Quarterly => Some((90, 100)),
        PaymentCadence::None => None,
    }
}

/// Required pay count inside a rolling window for a locked cadence.
pub fn cadence_count_window(locked_frequency: &str) -> Option<(usize, i64)> {
    match PaymentCadence::parse(locked_frequency)? {
        PaymentCadence::Weekly => Some((52, 368)),
        PaymentCadence::TwiceMonthly => Some((24, 375)),
        PaymentCadence::Monthly => Some((12, 375)),
        PaymentCadence::Quarterly => Some((4, 375)),
        PaymentCadence::None => None,
    }
}

/// Gap below which two dates are treated as ex/record + payable twins (not two periods).
pub fn cadence_twin_gap_ceiling_days(locked_frequency: &str) -> Option<i64> {
    match PaymentCadence::parse(locked_frequency)? {
        PaymentCadence::Weekly => Some(3),
        PaymentCadence::TwiceMonthly => Some(7),
        PaymentCadence::Monthly => Some(14),
        PaymentCadence::Quarterly => Some(45),
        PaymentCadence::None => None,
    }
}

fn parse_period_day(raw: &str) -> Option<chrono::NaiveDate> {
    let s = period_key(raw);
    chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()
}

fn sorted_plausible_days(periods: &[&str]) -> Vec<(String, chrono::NaiveDate)> {
    let mut days: Vec<(String, chrono::NaiveDate)> = periods
        .iter()
        .filter_map(|p| {
            let key = period_key(p);
            if !crate::schedule::is_plausible_payment_period(&key) {
                return None;
            }
            let d = parse_period_day(&key)?;
            Some((key, d))
        })
        .collect();
    days.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
    days.dedup_by(|a, b| a.0 == b.0);
    days
}

/// Consecutive gaps **shorter** than the band floor are a loud miss (twins / invents).
/// Upper band is enforced on future plan dates via `filter_dates_to_gap_band`.
/// Only pairs whose later date is in the runtime window (current month + future).
/// Decades-old locked pays are establish history — not an October collect miss.
pub fn cadence_spacing_issues(
    paid_periods: &[&str],
    locked_frequency: &str,
    as_of: &str,
) -> Vec<PostCheckIssue> {
    let Some((lo, hi)) = cadence_gap_band(locked_frequency) else {
        return Vec::new();
    };
    let locked = PaymentCadence::parse(locked_frequency)
        .map(|c| c.label())
        .unwrap_or(locked_frequency);
    let days = sorted_plausible_days(paid_periods);
    let mut issues = Vec::new();
    for w in days.windows(2) {
        if !crate::schedule::runtime_declaration_verify_period(&w[1].0, as_of) {
            continue;
        }
        let gap = (w[1].1 - w[0].1).num_days().abs();
        if gap < lo {
            issues.push(PostCheckIssue {
                code: "declaration_cadence_spacing",
                message: format!(
                    "Paid dates {} and {} are {gap} days apart; locked {locked} expects {lo}–{hi} days between pays.",
                    w[0].0, w[1].0
                ),
            });
        }
    }
    issues
}

/// Future-plan consecutive gaps must lie inside the band (short invents and long drift).
pub fn consecutive_gap_band_issues(
    periods: &[&str],
    locked_frequency: &str,
) -> Vec<PostCheckIssue> {
    let Some((lo, hi)) = cadence_gap_band(locked_frequency) else {
        return Vec::new();
    };
    let locked = PaymentCadence::parse(locked_frequency)
        .map(|c| c.label())
        .unwrap_or(locked_frequency);
    let days = sorted_plausible_days(periods);
    let mut issues = Vec::new();
    for w in days.windows(2) {
        let gap = (w[1].1 - w[0].1).num_days().abs();
        if gap < lo || gap > hi {
            issues.push(PostCheckIssue {
                code: "declaration_cadence_spacing",
                message: format!(
                    "Plan dates {} and {} are {gap} days apart; locked {locked} expects {lo}–{hi} days between pays.",
                    w[0].0, w[1].0
                ),
            });
        }
    }
    issues
}

/// True when `dates` (sorted ISO) has at least `count` pays whose span from first to last
/// in any window of `window_days` — checked as: from the earliest date ≥ as_of−window
/// through as_of+window, or simply count of dates in [anchor, anchor+window] ≥ count.
pub fn count_window_ok(dates: &[&str], locked_frequency: &str, as_of: &str) -> bool {
    let Some((need, window_days)) = cadence_count_window(locked_frequency) else {
        return true;
    };
    let Some(as_of_d) = parse_period_day(as_of) else {
        return false;
    };
    let days = sorted_plausible_days(dates);
    if days.is_empty() {
        return false;
    }
    // Prefer a window starting at last pay on/before as_of, else first upcoming.
    let anchor = days
        .iter()
        .rev()
        .find(|(_, d)| *d <= as_of_d)
        .or_else(|| days.first())
        .map(|(_, d)| *d)
        .unwrap();
    let end = anchor + chrono::Duration::days(window_days);
    let n = days
        .iter()
        .filter(|(_, d)| *d >= anchor && *d <= end)
        .count();
    n >= need
}

/// Keep dates that form a band-legal chain from `last_paid` (if any) through upcoming.
/// Drops invents outside the band (e.g. quarterly 9/30 after 8/19); keeps later in-band dates.
pub fn filter_dates_to_gap_band(
    dates: &[String],
    locked_frequency: &str,
    last_paid: Option<&str>,
) -> Vec<String> {
    let Some((lo, hi)) = cadence_gap_band(locked_frequency) else {
        return dates.to_vec();
    };
    let mut prev = last_paid.and_then(parse_period_day);
    let mut out = Vec::new();
    let mut sorted = dates.to_vec();
    sorted.sort();
    sorted.dedup();
    for on in sorted {
        let Some(d) = parse_period_day(&on) else {
            continue;
        };
        if let Some(p) = prev {
            let gap = (d - p).num_days().abs();
            if gap < lo || gap > hi {
                continue;
            }
        }
        out.push(on);
        prev = Some(d);
    }
    out
}

/// New paid amounts vs the immediately previous paid in date order. Only rows this run
/// actually recorded, so a reprint is never re-ticketed — but age does not silence it: a
/// pay recorded on the 1st for last month's pay date still has to clear the 30% rule.
pub fn new_amount_variation_issues(
    newly_posted: &[PaidDeclarationView<'_>],
    stored_after: &[PaidDeclarationView<'_>],
    pct: i64,
) -> Vec<PostCheckIssue> {
    let series = paid_sorted(stored_after);
    let mut issues = Vec::new();
    for row in newly_posted {
        let Some(new_amt) = row.amount_per_share_minor.filter(|a| *a > 0) else {
            continue;
        };
        let period = period_key(row.payment_period);
        if period.is_empty() || !crate::schedule::is_plausible_payment_period(&period) {
            continue;
        }
        let Some(prev) = series.iter().rev().find(|(p, _, _)| p.as_str() < period.as_str()) else {
            continue;
        };
        let to = row.amount_scale.max(prev.2);
        if amount_exceeds_variation_pct(
            crate::money::rescale(new_amt, row.amount_scale, to),
            crate::money::rescale(prev.1, prev.2, to),
            pct,
        ) == Some(true)
        {
            issues.push(PostCheckIssue {
                code: "declaration_amount_variation",
                message: amount_confirm_message(
                    &period,
                    new_amt,
                    row.amount_scale,
                    &prev.0,
                    prev.1,
                    prev.2,
                    pct,
                ),
            });
        }
    }
    issues
}

/// The newest stored pay against the one before it. Establish writes a whole page in one
/// run, so the runtime "new vs stored" check never sees that jump — read the series itself.
/// Anchored to the newest pay on purpose: a 2003 distribution change is history, not an
/// Except/Reject the owner can act on. Any cadence.
pub fn history_amount_variation_issues(
    stored: &[PaidDeclarationView<'_>],
    pct: i64,
) -> Vec<PostCheckIssue> {
    let series = paid_sorted(stored);
    let Some([(prior_period, prior_minor, prior_scale), (period, minor, scale)]) =
        series.last_chunk::<2>()
    else {
        return Vec::new();
    };
    let to = (*scale).max(*prior_scale);
    if amount_exceeds_variation_pct(
        crate::money::rescale(*minor, *scale, to),
        crate::money::rescale(*prior_minor, *prior_scale, to),
        pct,
    ) != Some(true)
    {
        return Vec::new();
    }
    vec![PostCheckIssue {
        code: "declaration_amount_variation",
        message: amount_confirm_message(
            period,
            *minor,
            *scale,
            prior_period,
            *prior_minor,
            *prior_scale,
            pct,
        ),
    }]
}

fn rescale_paid_plan(
    plan_minor: i64,
    plan_scale: u8,
    paid_minor: i64,
    paid_scale: u8,
) -> Option<(i64, i64)> {
    if plan_minor <= 0 || paid_minor <= 0 {
        return None;
    }
    let to = plan_scale.max(paid_scale);
    Some((
        crate::money::rescale(paid_minor, paid_scale, to),
        crate::money::rescale(plan_minor, plan_scale, to),
    ))
}

pub fn pay_is_under_plan(
    plan_minor: i64,
    plan_scale: u8,
    paid_minor: i64,
    paid_scale: u8,
    pct: i64,
) -> bool {
    let Some((paid, plan)) = rescale_paid_plan(plan_minor, plan_scale, paid_minor, paid_scale)
    else {
        return false;
    };
    paid < plan && amount_exceeds_variation_pct(paid, plan, pct) == Some(true)
}

pub fn pay_is_over_plan(
    plan_minor: i64,
    plan_scale: u8,
    paid_minor: i64,
    paid_scale: u8,
    pct: i64,
) -> bool {
    let Some((paid, plan)) = rescale_paid_plan(plan_minor, plan_scale, paid_minor, paid_scale)
    else {
        return false;
    };
    paid > plan && amount_exceeds_variation_pct(paid, plan, pct) == Some(true)
}

/// Non-blank pays, one row per period, oldest first. Blank is not under and not over.
fn comparable_pays(pays: &[(&str, i64, u8)]) -> Vec<(String, i64, u8)> {
    let mut by_period: std::collections::BTreeMap<String, (i64, u8)> =
        std::collections::BTreeMap::new();
    for (period, minor, scale) in pays {
        if *minor <= 0 {
            continue;
        }
        let period = period_key(period);
        if period.is_empty() || !crate::schedule::is_plausible_payment_period(&period) {
            continue;
        }
        by_period.insert(period, (*minor, *scale));
    }
    by_period.into_iter().map(|(p, (m, s))| (p, m, s)).collect()
}

/// Pays more than `pct` over Plan. The deposit match uses these amounts.
pub fn over_plan_pays(
    plan_minor: i64,
    plan_scale: u8,
    pays: &[(&str, i64, u8)],
    pct: i64,
) -> Vec<(String, i64, u8)> {
    comparable_pays(pays)
        .into_iter()
        .filter(|(_, minor, scale)| {
            pay_is_over_plan(plan_minor, plan_scale, *minor, *scale, pct)
        })
        .collect()
}

pub fn latest_paid_is_under_plan(
    plan_minor: i64,
    plan_scale: u8,
    pays: &[(&str, i64, u8)],
    pct: i64,
) -> bool {
    let series = comparable_pays(pays);
    let Some((_, minor, scale)) = series.last() else {
        return false;
    };
    pay_is_under_plan(plan_minor, plan_scale, *minor, *scale, pct)
}

/// Newest paid against Plan, with the under-plan count across every non-blank pay.
pub fn plan_vs_paid_series_issue(
    plan_minor: i64,
    plan_scale: u8,
    pays: &[(&str, i64, u8)],
    pct: i64,
) -> Option<PostCheckIssue> {
    if plan_minor <= 0 {
        return None;
    }
    let series = comparable_pays(pays);
    let (period, latest_minor, latest_scale) = series.last()?.clone();
    let under = pay_is_under_plan(plan_minor, plan_scale, latest_minor, latest_scale, pct);
    let over = pay_is_over_plan(plan_minor, plan_scale, latest_minor, latest_scale, pct);
    if !under && !over {
        return None;
    }
    let direction = if under { "under by" } else { "over" };
    let mut message = format!(
        "Paid {}/unit on {period} is more than {pct}% {direction} Plan {}/unit.",
        format_per_share(latest_minor, latest_scale),
        format_per_share(plan_minor, plan_scale),
    );
    if under {
        let y = series.len();
        let x = series
            .iter()
            .filter(|(_, minor, scale)| {
                pay_is_under_plan(plan_minor, plan_scale, *minor, *scale, pct)
            })
            .count();
        let z = (x * 100) / y;
        message.push_str(&format!(
            " This position has been under plan {x} times out of {y} periods ({z} percent reported under plan)."
        ));
    }
    message.push_str(
        " Plan drives the cart and Income Plan — re-confirm Plan or except this pay.",
    );
    Some(PostCheckIssue {
        code: "declaration_plan_mismatch",
        message,
    })
}

/// The newest paid amount against the in-force Plan per share. Plan is the only number the
/// cart and Income Plan use, so a paid amount far from Plan is an owner re-confirm, not a
/// silent Plan rewrite.
pub fn plan_vs_latest_paid_issue(
    plan_minor: i64,
    plan_scale: u8,
    latest_period: &str,
    latest_minor: i64,
    latest_scale: u8,
    pct: i64,
) -> Option<PostCheckIssue> {
    plan_vs_paid_series_issue(
        plan_minor,
        plan_scale,
        &[(latest_period, latest_minor, latest_scale)],
        pct,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn muib_establish_batch_tickets_the_in_history_jump() {
        let seeded = [
            PaidDeclarationView {
                payment_period: "2026-08-17",
                amount_per_share_minor: Some(23_461),
                amount_scale: 5,
            },
            PaidDeclarationView {
                payment_period: "2026-08-31",
                amount_per_share_minor: Some(23_967),
                amount_scale: 5,
            },
            PaidDeclarationView {
                payment_period: "2026-09-16",
                amount_per_share_minor: Some(24_931),
                amount_scale: 5,
            },
            PaidDeclarationView {
                payment_period: "2026-10-01",
                amount_per_share_minor: Some(57_925),
                amount_scale: 5,
            },
        ];
        let issues = history_amount_variation_issues(&seeded, AMOUNT_VARIATION_PCT);
        assert_eq!(issues.len(), 1, "one Except/Reject for the newest pay");
        assert_eq!(issues[0].code, "declaration_amount_variation");
        assert!(issues[0].message.contains("2026-10-01"), "{}", issues[0].message);
        assert!(
            history_amount_variation_issues(&seeded[..3], AMOUNT_VARIATION_PCT).is_empty(),
            "the first three pays are within 30%"
        );
        let healed = [
            seeded[0],
            seeded[1],
            seeded[2],
            seeded[3],
            PaidDeclarationView {
                payment_period: "2026-10-16",
                amount_per_share_minor: Some(57_000),
                amount_scale: 5,
            },
        ];
        assert!(
            history_amount_variation_issues(&healed, AMOUNT_VARIATION_PCT).is_empty(),
            "an old jump is history once a steady pay follows it — GLAD's 2003 \
             quarterly-to-monthly change is not an owner action in 2026"
        );
    }

    #[test]
    fn plan_far_from_latest_paid_is_a_plan_vs_declaration_ticket() {
        let issue = plan_vs_latest_paid_issue(2_300, 4, "2026-10-01", 57_925, 5, AMOUNT_VARIATION_PCT)
            .expect("MUIB Plan $0.2300 vs paid $0.57925");
        assert_eq!(issue.code, "declaration_plan_mismatch");
        assert!(issue.message.contains("$0.2300"), "{}", issue.message);
        assert!(issue.message.contains(" over "), "{}", issue.message);
        assert!(!issue.message.contains("away from"), "{}", issue.message);
        assert!(
            plan_vs_latest_paid_issue(3_800, 4, "2026-09-30", 3_936, 4, AMOUNT_VARIATION_PCT)
                .is_none(),
            "HAKY Plan $0.38 vs paid $0.3936 is inside the band"
        );
        assert!(
            plan_vs_latest_paid_issue(0, 4, "2026-10-01", 57_925, 5, AMOUNT_VARIATION_PCT).is_none(),
            "no Plan is not a drift ticket"
        );
    }

    #[test]
    fn under_plan_says_under_by_and_counts_the_series() {
        let pays = [
            ("2026-06-30", 400, 2),
            ("2026-07-15", 0, 2),
            ("2026-07-31", 900, 2),
            ("2026-08-31", 500, 2),
        ];
        let issue = plan_vs_paid_series_issue(1_000, 2, &pays, AMOUNT_VARIATION_PCT)
            .expect("latest $5 vs Plan $10");
        assert!(issue.message.contains("under by"), "{}", issue.message);
        assert!(
            issue.message.contains(
                "This position has been under plan 2 times out of 3 periods (66 percent reported under plan)."
            ),
            "{}",
            issue.message
        );
        assert!(!issue.message.contains("away from"), "{}", issue.message);
    }

    #[test]
    fn thirty_percent_rule_needs_stored_history() {
        assert!(!amount_variation_applies(0));
        assert!(amount_variation_applies(1));
        assert!(amount_variation_applies(7));
    }

    #[test]
    fn thirty_percent_is_integer_and_skips_unknown_prev() {
        assert_eq!(amount_exceeds_variation_pct(130, 100, 30), Some(false));
        assert_eq!(amount_exceeds_variation_pct(131, 100, 30), Some(true));
        assert_eq!(amount_exceeds_variation_pct(70, 100, 30), Some(false));
        assert_eq!(amount_exceeds_variation_pct(69, 100, 30), Some(true));
        assert_eq!(amount_exceeds_variation_pct(50, 0, 30), None);
        assert_eq!(amount_exceeds_variation_pct(50, -1, 30), None);
    }

    #[test]
    fn short_increment_page_does_not_drop_stored_history() {
        let stored = [
            PaidDeclarationView {
                payment_period: "2026-04-30",
                amount_per_share_minor: Some(100),
                amount_scale: 2,
            },
            PaidDeclarationView {
                payment_period: "2026-05-29",
                amount_per_share_minor: Some(100),
                amount_scale: 2,
            },
        ];
        let page = [
            PaidDeclarationView {
                payment_period: "2026-04-30",
                amount_per_share_minor: Some(100),
                amount_scale: 2,
            },
            PaidDeclarationView {
                payment_period: "2026-06-30",
                amount_per_share_minor: Some(100),
                amount_scale: 2,
            },
        ];
        assert!(previous_periods_retained(&stored, &page).is_empty());
        let changed = [PaidDeclarationView {
            payment_period: "2026-04-30",
            amount_per_share_minor: Some(140),
            amount_scale: 2,
        }];
        let issues = overlap_amount_change_issues(&stored, &changed, "2026-04-30");
        assert_eq!(issues[0].code, "declaration_amount_variation");
        assert!(issues[0].message.contains("2026-04-30"));
        assert!(
            overlap_amount_change_issues(&stored, &changed, "2026-10-01").is_empty(),
            "April page reprint is not an October collect miss"
        );
    }

    #[test]
    fn cadence_monthly_dates_match_locked_monthly() {
        let periods = [
            "2026-01-30",
            "2026-02-27",
            "2026-03-31",
            "2026-04-30",
        ];
        let refs: Vec<&str> = periods.iter().copied().collect();
        assert!(cadence_matches_locked(&refs, "Monthly").is_empty());
        let issues = cadence_matches_locked(&refs, "Weekly");
        assert_eq!(issues[0].code, "declaration_cadence_mismatch");
        assert!(issues[0].message.contains("hide the discovered"));
        assert!(cadence_matches_locked(&["2026-01-30"], "Monthly").is_empty());
    }

    #[test]
    fn recent_monthly_overrides_older_quarterly_history() {
        let periods = [
            "2025-03-31",
            "2025-06-30",
            "2025-09-30",
            "2025-12-31",
            "2026-01-15",
            "2026-02-13",
            "2026-03-13",
            "2026-04-15",
            "2026-05-15",
            "2026-06-11",
            "2026-07-15",
            "2026-08-14",
            "2026-09-10",
        ];
        let refs: Vec<&str> = periods.iter().copied().collect();
        assert!(cadence_matches_locked(&refs, "Monthly").is_empty());
        let issues = cadence_matches_locked(&refs, "Quarterly");
        assert_eq!(issues[0].code, "declaration_cadence_mismatch");
        assert!(issues[0].message.contains("Monthly"));
    }

    #[test]
    fn cadence_spacing_rejects_epd_style_quarterly_twins() {
        let twins = [
            "2025-07-31",
            "2025-08-14",
            "2025-10-31",
            "2025-11-14",
            "2026-01-30",
            "2026-02-13",
            "2026-04-30",
            "2026-05-14",
            "2026-07-31",
            "2026-08-14",
        ];
        let refs: Vec<&str> = twins.iter().copied().collect();
        let issues = cadence_spacing_issues(&refs, "Quarterly", "2026-08-14");
        assert!(
            issues.iter().any(|i| i.code == "declaration_cadence_spacing"),
            "{issues:?}"
        );
        assert!(issues.iter().any(|i| i.message.contains("14 days")), "{issues:?}");
        assert!(
            cadence_spacing_issues(&refs, "Quarterly", "2026-10-01").is_empty(),
            "August twins are establish/heal history in October"
        );
    }

    #[test]
    fn cadence_spacing_accepts_clean_quarterly_and_weekly() {
        let quarterly = [
            "2025-02-14",
            "2025-05-15",
            "2025-08-14",
            "2025-11-14",
            "2026-02-13",
            "2026-05-14",
            "2026-08-14",
        ];
        let q: Vec<&str> = quarterly.iter().copied().collect();
        assert!(
            cadence_spacing_issues(&q, "Quarterly", "2026-08-14").is_empty(),
            "{:?}",
            cadence_spacing_issues(&q, "Quarterly", "2026-08-14")
        );
        let weekly = [
            "2026-08-01",
            "2026-08-08",
            "2026-08-15",
            "2026-08-22",
            "2026-08-29",
        ];
        let w: Vec<&str> = weekly.iter().copied().collect();
        assert!(cadence_spacing_issues(&w, "Weekly", "2026-08-29").is_empty());
        let monthly = ["2026-05-30", "2026-06-30", "2026-07-31", "2026-08-29"];
        let m: Vec<&str> = monthly.iter().copied().collect();
        assert!(cadence_spacing_issues(&m, "Monthly", "2026-08-29").is_empty());
    }

    #[test]
    fn gap_band_rejects_et_sep_invent_keeps_nov() {
        let bad = ["2026-08-19", "2026-09-30"];
        let b: Vec<&str> = bad.iter().copied().collect();
        assert!(!consecutive_gap_band_issues(&b, "Quarterly").is_empty());
        let good = ["2026-08-19", "2026-11-19"];
        let g: Vec<&str> = good.iter().copied().collect();
        assert!(consecutive_gap_band_issues(&g, "Quarterly").is_empty());
        let filtered = filter_dates_to_gap_band(
            &["2026-09-30".into(), "2026-11-19".into()],
            "Quarterly",
            Some("2026-08-19"),
        );
        assert_eq!(filtered, vec!["2026-11-19".to_string()]);
    }

    #[test]
    fn count_window_quarterly_four_in_375() {
        let series = [
            "2026-08-19",
            "2026-11-19",
            "2027-02-19",
            "2027-05-20",
        ];
        let refs: Vec<&str> = series.iter().copied().collect();
        assert!(count_window_ok(&refs, "Quarterly", "2026-10-01"));
        assert!(!count_window_ok(
            &["2026-08-19", "2026-11-19"],
            "Quarterly",
            "2026-10-01"
        ));
    }

    #[test]
    fn new_amount_40_pct_vs_prior_fails() {
        let stored = [
            PaidDeclarationView {
                payment_period: "2026-07-31",
                amount_per_share_minor: Some(1000),
                amount_scale: 2,
            },
            PaidDeclarationView {
                payment_period: "2026-08-31",
                amount_per_share_minor: Some(1400),
                amount_scale: 2,
            },
        ];
        let newly = [PaidDeclarationView {
            payment_period: "2026-08-31",
            amount_per_share_minor: Some(1400),
            amount_scale: 2,
        }];
        let issues = new_amount_variation_issues(&newly, &stored, AMOUNT_VARIATION_PCT);
        assert_eq!(issues[0].code, "declaration_amount_variation");
        assert_eq!(
            new_amount_variation_issues(&newly, &stored, AMOUNT_VARIATION_PCT),
            issues,
            "an August pay recorded in October still has to clear the 30% rule — \
             reprints are filtered by being unchanged, not by the calendar"
        );
        let ok_new = [PaidDeclarationView {
            payment_period: "2026-08-31",
            amount_per_share_minor: Some(1200),
            amount_scale: 2,
        }];
        let stored_ok = [
            stored[0],
            PaidDeclarationView {
                payment_period: "2026-08-31",
                amount_per_share_minor: Some(1200),
                amount_scale: 2,
            },
        ];
        assert!(new_amount_variation_issues(&ok_new, &stored_ok, AMOUNT_VARIATION_PCT).is_empty());
    }

    #[test]
    fn implausible_pay_dates_are_not_amount_tickets() {
        let stored = [
            PaidDeclarationView {
                payment_period: "0000-02-04",
                amount_per_share_minor: Some(475),
                amount_scale: 2,
            },
            PaidDeclarationView {
                payment_period: "0000-02-10",
                amount_per_share_minor: Some(50),
                amount_scale: 2,
            },
        ];
        let newly = [stored[1]];
        assert!(
            new_amount_variation_issues(&newly, &stored, AMOUNT_VARIATION_PCT).is_empty(),
            "year 0000 is a parse miss, not Except/Reject"
        );
    }
}
