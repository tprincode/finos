# Cash integrity forensic (2026-10-10)

Live DB: `%LOCALAPPDATA%\com.finos.desktop\local.sqlite` (read-only probe).

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

## Gap that produced wrong true-up / cash flows

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

## Cart evidence

Completed carts (Income ×2, Car ×1) all `funding_source=sellLots` with a `cash_align` step, but **`cart_execute_step.activity_id` is NULL** on every align — journal rows exist as `ETF Purchase` / `deposit`, yet the cart step does not link them.

## IRA scope

One 2026 `ira-contribution`: Income **2026-10-03** **250000**. Not posted on Roth/Health/Car (by design). “Wrong on most accounts” on cash flows is the **reader splice** (snapshot past vs lot as-of) plus Income true-up missing the contribution inflow, not IRA posting to every book.

## Conclusion

1. Live SOT = money-market lot; Friday lock = snapshot; integrity = fold(snapshot, journal).
2. Stitch: include `deposit` + `ira-contribution` in week reference; persist align `activity_id`; Confirm-cash shows calculated ending balance; `CashIntegrityGet` guards pile == fold.
