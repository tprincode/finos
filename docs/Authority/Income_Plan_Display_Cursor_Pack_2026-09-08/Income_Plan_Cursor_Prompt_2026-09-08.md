# PASTE THIS INTO CURSOR

Attach only:
- Income_Plan_Grid_and_Week_Display_Requirements_2026-09-08.docx
- Income_Plan_Print_and_Export_2026-09-08.docx
- Income_Plan_Target_Architecture_v1.0_2026-08-13.docx
- income_plan_display_golden_pack_2026-09-08.json

Optional visualization (do not ship, do not copy into src):
- Income_Plan_Pattern_A_B_Simulation.html

Then paste:

---

Implement Income Plan display Patterns A and B exactly as Income_Plan_Grid_and_Week_Display_Requirements_2026-09-08.docx, and Print / Export exactly as Income_Plan_Print_and_Export_2026-09-08.docx. Those docs plus the 13 Aug Income Plan architecture are authority. The HTML mock is a picture of layout and navigation only. Do not add it to the desktop app, do not iframe it, do not use it as the print engine. Match information architecture and behaviors. Ignore CSS, chip style, and navy chrome.

## Slice boundary

Build only the Income Plan two-pattern display, the queries that feed it, and Income Plan Print / Export (print to page, save PDF, export Excel).

Do not reopen Calculator Plan ownership, collector last_run meaning, ROC, MAGI, Dashboard burndown, Holdings TVAL, Shopping Cart, Add Position wizard, or adapter hosts.

Do not persist owner identity. Tests use WEEK1 / MON1 / QTR1 / CASH1 only. Do not copy live household tickers into fixtures.

## What to build

Pattern A is two stacked tables that must not share columns.

Table 1 — account rollup only:
- Columns: row label, current-year summary, one column per visible week (Friday week-ending label).
- Rows in order: Delta to plan %; Total Plan; plan by selected account (Income, Health, FI Roth/Roth, 9, CAR); Total Actual; actual by the same accounts; Total Difference.
- Year column: totals on Total Plan / Total Actual / Total Difference / Delta; weekly averages on account detail rows.
- No Last Update, no symbol, no quantity, no annual %, no frequency group.
- Future Actual and Difference are null, not 0.
- Account multi-select drives which account rows and which dollars appear. Default on = Income, Car, Health, FI Roth, 9. Default off = Speculation, Energy, Robinhood.

Table 2 — position plan grid only:
- Frozen columns: Symbol, Last Update, then week columns.
- Last Update = last successful declaration-collector run date. Failed run is blank. Price collector is not this field.
- Remove live-sheet Sum of Quantity, Sum of Total $, Average of Annual %.
- Group Monthly / Quarterly / Weekly. Empty frequency is a ticket under Other.
- Default measure is Planned $ for selected accounts.
- Monthly/quarterly $ only in the planned pay week. Weekly $ every week. Off-cycle empty is not a miss and is not red.

Page controls: historical weeks (default 6, 0–26), future weeks (default 6, 0–26). In-progress week is not future.

Pattern B — one Sat–Fri week:
- Open from a Pattern A week header or week $ cell.
- Show Plan $, Actual $, variance, miss count, amount-exception count.
- Position rows: symbol, frequency, Last Update, Plan $, Declaration $, Actual $, Variance.
- Plan $ on a closed week is the PlanHistory amount in effect on that pay date, not the current Plan.
- Week strip changes week inside B. Back returns to Pattern A with that week still selected.

## Plan lock (must implement)

Future weeks use current Plan $/share.
Closed weeks freeze the Plan amount that was in effect on that pay date.
Declarations, collector runs, and actuals never write Plan.
Shares = open lots in selected accounts as-of week-end for past weeks; current open lots for future weeks.

Golden G-IP-03: WEEK1 100 sh Car, plan 0.15 through 2026-08-14, plan 0.17 from 2026-08-15. Week ending 2026-08-14 Plan $ = 15.00. Week ending 2026-08-21 Plan $ = 17.00.

## Color / empty rules

Table 2: future plan neutral; past actual within $1 of plan green; past variance amber; planned-not-paid red; not-a-pay-week empty and not red.
Table 1 Total Plan and Total Actual week cells may use the live-sheet green total treatment. Do not paint Table 1 account rows as misses.

## Data

Plan $ from PlanHistory + lots.
Actual $ from Activity Ledger dividend + cash interest.
Week = Saturday–Friday, header = Friday ending.
Last Update from declaration collector success timestamp only.

## Print / Export (same slice)

Primary control: Print / Export on the Income Plan page toolbar (same bar as account chips and week-window counts). Actions: Print to page, Save PDF, Export Excel.

Secondary: File → Print current view / Export current view call the same action only when Income Plan is focused. Ctrl/Cmd+P when Income Plan is focused opens Print to page for the current snapshot.

Binding: output equals the as-displayed configuration. Pattern A print includes Table 1 then Table 2 with current accounts and week window. Pattern B print is that one week only. Hidden rows stay hidden. Future Actual / Difference stay blank, not 0. Table 1 output has no Last Update column. Table 2 output has Last Update and does not have quantity / ytd_total / annual_pct.

Every PDF and Excel cover/header lists: pattern, printed-at, selected accounts, week window, and for B the Friday week-ending. PDF default path is local. Pattern A default paper is landscape. Excel Pattern A = AccountRollup + PositionGrid + Cover. Excel Pattern B = WeekDetail + Cover.

Do not invent print for Holdings or Dashboard in this slice.

## Tests / golden harness

Add tests that assert every id in income_plan_display_golden_pack_2026-09-08.json including G-IP-P01..P10. Fixture names stay WEEK1 / MON1 / QTR1 / CASH1. G-IP-10 fails if the companion HTML is added under src/ or the Tauri bundle. G-IP-P08 fails if print calls that HTML.

If the repo already has a Golden Business Outcome Harness, register these goldens there. If not, add a focused test module. Fixture green is the gate for this slice.

## Stop

Stop when G-IP-01..10 and G-IP-P01..P10 pass. Return files touched, test command, pass/fail per golden, confirmation that Table 1 has no Last Update column, and confirmation that Print / Export lives on the Income Plan page and File menu delegates to it. Do not write a new architecture doc.
