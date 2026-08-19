# Architecture checkpoint — Profile A toward implementation

Date of record: **2026-08-19**. Authority: locked V1.1 §12 and ADRs 0001–0013.

Interactive board: open the Cursor canvas `architecture-checkpoint.canvas.tsx` beside the chat (Overview / good / bad / next / AC-ARCH).

This is a **review**, not a milestone slice. It does not change [execution.md](execution.md) Now/Next. Do not start Postgres, OIDC, or the updater unless those items are named.

## Verdict

The **architecture is holding**. Windows work through M7 is **gate-green**, not **V1-complete**.

V1.1 says Profile A is the product: one-owner Tauri desktop, local SQLite, then expand to C/D without changing the domain. Implementation followed that order. What is missing is mostly **product depth and the other platform**, not a new architecture.

Do **not** treat M8 Postgres as the default next implementation step. Centralization is a required *capability* (ARCH-03, AC-ARCH-08). It is not how you finish a household-usable Profile A.

## What's good

- **Governance is real.** Locked V1.1, 13 ADRs, [mvp-boundary.md](mvp-boundary.md), [component-contracts.md](component-contracts.md). MAGI oracles are owner-approved and never auto-regenerated (ADR-0013).
- **Layering held.** UI → `LocalTauriFinanceClient` → `application-core` → ports → `storage-sqlite`. No UI SQL. Desktop Tauri crate is not in the root Cargo workspace. `financial-domain` tests run without Tauri/SQLite (AC-ARCH-02).
- **Profile A core on Windows is proven.** WAL SQLite, device identity, snapshot lineage, handoff block, restore (including leftover-WAL after `SnapshotRestore`). Schema version **11**.
- **Hard money rules are in code.** Fixed-scale cents, no FIFO, dual basis, CRF-only zero-cost DRIP, Plan remaining ≠ actual cash, MAGI INDETERMINATE vs SAFE.
- **M3–M5 are the strongest product slices.** Fidelity/Schwab capture → one `DividendActual` updates Income Plan / Dashboard / Trends; re-import is idempotent; MAGI pack + G10-ADJ-1 pass on the production path.
- **AI boundary is enforced.** `AiAnalyze` cannot post ledger/lots/MAGI; key from env only (ADR-0012).
- **Execution loop worked.** Gates are command-backed. The board correctly stopped at M8/M9 and Mac.

## What's bad

- **Gates ≠ product.** M6 allocation / cart / backtest / classification / AI mostly prove they **do not post facts**. Dashboard/Trends are dividend-total projections, not full reporting.
- **M7 is a produced NSIS, not a public CA.** `npm run desktop:build` (2026-08-19) wrote `apps/desktop/src-tauri/target/release/bundle/nsis/finos_0.1.0_x64-setup.exe`, signed `CN=finos` / `CEAAACA78BB136405A1496C2DD39EB0E8975D6E6`. This PC: CurrentUser Root + TrustedPublisher; `Get-AuthenticodeSignature` is `Valid`; installed `%LOCALAPPDATA%\finos`. Updater is parked. Accessibility is primary-action `aria-label`s. Performance is one MAGI inventory loop. AC-ARCH-11 still wants a signed **macOS** release.
- **M1 is still open.** ARCH-02 and AC-ARCH-17 require the same restore/handoff on macOS. A Windows PFX does not sign a Mac app.
- **Contracted surfaces missing** from `execute_query_on`: `ConnectorJobSubmit` / `ConnectorJobGet`, `RemoteHttpFinanceClient`. `PositionDetailsGet` and `TaxProjectionGet` landed 2026-08-19. Contract version is still `1.0.0-draft`. UI is one developer `App.tsx`.
- **Postgres is a placeholder.** `crates/storage-postgres/Cargo.toml` is a comment, not an adapter.

## What's next (pick one)

Recommended **implementation** order — not “unlock Postgres” by default:

1. ~~Close M7 in the real world~~ — `npm run desktop:build`, install, trust `CN=finos` on this PC (2026-08-19).
2. **M1 on a Mac** when a Mac is available — [m1-macos-gate.md](m1-macos-gate.md). Separate Apple signing identity.
3. **Profile A depth** (still SQLite) — `PositionDetailsGet`, `TaxProjectionGet`, import UX; promote one M6 stub to a real workflow.
4. **Unlock updater** only if you want a private update channel.
5. **Unlock Postgres (M8)** only when you want the centralization *proof* (same golden pack on a Postgres adapter).

Stay parked until named: OIDC / web (M9), Python connectors, AI posting of facts.
