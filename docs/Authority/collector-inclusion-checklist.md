# Collector inclusion checklist

Owner memory aid. **Not a new lock.** Authority remains:

- `.cursor/req_extract/Collector_Requirements_Initial_and_Runtime_Locked_2026-09-04.txt`
- `.cursor/req_extract/Adapter_Collector_Owner_Decisions_Locked_2026-09-02.txt`

If a row is not in `REQUIRED_FIELDS` (Establish) or on the named Runtime command path, the product will not enforce it. To add a requirement: add a row here **and** the code path.

---

## Establish (locked)

Gate: `collector_is_complete` in `crates/financial-domain/src/collector.rs`. First lot: `LotOpen` → `collector_incomplete`. After that first `LotOpen` succeeds, and after Research on a name that already has lots (Recreate adapter), `CollectorRecertify` re-runs the same gate. Existing lots do not skip it. Extra `LotOpen` still grandfathers.

| # | Must exist | Skip? | Code |
|---|---|---|---|
| E1 | Template Dividend (owner seed declaration URL) | No | `has_dividend_template` / `source_url` |
| E2 | Template ROC (reusable 19a-1/tax URL; search `19.1 tax ROC (TICKER) (Provider) website data source`) | No if in scope; CASH / not-in-scope skip | `has_roc_template` / `roc_source_url` |
| E3 | DIV-1 (`div_type`; CASH is the other path) | No | `REQUIRED_FIELDS` |
| E4 | Frequency | No | `REQUIRED_FIELDS` |
| E5 | Provider name | No | `REQUIRED_FIELDS` |
| E6 | Underlying | No | `REQUIRED_FIELDS`. Owner-stored underlying ≠ ticker files open `underlying` tickets and suppresses re-raise while set. |
| E7 | Risk: Foundation / Core / Risk On (owner sets; never auto) | No | `REQUIRED_FIELDS` `risk_tier` |
| E8 | Paid history: pay dates matched to amounts; inception Yes/No if &lt;12. Expected monthly count is completed month-ends after the launch month (current month only on that month-end). | No | `paid_history` |
| E9 | Remaining-year pay dates (vendor, else derive-once; cap 4/12/52) | No | `remaining_year` |
| E10 | Current-year ROC estimate accepted (0 only if notice said 0) | No if in scope | `roc_estimate` + owner accept (`needs_roc_research=false` via Collectors Accept ROC / `RocPlanConfirm`, or ticket Accept on `roc_pct_change`). Accept files open `roc_estimate` tickets. Monthly skip and same-% recheck must not undo Accept. |
| E11 | Backtest dates | Optional; Skip does not block | not in `REQUIRED_FIELDS` |
| E12 | Last price once (never $0) | Blocks Process A establish Complete (no lot required) | `establish_checklist` last_price row |
| E13 | Confirm Plan + Calculator eligible | Blocks Process A Complete | PlanHistory + `calculator_view_includes` — Cart/Calculator at 0 shares OK. First lot optional (N/A). |
| E14 | Issuer declaration adapter is registered (`declaration_source` on the template is non-blank and in `REGISTERED_DECLARATION_SOURCES`) | N/A for cash and holdings-only names | `establish_checklist` `declaration_adapter` row. A blank or unregistered adapter means **no declaration is ever retrieved** — the name looks established and silently never pays history. |
| E15 | Collector enabled, so Run enabled actually collects it | N/A for cash and holdings-only names | `establish_checklist` `collector_enabled` row, from `retrieval_template.collector_enabled`. |
| E16 | Annual period count > 0 for the rate | No | `establish_checklist` `plan_periods` row. Zero periods is how a Twice monthly name shipped a cart rate annualised on the wrong period count. |
| E17 | A confirmed Plan window covers the next unoccurred pay date | N/A when no pay date is published yet | `establish_checklist` `plan_window` row. A Plan whose `effective_from` is after the pay date leaves the week grid with no Plan $/sh even though the name looks established. |

---

## Runtime (declaration vs ROC stay apart)

