# Review pack — data model, components, Tools menu

**Product:** finos desktop (Profile A). Local SQLite is the system of record.  
**Commit:** `d1d2cb8` on `cursor/core-functions-decl-color` (13 Sep 2026).  
**Reviewer must not reopen:** MAGI oracles (ADR-0013); Postgres / OIDC / AI posting; FIFO; UI SQL.

Isolation claim (ADR-0005/0006): a component may reference another only through contracts or published events. No component reads or writes another component’s tables.

---

## 1. Locks

- Desktop writes SQLite first. Seed is a CLI, not an owner screen.
- Weeks are Saturday–Friday, numbered from the first Saturday of the year.
- Missing money is blank, never `$0` / N/A lecture.
- Owner-edit **Save** turns orange (`is-unsaved`). Do not invent a second save.
- Trends is graphing only. Weekly snapshot + SSA / distribution / withdrawal wizards live on Cash Management.

---

## 2. Data model (SQLite tables)

Grouped by the contract buckets. Names are from `crates/storage-sqlite/migrations`.

| Bucket | Tables |
|---|---|
| Platform | `device`, `snapshot_head`, `handoff_review`, `audit_record`, `app_exception`, `work_ticket` |
| Registries | `account`, `security`, `symbol_alias` |
| Capture | `evidence`, `import_batch`, `import_candidate`, `retrieve_run`, `retrieval_template` (+ `last_content_hash` column in 0021), `issuer_declaration`, `issuer_pay_date`, `collector_field_decision` |
| Ledger | `activity_event`, `lot`, `lot_assignment`, `distribution_characterization` |
| Market / dividend | `dividend_declaration`, `dividend_actual`, `price_quote`, `manual_price_override`, `expected_payment_pattern` |
| Planning | `income_plan`, `calculator_plan`, `plan_history`, `position_characteristic`, `magi_rule`, `magi_fact`, `magi_coverage`, `magi_adjustment`, `aca_threshold_rule` |
| Reporting / snapshots | `account_balance_snapshot`, `trends_week_source`, `account_market_value_daily` (+ cash columns in 0036) |
| Decision | `allocation_target`, `cart_item`, `cart_scenario`, `cart_sell_line`, `cart_buy_line`, `cart_eval_snapshot`, `cart_execute_step`, `backtest_run`, `backtest_period`, `position_backtest_result`, `classification_review`, `analysis_run` |
| Position facts | `position_tax_profile`, `position_characteristic.lookthrough_json` (0022 column), `roc_research_observation`, `remaining_payment_date_override` |

**Accuracy notes**

1. **Cash events reuse the ledger.** IRA / Roth / SSA / Withdrawal are `activity_event` rows, not a cash table. Withholding is identity on post (`0032`). Week/month boards are queries over those rows. SSA is two household payees (Barbara, Tom), one event each per month. Withdrawal is cash leaving a taxable / non-IRA brokerage (Car, Robinhood, and the like), not an IRA distribution.
2. **Weekly snapshot is Reporting.** `account_balance_snapshot` / `TrendsWeekSave` are written from Cash Management UI. `TrendsGet` still also returns `distributions` and `taxMonitor` for CM read-only boards — a reporting query leaking cash summaries.
3. **Shopping Cart has two models.** M6 `cart_item` plus SC-1+ scenario tables. Contracts still list both. Scenario commands are implemented, not parked.
4. **Collector / ticket tables sit in Capture + Platform.** `work_ticket` is not in `component-contracts.md`.
5. **Isolation is not enforced in SQLite.** One file, no schema-per-component. Isolation is a coding rule. `queries.rs` still joins across buckets for Home, Trends, Income Plan, CM.

---

## 3. Component architecture (contracts vs code)

`docs/architecture/component-contracts.md` is still `1.0.0-draft` and says “implementations land in later milestones.” That sentence is stale. Most listed queries exist in `queries.rs`.

### Implemented and used on desktop

| Contract area | Live names (not exhaustive) | UI host |
|---|---|---|
| Platform | `HealthGet`, `CoreFunctionsGet`, `ConfigGet`/`Set`, snapshot / handoff, `ExceptionList`/`Acknowledge`, `WorkTicketList` | Settings, File, Tickets |
| Registries | `AccountList`, `SecurityList`, register/update | Positions, Add Position |
| Capture | Import + collector set/run/stats, `ResearchGapsGet` | Data → Import / Collectors |
| Ledger | `ActivityPost`/`Correct`, lots, `CashDistributionPost`, `SsaConfirm` | CM, Add Lot, Holdings |
| Market | `DividendGet`, `CurrentPriceGet`, last-price window, `DividendPerformanceGet` | Home, Income Plan, Trends |
| Planning | `IncomePlanWeekGet`/`GridGet`/`ExportGet`, `CalculatorGet`, `MagiProjectionGet` | Income Plan, Calculator, CM |
| Reporting | `TrendsGet`/`TrendsWeekGet`/`Save`, `AccountValueHomeGet`, `DividendPlanHomeGet`, `DashboardBurndownGet`, `PositionDetailsGet` | Trends, Home, Dashboard, PD |
| Cart | Scenario create / evaluate / agree / execute | Shopping Cart |

