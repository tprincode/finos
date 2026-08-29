# Profile A depth

Still desktop-local SQLite. Not Milestone 8. Fill contracted queries that M2–M7 left as stubs or missing dispatch.

## Slice 1 — PositionDetailsGet

Read model over open lots (ADR-0008). Dual basis stays separate. Does not post dividend or lot facts. No schema bump.

```
cargo test -p financial-domain -- position
cargo test -p golden-harness -- position_details
cargo test --workspace
```

| Filter | What it proves |
|--------|----------------|
| `position_rollup_is_not_cash` | Two lots of one security become one line; cash is unchanged |
| `position_details_rollup_does_not_post_dividend_or_change_basis_totals` | PositionDetailsGet rolls up; DividendGet and BasisGet totals unchanged |

## Slice 2 — TaxProjectionGet

Decision-support view over `MagiProjectionGet`. Does not post MAGI or dividend facts. Does not rewrite oracles.

```
cargo test -p financial-domain -- tax_projection
cargo test -p golden-harness -- tax_projection
cargo test --workspace
```

| Filter | What it proves |
|--------|----------------|
| `tax_projection_does_not_change_magi_decision` | View copies SAFE; snapshot decision unchanged |
| `tax_projection_matches_magi_and_does_not_post_dividend_or_rewrite_oracles` | TaxProjectionGet matches MagiProjectionGet; DividendGet unchanged |

## Slice 3 — Import UX

Desktop stages a sample Fidelity CSV through FinanceClient (`ImportStage` → validate → approve → post). No UI SQL. Register the sample account first.

```
cargo test -p golden-harness -- accessibility
cargo test -p golden-harness -- dividend_slice
cargo test --workspace
```

## Out of these slices

Postgres, OIDC, updater, M1 macOS. Promoting one M6 stub to a real data workflow is a later Profile A slice.
