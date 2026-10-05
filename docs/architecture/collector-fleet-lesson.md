# Collector fleet lesson (after every collector bug fix)

Standing question — not a one-symbol patch:

> What failed? How do we prevent it? Fix for all collectors **as applicable**.

Fill a short entry below (newest first) after any collector / pay-date / ticket diagnostic fix.

## Template

1. **Failure class** — parse column / leftover twin / derive step / ROC / amount / lookback / …
2. **Cadences in scope** — Weekly / Monthly / Quarterly / None (interval checking is shared; do not invent Quarterly-only gates)
3. **Shared layer change?** — prefer `schedule` / `declaration_post` / retrieve post-checks / `CollectorCadenceHeal` over one adapter
4. **Fixture per family** — at least one Weekly YieldMax-shaped, one Monthly, one Quarterly HTML, one ET 8-K when the class touches pay dates
5. **Owner DB sample** — symbols touched after heal (prove live book, not only goldens)

## 2026-10-03 — Vendor date replaced the guess only after it paid

1. **Failure class:** Pay-date attach — the derived walk fills future pay dates, and the
   vendor-overwrites-derived rule only collapsed twins that had a **recorded amount**. An
   unoccurred guess the issuer had since contradicted therefore survived forever. BITO carried
   both the derived 2026-10-03 and the published 2026-10-07, four days apart under a Monthly
   lock, which broke that name's spacing band, its count window, and its remaining-year count.
   This is what the Add investment readiness sweep found: of 41 enabled names, BITO was the
   only open blocking row, and it was a real product bug rather than missing research.
2. **Cadences in scope:** All four. The walk fills dates for Weekly, Twice monthly, Monthly and
   Quarterly alike, so the drop is keyed off `cadence_twin_gap_ceiling_days` (3 / 7 / 14 / 45)
   rather than a Monthly special case, and the golden loops YMAX, MUIB, BITO and EPD.
3. **Shared layer:** `schedule::derived_twin_pay_date_drops` plus
   `schedule::is_derived_pay_date_source`, called from `drop_derived_pay_date_twins` in
   `queries.rs`. Wired into **both** `collector_cadence_heal_security` and the load-time
   `work_ticket_sync_misses` sweep, so a stale guess heals when the book is opened instead of
   waiting for someone to remember to run the fleet. No adapter code. Two *published* dates
   inside the twin gap are a conflict, not a filler, and are deliberately left for a ticket so
   the owner is never silently missing a date.
4. **Fixtures:** `vendor_pay_date_supersedes_the_derived_guess_it_contradicts` (BITO's exact
   live shape), `derived_twin_drop_applies_to_every_cadence`, and golden
   `collector_cadence_heal_drops_the_derived_date_the_vendor_replaced` across all four
   cadences — each asserting the contradicted guess goes, the published date stays, and an
   uncontradicted fallback stays.
5. **Owner DB sample:** BITO before — `issuer_pay_date` held 2026-10-03 (`derived_walk`) and
   2026-10-07 (vendor). After the heal: 2026-10-03 superseded, 2026-10-07 kept, and zero
   remaining derived/published twins across the whole enabled fleet.

**Rule reminders from this fix:**

- `remaining_year_matches` required `planned == issuer`, so any monthly payer that publishes
  one month at a time was permanently "incomplete" against its own derived walk. Equality
  against a vendor's publishing horizon is not a correctness check. It now requires only that
  no published date is missing and that the walk has not invented more pays than the year has
  left.
- Three pre-existing heal goldens were silently running as **Monthly** no matter what cadence
  they named, because `PositionCharacteristicUpsert` locks cadence once set unless
  `replaceCadence: true` is passed. They now pass that flag and assert the stored cadence
  before testing it, so a fixture cannot claim to cover Quarterly while testing Monthly.
- `stillMiss` is a same-day counter, so a golden asserting `stillMiss == 0` is red every
  morning before anyone clicks Run. The durable signal is an open `declaration_retrieve_miss`
  ticket; the fleet golden now asserts that instead.

