# M5 gate — Marketplace MAGI pack plus Plan / burndown

Exit (V1.1 §12): owner-approved G-MAGI-01–10 oracles pass through production-path commands (`MagiRuleSet`, `MagiFactRecord`, `MagiCoverageSet`, `MagiAdjustmentRecord`, `MagiProjectionGet`). Expected oracles are never generated or overwritten by the engine.

Household of 2, contiguous US/DC, coverage year 2026. Threshold $84,600. Safety reserve $5,000. OVER when actual is at or above the threshold. G10-ADJ-1 is owner-approved; posted MAGI $52,000 matches adjusted forms.

Calculator Plan is versioned and never rewrites actual cash. `BurndownGet` uses Plan remaining as the obligation.

Do not start M6+ (AI, PostgreSQL, OIDC, signed installers) until this list is green.

## Commands

From repo root:

```
cargo test --workspace
cargo test -p golden-harness --features magi-gate
cargo test -p golden-harness -- plan_burndown
cargo test -p financial-domain -- magi
cargo test -p financial-domain -- plan
```

## Packs that must pass for M5

| Filter | What it proves |
|--------|----------------|
| `approved_magi_oracles_exist_with_locked_rule` | G-MAGI-01–10 files exist; owner-approved; threshold 8460000 |
| `approval_alone_does_not_pass_the_sync_verdict` | Approval without the comparator is not `Passed` |
| `m5_magi_pack_must_pass` | Production-path compare matches independent oracles |
| `g01_safe_below_threshold` / `g02_cent_boundary` | L0 MAGI decision order |
| `plan_remaining_is_not_actual_cash` / `burndown_surplus_is_exact_cents` | L0 Plan vs cash |
| `plan_approve_does_not_rewrite_dividend_actuals` | PlanApprove leaves DividendGet actuals unchanged |
| `seed_counts_and_totals_reconcile` | M2 seed still reconciles |
| `fidelity_reimport_posts_one_actual_and_all_views_agree` | M3 dividend slice still holds |
| `explicit_lots_dual_basis_no_fifo_and_broker_recon` | M4 lots/ROI still holds |

## Out of this gate

PostgreSQL, OIDC, signed installers, AI posting.
