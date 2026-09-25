# Cash Management — post–Slice 1b update for external review

**Date:** 17 September 2026  
**Follows:** [cm-slice1b-status-2026-09-17.md](cm-slice1b-status-2026-09-17.md)  
**Product:** finos desktop (Profile A, local SQLite)  
**Audience:** same reviewers as the 14 Sep design pack and the 1b status  
**Not started:** Slice 2. `CashManagement.tsx` was not rewritten.

---

## Verdict

**Slice 1b still stands (G1–G7).** After the week dropdown the owner sees one six-row table and Accepts on that screen via `WeekCaptureAccept`. Blank cash still skips (not $0, no Adjust).

**Post-1b change that reviewers must see:** the single **Week income** figure is gone. The desk, capture review, and Trends now show two columns:

| Column | What it is | What it is not |
|---|---|---|
| **Planned weekly income** | Issuer Decl $ × remaining shares for that Sat–Fri week | Confirm Plan $; owner-typed Profit; seed Profit |
| **Reported weekly income** | Paid brokerage actuals (`dividend_actual` / Income Plan actuals) in that week | Declarations; blank does not mean $0 |

This split was forced by a live W37 validation: the owner did not believe they had provided any week data, yet the system showed **$896.68**. That number was collector-stored payables, not household transactions.

---

## 1b recap (unchanged)

| ID | Requirement | Status |
|----|-------------|--------|
| G1 | Six rows at once; no Next between accounts | Pass |
| G2 | Typed ETF 10000 → 70% is 7000 | Pass |
| G3 | Blank ETF is —, not $0 | Pass |
| G4 | Edit keeps values on the same table | Pass |
| G5 | No Adjust when typed cash matches reference | Pass |
| G6 | Speculation is a row | Pass |
| G7 | Blank cash allowed; skip update; no Adjust | Pass |

Locks: `golden-harness` `accessibility::g1_g6_slice1b_capture_grid_one_table` and `trends_capture::blank_cash_skips_update_and_does_not_adjust` (13/13 on 17 Sep after the income split). Accept remains `WeekCaptureAccept` (Slice 1 / PR #10 recon kept).

**10-second check (still valid):** open capture. If all six accounts show without Next, 1b is still landed. If the owner still walks one account at a time, reject.

---

## What changed after 1b

### Surfaces

- `apps/desktop/src/features/cash/CashWeekDesk.tsx` — two income columns on the saved-weeks table.
- `apps/desktop/src/features/graphing/TrendsCapture.tsx` — capture review labels match the desk. Accept still writes `monthlyDivsMinor` from the mixed suggestion (`suggestedMonthlyDivsMinor`) so stored DIVS charts do not change meaning in this slice.
- `apps/desktop/src/features/graphing/TrendsCharts.tsx` — two charts (Planned / Reported), not one mixed series.
- `TrendsWeekGet` body now includes `plannedWeeklyIncomeMinor` and `reportedWeeklyIncomeMinor` (`contracts.rs`, `trends_app.rs`).
- Core split: `week_aligned_income_parts` in `queries.rs` (planned = sum of `declaration_known`; reported = sum of `actual_known`). Mixed helper still prefers reported when any actual exists, else planned, **never Plan $**.

Capture still lives in `features/graphing/`; the **screen** is still Cash Management.

### What did not change

- One-table capture grid and G7 blank-cash rule.
- `WeekCaptureAccept` recon (match → no Adjust; material gap → reason; blank cash → skip).
- Income Plan weekly report: Plan / Decl / Decl−Plan. Broker Actual $ stays off that screen.
- `DividendPerformanceGet` still **excludes the in-progress week** (golden: this/future week must not appear). Current-week Planned is overlaid from `TrendsWeekGet`, not from performance history.
- Seed/manual Profit is still not shown as week income.

---

## Key implementation findings

### 1. “Week income” was two facts wearing one label

`weekIncomeMinor` / `week_aligned_income_minor` preferred paid actuals, else issuer Decl $. On a week with no brokerage actuals the UI showed collector payables as if they were household money. The owner read **$896.68** as transactions they had entered. They had entered none.

### 2. W37 (12–18 Sep 2026) live SQLite

| Fact | Value |
|---|---|
| `dividend_actual` in week | **0 rows** |
| Interest / income `activity_event` in week | **0 rows** |
| Saved `trends_week_source` for W37 | **none** (W35/W36 DIVS are prior captures ~$4,906 / $4,912) |
| Source of $896.68 | Remaining lots × collector `issuer_declaration` |

Eighteen names sum to **89668 cents**. Largest: TOPW $225.00 (16 Sep), QQQI $223.16 (18 Sep), NVDW $81.70 (15 Sep). Sources: Roundhill, YieldMax, NEOS. Several YieldMax rows were entered **17 Sep** (collector), not owner import.

Also stored for **15 Sep**, and **not** in the $896.68 total: **CLM $34.38** and **CRF $123.51** (Cornerstone, entered 28 Aug). Those two are the open `paid_payable_supersede` names. If counted, W37 declared cash is **$1,054.57**. Reviewers should not assume Planned equals “every declaration in the calendar week” without stating the CLM/CRF exception.

### 3. Planned is Decl $, not Confirm Plan

Income Plan already has Plan vs Decl. The new column name “Planned weekly income” means **expected issuer cash for that Sat–Fri week**, not owner Confirm Plan. Summing fleet Plan when Decl/actual are unknown was the old ~$4k weekly figure; that path stays closed.

### 4. Reported stays honest when empty

Blank Reported is “no paid rows,” not zero dollars. Same shape as G7 blank cash and blank ETF. Do not invent $0 so the column looks complete.

### 5. Current week vs closed weeks

Performance weeks stop before `thisWeekStart`. Closed weeks can show Planned from `declarationMinor` and Reported from actual points. The open week uses `TrendsWeekGet` parts so W37 Planned is visible while Reported is blank.

### 6. Accept save is still mixed on purpose

`suggestedMonthlyDivsMinor` (paid if any, else Decl) is what Accept still stores as `monthlyDivsMinor`. Reviewers should not treat that stored DIVS field as the new Planned column. A later slice can split the stored week row if charts must persist both numbers.

---

## Reviewer do / do not

**Do**

- Confirm the capture grid is still one table (G1–G7).
- Confirm the desk shows Planned and Reported as separate columns.
- Treat W37 Planned as collector Decl × lots until brokerage actuals exist.
- Ask whether CLM/CRF should join Planned while `paid_payable_supersede` is open.

**Do not**

- Require Next between accounts.
- Treat blank cash, blank ETF, or blank Reported as $0.
- Call Planned “Plan $” or “owner-provided transactions.”
- Start Slice 2 or rewrite `CashManagement.tsx` from this update.
- Move week entry back onto Trends.
- Regenerate MAGI oracles.

---

## Adjacent (not 1b, not this income split)

DeclarationRefresh on desktop open still runs **once per local calendar date**. Retrieve declarations still force-runs. Last-price charts now use remaining qty **on the event date** (separate Home/Trends work). Graphing period default on Home and Trends is **6 months**.
