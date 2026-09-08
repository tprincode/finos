# Collector program code zip — 2026-09-07

External-audit pack. **Does not include** live SQLite, `.env`, credentials, or household PDFs.

## Lock files used (do not invent spec)

- `.cursor/req_extract/Collector_Requirements_Initial_and_Runtime_Locked_2026-09-04.txt`
- `.cursor/req_extract/Adapter_Collector_Owner_Decisions_Locked_2026-09-02.txt`
- `.cursor/req_extract/Collector_Implementation_Plan_Audit_2026-09-04.txt` (F1 / F2 only)

## Gap report

- `docs/Authority/Collector_Software_and_Data_Gap_2026-09-07.md`
- `docs/Authority/collector_missing_ask_2026-09-07.json`
- `docs/Authority/collector_missing_ask_2026-09-07.prompt.md`

## Domain

- `crates/financial-domain/src/collector.rs`
- `crates/financial-domain/src/div1.rs`
- `crates/financial-domain/src/declaration_post.rs`
- `crates/financial-domain/src/declaration_lookback.rs`
- `crates/financial-domain/src/work_ticket.rs`
- `crates/financial-domain/src/schedule.rs`
- `crates/financial-domain/src/current_price.rs`
- `crates/financial-domain/src/lib.rs`

## Retrieve adapters

- `crates/import-engine/src/retrieve/` (entire tree)
- `crates/import-engine/src/lib.rs`
- `crates/import-engine/Cargo.toml`

## Application / storage

- `crates/application-core/src/queries.rs`
- `crates/application-core/src/contracts.rs`
- `crates/application-core/src/lib.rs`
- `crates/storage-sqlite/src/work_ticket.rs`
- `crates/storage-sqlite/src/issuer_pay.rs`
- `crates/storage-sqlite/src/wizard.rs`
- `crates/storage-sqlite/src/field_decision.rs`
- `crates/storage-sqlite/migrations/0015_investment_wizard.sql`
- `crates/storage-sqlite/migrations/0017_roc_research_observation.sql`
- `crates/storage-sqlite/migrations/0020_vendor_calendar.sql`
- `crates/storage-sqlite/migrations/0021_retrieval_page_hash.sql`
- `crates/storage-sqlite/migrations/0023_collector_enabled.sql`
- `crates/storage-sqlite/migrations/0024_retrieve_run.sql`
- `crates/storage-sqlite/migrations/0025_inception_on.sql`
- `crates/storage-sqlite/migrations/0029_work_ticket.sql`
- `crates/storage-sqlite/migrations/0030_phase2_establish.sql`

## Goldens / bins

- `crates/golden-harness/tests/collectors.rs`
- `crates/golden-harness/src/bin/collector_miss_loop.rs`
- `crates/golden-harness/src/bin/roc_19a1_fill.rs`
- `crates/golden-harness/src/bin/owner_lock_apply.rs`

## Live evidence (read-only snapshot, not the household DB file)

- `.cursor/collector_gap_audit_2026-09-07.json` — computed status for the 43 enabled names as of 2026-09-07. No secrets.
