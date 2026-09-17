# Component contracts

Versioned command/query names for the modular monolith (V1.1 §4.3, ADR-0005, ADR-0006).
Wire types live in `packages/app-contracts` and `crates/application-core`. Live names are registered in `crates/application-core/src/queries.rs`.

Isolation: one SQLite file and one `Canonical` port. A component must not reach another component’s tables except through these contracts or published events. Isolation is a review rule, not a schema-per-component guarantee.

Contract version: `1.0.0-draft` (`FINANCE_CLIENT_CONTRACT_VERSION`).

## Platform

| Component | Commands | Queries | Events |
|-----------|----------|---------|--------|
| Configuration | `ConfigSet` | `ConfigGet` | `ConfigChanged` |
| Audit | _(record only)_ | `AuditList` | `AuditRecorded` |
| Approvals | `ApprovalDecide` | `ApprovalGet` | `ApprovalResolved` |
| Exceptions | `ExceptionAcknowledge` | `ExceptionList` | `ExceptionRaised` |
| Health | — | `HealthGet` | — |
| Core functions | — | `CoreFunctionsGet` | — |
| Work tickets | `WorkTicketResolve`, `WorkTicketFile`, `WorkTicketSyncMisses` | `WorkTicketList` | — |
| Snapshot and restore | `SnapshotCreate`, `SnapshotRestore`, `HandoffResolve`, `SnapshotImport` | `SnapshotHeadGet`, `HandoffStatusGet` | `SnapshotPublished`, `HandoffBlocked` |

Does not touch tables of: Registries, Capture, Ledger, Market/dividend, Planning, Reporting — except through the canonical port.

## Registries

| Component | Commands | Queries | Events |
|-----------|----------|---------|--------|
| Account Registry | `AccountRegister`, `AccountUpdate` | `AccountGet`, `AccountList` | `AccountRegistered` |
| Security Master | `SecurityRegister`, `SecurityUpdate` | `SecurityGet`, `SecurityList` | `SecurityRegistered` |
| Symbol History | `SymbolAliasRecord` | `SymbolResolve` | `SymbolAliasEffective` |

## Capture

| Component | Commands | Queries | Events |
|-----------|----------|---------|--------|
| Source Evidence | `EvidenceStore` | `EvidenceGet` | `EvidenceStored` |
| Import Staging | `ImportStage`, `ImportValidate`, `ImportApprove`, `ImportPost` | `ImportBatchGet` | `ImportStaged`, `ImportPosted` |
| Collector runtime | `CollectorRetrieve`, `CollectorAlignFutureToPlan`, `CollectorFieldDecisionSet`, `ProviderDeclarationSourcesApply`, `DeclarationRefresh` | `CollectorSetGet` / fleet queries, `ResearchGapsGet` | — |
| Connector Runtime | `ConnectorJobSubmit` | `ConnectorJobGet` | `ConnectorCandidatesReady` |

## Financial ledger

| Component | Commands | Queries | Events |
|-----------|----------|---------|--------|
| Investment Activity Ledger | `ActivityPost`, `ActivityCorrect` | `ActivityGet`, `ActivityList` | `ActivityPosted`, `ActivityCorrected` |
| Lot/Basis | `LotOpen`, `LotAssign` | `LotGet`, `BasisGet` | `LotAssigned` |
| Distribution Characterization | `DistributionCharacterize` | `DistributionGet` | `DistributionCharacterized` |

## Cash Management (own area)

Week entry, SSA (Barbara $1,331 and Tom $2,865), IRA/Roth distributions, and Withdrawal. Not a Planning or Ledger sub-component. Week **entry** is this UI; `TrendsWeek*` remain Reporting writes used by the desk.

| Commands | Queries |
|----------|---------|
| `CashDistributionPost`, `SsaConfirm` | `CashManagementWeekGet`, `CashManagementRemindersGet`, `CashManagementMonthGet`, `CarRocPlanGet` |

`CarRocPlanGet` is the Car **planning** report: remaining ordinary vs ROC, YTD paid split by `roc_pct_2026_estimate`, and long/short tax-lot gain or loss. Seed `Form_1099` is tax-year 2025 ROC guidance and never appears on current-year YTD cards. Car 2026 tax stays unknown until April 2027. Planning writes stay on `RocPlanConfirm`.

