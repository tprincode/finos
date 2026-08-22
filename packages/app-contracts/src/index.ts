/** Shared FinanceClient contract version — must match Rust application-core. */
export const FINANCE_CLIENT_CONTRACT_VERSION = "1.0.0-draft";

/** Fixed-scale money DTO (ADR-0004). Scale is documented in schema metadata. */
export type Money = {
  amountMinor: number;
  scale: number;
};

export type CommandRequest = {
  contractVersion: string;
  commandName: string;
  correlationId: string;
  bodyJson?: string;
  expectedVersion?: number;
};

export type QueryRequest = {
  contractVersion: string;
  queryName: string;
  correlationId: string;
  bodyJson?: string;
};

export type CommandResult = {
  contractVersion: string;
  commandName: string;
  correlationId: string;
  ok: boolean;
  errorCode?: string;
  bodyJson?: string;
};

export type QueryResult = {
  contractVersion: string;
  queryName: string;
  correlationId: string;
  ok: boolean;
  errorCode?: string;
  bodyJson?: string;
};

/** Snapshot identity fields (ADR-0007, V1.1 §7). Matches crates/snapshot-service SnapshotManifest. */
export type SnapshotIdentity = {
  databaseId: string;
  snapshotId: string;
  parentSnapshotId: string | null;
  deviceId: string;
  deviceName: string;
  changeSequence: number;
  lastEventAt: string;
  schemaVersion: string;
  calculationVersion: string;
  appVersion: string;
  databaseHash: string;
  evidenceManifestHash: string;
  validationStatus: string;
  restoreTestStatus: string;
};

export type HandoffDecision =
  | "open_normally"
  | "block_until_restore"
  | "allow_write_publish_pending"
  | "branch_conflict"
  | "unverified_handoff"
  | "reject_invalid_keep_last";

export type HandoffStatus = {
  decision: HandoffDecision;
  writesAllowed: boolean;
  message: string;
  localHead: SnapshotIdentity | null;
  publishedHead: SnapshotIdentity | null;
};

export type DeviceConfig = {
  deviceId: string;
  deviceName: string;
  databaseId: string;
  schemaVersion: string;
  calculationVersion: string;
  appVersion: string;
};

export type AccountRecord = {
  accountId: string;
  name: string;
  kind: string;
  rowVersion?: number;
};

export type CanonicalWeek = {
  asOfDate: string;
  start: string;
  end: string;
};

export type ReconcileCounts = {
  accounts: number;
  securities: number;
  evidence: number;
  importBatches: number;
  postedActivities: number;
  amountMinorSum: number;
  scale: number;
  auditRecords: number;
  exceptionsOpen: number;
};

export type DividendActual = {
  actualId: string;
  accountId: string;
  securityId: string | null;
  occurredOn: string;
  amountMinor: number;
  scale: number;
  activityId: string | null;
};

export type DividendDeclaration = {
  declarationId: string;
  securitySymbol: string;
  declaredOn: string;
  amountMinor: number;
  scale: number;
};

export type DividendGet = {
  actuals: DividendActual[];
  declarations: DividendDeclaration[];
  actualTotalMinor: number;
  scale: number;
};

export type IncomePlanGet = {
  plannedMinor: number;
  actualMinor: number;
  scale: number;
};

export type IncomePlanWeekGet = {
  asOfDate: string;
  start: string;
  end: string;
  status: string;
  lines: Array<{
    accountName: string;
    actualMinor: number;
    plannedMinor: number;
    planKnown: boolean;
    scale: number;
  }>;
  drilldown: Array<{
    accountName: string;
    symbol: string;
    occurredOn: string;
    amountMinor: number;
    scale: number;
  }>;
  latestActualOn: string | null;
  yieldCount: number;
  scale: number;
};

export type HouseholdSummaryGet = {
  accountCount: number;
  openLotCount: number;
  yieldCount: number;
  disbursementCount: number;
  latestYieldOn: string | null;
  planCount: number;
};

export type CalculatorGet = {
  rows: Array<{
    symbol: string;
    paymentFrequency: string;
    planKnown: boolean;
    planPerShareMinor: number;
    planScale: number;
    planningPeriodsPerYear: number;
    remainingQuantityMinor: number;
    quantityScale: number;
    planPaymentMinor: number;
    remainingPerformanceMinor: number;
    rocPct2025ActualMinor: number | null;
    rocScale: number | null;
    scale: number;
  }>;
  planCount: number;
  scale: number;
};

export type DashboardBurndownGet = {
  asOfDate: string;
  start: string;
  end: string;
  status: string;
  note: string;
  lines: Array<{
    accountName: string;
    inflowMinor: number;
    outflowMinor: number;
    floorKnown: boolean;
    scale: number;
  }>;
  scale: number;
};

export type HoldingsGet = {
  lots: Array<{
    lotId: string;
    accountName: string;
    symbol: string;
    openedOn: string;
    remainingQuantityMinor: number;
    quantityScale: number;
    remainingPerformanceMinor: number;
    remainingTaxMinor: number;
    scale: number;
  }>;
  scale: number;
};

export type DashboardGet = {
  actualDividendMinor: number;
  plannedIncomeMinor: number;
  scale: number;
};

export type TrendsGet = {
  points: Array<{ occurredOn: string; amountMinor: number }>;
  totalMinor: number;
  scale: number;
};

export type BasisGet = {
  lots: Array<{
    lotId: string;
    remainingQuantityMinor: number;
    remainingPerformanceMinor: number;
    remainingTaxMinor: number;
    crfZeroCost: boolean;
  }>;
  openPerformanceMinor: number;
  openTaxMinor: number;
  scale: number;
};

