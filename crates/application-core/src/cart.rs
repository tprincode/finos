//! Shopping Cart host. Evaluate math lives here and in financial-domain — not queries.rs.

use std::collections::HashSet;

use uuid::Uuid;

use crate::contracts::{CartScenarioBody, CartScenarioListBody};
use crate::ports::canonical::Canonical;
use crate::ports::platform::PlatformError;
use financial_domain::calculator::PaymentCadence;
use financial_domain::cart::{no_cash_account, plan_fwd_yield_bps};
use financial_domain::current_price::uses_cash_par;

const CASH_SYMBOLS: &[&str] = &["SPAXX", "CASH", "FDRXX", "SWVXX"];

fn is_named_cash(symbol: &str) -> bool {
    CASH_SYMBOLS.iter().any(|s| s.eq_ignore_ascii_case(symbol))
}

fn dollars_from_qty(qty_minor: i64, qty_scale: u8, unit_minor: i64) -> i64 {
    let denom = 10_i64.pow(u32::from(qty_scale));
    if denom == 0 {
        return 0;
    }
    (qty_minor * unit_minor) / denom
}

async fn cash_plan_yield_bps(
    canonical: &dyn Canonical,
    account_id: Uuid,
) -> Result<Option<i64>, PlatformError> {
    let basis = canonical.basis_get().await?;
    let securities = canonical.security_list().await?;
    let plans = canonical.plan_history_list().await?;
    let characteristics = canonical.position_characteristic_list().await?;
    let mut best: Option<(String, i64)> = None;
    for lot in &basis.lots {
        if lot.account_id != account_id || lot.remaining_quantity_minor <= 0 {
            continue;
        }
        let Some(security) = securities.iter().find(|s| s.security_id == lot.security_id) else {
            continue;
        };
        if !is_named_cash(&security.symbol) {
            continue;
        }
        let Some(plan) = plans
            .iter()
            .filter(|p| p.security_id == lot.security_id)
            .max_by(|a, b| a.effective_from.cmp(&b.effective_from))
        else {
            continue;
        };
        let ch = characteristics
            .iter()
            .find(|c| c.security_id == lot.security_id);
        let periods = ch
            .and_then(|c| PaymentCadence::parse(&c.payment_frequency))
            .and_then(PaymentCadence::periods)
            .map(i64::from)
            .filter(|p| *p > 0)
            .or_else(|| {
                (plan.planning_periods_per_year > 0)
                    .then_some(i64::from(plan.planning_periods_per_year))
            })
            .unwrap_or(0);
        let div = ch.map(|c| c.div_type.as_str()).unwrap_or("");
        let price_cents = if uses_cash_par(div, &security.symbol) {
            100
        } else {
            continue;
        };
        let Some(bps) = plan_fwd_yield_bps(
            plan.amount_per_share_minor,
            plan.amount_scale,
            periods,
            price_cents,
        ) else {
            continue;
        };
        let key = plan.effective_from.clone();
        if best.as_ref().is_none_or(|(prev, _)| key > *prev) {
            best = Some((key, bps));
        }
    }
    Ok(best.map(|(_, bps)| bps))
}

pub fn parse_funding_source(raw: &str) -> Result<String, PlatformError> {
    match raw.trim() {
        "" | "sellLots" => Ok("sellLots".into()),
        "accountCash" => Ok("accountCash".into()),
        "newDeposit" => Ok("newDeposit".into()),
        _ => Err(PlatformError::new(
            "invalid_funding_source",
            "funding must be sellLots, accountCash, or newDeposit",
        )),
    }
}

pub async fn scenario_create(
    canonical: &dyn Canonical,
    account_id: Uuid,
    as_of: String,
    cash_yield_bps: i64,
    name: String,
    funding_source: String,
) -> Result<CartScenarioBody, PlatformError> {
    let funding = parse_funding_source(&funding_source)?;
    let account = canonical.account_get(account_id).await?;
    if funding == "accountCash" && no_cash_account(&account.name) {
        return Err(PlatformError::new(
            "account_cash_not_offered",
            "Energy and Robinhood do not have a cash pile",
        ));
    }
    let bps = cash_plan_yield_bps(canonical, account_id)
        .await?
        .unwrap_or(cash_yield_bps);
    canonical
        .cart_scenario_create(account_id, account.name, as_of, bps, name, funding)
        .await
}

