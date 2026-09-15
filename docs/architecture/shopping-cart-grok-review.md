# Shopping Cart review pack — cash is a swap

**Design correction after the 10 Sep owner / architecture reviews.** Product code is not started.

| | |
| --- | --- |
| Product | finos desktop (Profile A). Local SQLite is the system of record. |
| Date of facts | 10 September 2026 |
| Authority | Shopping Cart Target Architecture v1.0, locked 14 August 2026 (BR-SC-001–025) |
| Reviews accepted | `docs/Authority/cursor-shopping-cart-prompt-2026-09-10.zip` (Cursor Pack Review + Architecture Review + SC1 slice) |
| Board status | SC-1 named (`build`). Execution **Now** stays CM-4 import. Shopping Cart draft+evaluate is in product. |
| Correction | Cash deploy is a **swap**. Leftover cash **still earns** the money-market plan. Cart UI and evaluate math must not grow `App.tsx` or `queries.rs`. |

---

## 1. Review answers (locked)

1. **$340 with no covering lot.** Show the math. Label it **intent**, not a trade. Agree stays blocked (`insufficient_lot_qty`). Do not hide Evaluate.
2. **Leftover cash yield.** Include it. Net = buy plan income − plan income on **dollars spent**. Leftover keeps the cash yield. Do not freeze +$6.92.
3. **Agree then both sides.** Freeze first. Owner confirms each sell (`ActivityPost` + `LotAssign`). Then Add Lot opens prefilled. Owner clicks **Open lot**. Evaluate never `LotOpen`.
4. **First slice.** One account. One as-of. Multi-sell + multi-buy. No monthly drip / tranche.
5. **Added columns / surfaces.** Sale cost and gain/loss on non-cash sells; plan and price source on the header; cash-floor warning; leftover still-earns; override reason; step rail; afford strip; keep-vs-swap income; two mix bars.

---

## 2. Product lock

- A new-cash purchase is a **swap**. Named lots, including SPAXX / CASH / FDRXX / SWVXX at par. No FIFO.
- Qty to sell may be **less than remaining** (partial lot). Goldens compare spend to **live remaining**, not a frozen Roth $65.77 rule.
- Drafts must not change Holdings, ROI, `DividendGet`, `BasisGet`, or Income Plan (BR-SC-019).
- Uninvested cash is **off** the Foundation / Core / Risk On denominator (BR-SC-011).
- Weekly = annual ÷ 52. Monthly = annual ÷ 12. Missing plan or stale price is **unknown**, never `$0`.
- HAKY is **Core**. Risk on a line is Foundation, Core, or Risk On. Cash is a separate row.
- Status is a **word** (`draft` / `agreed` / `executing` / `complete`). Color is a cue only.
- Save is orange (`is-unsaved`) while dirty.
- **Cash floors** (owner-stated 10 Sep review, not in SQLite): Income $5,000, Car $4,000, Health $200. Warn if selling cash would drop the account below the floor. Override needs a typed reason. **No invented floors** for FI Roth, 9, Speculation, ENERGYX, Robinhood. Do not reuse MAGI `safety_reserve`.

---

## 3. What exists today (gaps)

| Piece | Limit |
| --- | --- |
| `CartItemAdd` / `CartItemRemove` / `CartGet` | Symbol + quantity only. Keep until a golden replaces the M6 test. Not the evaluate API. |
| `financial-domain` / `storage-sqlite` `cart.rs` | M6 thin cart only. |
| `apps/desktop/src/features/` | **Does not exist.** Account select, lot picker, and Add Lot combobox live in `App.tsx`. |
| Shopping Cart screen | None. |
| Sell path | `ActivityPost` + `LotAssign` to a named lot. No ActivityPost form. Broker CSV drops sells. |
| Buy path | Add Lot → `LotOpen`. Researched symbol required. |

---

## 4. Blank template (column contract)

Empty cells stay blank (unknown), never `$0`.

### 4.1 Header

| Field | Rule |
| --- | --- |
| Account | Required before evaluate (BR-SC-002). |
| As-of | Frozen until Refresh snapshot. |
| Plan source / price source | BR-SC-008. Stale or missing stays unknown. |
| Status | Word: `draft` until Agree, then `agreed` / `executing` / `complete` / `discarded`. |

