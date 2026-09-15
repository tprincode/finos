# Shopping Cart — implementation design

Authority: `docs/Authority/domains/Shopping_Cart_Target_Architecture_v1.0_2026-08-14.docx` (LOCKED 14 Aug 2026). Reviews: `docs/Authority/cursor-shopping-cart-prompt-2026-09-10.zip`. Pack: [shopping-cart-grok-review.md](shopping-cart-grok-review.md). Canvas: Shopping Cart design beside chat.

**Status:** SC-1 named (`build`). Draft + evaluate + agree + confirm sell + Add Lot prefill. Do not start CM-4. Keep M6 `CartItemAdd` / `CartGet`.

## Purpose

Pre-trade what-if. **Deploying cash is a swap:** named-lot sells (SPAXX / other CASH / any position) plus researched buys. Income first, principal second. Leftover cash **still earns** the money-market plan. Drafts do not change Holdings, ROI, or Income Plan (BR-SC-019).

## What already exists

| Piece | Limit |
| --- | --- |
| `CartItemAdd` / `CartGet` | M6 symbol + qty. Keep until that golden is replaced. |
| Scenario host | `application-core` / `cart.rs` + SQLite `0033_cart_scenario`. |
| UI | `apps/desktop/src/features/shopping-cart/` + shared pickers. |
| Sell | `CartExecuteSell` → `ActivityPost` + `LotAssign`. |
| Buy | Add Lot prefilled → owner `LotOpen` → `CartExecuteBuyStep`. |

## Fat-file ban (required)

| May receive new cart code | Must not |
| --- | --- |
| `apps/desktop/src/features/shopping-cart/` (screen, step rail, afford, income, mix, tradeoff, execute) | Cart grid or cart math in `App.tsx` |
| `apps/desktop/src/features/shared/pickers/` (extract account, lot, researched-symbol only) | Copy those forms into the cart folder |
| `crates/application-core/src/cart.rs` (or `cart/`) | Cart SQL or evaluate in `queries.rs` |
| Extend `financial-domain` and `storage-sqlite` cart modules | Silent `LotOpen` from Evaluate |

`App.tsx`: menu + `setScreen("shopping-cart")` only. `queries.rs`: dispatch into the host module only. Do not split the rest of `App.tsx` or move weekly actuals / collectors / Income Plan in this slice.

## First slice — when named SC-1

One account, one as-of, multi-sell + multi-buy, whole shares. Owner picks named lots (SPAXX included), adds researched buys, sees afford + keep-vs-swap income + two mix bars (cash off the bar). Save is a draft. Agree freezes, confirm sell, then Add Lot prefilled. Owner clicks Open lot.

Pass:

1. Evaluate uses frozen plan + price/par snapshots (as-of + source). Missing stays unknown.
2. `lot_id` required. No FIFO. Qty may be partial. Spend > live remaining → `insufficient_lot_qty`; Evaluate may still return intent numbers.
3. **Leftover-keeps-yield.** Net annual = buy plan − plan on **dollars spent**. Weekly / monthly = annual ÷ 52 / ÷ 12.
4. Cash off the risk denominator (BR-SC-011).
5. Cash-floor warn (Income $5,000, Car $4,000, Health $200 only). Override reason required. No invented floors for other accounts.
6. Mix-worsens → both sentences + typed reason (BR-SC-013).
7. Draft does not change `DividendGet` / `BasisGet` / Income Plan.
8. Save orange while dirty.
9. Evaluate never `LotOpen`.

Illustrative 10 Sep numbers (docs only; goldens use fixture remaining):

- Covering: spend $59.90 of $65.77 SPAXX → 2 HAKY $59.90, leftover $5.87 still earns. Surrendered **$2.00**. HAKY **$9.12**. Net **+$7.12**. Not +$6.92.
- Intent: $340 → 11 HAKY, leftover-keeps-yield net **+$39.16**. Agree `insufficient_lot_qty`.

## Storage

`cart_scenario`, `cart_sell_line` (`lot_id` required), `cart_buy_line`, `cart_eval_snapshot`, `cart_execute_step`.

## Commands (SC-1)

| Name | Writes |
| --- | --- |
| `CartScenarioCreate` | Draft header, kind `Swap` |
| `CartSellLineAdd` | lot_id required; snapshots |
| `CartBuyLineAdd` | researched security; adds one blend row (qty starts at 1) |
| `CartBuyLineQtySet` | in-row whole qty; may exceed budget; Agree still blocks |
| `CartScenarioEvaluate` | Query. Frozen rollup. |
| `CartScenarioSave` / discard | Draft only |
| `CartScenarioAgree` | Freeze. Blocked on unresolved lots, negative cash, or unwarned floor/mix. |
| Execute sell / buy | Existing `ActivityPost` + `LotAssign`; existing Add Lot / `LotOpen` |

Keep `CartItemAdd` / `CartGet` until the M6 golden is replaced.

## SC-2 / SC-3 (named)

Lot cost table (dual unit cost + perf/tax P/L, lowest-cost or largest-tax-loss rank). Named drafts + Keep column monthly compare. Tax **dollar** estimate and cross-account workbook stay later.

## Later (not SC-2/SC-3)

Tranches, portfolio-wide mix, fill paste, cumulative ledger P/L (BR-SC-005, 010, 016–018, 020–023). Fees unless entered. MAGI tax estimate.

## Locks

- No FIFO. No UI SQL. `App.tsx` stays `LocalTauriFinanceClient`.
- No MAGI oracle rewrite. Do not chase Save-facts a11y.
- HAKY is Core. Do not treat $340 as stored FI Roth SPAXX remaining.
- Do not put leftover cash yield to zero if those dollars remain in the account.