### Contract drift (incomplete or inaccurate)

| Issue | Detail |
|---|---|
| **Missing from contracts** | `CashManagementWeekGet` / `RemindersGet` / `MonthGet`, `CashDistributionPost`, `SsaConfirm`, `TrendsWeekGet`/`Save`/`Correct`/`Close`, `IncomePlanWeekGet`/`GridGet`/`ExportGet`, `DividendPerformanceGet`, `WorkTicket*`, `Collector*`, `LastPriceAutoWindowGet`, `CashPileGet` |
| **Stale “parked”** | Shopping Cart scenario commands are in product (`cart.rs`). The “parked until SC-1” line is wrong. |
| **Wrong owner screen** | `TrendsGet` / `TrendsWeek*` are Reporting, but week **entry** is Cash Management. Contracts still imply Trends owns the snapshot. |
| **Income Plan names** | Contract lists `IncomePlanGet` / `IncomePlanUpdate`. Desktop uses week/grid/export queries. |
| **Dashboard** | Contract `DashboardGet` vs UI `DashboardBurndownGet`. |
| **AI / Backtest / Allocation** | Tables and some queries exist. No owner menu for AI, Backtest, or Allocation. Correct as parked **UX**, not missing schema. |
| **Layering** | UI → FinanceClient → application-core → ports → storage-sqlite. `App.tsx` is still the shell for most screens. Extracted: graphing, shopping-cart, pickers, HomeDividendPlan, CashWeekDesk. CM, Income Plan, Collectors, PD remain in `App.tsx`. |

---

## 4. Tools → Components catalog (accuracy / completeness)

**Tools menu (in-app + native):** Reevaluate collector · Components · Settings.

`CoreFunctionsGet.modules` is `docs/architecture/ui-modules.json`. That table is the Components screen.

| Catalog row | Status in JSON | Accurate? | Gap |
|---|---|---|---|
| Graphing | extracted · Home, Trends | Partial | Week capture folder is still `features/graphing/TrendsCapture.tsx` but the **screen** is Cash Management. Menu areas omit Plan/CM. |
| Shopping Cart | extracted | Yes | — |
| Cash Management | in-app · `CashManagement.tsx` | Partial | Desk + wizards + `CashWeekDesk` are not listed as extracted pieces. Core function ids omit week snapshot (`TrendsWeekSave`). |
| Home | in-app · App.tsx | Partial | `HomeDividendPlan.tsx` is extracted; JSON still says App.tsx only. |
| Income Plan | in-app | Yes as host | Print Export is not a core-function id. |
| Position Details | in-app | Yes as host | — |
| Collectors | in-app · Data | Yes | Tickets share Data menu; tickets have no module row. |
| Import | in-app | Yes | No coreFunctionIds. |
| Lots | in-app · Positions | Weak | Add Lot / Holdings are screens; “Lots” is not a menu label. |
| Settings | in-app · Tools | Yes | — |
| Shell | in-app · File | Yes | — |

**Missing module rows (screens that exist, not in Components):**

- Calculator (Plan)
- Dashboard (Plan)
- Trends as its own row (charts-only; graphing is shared)
- Add Position / Add Lot (Positions)
- Holdings
- Tickets
- Reevaluate collector (Tools — same area as the catalog, easy to miss)
- Components itself (meta; optional)

**Tools menu completeness**

| Item | Needed? | Note |
|---|---|---|
| Reevaluate collector | Yes | Establish / field decisions. Not a “component.” |
| Components | Yes | Catalog, not a plugin host. Copy on screen is correct. |
| Settings | Yes | Health + core functions + templates. |
| Exceptions | Maybe | `ExceptionList` exists; not on Tools. Sometimes under Data. |
| Audit | No for V1 | `AuditList` exists; no owner screen. Fine. |
| Core functions as its own menu | No | Already inside Settings. Do not duplicate. |

**Accuracy of the Components screen purpose:** it answers “has this UI left App.tsx?” It does **not** answer “is the bounded context complete?” Mixing folder extraction with domain components will fail an external review unless the pack says so.

---

## 5. Suggested verdict for an external reviewer

Ask only:

1. Is the **table → bucket** map complete and are any writes crossing buckets without a contract?
2. Should Cash Management be a first-class **Planning/Ledger** component in `component-contracts.md` (week snapshot + SSA + distributions)?
3. Should Tools → Components stay a **UI extraction catalog**, or become a **domain component** index? Doing both in one table is the current completeness hole.
4. Which missing screens (Calculator, Dashboard, Tickets, Add Position) must appear in `ui-modules.json` before M7 accessibility/release?

Do not recommend regenerating MAGI oracles, cutting over to Postgres, or merging week entry back onto Trends.

---

## 6. Owner facts (confirmed 13 Sep 2026)

- Two SSA events per month: Barbara $1,331 (`133100`) and Tom $2,865 (`286500`). Separate confirms, not one combined household amount.
- Withdrawal is cash leaving a taxable / non-IRA brokerage — Car account, Robinhood, or any other non-IRA broker. IRA / Roth use their own distribution types.
- Week capture Accept may spawn the Cash Management follow-up chooser (SSA / Distribution / Withdrawal / None). The chooser is optional.
