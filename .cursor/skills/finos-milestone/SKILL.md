---
name: finos-milestone
description: Runs the finos V1.1 milestone loop from docs/architecture/execution.md — current Now item, pass checks, then Next without waiting. Use when the user says proceed, next step, milestone, gate, validation, MAGI pack, Plan/burndown, or asks to keep going.
---

# finos milestone loop

1. Open `docs/architecture/execution.md`. Treat **Now** as the only active work.
2. Execute every numbered step under Now, in order. After each pass, start the following step in the same turn.
3. On failure: stop, report the failing command, do not overwrite `tests/golden/oracle/`.
4. When Now is fully green: move Now to Done, promote Next to Now, write the new first-slice steps, and start step 1.
5. Do not ask for MAGI re-approval. Do not start AI posting, Postgres, OIDC, or signed installers unless Now says so.
6. M1 macOS stays parked on Windows; skip it and take the next unlocked Windows item.

Layering: UI → FinanceClient → application-core → ports → storage-sqlite. No UI SQL. No `apps/desktop/src-tauri` in the root Cargo workspace.
