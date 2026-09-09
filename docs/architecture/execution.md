# Execution board

Profile A product only. Local data SQLite is the system of record. Seed is a separate CLI, not an owner screen.

**Now:** Miss loop until 0 miss on enabled payers. MSTU is long-hold — do not collect.
**Next:** Owner desktop rebuild so Tickets can take a declared $ if a quarterly ask opens (first window 7 Nov 2026). Fully quit the app, then `npm run desktop`, then File → Save data snapshot and confirm `raw-data/<today>/` under the app data dir.
**Ops (parallel):** none.
**Done this turn:** Decl $/sh color (current green / stale amber / none gray) beats `.numeric` black. Core functions catalog lists that color rule. MAGI oracles unchanged.
**Parked:** M1 macOS; SQLite→Postgres cutover; live OIDC; public CA; Python connectors posting; Shopping Cart; MAGI oracle rewrite; M8/M9 theater; watchlist; live collector inventory fix until gate green (slice E); full position-liquidity classification replacing Acct9 interim heuristic

`App.tsx` stays `LocalTauriFinanceClient`. No UI SQL. Never regenerate MAGI oracles.
Owner facts use in-row display/edit in the same field. Save or Cancel; leaving with unsaved edits asks first.

**Collector completion gate:** issuer adapter work is not Done until `npm run desktop` → Run enabled collectors (or Run misses only until empty) shows **0 miss** and Income Plan has amounts for every enabled payer. Fixture tests alone do not satisfy this.

## Two jobs

1. **Seed (once):** `npm run data-seed` writes `database/seed/production` into `%LOCALAPPDATA%\com.finos.desktop` (or `%LOCALAPPDATA%\finos` if that path is a sync folder). Gate: 8 / 75 / 1679 / 1250 / 5862 / 129 and open cost $466,946.66 / tax $461,356.29. Complete data is a no-op. Desktop never searches for xlsx.
2. **Daily use:** desktop opens that SQLite file and queries it. Last prices refresh on open for open lots; a stored last price still displays when stale. Missing last price stays unknown, never $0. No load prompts. On open, ProviderDeclarationSourcesApply fills empty templates then DeclarationRefresh runs enabled collectors. **Coding launch** this month: Desktop `finos.bat` → `apps/desktop/start-finos-dev.bat` (`npm run desktop`). Console is expected. **Household launch** (no console): Desktop `finos-installed.bat` → `%LOCALAPPDATA%\finos\finos-desktop.exe` after an owner-requested `npm run desktop:build`. Do not run both copies. Do not `desktop:build` on every host change.

## Pass

```
cargo test -p financial-domain
cargo test -p golden-harness --test new_investment --test add_lot --test position_details --test roc_plan
cargo test -p golden-harness --test accessibility --test invariants --test calculator_plan --test lots_roi --test dividend_slice --test production_seed --test collectors --test data_snapshot --test desktop_menu --test core_functions --test income_plan_week
```

Desktop stays SQLite.