export type RoiGet = {
  proceedsMinor: number;
  performanceCostMinor: number;
  taxCostMinor: number;
  performanceGainMinor: number;
  taxGainMinor: number;
  dividendActualMinor: number;
  openPerformanceMinor: number;
  openTaxMinor: number;
  scale: number;
};

export type BrokerLotReconcileGet = {
  soldQuantityMinor: number;
  assignedQuantityMinor: number;
  unmatchedSells: number;
  matched: boolean;
  quantityScale: number;
};

export type PositionDetailsGet = {
  positions: Array<{
    accountId: string;
    accountName: string;
    securityId: string;
    symbol: string;
    remainingQuantityMinor: number;
    quantityScale: number;
    remainingPerformanceMinor: number;
    remainingTaxMinor: number;
    lotCount: number;
    scale: number;
  }>;
  openPerformanceMinor: number;
  openTaxMinor: number;
  scale: number;
};

export type TaxProjectionGet = {
  sourceQuery: string;
  decisionState: DecisionState;
  actualIncludedYtd: Money;
  applicableThreshold: Money;
  dataCompleteness: DataCompleteness;
};

export type DecisionState =
  | "SAFE"
  | "WATCH"
  | "LIKELY_OVER"
  | "OVER"
  | "INDETERMINATE";

export type DataCompleteness = "complete" | "incomplete" | "pending_review";

/** Owner-facing MAGI output shape (V1.1 §11.4). */
export type MagiProjection = {
  applicableThreshold: Money;
  actualIncludedYtd: Money;
  knownRemaining: Money;
  baseForecast: Money;
  conservativeForecast: Money;
  uncertainAmount: Money;
  rawHeadroom: Money;
  protectedHeadroom: Money;
  dataCompleteness: DataCompleteness;
  decisionState: DecisionState;
  warnings: string[];
  calculationTrace: string[];
};

export type MagiTaxPaymentGet = {
  amountMinor: number;
  scale: number;
};

export type PlanGet = {
  planId: string;
  version: number;
  remainingMinor: number;
  scale: number;
  approvedOn: string;
  supersedesPlanId?: string;
};

export type BurndownGet = {
  cashMinor: number;
  obligationMinor: number;
  surplusMinor: number;
  sufficient: boolean;
  scale: number;
};

export type AllocationGet = {
  targets: Array<{
    targetId: string;
    name: string;
    targetMinor: number;
    scale: number;
  }>;
  openPerformanceMinor: number;
  openTaxMinor: number;
  scale: number;
};

export type CartGet = {
  items: Array<{
    itemId: string;
    symbol: string;
    quantityMinor: number;
    quantityScale: number;
  }>;
};

export type BacktestGet = {
  runs: Array<{
    runId: string;
    scenario: string;
    hypotheticalPnlMinor: number;
    scale: number;
    completedAt: string;
  }>;
};

export type ClassificationReviewGet = {
  reviews: Array<{
    reviewId: string;
    factKey: string;
    classification: string;
    status: string;
  }>;
};

export type ActivityRecord = {
  activityId: string;
  accountId: string;
  securityId: string | null;
  activityType: string;
  amountMinor: number;
  scale: number;
  occurredOn: string;
};

export type ExceptionRecord = {
  exceptionId: string;
  code: string;
  message: string;
  acknowledged: boolean;
  createdAt: string;
};

export type DistributionGet = {
  characterizations: Array<{
    characterizationId: string;
    activityId: string;
    category: string;
    amountMinor: number;
    scale: number;
  }>;
};

export type AnalysisRunList = {
  runs: Array<{
    runId: string;
    prompt: string;
    recommendation: string;
    provider: string;
    model: string;
    status: string;
  }>;
};

export type AccountListItem = {
  accountId: string;
  name: string;
  kind: string;
};

export type SecurityListItem = {
  securityId: string;
  symbol: string;
  name: string;
};

export type PlanReviewGet = {
  securityId: string;
  observationCount: number;
  mostCurrentMinor: number | null;
  avg6Minor: number | null;
  minMinor: number | null;
  maxMinor: number | null;
  averageMinor: number | null;
  eightyPctOfAvgMinor: number | null;
  avg6Complete: boolean;
  fullAnalysisPossible: boolean;
  confirmBlocked: boolean;
  incompleteReasonRequired: boolean;
  amountScale: number;
};

export type CurrentPriceGet = {
  securityId: string;
  priceMinor: number | null;
  scale: number;
  freshness: string;
  priceDerivedValid: boolean;
};

export type InvestmentGet = {
  securityId: string;
  symbol: string;
  name: string;
  paymentFrequency: string;
  riskTier: string;
  provider: string;
  underlying: string;
  planKnown: boolean;
  planPerShareMinor: number;
  planScale: number;
  planningPeriodsPerYear: number;
  planReason: string;
  planEffectiveFrom: string;
  remainingQuantityMinor: number;
  quantityScale: number;
  remainingPerformanceMinor: number;
  rocPct2025ActualMinor: number | null;
  rocScale: number | null;
  price: CurrentPriceGet;
  review: PlanReviewGet;
  template: {
    priceSource: string;
    sourceSymbol: string;
    declarationSource: string;
    lookbackCount: number;
    paymentSource: string;
  } | null;
  lots: Array<{
    lotId: string;
    accountName: string;
    openedOn: string;
    remainingQuantityMinor: number;
    quantityScale: number;
    remainingPerformanceMinor: number;
    remainingTaxMinor: number;
    scale: number;
  }>;
  declarationCount: number;
  firstLotComplete: boolean;
  scale: number;
};

/** Check-for-update status. Fail closed: never applied, never posted. */
export type UpdaterCheckGet = {
  applied: boolean;
  posted: boolean;
  status: string;
};
