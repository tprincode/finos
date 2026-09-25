# CM Slice 3 evidence — 2026-09-18

Files: `cash_register.rs` (app + golden), `CashRegister.tsx`, `CashElementEditor.tsx`; Week Ahead Edit opens the editor; thin `CashManagement.tsx` / `App.tsx` mount.
R1–R8: `cargo test -p golden-harness --test cash_register` (Car 1M Actual+Projected, same Calendar/Trend series, Confirm → Actual posted amount, Add Element + E2 freeze, missing start —, Income three lines, W1–W7/G1–G7 source lock, Income Plan day Deposit absent from Week Ahead).
Switcher: Income | FI Roth | Health | Car | Account 9 | SSA_2026. No Speculation Register.
Frequencies: Weekly | Monthly | Annual | One-time. Horizon fill from cadence; `ensure_seed` does not re-create elements.
Week Ahead list still skips `note=dividend` / CLM / CRF; Register Plan deposits are Income Plan Plan $ on the payable day (virtual, not `planned_occurrence`).
Confirm remains one post per occurrence (`WeekAheadConfirm`). E3: `CashElementDelete` / `PlannedOccurrenceDelete` reject confirmed children.
`is-unsaved` only on Element Save while dirty. Period chips 2W | 1M | 3M | 1Y (default 1M; 1Y = Jan 1–Dec 31 of asOf).
File → Restart Application so the rust host loads `CashRegisterGet` / element commands.
Stop. Slice 4 / checking / transfers / MAGI / household budget not started.
