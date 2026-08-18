# Component contracts

Versioned command/query names for the modular monolith (V1.1 §4.3, ADR-0005, ADR-0006).
Wire types live in `packages/app-contracts` and `crates/application-core`. Implementations land in later milestones.

Isolation rule: a component may reference another only through these contracts or published domain events. No component reads or writes another component's tables.

Contract version: `1.0.0-draft` (`FINANCE_CLIENT_CONTRACT_VERSION`).

## Platform

| Component | Commands | Queries | Events |
|-----------|----------|---------|--------|
| Configuration | `ConfigSet` | `ConfigGet` | `ConfigChanged` |
| Audit | _(record only)_ | `AuditList` | `AuditRecorded` |
| Approvals | `ApprovalDecide` | `ApprovalGet` | `ApprovalResolved` |
| Exceptions | `ExceptionAcknowledge` | `ExceptionList` | `ExceptionRaised` |
| Health | — | `HealthGet` | — |
| Snapshot and restore | `SnapshotCreate`, `SnapshotRestore`, `HandoffResolve` | `SnapshotHeadGet`, `HandoffStatusGet` | `SnapshotPublished`, `HandoffBlocked` |

Does not touch tables of: Registries, Capture, Ledger, Market/dividend, Planning, Reporting.

## Registries

| Component | Commands | Queries | Events |
|-----------|----------|---------|--------|
| Account Registry | `AccountRegister`, `AccountUpdate` | `AccountGet`, `AccountList` | `AccountRegistered` |
| Security Master | `SecurityRegister`, `SecurityUpdate` | `SecurityGet` | `SecurityRegistered` |
| Symbol History | `SymbolAliasRecord` | `SymbolResolve` | `SymbolAliasEffective` |

Does not touch tables of: Capture, Ledger, Market/dividend, Planning.

## Capture

| Component | Commands | Queries | Events |
|-----------|----------|---------|--------|
| Source Evidence | `EvidenceStore` | `EvidenceGet` | `EvidenceStored` |
| Import Staging | `ImportStage`, `ImportValidate`, `ImportApprove`, `ImportPost` | `ImportBatchGet` | `ImportStaged`, `ImportPosted` |
| Connector Runtime | `ConnectorJobSubmit` | `ConnectorJobGet` | `ConnectorCandidatesReady` |

Does not touch tables of: Ledger (posts only via application transaction after approval), Planning, Reporting.

## Financial ledger

| Component | Commands | Queries | Events |
|-----------|----------|---------|--------|
| Investment Activity Ledger | `ActivityPost`, `ActivityCorrect` | `ActivityGet`, `ActivityList` | `ActivityPosted`, `ActivityCorrected` |
| Lot/Basis | `LotAssign` | `LotGet`, `BasisGet` | `LotAssigned` |
| Distribution Characterization | `DistributionCharacterize` | `DistributionGet` | `DistributionCharacterized` |

Does not touch tables of: Capture source blobs, Registries (references IDs only), Reporting read models.

## Market / dividend

| Component | Commands | Queries | Events |
|-----------|----------|---------|--------|
| Price Service | `PriceRecord` | `PriceGet` | `PriceRecorded` |
| Dividend Intelligence | `DividendDeclare`, `DividendActualRecord` | `DividendGet` | `DividendDeclared`, `DividendActualPosted` |
| Calendar | — | `CanonicalWeekGet` | — |

Does not touch tables of: Ledger, Planning.

## Planning

| Component | Commands | Queries | Events |
|-----------|----------|---------|--------|
| Calculator Plan | `PlanApprove` | `PlanGet` | `PlanApproved` |
| Income Plan | `IncomePlanUpdate` | `IncomePlanGet` | `IncomePlanChanged` |
| Cash Burndown | — | `BurndownGet` | — |
| Marketplace MAGI | `MagiRuleSet`, `MagiFactRecord`, `MagiCoverageSet`, `MagiAdjustmentRecord` | `MagiProjectionGet`, `MagiTaxPaymentGet` | — |

Does not touch tables of: Ledger (reads via queries), Capture.

## Reporting

| Component | Commands | Queries | Events |
|-----------|----------|---------|--------|
| Position Details | — | `PositionDetailsGet` | — |
| ROI | — | `RoiGet` | — |
| Trends | — | `TrendsGet` | — |
| Dashboard | — | `DashboardGet` | — |

Read models are not authoritative (ADR-0008). Does not write Ledger or Capture tables.

## Decision support

| Component | Commands | Queries | Events |
|-----------|----------|---------|--------|
| Allocation | `AllocationTargetSet` | `AllocationGet` | `AllocationTargetChanged` |
| Shopping Cart | `CartItemAdd`, `CartItemRemove` | `CartGet` | `CartChanged` |
| Backtesting | `BacktestRun` | `BacktestGet` | `BacktestCompleted` |
| Classification Review | `ClassificationReviewRecord` | `ClassificationReviewGet` | `ClassificationReviewChanged` |
| Tax Projection | — | `TaxProjectionGet` | — |

Does not post Ledger facts.

## AI advisory (V1, advisory only)

| Component | Commands | Queries | Events |
|-----------|----------|---------|--------|
| AI Gateway | `AiAnalyze` | `AiRunGet` | `AiRecommendationReady` |
| Analysis Run Registry | — | `AnalysisRunList` | — |

Must not mutate authoritative financial facts (ADR-0012). The Grok key is read from `XAI_API_KEY` or `GROK_API_KEY`; it is never stored in SQLite.
