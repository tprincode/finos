# Production seed (household)

These files replace the synthetic 2-account `database/seed/fixture.yaml` for **production reconcile gates**. Keep the fixture for fast unit tests.

## Files

| File | Gate role |
|------|-----------|
| Template_Accounts.xlsx | 8 accounts (cash_symbol, broker_account_number, min_balance_target) |
| Template_Positions.xlsx | ~74 symbols (ROC, frequency, risk_tier → Risk On) |
| Template_Lots.xlsx | ~1,679 lots / ~1,250 open, dual-cost |
| Template_Transactions_Yield.xlsx | ~5,862 YIELD rows |
| Template_Transactions_Disbursement.xlsx | ~129 non-ROI rows |
| expected-production.yaml | Count gates: 8 / 75 / 1679 / 1250 / 5862 / 129 |

## Steps (finos repo)

1. Copy this entire `production/` folder to:
   `finos/database/seed/production/`

2. Keep existing:
   `database/seed/fixture.yaml`
   `database/seed/expected.yaml`
   (fast harness — do not delete)

3. Load this machine’s desktop SQLite once (not an owner screen):

   `npm run household-seed`

   That writes into `%LOCALAPPDATA%\com.finos.desktop` (or `%LOCALAPPDATA%\finos` if that path is a sync folder). Complete household is a no-op. CI still uses a temp DB via `cargo test -p golden-harness -- production_seed_counts_reconcile`.

4. Load order (respect FKs / identity):
   Accounts → Positions (Security Master) → Lots → Yield transactions → Disbursement transactions

5. Run reconcile and compare to `expected-production.yaml`:
   - accounts = 8
   - positions = 74
   - lots_total = 1679
   - lots_open = 1250
   - transactions_yield = 5862
   - transactions_disbursement = 129

6. On first green run, record money totals in `expected-production.yaml` under `totals:`
   and owner-approve. Do not let tests auto-write expected values.

7. Gate for Profile A “household ready”:
   `cargo test -p golden-harness production_seed_counts_reconcile` (or equivalent name)
   must pass on SQLite before treating desktop as daily-driver data.

## Rules (do not violate)

- Unknown ≠ zero
- No FIFO; lots explicit
- Dual basis: unit_cost_original + unit_cost_tax
- CRF zero-cost only for CRF DRIP; auto-capture FI Roth only
- Idempotent re-import
- Authority: templates here → import commands → SQLite; not `app/portfolio.db` experiment
