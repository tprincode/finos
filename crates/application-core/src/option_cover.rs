//! Share cover for a short call, and the week a premium posts.
//! Money is integer cents. A missing price is not filled here.

use financial_domain::week::{parse_iso_day, week_containing};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharePiece {
    pub lot_id: String,
    pub opened_on: String,
    pub shares: i64,
    pub cost_minor: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverTake {
    pub lot_id: String,
    pub shares: i64,
    pub cost_minor: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PremiumPost {
    pub week_start: String,
    pub category: String,
    pub amount_minor: i64,
    pub reason: String,
}

pub fn account_allowed(name: &str) -> bool {
    matches!(name.trim(), "Income" | "Speculation" | "9" | "Account 9")
}

/// Short puts and an explicit uncovered cover are refused.
pub fn refuse_contract(side: &str, put_call: &str, cover: &str) -> Option<&'static str> {
    let side = side.trim().to_ascii_lowercase();
    let right = put_call.trim().to_ascii_uppercase();
    if side == "short" && right == "P" {
        return Some("short_put_refused");
    }
    if cover.trim().eq_ignore_ascii_case("none") || cover.trim().eq_ignore_ascii_case("uncovered")
    {
        return Some("uncovered_refused");
    }
    if side == "short" && right != "C" {
        return Some("uncovered_refused");
    }
    None
}

pub fn week_category(side: &str, put_call: &str) -> &'static str {
    let side = side.trim().to_ascii_lowercase();
    let right = put_call.trim().to_ascii_uppercase();
    if side == "short" && right == "C" {
        "Covered Call"
    } else if right == "P" {
        "Put"
    } else {
        "Call"
    }
}

pub fn week_start_of(day: &str) -> Option<String> {
    let date = parse_iso_day(day)?;
    Some(week_containing(date).start.format("%Y-%m-%d").to_string())
}

/// Short credit is positive. Long debit is negative. Prior balance has no week.
/// Shares promised to an open contract, in the lot's quantity units.
pub fn free_units(remaining_units: i64, quantity_scale: u8, reserved_shares: i64) -> i64 {
    let reserved_units = reserved_shares.saturating_mul(10i64.pow(u32::from(quantity_scale)));
    remaining_units.saturating_sub(reserved_units)
}

pub fn posts_for_open(
    side: &str,
    put_call: &str,
    open_on: &str,
    open_premium_minor: Option<i64>,
    prior_balance_minor: i64,
) -> Result<Vec<PremiumPost>, &'static str> {
    let category = week_category(side, put_call).to_string();
    let mut posts = Vec::new();
    if prior_balance_minor != 0 {
        posts.push(PremiumPost {
            week_start: String::new(),
            category: category.clone(),
            amount_minor: prior_balance_minor,
            reason: "prior_balance".into(),
        });
    }
    if let Some(premium) = open_premium_minor {
        let week = week_start_of(open_on).ok_or("bad_open_on")?;
        let signed = if side.trim().eq_ignore_ascii_case("long") {
            -premium.abs()
        } else {
            premium.abs()
        };
        posts.push(PremiumPost {
            week_start: week,
            category,
            amount_minor: signed,
            reason: "open".into(),
        });
    }
    Ok(posts)
}

pub fn post_for_close(
    side: &str,
    put_call: &str,
    closed_on: &str,
    open_premium_minor: i64,
    close_premium_minor: i64,
) -> Result<PremiumPost, &'static str> {
    let week = week_start_of(closed_on).ok_or("bad_closed_on")?;
    let amount = if side.trim().eq_ignore_ascii_case("long") {
        close_premium_minor - open_premium_minor
    } else {
        open_premium_minor - close_premium_minor
    };
    Ok(PremiumPost {
        week_start: week,
        category: week_category(side, put_call).to_string(),
        amount_minor: amount,
        reason: "close".into(),
    })
}

fn cheaper(a: &SharePiece, b: &SharePiece) -> std::cmp::Ordering {
    let left = a.cost_minor.saturating_mul(b.shares);
    let right = b.cost_minor.saturating_mul(a.shares);
    left.cmp(&right)
        .then_with(|| a.opened_on.cmp(&b.opened_on))
        .then_with(|| a.lot_id.cmp(&b.lot_id))
}

/// Cheapest free fragments first, then open date, then lot id.
pub fn take_cover(
    mut pieces: Vec<SharePiece>,
    shares_needed: i64,
) -> Result<(Vec<CoverTake>, Vec<SharePiece>), &'static str> {
    if shares_needed <= 0 {
        return Err("bad_qty");
    }
    pieces.retain(|piece| piece.shares > 0);
    pieces.sort_by(cheaper);
    let mut need = shares_needed;
    let mut taken = Vec::new();
    for piece in &mut pieces {
        if need == 0 {
            break;
        }
        let take = piece.shares.min(need);
        let cost = if take == piece.shares {
            piece.cost_minor
        } else {
            piece.cost_minor.saturating_mul(take) / piece.shares
        };
        piece.shares -= take;
        piece.cost_minor -= cost;
        need -= take;
        taken.push(CoverTake {
            lot_id: piece.lot_id.clone(),
            shares: take,
            cost_minor: cost,
        });
    }
    if need > 0 {
        return Err("uncovered_refused");
    }
    pieces.retain(|piece| piece.shares > 0);
    Ok((taken, pieces))
}

