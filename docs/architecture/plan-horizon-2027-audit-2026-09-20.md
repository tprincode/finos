# Plan horizon 2027 — external audit (20 Sep 2026)

**Status:** pre-feedback pack. Review accepted the same day.  
**Superseded recommendation:** §7 “assume all of 2027, collect never keeps 2027.”  
**Live plan:** `docs/architecture/plan-horizon-2027-plan-2026-09-20.md` — collect **saves** vendor-printed 2027; June **prompt** + 2a only for holes; tickets / MAGI / Tax stay this 31 Dec.  
**Companion canvases (open beside chat):**

- Writer SOT (live writers + one proposed 2027 row): `.cursor/projects/c-Users-EVTom-repo-finos/canvases/writer-source-of-truth.canvas.tsx`
- Horizon comparison: `.cursor/projects/c-Users-EVTom-repo-finos/canvases/plan-horizon-2027.canvas.tsx`

Working writer note: `docs/architecture/writer-source-of-truth-2026-09-20.md`.

---

## 1. Problem the 1Y chart shows

Income Plan **Plan $/share** already continues. Empty `plan_history.effective_to` means the current rate applies to any future pay date. The missing piece is **pay dates after 31 Dec of the as-of year**.

Home and Register Account cash flow (`AccountCashFlow.tsx`) ask `CashRegisterGet` with `period: "1Y"` for the axis window (from the 1st of this month forward). Plan dots come from `income_plan_week_views_in_range` → `remaining_pay_dates_for`. From 20 Sep 2026 a 12-month window includes Jan–Sep 2027. Those weeks have **no Plan dividend dates**, so the cash line is Element-only and the chart is unusable.

Elements are not the bug. Empty `cash_element.stop_on` already means never. Horizon fill already walks the requested window. Default `stop_on` stays empty.

## 2. Why 2027 is empty today (intentional)

These cuts are current-year by design. Do not “fix” them by widening remaining-year.

| Cut | File | Rule |
| --- | --- | --- |
| Year-end helper | `crates/financial-domain/src/schedule.rs` `remaining_year_end` | `31 Dec` of `as_of` |
| Same string helper | `crates/financial-domain/src/roc.rs` `remaining_year_end` | `YYYY-12-31` from as-of year |
| MLP persist | `crates/application-core/src/queries.rs` ~3527 | Comment: do not keep 2027 in remaining-year |
| Collector completeness | `crates/financial-domain/src/collector.rs` `remaining_year_matches` | Planned vs issuer counts for **this** calendar year |
| Cash YTD / Tax Planning remaining | `crates/application-core/src/cash_ytd.rs` | Tomorrow through **this** `Dec 31` |

A second cut makes a 2026–2027 range even worse:

1. `income_plan_remaining_as_of(week_end)` uses **1 Jan of that week’s year**, then `remaining_year_end` is 31 Dec of that year.
2. `remaining_pay_dates_for` caches **per security only**. A Sep 2026–Sep 2027 range computes 2026 dates on the first week and reuses them for 2027 weeks.

Widening `remaining_year_end` without a reader split would mix 2027 into collector tickets, MAGI, and Tax Planning projected. That is out of scope and wrong.

```
plan_history ($ / share, empty effective_to)
        │
        ├─ remaining_year (through this 31 Dec) ── Cash YTD, Tax Planning, collector remaining_year
        │
        └─ 2027 assumed dates (new, after review) ── Home / Register 1Y Plan dots only
```

## 3. Locked constraints (all three approaches)

- Do **not** move collector `remaining_year`, MAGI, or Tax Planning projected into 2027.
- Do **not** treat assumed 2027 dates as `issuer_declaration` or as seed Profit.
- Do **not** write assumed dates through `IssuerPayDateReplace`.
- **Weekly** = same weekday every week (already 100%).
- **Monthly / quarterly** = same cadence as 2026. Weekend or holiday may shift a day. Do **not** auto-guess Friday vs Monday. Owner pin stays `RemainingPaymentDateOverride`.
- Dollars stay `plan_history` × remaining qty. ROC 2026 estimate may **label** a 2027 hit as ordinary/ROC assumption. It is not a 2027 MAGI input.
- First fill is **now** (20 Sep 2026). 1 June 2026 already passed, so 2027 is assumed on first ship, then on each later 1 June.
- Leftover assumed dates are not SOT — prune like `planned_occurrence` when a vendor payable arrives.

