# Spreadsheet replacement (Profile A)

Product truth: [docs/Authority](../Authority/README.md). Order: Architecture V1.1 → BR-X → matching domain doc → production seed gates → code. [docs/architecture](.) is the execution board, not a substitute for domain specs.

Usable means: a researcher can do the spreadsheet job on the seeded data **without the xlsx open**. Gate-green is not sufficient. Seed is CLI `npm run data-seed`, not an owner screen. Tests use a temp DB; `%LOCALAPPDATA%\finos` is a different file.

## Feature → specification

| Feature / business process | Where it is specified |
|----------------------------|------------------------|
| Dividend capture once → plan vs actual | Income Plan target architecture; Calculator (declaration vs Plan vs actual); BR-X dividend-state rules |
| Weekly Income Plan / week close | `docs/Authority/domains/Income_Plan_Target_Architecture_v1.0_2026-08-13.docx`; Sat–Fri week in Architecture V1.1 |
| Holdings / lots / dual cost / no FIFO | Holdings working foundation; ROI ledger architecture |
| Position master (tier, ROC, frequency) | Position Details requirements; `Template_Positions.xlsx` |
| Prices | Last Price target architecture (parked until named) |
| Burndown | Income Plan + Dashboard; BR-X |
| Trends / non-ROI | Trends target architecture; disbursement template |
| Dashboard views | Dashboard target architecture |
| Marketplace MAGI / cliff | Architecture V1.1 G-MAGI-01–10; BR-X; never rewrite oracles |
| Shopping cart | Shopping Cart target architecture (later) |
| Accounts, cash symbols, floors | `Template_Accounts.xlsx`; BR-X |
| Import idempotency, exceptions | Import-engine; BR-X; unknown ≠ zero |

## Jobs vs desktop vs usable-means

| Spreadsheet job | Seed / contract | Desktop today | Usable means |
|-----------------|-----------------|---------------|--------------|
| Accounts | 8; `AccountList` / `AccountRegister` / `AccountUpdate` | Name + kind | Full list; register/update; no blank→zero |
| Positions | 74; `PositionDetailsGet` | Cost-basis rollup | Full rollup; filter by account/symbol; dual basis (no market value until Last Price) |
| Lots / dual basis | 1679 / 1250; `BasisGet`, `LotAssign` | Truncated | Full open lots; filter; explicit `LotAssign` (no FIFO) |
| Yield | 5862; `DividendGet`, `ActivityList` | Truncated + one total | Full dividend activity; `DividendGet` total; Income Plan planned vs actual |
| Disbursement | 129 five types | Flat list | Full list; gross visible; non-ROI types named |
| Import / approve | `ImportStage` → validate → approve → post | Auto-post | Review validate/exceptions **before** post; idempotent re-import |
| MAGI / tax | `MagiProjectionGet` / `TaxProjectionGet` | Empty after seed | Show projection; if coverage incomplete, record facts/coverage from UI; never SAFE from missing data |
| Calculator Plan / burndown | `PlanApprove` / `PlanGet` / `BurndownGet` | Read-only empty | `PlanApprove` from UI; remaining ≠ actual cash |
| Dashboard / Trends | `DashboardGet` / `TrendsGet` | Same dividend total | Same actual as DividendGet until a later reporting slice (domain docs); do not mix declarations |
| Allocation vs lots | `AllocationGet` / `AllocationTargetSet` | Crude form | Targets vs open-lot **cost basis**, not market value |
| ROC / 1099 | `DistributionCharacterize` / `DistributionGet` | Missing | Later; must not rewrite MAGI oracles |
| Snapshot / handoff | Platform commands | Settings | Keep under Settings |
| Cart / backtest / AI | M6 stubs | Primary page | Hidden from primary nav |
| Prices / cutover | Named later | — | Parked |

Locked: unknown amounts stay unknown; no FIFO; Plan, declaration, and actual stay separate; AI is advisory only; no UI SQL; `LocalTauriFinanceClient`.
