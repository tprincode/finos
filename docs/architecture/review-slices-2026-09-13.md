# Review slices — 13 Sep 2026

From the Grok pack plus the Opus review in-chat. Collector **Now** (0 miss, MSTU long-hold) stays. These slices are named parallel work. Do not regenerate MAGI oracles. Do not move week entry back onto Trends.

**Done 13 Sep 2026:** A–E. Live seed kinds: Income/Speculation/9 = `ira`; FI Roth = `fi_roth`; Car/Robinhood/ENERGYX = `taxable`; Health = `hsa`; External = `taxable` (SSA by name). `CashDistributionPost` / `SsaConfirm` refuse a mismatched type. `ui_modules` locks owner-facing headings, calculated fields, live kinds, cart blend columns, and Print Export surfaces.

## Can we implement now?

| Area | Ready? | Why |
|---|---|---|
| Catalog + contracts docs | **Yes** | Facts are in `ui-modules.json`, `queries.rs` command list, and the pack. |
| Week-save atomicity | **Yes** | `save_trends_week` upserts the week row, then each snapshot, each on its own `sqlx` execute. No `BEGIN`. sqlx pool can wrap that in one transaction. |
| SSA two payees | **No — owner lock** | Code is Tom-only (`TOM_SSA_EXPECTED_MINOR` $2,865, key `ssa-tom-YYYY-MM`). Owner fact is Barbara + Tom. Barbara’s expected cents is not locked. Test fixture used $1,331 — that is not an owner lock. |
| Account-kind post guard | **Yes — live printed** | Owner SQLite 13 Sep 2026: Income/Speculation/9 `ira`; FI Roth `fi_roth`; Car/Robinhood/ENERGYX `taxable`; Health `hsa`; External `taxable` (SSA by name). |

## Owner locks before SSA / kind slices

1. Barbara SSA expected amount (cents), or “unknown until posted, never $0.”
2. Both SSA events post to External (current `is_ssa_account_name`).
3. After kinds are read from live seed, confirm: Withdrawal only on taxable / non-IRA (Car, Robinhood, …); IRA_Distribution only on `ira`; Roth_Distribution only on `roth`; SSA only on External.

## Slices (thin, in order)

### A — Catalog truth (docs / JSON only)

Pass: `cargo test -p golden-harness --test core_functions --test desktop_menu`

1. `ui-modules.json` Cash Management: list desk pieces (`CashWeekDesk`, capture on CM) and add `TrendsWeekSave` to `coreFunctionIds`.
2. Graphing `menuAreas`: add Cash Management (capture folder still graphing, screen is CM).
3. Add module rows: Calculator, Dashboard, Tickets.
4. Split or relabel the weak `lots` row only if a menu label exists — do not invent “Lots” as a user-facing name.
5. Components screen copy: this is a UI-extraction catalog, not a domain-completeness index.
6. Pack §2: `retrieval_page_hash` and lookthrough are columns, not tables.

### B — Contracts match the registry (docs + one test)

Pass: new or existing harness assert that every name in the `queries.rs` command/query list is mentioned in `component-contracts.md`, or the test lists explicit parked exceptions (AI / Backtest / Allocation UX).

1. Drop “implementations land in later milestones.”
2. Add a **Cash Management** contract area (own area, not under Planning or Ledger): `CashDistributionPost`, `SsaConfirm`, `CashManagementWeekGet` / `RemindersGet` / `MonthGet`. Note week **entry** is CM; `TrendsWeek*` stay Reporting writes used by CM.
3. Restate isolation: one SQLite file; one `Canonical` port; isolation is review, not schema.
4. Replace stale Income Plan / Dashboard names with live week/grid/export and `DashboardBurndownGet`.
5. Mark Shopping Cart scenario commands as shipped, not parked.

### C — Week save is one transaction

Pass: `cargo test -p golden-harness --test` the existing trends-week / cash-management capture tests; add one test that a mid-list snapshot failure rolls back the week row.

1. One storage function: week upsert + snapshot upserts inside `pool.begin()` … commit.
2. `save_trends_week` calls that. No generic transaction API. No UI change.

### D — SSA two payees (after locks 1–2)

Pass: `cargo test -p financial-domain` + `cargo test -p golden-harness --test cash_management`

1. Per-payee expected amount and idempotency key (`ssa-tom-YYYY-MM`, `ssa-barbara-YYYY-MM`).
2. Month status is per payee. `extra_audit` means a third unexpected SSA row, not “two Toms.”
3. Chooser can confirm either payee. Do not combine household amounts.
4. No payee column unless keys + amount are not enough. Prefer no migration.

### E — Post type vs account.kind (after lock 3)

Pass: cash_management + production_seed tests; live seed kinds printed and confirmed.

1. Read live `AccountList` kinds from the owner SQLite (or seed xlsx). **Done.**
2. `CashDistributionPost` / `SsaConfirm` refuse a type that does not match `kind`. **Done.**
3. Withdrawal = taxable / non-IRA only. Do not treat it as IRA. **Done.**

## Reject this pass

Plugin host. Second Core-functions menu. Audit on Tools. Regenerating MAGI. Postgres. Moving week entry to Trends. Inventing Barbara’s amount. Inventing account kinds from fixtures.
