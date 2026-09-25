# 2027 Plan horizon — implementation plan (review accepted 20 Sep 2026)

**Status:** review feedback accepted. **S1–S4 shipped** (collect save, June Confirm holes, chart cache, vendor prune). Not a new Authority lock.  
**Supersedes** the recommendation in `docs/architecture/plan-horizon-2027-audit-2026-09-20.md` §7. That audit stays as the pre-feedback pack.  
**Now (execution.md):** Writer SOT reader pass. Amount / ROC Accept/Reject applies to any symbol.  
**Canvases:** `plan-horizon-2027` (locked date rules) · `writer-source-of-truth` (two 2027 writers) · `review-feedback-2026-09-20` (solid / fragile / next)

---

## 1. Feedback audit

The review is accepted. One rule in the 20 Sep audit was **too tight**: “do not keep 2027 in remaining-year” was written as if collect must never *save* a 2027 date. The intended split is:

| Rule from review | Verdict | What that means in this repo |
| --- | --- | --- |
| Collect does not stop at 31 Dec when the vendor page / 8-K already prints 2027. Save those dates. | **Accept. Changes the audit.** | Persist on `issuer_pay_date` / declaration payable (source `sec_8k` or vendor page). Today `persist_mlp_remaining_year` drops extra-year **derived_*** rows; vendor `sec_8k` 2027 should not be dropped. Readers that cap `d <= remaining_year_end` must still use 2027 issuer rows on the **chart** path. |
| Collect does not invent 2027 dates so it can look complete. | **Accept. Keeps the audit.** | `derived_template` / `derived_walk` 2027 is inventing. That is the June **prompt** + derive walk → `assumed_pay_date`. Completeness `remaining_year_matches` stays this calendar year. |
| A later vendor 2027 date prunes the assumed date. | **Accept.** Already in the audit. | Same leftover-to-payable move. Assumed row is not SOT after the vendor prints. |
| 2027 Decl $ does not change Plan $. | **Accept.** Already locked. | `PlanHistoryConfirm` only. Collect tickets; it does not overwrite Plan. |
| MAGI / Tax Planning / YTD remaining end this 31 Dec until a 2027 tax year is opened on purpose. | **Accept.** | No silent tax-year roll. Approach 3 year flag is that later unlock — not this ship. |
| Weekly: same weekday (assumed or known). Monthly: walk day ± weekend, owner pin if wrong. | **Accept.** | No auto Friday/Monday. `RemainingPaymentDateOverride`. |
| EPD: 8-K when it exists; derive quarters only in the hole. | **Accept. Narrows MLP.** | Keep 8-K 2027. Derive `derived_template` only for an unpaid **hole**. Do not derive 2027 to satisfy remaining-year. 2027 holes are the June prompt. |
| Do not widen collector **tickets** into 2027. Do widen collector **saves** when the vendor printed 2027. | **Accept. The sentence that replaces audit §7.** | |

Solid / fragile product notes from the same review are recorded in §6. They are not the 2027 date ship.

---

## 2. Two writers for 2027 dates

```
plan_history ($ / share, empty effective_to)     — dollars, already unbounded
        │
        ├─ issuer_pay_date / issuer_declaration     — vendor-printed dates (any year)
        │     writer: CollectorRetrieve, IssuerPayDateReplace, 8-K persist
        │     2027 allowed when the page/8-K printed it
        │
        ├─ assumed_pay_date                         — holes only
        │     writer: PlanHorizonAssumeNextYear (June prompt, first fill now)
        │     pruned when a vendor 2027 date lands
        │
        └─ remaining_year (this 31 Dec only)        — collector tickets, YTD, Tax, MAGI
```

**2a stays** for assumed holes. Vendor 2027 is **not** a new table — it is the existing issuer calendar, un-capped on **save**.

Do not put assumed rows on `issuer_pay_date` (2b). That is how collect would count them as remaining-year.

---

