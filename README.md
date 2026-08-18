# finos — Finance Management System

Local-first desktop financial operating system. **Locked target architecture V1.1** (2026-08-15).

| Layer | V1 stack |
|-------|----------|
| Desktop | Tauri 2 + React + TypeScript + Vite |
| Core | Rust (application + financial domain) |
| Local DB | SQLite (WAL) |
| Future central | PostgreSQL + Rust server + OIDC |

Authoritative architecture: [`System_Architecture_and_Implementation_Roadmap_v1.1_LOCKED_2026-08-15.docx`](System_Architecture_and_Implementation_Roadmap_v1.1_LOCKED_2026-08-15.docx)

Milestone 0 control docs:

- [MVP boundary](docs/architecture/mvp-boundary.md) — V1 in / out / forbidden
- [Component contracts](docs/architecture/component-contracts.md) — commands, queries, events, table isolation
- [Golden harness](tests/golden/README.md) — contract and G-MAGI-01–10 stubs

## Repository layout

```
apps/desktop          Tauri + React (V1)
apps/web              React web client (future)
services/server       Central Rust server (future)
crates/               Rust workspace — domain, application, storage, import, snapshot
packages/             Shared TypeScript contracts and UI components
workers/python-connectors   Optional Python jobs (future)
database/seed         Migration seed data
tests/golden          Golden Business Outcome Harness scenarios
docs/architecture     Architecture references
docs/adr              Architecture decision records (0001–0013)
```

## Milestone 0 — Architecture control

- [x] Monorepo structure and Rust workspace stubs
- [x] ADRs 0001–0013
- [x] MVP boundary
- [x] Component contracts (`packages/app-contracts` and Rust envelopes)
- [x] Golden harness contract and G-MAGI-01–10 (oracles owner-approved 2026-08-18)

## Milestone 1 — Platform proof

- [x] Slice 1: Tauri 2 + React shell on Windows; `HealthGet` via `LocalTauriFinanceClient`
- [x] SQLite WAL, device ID, snapshot, newer-device detection, restore (Windows)
- [ ] Same restore/handoff test on macOS ([m1-macos-gate.md](docs/architecture/m1-macos-gate.md))

## Milestone 2 — Canonical foundation

- [x] Seed counts and critical totals reconcile (`cargo test -p golden-harness seed_counts_and_totals_reconcile`)
- Gate: [docs/architecture/m2-gate.md](docs/architecture/m2-gate.md)

See [tests/golden/README.md](tests/golden/README.md) for the Golden Business Outcome Harness. MAGI pack G-MAGI-01–10 is owner-approved; `cargo test -p golden-harness --features magi-gate` is the M5 golden exit.

## Milestone 3 — Dividend vertical slice

- [x] Fidelity/Schwab-shaped capture → validate → approve → `DividendActual`
- [x] Income Plan / Dashboard / Trends share the same actual total
- [x] Re-import of the same file posts once
- Gate: [docs/architecture/m3-gate.md](docs/architecture/m3-gate.md)

## Milestone 4 — Lots and ROI

- [x] Dual basis (performance vs tax) stay separate
- [x] Explicit `LotAssign` — no FIFO
- [x] CRF zero-cost DRIP policy; FI Roth automatic capture only
- [x] Broker lot reconciliation after sales/option closes
- Gate: [docs/architecture/m4-gate.md](docs/architecture/m4-gate.md)

## Milestone 5 — Planning and tax

- [x] Owner-approved G-MAGI-01–10 pass on the production path
- [x] G10-ADJ-1 owner-approved; form gap closes after the adjustment
- [x] Versioned `PlanApprove` / `PlanGet`; Plan remaining is not actual cash
- [x] `BurndownGet` obligation is Plan remaining
- Gate: [docs/architecture/m5-gate.md](docs/architecture/m5-gate.md)

## Milestone 6 — Decision intelligence (in progress)

- [x] Slice 1: `AllocationTargetSet` / `AllocationGet` do not post cash
- [x] Slice 2: Shopping cart (`CartItemAdd` / `CartItemRemove` / `CartGet`) is not a fill
- [x] Slice 3: Backtesting stub (`BacktestRun` / `BacktestGet`) does not post facts
- [x] Slice 4: Classification review does not rewrite MAGI oracles
- [x] Slice 5: AI Gateway (`AiAnalyze`) is advisory-only; Grok key from env
- Gate: [docs/architecture/m6-gate.md](docs/architecture/m6-gate.md)

Active work board: [docs/architecture/execution.md](docs/architecture/execution.md) (Now / Next). Do not wait for a re-attached plan.

## Development

**Rust** (from repo root):

```
cargo check
cargo test
```

**Desktop app** (see `apps/desktop/README.md`):

```
npm install
npm run desktop
```

Start command: `npm run desktop` (from repo root). Exit: File → Exit, the Exit button, the window X, or `Ctrl+C` in that terminal.

The window should show HealthGet, HandoffStatusGet, CanonicalWeekGet (Sat–Fri), account/exception lists, dividend actual totals, lots/ROI, MAGI decision, and Calculator Plan / burndown.

## Governance

Changes to framework, domain language, databases, component boundaries, precision policy, device handoff or client contracts require an approved ADR and a new version of the target architecture document.