## 2026-10-03 — Seeded history and Plan drift both went unticketed

1. **Failure class:** Amount — the 30% rule only compared a newly recorded pay to pays **already stored**, and only inside the runtime month window. Establish writes the whole page in one run, so MUIB's $0.24931 → $0.57925 jump arrived with zero stored history and raised nothing; the same window silenced any pay recorded after its month turned (a monthly collector on the 1st–3rd). Separately `declaration_plan_mismatch` / `plan_vs_declaration` existed, was registered and resolvable, and **nothing ever raised it** — so a paid amount far from the in-force Plan was invisible, which is how the cart kept showing a Plan the owner had not revisited.
2. **Cadences in scope:** All four. Establish seeds the same way for Weekly, Twice monthly, Monthly, and Quarterly, so the golden loops all four rather than patching MUIB.
3. **Shared layer:** `declaration_post::history_amount_variation_issues` (seeded series, latest jump only) and `declaration_post::plan_vs_latest_paid_issue`, both called from `declaration_post_check_issues` / the shared `CollectorRetrieve` post-check path in `queries.rs` — no adapter code. `new_amount_variation_issues` dropped its `as_of` window: reprints are already excluded by being `unchanged`, so the calendar was doing nothing but hiding jumps. Overlap (same period, different page $) keeps the current-month window, and `runtime_declaration_verify_period` itself is unchanged — `last_run_ok` semantics stay exactly as locked.
4. **Fixtures:** `establish_batch_tickets_an_in_history_amount_jump_every_cadence` (W / TM / M / Q), `paid_far_from_plan_raises_plan_vs_declaration` (ticket + Plan not rewritten), `muib_establish_batch_tickets_the_in_history_jump`, `plan_far_from_latest_paid_is_a_plan_vs_declaration_ticket`. `pay1_overlap_amount_change_tickets_keeps_stored` now builds its disputed pay in the current month (`current_month_pay_on`) instead of a hard-coded 2026-08-31 that had aged out of the window and quietly stopped testing the rule.
5. **Owner DB sample:** see the `WorkTicketList` sample in the turn summary — MUIB must show an open `declaration_amount_variation` or `declaration_plan_mismatch` against Plan $0.2300 × 24.

**Rule reminder:** a date-sensitive window plus a hard-coded fixture date is a requirement that stops being tested on a calendar boundary, with no red test to warn you. Three variation goldens had gone red from time passing alone.

## 2026-10-03 — Actual-only account must not blank Plan $/sh

1. **Failure class:** Pay-date attach — HAKY plan $0.3800 from 2026-08-29 covers pay 2026-09-30, but FI Roth dividend cash has no lot. The week grid required every selected account slice to have a plan, so that cash-only slice cleared Plan $/sh.
2. **Cadences in scope:** All. The gate is account slices on the week row, not a cadence walk.
3. **Shared layer:** `IncomePlanWeekPanel` ignores slices that have neither a plan nor a declaration. A later `plan_history` row still must not erase an earlier window that covers the pay date.
4. **Fixtures:** `broker_cash_without_a_lot_does_not_clear_position_plan`, `prior_plan_window_still_supplies_plan_per_share`, `weekly_report_plan_per_share_ignores_actual_only_account`.
5. **Owner DB sample:** HAKY lots are Car 70 and Income 1; FI Roth cash $4.29 has no lot. Plan $/sh on that week is $0.3800. MUIB cart rate is stored Plan × 24, not most-current.

## 2026-10-03 — Plan that starts after pay_on is not that week’s ticket

1. **Failure class:** Pay-date attach — latest `plan_history` was painted onto a pay date before `effective_from` (MUIB plan 2026-10-02 on pay 2026-10-01), so the week stayed open with actual $0.
2. **Cadences in scope:** All. `plan_known` requires `plan_amount_on_pay_date` or latest `effective_from` on or before `pay_on`.
3. **Shared layer:** Income Plan week builder in `queries.rs` (not a collector adapter).
4. **Fixtures:** `plan_effective_after_pay_on_is_not_a_week_ticket`; open week after effective still counts in `dividend_performance_known_plan_percent_includes_the_open_week`.
5. **Owner DB sample:** Live W39 2026-09-26..10-02 — MUIB planKnown false / planned $0; HAKY plan $26.98 actual $31.59; 26 decl tickets, missing paid empty, week complete.