## 3. What changes from the 20 Sep audit

| Audit said | Plan now |
| --- | --- |
| Collector never keeps 2027 | Collector **saves** vendor-printed 2027; still **drops invented** 2027 derived_* |
| June 1 **job** fills all of 2027 | June 1 **prompt**: owner confirms assumed holes after the walk. First prompt is now (1 June 2026 already passed) |
| Chart reads only assumed 2027 | Chart reads **vendor 2027 ∪ assumed holes** |
| MLP golden “no 2027 in remaining-year” | Split: no *invented* 2027 in remaining-year; published 2027 is kept and is **not** a remaining-year ticket |

Open questions from the audit that this feedback closed: 2a for holes, June cadence, no auto weekend shift, first fill now. Still open: visible “assumed” mark on Income Plan 2027 weeks (default **yes** on the week row, not only the chart). Command name stays `PlanHorizonAssumeNextYear` (not `IssuerPayDateReplace`, not `PlanHistoryConfirm`).

---

## 4. Cadence (locked)

- **Weekly** — same weekday, assumed or known. 100%.
- **Monthly** — walk day from last paid / early-month 3rd. Weekend or holiday may shift one day. Owner pin if wrong. No auto Friday/Monday.
- **EPD / quarterly** — 8-K payable when it exists (`sec_8k`). Derive a quarter **only in a hole** (no 8-K for that slot). Collect does not derive 2027 to look complete. 2027 EPD holes wait for the June prompt (or a later 8-K, which then prunes the assumed row).

Elements: empty `stop_on` stays never. Out of this ship.

---

## 5. Reader split (after ship)

| Reader | Window | Vendor 2027 | Assumed / cadence hole |
| --- | --- | --- | --- |
| Home / Register / Income Plan **forecast** | As-of + duration (12m = as-of + 12 months) | Yes (wins the day) | Cadence walk prints Plan $; assumed/override only pins the day |
| Income Plan week (unlocked) | That Sat–Fri | Yes | Same — missing date does not drop Plan $ |
| Income Plan locked week / Coverage bottom | Lookback Week / Month / last year | n/a | That week’s Plan vs Declared (immutable) |
| Income Plan year-to-go / Cash YTD / Tax projected | This 31 Dec | No | No |
| MAGI | Locked 2026 until owner opens 2027 tax year | No | No |
| Collector `remaining_year` tickets | This calendar year | No (save ≠ ticket) | No invent-to-complete |
| Week Ahead | ~40 days | Only if the date falls in the window | Same |

`remaining_pay_dates_for` cache must key **security + horizon end**, not security only. A Sep 2026–Sep 2027 range cannot reuse the 2026 list.

---

## 6. Product audit (18 Sep tree + 20 Sep cash zip + writer SOT + execution.md)

### Solid — agree

- SQLite SOT and the writer table (element series vs posted `activity_event` vs Friday `cash_minor`).
- Week capture grid, G7 blank cash, recon Adjust.
- Week Ahead Confirm / Edit / Defer.
- Register Calendar \| Trend and shared `AccountCashFlow` on Home.
- Opening cash = Friday `cash_minor`, not leftover SPAXX.
- Planned week $ = Plan $, not Decl; off-calendar actual does not move Plan week.
- Tax Planning + Coverage as CM children; HSA on the tax table; MAGI read-only from `MagiProjectionGet`.
- Last-price 9–4 ET + 4-hour skip + 3s/symbol; miss ≠ $0.
- Honesty fixes on the board: false Elements “Loading Cash Management…”, Car ROC `?? 0`.

### Fragile / still open — agree, not this ship

