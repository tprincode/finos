//! One cash pile per account. Qty at $1.00 is the only stored amount.

use uuid::Uuid;

use crate::contracts::{CashLedgerBody, CashLedgerEntryBody, CashPileBody};
use crate::ports::canonical::Canonical;
use crate::ports::platform::PlatformError;
use financial_domain::cart::{cash_dollars_minor, cash_qty_for_dollars, no_cash_account};
use financial_domain::current_price::is_cash_par_symbol;

const LEDGER_TYPES: &[&str] = &["deposit", "withdrawal"];

pub struct CashPile {
    pub lot_id: Uuid,
    pub security_id: Uuid,
    pub symbol: String,
    pub remaining_qty_minor: i64,
    pub quantity_scale: u8,
    pub dollars_minor: i64,
}

pub async fn pile_for_account(
    canonical: &dyn Canonical,
    account_id: Uuid,
) -> Result<Option<CashPile>, PlatformError> {
    let account = canonical.account_get(account_id).await?;
    if no_cash_account(&account.name) {
        return Ok(None);
    }
    let basis = canonical.basis_get().await?;
    let securities = canonical.security_list().await?;
    let mut best: Option<CashPile> = None;
    for lot in &basis.lots {
        if lot.account_id != account_id || lot.remaining_quantity_minor <= 0 {
            continue;
        }
        let Some(security) = securities.iter().find(|s| s.security_id == lot.security_id) else {
            continue;
        };
        if !is_cash_par_symbol(&security.symbol) && !security.symbol.eq_ignore_ascii_case("CASH") {
            continue;
        }
        let dollars = cash_dollars_minor(lot.remaining_quantity_minor, lot.quantity_scale);
        let take = best
            .as_ref()
            .is_none_or(|prev| dollars > prev.dollars_minor);
        if take {
            best = Some(CashPile {
                lot_id: lot.lot_id,
                security_id: lot.security_id,
                symbol: security.symbol.clone(),
                remaining_qty_minor: lot.remaining_quantity_minor,
                quantity_scale: lot.quantity_scale,
                dollars_minor: dollars,
            });
        }
    }
    Ok(best)
}

fn basis_for_dollars(dollars_minor: i64, scale: u8) -> i64 {
    if scale == 2 {
        return dollars_minor;
    }
    if scale > 2 {
        return dollars_minor.saturating_mul(10_i64.pow(u32::from(scale - 2)));
    }
    dollars_minor / 10_i64.pow(u32::from(2 - scale))
}

/// The cash lot remaining quantity is the account’s current cash. Saturday entry sets it.
pub async fn set_pile_dollars(
    canonical: &dyn Canonical,
    account_id: Uuid,
    dollars_minor: i64,
) -> Result<(), PlatformError> {
    if dollars_minor < 0 {
        return Err(PlatformError::new(
            "invalid_qty",
            "cash total cannot be negative",
        ));
    }
    let Some(pile) = pile_for_account(canonical, account_id).await? else {
        return Ok(());
    };
    let lot = canonical.lot_get(pile.lot_id).await?;
    let qty = cash_qty_for_dollars(dollars_minor, pile.quantity_scale);
    let basis = basis_for_dollars(dollars_minor, lot.scale);
    canonical.lot_cash_set(pile.lot_id, qty, basis).await?;
    Ok(())
}

pub async fn pile_get(
    canonical: &dyn Canonical,
    account_id: Uuid,
) -> Result<CashPileBody, PlatformError> {
    let account = canonical.account_get(account_id).await?;
    match pile_for_account(canonical, account_id).await? {
        Some(pile) => Ok(CashPileBody {
            found: true,
            account_id,
            account_name: account.name,
            lot_id: Some(pile.lot_id),
            security_id: Some(pile.security_id),
            symbol: pile.symbol,
            remaining_qty_minor: pile.remaining_qty_minor,
            quantity_scale: pile.quantity_scale,
            dollars_minor: pile.dollars_minor,
        }),
        None => Ok(CashPileBody {
            found: false,
            account_id,
            account_name: account.name,
            lot_id: None,
            security_id: None,
            symbol: String::new(),
            remaining_qty_minor: 0,
            quantity_scale: 2,
            dollars_minor: 0,
        }),
    }
}