### 4.2 Sell / surrender (SPAXX included)

Lot ID required. Qty to sell ≤ remaining.

| Column | Source |
| --- | --- |
| Lot, Symbol, Account | Holdings / security. |
| Qty to sell | Owner. May be partial. |
| Unit $ | Snapshot. Cash/MM = $1.00 par. |
| Proceeds | Qty × unit. |
| Original cost / gain-loss | Non-cash sells (BR-SC-006). Tax estimate later. |
| Plan $/yr on **spent** dollars | Snapshot. Leftover cash is not in this surrender. |
| Risk / cadence | Characteristic. Cash listed off the mix. |

### 4.3 Buy / replace

Researched-symbol combobox only (not free-text). Whole shares.

| Column | Source |
| --- | --- |
| Symbol, Qty, Last $, Spend | Snapshot × whole shares. |
| Plan $/yr gained | Plan × qty × periods/year. |
| Risk / cadence | Characteristic. |
| Leftover | Proceeds − spend. Negative blocked (BR-SC-014). Leftover still earns MM plan. |

### 4.4 Same-screen visuals (required)

- **Step rail:** Draft · Evaluate · Agree · Confirm sell · Open lot. Current step marked. Not a wizard that hides the grid.
- **Afford strip:** named remaining vs spend vs leftover. Over remaining → cannot afford; intent numbers still visible.
- **Income keep-vs-swap:** year / month / week.
- **Two mix bars** for this account only. Cash is a label beside the bar, not a slice.
- **Tradeoff:** if income rises and mix vs target worsens, both sentences + typed reason to Agree (BR-SC-013).
- **Cash-floor** warning when a named cash sell would breach Income / Car / Health floors.

### 4.5 Actions

Save (orange while dirty) / Cancel / Evaluate / Refresh snapshot / Agree.

Agree blocked until named lots cover spend, buys are researched, spend ≤ proceeds, and cash-floor / mix-worsens overrides are typed when warned.

---

## 5. Worked examples — SPAXX swapped for HAKY

Source: live local SQLite, 10 September 2026. Illustrative only. Goldens use **fixture remaining vs spend**.

| Fact | Value |
| --- | --- |
| Account | FI Roth |
| SPAXX | Core, `CASH`, par $1.00, plan 3.34%/yr. Stored remaining **$65.77**. No cash floor on FI Roth. |
| HAKY | **Core**, Monthly, $0.3800/sh × 12 = $4.56/sh/yr, last $29.95. Already has lots. |
| FI Roth invested ex-cash | F $3,585.73 · C $1,769.76 · R $2,983.58 · total **$8,339**. |

### 5.1 Example A — $340 intent (blocked)

Sell $340 SPAXX (no covering lot) → 11 HAKY @ $29.95 = $329.45, leftover $10.55.

Leftover-keeps-yield: surrender only $329.45 × 3.34% ≈ **$11.00**. HAKY **+$50.16**. Net **+$39.16** · monthly **+$3.26** · weekly **+$0.75**.

**Status: `insufficient_lot_qty`.** Evaluate shows intent. Agree blocked.

### 5.2 Example B — covering lot (illustrative)

Sell **$59.90** of the $65.77 SPAXX lot (partial). 2 HAKY @ $29.95 = $59.90. Leftover **$5.87** still earns ≈ $0.20/yr.

| | Annual |
| --- | --- |
| Surrendered (spent $59.90 × 3.34%) | −$2.00 |
| HAKY 2 sh | +$9.12 |
| **Net vs keep** | **+$7.12** |
| Monthly / weekly | +$0.59 / +$0.14 |
| Forward yield on $59.90 | 15.23% |

Do not use +$6.92 (that surrendered yield on the leftover $5.87).

After 2 HAKY, invested ≈ $8,399 (Core +$59.90). Cash off the mix. HAKY is Core.

---

## 6. Component architecture (fat-file ban)

New cart code must not land as markup or evaluate math in `App.tsx` or `queries.rs`.

