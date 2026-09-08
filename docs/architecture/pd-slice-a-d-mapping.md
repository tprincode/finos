# Position Details slices A–D + ROC research mapping

Owner-named work 2026-08-24. Profile A only. Unknown ≠ $0. Original economic cost is the cost-recovery denominator (PD-BR-08, AC-PD-16/18/20).

## In this slice

| ID | Behavior | Where |
|---|---|---|
| AC-PD-02 (restated) | Coverage of every security with open lots or characteristics | `PositionDetailsCoverageGet` |
| AC-PD-04 | Account qtys sum to open-lot qty; mismatch raises `qty_reconcile_mismatch` | `PositionDetailsGet` + domain `account_qty_matches_open_lots` |
| AC-PD-06 | Plan YOC uses 52 / 12 / 4 from PlanHistory × qty / original cost | `PositionMasterGet.planYocBps` |
| AC-PD-07 | Car MV = Car lots × last price, never unit-cost × price | `carMarketValueMinor` / share bps on master + dossier |
| AC-PD-08 | Backtest period needs dates and kind; method is locked split-adjusted; reason is implied by kind | `BacktestPeriodRecord` → `regime_period_incomplete` |
| AC-PD-09 | Price return, cushion, and total return stay separate cells | master Bear/Bull columns |
| AC-PD-11 | Missing evidence lowers `dataConfidence`; remaining components stay unknown | domain `dimensions` + master `evidence` |
| AC-PD-12 | Calculate may suggest a tier; owner Apply with reason writes it | `ClassificationSuggestGet` / `ClassificationApply` |
| AC-PD-16 | ROC observation does not change original economic cost | `remainingPerformanceMinor` |
| AC-PD-17 | Total distributions = ledger cash; ROC is a component | `totalDistributionsReceivedMinor` / `rocDistributionsMinor` |
| AC-PD-18 | Cost-recovery denominator is opened original cost | `cost_recovery_bps(..., false)` |
| AC-PD-20 | Total distributions + ROC-reduced denominator is a domain error | `DomainError::CostRecoveryRocReducedDenominator` |
| PD-BR-08 / PD-BR-11 | Original economic cost is immutable for YOC and cost-recovery | lot `performance_basis` |
| TR-PD-24 | Price / cushion / total stay distinct | master cells |
| TR-PD-25 | Do not auto-apply tier | suggestion only until Apply |
| TR-PD-33 | ROC cash counted once in total distributions | lifetime join |
| PD-GAP-ROC-01 | Seed ROC observations with `source=template-positions`; research strip | seed + `rocResearchStatus` |
| Last Price source map | Per-symbol `retrieval_template`; daily `DeclarationRefresh` for registered vendors (`roundhill`, `amplify`, `neos`, `yieldmax`) with per-issuer table parsers | seed + host `DeclarationRefresh` |

ROC in-scope (owner 2026-08-24, not in the PD doc): `needs_roc_research` **or** any stored ROC year percent **or** open lots on account `Car`. `needsRocResearch` alone is never complete.

Research strip: `complete` \| `estimate-only` \| `missing-1099` \| `not-in-scope`.

Declaration freshness: `current` \| `stale` \| `unavailable`. Per-issuer retrieve parses candidates only; a site miss stays unknown, never $0. Roundhill GET is often headers-only (JS); CSV is tried when a href is present.

## Still out (do not build in this slice)

- Closed-position historical mode (AC-PD-15)
- PositionExposure (TR-PD-07)
- Effective-dated classification history
- AC-PD-01 field-by-field disposition
- AC-PD-10 / AC-PD-14 / AC-PD-19
- Global X / JPMorgan / CEF / money-market HTML parsers (seed-tagged `unassigned`; owner paste or a later adapter)
- Roundhill JS calendar (`calHisDistri`); empty GET stays unknown, not Yahoo dividends
- Python connector posting (parked)