## 4. Horizon policy (owner)

**Annual include-next-year on 1 June**, not a rolling 12-month window. Minimum: 100% of calendar 2027.

| As-of | Stored next year | 1Y chart (as-of + 12 months) |
| --- | --- | --- |
| 20 Sep 2026 (today) | 2027 (first ship, June 1 already passed) | Covered through Sep 2027 |
| 1 Jan–31 May 2027 | 2027 only until 1 June 2027 | Hole in Jan–May **2028** |
| 1 Jun 2027 | 2028 populated | Covered through May 2028 |

The Jan–May hole in year+2 is accepted for maintainability. An optional later stub (Approach 1 walk, not stored) can fill that hole without a second annual job. Do not add that stub in the first ship.

## 5. How dates would be generated (shared walk)

Reuse the existing walk. Do not invent a second calendar.

- **Weekly:** remaining Sat–Fri weeks (`weekly_starts` / same weekday). 100% deterministic.
- **Monthly:** `derive_remaining_pay_ons` — early-month names (paid day 1–10) on the 3rd; others +1 month from latest paid day-of-month. Loop long enough to reach 31 Dec of the **next** year when generating assumptions (today the loop stops at this 31 Dec and is capped at 12 steps).
- **Quarterly:** +3 calendar months from last accepted payable (MLP template) or ~91 days (generic). Same weekend caveat.
- **Issuer calendar:** published 2026 `issuer_pay_date` rows stay remaining-year. They do not become 2027 assumptions. A later vendor 2027 payable **replaces** the assumed row (same leftover-to-payable move already used in 2026).

## 6. Three approaches

### 1 — Derived walk, no stored 2027 dates

Pass a **horizon end** (`2027-12-31`) into the walk for chart/register readers only. Tax, collector, and Income Plan year-to-go keep passing this 31 Dec. Fix the `remaining_pay_dates_for` cache key to include horizon year.

- **Writers:** none new. `plan_history` still owns $.
- **Pros:** smallest code; no new table; 1Y chart works once the cache is keyed.
- **Cons:** 2027 dates are invisible except on the chart; owner cannot inspect or pin a weekend without a store; every query recomputes.

### 2 — Persist assumed dates (inspectable, editable)

On 1 June (and on first ship), write one assumed payable per remaining slot for the **next calendar year**. Provenance `assumed_next_year`. Generate with the Approach 1 walk.

Pick **one** store. Do not invent both.

| Option | Store | Writer | Risk |
| --- | --- | --- | --- |
| **2a (recommended)** | New table `assumed_pay_date` (`security_id`, `pay_on`, cadence, provenance, `assumed_on`) | `PlanHorizonAssumeNextYear` | New writer; must prune leftovers |
| **2b** | Rows on `issuer_pay_date` with a provenance column | Must not be `IssuerPayDateReplace` | Collector `remaining_year` will count them unless every reader filters |

Owner weekend fix stays `RemainingPaymentDateOverride`. Vendor 2027 payable replaces the assumed row.

- **Pros:** dates are facts you can audit; one annual command; weekend pins have a home.
- **Cons:** new writer; leftovers are not SOT (same class of bug as Element `planned_occurrence`).

### 3 — Year-window clone (Plan rate + cadence only)

Do not persist dates. Persist a year assumption: “2027 uses current Plan $/share and cadence.” Empty `effective_to` already does the dollars. A small `plan_year_assumption` (year, `confirmed_on`) makes the year explicit so 2026 MAGI cannot swallow 2027. Dates still come from Approach 1 at read time.

- **Pros:** honest year boundary for tax; no date leftover table.
- **Cons:** same invisible-date problem as 1; Confirm Plan this year must not silently lock 2027 without the assumption row.

## 7. Recommendation

**Approach 2a + 1 June job**, generated by the Approach 1 walk.

