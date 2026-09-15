# Execution board

Profile A product only. Local data SQLite is the system of record. Seed is a separate CLI, not an owner screen.

**Now:** Remaining InScope 19a-1 after CM Slice 1 (YBTC thin slice parked behind live Windows Profile A when needed): search → vendor host → hub/notice; raise `roc_pct_change` only on % change; do not overwrite plan %. 1099 actual stays parked until April 2027.
**Honesty:** Seed/`work_ticket` open count for TSPY amount, HAKY, and CLM/CRF/GLAD `paid_payable_supersede` is **0** (owner confirms none open on household). **Do not** demand Tickets → Except/Reject of nonexistent rows. Historical meaning of those items is documented as backlog IDs TKT-TSPY-01, TKT-HAKY-01, TKT-CLM-01, TKT-CRF-01, TKT-GLAD-01, TKT-STILLMISS-01, TKT-YBTC-01 (Project store `docs/phase2-ticket-backlog.md`) — planning tickets only; never invent live `work_ticket` rows to satisfy the board.
**Next:** Optional household confirm only: Collectors **still-miss 0** on the 40 when HAKY is present (seed-only hosts soft-skip; not an Except/Reject gate). Further CM recon slices only if owner unlocks.
**Ops (parallel):** none.
**Blocked (this host):** Live Roundhill/YBTC fetch and household still-miss proof need Windows Profile A (`%LOCALAPPDATA%\com.finos.desktop`). This VM is Linux seed-only (**no HAKY**; `work_ticket` empty after `data-seed`). Fixture Pass stays green; do not fake live green.
**Done this turn:** **CM Slice 1 week cash recon (Go):** `WeekCaptureAccept` (save body + `adjusts[]`, one tx) + internal `CashAdjustPost` / `Cash_Adjust`; Review panel `aria-label="Week cash recon"` (STEPS stay 8, Speculation index 3); reference = sum of open `cash_symbol` lots else prior snapshot `cash_minor` else null; T1–T7 green. MAGI frozen; `CashManagement.tsx` not rewritten. Prior tip Done: Board honesty YBTC Now; Phase 2 KEEP GOING; Phase 1 freeze + Phase 0 recover retained.
**Parked:** M1 macOS; SQLite→Postgres cutover; live OIDC; public CA; Python connectors posting; MAGI oracle rewrite; ACA tax-family size / $84,600 path ([immutable-tax-family-magi-freeze.md](immutable-tax-family-magi-freeze.md) — owner-locked 2026-09-15); M8/M9 theater; watchlist; full position-liquidity classification replacing Acct9 interim heuristic; Phase 3 hygiene (owner decisions)

`App.tsx` stays `LocalTauriFinanceClient`. No UI SQL. Never regenerate MAGI oracles.
Owner facts use in-row display/edit in the same field. Save or Cancel; leaving with unsaved edits asks first.

**Collector completion gate:** the income fleet is the 40 `INCOME_FLEET_SYMBOLS`. Issuer adapter work is not Done until `npm run desktop` → Run enabled collectors (or Run misses only until empty) shows **0 miss** and Income Plan has amounts for every enabled payer. Fixture tests alone do not satisfy this. Live lock on 13 Sep 2026: still-miss 0 on those 40. MSTU, TSLL, and SOXL are holdings only — not collectors.

## Two jobs

1. **Seed (once):** `npm run data-seed` writes `database/seed/production` into `%LOCALAPPDATA%\com.finos.desktop` (or `%LOCALAPPDATA%\finos` if that path is a sync folder). Gate: 8 / 75 / 1679 / 1250 / 5862 / 129 and open cost $466,946.66 / tax $461,356.29. Complete data is a no-op. Desktop never searches for xlsx.
2. **Daily use:** desktop opens that SQLite file and queries it. Last prices auto-refresh on open for open lots only on weekdays 9–4 Eastern (and skip if the last successful run is under 4 hours); a stored last price still displays when stale. Manual Refresh still force-runs. Missing last price stays unknown, never $0. Income Plan weekly report uses Decl $ as the week's money (Plan, Decl, Decl−Plan); broker Actual $ is not on that screen. No load prompts. On open, ProviderDeclarationSourcesApply fills empty templates then DeclarationRefresh runs enabled collectors. **Coding launch** this month: start `apps/desktop/start-finos-supervisor.bat` (keeps a `finos supervisor` window). File → Restart writes `restart.token` and exits; the supervisor Start-Process-es `start-finos-dev.bat`. Do not start the host with File → Restart from a stack that has no supervisor. Console is expected. **Household launch** (no console): Desktop `finos-installed.bat` → `%LOCALAPPDATA%\finos\finos-desktop.exe` after an owner-requested `npm run desktop:build`. Do not run both copies. Do not `desktop:build` on every host change.

## Miss loop (Done 13 Sep 2026)

Income fleet is locked at 40 last-run-OK names. Desktop Run enabled still-miss is 0. Residual ROC-template / Except-Reject work is a later slice.

## Contract backlog (Phase 1 remaining gaps)

Explicit IDs from review-pack contract drift after menu-query close-out. Do not invent new screens under these IDs without owner Now.

| ID | Gap | Notes |
|----|-----|-------|
| **BL-CONTRACT-01** | Components catalog model | UI extraction inventory vs bounded-context index (review pack §4–5). Owner chooses one in Phase 3. |
| **BL-CONTRACT-02** | Income Plan naming | Contract still lists `IncomePlanGet` / `IncomePlanUpdate`; desktop nav uses week/grid/export. Keep both until rename ADR. |
| **BL-CONTRACT-03** | Dashboard alias | `DashboardGet` vs live `DashboardBurndownGet` — both registered; alias cleanup later. |
| **BL-CONTRACT-04** | Isolation enforcement | One SQLite file; isolation is a review rule (stated on contracts). ADR-0005 addendum parked for Phase 3. |

## Pass

```
cargo test -p financial-domain
cargo test -p golden-harness --test new_investment --test add_lot --test position_details --test roc_plan
cargo test -p golden-harness --test accessibility --test invariants --test calculator_plan --test lots_roi --test dividend_slice --test production_seed --test collectors --test data_snapshot --test desktop_menu --test core_functions --test ui_modules --test income_plan_week --test cash_management
```

Desktop stays SQLite.