## 2026-10-02 — Week Plan $ / calendar authority vs gap band

1. **Failure class:** Pay-date attach — gap band from last paid dropped real IssuerPayDateReplace / month-end walks (HAKY 8/31, OPEN 8/31); derived_walk stamps in `issuer_pay_date` forced `issuer_calendar` and hid Aug/Sep when wall-clock LotOpen planted Oct+.
2. **Cadences in scope:** Monthly (primary); Weekly / Twice monthly / Quarterly share the same `remaining_pay_dates_for` authority vs invent split.
3. **Shared layer:** `remaining_pay_dates_for` keeps schedule/issuer/assumed as authority; gap-filters invents only; `CalendarPolicy::resolve` counts vendor (non-derived) pays only.
4. **Fixtures:** `monthly_with_prior_actual_uses_issuer_date_not_thirty_day_walk`, `pay_week_without_plan_history_still_lists_position_plan_unknown`, `week_plan_excludes_lot_opened_after_pay_on`, `register_1y_sees_assumed_2027_after_confirm`, `monthly_without_2027_vendor_dates_still_fills_twelve_months`.
5. **Owner DB sample:** Live week 2026-09-26..10-02 — positions Plan $ = lines Plan $ ($1,368.12); MUIB on Plan with eligible qty; HAKY listed; no undeclared non-MM.

## 2026-10-02 — Twice monthly across Income Plan / reports / discovery

1. **Failure class:** Cadence category — Twice monthly (24) existed for FWD math but Income Plan group order / UI `includes("month")` dropped or mis-bucketed names; remaining-pay/horizon maps omitted 24; bare "Monthly" page text could beat ~15d gaps; same-month mid+end pays collapsed like Monthly.
2. **Cadences in scope:** Twice monthly (new report section); Weekly/Monthly/Quarterly unchanged.
3. **Shared layer:** `CADENCE_GROUP_ORDER`, weak Monthly infer, `vendor_payables_same_period` twin ceiling for 24, collectors helpers, queries/plan_horizon label maps.
4. **Fixtures:** domain infer/same-period unit tests; golden `twice_monthly_is_income_plan_cadence_group`.
5. **Owner DB sample:** MUIB — Twice monthly / 24; MC FWD 41.84% at $33.23.

## 2026-10-02 — Twice-monthly FWD used Monthly×12 (MUIB ~21% vs ~42%)

1. **Failure class:** Cadence lock — Direxion Defined Income Boost pays ~every 2 weeks (24/yr). Process A stored `Monthly` so MC FWD = last pay × 12 ÷ price (~21%). Correct is × 24 (~42%).
2. **Cadences in scope:** New shared `PaymentCadence::TwiceMonthly` (24) — schedule walk, declaration spacing, Calculator/Cart periods, Process A / Position Details dropdowns.
3. **Shared layer:** `financial_domain::calculator::PaymentCadence` + `schedule` remaining walk / horizon; UI `periodsPerYear` / filter.
4. **Fixtures:** `cadence_is_one_value_label_or_periods` covers 24 / "Twice monthly".
5. **Owner DB sample:** MUIB — `Twice monthly`, plan periods 24; last paid $0.57925 @ $33.23 → MC FWD ≈ 41.84%.

## 2026-10-02 — Accept ROC left Gaps: ROC establish twin open

