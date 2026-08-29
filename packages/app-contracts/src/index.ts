/** Shared FinanceClient contract version — must match Rust application-core. */
export const FINANCE_CLIENT_CONTRACT_VERSION = "1.0.0-draft";

/** Registered issuer declaration sources. Yahoo is never in this list. */
export const REGISTERED_DECLARATION_SOURCES = [
  "roundhill",
  "amplify",
  "neos",
  "yieldmax",
  "cornerstone",
  "direxion",
  "proshares",
  "saba",
  "ellington",
  "enterprise",
  "energytransfer",
  "gladstone",
  "jpmorgan",
  "mplx",
  "orchidisland",
  "globalx",
  "simplify",
  "tappalpha",
  "trinity",
  "ftvest",
  "trex",
  "fidelity",
  "schwab",
] as const;

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

export type DataSummaryGet = {
  accountCount: number;
  openLotCount: number;
  yieldCount: number;
  disbursementCount: number;
  latestYieldOn: string | null;
  planCount: number;
  symbolCount: number;
  openPerformanceMinor: number;
  openTaxMinor: number;
  lastPriceCount: number;
  marketValueMinor: number | null;
  marketValueComplete: boolean;
  /** Lifetime paid dividends (all yield actuals). */
  incomeEarnedMinor: number;
  scale: number;
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
    rocPct2026EstimateMinor: number | null;
    rocPct2026ActualMinor: number | null;
    rocScale: number | null;
    lastPriceMinor: number | null;
    lastPriceScale: number | null;
    priceFreshness: string;
    marketValueMinor: number | null;
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
    endingBalanceMinor?: number | null;
    endingBalanceKnown?: boolean;
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

export type TrendsWeekPoint = {
  periodEnd: string;
  profitMinor: number;
  monthlyDivsMinor: number;
  divDeltaMinor: number;
  fidelityTotalMinor: number;
  schwabTotalMinor: number;
  fidSchCombinedMinor: number;
  wkToWkChangeMinor: number;
  incomeCashMinor: number;
  acct9CashMinor: number;
  acct9EtfProxyMinor: number;
  totalCashMinor: number;
  carBalanceMinor: number | null;
  incomeBalanceMinor: number | null;
  healthBalanceMinor: number | null;
  rothBalanceMinor: number | null;
  speculationBalanceMinor: number | null;
  closed?: boolean;
  scale: number;
};

export type TrendsGet = {
  points: Array<{ occurredOn: string; amountMinor: number }>;
  totalMinor: number;
  weeks?: TrendsWeekPoint[];
  note?: string;
  overview?: {
    fidSchCombinedMinor?: number | null;
    wkToWkChangeMinor?: number | null;
    profitMinor?: number | null;
    monthlyDivsMinor?: number | null;
    divDeltaMinor?: number | null;
    totalCashMinor?: number | null;
    scale: number;
  };
  distributions?: {
    grossMinor: number;
    lines: Array<{
      activityType: string;
      accountName: string;
      amountMinor: number;
      occurredOn: string;
      scale: number;
    }>;
    scale: number;
  };
  taxMonitor?: {
    federalWithholdingMinor: number;
    projectedLiabilityMinor?: number | null;
    gapMinor?: number | null;
    warning: boolean;
    acaThresholdMinor?: number | null;
    acaCoverageYear?: number | null;
    note: string;
    scale: number;
  };
  missingRequired?: string[];
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

export type AccountPositionTotal = {
  accountId: string;
  accountName: string;
  symbolCount: number;
  openLotCount: number;
  openPerformanceMinor: number;
  openTaxMinor: number;
  marketValueMinor: number | null;
  scale: number;
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
  accountTotals: AccountPositionTotal[];
  symbolCount: number;
  accountCount: number;
  openLotCount: number;
  openPerformanceMinor: number;
  openTaxMinor: number;
  marketValueMinor: number | null;
  marketValueComplete: boolean;
  scale: number;
};

export type PositionMasterGet = {
  rows: Array<{
    securityId: string;
    symbol: string;
    name: string;
    riskTier: string;
    provider: string;
    underlying: string;
    paymentFrequency: string;
    divType: string;
    needsRocResearch: boolean;
    isActive: boolean;
    notes: string;
    taxHandling: string;
    declarationWeekday: string;
    exdateWeekday: string;
    paydayWeekday: string;
    remainingQuantityMinor: number;
    quantityScale: number;
    remainingPerformanceMinor: number;
    remainingTaxMinor: number;
    unitCostMinor: number | null;
    lastPriceMinor: number | null;
    lastPriceScale: number | null;
    priceFreshness: string;
    priceDerivedValid: boolean;
    marketValueMinor: number | null;
    allocationBps: number | null;
    planKnown: boolean;
    planPerShareMinor: number;
    planScale: number;
    annualPlanMinor: number | null;
    planYocBps: number | null;
    planFwdYieldBps: number | null;
    mostCurrentFwdYieldBps: number | null;
    unrealizedPnlBps: number | null;
    rocPct2024ActualMinor: number | null;
    rocPct2025ActualMinor: number | null;
    rocPct2026EstimateMinor: number | null;
    rocPct2026ActualMinor: number | null;
    rocScale: number | null;
    declarationCount: number;
    periodDated: boolean;
    bearPriceReturnBps: number | null;
    bearTotalReturnBps: number | null;
    bullTotalReturnBps: number | null;
    completeness: string;
    scale: number;
    totalDistributionsReceivedMinor?: number | null;
    rocDistributionsMinor?: number | null;
    costRecoveryBps?: number | null;
    distributionsScope?: string;
    evidence?: {
      incomeReliability: number | null;
      downsideResilience: number | null;
      recoveryUpside: number | null;
      navPersistence: number | null;
      diversification: number | null;
      dataConfidence: number;
      knownComponents: number;
    } | null;
    bearCushionBps?: number | null;
    bullPriceReturnBps?: number | null;
    bullCushionBps?: number | null;
    carMarketValueMinor?: number | null;
    carShareOfSymbolBps?: number | null;
    carShareOfDataBps?: number | null;
    rocResearchStatus?: string;
    declarationFreshness?: string;
  }>;
  dataMarketValueMinor: number | null;
  marketValueComplete: boolean;
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
  asOfAt?: string | null;
};

export type LookthroughHolding = {
  ticker: string;
  weightBps?: number | null;
};

export type LookthroughSectorWeight = {
  label: string;
  weightBps?: number | null;
};

/** Owner-confirmed look-through research. Unknown status is not 0%. */
export type LookthroughResearch = {
  themeStrategy?: string;
  primaryRiskDriver?: string;
  concentrationStatus?: string;
  topHoldings?: LookthroughHolding[];
  sectorWeights?: LookthroughSectorWeight[];
  concentrationAsOf?: string | null;
  volProxy?: string;
  taxCharacter?: string;
  riskTierSuggestion?: string;
  riskTierSuggestionReason?: string;
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
  remainingTaxMinor: number;
  rocPct2024ActualMinor: number | null;
  rocPct2025ActualMinor: number | null;
  rocPct2026EstimateMinor: number | null;
  rocPct2026ActualMinor: number | null;
  rocScale: number | null;
  notes: string;
  divType: string;
  isActive?: boolean;
  needsRocResearch?: boolean;
  taxHandling?: string;
  declarationWeekday?: string;
  exdateWeekday?: string;
  paydayWeekday?: string;
  unitCostMinor?: number | null;
  planFwdYieldBps?: number | null;
  mostCurrentFwdYieldBps?: number | null;
  unrealizedPnlBps?: number | null;
  price: CurrentPriceGet;
  review: PlanReviewGet;
  template: {
    priceSource: string;
    sourceSymbol: string;
    declarationSource: string;
    lookbackCount: number;
    paymentSource: string;
    sourceUrl?: string;
    calendarPolicy?: string;
    lastRunAt?: string;
    lastRunOk?: boolean | null;
    lastRunMessage?: string;
    lastContentHash?: string;
    collectorEnabled?: boolean;
    /** ISO date; optional — only used when paid decls are under 12. */
    inceptionOn?: string;
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
  declarations: Array<{
    paymentPeriod: string;
    amountPerShareMinor: number | null;
    amountScale: number;
    source: string;
  }>;
  declarationCount: number;
  firstLotComplete: boolean;
  marketValueMinor: number | null;
  unrealizedPerformanceMinor: number | null;
  annualPlanMinor: number | null;
  planYocBps: number | null;
  mostCurrentVsPlanBps: number | null;
  periods: Array<{
    periodId: string;
    kind: string;
    name: string;
    startOn: string;
    endOn: string;
    benchmarkSymbol: string;
    selectionReason: string;
    method: string;
    status: string;
    recordedAt: string;
  }>;
  results: Array<{
    resultId: string;
    securityId: string;
    periodId: string;
    priceReturnBps: number | null;
    totalReturnBps: number | null;
    cushionBps: number | null;
    maxDrawdownBps: number | null;
    recoveryRatioBps: number | null;
    recoveryDays: number | null;
    incomeReliabilityBps: number | null;
    bearRelativeBps: number | null;
    downsideCaptureBps: number | null;
    upsideCaptureBps: number | null;
    completeness: string;
    source: string;
    calculatedAt: string;
  }>;
  evidence: {
    incomeReliability: number | null;
    downsideResilience: number | null;
    recoveryUpside: number | null;
    navPersistence: number | null;
    diversification: number | null;
    dataConfidence: number;
    knownComponents: number;
  } | null;
  suggestion: {
    suggestedTier: string;
    ruleset: string;
    reason: string;
    complete: boolean;
  } | null;
  totalDistributionsReceivedMinor?: number | null;
  rocDistributionsMinor?: number | null;
  costRecoveryBps?: number | null;
  distributionsScope?: string;
  carMarketValueMinor?: number | null;
  carShareOfSymbolBps?: number | null;
  carShareOfDataBps?: number | null;
  rocResearchStatus?: string;
  declarationFreshness?: string;
  rocEstimateMethod?: string;
  rocEstimateSourceUrl?: string;
  rocEstimateAsOf?: string;
  rocEstimateEstablishedHow?: string;
  rocResearchCompletedAt?: string | null;
  lookthrough?: LookthroughResearch;
  scale: number;
};

export type PositionDetailsCoverageRow = {
  securityId: string;
  symbol: string;
  active: boolean;
  openLots: boolean;
  recordedStatus: string;
  rocResearchStatus: string;
  declarationFreshness: string;
  distributionsScope: string;
  declarationSource?: string;
  lastRunAt?: string;
  lastRunOk?: boolean | null;
  lastRunMessage?: string;
};

export type PositionDetailsCoverageGet = {
  rows: PositionDetailsCoverageRow[];
};

export type Div1ComplianceSummaryRow = {
  securityId: string;
  symbol: string;
  futurePayDatesQty: number;
  priorDeclarationsQty: number;
  currentDeclarationAmountMinor: number | null;
  currentDeclarationAmountScale: number | null;
  currentDeclarationDate: string;
  lastRunOk: boolean | null;
  requiredPaid: number;
};

export type Div1ComplianceSummaryGet = {
  rows: Div1ComplianceSummaryRow[];
  asOfDate: string;
};

export type RocResearchGet = {
  securityId: string | null;
  rocPctMinor: number | null;
  scale: number;
  source: string;
  complete: boolean;
  reason: string;
  candidates: Array<{
    rocPctMinor: number | null;
    scale: number;
    taxYear: string;
    source: string;
    sourceUrl?: string;
    method?: string;
    asOf?: string;
    kind?: string;
    establishedHow?: string;
    ownerOverride?: boolean;
  }>;
  remainingPeriods: number | null;
  remainingTotalMinor: number | null;
  remainingOrdinaryMinor: number | null;
  remainingRocMinor: number | null;
  magiEligible: boolean;
  systemRocPctMinor?: number | null;
  sourceUrl?: string;
  method?: string;
  asOf?: string;
  kind?: string;
  establishedHow?: string;
  ownerOverride?: boolean;
  observations?: Array<{
    observationId: string;
    securityId: string;
    rocPctMinor: number | null;
    scale: number;
    taxYear: string;
    source: string;
    sourceUrl: string;
    method: string;
    asOf: string;
    kind: string;
    establishedHow: string;
    ownerOverride: boolean;
    recordedAt: string;
  }>;
};

/** Check-for-update status. Fail closed: never applied, never posted. */
export type RemainingYearIncomeGet = {
  securityId: string;
  asOfDate: string;
  known: boolean;
  provenance: string;
  paymentFrequency: string;
  latestDeclarationPeriod: string | null;
  remainingPeriods: number | null;
  yearToGoMinor: number | null;
  thisLotYearToGoMinor: number | null;
  positionAfterYearToGoMinor: number | null;
  existingQuantityMinor: number;
  thisLotQuantityMinor: number | null;
  hypothetical: boolean;
  planKnown: boolean;
  payments: Array<{
    payOn: string;
    originalPayOn: string;
    month: string;
    cashMinor: number | null;
    thisLotCashMinor: number | null;
    positionAfterCashMinor: number | null;
    ownerOverride: boolean;
    dateProvenance?: string;
  }>;
  months: Array<{
    month: string;
    cashMinor: number | null;
    thisLotCashMinor: number | null;
    positionAfterCashMinor: number | null;
  }>;
  orphanedOverrides?: Array<{
    originalPayOn: string;
    payOn: string;
  }>;
  calendarPolicy?: string;
  scale: number;
};

export type UpdaterCheckGet = {
  applied: boolean;
  posted: boolean;
  status: string;
};
