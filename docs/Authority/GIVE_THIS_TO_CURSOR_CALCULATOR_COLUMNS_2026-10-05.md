# Calculator repair — give this to Cursor

Date: 5 October 2026
Checked against finos-source-2026-10-05-e06516a.

Do not reinterpret. Symbol is the stored position key. One row documents that position. It is not a lot row and not a 60-day slice.

## What the code did

- `App.tsx:1045` starts history at `"60"`. Refresh calls `DeclarationHistoryGet` with `period: "60"` (`App.tsx:1333`). `historyBounds` (`App.tsx:843`) turns every other preset, and an empty custom start, into today minus 60 days.
- `week.rs:89` `history_window_start` is `as_of - 60 days`. The screen copy at `packages/ui-components/src/index.tsx:4063` states that default as if it were the rule.
- Week columns are newest first (`calculator_plan.rs`: `weekEnds[0]` is the newest Friday).
- `lastPaidCents` and `lastPaidDeclarations` (`index.tsx:4132` and `:4352`) walk `for (let i = cells.length - 1; i >= 0)`. The comment says newest first. The loop starts at the older end of the visible window.
- Avg 3, Avg 6, 3-pay yield, Recent / share, Current pay, Plan Δ, and Plan check all read those visible cells. Changing Period changes them. Only Paid is a view sum.

## Required fix

A Period control may choose which week columns are visible. Default visible range is a trailing year, or all in-force history. Not today−60.

Most Current, Avg 3, Avg 6, 3-pay yield, Plan check, MC FWD, Current pay, and Plan Δ read in-force issuer declarations, newest pay date first, ignoring the visible window. Walk from the newest week, not `cells.length - 1`.

Blank is not $0. A declaration is not broker cash. Div recv / Cost rec / ROC $ stay on the ledger. Plan is never auto-posted. Weekly annualization stays 52 until the owner closes 52 vs 53.

Year-end without a 53rd planning period: Annual = Plan × qty × 52. A calendar year may still show an extra Sat–Fri week (Saturday-year week identity), a Coverage / Year chip slot (as-of → +12m), a residual stub after 52 pays, or a count of expected pays in a named range — none of those rewrite `planningPeriodsPerYear` to 53. ISO week-year is not a V1.1 candidate while Sat–Fri is L0. Ledger R-23; golden `plan_cadence_guards::weekly_planning_periods_are_fifty_two_not_fifty_three`.

Column intent and corrected formulas: `Calculator_Column_Contract_for_Cursor_2026-10-05.xlsx` sheets Column contract, Code evidence, Do not.
