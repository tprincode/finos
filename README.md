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
apps/web              React web client (M9; RemoteHttpFinanceClient + OIDC)
services/server       Axum HTTP adapter over Postgres (M8 proof; loopback)
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

## Milestone 6 — Decision intelligence (**architecture proof**)

These checkboxes are **crate/API proofs**, not a claim that owner Profile A UX is complete.

- [x] Slice 1: `AllocationTargetSet` / `AllocationGet` do not post cash (**API proof**; no owner Allocation menu)
- [x] Slice 2: M6 `CartItemAdd` / `CartItemRemove` / `CartGet` is not a fill (**API proof**)
- [x] Slice 3: Backtesting stub (`BacktestRun` / `BacktestGet`) does not post facts (**API proof**; no owner Backtest menu)
- [x] Slice 4: Classification review does not rewrite MAGI oracles
- [x] Slice 5: AI Gateway (`AiAnalyze`) is advisory-only; Grok key from env (**parked UX**)
- Gate: [docs/architecture/m6-gate.md](docs/architecture/m6-gate.md)

**Owner Profile A product (separate from M6 boxes):** live Shopping Cart scenarios, Tools → Components inventory, and Cash Management desk ship on the recovered product tip — track via `execution.md` / `ui_modules` / `cash_management`, not these M6 checkboxes.

## Milestone 7 — Release hardening (in progress)

- [x] Slice 1: Backup/restore recovery drill
- [x] Slice 2: Signed NSIS installer (`npm run desktop:build` → `finos_0.1.0_x64-setup.exe`, `CN=finos`)
- [x] Slice 3: Accessibility names
- [x] Slice 4: Performance budget
- [x] Signed updater: minisign pubkey in `tauri.conf.json`; check-for-update reports status only (no post)
- Gate: [docs/architecture/m7-gate.md](docs/architecture/m7-gate.md)

## Profile A depth (still SQLite)

- [x] Slice 1: `PositionDetailsGet` rollup over open lots
- [x] Slice 2: `TaxProjectionGet` view over MAGI
- [x] Slice 3: Import sample Fidelity dividend through FinanceClient
- Gate: [docs/architecture/profile-a-depth.md](docs/architecture/profile-a-depth.md)

## Milestone 8 — Centralization proof (**architecture proof**; desktop stays SQLite)

M8 boxes mean **ports / server / Postgres adapter proofs**. They do **not** mean Profile A product cutover or that Components / Cash Management / Shopping Cart live only on Postgres.

- [x] `crates/storage-postgres` implements the same ports for Account / Dividend / Lot / PositionDetails / MAGI
- [x] Dual-adapter contract: same cents; `DATABASE_URL` required (no SQLite fallback)
- [x] MAGI pack on Postgres matches owner-approved oracles
- [x] Axum `finos-server` + `RemoteHttpFinanceClient` (desktop `App.tsx` stays `LocalTauriFinanceClient`)
- [x] Optimistic concurrency: `expectedVersion` / `concurrency_conflict`
- [x] `SnapshotImport` SQLite bundle → PostgreSQL reconcile (AC-ARCH-08)
- Gate: [docs/architecture/m8-gate.md](docs/architecture/m8-gate.md)

## Milestone 9 — Multi-client production (**architecture proof**; desktop stays SQLite)

M9 boxes are **HTTP / JWT / web-client proofs**. Owner Profile A remains local SQLite until the owner unlocks authority cutover.

- [x] Slice B1: Axum Bearer JWT (test issuer); `command_audit` user/device/correlation/result; unauthenticated fail closed
- [x] Slice B2: `apps/web` on `RemoteHttpFinanceClient`; Health / Dividend / MAGI; no UI SQL
- [x] Slice B3: Postgres logical dump/restore; MAGI/contract cents still match
- [ ] Authority cutover (owner-gated): do not switch `App.tsx` until named
- Gate: [docs/architecture/m9-gate.md](docs/architecture/m9-gate.md)

Active work board: [docs/architecture/execution.md](docs/architecture/execution.md) (Now / Next). Do not wait for a re-attached plan. Anti-erasure: `scripts/components-anti-erasure.sh` and CI job `components-anti-erasure` (`ui_modules` + `core_functions`).

## Development

**Rust** (from repo root):

```
cargo check
cargo test
```

**Desktop app** (see `apps/desktop/README.md`):

Coding days: Desktop `finos.bat` or `apps/desktop/start-finos-dev.bat` (`npm run dev`). The console is expected.

Household no-console sessions: Desktop `finos-installed.bat` after a one-off `npm run desktop:build` + NSIS install. Do not use that as the daily rebuild start.

```
npm install
npm run dev
```

`npm run dev` is the same launch as `npm run desktop` (`tauri dev`) with a
preflight in front and an exit-code explanation after. It works from the repo
root and from `apps/desktop`.

| Command | What it does |
|---------|--------------|
| `npm run dev` | Preflight, then the full app (Vite UI + Tauri host + SQLite) |
| `npm run doctor` | Preflight only — names a missing prerequisite, a busy port 1420, or a running `finos-desktop.exe` |
| `npm run desktop` | `tauri dev` directly (preflight still runs via `predesktop`) |
| `npm run dev:vite` | Frontend dev server only, no window — for isolating a UI-side failure |
| `npm run desktop:check` | `tsc` + `vite build`, the same type check `desktop:build` runs first |
| `npm run desktop:build` | Signed NSIS installer (long compile; needs `TAURI_SIGNING_PRIVATE_KEY`) |

npm reports only the last child's exit code. On Windows a terminated child
shows as `4294967295` (`-1`), which is what a normal shutdown, `Ctrl+C`, or
File → Restart looks like — not a build failure. The real error is earlier in
the console; `apps/desktop/capture-finos-dev-log.bat` saves the whole session
to a file.

Exit: File → Exit, the Exit button, the window X, or `Ctrl+C` in the dev console.

The window should show HealthGet, HandoffStatusGet, CanonicalWeekGet (Sat–Fri), account/exception lists, dividend actual totals, lots/ROI, MAGI decision, and Calculator Plan / burndown.

## Governance

Changes to framework, domain language, databases, component boundaries, precision policy, device handoff or client contracts require an approved ADR and a new version of the target architecture document.
