# Migration seed (Milestone 2)

Synthetic fixture for the canonical foundation exit gate. Values are **not** MAGI oracles.

- `fixture.yaml` — accounts, securities, and one import document with two candidates
- `expected.yaml` — exact counts and `amount_minor` total after production-path load (stage → validate → approve → post)

Load path: FinanceClient commands (`AccountRegister`, `SecurityRegister`, `ImportStage`, `ImportValidate`, `ImportApprove`, `ImportPost`). SQL is never loaded from the UI.

```
cargo test -p golden-harness seed_counts_and_totals_reconcile
```

Household production templates live in `production/` and are gated by `production_seed_counts_reconcile`. Keep this synthetic fixture for the fast M2 test.
