# Architecture documentation

Governing target architecture for the Finance Management System.

| Document | Description |
|----------|-------------|
| [System_Architecture_and_Implementation_Roadmap_v1.1_LOCKED_2026-08-15.docx](../../System_Architecture_and_Implementation_Roadmap_v1.1_LOCKED_2026-08-15.docx) | **Locked authority** — V1.1 target architecture (2026-08-14) |
| [_docx_extract.txt](../../_docx_extract.txt) | Plain-text extract for search and tooling |
| [mvp-boundary.md](mvp-boundary.md) | V1 desktop-local in / out / forbidden (Milestone 0) |
| [component-contracts.md](component-contracts.md) | Bounded component commands, queries, events and table isolation |
| [m1-macos-gate.md](m1-macos-gate.md) | Same restore/handoff cases to run on macOS |
| [m2-gate.md](m2-gate.md) | Canonical foundation seed-reconcile and anti-drift tests |
| [m3-gate.md](m3-gate.md) | Dividend vertical slice: one actual updates Income Plan / Dashboard / Trends |
| [m4-gate.md](m4-gate.md) | Lots and ROI: dual basis, CRF policy, explicit assignment, broker recon |
| [m5-gate.md](m5-gate.md) | Marketplace MAGI pack: owner-approved oracles pass on the production path |
| [m6-gate.md](m6-gate.md) | Decision intelligence: allocation, cart, backtest, classification, advisory AI |
| [m7-gate.md](m7-gate.md) | Release hardening: recovery drill, signed NSIS, accessibility |
| [m8-gate.md](m8-gate.md) | PostgreSQL adapter, MAGI pack, Axum, concurrency, snapshot import (desktop stays SQLite; no cutover) |
| [m9-gate.md](m9-gate.md) | OIDC Bearer JWT, apps/web, Postgres dump/restore (desktop stays SQLite; cutover owner-gated) |
| [profile-a-depth.md](profile-a-depth.md) | Profile A: PositionDetailsGet, TaxProjectionGet (still SQLite) |
| [checkpoint-2026-08-19.md](checkpoint-2026-08-19.md) | Profile A architecture checkpoint: good / bad / next (morning 2026-08-19) |
| [checkpoint-2026-08-19-rescore.md](checkpoint-2026-08-19-rescore.md) | Same-day rescore after updater + Docker Postgres adapter |
| [execution.md](execution.md) | Step-by-step board: Now / Next / pass checks (agent must not wait) |

## Locked outcomes (summary)

- **ARCH-01–09**: Local write-first SQLite, cross-platform Tauri desktop, modular monolith, deployment-independent domain, safe device handoff, path to PostgreSQL multi-client, explainable calculations, progressive automation, no autonomous financial action.
- **Golden Harness (ADR-0013)**: Required release-control component; first pack is 2026 Marketplace MAGI (G-MAGI-01–10).
- **Milestone 0**: Architecture control — ADRs, contracts, MVP boundary, golden-harness contract.
- **Milestone 2 gate**: [m2-gate.md](m2-gate.md) — seed reconcile; MAGI pack must not pass while pending-owner.
- **Milestone 3 gate**: [m3-gate.md](m3-gate.md) — one `DividendActual` updates Income Plan / Dashboard / Trends; re-import is idempotent.
- **Milestone 4 gate**: [m4-gate.md](m4-gate.md) — no FIFO; CRF tests and broker lot reconciliation.
- **Milestone 5 gate**: [m5-gate.md](m5-gate.md) — owner-approved G-MAGI-01–10 pass through `MagiProjectionGet`.
- **Execution board**: [execution.md](execution.md) — current Now item and the continue-on-pass loop.
- **Checkpoint 2026-08-19**: [checkpoint-2026-08-19.md](checkpoint-2026-08-19.md) — Profile A good / bad / next; do not treat M8 as the default next step.

See [../adr/](../adr/) for individual architecture decision records.
