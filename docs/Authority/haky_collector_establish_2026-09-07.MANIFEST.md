# HAKY collector establish zip — 2026-09-07

External-audit pack. **Does not include** live SQLite, `.env`, credentials, or household PDFs.

## Start here

- `docs/Authority/HAKY_Collector_Establish_Review_2026-09-07.md` — what ran, complete-gate, remaining hygiene, next-symbol pattern
- `docs/Authority/haky_live_facts_2026-09-07.json` — read-only snapshot of live HAKY fields (not the database file)

## Lock / board

- `.cursor/req_extract/Collector_Requirements_Initial_and_Runtime_Locked_2026-09-04.txt`
- `.cursor/req_extract/Adapter_Collector_Owner_Decisions_Locked_2026-09-02.txt`
- `docs/architecture/execution.md`

## Domain / post / retrieve

- `crates/financial-domain/src/collector.rs`
- `crates/financial-domain/src/declaration_post.rs`
- `crates/financial-domain/src/declaration_lookback.rs`
- `crates/import-engine/src/retrieve/mod.rs`
- `crates/import-engine/src/retrieve/adapters/amplify.rs`
- `crates/import-engine/src/lib.rs`

## Application / storage / desktop

- `crates/application-core/src/queries.rs`
- `crates/application-core/src/ports/canonical.rs`
- `crates/storage-sqlite/src/wizard.rs`
- `crates/storage-sqlite/src/canonical.rs`
- `apps/desktop/src/App.tsx`
- `apps/desktop/src-tauri/src/lib.rs`

## Tests / live helper

- `crates/golden-harness/tests/collectors.rs`
- `crates/golden-harness/tests/desktop_menu.rs`
- `crates/golden-harness/tests/accessibility.rs`
- `crates/golden-harness/src/bin/haky_live_fix.rs`
