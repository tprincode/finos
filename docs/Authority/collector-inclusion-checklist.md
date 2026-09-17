# Collector inclusion checklist

Owner memory aid. **Not a new lock.** Authority remains:

- `.cursor/req_extract/Collector_Requirements_Initial_and_Runtime_Locked_2026-09-04.txt`
- `.cursor/req_extract/Adapter_Collector_Owner_Decisions_Locked_2026-09-02.txt`

If a row is not in `REQUIRED_FIELDS` (Establish) or on the named Runtime command path, the product will not enforce it. To add a requirement: add a row here **and** the code path.

---

## Establish (locked)

Gate: `collector_is_complete` in `crates/financial-domain/src/collector.rs`. First lot: `LotOpen` → `collector_incomplete`. After that first `LotOpen` succeeds, and after Research on a name that already has lots (Recreate adapter), `CollectorRecertify` re-runs the same gate. Existing lots do not skip it. Extra `LotOpen` still grandfathers.

| # | Must exist | Skip? | Code |
|---|---|---|---|
| E1 | Template Dividend (owner seed declaration URL) | No | `has_dividend_template` / `source_url` |
| E2 | Template ROC (reusable 19a-1/tax URL; search `19.1 tax ROC (TICKER) (Provider) website data source`) | No if in scope; CASH / not-in-scope skip | `has_roc_template` / `roc_source_url` |
| E3 | DIV-1 (`div_type`; CASH is the other path) | No | `REQUIRED_FIELDS` |
| E4 | Frequency | No | `REQUIRED_FIELDS` |
| E5 | Provider name | No | `REQUIRED_FIELDS` |
| E6 | Underlying | No | `REQUIRED_FIELDS` |
| E7 | Risk: Foundation / Core / Risk On (owner sets; never auto) | No | `REQUIRED_FIELDS` `risk_tier` |
| E8 | Paid history: pay dates matched to amounts; inception Yes/No if &lt;12 | No | `paid_history` |
| E9 | Remaining-year pay dates (vendor, else derive-once; cap 4/12/52) | No | `remaining_year` |
| E10 | Current-year ROC estimate accepted (0 only if notice said 0) | No if in scope | `roc_estimate` + owner accept (`needs_roc_research=false` via Collectors Accept ROC / `RocPlanConfirm`, or ticket Accept on `roc_pct_change`) |
| E11 | Backtest dates | Optional; Skip does not block | not in `REQUIRED_FIELDS` |
| E12 | Last price once (never $0) | — | not in `REQUIRED_FIELDS` today |

---

## Runtime (declaration vs ROC stay apart)

| # | Rule | Fail → ticket? | Stored past pays? |
|---|---|---|---|
| R1 | Enabled + open lots. Add lot does not re-run history/ROC/dates. | — | Untouched |
| R2 | Collect uses **Template Dividend** only. New/changed dates + amounts. Do not clone stored months. GET timeout (`declaration_retrieve_timeout`) skips that name and continues the fleet; post-run and ticket Retry use the stored URL at 20s / 45s / 90s (three tries). Recreate is not the timeout path. | Miss / parse miss → `declaration_retrieve_miss`. Timeout → `declaration_retrieve_timeout` (Retry tool). `last_run_ok` = this run. Owner-filed or auto-resolved does **not** suppress a later failed retrieve (Home 39/40 with no ticket is a break). Miss payload keeps GET evidence (`htmlLen`, payable, td count) — not a screenshot, not a wipe of stored pays. | Keep. Owner/import/manual paid rows are not overwritten. Issuer same-period change may supersede after complete; 30% → amount ticket, not a wipe. |
| R3 | Future (unoccurred) dates move when vendor payable date changes. Plan $ unchanged. | Wrong count vs 4/12/52 → ticket | Occurred pays stay |
| R4 | Declaration run does not rewrite identity, frequency, Plan $, lots, or ROC. | — | — |
| R5 | Unchanged today + same hash → no fetch, **except** on the locked declaration weekday (must fetch again). | — | — |
| R6 | ROC uses **Template ROC** only (`PositionResearchRefresh` / Reevaluate). Not Collect / Force refresh. Unknown ≠ 0. Different % → `roc_pct_change`, do not overwrite. ROC miss does **not** flip `last_run_ok`. | ROC parse miss / % change tickets | Pays untouched |
| R7 | Last price is a separate job. Never $0. | — | — |
| R8 | Broker import = actual $ only. Never declarations or Plan $. | — | — |
| R9 | After the fleet run, fail count (not current today) **equals** open retrieve-failure tickets on those names (`declaration_fail_ticket_parity`). Extra ROC / amount tickets do not count. | Parity false is a product break. | — |

Owner Accept/Reject is not Establish or Runtime success.

---

## How often

- **Establish gate:** Collectors / Tools / LotOpen (`CollectorSetGet`).
- **Establish recertify:** `CollectorRecertify` after first `LotOpen` (`first_create`) and after `PositionResearchSeed` / `PositionResearchRefresh` when lots already exist (`recreate`). Same E gaps. Ticket `collector_establish_incomplete` if it fails. Does not wipe pays. Does not flip `last_run_ok`. Does not replace the first-lot block or extra-lot grandfather. Recreate loads stored Template Dividend (E1) and Template ROC (E2). It validates with `CollectorRecertify` and may retrieve with the stored URL. It does not re-run `PositionResearchSeed` and does not re-import pays, identity, ROC, or Plan. Owner pastes only when E1 is empty. Confirm Plan is not required when Plan is unchanged.
- **Runtime collect:** desktop open `DeclarationRefresh`; Run misses; Run enabled; Force refresh (one symbol, declarations only). GET timeout skips that name and continues; post-run and ticket Retry use the stored Template Dividend at 20s / 45s / 90s. Recreate is not the timeout path.
- **`cargo test`:** predicate and fixtures. Does not re-certify the live book. `recertify_runs_after_first_create_and_after_recreate` locks the product command path.

---

## Not collectors

**MSTU, TSLL, and SOXL** are holdings only. They are not on Income Plan. They are not collectors. Do not list them on Collectors, Run enabled, recertify, miss counts, or fleet reports. Seed cannot enable them. Lots and stored history stay. Last price is a separate job.

---

## Agent rule

When changing collectors, open this file and walk every E/R row. Do not summarize the list down. Do not call a name complete from last_run alone. Do not mention MSTU, TSLL, or SOXL in collector results.
