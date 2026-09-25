# Writer source of truth (working audit)

**Status:** second pass — owner-input fields, not just table names. Not a new Authority lock.  
**Date:** 2026-09-20.  
**Layer:** writers. Readers: `docs/architecture/writer-reader-map-2026-09-20.md`.  
**Canvas:** `.cursor/projects/c-Users-EVTom-repo-finos/canvases/writer-source-of-truth.canvas.tsx`

The first cut was short because it named **tables**, not **typed cells**. This pass walks the weekly capture grid and the other owner write surfaces. Derived / read-only values are listed so they are not mistaken for writers.

SQLite is the system of record. Seed CLI is not an owner screen.

---

## Weekly Updates capture (Cash Management)

One command: `WeekCaptureAccept` (Correct uses the same command with `allowClosed`). Edit only refills the grid. Close is `TrendsWeekClose`.

### Typed — required Total, Cash may be blank

| Owner cell | Lands on | Rule |
| --- | --- | --- |
| Week dropdown (Sat–Fri, labeled by Friday) | `trends_week_source.period_start` / `period_end` + snapshot `period_end` | Identity of the week row. |
| Income Total Balance | `account_balance_snapshot.balance_minor` (Income) | Required. |
| Income Cash Balance | `account_balance_snapshot.cash_minor` (Income) | Blank = skip (not $0, no Adjust). |
| FI Roth Total Balance | `account_balance_snapshot.balance_minor` (FI Roth) | Required. |
| FI Roth Cash Balance | `account_balance_snapshot.cash_minor` (FI Roth) | Blank skip. |
| Speculation Total Balance | `account_balance_snapshot.balance_minor` (Speculation) | Required. |
| Speculation Cash Balance | `account_balance_snapshot.cash_minor` (Speculation) | Blank skip. |
| Health Total Balance | `account_balance_snapshot.balance_minor` (Health) | Required. |
| Health Cash Balance | `account_balance_snapshot.cash_minor` (Health) | Blank skip. |
| Car Total Balance | `account_balance_snapshot.balance_minor` (Car) | Required. |
| Car Cash Balance | `account_balance_snapshot.cash_minor` (Car) | Blank skip. |
| Account 9 Total Balance | `account_balance_snapshot.balance_minor` (9) | Required. Also copied to `trends_week_source.schwab_total_minor` when SCH card is 0. |
| Account 9 Cash Balance | `account_balance_snapshot.cash_minor` (9) | Blank skip. Also stored on `trends_week_source.acct9_cash_minor`. |
| Account 9 ETF total | `trends_week_source.acct9_etf_value_minor` | Typed. Blank stays 0 — Accept does not invent last-price 70%. |
| Cash-gap reason (fee / split / other) | `activity_event` via `CashAdjustPost` (`Cash_Adjust`) | Only when typed cash vs reference is material. |
| Cash-gap detail | `activity_event.note` | Optional text after the kind. |

Fidelity book totals also sum into `trends_week_source.fidelity_total_minor` when that card is 0. Income cash is also copied to `trends_week_source.income_cash_minor`.

### Shown, not typed — do not treat as a writer

| Screen value | Reads | Not |
| --- | --- | --- |
| Account 9 70% ETF | 0.70 × typed ETF total (or — if blank) | Not stored as its own fact. |
| Week income / Planned weekly income | Income Plan Plan $ for that Sat–Fri (`plan_history` × qty). Stored copy may land on `trends_week_source.monthly_divs_minor` when the body sends 0. | Not an owner dollar. Not Decl $. Not seed Profit. |
| FID / SCH week-to-week change | Prior vs this `trends_week_source` cards | Derived. |
| Profit | Leftover `trends_week_source.profit_minor` (may fill from ledger if 0) | Not shown as an input. Not Plan $. |

---

## Week Ahead + Elements

