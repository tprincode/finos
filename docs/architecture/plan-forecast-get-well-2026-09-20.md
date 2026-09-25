# Plan forecast get-well

**Status:** owner lock 20 Sep 2026. Not a new Authority pack.  
**Supersedes** vendor-date-as-dollar-gate on Income Plan / cash-flow / Coverage Year forecast.  
**Does not change** collector `remaining_year` tickets, MAGI, Tax Planning, or YTD remaining (this 31 Dec).

## Two clocks

**Future (unlocked weeks, cash-flow, Coverage Year / next 12 months):**  
Plan $ = remaining qty × current `plan_history` (empty `effective_to`) × one cadence period, placed on every Weekly / Monthly / Quarterly slot in the window. Vendor `issuer_pay_date` wins the day when present. A hole walks the locked weekday / month-day / quarter. `assumed_pay_date` and `RemainingPaymentDateOverride` pin the day — they do not grant permission to print Plan $. Missing vendor / assumed rows do **not** drop dollars.

**Past (locked weeks):**  
That week’s Plan $ is the `plan_history` window on the pay date (`plan_amount_on_pay_date`). Compared to Declared (issuer Decl $). Confirm Plan closes the old window and does not rewrite locked weeks. Broker Actual is not this pair.

Income Plan Plan vs Declared is the authority. Coverage bottom table is the same lookback: % over or under plan for the selected Week / Month / Year lookback.

## Coverage Year

Forecast Plan income = as-of through as-of + 12 months of current qty × periods × plan (Income live target: over $39k, Home Dividend Plan formula). Caption says **next 12 months**, not last calendar year.

## Collector split

`skip_next_year_derive` still forbids inventing 2027 so remaining-year **tickets** look complete. Forecast readers still place Plan $ on the next 52 / 12 / 4 slots via `derive_horizon_pay_ons`.
