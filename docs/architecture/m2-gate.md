# M2 gate — Canonical foundation

Exit (V1.1 §12): migration seed counts and critical totals reconcile exactly.

Do not start M3 until this list is green.

## Commands

From repo root:

```
cargo test --workspace
cargo test -p golden-harness
cargo test -p golden-harness --features magi-gate
```

The last command is the **M5** MAGI gate. After owner approval it must pass on the production path (`cargo test -p golden-harness --features magi-gate`).

## Packs that must pass for M2

| Filter | What it proves |
|--------|----------------|
| `canonical_week` / financial-domain week tests | Saturday–Friday week (L0) |
| `unknown_amount` | Unknown is not coerced to zero (L0) |
| `posting_does_not_assign_a_fifo_lot` | No FIFO assumption (L0) |
| `import_cannot_post_without_approve` | Capture cannot post without `ImportApprove` (L2, AC-ARCH-12) |
| `seed_counts_and_totals_reconcile` | Production-path seed vs `database/seed/expected.yaml` |
| `application_core_and_financial_domain_do_not_depend_on_sqlx_or_tauri` | AC-ARCH-02 crate graph |
| `desktop_react_contains_no_sql` | UI never embeds SQL |
| `approval_alone_does_not_pass_the_sync_verdict` | Approval without the production-path comparator is not `Passed` |

## Out of this gate

MAGI engine, lots/CRF, PostgreSQL, OIDC, signed installers, AI posting.
