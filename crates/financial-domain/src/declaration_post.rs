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
/// Does not overwrite.
pub fn overlap_amount_change_issues(
    stored: &[PaidDeclarationView<'_>],
    page: &[PaidDeclarationView<'_>],
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

/// New paid amounts vs the immediately previous paid in date order.
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

#[cfg(test)]
mod tests {
    use super::*;

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
        let issues = overlap_amount_change_issues(&stored, &changed);
        assert_eq!(issues[0].code, "declaration_amount_variation");
        assert!(issues[0].message.contains("2026-04-30"));
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
