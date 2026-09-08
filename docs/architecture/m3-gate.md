# M3 gate — Dividend vertical slice

Exit (V1.1 §12): one posted `DividendActual` updates Income Plan, Dashboard, and Trends without re-entry. Re-import is idempotent.

Declaration, plan, and actual stay separate. View actuals come from posted dividend actuals, not declarations.

Do not start M4 (lots/ROI) until this list is green.

## Commands

From repo root:

```
cargo test --workspace
cargo test -p golden-harness
cargo test -p golden-harness -- dividend_slice
```

`cargo test -p golden-harness --features magi-gate` is the **M5** MAGI gate.

## Packs that must pass for M3

| Filter | What it proves |
|--------|----------------|
| `fidelity_reimport_posts_one_actual_and_all_views_agree` | Fidelity-shaped CSV staged twice after post → one actual; DividendGet / IncomePlanGet / DashboardGet / TrendsGet share 50000 minor; `DividendDeclare` does not change actuals |
| `schwab_capture_matches_fidelity_amount_on_the_same_views` | Schwab-shaped CSV posts the same 50000 minor through the same views |
| `fidelity_dividend_becomes_a_candidate_not_a_post` | Broker parse returns candidates only (ADR-0010) |
| `import_cannot_post_without_approve` | Capture still cannot post without `ImportApprove` |
| `seed_counts_and_totals_reconcile` | M2 seed still reconciles |
| `desktop_react_contains_no_sql` | UI never embeds SQL |

## Out of this gate

MAGI engine, lots/CRF, PostgreSQL, OIDC, signed installers, AI posting.