Withdrawal is cash leaving a taxable / non-IRA brokerage (Car, Robinhood, ENERGYX). `CashDistributionPost` / `SsaConfirm` refuse a type that does not match the account: IRA_Distribution only on `ira`; Roth_Distribution on `roth` / `fi_roth`; Withdrawal on taxable and not External; SSA only on External (by name — live seed stores External as `taxable`). SSA is two household payees, separate confirms.

## Market / dividend

| Component | Commands | Queries | Events |
|-----------|----------|---------|--------|
| Price Service | `PriceQuoteRecord`, `ManualPriceOverride`, `LastPriceRefresh` | `CurrentPriceGet`, `LastPriceAutoWindowGet` | `PriceRecorded` |
| Dividend Intelligence | `DividendDeclare`, `DividendActualRecord` | `DividendGet`, `DividendPerformanceGet` | `DividendDeclared`, `DividendActualPosted` |
| Calendar | — | `CanonicalWeekGet` | — |

## Planning

| Component | Commands | Queries | Events |
|-----------|----------|---------|--------|
| Calculator Plan | `PlanApprove` | `CalculatorGet`, `PlanGet` | `PlanApproved` |
| Income Plan | `IncomePlanUpdate` | `IncomePlanGet`, `IncomePlanWeekGet`, `IncomePlanGridGet`, `IncomePlanExportGet` | `IncomePlanChanged` |
| Cash Burndown | — | `BurndownGet` | — |
| Marketplace MAGI | `MagiRuleSet`, `MagiFactRecord`, `MagiCoverageSet`, `MagiAdjustmentRecord` | `MagiProjectionGet`, `MagiTaxPaymentGet` | — |

Does not write Ledger facts (reads via queries). MAGI oracles are locked (ADR-0013).

## Reporting

| Component | Commands | Queries | Events |
|-----------|----------|---------|--------|
| Position Details | — | `PositionDetailsGet` | — |
| ROI | — | `RoiGet` | — |
| Trends (charts) | `TrendsWeekSave`, `TrendsWeekCorrect`, `TrendsWeekClose`, `WeekCaptureAccept` | `TrendsGet`, `TrendsWeekGet` | — |
| Dashboard | — | `DashboardBurndownGet`, `DashboardGet` | — |
| Account values (Home) | — | `AccountValueHomeGet` (`points`, `trendsPoints`, `incomePoints`) | — |
| Dividend plan (Home) | — | `DividendPlanHomeGet` | — |
| Cash pile | — | `CashPileGet` | — |

Read models are not authoritative (ADR-0008). Week snapshot writes are Reporting; the owner enters them on Cash Management. `incomePoints` are closed Sat–Fri week actuals plotted on Friday.

## Decision support

| Component | Commands | Queries | Events |
|-----------|----------|---------|--------|
| Allocation | `AllocationTargetSet` | `AllocationGet` | `AllocationTargetChanged` |
| Shopping Cart | `CartItemAdd`, `CartItemRemove` | `CartGet` | `CartChanged` |
| Shopping Cart scenarios | `CartScenarioCreate`, `CartSellLineAdd`, `CartBuyLineAdd`, `CartBuyLineQtySet`, `CartScenarioSave`, `CartScenarioAgree`, `CartExecuteSell`, `CartExecuteFill`, `CartExecuteBuyStep`, `CartScenarioDiscard`, `CartScenarioRename`, `CartScenarioDuplicate`, `CashDeposit`, `CashWithdraw` | `CartScenarioEvaluate`, `CartScenarioGet`, `CartScenarioList` | — |
| Backtesting | `BacktestRun` | `BacktestGet` | `BacktestCompleted` |
| Classification Review | `ClassificationReviewRecord` | `ClassificationReviewGet` | `ClassificationReviewChanged` |
| Tax Projection | — | `TaxProjectionGet` | — |

`CartItemAdd` / `CartGet` are the M6 symbol+qty slice. Scenario commands are **shipped** (SC-1+). Evaluate and cart SQL live in `application-core` `cart.rs` plus `financial-domain` / `storage-sqlite` cart modules. Allocation, Backtest, and AI have tables; owner menus stay parked.

## AI advisory (V1, advisory only)

| Component | Commands | Queries | Events |
|-----------|----------|---------|--------|
| AI Gateway | `AiAnalyze` | `AiRunGet` | `AiRecommendationReady` |
| Analysis Run Registry | — | `AnalysisRunList` | — |

Must not mutate authoritative financial facts (ADR-0012). Parked UX.
