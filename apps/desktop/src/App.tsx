import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import {
  FINANCE_CLIENT_CONTRACT_VERSION,
  type AccountListItem,
  type CalculatorGet,
  type DeclarationHistoryGet,
  type CurrentPriceGet,
  type DashboardBurndownGet,
  type DividendGet,
  type DividendPerformanceGet,
  type DividendPerformanceRange,
  type ExceptionRecord,
  type WorkTicketRecord,
  type WorkTicketList,
  type HandoffStatus,
  type HoldingsGet,
  type DataSummaryGet,
  type CoreFunctionsGet,
  type IncomePlanWeekGet,
  type IncomePlanGridGet,
  type IncomePlanExportGet,
  type ImportBatchRecord,
  type ImportPendingGet,
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
  type CarRocPlanGet,
  type TaxPlanningGet,
  type CashManagementWeekGet,
  type CashManagementRemindersGet,
  type CashManagementMonthGet,
  type WeekAheadGet,
  type CashRegisterGet,
  type CashElementListGet,
  type CashElementRecord,
  type CashYtdGet,
  type CashCoverageGet,
  type MagiProjection,
  type AccountValueHomeGet,
  type DividendPlanHomeGet,
  type HomeOpenGet,
  type DataSnapshotExport,
} from "@finos/app-contracts";
import { invoke } from "@tauri-apps/api/core";
import { check } from "@tauri-apps/plugin-updater";
import { LocalTauriFinanceClient } from "./financeClient";
import { TrendsChartsPanel } from "./features/graphing/TrendsCharts";
import { DEFAULT_GRAPH_PERIOD, type GraphPeriod } from "./features/graphing/graphPeriod";
import { DeclarationPaymentsChart } from "./features/graphing/DeclarationPaymentsChart";
import { TrendsCapturePanel, type TrendsWeekCapture } from "./features/graphing/TrendsCapture";
import { CashManagementPanel } from "./CashManagement";
import { WeekAheadPanel } from "./features/cash/WeekAhead";
import { CashRegisterPanel } from "./features/cash/CashRegister";
import { CashElementsCatalog } from "./features/cash/CashElementsCatalog";
import {
  fetchCashNav,
  fetchElementList,
  magiQualifyingType,
} from "./features/cash/fetchCashNav";
import { scheduleIdleWarm } from "./features/shared/idleWarm";
import {
  incomeGridMemoryMatches,
  runBackgroundReads,
} from "./features/shared/backgroundReads";
import { CashYtdPanel } from "./features/cash/CashYtd";
import { CashCoveragePanel, type CoveragePeriod } from "./features/cash/CashCoverage";
import { ExternalRegister } from "./features/cash/ExternalRegister";
import { HomeAccountCharts } from "./features/graphing/HomeAccountCharts";
import { AccountCashFlow } from "./features/cash/AccountCashFlow";
import { HomeDividendPlan } from "./features/home/HomeDividendPlan";
import { PlanHorizonPrompt } from "./features/income-plan/PlanHorizonPrompt";
import { IncomePlanScreen } from "./features/income-plan/IncomePlanScreen";
import {
  CollectorEstablishScreen,
  CollectorsScreen,
  collectorCommandBody,
  collectorIsCash,
  collectorItemPays,
  collectorMissIsTimeout,
  collectorNeedsOwnerUrl,
  collectorRetrieveNeedsRun,
  isDeclarationWeekdayToday,
  type CollectorSetItem,
  type CollectorStats,
  type Div1ComplianceRow,
  type RetrieveRunRow,
} from "./features/collectors";
import {
  INCOME_TX_PERIOD_OPTIONS,
  incomeTxRange,
  inIncomeTxRange,
  localIsoDate,
  type IncomeTxPeriod,
} from "./incomeTxPeriod";
import { ShoppingCartScreen } from "./features/shopping-cart";
import {
  AccountSelect,
  LotCostTable,
  ResearchedSymbolCombobox,
  type LotSortMode,
} from "./features/shared/pickers";
import { PageActivityBar } from "./features/shared/PageActivityBar";
import { openImportWizardWindow } from "./openImportWizard";
import {
  CalculatorReturnSheet,
  type CollectorCompletionProof,
  DashboardBurndownPanel,
  ExceptionList,
  WorkTicketQueue,
  HoldingsPanel,
  INCOME_PLAN_DEFAULT_ACCOUNTS,
  PositionDetailsTable,
  PositionMasterTable,
  SymbolLotsTable,
  formatCount,
  formatPerShare,
  formatScaled,
  formatUsd,
  formatPercentScaled,
  addUtcDays,
  saturdayOfWeek,
  formatMenuWeek,
} from "@finos/ui-components";
import "./App.css";

const RISK_TIERS = ["Foundation", "Core", "Risk On"];

/** Preview/print tables get grid + color even if the host printHtml is stale. */
function styleIncomePrintHtml(html: string): string {
  const css = `<style data-finos-print-skin>
table{width:100%;border-collapse:collapse;font-size:11px;line-height:1.25}
table th,table td{border:1px solid #c5d0dc;padding:3px 6px}
table th{background:#1b365d;color:#fff;font-weight:600;text-align:left}
table td:not(:first-child){text-align:right}
table tr:nth-child(even) td{background:#f4f7fb}
table tr.ip-grp th{background:#e7eef6;color:#1b365d}
table tr.ip-plan-total td{background:#22c55e;color:#111;font-weight:700}
table tr.ip-act-total td{background:#86efac;color:#111;font-weight:700}
table tr.ip-planrow td{background:#eef8f1}
table tr.ip-actrow td{background:#f7fafc}
table tr.ip-diff-total td,table tr.ip-delta td{background:#eef2f7}
table tr.ip-current td{background:#2d5a8a !important;color:#fff;font-weight:700}
table tr.ip-grand td{background:#1f6b4a !important;color:#fff;font-weight:700}
</style>`;
  if (html.includes("data-finos-print-skin")) return html;
  if (html.includes("</head>")) return html.replace("</head>", `${css}</head>`);
  return `${css}${html}`;
}

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
  coveredCall: false,
  leveraged: false,
});

type RefreshProgress = {
  current: number;
  total: number;
  symbol: string;
};

function lastPriceRefreshLabel(lastPriceProgress: RefreshProgress | null): string {
  if (!lastPriceProgress || lastPriceProgress.total <= 0) {
    return "…";
  }
  const count = `${formatCount(lastPriceProgress.current)} of ${formatCount(lastPriceProgress.total)}`;
  return lastPriceProgress.symbol ? `${count} ${lastPriceProgress.symbol}` : count;
}

function declarationRefreshLabel(
  declarationProgress: RefreshProgress | null,
): string {
  if (!declarationProgress || declarationProgress.total <= 0) {
    return "…";
  }
  const count = `${formatCount(declarationProgress.current)} of ${formatCount(declarationProgress.total)}`;
  return declarationProgress.symbol ? `${count} ${declarationProgress.symbol}` : count;
}

function isNewOrChangedDeclaration(
  period: string | undefined,
  amount: number | null | undefined,
  scale: number | undefined,
  stored: Array<{
    paymentPeriod?: string;
    amountPerShareMinor?: number | null;
    amountScale?: number;
  }>,
): boolean {
  if (!period?.trim() || amount == null || amount <= 0) return false;
  const row = stored.find((d) => d.paymentPeriod?.trim() === period.trim());
  if (!row) return true;
  return (
    row.amountPerShareMinor !== amount || (row.amountScale ?? 0) !== (scale ?? 0)
  );
}

function mergeLookthrough(raw?: LookthroughResearch | null): LookthroughResearch {
  return {
    ...emptyLookthrough(),
    ...raw,
    topHoldings: raw?.topHoldings ?? [],
    sectorWeights: raw?.sectorWeights ?? [],
  };
}

