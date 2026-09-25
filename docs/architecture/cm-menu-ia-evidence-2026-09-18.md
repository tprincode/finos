# CM menu IA evidence — 18 Sep 2026

- In-app and native **Cash Management** is top-level after Trends. Children, in order: Element Management, Weekly Updates, Car Account Tax Planning, Coverage. Plan keeps the `cash-management` shortcut onto Weekly Updates.
- One screen `cash-management` plus `cmDesk` `elements` | `weekly` | `car` | `coverage`. Menu Working… (`aria-label="Menu working"`) until that pane paints. `elementDirty` blocks leaving elements; `weekWizardActive` / `cashDirty` block leaving weekly.
- Calendar | Trend toggle is rendered in `apps/desktop/src/features/cash/CashRegister.tsx` (`register.series`). That pane mounts on Element Management only.
- When YTD ROC cannot be computed, `CarRocPlanGet.ytdRocMinor` is null (never `$0.00`) and the UI prints the first owner reason, e.g. `no Car payments this year yet`.
- Visible heading is **Car Account Tax Planning**. Query name `CarRocPlanGet` unchanged. M1–M6 in `crates/golden-harness/tests/cash_menu_ia.rs`. File → Restart so the native menu and rust reason field load.
