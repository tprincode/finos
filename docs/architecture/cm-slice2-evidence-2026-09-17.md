# CM Slice 2 evidence — 2026-09-17

Files: `0039_cash_week_ahead.sql`; `week_ahead.rs` (app + golden); domain `HSA_Withdrawal`; `queries.rs` WeekAhead*; `WeekAhead.tsx`; thin `CashManagement`/`App` mount; contracts + core-functions/ui-modules.
W1–W7: `cargo test -p golden-harness --test week_ahead` 7/7. `g1_g6_slice1b_capture_grid_one_table` green. `cash_management` 11/11.
Confirm: `WeekAheadConfirm` → `CashDistributionPost` `IRA_Distribution` (Income), `SsaConfirm` (SSA_2026/External), `CashDistributionPost` `Withdrawal` (Car), `CashDistributionPost` `HSA_Withdrawal` (Health).
Edit/defer: `WeekAheadEdit`, `WeekAheadDefer`. List: `WeekAheadGet`.
Tables: `cash_element`, `planned_occurrence`.
Capture still one table: `TrendsCapture.tsx` `aria-label="Week capture grid"` + six account labels (W6).
Planned/Reported still present: Cash week desk headers; blank Reported is not stored as 0 (W7).
Register calendar not started.
