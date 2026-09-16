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

## Owner ROC / collector facts (mandatory before ask)

**Before asking the owner about ROC or collector establish facts** for a symbol:

1. Read `docs/Authority/owner-facts/<SYMBOL>.md` if it exists (see `docs/Authority/owner-facts/README.md`).
2. If the file is present and marks identity / scope / estimate / 1099 parking locked → **do not re-interview** those fields. Use the file + seed / `position_characteristic`.
3. Empty `roc_source_url` is not “unknown ROC %.” Optional URL persist must not reopen the % interview.
4. Ask only for fields marked **OPEN**, or when a live parsed 19a-1 % differs from stored plan (raise `roc_pct_change`; never invent plan % into SQLite from chat alone).
5. YBTC is locked in `docs/Authority/owner-facts/YBTC.md` (100% in-year estimate; not YBIT; actual empty until 2027 1099).

## Loop

1. Open `docs/architecture/execution.md`. Treat **Now** as the only active work.
2. Execute every numbered step under Now, in order. After each pass, start the following step in the same turn.
3. On failure: stop, report the failing command, do not overwrite `tests/golden/oracle/`.
4. When Now is fully green: move Now to Done, promote Next to Now, write the new first-slice steps, and start step 1.
5. Do not ask for MAGI re-approval. Do not start AI posting, Postgres, OIDC, or signed installers unless Now says so.
6. M1 macOS stays parked on Windows; skip it and take the next unlocked Windows item.

Layering: UI → FinanceClient → application-core → ports → storage-sqlite. No UI SQL. No `apps/desktop/src-tauri` in the root Cargo workspace.