pub async fn sell_line_add(
    canonical: &dyn Canonical,
    scenario_id: Uuid,
    lot_id: Uuid,
    qty_minor: i64,
    unit_minor: i64,
    is_cash: bool,
) -> Result<CartScenarioBody, PlatformError> {
    if qty_minor <= 0 {
        return Err(PlatformError::new(
            "invalid_qty",
            "sell qty must be positive",
        ));
    }
    let lot = canonical.lot_get(lot_id).await?;
    let security = canonical.security_get(lot.security_id).await?;
    let qty_scale = lot.quantity_scale;
    let proceeds_minor = dollars_from_qty(qty_minor, qty_scale, unit_minor);
    let cash = is_cash || is_named_cash(&security.symbol);
    let (original_cost, perf_cost, tax_cost, perf_gain, tax_gain) = if cash {
        (None, None, None, None, None)
    } else {
        let perf_cost = financial_domain::money::to_usd_cents(
            financial_domain::lot::proportional_basis(
                lot.remaining_performance_minor,
                lot.remaining_quantity_minor,
                qty_minor,
            ),
            lot.scale,
        );
        let tax_cost = financial_domain::money::to_usd_cents(
            financial_domain::lot::proportional_basis(
                lot.remaining_tax_minor,
                lot.remaining_quantity_minor,
                qty_minor,
            ),
            lot.scale,
        );
        let (pg, tg) = financial_domain::lot::lifetime_gains(proceeds_minor, perf_cost, tax_cost);
        (
            Some(perf_cost),
            Some(perf_cost),
            Some(tax_cost),
            Some(pg),
            Some(tg),
        )
    };
    canonical
        .cart_sell_line_add(
            scenario_id,
            lot_id,
            Some(lot.security_id),
            security.symbol,
            qty_minor,
            qty_scale,
            unit_minor,
            proceeds_minor,
            cash,
            original_cost,
            perf_cost,
            tax_cost,
            perf_gain,
            tax_gain,
        )
        .await
}

pub async fn buy_line_add(
    canonical: &dyn Canonical,
    scenario_id: Uuid,
    security_id: Uuid,
    qty_whole: i64,
    last_minor: i64,
    plan_annual_minor: Option<i64>,
) -> Result<CartScenarioBody, PlatformError> {
    if qty_whole <= 0 {
        return Err(PlatformError::new(
            "invalid_qty",
            "buy qty must be whole shares",
        ));
    }
    let security = canonical.security_get(security_id).await?;
    let spend_minor = qty_whole * last_minor;
    canonical
        .cart_buy_line_add(
            scenario_id,
            security_id,
            security.symbol,
            qty_whole,
            last_minor,
            spend_minor,
            plan_annual_minor,
        )
        .await
}

pub async fn buy_line_qty_set(
    canonical: &dyn Canonical,
    scenario_id: Uuid,
    line_id: Uuid,
    qty_whole: i64,
    last_minor: Option<i64>,
    plan_annual_minor: Option<i64>,
) -> Result<CartScenarioBody, PlatformError> {
    if qty_whole <= 0 {
        return Err(PlatformError::new(
            "invalid_qty",
            "buy qty must be whole shares",
        ));
    }
    let scene = canonical.cart_scenario_get(scenario_id).await?;
    if scene.status != "draft" {
        return Err(PlatformError::new(
            "not_draft",
            "only a draft qty can change",
        ));
    }
    let Some(line) = scene.buy_lines.iter().find(|l| l.line_id == line_id) else {
        return Err(PlatformError::new(
            "missing_line",
            "buy line not on this draft",
        ));
    };
    let last = last_minor.filter(|p| *p > 0).unwrap_or(line.last_minor);
    if last <= 0 {
        return Err(PlatformError::new(
            "last_unknown",
            "last price unknown — will not invent $0",
        ));
    }
    let spend_minor = qty_whole * last;
    let plan = plan_annual_minor.or_else(|| {
        line.plan_annual_minor.map(|p| {
            if line.qty_whole <= 0 {
                p
            } else {
                p * qty_whole / line.qty_whole
            }
        })
    });
    canonical
        .cart_buy_line_set(line_id, qty_whole, last, spend_minor, plan)
        .await?;
    canonical.cart_scenario_get(scenario_id).await
}

