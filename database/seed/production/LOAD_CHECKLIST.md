# Production seed — load checklist

- [x] Copy `database/seed/production/` into finos repo
- [x] Fixture tests still pass (unchanged)
- [x] Import path reads production templates (same commands as fixture)
- [x] Load order: Accounts → Positions → Lots → Yield → Disbursement
- [x] Reconcile counts: 8 / 75 / 1679 / 1250 / 5862 / 129
- [x] Exceptions visible for bad rows (no silent drop to zero)
- [x] Record money totals in expected-production.yaml after owner review
- [x] Golden / seed test gate green on SQLite
- [ ] Optional later: same gate on Postgres adapter (not authority cutover)
