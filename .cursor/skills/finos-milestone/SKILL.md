---
name: finos-milestone
description: Runs the finos V1.1 milestone loop from docs/architecture/execution.md — current Now item, pass checks, then Next without waiting. Use when the user says proceed, next step, milestone, gate, validation, MAGI pack, Plan/burndown, or asks to keep going.
---

# finos milestone loop

## Product tip (mandatory)

**Authoritative product tip:** recovery baseline `origin/cursor/core-functions-decl-color` @ **`fc84bfe`**, or its successor merge on the default line that still contains Tools → Components.

Before any Profile A product Now work:

1. Confirm checkout is the recovered tip (or a branch that already contains that merge) — **not** pre-recovery `master` alone.
2. Confirm Tools → Components is present: `.text("components", "Components")` in `apps/desktop/src-tauri/src/lib.rs` and `navButton("components", "Components")` in `App.tsx`.
3. Pass must include `cargo test -p golden-harness --test ui_modules --test core_functions` (see `scripts/components-anti-erasure.sh` / CI job `components-anti-erasure`).

**Stop immediately** if the tip is wrong or Components / `CoreFunctionsGet` / `ui-modules.json` / `core-functions.json` / Cash Management catalogs are missing. Do not recreate those surfaces from a contaminated default. See `.cursor/rules/phase0-recovery-baseline.mdc` and `.cursor/rules/execution-framework.mdc`.

## Owner facts — global (all collectors)

**Before asking the owner about ROC, issuer, frequency, scope, or amount facts for any symbol:**

1. Read `docs/Authority/owner-facts/<SYMBOL>.md` if it exists (policy: `docs/Authority/owner-facts/README.md`; fleet index: `INDEX.md`).
2. Read related `docs/Authority/` packs when that file cites them.
3. If the file covers the question (locked or book-cited, not `PENDING_OWNER`) → **do not re-interview**.
4. Ask only `PENDING_OWNER` / OPEN fields (once), or when a live parsed 19a-1 % differs from stored plan (raise `roc_pct_change`; never invent plan % into SQLite from chat).
5. Empty `roc_source_url` is not “unknown ROC %.”
6. **Recreate / first-enable:** if owner-facts file is missing or required fields incomplete → create/complete it from [`TEMPLATE.md`](../../../docs/Authority/owner-facts/TEMPLATE.md) (seed cites + `PENDING_OWNER` for unknowns). Do not fake-fill. Fail the recreate slice if the durable file was skipped.
7. Filled example: `docs/Authority/owner-facts/YBTC.md`. Other fleet symbols may be SCAFFOLD or PENDING_RECREATE — that is honest.

Gate: `./scripts/owner-facts-anti-reask.sh`

## Loop

1. Open `docs/architecture/execution.md`. Treat **Now** as the only active work.
2. Execute every numbered step under Now, in order. After each pass, start the following step in the same turn.
3. On failure: stop, report the failing command, do not overwrite `tests/golden/oracle/`.
4. When Now is fully green: move Now to Done, promote Next to Now, write the new first-slice steps, and start step 1.
5. Do not ask for MAGI re-approval. Do not start AI posting, Postgres, OIDC, or signed installers unless Now says so.
6. M1 macOS stays parked on Windows; skip it and take the next unlocked Windows item.

Layering: UI → FinanceClient → application-core → ports → storage-sqlite. No UI SQL. No `apps/desktop/src-tauri` in the root Cargo workspace.