function strategyCharacteristics(
  lt: LookthroughResearch,
  underlying?: string,
): string {
  const bits: string[] = [];
  const theme = lt.themeStrategy ?? "";
  if (
    lt.coveredCall ||
    /covered[-\s]?call/i.test(theme) ||
    /covered[-\s]?call/i.test(lt.taxCharacter ?? "")
  ) {
    bits.push("covered-call");
  }
  if (lt.leveraged || /leveraged/i.test(theme)) {
    bits.push("leveraged");
  }
  const under = (underlying ?? "").trim();
  if (under) {
    bits.push(`look-through ${under}`);
  } else if ((lt.primaryRiskDriver ?? "").trim()) {
    bits.push(`look-through ${lt.primaryRiskDriver}`);
  }
  return bits.join(" · ");
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
type CmDesk = "elements" | "cashflow" | "weekly" | "car" | "coverage" | "external";
type Screen =
  | "home"
  | "income-plan"
  | "calculator"
  | "dashboard"
  | "trends"
  | "cash-management"
  | "shopping-cart"
  | "holdings"
  | "import"
  | "settings"
  | "new-investment"
  | "add-lot"
  | "position-details"
  | "collectors"
  | "tickets"
  | "collector-establish"
  | "components";

const SCREENS: readonly Screen[] = [
  "home",
  "income-plan",
  "calculator",
  "dashboard",
  "trends",
  "cash-management",
  "shopping-cart",
  "holdings",
  "import",
  "settings",
  "new-investment",
  "add-lot",
  "position-details",
  "collectors",
  "tickets",
  "collector-establish",
  "components",
];

function isScreen(id: string): id is Screen {
  return (SCREENS as readonly string[]).includes(id);
}

function paidRowsFromDeclarations(
  decls: Array<{
    paymentPeriod: string;
    amountPerShareMinor: number | null;
    amountScale: number;
  }>,
): CollectorCompletionProof["paid"] {
  return [...decls]
    .filter((d) => (d.amountPerShareMinor ?? 0) > 0)
    .sort((a, b) => b.paymentPeriod.localeCompare(a.paymentPeriod))
    .map((d) => ({
      payOn: d.paymentPeriod.slice(0, 10),
      amountPerShare: formatScaled(
        d.amountPerShareMinor ?? 0,
        d.amountScale ?? 2,
      ),
    }));
}

function proofsFromFleet(
  items: CollectorSetItem[],
  compliance: Div1ComplianceRow[],
  calc: CalculatorGet | null,
  history: DeclarationHistoryGet | null,
): CollectorCompletionProof[] {
  const ok = items.filter((r) => r.collectorEnabled && r.lastRunOk === true);
  return ok
    .map((row) => {
      const comp = compliance.find((c) => c.symbol === row.symbol);
      const calcRow = calc?.rows.find((c) => c.symbol === row.symbol);
      const hist = history?.rows.find((h) => h.symbol === row.symbol);
      const scale = calcRow?.rocScale;
      return {
        symbol: row.symbol,
        futureDates: comp?.futurePayDates ?? [],
        roc2024: "",
        roc2025: rocText(calcRow?.rocPct2025ActualMinor, scale),
        roc2026e: rocText(calcRow?.rocPct2026EstimateMinor, scale),
        roc2026a: rocText(calcRow?.rocPct2026ActualMinor, scale),
        weekEnds: history?.weekEnds ?? [],
        weekStarts: history?.weekStarts,
        cells: hist?.cells ?? [],
        paid: [],
      };
    })
    .sort((a, b) => a.symbol.localeCompare(b.symbol));
}

function parseHandoff(bodyJson?: string): HandoffStatus | null {
  if (!bodyJson) return null;
  try {
    return JSON.parse(bodyJson) as HandoffStatus;
  } catch {
    return null;
  }
}

function historyBounds(
  preset: string,
  asOf: string,
  startOn: string,
  endOn: string,
): { startOn: string; endOn: string } {
  switch (preset) {
    case "month":
      return { startOn: `${asOf.slice(0, 7)}-01`, endOn: asOf };
    case "90":
      return { startOn: addUtcDays(asOf, -90), endOn: asOf };
    case "ytd":
      return { startOn: `${asOf.slice(0, 4)}-01-01`, endOn: asOf };
    case "custom":
      return {
        startOn: startOn || addUtcDays(asOf, -60),
        endOn: endOn || asOf,
      };
    default:
      return { startOn: addUtcDays(asOf, -60), endOn: asOf };
  }
}

function satFriWeek(iso: string): { start: string; end: string } | null {
  const day = iso.slice(0, 10);
  if (!/^\d{4}-\d{2}-\d{2}$/.test(day)) return null;
  const [year, month, date] = day.split("-").map(Number);
  const cursor = new Date(year, month - 1, date);
  const sinceSat = (cursor.getDay() + 1) % 7;
  const start = new Date(year, month - 1, date - sinceSat);
  const end = new Date(start);
  end.setDate(start.getDate() + 6);
  const fmt = (value: Date) => {
    const mm = String(value.getMonth() + 1).padStart(2, "0");
    const dd = String(value.getDate()).padStart(2, "0");
    return `${value.getFullYear()}-${mm}-${dd}`;
  };
  return { start: fmt(start), end: fmt(end) };
}

function usdCents(minor: number, scale: number): number {
  if (!Number.isFinite(minor)) return 0;
  const places = Number.isFinite(scale) ? scale : 2;
  if (places === 2) return minor;
  if (places < 2) return minor * 10 ** (2 - places);
  return Math.round(minor / 10 ** (places - 2));
}

/** Same Plan $ total the Income Plan week screen prints. */
function incomePlanWeekPlanMinor(week: IncomePlanWeekGet | null | undefined): number | null {
  const rows = week?.positions ?? [];
  if (!rows.some((row) => row.planKnown)) return null;
  return rows.reduce(
    (sum, row) => sum + (row.planKnown ? (row.plannedMinor ?? 0) : 0),
    0,
  );
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
  const [screen, setScreen] = useState<Screen>("home");
  const [cmDesk, setCmDesk] = useState<CmDesk>("weekly");
  const screenRef = useRef(screen);
  screenRef.current = screen;
  const [openMenu, setOpenMenu] = useState<string | null>(null);
  const [health, setHealth] = useState<HealthView | null>(null);
  const [coreFunctions, setCoreFunctions] = useState<CoreFunctionsGet | null>(null);
  const [handoff, setHandoff] = useState<HandoffStatus | null>(null);
  const [handoffError, setHandoffError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [restarting, setRestarting] = useState(false);
  const [actionMessage, setActionMessage] = useState<string | null>(null);
  const [deviceName, setDeviceName] = useState("");
  const [asOfDate, setAsOfDate] = useState(() =>
    saturdayOfWeek(new Date().toISOString().slice(0, 10)),
  );
  const [incomeWeek, setIncomeWeek] = useState<IncomePlanWeekGet | null>(null);
  const [incomeWeekLoading, setIncomeWeekLoading] = useState(false);
  const [incomeWeekNav, setIncomeWeekNav] = useState<"prev" | "next" | null>(null);
  const prevIncomeAsOfRef = useRef<string | null>(null);
  const incomeLoadGen = useRef(0);
  const skipFullRefreshOnAsOfRef = useRef(false);
  const [incomeGrid, setIncomeGrid] = useState<IncomePlanGridGet | null>(null);
  const [incomePattern, setIncomePattern] = useState<"A" | "B">("B");
  const [incomeHistWeeks, setIncomeHistWeeks] = useState(6);
  const [incomeFutWeeks, setIncomeFutWeeks] = useState(6);
  const [burndown, setBurndown] = useState<DashboardBurndownGet | null>(null);
  const [trends, setTrends] = useState<TrendsGet | null>(null);
  const [trendsError, setTrendsError] = useState<string | null>(null);
  const [trendsCapture, setTrendsCapture] = useState<TrendsWeekCapture | null>(null);
  const trendsWeekAsOfRef = useRef("");
  const [cashWeek, setCashWeek] = useState<CashManagementWeekGet | null>(null);
  const [cashReminders, setCashReminders] =
    useState<CashManagementRemindersGet | null>(null);
  const [cashMonth, setCashMonth] = useState<CashManagementMonthGet | null>(null);
  const [carRocPlan, setCarRocPlan] = useState<CarRocPlanGet | null>(null);
  const [taxPlanning, setTaxPlanning] = useState<TaxPlanningGet | null>(null);
  const [weekAhead, setWeekAhead] = useState<WeekAheadGet | null>(null);
  const [weekAheadPending, setWeekAheadPending] = useState<string | null>(null);
  const [cashRegister, setCashRegister] = useState<CashRegisterGet | null>(null);
  const [cashElements, setCashElements] = useState<CashElementListGet | null>(null);
  const [registerBook, setRegisterBook] = useState("Income");
  const [registerPeriod, setRegisterPeriod] = useState("1M");
  const [registerView, setRegisterView] = useState<"calendar" | "trend">("calendar");
  const [elementEditorOpen, setElementEditorOpen] = useState(false);
  const [elementDirty, setElementDirty] = useState(false);
  const [externalDirty, setExternalDirty] = useState(false);
  const [externalReset, setExternalReset] = useState(0);
  const [menuWorking, setMenuWorking] = useState<string | null>(null);
  const [catalogBook, setCatalogBook] = useState("all");
  const [cashYtd, setCashYtd] = useState<CashYtdGet | null>(null);
  const [ytdView, setYtdView] = useState<"account" | "tax">("account");
  const [cashCoverage, setCashCoverage] = useState<CashCoverageGet | null>(null);
  const [coveragePeriod, setCoveragePeriod] = useState<CoveragePeriod>("week");
  const coveragePeriodRef = useRef<CoveragePeriod>("week");
  const [coverageBusy, setCoverageBusy] = useState(false);
  const [editorElement, setEditorElement] = useState<CashElementRecord | null>(null);
  const [editorOccurrenceId, setEditorOccurrenceId] = useState<string | null>(null);
  const registerBookRef = useRef("Income");
  const registerPeriodRef = useRef("1M");
  const registerFetchSeq = useRef(0);
  const catalogFetchSeq = useRef(0);
  const [registerBusy, setRegisterBusy] = useState(false);
  const ytdViewRef = useRef<"account" | "tax">("account");
  const loadRegisterMonth = useCallback(async (monthEnd: string) => {
    const monthStart = `${monthEnd.slice(0, 8)}01`;
    const now = new Date();
    const today = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}-${String(now.getDate()).padStart(2, "0")}`;
    const result = await client.executeQuery("CashRegisterGet", {
      asOfDate: today,
      account: registerBookRef.current,
      period: "1M",
      periodStart: monthStart,
      periodEnd: monthEnd,
    });
    if (!result.ok || !result.bodyJson) {
      return null;
    }
    return JSON.parse(result.bodyJson) as CashRegisterGet;
  }, []);
  const [cashDirty, setCashDirty] = useState(false);
  const [cartDirty, setCartDirty] = useState(false);
  const [holdingsLotSort, setHoldingsLotSort] = useState<LotSortMode>("lowest-cost");
  const pendingCartBuyRef = useRef<{ scenarioId: string } | null>(null);
  const [cashMagi, setCashMagi] = useState<MagiProjection | null>(null);
  const [accountValues, setAccountValues] = useState<AccountValueHomeGet | null>(null);
  const [dividendPlan, setDividendPlan] = useState<DividendPlanHomeGet | null>(null);
  const [perfRange, setPerfRange] = useState<DividendPerformanceRange>("ytd");
  const [dividendPerf, setDividendPerf] = useState<DividendPerformanceGet | null>(null);
  const [trendsDividendPerf, setTrendsDividendPerf] =
    useState<DividendPerformanceGet | null>(null);
  const perfRangeRef = useRef(perfRange);
  perfRangeRef.current = perfRange;
  const trendsGraphPeriodRef = useRef<GraphPeriod>(DEFAULT_GRAPH_PERIOD);
  const [trendsChartEpoch, setTrendsChartEpoch] = useState(0);
  const [holdings, setHoldings] = useState<HoldingsGet | null>(null);
  const [calculator, setCalculator] = useState<CalculatorGet | null>(null);
  const [declHistory, setDeclHistory] = useState<DeclarationHistoryGet | null>(null);
  const [histCadence, setHistCadence] = useState("all");
  const [histPeriod, setHistPeriod] = useState("60");
  const [histStartOn, setHistStartOn] = useState("");
  const [histEndOn, setHistEndOn] = useState("");
  const histPeriodRef = useRef(histPeriod);
  histPeriodRef.current = histPeriod;
  const histStartRef = useRef(histStartOn);
  histStartRef.current = histStartOn;
  const histEndRef = useRef(histEndOn);
  histEndRef.current = histEndOn;
  const [summary, setSummary] = useState<DataSummaryGet | null>(null);
  const [dividendLifetime, setDividendLifetime] = useState<DividendGet | null>(null);
  const [exceptions, setExceptions] = useState<ExceptionRecord[]>([]);
  const [workTickets, setWorkTickets] = useState<WorkTicketRecord[]>([]);
  const [ticketFocusSymbol, setTicketFocusSymbol] = useState("");
  const [fleetDetailId, setFleetDetailId] = useState<string | null>(null);
  const [positionFocusPanel, setPositionFocusPanel] = useState<
    "lots" | "income" | "declarations" | "ledger" | ""
  >("");
  const [pdLedger, setPdLedger] = useState<DividendGet | null>(null);
  const [pendingBatchId, setPendingBatchId] = useState<string | null>(null);
  const [pendingBatchStatus, setPendingBatchStatus] = useState<string>("none");
  const [pendingBatchFilename, setPendingBatchFilename] = useState("");
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
  const [drillAccounts, setDrillAccounts] = useState<string[]>(() => [
    ...INCOME_PLAN_DEFAULT_ACCOUNTS,
  ]);
  const [accounts, setAccounts] = useState<AccountListItem[]>([]);
  const [securities, setSecurities] = useState<SecurityListItem[]>([]);
  const [incomeTxOpen, setIncomeTxOpen] = useState(false);
  const [incomeTxPeriod, setIncomeTxPeriod] = useState<IncomeTxPeriod>("month");
  const [incomeTxCustomStart, setIncomeTxCustomStart] = useState("");
  const [incomeTxCustomEnd, setIncomeTxCustomEnd] = useState("");
  const [incomeTxAccountId, setIncomeTxAccountId] = useState("");
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
  const [wizStoredPlan, setWizStoredPlan] = useState<{
    minor: number;
    scale: number;
  } | null>(null);
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
  const [wizCollectorComplete, setWizCollectorComplete] = useState(false);
  const [wizCollectorGaps, setWizCollectorGaps] = useState<string[]>([]);
  const [wizSecondUrl, setWizSecondUrl] = useState("");
  const [wizAskSecondUrl, setWizAskSecondUrl] = useState(false);
  const [wizSecondUrlTried, setWizSecondUrlTried] = useState(false);
  const [wizAskInception, setWizAskInception] = useState(false);
  const [wizInceptionCandidate, setWizInceptionCandidate] = useState("");
  const [wizInceptionOn, setWizInceptionOn] = useState("");
  const [wizExpectedPaid, setWizExpectedPaid] = useState<number | null>(null);
  const [wizRocUrl, setWizRocUrl] = useState("");
  const [wizFieldDecision, setWizFieldDecision] = useState<Record<string, string>>({});
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
  const [ownerRiskChoice, setOwnerRiskChoice] = useState("");
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
  const [lastPriceProgress, setLastPriceProgress] = useState<RefreshProgress | null>(
    null,
  );
  const [declarationBusy, setDeclarationBusy] = useState(false);
  const [declarationProgress, setDeclarationProgress] = useState<RefreshProgress | null>(
    null,
  );
  const [snapshotConfirm, setSnapshotConfirm] = useState(false);
  const [incomeExportPreview, setIncomeExportPreview] =
    useState<IncomePlanExportGet | null>(null);
  const [incomeExportLoading, setIncomeExportLoading] = useState(false);
  const jobsBusyRef = useRef(false);
  const restartingRef = useRef(false);
  const lastPriceKickoff = useRef(false);
  const loadedIncomeRef = useRef(false);
  const loadedTrendsRef = useRef(false);
  const loadedCashRef = useRef(false);
  const loadedElementsRef = useRef(false);
  const elementsWarmAsOfRef = useRef("");
  const incomeGridRef = useRef(incomeGrid);
  incomeGridRef.current = incomeGrid;
  const dividendPerfRef = useRef(dividendPerf);
  dividendPerfRef.current = dividendPerf;
  const taxPlanningRef = useRef(taxPlanning);
  taxPlanningRef.current = taxPlanning;
  const loadedPositionsRef = useRef(false);
  const loadedCalcRef = useRef(false);
  const loadedHistoryRef = useRef(false);
  const loadedDashboardRef = useRef(false);
  const [weekWizardActive, setWeekWizardActive] = useState(false);
  const [cashWeekSavedAt, setCashWeekSavedAt] = useState(0);
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
        rocSourceUrl: string;
      }
    >
  >({});
  const [collectorStatusAt, setCollectorStatusAt] = useState(() =>
    new Date().toISOString(),
  );
  const [collectorRunProgress, setCollectorRunProgress] = useState<{
    running: boolean;
    total: number;
    current: number;
    symbol: string;
    ok: number;
    miss: number;
    lines: string[];
    finishedAt?: string;
  } | null>(null);
  const [retryingTicket, setRetryingTicket] = useState<{
    ticketId: string;
    symbol: string;
  } | null>(null);
  const [ticketDecision, setTicketDecision] = useState<{
    ticketId: string;
    action: "accept" | "reject" | "except";
  } | null>(null);
  const [, setDiv1Compliance] = useState<Div1ComplianceRow[]>([]);
  const [, setCollectorProofs] = useState<CollectorCompletionProof[]>([]);
  const [missingUrlDrafts, setMissingUrlDrafts] = useState<
    Record<string, { sourceUrl: string; rocSourceUrl: string }>
  >({});

  const refreshCollectors = useCallback(async (asOf: string, symbol?: string) => {
    setBusy(true);
    setCollectorAction("Loading collector fleet…");
    try {
      const [setResult, statsResult, complianceResult, calcResult, historyResult] =
        await Promise.all([
        client.executeQuery("CollectorSetGet", { asOfDate: asOf || "2026-08-26" }),
        client.executeQuery("CollectorStatsGet", { asOfDate: asOf || "2026-08-26" }),
        client.executeQuery("Div1ComplianceSummaryGet", {}),
        client.executeQuery("CalculatorGet"),
        client.executeQuery("DeclarationHistoryGet", {
          asOfDate: asOf || "2026-08-26",
          cadence: "all",
          period: "60",
        }),
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
          setMissingUrlDrafts((prev) => {
            const next = { ...prev };
            for (const row of items) {
              if (!collectorNeedsOwnerUrl(row) || next[row.securityId]) continue;
              next[row.securityId] = {
                sourceUrl: row.sourceUrl ?? "",
                rocSourceUrl: row.rocSourceUrl ?? "",
              };
            }
            return next;
          });
          setCollectorStatusAt(new Date().toISOString());
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
                rocSourceUrl: row.rocSourceUrl ?? "",
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
      let complianceRows: Div1ComplianceRow[] = [];
      if (complianceResult.ok && complianceResult.bodyJson) {
        try {
          const body = JSON.parse(complianceResult.bodyJson) as {
            rows?: Div1ComplianceRow[];
          };
          complianceRows = Array.isArray(body.rows) ? body.rows : [];
          setDiv1Compliance(complianceRows);
        } catch {
          setDiv1Compliance([]);
        }
      } else {
        setDiv1Compliance([]);
      }
      let calcBody: CalculatorGet | null = null;
      if (calcResult.ok && calcResult.bodyJson) {
        try {
          calcBody = JSON.parse(calcResult.bodyJson) as CalculatorGet;
          setCalculator(calcBody);
        } catch {
          calcBody = null;
        }
      }
      let histBody: DeclarationHistoryGet | null = null;
      if (historyResult.ok && historyResult.bodyJson) {
        try {
          histBody = JSON.parse(historyResult.bodyJson) as DeclarationHistoryGet;
          setDeclHistory(histBody);
        } catch {
          histBody = null;
        }
      }
      setCollectorProofs(proofsFromFleet(items, complianceRows, calcBody, histBody));
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

  const recordCollectorProof = async (
    row: CollectorSetItem,
    retrieveJson?: string | null,
  ) => {
    let futureDates: string[] = [];
    if (retrieveJson) {
      try {
        const retrieve = JSON.parse(retrieveJson) as { payloadJson?: string };
        const payload = retrieve.payloadJson
          ? (JSON.parse(retrieve.payloadJson) as {
              upcomingPays?: Array<{ payOn?: string; paymentPeriod?: string }>;
              payDates?: Array<{ payOn?: string; paymentPeriod?: string }>;
            })
          : {};
        const pays = payload.upcomingPays ?? payload.payDates ?? [];
        futureDates = [
          ...new Set(
            pays
              .map((p) => (p.payOn || p.paymentPeriod || "").slice(0, 10))
              .filter((d) => d && d > asOfDate),
          ),
        ].sort();
      } catch {
        /* keep empty until fleet refresh */
      }
    }
    const invResult = await client.executeQuery("InvestmentGet", {
      securityId: row.securityId,
      asOfDate,
    });
    let roc2024 = "";
    let roc2025 = "";
    let roc2026e = "";
    let roc2026a = "";
    let paid: CollectorCompletionProof["paid"] = [];
    if (invResult.ok && invResult.bodyJson) {
      try {
        const inv = JSON.parse(invResult.bodyJson) as InvestmentGet;
        const scale = inv.rocScale;
        roc2024 = rocText(inv.rocPct2024ActualMinor, scale);
        roc2025 = rocText(inv.rocPct2025ActualMinor, scale);
        roc2026e = rocText(inv.rocPct2026EstimateMinor, scale);
        roc2026a = rocText(inv.rocPct2026ActualMinor, scale);
        paid = paidRowsFromDeclarations(inv.declarations ?? []);
      } catch {
        /* unknown */
      }
    }
    const histRow = declHistory?.rows.find((h) => h.symbol === row.symbol);
    const proof: CollectorCompletionProof = {
      symbol: row.symbol,
      futureDates,
      roc2024,
      roc2025,
      roc2026e,
      roc2026a,
      weekEnds: histRow ? (declHistory?.weekEnds ?? []) : [],
      weekStarts: histRow ? declHistory?.weekStarts : undefined,
      cells: histRow?.cells ?? [],
      paid,
    };
    setCollectorProofs((prev) => {
      const next = prev.filter((p) => p.symbol !== row.symbol);
      next.push(proof);
      next.sort((a, b) => a.symbol.localeCompare(b.symbol));
      return next;
    });
  };

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
    const timeoutRows: CollectorSetItem[] = [];
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
            collectorCommandBody(
              row,
              forceRefresh
                || isDeclarationWeekdayToday(asOfDate, row.declarationWeekday),
            ),
          );
          if (!result.ok) {
            miss += 1;
            if (collectorMissIsTimeout(result.errorCode ?? "", result.errorCode)) {
              timeoutRows.push(row);
            }
            lines.push(
              `${row.symbol}: fail ${result.errorCode ?? "error"}`,
            );
          } else {
            let recorded = 0;
            let skipped = 0;
            let unchanged = 0;
            let runOk = true;
            let message = "";
            let code = "";
            if (result.bodyJson) {
              try {
                const body = JSON.parse(result.bodyJson) as {
                  recorded?: number;
                  skipped?: number;
                  unchanged?: number;
                  ok?: boolean;
                  message?: string;
                  code?: string;
                };
                recorded = body.recorded ?? 0;
                skipped = body.skipped ?? 0;
                unchanged = body.unchanged ?? 0;
                runOk = body.ok !== false;
                message = body.message ?? "";
                code = body.code ?? "";
              } catch {
                /* ignore */
              }
            }
            if (runOk) {
              ok += 1;
              await recordCollectorProof(row, result.bodyJson);
              lines.push(
                unchanged > 0
                  ? `${row.symbol}: unchanged (fresh today)`
                  : `${row.symbol}: ok — ${formatCount(recorded)} recorded, ${formatCount(skipped)} skipped`,
              );
            } else {
              miss += 1;
              if (collectorMissIsTimeout(message, code)) {
                timeoutRows.push(row);
              }
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
      let retryRows = timeoutRows;
      for (let attempt = 1; attempt < 3 && retryRows.length > 0; attempt++) {
        const wait = [20, 45, 90][attempt];
        lines.push(
          `Timeout retry ${formatCount(attempt + 1)} of 3 (${formatCount(wait)}s) on stored Template Dividend…`,
        );
        const next: CollectorSetItem[] = [];
        for (const row of retryRows) {
          flushSync(() => {
            setCollectorAction(
              `${row.symbol}: timeout retry wait ${formatCount(wait)}s…`,
            );
          });
          const result = await client.executeCommand(
            "CollectorRetrieve",
            collectorCommandBody(row, true, false, { timeoutAttempt: attempt }),
          );
          let runOk = result.ok;
          let message = "";
          let code = "";
          if (result.bodyJson) {
            try {
              const body = JSON.parse(result.bodyJson) as {
                ok?: boolean;
                message?: string;
                code?: string;
                recorded?: number;
              };
              runOk = result.ok && body.ok !== false;
              message = body.message ?? "";
              code = body.code ?? "";
              if (runOk) {
                await recordCollectorProof(row, result.bodyJson);
              }
            } catch {
              /* ignore */
            }
          }
          if (runOk) {
            miss = Math.max(0, miss - 1);
            ok += 1;
            lines.push(`${row.symbol}: ok after ${formatCount(wait)}s retry`);
          } else if (collectorMissIsTimeout(message, code || result.errorCode)) {
            next.push(row);
            lines.push(`${row.symbol}: still timeout at ${formatCount(wait)}s`);
          } else {
            lines.push(
              `${row.symbol}: miss${message ? ` — ${message}` : ""}`,
            );
          }
        }
        retryRows = next;
      }
      const summary = `${label} finished: ${formatCount(ok)} ok, ${formatCount(miss)} miss of ${formatCount(targets.length)}. Reloading fleet…`;
      setCollectorAction(summary);
      setActionMessage(summary);
      const synced = await client.executeCommand("WorkTicketSyncMisses", {});
      if (synced.ok && synced.bodyJson) {
        try {
          const gate = JSON.parse(synced.bodyJson) as {
            failCount?: number;
            failTicketCount?: number;
            failTicketParity?: boolean;
          };
          if (gate.failTicketParity === false) {
            lines.push(
              `Gate broken: ${formatCount(gate.failCount ?? 0)} fail / ${formatCount(gate.failTicketCount ?? 0)} miss tickets`,
            );
          }
        } catch {
          /* stats still reload */
        }
      }
      await refreshCollectors(asOfDate);
      const done = `${label} finished: ${formatCount(ok)} ok, ${formatCount(miss)} miss of ${formatCount(targets.length)}.`;
      setCollectorAction(done);
      setActionMessage(done);
      setCollectorStatusAt(new Date().toISOString());
      setCollectorRunProgress({
        running: false,
        total: targets.length,
        current: targets.length,
        symbol: "",
        ok,
        miss,
        lines: [...lines],
        finishedAt: new Date().toISOString(),
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
      (row) =>
        row.openLots &&
        row.collectorEnabled &&
        row.declarationSource.trim(),
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
    setActionMessage(`Collect fresh distribution data ${row.symbol}…`);
    try {
      const result = await client.executeCommand(
        "CollectorRetrieve",
        collectorCommandBody(row, true),
      );
      if (!result.ok) {
        setActionMessage(
          `Collect fresh distribution data failed: ${result.errorCode ?? "error"}`,
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
          if (body.ok !== false) {
            await recordCollectorProof(row, result.bodyJson);
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

  const establishCollector = async (row: CollectorSetItem) => {
    setBusy(true);
    setActionMessage(`Establish collector ${row.symbol}…`);
    try {
      const retrieve = await client.executeCommand(
        "CollectorRetrieve",
        collectorCommandBody(row, true, true),
      );
      if (!retrieve.ok) {
        setActionMessage(
          `Establish ${row.symbol} retrieve failed: ${retrieve.errorCode ?? "error"}`,
        );
        return;
      }
      const research = await client.executeCommand("PositionResearchRefresh", {
        securityId: row.securityId,
        symbol: row.symbol,
        rocSourceUrl: row.rocSourceUrl ?? "",
        sourceUrl: row.sourceUrl ?? "",
        declarationSource: row.declarationSource,
        forceRefresh: true,
      });
      if (!research.ok) {
        setActionMessage(
          `Establish ${row.symbol} characteristics failed: ${research.errorCode ?? "error"}`,
        );
        return;
      }
      let rocLine = "";
      if (research.bodyJson) {
        try {
          const body = JSON.parse(research.bodyJson) as {
            rocPctMinor?: number | null;
            rocScale?: number;
            rocSourceUrl?: string;
            retrieveOk?: boolean;
            retrieveMessage?: string;
          };
          if (body.rocPctMinor != null) {
            const scale = body.rocScale ?? 2;
            rocLine = ` Recommended ROC ${(body.rocPctMinor / 10 ** scale).toFixed(scale)}% from ${body.rocSourceUrl || "19a-1"}. Accept on this page to complete.`;
          } else {
            rocLine = " ROC unknown — not 0%. Accept is still required if in scope.";
          }
          if (body.retrieveMessage) {
            rocLine += ` ${body.retrieveMessage}`;
          }
        } catch {
          /* ignore */
        }
      }
      setActionMessage(`Established ${row.symbol}.${rocLine}`);
      await refreshCollectors(asOfDate, row.symbol);
      await refreshData(asOfDate);
    } catch (err: unknown) {
      setActionMessage(String(err));
    } finally {
      setBusy(false);
    }
  };

  const reevaluateCollector = async (row: CollectorSetItem) => {
    setBusy(true);
    setActionMessage(`Reevaluate collector ${row.symbol}…`);
    try {
      const research = await client.executeCommand("PositionResearchRefresh", {
        securityId: row.securityId,
        symbol: row.symbol,
        rocSourceUrl: row.rocSourceUrl ?? "",
        sourceUrl: row.sourceUrl ?? "",
        declarationSource: row.declarationSource,
      });
      if (!research.ok) {
        setActionMessage(
          `Reevaluate ${row.symbol} failed: ${research.errorCode ?? "error"}`,
        );
        return;
      }
      setActionMessage(
        `Reevaluated ${row.symbol} characteristics. Pays were not rewritten.`,
      );
      await refreshCollectors(asOfDate, row.symbol);
      await refreshData(asOfDate);
    } catch (err: unknown) {
      setActionMessage(String(err));
    } finally {
      setBusy(false);
    }
  };

  const acceptCollectorRoc = async (row: CollectorSetItem) => {
    if (row.rocEstimateMinor == null) {
      setActionMessage(
        `${row.symbol}: no recommended ROC to accept. Establish or Reevaluate first.`,
      );
      return;
    }
    setBusy(true);
    setActionMessage(`Accept ROC ${row.symbol}…`);
    try {
      const result = await client.executeCommand("RocPlanConfirm", {
        securityId: row.securityId,
        symbol: row.symbol,
        rocPctMinor: row.rocEstimateMinor,
        rocScale: row.rocScale || 2,
        source: "19a-1",
        sourceUrl: row.rocSourceUrl ?? "",
        method: "19a-1-current-year",
        kind: "estimate",
        establishedHow: "current distribution 19a-1 estimate",
        ownerOverride: false,
        asOfDate: asOfDate || new Date().toISOString().slice(0, 10),
      });
      if (!result.ok) {
        setActionMessage(
          `Accept ROC ${row.symbol} failed: ${result.errorCode ?? "error"}`,
        );
        return;
      }
      setActionMessage(
        `Accepted ${row.symbol} ROC ${(row.rocEstimateMinor / 10 ** (row.rocScale || 2)).toFixed(row.rocScale || 2)}%.`,
      );
      await refreshCollectors(asOfDate, row.symbol);
      await refreshData(asOfDate);
    } catch (err: unknown) {
      setActionMessage(String(err));
    } finally {
      setBusy(false);
    }
  };

  const resolveTicketRetry = async (ticket: WorkTicketRecord) => {
    setBusy(true);
    setRetryingTicket({ ticketId: ticket.ticketId, symbol: ticket.symbol });
    setCollectorRunProgress({
      running: true,
      total: 1,
      current: 0,
      symbol: ticket.symbol,
      ok: 0,
      miss: 0,
      lines: [`${ticket.symbol}: retry started — fetching issuer page…`],
    });
    setActionMessage(`${ticket.symbol}: retrying stored adapter…`);
    setCollectorAction(`${ticket.symbol}: retrying stored adapter…`);
    try {
      const { flushSync } = await import("react-dom");
      flushSync(() => {});
      await new Promise<void>((resolve) => {
        window.setTimeout(() => resolve(), 0);
      });
      const resolved = await client.executeCommand("WorkTicketResolve", {
        ticketId: ticket.ticketId,
        tool: "retry_retrieve",
        sourceUrl: "",
      });
      if (!resolved.ok) {
        setActionMessage(`Retry refused: ${resolved.errorCode ?? "error"}`);
        setCollectorRunProgress({
          running: false,
          total: 1,
          current: 1,
          symbol: ticket.symbol,
          ok: 0,
          miss: 1,
          lines: [`${ticket.symbol}: retry refused ${resolved.errorCode ?? "error"}`],
        });
        await refreshData(asOfDate);
        return;
      }
      if (resolved.bodyJson) {
        try {
          const body = JSON.parse(resolved.bodyJson) as { code?: string; message?: string; ok?: boolean };
          const code = body.code ?? "";
          if (body.ok === false || code === "adapter_url_mismatch") {
            setActionMessage(
              body.message ||
                "That URL is not this adapter. Assigned adapter unchanged.",
            );
            setCollectorRunProgress({
              running: false,
              total: 1,
              current: 1,
              symbol: ticket.symbol,
              ok: 0,
              miss: 1,
              lines: [body.message || `${ticket.symbol}: adapter URL mismatch`],
            });
            await refreshData(asOfDate);
            return;
          }
        } catch {
          /* ignore */
        }
      }
      const row = collectorItems.find((r) => r.securityId === ticket.securityId);
      if (!row) {
        setActionMessage(`${ticket.symbol}: retry used the stored adapter URL.`);
        setCollectorRunProgress({
          running: false,
          total: 1,
          current: 1,
          symbol: ticket.symbol,
          ok: 0,
          miss: 1,
          lines: [`${ticket.symbol}: not in collector fleet`],
        });
        await refreshData(asOfDate);
        return;
      }
      flushSync(() => {
        setCollectorRunProgress({
          running: true,
          total: 1,
          current: 1,
          symbol: ticket.symbol,
          ok: 0,
          miss: 0,
          lines: [`${ticket.symbol}: retrieving ${row.declarationSource}…`],
        });
        setCollectorAction(`${ticket.symbol}: retrieving ${row.declarationSource}…`);
      });
      const result = await client.executeCommand("CollectorRetrieve", {
        ...collectorCommandBody(row, true, false, { progressiveRetry: true }),
        sourceUrl: row.sourceUrl || "",
        declarationSource: row.declarationSource,
      });
      let runOk = result.ok;
      let message = "";
      let recorded = 0;
      if (result.bodyJson) {
        try {
          const body = JSON.parse(result.bodyJson) as {
            ok?: boolean;
            message?: string;
            recorded?: number;
            code?: string;
          };
          runOk = result.ok && body.ok !== false;
          message = body.message || body.code || "";
          recorded = body.recorded ?? 0;
        } catch {
          /* ignore */
        }
      }
      const line = runOk
        ? `${ticket.symbol}: ok — ${formatCount(recorded)} recorded${message ? ` (${message})` : ""}`
        : `${ticket.symbol}: miss${message ? ` — ${message}` : result.errorCode ? ` ${result.errorCode}` : ""}`;
      setActionMessage(line);
      setCollectorAction(line);
      setCollectorRunProgress({
        running: false,
        total: 1,
        current: 1,
        symbol: ticket.symbol,
        ok: runOk ? 1 : 0,
        miss: runOk ? 0 : 1,
        lines: [line],
      });
      await refreshCollectors(asOfDate, ticket.symbol);
      await refreshData(asOfDate);
      setActionMessage(line);
      setCollectorAction(line);
    } catch (err: unknown) {
      setActionMessage(String(err));
      setCollectorRunProgress({
        running: false,
        total: 1,
        current: 1,
        symbol: ticket.symbol,
        ok: 0,
        miss: 1,
        lines: [`${ticket.symbol}: ${String(err)}`],
      });
    } finally {
      setRetryingTicket(null);
      setBusy(false);
    }
  };

  const fileWorkTicket = async (ticket: WorkTicketRecord) => {
    const note = window.prompt("Note to file this ticket (required)")?.trim() ?? "";
    if (!note) {
      setActionMessage("File requires a note.");
      return;
    }
    setBusy(true);
    try {
      const result = await client.executeCommand("WorkTicketFile", {
        ticketId: ticket.ticketId,
        note,
      });
      setActionMessage(
        result.ok ? `${ticket.symbol} ${ticket.code} filed.` : `File failed: ${result.errorCode ?? "error"}`,
      );
      await refreshData(asOfDate);
    } catch (err: unknown) {
      setActionMessage(String(err));
    } finally {
      setBusy(false);
    }
  };

  const resolveEnterDeclaredAmount = async (
    ticket: WorkTicketRecord,
    amount: string,
  ) => {
    setBusy(true);
    try {
      const result = await client.executeCommand("WorkTicketResolve", {
        ticketId: ticket.ticketId,
        tool: "enter_declared_amount",
        amount,
      });
      setActionMessage(
        result.ok
          ? `${ticket.symbol}: declared amount saved.`
          : `Save amount failed: ${result.errorCode ?? "error"}`,
      );
      await refreshData(asOfDate);
    } catch (err: unknown) {
      setActionMessage(String(err));
    } finally {
      setBusy(false);
    }
  };

  const resolveAmountConfirm = async (
    ticket: WorkTicketRecord,
    action: "except" | "reject",
  ) => {
    setBusy(true);
    try {
      const result = await client.executeCommand("WorkTicketResolve", {
        ticketId: ticket.ticketId,
        tool: ticket.tool,
        action,
      });
      setActionMessage(
        result.ok
          ? `${ticket.symbol}: ${action === "except" ? "excepted — vendor amount kept." : "rejected — vendor amount discarded."}`
          : `${action} failed: ${result.errorCode ?? "error"}`,
      );
      await refreshData(asOfDate);
    } catch (err: unknown) {
      setActionMessage(String(err));
    } finally {
      setTicketDecision(null);
      setBusy(false);
    }
  };

  const resolveRocPctChange = async (
    ticket: WorkTicketRecord,
    action: "accept" | "reject",
  ) => {
    setBusy(true);
    try {
      const result = await client.executeCommand("WorkTicketResolve", {
        ticketId: ticket.ticketId,
        tool: ticket.tool,
        action,
      });
      setActionMessage(
        result.ok
          ? `${ticket.symbol}: ${
              action === "accept"
                ? "accepted — current-year ROC projection updated."
                : "rejected — previous ROC % kept."
            }`
          : `${action} failed: ${result.errorCode ?? "error"}`,
      );
      await refreshData(asOfDate);
    } catch (err: unknown) {
      setActionMessage(String(err));
    } finally {
      setTicketDecision(null);
      setBusy(false);
    }
  };

  const resolveConfirmTicket = (
    ticket: WorkTicketRecord,
    kind: "positive" | "reject",
  ) => {
    if (ticketDecision) {
      return;
    }
    const action =
      ticket.tool === "roc_confirm"
        ? kind === "positive"
          ? "accept"
          : "reject"
        : kind === "positive"
          ? "except"
          : "reject";
    setTicketDecision({ ticketId: ticket.ticketId, action });
    if (ticket.tool === "roc_confirm") {
      void resolveRocPctChange(ticket, action === "accept" ? "accept" : "reject");
      return;
    }
    void resolveAmountConfirm(ticket, action === "except" ? "except" : "reject");
  };

  const applyMissingCollectorUrls = async () => {
    const rows = collectorItems.filter(collectorNeedsOwnerUrl);
    const filled = rows.filter((row) => {
      const draft = missingUrlDrafts[row.securityId];
      return Boolean(
        draft &&
          (draft.sourceUrl.trim() ||
            (!collectorIsCash(row) && draft.rocSourceUrl.trim())),
      );
    });
    if (filled.length === 0) {
      setActionMessage("Paste at least one issuer URL, then Apply URLs.");
      return;
    }
    setBusy(true);
    setCollectorAction("Applying owner issuer URLs…");
    try {
      let wrote = 0;
      for (const row of filled) {
        const draft = missingUrlDrafts[row.securityId];
        if (!draft) continue;
        const sourceUrl = draft.sourceUrl.trim() || row.sourceUrl || "";
        const rocSourceUrl = collectorIsCash(row)
          ? ""
          : draft.rocSourceUrl.trim() || sourceUrl || row.rocSourceUrl || "";
        if (!sourceUrl && !rocSourceUrl) continue;
        const result = await client.executeCommand("RetrievalTemplateSet", {
          securityId: row.securityId,
          declarationSource: row.declarationSource,
          priceSource: row.priceSource || "public",
          sourceSymbol: row.symbol,
          sourceUrl,
          calendarPolicy: row.calendarPolicy ?? "",
          lookbackCount: 12,
          collectorEnabled: row.collectorEnabled,
          inceptionOn: row.inceptionOn ?? "",
          rocSourceUrl,
        });
        if (!result.ok) {
          setActionMessage(
            `Apply URLs failed for ${row.symbol}: ${result.errorCode ?? "error"}`,
          );
          return;
        }
        wrote += 1;
      }
      setActionMessage(`Applied issuer URLs for ${formatCount(wrote)} collectors.`);
      await refreshCollectors(asOfDate);
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

  const loadDeclHistory = useCallback(
    async (asOf: string, preset: string, startOn: string, endOn: string) => {
      const bounds = historyBounds(preset, asOf, startOn, endOn);
      const result = await client.executeQuery("DeclarationHistoryGet", {
        asOfDate: asOf,
        cadence: "all",
        startOn: bounds.startOn,
        endOn: bounds.endOn,
      });
      if (!result.ok || !result.bodyJson) {
        return;
      }
      try {
        setDeclHistory(JSON.parse(result.bodyJson) as DeclarationHistoryGet);
      } catch {
        setDeclHistory(null);
      }
    },
    [],
  );

  const parseQuery = <T,>(raw?: string): T | null => {
    if (!raw) return null;
    try {
      return JSON.parse(raw) as T;
    } catch {
      return null;
    }
  };

  const refreshHomeData = useCallback(async (asOf: string) => {
    const [
      homeResult,
      dividendResult,
      holdingsResult,
      exceptionResult,
      accountResult,
      securityResult,
    ] = await Promise.all([
      client.executeQuery("HomeOpenGet", { asOfDate: asOf }),
      client.executeQuery("DividendGet"),
      client.executeQuery("HoldingsGet"),
      client.executeQuery("ExceptionList"),
      client.executeQuery("AccountList"),
      client.executeQuery("SecurityList"),
    ]);
    if (homeResult.ok && homeResult.bodyJson) {
      const bundle = parseQuery<HomeOpenGet>(homeResult.bodyJson);
      if (bundle) {
        setSummary(bundle.summary);
        setAccountValues(bundle.accountValue);
        setDividendPlan(bundle.dividendPlan);
      }
    } else {
      const [summaryResult, accountValueResult, dividendPlanResult] = await Promise.all([
        client.executeQuery("DataSummaryGet", { asOfDate: asOf }),
        client.executeQuery("AccountValueHomeGet"),
        client.executeQuery("DividendPlanHomeGet", { asOfDate: asOf }),
      ]);
      if (summaryResult.ok) {
        setSummary(parseQuery<DataSummaryGet>(summaryResult.bodyJson));
      }
      if (accountValueResult.ok) {
        setAccountValues(parseQuery<AccountValueHomeGet>(accountValueResult.bodyJson));
      }
      if (dividendPlanResult.ok) {
        setDividendPlan(parseQuery<DividendPlanHomeGet>(dividendPlanResult.bodyJson));
      }
    }
    if (dividendResult.ok) {
      setDividendLifetime(parseQuery<DividendGet>(dividendResult.bodyJson));
    }
    if (holdingsResult.ok) {
      setHoldings(parseQuery<HoldingsGet>(holdingsResult.bodyJson));
    }
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
    void client.executeQuery("IncomePlanWeekGet", { asOfDate: asOf }).then((week) => {
      if (week.ok && week.bodyJson) {
        setIncomeWeek(parseQuery<IncomePlanWeekGet>(week.bodyJson));
      }
    });
    if (exceptionResult.bodyJson) {
      try {
        setExceptions(JSON.parse(exceptionResult.bodyJson) as ExceptionRecord[]);
      } catch {
        setExceptions([]);
      }
    }
    const syncTickets = await client.executeCommand("WorkTicketSyncMisses", {});
    if (syncTickets.ok && syncTickets.bodyJson) {
      try {
        const body = JSON.parse(syncTickets.bodyJson) as {
          raised?: number;
          openCount?: number;
        };
        if ((body.raised ?? 0) > 0) {
          setActionMessage(
            `Opened ${formatCount(body.raised ?? 0)} collector tickets (${formatCount(body.openCount ?? 0)} open).`,
          );
        }
      } catch {
        /* list still loads */
      }
    }
    const ticketResult = await client.executeQuery("WorkTicketList", {});
    if (ticketResult.ok && ticketResult.bodyJson) {
      try {
        const body = JSON.parse(ticketResult.bodyJson) as WorkTicketList;
        setWorkTickets(Array.isArray(body.items) ? body.items : []);
      } catch {
        setWorkTickets([]);
      }
    } else {
      setWorkTickets([]);
    }
  }, []);

  const loadIncomePack = useCallback(async (asOf: string) => {
    loadedIncomeRef.current = true;
    const weekResult = await client.executeQuery("IncomePlanWeekGet", { asOfDate: asOf });
    if (weekResult.ok) {
      setIncomeWeek(parseQuery<IncomePlanWeekGet>(weekResult.bodyJson));
    }
  }, []);

  const loadTrendsPack = useCallback(async (asOf: string) => {
    loadedTrendsRef.current = true;
    const [trendsResult, trendsWeekResult, trendsPerfResult] = await Promise.all([
      client.executeQuery("TrendsGet", { asOfDate: asOf }),
      client.executeQuery("TrendsWeekGet", {
        asOfDate: trendsWeekAsOfRef.current || "",
      }),
      client.executeQuery("DividendPerformanceGet", {
        asOfDate: asOf,
        range: trendsGraphPeriodRef.current,
      }),
    ]);
    if (trendsResult.ok) {
      const parsed = parseQuery<TrendsGet>(trendsResult.bodyJson);
      if (parsed) {
        setTrends(parsed);
        setTrendsError(null);
      } else {
        setTrends({ points: [], totalMinor: 0, weeks: [], scale: 2 });
        setTrendsError("TrendsGet returned nothing");
      }
    } else {
      setTrends({ points: [], totalMinor: 0, weeks: [], scale: 2 });
      setTrendsError(trendsResult.errorCode ?? "TrendsGet failed");
    }
    if (trendsWeekResult.ok) {
      const parsed = parseQuery<TrendsWeekCapture>(trendsWeekResult.bodyJson);
      setTrendsCapture(parsed);
      if (!trendsWeekAsOfRef.current) {
        trendsWeekAsOfRef.current =
          parsed?.firstUnpopulatedStart || parsed?.periodStart || "";
      }
    }
    if (trendsPerfResult.ok) {
      setTrendsDividendPerf(parseQuery<DividendPerformanceGet>(trendsPerfResult.bodyJson));
    }
  }, []);

  const loadCashPack = useCallback(async (asOf: string) => {
    loadedCashRef.current = true;
    const [
      cashWeekResult,
      cashRemindersResult,
      cashMonthResult,
      cashMagiResult,
      taxPlanningResult,
    ] = await Promise.all([
      client.executeQuery("CashManagementWeekGet", { asOfDate: asOf }),
      client.executeQuery("CashManagementRemindersGet", { asOfDate: asOf }),
      client.executeQuery("CashManagementMonthGet", { asOfDate: asOf }),
      client.executeQuery("MagiProjectionGet"),
      client.executeQuery("TaxPlanningGet", { asOfDate: asOf }),
    ]);
    if (cashWeekResult.ok) {
      setCashWeek(parseQuery<CashManagementWeekGet>(cashWeekResult.bodyJson));
    }
    if (cashRemindersResult.ok) {
      setCashReminders(
        parseQuery<CashManagementRemindersGet>(cashRemindersResult.bodyJson),
      );
    }
    if (cashMonthResult.ok) {
      setCashMonth(parseQuery<CashManagementMonthGet>(cashMonthResult.bodyJson));
    }
    if (cashMagiResult.ok) {
      setCashMagi(parseQuery<MagiProjection>(cashMagiResult.bodyJson));
    } else {
      setCashMagi(null);
    }
    if (taxPlanningResult.ok) {
      const plan = parseQuery<TaxPlanningGet>(taxPlanningResult.bodyJson);
      setTaxPlanning(plan);
      if (plan?.car) {
        setCarRocPlan(plan.car);
      }
      if (plan?.ytd) {
        setCashYtd(plan.ytd);
      }
    } else {
      setTaxPlanning(null);
    }
    const weekAheadResult = await client.executeQuery("WeekAheadGet", {
      asOfDate: asOf,
    });
    if (weekAheadResult.ok) {
      setWeekAhead(parseQuery<WeekAheadGet>(weekAheadResult.bodyJson));
    }
    const cashRegisterResult = await client.executeQuery("CashRegisterGet", {
      asOfDate: asOf,
      account: registerBookRef.current,
      period: registerPeriodRef.current,
    });
    if (cashRegisterResult.ok) {
      setCashRegister(parseQuery<CashRegisterGet>(cashRegisterResult.bodyJson));
    }
    const cashElementsResult = await client.executeQuery("CashElementListGet", {
      account: "all",
      asOfDate: asOf,
    });
    if (cashElementsResult.ok) {
      setCashElements(parseQuery<CashElementListGet>(cashElementsResult.bodyJson));
    }
    const cashYtdResult = await client.executeQuery("CashYtdGet", {
      asOfDate: asOf,
      view: ytdViewRef.current,
    });
    if (cashYtdResult.ok) {
      setCashYtd(parseQuery<CashYtdGet>(cashYtdResult.bodyJson));
    }
    const cashCoverageResult = await client.executeQuery("CashCoverageGet", {
      asOfDate: asOf,
      period: coveragePeriodRef.current,
    });
    if (cashCoverageResult.ok) {
      setCashCoverage(parseQuery<CashCoverageGet>(cashCoverageResult.bodyJson));
    }
  }, []);

  const loadElementsPack = useCallback(async (asOf: string) => {
    loadedElementsRef.current = true;
    const els = await fetchElementList(client, asOf);
    if (els) {
      setCashElements(els);
      elementsWarmAsOfRef.current = asOf;
    }
  }, []);

  const loadPositionsPack = useCallback(async (asOf: string) => {
    loadedPositionsRef.current = true;
    const [positionResult, masterResult, coverageResult] = await Promise.all([
      client.executeQuery("PositionDetailsGet", { asOfDate: asOf }),
      client.executeQuery("PositionMasterGet"),
      client.executeQuery("PositionDetailsCoverageGet"),
    ]);
    if (positionResult.ok) {
      setPositionDetails(parseQuery<PositionDetailsGet>(positionResult.bodyJson));
    }
    if (masterResult.ok) {
      setPositionMaster(parseQuery<PositionMasterGet>(masterResult.bodyJson));
    }
    if (coverageResult.ok) {
      setIssuerCoverage(parseQuery<PositionDetailsCoverageGet>(coverageResult.bodyJson));
    }
  }, []);

  const loadCalcPack = useCallback(async () => {
    loadedCalcRef.current = true;
    const calcResult = await client.executeQuery("CalculatorGet");
    if (calcResult.ok) {
      setCalculator(parseQuery<CalculatorGet>(calcResult.bodyJson));
    }
  }, []);

  const loadHistoryPack = useCallback(async (asOf: string) => {
    loadedHistoryRef.current = true;
    const historyResult = await client.executeQuery("DeclarationHistoryGet", {
      asOfDate: asOf,
      cadence: "all",
      startOn: historyBounds(
        histPeriodRef.current,
        asOf,
        histStartRef.current,
        histEndRef.current,
      ).startOn,
      endOn: historyBounds(
        histPeriodRef.current,
        asOf,
        histStartRef.current,
        histEndRef.current,
      ).endOn,
    });
    if (historyResult.ok) {
      setDeclHistory(parseQuery<DeclarationHistoryGet>(historyResult.bodyJson));
    }
  }, []);

  const loadDashboardPack = useCallback(async (asOf: string) => {
    loadedDashboardRef.current = true;
    const burnResult = await client.executeQuery("DashboardBurndownGet", {
      asOfDate: asOf,
    });
    if (burnResult.ok) {
      setBurndown(parseQuery<DashboardBurndownGet>(burnResult.bodyJson));
    }
  }, []);

  const refreshData = useCallback(async (asOf: string) => {
    await refreshHomeData(asOf);
    const extra: Promise<void>[] = [];
    if (loadedIncomeRef.current) extra.push(loadIncomePack(asOf));
    if (loadedTrendsRef.current) extra.push(loadTrendsPack(asOf));
    if (loadedCashRef.current) extra.push(loadCashPack(asOf));
    else if (loadedElementsRef.current) extra.push(loadElementsPack(asOf));
    if (loadedPositionsRef.current) extra.push(loadPositionsPack(asOf));
    if (loadedCalcRef.current) extra.push(loadCalcPack());
    if (loadedHistoryRef.current) extra.push(loadHistoryPack(asOf));
    if (loadedDashboardRef.current) extra.push(loadDashboardPack(asOf));
    await Promise.all(extra);
  }, [refreshHomeData, loadIncomePack, loadTrendsPack, loadCashPack, loadElementsPack, loadPositionsPack, loadCalcPack, loadHistoryPack, loadDashboardPack]);


  const loadIncomeGrid = useCallback(
    async (
      asOf: string,
      hist: number,
      fut: number,
      accounts: string[],
      weekEnding?: string,
    ) => {
      const result = await client.executeQuery("IncomePlanGridGet", {
        asOfDate: asOf,
        historicalWeeks: hist,
        futureWeeks: fut,
        accounts,
        weekEnding: weekEnding || asOf,
      });
      if (result.ok && result.bodyJson) {
        try {
          setIncomeGrid(JSON.parse(result.bodyJson) as IncomePlanGridGet);
        } catch {
          setIncomeGrid(null);
        }
      }
      const week = await client.executeQuery("IncomePlanWeekGet", {
        asOfDate: asOf,
        weekEnding: weekEnding || asOf,
      });
      if (week.ok && week.bodyJson) {
        try {
          setIncomeWeek(JSON.parse(week.bodyJson) as IncomePlanWeekGet);
        } catch {
          setIncomeWeek(null);
        }
      }
    },
    [],
  );

  const withIncomeLoading = useCallback(async (work: () => Promise<void>) => {
    const gen = ++incomeLoadGen.current;
    setIncomeWeekLoading(true);
    try {
      await work();
    } finally {
      if (gen === incomeLoadGen.current) {
        setIncomeWeekLoading(false);
        setIncomeWeekNav(null);
      }
    }
  }, []);

  const loadIncomeWeek = useCallback(
    async (sat: string) => {
      skipFullRefreshOnAsOfRef.current = true;
      setAsOfDate(sat);
      await withIncomeLoading(async () => {
        const week = await client.executeQuery("IncomePlanWeekGet", {
          asOfDate: sat,
        });
        if (week.ok && week.bodyJson) {
          try {
            setIncomeWeek(JSON.parse(week.bodyJson) as IncomePlanWeekGet);
          } catch {
            setIncomeWeek(null);
          }
        }
      });
    },
    [withIncomeLoading],
  );

  const incomeExportQuery = useCallback(
    (format: "html" | "pdf" | "xlsx") =>
      client.executeQuery("IncomePlanExportGet", {
        pattern: incomePattern,
        format,
        asOfDate: asOfDate,
        historicalWeeks: incomeHistWeeks,
        futureWeeks: incomeFutWeeks,
        accounts: drillAccounts,
        weekEnding: incomeWeek?.end || asOfDate,
        printedAt: new Date().toISOString().slice(0, 16) + "Z",
      }),
    [asOfDate, drillAccounts, incomeFutWeeks, incomeHistWeeks, incomePattern, incomeWeek?.end],
  );

  const requestIncomeExport = useCallback(async () => {
    if (incomeExportLoading) return;
    setIncomeExportLoading(true);
    setBusy(true);
    try {
      const result = await incomeExportQuery("html");
      if (!result.ok || !result.bodyJson) {
        setActionMessage(`Print / Export failed: ${result.errorCode ?? "error"}`);
        return;
      }
      setIncomeExportPreview(JSON.parse(result.bodyJson) as IncomePlanExportGet);
    } finally {
      setIncomeExportLoading(false);
      setBusy(false);
    }
  }, [incomeExportLoading, incomeExportQuery]);

  const commitIncomeExport = useCallback(
    async (action: "print" | "pdf" | "excel") => {
      const preview = incomeExportPreview;
      if (!preview) return;
      if (action === "print") {
        setIncomeExportPreview(null);
        const frame = document.createElement("iframe");
        frame.setAttribute("aria-hidden", "true");
        frame.style.position = "fixed";
        frame.style.right = "0";
        frame.style.bottom = "0";
        frame.style.width = "0";
        frame.style.height = "0";
        frame.style.border = "0";
        document.body.appendChild(frame);
        const doc = frame.contentDocument;
        if (doc) {
          doc.open();
          doc.write(styleIncomePrintHtml(preview.printHtml));
          doc.close();
          frame.contentWindow?.focus();
          frame.contentWindow?.print();
        }
        setTimeout(() => frame.remove(), 1000);
        return;
      }
      setIncomeExportLoading(true);
      setBusy(true);
      try {
        const result = await incomeExportQuery(action === "excel" ? "xlsx" : "pdf");
        if (!result.ok || !result.bodyJson) {
          setActionMessage(`Print / Export failed: ${result.errorCode ?? "error"}`);
          return;
        }
        const body = JSON.parse(result.bodyJson) as IncomePlanExportGet;
        const bytes = Uint8Array.from(atob(body.bytesBase64), (c) => c.charCodeAt(0));
        await invoke("save_local_bytes", {
          defaultFileName: body.defaultFileName,
          bytes: Array.from(bytes),
        });
        setIncomeExportPreview(null);
        setActionMessage(`Saved ${body.defaultFileName} on this computer.`);
      } catch (err: unknown) {
        setActionMessage(`Save failed: ${String(err)}`);
      } finally {
        setIncomeExportLoading(false);
        setBusy(false);
      }
    },
    [incomeExportPreview, incomeExportQuery],
  );

  const requestIncomeExportRef = useRef(requestIncomeExport);
  requestIncomeExportRef.current = requestIncomeExport;

  const runDataSnapshot = useCallback(async () => {
    setBusy(true);
    setActionMessage(null);
    try {
      const result = await client.executeCommand("DataSnapshotExport", {});
      if (!result.ok || !result.bodyJson) {
        setActionMessage(`Data snapshot failed: ${result.errorCode ?? "error"}`);
        return;
      }
      const body = JSON.parse(result.bodyJson) as DataSnapshotExport;
      setActionMessage(`Saved ${body.files.length} files to ${body.folder}`);
    } catch (err: unknown) {
      setActionMessage(String(err));
    } finally {
      setBusy(false);
    }
  }, []);
  const requestDataSnapshot = useCallback(() => {
    setSnapshotConfirm(true);
  }, []);
  const requestDataSnapshotRef = useRef(requestDataSnapshot);
  requestDataSnapshotRef.current = requestDataSnapshot;
  const runDataSnapshotRef = useRef(runDataSnapshot);
  runDataSnapshotRef.current = runDataSnapshot;

  const refreshLastPrices = useCallback(async (force = false) => {
    if (!force) {
      let allowed = false;
      try {
        const window = await client.executeQuery("LastPriceAutoWindowGet", {});
        if (window.ok && window.bodyJson) {
          const body = JSON.parse(window.bodyJson) as {
            allowed?: boolean;
          };
          allowed = body.allowed === true;
        }
      } catch {
        allowed = false;
      }
      if (!allowed) {
        return;
      }
    }
    // Window already said run. Do not set the bar on a skip (weekend / hours / 4-hour).
    setLastPriceBusy(true);
    setLastPriceProgress({ current: 0, total: 0, symbol: "" });
    try {
      await client.executeCommand("ProviderDeclarationSourcesApply", {});
      const result = await client.executeCommand(
        "LastPriceRefresh",
        force ? { force: true } : {},
      );
      if (!result.ok) {
        setActionMessage(
          `Last price refresh failed: ${result.errorCode ?? "error"}. Last stored price still shows, including stale.`,
        );
        return;
      }
      if (result.bodyJson) {
        try {
          const body = JSON.parse(result.bodyJson) as { skipReason?: string };
          if (body.skipReason) {
            return;
          }
        } catch {
          /* keep going */
        }
      }
      const snap = await client.executeCommand("AccountValueSnapshotRecord", {});
      if (!snap.ok && snap.errorCode !== "unknown_command") {
        setActionMessage(
          `Account value snapshot failed: ${snap.errorCode ?? "error"}.`,
        );
      }
      await refreshHomeData(asOfDate);
    } catch (err: unknown) {
      setActionMessage(String(err));
    } finally {
      setLastPriceBusy(false);
      setLastPriceProgress(null);
    }
  }, [asOfDate, refreshHomeData]);

  const refreshDeclarations = useCallback(async (force = false) => {
    if (!force) {
      let inSchedule = false;
      try {
        const window = await client.executeQuery("LastPriceAutoWindowGet", {});
        if (window.ok && window.bodyJson) {
          const body = JSON.parse(window.bodyJson) as {
            inSchedule?: boolean;
            allowed?: boolean;
          };
          inSchedule = body.inSchedule === true;
        }
      } catch {
        inSchedule = false;
      }
      if (!inSchedule) {
        return;
      }
    }
    setDeclarationBusy(true);
    setDeclarationProgress({ current: 0, total: 0, symbol: "" });
    try {
      await client.executeCommand("ProviderDeclarationSourcesApply", {});
      const result = await client.executeCommand(
        "DeclarationRefresh",
        force ? { force: true } : {},
      );
      if (!result.ok) {
        setActionMessage(
          `Declaration refresh failed: ${result.errorCode ?? "error"}.`,
        );
        return;
      }
      await refreshData(asOfDate);
    } catch (err: unknown) {
      setActionMessage(String(err));
    } finally {
      setDeclarationBusy(false);
      setDeclarationProgress(null);
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
          securityId: body.securityId,
          priceSource: draft.priceSource,
          sourceSymbol: draft.sourceSymbol,
          autoPrice: true,
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
        const knownDeclarationAmounts = body.declarations
          .filter(
            (d) =>
              d.paymentPeriod?.trim() &&
              d.amountPerShareMinor != null &&
              d.amountPerShareMinor > 0,
          )
          .map((d) => ({
            paymentPeriod: d.paymentPeriod,
            amountPerShareMinor: d.amountPerShareMinor,
            amountScale: d.amountScale,
          }));
        const result = await client.executeCommand("MarketRetrieve", {
          symbol: body.symbol,
          securityId: body.securityId,
          declarationSource:
            body.template?.declarationSource ?? draft.declarationSource ?? "",
          sourceSymbol: body.template?.sourceSymbol ?? draft.sourceSymbol ?? body.symbol,
          priceSource: body.template?.priceSource ?? draft.priceSource ?? "",
          sourceUrl: body.template?.sourceUrl ?? draft.sourceUrl,
          knownPaymentPeriods,
          knownDeclarationAmounts,
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
          const retrieved = (retrieve.candidates ?? []).filter((d) =>
            isNewOrChangedDeclaration(
              d.paymentPeriod,
              d.amountPerShareMinor,
              d.amountScale,
              body.declarations,
            ) && Boolean(d.source),
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
    client.executeQuery("CoreFunctionsGet").then((result) => {
      if (cancelled || !result.ok || !result.bodyJson) return;
      try {
        setCoreFunctions(JSON.parse(result.bodyJson) as CoreFunctionsGet);
      } catch {
        /* ignore */
      }
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
    if (skipFullRefreshOnAsOfRef.current) {
      skipFullRefreshOnAsOfRef.current = false;
      prevIncomeAsOfRef.current = asOfDate;
      return;
    }
    const prev = prevIncomeAsOfRef.current;
    const weekChanged = prev !== null && prev !== asOfDate;
    prevIncomeAsOfRef.current = asOfDate;
    if (prev !== null && !weekChanged) {
      return;
    }
    if (weekChanged) {
      setIncomeWeekLoading(true);
    }
    const gen = ++incomeLoadGen.current;
    refreshData(asOfDate)
      .catch((err: unknown) => {
        setActionMessage(String(err));
      })
      .finally(() => {
        if (gen === incomeLoadGen.current) {
          setIncomeWeekLoading(false);
          setIncomeWeekNav(null);
        }
      });
  }, [asOfDate, refreshData]);

  useEffect(() => {
    if (screen === "income-plan") {
      void loadIncomePack(asOfDate);
    } else if (screen === "trends") {
      void loadTrendsPack(asOfDate).catch((err: unknown) => {
        setTrends({ points: [], totalMinor: 0, weeks: [], scale: 2 });
        setTrendsError(err instanceof Error ? err.message : String(err));
      });
    } else if (screen === "cash-management") {
      if (cmDesk === "elements") {
        void loadElementsPack(asOfDate);
      } else if (cmDesk !== "external") {
        void loadCashPack(asOfDate);
      }
    } else if (screen === "position-details" || screen === "holdings") {
      void loadPositionsPack(asOfDate);
    } else if (screen === "calculator") {
      void loadCalcPack();
      void loadHistoryPack(asOfDate);
      void loadPositionsPack(asOfDate);
    } else if (screen === "dashboard") {
      void loadDashboardPack(asOfDate);
    }
    if (screen === "position-details") {
      void loadHistoryPack(asOfDate);
    }
  }, [
    screen,
    cmDesk,
    asOfDate,
    loadIncomePack,
    loadTrendsPack,
    loadCashPack,
    loadElementsPack,
    loadPositionsPack,
    loadCalcPack,
    loadDashboardPack,
    loadHistoryPack,
  ]);

  useEffect(() => {
    if (screen !== "home" && screen !== "income-plan") {
      return;
    }
    if (busy || elementDirty || lastPriceBusy || declarationBusy) {
      return;
    }
    if (screen === "home" && !accountValues) {
      return;
    }
    if (screen === "income-plan" && (incomeWeekLoading || !incomeWeek)) {
      return;
    }
    let cancelled = false;
    const stop = scheduleIdleWarm(() => {
      if (cancelled) return;
      void runBackgroundReads(
        client,
        {
          asOf: asOfDate,
          hist: incomeHistWeeks,
          fut: incomeFutWeeks,
          accounts: drillAccounts,
          range: perfRangeRef.current,
          haveGrid:
            screen === "home"
            || incomeGridMemoryMatches(
              incomeGridRef.current,
              asOfDate,
              incomeHistWeeks,
              incomeFutWeeks,
              drillAccounts,
            ),
          havePerf:
            screen === "home"
            || (dividendPerfRef.current?.asOfDate === asOfDate
              && dividendPerfRef.current.range === perfRangeRef.current),
          haveElements: elementsWarmAsOfRef.current === asOfDate,
          haveTax: taxPlanningRef.current?.asOfDate === asOfDate,
        },
        () => cancelled,
      ).then((memory) => {
        if (cancelled) return;
        if (memory.grid) setIncomeGrid(memory.grid);
        if (memory.perf) setDividendPerf(memory.perf);
        if (memory.elements) {
          setCashElements(memory.elements);
          elementsWarmAsOfRef.current = asOfDate;
          loadedElementsRef.current = true;
        }
        if (memory.tax) {
          setTaxPlanning(memory.tax);
          if (memory.tax.car) setCarRocPlan(memory.tax.car);
          if (memory.tax.ytd) setCashYtd(memory.tax.ytd);
        }
      });
    });
    return () => {
      cancelled = true;
      stop();
    };
  }, [
    screen,
    asOfDate,
    busy,
    elementDirty,
    lastPriceBusy,
    declarationBusy,
    accountValues,
    incomeWeek,
    incomeWeekLoading,
    incomeHistWeeks,
    incomeFutWeeks,
    drillAccounts,
  ]);

  useEffect(() => {
    if (screen !== "import") {
      return;
    }
    let cancelled = false;
    (async () => {
      const result = await client.executeQuery("ImportPendingGet");
      if (cancelled || !result.ok || !result.bodyJson) return;
      try {
        const body = JSON.parse(result.bodyJson) as ImportPendingGet;
        if (body.batch?.batchId) {
          setPendingBatchId(body.batch.batchId);
          setPendingBatchStatus(body.batch.status);
          setPendingBatchFilename(body.batch.filename ?? "");
        } else {
          setPendingBatchId(null);
          setPendingBatchStatus("none");
          setPendingBatchFilename("");
        }
      } catch {
        /* ignore */
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [screen]);

  useEffect(() => {
    if (
      screen !== "collectors" &&
      screen !== "settings" &&
      screen !== "collector-establish"
    ) {
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
        rocSourceUrl: draft.rocSourceUrl.trim(),
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
    if (!summary) {
      return;
    }
    lastPriceKickoff.current = true;
    void (async () => {
      if (summary.openLotCount > 0) {
        await refreshLastPrices();
      }
      const alreadyToday =
        Boolean(summary.declarationRefreshedOn) &&
        summary.declarationRefreshedOn === summary.declarationAsOf;
      if (!alreadyToday) {
        await refreshDeclarations();
      }
      if (summary.openLotCount > 0) {
        return;
      }
      await client.executeCommand("AccountValueSnapshotRecord", {});
      await refreshData(asOfDate);
    })().catch((err: unknown) => {
      setActionMessage(String(err));
    });
  }, [summary, refreshLastPrices, refreshDeclarations, refreshData, asOfDate]);

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
        force: true,
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
        force: true,
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
        force: true,
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
      setWizStoredPlan({ minor: planMinor, scale: planScale });
      setActionMessage(
        `Stored Plan ${formatPerShare(planMinor, planScale)}/share (${wizPlanReason}). Confirm Plan wrote Plan / share only. Add lots from Add Lot when ready — not on this screen.`,
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
  const runProcessAResearch = async (overrides?: {
    symbol?: string;
    sourceUrl?: string;
  }) => {
    const symbol = (overrides?.symbol ?? wizSymbol).trim().toUpperCase();
    const sourceUrl = (overrides?.sourceUrl ?? wizSourceUrl).trim();
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
        needsSecondUrl?: boolean;
        secondUrlTried?: boolean;
        adapterFailed?: boolean;
        paidCount?: number;
        needsInceptionConfirm?: boolean;
        inceptionCandidate?: string;
        expectedPaidSinceInception?: number | null;
        inceptionSearchMiss?: boolean;
        recertified?: boolean;
        collectorComplete?: boolean;
        collectorGaps?: string[];
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
      if (inv.planKnown) {
        setWizStoredPlan({
          minor: inv.planPerShareMinor,
          scale: inv.planScale,
        });
        setWizPlan(formatPerShare(inv.planPerShareMinor, inv.planScale));
        if (inv.planReason) setWizPlanReason(inv.planReason);
      } else if (inv.review?.mostCurrentMinor != null) {
        setWizStoredPlan(null);
        setWizPlan(
          formatPerShare(inv.review.mostCurrentMinor, inv.review.amountScale),
        );
        if (!wizPlanReason) setWizPlanReason("Match Most Current");
      } else {
        setWizStoredPlan(null);
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

      // Process A proposes 19a-1 estimate only — never marks research complete.
      // Parsed 0% is valid only when the notice said 0.
      if (seed.rocPctMinor != null && seed.rocPctMinor >= 0) {
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
      setWizCollectorComplete(
        seed.recertified
          ? Boolean(seed.collectorComplete)
          : Boolean(inv.collectorComplete),
      );
      setWizCollectorGaps(
        seed.recertified
          ? (seed.collectorGaps ?? [])
          : (inv.collectorGaps ?? []),
      );
      setWizAskSecondUrl(Boolean(seed.needsSecondUrl) && !seed.secondUrlTried);
      setWizSecondUrlTried(Boolean(seed.secondUrlTried || seed.adapterFailed));
      setWizAskInception(Boolean(seed.needsInceptionConfirm || seed.inceptionSearchMiss));
      setWizInceptionCandidate(seed.inceptionCandidate || "");
      setWizInceptionOn(seed.inceptionCandidate || "");
      setWizExpectedPaid(seed.expectedPaidSinceInception ?? null);
      if (seed.rocSourceUrl) setWizRocUrl(seed.rocSourceUrl);
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
        plan: inv.planKnown
          ? formatPerShare(inv.planPerShareMinor, inv.planScale)
          : inv.review?.mostCurrentMinor != null
            ? formatPerShare(inv.review.mostCurrentMinor, inv.review.amountScale)
            : "",
      });
      const filledBits = [
        inv.provider ? `provider ${inv.provider}` : null,
        inv.underlying ? `underlying ${inv.underlying}` : null,
        (seed.paymentFrequency || inv.paymentFrequency)
          ? `frequency ${seed.paymentFrequency || inv.paymentFrequency}`
          : null,
        seed.rocPctMinor != null && seed.rocPctMinor >= 0
          ? `ROC estimate ${(seed.rocPctMinor / 10 ** (seed.rocScale ?? 2)).toFixed(seed.rocScale ?? 2)}%`
          : null,
      ].filter(Boolean);
      const unknownBits = [
        !inv.provider ? "provider" : null,
        !inv.underlying ? "underlying" : null,
        !(seed.paymentFrequency || inv.paymentFrequency) ? "frequency" : null,
        seed.rocPctMinor == null ? "ROC estimate" : null,
      ].filter(Boolean);
      const retrieveNote = seed.retrieveOk
        ? "Retrieve finished."
        : `Retrieve miss${seed.retrieveCode ? ` (${seed.retrieveCode})` : ""}${seed.retrieveMessage ? `: ${seed.retrieveMessage}` : ""}. Unknown stays unknown — never $0.`;
      const recertNote = seed.recertified
        ? seed.collectorComplete
          ? " Recertify after recreate: complete."
          : ` Recertify after recreate failed${
              seed.collectorGaps?.length
                ? ` (gaps: ${seed.collectorGaps.join(", ")})`
                : ""
            }. Existing lots kept.`
        : "";
      const resultLine = `Filled: ${filledBits.join(", ") || "none"}. Unknown: ${unknownBits.join(", ") || "none"}. ${retrieveNote}${recertNote}`;
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
        if (opts.securityId === wizSecurityId) {
          setWizCollectorComplete(Boolean(inv.collectorComplete));
          setWizCollectorGaps(inv.collectorGaps ?? []);
        }
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
        RISK_TIERS.includes(nextSuggested) ? nextSuggested : "",
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
    ...(cashDirty || elementDirty || externalDirty ? (["cash-management"] as const) : []),
    ...(cartDirty ? (["shopping-cart"] as const) : []),
  ];
  const dirtyScreenNames = [
    pdDirty ? "Position Details" : null,
    wizDirty ? "Add Position" : null,
    addLotDirty ? "Add Lot" : null,
    cashDirty || elementDirty || externalDirty ? "Cash Management" : null,
    cartDirty ? "Shopping Cart" : null,
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
    if (cashDirty) setCashDirty(false);
    if (elementDirty) {
      setElementDirty(false);
      setElementEditorOpen(false);
    }
    if (externalDirty) {
      setExternalDirty(false);
      setExternalReset((n) => n + 1);
    }
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
      setActionMessage("Choose Foundation, Core, or Risk On. Risk is mandatory.");
      return;
    }
    await applySuggestedTier(ownerRiskChoice);
    setPdDraft((prev) => (prev ? { ...prev, risk: ownerRiskChoice } : prev));
  };

  jobsBusyRef.current = busy || lastPriceBusy || declarationBusy;

  const leaveWithoutSaving = (
    next: () => void,
    target?: Screen,
    destDesk?: CmDesk,
  ) => {
    const ontoCm = target === "cash-management";
    const dest = destDesk ?? (ontoCm ? cmDesk : undefined);
    if (weekWizardActive && !(ontoCm && dest === "weekly")) {
      setMenuWorking(null);
      setActionMessage(
        "Finish the week on Review → Accept before leaving.",
      );
      return;
    }
    if (elementDirty && !(ontoCm && dest === "elements")) {
      setMenuWorking(null);
      setActionMessage(
        "Save or Cancel before leaving. Navigation stays blocked while edits are unsaved.",
      );
      return;
    }
    if (externalDirty && !(ontoCm && dest === "external")) {
      setMenuWorking(null);
      setActionMessage(
        "Save or Cancel before leaving. Navigation stays blocked while edits are unsaved.",
      );
      return;
    }
    if (cashDirty && !(ontoCm && dest === "weekly")) {
      setMenuWorking(null);
      setActionMessage(
        "Save or Cancel before leaving. Navigation stays blocked while edits are unsaved.",
      );
      return;
    }
    if (
      (pdDirty || wizDirty || addLotDirty || cashDirty || elementDirty || externalDirty) &&
      (target == null || !dirtyTargets.includes(target))
    ) {
      setMenuWorking(null);
      setActionMessage(
        "Save or Cancel before leaving. Navigation stays blocked while edits are unsaved.",
      );
      return;
    }
    next();
  };

  const leaveWithoutSavingRef = useRef(leaveWithoutSaving);
  leaveWithoutSavingRef.current = leaveWithoutSaving;

  const runAppRestart = useCallback(async () => {
    if (restartingRef.current) {
      return;
    }
    if (pdDirty || wizDirty || addLotDirty || cashDirty || elementDirty || externalDirty || weekWizardActive) {
      setActionMessage(
        weekWizardActive
          ? "Finish the week on Review → Accept before restarting."
          : "Save or Cancel before restarting. Restart stays blocked while edits are unsaved.",
      );
      return;
    }
    restartingRef.current = true;
    setRestarting(true);
    setActionMessage(
      "Restarting coding launch. Waiting for in-flight work, then closing the data file. Coding start is finos.bat; a new finos (dev) console opens.",
    );
    await new Promise((resolve) => window.setTimeout(resolve, 0));
    const deadline = Date.now() + 30_000;
    while (jobsBusyRef.current && Date.now() < deadline) {
      await new Promise((resolve) => window.setTimeout(resolve, 200));
    }
    try {
      const { WebviewWindow } = await import("@tauri-apps/api/webviewWindow");
      const wizard = await WebviewWindow.getByLabel("import-wizard");
      await wizard?.close();
    } catch {
      /* no extra window */
    }
    try {
      await invoke("app_restart");
    } catch (err: unknown) {
      restartingRef.current = false;
      setRestarting(false);
      setActionMessage(`Restart failed: ${String(err)}`);
    }
  }, [pdDirty, wizDirty, addLotDirty, cashDirty, elementDirty, externalDirty, weekWizardActive]);
  const runAppRestartRef = useRef(runAppRestart);
  runAppRestartRef.current = runAppRestart;

  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;
    void (async () => {
      try {
        const { listen } = await import("@tauri-apps/api/event");
        const stop = await listen<string>("finos-navigate", (event) => {
          const id = event.payload;
          if (id === "home") {
            goHomeRef.current();
            return;
          }
          if (id === "cash-elements") {
            leaveWithoutSavingRef.current(
              () => {
                setCmDesk("elements");
                setScreen("cash-management");
              },
              "cash-management",
              "elements",
            );
            return;
          }
          if (id === "cash-cashflow") {
            leaveWithoutSavingRef.current(
              () => {
                setCmDesk("cashflow");
                setScreen("cash-management");
              },
              "cash-management",
              "cashflow",
            );
            return;
          }
          if (id === "cash-weekly" || id === "cash-management") {
            leaveWithoutSavingRef.current(
              () => {
                setCmDesk("weekly");
                setScreen("cash-management");
              },
              "cash-management",
              "weekly",
            );
            return;
          }
          if (id === "cash-car-tax") {
            leaveWithoutSavingRef.current(
              () => {
                setCmDesk("car");
                setScreen("cash-management");
              },
              "cash-management",
              "car",
            );
            return;
          }
          if (id === "cash-coverage") {
            leaveWithoutSavingRef.current(
              () => {
                setCmDesk("coverage");
                setScreen("cash-management");
              },
              "cash-management",
              "coverage",
            );
            return;
          }
          if (id === "cash-external") {
            leaveWithoutSavingRef.current(
              () => {
                setCmDesk("external");
                setScreen("cash-management");
              },
              "cash-management",
              "external",
            );
            return;
          }
          if (!isScreen(id)) return;
          leaveWithoutSavingRef.current(() => setScreen(id), id);
        });
        const stopSnapshot = await listen<string>("finos-data-snapshot", () => {
          requestDataSnapshotRef.current();
        });
        const stopRestart = await listen<string>("finos-app-restart", () => {
          void runAppRestartRef.current();
        });
        const stopDeclProgress = await listen<{
          current?: number;
          total?: number;
          symbol?: string;
        }>("declaration-refresh-progress", (event) => {
          const p = event.payload;
          setDeclarationProgress({
            current: p.current ?? 0,
            total: p.total ?? 0,
            symbol: p.symbol ?? "",
          });
        });
        const stopPriceProgress = await listen<{
          current?: number;
          total?: number;
          symbol?: string;
        }>("last-price-refresh-progress", (event) => {
          const p = event.payload;
          setLastPriceProgress({
            current: p.current ?? 0,
            total: p.total ?? 0,
            symbol: p.symbol ?? "",
          });
        });
        if (cancelled) {
          stop();
          stopSnapshot();
          stopRestart();
          stopDeclProgress();
          stopPriceProgress();
          return;
        }
        unlisten = () => {
          stop();
          stopSnapshot();
          stopRestart();
          stopDeclProgress();
          stopPriceProgress();
        };
      } catch {
        /* browser preview without Tauri events */
      }
    })();
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    const close = () => setOpenMenu(null);
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        close();
        setSnapshotConfirm(false);
        setIncomeExportPreview(null);
      }
      if (
        (event.ctrlKey || event.metaKey) &&
        event.key.toLowerCase() === "p" &&
        screenRef.current === "income-plan"
      ) {
        event.preventDefault();
        void requestIncomeExportRef.current();
      }
    };
    window.addEventListener("click", close);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("click", close);
      window.removeEventListener("keydown", onKey);
    };
  }, []);

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
    if (!RISK_TIERS.includes(pdDraft.risk)) {
      setActionMessage("Choose Foundation, Core, or Risk On before saving.");
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
        replaceCadence: true,
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
        securityId: investment.securityId,
        priceSource: pdDraft.priceSource,
        sourceSymbol: pdDraft.sourceSymbol,
        force: true,
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
        knownDeclarationAmounts: (investment.declarations ?? [])
          .filter(
            (d) =>
              d.paymentPeriod?.trim() &&
              d.amountPerShareMinor != null &&
              d.amountPerShareMinor > 0,
          )
          .map((d) => ({
            paymentPeriod: d.paymentPeriod,
            amountPerShareMinor: d.amountPerShareMinor,
            amountScale: d.amountScale,
          })),
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
      const retrieved = (body.candidates ?? []).filter((d) =>
        Boolean(d.source) &&
        isNewOrChangedDeclaration(
          d.paymentPeriod,
          d.amountPerShareMinor,
          d.amountScale,
          investment.declarations ?? [],
        ),
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
        const msg = "No research gaps — open-lot names have provider, frequency, DIV-1, and ROC estimate.";
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
      const opened = result.bodyJson
        ? (JSON.parse(result.bodyJson) as { lotId?: string })
        : {};
      const pending = pendingCartBuyRef.current;
      if (pending && opened.lotId) {
        const step = await client.executeCommand("CartExecuteBuyStep", {
          scenarioId: pending.scenarioId,
          lotId: opened.lotId,
        });
        pendingCartBuyRef.current = null;
        if (!step.ok) {
          setActionMessage(
            `Lot opened; cart buy step not recorded: ${step.errorCode ?? "error"}`,
          );
        }
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
    try {
      const content = await file.text();
      const staged = await client.executeCommand("ImportStage", {
        sourceId: `ui-file-${file.name}`,
        filename: file.name,
        content,
      });
      if (!staged.ok || !staged.bodyJson) {
        setActionMessage(`Import failed: ${staged.errorCode ?? "error"}`);
        return;
      }
      const stagedBody = JSON.parse(staged.bodyJson) as ImportBatchRecord;
      const batchId = stagedBody.batchId;
      if (!batchId) {
        setActionMessage("Import failed: missing batch");
        return;
      }
      setPendingBatchId(batchId);
      setPendingBatchFilename(file.name);
      const validated = await client.executeCommand("ImportValidate", { batchId });
      const status =
        validated.bodyJson != null
          ? (JSON.parse(validated.bodyJson) as ImportBatchRecord).status ?? "unknown"
          : "unknown";
      setPendingBatchStatus(status);
      if (!validated.ok) {
        setActionMessage(
          `Read ${file.name} (${stagedBody.candidateCount ?? 0} transactions). Validation failed: ${validated.errorCode ?? "error"}.`,
        );
        return;
      }
      await openImportWizardWindow(batchId, file.name);
      setActionMessage(
        `Opened Import for ${file.name} (${stagedBody.candidateCount ?? 0} transactions). Load or Cancel in that window.`,
      );
    } catch (err: unknown) {
      setActionMessage(String(err));
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
      if (!pdDirty && !wizDirty && !addLotDirty && !weekWizardActive) return;
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
          if (!pdDirty && !wizDirty && !addLotDirty && !weekWizardActive) return;
          event.preventDefault();
          setActionMessage(
            weekWizardActive
              ? "Finish the week on Review → Accept before leaving."
              : "Save or Cancel before leaving. Navigation stays blocked while edits are unsaved.",
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
  }, [pdDirty, wizDirty, addLotDirty, weekWizardActive]);

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

  const incomeThroughOn = summary?.latestYieldOn?.trim() ?? "";
  const incomeWeekDeclared = (() => {
    const rows = incomeWeek?.positions ?? [];
    const declared = rows.filter((row) => row.declarationKnown);
    if (declared.length === 0) return null;
    return declared.reduce((sum, row) => sum + (row.declarationMinor ?? 0), 0);
  })();
  const incomeWeekImported = (() => {
    const bounds = satFriWeek(incomeThroughOn);
    if (!bounds || !dividendLifetime) return null;
    const cents = dividendLifetime.actuals.reduce((sum, row) => {
      const day = row.occurredOn.slice(0, 10);
      if (day < bounds.start || day > bounds.end) return sum;
      return sum + usdCents(row.amountMinor, row.scale);
    }, 0);
    return cents;
  })();
  const incomeWeekTotal = incomeWeekDeclared ?? incomeWeekImported;
  const incomeTxToday = localIsoDate();
  const incomeTxWindow = useMemo(
    () =>
      incomeTxRange(
        incomeTxPeriod,
        incomeTxToday,
        incomeTxCustomStart,
        incomeTxCustomEnd,
      ),
    [incomeTxPeriod, incomeTxToday, incomeTxCustomStart, incomeTxCustomEnd],
  );
  const incomeTxAccountOptions = useMemo(
    () =>
      accounts
        .filter((a) => a.name.trim() && a.name.toLowerCase() !== "external")
        .slice()
        .sort((a, b) => a.name.localeCompare(b.name)),
    [accounts],
  );
  const incomeThroughRows = useMemo(() => {
    const accountName = new Map(accounts.map((a) => [a.accountId, a.name]));
    const symbolOf = new Map(securities.map((s) => [s.securityId, s.symbol]));
    return (dividendLifetime?.actuals ?? [])
      .filter((row) => inIncomeTxRange(row.occurredOn, incomeTxWindow))
      .filter((row) => !incomeTxAccountId || row.accountId === incomeTxAccountId)
      .slice()
      .sort((a, b) => {
        const day = b.occurredOn.localeCompare(a.occurredOn);
        if (day !== 0) return day;
        const acct = (accountName.get(a.accountId) ?? "").localeCompare(
          accountName.get(b.accountId) ?? "",
        );
        if (acct !== 0) return acct;
        const sa = a.securityId ? (symbolOf.get(a.securityId) ?? "") : "";
        const sb = b.securityId ? (symbolOf.get(b.securityId) ?? "") : "";
        return sa.localeCompare(sb);
      })
      .map((row) => ({
        id: row.actualId,
        date: row.occurredOn,
        account: accountName.get(row.accountId) ?? "unknown",
        symbol: row.securityId
          ? (symbolOf.get(row.securityId) ?? "unknown")
          : "unknown",
        amountMinor: row.amountMinor,
        scale: row.scale,
      }));
  }, [accounts, securities, dividendLifetime, incomeTxWindow, incomeTxAccountId]);

  useEffect(() => {
    if (!incomeTxOpen) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") setIncomeTxOpen(false);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [incomeTxOpen]);

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
    roc: wizRoc != null && wizRoc.rocPctMinor != null,
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
      wizCollectorComplete
        ? `Saved ${wizSymbol.trim().toUpperCase()}. Collector is complete — Add lots when ready.`
        : processALotCount > 0
          ? `Saved ${wizSymbol.trim().toUpperCase()}. Existing lots kept. Collector incomplete${
              wizCollectorGaps.length ? ` (gaps: ${wizCollectorGaps.join(", ")})` : ""
            } — Recreate Research recertifies the same Establish gate.`
          : `Saved ${wizSymbol.trim().toUpperCase()}. Identity saved. Add lots stays blocked until the collector is complete${
              wizCollectorGaps.length ? ` (gaps: ${wizCollectorGaps.join(", ")})` : ""
            }.`,
    );
    void refreshData(asOfDate || new Date().toISOString().slice(0, 10));
  };

  const resetProcessAForm = () => {
    setWizProcessASaved(false);
    setWizResearchDone(false);
    setWizCollectorComplete(false);
    setWizCollectorGaps([]);
    setWizSecondUrl("");
    setWizAskSecondUrl(false);
    setWizSecondUrlTried(false);
    setWizAskInception(false);
    setWizInceptionCandidate("");
    setWizInceptionOn("");
    setWizExpectedPaid(null);
    setWizRocUrl("");
    setWizFieldDecision({});
    setWizSecurityId("");
    setWizName("");
    setWizFreq("");
    setWizDecls([]);
    setWizRoc(null);
    setWizRocPct("");
    setWizPrice("");
    setWizPriceState(null);
    setWizRetrieveNote("");
    setWizTierSuggestion(null);
    setWizPlanStored(false);
    setWizStoredPlan(null);
    setWizRisk("");
    setWizDeclSource("");
  };

  const startAnotherProcessA = () => {
    resetProcessAForm();
    setWizSymbol("");
    setWizSourceUrl("");
    setWizBaseline(JSON.stringify(emptyWizEdit()));
  };

  const openRecreateAdapter = (ticket: WorkTicketRecord) => {
    leaveWithoutSaving(() => {
      void (async () => {
        const symbol = ticket.symbol.trim().toUpperCase();
        const row =
          collectorItems.find(
            (r) => r.securityId && r.securityId === ticket.securityId,
          ) ||
          collectorItems.find((r) => r.symbol.trim().toUpperCase() === symbol);
        let storedUrl = (row?.sourceUrl || "").trim();
        let storedRoc = (row?.rocSourceUrl || "").trim();
        const securityId = ticket.securityId || row?.securityId || "";
        if (!storedUrl && securityId) {
          const asOf = asOfDate || new Date().toISOString().slice(0, 10);
          const invResult = await client.executeQuery("InvestmentGet", {
            securityId,
            asOfDate: asOf,
          });
          if (invResult.ok && invResult.bodyJson) {
            const inv = JSON.parse(invResult.bodyJson) as InvestmentGet;
            storedUrl = (inv.template?.sourceUrl || "").trim();
            storedRoc = storedRoc || (inv.template?.rocSourceUrl || "").trim();
          }
        }
        resetProcessAForm();
        setWizSymbol(symbol);
        setWizSourceUrl(storedUrl);
        if (securityId) setWizSecurityId(securityId);
        if (storedRoc) setWizRocUrl(storedRoc);
        if (row?.declarationSource) setWizDeclSource(row.declarationSource);
        setWizBaseline(
          JSON.stringify({ ...emptyWizEdit(), symbol, sourceUrl: storedUrl }),
        );
        setScreen("new-investment");
        if (securityId) {
          setActionMessage(
            `${symbol}: using stored Template Dividend. Validating stored facts…`,
          );
          await validateRecreateFromStored({
            symbol,
            securityId,
            sourceUrl: storedUrl,
            declarationSource: row?.declarationSource || "",
          });
        } else if (storedUrl) {
          setActionMessage(
            `${symbol}: using stored Template Dividend. Researching to self-heal…`,
          );
          await runProcessAResearch({ symbol, sourceUrl: storedUrl });
        } else {
          setActionMessage(
            `${symbol}: no stored Template Dividend. Paste the issuer distribution URL, then Research.`,
          );
        }
      })();
    });
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

  const reloadProcessAFacts = async () => {
    if (!wizSecurityId) return;
    const asOf = asOfDate || new Date().toISOString().slice(0, 10);
    const invResult = await client.executeQuery("InvestmentGet", {
      securityId: wizSecurityId,
      asOfDate: asOf,
    });
    if (!invResult.ok || !invResult.bodyJson) return;
    const inv = JSON.parse(invResult.bodyJson) as InvestmentGet;
    setWizCollectorComplete(Boolean(inv.collectorComplete));
    setWizCollectorGaps(inv.collectorGaps ?? []);
    setWizProvider(inv.provider || "");
    setWizUnderlying(inv.underlying || "");
    setWizFreq(inv.paymentFrequency || wizFreq);
    setWizRisk(inv.riskTier || wizRisk);
    if (inv.template?.sourceUrl) setWizSourceUrl(inv.template.sourceUrl);
    if (inv.template?.rocSourceUrl) setWizRocUrl(inv.template.rocSourceUrl);
    if (inv.template?.inceptionOn) setWizInceptionOn(inv.template.inceptionOn);
    const rem = await client.executeQuery("RemainingYearIncomeGet", {
      securityId: wizSecurityId,
      asOfDate: asOf,
    });
    if (rem.ok && rem.bodyJson) {
      const body = JSON.parse(rem.bodyJson) as RemainingYearIncomeGet;
      setWizRemaining(body);
    }
    if (inv.rocPct2026EstimateMinor != null) {
      setWizRocPct((inv.rocPct2026EstimateMinor / 100).toFixed(2));
    }
    await refreshData(asOf);
  };

  const hydrateProcessAStoredFacts = async (securityId: string, symbol: string) => {
    const asOf = asOfDate || new Date().toISOString().slice(0, 10);
    const invResult = await client.executeQuery("InvestmentGet", {
      securityId,
      asOfDate: asOf,
    });
    if (!invResult.ok || !invResult.bodyJson) {
      return null;
    }
    const inv = JSON.parse(invResult.bodyJson) as InvestmentGet;
    setWizSecurityId(securityId);
    setWizSymbol(inv.symbol || symbol);
    setWizName(inv.name || symbol);
    setWizProvider(inv.provider || "");
    setWizUnderlying(inv.underlying || "");
    setWizFreq(inv.paymentFrequency || "");
    setWizRisk(inv.riskTier || "");
    setWizLookthrough(mergeLookthrough(inv.lookthrough));
    setWizReview(inv.review);
    setWizPriceState(inv.price);
    if (inv.price.priceMinor != null && inv.price.priceMinor > 0) {
      setWizPrice(scaledDollars(inv.price.priceMinor, inv.price.scale));
    }
    if (inv.template?.sourceUrl) setWizSourceUrl(inv.template.sourceUrl);
    if (inv.template?.rocSourceUrl) setWizRocUrl(inv.template.rocSourceUrl);
    if (inv.template?.declarationSource) setWizDeclSource(inv.template.declarationSource);
    if (inv.template?.calendarPolicy) setWizCalendarPolicy(inv.template.calendarPolicy);
    if (inv.template?.inceptionOn) setWizInceptionOn(inv.template.inceptionOn);
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
    if (inv.planKnown) {
      setWizStoredPlan({
        minor: inv.planPerShareMinor,
        scale: inv.planScale,
      });
      setWizPlan(formatPerShare(inv.planPerShareMinor, inv.planScale));
      if (inv.planReason) setWizPlanReason(inv.planReason);
    }
    setWizPlanStored(Boolean(inv.planKnown));
    setWizCollectorComplete(Boolean(inv.collectorComplete));
    setWizCollectorGaps(inv.collectorGaps ?? []);
    if (inv.rocPct2026EstimateMinor != null) {
      setWizRocPct((inv.rocPct2026EstimateMinor / 100).toFixed(2));
    }
    const rem = await client.executeQuery("RemainingYearIncomeGet", {
      securityId,
      asOfDate: asOf,
    });
    if (rem.ok && rem.bodyJson) {
      const body = JSON.parse(rem.bodyJson) as RemainingYearIncomeGet;
      setWizRemaining(body);
    }
    setWizResearchDone(true);
    setWizPart1Stored(true);
    snapshotWiz({
      symbol: inv.symbol || symbol,
      name: inv.name || symbol,
      provider: inv.provider || "",
      underlying: inv.underlying || "",
      freq: inv.paymentFrequency || "",
      risk: inv.riskTier || "",
      sourceUrl: inv.template?.sourceUrl || "",
      declSource: inv.template?.declarationSource || "",
      calendarPolicy: inv.template?.calendarPolicy || "",
      plan: inv.planKnown
        ? formatPerShare(inv.planPerShareMinor, inv.planScale)
        : "",
    });
    return inv;
  };

  const validateRecreateFromStored = async (opts: {
    symbol: string;
    securityId: string;
    sourceUrl: string;
    declarationSource: string;
  }) => {
    const PROCESS_A_TOTAL = 2;
    setBusy(true);
    setWizResearchDone(false);
    setResearchActivity({
      running: true,
      step: 1,
      total: PROCESS_A_TOTAL,
      label: "validating stored facts",
      resultLine: null,
    });
    try {
      const asOf = asOfDate || new Date().toISOString().slice(0, 10);
      const recert = await client.executeCommand("CollectorRecertify", {
        securityId: opts.securityId,
        asOfDate: asOf,
        trigger: "recreate",
      });
      let recertComplete = false;
      let recertGaps: string[] = [];
      if (recert.ok && recert.bodyJson) {
        const body = JSON.parse(recert.bodyJson) as {
          complete?: boolean;
          gaps?: string[];
        };
        recertComplete = Boolean(body.complete);
        recertGaps = body.gaps ?? [];
      }
      let retrieveNote = "Retrieve skipped (no stored Template Dividend).";
      if (opts.sourceUrl) {
        setResearchActivity({
          running: true,
          step: 2,
          total: PROCESS_A_TOTAL,
          label: "retrieving with stored Template Dividend",
          resultLine: null,
        });
        const row =
          collectorItems.find((r) => r.securityId === opts.securityId) ||
          collectorItems.find((r) => r.symbol.trim().toUpperCase() === opts.symbol);
        const retrieve = await client.executeCommand("CollectorRetrieve", {
          ...(row
            ? collectorCommandBody(row, true)
            : {
                securityId: opts.securityId,
                symbol: opts.symbol,
                declarationSource: opts.declarationSource,
                sourceUrl: opts.sourceUrl,
                forceRefresh: true,
              }),
          sourceUrl: opts.sourceUrl,
        });
        let runOk = retrieve.ok;
        let message = "";
        if (retrieve.bodyJson) {
          try {
            const body = JSON.parse(retrieve.bodyJson) as {
              ok?: boolean;
              message?: string;
              code?: string;
            };
            runOk = retrieve.ok && body.ok !== false;
            message = body.message || body.code || "";
          } catch {
            /* ignore */
          }
        }
        retrieveNote = runOk
          ? `Retrieve ok${message ? ` (${message})` : ""}. New pays only; stored pays kept.`
          : `Retrieve miss${message ? ` — ${message}` : retrieve.errorCode ? ` ${retrieve.errorCode}` : ""}. Stored pays kept.`;
      }
      const inv = await hydrateProcessAStoredFacts(opts.securityId, opts.symbol);
      const gaps = recertGaps.length ? recertGaps.join(", ") : "none";
      const resultLine = `${opts.symbol}: stored facts kept. Validation ${
        recertComplete ? "complete" : `gaps: ${gaps}`
      }. ${retrieveNote} Confirm Plan is not required unless you change Plan.`;
      setWizRetrieveNote(resultLine);
      setActionMessage(resultLine);
      setResearchActivity({
        running: false,
        step: PROCESS_A_TOTAL,
        total: PROCESS_A_TOTAL,
        label: "",
        resultLine,
      });
      if (inv) {
        setWizCollectorComplete(recertComplete || Boolean(inv.collectorComplete));
        if (recertGaps.length) setWizCollectorGaps(recertGaps);
      }
      await refreshData(asOf);
    } catch (err: unknown) {
      setActionMessage(String(err));
      setResearchActivity({
        running: false,
        step: 0,
        total: PROCESS_A_TOTAL,
        label: "",
        resultLine: String(err),
      });
    } finally {
      setBusy(false);
    }
  };

  const retryProcessASecondUrl = async () => {
    if (!wizSecurityId || !wizSecondUrl.trim()) {
      setActionMessage("Paste a second issuer distribution URL (same adapter).");
      return;
    }
    setBusy(true);
    try {
      const result = await client.executeCommand("PositionResearchRefresh", {
        securityId: wizSecurityId,
        sourceUrl: wizSecondUrl.trim(),
        secondUrlAttempt: true,
      });
      if (!result.ok || !result.bodyJson) {
        setActionMessage(`Second URL failed: ${result.errorCode ?? "error"}`);
        return;
      }
      const body = JSON.parse(result.bodyJson) as {
        retrieveOk?: boolean;
        secondUrlTried?: boolean;
        adapterFailed?: boolean;
        retrieveMessage?: string;
        retrieveCode?: string;
      };
      setWizAskSecondUrl(false);
      setWizSecondUrlTried(Boolean(body.secondUrlTried || body.adapterFailed));
      if (body.adapterFailed || !body.retrieveOk) {
        setActionMessage(
          body.retrieveMessage ||
            `Second distribution URL failed (${body.retrieveCode ?? "miss"}). Adapter not built. Manual adapter is parked.`,
        );
      }
      await reloadProcessAFacts();
    } finally {
      setBusy(false);
    }
  };

  const confirmProcessAInception = async (yes: boolean) => {
    if (!wizSecurityId) return;
    setBusy(true);
    try {
      const result = await client.executeCommand("PositionResearchRefresh", {
        securityId: wizSecurityId,
        sourceUrl: wizSourceUrl.trim(),
        inceptionConfirmed: yes,
        inceptionOn: yes ? wizInceptionOn.trim() || wizInceptionCandidate : "",
        asOfDate: asOfDate || new Date().toISOString().slice(0, 10),
      });
      if (!result.ok) {
        setActionMessage(`Inception confirm failed: ${result.errorCode ?? "error"}`);
        return;
      }
      setWizAskInception(false);
      setActionMessage(
        yes
          ? "Inception stored. Collector stays incomplete until the rest of the checklist is accepted."
          : "Owner said No. Paid history stays short — ticket open, collector incomplete.",
      );
      await reloadProcessAFacts();
    } finally {
      setBusy(false);
    }
  };

  const submitProcessARocUrl = async () => {
    if (!wizSecurityId || !wizRocUrl.trim()) {
      setActionMessage("Paste a 19a-1 / ROC notice URL.");
      return;
    }
    setBusy(true);
    try {
      const result = await client.executeCommand("PositionResearchRefresh", {
        securityId: wizSecurityId,
        sourceUrl: wizSourceUrl.trim(),
        rocSourceUrl: wizRocUrl.trim(),
      });
      if (!result.ok) {
        setActionMessage(`ROC URL failed: ${result.errorCode ?? "error"}`);
        return;
      }
      setActionMessage("ROC URL stored as the reusable template. 0% only if the notice said 0.");
      await reloadProcessAFacts();
    } finally {
      setBusy(false);
    }
  };

  const decideProcessAField = async (field: string, decision: "accept" | "skip") => {
    if (!wizSecurityId) return;
    setBusy(true);
    try {
      const result = await client.executeCommand("CollectorFieldDecisionSet", {
        securityId: wizSecurityId,
        field,
        decision,
      });
      if (!result.ok) {
        setActionMessage(`Checklist ${decision} failed: ${result.errorCode ?? "error"}`);
        return;
      }
      setWizFieldDecision((prev) => ({ ...prev, [field]: decision }));
      await reloadProcessAFacts();
      setActionMessage(
        decision === "skip" && field !== "backtest"
          ? `Skipped ${field} — ticket opened, collector incomplete.`
          : decision === "accept"
            ? `Accepted ${field}.`
            : `Backtest skipped — does not block complete.`,
      );
    } finally {
      setBusy(false);
    }
  };

  const goProcessAToAddLot = () => {
    if (!wizSecurityId) return;
    if (!wizCollectorComplete && processALotCount === 0) {
      setActionMessage(
        `Add lots is blocked until the collector is complete${
          wizCollectorGaps.length ? ` (gaps: ${wizCollectorGaps.join(", ")})` : ""
        }.`,
      );
      return;
    }
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

  const goHome = () => {
    setOpenMenu(null);
    leaveWithoutSaving(() => {
      setScreen("home");
    }, "home");
  };
  const goHomeRef = useRef(goHome);
  goHomeRef.current = goHome;

  useEffect(() => {
    if (menuWorking) {
      setMenuWorking(null);
    }
  }, [screen, cmDesk]);

  const goCmDesk = (desk: CmDesk, label: string) => {
    setOpenMenu(null);
    setMenuWorking(label);
    leaveWithoutSaving(
      () => {
        setCmDesk(desk);
        setScreen("cash-management");
        if (screen === "cash-management" && cmDesk === desk) {
          setMenuWorking(null);
        }
      },
      "cash-management",
      desk,
    );
  };

  const cmDeskButton = (desk: CmDesk, label: string) => (
    <button
      type="button"
      role="menuitem"
      className="menubar-item"
      aria-label={label}
      aria-current={
        screen === "cash-management" && cmDesk === desk ? "page" : undefined
      }
      aria-busy={menuWorking === label || undefined}
      disabled={
        busy ||
        (!(screen === "cash-management" && cmDesk === desk) &&
          ((weekWizardActive && desk !== "weekly") ||
            (elementDirty && desk !== "elements") ||
            (externalDirty && desk !== "external") ||
            (cashDirty && desk !== "weekly") ||
            ((pdDirty || wizDirty || addLotDirty) &&
              !dirtyTargets.includes("cash-management"))))
      }
      onClick={() => goCmDesk(desk, label)}
    >
      {label}
    </button>
  );

  const navButton = (id: Screen, label: string) => (
    <button
      type="button"
      role="menuitem"
      className="menubar-item"
      aria-label={label}
      aria-current={screen === id ? "page" : undefined}
      aria-busy={menuWorking === label || undefined}
      disabled={
        busy ||
        (screen !== id &&
          ((weekWizardActive && id !== "cash-management") ||
            ((pdDirty || wizDirty || addLotDirty || cashDirty || elementDirty || externalDirty) &&
              !dirtyTargets.includes(id))))
      }
      onClick={() => {
        setOpenMenu(null);
        setMenuWorking(label);
        leaveWithoutSaving(
          () => {
            if (id === "cash-management") {
              setCmDesk("weekly");
            }
            setScreen(id);
            if (
              screen === id &&
              (id !== "cash-management" || cmDesk === "weekly")
            ) {
              setMenuWorking(null);
            }
          },
          id,
          id === "cash-management" ? "weekly" : undefined,
        );
      }}
    >
      {label}
    </button>
  );

  const menuGroup = (id: string, label: string, items: ReactNode) => (
    <div
      className={`menubar-group${openMenu === id ? " is-open" : ""}`}
      onClick={(event) => event.stopPropagation()}
    >
      <button
        type="button"
        className="menubar-trigger"
        aria-haspopup="true"
        aria-expanded={openMenu === id}
        onClick={() => setOpenMenu(openMenu === id ? null : id)}
      >
        {label}
      </button>
      {openMenu === id ? (
        <div className="menubar-dropdown" role="menu">
          {items}
        </div>
      ) : null}
    </div>
  );

  const accountCashRows = [
    ["Income", "SPAXX"],
    ["FI Roth", "SPAXX"],
    ["Speculation", "SPAXX"],
    ["Health", "FDRXX"],
    ["Car", "SPAXX"],
    ["9", "SWVXX"],
  ].map(([account, symbol]) => {
    const cents = (holdings?.lots ?? [])
      .filter(
        (lot) =>
          lot.accountName === account &&
          lot.symbol.toUpperCase() === symbol &&
          lot.remainingQuantityMinor > 0,
      )
      .reduce((sum, lot) => {
        const denom = 10 ** lot.quantityScale;
        return sum + (denom ? Math.round((lot.remainingQuantityMinor * 100) / denom) : 0);
      }, 0);
    const found = (holdings?.lots ?? []).some(
      (lot) =>
        lot.accountName === account &&
        lot.symbol.toUpperCase() === symbol &&
        lot.remainingQuantityMinor > 0,
    );
    return {
      name: account === "9" ? "Account 9" : account,
      symbol,
      cents,
      found,
    };
  });
  const cashTotalCents = accountCashRows.reduce(
    (sum, row) => sum + (row.found ? row.cents : 0),
    0,
  );

  return (
    <>
      <nav className="menubar" aria-label="Application">
        <div className="menubar-menus">
          <button
            type="button"
            className="menubar-trigger"
            aria-label="Home"
            aria-current={screen === "home" ? "page" : undefined}
            onClick={() => goHome()}
          >
            Home
          </button>
          <button
            type="button"
            className="menubar-trigger"
            aria-label="Income Plan"
            aria-current={screen === "income-plan" ? "page" : undefined}
            onClick={() => {
              setOpenMenu(null);
              leaveWithoutSaving(() => setScreen("income-plan"), "income-plan");
            }}
          >
            Income Plan
          </button>
          <button
            type="button"
            className="menubar-trigger"
            aria-label="Trends"
            aria-current={screen === "trends" ? "page" : undefined}
            onClick={() => {
              setOpenMenu(null);
              leaveWithoutSaving(() => setScreen("trends"), "trends");
            }}
          >
            Trends
          </button>
          {menuGroup(
            "cash",
            "Cash Management",
            <>
              {cmDeskButton("elements", "Element Management")}
              {cmDeskButton("cashflow", "Cashflow Manager")}
              {cmDeskButton("weekly", "System update tasks and confirmations")}
              {cmDeskButton("car", "Tax Planning")}
              {cmDeskButton("coverage", "Coverage")}
              {cmDeskButton("external", "External accounts")}
            </>,
          )}
          {menuGroup(
            "plan",
            "Plan",
            <>
              {navButton("calculator", "Calculator")}
              {navButton("dashboard", "Dashboard")}
              {navButton("cash-management", "Cash Management")}
              {navButton("shopping-cart", "Shopping Cart")}
            </>,
          )}
          {menuGroup(
            "positions",
            "Positions",
            <>
              {navButton("position-details", "Position Details")}
              {navButton("holdings", "Holdings")}
              {navButton("new-investment", "Add Position")}
              {navButton("add-lot", "Add Lot")}
            </>,
          )}
          {menuGroup(
            "data",
            "Data",
            <>
              {navButton("import", "Import")}
              {navButton("collectors", "Collectors")}
              {navButton("tickets", "Tickets")}
            </>,
          )}
          {menuGroup(
            "tools",
            "Tools",
            <>
              {navButton("collector-establish", "Reevaluate collector")}
              {navButton("components", "Components")}
              {navButton("settings", "Settings")}
            </>,
          )}
        </div>
        <div className="menubar-status">
          <PageActivityBar />
          <p className="menubar-week" aria-label="Current week">
            {formatMenuWeek(incomeWeek?.start ?? asOfDate)}
          </p>
        </div>
      </nav>
      {menuWorking ? (
        <div
          className="menu-working"
          role="status"
          aria-busy="true"
          aria-label="Menu working"
        >
          Working…
        </div>
      ) : null}
      {restarting ? (
        <div
          className="blocked unsaved-bar restart-bar"
          role="status"
          aria-busy="true"
          aria-label="Restart in progress"
        >
          <p>
            Restart in progress. Waiting for in-flight work, then closing the
            data file. Do not force-quit.
          </p>
        </div>
      ) : null}
      <main className="container" aria-label="finos">
      {screen === "home" ? (
        <>
        <div className="home-top-row">
        {summary ? (
        <section className="home-portfolio-pane" aria-label="Portfolio summary">
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
            <div className="ps-cell ps-cash" aria-label="Account cash balances">
              <dt>
                Cash
                <span className="ps-cash-total">{formatUsd(cashTotalCents, 2)}</span>
              </dt>
              <dd>
                <ul>
                  {accountCashRows.map((row) => (
                    <li key={row.name}>
                      <span className="ps-cash-name">{row.name}</span>
                      <span className="ps-cash-amt">
                        {row.found ? formatUsd(row.cents, 2) : "none"}
                      </span>
                    </li>
                  ))}
                </ul>
              </dd>
              <p className="ps-note">Stored cash lot quantity</p>
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
            <div className="ps-cell ps-coverage">
              <dt>Last Price all symbols</dt>
              <dd>
                {formatCount(summary.lastPriceCount ?? 0)} of{" "}
                {formatCount(summary.symbolCount ?? 0)}
              </dd>
              <p className="ps-note">
                Last refresh{" "}
                {lastPriceBusy
                  ? "…"
                  : summary.lastPriceRefreshedOn?.trim().slice(0, 10) || "none"}
              </p>
              <button
                type="button"
                aria-label="Refresh last prices"
                aria-busy={lastPriceBusy}
                disabled={lastPriceBusy || busy}
                onClick={() => {
                  void refreshLastPrices(true);
                }}
              >
                {lastPriceBusy ? (
                  <span
                    className="ps-refresh-count"
                    role="status"
                    aria-live="polite"
                    aria-label="Last price refresh progress"
                  >
                    {lastPriceRefreshLabel(lastPriceProgress)}
                  </span>
                ) : (
                  "Refresh last prices"
                )}
              </button>
            </div>
            <div className="ps-pair" aria-label="Average monthly income">
              <div className="ps-cell ps-income">
                <dt>
                  Average <strong>previous</strong> 12 months
                  <span className="ps-avg-income">Income</span>
                </dt>
                <dd>
                  {summary.avgMonthlyActualIncomeMinor == null
                    ? "unknown"
                    : `${formatUsd(
                        summary.avgMonthlyActualIncomeMinor,
                        summary.scale ?? 2,
                      )} (${formatUsd(
                        summary.avgMonthlyActualIncomeMinor * 12,
                        summary.scale ?? 2,
                      )})`}
                </dd>
                <p className="ps-note">Actual paid dividends, same accounts</p>
              </div>
              <div className="ps-cell ps-income">
                <dt>
                  Average Monthly Plan
                  <span className="ps-avg-income">Income</span>
                </dt>
                <dd>
                  {summary.avgMonthlyPlanIncomeMinor == null
                    ? "unknown"
                    : `${formatUsd(
                        summary.avgMonthlyPlanIncomeMinor,
                        summary.scale ?? 2,
                      )} (${formatUsd(
                        summary.avgMonthlyPlanIncomeMinor * 12,
                        summary.scale ?? 2,
                      )})`}
                </dd>
                <p className="ps-note">9, Income, FI Roth, Car — annual ÷ 12</p>
              </div>
            </div>
            <div className="ps-cell ps-coverage">
              <dt>Dividend Managed positions</dt>
              <dd>
                {formatCount(summary.declarationCount ?? 0)} of{" "}
                {formatCount(summary.declarationCollectorCount ?? 0)}
              </dd>
              <p className="ps-note">
                Last update{" "}
                {declarationBusy
                  ? "…"
                  : summary.declarationRefreshedOn?.trim().slice(0, 10) ||
                    "none"}
              </p>
              <button
                type="button"
                aria-label="Refresh declarations"
                aria-busy={declarationBusy}
                disabled={declarationBusy || busy}
                onClick={() => {
                  void refreshDeclarations(true);
                }}
              >
                {declarationBusy ? (
                  <span
                    className="ps-refresh-count"
                    role="status"
                    aria-live="polite"
                    aria-label="Declaration refresh progress"
                  >
                    {declarationRefreshLabel(declarationProgress)}
                  </span>
                ) : (
                  "Refresh declarations"
                )}
              </button>
            </div>
            <div className="ps-cell ps-coverage">
              <dt>Open tickets</dt>
              <dd>{formatCount(summary.openTicketCount ?? 0)}</dd>
              <p className="ps-note">
                Stay until filed — not today&apos;s miss count
              </p>
              <button
                type="button"
                aria-label="Work Tickets"
                onClick={() => {
                  setTicketFocusSymbol("");
                  setScreen("tickets");
                }}
              >
                Work Tickets
              </button>
            </div>
            <div className="ps-cell ps-date">
              <dt>Income through</dt>
              <dd>
                {incomeThroughOn ? (
                  <button
                    type="button"
                    className="ps-date-link"
                    aria-label="Income through transactions"
                    onClick={() => setIncomeTxOpen(true)}
                  >
                    {incomeThroughOn}
                  </button>
                ) : (
                  "none"
                )}
                {incomeWeekTotal == null ? null : (
                  <span className="ps-week-declared">
                    {formatUsd(incomeWeekTotal, 2)}
                  </span>
                )}
              </dd>
              <p className="ps-note">
                {incomeWeekDeclared == null ? "This week imported" : "This week declared"}
              </p>
            </div>
          </dl>
        </section>
        ) : (
          <p role="status">Loading portfolio summary…</p>
        )}
        <div className="home-top-right">
        <HomeDividendPlan plan={dividendPlan} />
        <AccountCashFlow
          weeks={accountValues?.weeks ?? trends?.weeks}
          asOf={asOfDate}
        />
        </div>
        </div>
        {incomeTxOpen ? (
          <div
            className="home-av-dialog-backdrop"
            onClick={() => setIncomeTxOpen(false)}
          >
            <div
              role="dialog"
              aria-modal="true"
              aria-label="Income through transactions"
              className="home-av-dialog income-tx-dialog"
              onClick={(event) => event.stopPropagation()}
            >
              <header>
                <h3>
                  Income through {incomeThroughOn || "none"}
                </h3>
                <button
                  type="button"
                  aria-label="Exit income through transactions"
                  onClick={() => setIncomeTxOpen(false)}
                >
                  Exit
                </button>
              </header>
              <div className="income-tx-period-bar">
                <label>
                  Period
                  <select
                    aria-label="Income transaction period"
                    value={incomeTxPeriod}
                    onChange={(e) => {
                      const next = e.target.value as IncomeTxPeriod;
                      setIncomeTxPeriod(next);
                      if (next === "custom" && (!incomeTxCustomStart || !incomeTxCustomEnd)) {
                        const month = incomeTxRange("month", incomeTxToday, "", "");
                        setIncomeTxCustomStart(month?.startOn ?? incomeTxToday);
                        setIncomeTxCustomEnd(month?.endOn ?? incomeTxToday);
                      }
                    }}
                  >
                    {INCOME_TX_PERIOD_OPTIONS.map((opt) => (
                      <option key={opt.value} value={opt.value}>
                        {opt.label}
                      </option>
                    ))}
                  </select>
                </label>
                <label>
                  Account
                  <select
                    aria-label="Income transaction account"
                    value={incomeTxAccountId}
                    onChange={(e) => setIncomeTxAccountId(e.target.value)}
                  >
                    <option value="">All accounts</option>
                    {incomeTxAccountOptions.map((acct) => (
                      <option key={acct.accountId} value={acct.accountId}>
                        {acct.name}
                      </option>
                    ))}
                  </select>
                </label>
                {incomeTxPeriod === "custom" ? (
                  <>
                    <label>
                      From
                      <input
                        type="date"
                        aria-label="Income transaction start"
                        value={incomeTxCustomStart}
                        onChange={(e) => setIncomeTxCustomStart(e.target.value)}
                      />
                    </label>
                    <label>
                      To
                      <input
                        type="date"
                        aria-label="Income transaction end"
                        value={incomeTxCustomEnd}
                        onChange={(e) => setIncomeTxCustomEnd(e.target.value)}
                      />
                    </label>
                  </>
                ) : null}
                <p className="income-tx-range-note">
                  {incomeTxWindow
                    ? `${incomeTxWindow.startOn} to ${incomeTxWindow.endOn}`
                    : "Choose a start and end date"}
                </p>
              </div>
              {incomeThroughRows.length === 0 ? (
                <p className="home-av-empty">No paid dividends in this range.</p>
              ) : (
                <div className="income-tx-scroll">
                  <table className="income-tx-table">
                    <thead>
                      <tr>
                        <th>Date</th>
                        <th>Acct</th>
                        <th>Symbol</th>
                        <th className="numeric">Amount</th>
                      </tr>
                    </thead>
                    <tbody>
                      {incomeThroughRows.map((row) => (
                        <tr key={row.id}>
                          <td>{row.date}</td>
                          <td>{row.account}</td>
                          <td>{row.symbol}</td>
                          <td className="numeric">
                            {formatUsd(row.amountMinor, row.scale)}
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              )}
            </div>
          </div>
        ) : null}
        <HomeAccountCharts values={accountValues} />
        </>
      ) : null}
      {incomeExportPreview ? (
        <div
          className="home-av-dialog-backdrop"
          onClick={() => setIncomeExportPreview(null)}
        >
          <div
            role="dialog"
            aria-modal="true"
            aria-label="Income Plan export preview"
            className="home-av-dialog income-export-preview-dialog"
            onClick={(event) => event.stopPropagation()}
          >
            <header>
              <h3>Print / Export preview</h3>
              <button
                type="button"
                aria-label="Cancel Income Plan export"
                onClick={() => setIncomeExportPreview(null)}
              >
                Cancel
              </button>
            </header>
            <iframe
              className="income-export-preview-frame"
              title="Income Plan export preview"
              srcDoc={styleIncomePrintHtml(incomeExportPreview.printHtml)}
            />
            <p>
              Review the {incomeExportPreview.cover.pattern === "B" ? "weekly report" : "weekly grid"}{" "}
              then choose print, PDF, or Excel.
            </p>
            <div
              className="income-export-preview-actions"
              aria-label="Confirm Income Plan export"
            >
              <button
                type="button"
                aria-label="Print to page"
                disabled={busy}
                onClick={() => void commitIncomeExport("print")}
              >
                Print
              </button>
              <button
                type="button"
                aria-label="Save PDF"
                disabled={busy}
                onClick={() => void commitIncomeExport("pdf")}
              >
                Save PDF
              </button>
              <button
                type="button"
                aria-label="Export Excel"
                disabled={busy}
                onClick={() => void commitIncomeExport("excel")}
              >
                Export Excel
              </button>
              <button
                type="button"
                aria-label="Cancel Income Plan export"
                disabled={busy}
                onClick={() => setIncomeExportPreview(null)}
              >
                Cancel
              </button>
            </div>
          </div>
        </div>
      ) : null}
      {snapshotConfirm ? (
        <div
          className="home-av-dialog-backdrop"
          onClick={() => {
            if (!busy) {
              setSnapshotConfirm(false);
            }
          }}
        >
          <div
            role="alertdialog"
            aria-modal="true"
            aria-label="Confirm save data snapshot"
            className="home-av-dialog snapshot-confirm-dialog"
            onClick={(event) => event.stopPropagation()}
          >
            <header>
              <h3>Save data snapshot</h3>
              <button
                type="button"
                aria-label="Cancel save data snapshot"
                disabled={busy}
                onClick={() => setSnapshotConfirm(false)}
              >
                Cancel
              </button>
            </header>
            <p>
              Save a data snapshot under raw-data for today? Workbooks and a copy of
              local.sqlite are written from the live database. Today’s folder is
              replaced if it already exists.
            </p>
            <div className="buttons">
              <button
                type="button"
                aria-label="Confirm save data snapshot"
                disabled={busy}
                onClick={() => {
                  setSnapshotConfirm(false);
                  void runDataSnapshot();
                }}
              >
                Save snapshot
              </button>
              <button
                type="button"
                aria-label="Cancel save data snapshot"
                disabled={busy}
                onClick={() => setSnapshotConfirm(false)}
              >
                Cancel
              </button>
            </div>
          </div>
        </div>
      ) : null}
      {pdDirty || wizDirty || addLotDirty || cashDirty || elementDirty || externalDirty ? (
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
                  onClick={() => {
                    if (id === "cash-management") {
                      setCmDesk(
                        externalDirty ? "external" : elementDirty ? "elements" : "weekly",
                      );
                    }
                    setScreen(id);
                  }}
                >
                  Open{" "}
                  {id === "position-details"
                    ? "Position Details"
                    : id === "new-investment"
                      ? "Add Position"
                      : id === "cash-management"
                        ? "Cash Management"
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
      {(screen !== "home" ||
        /snapshot|Saved |Restart/i.test(actionMessage ?? "")) &&
      actionMessage &&
      !restarting ? (
        <p>{actionMessage}</p>
      ) : null}
      {writesBlocked ? (
        <p className="blocked">Ordinary writes are blocked until restore or explicit review.</p>
      ) : null}

      {screen === "income-plan" ? (
        <IncomePlanScreen
          incomeWeekLoading={incomeWeekLoading}
          incomeWeek={incomeWeek}
          incomePattern={incomePattern}
          setIncomePattern={setIncomePattern}
          incomeGrid={incomeGrid}
          asOfDate={asOfDate}
          incomeHistWeeks={incomeHistWeeks}
          setIncomeHistWeeks={setIncomeHistWeeks}
          incomeFutWeeks={incomeFutWeeks}
          setIncomeFutWeeks={setIncomeFutWeeks}
          drillAccounts={drillAccounts}
          setDrillAccounts={setDrillAccounts}
          withIncomeLoading={withIncomeLoading}
          loadIncomeGrid={loadIncomeGrid}
          loadIncomeWeek={loadIncomeWeek}
          requestIncomeExport={requestIncomeExport}
          incomeExportLoading={incomeExportLoading}
          incomeWeekNav={incomeWeekNav}
          setIncomeWeekNav={setIncomeWeekNav}
          openPositionHub={openPositionHub}
          dividendPerf={dividendPerf}
          perfRange={perfRange}
          onPerfRangeChange={(next) => {
            setPerfRange(next);
            void (async () => {
              const r = await client.executeQuery("DividendPerformanceGet", {
                asOfDate,
                range: next,
              });
              if (r.ok && r.bodyJson) {
                try {
                  setDividendPerf(JSON.parse(r.bodyJson) as DividendPerformanceGet);
                } catch {
                  setDividendPerf(null);
                }
              }
            })();
          }}
        />
      ) : null}

      {screen === "calculator" ? (
        <section aria-label="Calculator" className="calculator-page">
          <h2>Calculator</h2>
          <CalculatorReturnSheet
            rows={positionMaster?.rows ?? null}
            history={declHistory}
            accountsBySymbol={Object.fromEntries(
              (positionDetails?.positions ?? []).reduce((map, line) => {
                const prior = map.get(line.symbol);
                if (!prior) {
                  map.set(line.symbol, line.accountName);
                } else if (!prior.split(", ").includes(line.accountName)) {
                  map.set(line.symbol, `${prior}, ${line.accountName}`);
                }
                return map;
              }, new Map<string, string>()),
            )}
            cadence={histCadence}
            onCadenceChange={setHistCadence}
            period={histPeriod}
            startOn={
              historyBounds(histPeriod, asOfDate, histStartOn, histEndOn).startOn
            }
            endOn={historyBounds(histPeriod, asOfDate, histStartOn, histEndOn).endOn}
            onPeriodChange={(preset) => {
              setHistPeriod(preset);
              const bounds = historyBounds(
                preset,
                asOfDate,
                histStartOn,
                histEndOn,
              );
              if (preset !== "custom") {
                setHistStartOn(bounds.startOn);
                setHistEndOn(bounds.endOn);
              }
              void loadDeclHistory(
                asOfDate,
                preset,
                bounds.startOn,
                bounds.endOn,
              );
            }}
            onStartOnChange={(iso) => {
              setHistPeriod("custom");
              setHistStartOn(iso);
              const end = histEndOn || asOfDate;
              setHistEndOn(end);
              void loadDeclHistory(asOfDate, "custom", iso, end);
            }}
            onEndOnChange={(iso) => {
              setHistPeriod("custom");
              const start =
                histStartOn ||
                historyBounds("60", asOfDate, "", "").startOn;
              setHistStartOn(start);
              setHistEndOn(iso);
              void loadDeclHistory(asOfDate, "custom", start, iso);
            }}
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
              aria-busy={lastPriceBusy}
              disabled={lastPriceBusy || busy}
              onClick={() => void refreshLastPrices(true)}
            >
              {lastPriceBusy
                ? lastPriceRefreshLabel(lastPriceProgress)
                : "Refresh last prices"}
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
                  className={pdDirty ? "is-unsaved" : undefined}
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
                          <option value="">Choose owner tier</option>
                          {RISK_TIERS.map((tier) => (
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
                          No suggested tier. Choose Foundation, Core, or Risk On.
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
              <WorkTicketQueue
                tickets={workTickets}
                filterSymbol={investment.symbol}
                retryingTicketId={retryingTicket?.ticketId}
                retryingSymbol={retryingTicket?.symbol}
                pendingTicketId={ticketDecision?.ticketId}
                pendingAction={ticketDecision?.action}
                onRetry={(t) => void resolveTicketRetry(t as WorkTicketRecord)}
                onRecreateAdapter={(t) =>
                  openRecreateAdapter(t as WorkTicketRecord)
                }
                onExcept={(t) =>
                  resolveConfirmTicket(t as WorkTicketRecord, "positive")
                }
                onReject={(t) =>
                  resolveConfirmTicket(t as WorkTicketRecord, "reject")
                }
                onEnterAmount={(t, amount) =>
                  void resolveEnterDeclaredAmount(t as WorkTicketRecord, amount)
                }
                onFile={(t) => void fileWorkTicket(t as WorkTicketRecord)}
              />
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
                <p>
                  Owner facts edit in this table. Risk is mandatory: Foundation, Core,
                  or Risk On. Frequency is required. Save or Cancel.
                </p>
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
                        <td>
                          <input
                            aria-label="Position name"
                            value={pdDraft.name}
                            onChange={(e) => patchDraft({ name: e.target.value })}
                            disabled={busy || writesBlocked}
                          />
                        </td>
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
                            {RISK_TIERS.map((tier) => (
                              <option key={tier} value={tier}>
                                {tier}
                              </option>
                            ))}
                          </select>
                        </td>
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
                                ? "unknown"
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
                          {RISK_TIERS.map((tier) => (
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
                      <th scope="row">Strategy characteristics</th>
                      <td aria-label="Strategy characteristics">
                        {strategyCharacteristics(
                          pdDraft.lookthrough,
                          pdDraft.underlying,
                        ) || "—"}
                      </td>
                      <td>
                        {strategyCharacteristics(
                          pdDraft.lookthrough,
                          pdDraft.underlying,
                        )
                          ? "researched"
                          : "unknown"}
                      </td>
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
                        <p aria-label="Template Dividend">
                          Template Dividend:{" "}
                          {investment.template?.sourceUrl?.trim()
                            ? investment.template.sourceUrl
                            : "—"}
                        </p>
                        <p aria-label="Template ROC">
                          Template ROC:{" "}
                          {investment.template?.rocSourceUrl?.trim()
                            ? investment.template.rocSourceUrl
                            : "—"}
                        </p>
                        <p aria-label="Template content hash">
                          Content hash:{" "}
                          {investment.template?.lastContentHash?.trim()
                            ? investment.template.lastContentHash
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
                  short = `Adapter returned ${formatCount(paid)} of ${formatCount(DECLARATION_LOOKBACK_TARGET)} required paid declarations. Confirm inception Yes/No on Add Position — do not defer to Settings.`;
                }
                return (
                  <>
                    <h4>Latest paid ({needLabel})</h4>
                    <p>
                      Paid $/share from the issuer adapter. Period is the payable date,
                      not the declaration date. Blank stays unknown, not $0.
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
                              <th scope="col">Pay date</th>
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
                  className={periodDirty ? "is-unsaved" : undefined}
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
          <p>
            Charts from saved weeks and declared income. Enter the week on Cash
            Management.
          </p>
          {trendsCapture &&
          (!trendsCapture.exists || trendsCapture.firstUnpopulatedStart) ? (
            <p role="status">
              <button
                type="button"
                aria-label="Finish this week on Cash Management"
                onClick={() =>
                  leaveWithoutSaving(
                    () => {
                      setCmDesk("weekly");
                      setScreen("cash-management");
                    },
                    "cash-management",
                    "weekly",
                  )
                }
              >
                Finish this week on Cash Management
              </button>
            </p>
          ) : null}
          <TrendsChartsPanel
            key={trendsChartEpoch}
            weeks={trends?.weeks}
            points={trends?.points}
            dividendPerf={trendsDividendPerf}
            onGraphPeriodChange={(period) => {
              trendsGraphPeriodRef.current = period;
              void (async () => {
                const r = await client.executeQuery("DividendPerformanceGet", {
                  asOfDate,
                  range: period,
                });
                if (r.ok && r.bodyJson) {
                  try {
                    setTrendsDividendPerf(
                      JSON.parse(r.bodyJson) as DividendPerformanceGet,
                    );
                    return;
                  } catch {
                    /* fall through */
                  }
                }
                if (period === "6m" || period === "12m") {
                  const fallback = await client.executeQuery(
                    "DividendPerformanceGet",
                    { asOfDate, range: "all" },
                  );
                  if (fallback.ok && fallback.bodyJson) {
                    try {
                      setTrendsDividendPerf(
                        JSON.parse(fallback.bodyJson) as DividendPerformanceGet,
                      );
                      return;
                    } catch {
                      /* fall through */
                    }
                  }
                }
                setTrendsDividendPerf(null);
              })();
            }}
            note={trends?.note}
            error={trendsError}
            missingRequired={trends?.missingRequired}
            accountValues={accountValues}
            risk={accountValues?.risk ?? null}
            asOf={accountValues?.asOf ?? asOfDate}
          />
        </section>
      ) : null}

      {screen === "shopping-cart" ? (
        <ShoppingCartScreen
          client={client}
          accounts={accounts}
          holdings={holdings}
          calculator={calculator}
          researched={addLotSecurityOptions}
          positionMaster={positionMaster}
          asOfDate={asOfDate}
          busy={busy}
          writesBlocked={writesBlocked}
          onMessage={setActionMessage}
          onBusy={setBusy}
          onDirtyChange={setCartDirty}
          onOpenBuyLot={(prefill) => {
            leaveWithoutSaving(() => {
              pendingCartBuyRef.current = { scenarioId: prefill.scenarioId };
              setAddLotAccountId(prefill.accountId);
              setAddLotSecurityId(prefill.securityId);
              const row = securities.find((s) => s.securityId === prefill.securityId);
              setAddLotQuery(
                row
                  ? `${row.symbol}${row.name ? ` — ${row.name}` : ""}`
                  : prefill.symbol,
              );
              setAddLotQty(String(prefill.qtyWhole));
              setAddLotCost((prefill.lastMinor / 100).toFixed(2));
              setAddLotOpenedOn(prefill.openedOn);
              setScreen("add-lot");
            }, "add-lot");
          }}
        />
      ) : null}

      {screen === "cash-management" ? (
        <section aria-label="Cash Management">
          {cmDesk === "elements" ? (
            <h2 aria-label="Element Management">Element Management</h2>
          ) : cmDesk === "cashflow" ? (
            <h2 aria-label="Cashflow Manager">Cashflow Manager</h2>
          ) : cmDesk === "car" ? (
            <h2 aria-label="Tax Planning">Tax Planning</h2>
          ) : cmDesk === "external" ? (
            <h2 aria-label="External accounts">External accounts</h2>
          ) : cmDesk === "coverage" ? (
            <h2 aria-label="Coverage">
              {coveragePeriod === "month"
                ? "Monthly comparison"
                : coveragePeriod === "year"
                  ? "Annual comparison"
                  : "Weekly comparison"}
            </h2>
          ) : (
            <h2 aria-label="System update tasks and confirmations">System update tasks and confirmations</h2>
          )}
          {cmDesk === "weekly" ? (
            <PlanHorizonPrompt
              asOfDate={asOfDate}
              disabled={busy || writesBlocked}
            />
          ) : null}
          {cmDesk === "cashflow" || cmDesk === "external" ? null : (
          <p>
            {cmDesk === "elements"
              ? "Deposits and Withdrawals for the selected managed account."
              : cmDesk === "car"
                ? "Household YTD and projected income by tax type, MAGI threshold, the Car table, and Cash YTD."
                : cmDesk === "coverage"
                  ? "Planned dividend income versus planned withdrawals for the next 12 months. Week is that year ÷ 52. Month is that year ÷ 12. The tables under the comparison list each amount × periods."
                  : "Enter this Sat–Fri week, then declare SSA, distributions, or withdrawals."}
          </p>
          )}
          {cmDesk === "coverage" ? (
            <CashCoveragePanel
              coverage={cashCoverage}
              period={coveragePeriod}
              busy={coverageBusy}
              onPeriod={(next) => {
                coveragePeriodRef.current = next;
                setCoveragePeriod(next);
                setCoverageBusy(true);
                void (async () => {
                  try {
                    const result = await client.executeQuery("CashCoverageGet", {
                      asOfDate: asOfDate,
                      period: next,
                    });
                    if (result.ok && result.bodyJson) {
                      try {
                        setCashCoverage(JSON.parse(result.bodyJson) as CashCoverageGet);
                      } catch {
                        setCashCoverage(null);
                      }
                    }
                  } finally {
                    setCoverageBusy(false);
                  }
                })();
              }}
            />
          ) : null}
          {cmDesk === "external" ? (
            <ExternalRegister
              resetToken={externalReset}
              onDirtyChange={setExternalDirty}
            />
          ) : null}
          {cmDesk === "coverage" || cmDesk === "external" ? null : (
          <CashManagementPanel
            desk={cmDesk}
            week={cashWeek}
            month={cashMonth}
            reminders={cashReminders}
            magi={cashMagi}
            accounts={accounts}
            busy={busy}
            distributions={trends?.distributions as never}
            taxMonitor={trends?.taxMonitor as never}
            carRocPlan={carRocPlan}
            taxPlanning={taxPlanning}
            weekJustSavedAt={cashWeekSavedAt}
            weekDesk={{
              weeks: trends?.weeks,
              points: trends?.points,
              dividendPerf: dividendPerf ?? trendsDividendPerf,
              overview: trends?.overview as never,
              incomePlanWeek: incomeWeek
                ? {
                    start: incomeWeek.start,
                    end: incomeWeek.end,
                    plannedMinor: incomePlanWeekPlanMinor(incomeWeek),
                    reportedMinor: null,
                  }
                : null,
              openWeek: trendsCapture
                ? {
                    start: trendsCapture.periodStart,
                    end: trendsCapture.periodEnd,
                    plannedMinor:
                      incomeWeek &&
                      (incomeWeek.start === trendsCapture.periodStart ||
                        incomeWeek.end === trendsCapture.periodEnd)
                        ? incomePlanWeekPlanMinor(incomeWeek)
                        : (trendsCapture.plannedWeeklyIncomeMinor ?? null),
                    reportedMinor: trendsCapture.reportedWeeklyIncomeMinor || null,
                  }
                : null,
            }}
            weekAhead={
              <WeekAheadPanel
                week={weekAhead}
                busy={busy}
                pendingId={weekAheadPending}
                onConfirm={(occurrenceId) => {
                  void (async () => {
                    setWeekAheadPending(occurrenceId);
                    setBusy(true);
                    try {
                      const r = await client.executeCommand("WeekAheadConfirm", {
                        occurrenceId,
                      });
                      if (!r.ok) {
                        setActionMessage(
                          `Week ahead confirm failed: ${r.errorCode ?? "error"}`,
                        );
                        return;
                      }
                      const asOf = cashWeek?.periodEnd || asOfDate;
                      const posted = r.bodyJson
                        ? (JSON.parse(r.bodyJson) as { activityType?: string })
                        : {};
                      const [ahead, week, rem, registerPack, magi] = await Promise.all([
                        client.executeQuery("WeekAheadGet", { asOfDate: asOf }),
                        client.executeQuery("CashManagementWeekGet", {
                          asOfDate: asOf,
                        }),
                        client.executeQuery("CashManagementRemindersGet", {
                          asOfDate: asOf,
                        }),
                        fetchCashNav(client,
                          asOf,
                          registerBookRef.current,
                          registerPeriodRef.current,
                          ytdViewRef.current,
                        ),
                        magiQualifyingType(posted.activityType)
                          ? client.executeQuery("MagiProjectionGet")
                          : Promise.resolve(null),
                      ]);
                      if (ahead.ok && ahead.bodyJson) {
                        setWeekAhead(JSON.parse(ahead.bodyJson) as WeekAheadGet);
                      }
                      if (week.ok && week.bodyJson) {
                        setCashWeek(
                          JSON.parse(week.bodyJson) as CashManagementWeekGet,
                        );
                      }
                      if (rem.ok && rem.bodyJson) {
                        setCashReminders(
                          JSON.parse(rem.bodyJson) as CashManagementRemindersGet,
                        );
                      }
                      if (registerPack.register) {
                        setCashRegister(registerPack.register);
                      }
                      if (registerPack.elements) {
                        setCashElements(registerPack.elements);
                      }
                      if (registerPack.ytd) {
                        setCashYtd(registerPack.ytd);
                      }
                      if (magi && magi.ok && magi.bodyJson) {
                        setCashMagi(JSON.parse(magi.bodyJson) as MagiProjection);
                      }
                    } finally {
                      setWeekAheadPending(null);
                      setBusy(false);
                    }
                  })();
                }}
                onOpenEditor={(row) => {
                  leaveWithoutSaving(() => {
                  void (async () => {
                    registerBookRef.current = row.account;
                    setRegisterBook(row.account);
                    setCmDesk("elements");
                    const asOf = cashWeek?.periodEnd || asOfDate;
                    const els = await fetchElementList(client, asOf);
                    if (els) setCashElements(els);
                    setEditorElement(
                      els?.items.find((e) => e.elementId === row.elementId) ?? null,
                    );
                    setEditorOccurrenceId(row.occurrenceId);
                    setElementEditorOpen(true);
                  })();
                  }, "cash-management", "elements");
                }}
                onDefer={(occurrenceId) => {
                  void (async () => {
                    const r = await client.executeCommand("WeekAheadDefer", {
                      occurrenceId,
                    });
                    if (!r.ok) {
                      setActionMessage(
                        `Week ahead defer failed: ${r.errorCode ?? "error"}`,
                      );
                      return;
                    }
                    const asOf = cashWeek?.periodEnd || asOfDate;
                    const ahead = await client.executeQuery("WeekAheadGet", {
                      asOfDate: asOf,
                    });
                    if (ahead.ok && ahead.bodyJson) {
                      setWeekAhead(JSON.parse(ahead.bodyJson) as WeekAheadGet);
                    }
                  })();
                }}
              />
            }
            cashElements={
              <CashElementsCatalog
                catalog={cashElements}
                book={catalogBook}
                editorOpen={elementEditorOpen}
                editorAccount={
                  editorElement?.account
                  || (catalogBook === "all" ? registerBook : catalogBook)
                }
                editorElement={editorElement}
                editorOccurrenceId={editorOccurrenceId}
                editorOccurrences={
                  (editorElement?.upcoming ?? editorElement?.exceptions ?? []).map(
                    (row) => ({
                      occurrenceId: row.occurrenceId,
                      occurredOn: row.occurredOn,
                      amountMinor: row.amountMinor,
                      isException: row.isException,
                      isCancelled: row.isCancelled,
                    }),
                  )
                }
                busy={busy}
                onBook={setCatalogBook}
                onAddElement={() => {
                  if (!catalogBook || catalogBook === "all") return;
                  const book = catalogBook;
                  registerBookRef.current = book;
                  setRegisterBook(book);
                  setEditorElement(null);
                  setEditorOccurrenceId(null);
                  setElementDirty(false);
                  setElementEditorOpen(true);
                }}
                onOpenElement={(element) => {
                  registerBookRef.current = element.account;
                  setRegisterBook(element.account);
                  setCatalogBook(element.account);
                  setEditorElement(element);
                  setEditorOccurrenceId(null);
                  setElementDirty(false);
                  setElementEditorOpen(true);
                  setBusy(true);
                  const seq = ++catalogFetchSeq.current;
                  void (async () => {
                    try {
                      const els = await fetchElementList(
                        client,
                        cashWeek?.periodEnd || asOfDate,
                      );
                      if (seq !== catalogFetchSeq.current) return;
                      if (els) {
                        setCashElements(els);
                        const updated = els.items.find(
                          (e) => e.elementId === element.elementId,
                        );
                        if (updated) setEditorElement(updated);
                      }
                    } finally {
                      if (seq === catalogFetchSeq.current) setBusy(false);
                    }
                  })();
                }}
                onCloseEditor={() => {
                  setElementEditorOpen(false);
                  setEditorElement(null);
                  setEditorOccurrenceId(null);
                  setElementDirty(false);
                  const seq = ++catalogFetchSeq.current;
                  void fetchElementList(
                    client,
                    cashWeek?.periodEnd || asOfDate,
                  ).then((els) => {
                    if (seq !== catalogFetchSeq.current) return;
                    if (els) setCashElements(els);
                  });
                }}
                onDirtyChange={setElementDirty}
                asOfDate={asOfDate}
                onSaveElement={async (body) => {
                  setBusy(true);
                  const seq = ++catalogFetchSeq.current;
                  try {
                    const r = await client.executeCommand("CashElementSave", {
                      ...body,
                      asOfDate: cashWeek?.periodEnd || asOfDate,
                    });
                    if (!r.ok) {
                      setActionMessage(
                        `Element save failed: ${r.errorCode ?? "error"}`,
                      );
                      return false;
                    }
                    const els = await fetchElementList(
                      client,
                      cashWeek?.periodEnd || asOfDate,
                    );
                    if (seq !== catalogFetchSeq.current) return true;
                    if (els) setCashElements(els);
                    const saved = r.bodyJson
                      ? (JSON.parse(r.bodyJson) as {
                          element?: { elementId?: string };
                        })
                      : {};
                    const id = body.elementId || saved.element?.elementId;
                    const updated = els?.items.find((e) => e.elementId === id);
                    if (updated) setEditorElement(updated);
                    setElementDirty(false);
                    setActionMessage("Element saved.");
                    return true;
                  } finally {
                    setBusy(false);
                  }
                }}
                onDeleteSeries={(elementId) => {
                  setBusy(true);
                  void (async () => {
                    try {
                      const r = await client.executeCommand("CashElementDelete", {
                        elementId,
                      });
                      if (!r.ok) {
                        setActionMessage(
                          `Element delete failed: ${r.errorCode ?? "error"}`,
                        );
                        return;
                      }
                      const els = await fetchElementList(
                        client,
                        cashWeek?.periodEnd || asOfDate,
                      );
                      if (els) setCashElements(els);
                      setElementDirty(false);
                      setElementEditorOpen(false);
                    } finally {
                      setBusy(false);
                    }
                  })();
                }}
                onSaveExceptions={async (body) => {
                  setBusy(true);
                  try {
                    const r = await client.executeCommand(
                      "PlannedOccurrenceSave",
                      {
                        elementId: body.elementId,
                        occurrenceId: body.occurrenceId,
                        asOfDate: cashWeek?.periodEnd || asOfDate,
                        cancel: body.cancel,
                        occurredOn: body.occurredOn,
                        amountMinor: body.amountMinor,
                      },
                    );
                    if (!r.ok) {
                      setActionMessage(
                        `Exception save failed: ${r.errorCode ?? "error"}`,
                      );
                      return false;
                    }
                    const els = await fetchElementList(
                      client,
                      cashWeek?.periodEnd || asOfDate,
                    );
                    if (els) {
                      setCashElements(els);
                      const updated = els.items.find(
                        (e) => e.elementId === body.elementId,
                      );
                      if (updated) setEditorElement(updated);
                    }
                    setElementDirty(false);
                    setActionMessage("Exceptions saved.");
                    return true;
                  } finally {
                    setBusy(false);
                  }
                }}
              />
            }
            cashRegister={
              <CashRegisterPanel
                register={cashRegister}
                book={registerBook}
                period={registerPeriod}
                view={registerView}
                asOfDate={cashWeek?.periodEnd || asOfDate}
                weeks={accountValues?.weeks ?? trends?.weeks}
                busy={busy}
                loading={registerBusy}
                onBook={(book) => {
                  const seq = ++registerFetchSeq.current;
                  registerBookRef.current = book;
                  setRegisterBook(book);
                  setRegisterBusy(true);
                  void (async () => {
                    try {
                      const pack = await fetchCashNav(client,
                        cashWeek?.periodEnd || asOfDate,
                        book,
                        registerPeriodRef.current,
                        ytdViewRef.current,
                      );
                      if (seq !== registerFetchSeq.current) {
                        return;
                      }
                      if (pack.register) setCashRegister(pack.register);
                      if (pack.elements) setCashElements(pack.elements);
                      if (pack.ytd) setCashYtd(pack.ytd);
                    } finally {
                      if (seq === registerFetchSeq.current) {
                        setRegisterBusy(false);
                      }
                    }
                  })();
                }}
                onPeriod={(period) => {
                  const seq = ++registerFetchSeq.current;
                  registerPeriodRef.current = period;
                  setRegisterPeriod(period);
                  setRegisterBusy(true);
                  void (async () => {
                    try {
                      const pack = await fetchCashNav(client,
                        cashWeek?.periodEnd || asOfDate,
                        registerBookRef.current,
                        period,
                        ytdViewRef.current,
                      );
                      if (seq !== registerFetchSeq.current) {
                        return;
                      }
                      if (pack.register) setCashRegister(pack.register);
                      if (pack.elements) setCashElements(pack.elements);
                      if (pack.ytd) setCashYtd(pack.ytd);
                    } finally {
                      if (seq === registerFetchSeq.current) {
                        setRegisterBusy(false);
                      }
                    }
                  })();
                }}
                onView={setRegisterView}
                onLoadMonth={loadRegisterMonth}
                onManageElements={() => {
                  setCatalogBook(registerBookRef.current);
                  goCmDesk("elements", "Element Management");
                }}
                onAddElement={() => {
                  setEditorElement(null);
                  setEditorOccurrenceId(null);
                  setElementDirty(false);
                  setElementEditorOpen(true);
                }}
              />
            }
            cashYtd={
              <CashYtdPanel
                ytd={cashYtd}
                view={ytdView}
                busy={busy}
                onView={(view) => {
                  ytdViewRef.current = view;
                  setYtdView(view);
                  void (async () => {
                    const pack = await fetchCashNav(client,
                      cashWeek?.periodEnd || asOfDate,
                      registerBookRef.current,
                      registerPeriodRef.current,
                      view,
                    );
                    if (pack.register) setCashRegister(pack.register);
                    if (pack.elements) setCashElements(pack.elements);
                    if (pack.ytd) setCashYtd(pack.ytd);
                  })();
                }}
              />
            }
            onDirtyChange={setCashDirty}
            onReload={(d) => {
              void (async () => {
                const [r, rem, mo, roc, tax, ahead, registerPack] = await Promise.all([
                  client.executeQuery("CashManagementWeekGet", {
                    asOfDate: d,
                  }),
                  client.executeQuery("CashManagementRemindersGet", {
                    asOfDate: d,
                  }),
                  client.executeQuery("CashManagementMonthGet", {
                    asOfDate: d,
                  }),
                  client.executeQuery("CarRocPlanGet", { asOfDate: d }),
                  client.executeQuery("TaxPlanningGet", { asOfDate: d }),
                  client.executeQuery("WeekAheadGet", { asOfDate: d }),
                  fetchCashNav(client,
                    d,
                    registerBookRef.current,
                    registerPeriodRef.current,
                    ytdViewRef.current,
                  ),
                ]);
                if (r.ok && r.bodyJson) {
                  setCashWeek(JSON.parse(r.bodyJson) as CashManagementWeekGet);
                }
                if (rem.ok && rem.bodyJson) {
                  setCashReminders(
                    JSON.parse(rem.bodyJson) as CashManagementRemindersGet,
                  );
                }
                if (mo.ok && mo.bodyJson) {
                  setCashMonth(
                    JSON.parse(mo.bodyJson) as CashManagementMonthGet,
                  );
                }
                if (roc.ok && roc.bodyJson) {
                  setCarRocPlan(JSON.parse(roc.bodyJson) as CarRocPlanGet);
                }
                if (tax.ok && tax.bodyJson) {
                  setTaxPlanning(JSON.parse(tax.bodyJson) as TaxPlanningGet);
                }
                if (ahead.ok && ahead.bodyJson) {
                  setWeekAhead(JSON.parse(ahead.bodyJson) as WeekAheadGet);
                }
                if (registerPack.register) {
                  setCashRegister(registerPack.register);
                }
                if (registerPack.elements) {
                  setCashElements(registerPack.elements);
                }
                if (registerPack.ytd) {
                  setCashYtd(registerPack.ytd);
                }
              })();
            }}
            onSave={async (body) => {
              const r = await client.executeCommand("CashDistributionPost", body);
              if (!r.ok) {
                setActionMessage(
                  `Cash distribution failed: ${r.errorCode ?? "error"}`,
                );
                return false;
              }
              const asOf = (body.occurredOn as string) || asOfDate;
              const [week, rem, mo, registerPack, magi] = await Promise.all([
                client.executeQuery("CashManagementWeekGet", {
                  asOfDate: asOf,
                }),
                client.executeQuery("CashManagementRemindersGet", {
                  asOfDate: asOf,
                }),
                client.executeQuery("CashManagementMonthGet", {
                  asOfDate: asOf,
                }),
                fetchCashNav(client,
                  asOf,
                  registerBookRef.current,
                  registerPeriodRef.current,
                  ytdViewRef.current,
                ),
                magiQualifyingType(body.activityType as string)
                  ? client.executeQuery("MagiProjectionGet")
                  : Promise.resolve(null),
              ]);
              if (week.ok && week.bodyJson) {
                setCashWeek(JSON.parse(week.bodyJson) as CashManagementWeekGet);
              }
              if (rem.ok && rem.bodyJson) {
                setCashReminders(
                  JSON.parse(rem.bodyJson) as CashManagementRemindersGet,
                );
              }
              if (mo.ok && mo.bodyJson) {
                setCashMonth(JSON.parse(mo.bodyJson) as CashManagementMonthGet);
              }
              if (registerPack.register) setCashRegister(registerPack.register);
              if (registerPack.elements) setCashElements(registerPack.elements);
              if (registerPack.ytd) setCashYtd(registerPack.ytd);
              if (magi && magi.ok && magi.bodyJson) {
                setCashMagi(JSON.parse(magi.bodyJson) as MagiProjection);
              }
              setCashDirty(false);
              setActionMessage("Cash distribution saved.");
              return true;
            }}
            onSsaConfirm={async (body) => {
              const r = await client.executeCommand("SsaConfirm", body);
              if (!r.ok) {
                setActionMessage(
                  `Social Security retirement confirm failed: ${r.errorCode ?? "error"}`,
                );
                return false;
              }
              const asOf = (body.occurredOn as string) || asOfDate;
              const posted = r.bodyJson
                ? (JSON.parse(r.bodyJson) as { activityType?: string })
                : {};
              const [week, rem, mo, registerPack, magi] = await Promise.all([
                client.executeQuery("CashManagementWeekGet", {
                  asOfDate: asOf,
                }),
                client.executeQuery("CashManagementRemindersGet", {
                  asOfDate: asOf,
                }),
                client.executeQuery("CashManagementMonthGet", {
                  asOfDate: asOf,
                }),
                fetchCashNav(client,
                  asOf,
                  registerBookRef.current,
                  registerPeriodRef.current,
                  ytdViewRef.current,
                ),
                magiQualifyingType(posted.activityType)
                  ? client.executeQuery("MagiProjectionGet")
                  : Promise.resolve(null),
              ]);
              if (week.ok && week.bodyJson) {
                setCashWeek(JSON.parse(week.bodyJson) as CashManagementWeekGet);
              }
              if (rem.ok && rem.bodyJson) {
                setCashReminders(
                  JSON.parse(rem.bodyJson) as CashManagementRemindersGet,
                );
              }
              if (mo.ok && mo.bodyJson) {
                setCashMonth(JSON.parse(mo.bodyJson) as CashManagementMonthGet);
              }
              if (registerPack.register) setCashRegister(registerPack.register);
              if (registerPack.elements) setCashElements(registerPack.elements);
              if (registerPack.ytd) setCashYtd(registerPack.ytd);
              if (magi && magi.ok && magi.bodyJson) {
                setCashMagi(JSON.parse(magi.bodyJson) as MagiProjection);
              }
              setCashDirty(false);
              setActionMessage("Tom Social Security retirement confirmed.");
              return true;
            }}
          >
            <TrendsCapturePanel
              capture={
                trendsCapture &&
                incomeWeek &&
                (incomeWeek.start === trendsCapture.periodStart ||
                  incomeWeek.end === trendsCapture.periodEnd)
                  ? {
                      ...trendsCapture,
                      plannedWeeklyIncomeMinor:
                        incomePlanWeekPlanMinor(incomeWeek) ??
                        trendsCapture.plannedWeeklyIncomeMinor,
                    }
                  : trendsCapture
              }
              busy={busy}
              onReload={(d) => {
                trendsWeekAsOfRef.current = d;
                void (async () => {
                  setBusy(true);
                  try {
                    const r = await client.executeQuery("TrendsWeekGet", { asOfDate: d });
                    if (r.ok && r.bodyJson) {
                      setTrendsCapture(JSON.parse(r.bodyJson) as TrendsWeekCapture);
                    }
                  } finally {
                    setBusy(false);
                  }
                })();
              }}
              onSave={async (body, correct) => {
                setBusy(true);
                try {
                  const r = await client.executeCommand("WeekCaptureAccept", {
                    ...body,
                    correct,
                    allowClosed: correct,
                  });
                  if (!r.ok) {
                    setActionMessage(`Trends save failed: ${r.errorCode ?? "error"}`);
                    return;
                  }
                  const saved = r.bodyJson
                    ? (JSON.parse(r.bodyJson) as TrendsWeekCapture)
                    : null;
                  if (saved) {
                    setTrendsCapture(saved);
                  }
                  trendsGraphPeriodRef.current = DEFAULT_GRAPH_PERIOD;
                  setTrendsChartEpoch((n) => n + 1);
                  await refreshData(
                    saved?.periodEnd || asOfDate || (body.periodEnd as string),
                  );
                  if (!correct) {
                    setCashWeekSavedAt(Date.now());
                  }
                  const next = saved?.firstUnpopulatedStart;
                  if (next && next !== saved?.periodStart) {
                    trendsWeekAsOfRef.current = next;
                    const q = await client.executeQuery("TrendsWeekGet", {
                      asOfDate: next,
                    });
                    if (q.ok && q.bodyJson) {
                      setTrendsCapture(JSON.parse(q.bodyJson) as TrendsWeekCapture);
                    }
                  }
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
              onWizardActive={setWeekWizardActive}
            />
          </CashManagementPanel>
          )}
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
          <LotCostTable
            lots={(holdings?.lots ?? []).map((l) => ({
              lotId: l.lotId,
              symbol: l.symbol,
              accountName: l.accountName,
              remainingQuantityMinor: l.remainingQuantityMinor,
              quantityScale: l.quantityScale,
              openedOn: l.openedOn,
              remainingPerformanceMinor: l.remainingPerformanceMinor,
              remainingTaxMinor: l.remainingTaxMinor,
              scale: l.scale,
            }))}
            value={lotId}
            onChange={setLotId}
            lastBySymbol={Object.fromEntries(
              (calculator?.rows ?? []).map((r) => [r.symbol, r.lastPriceMinor]),
            )}
            sortMode={holdingsLotSort}
            onSortModeChange={setHoldingsLotSort}
            disabled={busy || writesBlocked}
            ariaLabel="Lot id"
          />
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
                  disabled={busy || (!wizCollectorComplete && processALotCount === 0)}
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
              Template Dividend
              <input
                aria-label="Template Dividend"
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
                className={wizDirty ? "is-unsaved" : undefined}
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
              {wizAskSecondUrl && !wizSecondUrlTried ? (
                <section aria-label="Second distribution URL" className="process-a-second-url">
                  <p role="status">
                    First history parse failed. Paste a second issuer URL once (same
                    adapter). A second miss is a loud fail — no half adapter. Manual
                    adapter stays parked.
                  </p>
                  <label>
                    Second distribution URL
                    <input
                      aria-label="Second distribution URL"
                      value={wizSecondUrl}
                      onChange={(e) => setWizSecondUrl(e.target.value)}
                      disabled={busy || writesBlocked}
                    />
                  </label>
                  <button
                    type="button"
                    aria-label="Retry with second URL"
                    disabled={busy || writesBlocked || !wizSecondUrl.trim()}
                    onClick={() => void retryProcessASecondUrl()}
                  >
                    Retry with this URL
                  </button>
                </section>
              ) : null}
              {wizSecondUrlTried ? (
                <p role="alert">
                  Second distribution URL failed. Adapter not built. Manual adapter
                  is parked. Collector stays incomplete.
                </p>
              ) : null}
              {wizAskInception ? (
                <section aria-label="Inception confirm" className="process-a-inception">
                  <p role="status">
                    Paid history is under 12.
                    {wizInceptionCandidate
                      ? ` Search found inception ${wizInceptionCandidate}${
                          wizExpectedPaid != null
                            ? ` — expected ${formatCount(wizExpectedPaid)} paid periods since then`
                            : ""
                        }. Is this name too new for 12 paid declarations?`
                      : " Inception search missed. Yes stores a date you type; No opens a ticket and leaves the collector incomplete."}
                  </p>
                  <label>
                    Inception date
                    <input
                      aria-label="Inception date"
                      type="date"
                      value={wizInceptionOn}
                      onChange={(e) => setWizInceptionOn(e.target.value)}
                      disabled={busy || writesBlocked}
                    />
                  </label>
                  <div className="buttons">
                    <button
                      type="button"
                      aria-label="Inception yes"
                      disabled={busy || writesBlocked || !wizInceptionOn.trim()}
                      onClick={() => void confirmProcessAInception(true)}
                    >
                      Yes — store inception
                    </button>
                    <button
                      type="button"
                      aria-label="Inception no"
                      disabled={busy || writesBlocked}
                      onClick={() => void confirmProcessAInception(false)}
                    >
                      No
                    </button>
                  </div>
                </section>
              ) : null}
              <section aria-label="Mandatory data checklist">
              <table aria-label="Retrieved versus unknown">
                <thead>
                  <tr>
                    <th scope="col">Field</th>
                    <th scope="col">Value</th>
                    <th scope="col">Status</th>
                    <th scope="col">Checklist</th>
                  </tr>
                </thead>
                <tbody>
                  <tr>
                    <th scope="row">Provider / type</th>
                    <td>
                      {[wizProvider, wizDeclSource].filter(Boolean).join(" · ") || "—"}
                    </td>
                    <td>{wizProvider.trim() || wizDeclSource.trim() ? "retrieved" : "unknown"}</td>
                    <td>
                      <button type="button" aria-label="Accept provider" disabled={busy} onClick={() => void decideProcessAField("provider", "accept")}>Accept</button>
                      <button type="button" aria-label="Skip provider" disabled={busy} onClick={() => void decideProcessAField("provider", "skip")}>Skip</button>
                    </td>
                  </tr>
                    <tr>
                      <th scope="row">Underlying</th>
                      <td>{wizUnderlying.trim() || "—"}</td>
                      <td>{wizUnderlying.trim() ? "retrieved" : "unknown"}</td>
                      <td>
                        <button type="button" aria-label="Accept underlying" disabled={busy} onClick={() => void decideProcessAField("underlying", "accept")}>Accept</button>
                        <button type="button" aria-label="Skip underlying" disabled={busy} onClick={() => void decideProcessAField("underlying", "skip")}>Skip</button>
                      </td>
                    </tr>
                    <tr>
                      <th scope="row">Strategy characteristics</th>
                      <td aria-label="Strategy characteristics">
                        {strategyCharacteristics(wizLookthrough, wizUnderlying) || "—"}
                      </td>
                      <td>
                        {strategyCharacteristics(wizLookthrough, wizUnderlying)
                          ? "retrieved"
                          : "unknown"}
                      </td>
                      <td />
                    </tr>
                  <tr>
                    <th scope="row">Frequency</th>
                    <td>{wizFreq.trim() || "—"}</td>
                    <td>{parseCadence(wizFreq) ? "retrieved" : "unknown"}</td>
                    <td>
                      <button type="button" aria-label="Accept frequency" disabled={busy} onClick={() => void decideProcessAField("frequency", "accept")}>Accept</button>
                      <button type="button" aria-label="Skip frequency" disabled={busy} onClick={() => void decideProcessAField("frequency", "skip")}>Skip</button>
                    </td>
                  </tr>
                  <tr>
                    <th scope="row">Last 12 declarations</th>
                    <td>
                      {wizDecls.length > 0
                        ? `${formatCount(wizDecls.length)} paid`
                        : "—"}
                    </td>
                    <td>{wizDecls.length > 0 ? "retrieved" : "unknown"}</td>
                    <td />
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
                    <td>
                      <button type="button" aria-label="Accept remaining year" disabled={busy} onClick={() => void decideProcessAField("remaining_year", "accept")}>Accept</button>
                      <button type="button" aria-label="Skip remaining year" disabled={busy} onClick={() => void decideProcessAField("remaining_year", "skip")}>Skip</button>
                    </td>
                  </tr>
                  <tr>
                    <th scope="row">Suggested Plan</th>
                    <td>
                      {wizReview?.mostCurrentMinor != null
                        ? `${formatPerShare(wizReview.mostCurrentMinor, wizReview.amountScale)}/share (Most Current)`
                        : "—"}
                      {wizReview?.avg6Minor != null
                        ? `; Avg 6 ${formatPerShare(wizReview.avg6Minor, wizReview.amountScale)}`
                        : ""}
                    </td>
                    <td>
                      {wizReview?.mostCurrentMinor != null || wizReview?.avg6Minor != null
                        ? "retrieved"
                        : "unknown"}
                    </td>
                    <td />
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
                    <td />
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
                    <td>
                      <button type="button" aria-label="Accept risk" disabled={busy} onClick={() => void decideProcessAField("risk_tier", "accept")}>Accept</button>
                      <button type="button" aria-label="Skip risk" disabled={busy} onClick={() => void decideProcessAField("risk_tier", "skip")}>Skip</button>
                      <button type="button" aria-label="Skip backtest" disabled={busy} onClick={() => void decideProcessAField("backtest", "skip")}>Skip backtest</button>
                    </td>
                  </tr>
                  <tr>
                    <th scope="row">DIV-1 / CASH</th>
                    <td>—</td>
                    <td>{wizCollectorGaps.includes("div_type") ? "unknown" : "retrieved"}</td>
                    <td>
                      <button type="button" aria-label="Accept div type" disabled={busy} onClick={() => void decideProcessAField("div_type", "accept")}>Accept</button>
                      <button type="button" aria-label="Skip div type" disabled={busy} onClick={() => void decideProcessAField("div_type", "skip")}>Skip</button>
                    </td>
                  </tr>
                </tbody>
              </table>
              </section>

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
                {!(wizRoc && wizRoc.rocPctMinor != null) ? (
                  <label>
                    Template ROC
                    <input
                      aria-label="Template ROC"
                      value={wizRocUrl}
                      onChange={(e) => setWizRocUrl(e.target.value)}
                      disabled={busy || writesBlocked}
                      placeholder="https://…19a-1…"
                    />
                  </label>
                ) : null}
                {!(wizRoc && wizRoc.rocPctMinor != null) ? (
                  <button
                    type="button"
                    aria-label="Store ROC URL"
                    disabled={busy || writesBlocked || !wizRocUrl.trim()}
                    onClick={() => void submitProcessARocUrl()}
                  >
                    Store ROC URL and parse
                  </button>
                ) : null}
                <div className="buttons">
                  <button type="button" aria-label="Accept ROC estimate" disabled={busy} onClick={() => void decideProcessAField("roc_estimate", "accept")}>
                    Accept ROC
                  </button>
                  <button type="button" aria-label="Skip ROC estimate" disabled={busy} onClick={() => void decideProcessAField("roc_estimate", "skip")}>
                    Skip ROC
                  </button>
                </div>
                {wizCollectorComplete ? (
                  <p role="status">
                    Collector complete. Add lots is offered below — research does not open a lot.
                  </p>
                ) : wizCollectorGaps.length ? (
                  <p role="status">Incomplete: {wizCollectorGaps.join(", ")}.</p>
                ) : null}
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
                            {formatPerShare(d.amountPerShareMinor, d.amountScale ?? 4)}
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
                  {processALotCount > 0
                    ? "Recreate validates stored facts. It does not re-import pays, identity, ROC, or Plan. Confirm Plan is not required unless you change Plan."
                    : "Confirm Plan writes Plan / share only. It does not replace stored paid history, lots, or remaining-year dates."}
                  {parseCadence(wizFreq) ? "" : " Frequency research is still required."}
                </p>
                {(() => {
                  const planScale = 4;
                  const proposedMinor = Number(wizPlan);
                  const proposedOk = Number.isFinite(proposedMinor);
                  const proposedLabel = proposedOk
                    ? formatPerShare(Math.round(proposedMinor * 10 ** planScale), planScale)
                    : "—";
                  const storedLabel = wizStoredPlan
                    ? formatPerShare(wizStoredPlan.minor, wizStoredPlan.scale)
                    : "none";
                  const planChanges =
                    wizStoredPlan != null &&
                    proposedOk &&
                    storedLabel !== proposedLabel;
                  return (
                    <div
                      className="process-a-plan-diff"
                      aria-label="Proposed plan changes"
                    >
                      <p>
                        Stored Plan {storedLabel} → this button {proposedLabel}
                        {planChanges
                          ? `. This will change the stored Plan from ${storedLabel} to ${proposedLabel}.`
                          : ". No Plan amount change."}
                      </p>
                      <p>
                        {processALotCount > 0 && !planChanges
                          ? "Proposed writes: none. Stored facts stay. Validation only."
                          : "Proposed writes: Plan / share only. Not written: stored paid history, lots, remaining-year dates, last price, ROC, or tier (Apply tier is a separate button)."}
                      </p>
                      {processALotCount > 0 ? (
                        <p>
                          Recreate lists stored pays at five decimal places so
                          0.13000 is not shown as 0.13. It does not re-import
                          them.
                        </p>
                      ) : null}
                    </div>
                  );
                })()}
                <div className="form-grid">
                  <label>
                    Plan / share
                    <input
                      aria-label="Plan per share"
                      className="per-share-input"
                      inputMode="decimal"
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
                      setWizPlan(formatPerShare(minor, scale));
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
                      setWizPlan(formatPerShare(wizReview.avg6Minor, wizReview.amountScale));
                      setWizPlanReason("Match Avg 6 (owner typed)");
                    }}
                  >
                    Use Avg 6 as Plan
                  </button>
                  <button
                    type="button"
                    aria-label="Confirm Plan"
                    className={wizDirty ? "is-unsaved" : undefined}
                    disabled={
                      busy ||
                      writesBlocked ||
                      !wizSecurityId ||
                      !wizPlan.trim() ||
                      !wizPlanReason ||
                      !parseCadence(wizFreq) ||
                      Boolean(wizReview?.confirmBlocked) ||
                      (wizReview?.observationCount ?? 0) < 1 ||
                      (Boolean(wizReview?.incompleteReasonRequired) && !wizIncomplete.trim()) ||
                      (processALotCount > 0 &&
                        wizPlanStored &&
                        wizStoredPlan != null &&
                        formatPerShare(wizStoredPlan.minor, wizStoredPlan.scale) ===
                          (Number.isFinite(Number(wizPlan))
                            ? formatPerShare(Math.round(Number(wizPlan) * 10 ** 4), 4)
                            : ""))
                    }
                    onClick={() => void confirmPlan()}
                  >
                    {processALotCount > 0
                      ? "Update Plan / share"
                      : "Confirm Plan"}
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
              className={addLotDirty ? "is-unsaved" : undefined}
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
            <ResearchedSymbolCombobox
              options={filteredAddLotSecurities}
              query={addLotQuery}
              onQueryChange={(q) => {
                setAddLotQuery(q);
                setAddLotSecurityId("");
              }}
              open={addLotSymbolOpen}
              onOpenChange={setAddLotSymbolOpen}
              selectedId={addLotSecurityId}
              onSelect={selectAddLotSecurity}
              disabled={busy || writesBlocked}
              inputAriaLabel="Add lot symbol"
              listId="add-lot-symbol-list"
              listAriaLabel="Add lot symbol matches"
            />
            <AccountSelect
              accounts={accounts}
              value={addLotAccountId}
              onChange={setAddLotAccountId}
              ariaLabel="Add lot account"
              disabled={busy || writesBlocked}
            />
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
            Choose a Fidelity or Schwab CSV. A new window walks Import, then Validate (the
            transaction list), then Load or Cancel. Load writes cash. Cancel writes nothing.
          </p>
          {pendingBatchId && pendingBatchStatus !== "posted" && pendingBatchStatus !== "none" ? (
            <p>
              Last CSV is {pendingBatchStatus}
              {pendingBatchFilename ? ` (${pendingBatchFilename})` : ""}. Open the window to
              inspect rows, then Load or Cancel.
            </p>
          ) : null}
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
            {pendingBatchId && pendingBatchStatus !== "posted" && pendingBatchStatus !== "none" ? (
              <button
                type="button"
                aria-label="Continue last import"
                disabled={busy || writesBlocked}
                onClick={() =>
                  void openImportWizardWindow(
                    pendingBatchId,
                    pendingBatchFilename || "CSV",
                  )
                }
              >
                Continue last import
              </button>
            ) : null}
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

      {screen === "tickets" ? (
        <section aria-label="Tickets">
          <h2>Tickets</h2>
          <p>
            {ticketFocusSymbol
              ? `Open work tickets for ${ticketFocusSymbol}. `
              : "All open work tickets, every symbol. "}
            Recreate adapter opens Add Position with the stored Template
            Dividend and researches that URL (self-heal). Paste only when no
            seed URL is stored. A GET timeout skips that name, continues the
            fleet, then retries the same stored URL at 20s / 45s / 90s — Retry
            does that too. Recreate is not the timeout path. Amount variation
            stays open until Except (keep the vendor amount) or Reject
            (discard it).
          </p>
          {ticketFocusSymbol ? (
            <button
              type="button"
              aria-label="Show all tickets"
              onClick={() => setTicketFocusSymbol("")}
            >
              Show all tickets
            </button>
          ) : null}
          <WorkTicketQueue
            tickets={workTickets}
            filterSymbol={ticketFocusSymbol || undefined}
            retryingTicketId={retryingTicket?.ticketId}
            retryingSymbol={retryingTicket?.symbol}
            pendingTicketId={ticketDecision?.ticketId}
            pendingAction={ticketDecision?.action}
            onRetry={(t) => void resolveTicketRetry(t as WorkTicketRecord)}
            onRecreateAdapter={(t) =>
              openRecreateAdapter(t as WorkTicketRecord)
            }
            onExcept={(t) =>
              resolveConfirmTicket(t as WorkTicketRecord, "positive")
            }
            onReject={(t) =>
              resolveConfirmTicket(t as WorkTicketRecord, "reject")
            }
            onEnterAmount={(t, amount) =>
              void resolveEnterDeclaredAmount(t as WorkTicketRecord, amount)
            }
            onFile={(t) => void fileWorkTicket(t as WorkTicketRecord)}
          />
        </section>
      ) : null}

      {screen === "collectors" ? (
        <CollectorsScreen
          busy={busy}
          writesBlocked={writesBlocked}
          collectorItems={collectorItems}
          collectorStats={collectorStats}
          collectorAction={collectorAction}
          collectorStatusAt={collectorStatusAt}
          collectorRunProgress={collectorRunProgress}
          researchActivity={researchActivity}
          missingUrlDrafts={missingUrlDrafts}
          setMissingUrlDrafts={setMissingUrlDrafts}
          fleetDetailId={fleetDetailId}
          setFleetDetailId={setFleetDetailId}
          collectorSymbol={collectorSymbol}
          collectorRuns={collectorRuns}
          collectorPayload={collectorPayload}
          collectorPlan={collectorPlan}
          exceptions={exceptions}
          workTickets={workTickets}
          retryingTicket={retryingTicket}
          ticketDecision={ticketDecision}
          onRunEnabled={() => void runEnabledCollectors()}
          onRunMisses={() => void runMissesOnlyCollectors()}
          onFillResearchGaps={() => void fillResearchGaps()}
          onApplyIssuerSources={() => void applyIssuerSources()}
          onApplyMissingUrls={() => void applyMissingCollectorUrls()}
          onToggleEnabled={(row, enabled) => void toggleCollectorEnabled(row, enabled)}
          onForceRefresh={(row) => void forceCollectorRefresh(row)}
          onOpenPosition={(symbol) => {
            leaveWithoutSaving(() => {
              setPositionSymbol(symbol);
              setScreen("position-details");
              void loadInvestment(symbol);
            });
          }}
          onOpenTickets={(symbol) => {
            setTicketFocusSymbol(symbol);
            setScreen("tickets");
          }}
          onOpenPositionHub={(symbol) => openPositionHub(symbol)}
          onOpenExceptionLog={() => void openExceptionLog()}
          onRetryTicket={(t) => void resolveTicketRetry(t)}
          onRecreateAdapter={(t) => openRecreateAdapter(t as WorkTicketRecord)}
          onExceptTicket={(t) => resolveConfirmTicket(t, "positive")}
          onRejectTicket={(t) => resolveConfirmTicket(t, "reject")}
          onEnterAmount={(t, amount) => void resolveEnterDeclaredAmount(t, amount)}
          onFileTicket={(t) => void fileWorkTicket(t)}
        />
      ) : null}

      {screen === "collector-establish" ? (
        <CollectorEstablishScreen
          busy={busy}
          writesBlocked={writesBlocked}
          collectorItems={collectorItems}
          onEstablish={(row) => void establishCollector(row)}
          onReevaluate={(row) => void reevaluateCollector(row)}
          onAcceptRoc={(row) => void acceptCollectorRoc(row)}
        />
      ) : null}

      {screen === "components" ? (
        <section className="actions" aria-label="Component registry">
          <h2>Components</h2>
          <p>
            UI extraction catalog: whether a screen has left App.tsx. Not a
            domain-completeness index and not a plugin host. Core functions
            stay in Settings.
          </p>
          <div className="table-wrap">
            <table aria-label="Component registry">
              <thead>
                <tr>
                  <th scope="col">Module</th>
                  <th scope="col">Status</th>
                  <th scope="col">Folder</th>
                  <th scope="col">Menu areas</th>
                  <th scope="col">Core functions</th>
                  <th scope="col">Host</th>
                </tr>
              </thead>
              <tbody>
                {coreFunctions?.modules?.length ? (
                  coreFunctions.modules.map((row) => (
                    <tr key={row.id}>
                      <td>{row.title}</td>
                      <td>{row.status}</td>
                      <td>{row.folder}</td>
                      <td>{row.menuAreas.join(", ")}</td>
                      <td>
                        {row.coreFunctionIds?.length
                          ? row.coreFunctionIds.join(", ")
                          : "—"}
                      </td>
                      <td>{row.host || "—"}</td>
                    </tr>
                  ))
                ) : (
                  <tr>
                    <td colSpan={6}>Loading components…</td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
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
          <h3>Core functions</h3>
          <p>
            Owner-facing functions the golden harness must still find after a
            change. Last changed is when that function's files last moved. Last
            verified is when its sentinel last passed. Also verify lists the
            golden test names that must still pass when that function changes.
          </p>
          <div className="table-wrap">
            <table aria-label="Core functions">
              <thead>
                <tr>
                  <th scope="col">Menu area</th>
                  <th scope="col">Function</th>
                  <th scope="col">Last changed</th>
                  <th scope="col">Last verified</th>
                  <th scope="col">Also verify</th>
                </tr>
              </thead>
              <tbody>
                {coreFunctions?.items.length ? (
                  coreFunctions.items.map((row) => (
                    <tr key={row.id}>
                      <td>{row.menuArea}</td>
                      <td>{row.function}</td>
                      <td>{row.lastChanged}</td>
                      <td>{row.lastVerified}</td>
                      <td>{row.alsoVerify?.length ? row.alsoVerify.join(", ") : "—"}</td>
                    </tr>
                  ))
                ) : (
                  <tr>
                    <td colSpan={5}>Loading core functions…</td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
          <h3>Retrieval templates</h3>
          <p>
            Standing templates: adapter name, Template Dividend, Template ROC, and
            content hash. Inception is optional — rare exception for names too
            new for 12 paid points. Collectors retrieve first; if 12+ paid decls
            land, inception is N/A. Under 12, inception (when set) confirms the
            short history is complete.
          </p>
          <div className="table-wrap">
            <table aria-label="Retrieval templates">
              <thead>
                <tr>
                  <th scope="col">Symbol</th>
                  <th scope="col">Adapter</th>
                  <th scope="col">Template Dividend</th>
                  <th scope="col">Template ROC</th>
                  <th scope="col">Content hash</th>
                  <th scope="col">Schedule</th>
                  <th scope="col">Inception</th>
                  <th scope="col">Lookback</th>
                  <th scope="col">Actions</th>
                </tr>
              </thead>
              <tbody>
                {collectorItems.length === 0 ? (
                  <tr>
                    <td colSpan={9}>
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
                      rocSourceUrl: row.rocSourceUrl ?? "",
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
                          <input
                            aria-label={`Template ROC for ${row.symbol}`}
                            value={draft.rocSourceUrl ?? ""}
                            onChange={(e) =>
                              patch({ rocSourceUrl: e.target.value })
                            }
                            disabled={busy || writesBlocked}
                          />
                        </td>
                        <td aria-label={`Content hash for ${row.symbol}`}>
                          {row.lastContentHash?.trim() || "—"}
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
              aria-label="Save data snapshot"
              disabled={busy}
              onClick={() => requestDataSnapshot()}
            >
              Save data snapshot
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
    </>
  );
}