| Layer | Where |
| --- | --- |
| UI | `apps/desktop/src/features/shopping-cart/` — `ShoppingCartScreen.tsx`, `StepRail.tsx`, `AffordStrip.tsx`, `IncomeCompare.tsx`, `MixBars.tsx`, `TradeoffCallout.tsx`, `ExecutePanel.tsx` |
| Shared pickers | `apps/desktop/src/features/shared/pickers/` — extract **only** account select, lot picker, Add Lot symbol combobox from `App.tsx` when SC-1 is named. Cart imports them. Do not copy the forms into the cart. Do not split the rest of `App.tsx`. |
| Host | New `crates/application-core/src/cart.rs` (or `cart/`) — scenario, lines, evaluate, agree. |
| Domain / SQL | Keep existing `financial-domain/src/cart.rs` and `storage-sqlite/src/cart.rs` (M6). Extend those crates; do not put evaluate math in `queries.rs`. |
| `App.tsx` | Menu label + `setScreen("shopping-cart")` only. |
| `queries.rs` | Register/dispatch match arms only. |

Precedent: Home already lives in `HomeAccountCharts.tsx`. SC-1 is not a license to move weekly actuals, collectors, or Income Plan out of `queries.rs`.

Reject a patch that adds a cart grid or cart SQL/evaluate to `App.tsx` or `queries.rs`.

---

## 7. Storage

New tables. Not `LotOpen`. Not `cart_item`.

| Table | Holds |
| --- | --- |
| `cart_scenario` | id, account_id, kind=`Swap`, status word, created_at, agreed_at |
| `cart_sell_line` | **lot_id** required, qty (may be partial), proceeds / plan / cost / risk snapshots |
| `cart_buy_line` | researched security, whole qty, price / plan / risk snapshots |
| `cart_eval_snapshot` | as-of, sources, mix before, weekly/monthly/annual, leftover, floor/mix warnings |
| `cart_execute_step` | sell `activity_id`, `assignment_id`, buy `lot_id` after Open lot |

---

## 8. After Agree — both sides

```
Pick named lots → Pick researched buys → Evaluate
    → Save draft → Agree freeze
        → Confirm sell (ActivityPost + LotAssign)
        → Add Lot prefilled → owner clicks Open lot
```

Complete when every sell is assigned and every buy has a `lot_id`. No silent `LotOpen`. If a buy has no retrieval template → Add Position (`not_researched`). First lot + collector incomplete → `collector_incomplete`.

---

## 9. Pushbacks (kept)

1. There is **no** `features/` folder today. Extract three pickers when SC-1 is named; do not invent folders before that.
2. Host `cart.rs` is **application-core**. Domain and storage cart modules already exist.
3. Cash floors are the three named amounts only.
4. Cart-only split. Do not migrate other fat-file work in this slice.
5. Do not chase `aria-label="Save new investment facts"`.
6. No CM-4, MAGI rewrite, FIFO, silent `LotOpen`, HAKY as Risk On, monthly drip, or portfolio-wide mix.

---

## 10. BR-SC-001–025

Unchanged from the 14 Aug lock. This pack treats cash deploy as Exit/Replacement (BR-SC-001), not New Money. Cash off the mix (BR-SC-011). Drafts do not change the books (BR-SC-019).

---

## 11. When the owner names SC-1

Keep `LocalTauriFinanceClient`. No UI SQL. No MAGI oracle rewrite.

- **SC-1a:** five tables + application-core host + leftover-keeps-yield evaluate. Golden: covering-lot net uses leftover-keeps-yield; over-remaining → `insufficient_lot_qty`; `DividendGet` / `BasisGet` unchanged.
- **SC-1b:** feature folder + shared picker extract. Step rail, afford strip, income compare, mix bars, tradeoff, orange Save.
- **SC-1c:** Agree freeze → confirm sell → prefilled Add Lot.

Pass: `cargo test -p financial-domain` and golden-harness `cart` + `add_lot` + `lots_roi` + `core_functions` + `accessibility`. Feature folder exists; `App.tsx` has no cart grid; `queries.rs` has no cart SQL/evaluate.
