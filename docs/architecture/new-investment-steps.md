# New investment steps — HAKY and MUIB (2026-10-03)

A name is fully functional only when every blocking establish row is stored, a confirmed Plan covers the pay date, the issuer declaration (when published) is a separate row, and a lot opened on or before that pay date supplies quantity. Broker cash is not a Plan and not a declaration. Unknown stays blank.

Shopping cart interest rate, sell and buy, is stored Plan per share × payment-frequency periods ÷ price. Twice monthly is 24. Most-current paid is not the cart rate.

## Required steps

| Step | Command | Table / field | Blocks |
|---|---|---|---|
| Identity | `SecurityRegister` / `PositionResearchSeed` | `security` | Yes |
| E1 Template Dividend | `RetrievalTemplateSet` | `retrieval_template.source_url` | Yes |
| E2 Template ROC | same, when ROC is in scope | `retrieval_template.roc_source_url` | Yes if in scope |
| E3 DIV-1 | `PositionCharacteristicUpsert` | `position_characteristic.div_type` | Yes |
| E4 Frequency | same | `payment_frequency` → 52 / 24 / 12 / 4. No default 12 | Yes |
| E5 Provider | same | `provider` | Yes |
| E6 Underlying | same | `underlying` | Yes |
| E7 Risk | same | `risk_tier` Foundation / Core / Risk On | Yes |
| E8 Paid history | declaration collect | `issuer_declaration` amounts on pay dates | Yes |
| E9 Remaining-year pays | collect or `IssuerPayDateReplace` | `issuer_pay_date` (vendor; else derive-once) | Yes |
| E10 ROC accepted | `RocPlanConfirm` or ticket Accept | `roc_pct_*` and `needs_roc_research = 0` | Yes if in scope |
| E12 Last price | price retrieve | `price_quote` never $0 | Yes |
| E13 Confirm Plan | `PlanHistoryConfirm` | `plan_history` amount, scale, `effective_from`, `effective_to` | Yes |
| Calculator / cart | plan exists and cadence is a payer | joined, not a column | Yes |
| First lot | `LotOpen` | `lot.opened_on`, qty, performance basis, tax basis | Income Plan $ only |
| Declaration | collect | `issuer_declaration` on the payable | Week Decl $/sh |
| Broker cash | import / `ActivityPost` | `activity_event` dividend | Not a plan |

## Live book

Week in the screenshot: 2026-09-26..2026-10-02. HAKY pay 2026-09-30. Decl $/sh $0.39, Decl $ $27.69 (71 shares × $0.39).

| Step | HAKY | MUIB | How verified |
|---|---|---|---|
| Identity | `security` HAKY | `security` MUIB | Row exists |
| E1 Template Dividend | `https://amplifyetfs.com/haky/` | Direxion MUIB product URL | `source_url` non-empty |
| E2 Template ROC | Amplify 19a-1 PDF 2026-05-29 | Direxion 19a PDF 2026-09-16 | `roc_source_url` non-empty |
| E3 DIV-1 | DIV-1 | DIV-1 | `div_type` |
| E4 Frequency | Monthly (12) | Twice monthly (24) | `payment_frequency` |
| E5 Provider | Amplify | Direxion | `provider` |
| E6 Underlying | HACK | mu | `underlying` |
| E7 Risk | Foundation | Risk On | `risk_tier` |
| E8 Paid history | Declarations through 2026-09-30 | Declarations through 2026-10-01 ($0.57925) | `issuer_declaration` |
| E9 Remaining-year pays | Month-ends stored (many duplicate `amplify` rows) | Vendor payables from 2026-10-16; 2026-10-01 is a declaration period, not in `issuer_pay_date` | `issuer_pay_date` |
| E10 ROC | 100.00 accepted | 80.00 accepted | `needs_roc_research = 0` |
| E12 Last price | $32.39 on 2026-10-02 | $33.23 on 2026-10-02 | `price_quote` |
| E13 Plan | $0.3800 scale 4 from 2026-08-29, still open. Covers 2026-09-30 | $0.2300 scale 4 from 2026-10-02 only. Does not cover pay 2026-10-01 | `plan_history` windows |
| Lot | Car 70 sh (opened 2026-08-21 and 2026-08-25). Income 1 sh opened 2026-09-26 | Income 20 sh opened 2026-09-26 | `lot.opened_on` ≤ pay date |
| Declaration on the pay | 2026-09-30 $0.39 scale 2 | 2026-10-01 $0.57925 scale 5. Not the cart rate | `issuer_declaration` |
| Broker cash | Car $27.30 and FI Roth $4.29 on 2026-09-30. FI Roth has no lot | None in this week | `activity_event` |
| Week Plan $/sh | Server: plan known, $0.3800, Plan $ $26.98. Screen was blank because the FI Roth cash slice (no lot) vetoed every selected account | Not a plan ticket on 2026-10-01. Correct: the only plan starts 2026-10-02 | `prior_plan_window_still_supplies_plan_per_share`, `broker_cash_without_a_lot_does_not_clear_position_plan`, `weekly_report_plan_per_share_ignores_actual_only_account` |
| Cart rate | Plan × 12 ÷ price | Plan × 24 ÷ $33.23 = 16.61% (1,661 bps). Most-current × 24 ≈ 41.84% is not the cart rate. ×12 of the same plan is not the cart rate | `cart_twice_monthly_yield_is_plan_times_24` |

## Gaps closed this pass

- HAKY Plan $/sh was blank on the week grid even though the 2026-08-29 plan covers 2026-09-30. FI Roth was paid $4.29 and has no HAKY lot. The grid required every selected account slice to have a plan, so that cash-only slice cleared Plan $/sh and Plan $. Slices with no plan and no declaration no longer join that gate. The Income and Car lots still show $0.3800.
- Cart sell facts preferred `planningPeriodsPerYear` whenever it was non-zero, so a stale 12 could halve a Twice-monthly name. Sell quote, buy facts, and both sheets now use `periodsForPlan`: frequency first, then a stored period count. The rate is stored Plan, not most-current.

## Still on the book, not rewritten here

- HAKY’s $4.29 sits on FI Roth while the shares sit on Car and Income. Plan $ uses the lots (71 × $0.3800). Moving that cash or that lot is an owner posting, not a missing Plan.
- HAKY `issuer_pay_date` has repeated identical month-end rows.
- MUIB has no plan window before 2026-10-02, so the 2026-10-01 declaration does not create Plan $.