pub fn cover_count(mut pieces: Vec<SharePiece>, shares_per: i64) -> (u32, i64) {
    let mut covers = 0u32;
    loop {
        let shares: i64 = pieces.iter().map(|piece| piece.shares).sum();
        if shares < shares_per {
            return (covers, shares);
        }
        match take_cover(pieces.clone(), shares_per) {
            Ok((_, rest)) => {
                covers += 1;
                pieces = rest;
            }
            Err(_) => {
                let left: i64 = pieces.iter().map(|piece| piece.shares).sum();
                return (covers, left);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn piece(id: &str, opened: &str, shares: i64, cost_minor: i64) -> SharePiece {
        SharePiece {
            lot_id: id.into(),
            opened_on: opened.into(),
            shares,
            cost_minor,
        }
    }

    fn account_9_tsll() -> Vec<SharePiece> {
        vec![
            piece("35bd5943", "2023-08-24", 10, 13_900),
            piece("7f075b6d", "2023-08-24", 10, 13_936),
            piece("23b99fa6", "2023-08-24", 20, 28_200),
            piece("bee58a19", "2023-08-24", 20, 28_440),
            piece("a8846a4b", "2023-08-24", 10, 14_240),
            piece("af2e5b03", "2023-08-23", 10, 14_390),
            piece("f3331717", "2023-08-23", 20, 29_280),
            piece("93a61b41", "2024-01-05", 100, 147_300),
            piece("d79d96a1", "2023-09-25", 30, 44_250),
            piece("c1ddfedc", "2023-09-26", 13, 19_430),
            piece("56f8f1a1", "2023-09-05", 10, 14_990),
            piece("0d9ed693", "2023-09-25", 30, 45_000),
            piece("1925eb12", "2023-09-25", 17, 25_670),
            piece("8114ac6a", "2023-09-25", 13, 19_630),
            piece("77806b9b", "2023-09-22", 16, 24_240),
            piece("176014e5", "2023-09-01", 10, 15_420),
            piece("b17ba2d6", "2023-09-06", 10, 15_750),
            piece("e3e270df", "2023-09-22", 21, 33_390),
            piece("02d02555", "2023-09-22", 9, 14_310),
            piece("f00a3faf", "2023-09-06", 10, 15_910),
            piece("99164a3f", "2023-09-01", 20, 31_940),
            piece("a7a5c628", "2023-09-01", 20, 31_980),
            piece("1c6a8597", "2023-08-30", 10, 16_310),
            piece("ff180288", "2023-08-29", 10, 16_340),
            piece("e8573e2c", "2023-09-20", 10, 17_000),
            piece("488ed3fd", "2023-09-18", 11, 18_810),
            piece("134d2c61", "2023-09-18", 14, 23_940),
            piece("1b96b454", "2023-09-20", 16, 27_440),
            piece("b7c0991e", "2023-09-20", 10, 17_400),
            piece("297a126f", "2023-09-15", 100, 180_000),
            piece("79f1ef70", "2023-09-25", 30, 45_528),
        ]
    }

    #[test]
    fn ira_names_and_refused_paths() {
        assert!(account_allowed("Income"));
        assert!(account_allowed("Speculation"));
        assert!(account_allowed("9"));
        assert!(account_allowed("Account 9"));
        for name in ["Car", "Health", "Robinhood", "FI Roth", "Energy"] {
            assert!(!account_allowed(name), "{name}");
        }
        assert_eq!(
            refuse_contract("short", "P", ""),
            Some("short_put_refused")
        );
        assert_eq!(
            refuse_contract("short", "C", "none"),
            Some("uncovered_refused")
        );
        assert_eq!(refuse_contract("short", "C", ""), None);
        assert_eq!(refuse_contract("long", "C", ""), None);
        assert_eq!(refuse_contract("long", "P", ""), None);
    }

    #[test]
    fn premium_posts_in_the_cash_week_and_prior_has_none() {
        let posts = posts_for_open("short", "C", "2026-10-06", Some(11_500), 26_400).unwrap();
        assert_eq!(posts[0].week_start, "");
        assert_eq!(posts[0].amount_minor, 26_400);
        assert_eq!(posts[0].reason, "prior_balance");
        assert_eq!(posts[1].week_start, "2026-10-03");
        assert_eq!(posts[1].amount_minor, 11_500);
        assert_eq!(posts[1].category, "Covered Call");
        let long = posts_for_open("long", "C", "2026-10-06", Some(11_500), 0).unwrap();
        assert_eq!(long.len(), 1);
        assert_eq!(long[0].amount_minor, -11_500);
        assert_eq!(long[0].category, "Call");
        let close = post_for_close("short", "C", "2026-10-09", 11_500, 4_000).unwrap();
        assert_eq!(close.week_start, "2026-10-03");
        assert_eq!(close.amount_minor, 7_500);
    }

    #[test]
    fn account_9_tsll_has_six_covers_and_the_first_is_the_cheap_fragments() {
        let lots = account_9_tsll();
        assert_eq!(lots.iter().map(|lot| lot.shares).sum::<i64>(), 640);
        let (count, leftover) = cover_count(lots.clone(), 100);
        assert_eq!(count, 6);
        assert_eq!(leftover, 40);
        let (first, rest) = take_cover(lots, 100).unwrap();
        assert_eq!(
            first.iter().map(|take| take.cost_minor).sum::<i64>(),
            142_386
        );
        assert!(first.iter().all(|take| take.lot_id != "93a61b41"));
        let (second, _) = take_cover(rest, 100).unwrap();
        assert_eq!(
            second.iter().map(|take| take.cost_minor).sum::<i64>(),
            147_300
        );
        let income = vec![piece("income-lot", "2023-09-06", 49, 79_919)];
        assert!(take_cover(income, 100).is_err());
    }
}