| Owner field | Table | Writer |
| --- | --- | --- |
| Element name / note | `cash_element.note` | `CashElementSave` |
| Element type | `cash_element.kind` | same |
| Frequency | `cash_element.cadence` | same |
| Schedule day (weekday or month-day) | `cash_element.weekday_or_month_day` | same |
| Start / stop | `cash_element.start_on` / `stop_on` | same; empty stop = never |
| Series amount | `cash_element.amount_minor` | same. Off-schedule editor dates are not SOT. |
| Confirm this week | `activity_event` | `WeekAheadConfirm` → IRA_Distribution / `SsaConfirm` / Withdrawal / HSA_Withdrawal |
| Edit occurrence (opens editor) | series, not leftover date | `WeekAheadEdit` → `CashElementSave` |
| Defer | occurrence stays unconfirmed | `WeekAheadDefer` |
| Delete one future hit | `planned_occurrence` | `PlannedOccurrenceDelete` |
| Delete series | `cash_element` | `CashElementDelete` |

Unconfirmed `planned_occurrence` is generated from the series. Leftover off-schedule dates are not authority.

---

## Other owner write surfaces (compressed)

| Surface | Facts | Writer |
| --- | --- | --- |
| Income Plan Confirm next-year holes | `assumed_pay_date` | `PlanHorizonAssumeNextYear` |
| Confirm Plan / share | `plan_history` | `PlanHistoryConfirm` |
| Pay-date pin | `remaining_payment_date_override` | `RemainingPaymentDateOverride` |
| Position information (freq, risk, provider, underlying, notes, active, look-through) | `position_characteristic` (+ lookthrough columns) | `PositionCharacteristicUpsert` |
| Declaration weekday | `expected_payment_pattern` | `ExpectedPaymentPatternUpsert` |
| Expected tax handling | `position_tax_profile` | `PositionTaxProfileUpsert` |
| Accept ROC | `position_characteristic.roc_pct_2026_estimate` | `RocPlanConfirm` |
| Template Dividend / ROC URL | `retrieval_template` | `RetrievalTemplateSet` / Recertify |
| Open lot (date, origin, qty, dual basis) | `lot` | `LotOpen` |
| Manual last price | `price_quote` / `manual_price_override` | `PriceQuoteRecord` / `ManualPriceOverride` |
| Manual / import paid dividend | `dividend_actual` | `DividendActualRecord` / `ImportPost` |
| IRA distribution (gross, fed, state) | `activity_event` + withholding columns | `CashDistributionPost` |
| Tom SSA confirm | `activity_event` | `SsaConfirm` |
| Ticket Accept / Reject / File | `work_ticket` | `WorkTicketResolve` / `WorkTicketFile` |
| Cart draft / execute | `cart_*` then `lot` + `activity_event` | `CartScenario*` / `CartExecute*` / `CashDeposit` |
| Save data snapshot | copy of SQLite (+ workbooks) | `DataSnapshotExport` / `SnapshotCreate` |

Collect writes `issuer_declaration`, `issuer_pay_date`, `retrieve_run`, and tickets. It does not write Plan $ or ROC estimate (tickets only).

---

## Watch — do not treat the right column as SOT

| Use this | Do not treat as SOT |
| --- | --- |
| Per-book Total / Cash on `account_balance_snapshot` | Leftover SPAXX / FDRXX qty; blank cash ≠ $0 |
| `trends_week_source` FID/SCH cards + ETF total | Account 9 70% (derived); week-to-week Δ |
| Income Plan Plan $ for week income | `monthly_divs_minor` leftover; seed Profit; Decl $ |
| `cash_element` series | Leftover `planned_occurrence` dates |
| `activity_event` after Confirm | Unconfirmed occurrence as a posted fact |
| `plan_history` × remaining qty | `income_plan` singleton |
| `issuer_declaration` | `dividend_declaration` |
| `RocPlanConfirm` estimate | Collect live % |
| `assumed_pay_date` after June Confirm | Invented collect next-year; remaining-year tickets |
| Vendor next-year on `issuer_pay_date` | Assumed hole for the same slot |

---

## Still not 100%

Parked / not an owner weekly field (commands exist, no product grid here): MAGI oracles (`MagiRuleSet` / Fact / Coverage / Adjustment), ACA threshold seed row, Allocation targets, PlanApprove, `IncomePlanUpdate` singleton, Backtest, Classification, AI analyze, `ProductionSeedLoad`, calculator_plan, symbol_alias, roc_research_observation (collect path), analysis_run.

Household budget / checking / transfers / MAGI screen / phone clone are not started — there is nothing to list.
