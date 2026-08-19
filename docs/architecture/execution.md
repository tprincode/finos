# Execution board

Single place the agent reads so work proceeds step by step. Locked roadmap is V1.1 §12. Gate details stay in `m1-macos-gate.md` … `m9-gate.md`.

**Now:** parked until named (authority cutover; M1 macOS)  
**Next:** none until the owner names a slice  
**Parked:** SQLite→Postgres authority cutover (owner-gated after M9); M1 macOS restore/handoff (needs a Mac)

## Loop

1. Do the current numbered step.
2. Run its **Pass** command.
3. If pass → start the next number immediately.
4. If fail → stop and fix. Never regenerate MAGI oracles.
5. When the slice’s last step passes → update Now/Next above and start the new Now.

Stop only for golden-oracle owner approval, a Mac-only step on Windows, or Postgres that cannot start (fix Postgres; do not substitute SQLite).

## Done (Windows)

| Gate | Evidence |
|------|----------|
| M0–M7 | Signed NSIS trusted and installed on this PC |
| Profile A depth 1–3 | PositionDetailsGet, TaxProjectionGet, sample Fidelity import |

## Done this board

| Slice | Evidence |
|-------|----------|
| Phase 1 Allocation vs positions | `AllocationGet` composes `PositionDetailsGet`; 60.00% target; open performance 140000; DividendGet unchanged |
| Phase 2 Signed updater | GitHub `latest.json` endpoint; minisign pubkey in config; private key gitignored; check-for-update does not post |
| Phase 3 Postgres adapter | `storage-postgres` vs Docker `postgres:16`; dual-adapter cents; desktop stays SQLite |
| MAGI pack on Postgres | G-MAGI-01–10 match the same owner-approved oracles on PostgreSQL; SQLite magi-gate still passes; no oracle writes |
| M8 slice 3 Axum + remote client | `POST /v1/commands` and `/v1/queries`; contract cents; G-MAGI-01 over HTTP; App.tsx stays LocalTauri |
| M8 slice 4 Concurrency | Two `AccountUpdate`s with the same `expectedVersion`; one `concurrency_conflict` |
| M8 slice 5 Snapshot import | `SnapshotImport` reconciles counts, money, lots, MAGI; desktop stays SQLite writer |
| M9 B1 OIDC + command audit | Bearer JWT test issuer; unauthenticated 401; two clients; `concurrency_conflict`; `command_audit` |
| M9 B2 apps/web | `RemoteHttpFinanceClient`; Health/Dividend/MAGI; no UI SQL |
| M9 B3 Postgres backup | Logical dump/restore; MAGI/contract cents still match |

Desktop stays SQLite. Authority cutover stays parked until the owner names it. M1 macOS stays parked until a Mac is available.

## How you use this

Keep working in this repo. The agent reads this file and continues **Now**.
