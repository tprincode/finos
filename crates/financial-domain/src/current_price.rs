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
}
