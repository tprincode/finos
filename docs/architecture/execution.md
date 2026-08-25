# Execution board

Profile A product only. Household SQLite is the system of record. Seed is a separate CLI, not an owner screen.

**Now:** Owner re-add HAKY — source research (issuer site first, Yahoo last price only), then remaining-year pay dates, 19a-1 ROC, first lot, Bull/Bear.
**Next:** Finish HAKY first lot + remaining-year dates + ROC confirm on the live household
**Done this turn:** Trustworthy issuer retrieve (`household-pd-issuer-adapters-2026-08-25`). Coverage + dossier strip show source/freshness/last run/miss. Apply issuer sources from provider fills empty/public/unassigned only. Same GET body hash skips re-parse and extra exceptions. Miss copy: Issuer page empty — not using Yahoo. Yahoo stays last-price-only.
**Parked:** M1 macOS; SQLite→Postgres cutover; live OIDC; public CA; Python connectors posting; Shopping Cart; week-close immutability; MAGI oracle rewrite; M8/M9 theater; watchlist

`App.tsx` stays `LocalTauriFinanceClient`. No UI SQL. Never regenerate MAGI oracles.
Owner facts use in-row display/edit in the same field. Save or Cancel; leaving with unsaved edits asks first.

## Two jobs

1. **Seed (once):** `npm run household-seed` writes `database/seed/production` into `%LOCALAPPDATA%\com.finos.desktop` (or `%LOCALAPPDATA%\finos` if that path is a sync folder). Gate: 8 / 75 / 1679 / 1250 / 5862 / 129 and open cost $466,946.66 / tax $461,356.29. Complete household is a no-op. Desktop never searches for xlsx.
2. **Daily use:** desktop opens that SQLite file and queries it. Last prices refresh on open for open lots; a stored last price still displays when stale. Missing last price stays unknown, never $0. No load prompts.

## Pass

```
cargo test -p financial-domain
cargo test -p golden-harness --test new_investment --test add_lot --test position_details --test roc_plan
cargo test -p golden-harness --test accessibility --test invariants --test calculator_plan --test lots_roi --test dividend_slice --test production_seed
```

Desktop stays SQLite.