| # | Rule | Fail → ticket? | Stored past pays? |
|---|---|---|---|
| R1 | Enabled + open lots. Add lot does not re-run history/ROC/dates. | — | Untouched |
| R2 | Collect uses **Template Dividend** only. New/changed dates + amounts **for today / current month / future**. Do not re-verify or fail `last_run_ok` on months-old page history already locked in SQLite (establish/inception owns paid history). Do not clone stored months. GET timeout (`declaration_retrieve_timeout`) skips that name and continues the fleet; post-run and ticket Retry use the stored URL at 20s / 45s / 90s (three tries). Recreate is not the timeout path. | Miss / parse miss → `declaration_retrieve_miss`. Timeout → `declaration_retrieve_timeout` (Retry tool). `last_run_ok` = this run. A later **ok** retrieve — including fleet/DeclarationRefresh `unchanged` — auto-files that miss ticket (`is_auto_file_on_ok`; golden `unchanged_ok_auto_files_open_retrieve_miss`). Owner-filed or auto-resolved does **not** suppress a later failed retrieve (Home 39/40 with no ticket is a break). Miss payload keeps GET evidence (`htmlLen`, payable, td count) — not a screenshot, not a wipe of stored pays. Historical page rows that disagree with locked store are not a retrieve miss (golden `declaration_refresh_ignores_historical_page_pays`). | Keep. Owner/import/manual paid rows are not overwritten. Issuer same-period change may supersede after complete; 30% → amount ticket, not a wipe. |
| R3 | Week key is vendor **Payable** (never ex / record / press). Future dates **and leftover same-month record/ex dates** move when the vendor payable changes, even after the leftover day has passed and even if that payable is already stored. Declaration date is ignored; amount is stored on the payable. Plan $ unchanged. Vendor page / 8-K dates **after this 31 Dec** are saved; they do **not** count toward remaining-year completeness and do not invent 2027 `derived_*` rows. Golden: `collector_retrieve_moves_leftover_record_after_date_passed`, `leftover_record_pay_date_plans_on_payable_week_not_record_week`, `collector_saves_vendor_2027_without_remaining_year_ticket`. | Wrong count vs 4/12/52 → ticket. Leftover record left live → product break. `paid_payable_supersede` only when a **paid** month would be pulled earlier. Invented 2027 to look complete → product break. | Occurred paid pays stay |
| R4 | Declaration run does not rewrite identity, frequency, Plan $, lots, or ROC. | — | — |
| R5 | Unchanged today + same hash → no fetch, **except** on the locked declaration weekday (must fetch again). | — | — |
| R6 | ROC uses **Template ROC** only (`PositionResearchRefresh` / Reevaluate). Not Collect / Force refresh. **HTTP once per calendar month** unless `forceRoc` (Reevaluate, ticket Run ROC, Store ROC URL, establish ROC hole). Unknown ≠ 0. Different % → `roc_pct_change`, do not overwrite. ROC miss does **not** flip `last_run_ok`. Owner may type **Manual ROC %** on Process A (`RocPlanConfirm` owner-override). | ROC parse miss / % change tickets | Pays untouched |
| R7 | Last price is a separate job. Never $0. | — | — |
| R8 | Broker import = actual $ only. Never declarations or Plan $. | — | — |
| R9 | After the fleet run, fail count (not current today) **equals** open retrieve-failure tickets on those names (`declaration_fail_ticket_parity`). Extra ROC / amount tickets do not count. | Parity false is a product break. | — |
| R10 | Income Plan Plan $ (`plan_history` × remaining qty) is the planned weekly income on **every** page (`IncomePlanWeekGet`, `TrendsWeekGet.plannedWeeklyIncomeMinor`, Cash week desk). Off-calendar broker actuals are Reported only — they do not move Plan $ onto that week. Golden: `off_calendar_actual_does_not_move_plan_week`. | Planned totals disagree → product break | — |
| R11 | The 30% amount rule reads the **whole stored series**, not only the runtime window. Establish seeds a page in one run, so consecutive seeded pays are compared to each other (`history_amount_variation_issues`); a pay recorded after its month turned is still compared to the prior paid (`new_amount_variation_issues` — reprints are excluded by being `unchanged`, not by the calendar). Overlap (same period, different page $) stays current-month only. Every cadence. Goldens: `establish_batch_tickets_an_in_history_amount_jump_every_cadence`, `stored_history_tickets_amount_variation_even_if_incomplete`, `pay1_overlap_amount_change_tickets_keeps_stored`. | → `declaration_amount_variation` (tool `amount_confirm`, soft — never flips `last_run_ok`). Silent seeded jump is a product break. | Keep. Ticket is the signal; the row is a fact. |
| R13 | An unoccurred `derived_walk` pay date the issuer has since contradicted is superseded. Vendor dates are the standard; derived is only the fallback, so once the issuer publishes the date the walk guessed, the guess goes. Keyed off `cadence_twin_gap_ceiling_days` (Weekly 3 / Twice monthly 7 / Monthly 14 / Quarterly 45), so it applies to every cadence. Two **published** dates inside the twin gap are a conflict, not a filler, and are left alone. Runs in `CollectorCadenceHeal` **and** in the load-time miss sweep, so a stale guess heals on open. Goldens: `collector_cadence_heal_drops_the_derived_date_the_vendor_replaced`, `derived_twin_drop_applies_to_every_cadence`. | Two published twins → ticket, never a silent delete. A surviving contradicted guess breaks that name's spacing band, count window, and remaining-year count. | Occurred paid pays untouched — only unoccurred guesses are dropped. |
| R14 | Remaining-year completeness does **not** require the derived walk to equal the issuer's published count. A monthly payer that publishes one month at a time was permanently incomplete under equality. The rule is: no published date is missing, and the walk has not invented more pays than the year has left (`remaining_periods_to_year_end`). Goldens: `derived_fillers_past_the_published_horizon_are_not_a_remaining_year_gap`. | False permanent `remaining_year` gap is a product break. | — |
| R12 | Newest paid more than 30% from the **in-force Plan $/unit** → `declaration_plan_mismatch` (tool `plan_vs_declaration`), raised only on the run that records that pay. The collector never rewrites Plan, and the Shopping Cart / Income Plan keep reading Plan only — the ticket is where Plan is confirmed, not in the cart. Golden: `paid_far_from_plan_raises_plan_vs_declaration`. | → `declaration_plan_mismatch`, soft. Declaration rewriting Plan is a product break. | Keep |

