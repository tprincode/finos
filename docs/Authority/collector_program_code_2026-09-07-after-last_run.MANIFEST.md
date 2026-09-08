# Collector program code zip — 2026-09-07 after last_run repair

External-audit pack. **Does not include** live SQLite, `.env`, credentials, or household PDFs.

## Lock files used (do not invent spec)

- `.cursor/req_extract/Collector_Requirements_Initial_and_Runtime_Locked_2026-09-04.txt`
- `.cursor/req_extract/Adapter_Collector_Owner_Decisions_Locked_2026-09-02.txt`
- `.cursor/req_extract/Collector_Implementation_Plan_Audit_2026-09-04.txt` (F1 / F2 only)

## Gap reports

- `docs/Authority/Collector_Software_and_Data_Gap_2026-09-07.md` (pre-repair snapshot)
- `docs/Authority/Collector_Software_and_Data_Gap_2026-09-07-after-last_run.md`
- `docs/Authority/collector_missing_ask_2026-09-07-after-last_run.json`
- `.cursor/collector_gap_audit_2026-09-07.json`
- `.cursor/collector_gap_after_last_run_repair.json`

## Domain / retrieve / application (same tree as 2026-09-07 pack)

- `crates/financial-domain/src/collector.rs`
- `crates/financial-domain/src/div1.rs`
- `crates/financial-domain/src/declaration_post.rs`
- `crates/financial-domain/src/declaration_lookback.rs`
- `crates/financial-domain/src/work_ticket.rs`
- `crates/financial-domain/src/schedule.rs`
- `crates/financial-domain/src/current_price.rs`
- `crates/financial-domain/src/lib.rs`
- `crates/import-engine/src/retrieve/` (entire tree)
- `crates/import-engine/src/lib.rs`
- `crates/import-engine/Cargo.toml`
- `crates/application-core/src/queries.rs`
- `crates/application-core/src/contracts.rs`
- `crates/application-core/src/lib.rs`
- `crates/storage-sqlite/src/work_ticket.rs`
- `crates/storage-sqlite/src/issuer_pay.rs`
- `crates/storage-sqlite/src/wizard.rs`
- `crates/storage-sqlite/src/field_decision.rs`
- `crates/storage-sqlite/migrations/0029_work_ticket.sql`
- `crates/storage-sqlite/migrations/0030_phase2_establish.sql`
- `crates/golden-harness/tests/collectors.rs`
- `crates/golden-harness/src/bin/collector_miss_loop.rs`