pub async fn evaluate(
    canonical: &dyn Canonical,
    scenario_id: Uuid,
) -> Result<CartScenarioBody, PlatformError> {
    let mut scene = canonical.cart_scenario_get(scenario_id).await?;
    scene = refresh_buy_prices(canonical, scene).await?;
    let basis = canonical.basis_get().await.ok();
    let account_cash = scene.funding_source == "accountCash";
    let mut live_short = false;
    let mut seen = HashSet::new();
    let mut remaining_minor = 0_i64;
    if account_cash {
        remaining_minor = crate::cash_pile::pile_for_account(canonical, scene.account_id)
            .await?
            .map(|p| p.dollars_minor)
            .unwrap_or(0);
    }
    for sell in scene.sell_lines.iter().filter(|_| !account_cash) {
        if let Some(basis) = &basis {
            if let Some(lot) = basis.lots.iter().find(|l| l.lot_id == sell.lot_id) {
                let lot_dollars = dollars_from_qty(
                    lot.remaining_quantity_minor,
                    lot.quantity_scale,
                    sell.unit_minor,
                );
                let sell_dollars =
                    dollars_from_qty(sell.qty_minor, sell.qty_scale, sell.unit_minor);
                if sell_dollars > lot_dollars || sell.qty_minor > lot.remaining_quantity_minor {
                    live_short = true;
                }
                if seen.insert(sell.lot_id) {
                    remaining_minor += lot_dollars;
                }
            } else {
                live_short = true;
                if seen.insert(sell.lot_id) {
                    remaining_minor += sell.proceeds_minor;
                }
            }
        } else if seen.insert(sell.lot_id) {
            remaining_minor += sell.proceeds_minor;
        }
    }
    if !account_cash && basis.is_none() && remaining_minor == 0 {
        remaining_minor = scene.sell_lines.iter().map(|s| s.proceeds_minor).sum();
    }
    let spend_minor: i64 = scene.buy_lines.iter().map(|b| b.spend_minor).sum();
    let plans_known = scene
        .buy_lines
        .iter()
        .all(|b| b.plan_annual_minor.is_some());
    let buy_annual = scene
        .buy_lines
        .iter()
        .filter_map(|b| b.plan_annual_minor)
        .sum::<i64>();
    let yield_bps = cash_plan_yield_bps(canonical, scene.account_id)
        .await?
        .or_else(|| (scene.cash_yield_bps > 0).then_some(scene.cash_yield_bps));
    let eval = financial_domain::cart::evaluate_swap(
        remaining_minor,
        spend_minor,
        buy_annual,
        yield_bps.unwrap_or(0),
        &scene.account_name,
    );
    let insufficient = eval.insufficient_lot_qty || live_short;
    let plans_ok = plans_known || scene.buy_lines.is_empty();
    let yield_ok = yield_bps.is_some();
    let (buy_a, surr, left, net, month, week) = match (plans_ok, yield_ok) {
        (true, true) => (
            Some(eval.buy_annual_minor),
            Some(eval.surrendered_annual_minor),
            Some(eval.leftover_annual_minor),
            Some(eval.net_annual_minor),
            Some(eval.net_monthly_minor),
            Some(eval.net_weekly_minor),
        ),
        (true, false) => (Some(eval.buy_annual_minor), None, None, None, None, None),
        (false, _) => (None, None, None, None, None, None),
    };
    canonical
        .cart_eval_save(
            scenario_id,
            eval.remaining_minor,
            eval.spend_minor,
            eval.leftover_minor,
            buy_a,
            surr,
            left,
            net,
            month,
            week,
            insufficient,
            eval.cash_floor_warn,
            scene.status,
        )
        .await
}

