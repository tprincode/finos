# Writer → reader map (working)

**Status:** first slice of the writer SOT reader pass. Not a new Authority lock.  
**Date:** 2026-09-20.  
**Writers:** `docs/architecture/writer-source-of-truth-2026-09-20.md`.  
**2027 dates:** `docs/architecture/plan-horizon-2027-plan-2026-09-20.md` (S1–S4 shipped).

Each screen field below names the SQLite row that owns it. A second query or leftover table is not authority.

## Income Plan

| Field | Reads | Writer |
| --- | --- | --- |
| Plan $ / share | `plan_history` (empty `effective_to`) × remaining qty | `PlanHistoryConfirm` |
| Decl $ | `issuer_declaration` in that Sat–Fri week | Collect / `IssuerDeclarationRecord` |
| Pay date this year | `issuer_pay_date` through this 31 Dec | Collect / `IssuerPayDateReplace` |
| Pay date next year (vendor) | `issuer_pay_date` after this 31 Dec | Collect save (no remaining-year ticket) |
| Pay date next year (hole) | `assumed_pay_date` after Confirm | `PlanHorizonAssumeNextYear` |
| Year-to-go / remaining-year | Remaining-year helper through this 31 Dec | Not assumed; not vendor 2027 |
| Next-year Confirm banner | `PlanHorizonGet` hole count | Confirm writes assumed rows |

Week row “assumed” mark is not shipped. 2027 weeks use Plan $ from `plan_history`; they do not write Plan from Decl $.

## Home / Register (`AccountCashFlow` + `CashRegisterGet`)

| Field | Reads | Writer |
| --- | --- | --- |
| Ending / running cash | Confirmed Friday `cash_minor`, then each later week + planned dividends − future debits | Week capture Accept replaces the projection |
| Green income-plan dots | `remaining_pay_dates_for` = vendor dates ∪ confirmed assumed holes | Same writers as Income Plan dates |
| Plan $ on a hit | `plan_history` × remaining qty | `PlanHistoryConfirm` |
| Paid dividend hit | `dividend_actual` / posted `activity_event` | Broker import / `DividendActualRecord` |
| Element hit | `cash_element` series → `planned_occurrence` | `CashElementSave`; Confirm posts `activity_event` |

1Y in late 2026 plots 2027 vendor ∪ assumed. Leftover SPAXX qty is not opening cash.

## Coverage (`CashCoverageGet`)

| Field | Reads | Writer |
| --- | --- | --- |
| Plan income | Income Plan Plan $ for the lookback | `plan_history` |
| Plan expenses | `cash_element` series | `CashElementSave` |
| Actual | Posted hits in that same period | `activity_event` / dividend actuals |

SSA and typed household budget stay out. Blank average ≠ $0.

## Tax Planning / MAGI / YTD

| Field | Reads | Writer |
| --- | --- | --- |
| Projected remaining | This 31 Dec only | Remaining-year path |
| MAGI threshold | `MagiProjectionGet` (ADR-0013) | Locked oracles; no 2027 tax year yet |
| HSA withdrawals | Health book activity | Same YTD/projected as Cash YTD |

Vendor 2027 and assumed holes do not enter these numbers until a 2027 tax year is opened on purpose.

## Weekly Updates (owner grid)

| Field | Reads / writes | Writer |
| --- | --- | --- |
| Week dropdown | `trends_week_source` period | `WeekCaptureAccept` |
| Six book Totals | `account_balance_snapshot.balance_minor` | same — required |
| Six book Cashes | `account_balance_snapshot.cash_minor` | same — blank skips |
| Account 9 ETF total | `trends_week_source.acct9_etf_value_minor` | same — typed |
| Account 9 70% | 0.70 × ETF total | none |
| Week income | Income Plan Plan $ | `PlanHistoryConfirm` (not this grid) |
| Cash-gap reason | `activity_event` Cash_Adjust | Accept → adjust |

## Still to map

Home account cards, Collectors ticket queue layout, Shopping Cart line cells. No new tables in this pass.