pub async fn ledger_get(
    canonical: &dyn Canonical,
    account_id: Uuid,
) -> Result<CashLedgerBody, PlatformError> {
    let pile = pile_for_account(canonical, account_id).await?;
    let Some(pile) = pile else {
        return Ok(CashLedgerBody {
            account_id,
            symbol: String::new(),
            dollars_minor: 0,
            entries: Vec::new(),
        });
    };
    let activities = canonical.activity_list().await.unwrap_or_default();
    let mut entries: Vec<CashLedgerEntryBody> = activities
        .into_iter()
        .filter(|a| {
            a.account_id == account_id
                && a.security_id == Some(pile.security_id)
                && LEDGER_TYPES
                    .iter()
                    .any(|t| t.eq_ignore_ascii_case(&a.activity_type))
        })
        .map(|a| CashLedgerEntryBody {
            activity_id: a.activity_id,
            activity_type: a.activity_type,
            amount_minor: a.amount_minor,
            scale: a.scale,
            occurred_on: a.occurred_on,
        })
        .collect();
    entries.sort_by(|a, b| {
        a.occurred_on
            .cmp(&b.occurred_on)
            .then(a.activity_id.cmp(&b.activity_id))
    });
    Ok(CashLedgerBody {
        account_id,
        symbol: pile.symbol,
        dollars_minor: pile.dollars_minor,
        entries,
    })
}

pub async fn deposit(
    canonical: &dyn Canonical,
    account_id: Uuid,
    amount_minor: i64,
    occurred_on: String,
) -> Result<CashPileBody, PlatformError> {
    if amount_minor <= 0 {
        return Err(PlatformError::new(
            "invalid_qty",
            "deposit must be positive",
        ));
    }
    let account = canonical.account_get(account_id).await?;
    if no_cash_account(&account.name) {
        return Err(PlatformError::new(
            "account_cash_not_offered",
            "Energy and Robinhood do not have a cash pile",
        ));
    }
    let Some(pile) = pile_for_account(canonical, account_id).await? else {
        return Err(PlatformError::new(
            "missing_cash_pile",
            "this account has no money-market position",
        ));
    };
    let qty = cash_qty_for_dollars(amount_minor, pile.quantity_scale);
    if qty <= 0 {
        return Err(PlatformError::new(
            "invalid_qty",
            "deposit is smaller than the pile step",
        ));
    }
    canonical
        .activity_post(
            account_id,
            Some(pile.security_id),
            "deposit".into(),
            Some(amount_minor),
            2,
            occurred_on,
            None,
            None,
            None,
        )
        .await?;
    canonical
        .lot_qty_add(pile.lot_id, qty, amount_minor)
        .await?;
    pile_get(canonical, account_id).await
}

pub async fn withdraw(
    canonical: &dyn Canonical,
    account_id: Uuid,
    amount_minor: i64,
    occurred_on: String,
) -> Result<CashPileBody, PlatformError> {
    if amount_minor <= 0 {
        return Err(PlatformError::new(
            "invalid_qty",
            "withdrawal must be positive",
        ));
    }
    let Some(pile) = pile_for_account(canonical, account_id).await? else {
        return Err(PlatformError::new(
            "missing_cash_pile",
            "this account has no money-market position",
        ));
    };
    if amount_minor > pile.dollars_minor {
        return Err(PlatformError::new(
            "insufficient_account_cash",
            "live cash does not cover the withdrawal",
        ));
    }
    let qty = cash_qty_for_dollars(amount_minor, pile.quantity_scale);
    if qty <= 0 || qty > pile.remaining_qty_minor {
        return Err(PlatformError::new(
            "insufficient_account_cash",
            "live cash qty does not cover the withdrawal",
        ));
    }
    let posted = canonical
        .activity_post(
            account_id,
            Some(pile.security_id),
            "withdrawal".into(),
            Some(amount_minor),
            2,
            occurred_on,
            None,
            None,
            None,
        )
        .await?;
    canonical
        .lot_assign(pile.lot_id, posted.activity_id, qty, pile.quantity_scale)
        .await?;
    pile_get(canonical, account_id).await
}