1. **Failure class:** ROC — Accept on `roc_pct_change` filed that ticket and wrote the plan %, but twin `collector_establish_incomplete` (Gaps: ROC) stayed open. Owner had no second action that auto-cleared it.
2. **Cadences in scope:** All collectors with ROC establish / roc_pct_change (shared ticket resolve).
3. **Shared layer:** `WorkTicketResolve` `roc_confirm` Accept prefers newest 19a-1 observation, files ROC establish twins when the ROC gap is cleared, then `collector_recertify`. Ticket ROC strip (parse feedback, one form per symbol) is on Tickets / Collectors / Position Details for every symbol.
4. **Fixtures:** `accept_roc_change_files_establish_gap_roc_ticket`; `accept_roc_change_updates_current_year_projection`.
5. **Owner DB sample:** YMAX — estimate 6869, `roc_pct_change` done; leftover Gaps: ROC establish filed after heal.

## 2026-10-02 — Multi-fund 19a-1 PDF picked wrong ticker ROC%

1. **Failure class:** ROC — YieldMax Group 1 (and similar) 19a-1 PDFs list many funds. Untickered parse took the last/first `Estimated Return of Capital` (YRAM 97.31%). Worse: PDF text glues `en-US` to tickers (`en-USYMAX`), so whole-token ticker match failed and fell through. Stored/ticket YMAX showed ~35.99% from an earlier bad estimate while the 9.17.26 notice says **68.69%**.
2. **Cadences in scope:** All vendors that publish multi-fund 19a-1 notices (YieldMax Group weekly first).
3. **Shared layer:** `parse_19a1_notice_for_symbol` / `parse_19a1_ticker_block` strip `en-US` tags; `follow_vendor_19a1_seeds`, search hits, and Neos paths pass the symbol. Prefer Template ROC URL + ticker block over HTML table when the standing URL is a Group PDF.
4. **Fixtures:** `yieldmax_group1_notice_picks_ymax_not_first_or_last_fund`; live PDF `yieldmax_group1_pdf_parses_ymax_ticker_block` → YMAX 6869 / YMAG 8020 / YRAM 9731.
5. **Owner DB sample:** Force ROC on YMAX after host restart — estimate must be 6869 (68.69%), not 3590/9731.

## 2026-10-01 — Runtime collect re-checked months-old history

1. **Failure class:** DeclarationRefresh store-verify + post-checks compared the full issuer page / all stored pays (July, 1990s EPD, year-long ORC series) and flipped `last_run_ok` on `missing stored declaration`, amount mismatch, and `declaration_cadence_spacing`. Runtime is today + current month + future; paid history is establish/inception / CadenceHeal only (R2).
2. **Cadences in scope:** All — `runtime_declaration_verify_period` is date/month scoped; spacing/amount overlap use the same window.
3. **Shared layer:** `schedule::runtime_declaration_verify_period`; `declaration_store_verify_issues` skips out-of-window periods; DeclarationRefresh soft-skips missing/amount; `cadence_spacing_issues` / `overlap_amount_change_issues` take `as_of` and ignore later-dates outside the window.
4. **Fixtures:** `runtime_verify_skips_months_old_history`; `declaration_refresh_ignores_historical_page_pays`; spacing/overlap unit as_of cases.
5. **Owner DB sample:** CLM/CRF/EPD/ORC/PLTW/TSLW re-collected — all `last_run_ok`; spacing tickets auto-filed on ok; EPD/ORC 2004/2022 amount tickets filed (outside runtime window).

## 2026-10-01 — Derived remaining-year + vendor subset falsely ticketed

1. **Failure class:** `remaining_year` ticket when leftover `derived_walk` fillers coexist with vendor-posted dates. Collect already overwrote same-period derived (BITO Oct 3 → Oct 7 `vendor_payable`); disagree check compared **all** stored dates to vendor-only and ticketed. Wrong fix was year rebuild.
2. **Cadences in scope:** All — `vendor_payables_same_period` is Weekly/Monthly/Quarterly aware.
3. **Shared layer:** `remaining_year_authoritative_disagree` + `is_derived_pay_source`; `ticket_if_remaining_count_mismatch` / `persist_phase_i_remaining_year` ignore derived fillers; auto-file `remaining_year` after successful vendor merge; Fix remaining year = collect heal not `issuer_pay_date_replace`.
4. **Fixtures:** `collector_vendor_overwrites_derived_same_period_no_remaining_year_ticket`; updated remaining_year mismatch golden expects no false ticket.
5. **Owner DB sample:** BITO — `2026-10-07` `vendor_payable`, `2026-11-03`/`2026-12-03` still `derived_walk`, Oct derived superseded. Open `remaining_year` is stale under old rule; Fix remaining year / next collect should auto-file.

