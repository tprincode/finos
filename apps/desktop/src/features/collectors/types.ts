/** Collectors fleet row — same shape CollectorSetGet returns. */
export type CollectorSetItem = {
  securityId: string;
  symbol: string;
  provider: string;
  divType: string;
  paymentFrequency?: string;
  declarationSource: string;
  priceSource: string;
  sourceUrl?: string;
  calendarPolicy?: string;
  collectorEnabled: boolean;
  lastRunAt?: string;
  lastRunOk?: boolean | null;
  lastRunMessage?: string;
  lastContentHash?: string;
  rocSourceUrl?: string;
  openLots: boolean;
  inceptionOn?: string;
  complete?: boolean;
  gaps?: string[];
  fillGapsProviderBlank?: boolean;
  fillGapsFrequencyBlank?: boolean;
  fillGapsDivTypeBlank?: boolean;
  fillGapsRocBlank?: boolean;
  paidDeclarationCount?: number;
  remainingPlanned?: number | null;
  remainingExpected?: number | null;
  successfulRunCount?: number;
  failureCount?: number;
  openTicketCount?: number;
  latestTicketField?: string;
  lastPriceAsOf?: string;
  lastPriceFreshness?: string;
  underlying?: string;
  rocEstimateMinor?: number | null;
  rocScale?: number;
  rocTaxYear?: string;
  openLotCount?: number;
  lastPayableOn?: string;
  declarationWeekday?: string;
};

export type Div1ComplianceRow = {
  securityId: string;
  symbol: string;
  futurePayDatesQty: number;
  futurePayDates?: string[];
  priorDeclarationsQty: number;
  currentDeclarationAmountMinor: number | null;
  currentDeclarationAmountScale: number | null;
  currentDeclarationDate: string;
  lastRunOk?: boolean | null;
  requiredPaid: number;
};

export type CollectorStats = {
  assigned: number;
  enabled: number;
  ranToday: number;
  stillMiss: number;
  hadMissToday: number;
  missToday: number;
  unchangedToday: number;
  cashPar: number;
  priceCurrent: number;
  priceStale: number;
  openExceptions: number;
  ranOutsideFleet?: string[];
  asOfDate: string;
  failCount?: number;
  failTicketCount?: number;
  failTicketParity?: boolean;
};

export type RetrieveRunRow = {
  runId: string;
  securityId: string;
  kind: string;
  requestedAt: string;
  ok: boolean;
  code?: string;
  message?: string;
  attempted: number;
  recorded: number;
  skipped: number;
  unchanged: number;
  payloadJson?: string;
};

export type CollectorRunProgress = {
  running: boolean;
  total: number;
  current: number;
  symbol: string;
  ok: number;
  miss: number;
  lines: string[];
  finishedAt?: string;
};

export type ResearchActivity = {
  running: boolean;
  label: string;
  step: number;
  total: number;
  resultLine?: string | null;
};
