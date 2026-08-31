import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  FINANCE_CLIENT_CONTRACT_VERSION,
  type AccountListItem,
  type CalculatorGet,
  type CurrentPriceGet,
  type DashboardBurndownGet,
  type DividendGet,
  type ExceptionRecord,
  type HandoffStatus,
  type HoldingsGet,
  type DataSummaryGet,
  type IncomePlanWeekGet,
  type InvestmentGet,
  type LookthroughResearch,
  type PositionDetailsGet,
  type PositionDetailsCoverageGet,
  type PositionMasterGet,
  type PlanReviewGet,
  type RemainingYearIncomeGet,
  type RocResearchGet,
  type SecurityListItem,
  type TrendsGet,
} from "@finos/app-contracts";
import { invoke } from "@tauri-apps/api/core";
import { check } from "@tauri-apps/plugin-updater";
import { LocalTauriFinanceClient } from "./financeClient";
import { TrendsChartsPanel } from "./TrendsCharts";
import { DeclarationPaymentsChart } from "./DeclarationPaymentsChart";
import { TrendsCapturePanel, type TrendsWeekCapture } from "./TrendsCapture";
import {
  CalculatorPanel,
  DashboardBurndownPanel,
  ExceptionList,
  HoldingsPanel,
  IncomePlanWeekPanel,
  PositionDetailsTable,
  PositionMasterTable,
  SymbolLotsTable,
  formatCount,
  formatScaled,
  formatUsd,
  formatPercentScaled,
} from "@finos/ui-components";
import "./App.css";

const RISK_TIERS = ["Foundation", "Core", "Risk On"];
const OWNER_RISK_CHOICES = ["Foundation", "Core", "Risk On", "Undecided"];

const emptyLookthrough = (): LookthroughResearch => ({
  themeStrategy: "",
  primaryRiskDriver: "",
  concentrationStatus: "unknown",
  topHoldings: [],
  sectorWeights: [],
  concentrationAsOf: null,
  volProxy: "",
  taxCharacter: "",
  riskTierSuggestion: "",
  riskTierSuggestionReason: "",
});

function mergeLookthrough(raw?: LookthroughResearch | null): LookthroughResearch {
  return {
    ...emptyLookthrough(),
    ...raw,
    topHoldings: raw?.topHoldings ?? [],
    sectorWeights: raw?.sectorWeights ?? [],
  };
}

function concentrationSummary(lt: LookthroughResearch): string {
  const names = (lt.topHoldings ?? []).map((h) =>
    h.weightBps == null ? h.ticker : `${h.ticker} ${(h.weightBps / 100).toFixed(2)}%`,
  );
  const sectors = (lt.sectorWeights ?? []).map((s) =>
    s.weightBps == null ? s.label : `${s.label} ${(s.weightBps / 100).toFixed(2)}%`,
  );
  const bits = [...names, ...sectors];
  if (bits.length > 0) return bits.join(", ");
  return lt.concentrationStatus?.trim() || "unknown";
}

const WEEKDAYS = [
  "",
  "Sunday",
  "Monday",
  "Tuesday",
  "Wednesday",
  "Thursday",
  "Friday",
  "Saturday",
];
const TAX_HANDLING = ["", "Ordinary", "Qualified", "Non Qualified"];

const PLAN_REASONS = [
  "Match Most Current",
  "Conservative vs Most Current",
  "Match Avg 6 (owner typed)",
  "Incomplete history — owner estimate",
  "Continue prior Plan",
  "Other",
];

const INCOMPLETE_REASONS = [
  "Fewer than 6 observations",
  "New issue / short history",
  "Retrieve miss — owner paste incomplete",
];

function scaledDollars(minor: number, scale: number): string {
  return (minor / 10 ** scale).toFixed(Math.max(2, scale));
}

type PdDraft = {
  name: string;
  provider: string;
  risk: string;
  freq: string;
  underlying: string;
  lookthrough: LookthroughResearch;
  plan: string;
  planReason: string;
  incomplete: string;
  roc2024: string;
  roc2025: string;
  roc2026e: string;
  roc2026a: string;
  notes: string;
  divType: string;
  needsRoc: boolean;
  isActive: boolean;
  taxHandling: string;
  declarationWeekday: string;
  exdateWeekday: string;
  paydayWeekday: string;
  priceSource: string;
  sourceSymbol: string;
  declarationSource: string;
  lookbackCount: string;
  sourceUrl: string;
  calendarPolicy: string;
};

type PdPeriodDraft = {
  kind: string;
  name: string;
  startOn: string;
  endOn: string;
  benchmark: string;
};

const emptyPeriod = (): PdPeriodDraft => ({
  kind: "",
  name: "",
  startOn: "",
  endOn: "",
  benchmark: "",
});

type WizEditDraft = {
  symbol: string;
  name: string;
  provider: string;
  underlying: string;
  lookthrough: LookthroughResearch;
  risk: string;
  freq: string;
  price: string;
  priceSource: string;
  declSource: string;
  lookback: string;
  sourceUrl: string;
  calendarPolicy: string;
  declAmounts: string;
  plan: string;
  planReason: string;
  incomplete: string;
  accountId: string;
  qty: string;
  cost: string;
  rocPct: string;
  bullStart: string;
  bullEnd: string;
  bearStart: string;
  bearEnd: string;
  remainingPays: string;
  nextPayDate: string;
};

const emptyWizEdit = (): WizEditDraft => ({
  symbol: "",
  name: "",
  provider: "",
  underlying: "",
  lookthrough: emptyLookthrough(),
  risk: "",
  freq: "",
  price: "",
  priceSource: "public",
  declSource: "",
  lookback: "12",
  sourceUrl: "",
  calendarPolicy: "",
  declAmounts: "",
  plan: "",
  planReason: "",
  incomplete: "",
  accountId: "",
  qty: "1",
  cost: "",
  rocPct: "",
  bullStart: "",
  bullEnd: "",
  bearStart: "",
  bearEnd: "",
  remainingPays: "|next:",
  nextPayDate: "",
});

type AddLotDraft = {
  securityId: string;
  accountId: string;
  openedOn: string;
  qty: string;
  cost: string;
  taxCost: string;
  taxCostDifferent: boolean;
  origin: string;
};

const LOT_ORIGINS = ["purchase", "drip", "transfer"] as const;

const emptyAddLot = (): AddLotDraft => ({
  securityId: "",
  accountId: "",
  openedOn: new Date().toISOString().slice(0, 10),
  qty: "1",
  cost: "",
  taxCost: "",
  taxCostDifferent: false,
  origin: "purchase",
});

function remainingPaysKey(
  pays: Array<{ originalPayOn: string; payOn: string }>,
  nextPay: string,
): string {
  return `${pays.map((p) => `${p.originalPayOn}->${p.payOn}`).join("|")}|next:${nextPay}`;
}

function parseRemainingPaysKey(key: string): {
  pays: Array<{ originalPayOn: string; payOn: string }>;
  nextPay: string;
} {
  const marker = "|next:";
  const nextIdx = key.lastIndexOf(marker);
  const nextPay = nextIdx >= 0 ? key.slice(nextIdx + marker.length) : "";
  const body = nextIdx >= 0 ? key.slice(0, nextIdx) : key;
  const pays = body
    .split("|")
    .filter((part) => part.includes("->"))
    .map((part) => {
      const splitAt = part.indexOf("->");
      return {
        originalPayOn: part.slice(0, splitAt),
        payOn: part.slice(splitAt + 2),
      };
    });
  return { pays, nextPay };
}

const PERIOD_KINDS = ["Bull", "Bear", "Recovery", "Stress"];

function rocText(minor: number | null | undefined, scale: number | null | undefined): string {
  if (minor == null || scale == null) return "";
  return scaledDollars(minor, scale);
}

function rocMinor(raw: string): number | null {
  const trimmed = raw.trim();
  if (!trimmed) return null;
  const n = Number(trimmed);
  return Number.isFinite(n) ? Math.round(n * 100) : null;
}

function draftFromInvestment(body: InvestmentGet): PdDraft {
  return {
    name: body.name,
    provider: body.provider,
    risk: body.riskTier,
    freq: body.paymentFrequency,
    underlying: body.underlying,
    lookthrough: mergeLookthrough(body.lookthrough),
    plan: body.planKnown ? scaledDollars(body.planPerShareMinor, body.planScale) : "",
    planReason: body.planReason,
    incomplete: "",
    roc2024: rocText(body.rocPct2024ActualMinor, body.rocScale),
    roc2025: rocText(body.rocPct2025ActualMinor, body.rocScale),
    roc2026e: rocText(body.rocPct2026EstimateMinor, body.rocScale),
    roc2026a: rocText(body.rocPct2026ActualMinor, body.rocScale),
    notes: body.notes ?? "",
    divType: body.divType ?? "",
    needsRoc: Boolean(body.needsRocResearch),
    isActive: body.isActive !== false,
    taxHandling: body.taxHandling ?? "",
    declarationWeekday: body.declarationWeekday ?? "",
    exdateWeekday: body.exdateWeekday ?? "",
    paydayWeekday: body.paydayWeekday ?? "",
    priceSource: body.template?.priceSource ?? "public",
    sourceSymbol: body.template?.sourceSymbol ?? body.symbol,
    declarationSource: body.template?.declarationSource ?? "",
    lookbackCount: String(body.template?.lookbackCount ?? 12),
    sourceUrl: body.template?.sourceUrl ?? "",
    calendarPolicy: body.template?.calendarPolicy ?? "",
  };
}

function formatBps(bps: number | null | undefined): string {
  if (bps == null) return "unknown";
  return `${(bps / 100).toFixed(2)}%`;
}

/** Most Current vs Plan as Above / Equal / Below (spreadsheet control signal). */
function mostCurrentVsPlanFace(bps: number | null | undefined): string {
  if (bps == null) return "unknown";
  if (bps === 0) return "Equal to Plan";
  const pct = `${(Math.abs(bps) / 100).toFixed(2)}%`;
  return bps > 0 ? `Above Plan ${pct}` : `Below Plan ${pct}`;
}

/** Hover/aria formula hint for Plan and yields metrics. */
function metricHint(label: string, formula: string): { title: string; "aria-label": string } {
  const text = `${label}: ${formula}`;
  return { title: text, "aria-label": text };
}

function priceUpdatedOn(price: CurrentPriceGet | null | undefined): string {
  const raw = price?.asOfAt?.trim();
  if (!raw) return "";
  return raw.length >= 10 ? raw.slice(0, 10) : raw;
}

function formatHubPrice(price: CurrentPriceGet | null | undefined, scale = 2): string {
  if (!price?.priceDerivedValid || price.priceMinor == null || price.priceMinor <= 0) {
    return "unknown";
  }
  const amount = formatUsd(price.priceMinor, price.scale ?? scale);
  const updated = priceUpdatedOn(price);
  return updated ? `${amount} · updated ${updated}` : amount;
}

function rocResearchLabel(inv: InvestmentGet): string {
  const status = inv.rocResearchStatus || "not-in-scope";
  const completed = inv.rocResearchCompletedAt?.trim();
  if (status === "complete" && completed) {
    const day = completed.length >= 10 ? completed.slice(0, 10) : completed;
    return `complete · ${day}`;
  }
  return status;
}

function heldInCalendarYear(
  lots: Array<{ openedOn?: string }> | undefined,
  year: number,
): boolean {
  const y = String(year).padStart(4, "0");
  return (lots ?? []).some((lot) => {
    const stamp = (lot.openedOn || "").trim();
    return stamp.length >= 4 && stamp.slice(0, 4) <= y;
  });
}

/** 1099 actual for year Y is only researchable in Y+1. A year not held is N/A. */
function rocActualUnavailable(
  lots: Array<{ openedOn?: string }> | undefined,
  year: number,
  asOf: string,
): string | null {
  if (!heldInCalendarYear(lots, year)) {
    return `N/A — not held in ${year}`;
  }
  const asOfYear = Number.parseInt((asOf || "").slice(0, 4), 10);
  if (Number.isFinite(asOfYear) && asOfYear < year + 1) {
    return `N/A — 1099 not researchable until ${year + 1}`;
  }
  return null;
}

function rocResearchUpdated(inv: InvestmentGet): string {
  const completed = inv.rocResearchCompletedAt?.trim();
  if (completed) {
    return completed.length >= 10 ? completed.slice(0, 10) : completed;
  }
  const asOf = inv.rocEstimateAsOf?.trim();
  if (asOf) {
    return asOf.length >= 10 ? asOf.slice(0, 10) : asOf;
  }
  return "—";
}

/** Owner-facing calendar policy — never say "walk" on the hub face. */
function calendarPolicyLabel(policy: string | null | undefined): string {
  const p = (policy || "").trim();
  if (p === "issuer_calendar") return "Issuer published dates";
  if (p === "derived_walk") return "Cadence from last pay";
  if (p === "none") return "Does not pay";
  return p || "—";
}

function dateProvenanceLabel(provenance: string | null | undefined): string {
  const p = (provenance || "").trim();
  if (!p) return "—";
  if (p === "issuer_calendar" || p.includes("issuer_calendar")) {
    return "Issuer published date";
  }
  if (p === "derived_walk" || p.includes("derived_walk") || /\bwalk\b/i.test(p)) {
    return "Cadence from last pay";
  }
  if (p === "owner_override" || p.includes("owner")) return "Owner override";
  return p;
}

const DECLARATION_LOOKBACK_TARGET = 12;

/** Mirror of financial-domain expected_declaration_lookback for hub labels. */
function expectedDeclarationLookback(
  inceptionOn: string | null | undefined,
  asOf: string,
  paymentFrequency: string | null | undefined,
): number {
  const freq = (paymentFrequency || "").trim().toLowerCase();
  const periods =
    freq === "weekly" || freq === "52"
      ? 52
      : freq === "monthly" || freq === "12"
        ? 12
        : freq === "quarterly" || freq === "4"
          ? 4
          : 0;
  if (!periods) return DECLARATION_LOOKBACK_TARGET;
  const inc = (inceptionOn || "").trim();
  if (!/^\d{4}-\d{2}-\d{2}$/.test(inc) || !/^\d{4}-\d{2}-\d{2}$/.test(asOf)) {
    return DECLARATION_LOOKBACK_TARGET;
  }
  const [iy, im, id] = inc.split("-").map(Number);
  const [ay, am, ad] = asOf.split("-").map(Number);
  if (ay < iy || (ay === iy && am < im) || (ay === iy && am === im && ad <= id)) {
    return 0;
  }
  if (periods === 52) {
    const ms =
      Date.UTC(ay, am - 1, ad) - Date.UTC(iy, im - 1, id);
    return Math.min(DECLARATION_LOOKBACK_TARGET, Math.floor(ms / 86_400_000 / 7));
  }
  let months = (ay - iy) * 12 + (am - im);
  if (ad < id) months -= 1;
  if (periods === 12) {
    return Math.min(DECLARATION_LOOKBACK_TARGET, Math.max(0, months));
  }
  return Math.min(DECLARATION_LOOKBACK_TARGET, Math.max(0, Math.floor(months / 3)));
}

function receivedByYear(
  actuals: Array<{ occurredOn: string; amountMinor: number }>,
): Array<{ year: string; amountMinor: number }> {
  const map = new Map<string, number>();
  for (const a of actuals) {
    const year = (a.occurredOn || "").slice(0, 4);
    if (!/^\d{4}$/.test(year)) continue;
    map.set(year, (map.get(year) ?? 0) + a.amountMinor);
  }
  return [...map.entries()]
    .sort((a, b) => b[0].localeCompare(a[0]))
    .map(([year, amountMinor]) => ({ year, amountMinor }));
}

const client = new LocalTauriFinanceClient();

type HealthView = {
  ok: boolean;
  status: string;
  contractVersion: string;
  error?: string;
};

type Screen =
  | "income-plan"
  | "calculator"
  | "dashboard"
  | "trends"
  | "holdings"
  | "import"
  | "settings"
  | "new-investment"
  | "add-lot"
  | "position-details"
  | "collectors";

type CollectorSetItem = {
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
  openLots: boolean;
  inceptionOn?: string;
};

type Div1ComplianceRow = {
  securityId: string;
  symbol: string;
  futurePayDatesQty: number;
  priorDeclarationsQty: number;
  currentDeclarationAmountMinor: number | null;
  currentDeclarationAmountScale: number | null;
  currentDeclarationDate: string;
  lastRunOk?: boolean | null;
  requiredPaid: number;
};

function collectorRetrieveNeedsRun(row: CollectorSetItem, asOfDate: string): boolean {
  if (!row.collectorEnabled || !row.declarationSource.trim()) {
    return false;
  }
  if (row.lastRunOk !== true) {
    return true;
  }
  const ran = (row.lastRunAt ?? "").trim();
  if (!ran) {
    return true;
  }
  return !ran.startsWith(asOfDate);
}

function collectorCommandBody(row: CollectorSetItem, forceRefresh: boolean) {
  return {
    securityId: row.securityId,
    symbol: row.symbol,
    declarationSource: row.declarationSource,
    sourceUrl: row.sourceUrl ?? "",
    divType: row.divType ?? "",
    lastContentHash: row.lastContentHash ?? "",
    lastRunOk: row.lastRunOk === true,
    lastRunAt: row.lastRunAt ?? "",
    paymentFrequency: row.paymentFrequency ?? "",
    inceptionOn: row.inceptionOn ?? "",
    forceRefresh,
  };
}

/** Fleet shows income names only — matches storage collector_symbol_pays. */
function collectorItemPays(row: CollectorSetItem): boolean {
  const sym = row.symbol.trim().toUpperCase();
  const div = (row.divType || "").trim().toUpperCase().replace(/\s+/g, "-");
  if (div === "CASH" || sym === "SPAXX" || sym === "FDRXX" || sym === "SWVXX") {
    return true;
  }
  if (div === "DIV-1" || div === "DIV1") {
    return true;
  }
  const freq = (row.paymentFrequency || "").trim().toLowerCase();
  return (
    freq === "weekly" ||
    freq === "52" ||
    freq === "monthly" ||
    freq === "12" ||
    freq === "quarterly" ||
    freq === "4"
  );
}

type CollectorStats = {
  assigned: number;
  enabled: number;
  ranToday: number;
  missToday: number;
  unchangedToday: number;
  cashPar: number;
  priceCurrent: number;
  priceStale: number;
  openExceptions: number;
  asOfDate: string;
};

