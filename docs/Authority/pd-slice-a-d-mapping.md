# Position Details Slices A–D Mapping

Superseded as operating docs by [`docs/architecture/pd-slice-a-d-mapping.md`](../architecture/pd-slice-a-d-mapping.md). This file is the 2026-08-24 checkpoint; do not rewrite the body.

Date: 2026-08-24  
Authority: Position Details LOCKED v1.2; owner ROC in-scope 2026-08-24  
Status: **COMPLETE** (pass green). HAKY was not started.

In-repo counterpart: `docs/architecture/pd-slice-a-d-mapping.md`

## Owner ROC in-scope

A symbol is **in-scope** if any of:

- `needs_roc_research`, or
- any ROC year percent is stored, or
- it has open lots on account **Car**

`needsRocResearch` alone is **never** complete.

**Research status strip** (derived, not a second store):

| Status | Meaning |
|--------|---------|
| `complete` | in-scope and 2025 actual observation with source and (2026 actual or 2026 estimate) |
| `estimate-only` | 2026 estimate (or 19a-1 observation) but no 2025 actual |
| `missing-1099` | in-scope and 2025 actual missing |
| `not-in-scope` | otherwise |

Blank template cells stay `None` (not 0). Unknown ≠ $0.

## Slice → requirement IDs

### A — Acceptance harness

| Deliverable | IDs |
|-------------|-----|
| `ac_pd_04_as_of_qty_reconciles_to_open_lots` | AC-PD-04 |
| `ac_pd_06_plan_yoc_uses_52_12_4` | AC-PD-06, PD-BR-05 |
| `ac_pd_07_car_mv_from_car_lots` | AC-PD-07, TR-PD-18 |
| `ac_pd_08_backtest_requires_dates_method_source` | AC-PD-08, PD-BR-03, TR-PD-20 |
| `ac_pd_11_missing_evidence_lowers_confidence` | AC-PD-11, PD-BR-10, TR-PD-26 |
| `ac_pd_12_tier_suggest_does_not_apply` | AC-PD-12, TR-PD-25 |
| `ac_pd_16_roc_does_not_change_original_cost` | AC-PD-16, PD-BR-08, PD-BR-11 |
| `ac_pd_20_rejects_distributions_over_roc_reduced_cost` | AC-PD-20, TR-PD-34 |
| `PositionDetailsCoverageGet` | AC-PD-02 (restated) |
| production seed → `target/position-details-coverage.json` | AC-PD-02 |

### B — Lifetime economic performance

| Deliverable | IDs |
|-------------|-----|
| `total_distributions_received_minor` | TR-PD-33, AC-PD-17 |
| `roc_distributions_minor` (component, not extra cash) | TR-PD-33, AC-PD-17 |
| `cost_recovery_bps` on original economic cost only | PD-BR-08, AC-PD-18, TR-PD-34 |
| unknown when no dividends / incomplete scope | PD-BR-10 |
| AC-PD-20 domain guard | AC-PD-20 |

Denominator = sum of lot performance_basis (opened original cost). Never remaining tax / ROC-reduced basis.

### C — Decision profile on master row

| Deliverable | IDs |
|-------------|-----|
| Evidence dimensions on `PositionMasterRow` | PD-BR-02, TR-PD-24 |
| unknown when no dated result | AC-PD-11, TR-PD-26 |
| price return / cushion / total return separate | AC-PD-09, PD-BR-04 |
| periodDated gate on bear/bull cells | AC-PD-08 |
| no auto tier apply | TR-PD-25, AC-PD-12 |

### D — Concentration (no fixed account columns)

| Deliverable | IDs |
|-------------|-----|
| household `allocationBps` | TR-PD-15 |
| `carMarketValueMinor`, `carShareOfSymbolBps`, `carShareOfHouseholdBps` | AC-PD-07, AC-PD-03 |
| Account splits remain on `PositionDetailsGet` only | AC-PD-03, TR-PD-12 |

Car = account name **Car**. MV = Car lots × last price (cents).

### PD-GAP-ROC-01

| Deliverable | IDs / rule |
|-------------|------------|
| `RocResearchObservation` with `source=template-positions` | Owner 2026-08-24; ROC field design |
| Research strip on master + dossier | Owner in-scope rule |
| Blank template → None not 0 | Unknown ≠ 0 |
| Roundhill `retrieval_template` + `DeclarationRefresh` on desktop open | Declaration freshness; not a daily job |
| Retrieve miss → unknown never $0 | PD-BR-10 |
| Yahoo only when `declarationSource` empty or public | Owner retrieve path |

## Still out (report only; do not build without owner expansion)

- AC-PD-15 closed-position historical mode
- TR-PD-07 PositionExposure
- Effective-dated classification history (TR-PD-05 partial)
- AC-PD-01 field-by-field disposition
- AC-PD-10, AC-PD-14, AC-PD-19
- Amplify / NEOS / YieldMax site adapters
- Python connector posting (parked)

## Execution

| Board | Item |
|-------|------|
| **Now** (at completion) | Position Details slices A–D + PD-GAP-ROC-01 — DONE |
| **Next** | Owner re-add HAKY |

## Ops

1. Fully quit finos  
2. `npm run desktop`  
3. Re-run `npm run household-seed` so the live household picks up ROC observations and Roundhill templates  

Roundhill public fund HTML often has no parseable amounts. A miss stays **unknown**, never $0. TOPW retrieve looks empty until the page exposes parseable rows.
