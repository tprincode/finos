# M4 gate — Lots and ROI

Exit (V1.1 §12): no FIFO assumption; CRF tests and broker lot reconciliation pass.

Dual basis (price-paid performance vs tax-adjusted) stay separate. Zero-cost DRIP lots are allowed only on CRF (including FI Roth). Automatic CRF DRIP capture is FI Roth only. Sales and option closes stay unmatched until `LotAssign` names a lot.

Do not start M5 (MAGI) until this list is green.

## Commands

From repo root:

```
cargo test --workspace
cargo test -p golden-harness -- lots_roi
cargo test -p financial-domain -- lot
```

`cargo test -p golden-harness --features magi-gate` is the **M5** MAGI gate.

## Packs that must pass for M4

| Filter | What it proves |
|--------|----------------|
| `sale_without_lot_id_is_not_fifo` | Missing lot id is refused (L0) |
| `taxable_zero_cost_drip_is_refused` / `crf_zero_cost_drip_is_allowed` | CRF policy (L0) |
| `explicit_lots_dual_basis_no_fifo_and_broker_recon` | Owner can assign the expensive lot; broker recon matches after `LotAssign`; ROI components differ by basis |
| `crf_policy_and_fi_roth_auto_capture` | Explicit CRF zero-cost DRIP; FI Roth import auto-captures; taxable import raises `zero_cost_drip_not_crf` |
| `option_close_requires_explicit_lot` | Option close is unmatched until explicit assign |
| `seed_counts_and_totals_reconcile` | M2 seed still reconciles |
| `fidelity_reimport_posts_one_actual_and_all_views_agree` | M3 dividend slice still holds |

## Out of this gate

MAGI engine, PostgreSQL, OIDC, signed installers, AI posting.