type RetrieveRunRow = {
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

function parseHandoff(bodyJson?: string): HandoffStatus | null {
  if (!bodyJson) return null;
  try {
    return JSON.parse(bodyJson) as HandoffStatus;
  } catch {
    return null;
  }
}

function shiftIso(date: string, days: number): string {
  const d = new Date(`${date}T12:00:00`);
  d.setDate(d.getDate() + days);
  return d.toISOString().slice(0, 10);
}

/** Saturday that starts the Sat–Fri Income Plan week containing `iso`. */
function saturdayOfWeek(iso: string): string {
  const d = new Date(`${iso}T12:00:00`);
  if (Number.isNaN(d.getTime())) {
    return iso;
  }
  const daysSinceSaturday = (d.getDay() + 1) % 7;
  d.setDate(d.getDate() - daysSinceSaturday);
  return d.toISOString().slice(0, 10);
}

function incomePlanWeekChoices(asOf: string, latestActualOn?: string | null): string[] {
  const today = new Date().toISOString().slice(0, 10);
  const anchor = saturdayOfWeek(asOf || latestActualOn || today);
  const todaySat = saturdayOfWeek(today);
  const latestSat = latestActualOn ? saturdayOfWeek(latestActualOn) : anchor;
  const start = [anchor, todaySat, latestSat].reduce((min, sat) =>
    sat < min ? sat : min,
  );
  const end = [anchor, todaySat, shiftIso(todaySat, 16 * 7)].reduce((max, sat) =>
    sat > max ? sat : max,
  );
  const pastStart = shiftIso(start, -52 * 7);
  const weeks: string[] = [];
  for (let sat = pastStart; sat <= end; sat = shiftIso(sat, 7)) {
    weeks.push(sat);
  }
  if (!weeks.includes(anchor)) {
    weeks.push(anchor);
    weeks.sort();
  }
  return weeks;
}

function incomePlanWeekOptionLabel(saturday: string, thisWeekSaturday: string): string {
  const friday = shiftIso(saturday, 6);
  const mark = saturday === thisWeekSaturday ? " · this week" : "";
  return `Sat ${saturday} – Fri ${friday}${mark}`;
}

type ManualDividendRow = {
  id: string;
  accountId: string;
  ticker: string;
  occurredOn: string;
  amount: string;
};

let manualDividendSeq = 0;

type CaptureProcess = {
  running: boolean;
  title: string;
  current: number;
  total: number;
  detail: string;
  lines: string[];
  posted: number;
  skipped: number;
  errors: number;
};

function blankManualDividendRow(): ManualDividendRow {
  manualDividendSeq += 1;
  return {
    id: `md-${manualDividendSeq}`,
    accountId: "",
    ticker: "",
    occurredOn: "",
    amount: "",
  };
}

export default function App() {
  const [screen, setScreen] = useState<Screen>("income-plan");
  const [health, setHealth] = useState<HealthView | null>(null);
  const [handoff, setHandoff] = useState<HandoffStatus | null>(null);
  const [handoffError, setHandoffError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [actionMessage, setActionMessage] = useState<string | null>(null);
  const [deviceName, setDeviceName] = useState("");
  const [asOfDate, setAsOfDate] = useState("");
  const [incomeWeek, setIncomeWeek] = useState<IncomePlanWeekGet | null>(null);
  const [burndown, setBurndown] = useState<DashboardBurndownGet | null>(null);
  const [trends, setTrends] = useState<TrendsGet | null>(null);
  const [trendsError, setTrendsError] = useState<string | null>(null);
  const [trendsCapture, setTrendsCapture] = useState<TrendsWeekCapture | null>(null);
  const [holdings, setHoldings] = useState<HoldingsGet | null>(null);
  const [calculator, setCalculator] = useState<CalculatorGet | null>(null);
  const [summary, setSummary] = useState<DataSummaryGet | null>(null);
  const [dividendLifetime, setDividendLifetime] = useState<DividendGet | null>(null);
  const [exceptions, setExceptions] = useState<ExceptionRecord[]>([]);
  const [positionFocusPanel, setPositionFocusPanel] = useState<
    "lots" | "income" | "declarations" | "ledger" | ""
  >("");
  const [pdLedger, setPdLedger] = useState<DividendGet | null>(null);
  const [pendingBatchId, setPendingBatchId] = useState<string | null>(null);
  const [pendingBatchStatus, setPendingBatchStatus] = useState<string>("none");
  const [manualDividendRows, setManualDividendRows] = useState<ManualDividendRow[]>(() => [
    blankManualDividendRow(),
    blankManualDividendRow(),
    blankManualDividendRow(),
  ]);
  const [captureProcess, setCaptureProcess] = useState<CaptureProcess | null>(null);
  const [holdingsFilter, setHoldingsFilter] = useState("");
  const [lotId, setLotId] = useState("");
  const [assignActivityId, setAssignActivityId] = useState("");
  const [assignQty, setAssignQty] = useState("1");
  const [updateStatus, setUpdateStatus] = useState("not checked");
  const [drillAccount, setDrillAccount] = useState<string | null>(null);
  const [accounts, setAccounts] = useState<AccountListItem[]>([]);
  const [securities, setSecurities] = useState<SecurityListItem[]>([]);
  const [wizSymbol, setWizSymbol] = useState("");
  const [wizName, setWizName] = useState("");
  const [wizProvider, setWizProvider] = useState("");
  const [wizUnderlying, setWizUnderlying] = useState("");
  const [wizLookthrough, setWizLookthrough] = useState<LookthroughResearch>(emptyLookthrough);
  const [wizRisk, setWizRisk] = useState("");
  const [wizFreq, setWizFreq] = useState("");
  const [wizSecurityId, setWizSecurityId] = useState("");
  const [wizPrice, setWizPrice] = useState("");
  const [wizPriceSource, setWizPriceSource] = useState("public");
  const [wizDeclSource, setWizDeclSource] = useState("");
  const [wizLookback, setWizLookback] = useState("12");
  const [, setWizAnalytics] = useState<{
    htmlReturned?: boolean;
    fundPage?: boolean;
    tableOnGet?: boolean;
    tableRowCount?: number;
    jsLikely?: boolean;
  } | null>(null);
  const [, setWizFutureStrategy] = useState("");
  const [wizDeclAmounts, setWizDeclAmounts] = useState("");
  const [, setWizAttempts] = useState<
    Array<{ vendor: string; url: string; found: boolean; note: string }>
  >([]);
  const [, setWizMiss] = useState("");
  const [wizSourceUrl, setWizSourceUrl] = useState("");
  const [wizCalendarPolicy, setWizCalendarPolicy] = useState("");
  const [wizUpcomingPays, setWizUpcomingPays] = useState<
    Array<{ payOn: string; amountPerShareMinor?: number | null; amountScale?: number }>
  >([]);
  const [wizPlan, setWizPlan] = useState("");
  const [wizPlanReason, setWizPlanReason] = useState("");
  const [wizIncomplete, setWizIncomplete] = useState("");
  const [wizAccountId, setWizAccountId] = useState("");
  const [wizQty, setWizQty] = useState("1");
  const [wizCost, setWizCost] = useState("");
  const [wizReview, setWizReview] = useState<PlanReviewGet | null>(null);
  const [wizPriceState, setWizPriceState] = useState<CurrentPriceGet | null>(null);
  const [wizDecls, setWizDecls] = useState<
    Array<{
      amountPerShareMinor: number;
      amountScale?: number;
      paymentPeriod?: string;
      source?: string;
    }>
  >([]);
  const [wizRetrieveNote, setWizRetrieveNote] = useState("");
  const [addLotSecurityId, setAddLotSecurityId] = useState("");
  const [addLotAccountId, setAddLotAccountId] = useState("");
  const [addLotOpenedOn, setAddLotOpenedOn] = useState(() =>
    new Date().toISOString().slice(0, 10),
  );
  const [addLotQty, setAddLotQty] = useState("1");
  const [addLotCost, setAddLotCost] = useState("");
  const [addLotTaxCost, setAddLotTaxCost] = useState("");
  const [addLotTaxDifferent, setAddLotTaxDifferent] = useState(false);
  const [addLotOrigin, setAddLotOrigin] = useState("purchase");
  const [addLotBaseline, setAddLotBaseline] = useState(() => JSON.stringify(emptyAddLot()));
  const [wizRemaining, setWizRemaining] = useState<RemainingYearIncomeGet | null>(null);
  const [wizPayDraft, setWizPayDraft] = useState<Array<{ originalPayOn: string; payOn: string }>>(
    [],
  );
  const [wizNextPay, setWizNextPay] = useState("");
  const [positionDetails, setPositionDetails] = useState<PositionDetailsGet | null>(null);
  const [positionMaster, setPositionMaster] = useState<PositionMasterGet | null>(null);
  const [issuerCoverage, setIssuerCoverage] = useState<PositionDetailsCoverageGet | null>(null);
  const [pdRemaining, setPdRemaining] = useState<RemainingYearIncomeGet | null>(null);
  const [positionSymbol, setPositionSymbol] = useState("");
  const [positionSymbolQuery, setPositionSymbolQuery] = useState("");
  const [positionSymbolOpen, setPositionSymbolOpen] = useState(false);
  const [investment, setInvestment] = useState<InvestmentGet | null>(null);
  const [, setWizPart1Stored] = useState(false);
  const [wizPlanStored, setWizPlanStored] = useState(false);
  const [, setWizLotStored] = useState(false);
  const [, setWizStep] = useState(1);
  const [wizRocPct, setWizRocPct] = useState("");
  const [wizRoc, setWizRoc] = useState<RocResearchGet | null>(null);
  const [wizBullStart, setWizBullStart] = useState("");
  const [wizBullEnd, setWizBullEnd] = useState("");
  const [wizBearStart, setWizBearStart] = useState("");
  const [wizBearEnd, setWizBearEnd] = useState("");
  const [wizBullStored, setWizBullStored] = useState(false);
  const [wizBearStored, setWizBearStored] = useState(false);
  const [wizBaseline, setWizBaseline] = useState(() => JSON.stringify(emptyWizEdit()));
  /** Process A: results panel after Research (symbol + distribution URL). */
  const [wizResearchDone, setWizResearchDone] = useState(false);
  /** Process A: owner Save → explicit completion screen (not a blank wizard). */
  const [wizProcessASaved, setWizProcessASaved] = useState(false);
  /** Shared long-action indicator for Complete research / Add Position / Fill gaps. */
  const [researchActivity, setResearchActivity] = useState<{
    running: boolean;
    step: number;
    total: number;
    label: string;
    resultLine: string | null;
  } | null>(null);
  /** Research notes overview (issuer + AiAnalyze draft). Suggestion only for tier. */
  const [researchNotes, setResearchNotes] = useState<{
    overview: string;
    suggestedTier: string;
    suggestedReason: string;
    source: string;
  } | null>(null);
  /** Post–Complete research owner risk; never auto-applied. */
  const [ownerRiskChoice, setOwnerRiskChoice] = useState("Undecided");
  const [addLotQuery, setAddLotQuery] = useState("");
  const [addLotSymbolOpen, setAddLotSymbolOpen] = useState(false);
  const [wizTierSuggestion, setWizTierSuggestion] = useState<{
    suggestedTier: string;
    ruleset: string;
    reason: string;
    complete: boolean;
  } | null>(null);
  /** Process A: advisory AI thesis (never auto-applies). */
  const [wizAiThesis, setWizAiThesis] = useState<string | null>(null);
  /** Process A: when ClassificationSuggest needs backtest dates. */
  const [wizBacktestNeeded, setWizBacktestNeeded] = useState(false);
  const [pdDraft, setPdDraft] = useState<PdDraft | null>(null);
  const [pdBaseline, setPdBaseline] = useState("");
  const [pdPeriod, setPdPeriod] = useState<PdPeriodDraft>(() => emptyPeriod());
  const [pdPeriodBaseline, setPdPeriodBaseline] = useState(() => JSON.stringify(emptyPeriod()));
  const [savedPeriodId, setSavedPeriodId] = useState("");
  const [lastPriceBusy, setLastPriceBusy] = useState(false);
  const lastPriceKickoff = useRef(false);
  const [collectorItems, setCollectorItems] = useState<CollectorSetItem[]>([]);
  const [collectorStats, setCollectorStats] = useState<CollectorStats | null>(null);
  const [collectorSymbol, setCollectorSymbol] = useState("");
  const [collectorRuns, setCollectorRuns] = useState<RetrieveRunRow[]>([]);
  const [collectorPayload, setCollectorPayload] = useState<Record<string, unknown> | null>(null);
  const [collectorPlan, setCollectorPlan] = useState<{
    known: boolean;
    perShareMinor: number;
    scale: number;
    reason: string;
  } | null>(null);
  const [collectorAction, setCollectorAction] = useState<string | null>(null);
  const [settingsTemplateDrafts, setSettingsTemplateDrafts] = useState<
    Record<
      string,
      {
        declarationSource: string;
        sourceUrl: string;
        calendarPolicy: string;
        lookbackCount: string;
        inceptionOn: string;
      }
    >
  >({});
  const [collectorRunProgress, setCollectorRunProgress] = useState<{
    running: boolean;
    total: number;
    current: number;
    symbol: string;
    ok: number;
    miss: number;
    lines: string[];
  } | null>(null);
  const [div1Compliance, setDiv1Compliance] = useState<Div1ComplianceRow[]>([]);

  const refreshCollectors = useCallback(async (asOf: string, symbol?: string) => {
    setBusy(true);
    setCollectorAction("Loading collector fleet…");
    try {
      const [setResult, statsResult, complianceResult] = await Promise.all([
        client.executeQuery("CollectorSetGet", {}),
        client.executeQuery("CollectorStatsGet", { asOfDate: asOf || "2026-08-26" }),
        client.executeQuery("Div1ComplianceSummaryGet", {}),
      ]);
      if (!setResult.ok) {
        const msg = `Collector fleet failed: ${setResult.errorCode ?? "error"}. Restart with npm run desktop so Rust is rebuilt.`;
        setCollectorAction(msg);
        setActionMessage(msg);
        return;
      }
      let items: CollectorSetItem[] = [];
      if (setResult.bodyJson) {
        try {
          const body = JSON.parse(setResult.bodyJson) as { items?: CollectorSetItem[] };
          const raw = Array.isArray(body.items) ? body.items : [];
          items = raw.filter(collectorItemPays);
          setCollectorItems(items);
          setSettingsTemplateDrafts((prev) => {
            const next = { ...prev };
            for (const row of items) {
              if (next[row.securityId]) continue;
              next[row.securityId] = {
                declarationSource: row.declarationSource || "",
                sourceUrl: row.sourceUrl ?? "",
                calendarPolicy: row.calendarPolicy ?? "",
                lookbackCount: String(DECLARATION_LOOKBACK_TARGET),
                inceptionOn: row.inceptionOn ?? "",
              };
            }
            return next;
          });
        } catch {
          setCollectorItems([]);
        }
      } else {
        setCollectorItems([]);
      }
      if (statsResult.ok && statsResult.bodyJson) {
        try {
          setCollectorStats(JSON.parse(statsResult.bodyJson) as CollectorStats);
        } catch {
          setCollectorStats(null);
        }
      }
      if (complianceResult.ok && complianceResult.bodyJson) {
        try {
          const body = JSON.parse(complianceResult.bodyJson) as {
            rows?: Div1ComplianceRow[];
          };
          setDiv1Compliance(Array.isArray(body.rows) ? body.rows : []);
        } catch {
          setDiv1Compliance([]);
        }
      } else {
        setDiv1Compliance([]);
      }
      const focus = symbol ?? collectorSymbol;
      if (!focus) {
        const msg = `Fleet loaded: ${formatCount(items.length)} paying symbols (non-payers excluded).`;
        setCollectorAction(msg);
        setActionMessage(msg);
        return;
      }
      const row = items.find((i) => i.symbol === focus);
      if (!row) {
        setCollectorSymbol("");
        setCollectorRuns([]);
        setCollectorPayload(null);
        setCollectorPlan(null);
        const msg = `Fleet loaded: ${formatCount(items.length)} paying symbols. ${focus} is not a payer — removed from list.`;
        setCollectorAction(msg);
        setActionMessage(msg);
        return;
      }
      setCollectorAction(`Loading ${focus} retrieve runs…`);
      const [runsResult, invResult] = await Promise.all([
        client.executeQuery("RetrieveRunList", {
          securityId: row.securityId,
          limit: 40,
        }),
        client.executeQuery("InvestmentGet", {
          securityId: row.securityId,
          asOfDate: asOf || "2026-08-26",
        }),
      ]);
      if (runsResult.ok && runsResult.bodyJson) {
        try {
          const body = JSON.parse(runsResult.bodyJson) as { runs?: RetrieveRunRow[] };
          const runs = Array.isArray(body.runs) ? body.runs : [];
          setCollectorRuns(runs);
          const declarationRun =
            runs.find((r) => r.kind === "declaration" || r.kind === "moneymarket") ??
            runs.find((r) => r.kind !== "price");
          const raw = declarationRun?.payloadJson ?? null;
          if (raw) {
            try {
              setCollectorPayload(JSON.parse(raw) as Record<string, unknown>);
            } catch {
              setCollectorPayload({ raw });
            }
          } else {
            setCollectorPayload(null);
          }
        } catch {
          setCollectorRuns([]);
          setCollectorPayload(null);
        }
      }
      if (invResult.ok && invResult.bodyJson) {
        try {
          const inv = JSON.parse(invResult.bodyJson) as {
            planKnown?: boolean;
            planPerShareMinor?: number;
            planScale?: number;
            planReason?: string;
            template?: { declarationSource?: string; lastRunOk?: boolean | null };
          };
          setCollectorPlan({
            known: !!inv.planKnown,
            perShareMinor: inv.planPerShareMinor ?? 0,
            scale: inv.planScale ?? 2,
            reason: inv.planReason ?? "",
          });
        } catch {
          setCollectorPlan(null);
        }
      } else {
        setCollectorPlan(null);
      }
      const lastOk = row.lastRunOk === true ? "ok" : row.lastRunOk === false ? "miss" : "never";
      const msg = `${focus}: last run ${lastOk}${
        row.declarationSource ? ` via ${row.declarationSource}` : ""
      }. Open Position Details for declarations and received. Fleet ${formatCount(items.length)} paying symbols.`;
      setCollectorAction(msg);
      setActionMessage(msg);
    } catch (err: unknown) {
      const msg = String(err);
      setCollectorAction(msg);
      setActionMessage(msg);
    } finally {
      setBusy(false);
    }
  }, [collectorSymbol]);

  const runCollectorTargets = async (
    targets: CollectorSetItem[],
    label: string,
    forceRefresh: boolean,
  ) => {
    if (targets.length === 0) {
      const msg = `No ${label.toLowerCase()} to run.`;
      setCollectorAction(msg);
      setActionMessage(msg);
      setCollectorRunProgress(null);
      return;
    }
    setBusy(true);
    setCollectorRunProgress({
      running: true,
      total: targets.length,
      current: 0,
      symbol: "",
      ok: 0,
      miss: 0,
      lines: [],
    });
    setCollectorAction(`Running ${formatCount(targets.length)} ${label}…`);
    setActionMessage(`Running ${formatCount(targets.length)} ${label}…`);
    let ok = 0;
    let miss = 0;
    const lines: string[] = [];
    try {
      const { flushSync } = await import("react-dom");
      for (let i = 0; i < targets.length; i++) {
        const row = targets[i];
        flushSync(() => {
          setCollectorRunProgress({
            running: true,
            total: targets.length,
            current: i + 1,
            symbol: row.symbol,
            ok,
            miss,
            lines: [...lines],
          });
          setCollectorAction(
            `Running ${formatCount(i + 1)} of ${formatCount(targets.length)}: ${row.symbol}…`,
          );
        });
        await new Promise<void>((resolve) => {
          window.setTimeout(() => resolve(), 0);
        });
        try {
          const result = await client.executeCommand(
            "CollectorRetrieve",
            collectorCommandBody(row, forceRefresh),
          );
          if (!result.ok) {
            miss += 1;
            lines.push(
              `${row.symbol}: fail ${result.errorCode ?? "error"}`,
            );
          } else {
            let recorded = 0;
            let skipped = 0;
            let unchanged = 0;
            let runOk = true;
            let message = "";
            if (result.bodyJson) {
              try {
                const body = JSON.parse(result.bodyJson) as {
                  recorded?: number;
                  skipped?: number;
                  unchanged?: number;
                  ok?: boolean;
                  message?: string;
                };
                recorded = body.recorded ?? 0;
                skipped = body.skipped ?? 0;
                unchanged = body.unchanged ?? 0;
                runOk = body.ok !== false;
                message = body.message ?? "";
              } catch {
                /* ignore */
              }
            }
            if (runOk) {
              ok += 1;
              lines.push(
                unchanged > 0
                  ? `${row.symbol}: unchanged (fresh today)`
                  : `${row.symbol}: ok — ${formatCount(recorded)} recorded, ${formatCount(skipped)} skipped`,
              );
            } else {
              miss += 1;
              lines.push(
                `${row.symbol}: miss${message ? ` — ${message}` : ""}`,
              );
            }
          }
        } catch (err: unknown) {
          miss += 1;
          lines.push(`${row.symbol}: ${String(err)}`);
        }
        if (lines.length > 40) {
          lines.splice(0, lines.length - 40);
        }
        flushSync(() => {
          setCollectorRunProgress({
            running: true,
            total: targets.length,
            current: i + 1,
            symbol: row.symbol,
            ok,
            miss,
            lines: [...lines],
          });
        });
      }
      const summary = `${label} finished: ${formatCount(ok)} ok, ${formatCount(miss)} miss of ${formatCount(targets.length)}. Reloading fleet…`;
      setCollectorAction(summary);
      setActionMessage(summary);
      await refreshCollectors(asOfDate);
      const done = `${label} finished: ${formatCount(ok)} ok, ${formatCount(miss)} miss of ${formatCount(targets.length)}.`;
      setCollectorAction(done);
      setActionMessage(done);
      setCollectorRunProgress({
        running: false,
        total: targets.length,
        current: targets.length,
        symbol: "",
        ok,
        miss,
        lines: [...lines],
      });
    } catch (err: unknown) {
      const msg = String(err);
      setCollectorAction(msg);
      setActionMessage(msg);
      setCollectorRunProgress((prev) =>
        prev ? { ...prev, running: false } : null,
      );
    } finally {
      setBusy(false);
    }
  };

  const runEnabledCollectors = async () => {
    const targets = collectorItems.filter(
      (row) => row.collectorEnabled && row.declarationSource.trim(),
    );
    if (targets.length === 0) {
      const msg =
        "No enabled collectors with an assigned source. Enable rows in the fleet first.";
      setCollectorAction(msg);
      setActionMessage(msg);
      setCollectorRunProgress(null);
      return;
    }
    await runCollectorTargets(targets, "Enabled collectors", false);
  };

  const runMissesOnlyCollectors = async () => {
    const targets = collectorItems.filter((row) =>
      collectorRetrieveNeedsRun(row, asOfDate),
    );
    if (targets.length === 0) {
      const msg = "No misses or stale runs — every enabled collector is fresh today.";
      setCollectorAction(msg);
      setActionMessage(msg);
      setCollectorRunProgress(null);
      return;
    }
    await runCollectorTargets(targets, "Miss/stale collectors", false);
  };

  const forceCollectorRefresh = async (row: CollectorSetItem) => {
    setBusy(true);
    setActionMessage(`Force refresh ${row.symbol}…`);
    try {
      const result = await client.executeCommand(
        "CollectorRetrieve",
        collectorCommandBody(row, true),
      );
      if (!result.ok) {
        setActionMessage(`Force refresh failed: ${result.errorCode ?? "error"}`);
        return;
      }
      let recorded = 0;
      let skipped = 0;
      if (result.bodyJson) {
        try {
          const body = JSON.parse(result.bodyJson) as {
            recorded?: number;
            skipped?: number;
            payloadJson?: string;
            ok?: boolean;
            message?: string;
          };
          recorded = body.recorded ?? 0;
          skipped = body.skipped ?? 0;
          if (body.payloadJson) {
            try {
              setCollectorPayload(JSON.parse(body.payloadJson) as Record<string, unknown>);
            } catch {
              setCollectorPayload({ raw: body.payloadJson });
            }
          }
          setActionMessage(
            `${row.symbol}: ${body.ok === false ? "miss" : "ok"} — ${formatCount(recorded)} recorded, ${formatCount(skipped)} skipped${
              body.message ? `. ${body.message}` : ""
            }`,
          );
        } catch {
          setActionMessage(`${row.symbol}: retrieve finished.`);
        }
      }
      setCollectorSymbol(row.symbol);
      await refreshCollectors(asOfDate, row.symbol);
      await refreshData(asOfDate);
    } catch (err: unknown) {
      setActionMessage(String(err));
    } finally {
      setBusy(false);
    }
  };

  const toggleCollectorEnabled = async (row: CollectorSetItem, enabled: boolean) => {
    setBusy(true);
    try {
      const result = await client.executeCommand("RetrievalTemplateSet", {
        securityId: row.securityId,
        declarationSource: row.declarationSource,
        priceSource: row.priceSource || "public",
        sourceSymbol: row.symbol,
        sourceUrl: row.sourceUrl ?? "",
        calendarPolicy: row.calendarPolicy ?? "",
        lookbackCount: 12,
        collectorEnabled: enabled,
        inceptionOn: row.inceptionOn ?? "",
      });
      if (!result.ok) {
        setActionMessage(`Enable failed: ${result.errorCode ?? "error"}`);
        return;
      }
      setActionMessage(
        `${row.symbol}: collector ${enabled ? "enabled" : "disabled"}.`,
      );
      await refreshCollectors(asOfDate, row.symbol);
    } catch (err: unknown) {
      setActionMessage(String(err));
    } finally {
      setBusy(false);
    }
  };

  const openExceptionLog = useCallback(async () => {
    try {
      const path = await invoke<string>("open_exception_log", {
        exceptions,
        processLines: captureProcess?.lines ?? [],
      });
      setActionMessage(`Log opened: ${path}`);
    } catch (err: unknown) {
      setActionMessage(`Could not open log: ${String(err)}`);
    }
  }, [exceptions, captureProcess]);

  const refreshData = useCallback(async (asOf: string) => {
    await client.executeQuery("CashDividendCoverageGet", { asOfDate: asOf });
    const [weekResult, burnResult, trendsResult, trendsWeekResult, holdingsResult, exceptionResult, summaryResult, dividendResult, calcResult, accountResult, securityResult, positionResult, masterResult, coverageResult] =
      await Promise.all([
        client.executeQuery("IncomePlanWeekGet", { asOfDate: asOf }),
        client.executeQuery("DashboardBurndownGet", { asOfDate: asOf }),
        client.executeQuery("TrendsGet", { asOfDate: asOf }),
        client.executeQuery("TrendsWeekGet", { asOfDate: asOf }),
        client.executeQuery("HoldingsGet"),
        client.executeQuery("ExceptionList"),
        client.executeQuery("DataSummaryGet"),
        client.executeQuery("DividendGet"),
        client.executeQuery("CalculatorGet"),
        client.executeQuery("AccountList"),
        client.executeQuery("SecurityList"),
        client.executeQuery("PositionDetailsGet", { asOfDate: asOf }),
        client.executeQuery("PositionMasterGet"),
        client.executeQuery("PositionDetailsCoverageGet"),
      ]);
    const failed = [
      weekResult,
      burnResult,
      trendsResult,
      trendsWeekResult,
      holdingsResult,
      exceptionResult,
      summaryResult,
      dividendResult,
      calcResult,
      accountResult,
      securityResult,
      positionResult,
      masterResult,
      coverageResult,
    ].filter((r) => !r.ok);
    if (failed.length > 0) {
      const stale = failed.some((r) => r.errorCode === "unknown_query");
      const detail = failed
        .map((r) => `${r.queryName}: ${r.errorCode ?? "error"}`)
        .join("; ");
      setActionMessage(
        stale
          ? `Desktop host is stale (${detail}). Fully quit finos and run npm run desktop so rust rebuilds.`
          : detail,
      );
    }
    const parse = <T,>(raw?: string): T | null => {
      if (!raw) return null;
      try {
        return JSON.parse(raw) as T;
      } catch {
        return null;
      }
    };
    setIncomeWeek(parse<IncomePlanWeekGet>(weekResult.bodyJson));
    setBurndown(parse<DashboardBurndownGet>(burnResult.bodyJson));
    if (trendsResult.ok) {
      setTrends(parse<TrendsGet>(trendsResult.bodyJson));
      setTrendsError(null);
    } else {
      setTrends({ points: [], totalMinor: 0, weeks: [], scale: 2 });
      setTrendsError(trendsResult.errorCode ?? "TrendsGet failed");
    }
    if (trendsWeekResult.ok) {
      setTrendsCapture(parse<TrendsWeekCapture>(trendsWeekResult.bodyJson));
    }
    setHoldings(parse<HoldingsGet>(holdingsResult.bodyJson));
    setCalculator(parse<CalculatorGet>(calcResult.bodyJson));
    setSummary(parse<DataSummaryGet>(summaryResult.bodyJson));
    setDividendLifetime(parse<DividendGet>(dividendResult.bodyJson));
    setPositionDetails(parse<PositionDetailsGet>(positionResult.bodyJson));
    setPositionMaster(parse<PositionMasterGet>(masterResult.bodyJson));
    setIssuerCoverage(parse<PositionDetailsCoverageGet>(coverageResult.bodyJson));
    if (accountResult.bodyJson) {
      try {
        const listed = JSON.parse(accountResult.bodyJson) as AccountListItem[];
        setAccounts(Array.isArray(listed) ? listed : []);
      } catch {
        setAccounts([]);
      }
    }
    if (securityResult.bodyJson) {
      try {
        const listed = JSON.parse(securityResult.bodyJson) as SecurityListItem[];
        setSecurities(Array.isArray(listed) ? listed : []);
      } catch {
        setSecurities([]);
      }
    }
    if (exceptionResult.bodyJson) {
      try {
        setExceptions(JSON.parse(exceptionResult.bodyJson) as ExceptionRecord[]);
      } catch {
        setExceptions([]);
      }
    }
  }, []);

  const refreshLastPrices = useCallback(async () => {
    setLastPriceBusy(true);
    setActionMessage("Refreshing last prices…");
    try {
      const apply = await client.executeCommand("ProviderDeclarationSourcesApply", {});
      if (apply.ok && apply.bodyJson) {
        try {
          const body = JSON.parse(apply.bodyJson) as { updated?: number };
          if ((body.updated ?? 0) > 0) {
            setActionMessage(
              `Applied issuer sources from provider: ${formatCount(body.updated ?? 0)} updated.`,
            );
          }
        } catch {
          /* ignore */
        }
      }
      const result = await client.executeCommand("LastPriceRefresh", {});
      if (!result.ok) {
        setActionMessage(
          `Last price refresh failed: ${result.errorCode ?? "error"}. Last stored price still shows, including stale.`,
        );
        return;
      }
      let recorded = 0;
      let skipped = 0;
      if (result.bodyJson) {
        try {
          const body = JSON.parse(result.bodyJson) as {
            recorded?: number;
            skipped?: number;
          };
          recorded = body.recorded ?? 0;
          skipped = body.skipped ?? 0;
        } catch {
          /* ignore */
        }
      }
      setActionMessage(
        `Last prices updated: ${formatCount(recorded)} recorded, ${formatCount(skipped)} skipped. Stale last price still displays. Miss stays unknown.`,
      );
      await refreshData(asOfDate);
    } catch (err: unknown) {
      setActionMessage(String(err));
    } finally {
      setLastPriceBusy(false);
    }
    try {
      const decls = await client.executeCommand("DeclarationRefresh", {});
      let declRecorded = 0;
      let declSkipped = 0;
      if (decls.ok && decls.bodyJson) {
        try {
          const body = JSON.parse(decls.bodyJson) as {
            recorded?: number;
            skipped?: number;
          };
          declRecorded = body.recorded ?? 0;
          declSkipped = body.skipped ?? 0;
        } catch {
          /* ignore */
        }
      }
      setActionMessage(
        (prev) =>
          `${prev ?? "Last prices updated."} Declarations: ${formatCount(declRecorded)} recorded, ${formatCount(declSkipped)} skipped.`,
      );
      await refreshData(asOfDate);
    } catch (err: unknown) {
      setActionMessage(String(err));
    }
  }, [asOfDate, refreshData]);

  const applyIssuerSources = async () => {
    setBusy(true);
    setCollectorAction("Applying issuer sources from provider…");
    try {
      const result = await client.executeCommand("ProviderDeclarationSourcesApply", {});
      if (!result.ok) {
        const msg = `Apply issuer sources failed: ${result.errorCode ?? "error"}`;
        setCollectorAction(msg);
        setActionMessage(msg);
        return;
      }
      let updated = 0;
      if (result.bodyJson) {
        try {
          const body = JSON.parse(result.bodyJson) as { updated?: number };
          updated = body.updated ?? 0;
        } catch {
          /* ignore */
        }
      }
      const msg =
        updated > 0
          ? `Applied issuer sources: ${formatCount(updated)} templates updated (empty/public/unassigned only).`
          : "Apply issuer sources: 0 updated — assigned sources already match provider (nothing left to fill).";
      setCollectorAction(msg);
      setActionMessage(msg);
      if (screen === "collectors") {
        await refreshCollectors(asOfDate);
        setCollectorAction(msg);
      } else {
        await refreshData(asOfDate);
        if (positionSymbol) {
          await loadInvestment(positionSymbol);
        }
      }
    } catch (err: unknown) {
      const msg = String(err);
      setCollectorAction(msg);
      setActionMessage(msg);
    } finally {
      setBusy(false);
    }
  };

  const loadInvestment = useCallback(async (symbol: string, options?: { skipPriceRefresh?: boolean }) => {
    if (!symbol) {
      setInvestment(null);
      setPdDraft(null);
      setPdBaseline("");
      setPdPeriod(emptyPeriod());
      setPdPeriodBaseline(JSON.stringify(emptyPeriod()));
      setSavedPeriodId("");
      setPdRemaining(null);
      return;
    }
    const asOf = asOfDate || new Date().toISOString().slice(0, 10);
    const fetchInvestment = async (): Promise<InvestmentGet | null> => {
      const result = await client.executeQuery("InvestmentGet", {
        symbol,
        asOfDate: asOf,
      });
      if (!result.ok || !result.bodyJson) {
        setInvestment(null);
        setActionMessage(`InvestmentGet failed: ${result.errorCode ?? "not found"}`);
        return null;
      }
      try {
        return JSON.parse(result.bodyJson) as InvestmentGet;
      } catch {
        setInvestment(null);
        setPdDraft(null);
        setPdBaseline("");
        return null;
      }
    };
    const applyInvestment = (body: InvestmentGet) => {
      const draft = draftFromInvestment(body);
      setInvestment(body);
      setPdDraft(draft);
      setPdBaseline(JSON.stringify(draft));
      setPdPeriod(emptyPeriod());
      setPdPeriodBaseline(JSON.stringify(emptyPeriod()));
      setSavedPeriodId(body.periods?.at(-1)?.periodId ?? "");
      if ((body.notes || "").trim()) {
        setResearchNotes((prev) =>
          prev?.overview
            ? prev
            : {
                overview: body.notes.trim(),
                suggestedTier:
                  body.suggestion?.suggestedTier ||
                  body.lookthrough?.riskTierSuggestion ||
                  "",
                suggestedReason:
                  body.suggestion?.reason ||
                  "Suggestion only — owner sets risk manually.",
                source: "stored notes",
              },
        );
      }
    };
    let body = await fetchInvestment();
    if (!body) return;
    applyInvestment(body);
    let draft = draftFromInvestment(body);
    let changed = false;
    if (!options?.skipPriceRefresh && body.price?.freshness !== "current") {
      try {
        const result = await client.executeCommand("MarketRetrieve", {
          symbol: body.symbol,
          priceSource: draft.priceSource,
          sourceSymbol: draft.sourceSymbol,
        });
        if (result.ok && result.bodyJson) {
          const retrieve = JSON.parse(result.bodyJson) as {
            quote?: { priceMinor?: number; scale?: number; asOfAt?: string; source?: string } | null;
          };
          const quote = retrieve.quote;
          if (quote?.priceMinor && quote.priceMinor > 0) {
            const posted = await client.executeCommand("PriceQuoteRecord", {
              securityId: body.securityId,
              priceMinor: quote.priceMinor,
              scale: quote.scale ?? 2,
              asOfAt: quote.asOfAt ?? asOf,
              source: quote.source ?? "yahoo",
            });
            if (posted.ok) {
              changed = true;
            }
          }
        }
      } catch {
        /* keep stored price + date */
      }
    }
    const paidDecls = body.declarations.filter(
      (d) => d.amountPerShareMinor != null && d.amountPerShareMinor > 0,
    ).length;
    if (
      !options?.skipPriceRefresh &&
      paidDecls < DECLARATION_LOOKBACK_TARGET &&
      (body.template?.declarationSource?.trim() || draft.declarationSource.trim())
    ) {
      try {
        const knownPaymentPeriods = body.declarations
          .map((d) => d.paymentPeriod?.trim())
          .filter((p): p is string => Boolean(p));
        const result = await client.executeCommand("MarketRetrieve", {
          symbol: body.symbol,
          securityId: body.securityId,
          declarationSource:
            body.template?.declarationSource ?? draft.declarationSource ?? "",
          sourceSymbol: body.template?.sourceSymbol ?? draft.sourceSymbol ?? body.symbol,
          priceSource: body.template?.priceSource ?? draft.priceSource ?? "",
          sourceUrl: body.template?.sourceUrl ?? draft.sourceUrl,
          knownPaymentPeriods,
        });
        if (result.ok && result.bodyJson) {
          const retrieve = JSON.parse(result.bodyJson) as {
            candidates?: Array<{
              amountPerShareMinor?: number;
              amountScale?: number;
              paymentPeriod?: string;
              source?: string;
            }>;
          };
          const existing = new Set(knownPaymentPeriods);
          const retrieved = (retrieve.candidates ?? []).filter(
            (d) =>
              d.source &&
              d.amountPerShareMinor != null &&
              d.amountPerShareMinor > 0 &&
              d.paymentPeriod?.trim() &&
              !existing.has(d.paymentPeriod.trim()),
          );
          let posted = 0;
          for (const [i, cand] of retrieved.entries()) {
            const rec = await client.executeCommand("IssuerDeclarationRecord", {
              securityId: body.securityId,
              amountPerShareMinor: cand.amountPerShareMinor,
              amountScale: cand.amountScale ?? 4,
              paymentPeriod: cand.paymentPeriod ?? `period-${i}`,
              source: cand.source,
              enteredAt: asOf,
            });
            if (rec.ok) posted += 1;
          }
          if (posted > 0) {
            changed = true;
          }
        }
      } catch {
        /* keep stored declarations */
      }
    }
    if (changed) {
      const refreshed = await fetchInvestment();
      if (refreshed) {
        applyInvestment(refreshed);
      }
    }
  }, [asOfDate]);

  const refreshHandoff = useCallback(async (): Promise<HandoffStatus | null> => {
    const result = await client.executeQuery("HandoffStatusGet");
    if (!result.ok) {
      setHandoff(null);
      setHandoffError(result.errorCode ?? "handoff_failed");
      return null;
    }
    setHandoffError(null);
    const status = parseHandoff(result.bodyJson ?? undefined);
    setHandoff(status);
    return status;
  }, []);

  useEffect(() => {
    let cancelled = false;
    client
      .executeQuery("HealthGet")
      .then((result) => {
        if (cancelled) return;
        let status = "unknown";
        let contractVersion = FINANCE_CLIENT_CONTRACT_VERSION;
        if (result.bodyJson) {
          try {
            const body = JSON.parse(result.bodyJson) as {
              status?: string;
              contractVersion?: string;
            };
            status = body.status ?? status;
            contractVersion = body.contractVersion ?? contractVersion;
          } catch {
            status = "invalid-body";
          }
        }
        setHealth({
          ok: result.ok,
          status,
          contractVersion,
          error: result.errorCode,
        });
      })
      .catch((err: unknown) => {
        if (cancelled) return;
        setHealth({
          ok: false,
          status: "invoke-failed",
          contractVersion: FINANCE_CLIENT_CONTRACT_VERSION,
          error: String(err),
        });
      });
    client.executeQuery("ConfigGet").then((result) => {
      if (cancelled || !result.bodyJson) return;
      try {
        const body = JSON.parse(result.bodyJson) as { deviceName?: string };
        if (body.deviceName) setDeviceName(body.deviceName);
      } catch {
        /* ignore */
      }
    });
    (async () => {
      try {
        await refreshHandoff();
      } catch (err: unknown) {
        if (!cancelled) setHandoffError(String(err));
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [refreshHandoff]);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const [summaryResult, dividendResult] = await Promise.all([
          client.executeQuery("DataSummaryGet"),
          client.executeQuery("DividendGet"),
        ]);
        if (cancelled) return;
        if (!summaryResult.ok || !summaryResult.bodyJson) {
          setAsOfDate("");
          return;
        }
        const body = JSON.parse(summaryResult.bodyJson) as DataSummaryGet;
        setSummary(body);
        if (dividendResult.ok && dividendResult.bodyJson) {
          setDividendLifetime(JSON.parse(dividendResult.bodyJson) as DividendGet);
        }
        const today = new Date().toISOString().slice(0, 10);
        setAsOfDate(saturdayOfWeek(today));
      } catch {
        if (!cancelled) setAsOfDate("");
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    refreshData(asOfDate).catch((err: unknown) => {
      setActionMessage(String(err));
    });
  }, [asOfDate, refreshData]);

  useEffect(() => {
    if (screen !== "collectors" && screen !== "settings") {
      return;
    }
    void refreshCollectors(asOfDate).catch((err: unknown) => {
      setActionMessage(String(err));
    });
  }, [screen, asOfDate, refreshCollectors]);

  const saveSettingsTemplate = async (row: CollectorSetItem) => {
    const draft = settingsTemplateDrafts[row.securityId];
    if (!draft) return;
    setBusy(true);
    try {
      const result = await client.executeCommand("RetrievalTemplateSet", {
        securityId: row.securityId,
        declarationSource: draft.declarationSource.trim(),
        priceSource: row.priceSource || "public",
        sourceSymbol: row.symbol,
        sourceUrl: draft.sourceUrl.trim(),
        calendarPolicy: draft.calendarPolicy.trim(),
        lookbackCount:
          Number(draft.lookbackCount) || DECLARATION_LOOKBACK_TARGET,
        collectorEnabled: row.collectorEnabled,
        inceptionOn: draft.inceptionOn.trim(),
      });
      if (!result.ok) {
        setActionMessage(
          `Template not stored for ${row.symbol}: ${result.errorCode ?? "error"}`,
        );
        return;
      }
      setActionMessage(`${row.symbol}: retrieval template saved.`);
      await refreshCollectors(asOfDate, row.symbol);
    } catch (err: unknown) {
      setActionMessage(String(err));
    } finally {
      setBusy(false);
    }
  };

  useEffect(() => {
    if (lastPriceKickoff.current) {
      return;
    }
    if (!summary || summary.openLotCount <= 0) {
      return;
    }
    lastPriceKickoff.current = true;
    refreshLastPrices().catch((err: unknown) => {
      setActionMessage(String(err));
    });
  }, [summary, refreshLastPrices]);

  useEffect(() => {
    if (screen !== "new-investment" || !wizSecurityId) {
      return;
    }
    let cancelled = false;
    const asOf = asOfDate || new Date().toISOString().slice(0, 10);
    void client
      .executeQuery("RemainingYearIncomeGet", {
        securityId: wizSecurityId,
        asOfDate: asOf,
      })
      .then((result) => {
        if (cancelled || !result.ok || !result.bodyJson) return;
        const body = JSON.parse(result.bodyJson) as RemainingYearIncomeGet;
        const next = body.payments.map((p) => ({
          originalPayOn: p.originalPayOn,
          payOn: p.payOn,
        }));
        setWizRemaining(body);
        setWizPayDraft((prev) => {
          if (prev.some((p) => p.payOn !== p.originalPayOn)) return prev;
          return next;
        });
        setWizBaseline((prev) => {
          const draft = JSON.parse(prev) as WizEditDraft;
          const parsed = parseRemainingPaysKey(draft.remainingPays ?? "");
          if (parsed.pays.some((p) => p.payOn !== p.originalPayOn)) return prev;
          draft.remainingPays = remainingPaysKey(next, draft.nextPayDate ?? "");
          return JSON.stringify(draft);
        });
      });
    return () => {
      cancelled = true;
    };
  }, [screen, wizSecurityId, wizPlanStored, asOfDate]);

  useEffect(() => {
    if (screen !== "position-details" || !positionFocusPanel) {
      return;
    }
    const el = document.getElementById(`hub-${positionFocusPanel}`);
    if (el) {
      el.scrollIntoView({ behavior: "smooth", block: "start" });
    }
  }, [screen, positionFocusPanel, investment?.symbol]);

  useEffect(() => {
    if (screen !== "position-details" || !investment) {
      return;
    }
    let cancelled = false;
    const asOf = asOfDate || new Date().toISOString().slice(0, 10);
    void client
      .executeQuery("RemainingYearIncomeGet", {
        securityId: investment.securityId,
        asOfDate: asOf,
      })
      .then((result) => {
        if (cancelled || !result.ok || !result.bodyJson) return;
        setPdRemaining(JSON.parse(result.bodyJson) as RemainingYearIncomeGet);
      });
    void client.executeQuery("DividendGet", {}).then((result) => {
      if (cancelled || !result.ok || !result.bodyJson) return;
      try {
        const body = JSON.parse(result.bodyJson) as DividendGet;
        const sid = investment.securityId;
        setPdLedger({
          ...body,
          actuals: (body.actuals ?? []).filter((a) => a.securityId === sid),
        });
      } catch {
        setPdLedger(null);
      }
    });
    return () => {
      cancelled = true;
    };
  }, [screen, investment, asOfDate]);

  const runCommand = async (name: string, body?: unknown) => {
    setBusy(true);
    setActionMessage(null);
    try {
      const result = await client.executeCommand(name, body);
      setActionMessage(
        result.ok ? `${name} ok` : `${name} failed: ${result.errorCode ?? "error"}`,
      );
      await refreshHandoff();
      await refreshData(asOfDate);
      if (name === "ConfigSet" && result.bodyJson) {
        const cfg = JSON.parse(result.bodyJson) as { deviceName?: string };
        if (cfg.deviceName) setDeviceName(cfg.deviceName);
      }
    } catch (err: unknown) {
      setActionMessage(String(err));
    } finally {
      setBusy(false);
    }
  };

  const dollarsToMinor = (raw: string) => Math.round(Number(raw) * 100);

  const totalDollarsToMinor = (raw: string): number | null => {
    const n = Number(raw.replace(/[$,\s]/g, ""));
    if (!Number.isFinite(n) || n <= 0) return null;
    return Math.round(n * 100);
  };

  /** Lot total USD cents from unit price cents and qty (quantity_scale 0 for Add Lot). */
  const lotTotalFromUnitCents = (
    qtyMinor: number,
    quantityScale: number,
    unitCents: number,
  ) => {
    if (qtyMinor <= 0 || unitCents < 0) return 0;
    const den = 10 ** quantityScale;
    return Math.trunc((qtyMinor * unitCents) / den);
  };

  const formatMoneyInput = (raw: string) => {
    const n = Number(raw);
    if (!Number.isFinite(n)) return raw.trim();
    return n.toLocaleString("en-US", {
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
    });
  };

  const currentWizEdit = (): WizEditDraft => ({
    symbol: wizSymbol,
    name: wizName,
    provider: wizProvider,
    underlying: wizUnderlying,
    lookthrough: wizLookthrough,
    risk: wizRisk,
    freq: wizFreq,
    price: wizPrice,
    priceSource: wizPriceSource,
    declSource: wizDeclSource,
    lookback: wizLookback,
    sourceUrl: wizSourceUrl,
    calendarPolicy: wizCalendarPolicy,
    declAmounts: wizDeclAmounts,
    plan: wizPlan,
    planReason: wizPlanReason,
    incomplete: wizIncomplete,
    accountId: wizAccountId,
    qty: wizQty,
    cost: wizCost,
    rocPct: wizRocPct,
    bullStart: wizBullStart,
    bullEnd: wizBullEnd,
    bearStart: wizBearStart,
    bearEnd: wizBearEnd,
    remainingPays: remainingPaysKey(wizPayDraft, wizNextPay),
    nextPayDate: wizNextPay,
  });
  const wizDirty = JSON.stringify(currentWizEdit()) !== wizBaseline;
  const snapshotWiz = (patch: Partial<WizEditDraft> = {}) => {
    setWizBaseline(JSON.stringify({ ...currentWizEdit(), ...patch }));
  };
  const cancelWizEdits = () => {
    const draft = JSON.parse(wizBaseline) as WizEditDraft;
    setWizSymbol(draft.symbol);
    setWizName(draft.name);
    setWizProvider(draft.provider);
    setWizUnderlying(draft.underlying ?? "");
    setWizLookthrough(mergeLookthrough(draft.lookthrough));
    setWizRisk(draft.risk);
    setWizFreq(draft.freq);
    setWizPrice(draft.price);
    setWizPriceSource(draft.priceSource ?? "public");
    setWizDeclSource(draft.declSource);
    setWizLookback(draft.lookback ?? "12");
    setWizSourceUrl(draft.sourceUrl ?? "");
    setWizCalendarPolicy(draft.calendarPolicy ?? "");
    setWizDeclAmounts(draft.declAmounts);
    setWizPlan(draft.plan);
    setWizPlanReason(draft.planReason);
    setWizIncomplete(draft.incomplete);
    setWizAccountId(draft.accountId);
    setWizQty(draft.qty);
    setWizCost(draft.cost);
    setWizRocPct(draft.rocPct);
    setWizBullStart(draft.bullStart);
    setWizBullEnd(draft.bullEnd);
    setWizBearStart(draft.bearStart);
    setWizBearEnd(draft.bearEnd);
    const remaining = parseRemainingPaysKey(draft.remainingPays ?? "");
    setWizPayDraft(remaining.pays);
    setWizNextPay(draft.nextPayDate ?? remaining.nextPay);
    setActionMessage("Edits discarded.");
  };
  const currentAddLot = (): AddLotDraft => ({
    securityId: addLotSecurityId,
    accountId: addLotAccountId,
    openedOn: addLotOpenedOn,
    qty: addLotQty,
    cost: addLotCost,
    taxCost: addLotTaxCost,
    taxCostDifferent: addLotTaxDifferent,
    origin: addLotOrigin,
  });
  const addLotDirty = JSON.stringify(currentAddLot()) !== addLotBaseline;
  const cancelAddLotEdits = () => {
    const draft = JSON.parse(addLotBaseline) as AddLotDraft;
    setAddLotSecurityId(draft.securityId);
    const sec = securities.find((s) => s.securityId === draft.securityId);
    setAddLotQuery(sec ? `${sec.symbol}${sec.name ? ` — ${sec.name}` : ""}` : "");
    setAddLotAccountId(draft.accountId);
    setAddLotOpenedOn(draft.openedOn || new Date().toISOString().slice(0, 10));
    setAddLotQty(draft.qty);
    setAddLotCost(draft.cost);
    setAddLotTaxCost(draft.taxCost ?? "");
    setAddLotTaxDifferent(Boolean(draft.taxCostDifferent));
    setAddLotOrigin(draft.origin || "purchase");
    setActionMessage("Edits discarded.");
  };

  const parseCadence = (raw: string) => {
    const key = raw.trim().toLowerCase();
    if (key === "weekly" || key === "52") return { label: "Weekly", periods: 52 as const };
    if (key === "monthly" || key === "12") return { label: "Monthly", periods: 12 as const };
    if (key === "quarterly" || key === "4") return { label: "Quarterly", periods: 4 as const };
    if (key === "none") return { label: "None", periods: 0 as const };
    return null;
  };

  const persistRemainingDates = async (
    securityId: string,
    pays: Array<{ originalPayOn: string; payOn: string }>,
    nextPay: string,
  ) => {
    const asOf = asOfDate || new Date().toISOString().slice(0, 10);
    for (const p of pays) {
      if (p.payOn.trim() === "" || p.payOn === p.originalPayOn) continue;
      const result = await client.executeCommand("RemainingPaymentDateOverride", {
        securityId,
        originalPayOn: p.originalPayOn,
        payOn: p.payOn.trim(),
        asOfDate: asOf,
      });
      if (!result.ok) return result;
    }
    if (nextPay.trim()) {
      const result = await client.executeCommand("RemainingPaymentDateOverride", {
        securityId,
        originalPayOn: "",
        payOn: nextPay.trim(),
        asOfDate: asOf,
      });
      if (!result.ok) return result;
    }
    return { ok: true as const, errorCode: undefined as string | undefined };
  };

  const saveWizRemainingDates = async () => {
    if (!wizSecurityId) {
      setActionMessage("Save Part 1 before remaining payment dates.");
      return;
    }
    setBusy(true);
    try {
      const result = await persistRemainingDates(wizSecurityId, wizPayDraft, wizNextPay);
      if (!result.ok) {
        setActionMessage(`Remaining dates not stored: ${result.errorCode ?? "error"}`);
        return;
      }
      snapshotWiz();
      setActionMessage("Stored remaining payment dates.");
    } finally {
      setBusy(false);
    }
  };

  const applyMarketBody = (raw: string, mode: "research" | "retrieve" = "retrieve") => {
    const body = JSON.parse(raw) as {
      name?: string;
      suggestedProvider?: string;
      suggestedFrequency?: string;
      suggestedCalendarPolicy?: string;
      underlying?: string;
      lookthrough?: LookthroughResearch;
      declarationSource?: string;
      priceSource?: string;
      lookbackCount?: number;
      sourceUrl?: string;
      missExplanation?: string;
      futureDeclarationStrategy?: string;
      sourceAnalytics?: {
        htmlReturned?: boolean;
        fundPage?: boolean;
        tableOnGet?: boolean;
        tableRowCount?: number;
        jsLikely?: boolean;
      };
      researchAttempts?: Array<{ vendor: string; url: string; found: boolean; note: string }>;
      upcomingPays?: Array<{
        payOn?: string;
        amountPerShareMinor?: number | null;
        amountScale?: number;
      }>;
      quote?: { priceMinor?: number; scale?: number; asOfAt?: string; source?: string } | null;
      candidates?: Array<{
        amountPerShareMinor?: number;
        amountScale?: number;
        paymentPeriod?: string;
        source?: string;
      }>;
      source?: string;
    };
    if (body.name) {
      setWizName(body.name);
    }
    if (body.suggestedProvider?.trim()) {
      setWizProvider(body.suggestedProvider.trim());
    }
    if (body.underlying?.trim()) {
      setWizUnderlying(body.underlying.trim());
    }
    if (body.lookthrough) {
      setWizLookthrough(mergeLookthrough(body.lookthrough));
    }
    if (body.declarationSource?.trim()) {
      setWizDeclSource(body.declarationSource.trim());
    }
    if (body.priceSource?.trim()) {
      setWizPriceSource(body.priceSource.trim());
    }
    if (body.lookbackCount && body.lookbackCount > 0) {
      setWizLookback(String(body.lookbackCount));
    }
    if (body.sourceUrl) {
      setWizSourceUrl(body.sourceUrl);
    }
    if (body.researchAttempts) {
      setWizAttempts(body.researchAttempts);
    }
    if (body.missExplanation != null) {
      setWizMiss(body.missExplanation);
    }
    if (body.sourceAnalytics) {
      setWizAnalytics(body.sourceAnalytics);
    }
    if (body.futureDeclarationStrategy) {
      setWizFutureStrategy(body.futureDeclarationStrategy);
    }
    if (body.suggestedFrequency) {
      const cadence = parseCadence(body.suggestedFrequency);
      if (cadence) setWizFreq(cadence.label);
    }
    if (body.suggestedCalendarPolicy?.trim()) {
      setWizCalendarPolicy(body.suggestedCalendarPolicy.trim());
    }
    if (mode === "research") {
      return { body, n: 0, hasQuote: false };
    }
    if (body.upcomingPays) {
      setWizUpcomingPays(
        body.upcomingPays
          .filter((p) => p.payOn)
          .map((p) => ({
            payOn: p.payOn as string,
            amountPerShareMinor: p.amountPerShareMinor,
            amountScale: p.amountScale,
          })),
      );
    }
    if (body.quote?.priceMinor && body.quote.priceMinor > 0) {
      const scale = body.quote.scale ?? 2;
      setWizPrice((body.quote.priceMinor / 10 ** scale).toFixed(Math.max(2, scale)));
    }
    const decls = (body.candidates ?? []).filter(
      (d) => d.amountPerShareMinor != null && d.amountPerShareMinor > 0,
    );
    setWizDecls(
      decls.map((d) => ({
        amountPerShareMinor: d.amountPerShareMinor as number,
        amountScale: d.amountScale,
        paymentPeriod: d.paymentPeriod,
        source: d.source,
      })),
    );
    const amounts = decls.map((d) => d.amountPerShareMinor as number);
    const n = amounts.length;
    const scale = decls[0]?.amountScale ?? 4;
    const avgAll = n === 0 ? 0 : Math.round(amounts.reduce((a, b) => a + b, 0) / n);
    setWizReview({
      securityId: "",
      observationCount: n,
      mostCurrentMinor: amounts[0] ?? null,
      avg6Minor: n >= 6 ? Math.round(amounts.slice(0, 6).reduce((a, b) => a + b, 0) / 6) : null,
      minMinor: n === 0 ? null : Math.min(...amounts),
      maxMinor: n === 0 ? null : Math.max(...amounts),
      averageMinor: n === 0 ? null : avgAll,
      eightyPctOfAvgMinor: n >= 6 ? Math.round((avgAll * 80) / 100) : null,
      avg6Complete: n >= 6,
      fullAnalysisPossible: n >= 6,
      confirmBlocked: n === 0,
      incompleteReasonRequired: n > 0 && n < 6,
      amountScale: scale,
    });
    if (body.quote?.priceMinor && body.quote.priceMinor > 0) {
      setWizPriceState({
        securityId: "",
        priceMinor: body.quote.priceMinor,
        scale: body.quote.scale ?? 2,
        freshness: "current",
        priceDerivedValid: true,
      });
    }
    const quoteBit =
      body.quote?.priceMinor && body.quote.priceMinor > 0
        ? `last price ${formatUsd(body.quote.priceMinor, body.quote.scale ?? 2)} from Yahoo`
        : "no last price yet";
    const src = body.declarationSource || body.source || "unassigned";
    const note = `Source ${src}. ${formatCount(n)} issuer declarations. ${quoteBit}. Yahoo is last price only.`;
    setWizRetrieveNote(note);
    return { body, n, hasQuote: Boolean(body.quote?.priceMinor && body.quote.priceMinor > 0) };
  };

  const researchSource = async () => {
    const symbol = wizSymbol.trim().toUpperCase();
    if (!symbol) {
      setActionMessage("Type a ticker first. Research fills the maintenance strategy; it does not retrieve.");
      return false;
    }
    setBusy(true);
    setActionMessage(null);
    try {
      const result = await client.executeCommand("MarketRetrieve", {
        symbol,
        researchOnly: true,
      });
      if (!result.ok || !result.bodyJson) {
        setActionMessage(`Source research failed: ${result.errorCode ?? "error"}`);
        return false;
      }
      applyMarketBody(result.bodyJson, "research");
      setActionMessage(
        "Research filled provider, source analytics, future-declaration method, underlying, and look-through when found. Confirm the standing template, then Next retrieves. Yahoo is last price only. Risk On is not auto-applied.",
      );
      return true;
    } catch (err: unknown) {
      setActionMessage(String(err));
      return false;
    } finally {
      setBusy(false);
    }
  };

  const retrieveFromMarket = async () => {
    const symbol = wizSymbol.trim().toUpperCase();
    if (!symbol) {
      setActionMessage("Enter a ticker, then research the issuer source.");
      return false;
    }
    setBusy(true);
    setActionMessage(null);
    try {
      const result = await client.executeCommand("MarketRetrieve", {
        symbol,
        declarationSource: wizDeclSource || undefined,
        priceSource: wizPriceSource || undefined,
        sourceUrl: wizSourceUrl || undefined,
      });
      if (!result.ok || !result.bodyJson) {
        setActionMessage(`Issuer retrieve failed: ${result.errorCode ?? "error"}`);
        return false;
      }
      const { n, hasQuote, body } = applyMarketBody(result.bodyJson);
      const miss = body.missExplanation?.trim();
      if (miss) {
        setActionMessage(miss);
      } else if (!hasQuote && n === 0) {
        setActionMessage(
          "Issuer page empty.",
        );
      } else {
        const provider = body.suggestedProvider?.trim() || wizProvider;
        setActionMessage(
          `Retrieved ${formatCount(n)} declarations from ${body.declarationSource || body.source || "issuer"}${provider ? ` (${provider})` : ""}${hasQuote ? "; Yahoo last price recorded" : ""}.`,
        );
      }
      return true;
    } catch (err: unknown) {
      setActionMessage(String(err));
      return false;
    } finally {
      setBusy(false);
    }
  };

  const newInvestmentPart1 = async () => {
    if (!RISK_TIERS.includes(wizRisk)) {
      setActionMessage(
        "Choose Foundation, Core, or Risk On. The form does not assume Risk On.",
      );
      return;
    }
    const cadence = parseCadence(wizFreq);
    if (!cadence) {
      setActionMessage(
        "Choose Weekly (52), Monthly (12), Quarterly (4), or None (does not pay). Cadence is required to add the position; there is no default.",
      );
      return;
    }
    const calendarPolicy =
      cadence.label === "None" ? "none" : wizCalendarPolicy.trim();
    if (!calendarPolicy) {
      setActionMessage(
        "Choose issuer published dates or cadence from last pay. The standing order needs one calendar policy; there is no default.",
      );
      return;
    }
    setBusy(true);
    setActionMessage(null);
    try {
      let market: {
        quote?: { priceMinor?: number; scale?: number; asOfAt?: string; source?: string } | null;
        candidates?: Array<{
          amountPerShareMinor?: number;
          amountScale?: number;
          paymentPeriod?: string;
          source?: string;
        }>;
        name?: string;
        suggestedProvider?: string;
        suggestedFrequency?: string;
      } = {};
      const marketResult = await client.executeCommand("MarketRetrieve", {
        symbol: wizSymbol.trim().toUpperCase(),
        declarationSource: wizDeclSource || undefined,
        priceSource: wizPriceSource || undefined,
        sourceUrl: wizSourceUrl || undefined,
      });
      if (marketResult.ok && marketResult.bodyJson) {
        market = applyMarketBody(marketResult.bodyJson).body;
      }
      const registered = await client.executeCommand("SecurityRegister", {
        symbol: wizSymbol.trim().toUpperCase(),
        name: (market.name || wizName).trim() || wizSymbol.trim(),
      });
      if (!registered.ok || !registered.bodyJson) {
        setActionMessage(`SecurityRegister failed: ${registered.errorCode ?? "error"}`);
        return;
      }
      const securityId = (JSON.parse(registered.bodyJson) as { securityId?: string }).securityId;
      if (!securityId) {
        setActionMessage("SecurityRegister failed: missing securityId");
        return;
      }
      setWizSecurityId(securityId);
      await client.executeCommand("RetrievalTemplateSet", {
        securityId,
        priceSource: wizPriceSource.trim() || "public",
        sourceSymbol: wizSymbol.trim().toUpperCase(),
        declarationSource: wizDeclSource.trim() || "unassigned",
        lookbackCount: Number(wizLookback) || 12,
        sourceUrl: wizSourceUrl.trim(),
        calendarPolicy,
      });
      await client.executeCommand("PositionCharacteristicUpsert", {
        securityId,
        paymentFrequency: cadence.label,
        riskTier: wizRisk,
        provider: wizProvider.trim(),
        underlying: wizUnderlying.trim(),
        lookthrough: wizLookthrough,
      });
      let pricePosted = false;
      const live = market.quote;
      if (live?.priceMinor && live.priceMinor > 0) {
        await client.executeCommand("PriceQuoteRecord", {
          securityId,
          priceMinor: live.priceMinor,
          scale: live.scale ?? 2,
          asOfAt: live.asOfAt ?? new Date().toISOString().slice(0, 10),
          source: live.source ?? "yahoo",
        });
        pricePosted = true;
      }
      if (!pricePosted) {
        const prompted = dollarsToMinor(wizPrice);
        if (!Number.isFinite(prompted) || prompted <= 0) {
          setActionMessage("Live quote unavailable. Enter a current price to continue.");
          return;
        }
        await client.executeCommand("ManualPriceOverride", {
          securityId,
          priceMinor: prompted,
          scale: 2,
          reason: "prompt after live miss",
          effectiveFrom: new Date().toISOString().slice(0, 10),
        });
      }
      let postedDecls = 0;
      const retrievedDecls = (market.candidates ?? []).filter(
        (d) => d.source && d.amountPerShareMinor != null && d.amountPerShareMinor > 0,
      );
      for (const [i, cand] of retrievedDecls.slice(0, 12).entries()) {
        await client.executeCommand("IssuerDeclarationRecord", {
          securityId,
          amountPerShareMinor: cand.amountPerShareMinor,
          amountScale: cand.amountScale ?? 4,
          paymentPeriod: cand.paymentPeriod ?? `period-${i}`,
          source: cand.source,
          enteredAt: new Date().toISOString().slice(0, 10),
        });
        postedDecls += 1;
      }
      if (postedDecls === 0) {
        const amounts = wizDeclAmounts
          .split(/[\s,]+/)
          .map((s) => s.trim())
          .filter(Boolean);
        if (amounts.length === 0 || !wizDeclSource.trim()) {
          setActionMessage(
            "Declaration retrieve empty. Paste up to 12 amounts and a source, then retry.",
          );
          return;
        }
        for (const [i, raw] of amounts.slice(0, 12).entries()) {
          await client.executeCommand("IssuerDeclarationRecord", {
            securityId,
            amountPerShareMinor: dollarsToMinor(raw),
            amountScale: 2,
            paymentPeriod: `obs-${i + 1}`,
            source: wizDeclSource,
            enteredAt: new Date().toISOString().slice(0, 10),
          });
        }
      }
      if (calendarPolicy === "issuer_calendar" && wizUpcomingPays.length > 0) {
        await client.executeCommand("IssuerPayDateReplace", {
          securityId,
          asOfDate: new Date().toISOString().slice(0, 10),
          dates: wizUpcomingPays.map((p) => ({
            payOn: p.payOn,
            source: wizDeclSource.trim() || "issuer",
          })),
        });
      }
      const review = await client.executeQuery("PlanReviewGet", { securityId });
      if (review.bodyJson) {
        setWizReview(JSON.parse(review.bodyJson) as PlanReviewGet);
      }
      const priceState = await client.executeQuery("CurrentPriceGet", {
        securityId,
        asOfDate: new Date().toISOString().slice(0, 10),
      });
      if (priceState.bodyJson) {
        setWizPriceState(JSON.parse(priceState.bodyJson) as CurrentPriceGet);
      }
      setActionMessage(
        `Stored ${wizSymbol.trim().toUpperCase()}: identity, ${formatCount(postedDecls || retrievedDecls.length)} declarations, price ${pricePosted ? "live" : "prompted override"}. Confirm Plan next.`,
      );
      setWizPart1Stored(true);
      setWizStep(6);
      snapshotWiz();
      await refreshData(asOfDate);
    } catch (err: unknown) {
      setActionMessage(String(err));
    } finally {
      setBusy(false);
    }
  };

  const confirmPlan = async () => {
    if (!wizSecurityId) {
      setActionMessage("Run Research first.");
      return;
    }
    if (!wizPlanReason) {
      setActionMessage("Choose a Plan reason.");
      return;
    }
    if (wizReview?.confirmBlocked || (wizReview?.observationCount ?? 0) < 1) {
      setActionMessage("Confirm Plan stays disabled until declaration research exists.");
      return;
    }
    setBusy(true);
    setActionMessage(null);
    try {
      const cadence = parseCadence(wizFreq);
      if (!cadence) {
        setActionMessage(
          "Confirm Plan stays disabled until payment frequency research exists.",
        );
        return;
      }
      const planScale = 4;
      const planMinor = Math.round(Number(wizPlan) * 10 ** planScale);
      const result = await client.executeCommand("PlanHistoryConfirm", {
        securityId: wizSecurityId,
        amountPerShareMinor: planMinor,
        amountScale: planScale,
        planningPeriodsPerYear: cadence.periods,
        effectiveFrom: new Date().toISOString().slice(0, 10),
        decisionReason: wizPlanReason,
        incompleteAnalysisReason: wizIncomplete,
      });
      if (!result.ok) {
        setActionMessage(`Plan not stored: ${result.errorCode ?? "error"}`);
        return;
      }
      setWizPlanStored(true);
      snapshotWiz();
      setActionMessage(
        `Stored Plan ${wizPlan}/share (${wizPlanReason}). Add lots from Add Lot when ready — not on this screen.`,
      );
      await refreshData(asOfDate);
    } catch (err: unknown) {
      setActionMessage(String(err));
    } finally {
      setBusy(false);
    }
  };

  const applyWizSuggestedTier = async () => {
    if (!wizSecurityId) {
      setActionMessage("Run Research first.");
      return;
    }
    const tier =
      wizTierSuggestion?.suggestedTier?.trim() ||
      wizLookthrough.riskTierSuggestion?.trim() ||
      "";
    if (!tier || !RISK_TIERS.includes(tier)) {
      setActionMessage("Apply tier stays disabled until a suggested tier exists.");
      return;
    }
    setBusy(true);
    setActionMessage(null);
    try {
      const result = await client.executeCommand("ClassificationApply", {
        securityId: wizSecurityId,
        riskTier: tier,
      });
      if (!result.ok) {
        setActionMessage(`Apply failed: ${result.errorCode ?? "error"}`);
        return;
      }
      setWizRisk(tier);
      setActionMessage(`Applied ${tier}.`);
      snapshotWiz({ risk: tier });
      await refreshData(asOfDate);
    } catch (err: unknown) {
      setActionMessage(String(err));
    } finally {
      setBusy(false);
    }
  };

  /** Process A: symbol + distribution URL → seed + retrieve; no lot / questionnaire. */
  const runProcessAResearch = async () => {
    const symbol = wizSymbol.trim().toUpperCase();
    const sourceUrl = wizSourceUrl.trim();
    if (!symbol || !sourceUrl) {
      setActionMessage("Enter symbol and distribution URL, then Research.");
      return;
    }
    const PROCESS_A_TOTAL = 4;
    setBusy(true);
    setActionMessage(null);
    setWizResearchDone(false);
    setWizProcessASaved(false);
    setWizTierSuggestion(null);
    setWizAiThesis(null);
    setWizBacktestNeeded(false);
    setWizMiss("");
    setWizRetrieveNote("");
    setResearchNotes(null);
    setResearchActivity({
      step: 1,
      total: PROCESS_A_TOTAL,
      label: "retrieving declarations",
      running: true,
      resultLine: null,
    });
    let unlistenProgress: (() => void) | undefined;
    try {
      try {
        const { listen } = await import("@tauri-apps/api/event");
        unlistenProgress = await listen<{
          step: number;
          total: number;
          label: string;
        }>("position-research-progress", (event) => {
          const p = event.payload;
          if (!p?.label) return;
          setResearchActivity({
            running: true,
            step: p.step ?? 1,
            total: p.total > 0 ? p.total : PROCESS_A_TOTAL,
            label: p.label,
            resultLine: null,
          });
        });
      } catch {
        /* browser preview without Tauri events — keep client status line */
      }

      setResearchActivity({
        running: true,
        step: 1,
        total: PROCESS_A_TOTAL,
        label: "retrieving declarations",
        resultLine: null,
      });
      const seedResult = await client.executeCommand("PositionResearchSeed", {
        symbol,
        sourceUrl,
      });
      if (!seedResult.ok || !seedResult.bodyJson) {
        setActionMessage(`Research seed failed: ${seedResult.errorCode ?? "error"}`);
        setResearchActivity({
          running: false,
          step: 0,
          total: PROCESS_A_TOTAL,
          label: "",
          resultLine: `Research failed: ${seedResult.errorCode ?? "error"}`,
        });
        return;
      }
      const seed = JSON.parse(seedResult.bodyJson) as {
        securityId: string;
        symbol: string;
        declarationSource: string;
        sourceUrl: string;
        calendarPolicy: string;
        paymentFrequency?: string;
        retrieveOk: boolean;
        retrieveCode?: string;
        retrieveMessage?: string;
        rocPctMinor?: number | null;
        rocScale?: number;
        rocSourceUrl?: string;
        rocMethod?: string;
        rocKind?: string;
        rocAsOf?: string;
        rocEstablishedHow?: string;
        rocComplete?: boolean;
        rocProbes?: { url?: string; status?: number; httpStatus?: number }[];
      };
      setResearchActivity({
        running: true,
        step: 4,
        total: PROCESS_A_TOTAL,
        label: "drafting overview",
        resultLine: null,
      });
      setWizSecurityId(seed.securityId);
      setWizSymbol(seed.symbol || symbol);
      setWizDeclSource(seed.declarationSource || "");
      setWizSourceUrl(seed.sourceUrl || sourceUrl);
      setWizCalendarPolicy(seed.calendarPolicy || "");
      setWizPart1Stored(true);

      const asOf = asOfDate || new Date().toISOString().slice(0, 10);
      const invResult = await client.executeQuery("InvestmentGet", {
        securityId: seed.securityId,
        asOfDate: asOf,
      });
      if (!invResult.ok || !invResult.bodyJson) {
        setActionMessage(`InvestmentGet failed: ${invResult.errorCode ?? "error"}`);
        return;
      }
      const inv = JSON.parse(invResult.bodyJson) as InvestmentGet;
      setWizName(inv.name || symbol);
      setWizProvider(inv.provider || "");
      setWizUnderlying(inv.underlying || "");
      setWizFreq(seed.paymentFrequency || inv.paymentFrequency || "");
      setWizRisk(inv.riskTier || "");
      setWizLookthrough(mergeLookthrough(inv.lookthrough));
      setWizReview(inv.review);
      setWizPriceState(inv.price);
      if (inv.price.priceMinor != null && inv.price.priceMinor > 0) {
        setWizPrice(scaledDollars(inv.price.priceMinor, inv.price.scale));
      } else {
        setWizPrice("");
      }
      const decls = (inv.declarations ?? [])
        .filter((d) => d.amountPerShareMinor != null && (d.amountPerShareMinor as number) > 0)
        .slice(0, 12)
        .map((d) => ({
          amountPerShareMinor: d.amountPerShareMinor as number,
          amountScale: d.amountScale,
          paymentPeriod: d.paymentPeriod,
          source: d.source,
        }));
      setWizDecls(decls);
      if (inv.template?.declarationSource) {
        setWizDeclSource(inv.template.declarationSource);
      }
      if (inv.template?.sourceUrl) {
        setWizSourceUrl(inv.template.sourceUrl);
      }
      if (inv.template?.calendarPolicy) {
        setWizCalendarPolicy(inv.template.calendarPolicy);
      }
      if (inv.review?.mostCurrentMinor != null) {
        setWizPlan(scaledDollars(inv.review.mostCurrentMinor, inv.review.amountScale));
        if (!wizPlanReason) setWizPlanReason("Match Most Current");
      }
      if (inv.suggestion?.suggestedTier) {
        setWizTierSuggestion(inv.suggestion);
      }
      setWizPlanStored(inv.planKnown);

      const rem = await client.executeQuery("RemainingYearIncomeGet", {
        securityId: seed.securityId,
        asOfDate: asOf,
      });
      if (rem.ok && rem.bodyJson) {
        const body = JSON.parse(rem.bodyJson) as RemainingYearIncomeGet;
        setWizRemaining(body);
        setWizUpcomingPays(
          body.payments.map((p) => ({
            payOn: p.payOn,
            amountPerShareMinor: null,
            amountScale: 2,
          })),
        );
        setWizPayDraft(
          body.payments.map((p) => ({
            originalPayOn: p.originalPayOn,
            payOn: p.payOn,
          })),
        );
      } else {
        setWizRemaining(null);
        setWizUpcomingPays([]);
      }

      const sug = await client.executeQuery("ClassificationSuggestGet", {
        securityId: seed.securityId,
      });
      if (sug.ok && sug.bodyJson) {
        const body = JSON.parse(sug.bodyJson) as {
          suggestedTier: string;
          ruleset: string;
          reason: string;
          complete: boolean;
        };
        if (body.suggestedTier?.trim()) {
          setWizTierSuggestion(body);
          setWizBacktestNeeded(false);
        } else if (
          !body.complete ||
          body.reason === "no calculated window" ||
          /no calculated window/i.test(body.reason || "")
        ) {
          setWizTierSuggestion(body);
          setWizBacktestNeeded(true);
        }
      }

      // Optional advisory thesis — does not post facts or apply tier.
      try {
        const thesisPrompt = [
          `Write a 4-8 sentence investment overview for ${seed.symbol}.`,
          inv.name ? `Legal name: ${inv.name}.` : "",
          inv.provider ? `Provider: ${inv.provider}.` : "",
          inv.underlying
            ? `Underlying/look-through is ${inv.underlying} (not the ticker ${seed.symbol}).`
            : "",
          inv.paymentFrequency ? `Pays ${inv.paymentFrequency}.` : "",
          inv.lookthrough?.themeStrategy
            ? `Theme: ${inv.lookthrough.themeStrategy}.`
            : "",
          inv.lookthrough?.primaryRiskDriver
            ? `Primary risk: ${inv.lookthrough.primaryRiskDriver}.`
            : "",
          "Cover what it is, how it pays, main risks, and that underlying is not the sleeve ticker.",
          "Advisory only; do not post facts or apply classification.",
        ]
          .filter(Boolean)
          .join(" ");
        const ai = await client.executeCommand("AiAnalyze", { prompt: thesisPrompt });
        if (ai.ok && ai.bodyJson) {
          const run = JSON.parse(ai.bodyJson) as { recommendation?: string };
          if (run.recommendation?.trim()) {
            setWizAiThesis(run.recommendation.trim());
            setResearchNotes({
              overview: run.recommendation.trim(),
              suggestedTier: inv.suggestion?.suggestedTier || wizTierSuggestion?.suggestedTier || "",
              suggestedReason:
                inv.suggestion?.reason ||
                wizTierSuggestion?.reason ||
                inv.lookthrough?.riskTierSuggestion ||
                "",
              source: "issuer page + AiAnalyze draft",
            });
          }
        }
      } catch {
        /* advisory miss stays blank */
      }

      // Process A proposes 19a-1 estimate only — never marks research complete, never 0%.
      if (seed.rocPctMinor != null && seed.rocPctMinor > 0) {
        const scale = seed.rocScale ?? 2;
        setWizRoc({
          securityId: seed.securityId,
          rocPctMinor: seed.rocPctMinor,
          scale,
          source: "19a-1",
          complete: false,
          reason: "current-year 19a-1 estimate",
          candidates: [],
          remainingPeriods: null,
          remainingTotalMinor: null,
          remainingOrdinaryMinor: null,
          remainingRocMinor: null,
          magiEligible: false,
          systemRocPctMinor: seed.rocPctMinor,
          sourceUrl: seed.rocSourceUrl || "",
          method: seed.rocMethod || "19a-1-current-year",
          asOf: seed.rocAsOf || "",
          kind: seed.rocKind || "estimate",
          establishedHow: seed.rocEstablishedHow || "",
          ownerOverride: false,
          observations: [],
        });
        setWizRocPct((seed.rocPctMinor / 10 ** scale).toFixed(scale));
      } else {
        setWizRoc(null);
        setWizRocPct("");
      }
      setWizResearchDone(true);
      snapshotWiz({
        symbol: seed.symbol || symbol,
        name: inv.name || symbol,
        provider: inv.provider || "",
        underlying: inv.underlying || "",
        freq: seed.paymentFrequency || inv.paymentFrequency || "",
        risk: inv.riskTier || "",
        sourceUrl: seed.sourceUrl || sourceUrl,
        declSource: seed.declarationSource || "",
        calendarPolicy: seed.calendarPolicy || "",
        plan: inv.review?.mostCurrentMinor != null
          ? scaledDollars(inv.review.mostCurrentMinor, inv.review.amountScale)
          : "",
      });
      const filledBits = [
        inv.provider ? `provider ${inv.provider}` : null,
        inv.underlying ? `underlying ${inv.underlying}` : null,
        (seed.paymentFrequency || inv.paymentFrequency)
          ? `frequency ${seed.paymentFrequency || inv.paymentFrequency}`
          : null,
        seed.rocPctMinor != null && seed.rocPctMinor > 0
          ? `ROC estimate ${(seed.rocPctMinor / 10 ** (seed.rocScale ?? 2)).toFixed(seed.rocScale ?? 2)}%`
          : null,
      ].filter(Boolean);
      const unknownBits = [
        !inv.provider ? "provider" : null,
        !inv.underlying ? "underlying" : null,
        !(seed.paymentFrequency || inv.paymentFrequency) ? "frequency" : null,
        !(seed.rocPctMinor != null && seed.rocPctMinor > 0) ? "ROC estimate" : null,
      ].filter(Boolean);
      const retrieveNote = seed.retrieveOk
        ? "Retrieve finished."
        : `Retrieve miss${seed.retrieveCode ? ` (${seed.retrieveCode})` : ""}${seed.retrieveMessage ? `: ${seed.retrieveMessage}` : ""}. Unknown stays unknown — never $0.`;
      const resultLine = `Filled: ${filledBits.join(", ") || "none"}. Unknown: ${unknownBits.join(", ") || "none"}. ${retrieveNote}`;
      setWizRetrieveNote(resultLine);
      setActionMessage(`Researched ${seed.symbol}. ${resultLine}`);
      setResearchActivity({
        running: false,
        step: PROCESS_A_TOTAL,
        total: PROCESS_A_TOTAL,
        label: "",
        resultLine,
      });
      await refreshData(asOf);
    } catch (err: unknown) {
      setActionMessage(String(err));
      setWizResearchDone(false);
      setResearchActivity({
        running: false,
        step: 0,
        total: 4,
        label: "",
        resultLine: String(err),
      });
    } finally {
      unlistenProgress?.();
      setBusy(false);
    }
  };

  const completePositionResearch = async (opts: {
    securityId: string;
    symbol: string;
    listenProgress?: boolean;
    /** When true, leave researchActivity.running for the caller (Fill research gaps). */
    keepActivityRunning?: boolean;
  }) => {
    const symbol = opts.symbol.trim().toUpperCase();
    if (!opts.securityId || !symbol) {
      setActionMessage("Choose a researched symbol first.");
      return null;
    }
    const TOTAL = 4;
    setResearchActivity({
      running: true,
      step: 1,
      total: TOTAL,
      label: "retrieving declarations",
      resultLine: null,
    });
    setActionMessage("Complete research: retrieving declarations…");
    let unlistenProgress: (() => void) | undefined;
    try {
      if (opts.listenProgress !== false) {
        try {
          const { listen } = await import("@tauri-apps/api/event");
          unlistenProgress = await listen<{
            step: number;
            total: number;
            label: string;
          }>("position-research-progress", (event) => {
            const p = event.payload;
            if (!p?.label) return;
            setResearchActivity({
              running: true,
              step: p.step ?? 1,
              total: p.total > 0 ? p.total : TOTAL,
              label: p.label,
              resultLine: null,
            });
            setActionMessage(`Complete research: ${p.label}`);
          });
        } catch {
          /* no Tauri events in browser preview */
        }
      }

      const result = await client.executeCommand("PositionResearchRefresh", {
        securityId: opts.securityId,
        symbol,
      });
      if (!result.ok || !result.bodyJson) {
        const fail = `Complete research failed: ${result.errorCode ?? "error"}`;
        setActionMessage(fail);
        setResearchActivity({
          running: false,
          step: 0,
          total: TOTAL,
          label: "",
          resultLine: fail,
        });
        return null;
      }
      const body = JSON.parse(result.bodyJson) as {
        provider?: string;
        underlying?: string;
        paymentFrequency?: string;
        name?: string;
        retrieveOk?: boolean;
        retrieveCode?: string;
        retrieveMessage?: string;
        rocPctMinor?: number | null;
        rocScale?: number;
        rocSourceUrl?: string;
        rocProbes?: { url?: string; status?: number; httpStatus?: number }[];
        needsRocResearch?: boolean;
      };
      // Never treat ticker as underlying on the result line.
      if ((body.underlying || "").toUpperCase() === symbol) {
        body.underlying = "";
      }
      const scale = body.rocScale ?? 2;
      const probes = body.rocProbes ?? [];

      setResearchActivity({
        running: true,
        step: 4,
        total: TOTAL,
        label: "drafting overview",
        resultLine: null,
      });
      setActionMessage("Complete research: drafting overview…");

      const asOf = asOfDate || new Date().toISOString().slice(0, 10);
      const invResult = await client.executeQuery("InvestmentGet", {
        securityId: opts.securityId,
        asOfDate: asOf,
      });
      let inv: InvestmentGet | null = null;
      if (invResult.ok && invResult.bodyJson) {
        inv = JSON.parse(invResult.bodyJson) as InvestmentGet;
      }

      let suggestedTier = "";
      let suggestedReason = "";
      try {
        const sug = await client.executeQuery("ClassificationSuggestGet", {
          securityId: opts.securityId,
        });
        if (sug.ok && sug.bodyJson) {
          const s = JSON.parse(sug.bodyJson) as {
            suggestedTier?: string;
            reason?: string;
          };
          suggestedTier = s.suggestedTier?.trim() || "";
          suggestedReason = s.reason?.trim() || "";
        }
      } catch {
        /* suggestion miss stays blank */
      }

      const provider = body.provider || inv?.provider || "";
      const underlying =
        body.underlying ||
        (inv?.underlying && inv.underlying.toUpperCase() !== symbol
          ? inv.underlying
          : "") ||
        "";
      const freq = body.paymentFrequency || inv?.paymentFrequency || "";
      const fundName =
        inv?.name && inv.name.toUpperCase() !== symbol ? inv.name : "";

      let overview = "";
      try {
        const thesisPrompt = [
          `Write a 4-8 sentence investment overview for ${symbol}.`,
          fundName ? `Legal name: ${fundName}.` : "",
          provider ? `Provider: ${provider}.` : "",
          underlying
            ? `Underlying/look-through is ${underlying} (not the ticker ${symbol}).`
            : "",
          freq ? `Pays ${freq}.` : "",
          inv?.lookthrough?.themeStrategy
            ? `Theme: ${inv.lookthrough.themeStrategy}.`
            : "",
          inv?.lookthrough?.primaryRiskDriver
            ? `Primary risk: ${inv.lookthrough.primaryRiskDriver}.`
            : "",
          "Cover what it is, how it pays, main risks, and that underlying is not the sleeve ticker.",
          suggestedTier
            ? `Also suggest risk tier ${suggestedTier} with a short reason (suggestion only).`
            : "Also suggest a risk tier with a short reason (suggestion only).",
          "Advisory only; do not post facts or apply classification.",
        ]
          .filter(Boolean)
          .join(" ");
        const ai = await client.executeCommand("AiAnalyze", {
          prompt: thesisPrompt,
        });
        if (ai.ok && ai.bodyJson) {
          const run = JSON.parse(ai.bodyJson) as { recommendation?: string };
          overview = run.recommendation?.trim() || "";
        }
      } catch {
        /* overview miss — still show issuer facts */
      }
      if (!overview) {
        overview = [
          fundName || symbol,
          provider ? `is an ${provider} product` : "is an income ETF",
          underlying
            ? `with look-through to ${underlying} (not ${symbol})`
            : "",
          freq ? `paying ${freq}` : "",
          "Distributions and ROC estimates stay unknown until issuer notices parse; never invent 0%.",
          "Risk tier is owner-set; any AI tier below is a suggestion only.",
        ]
          .filter(Boolean)
          .join(". ")
          .replace(/\.\./g, ".");
      }

      const nextSuggested =
        suggestedTier ||
        inv?.lookthrough?.riskTierSuggestion ||
        inv?.suggestion?.suggestedTier ||
        "";
      setResearchNotes({
        overview,
        suggestedTier: nextSuggested,
        suggestedReason:
          suggestedReason ||
          inv?.suggestion?.reason ||
          "Suggestion only — owner sets risk.",
        source: "issuer page + AiAnalyze draft",
      });
      setOwnerRiskChoice(
        RISK_TIERS.includes(nextSuggested) ? nextSuggested : "Undecided",
      );
      setWizAiThesis(overview);

      // Persist overview into blank notes only (holes); never wipe owner notes or lots/plan.
      if (inv && !(inv.notes || "").trim() && overview) {
        try {
          await client.executeCommand("PositionCharacteristicUpsert", {
            securityId: opts.securityId,
            notes: overview,
            paymentFrequency: inv.paymentFrequency || freq || undefined,
            provider: inv.provider || provider || undefined,
            underlying: inv.underlying || underlying || undefined,
            needsRocResearch: true,
            isActive: inv.isActive !== false,
          });
        } catch {
          /* notes persist miss is non-fatal */
        }
      }

      const filledBits = [
        provider ? `provider ${provider}` : null,
        underlying ? `underlying ${underlying}` : null,
        freq ? `frequency ${freq}` : null,
        body.rocPctMinor != null && body.rocPctMinor > 0
          ? `ROC estimate ${(body.rocPctMinor / 10 ** scale).toFixed(scale)}%`
          : null,
        overview ? "research notes" : null,
      ].filter(Boolean);
      const unknownBits = [
        !provider ? "provider" : null,
        !underlying ? "underlying" : null,
        !freq ? "frequency" : null,
        !(body.rocPctMinor != null && body.rocPctMinor > 0)
          ? "ROC estimate"
          : null,
      ].filter(Boolean);

      let resultLine = `Filled: ${filledBits.join(", ") || "none"}. Unknown: ${unknownBits.join(", ") || "none"}.`;
      if (body.rocPctMinor != null && body.rocPctMinor > 0) {
        resultLine += ` ROC ${(body.rocPctMinor / 10 ** scale).toFixed(scale)}%${
          body.rocSourceUrl ? ` — ${body.rocSourceUrl}` : ""
        }. needsRocResearch stays true.`;
      } else {
        const tried =
          probes.length > 0
            ? probes
                .map(
                  (p) =>
                    `${p.url ?? "url"} → ${p.status ?? p.httpStatus ?? "?"}`,
                )
                .join("; ")
            : body.retrieveMessage || "no 19a-1 notice";
        resultLine += ` ROC estimate unknown — not 0%. Tried: ${tried}`;
      }
      setActionMessage(resultLine);
      setResearchActivity({
        running: Boolean(opts.keepActivityRunning),
        step: TOTAL,
        total: TOTAL,
        label: opts.keepActivityRunning ? `Fill research gaps: ${symbol}` : "",
        resultLine,
      });
      return body;
    } catch (err: unknown) {
      const fail = String(err);
      setActionMessage(fail);
      setResearchActivity({
        running: false,
        step: 0,
        total: TOTAL,
        label: "",
        resultLine: fail,
      });
      return null;
    } finally {
      unlistenProgress?.();
    }
  };

  const researchRoc = async () => {
    if (!wizSecurityId) {
      setActionMessage("Save Part 1 before complete research.");
      return;
    }
    setBusy(true);
    setActionMessage(null);
    try {
      const body = await completePositionResearch({
        securityId: wizSecurityId,
        symbol: wizSymbol,
      });
      if (!body) return;
      const asOf = asOfDate || new Date().toISOString().slice(0, 10);
      const invResult = await client.executeQuery("InvestmentGet", {
        securityId: wizSecurityId,
        asOfDate: asOf,
      });
      if (invResult.ok && invResult.bodyJson) {
        const inv = JSON.parse(invResult.bodyJson) as InvestmentGet;
        setWizProvider(inv.provider || "");
        const und =
          inv.underlying &&
          inv.underlying.toUpperCase() !== (wizSymbol || "").toUpperCase()
            ? inv.underlying
            : body.underlying || "";
        setWizUnderlying(und);
        setWizFreq(inv.paymentFrequency || wizFreq);
        if (inv.name && inv.name.toUpperCase() !== wizSymbol.toUpperCase()) {
          setWizName(inv.name);
        }
        setWizLookthrough(mergeLookthrough(inv.lookthrough));
        setWizPriceState(inv.price);
        if (inv.suggestion?.suggestedTier) {
          setWizTierSuggestion(inv.suggestion);
        }
      }
      if (body.rocPctMinor != null && body.rocPctMinor > 0) {
        const scale = body.rocScale ?? 2;
        setWizRoc({
          securityId: wizSecurityId,
          accountId: wizAccountId || null,
          rocPctMinor: body.rocPctMinor,
          scale,
          complete: false,
          source: "19a-1",
          sourceUrl: body.rocSourceUrl || "",
          method: "19a-1-current-year",
          kind: "estimate",
          systemRocPctMinor: body.rocPctMinor,
          candidates: [],
          observations: [],
        } as RocResearchGet);
        setWizRocPct((body.rocPctMinor / 10 ** scale).toFixed(scale));
      }
    } catch (err: unknown) {
      setActionMessage(String(err));
    } finally {
      setBusy(false);
    }
  };

  const confirmRocPlan = async () => {
    if (!wizSecurityId) {
      setActionMessage("Save Part 1 first.");
      return;
    }
    const trimmed = wizRocPct.trim();
    const rocPctMinor = trimmed === "" ? null : Math.round(Number(trimmed) * 100);
    if (trimmed !== "" && !Number.isFinite(rocPctMinor)) {
      setActionMessage("ROC percent is not a number. Leave blank if unknown.");
      return;
    }
    const systemMinor = wizRoc?.systemRocPctMinor ?? null;
    const ownerOverride =
      rocPctMinor != null && systemMinor != null && rocPctMinor !== systemMinor;
    setBusy(true);
    setActionMessage(null);
    try {
      const result = await client.executeCommand("RocPlanConfirm", {
        securityId: wizSecurityId,
        accountId: wizAccountId || undefined,
        rocPctMinor,
        rocScale: 2,
        source: ownerOverride ? "owner-override" : wizRoc?.source || "owner-confirmed",
        sourceUrl: wizRoc?.sourceUrl,
        method: wizRoc?.method,
        asOf: wizRoc?.asOf,
        kind: wizRoc?.kind || "estimate",
        establishedHow: wizRoc?.establishedHow,
        systemRocPctMinor: systemMinor,
        ownerOverride,
        asOfDate: asOfDate || new Date().toISOString().slice(0, 10),
      });
      if (!result.ok || !result.bodyJson) {
        setActionMessage(`ROC plan not stored: ${result.errorCode ?? "error"}`);
        return;
      }
      const body = JSON.parse(result.bodyJson) as RocResearchGet;
      setWizRoc(body);
      setWizStep(8);
      snapshotWiz();
      const magiBit = body.magiEligible
        ? ` Car MAGI planned ordinary remaining ${
            body.remainingOrdinaryMinor == null
              ? "unknown"
              : formatUsd(body.remainingOrdinaryMinor, 2)
          }; ROC ${
            body.remainingRocMinor == null ? "unknown" : formatUsd(body.remainingRocMinor, 2)
          } is not MAGI.`
        : " Choose Car as the first-lot account to write remaining-year MAGI facts.";
      setActionMessage(`Stored ROC plan.${magiBit} Open the first lot.`);
    } catch (err: unknown) {
      setActionMessage(String(err));
    } finally {
      setBusy(false);
    }
  };

  const openFirstLot = async () => {
    if (!wizSecurityId || !wizAccountId) {
      setActionMessage("Choose an account and complete Part 1 first.");
      return;
    }
    setBusy(true);
    setActionMessage(null);
    try {
      const dates = await persistRemainingDates(wizSecurityId, wizPayDraft, wizNextPay);
      if (!dates.ok) {
        setActionMessage(`Remaining dates not stored: ${dates.errorCode ?? "error"}`);
        return;
      }
      const result = await client.executeCommand("LotOpen", {
        accountId: wizAccountId,
        securityId: wizSecurityId,
        openedOn: new Date().toISOString().slice(0, 10),
        origin: "purchase",
        quantityMinor: Number(wizQty),
        quantityScale: 0,
        performanceBasisMinor: dollarsToMinor(wizCost),
        taxBasisMinor: dollarsToMinor(wizCost),
        scale: 2,
        isOpen: true,
      });
      if (!result.ok) {
        setActionMessage(`Lot not stored: ${result.errorCode ?? "error"}`);
        return;
      }
      setWizLotStored(true);
      setWizStep(9);
      snapshotWiz();
      let magiNote = "";
      if (wizRocPct.trim() !== "" || wizRoc != null) {
        const trimmed = wizRocPct.trim();
        const rocPctMinor = trimmed === "" ? null : Math.round(Number(trimmed) * 100);
        const roc = await client.executeCommand("RocPlanConfirm", {
          securityId: wizSecurityId,
          accountId: wizAccountId,
          rocPctMinor,
          rocScale: 2,
          source: wizRoc?.source || "owner-confirmed",
          asOfDate: asOfDate || new Date().toISOString().slice(0, 10),
        });
        if (roc.ok && roc.bodyJson) {
          const body = JSON.parse(roc.bodyJson) as RocResearchGet;
          setWizRoc(body);
          if (body.magiEligible) {
            magiNote = ` Car MAGI planned ordinary ${
              body.remainingOrdinaryMinor == null
                ? "unknown"
                : formatUsd(body.remainingOrdinaryMinor, 2)
            }.`;
          }
        }
      }
      const symbol = wizSymbol.trim().toUpperCase();
      setActionMessage(
        `Stored first lot for ${symbol}.${magiNote} Name this position's Bull and Bear windows. The system does not pick dates.`,
      );
      setPositionSymbol(symbol);
      setPositionFocusPanel("lots");
      await refreshData(asOfDate);
      await loadInvestment(symbol);
      setScreen("position-details");
    } catch (err: unknown) {
      setActionMessage(String(err));
    } finally {
      setBusy(false);
    }
  };

  const recordWizardPeriod = async (kind: "Bull" | "Bear") => {
    const start = kind === "Bull" ? wizBullStart : wizBearStart;
    const end = kind === "Bull" ? wizBullEnd : wizBearEnd;
    if (!start.trim() || !end.trim()) {
      setActionMessage(
        "Bull and Bear need start and end dates. The system does not pick dates.",
      );
      return;
    }
    setBusy(true);
    setActionMessage(null);
    try {
      const result = await client.executeCommand("BacktestPeriodRecord", {
        kind,
        name: `${kind} ${wizSymbol.trim().toUpperCase()}`,
        startOn: start,
        endOn: end,
        recordedAt: asOfDate || new Date().toISOString().slice(0, 10),
      });
      if (!result.ok) {
        setActionMessage(`Period not stored: ${result.errorCode ?? "error"}`);
        return;
      }
      if (kind === "Bull") {
        setWizBullStored(true);
      } else {
        setWizBearStored(true);
      }
      snapshotWiz();
      const otherStored = kind === "Bull" ? wizBearStored : wizBullStored;
      setActionMessage(
        otherStored
          ? `Stored ${kind} window. Bull and Bear dates are named. Open Position Details to calculate.`
          : `Stored ${kind} window. Name the ${kind === "Bull" ? "Bear" : "Bull"} window next.`,
      );
    } catch (err: unknown) {
      setActionMessage(String(err));
    } finally {
      setBusy(false);
    }
  };

  const factsDirty = pdDraft != null && JSON.stringify(pdDraft) !== pdBaseline;
  const periodDirty = JSON.stringify(pdPeriod) !== pdPeriodBaseline;
  const pdDirty = factsDirty || (investment != null && periodDirty);
  const dirtyTargets: Screen[] = [
    ...(pdDirty ? (["position-details"] as const) : []),
    ...(wizDirty ? (["new-investment"] as const) : []),
    ...(addLotDirty ? (["add-lot"] as const) : []),
  ];
  const dirtyScreenNames = [
    pdDirty ? "Position Details" : null,
    wizDirty ? "Add Position" : null,
    addLotDirty ? "Add Lot" : null,
  ]
    .filter((name): name is string => name != null)
    .join(" and ");
  const periodReady =
    PERIOD_KINDS.includes(pdPeriod.kind) &&
    pdPeriod.startOn.trim() !== "" &&
    pdPeriod.endOn.trim() !== "";

  const patchPeriod = (patch: Partial<PdPeriodDraft>) => {
    setPdPeriod((current) => ({ ...current, ...patch }));
  };

  const patchDraft = (patch: Partial<PdDraft>) => {
    setPdDraft((current) => (current ? { ...current, ...patch } : current));
  };

  const cancelPositionEdits = () => {
    if (!pdBaseline) {
      setPdDraft(null);
      setPdPeriod(emptyPeriod());
      setPdPeriodBaseline(JSON.stringify(emptyPeriod()));
      return;
    }
    setPdDraft(JSON.parse(pdBaseline) as PdDraft);
    setPdPeriod(JSON.parse(pdPeriodBaseline) as PdPeriodDraft);
    setActionMessage("Edits discarded.");
  };

  const discardUnsavedEdits = () => {
    if (pdDirty) cancelPositionEdits();
    if (wizDirty) cancelWizEdits();
    if (addLotDirty) cancelAddLotEdits();
  };

  const saveOwnerPeriod = async () => {
    if (!periodReady) {
      setActionMessage("Period needs kind and dates.");
      return;
    }
    setBusy(true);
    setActionMessage(null);
    try {
      const result = await client.executeCommand("BacktestPeriodRecord", {
        kind: pdPeriod.kind,
        name: pdPeriod.name.trim() || `${pdPeriod.kind} ${pdPeriod.startOn}`,
        startOn: pdPeriod.startOn,
        endOn: pdPeriod.endOn,
        benchmarkSymbol: pdPeriod.benchmark.trim(),
      });
      if (!result.ok || !result.bodyJson) {
        setActionMessage(`Period not stored: ${result.errorCode ?? "error"}`);
        return;
      }
      const body = JSON.parse(result.bodyJson) as { periodId?: string };
      if (body.periodId) {
        setSavedPeriodId(body.periodId);
      }
      setPdPeriodBaseline(JSON.stringify(pdPeriod));
      setActionMessage("Owner period stored. Calculate window when ready.");
      if (investment) {
        await loadInvestment(investment.symbol);
      }
    } catch (err: unknown) {
      setActionMessage(String(err));
    } finally {
      setBusy(false);
    }
  };

  const calculateWindow = async () => {
    if (!investment) {
      setActionMessage("Choose a symbol first.");
      return;
    }
    const periodId = savedPeriodId || investment.periods.at(-1)?.periodId;
    if (!periodId) {
      setActionMessage("Save an owner period before calculating.");
      return;
    }
    const period =
      investment.periods.find((p) => p.periodId === periodId) ?? {
        startOn: pdPeriod.startOn,
        endOn: pdPeriod.endOn,
        benchmarkSymbol: pdPeriod.benchmark,
      };
    if (!period.startOn || !period.endOn) {
      setActionMessage("Period dates are required before calculating.");
      return;
    }
    setBusy(true);
    setActionMessage(null);
    try {
      const series = await client.executeCommand("PeriodSeriesRetrieve", {
        symbol: investment.symbol,
        startOn: period.startOn,
        endOn: period.endOn,
        benchmarkSymbol: period.benchmarkSymbol,
      });
      if (!series.ok || !series.bodyJson) {
        setActionMessage(`Window retrieve missed: ${series.errorCode ?? "error"}`);
        return;
      }
      const retrieved = JSON.parse(series.bodyJson) as {
        candidates?: unknown[];
        benchmarkCandidates?: unknown[];
        posted?: boolean;
      };
      if (retrieved.posted) {
        setActionMessage("Window retrieve must stay candidates; nothing posted.");
        return;
      }
      const calc = await client.executeCommand("PositionBacktestCalculate", {
        securityId: investment.securityId,
        periodId,
        candidates: retrieved.candidates ?? [],
        benchmarkCandidates: retrieved.benchmarkCandidates ?? [],
      });
      if (!calc.ok) {
        setActionMessage(`Calculate window failed: ${calc.errorCode ?? "error"}`);
        return;
      }
      setActionMessage("Window calculated. Suggestion does not change Risk until you Apply.");
      await loadInvestment(investment.symbol);
    } catch (err: unknown) {
      setActionMessage(String(err));
    } finally {
      setBusy(false);
    }
  };

  const applySuggestedTier = async (tier: string) => {
    if (!investment) {
      return;
    }
    setBusy(true);
    setActionMessage(null);
    try {
      const result = await client.executeCommand("ClassificationApply", {
        securityId: investment.securityId,
        riskTier: tier,
      });
      if (!result.ok) {
        setActionMessage(`Apply failed: ${result.errorCode ?? "error"}`);
        return;
      }
      setActionMessage(`Applied ${tier}.`);
      await refreshData(asOfDate);
      await loadInvestment(investment.symbol, { skipPriceRefresh: true });
    } catch (err: unknown) {
      setActionMessage(String(err));
    } finally {
      setBusy(false);
    }
  };

  const setOwnerRisk = async () => {
    if (!investment) {
      return;
    }
    if (!RISK_TIERS.includes(ownerRiskChoice)) {
      setActionMessage(
        "Choose Foundation, Core, or Risk On to set risk. Or Leave undecided.",
      );
      return;
    }
    await applySuggestedTier(ownerRiskChoice);
    setPdDraft((prev) => (prev ? { ...prev, risk: ownerRiskChoice } : prev));
  };

  const leaveRiskUndecided = async () => {
    if (!investment) {
      return;
    }
    const cadence =
      parseCadence(pdDraft?.freq || investment.paymentFrequency || "")?.label ||
      investment.paymentFrequency ||
      "";
    if (!cadence) {
      setActionMessage("Frequency must be set before leaving risk undecided.");
      return;
    }
    setBusy(true);
    setActionMessage(null);
    try {
      const result = await client.executeCommand("PositionCharacteristicUpsert", {
        securityId: investment.securityId,
        paymentFrequency: cadence,
        riskTier: "Undecided",
        provider: investment.provider || pdDraft?.provider || undefined,
        underlying: investment.underlying || pdDraft?.underlying || undefined,
        needsRocResearch: true,
        isActive: investment.isActive !== false,
      });
      if (!result.ok) {
        setActionMessage(
          `Leave undecided failed: ${result.errorCode ?? "error"}`,
        );
        return;
      }
      setOwnerRiskChoice("Undecided");
      setPdDraft((prev) => (prev ? { ...prev, risk: "Undecided" } : prev));
      setActionMessage("Risk left undecided. Overview stays; not auto-applied.");
      await refreshData(asOfDate);
      await loadInvestment(investment.symbol, { skipPriceRefresh: true });
    } catch (err: unknown) {
      setActionMessage(String(err));
    } finally {
      setBusy(false);
    }
  };

  const leaveWithoutSaving = (next: () => void, target?: Screen) => {
    if (
      (pdDirty || wizDirty || addLotDirty) &&
      (target == null || !dirtyTargets.includes(target))
    ) {
      setActionMessage(
        "Save or Cancel before leaving. Navigation stays blocked while edits are unsaved.",
      );
      return;
    }
    next();
  };

  const openPositionHub = (
    symbol: string,
    focus: "lots" | "income" | "declarations" | "ledger" | "" = "",
  ) => {
    leaveWithoutSaving(() => {
      setPositionFocusPanel(focus);
      setPositionSymbol(symbol);
      setScreen("position-details");
      void loadInvestment(symbol);
    });
  };

  const saveStoredFacts = async () => {
    if (!investment || !pdDraft) {
      setActionMessage("Choose a symbol first.");
      return;
    }
    if (!OWNER_RISK_CHOICES.includes(pdDraft.risk)) {
      setActionMessage(
        "Choose Foundation, Core, Risk On, or Undecided before saving.",
      );
      return;
    }
    const cadence = parseCadence(pdDraft.freq);
    if (!cadence) {
      setActionMessage(
        "Choose Weekly (52), Monthly (12), Quarterly (4), or None (does not pay). Cadence is required; there is no default.",
      );
      return;
    }
    const planTyped = pdDraft.plan.trim() !== "";
    if (planTyped && !pdDraft.planReason) {
      setActionMessage("Choose a Plan reason before saving Plan.");
      return;
    }
    if (
      planTyped &&
      investment.review.incompleteReasonRequired &&
      !pdDraft.incomplete
    ) {
      setActionMessage("Choose why full Plan analysis is not possible.");
      return;
    }
    setBusy(true);
    setActionMessage(null);
    try {
      const renamed = await client.executeCommand("SecurityUpdate", {
        securityId: investment.securityId,
        name: pdDraft.name.trim(),
      });
      if (!renamed.ok) {
        setActionMessage(`Name not stored: ${renamed.errorCode ?? "error"}`);
        return;
      }
      const chars = await client.executeCommand("PositionCharacteristicUpsert", {
        securityId: investment.securityId,
        paymentFrequency: cadence.label,
        riskTier: pdDraft.risk,
        provider: pdDraft.provider.trim(),
        underlying: pdDraft.underlying.trim(),
        lookthrough: pdDraft.lookthrough,
        notes: pdDraft.notes,
        divType: pdDraft.divType.trim(),
        needsRocResearch: pdDraft.needsRoc,
        isActive: pdDraft.isActive,
        rocPct2024ActualMinor: rocActualUnavailable(
          investment.lots,
          2024,
          asOfDate,
        )
          ? null
          : rocMinor(pdDraft.roc2024),
        rocPct2025ActualMinor: rocActualUnavailable(
          investment.lots,
          2025,
          asOfDate,
        )
          ? null
          : rocMinor(pdDraft.roc2025),
        rocPct2026EstimateMinor: rocMinor(pdDraft.roc2026e),
        rocPct2026ActualMinor: rocActualUnavailable(
          investment.lots,
          2026,
          asOfDate,
        )
          ? null
          : rocMinor(pdDraft.roc2026a),
        rocScale: 2,
      });
      if (!chars.ok) {
        setActionMessage(`Characteristics not stored: ${chars.errorCode ?? "error"}`);
        return;
      }
      const template = await client.executeCommand("RetrievalTemplateSet", {
        securityId: investment.securityId,
        priceSource:
          investment.template?.priceSource?.trim() ||
          pdDraft.priceSource.trim() ||
          "public",
        sourceSymbol:
          investment.template?.sourceSymbol?.trim() ||
          pdDraft.sourceSymbol.trim() ||
          investment.symbol,
        declarationSource:
          investment.template?.declarationSource?.trim() ||
          pdDraft.declarationSource.trim(),
        lookbackCount:
          investment.template?.lookbackCount ??
          (Number(pdDraft.lookbackCount) || DECLARATION_LOOKBACK_TARGET),
        sourceUrl:
          investment.template?.sourceUrl?.trim() ?? pdDraft.sourceUrl.trim(),
        calendarPolicy:
          investment.template?.calendarPolicy?.trim() ||
          pdDraft.calendarPolicy.trim(),
        collectorEnabled: investment.template?.collectorEnabled,
        inceptionOn: investment.template?.inceptionOn?.trim() ?? "",
      });
      if (!template.ok) {
        setActionMessage(`Retrieval template not stored: ${template.errorCode ?? "error"}`);
        return;
      }
      const pattern = await client.executeCommand("ExpectedPaymentPatternUpsert", {
        securityId: investment.securityId,
        declarationWeekday: pdDraft.declarationWeekday,
        exdateWeekday: pdDraft.exdateWeekday,
        paydayWeekday: pdDraft.paydayWeekday,
      });
      if (!pattern.ok) {
        setActionMessage(`Weekday pattern not stored: ${pattern.errorCode ?? "error"}`);
        return;
      }
      const tax = await client.executeCommand("PositionTaxProfileUpsert", {
        securityId: investment.securityId,
        expectedHandling: pdDraft.taxHandling,
      });
      if (!tax.ok) {
        setActionMessage(`Tax handling not stored: ${tax.errorCode ?? "error"}`);
        return;
      }
      let planNote = "";
      if (planTyped) {
        const cadence = parseCadence(pdDraft.freq || investment.paymentFrequency);
        if (!cadence) {
          setActionMessage(
            "Choose Weekly (52), Monthly (12), Quarterly (4), or None before Confirm Plan. There is no default.",
          );
          return;
        }
        const planScale = 4;
        const planMinor = Math.round(Number(pdDraft.plan) * 10 ** planScale);
        const planResult = await client.executeCommand("PlanHistoryConfirm", {
          securityId: investment.securityId,
          amountPerShareMinor: planMinor,
          amountScale: planScale,
          planningPeriodsPerYear: cadence.periods,
          effectiveFrom: new Date().toISOString().slice(0, 10),
          decisionReason: pdDraft.planReason,
          incompleteAnalysisReason: pdDraft.incomplete,
        });
        if (!planResult.ok) {
          setActionMessage(`Facts stored; Plan not stored: ${planResult.errorCode ?? "error"}`);
      await refreshData(asOfDate);
      await loadInvestment(investment.symbol, { skipPriceRefresh: true });
          return;
        }
        planNote = `; Plan ${pdDraft.plan}/share`;
      }
      setActionMessage(`Stored ${investment.symbol}${planNote}.`);
      await refreshData(asOfDate);
      await loadInvestment(investment.symbol, { skipPriceRefresh: true });
    } catch (err: unknown) {
      setActionMessage(String(err));
    } finally {
      setBusy(false);
    }
  };

  const refreshPdLastPrice = async () => {
    if (!investment || !pdDraft) {
      setActionMessage("Choose a symbol first.");
      return;
    }
    setBusy(true);
    setActionMessage(null);
    try {
      const result = await client.executeCommand("MarketRetrieve", {
        symbol: investment.symbol,
        priceSource: pdDraft.priceSource,
        sourceSymbol: pdDraft.sourceSymbol,
      });
      if (!result.ok || !result.bodyJson) {
        setActionMessage(`Last price retrieve missed: ${result.errorCode ?? "error"}`);
        return;
      }
      const body = JSON.parse(result.bodyJson) as {
        quote?: { priceMinor?: number; scale?: number; asOfAt?: string; source?: string } | null;
      };
      const quote = body.quote;
      if (!quote?.priceMinor || quote.priceMinor <= 0) {
        await refreshData(asOfDate);
        await loadInvestment(investment.symbol, { skipPriceRefresh: true });
        return;
      }
      const posted = await client.executeCommand("PriceQuoteRecord", {
        securityId: investment.securityId,
        priceMinor: quote.priceMinor,
        scale: quote.scale ?? 2,
        asOfAt: quote.asOfAt ?? new Date().toISOString().slice(0, 10),
        source: quote.source ?? "yahoo",
      });
      if (!posted.ok) {
        setActionMessage(`Last price not stored: ${posted.errorCode ?? "error"}`);
        return;
      }
      setActionMessage(
        `Stored last price ${formatUsd(quote.priceMinor, quote.scale ?? 2)} for ${investment.symbol}.`,
      );
      await refreshData(asOfDate);
      await loadInvestment(investment.symbol, { skipPriceRefresh: true });
    } catch (err: unknown) {
      setActionMessage(String(err));
    } finally {
      setBusy(false);
    }
  };

  const refreshPdDeclarations = async () => {
    if (!investment || !pdDraft) {
      setActionMessage("Choose a symbol first.");
      return;
    }
    setBusy(true);
    setActionMessage(null);
    try {
      const result = await client.executeCommand("MarketRetrieve", {
        symbol: investment.symbol,
        securityId: investment.securityId,
        declarationSource: investment.template?.declarationSource ?? "",
        sourceSymbol: investment.template?.sourceSymbol ?? investment.symbol,
        priceSource: investment.template?.priceSource ?? "",
        sourceUrl: investment.template?.sourceUrl ?? pdDraft.sourceUrl,
        knownPaymentPeriods: (investment.declarations ?? [])
          .map((d) => d.paymentPeriod?.trim())
          .filter((p): p is string => Boolean(p)),
      });
      if (!result.ok || !result.bodyJson) {
        setActionMessage(`Declaration retrieve missed: ${result.errorCode ?? "error"}`);
        return;
      }
      const body = JSON.parse(result.bodyJson) as {
        candidates?: Array<{
          amountPerShareMinor?: number;
          amountScale?: number;
          paymentPeriod?: string;
          source?: string;
        }>;
        upcomingPays?: Array<{ payOn?: string; source?: string }>;
        missExplanation?: string;
      };
      const existingPeriods = new Set(
        (investment.declarations ?? [])
          .map((d) => d.paymentPeriod?.trim())
          .filter((p): p is string => Boolean(p)),
      );
      const retrieved = (body.candidates ?? []).filter(
        (d) =>
          d.source &&
          d.amountPerShareMinor != null &&
          d.amountPerShareMinor > 0 &&
          d.paymentPeriod?.trim() &&
          !existingPeriods.has(d.paymentPeriod.trim()),
      );
      if (retrieved.length === 0) {
        await client.executeCommand("DeclarationRefresh", {
          misses: [{
            securityId: investment.securityId,
            symbol: investment.symbol,
            code: "declaration_retrieve_miss",
            reason: body.missExplanation || "Issuer page empty.",
          }],
        });
        setActionMessage("Issuer page empty.");
        await refreshData(asOfDate);
        return;
      }
      let posted = 0;
      for (const [i, cand] of retrieved.slice(0, 12).entries()) {
        const rec = await client.executeCommand("IssuerDeclarationRecord", {
          securityId: investment.securityId,
          amountPerShareMinor: cand.amountPerShareMinor,
          amountScale: cand.amountScale ?? 4,
          paymentPeriod: cand.paymentPeriod ?? `period-${i}`,
          source: cand.source,
          enteredAt: new Date().toISOString().slice(0, 10),
        });
        if (rec.ok) posted += 1;
      }
      const upcoming = (body.upcomingPays ?? []).filter((p) => p.payOn);
      if (
        upcoming.length > 0 &&
        (investment.template?.calendarPolicy === "issuer_calendar" ||
          pdDraft.calendarPolicy === "issuer_calendar")
      ) {
        await client.executeCommand("IssuerPayDateReplace", {
          securityId: investment.securityId,
          asOfDate: new Date().toISOString().slice(0, 10),
          dates: upcoming.map((p) => ({
            payOn: p.payOn,
            source: p.source || investment.template?.declarationSource || "issuer",
          })),
        });
      }
      if (posted === 0) {
        setActionMessage("Declarations not stored. Miss stays unknown.");
        return;
      }
      setActionMessage(`Stored ${formatCount(posted)} declarations for ${investment.symbol}.`);
      await refreshData(asOfDate);
      await loadInvestment(investment.symbol, { skipPriceRefresh: true });
    } catch (err: unknown) {
      setActionMessage(String(err));
    } finally {
      setBusy(false);
    }
  };

  const researchPdRoc = async () => {
    if (!investment) {
      setActionMessage("Choose a symbol first.");
      return;
    }
    setBusy(true);
    setActionMessage(null);
    try {
      await completePositionResearch({
        securityId: investment.securityId,
        symbol: investment.symbol,
      });
      await loadInvestment(investment.symbol, { skipPriceRefresh: true });
    } catch (err: unknown) {
      setActionMessage(String(err));
    } finally {
      setBusy(false);
    }
  };

  const fillResearchGaps = async () => {
    setBusy(true);
    setCollectorAction("Loading research gaps…");
    setActionMessage("Loading research gaps…");
    setResearchActivity({
      running: true,
      step: 0,
      total: 1,
      label: "loading research gaps",
      resultLine: null,
    });
    try {
      const gapsResult = await client.executeQuery("ResearchGapsGet", {});
      if (!gapsResult.ok || !gapsResult.bodyJson) {
        const msg = `Research gaps query failed: ${gapsResult.errorCode ?? "error"}`;
        setCollectorAction(msg);
        setActionMessage(msg);
        setResearchActivity({
          running: false,
          step: 0,
          total: 1,
          label: "",
          resultLine: msg,
        });
        return;
      }
      const gaps = JSON.parse(gapsResult.bodyJson) as {
        items: { securityId: string; symbol: string }[];
      };
      const targets = gaps.items ?? [];
      if (targets.length === 0) {
        const msg = "No research gaps — enabled income names have provider, underlying, frequency, and ROC observation.";
        setCollectorAction(msg);
        setActionMessage(msg);
        setCollectorRunProgress(null);
        setResearchActivity({
          running: false,
          step: 0,
          total: 1,
          label: "",
          resultLine: msg,
        });
        return;
      }
      setCollectorRunProgress({
        running: true,
        total: targets.length,
        current: 0,
        symbol: "",
        ok: 0,
        miss: 0,
        lines: [],
      });
      setResearchActivity({
        running: true,
        step: 0,
        total: targets.length,
        label: `Fill research gaps (0 of ${targets.length})`,
        resultLine: null,
      });
      let ok = 0;
      let miss = 0;
      const lines: string[] = [];
      const { flushSync } = await import("react-dom");
      for (let i = 0; i < targets.length; i++) {
        const row = targets[i];
        flushSync(() => {
          setCollectorRunProgress({
            running: true,
            total: targets.length,
            current: i + 1,
            symbol: row.symbol,
            ok,
            miss,
            lines: [...lines],
          });
          setResearchActivity({
            running: true,
            step: i + 1,
            total: targets.length,
            label: `Fill research gaps: ${row.symbol}`,
            resultLine: null,
          });
          setCollectorAction(
            `Fill research gaps ${formatCount(i + 1)} of ${formatCount(targets.length)}: ${row.symbol}…`,
          );
        });
        await new Promise<void>((resolve) => {
          window.setTimeout(() => resolve(), 0);
        });
        try {
          const body = await completePositionResearch({
            securityId: row.securityId,
            symbol: row.symbol,
            listenProgress: true,
            keepActivityRunning: true,
          });
          if (!body) {
            miss += 1;
            lines.push(`${row.symbol}: fail`);
          } else {
            ok += 1;
            let note = "ok";
            if (body.rocPctMinor != null && body.rocPctMinor > 0) {
              note = `ROC ${body.rocSourceUrl || "estimate"}`;
            } else if ((body.rocProbes?.length ?? 0) > 0) {
              note = `19a-1 miss (${body.rocProbes!.length} URLs)`;
            } else {
              note = `provider ${body.provider || "—"}`;
            }
            lines.push(`${row.symbol}: ${note}`);
          }
        } catch (err: unknown) {
          miss += 1;
          lines.push(`${row.symbol}: ${String(err)}`);
        }
        if (lines.length > 40) {
          lines.splice(0, lines.length - 40);
        }
        flushSync(() => {
          setCollectorRunProgress({
            running: true,
            total: targets.length,
            current: i + 1,
            symbol: row.symbol,
            ok,
            miss,
            lines: [...lines],
          });
        });
      }
      await refreshCollectors(asOfDate);
      const done = `Fill research gaps finished: ${formatCount(ok)} ok, ${formatCount(miss)} miss of ${formatCount(targets.length)}.`;
      setCollectorAction(done);
      setActionMessage(done);
      setResearchActivity({
        running: false,
        step: targets.length,
        total: targets.length,
        label: "",
        resultLine: done,
      });
      setCollectorRunProgress({
        running: false,
        total: targets.length,
        current: targets.length,
        symbol: "",
        ok,
        miss,
        lines: [...lines],
      });
    } catch (err: unknown) {
      const msg = String(err);
      setCollectorAction(msg);
      setActionMessage(msg);
      setResearchActivity({
        running: false,
        step: 0,
        total: 1,
        label: "",
        resultLine: msg,
      });
    } finally {
      setBusy(false);
    }
  };

  const openAddLot = async () => {
    if (!addLotSecurityId || !addLotAccountId) {
      setActionMessage("Choose a researched symbol and account.");
      return;
    }
    if (!addLotOpenedOn.trim()) {
      setActionMessage("Opened-on date is required.");
      return;
    }
    if (!LOT_ORIGINS.includes(addLotOrigin as (typeof LOT_ORIGINS)[number])) {
      setActionMessage("Choose origin: purchase, drip, or transfer.");
      return;
    }
    if (!addLotQty.trim() || !addLotCost.trim()) {
      setActionMessage("Enter quantity and unit original $.");
      return;
    }
    if (addLotTaxDifferent && !addLotTaxCost.trim()) {
      setActionMessage("Enter unit tax $, or uncheck Unit tax $ different.");
      return;
    }
    const qtyMinor = Number(addLotQty);
    const quantityScale = 0;
    if (!Number.isFinite(qtyMinor) || qtyMinor <= 0) {
      setActionMessage("Quantity must be a positive number.");
      return;
    }
    const unitOrigCents = dollarsToMinor(addLotCost);
    const unitTaxCents = addLotTaxDifferent
      ? dollarsToMinor(addLotTaxCost)
      : unitOrigCents;
    if (!Number.isFinite(unitOrigCents) || unitOrigCents < 0) {
      setActionMessage("Unit original $ is not a number.");
      return;
    }
    if (!Number.isFinite(unitTaxCents) || unitTaxCents < 0) {
      setActionMessage("Unit tax $ is not a number.");
      return;
    }
    const performance = lotTotalFromUnitCents(qtyMinor, quantityScale, unitOrigCents);
    const tax = lotTotalFromUnitCents(qtyMinor, quantityScale, unitTaxCents);
    const symbol =
      securities.find((s) => s.securityId === addLotSecurityId)?.symbol ||
      addLotQuery.split("—")[0]?.trim() ||
      "symbol";
    const accountName =
      accounts.find((a) => a.accountId === addLotAccountId)?.name || "account";
    const qtyLabel = formatScaled(qtyMinor, quantityScale);
    const unitOrigLabel = formatMoneyInput(addLotCost);
    const lotOrigLabel = (performance / 100).toLocaleString("en-US", {
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
    });
    const unitTaxLabel = addLotTaxDifferent
      ? formatMoneyInput(addLotTaxCost)
      : unitOrigLabel;
    const lotTaxLabel = (tax / 100).toLocaleString("en-US", {
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
    });
    setBusy(true);
    setActionMessage(null);
    try {
      const result = await client.executeCommand("LotOpen", {
        accountId: addLotAccountId,
        securityId: addLotSecurityId,
        openedOn: addLotOpenedOn,
        origin: addLotOrigin,
        quantityMinor: qtyMinor,
        quantityScale,
        performanceBasisMinor: performance,
        taxBasisMinor: tax,
        scale: 2,
        isOpen: true,
      });
      if (!result.ok) {
        setActionMessage(
          `LotOpen failed: ${result.errorCode ?? "error"}. Fields kept — fix and retry.`,
        );
        await refreshHandoff();
        return;
      }
      // Success: keep symbol (and account/origin); clear qty/cost/date so submit is not a duplicate.
      setAddLotOpenedOn("");
      setAddLotQty("");
      setAddLotCost("");
      setAddLotTaxCost("");
      setAddLotTaxDifferent(false);
      const cleared: AddLotDraft = {
        securityId: addLotSecurityId,
        accountId: addLotAccountId,
        openedOn: "",
        qty: "",
        cost: "",
        taxCost: "",
        taxCostDifferent: false,
        origin: addLotOrigin,
      };
      setAddLotBaseline(JSON.stringify(cleared));
      setActionMessage(
        `Opened lot: ${symbol} in ${accountName}, qty ${qtyLabel} × $${unitOrigLabel} = $${lotOrigLabel} lot orig` +
          (addLotTaxDifferent
            ? `; unit tax $${unitTaxLabel} → lot tax $${lotTaxLabel}`
            : "") +
          `. Enter qty and unit $ again for another lot on ${symbol}.`,
      );
      await refreshHandoff();
      await refreshData(asOfDate);
    } catch (err: unknown) {
      setActionMessage(String(err));
    } finally {
      setBusy(false);
    }
  };

  const importBrokerCsv = async (file: File) => {
    setBusy(true);
    setActionMessage(null);
    const lines: string[] = [`CSV  ${file.name}`];
    setCaptureProcess({
      running: true,
      title: "CSV import",
      current: 0,
      total: 3,
      detail: `Staging ${file.name}`,
      lines: [...lines],
      posted: 0,
      skipped: 0,
      errors: 0,
    });
    try {
      const content = await file.text();
      const staged = await client.executeCommand("ImportStage", {
        sourceId: `ui-file-${file.name}`,
        filename: file.name,
        content,
      });
      if (!staged.ok || !staged.bodyJson) {
        const msg = `ImportStage failed: ${staged.errorCode ?? "error"}`;
        lines.push(msg);
        setCaptureProcess((p) =>
          p ? { ...p, running: false, errors: 1, detail: msg, lines: [...lines] } : p,
        );
        setActionMessage(msg);
        return;
      }
      const stagedBody = JSON.parse(staged.bodyJson) as {
        batchId?: string;
        candidateCount?: number;
        status?: string;
      };
      const batchId = stagedBody.batchId;
      if (!batchId) {
        const msg = "ImportStage failed: missing batchId";
        lines.push(msg);
        setCaptureProcess((p) =>
          p ? { ...p, running: false, errors: 1, detail: msg, lines: [...lines] } : p,
        );
        setActionMessage(msg);
        return;
      }
      setPendingBatchId(batchId);
      const n = stagedBody.candidateCount ?? 0;
      lines.push(`staged  ${n} candidate${n === 1 ? "" : "s"}  ${batchId}`);
      setCaptureProcess((p) =>
        p
          ? { ...p, current: 1, detail: "Validating", lines: [...lines] }
          : p,
      );
      const validated = await client.executeCommand("ImportValidate", { batchId });
      const status =
        validated.bodyJson != null
          ? (JSON.parse(validated.bodyJson) as { status?: string }).status ?? "unknown"
          : "unknown";
      setPendingBatchStatus(status);
      lines.push(`validate  ${status}`);
      await refreshData(asOfDate);
      if (!validated.ok || status !== "validated") {
        const msg = `Staged ${batchId}; status ${status}. Review exceptions before approve/post.`;
        lines.push(msg);
        setCaptureProcess((p) =>
          p
            ? { ...p, running: false, current: 2, errors: 1, detail: msg, lines: [...lines] }
            : p,
        );
        setActionMessage(msg);
        return;
      }
      setCaptureProcess((p) =>
        p
          ? {
              ...p,
              running: false,
              current: 2,
              detail: "Validated. Approve, then post.",
              lines: [...lines],
            }
          : p,
      );
      setActionMessage(`Staged and validated ${batchId}. Approve, then post.`);
    } catch (err: unknown) {
      const msg = String(err);
      lines.push(msg);
      setCaptureProcess((p) =>
        p ? { ...p, running: false, errors: 1, detail: msg, lines: [...lines] } : p,
      );
      setActionMessage(msg);
    } finally {
      setBusy(false);
    }
  };

  const updateManualDividendRow = (id: string, patch: Partial<ManualDividendRow>) => {
    setManualDividendRows((rows) =>
      rows.map((row) => (row.id === id ? { ...row, ...patch } : row)),
    );
  };

  const postManualDividends = async () => {
    const started = manualDividendRows.filter(
      (row) =>
        row.accountId ||
        row.ticker.trim() ||
        row.occurredOn ||
        row.amount.trim(),
    );
    const complete = started.filter(
      (row) =>
        row.accountId &&
        row.ticker.trim() &&
        row.occurredOn &&
        row.amount.trim(),
    );
    if (complete.length !== started.length) {
      setActionMessage("Fill account, ticker, date, and amount on each started row.");
      return;
    }
    if (complete.length === 0) {
      setActionMessage("Enter at least one dividend row.");
      return;
    }
    setBusy(true);
    setActionMessage(null);
    const lines: string[] = [];
    let posted = 0;
    let skipped = 0;
    let errorCount = 0;
    const postedIds: string[] = [];
    setCaptureProcess({
      running: true,
      title: "Manual dividend",
      current: 0,
      total: complete.length,
      detail: "Starting",
      lines: [],
      posted: 0,
      skipped: 0,
      errors: 0,
    });
    try {
      for (let i = 0; i < complete.length; i += 1) {
        const row = complete[i];
        const ticker = row.ticker.trim().toUpperCase();
        const accountName =
          accounts.find((a) => a.accountId === row.accountId)?.name || "account";
        const amountMinor = totalDollarsToMinor(row.amount);
        const amountLabel =
          amountMinor == null ? row.amount.trim() : `$${(amountMinor / 100).toFixed(2)}`;
        setCaptureProcess((p) =>
          p
            ? {
                ...p,
                current: i + 1,
                detail: `${accountName}  ${ticker}  ${row.occurredOn}  ${amountLabel}`,
              }
            : p,
        );
        const security = securities.find((s) => s.symbol.toUpperCase() === ticker);
        if (!security) {
          errorCount += 1;
          const line = `error  ${accountName}  ${ticker}  ${row.occurredOn}  unknown ticker`;
          lines.push(line);
          setCaptureProcess((p) =>
            p ? { ...p, errors: errorCount, lines: [...lines] } : p,
          );
          continue;
        }
        if (amountMinor == null) {
          errorCount += 1;
          const line = `error  ${accountName}  ${ticker}  ${row.occurredOn}  amount must be the positive cash total`;
          lines.push(line);
          setCaptureProcess((p) =>
            p ? { ...p, errors: errorCount, lines: [...lines] } : p,
          );
          continue;
        }
        const result = await client.executeCommand("DividendActualRecord", {
          accountId: row.accountId,
          securityId: security.securityId,
          occurredOn: row.occurredOn,
          amountMinor,
          scale: 2,
          idempotencyKey: `manual-${row.accountId}-${security.securityId}-${row.occurredOn}-${amountMinor}`,
        });
        if (!result.ok) {
          errorCount += 1;
          const line = `error  ${accountName}  ${ticker}  ${row.occurredOn}  ${result.errorCode ?? "error"}`;
          lines.push(line);
          setCaptureProcess((p) =>
            p ? { ...p, errors: errorCount, lines: [...lines] } : p,
          );
          continue;
        }
        let already = false;
        if (result.bodyJson) {
          try {
            already = Boolean(
              (JSON.parse(result.bodyJson) as { alreadyPosted?: boolean }).alreadyPosted,
            );
          } catch {
            already = false;
          }
        }
        if (already) {
          skipped += 1;
          lines.push(
            `skipped duplicate  ${accountName}  ${ticker}  ${row.occurredOn}  ${amountLabel}`,
          );
        } else {
          posted += 1;
          lines.push(`posted  ${accountName}  ${ticker}  ${row.occurredOn}  ${amountLabel}`);
        }
        postedIds.push(row.id);
        setCaptureProcess((p) =>
          p
            ? { ...p, posted, skipped, errors: errorCount, lines: [...lines] }
            : p,
        );
      }
      setManualDividendRows((rows) => {
        const kept = rows.filter((row) => !postedIds.includes(row.id));
        while (kept.length < 3) {
          kept.push(blankManualDividendRow());
        }
        return kept;
      });
      const summary = `Posted ${posted}, skipped ${skipped} duplicate${skipped === 1 ? "" : "s"}, ${errorCount} error${errorCount === 1 ? "" : "s"}.`;
      setCaptureProcess((p) =>
        p ? { ...p, running: false, detail: summary, lines: [...lines] } : p,
      );
      setActionMessage(summary);
      await refreshHandoff();
      await refreshData(asOfDate);
    } catch (err: unknown) {
      const msg = String(err);
      lines.push(msg);
      setCaptureProcess((p) =>
        p ? { ...p, running: false, errors: errorCount + 1, detail: msg, lines: [...lines] } : p,
      );
      setActionMessage(msg);
    } finally {
      setBusy(false);
    }
  };

  const runPendingImport = async (name: "ImportValidate" | "ImportApprove" | "ImportPost") => {
    if (!pendingBatchId) {
      setActionMessage("No staged import batch");
      return;
    }
    setBusy(true);
    setActionMessage(null);
    if (name === "ImportPost") {
      setCaptureProcess((p) => ({
        running: true,
        title: p?.title || "CSV import",
        current: p?.current ?? 0,
        total: Math.max(p?.total ?? 3, 3),
        detail: "Posting",
        lines: [...(p?.lines ?? []), "post  starting"],
        posted: 0,
        skipped: 0,
        errors: 0,
      }));
    }
    try {
      const result = await client.executeCommand(name, { batchId: pendingBatchId });
      let body: {
        status?: string;
        postedCount?: number;
        skippedDuplicateCount?: number;
        errorCount?: number;
        processLines?: string[];
        candidateCount?: number;
      } = {};
      if (result.bodyJson) {
        try {
          body = JSON.parse(result.bodyJson) as typeof body;
          if (body.status) setPendingBatchStatus(body.status);
        } catch {
          /* ignore */
        }
      }
      if (name === "ImportPost") {
        const posted = body.postedCount ?? 0;
        const skipped = body.skippedDuplicateCount ?? 0;
        const errors = body.errorCount ?? 0;
        const report = body.processLines ?? [];
        const summary = result.ok
          ? `Posted ${posted}, skipped ${skipped} duplicate${skipped === 1 ? "" : "s"}, ${errors} error${errors === 1 ? "" : "s"}.`
          : `ImportPost failed: ${result.errorCode ?? "error"}`;
        setCaptureProcess((p) => ({
          running: false,
          title: p?.title || "CSV import",
          current: p?.total || 3,
          total: p?.total || 3,
          detail: summary,
          lines: [...(p?.lines ?? []), ...report, summary],
          posted,
          skipped,
          errors: result.ok ? errors : errors + 1,
        }));
        setActionMessage(summary);
      } else {
        const msg = result.ok ? `${name} ok` : `${name} failed: ${result.errorCode ?? "error"}`;
        setCaptureProcess((p) =>
          p ? { ...p, detail: msg, lines: [...p.lines, msg] } : p,
        );
        setActionMessage(msg);
      }
      await refreshHandoff();
      await refreshData(asOfDate);
    } catch (err: unknown) {
      const msg = String(err);
      setCaptureProcess((p) =>
        p ? { ...p, running: false, errors: p.errors + 1, detail: msg, lines: [...p.lines, msg] } : p,
      );
      setActionMessage(msg);
    } finally {
      setBusy(false);
    }
  };

  const checkForUpdates = async () => {
    setBusy(true);
    setActionMessage(null);
    try {
      const update = await check();
      if (update) {
        setUpdateStatus(`available ${update.version} (not applied)`);
        setActionMessage(`Update ${update.version} available; not applied`);
      } else {
        setUpdateStatus("none");
      }
    } catch (err: unknown) {
      setUpdateStatus("failed closed");
      setActionMessage(String(err));
    } finally {
      setBusy(false);
    }
  };

  const writesBlocked = handoff !== null && !handoff.writesAllowed;
  const selectedLot = holdings?.lots.find((lot) => lot.lotId === lotId);

  useEffect(() => {
    const onLeave = (event: BeforeUnloadEvent) => {
      if (!pdDirty && !wizDirty && !addLotDirty) return;
      event.preventDefault();
      event.returnValue = "";
    };
    window.addEventListener("beforeunload", onLeave);
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    void import("@tauri-apps/api/window")
      .then(({ getCurrentWindow }) => {
        if (cancelled) return undefined;
        return getCurrentWindow().onCloseRequested((event) => {
          if (!pdDirty && !wizDirty && !addLotDirty) return;
          event.preventDefault();
          setActionMessage(
            "Save or Cancel before leaving. Navigation stays blocked while edits are unsaved.",
          );
        });
      })
      .then((fn) => {
        if (fn) unlisten = fn;
      })
      .catch(() => {
        /* browser preview without Tauri window API */
      });
    return () => {
      cancelled = true;
      window.removeEventListener("beforeunload", onLeave);
      unlisten?.();
    };
  }, [pdDirty, wizDirty, addLotDirty]);

  // Process A UI does not call the former 9-step handlers; keep bindings referenced for tsc.
  void [
    saveWizRemainingDates,
    researchSource,
    retrieveFromMarket,
    newInvestmentPart1,
    researchRoc,
    confirmRocPlan,
    openFirstLot,
    recordWizardPeriod,
  ];

  const positionSymbolOptions = useMemo(() => {
    const fromSecs = securities.map((s) => s.symbol);
    const fromCalc = (calculator?.rows ?? []).map((row) => row.symbol);
    return [...new Set([...fromSecs, ...fromCalc])].sort();
  }, [securities, calculator]);

  const filteredPositionSymbols = useMemo(() => {
    const q = positionSymbolQuery.trim().toUpperCase();
    if (!q) return positionSymbolOptions;
    return positionSymbolOptions.filter((sym) => sym.toUpperCase().startsWith(q));
  }, [positionSymbolOptions, positionSymbolQuery]);

  const addLotSecurityOptions = useMemo(() => {
    return securities
      .map((s) => {
        const byHoldings =
          holdings?.lots.filter(
            (l) =>
              l.symbol.toUpperCase() === s.symbol.toUpperCase() &&
              l.remainingQuantityMinor > 0,
          ).length ?? 0;
        const collector = collectorItems.find((c) => c.securityId === s.securityId);
        const openLotCount = byHoldings > 0 ? byHoldings : collector?.openLots ? 1 : 0;
        return {
          securityId: s.securityId,
          symbol: s.symbol,
          name: s.name || "",
          openLotCount,
        };
      })
      .sort((a, b) => a.symbol.localeCompare(b.symbol));
  }, [securities, holdings, collectorItems]);

  const filteredAddLotSecurities = useMemo(() => {
    const q = addLotQuery.trim().toUpperCase();
    if (!q) return addLotSecurityOptions.slice(0, 12);
    return addLotSecurityOptions
      .filter((s) => {
        const sym = s.symbol.toUpperCase();
        const name = s.name.toUpperCase();
        return (
          sym.startsWith(q) ||
          sym.includes(q) ||
          name.startsWith(q) ||
          name.includes(q)
        );
      })
      .slice(0, 12);
  }, [addLotSecurityOptions, addLotQuery]);

  const selectAddLotSecurity = (row: {
    securityId: string;
    symbol: string;
    name: string;
  }) => {
    setAddLotSecurityId(row.securityId);
    setAddLotQuery(`${row.symbol}${row.name ? ` — ${row.name}` : ""}`);
    setAddLotSymbolOpen(false);
  };

  const processALotCount =
    holdings?.lots.filter(
      (l) =>
        l.symbol.toUpperCase() === wizSymbol.trim().toUpperCase() &&
        l.remainingQuantityMinor > 0,
    ).length ?? 0;

  const processAFieldStatus = {
    frequency: Boolean(parseCadence(wizFreq)),
    declarations: wizDecls.length > 0,
    roc:
      wizRoc != null &&
      wizRoc.rocPctMinor != null &&
      (wizRoc.rocPctMinor as number) > 0,
    price:
      wizPriceState?.priceMinor != null && wizPriceState.priceMinor > 0,
  };

  const saveProcessAResearch = () => {
    if (!wizSecurityId || !wizResearchDone) {
      setActionMessage("Run Research before Save.");
      return;
    }
    snapshotWiz();
    setWizProcessASaved(true);
    setActionMessage(
      `Saved ${wizSymbol.trim().toUpperCase()}. Open Position Details, Validate ROC, or Add lots — zero lots is valid.`,
    );
    void refreshData(asOfDate || new Date().toISOString().slice(0, 10));
  };

  const startAnotherProcessA = () => {
    setWizProcessASaved(false);
    setWizResearchDone(false);
    setWizSecurityId("");
    setWizSymbol("");
    setWizName("");
    setWizSourceUrl("");
    setWizFreq("");
    setWizDecls([]);
    setWizRoc(null);
    setWizRocPct("");
    setWizPrice("");
    setWizPriceState(null);
    setWizRetrieveNote("");
    setWizTierSuggestion(null);
    setWizPlanStored(false);
    setWizRisk("");
    setWizBaseline(JSON.stringify(emptyWizEdit()));
  };

  const goProcessAToPositionDetails = () => {
    if (!wizSymbol.trim()) return;
    leaveWithoutSaving(() => {
      setPositionSymbol(wizSymbol.trim().toUpperCase());
      setPositionSymbolQuery(wizSymbol.trim().toUpperCase());
      setScreen("position-details");
      void loadInvestment(wizSymbol.trim().toUpperCase());
    }, "position-details");
  };

  const goProcessAToAddLot = () => {
    if (!wizSecurityId) return;
    leaveWithoutSaving(() => {
      const row = securities.find((s) => s.securityId === wizSecurityId);
      setAddLotSecurityId(wizSecurityId);
      setAddLotQuery(
        row
          ? `${row.symbol}${row.name ? ` — ${row.name}` : ""}`
          : wizSymbol.trim().toUpperCase(),
      );
      setScreen("add-lot");
    }, "add-lot");
  };

  useEffect(() => {
    setPositionSymbolQuery(positionSymbol);
  }, [positionSymbol]);

  const selectPositionSymbol = (symbol: string) => {
    leaveWithoutSaving(() => {
      setPositionFocusPanel("");
      setPositionSymbol(symbol);
      setPositionSymbolQuery(symbol);
      setPositionSymbolOpen(false);
      if (symbol) {
        void loadInvestment(symbol);
      } else {
        setInvestment(null);
        setPdDraft(null);
      }
    });
  };

  const navButton = (id: Screen, label: string) => (
    <button
      type="button"
      aria-label={label}
      aria-current={screen === id ? "page" : undefined}
      disabled={
        busy ||
        (screen !== id &&
          (pdDirty || wizDirty || addLotDirty) &&
          !dirtyTargets.includes(id))
      }
      onClick={() => leaveWithoutSaving(() => setScreen(id), id)}
    >
      {label}
    </button>
  );

  return (
    <main className="container" aria-label="finos">
      <h1>finos</h1>
      {summary ? (
        <section aria-label="Portfolio summary">
          <dl className="portfolio-summary">
            <div className="ps-cell ps-mv">
              <dt>Market value</dt>
              <dd>
                {summary.marketValueMinor == null
                  ? "unknown"
                  : `${formatUsd(summary.marketValueMinor, summary.scale ?? 2)}${
                      summary.marketValueComplete ? "" : " (incomplete)"
                    }`}
              </dd>
            </div>
            <div className="ps-cell ps-cost">
              <dt>Original cost</dt>
              <dd>{formatUsd(summary.openPerformanceMinor ?? 0, summary.scale ?? 2)}</dd>
            </div>
            <div className="ps-cell ps-tax">
              <dt>Tax basis</dt>
              <dd>{formatUsd(summary.openTaxMinor ?? 0, summary.scale ?? 2)}</dd>
            </div>
            <div
              className={`ps-cell ps-unrealized${
                summary.marketValueMinor == null
                  ? ""
                  : summary.marketValueMinor - (summary.openPerformanceMinor ?? 0) >= 0
                    ? " ps-gain"
                    : " ps-loss"
              }`}
            >
              <dt>Unrealized</dt>
              <dd>
                {summary.marketValueMinor == null
                  ? "unknown"
                  : formatUsd(
                      summary.marketValueMinor - (summary.openPerformanceMinor ?? 0),
                      summary.scale ?? 2,
                    )}
              </dd>
            </div>
            <div className="ps-cell ps-income">
              <dt>Income earned</dt>
              <dd>
                {formatUsd(
                  dividendLifetime?.actualTotalMinor ??
                    summary.incomeEarnedMinor ??
                    0,
                  dividendLifetime?.scale ?? summary.scale ?? 2,
                )}
              </dd>
              <p className="ps-note">Lifetime paid dividends (cash)</p>
            </div>
            <div className="ps-cell ps-count">
              <dt>Accounts</dt>
              <dd>{formatCount(summary.accountCount)}</dd>
            </div>
            <div className="ps-cell ps-count">
              <dt>Symbols</dt>
              <dd>{formatCount(summary.symbolCount ?? 0)}</dd>
            </div>
            <div className="ps-cell ps-count">
              <dt>Open lots</dt>
              <dd>{formatCount(summary.openLotCount)}</dd>
            </div>
            <div className="ps-cell ps-coverage">
              <dt>Last prices</dt>
              <dd>
                {formatCount(summary.lastPriceCount ?? 0)} of{" "}
                {formatCount(summary.symbolCount ?? 0)}
              </dd>
            </div>
            <div className="ps-cell ps-count">
              <dt>Yield events</dt>
              <dd>{formatCount(summary.yieldCount)}</dd>
              <p className="ps-note">Count of paid dividend posts</p>
            </div>
            <div className="ps-cell ps-count">
              <dt>Disbursements</dt>
              <dd>{formatCount(summary.disbursementCount)}</dd>
              <p className="ps-note">Count of Non-ROI posts</p>
            </div>
            <div className="ps-cell ps-count">
              <dt>Calculator plans</dt>
              <dd>{formatCount(summary.planCount)}</dd>
            </div>
            <div className="ps-cell ps-date">
              <dt>Last yield</dt>
              <dd>{summary.latestYieldOn?.trim() || "none"}</dd>
            </div>
          </dl>
        </section>
      ) : (
        <p role="status">Loading portfolio summary…</p>
      )}
      <p>
        <button
          type="button"
          aria-label="Refresh last prices"
          disabled={lastPriceBusy || busy}
          onClick={() => {
            void refreshLastPrices();
          }}
        >
          {lastPriceBusy ? "Refreshing last prices…" : "Refresh last prices"}
        </button>
      </p>
      <p>Week labeled by Friday {incomeWeek?.end ?? asOfDate} (Sat–Fri).</p>
      <nav className="nav" aria-label="Data screens">
        {navButton("income-plan", "Income Plan")}
        {navButton("calculator", "Calculator")}
        {navButton("position-details", "Position Details")}
        {navButton("dashboard", "Dashboard")}
        {navButton("trends", "Trends")}
        {navButton("holdings", "Holdings")}
        {navButton("new-investment", "Add Position")}
        {navButton("add-lot", "Add Lot")}
        {navButton("import", "Import")}
        {navButton("collectors", "Collectors")}
        {navButton("settings", "Settings")}
      </nav>
      {pdDirty || wizDirty || addLotDirty ? (
        <div className="blocked unsaved-bar" role="alert">
          <p>
            Unsaved edits on {dirtyScreenNames}. Save or Cancel. Other screens stay
            blocked until you do.
          </p>
          <div className="buttons">
            {dirtyTargets
              .filter((id) => id !== screen)
              .map((id) => (
                <button
                  key={id}
                  type="button"
                  aria-label="Open screen with unsaved edits"
                  onClick={() => setScreen(id)}
                >
                  Open{" "}
                  {id === "position-details"
                    ? "Position Details"
                    : id === "new-investment"
                      ? "Add Position"
                      : "Add Lot"}
                </button>
              ))}
            <button
              type="button"
              aria-label="Cancel unsaved edits"
              disabled={busy}
              onClick={() => discardUnsavedEdits()}
            >
              Cancel
            </button>
          </div>
        </div>
      ) : null}
      {actionMessage ? <p>{actionMessage}</p> : null}
      {writesBlocked ? (
        <p className="blocked">Ordinary writes are blocked until restore or explicit review.</p>
      ) : null}

      {screen === "income-plan" ? (
        <section aria-label="Income Plan">
          <h2>Income Plan</h2>
          <p>
            Dividend cash only. Current and future week Plan comes from Calculator Plan ×
            eligible quantity × expected pay week. Status Open; not official Dashboard close.
          </p>
          {(() => {
            const todaySat = saturdayOfWeek(new Date().toISOString().slice(0, 10));
            const selectedSat = saturdayOfWeek(
              incomeWeek?.start || asOfDate || todaySat,
            );
            const selectedFri = incomeWeek?.end || shiftIso(selectedSat, 6);
            const weekChoices = incomePlanWeekChoices(
              selectedSat,
              incomeWeek?.latestActualOn,
            );
            return (
              <>
                <div className="income-week-bar">
                  <label className="income-week-label">
                    Week
                    <select
                      aria-label="Select week"
                      value={selectedSat}
                      onChange={(e) => setAsOfDate(e.target.value)}
                    >
                      {weekChoices.map((sat) => (
                        <option key={sat} value={sat}>
                          {incomePlanWeekOptionLabel(sat, todaySat)}
                        </option>
                      ))}
                    </select>
                  </label>
                  <button
                    type="button"
                    aria-label="Previous week"
                    disabled={!selectedSat}
                    onClick={() => setAsOfDate(shiftIso(selectedSat, -7))}
                  >
                    Previous week
                  </button>
                  <button
                    type="button"
                    aria-label="Next week"
                    disabled={!selectedSat}
                    onClick={() => setAsOfDate(shiftIso(selectedSat, 7))}
                  >
                    Next week
                  </button>
                </div>
                <p className="income-current-week" aria-label="Current week">
                  Current week: Saturday {selectedSat} through Friday {selectedFri}
                  {selectedSat === todaySat ? (
                    <span className="this-week-mark"> · this week</span>
                  ) : null}
                </p>
              </>
            );
          })()}
          <label>
            Drill account
            <select
              aria-label="Drill account"
              value={drillAccount ?? ""}
              onChange={(e) => setDrillAccount(e.target.value || null)}
            >
              <option value="">All control accounts</option>
              {(incomeWeek?.lines ?? []).map((line) => (
                <option key={line.accountName} value={line.accountName}>
                  {line.accountName}
                </option>
              ))}
            </select>
          </label>
          <IncomePlanWeekPanel
            week={incomeWeek}
            selectedAccount={drillAccount}
            onOpenSymbol={(symbol) => openPositionHub(symbol, "income")}
          />
        </section>
      ) : null}

      {screen === "calculator" ? (
        <section aria-label="Calculator">
          <h2>Calculator</h2>
          <p>
            Plan × quantity for completed investments. Open Position Details to see price,
            declarations, Most Current, and Avg 6 for one symbol.
          </p>
          <CalculatorPanel
            rows={calculator?.rows ?? null}
            onOpenSymbol={(symbol) => openPositionHub(symbol)}
          />
        </section>
      ) : null}

      {screen === "position-details" ? (
        <section aria-label="Position Details">
          <h2>Position Details</h2>
          <p>
            Open a symbol for the position hub: Calculator metrics, Plan payment summary,
            future pay dates, declarations, and holdings by account. Fleet compare stays
            below. Save or Cancel; other screens stay blocked while edits are unsaved.
          </p>
          <div className="buttons">
            <button
              type="button"
              aria-label="Refresh last prices"
              disabled={lastPriceBusy || busy}
              onClick={() => void refreshLastPrices()}
            >
              {lastPriceBusy ? "Refreshing last prices…" : "Refresh last prices"}
            </button>
            <button
              type="button"
              aria-label="Apply issuer sources from provider"
              disabled={busy || writesBlocked}
              onClick={() => void applyIssuerSources()}
            >
              Apply issuer sources from provider
            </button>
          </div>
          <label className="symbol-combobox">
            Symbol
            <input
              aria-label="Position symbol"
              aria-expanded={positionSymbolOpen}
              aria-controls="position-symbol-list"
              aria-autocomplete="list"
              role="combobox"
              value={positionSymbolQuery}
              onChange={(e) => {
                setPositionSymbolQuery(e.target.value.toUpperCase());
                setPositionSymbolOpen(true);
              }}
              onFocus={() => setPositionSymbolOpen(true)}
              onBlur={() => {
                window.setTimeout(() => setPositionSymbolOpen(false), 150);
              }}
              onKeyDown={(e) => {
                if (e.key === "Enter" && filteredPositionSymbols[0]) {
                  e.preventDefault();
                  selectPositionSymbol(filteredPositionSymbols[0]);
                }
                if (e.key === "Escape") {
                  setPositionSymbolOpen(false);
                }
              }}
            />
            {positionSymbolOpen && filteredPositionSymbols.length > 0 ? (
              <ul
                id="position-symbol-list"
                className="symbol-combobox-list"
                role="listbox"
                aria-label="Position symbols"
              >
                {filteredPositionSymbols.slice(0, 30).map((sym) => (
                  <li
                    key={sym}
                    role="option"
                    aria-selected={sym === positionSymbol}
                    onMouseDown={(e) => {
                      e.preventDefault();
                      selectPositionSymbol(sym);
                    }}
                  >
                    {sym}
                  </li>
                ))}
              </ul>
            ) : null}
          </label>
          {investment && pdDraft ? (
            <>
              {pdDirty ? (
                <p className="blocked" role="status">
                  Unsaved edits. Save or Cancel — other screens stay blocked.
                </p>
              ) : null}
              <div className="buttons dossier-actions">
                <button
                  type="button"
                  aria-label="Save stored facts"
                  disabled={busy || writesBlocked || !pdDirty}
                  onClick={() => void saveStoredFacts()}
                >
                  Save
                </button>
                <button
                  type="button"
                  aria-label="Cancel position edits"
                  disabled={busy || !pdDirty}
                  onClick={() => cancelPositionEdits()}
                >
                  Cancel
                </button>
                <button
                  type="button"
                  aria-label="Use Most Current as Plan"
                  disabled={
                    busy || writesBlocked || investment.review.mostCurrentMinor == null
                  }
                  onClick={() => {
                    if (investment.review.mostCurrentMinor == null) return;
                    patchDraft({
                      plan: scaledDollars(
                        investment.review.mostCurrentMinor,
                        investment.review.amountScale,
                      ),
                      planReason: "Match Most Current",
                    });
                  }}
                >
                  Use Most Current as Plan
                </button>
                <button
                  type="button"
                  aria-label="Use Avg 6 as Plan"
                  disabled={busy || writesBlocked || investment.review.avg6Minor == null}
                  onClick={() => {
                    if (investment.review.avg6Minor == null) return;
                    patchDraft({
                      plan: scaledDollars(
                        investment.review.avg6Minor,
                        investment.review.amountScale,
                      ),
                      planReason: "Match Avg 6 (owner typed)",
                    });
                  }}
                >
                  Use Avg 6 as Plan
                </button>
                <button
                  type="button"
                  aria-label="Refresh last price for this symbol"
                  disabled={busy || writesBlocked}
                  onClick={() => void refreshPdLastPrice()}
                >
                  Refresh last price
                </button>
                <button
                  type="button"
                  aria-label="Retrieve declarations for this symbol"
                  disabled={busy || writesBlocked}
                  onClick={() => void refreshPdDeclarations()}
                >
                  Retrieve declarations
                </button>
                <button
                  type="button"
                  aria-label="Complete research"
                  disabled={
                    busy ||
                    writesBlocked ||
                    !investment.securityId ||
                    Boolean(researchActivity?.running)
                  }
                  onClick={() => void researchPdRoc()}
                >
                  {researchActivity?.running ? "Completing research…" : "Complete research"}
                </button>
              </div>
              {researchActivity?.running ? (
                <section
                  className="process-a-research-progress"
                  aria-label="Research progress"
                  aria-busy="true"
                >
                  <p role="status" aria-live="polite">
                    {researchActivity.label}
                  </p>
                  {researchActivity.total > 0 ? (
                    <progress
                      max={researchActivity.total}
                      value={Math.min(
                        researchActivity.step,
                        researchActivity.total,
                      )}
                    />
                  ) : (
                    <div
                      className="process-a-research-spinner"
                      aria-hidden="true"
                    />
                  )}
                </section>
              ) : null}
              {researchActivity?.resultLine && !researchActivity.running ? (
                <p role="status" aria-label="Research result">
                  {researchActivity.resultLine}
                </p>
              ) : null}
              {(() => {
                const finished =
                  Boolean(researchActivity?.resultLine) &&
                  !researchActivity?.running &&
                  !(researchActivity?.resultLine || "").startsWith(
                    "Complete research failed",
                  );
                const storedNotes = (
                  pdDraft?.notes ||
                  investment.notes ||
                  ""
                ).trim();
                const suggested = (
                  researchNotes?.suggestedTier ||
                  investment.suggestion?.suggestedTier ||
                  pdDraft?.lookthrough?.riskTierSuggestion ||
                  ""
                ).trim();
                if (!researchNotes && !storedNotes && !finished) {
                  return null;
                }
                return (
                <section
                  className="hub-panel research-notes-panel"
                  aria-label="Research notes"
                >
                  <h3>Research notes</h3>
                  <p className="research-notes-overview">
                    {researchNotes?.overview ||
                      pdDraft?.notes ||
                      investment.notes}
                  </p>
                  {researchNotes?.source ? (
                    <p className="muted">Source: {researchNotes.source}</p>
                  ) : null}
                  {finished || researchNotes ? (
                    <fieldset
                      className="research-risk-control"
                      aria-label="Owner risk choice"
                    >
                      <legend>Risk (required)</legend>
                      <label>
                        Owner tier
                        <select
                          aria-label="Owner risk choice"
                          value={ownerRiskChoice}
                          onChange={(e) => setOwnerRiskChoice(e.target.value)}
                          disabled={busy || writesBlocked}
                        >
                          {OWNER_RISK_CHOICES.map((tier) => (
                            <option key={tier} value={tier}>
                              {suggested && tier === suggested
                                ? `${tier} (suggestion)`
                                : tier}
                            </option>
                          ))}
                        </select>
                      </label>
                      {suggested ? (
                        <p role="note" aria-label="Suggested risk tier">
                          Suggested: {suggested}
                          {researchNotes?.suggestedReason ||
                          investment.suggestion?.reason
                            ? ` — ${
                                researchNotes?.suggestedReason ||
                                investment.suggestion?.reason
                              }`
                            : ""}
                          . Not applied until you Set risk.
                        </p>
                      ) : (
                        <p role="note">
                          No suggested tier. Choose one or Leave undecided.
                        </p>
                      )}
                      <div className="buttons">
                        <button
                          type="button"
                          aria-label="Set risk"
                          disabled={
                            busy ||
                            writesBlocked ||
                            !RISK_TIERS.includes(ownerRiskChoice)
                          }
                          onClick={() => void setOwnerRisk()}
                        >
                          Set risk
                        </button>
                        <button
                          type="button"
                          aria-label="Leave undecided"
                          disabled={busy || writesBlocked}
                          onClick={() => void leaveRiskUndecided()}
                        >
                          Leave undecided
                        </button>
                      </div>
                    </fieldset>
                  ) : null}
                </section>
                );
              })()}
              <dl className="hub-hero" aria-label="Position hub summary">
                <div>
                  <dt>Symbol</dt>
                  <dd>{investment.symbol}</dd>
                </div>
                <div>
                  <dt>Market value</dt>
                  <dd>
                    {investment.marketValueMinor == null
                      ? "unknown"
                      : formatUsd(investment.marketValueMinor, investment.scale)}
                  </dd>
                </div>
                <div>
                  <dt>Plan annual</dt>
                  <dd>
                    {investment.annualPlanMinor == null
                      ? "unknown"
                      : formatUsd(investment.annualPlanMinor, investment.scale)}
                  </dd>
                </div>
                <div>
                  <dt>Underlying</dt>
                  <dd>{investment.underlying?.trim() || "—"}</dd>
                </div>
                <div>
                  <dt>Plan YOC</dt>
                  <dd>{formatBps(investment.planYocBps ?? null)}</dd>
                </div>
                <div>
                  <dt>Price</dt>
                  <dd>{formatHubPrice(investment.price, investment.scale)}</dd>
                </div>
                <div>
                  <dt>Total distributions</dt>
                  <dd>
                    {investment.distributionsScope === "incomplete" ||
                    investment.totalDistributionsReceivedMinor == null
                      ? "unknown"
                      : formatUsd(
                          investment.totalDistributionsReceivedMinor,
                          investment.scale,
                        )}
                  </dd>
                </div>
                <div>
                  <dt>ROC component</dt>
                  <dd>
                    {investment.distributionsScope === "incomplete" ||
                    investment.rocDistributionsMinor == null
                      ? "unknown"
                      : formatUsd(investment.rocDistributionsMinor, investment.scale)}
                  </dd>
                </div>
                <div>
                  <dt>Cost recovery</dt>
                  <dd>{formatBps(investment.costRecoveryBps ?? null)}</dd>
                </div>
                {(() => {
                  const master = (positionMaster?.rows ?? []).find(
                    (r) => r.symbol === investment.symbol,
                  );
                  if (!master) return null;
                  return (
                    <div>
                      <dt title="Share of total data portfolio market value">Portfolio %</dt>
                      <dd>{formatBps(master.allocationBps)}</dd>
                    </div>
                  );
                })()}
              </dl>
              <nav className="hub-toc" aria-label="Position hub sections">
                <a href="#hub-identity">Position information</a>
                <a href="#hub-calculator">Calculator</a>
                <a href="#hub-plan-yields">Plan</a>
                <a href="#hub-income">Payment summary</a>
                <a href="#hub-pay-dates">Pay dates</a>
                <a href="#hub-received">Received</a>
                <a href="#hub-declarations">Declarations</a>
                <a href="#hub-accounts">By account</a>
                <a href="#hub-lots">Lots</a>
                <a href="#hub-ledger">Ledger</a>
              </nav>
              <section className="hub-panel" id="hub-identity" aria-label="Position information">
                <h3>Position information</h3>
                <div className="table-wrap">
                  <table aria-label="Position information">
                    <thead>
                      <tr>
                        <th scope="col">Fact</th>
                        <th scope="col">Value</th>
                      </tr>
                    </thead>
                    <tbody>
                      <tr>
                        <th scope="row">Symbol</th>
                        <td>{investment.symbol}</td>
                      </tr>
                      <tr>
                        <th scope="row">Name</th>
                        <td>{pdDraft.name.trim() || investment.name || "—"}</td>
                      </tr>
                      <tr>
                        <th scope="row">Provider</th>
                        <td>{pdDraft.provider.trim() || investment.provider || "—"}</td>
                      </tr>
                      <tr>
                        <th scope="row">Underlying</th>
                        <td>
                          {pdDraft.underlying.trim() ||
                            investment.underlying?.trim() ||
                            "—"}
                        </td>
                      </tr>
                      <tr>
                        <th scope="row">Risk</th>
                        <td>
                          {pdDraft.risk.trim() || investment.riskTier || "—"}
                        </td>
                      </tr>
                      <tr>
                        <th scope="row">Frequency</th>
                        <td>
                          {pdDraft.freq.trim() ||
                            investment.paymentFrequency ||
                            "—"}
                        </td>
                      </tr>
                      <tr>
                        <th scope="row">ROC research</th>
                        <td aria-label="Position ROC research status">
                          {rocResearchLabel(investment)}
                        </td>
                      </tr>
                      <tr>
                        <th scope="row">ROC last update</th>
                        <td>{rocResearchUpdated(investment)}</td>
                      </tr>
                    </tbody>
                  </table>
                </div>
              </section>
              <section className="hub-panel" id="hub-calculator" aria-label="Calculator snapshot">
                <h3>Calculator snapshot</h3>
                {(() => {
                  const calc = (calculator?.rows ?? []).find(
                    (r) => r.symbol === investment.symbol,
                  );
                  if (!calc) {
                    return (
                      <p>
                        No Calculator row yet (needs Plan + first lot). Holdings and
                        remaining-year panels below still apply.
                      </p>
                    );
                  }
                  return (
                    <div className="table-wrap">
                      <table aria-label="Calculator snapshot">
                        <thead>
                          <tr>
                            <th scope="col">Metric</th>
                            <th className="numeric" scope="col">Value</th>
                          </tr>
                        </thead>
                        <tbody>
                          <tr>
                            <th scope="row">Frequency</th>
                            <td className="numeric">{calc.paymentFrequency || "—"}</td>
                          </tr>
                          <tr>
                            <th scope="row">Plan / share</th>
                            <td className="numeric">
                              {calc.planKnown
                                ? `$${formatScaled(calc.planPerShareMinor, calc.planScale)}`
                                : "N/A"}
                            </td>
                          </tr>
                          <tr>
                            <th scope="row">Periods / year</th>
                            <td className="numeric">
                              {calc.planningPeriodsPerYear > 0
                                ? formatCount(calc.planningPeriodsPerYear)
                                : "N/A"}
                            </td>
                          </tr>
                          <tr>
                            <th scope="row">Open quantity</th>
                            <td className="numeric">
                              {formatScaled(calc.remainingQuantityMinor, calc.quantityScale)}
                            </td>
                          </tr>
                          <tr>
                            <th scope="row">Plan payment</th>
                            <td className="numeric">
                              {calc.planKnown
                                ? formatUsd(calc.planPaymentMinor, calc.scale)
                                : "N/A"}
                            </td>
                          </tr>
                          <tr>
                            <th scope="row">Original cost</th>
                            <td className="numeric">
                              {formatUsd(calc.remainingPerformanceMinor, calc.scale)}
                            </td>
                          </tr>
                          <tr>
                            <th scope="row">Last price</th>
                            <td className="numeric">
                              {calc.lastPriceMinor == null
                                ? "unknown"
                                : formatUsd(
                                    calc.lastPriceMinor,
                                    calc.lastPriceScale ?? calc.scale,
                                  )}
                            </td>
                          </tr>
                          <tr>
                            <th scope="row">Price freshness</th>
                            <td className="numeric">{calc.priceFreshness || "unavailable"}</td>
                          </tr>
                          <tr>
                            <th scope="row">Market value</th>
                            <td className="numeric">
                              {calc.marketValueMinor == null
                                ? "unknown"
                                : formatUsd(calc.marketValueMinor, calc.scale)}
                            </td>
                          </tr>
                          <tr>
                            <th scope="row">ROC 2025</th>
                            <td className="numeric">
                              {rocActualUnavailable(
                                investment.lots,
                                2025,
                                asOfDate,
                              ) ??
                                (calc.rocPct2025ActualMinor == null ||
                                calc.rocScale == null
                                  ? "missing-1099"
                                  : formatPercentScaled(
                                      calc.rocPct2025ActualMinor,
                                      calc.rocScale,
                                    ))}
                            </td>
                          </tr>
                          <tr>
                            <th scope="row">ROC 2026 estimate</th>
                            <td className="numeric">
                              {calc.rocPct2026EstimateMinor == null || calc.rocScale == null
                                ? "N/A"
                                : formatPercentScaled(
                                    calc.rocPct2026EstimateMinor,
                                    calc.rocScale,
                                  )}
                            </td>
                          </tr>
                          <tr>
                            <th scope="row">ROC 2026 actual</th>
                            <td className="numeric">
                              {rocActualUnavailable(
                                investment.lots,
                                2026,
                                asOfDate,
                              ) ??
                                (calc.rocPct2026ActualMinor == null ||
                                calc.rocScale == null
                                  ? "missing-1099"
                                  : formatPercentScaled(
                                      calc.rocPct2026ActualMinor,
                                      calc.rocScale,
                                    ))}
                            </td>
                          </tr>
                        </tbody>
                      </table>
                    </div>
                  );
                })()}
              </section>
              <section className="hub-panel" id="hub-plan-yields" aria-label="Plan and yields">
                <h3>Plan and yields</h3>
                <div className="table-wrap">
                  <table aria-label="Plan and yields">
                    <thead>
                      <tr>
                        <th scope="col">Metric</th>
                        <th scope="col">Value</th>
                      </tr>
                    </thead>
                    <tbody>
                      {(() => {
                        const planShare = investment.planKnown
                          ? `$${formatScaled(investment.planPerShareMinor, investment.planScale)}`
                          : "unknown";
                        const periods =
                          investment.planningPeriodsPerYear > 0
                            ? formatCount(investment.planningPeriodsPerYear)
                            : "unknown";
                        const qty = formatScaled(
                          investment.remainingQuantityMinor,
                          investment.quantityScale,
                        );
                        const cost = formatUsd(
                          investment.remainingPerformanceMinor,
                          investment.scale,
                        );
                        const price =
                          investment.price?.priceMinor == null
                            ? "unknown"
                            : formatUsd(
                                investment.price.priceMinor,
                                investment.price.scale ?? investment.scale,
                              );
                        const mostCurrent =
                          investment.review.mostCurrentMinor == null
                            ? "unknown"
                            : `$${formatScaled(
                                investment.review.mostCurrentMinor,
                                investment.review.amountScale,
                              )}`;
                        const annual =
                          investment.annualPlanMinor == null
                            ? "unknown"
                            : formatUsd(investment.annualPlanMinor, investment.scale);
                        return (
                          <>
                            <tr>
                              <th
                                scope="row"
                                className="metric-hint"
                                {...metricHint(
                                  "Plan / share",
                                  `Owner PlanHistory $/share (never auto from declarations). Last adjusted ${
                                    investment.planEffectiveFrom?.trim() || "unknown"
                                  }.`,
                                )}
                              >
                                Plan / share
                              </th>
                              <td>
                                {investment.planKnown
                                  ? `$${formatScaled(investment.planPerShareMinor, investment.planScale)}`
                                  : "unknown"}
                                {investment.planEffectiveFrom?.trim()
                                  ? ` — last adjusted ${investment.planEffectiveFrom}`
                                  : ""}
                              </td>
                            </tr>
                            <tr>
                              <th
                                scope="row"
                                className="metric-hint"
                                {...metricHint(
                                  "Annual plan",
                                  `Plan/share × open qty × periods/year. Inputs: ${planShare} × ${qty} × ${periods} → ${annual}.`,
                                )}
                              >
                                Annual plan
                              </th>
                              <td>
                                {investment.annualPlanMinor == null
                                  ? "unknown"
                                  : formatUsd(
                                      investment.annualPlanMinor,
                                      investment.scale,
                                    )}
                              </td>
                            </tr>
                            <tr>
                              <th
                                scope="row"
                                className="metric-hint"
                                {...metricHint(
                                  "Plan YOC",
                                  `Annual plan $ ÷ original economic cost. Inputs: ${annual} ÷ ${cost}.`,
                                )}
                              >
                                Plan YOC
                              </th>
                              <td>{formatBps(investment.planYocBps ?? null)}</td>
                            </tr>
                            <tr>
                              <th
                                scope="row"
                                className="metric-hint"
                                {...metricHint(
                                  "Plan FWD",
                                  `(Plan/share × periods) ÷ last price. Inputs: (${planShare} × ${periods}) ÷ ${price}.`,
                                )}
                              >
                                Plan FWD
                              </th>
                              <td>
                                {formatBps(investment.planFwdYieldBps ?? null)}
                              </td>
                            </tr>
                            <tr>
                              <th
                                scope="row"
                                className="metric-hint"
                                {...metricHint(
                                  "Most Current",
                                  `Latest paid issuer declaration $/share (Plan-review). This position: ${mostCurrent}.`,
                                )}
                              >
                                Most Current
                              </th>
                              <td>
                                {investment.review.mostCurrentMinor == null
                                  ? "unknown"
                                  : `$${formatScaled(
                                      investment.review.mostCurrentMinor,
                                      investment.review.amountScale,
                                    )}`}
                              </td>
                            </tr>
                            <tr>
                              <th
                                scope="row"
                                className="metric-hint"
                                {...metricHint(
                                  "Avg 6",
                                  "Mean of up to 6 most recent paid declarations (blank weeks are not $0).",
                                )}
                              >
                                Avg 6
                              </th>
                              <td>
                                {investment.review.avg6Minor == null
                                  ? "unknown"
                                  : `$${formatScaled(
                                      investment.review.avg6Minor,
                                      investment.review.amountScale,
                                    )}`}
                              </td>
                            </tr>
                            <tr>
                              <th
                                scope="row"
                                className="metric-hint"
                                {...metricHint(
                                  "Most Current FWD",
                                  `(Most Current × periods) ÷ last price. Inputs: (${mostCurrent} × ${periods}) ÷ ${price}.`,
                                )}
                              >
                                Most Current FWD
                              </th>
                              <td>
                                {formatBps(
                                  investment.mostCurrentFwdYieldBps ?? null,
                                )}
                              </td>
                            </tr>
                            <tr>
                              <th
                                scope="row"
                                className="metric-hint"
                                {...metricHint(
                                  "Most Current vs Plan",
                                  `(Most Current − Plan) ÷ Plan → Above / Equal / Below. Inputs: (${mostCurrent} − ${planShare}) ÷ ${planShare}.`,
                                )}
                              >
                                Most Current vs Plan
                              </th>
                              <td>
                                {mostCurrentVsPlanFace(
                                  investment.mostCurrentVsPlanBps ?? null,
                                )}
                              </td>
                            </tr>
                          </>
                        );
                      })()}
                    </tbody>
                  </table>
                </div>
              </section>
              <section className="hub-panel" id="hub-accounts" aria-label="Holdings by account">
                <h3>Holdings by account</h3>
                <p>
                  Open quantity and cost for this symbol by control account. Market value is
                  qty × last price when the price is valid. P&amp;L is market value minus tax
                  basis.
                </p>
                {(() => {
                  const rows = (positionDetails?.positions ?? []).filter(
                    (r) => r.symbol === investment.symbol,
                  );
                  if (rows.length === 0) {
                    return <p>No open account splits for this symbol.</p>;
                  }
                  const price =
                    investment.price?.priceDerivedValid && investment.price.priceMinor != null
                      ? {
                          minor: investment.price.priceMinor,
                          scale: investment.price.scale ?? investment.scale,
                        }
                      : null;
                  return (
                    <div className="table-wrap">
                      <table aria-label="Holdings by account">
                        <thead>
                          <tr>
                            <th scope="col">Account</th>
                            <th className="numeric" scope="col">Qty</th>
                            <th className="numeric" scope="col">Lots</th>
                            <th className="numeric" scope="col">Original cost</th>
                            <th className="numeric" scope="col">Tax basis</th>
                            <th className="numeric" scope="col">Market value</th>
                            <th className="numeric" scope="col">P&amp;L</th>
                          </tr>
                        </thead>
                        <tbody>
                          {rows.map((row) => {
                            let mv: string = "unknown";
                            let pnl: string = "unknown";
                            if (price && row.remainingQuantityMinor > 0) {
                              const qScale = row.quantityScale ?? 0;
                              const mvMinor = Math.trunc(
                                (row.remainingQuantityMinor * price.minor) /
                                  10 ** qScale,
                              );
                              mv = formatUsd(mvMinor, price.scale);
                              pnl = formatUsd(
                                mvMinor - row.remainingTaxMinor,
                                row.scale,
                              );
                            }
                            return (
                              <tr key={`${row.accountId}-${row.symbol}`}>
                                <td>{row.accountName}</td>
                                <td className="numeric">
                                  {formatScaled(
                                    row.remainingQuantityMinor,
                                    row.quantityScale,
                                  )}
                                </td>
                                <td className="numeric">{formatCount(row.lotCount)}</td>
                                <td className="numeric">
                                  {formatUsd(row.remainingPerformanceMinor, row.scale)}
                                </td>
                                <td className="numeric">
                                  {formatUsd(row.remainingTaxMinor, row.scale)}
                                </td>
                                <td className="numeric">{mv}</td>
                                <td className="numeric">{pnl}</td>
                              </tr>
                            );
                          })}
                        </tbody>
                      </table>
                    </div>
                  );
                })()}
              </section>
              <details className="hub-panel" aria-label="Identity and holdings facts">
              <summary>Identity and edit facts (name, provider, Plan edit, template)</summary>
              <h3>Identity, economics, and Plan (edit)</h3>
              <div className="table-wrap">
                <table aria-label="Position dossier">
                  <thead>
                    <tr>
                      <th scope="col">Fact</th>
                      <th scope="col">Value</th>
                      <th scope="col">Kind</th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr>
                      <th scope="row">Symbol</th>
                      <td>{investment.symbol}</td>
                      <td>identity</td>
                    </tr>
                    <tr>
                      <th scope="row">Name</th>
                      <td>
                        <input
                          aria-label="Position name"
                          value={pdDraft.name}
                          onChange={(e) => patchDraft({ name: e.target.value })}
                          disabled={busy || writesBlocked}
                        />
                      </td>
                      <td>editable</td>
                    </tr>
                    <tr>
                      <th scope="row">Provider</th>
                      <td>
                        <input
                          aria-label="Position provider"
                          value={pdDraft.provider}
                          onChange={(e) => patchDraft({ provider: e.target.value })}
                          disabled={busy || writesBlocked}
                        />
                      </td>
                      <td>editable</td>
                    </tr>
                    <tr>
                      <th scope="row">Risk</th>
                      <td>
                        <select
                          aria-label="Position risk"
                          value={pdDraft.risk}
                          onChange={(e) => patchDraft({ risk: e.target.value })}
                          disabled={busy || writesBlocked}
                        >
                          <option value="">Choose owner tier</option>
                          {OWNER_RISK_CHOICES.map((tier) => (
                            <option key={tier} value={tier}>
                              {tier}
                            </option>
                          ))}
                        </select>
                      </td>
                      <td>editable</td>
                    </tr>
                    <tr>
                      <th scope="row">Frequency</th>
                      <td>
                        <select
                          aria-label="Position frequency"
                          value={pdDraft.freq}
                          onChange={(e) => patchDraft({ freq: e.target.value })}
                          disabled={busy || writesBlocked}
                        >
                          <option value="">Choose cadence</option>
                          <option value="Weekly">Weekly (52)</option>
                          <option value="Monthly">Monthly (12)</option>
                          <option value="Quarterly">Quarterly (4)</option>
                          <option value="None">None (does not pay)</option>
                        </select>
                      </td>
                      <td>editable</td>
                    </tr>
                    <tr>
                      <th scope="row">Underlying</th>
                      <td>
                        <input
                          aria-label="Position underlying"
                          value={pdDraft.underlying}
                          onChange={(e) => patchDraft({ underlying: e.target.value })}
                          disabled={busy || writesBlocked}
                        />
                      </td>
                      <td>editable</td>
                    </tr>
                    <tr>
                      <th scope="row">Theme / strategy</th>
                      <td>
                        <input
                          aria-label="Position theme strategy"
                          value={pdDraft.lookthrough.themeStrategy ?? ""}
                          onChange={(e) =>
                            patchDraft({
                              lookthrough: {
                                ...pdDraft.lookthrough,
                                themeStrategy: e.target.value,
                              },
                            })
                          }
                          disabled={busy || writesBlocked}
                        />
                      </td>
                      <td>editable</td>
                    </tr>
                    <tr>
                      <th scope="row">Primary risk driver</th>
                      <td>
                        <input
                          aria-label="Position primary risk driver"
                          value={pdDraft.lookthrough.primaryRiskDriver ?? ""}
                          onChange={(e) =>
                            patchDraft({
                              lookthrough: {
                                ...pdDraft.lookthrough,
                                primaryRiskDriver: e.target.value,
                              },
                            })
                          }
                          disabled={busy || writesBlocked}
                        />
                      </td>
                      <td>editable</td>
                    </tr>
                    <tr>
                      <th scope="row">Concentration</th>
                      <td aria-label="Position concentration">
                        {concentrationSummary(pdDraft.lookthrough)}
                      </td>
                      <td>
                        {(pdDraft.lookthrough.concentrationStatus ?? "unknown") === "unknown"
                          ? "unknown"
                          : "researched"}
                      </td>
                    </tr>
                    <tr>
                      <th scope="row">Volatility / beta proxy</th>
                      <td>
                        <input
                          aria-label="Position volatility proxy"
                          value={pdDraft.lookthrough.volProxy ?? ""}
                          onChange={(e) =>
                            patchDraft({
                              lookthrough: { ...pdDraft.lookthrough, volProxy: e.target.value },
                            })
                          }
                          disabled={busy || writesBlocked}
                        />
                      </td>
                      <td>editable</td>
                    </tr>
                    <tr>
                      <th scope="row">Tax character note</th>
                      <td>
                        <input
                          aria-label="Position tax character"
                          value={pdDraft.lookthrough.taxCharacter ?? ""}
                          onChange={(e) =>
                            patchDraft({
                              lookthrough: {
                                ...pdDraft.lookthrough,
                                taxCharacter: e.target.value,
                              },
                            })
                          }
                          disabled={busy || writesBlocked}
                        />
                      </td>
                      <td>editable</td>
                    </tr>
                    <tr>
                      <th scope="row">Type / div_type</th>
                      <td>
                        <input
                          aria-label="Position div type"
                          value={pdDraft.divType}
                          onChange={(e) => patchDraft({ divType: e.target.value })}
                          disabled={busy || writesBlocked}
                        />
                      </td>
                      <td>editable</td>
                    </tr>
                    <tr>
                      <th scope="row">Needs ROC research</th>
                      <td>
                        <label>
                          <input
                            type="checkbox"
                            aria-label="Needs ROC research"
                            checked={pdDraft.needsRoc}
                            onChange={(e) => patchDraft({ needsRoc: e.target.checked })}
                            disabled={busy || writesBlocked}
                          />{" "}
                          flag for 19a-1 research
                        </label>
                      </td>
                      <td>editable</td>
                    </tr>
                    <tr>
                      <th scope="row">Active</th>
                      <td>
                        <label>
                          <input
                            type="checkbox"
                            aria-label="Position is active"
                            checked={pdDraft.isActive}
                            onChange={(e) => patchDraft({ isActive: e.target.checked })}
                            disabled={busy || writesBlocked}
                          />{" "}
                          include in daily last-price set
                        </label>
                      </td>
                      <td>editable</td>
                    </tr>
                    <tr>
                      <th scope="row">Expected tax handling</th>
                      <td>
                        <select
                          aria-label="Expected tax handling"
                          value={pdDraft.taxHandling}
                          onChange={(e) => patchDraft({ taxHandling: e.target.value })}
                          disabled={busy || writesBlocked}
                        >
                          {TAX_HANDLING.map((item) => (
                            <option key={item || "blank"} value={item}>
                              {item || "unknown"}
                            </option>
                          ))}
                        </select>
                      </td>
                      <td>editable</td>
                    </tr>
                    <tr>
                      <th scope="row">Declaration weekday</th>
                      <td>
                        <select
                          aria-label="Declaration weekday"
                          value={pdDraft.declarationWeekday}
                          onChange={(e) => patchDraft({ declarationWeekday: e.target.value })}
                          disabled={busy || writesBlocked}
                        >
                          {WEEKDAYS.map((day) => (
                            <option key={day || "blank"} value={day}>
                              {day || "unknown"}
                            </option>
                          ))}
                        </select>
                      </td>
                      <td>editable</td>
                    </tr>
                    <tr>
                      <th scope="row">Ex-date weekday</th>
                      <td>
                        <select
                          aria-label="Ex-date weekday"
                          value={pdDraft.exdateWeekday}
                          onChange={(e) => patchDraft({ exdateWeekday: e.target.value })}
                          disabled={busy || writesBlocked}
                        >
                          {WEEKDAYS.map((day) => (
                            <option key={day || "blank"} value={day}>
                              {day || "unknown"}
                            </option>
                          ))}
                        </select>
                      </td>
                      <td>editable</td>
                    </tr>
                    <tr>
                      <th scope="row">Payday weekday</th>
                      <td>
                        <select
                          aria-label="Payday weekday"
                          value={pdDraft.paydayWeekday}
                          onChange={(e) => patchDraft({ paydayWeekday: e.target.value })}
                          disabled={busy || writesBlocked}
                        >
                          {WEEKDAYS.map((day) => (
                            <option key={day || "blank"} value={day}>
                              {day || "unknown"}
                            </option>
                          ))}
                        </select>
                      </td>
                      <td>editable</td>
                    </tr>
                    <tr>
                      <th scope="row">Completeness</th>
                      <td aria-label="Position completeness">
                        {positionMaster?.rows.find(
                          (row) => row.symbol.toUpperCase() === investment.symbol.toUpperCase(),
                        )?.completeness ?? "unknown"}
                      </td>
                      <td>joined</td>
                    </tr>
                    <tr>
                      <th scope="row">CurrentPrice</th>
                      <td>
                        {investment.price.priceMinor != null
                          ? `${formatUsd(investment.price.priceMinor, investment.price.scale)} (${investment.price.freshness})`
                          : `unavailable (${investment.price.freshness})`}
                      </td>
                      <td>calculated</td>
                    </tr>
                    <tr>
                      <th scope="row">Price-derived valid</th>
                      <td>{investment.price.priceDerivedValid ? "yes" : "no"}</td>
                      <td>calculated</td>
                    </tr>
                    <tr>
                      <th scope="row">Open qty</th>
                      <td>
                        {formatScaled(
                          investment.remainingQuantityMinor,
                          investment.quantityScale,
                        )}
                      </td>
                      <td>from lots</td>
                    </tr>
                    <tr>
                      <th scope="row">Original cost</th>
                      <td>
                        {formatUsd(investment.remainingPerformanceMinor, investment.scale)}
                      </td>
                      <td>from lots</td>
                    </tr>
                    <tr>
                      <th scope="row">Avg unit cost</th>
                      <td>
                        {investment.unitCostMinor == null
                          ? "unknown"
                          : formatUsd(investment.unitCostMinor, 2)}
                      </td>
                      <td>calculated</td>
                    </tr>
                    <tr>
                      <th scope="row">Tax basis</th>
                      <td>{formatUsd(investment.remainingTaxMinor, investment.scale)}</td>
                      <td>from lots</td>
                    </tr>
                    <tr>
                      <th scope="row">Market value</th>
                      <td>
                        {investment.marketValueMinor == null
                          ? "unknown — needs a valid CurrentPrice"
                          : formatUsd(investment.marketValueMinor, investment.scale)}
                      </td>
                      <td>calculated</td>
                    </tr>
                    <tr>
                      <th scope="row">Unrealized vs cost</th>
                      <td>
                        {investment.unrealizedPerformanceMinor == null
                          ? "unknown"
                          : formatUsd(
                              investment.unrealizedPerformanceMinor,
                              investment.scale,
                            )}
                      </td>
                      <td>calculated</td>
                    </tr>
                    <tr>
                      <th scope="row">Unrealized %</th>
                      <td>{formatBps(investment.unrealizedPnlBps)}</td>
                      <td>calculated</td>
                    </tr>
                    <tr>
                      <th scope="row">Plan / share</th>
                      <td>
                        <input
                          aria-label="Stored Plan per share"
                          value={pdDraft.plan}
                          onChange={(e) => patchDraft({ plan: e.target.value })}
                          disabled={busy || writesBlocked}
                        />
                      </td>
                      <td>editable</td>
                    </tr>
                    <tr>
                      <th scope="row">Plan last adjusted</th>
                      <td>
                        {investment.planEffectiveFrom?.trim() || "unknown"}
                      </td>
                      <td>from PlanHistory</td>
                    </tr>
                    <tr>
                      <th scope="row">Plan reason</th>
                      <td>
                        <select
                          aria-label="Stored Plan decision reason"
                          value={pdDraft.planReason}
                          onChange={(e) => patchDraft({ planReason: e.target.value })}
                          disabled={busy || writesBlocked}
                        >
                          <option value="">Choose reason</option>
                          {PLAN_REASONS.map((reason) => (
                            <option key={reason} value={reason}>
                              {reason}
                            </option>
                          ))}
                        </select>
                      </td>
                      <td>editable</td>
                    </tr>
                    {investment.review.incompleteReasonRequired ? (
                      <tr>
                        <th scope="row">Incomplete analysis</th>
                        <td>
                          <select
                            aria-label="Stored incomplete analysis reason"
                            value={pdDraft.incomplete}
                            onChange={(e) => patchDraft({ incomplete: e.target.value })}
                            disabled={busy || writesBlocked}
                          >
                            <option value="">Choose why full analysis is not possible</option>
                            {INCOMPLETE_REASONS.map((reason) => (
                              <option key={reason} value={reason}>
                                {reason}
                              </option>
                            ))}
                          </select>
                        </td>
                        <td>editable</td>
                      </tr>
                    ) : null}
                    <tr>
                      <th scope="row">Annual Plan cash</th>
                      <td>
                        {investment.annualPlanMinor == null
                          ? "unknown — confirm Plan"
                          : formatUsd(investment.annualPlanMinor, investment.scale)}
                      </td>
                      <td>calculated</td>
                    </tr>
                    <tr>
                      <th scope="row">Plan YOC</th>
                      <td>{formatBps(investment.planYocBps)}</td>
                      <td>calculated</td>
                    </tr>
                    <tr>
                      <th scope="row">Plan FWD yield</th>
                      <td>{formatBps(investment.planFwdYieldBps)}</td>
                      <td>calculated</td>
                    </tr>
                    <tr>
                      <th scope="row">Most Current FWD yield</th>
                      <td>{formatBps(investment.mostCurrentFwdYieldBps)}</td>
                      <td>calculated</td>
                    </tr>
                    <tr>
                      <th scope="row">Most Current</th>
                      <td>
                        {investment.review.mostCurrentMinor == null
                          ? "N/A"
                          : `$${formatScaled(investment.review.mostCurrentMinor, investment.review.amountScale)}`}
                      </td>
                      <td>calculated</td>
                    </tr>
                    <tr>
                      <th scope="row">Avg 6</th>
                      <td>
                        {investment.review.avg6Minor == null
                          ? "incomplete"
                          : `$${formatScaled(investment.review.avg6Minor, investment.review.amountScale)}`}
                      </td>
                      <td>calculated</td>
                    </tr>
                    <tr>
                      <th scope="row">Min / max / 80% of avg</th>
                      <td>
                        {investment.review.minMinor == null
                          ? "N/A"
                          : `$${formatScaled(investment.review.minMinor, investment.review.amountScale)} / $${formatScaled(investment.review.maxMinor ?? 0, investment.review.amountScale)} / ${
                              investment.review.eightyPctOfAvgMinor == null
                                ? "N/A"
                                : `$${formatScaled(investment.review.eightyPctOfAvgMinor, investment.review.amountScale)}`
                            }`}
                      </td>
                      <td>calculated</td>
                    </tr>
                    <tr>
                      <th scope="row">Most Current vs Plan</th>
                      <td>
                        {mostCurrentVsPlanFace(investment.mostCurrentVsPlanBps)}
                      </td>
                      <td>calculated</td>
                    </tr>
                    <tr>
                      <th scope="row">Declarations</th>
                      <td>{formatCount(investment.declarationCount)} / 12 stored</td>
                      <td>observations</td>
                    </tr>
                    <tr>
                      <th scope="row">ROC 2024 actual %</th>
                      <td>
                        <input
                          aria-label="ROC 2024 actual"
                          value={pdDraft.roc2024}
                          onChange={(e) => patchDraft({ roc2024: e.target.value })}
                          disabled={busy || writesBlocked}
                        />
                      </td>
                      <td>editable</td>
                    </tr>
                    <tr>
                      <th scope="row">ROC 2025 actual %</th>
                      <td>
                        {rocActualUnavailable(investment.lots, 2025, asOfDate) ? (
                          <p aria-label="ROC 2025 actual">
                            {rocActualUnavailable(
                              investment.lots,
                              2025,
                              asOfDate,
                            )}
                          </p>
                        ) : (
                          <input
                            aria-label="ROC 2025 actual"
                            value={pdDraft.roc2025}
                            onChange={(e) =>
                              patchDraft({ roc2025: e.target.value })
                            }
                            disabled={busy || writesBlocked}
                          />
                        )}
                      </td>
                      <td>
                        {rocActualUnavailable(investment.lots, 2025, asOfDate) ||
                          "editable after 2025 1099"}
                      </td>
                    </tr>
                    <tr>
                      <th scope="row">ROC 2026 estimate %</th>
                      <td>
                        <input
                          aria-label="ROC 2026 estimate"
                          value={pdDraft.roc2026e}
                          onChange={(e) => patchDraft({ roc2026e: e.target.value })}
                          disabled={busy || writesBlocked}
                        />
                      </td>
                      <td>editable</td>
                    </tr>
                    <tr>
                      <th scope="row">ROC 2026 actual %</th>
                      <td>
                        {rocActualUnavailable(investment.lots, 2026, asOfDate) ? (
                          <p aria-label="ROC 2026 actual">
                            {rocActualUnavailable(
                              investment.lots,
                              2026,
                              asOfDate,
                            )}
                          </p>
                        ) : (
                          <input
                            aria-label="ROC 2026 actual"
                            value={pdDraft.roc2026a}
                            onChange={(e) =>
                              patchDraft({ roc2026a: e.target.value })
                            }
                            disabled={busy || writesBlocked}
                          />
                        )}
                      </td>
                      <td>
                        {rocActualUnavailable(investment.lots, 2026, asOfDate) ||
                          "editable after 2026 1099"}
                      </td>
                    </tr>
                    <tr>
                      <th scope="row">Notes</th>
                      <td>
                        <input
                          aria-label="Position notes"
                          value={pdDraft.notes}
                          onChange={(e) => patchDraft({ notes: e.target.value })}
                          disabled={busy || writesBlocked}
                        />
                      </td>
                      <td>editable</td>
                    </tr>
                    <tr>
                      <th scope="row">Retrieval template</th>
                      <td>
                        <p aria-label="Adapter source">
                          Adapter:{" "}
                          {investment.template?.declarationSource ||
                            "unassigned"}
                        </p>
                        <p>
                          Schedule:{" "}
                          {calendarPolicyLabel(
                            investment.template?.calendarPolicy,
                          )}
                        </p>
                        <p>
                          Source URL:{" "}
                          {investment.template?.sourceUrl?.trim()
                            ? investment.template.sourceUrl
                            : "—"}
                        </p>
                        <p>
                          Lookback:{" "}
                          {formatCount(
                            investment.template?.lookbackCount ??
                              DECLARATION_LOOKBACK_TARGET,
                          )}
                        </p>
                        <p>
                          Inception (optional):{" "}
                          {investment.template?.inceptionOn?.trim()
                            ? investment.template.inceptionOn
                            : "— (only used when paid decls are under 12)"}
                        </p>
                        {investment.template?.lastRunAt ? (
                          <p>
                            Last run {investment.template.lastRunAt}
                            {investment.template.lastRunOk == null
                              ? ""
                              : investment.template.lastRunOk
                                ? " succeeded"
                                : " missed"}
                            {investment.template.lastRunMessage
                              ? `: ${investment.template.lastRunMessage}`
                              : ""}
                          </p>
                        ) : (
                          <p>Last run: never</p>
                        )}
                        <p>
                          Edit retrieval templates on Settings. Hub face source is
                          the adapter, not the stored template knobs.
                        </p>
                        <button
                          type="button"
                          aria-label="Open Settings retrieval templates"
                          onClick={() => setScreen("settings")}
                        >
                          Open Settings
                        </button>
                      </td>
                      <td>Settings</td>
                    </tr>
                    <tr>
                      <th scope="row">Backtest / scores</th>
                      <td>
                        {(investment.results ?? []).length === 0
                          ? "unknown — dated bull/bear windows required before scores. Missing evidence stays missing; it does not invent a rank."
                          : `${formatCount(investment.results.length)} stored window result${investment.results.length === 1 ? "" : "s"}. Suggestion does not change Risk until you Apply.`}
                      </td>
                      <td>
                        {(investment.results ?? []).length === 0 ? "unknown" : "calculated"}
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>
              </details>
              <section className="hub-panel" id="hub-declarations" aria-label="Declarations" data-focus={positionFocusPanel === "declarations" ? "1" : undefined}>
              <h3>Declarations</h3>
              {(() => {
                const decls = investment.declarations ?? [];
                const paid = decls.filter((d) => d.amountPerShareMinor != null).length;
                const stored = decls.length;
                const asOf = asOfDate || new Date().toISOString().slice(0, 10);
                const inception = investment.template?.inceptionOn?.trim() || "";
                let short: string | null = null;
                let needLabel = `need ${formatCount(DECLARATION_LOOKBACK_TARGET)}`;
                if (paid >= DECLARATION_LOOKBACK_TARGET) {
                  needLabel = `${formatCount(DECLARATION_LOOKBACK_TARGET)}+ paid — inception N/A`;
                } else if (inception) {
                  const expected = expectedDeclarationLookback(
                    inception,
                    asOf,
                    pdDraft.freq || investment.paymentFrequency,
                  );
                  needLabel = `need ${formatCount(expected)} (inception-limited; full is ${formatCount(DECLARATION_LOOKBACK_TARGET)})`;
                  if (paid < expected) {
                    short = `Adapter returned ${formatCount(paid)} of ${formatCount(expected)} paid declarations expected since inception ${inception}.`;
                  }
                } else {
                  short = `Adapter returned ${formatCount(paid)} of ${formatCount(DECLARATION_LOOKBACK_TARGET)} required paid declarations. Optional inception on Settings only if this name is too new for a full lookback.`;
                }
                return (
                  <>
                    <h4>Latest paid ({needLabel})</h4>
                    <p>
                      Paid $/share from the adapter. Blank stays unknown, not $0.
                      Stored: {formatCount(stored)}. Retrieve first; inception is
                      consulted only when under{" "}
                      {formatCount(DECLARATION_LOOKBACK_TARGET)} paid points.
                    </p>
                    {short ? (
                      <p role="status" aria-label="Declaration shortfall">
                        {short}
                      </p>
                    ) : null}
                    <DeclarationPaymentsChart
                      declarations={(investment.declarations ?? []).map((d) => ({
                        paymentPeriod: d.paymentPeriod,
                        amountPerShareMinor: d.amountPerShareMinor ?? 0,
                        amountScale: d.amountScale,
                      }))}
                      asOfDate={asOf}
                    />
                    {stored === 0 ? (
                      <p>No issuer declarations stored.</p>
                    ) : (
                      <div className="table-wrap">
                        <table aria-label="Stored declarations">
                          <thead>
                            <tr>
                              <th scope="col">Period</th>
                              <th className="numeric" scope="col">
                                Per share
                              </th>
                              <th scope="col">Source</th>
                            </tr>
                          </thead>
                          <tbody>
                            {decls.map((row, i) => (
                              <tr key={`${row.paymentPeriod}-${i}`}>
                                <td>{row.paymentPeriod || "—"}</td>
                                <td className="numeric">
                                  {row.amountPerShareMinor == null
                                    ? "—"
                                    : `$${formatScaled(row.amountPerShareMinor, row.amountScale)}`}
                                </td>
                                <td>{row.source || "—"}</td>
                              </tr>
                            ))}
                          </tbody>
                        </table>
                      </div>
                    )}
                  </>
                );
              })()}
              </section>
              <section className="hub-panel" id="hub-income" aria-label="Plan payment summary" data-focus={positionFocusPanel === "income" ? "1" : undefined}>
              <h3>Plan payment summary</h3>
              <p>
                Remaining-year Plan cash (Plan × quantity). Blank stays unknown, not $0.
                Separate from paid declaration history and broker ledger.
              </p>
              {pdRemaining == null ? (
                <p>Loading plan payment summary…</p>
              ) : (
                <>
                  <p aria-label="Remaining year schedule provenance">
                    {pdRemaining.known
                      ? dateProvenanceLabel(pdRemaining.provenance)
                      : `${dateProvenanceLabel(pdRemaining.provenance)}. Unknown stays unknown.`}
                    {pdRemaining.calendarPolicy
                      ? ` Schedule: ${calendarPolicyLabel(pdRemaining.calendarPolicy)}.`
                      : ""}
                  </p>
                  <div className="table-wrap">
                    <table aria-label="Plan payment summary">
                      <thead>
                        <tr>
                          <th scope="col">Period</th>
                          <th className="numeric" scope="col">Plan cash</th>
                        </tr>
                      </thead>
                      <tbody>
                        <tr>
                          <th scope="row">Remaining this year</th>
                          <td className="numeric">
                            {pdRemaining.yearToGoMinor == null
                              ? "unknown"
                              : formatUsd(pdRemaining.yearToGoMinor, pdRemaining.scale)}
                          </td>
                        </tr>
                        {pdRemaining.months.map((m) => (
                          <tr key={m.month}>
                            <th scope="row">{m.month}</th>
                            <td className="numeric">
                              {m.cashMinor == null
                                ? "unknown"
                                : formatUsd(m.cashMinor, pdRemaining.scale)}
                            </td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                </>
              )}
              </section>
              <section className="hub-panel" id="hub-pay-dates" aria-label="Future plan payment dates">
              <h3>Future plan payment dates</h3>
              <p>Each remaining pay date × Plan × open quantity for this symbol.</p>
              {pdRemaining == null ? (
                <p>Loading pay dates…</p>
              ) : (pdRemaining.payments ?? []).length === 0 ? (
                <p>No remaining-year pay dates yet.</p>
              ) : (
                <div className="table-wrap">
                  <table aria-label="Future plan payment dates">
                    <thead>
                      <tr>
                        <th scope="col">Pay on</th>
                        <th className="numeric" scope="col">Plan cash</th>
                        <th scope="col">Date source</th>
                      </tr>
                    </thead>
                    <tbody>
                      {pdRemaining.payments.map((pay) => (
                        <tr key={pay.originalPayOn}>
                          <td>{pay.payOn}</td>
                          <td className="numeric">
                            {pay.cashMinor == null
                              ? "unknown"
                              : formatUsd(pay.cashMinor, pdRemaining.scale)}
                          </td>
                          <td>
                            {dateProvenanceLabel(
                              pay.dateProvenance || pay.originalPayOn,
                            )}
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              )}
              </section>
              <section
                className="hub-panel"
                id="hub-received"
                aria-label="Received payments"
              >
                <h3>Received payments</h3>
                <p>
                  Broker cash already received for this symbol (Investment Activity
                  Ledger). All-years and per calendar year. Unknown stays unknown.
                </p>
                {(() => {
                  const actuals = pdLedger?.actuals ?? [];
                  const scale = pdLedger?.scale ?? investment.scale;
                  const byYear = receivedByYear(actuals);
                  const allYears =
                    investment.distributionsScope === "incomplete" ||
                    investment.totalDistributionsReceivedMinor == null
                      ? null
                      : investment.totalDistributionsReceivedMinor;
                  const ledgerSum = actuals.reduce((s, a) => s + a.amountMinor, 0);
                  return (
                    <>
                      <div className="table-wrap">
                        <table aria-label="Received payment totals">
                          <thead>
                            <tr>
                              <th scope="col">Scope</th>
                              <th className="numeric" scope="col">
                                Received
                              </th>
                            </tr>
                          </thead>
                          <tbody>
                            <tr>
                              <th scope="row">All years</th>
                              <td className="numeric">
                                {allYears == null
                                  ? actuals.length === 0
                                    ? "unknown"
                                    : formatUsd(ledgerSum, scale)
                                  : formatUsd(allYears, investment.scale)}
                              </td>
                            </tr>
                            {investment.rocDistributionsMinor != null ? (
                              <tr>
                                <th scope="row">ROC component (all years)</th>
                                <td className="numeric">
                                  {formatUsd(
                                    investment.rocDistributionsMinor,
                                    investment.scale,
                                  )}
                                </td>
                              </tr>
                            ) : null}
                          </tbody>
                        </table>
                      </div>
                      {byYear.length === 0 ? (
                        <p>No ledger dividends stored for this symbol yet.</p>
                      ) : (
                        <div className="table-wrap">
                          <table aria-label="Received payments by year">
                            <thead>
                              <tr>
                                <th scope="col">Year</th>
                                <th className="numeric" scope="col">
                                  Received
                                </th>
                              </tr>
                            </thead>
                            <tbody>
                              {byYear.map((row) => (
                                <tr key={row.year}>
                                  <td>{row.year}</td>
                                  <td className="numeric">
                                    {formatUsd(row.amountMinor, scale)}
                                  </td>
                                </tr>
                              ))}
                            </tbody>
                          </table>
                        </div>
                      )}
                    </>
                  );
                })()}
              </section>
              <section
                className="hub-panel"
                id="hub-ledger"
                aria-label="Ledger income"
                data-focus={positionFocusPanel === "ledger" ? "1" : undefined}
              >
                <h3>Ledger income</h3>
                <p>
                  Broker-paid dividends for this symbol (Investment Activity Ledger).
                  Unknown stays unknown.
                </p>
                {(pdLedger?.actuals ?? []).length === 0 ? (
                  <p>No ledger dividends stored for this symbol yet.</p>
                ) : (
                  <div className="table-wrap">
                    <table aria-label="Ledger dividends">
                      <thead>
                        <tr>
                          <th scope="col">Occurred</th>
                          <th className="numeric" scope="col">
                            Amount
                          </th>
                        </tr>
                      </thead>
                      <tbody>
                        {(pdLedger?.actuals ?? []).map((row) => (
                          <tr key={row.actualId}>
                            <td>{row.occurredOn}</td>
                            <td className="numeric">
                              {formatUsd(row.amountMinor, row.scale)}
                            </td>
                          </tr>
                        ))}
                      </tbody>
                      <tfoot>
                        <tr>
                          <td>Total</td>
                          <td className="numeric">
                            {formatUsd(
                              (pdLedger?.actuals ?? []).reduce(
                                (s, a) => s + a.amountMinor,
                                0,
                              ),
                              pdLedger?.scale ?? 2,
                            )}
                          </td>
                        </tr>
                      </tfoot>
                    </table>
                  </div>
                )}
              </section>
              <section
                className="hub-panel"
                id="hub-lots"
                aria-label="Lots by account"
                data-focus={positionFocusPanel === "lots" ? "1" : undefined}
              >
                <h3>Lots by account</h3>
                <p>Lots and cost for the chosen ticker only. Data totals above stay put.</p>
                {investment.lots.length === 0 ? (
                  <p>No open lots. Calculator omits this symbol until the first lot.</p>
                ) : (
                  <SymbolLotsTable lots={investment.lots} />
                )}
              </section>
              <section className="hub-panel" aria-label="Backtests">
              <h3>Owner period</h3>
              <p>
                You name the window. The system does not pick bull or bear dates.
                Save the period, then calculate. Window prices stay candidates.
              </p>
              <div className="table-wrap">
                <table aria-label="Owner period draft">
                  <tbody>
                    <tr>
                      <th scope="row">Kind</th>
                      <td>
                        <select
                          aria-label="Period kind"
                          value={pdPeriod.kind}
                          onChange={(e) => patchPeriod({ kind: e.target.value })}
                          disabled={busy || writesBlocked}
                        >
                          <option value="">Choose kind</option>
                          {PERIOD_KINDS.map((kind) => (
                            <option key={kind} value={kind}>
                              {kind}
                            </option>
                          ))}
                        </select>
                      </td>
                    </tr>
                    <tr>
                      <th scope="row">Name</th>
                      <td>
                        <input
                          aria-label="Period name"
                          value={pdPeriod.name}
                          onChange={(e) => patchPeriod({ name: e.target.value })}
                          disabled={busy || writesBlocked}
                        />
                      </td>
                    </tr>
                    <tr>
                      <th scope="row">Start</th>
                      <td>
                        <input
                          type="date"
                          aria-label="Period start"
                          value={pdPeriod.startOn}
                          onChange={(e) => patchPeriod({ startOn: e.target.value })}
                          disabled={busy || writesBlocked}
                        />
                      </td>
                    </tr>
                    <tr>
                      <th scope="row">End</th>
                      <td>
                        <input
                          type="date"
                          aria-label="Period end"
                          value={pdPeriod.endOn}
                          onChange={(e) => patchPeriod({ endOn: e.target.value })}
                          disabled={busy || writesBlocked}
                        />
                      </td>
                    </tr>
                    <tr>
                      <th scope="row">Benchmark</th>
                      <td>
                        <input
                          aria-label="Period benchmark"
                          value={pdPeriod.benchmark}
                          onChange={(e) => patchPeriod({ benchmark: e.target.value })}
                          disabled={busy || writesBlocked}
                        />
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>
              <div className="buttons dossier-actions">
                <button
                  type="button"
                  aria-label="Save owner period"
                  disabled={busy || writesBlocked || !periodReady || !periodDirty}
                  onClick={() => void saveOwnerPeriod()}
                >
                  Save period
                </button>
                <button
                  type="button"
                  aria-label="Calculate window"
                  disabled={
                    busy ||
                    writesBlocked ||
                    periodDirty ||
                    !(savedPeriodId || investment.periods.at(-1)?.periodId)
                  }
                  onClick={() => void calculateWindow()}
                >
                  Calculate window
                </button>
              </div>
              {(investment.periods ?? []).length === 0 ? (
                <p>No owner periods stored.</p>
              ) : (
                <div className="table-wrap">
                  <table aria-label="Stored owner periods">
                    <thead>
                      <tr>
                        <th scope="col">Kind</th>
                        <th scope="col">Dates</th>
                        <th scope="col">Benchmark</th>
                      </tr>
                    </thead>
                    <tbody>
                      {investment.periods.map((period) => (
                        <tr key={period.periodId}>
                          <td>{period.kind}</td>
                          <td>
                            {period.startOn} – {period.endOn}
                          </td>
                          <td>{period.benchmarkSymbol || "—"}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              )}
              <h3>Window results</h3>
              {(investment.results ?? []).length === 0 ? (
                <p>No calculated window. Unknown stays unknown.</p>
              ) : (
                <div className="table-wrap">
                  <table aria-label="Window results">
                    <thead>
                      <tr>
                        <th scope="col">Metric</th>
                        <th scope="col">Value</th>
                      </tr>
                    </thead>
                    <tbody>
                      {(() => {
                        const latest = investment.results[investment.results.length - 1];
                        return (
                          <>
                            <tr>
                              <th scope="row">Price return</th>
                              <td>{formatBps(latest.priceReturnBps)}</td>
                            </tr>
                            <tr>
                              <th scope="row">Total return</th>
                              <td>{formatBps(latest.totalReturnBps)}</td>
                            </tr>
                            <tr>
                              <th scope="row">Cushion</th>
                              <td>{formatBps(latest.cushionBps)}</td>
                            </tr>
                            <tr>
                              <th scope="row">Max drawdown</th>
                              <td>{formatBps(latest.maxDrawdownBps)}</td>
                            </tr>
                            <tr>
                              <th scope="row">Recovery</th>
                              <td>
                                {latest.recoveryRatioBps == null
                                  ? "unknown"
                                  : `${formatBps(latest.recoveryRatioBps)}${
                                      latest.recoveryDays != null
                                        ? ` / ${latest.recoveryDays}d`
                                        : ""
                                    }`}
                              </td>
                            </tr>
                            <tr>
                              <th scope="row">Income reliability</th>
                              <td>{formatBps(latest.incomeReliabilityBps)}</td>
                            </tr>
                            <tr>
                              <th scope="row">Bear relative</th>
                              <td>{formatBps(latest.bearRelativeBps)}</td>
                            </tr>
                            <tr>
                              <th scope="row">Downside capture</th>
                              <td>{formatBps(latest.downsideCaptureBps)}</td>
                            </tr>
                            <tr>
                              <th scope="row">Upside capture</th>
                              <td>{formatBps(latest.upsideCaptureBps)}</td>
                            </tr>
                            <tr>
                              <th scope="row">Completeness</th>
                              <td>{latest.completeness}</td>
                            </tr>
                          </>
                        );
                      })()}
                    </tbody>
                  </table>
                </div>
              )}
              <h3>Evidence</h3>
              {investment.evidence == null ? (
                <p>No evidence dimensions until a window is calculated.</p>
              ) : (
                <dl className="health" aria-label="Evidence dimensions">
                  <dt>Income Reliability</dt>
                  <dd>
                    {investment.evidence.incomeReliability == null
                      ? "unknown"
                      : investment.evidence.incomeReliability}
                  </dd>
                  <dt>Downside Resilience</dt>
                  <dd>
                    {investment.evidence.downsideResilience == null
                      ? "unknown"
                      : investment.evidence.downsideResilience}
                  </dd>
                  <dt>Recovery / Upside</dt>
                  <dd>
                    {investment.evidence.recoveryUpside == null
                      ? "unknown"
                      : investment.evidence.recoveryUpside}
                  </dd>
                  <dt>NAV Persistence</dt>
                  <dd>
                    {investment.evidence.navPersistence == null
                      ? "unknown"
                      : investment.evidence.navPersistence}
                  </dd>
                  <dt>Diversification</dt>
                  <dd>
                    {investment.evidence.diversification == null
                      ? "legacy text only"
                      : investment.evidence.diversification}
                  </dd>
                  <dt>Data Confidence</dt>
                  <dd>
                    {investment.evidence.dataConfidence} ({investment.evidence.knownComponents} known)
                  </dd>
                </dl>
              )}
              <h3>Tier suggestion</h3>
              {investment.suggestion == null || !investment.suggestion.complete ? (
                <p>
                  {investment.suggestion?.reason ??
                    "No complete suggestion. Apply stays disabled."}
                </p>
              ) : (
                <p>
                  {investment.suggestion.suggestedTier} ({investment.suggestion.ruleset}):{" "}
                  {investment.suggestion.reason}
                </p>
              )}
              <div className="buttons dossier-actions">
                {RISK_TIERS.map((tier) => (
                  <button
                    key={tier}
                    type="button"
                    aria-label={`Apply ${tier}`}
                    disabled={
                      busy ||
                      writesBlocked ||
                      factsDirty ||
                      !investment.suggestion?.complete
                    }
                    onClick={() => void applySuggestedTier(tier)}
                  >
                    Apply {tier}
                  </button>
                ))}
              </div>
              </section>
            </>

          ) : (
            <p>Choose a symbol to open the position hub.</p>
          )}
          <details className="hub-fleet" aria-label="Position fleet compare">
            <summary>
              {positionSymbol
                ? `Fleet compare (all symbols) — ${positionSymbol} hub is above`
                : "Fleet compare — all symbols"}
            </summary>
            <section aria-label="Issuer retrieve miss summary">
              <ExceptionList exceptions={exceptions} onOpenLog={() => void openExceptionLog()} />
            </section>
            <h3>Issuer retrieve</h3>
            <div className="table-wrap">
              <table aria-label="Issuer retrieve">
                <thead>
                  <tr>
                    <th scope="col">Symbol</th>
                    <th scope="col">Source</th>
                    <th scope="col">Freshness</th>
                    <th scope="col">Last run</th>
                    <th scope="col">Miss</th>
                  </tr>
                </thead>
                <tbody>
                  {(issuerCoverage?.rows ?? []).map((row) => (
                    <tr key={row.securityId}>
                      <td>{row.symbol}</td>
                      <td>{row.declarationSource || "unassigned"}</td>
                      <td>{row.declarationFreshness || "unavailable"}</td>
                      <td>
                        {row.lastRunAt || "never"}
                        {row.lastRunOk === true
                          ? " ok"
                          : row.lastRunOk === false
                            ? " miss"
                            : ""}
                      </td>
                      <td>
                        {row.lastRunOk === false
                          ? row.lastRunMessage || "Issuer page empty."
                          : "—"}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            <PositionMasterTable
              rows={positionMaster?.rows ?? null}
              selectedSymbol={positionSymbol}
              onOpenSymbol={(symbol) => {
                leaveWithoutSaving(() => {
                  setPositionFocusPanel("");
                  setPositionSymbol(symbol);
                  void loadInvestment(symbol);
                });
              }}
            />
            <h3>Account splits (all symbols)</h3>
            <p>Account × symbol quantity and cost. Filter to the open hub symbol when set.</p>
            <PositionDetailsTable
              positions={positionDetails}
              filter={positionSymbol}
            />
          </details>
        </section>
      ) : null}

      {screen === "dashboard" ? (
        <section aria-label="Dashboard">
          <h2>Dashboard</h2>
          <DashboardBurndownPanel burndown={burndown} />
        </section>
      ) : null}

      {screen === "trends" ? (
        <section aria-label="Trends">
          <h2>Trends</h2>
          <TrendsCapturePanel
            capture={trendsCapture}
            busy={busy}
            onReload={(d) => {
              void (async () => {
                const r = await client.executeQuery("TrendsWeekGet", { asOfDate: d });
                if (r.ok && r.bodyJson) {
                  setTrendsCapture(JSON.parse(r.bodyJson) as TrendsWeekCapture);
                }
              })();
            }}
            onCopyPrior={() => {
              if (!trendsCapture?.prior) return;
              setTrendsCapture({
                ...trendsCapture,
                current: {
                  profitMinor: trendsCapture.prior.profitMinor,
                  monthlyDivsMinor: trendsCapture.prior.monthlyDivsMinor,
                  fidelityTotalMinor: trendsCapture.prior.fidelityTotalMinor,
                  schwabTotalMinor: trendsCapture.prior.schwabTotalMinor,
                  incomeCashMinor: trendsCapture.prior.incomeCashMinor,
                  acct9CashMinor: trendsCapture.prior.acct9CashMinor,
                  acct9EtfValueMinor: trendsCapture.prior.acct9EtfValueMinor,
                },
              });
            }}
            onSave={async (body, correct) => {
              setBusy(true);
              try {
                const r = await client.executeCommand(
                  correct ? "TrendsWeekCorrect" : "TrendsWeekSave",
                  body,
                );
                if (!r.ok) {
                  setActionMessage(`Trends save failed: ${r.errorCode ?? "error"}`);
                  return;
                }
                if (r.bodyJson) {
                  setTrendsCapture(JSON.parse(r.bodyJson) as TrendsWeekCapture);
                }
                await refreshData(asOfDate || body.periodEnd as string);
              } finally {
                setBusy(false);
              }
            }}
            onClose={async (periodEnd) => {
              setBusy(true);
              try {
                const r = await client.executeCommand("TrendsWeekClose", { periodEnd });
                if (!r.ok) {
                  setActionMessage(`Trends close failed: ${r.errorCode ?? "error"}`);
                  return;
                }
                if (r.bodyJson) {
                  setTrendsCapture(JSON.parse(r.bodyJson) as TrendsWeekCapture);
                }
                await refreshData(asOfDate || periodEnd);
              } finally {
                setBusy(false);
              }
            }}
          />
          <TrendsChartsPanel
            weeks={trends?.weeks}
            note={trends?.note}
            error={trendsError}
            overview={trends?.overview as never}
            distributions={trends?.distributions as never}
            taxMonitor={trends?.taxMonitor as never}
            missingRequired={trends?.missingRequired}
          />
        </section>
      ) : null}

      {screen === "holdings" ? (
        <section aria-label="Holdings">
          <h2>Holdings</h2>
          <p>Open lots. Owner assigns sales; no FIFO.</p>
          <label>
            Filter holdings
            <input
              value={holdingsFilter}
              onChange={(e) => setHoldingsFilter(e.target.value)}
              aria-label="Filter holdings"
            />
          </label>
          <HoldingsPanel
            lots={holdings?.lots ?? null}
            filter={holdingsFilter}
            onOpenSymbol={(symbol) => openPositionHub(symbol, "lots")}
          />
          <p>Open lots. Owner assigns sales; no FIFO. Type a sell activity id to assign.</p>
          <label>
            Lot id
            <input
              value={lotId}
              onChange={(e) => setLotId(e.target.value)}
              disabled={busy || writesBlocked}
            />
          </label>
          <label>
            Sell activity id
            <input
              value={assignActivityId}
              onChange={(e) => setAssignActivityId(e.target.value)}
              disabled={busy || writesBlocked}
            />
          </label>
          <label>
            Quantity minor
            <input
              value={assignQty}
              onChange={(e) => setAssignQty(e.target.value)}
              disabled={busy || writesBlocked}
            />
          </label>
          <button
            type="button"
            aria-label="Assign lot"
            disabled={busy || writesBlocked}
            onClick={() =>
              void runCommand("LotAssign", {
                lotId,
                activityId: assignActivityId,
                quantityMinor: Number(assignQty),
                quantityScale: selectedLot?.quantityScale ?? 0,
              })
            }
          >
            Assign lot
          </button>
        </section>
      ) : null}

      {screen === "new-investment" ? (
        <section aria-label="Add Position">
          <h2>Add Position</h2>
          {wizProcessASaved && wizSecurityId ? (
            <section aria-label="Process A completion">
              <h3>Research saved</h3>
              <p>
                Process A finished for this identity. Zero lots is valid. Choose a next
                action — this screen stays until you research another symbol.
              </p>
              <dl className="process-a-completion-summary" aria-label="Saved identity summary">
                <div>
                  <dt>Symbol</dt>
                  <dd>{wizSymbol.trim().toUpperCase() || "—"}</dd>
                </div>
                <div>
                  <dt>Name</dt>
                  <dd>{wizName.trim() || "unknown"}</dd>
                </div>
                <div>
                  <dt>Lot count</dt>
                  <dd>{formatCount(processALotCount)}</dd>
                </div>
                <div>
                  <dt>Overall</dt>
                  <dd>
                    {processAFieldStatus.frequency &&
                    processAFieldStatus.declarations &&
                    processAFieldStatus.roc &&
                    processAFieldStatus.price
                      ? "researched"
                      : "incomplete"}
                  </dd>
                </div>
              </dl>
              <table aria-label="Researched versus incomplete">
                <thead>
                  <tr>
                    <th scope="col">Field</th>
                    <th scope="col">Status</th>
                  </tr>
                </thead>
                <tbody>
                  <tr>
                    <th scope="row">Frequency</th>
                    <td>{processAFieldStatus.frequency ? "filled" : "unknown"}</td>
                  </tr>
                  <tr>
                    <th scope="row">Declarations</th>
                    <td>
                      {processAFieldStatus.declarations
                        ? `filled (${formatCount(wizDecls.length)})`
                        : "unknown"}
                    </td>
                  </tr>
                  <tr>
                    <th scope="row">ROC estimate</th>
                    <td>
                      {processAFieldStatus.roc
                        ? `filled (${((wizRoc!.rocPctMinor as number) / 10 ** wizRoc!.scale).toFixed(wizRoc!.scale)}%)`
                        : "unknown"}
                    </td>
                  </tr>
                  <tr>
                    <th scope="row">Price</th>
                    <td>
                      {processAFieldStatus.price
                        ? `filled (${scaledDollars(wizPriceState!.priceMinor as number, wizPriceState!.scale)})`
                        : "unknown"}
                    </td>
                  </tr>
                </tbody>
              </table>
              <div className="buttons dossier-actions">
                <button
                  type="button"
                  aria-label="Open Position Details"
                  disabled={busy}
                  onClick={() => goProcessAToPositionDetails()}
                >
                  Open Position Details
                </button>
                <button
                  type="button"
                  aria-label="Complete research"
                  disabled={
                    busy ||
                    writesBlocked ||
                    Boolean(researchActivity?.running) ||
                    !(wizSourceUrl.trim() || wizDeclSource.trim())
                  }
                  onClick={() => void researchRoc()}
                >
                  {researchActivity?.running ? "Completing research…" : "Complete research"}
                </button>
                <button
                  type="button"
                  aria-label="Add lots"
                  disabled={busy}
                  onClick={() => goProcessAToAddLot()}
                >
                  Add lots
                </button>
                <button
                  type="button"
                  aria-label="Research another position"
                  disabled={busy || writesBlocked}
                  onClick={() => startAnotherProcessA()}
                >
                  Research another
                </button>
              </div>
            </section>
          ) : (
            <>
          <p>
            Process A: enter symbol and the issuer distribution URL, then Research.
            Retrieved facts show below; missing stays unknown — never $0. Confirm Plan
            and Apply tier are explicit owner actions. Lots are opened on Add Lot, not here.
            Save ends Process A on an explicit completion screen.
          </p>
          <div className="form-grid process-a-inputs">
            <label>
              Symbol
              <input
                aria-label="Symbol"
                value={wizSymbol}
                onChange={(e) => {
                  setWizSymbol(e.target.value.toUpperCase());
                  setWizResearchDone(false);
                  setWizProcessASaved(false);
                }}
                disabled={busy || writesBlocked}
                autoComplete="off"
              />
            </label>
            <label>
              Distribution URL
              <input
                aria-label="Distribution URL"
                value={wizSourceUrl}
                onChange={(e) => {
                  setWizSourceUrl(e.target.value);
                  setWizResearchDone(false);
                  setWizProcessASaved(false);
                }}
                disabled={busy || writesBlocked}
                placeholder="https://…/#distributions"
                autoComplete="off"
              />
            </label>
          </div>
          <div className="buttons dossier-actions">
            <button
              type="button"
              aria-label="Research"
              disabled={
                busy ||
                Boolean(researchActivity?.running) ||
                writesBlocked ||
                !wizSymbol.trim() ||
                !wizSourceUrl.trim()
              }
              onClick={() => void runProcessAResearch()}
            >
              Research
            </button>
            {wizResearchDone ? (
              <button
                type="button"
                aria-label="Save Process A research"
                disabled={busy || writesBlocked || !wizSecurityId}
                onClick={() => saveProcessAResearch()}
              >
                Save
              </button>
            ) : null}
          </div>
          {researchActivity?.running ? (
            <section
              className="process-a-research-progress"
              aria-label="Research progress"
              aria-busy="true"
            >
              <p role="status" aria-live="polite">
                {researchActivity.label}
              </p>
              {researchActivity.total > 0 ? (
                <progress
                  max={researchActivity.total}
                  value={Math.min(researchActivity.step, researchActivity.total)}
                />
              ) : (
                <div className="process-a-research-spinner" aria-hidden="true" />
              )}
            </section>
          ) : null}
          {researchActivity?.resultLine && !researchActivity.running ? (
            <p role="status" aria-label="Research result">
              {researchActivity.resultLine}
            </p>
          ) : null}
          {wizRetrieveNote && !researchActivity?.resultLine ? (
            <p role="status">{wizRetrieveNote}</p>
          ) : null}
          {wizResearchDone && !researchActivity?.running ? (
            <section aria-label="Research results">
              <h3>Research results</h3>
              <table aria-label="Retrieved versus unknown">
                <thead>
                  <tr>
                    <th scope="col">Field</th>
                    <th scope="col">Value</th>
                    <th scope="col">Status</th>
                  </tr>
                </thead>
                <tbody>
                  <tr>
                    <th scope="row">Provider / type</th>
                    <td>
                      {[wizProvider, wizDeclSource].filter(Boolean).join(" · ") || "—"}
                    </td>
                    <td>{wizProvider.trim() || wizDeclSource.trim() ? "retrieved" : "unknown"}</td>
                  </tr>
                  <tr>
                    <th scope="row">Underlying</th>
                    <td>{wizUnderlying.trim() || "—"}</td>
                    <td>{wizUnderlying.trim() ? "retrieved" : "unknown"}</td>
                  </tr>
                  <tr>
                    <th scope="row">Frequency</th>
                    <td>{wizFreq.trim() || "—"}</td>
                    <td>{parseCadence(wizFreq) ? "retrieved" : "unknown"}</td>
                  </tr>
                  <tr>
                    <th scope="row">Last 12 declarations</th>
                    <td>
                      {wizDecls.length > 0
                        ? `${formatCount(wizDecls.length)} paid`
                        : "—"}
                    </td>
                    <td>{wizDecls.length > 0 ? "retrieved" : "unknown"}</td>
                  </tr>
                  <tr>
                    <th scope="row">Remaining-year dates</th>
                    <td>
                      {(wizRemaining?.payments?.length ?? wizUpcomingPays.length) > 0
                        ? `${formatCount(wizRemaining?.payments?.length ?? wizUpcomingPays.length)} dates`
                        : "—"}
                    </td>
                    <td>
                      {(wizRemaining?.payments?.length ?? wizUpcomingPays.length) > 0
                        ? "retrieved"
                        : "unknown"}
                    </td>
                  </tr>
                  <tr>
                    <th scope="row">Suggested Plan</th>
                    <td>
                      {wizReview?.mostCurrentMinor != null
                        ? `${scaledDollars(wizReview.mostCurrentMinor, wizReview.amountScale)}/share (Most Current)`
                        : "—"}
                      {wizReview?.avg6Minor != null
                        ? `; Avg 6 ${scaledDollars(wizReview.avg6Minor, wizReview.amountScale)}`
                        : ""}
                    </td>
                    <td>
                      {wizReview?.mostCurrentMinor != null || wizReview?.avg6Minor != null
                        ? "retrieved"
                        : "unknown"}
                    </td>
                  </tr>
                  <tr>
                    <th scope="row">Last price</th>
                    <td>
                      {wizPriceState?.priceMinor != null && wizPriceState.priceMinor > 0
                        ? `${scaledDollars(wizPriceState.priceMinor, wizPriceState.scale)} (${wizPriceState.freshness || "unknown"})`
                        : "—"}
                    </td>
                    <td>
                      {wizPriceState?.priceMinor != null && wizPriceState.priceMinor > 0
                        ? "retrieved"
                        : "unknown"}
                    </td>
                  </tr>
                  <tr>
                    <th scope="row">Suggested tier</th>
                    <td>
                      {(wizTierSuggestion?.suggestedTier || wizLookthrough.riskTierSuggestion || "").trim()
                        ? `${wizTierSuggestion?.suggestedTier || wizLookthrough.riskTierSuggestion}${
                            (wizTierSuggestion?.reason || wizLookthrough.riskTierSuggestionReason)
                              ? ` — ${wizTierSuggestion?.reason || wizLookthrough.riskTierSuggestionReason}`
                              : ""
                          }`
                        : "—"}
                      {wizBacktestNeeded ? (
                        <p role="status" aria-label="Backtest dates required for tier suggest">
                          Backtest start/end (and method/source) are required to complete
                          ClassificationSuggest. Other research continues without a backtest.
                          Accept tier remains owner-only.
                        </p>
                      ) : null}
                      {wizAiThesis ? (
                        <p role="status" aria-label="AI thesis narrative">
                          Thesis (advisory): {wizAiThesis}
                        </p>
                      ) : null}
                    </td>
                    <td>
                      {(wizTierSuggestion?.suggestedTier || wizLookthrough.riskTierSuggestion || "").trim()
                        ? "retrieved"
                        : wizBacktestNeeded
                          ? "needs backtest"
                          : "unknown"}
                    </td>
                  </tr>
                </tbody>
              </table>

              {(researchNotes || wizAiThesis) ? (
                <section
                  className="research-notes-panel"
                  aria-label="Research notes"
                >
                  <h4>Research notes</h4>
                  <p className="research-notes-overview">
                    {researchNotes?.overview || wizAiThesis}
                  </p>
                  {(researchNotes?.suggestedTier ||
                    wizTierSuggestion?.suggestedTier ||
                    wizLookthrough.riskTierSuggestion) ? (
                    <p role="note" aria-label="Suggested risk tier">
                      Suggested risk (not applied):{" "}
                      {researchNotes?.suggestedTier ||
                        wizTierSuggestion?.suggestedTier ||
                        wizLookthrough.riskTierSuggestion}
                      . Owner sets Risk manually — never auto-applied.
                    </p>
                  ) : null}
                </section>
              ) : null}

              <section aria-label="ROC research strip" className="roc-research-strip">
                <h4>ROC research</h4>
                <p>
                  {wizRoc && wizRoc.rocPctMinor != null
                    ? `Proposed ${((wizRoc.rocPctMinor as number) / 10 ** wizRoc.scale).toFixed(wizRoc.scale)}% (2026 estimate, ${wizRoc.kind || "estimate"})${
                        wizRoc.sourceUrl ? ` — ${wizRoc.sourceUrl}` : ""
                      }. Not research-complete.`
                    : "2026 estimate unknown — not 0%. 2025 actual stays N/A when the position was not held that year."}
                </p>
                {wizSourceUrl.trim() || wizDeclSource.trim() ? (
                  <button
                    type="button"
                    aria-label="Complete research"
                    disabled={
                      busy ||
                      writesBlocked ||
                      !wizSecurityId ||
                      Boolean(researchActivity?.running)
                    }
                    onClick={() => void researchRoc()}
                  >
                    {researchActivity?.running
                      ? "Completing research…"
                      : "Complete research"}
                  </button>
                ) : null}
              </section>

              {wizDecls.length > 0 ? (
                <div className="table-wrap">
                  <p>Last {formatCount(wizDecls.length)} declarations (newest first).</p>
                  <table aria-label="Last declarations">
                    <thead>
                      <tr>
                        <th scope="col">Period</th>
                        <th className="numeric" scope="col">Per share</th>
                        <th scope="col">Source</th>
                      </tr>
                    </thead>
                    <tbody>
                      {wizDecls.map((d, i) => (
                        <tr key={`${d.paymentPeriod ?? i}`}>
                          <td>{d.paymentPeriod ?? "—"}</td>
                          <td className="numeric">
                            {formatScaled(d.amountPerShareMinor, d.amountScale ?? 4)}
                          </td>
                          <td>{d.source ?? "—"}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              ) : null}

              {(wizRemaining?.payments?.length ?? 0) > 0 ? (
                <div className="table-wrap">
                  <p>Remaining-year pay dates.</p>
                  <table aria-label="Remaining year dates">
                    <thead>
                      <tr>
                        <th scope="col">Pay on</th>
                        <th scope="col">Source</th>
                      </tr>
                    </thead>
                    <tbody>
                      {wizRemaining!.payments.map((p) => (
                        <tr key={p.payOn}>
                          <td>{p.payOn}</td>
                          <td>{dateProvenanceLabel(p.dateProvenance) || "—"}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              ) : null}

              <section aria-label="Owner plan and tier actions">
                <h4>Owner actions</h4>
                <p>
                  Confirm Plan needs declaration research
                  {parseCadence(wizFreq) ? "" : " and retrieved frequency"}.
                  Apply tier needs a suggested tier. Neither runs automatically.
                </p>
                <div className="form-grid">
                  <label>
                    Plan / share
                    <input
                      aria-label="Plan per share"
                      value={wizPlan}
                      onChange={(e) => setWizPlan(e.target.value)}
                      disabled={busy || writesBlocked}
                    />
                  </label>
                  <label>
                    Plan reason
                    <select
                      aria-label="Plan reason"
                      value={wizPlanReason}
                      onChange={(e) => setWizPlanReason(e.target.value)}
                      disabled={busy || writesBlocked}
                    >
                      <option value="">Select reason</option>
                      {PLAN_REASONS.map((r) => (
                        <option key={r} value={r}>
                          {r}
                        </option>
                      ))}
                    </select>
                  </label>
                  {wizReview?.incompleteReasonRequired ? (
                    <label>
                      Incomplete analysis reason
                      <input
                        aria-label="Incomplete analysis reason"
                        value={wizIncomplete}
                        onChange={(e) => setWizIncomplete(e.target.value)}
                        disabled={busy || writesBlocked}
                      />
                    </label>
                  ) : null}
                </div>
                <div className="buttons">
                  <button
                    type="button"
                    aria-label="Use Most Current as Plan"
                    disabled={
                      busy ||
                      writesBlocked ||
                      (wizReview?.mostCurrentMinor == null && wizDecls[0] == null)
                    }
                    onClick={() => {
                      const minor = wizReview?.mostCurrentMinor ?? wizDecls[0]?.amountPerShareMinor;
                      const scale = wizReview?.amountScale ?? wizDecls[0]?.amountScale ?? 4;
                      if (minor == null) return;
                      setWizPlan(scaledDollars(minor, scale));
                      setWizPlanReason("Match Most Current");
                    }}
                  >
                    Use Most Current as Plan
                  </button>
                  <button
                    type="button"
                    aria-label="Use Avg 6 as Plan"
                    disabled={busy || writesBlocked || wizReview?.avg6Minor == null}
                    onClick={() => {
                      if (wizReview?.avg6Minor == null) return;
                      setWizPlan(scaledDollars(wizReview.avg6Minor, wizReview.amountScale));
                      setWizPlanReason("Match Avg 6 (owner typed)");
                    }}
                  >
                    Use Avg 6 as Plan
                  </button>
                  <button
                    type="button"
                    aria-label="Confirm Plan"
                    disabled={
                      busy ||
                      writesBlocked ||
                      !wizSecurityId ||
                      !wizPlan.trim() ||
                      !wizPlanReason ||
                      !parseCadence(wizFreq) ||
                      Boolean(wizReview?.confirmBlocked) ||
                      (wizReview?.observationCount ?? 0) < 1 ||
                      (Boolean(wizReview?.incompleteReasonRequired) && !wizIncomplete.trim())
                    }
                    onClick={() => void confirmPlan()}
                  >
                    Confirm Plan
                  </button>
                  <button
                    type="button"
                    aria-label="Apply suggested tier"
                    disabled={
                      busy ||
                      writesBlocked ||
                      !wizSecurityId ||
                      !RISK_TIERS.includes(
                        (wizTierSuggestion?.suggestedTier ||
                          wizLookthrough.riskTierSuggestion ||
                          "").trim(),
                      )
                    }
                    onClick={() => void applyWizSuggestedTier()}
                  >
                    Apply tier
                  </button>
                </div>
                {wizPlanStored ? <p role="status">Plan confirmed.</p> : null}
                {wizRisk.trim() ? <p role="status">Applied tier: {wizRisk}</p> : null}
              </section>
            </section>
          ) : null}
            </>
          )}
        </section>
      ) : null}

      {screen === "add-lot" ? (
        <section aria-label="Add Lot">
          <h2>Add Lot</h2>
          <p>
            Process B: open an explicit lot on a researched symbol (Add Position first).
            Account, opened-on, quantity, original cost, tax cost if different, and origin only.
            No frequency, URL, ROC, or tier here. Opening is explicit — no FIFO. Zero lots after
            research is valid until you save.
          </p>
          {addLotDirty ? (
            <p className="blocked" role="status">
              Unsaved edits. Save or Cancel — other screens stay blocked.
            </p>
          ) : null}
          <div className="buttons dossier-actions">
            <button
              type="button"
              aria-label="Open lot"
              disabled={
                busy ||
                writesBlocked ||
                !addLotSecurityId ||
                !addLotAccountId ||
                !addLotOpenedOn.trim() ||
                !addLotQty.trim() ||
                !addLotCost.trim() ||
                (addLotTaxDifferent && !addLotTaxCost.trim())
              }
              onClick={() => void openAddLot()}
            >
              Open lot
            </button>
            <button
              type="button"
              aria-label="Cancel add lot edits"
              disabled={busy || !addLotDirty}
              onClick={() => cancelAddLotEdits()}
            >
              Cancel
            </button>
          </div>
          <div className="form-grid">
            <label className="symbol-combobox">
              Researched symbol
              <input
                aria-label="Add lot symbol"
                aria-expanded={addLotSymbolOpen}
                aria-controls="add-lot-symbol-list"
                aria-autocomplete="list"
                role="combobox"
                value={addLotQuery}
                placeholder="Type to filter (e.g. NV)"
                autoComplete="off"
                onChange={(e) => {
                  setAddLotQuery(e.target.value.toUpperCase());
                  setAddLotSecurityId("");
                  setAddLotSymbolOpen(true);
                }}
                onFocus={() => setAddLotSymbolOpen(true)}
                onBlur={() => {
                  window.setTimeout(() => setAddLotSymbolOpen(false), 150);
                }}
                onKeyDown={(e) => {
                  if (e.key === "Enter" && filteredAddLotSecurities[0]) {
                    e.preventDefault();
                    selectAddLotSecurity(filteredAddLotSecurities[0]);
                  }
                  if (e.key === "Escape") {
                    setAddLotSymbolOpen(false);
                  }
                }}
                disabled={busy || writesBlocked}
              />
              {addLotSymbolOpen && filteredAddLotSecurities.length > 0 ? (
                <ul
                  id="add-lot-symbol-list"
                  className="symbol-combobox-list"
                  role="listbox"
                  aria-label="Add lot symbol matches"
                >
                  {filteredAddLotSecurities.map((row) => (
                    <li
                      key={row.securityId}
                      role="option"
                      aria-selected={row.securityId === addLotSecurityId}
                      onMouseDown={(e) => {
                        e.preventDefault();
                        selectAddLotSecurity(row);
                      }}
                    >
                      {row.symbol}
                      {row.name ? ` — ${row.name}` : ""}
                      {row.openLotCount === 0 ? " (no lots yet)" : ""}
                    </li>
                  ))}
                </ul>
              ) : null}
            </label>
            <label>
              Account
              <select
                aria-label="Add lot account"
                value={addLotAccountId}
                onChange={(e) => setAddLotAccountId(e.target.value)}
                disabled={busy || writesBlocked}
              >
                <option value="">Choose account</option>
                {accounts.map((a) => (
                  <option key={a.accountId} value={a.accountId}>
                    {a.name}
                  </option>
                ))}
              </select>
            </label>
            <label>
              Opened on
              <input
                aria-label="Add lot opened on"
                type="date"
                value={addLotOpenedOn}
                onChange={(e) => setAddLotOpenedOn(e.target.value)}
                disabled={busy || writesBlocked}
              />
            </label>
            <label>
              Quantity
              <input
                aria-label="Add lot quantity"
                value={addLotQty}
                onChange={(e) => setAddLotQty(e.target.value)}
                disabled={busy || writesBlocked}
              />
            </label>
            <label>
              Unit original $
              <input
                aria-label="Add lot unit original cost"
                value={addLotCost}
                onChange={(e) => setAddLotCost(e.target.value)}
                disabled={busy || writesBlocked}
                placeholder="29.46"
              />
            </label>
            <label>
              <span>
                <input
                  type="checkbox"
                  aria-label="Tax cost different"
                  checked={addLotTaxDifferent}
                  onChange={(e) => setAddLotTaxDifferent(e.target.checked)}
                  disabled={busy || writesBlocked}
                />{" "}
                Unit tax $ different
              </span>
              <input
                aria-label="Add lot unit tax cost"
                value={addLotTaxDifferent ? addLotTaxCost : addLotCost}
                onChange={(e) => setAddLotTaxCost(e.target.value)}
                disabled={busy || writesBlocked || !addLotTaxDifferent}
              />
            </label>
            {(() => {
              const qty = Number(addLotQty);
              const unit = Number(addLotCost);
              if (
                !Number.isFinite(qty) ||
                qty <= 0 ||
                !Number.isFinite(unit) ||
                unit < 0 ||
                !addLotQty.trim() ||
                !addLotCost.trim()
              ) {
                return (
                  <p role="status" aria-label="Add lot total confirmation">
                    Enter qty and unit original $ — lot total = qty × unit.
                  </p>
                );
              }
              const totalCents = lotTotalFromUnitCents(qty, 0, dollarsToMinor(addLotCost));
              const totalLabel = (totalCents / 100).toLocaleString("en-US", {
                minimumFractionDigits: 2,
                maximumFractionDigits: 2,
              });
              const unitLabel = formatMoneyInput(addLotCost);
              return (
                <p role="status" aria-label="Add lot total confirmation">
                  Confirm: {formatScaled(qty, 0)} × ${unitLabel} = ${totalLabel} lot
                  original (saved as performance basis total).
                  {addLotTaxDifferent && addLotTaxCost.trim()
                    ? (() => {
                        const taxUnit = Number(addLotTaxCost);
                        if (!Number.isFinite(taxUnit) || taxUnit < 0) return "";
                        const taxTotal = lotTotalFromUnitCents(
                          qty,
                          0,
                          dollarsToMinor(addLotTaxCost),
                        );
                        return ` Tax: ${formatScaled(qty, 0)} × $${formatMoneyInput(addLotTaxCost)} = $${(taxTotal / 100).toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 })}.`;
                      })()
                    : ""}
                </p>
              );
            })()}
            <label>
              Origin
              <select
                aria-label="Add lot origin"
                value={addLotOrigin}
                onChange={(e) => setAddLotOrigin(e.target.value)}
                disabled={busy || writesBlocked}
              >
                {LOT_ORIGINS.map((o) => (
                  <option key={o} value={o}>
                    {o}
                  </option>
                ))}
              </select>
            </label>
          </div>
          {securities.length === 0 ? (
            <p role="status">
              No securities yet. Use Add Position (symbol + distribution URL), then return
              here to open a lot. Researched names with zero lots are included.
            </p>
          ) : addLotQuery.trim() && !addLotSecurityId && filteredAddLotSecurities.length === 0 ? (
            <p role="status">
              No matching researched symbol. Cannot create a new ticker here — research it
              on Add Position first.
            </p>
          ) : null}
        </section>
      ) : null}

      {screen === "import" ? (
        <section aria-label="Import">
          <h2>Import</h2>
          <p>
            Stage a Fidelity or Schwab CSV, review exceptions, then approve and post. Unknown
            amounts stay exceptions.
          </p>
          <p>Pending: {pendingBatchId ?? "none"} ({pendingBatchStatus}).</p>
          <section aria-label="Exceptions">
            <h3>Exceptions</h3>
            <ExceptionList exceptions={exceptions} onOpenLog={() => void openExceptionLog()} />
          </section>
          {captureProcess ? (
            <section className="capture-process" aria-label="Capture process">
              <h3>{captureProcess.title}</h3>
              <p role="status" aria-live="polite">
                {captureProcess.running
                  ? `${captureProcess.detail} (${captureProcess.current} of ${captureProcess.total})`
                  : captureProcess.detail}
              </p>
              {captureProcess.running && captureProcess.total > 0 ? (
                <progress
                  max={captureProcess.total}
                  value={Math.min(captureProcess.current, captureProcess.total)}
                />
              ) : null}
              {captureProcess.lines.length > 0 ? (
                <pre aria-label="Capture process lines">
                  {captureProcess.lines.slice(-8).join("\n")}
                </pre>
              ) : null}
              <p>
                Posted {captureProcess.posted}, skipped {captureProcess.skipped} duplicate
                {captureProcess.skipped === 1 ? "" : "s"}, {captureProcess.errors} error
                {captureProcess.errors === 1 ? "" : "s"}.
              </p>
              <div className="buttons">
                <button
                  type="button"
                  aria-label="Open process log"
                  onClick={() => void openExceptionLog()}
                >
                  Open process log
                </button>
                <button
                  type="button"
                  aria-label="Dismiss capture process"
                  disabled={captureProcess.running}
                  onClick={() => setCaptureProcess(null)}
                >
                  Dismiss
                </button>
              </div>
            </section>
          ) : null}
          <div className="buttons">
            <label>
              Fidelity or Schwab CSV
              <input
                type="file"
                accept=".csv,text/csv"
                aria-label="Import Fidelity or Schwab CSV"
                disabled={busy || writesBlocked}
                onChange={(e) => {
                  const file = e.target.files?.[0];
                  if (file) {
                    void importBrokerCsv(file);
                  }
                  e.target.value = "";
                }}
              />
            </label>
            <button
              type="button"
              aria-label="Validate import"
              disabled={busy || writesBlocked || !pendingBatchId}
              onClick={() => void runPendingImport("ImportValidate")}
            >
              Validate import
            </button>
            <button
              type="button"
              aria-label="Approve import"
              disabled={busy || writesBlocked || !pendingBatchId}
              onClick={() => void runPendingImport("ImportApprove")}
            >
              Approve import
            </button>
            <button
              type="button"
              aria-label="Post import"
              disabled={busy || writesBlocked || !pendingBatchId}
              onClick={() => void runPendingImport("ImportPost")}
            >
              Post import
            </button>
          </div>
          <section aria-label="Manual dividend">
            <h3>Enter missing dividend</h3>
            <p>
              Cash positions (SPAXX, FDRXX, SWVXX) and Account 9. Qty is 1. Description is
              Dividend. Amount is the cash total, not per share.
            </p>
            <div className="table-wrap">
              <table aria-label="Manual dividend rows">
                <thead>
                  <tr>
                    <th scope="col">Account</th>
                    <th scope="col">Ticker</th>
                    <th scope="col">Date</th>
                    <th scope="col">Qty</th>
                    <th scope="col">Description</th>
                    <th scope="col">Amount $</th>
                  </tr>
                </thead>
                <tbody>
                  {manualDividendRows.map((row, index) => (
                    <tr key={row.id}>
                      <td>
                        <select
                          aria-label={`Manual dividend account ${index + 1}`}
                          value={row.accountId}
                          onChange={(e) =>
                            updateManualDividendRow(row.id, { accountId: e.target.value })
                          }
                          disabled={busy || writesBlocked}
                        >
                          <option value="">Account</option>
                          {accounts.map((a) => (
                            <option key={a.accountId} value={a.accountId}>
                              {a.name}
                            </option>
                          ))}
                        </select>
                      </td>
                      <td>
                        <input
                          aria-label={`Manual dividend ticker ${index + 1}`}
                          value={row.ticker}
                          onChange={(e) =>
                            updateManualDividendRow(row.id, { ticker: e.target.value })
                          }
                          disabled={busy || writesBlocked}
                          list="manual-dividend-tickers"
                          autoComplete="off"
                        />
                      </td>
                      <td>
                        <input
                          aria-label={`Manual dividend date ${index + 1}`}
                          type="date"
                          value={row.occurredOn}
                          onChange={(e) =>
                            updateManualDividendRow(row.id, { occurredOn: e.target.value })
                          }
                          disabled={busy || writesBlocked}
                        />
                      </td>
                      <td className="numeric">1</td>
                      <td>Dividend</td>
                      <td>
                        <input
                          aria-label={`Manual dividend amount ${index + 1}`}
                          inputMode="decimal"
                          value={row.amount}
                          onChange={(e) =>
                            updateManualDividendRow(row.id, { amount: e.target.value })
                          }
                          disabled={busy || writesBlocked}
                          placeholder="21.27"
                        />
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            <datalist id="manual-dividend-tickers">
              {securities.map((s) => (
                <option key={s.securityId} value={s.symbol} />
              ))}
            </datalist>
            <div className="buttons">
              <button
                type="button"
                aria-label="Add manual dividend row"
                disabled={busy || writesBlocked}
                onClick={() =>
                  setManualDividendRows((rows) => [...rows, blankManualDividendRow()])
                }
              >
                Add row
              </button>
              <button
                type="button"
                aria-label="Post manual dividends"
                disabled={busy || writesBlocked}
                onClick={() => void postManualDividends()}
              >
                Post dividends
              </button>
            </div>
          </section>
        </section>
      ) : null}

      {screen === "collectors" ? (
        <section aria-label="Collectors">
          <h2>Collectors</h2>
          <p>
            Income names only (DIV-1, CASH, Weekly/Monthly/Quarterly). Non-payers
            are excluded. Force refresh runs the issuer declaration collector;
            Yahoo last price is separate and does not fill this page. Yahoo is
            never a declaration source.
          </p>
          <div className="buttons">
            <button
              type="button"
              aria-label="Refresh collector fleet"
              disabled={busy}
              onClick={() => void refreshCollectors(asOfDate)}
            >
              {busy && !collectorRunProgress?.running ? "Working…" : "Reload fleet"}
            </button>
            <button
              type="button"
              aria-label="Run enabled collectors"
              disabled={busy || writesBlocked}
              onClick={() => void runEnabledCollectors()}
            >
              {collectorRunProgress?.running
                ? `Running ${formatCount(collectorRunProgress.current)} of ${formatCount(collectorRunProgress.total)}${
                    collectorRunProgress.symbol
                      ? `: ${collectorRunProgress.symbol}`
                      : ""
                  }…`
                : "Run enabled collectors"}
            </button>
            <button
              type="button"
              aria-label="Run misses only"
              disabled={busy || writesBlocked}
              onClick={() => void runMissesOnlyCollectors()}
            >
              Run misses only
            </button>
            <button
              type="button"
              aria-label="Fill research gaps"
              disabled={busy || writesBlocked || Boolean(researchActivity?.running)}
              onClick={() => void fillResearchGaps()}
            >
              {researchActivity?.running && screen === "collectors"
                ? "Filling research gaps…"
                : "Fill research gaps"}
            </button>
            <button
              type="button"
              aria-label="Apply issuer sources from provider"
              disabled={busy || writesBlocked}
              onClick={() => void applyIssuerSources()}
            >
              Apply issuer sources from provider
            </button>
          </div>
          {researchActivity?.running && screen === "collectors" ? (
            <section
              className="process-a-research-progress"
              aria-label="Research progress"
              aria-busy="true"
            >
              <p role="status" aria-live="polite">
                {researchActivity.label}
              </p>
              {researchActivity.total > 0 ? (
                <progress
                  max={researchActivity.total}
                  value={Math.min(researchActivity.step, researchActivity.total)}
                />
              ) : (
                <div className="process-a-research-spinner" aria-hidden="true" />
              )}
            </section>
          ) : null}
          {researchActivity?.resultLine &&
          !researchActivity.running &&
          screen === "collectors" ? (
            <p role="status" aria-label="Research result">
              {researchActivity.resultLine}
            </p>
          ) : null}
          {writesBlocked ? (
            <p role="status">
              Writes are blocked on this device, so Apply and Run enabled are
              disabled. Reload fleet still works.
            </p>
          ) : null}
          {collectorAction ? (
            <p aria-label="Collector action status" role="status" aria-live="polite">
              {collectorAction}
            </p>
          ) : (
            <p aria-label="Collector action status" role="status">
              Reload fleet refreshes this list from the database. Run enabled
              collectors hits issuer sites one symbol at a time with live
              progress. Run misses only skips symbols already ok today
              (declarations only). Fill research gaps runs Complete research
              for enabled names missing provider, underlying, frequency, or
              ROC observation. Apply fills empty templates from provider (0
              updated means already assigned).
            </p>
          )}
          {collectorRunProgress ? (
            <section
              aria-label="Collector run progress"
              aria-busy={collectorRunProgress.running}
            >
              <h3>
                {collectorRunProgress.running ? "Collecting…" : "Last run"}
              </h3>
              <p>
                Progress {formatCount(collectorRunProgress.current)} /{" "}
                {formatCount(collectorRunProgress.total)}
                {collectorRunProgress.symbol
                  ? ` — ${collectorRunProgress.symbol}`
                  : ""}
                . Ok {formatCount(collectorRunProgress.ok)}. Miss{" "}
                {formatCount(collectorRunProgress.miss)}.
              </p>
              <progress
                max={Math.max(collectorRunProgress.total, 1)}
                value={collectorRunProgress.current}
              />
              <div className="table-wrap">
                <table aria-label="Collector run log">
                  <thead>
                    <tr>
                      <th scope="col">Result</th>
                    </tr>
                  </thead>
                  <tbody>
                    {collectorRunProgress.lines.length === 0 ? (
                      <tr>
                        <td>
                          {collectorRunProgress.running
                            ? "Waiting for first symbol…"
                            : "No lines."}
                        </td>
                      </tr>
                    ) : (
                      [...collectorRunProgress.lines].reverse().map((line, idx) => (
                        <tr key={`${idx}-${line}`}>
                          <td>{line}</td>
                        </tr>
                      ))
                    )}
                  </tbody>
                </table>
              </div>
            </section>
          ) : null}
          {collectorStats ? (
            <p aria-label="Collector statistics">
              Assigned {formatCount(collectorStats.assigned)}. Enabled{" "}
              {formatCount(collectorStats.enabled)}. Ran today{" "}
              {formatCount(collectorStats.ranToday)}. Miss today{" "}
              {formatCount(collectorStats.missToday)}. Unchanged today{" "}
              {formatCount(collectorStats.unchangedToday)}. Price current{" "}
              {formatCount(collectorStats.priceCurrent)} / stale{" "}
              {formatCount(collectorStats.priceStale)}. Open exceptions{" "}
              {formatCount(collectorStats.openExceptions)}.
            </p>
          ) : null}
          <div className="table-wrap">
            <table aria-label="Collector fleet">
              <thead>
                <tr>
                  <th scope="col">Symbol</th>
                  <th scope="col">Cadence</th>
                  <th scope="col">Provider</th>
                  <th scope="col">Source</th>
                  <th scope="col">Enabled</th>
                  <th scope="col">Last declaration run</th>
                  <th scope="col">Actions</th>
                </tr>
              </thead>
              <tbody>
                {collectorItems.map((row) => (
                  <tr key={row.securityId}>
                    <td>
                      <button
                        type="button"
                        aria-label={`Open ${row.symbol} collector`}
                        onClick={() => {
                          setCollectorSymbol(row.symbol);
                          void refreshCollectors(asOfDate, row.symbol);
                        }}
                      >
                        {row.symbol}
                      </button>
                    </td>
                    <td>{row.paymentFrequency || row.divType || "—"}</td>
                    <td>{row.provider || "—"}</td>
                    <td>{row.declarationSource || "unassigned"}</td>
                    <td>{row.collectorEnabled ? "yes" : "no"}</td>
                    <td>
                      {row.lastRunAt || "never"}
                      {row.lastRunOk === true
                        ? " ok"
                        : row.lastRunOk === false
                          ? " miss"
                          : ""}
                      {row.lastRunMessage ? (
                        <span className="collector-run-message">
                          {" "}
                          — {row.lastRunMessage}
                        </span>
                      ) : null}
                    </td>
                    <td>
                      <button
                        type="button"
                        aria-label={
                          row.collectorEnabled
                            ? `Disable collector for ${row.symbol}`
                            : `Enable collector for ${row.symbol}`
                        }
                        disabled={busy || writesBlocked || !row.declarationSource}
                        onClick={() =>
                          void toggleCollectorEnabled(row, !row.collectorEnabled)
                        }
                      >
                        {row.collectorEnabled ? "Disable" : "Enable"}
                      </button>{" "}
                      <button
                        type="button"
                        aria-label="Force refresh this symbol"
                        disabled={busy || writesBlocked}
                        onClick={() => void forceCollectorRefresh(row)}
                      >
                        Force refresh
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <h3>DIV-1 compliance</h3>
          <p>
            Numeric summary per enabled DIV-1 collector: upcoming pay dates,
            stored declaration history, and the newest paid declaration.
          </p>
          <div className="table-wrap">
            <table aria-label="DIV-1 compliance summary">
              <thead>
                <tr>
                  <th scope="col">Symbol</th>
                  <th scope="col">Future dates qty</th>
                  <th scope="col">Prior decls qty</th>
                  <th scope="col">Current amount</th>
                  <th scope="col">Current date</th>
                  <th scope="col">Required paid</th>
                  <th scope="col">OK</th>
                </tr>
              </thead>
              <tbody>
                {div1Compliance.length === 0 ? (
                  <tr>
                    <td colSpan={7}>No DIV-1 compliance rows yet.</td>
                  </tr>
                ) : (
                  div1Compliance.map((row) => (
                    <tr key={row.securityId}>
                      <td>{row.symbol}</td>
                      <td className="numeric">{formatCount(row.futurePayDatesQty)}</td>
                      <td className="numeric">{formatCount(row.priorDeclarationsQty)}</td>
                      <td className="numeric">
                        {row.currentDeclarationAmountMinor == null ||
                        row.currentDeclarationAmountScale == null
                          ? "—"
                          : `$${formatScaled(
                              row.currentDeclarationAmountMinor,
                              row.currentDeclarationAmountScale,
                            )}`}
                      </td>
                      <td>{row.currentDeclarationDate || "—"}</td>
                      <td className="numeric">{formatCount(row.requiredPaid)}</td>
                      <td>
                        {row.lastRunOk === true
                          ? "yes"
                          : row.lastRunOk === false
                            ? "no"
                            : "—"}
                      </td>
                    </tr>
                  ))
                )}
              </tbody>
            </table>
          </div>
          {collectorSymbol ? (
            <section aria-label="Collector symbol page">
              <h3>{collectorSymbol}</h3>
              <div className="buttons">
                <button
                  type="button"
                  aria-label="Open in Position Details"
                  onClick={() => openPositionHub(collectorSymbol)}
                >
                  Open in Position Details
                </button>
              </div>
              <p>
                Collectors is ops only — last retrieve runs and payload. Position
                data (declarations, plan pays, received totals) lives on Position
                Details.
              </p>
              {collectorPlan ? (
                <p aria-label="Collector plan">
                  Plan check:{" "}
                  {collectorPlan.known
                    ? `${formatUsd(collectorPlan.perShareMinor, collectorPlan.scale)}${
                        collectorPlan.reason ? ` — ${collectorPlan.reason}` : ""
                      }`
                    : "unknown (not $0)"}
                </p>
              ) : null}
              <div className="table-wrap">
                <table aria-label="Retrieve runs">
                  <thead>
                    <tr>
                      <th scope="col">When</th>
                      <th scope="col">Kind</th>
                      <th scope="col">Ok</th>
                      <th scope="col">Recorded</th>
                      <th scope="col">Skipped</th>
                      <th scope="col">Message</th>
                    </tr>
                  </thead>
                  <tbody>
                    {collectorRuns
                      .filter((run) => run.kind !== "price")
                      .concat(collectorRuns.filter((run) => run.kind === "price").slice(0, 3))
                      .map((run) => (
                      <tr key={run.runId}>
                        <td>{run.requestedAt}</td>
                        <td>{run.kind}</td>
                        <td>{run.ok ? "ok" : "miss"}</td>
                        <td>{formatCount(run.recorded)}</td>
                        <td>{formatCount(run.skipped)}</td>
                        <td>
                          {run.code ? `${run.code}: ` : ""}
                          {run.message || "—"}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
              {collectorPayload ? (
                <div className="table-wrap">
                  <table aria-label="Collector retrieve payload">
                    <thead>
                      <tr>
                        <th scope="col">Field</th>
                        <th scope="col">Value</th>
                      </tr>
                    </thead>
                    <tbody>
                      {Object.entries(collectorPayload).map(([key, value]) => (
                        <tr key={key}>
                          <td>{key}</td>
                          <td>
                            <code>
                              {typeof value === "string"
                                ? value
                                : JSON.stringify(value)}
                            </code>
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              ) : (
                <p>
                  No declaration retrieve payload yet. Daily open and Force
                  refresh write declaration runs here; last-price-only runs are
                  not shown as the payload.
                </p>
              )}
            </section>
          ) : null}
        </section>
      ) : null}

      {screen === "settings" ? (
        <section className="actions" aria-label="Settings">
          <h2>Settings</h2>
          {health === null ? (
            <p>Checking HealthGet…</p>
          ) : (
            <dl className="health">
              <dt>ok</dt>
              <dd>{health.ok ? "true" : "false"}</dd>
              <dt>status</dt>
              <dd>{health.status}</dd>
              <dt>contract</dt>
              <dd>{health.contractVersion}</dd>
            </dl>
          )}
          <h3>Retrieval templates</h3>
          <p>
            Adapter, source URL, lookback, and schedule live here. Inception is
            optional — rare exception for names too new for 12 paid points.
            Collectors retrieve first; if 12+ paid decls land, inception is N/A.
            Under 12, inception (when set) confirms the short history is complete.
          </p>
          <div className="table-wrap">
            <table aria-label="Retrieval templates">
              <thead>
                <tr>
                  <th scope="col">Symbol</th>
                  <th scope="col">Adapter</th>
                  <th scope="col">Source URL</th>
                  <th scope="col">Schedule</th>
                  <th scope="col">Inception</th>
                  <th scope="col">Lookback</th>
                  <th scope="col">Actions</th>
                </tr>
              </thead>
              <tbody>
                {collectorItems.length === 0 ? (
                  <tr>
                    <td colSpan={7}>
                      No paying symbols loaded yet. Open Collectors once, or wait
                      for this list to refresh.
                    </td>
                  </tr>
                ) : (
                  collectorItems.map((row) => {
                    const draft = settingsTemplateDrafts[row.securityId] ?? {
                      declarationSource: row.declarationSource || "",
                      sourceUrl: row.sourceUrl ?? "",
                      calendarPolicy: row.calendarPolicy ?? "",
                      lookbackCount: String(DECLARATION_LOOKBACK_TARGET),
                      inceptionOn: row.inceptionOn ?? "",
                    };
                    const patch = (
                      patch: Partial<typeof draft>,
                    ) => {
                      setSettingsTemplateDrafts((prev) => ({
                        ...prev,
                        [row.securityId]: { ...draft, ...patch },
                      }));
                    };
                    return (
                      <tr key={row.securityId}>
                        <td>{row.symbol}</td>
                        <td>
                          <input
                            aria-label={`Adapter for ${row.symbol}`}
                            value={draft.declarationSource}
                            onChange={(e) =>
                              patch({ declarationSource: e.target.value })
                            }
                            disabled={busy || writesBlocked}
                          />
                        </td>
                        <td>
                          <input
                            aria-label={`Source URL for ${row.symbol}`}
                            value={draft.sourceUrl}
                            onChange={(e) =>
                              patch({ sourceUrl: e.target.value })
                            }
                            disabled={busy || writesBlocked}
                          />
                        </td>
                        <td>
                          <select
                            aria-label={`Schedule for ${row.symbol}`}
                            value={draft.calendarPolicy}
                            onChange={(e) =>
                              patch({ calendarPolicy: e.target.value })
                            }
                            disabled={busy || writesBlocked}
                          >
                            <option value="">Choose policy</option>
                            <option value="issuer_calendar">
                              Issuer published dates
                            </option>
                            <option value="derived_walk">
                              Cadence from last pay
                            </option>
                            <option value="none">Does not pay</option>
                          </select>
                        </td>
                        <td>
                          <input
                            aria-label={`Inception date for ${row.symbol}`}
                            placeholder="optional YYYY-MM-DD"
                            value={draft.inceptionOn}
                            onChange={(e) =>
                              patch({ inceptionOn: e.target.value })
                            }
                            disabled={busy || writesBlocked}
                          />
                        </td>
                        <td>
                          <input
                            aria-label={`Lookback for ${row.symbol}`}
                            value={draft.lookbackCount}
                            onChange={(e) =>
                              patch({ lookbackCount: e.target.value })
                            }
                            disabled={busy || writesBlocked}
                          />
                        </td>
                        <td>
                          <button
                            type="button"
                            aria-label={`Save template for ${row.symbol}`}
                            disabled={busy || writesBlocked}
                            onClick={() => void saveSettingsTemplate(row)}
                          >
                            Save
                          </button>
                        </td>
                      </tr>
                    );
                  })
                )}
              </tbody>
            </table>
          </div>
          <h3>HandoffStatusGet</h3>
          {handoffError ? <p className="blocked">{handoffError}</p> : null}
          {handoff ? (
            <dl className="health">
              <dt>decision</dt>
              <dd>{handoff.decision}</dd>
              <dt>writes</dt>
              <dd className={writesBlocked ? "blocked" : undefined}>
                {handoff.writesAllowed ? "allowed" : "blocked"}
              </dd>
              <dt>message</dt>
              <dd>{handoff.message}</dd>
            </dl>
          ) : null}
          <label>
            Device name
            <input
              value={deviceName}
              onChange={(e) => setDeviceName(e.target.value)}
              disabled={busy || writesBlocked}
            />
          </label>
          <div className="buttons">
            <button
              type="button"
              aria-label="Save device name"
              disabled={busy || writesBlocked}
              onClick={() => runCommand("ConfigSet", { deviceName })}
            >
              Save device name
            </button>
            <button
              type="button"
              aria-label="Create snapshot"
              disabled={busy}
              onClick={() => runCommand("SnapshotCreate")}
            >
              Create snapshot
            </button>
            <button
              type="button"
              aria-label="Restore published"
              disabled={busy}
              onClick={() => runCommand("SnapshotRestore")}
            >
              Restore published
            </button>
            <button
              type="button"
              aria-label="Acknowledge review"
              disabled={busy || handoff?.decision !== "block_until_restore"}
              onClick={() => runCommand("HandoffResolve", { action: "review" })}
            >
              Acknowledge review
            </button>
            <button
              type="button"
              aria-label="Exit"
              onClick={() => {
                void invoke("app_exit");
              }}
            >
              Exit
            </button>
            <button
              type="button"
              aria-label="Check for updates"
              disabled={busy}
              onClick={() => void checkForUpdates()}
            >
              Check for updates
            </button>
          </div>
          <p>Update check: {updateStatus}. Missing releases fail closed and do not post.</p>
        </section>
      ) : null}
    </main>
  );
}