## 2026-10-01 — Open tickets stayed opaque despite Steps/Remediation lock

1. **Failure class:** Formatter + goldens shipped; **owner open tickets (BITO remaining_year, YMAX establish/ROC) were never rewritten**. Sync/raise paths that pass plain strings skipped Steps/Remediation until bump.
2. **Cadences in scope:** All — ticket body contract is cadence-agnostic.
3. **Shared layer:** `ensure_ticket_reason_diagnostics` on every `work_ticket_raise_or_bump_run`; `WorkTicketSyncMisses` rewrites any open ticket missing Steps/pass/FAIL/Remediation; `owner-ticket-diag-rewrite` bin for Profile A.
4. **Fixtures:** `sync_misses_rewrites_opaque_open_ticket_reasons` (opaque → diagnostics).
5. **Owner DB sample:** BITO + YMAX open reasons now include Steps + Remediation (verified after rewrite).

**Rule reminder:** Green goldens alone are not Done when open tickets exist — rewrite the live book in the same turn.

## 2026-10-01 — ET Sep invent + plan gap bands + ticket Remediation

1. **Failure class:** Future plan kept a short invent (ET `2026-09-30` after paid `2026-08-19`) instead of issuer/template `2026-11-19` (~92d). Open tickets had Steps/FAIL but no Remediation action plan.
2. **Cadences in scope:** Weekly 5–9 / Monthly 25–35 / Quarterly 90–100 consecutive gaps; count windows 52/368, 12/375, 4/375. All 40 `INCOME_FLEET_SYMBOLS` map to one class.
3. **Shared layer:** `cadence_gap_band` / `filter_dates_to_gap_band` on `remaining_pay_dates_for`; history spacing stays short-only floor; `format_collector_ticket_reason` always appends `Remediation:` + numbered tool steps.
4. **Fixtures:** `plan_cadence_guards` — ET 8/19→drop 9/30 keep 11/19; W/M/Q family + fleet-40 class map; ticket Steps pass+FAIL+Remediation.
5. **Owner DB sample:** After host restart, ET upcoming must be `2026-11-19` (not Sep invent). CadenceHeal still required if live pays were dirty before this filter.

**Next separate risk:** YMAX `collector_establish_incomplete` (ROC establish) — Remediation now locked; confirm live ROC tickets show numbered plan.

## 2026-10-01 — EPD leftover twin + +91 walk

1. **Failure class:** Leftover ex/record stored as `issuer_pay_date` / declaration beside payable (~14d twin); quarterly hole walk used +91 days and invented a near-miss next pay beside IR payable.
2. **Cadences in scope:** Weekly (~7), Monthly (~30), Quarterly (~90) — twin collapse + spacing floors are shared; Income Plan reads healed pay_ons for every locked frequency.
3. **Shared layer:** `short_gap_twin_moves` / `collapse_short_gap_pay_ons` in `schedule`; retrieve + `CollectorCadenceHeal` / DeclarationRefresh fleet heal; +3 calendar months for quarterly derive (not +91).
4. **Fixtures:** Quarterly HTML EPD twins golden; Monthly/Weekly spacing goldens; ET 8-K family covered by mlp_sec path.
5. **Owner DB sample:** EPD, MPLX, ET after CadenceHeal — one mid-Nov payable each; no `2026-10-30` twin on EPD.

**Next separate risk:** YMAX `collector_establish_incomplete` (ROC establish) — not the same class as pay-date twins; ticket reason must show Steps + FAIL for ROC gap.
