# Execution board

Profile A product only. Household SQLite is the system of record. Seed is a separate CLI, not an owner screen.

**Now:** New Investment (mandatory facts + live/prompted CurrentPrice + 12 declarations + Plan-review + first lot) and Add Lot (existing symbol → upcoming Income Plan + maintenance set)  
**Next:** none until named  
**Parked:** M1 macOS; SQLite→Postgres cutover; live OIDC; public CA; Python connectors posting; Shopping Cart; week-close immutability; MAGI oracle rewrite; M8/M9 theater; watchlist

`App.tsx` stays `LocalTauriFinanceClient`. No UI SQL. Never regenerate MAGI oracles.

## Two jobs

1. **Seed (once):** `npm run household-seed` writes `database/seed/production` into `%LOCALAPPDATA%\com.finos.desktop` (or `%LOCALAPPDATA%\finos` if that path is a sync folder). Gate: 8 / 74 / 1679 / 1250 / 5862 / 129. Complete household is a no-op. Desktop never searches for xlsx.
2. **Daily use:** desktop opens that SQLite file and queries it. No load prompts.

## Pass

```
cargo test -p financial-domain
cargo test -p golden-harness --test new_investment --test add_lot
cargo test -p golden-harness --test accessibility --test invariants --test calculator_plan --test lots_roi --test dividend_slice --test production_seed
```

Desktop stays SQLite.