pub async fn agree(
    canonical: &dyn Canonical,
    scenario_id: Uuid,
    override_reason: Option<String>,
    mix_worsens: bool,
) -> Result<CartScenarioBody, PlatformError> {
    let evaluated = evaluate(canonical, scenario_id).await?;
    let Some(eval) = &evaluated.eval else {
        return Err(PlatformError::new(
            "evaluate_required",
            "evaluate before agree",
        ));
    };
    let account_cash = evaluated.funding_source == "accountCash";
    // A cash plan may exceed the pile until execution. The fill checks the last quote.
    if !account_cash && eval.insufficient_lot_qty {
        return Err(PlatformError::new(
            "insufficient_lot_qty",
            "named remaining does not cover spend",
        ));
    }
    if evaluated.funding_source == "accountCash" && evaluated.buy_lines.is_empty() {
        return Err(PlatformError::new(
            "no_buy",
            "account cash needs a researched buy",
        ));
    }
    let listed = canonical.cart_scenario_list(evaluated.account_id).await?;
    if listed
        .items
        .iter()
        .any(|s| s.scenario_id != scenario_id && (s.status == "agreed" || s.status == "executing"))
    {
        return Err(PlatformError::new(
            "scenario_already_agreed",
            "agree exactly one draft on this account",
        ));
    }
    if !account_cash && eval.leftover_minor < 0 {
        return Err(PlatformError::new(
            "negative_cash",
            "spend exceeds proceeds",
        ));
    }
    let needs_reason = (eval.cash_floor_warn && !(account_cash && eval.leftover_minor < 0))
        || mix_worsens;
    if needs_reason && override_reason.as_deref().unwrap_or("").trim().is_empty() {
        return Err(PlatformError::new(
            if eval.cash_floor_warn {
                "cash_floor_override_required"
            } else {
                "mix_override_required"
            },
            "typed reason required to sell below the cash floor or worsen mix",
        ));
    }
    canonical
        .cart_scenario_agree(scenario_id, override_reason)
        .await
}

pub async fn execute_sell(
    canonical: &dyn Canonical,
    scenario_id: Uuid,
    occurred_on: String,
) -> Result<CartScenarioBody, PlatformError> {
    let scene = canonical.cart_scenario_get(scenario_id).await?;
    if scene.status != "agreed" && scene.status != "executing" {
        return Err(PlatformError::new(
            "not_agreed",
            "agree before confirm sell",
        ));
    }
    for sell in &scene.sell_lines {
        let posted = canonical
            .activity_post(
                scene.account_id,
                sell.security_id,
                "sell".into(),
                Some(sell.proceeds_minor),
                2,
                occurred_on.clone(),
                None,
                None,
                None,
            )
            .await?;
        let assigned = canonical
            .lot_assign(
                sell.lot_id,
                posted.activity_id,
                sell.qty_minor,
                sell.qty_scale,
            )
            .await?;
        canonical
            .cart_execute_step_add(
                scenario_id,
                "sell".into(),
                Some(posted.activity_id),
                Some(assigned.assignment_id),
                Some(sell.lot_id),
            )
            .await?;
    }
    canonical.cart_scenario_get(scenario_id).await
}

pub async fn discard(canonical: &dyn Canonical, scenario_id: Uuid) -> Result<(), PlatformError> {
    canonical.cart_scenario_discard(scenario_id).await
}

pub async fn sell_line_remove(
    canonical: &dyn Canonical,
    scenario_id: Uuid,
    line_id: Uuid,
) -> Result<CartScenarioBody, PlatformError> {
    canonical.cart_sell_line_remove(scenario_id, line_id).await
}

pub async fn sell_symbol_clear(
    canonical: &dyn Canonical,
    scenario_id: Uuid,
    symbol: String,
) -> Result<CartScenarioBody, PlatformError> {
    canonical
        .cart_sell_symbol_clear(scenario_id, symbol)
        .await
}

pub async fn buy_line_remove(
    canonical: &dyn Canonical,
    scenario_id: Uuid,
    line_id: Uuid,
) -> Result<CartScenarioBody, PlatformError> {
    canonical.cart_buy_line_remove(scenario_id, line_id).await
}

pub async fn scenario_slot_add(
    canonical: &dyn Canonical,
    scenario_id: Uuid,
) -> Result<CartScenarioBody, PlatformError> {
    canonical.cart_scenario_slot_add(scenario_id).await
}

pub async fn plan_deposit_set(
    canonical: &dyn Canonical,
    scenario_id: Uuid,
    deposit_minor: i64,
) -> Result<CartScenarioBody, PlatformError> {
    canonical
        .cart_plan_deposit_set(scenario_id, deposit_minor)
        .await
}

pub async fn list(
    canonical: &dyn Canonical,
    account_id: Uuid,
) -> Result<CartScenarioListBody, PlatformError> {
    canonical.cart_scenario_list(account_id).await
}

pub async fn rename(
    canonical: &dyn Canonical,
    scenario_id: Uuid,
    name: String,
) -> Result<CartScenarioBody, PlatformError> {
    canonical.cart_scenario_rename(scenario_id, name).await
}