Owner Accept/Reject is not Establish or Runtime success.

---

## How often

- **Establish gate:** Collectors / Tools / LotOpen (`CollectorSetGet`).
- **Establish recertify:** `CollectorRecertify` after first `LotOpen` (`first_create`) and after `PositionResearchSeed` / `PositionResearchRefresh` when lots already exist (`recreate`). Same E gaps. Ticket `collector_establish_incomplete` if it fails. Does not wipe pays. Does not flip `last_run_ok`. Does not replace the first-lot block or extra-lot grandfather. Recreate loads stored Template Dividend (E1) and Template ROC (E2). It validates with `CollectorRecertify` and may retrieve with the stored URL. It does not re-run `PositionResearchSeed` and does not re-import pays, identity, ROC, or Plan. Owner pastes only when E1 is empty. Confirm Plan is not required when Plan is unchanged.
- **Runtime collect:** desktop open `DeclarationRefresh` on the same weekday 9–4 Eastern window as last price, once per local date (`inSchedule` + `declarationRefreshedOn` already today). Weekend / outside hours skip. The 4-hour last-price cooldown does not skip collectors. Run misses; Run enabled; Force refresh / Refresh declarations (one symbol or fleet, declarations only) stay force. GET timeout skips that name and continues; post-run and ticket Retry use the stored Template Dividend at 20s / 45s / 90s. Recreate is not the timeout path.
- **`cargo test`:** predicate and fixtures. Does not re-certify the live book. `recertify_runs_after_first_create_and_after_recreate` locks the product command path.

---

## Adding a position adds the symbol everywhere

The owner must not have to remember to update a roster. Every roster in the product is
**derived** from open lots + `position_characteristic` + `retrieval_template.collector_enabled`
— last price, the declaration queue, the Calculator, the Cart, Income Plan, the managed count
and the fleet reports all read that derivation, so a new position appears on all of them with
no list to edit.

Enforced by:

| Guard | What it stops |
|---|---|
| `establish_checklist` + `ReadinessFacts` (`collector.rs`) | A new MUST-satisfy step is a **struct field**, so adding one is a compile error at every call site rather than a check someone forgets to call. The rows render at the bottom of Add investment and are the same rows the goldens read. |
| `every_enabled_position_passes_the_add_investment_readiness_list` | Runs the readiness list over **every enabled symbol in the live book**, not a fixed roster. A new position is held to the same bar the day it is added. |
| `profile_a_income_fleet_is_forty_and_still_miss_zero` | The 13 Sep 2026 owner lock of 40 names is a **floor, not a ceiling**: those names must still be enabled and miss-free, and so must every newer one. Adding a position cannot turn this red. |
| `live_enabled_collectors_all_map_to_a_cadence_class` | Every enabled payer in the live book maps to a real cadence, derived from the book instead of three hard-coded symbol lists. |
| `no_income_fleet_symbol_is_hard_coded_in_the_desktop_ui` | A ticker in a screen is a roster that stops growing. Comments may name a symbol; code may not branch on one. |
| `cash_symbol_lists_match_the_rust_authority` | One money-market roster per language, and the TypeScript copy must agree with `is_cash_par_symbol`. Ten separate lists is how a new sweep account shows the right balance on one screen and zero on another. |

`INCOME_FLEET_SYMBOLS` is **test-only** and gates nothing live. Do not add product behaviour
behind it.

---

## Not collectors

**MSTU, TSLL, and SOXL** are holdings only. They are not on Income Plan. They are not collectors. Do not list them on Collectors, Run enabled, recertify, miss counts, or fleet reports. Seed cannot enable them. Lots and stored history stay. Last price is a separate job.

---

## Agent rule

When changing collectors, open this file and walk every E/R row. Do not summarize the list down. Do not call a name complete from last_run alone. Do not mention MSTU, TSLL, or SOXL in collector results.