- Most accurate to **maintain**: one annual command, one inspectable list, owner pins weekends.
- Meets **100% of 2027**.
- Keeps remaining-year / collector / tax on 2026.
- Approach 3’s year flag is optional later if Tax Planning grows a 2027 column. Not required to fix the chart.

Proposed writer (not shipped):

| Fact | Table | Writer | Note |
| --- | --- | --- | --- |
| Assumed next-year pay date | `assumed_pay_date` | `PlanHorizonAssumeNextYear` | Provenance `assumed_next_year`. Not `issuer_declaration`. Not remaining-year. |

## 8. Reader split after a 2a ship

| Reader | Window | Reads assumed 2027? |
| --- | --- | --- |
| Home / Register 1Y (`AccountCashFlow` → `CashRegisterGet`) | Axis window (month-1st + duration) | Yes |
| Income Plan week/grid Plan $ for a 2027 week | That Sat–Fri week | Yes (date + `plan_history` × qty) |
| Income Plan year-to-go / remaining periods | This 31 Dec | No |
| Cash YTD remaining / Tax Planning projected | Tomorrow–this 31 Dec | No |
| MAGI | Locked 2026 oracles | No |
| Collector `remaining_year` / `fix_remaining_year` | This calendar year issuer dates | No |
| Week Ahead (~40 days) | Requested window | Only if a date falls in that window |
| Elements | `stop_on` empty = never | Unchanged |

## 9. Goldens that must stay green (do not regenerate MAGI)

Any later ship must keep these classes green. This pack does not run them.

- Collector: `remaining_year` count mismatch, leftover record vs payable week, `unchanged_ok_auto_files_open_retrieve_miss`, `pay1_stale_expected_4_remaining_year_closes_without_december`, `recertify_mlp_remaining_year_keeps_one_nov_19`.
- Income Plan: `issuer_calendar_remaining_year_uses_published_dates`, `wz_monthly_remaining_year_without_actuals_schedules_income_plan`, off-calendar actual does not move Plan week.
- Cash: `week_ahead_follows_element_series`, cash_slice4 YTD remaining null ≠ $0, Tax Planning HSA row.
- MLP: persist comment — do not keep 2027 in remaining-year.

New goldens (only after a 2a ship, not this turn):

- 2027 assumed dates appear on a 1Y `CashRegisterGet` Income window that crosses 1 Jan.
- Those dates **do not** increment collector `remaining_year` planned count.
- Cash YTD / Tax Planning projected still end 31 Dec of as-of year.
- `remaining_pay_dates_for` cache key includes horizon year (2026 week + 2027 week in one range both resolve).
- Weekly assumed dates keep the same weekday; monthly/quarterly keep the walk day (no auto weekend shift).
- Vendor 2027 payable replaces the assumed row; leftover assumed date is pruned.

## 10. Open questions for the external reviewer

1. Confirm **2a** (`assumed_pay_date`) over 2b (`issuer_pay_date` + provenance). 2b is fewer tables and a higher leftover risk.
2. Confirm **1 June** annual fill vs a rolling 12-month window. The Jan–May year+2 hole is documented and accepted unless the reviewer requires the optional stub.
3. Confirm **no auto weekend shift** for monthly/quarterly assumed dates. Owner pin only.
4. First ship backfill: run the June job immediately for **2027**, not wait until 1 June 2027.
5. Should Income Plan’s 2027 week rows show a visible “assumed” mark, or only the chart?
6. Command name `PlanHorizonAssumeNextYear` is TBD. It must not be `IssuerPayDateReplace` or `PlanHistoryConfirm`.

## 11. Out of scope until owner picks 2a

Do not edit `schedule.rs`, `cash_ytd.rs`, `AccountCashFlow.tsx`, or collector completeness for this pack.

After approval only: add `assumed_pay_date` + June 1 / first-ship command, tag provenance, filter collector/tax, fix `remaining_pay_dates_for` cache to include horizon year, add the new goldens. File → Restart Application after rust; File → Reload the Vite screen for the chart.

Execution **Now** stays Except/Reject tickets (TSPY amount, HAKY). This pack is owner-named review work, not that gate.