| Item | Live fact | Disposition |
| --- | --- | --- |
| `App.tsx` ~13.6k lines | **13,869** lines. Nav, dirty, Restart live there. | Structural risk. Do not grow it for 2027. |
| IA — calendar/burndown under Element Management | Coverage + Tax Planning are more CM children. Phone screenshots still not the IA. | Parked. Not the 2027 date ship. |
| 1Y Plan dots die 31 Dec 2026 | Horizon hole. Not an EPD adapter miss. | **This plan.** |
| Income three-line vs one Confirm | Slice 3 locked “Income three lines”; live check still open. | Live check. Not this ship. |
| `monthlyDivsMinor` mixed on Accept | `TrendsCapture` still stores mixed `weekIncome` / `suggestedMonthlyDivsMinor`. Charts that read it are not the Planned column. | Later split. Planned column stays Plan $. |
| Holdings sell typed activity id; TVAL; Dashboard Roth floor; Cart before/after tier | Authority packs parked these. | Stay parked. |
| MAGI engine frozen; no full MAGI screen; INDETERMINATE later | ADR-0013. | Stay frozen. 2027 tax year is an explicit unlock. |
| Restart / migrations 0039–0041 CRLF | Checksums match live DB after LF restore. `.gitattributes` `*.sql eol=lf`. | Ops landmine. Do not rewrite applied SQL. |
| Checking / transfers / household budget | Roadmap. | Correctly out. |

### Next design / product (order)

1. **Writer SOT reader pass** (Home, Register, Coverage, Income Plan) — design pass the review asked for. Maps each screen field to a writer row. No new tables.
2. **2027 dates** (this plan) — collector persist of published 2027 + June prompt + assumed holes + chart merge. Owner-named; not the Except/Reject Now.
3. Do **not** start Checking / MAGI screen / TVAL / phone IA / collector HTML from this review.

---

## 7. Implementation slices (after owner says implement)

Do not edit `schedule.rs` remaining-year helper’s *meaning* for tax/collector. Add a **horizon** argument for chart/register. Do not regenerate MAGI oracles.

**S1 — Collect saves published 2027**  
Stop dropping / ignoring vendor page and 8-K payables with `pay_on` in 2027. Keep dropping invented `derived_walk` / `derived_template` 2027. `remaining_year_matches` and `fix_remaining_year` stay this year. Goldens: keep `recertify_mlp_remaining_year_keeps_one_nov_19` for invented 2027; add “8-K 2027 is stored and does not ticket remaining_year.”

**S2 — June prompt + assumed holes (2a)**  
Table `assumed_pay_date`. Command `PlanHorizonAssumeNextYear`. First run now (2027 holes). Later, 1 June each year. Walk weekly same weekday; monthly walk day; EPD derive only where no 8-K. Owner confirms. `is-unsaved` on that Confirm.

**S3 — Chart / Income Plan merge + cache**  
`remaining_pay_dates_for` key = security + horizon end. `CashRegisterGet` 1Y and Income Plan 2027 weeks read vendor 2027 ∪ assumed holes. Plan $ = `plan_history` × qty. Decl $ on a 2027 declaration does not write Plan.

**S4 — Prune**  
Vendor 2027 payable replaces the assumed row for that slot. Leftover assumed date is not SOT.

**S5 — Reader SOT map** (can run in parallel as design-only)  
One table: screen field → writer row. Home, Register, Coverage, Income Plan.

File → Restart Application after rust; File → Reload the Vite screen for the chart.

---

## 8. Goldens

**Keep green (do not loosen):** leftover record vs payable week; off-calendar actual does not move Plan week; Plan $ ≠ Decl $; YTD remaining null ≠ $0; HSA tax row; week_ahead follows element series; last-price miss ≠ $0; MAGI oracles.

**Reword, do not delete:** MLP “do not keep 2027 in remaining-year” → “do not *invent* 2027 in remaining-year.” Published 2027 is a different golden.

**Add with S1–S4:** vendor 2027 stored, no remaining_year ticket; assumed hole on 1Y; assumed pruned by later 8-K; Tax/YTD still this 31 Dec; cache serves both 2026 and 2027 weeks in one range; weekly weekday preserved.
