# Architecture checkpoint — rescore (same day)

Date of record: **2026-08-19** (after Docker `postgres:16` adapter proof). Authority: locked V1.1 §12 and ADRs 0001–0013.

Interactive board: [architecture checkpoint](../../../.cursor/projects/c-Users-EVTom-repo-finos/canvases/architecture-checkpoint.canvas.tsx) (Overview / good / bad / next / AC-ARCH). Morning snapshot: [checkpoint-2026-08-19.md](checkpoint-2026-08-19.md).

This is a **review**, not a milestone slice. It does not change [execution.md](execution.md) Now/Next.

## Verdict

The **architecture is holding**. Windows Profile A is **gate-green and deeper**, not **V1-complete**. Desktop authority is still SQLite.

M8 slice 1 proved the same cents on Docker PostgreSQL 16. That is not core-on-server, MAGI-on-Postgres, or authority cutover.

## Moved since morning

| Item | Morning | Now |
|------|---------|-----|
| Profile A depth | Recommended | PositionDetailsGet, TaxProjectionGet, sample Fidelity import |
| One M6 workflow | Recommended | AllocationGet composes PositionDetailsGet (cost basis) |
| Updater | Parked | Pubkey + GitHub `latest.json`; check fails closed; no live Release |
| Postgres | Placeholder crate | `storage-postgres` vs `postgres:16`; dual-adapter 50000 / 140000 / 120000 |
| AC-ARCH-03 | Open | Partial (slice, not MAGI pack on Postgres) |

## Still open

M1 macOS. M8 remainder (Axum, remote client, concurrency, snapshot import). M9 OIDC/web. Cart/backtest/classification/AI remain no-post stubs. NSIS is self-signed `CN=finos`, not a public CA.

## What's next (pick one)

Parked until named: remaining M8 server path, M1 on a Mac, more data SQLite workflows, or OIDC.

## Later same day

MAGI pack on Postgres is now green (`postgres_magi_pack_matches_approved_oracles`): G-MAGI-01–10 match the same owner-approved oracles. Desktop authority is still SQLite. Axum, remote client, concurrency, snapshot import, OIDC, and cutover remain parked.
