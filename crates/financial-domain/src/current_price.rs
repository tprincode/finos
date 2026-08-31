//! CurrentPrice selection (Last Price TR-LP-07–09). Never substitute zero.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriceFreshness {
    Current,
    ManualOverride,
    Stale,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuoteObservation {
    pub price_minor: i64,
    pub scale: u8,
    pub accepted: bool,
    pub as_of: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverrideObservation {
    pub price_minor: i64,
    pub scale: u8,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentPrice {
    pub price_minor: Option<i64>,
    pub scale: u8,
    pub freshness: PriceFreshness,
}

/// Money-market NAV is always $1.00 (100 minor at scale 2).
pub const CASH_PAR_MINOR: i64 = 100;
pub const CASH_PAR_SCALE: u8 = 2;

pub fn is_cash(div_type: &str) -> bool {
    div_type.trim().eq_ignore_ascii_case("CASH")
}

/// Known money-market symbols that always use par (SPAXX/FDRXX/SWVXX).
pub fn is_cash_par_symbol(symbol: &str) -> bool {
    matches!(
        symbol.trim().to_ascii_uppercase().as_str(),
        "SPAXX" | "FDRXX" | "SWVXX"
    )
}

pub fn uses_cash_par(div_type: &str, symbol: &str) -> bool {
    is_cash(div_type) || is_cash_par_symbol(symbol)
}

/// Stable exception text for BR-CASH-01 (account + symbol + YYYY-MM).
pub fn missing_cash_dividend_message(account_name: &str, symbol: &str, month: &str) -> String {
    format!("missing cash dividend: {account_name} {symbol} {month}")
}

pub fn cash_par_current_price() -> CurrentPrice {
    CurrentPrice {
        price_minor: Some(CASH_PAR_MINOR),
        scale: CASH_PAR_SCALE,
        freshness: PriceFreshness::Current,
    }
}

fn valid_price(minor: i64) -> bool {
    minor > 0
}

/// Override (active, positive) > newest accepted quote (Current if as-of is today, else Stale) >
/// unavailable. Nonpositive quotes never become CurrentPrice.
pub fn select_current_price(
    price_override: Option<&OverrideObservation>,
    quotes_newest_first: &[QuoteObservation],
    today: &str,
) -> CurrentPrice {
    if let Some(over) = price_override {
        if over.active && valid_price(over.price_minor) {
            return CurrentPrice {
                price_minor: Some(over.price_minor),
                scale: over.scale,
                freshness: PriceFreshness::ManualOverride,
            };
        }
    }
    let accepted: Vec<&QuoteObservation> = quotes_newest_first
        .iter()
        .filter(|q| q.accepted && valid_price(q.price_minor))
        .collect();
    let Some(newest) = accepted.first() else {
        return CurrentPrice {
            price_minor: None,
            scale: 2,
            freshness: PriceFreshness::Unavailable,
        };
    };
    let freshness = if newest.as_of.len() >= 10 && today.len() >= 10 && newest.as_of[..10] == today[..10]
    {
        PriceFreshness::Current
    } else {
        PriceFreshness::Stale
    };
    CurrentPrice {
        price_minor: Some(newest.price_minor),
        scale: newest.scale,
        freshness,
    }
}

pub fn price_derived_valid(freshness: PriceFreshness) -> bool {
    matches!(
        freshness,
        PriceFreshness::Current | PriceFreshness::ManualOverride | PriceFreshness::Stale
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(minor: i64, accepted: bool, as_of: &str) -> QuoteObservation {
        QuoteObservation {
            price_minor: minor,
            scale: 2,
            accepted,
            as_of: as_of.to_string(),
        }
    }

    #[test]
    fn override_beats_quote_and_zero_is_unavailable() {
        let over = OverrideObservation {
            price_minor: 1250,
            scale: 2,
            active: true,
        };
        let quotes = [q(9999, true, "2026-08-21")];
        let selected = select_current_price(Some(&over), &quotes, "2026-08-21");
        assert_eq!(selected.freshness, PriceFreshness::ManualOverride);
        assert_eq!(selected.price_minor, Some(1250));

        let zero = select_current_price(None, &[q(0, true, "2026-08-21")], "2026-08-21");
        assert_eq!(zero.freshness, PriceFreshness::Unavailable);
        assert_eq!(zero.price_minor, None);
    }

    #[test]
    fn today_is_current_yesterday_is_stale() {
        let today = select_current_price(None, &[q(100, true, "2026-08-21")], "2026-08-21");
        assert_eq!(today.freshness, PriceFreshness::Current);
        let stale = select_current_price(None, &[q(100, true, "2026-08-20")], "2026-08-21");
        assert_eq!(stale.freshness, PriceFreshness::Stale);
        assert!(price_derived_valid(stale.freshness));
        assert!(!price_derived_valid(PriceFreshness::Unavailable));
    }

    #[test]
    fn cash_par_is_one_dollar_current() {
        let par = cash_par_current_price();
        assert_eq!(par.price_minor, Some(CASH_PAR_MINOR));
        assert_eq!(par.scale, CASH_PAR_SCALE);
        assert_eq!(par.freshness, PriceFreshness::Current);
        assert!(uses_cash_par("CASH", "SPAXX"));
        assert!(uses_cash_par("", "FDRXX"));
        assert!(uses_cash_par("CASH", "OTHER"));
        assert!(!uses_cash_par("DIV-1", "QYLD"));
    }
}
