# Cash integrity forensic (2026-10-10)

**Status (2026-10-11): Closed forward.** The $2,500 week-fold omission and new-cart `cash_align` without `activity_id` are fixed in product code and goldened. This note is the RCA evidence, not an open defect list.

| Issue | Forward path | Residual |
|-------|--------------|----------|
| Week reference omitted `ira-contribution` / `deposit` | `cash_fold_through` + `cash_journal_inflow`; golden `contribution_deposit_and_withdrawal_move_the_lot_once` | None |
| Cart `cash_align` NULL `activity_id` while journal existed | `execute_cash_align` persists id when `delta != 0`; refuses `cash_align_missing_activity` / `cash_integrity_gap`; golden `cart_purchase_then_align_pile_equals_fold` | Live historical NULLs linked 2026-10-11 via `CartCashAlignLinkRepair` (Car+Income 9/26 → `withdrawal`; Income 10/3 → `ETF Purchase`). Residual only if a future NULL has no amount-matched same-day journal |
| Zero-delta `cash_align` with NULL `activity_id` | **Intentional** — no journal row to link | Sync does **not** ticket unless a matching journal row exists that day |

Live DB probe context below was read-only at the time of the RCA.

## Current lock (after Friday 2026-10-09 Accept)

| Book | Pile (symbol) | Snapshot `@2026-10-09` | Fold after snap | gap pile−fold |
|------|---------------|------------------------|-----------------|---------------|
| Income | 489964 (SPAXX) | 489964 | 489964 | 0 |
| FI Roth | 9275 (SPAXX) | 9275 | 9275 | 0 |
| Speculation | 902 (SPAXX) | 902 | 902 | 0 |
| Car | 18481 (SPAXX) | 18481 | 18481 | 0 |
| Health | 38908 (FDRXX) | 38908 | 38908 | 0 |
| Account 9 | 1068624 (SWVXX) | 1068624 | 1068624 | 0 |

Pile, Friday snapshot, and post-Friday journal fold agree **today** because Accept rewrote lot + snapshot together.

## Gap that produced wrong true-up / cash flows (RCA)

Income week ending **2026-10-03**, prior snap **2026-10-02** cash **527822**:

| Activity | Amount (cents) |
|----------|----------------|
| `IRA_Distribution` | 70000 |
| `ira-contribution` | 250000 |
| `ETF Purchase` (cart align) | 266385 |

- **Old** `reference_cash_minor` (leaving + ETF Sale only; **no** `ira-contribution` / `deposit`):  
  `527822 + 0 + 0 − (70000 + 266385) = 191437`
- **Correct** fold (inflows include `ira-contribution`):  
  `527822 + 250000 − (70000 + 266385) = 441437`
- **Delta = 250000** — exactly the Traditional IRA contribution. Week true-up and charts that trusted old reference understated Income cash by **$2,500.00** while SPAXX already held the contribution.

## Cart evidence (RCA — historical)

Completed carts (Income ×2, Car ×1) all `funding_source=sellLots` with a `cash_align` step, but **`cart_execute_step.activity_id` was NULL** on every align — journal rows existed as `ETF Purchase` / `deposit`, yet the cart step did not link them. **New aligns persist the link.** `CashIntegritySync` raises `cart_align_unlinked` when a completed NULL align still has a matching same-day journal row.

## IRA scope

One 2026 `ira-contribution`: Income **2026-10-03** **250000**. Not posted on Roth/Health/Car (by design). “Wrong on most accounts” on cash flows was the **reader splice** (snapshot past vs lot as-of) plus Income true-up missing the contribution inflow, not IRA posting to every book.

## Conclusion (enforced)

1. Live SOT = money-market lot; Friday lock = snapshot; integrity = fold(snapshot, journal).
2. Stitch: include `deposit` + `ira-contribution` in week reference; persist align `activity_id` when cash moves; Confirm-cash shows calculated ending balance; `CashIntegrityGet` guards pile == fold; `CashIntegritySync` tickets gaps and historical unlinked aligns (Home load runs sync beside `WorkTicketSyncMisses`).