pub async fn duplicate(
    canonical: &dyn Canonical,
    scenario_id: Uuid,
) -> Result<CartScenarioBody, PlatformError> {
    canonical.cart_scenario_duplicate(scenario_id).await
}

async fn refresh_buy_prices(
    canonical: &dyn Canonical,
    scene: CartScenarioBody,
) -> Result<CartScenarioBody, PlatformError> {
    let mut changed = false;
    for buy in &scene.buy_lines {
        let live = canonical
            .current_price_get(buy.security_id, scene.as_of.clone())
            .await
            .ok()
            .and_then(|p| p.price_minor)
            .filter(|p| *p > 0);
        let Some(last) = live else {
            continue;
        };
        if last == buy.last_minor {
            continue;
        }
        canonical
            .cart_buy_line_price_set(buy.line_id, last, buy.qty_whole * last)
            .await?;
        changed = true;
    }
    if changed {
        canonical.cart_scenario_get(scene.scenario_id).await
    } else {
        Ok(scene)
    }
}

pub async fn execute_fill(
    canonical: &dyn Canonical,
    scenario_id: Uuid,
    occurred_on: String,
    fills: Vec<(Uuid, i64)>,
) -> Result<CartScenarioBody, PlatformError> {
    let scene = canonical.cart_scenario_get(scenario_id).await?;
    if scene.status != "agreed" && scene.status != "executing" {
        return Err(PlatformError::new(
            "not_agreed",
            "agree before confirm fill",
        ));
    }
    if scene.funding_source != "accountCash" {
        return Err(PlatformError::new(
            "wrong_funding",
            "confirm fill is for account cash",
        ));
    }
    if scene.buy_lines.is_empty() {
        return Err(PlatformError::new(
            "no_buy",
            "account cash needs a researched buy",
        ));
    }
    let mut spend_minor = 0_i64;
    for buy in &scene.buy_lines {
        let fill = fills
            .iter()
            .find(|(id, _)| *id == buy.line_id)
            .map(|(_, p)| *p)
            .unwrap_or(0);
        if fill <= 0 {
            return Err(PlatformError::new(
                "fill_required",
                "confirm the price paid for each buy",
            ));
        }
        spend_minor += buy.qty_whole * fill;
    }
    let Some(pile) = crate::cash_pile::pile_for_account(canonical, scene.account_id).await? else {
        return Err(PlatformError::new(
            "missing_cash_pile",
            "this account has no money-market position",
        ));
    };
    if spend_minor > pile.dollars_minor {
        return Err(PlatformError::new(
            "insufficient_account_cash",
            "live cash does not cover the confirmed fill",
        ));
    }
    let qty = financial_domain::cart::cash_qty_for_dollars(spend_minor, pile.quantity_scale);
    if qty <= 0 || qty > pile.remaining_qty_minor {
        return Err(PlatformError::new(
            "insufficient_account_cash",
            "live cash qty does not cover the confirmed fill",
        ));
    }
    for buy in &scene.buy_lines {
        if let Some((_, fill)) = fills.iter().find(|(id, _)| *id == buy.line_id) {
            canonical
                .cart_buy_line_price_set(buy.line_id, *fill, buy.qty_whole * *fill)
                .await?;
        }
    }
    let posted = canonical
        .activity_post(
            scene.account_id,
            Some(pile.security_id),
            "withdrawal".into(),
            Some(spend_minor),
            2,
            occurred_on,
            None,
            None,
            None,
        )
        .await?;
    let assigned = canonical
        .lot_assign(pile.lot_id, posted.activity_id, qty, pile.quantity_scale)
        .await?;
    canonical
        .cart_execute_step_add(
            scenario_id,
            "cash".into(),
            Some(posted.activity_id),
            Some(assigned.assignment_id),
            Some(pile.lot_id),
        )
        .await?;
    canonical.cart_scenario_get(scenario_id).await
}

pub async fn execute_buy_step(
    canonical: &dyn Canonical,
    scenario_id: Uuid,
    lot_id: Uuid,
) -> Result<CartScenarioBody, PlatformError> {
    let scene = canonical.cart_scenario_get(scenario_id).await?;
    if scene.status != "agreed" && scene.status != "executing" {
        return Err(PlatformError::new(
            "not_agreed",
            "agree before recording the buy lot",
        ));
    }
    canonical
        .cart_execute_step_add(scenario_id, "buy".into(), None, None, Some(lot_id))
        .await?;
    canonical.cart_scenario_get(scenario_id).await
}
