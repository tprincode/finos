import { useCallback, useEffect, useRef, useState } from "react";
import {
  FINANCE_CLIENT_CONTRACT_VERSION,
  type AccountListItem,
  type CalculatorGet,
  type CurrentPriceGet,
  type DashboardBurndownGet,
  type ExceptionRecord,
  type HandoffStatus,
  type HoldingsGet,
  type HouseholdSummaryGet,
  type IncomePlanWeekGet,
  type InvestmentGet,
  type PositionDetailsGet,
  type PositionDetailsCoverageGet,
  type PositionMasterGet,
  type PlanReviewGet,
  type RemainingYearIncomeGet,
  type RocResearchGet,
  type SecurityListItem,
} from "@finos/app-contracts";
import { invoke } from "@tauri-apps/api/core";
import { check } from "@tauri-apps/plugin-updater";
import { LocalTauriFinanceClient } from "./financeClient";
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
} from "@finos/ui-components";
import "./App.css";

const RISK_TIERS = ["Foundation", "Core", "Risk On"];
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
  qty: string;
  cost: string;
  remainingPays: string;
  nextPayDate: string;
};

const emptyAddLot = (): AddLotDraft => ({
  securityId: "",
  accountId: "",
  qty: "1",
  cost: "",
  remainingPays: "|next:",
  nextPayDate: "",
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

function monthTotalsFromPays(
  pays: Array<{ payOn: string; cashMinor: number | null; thisLotCashMinor?: number | null; positionAfterCashMinor?: number | null }>,
): Array<{ month: string; cashMinor: number | null; thisLotCashMinor: number | null; positionAfterCashMinor: number | null }> {
  const months: Array<{
    month: string;
    cashMinor: number | null;
    thisLotCashMinor: number | null;
    positionAfterCashMinor: number | null;
  }> = [];
  for (const p of pays) {
    const month = p.payOn.length >= 7 ? p.payOn.slice(0, 7) : "";
    if (!month) continue;
    const last = months[months.length - 1];
    if (last && last.month === month) {
      last.cashMinor =
        last.cashMinor == null || p.cashMinor == null ? null : last.cashMinor + p.cashMinor;
      last.thisLotCashMinor =
        last.thisLotCashMinor == null || p.thisLotCashMinor == null
          ? null
          : last.thisLotCashMinor + p.thisLotCashMinor;
      last.positionAfterCashMinor =
        last.positionAfterCashMinor == null || p.positionAfterCashMinor == null
          ? null
          : last.positionAfterCashMinor + p.positionAfterCashMinor;
    } else {
      months.push({
        month,
        cashMinor: p.cashMinor,
        thisLotCashMinor: p.thisLotCashMinor ?? null,
        positionAfterCashMinor: p.positionAfterCashMinor ?? null,
      });
    }
  }
  return months;
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
  | "holdings"
  | "import"
  | "settings"
  | "new-investment"
  | "add-lot"
  | "position-details";

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
  const [holdings, setHoldings] = useState<HoldingsGet | null>(null);
  const [calculator, setCalculator] = useState<CalculatorGet | null>(null);
  const [summary, setSummary] = useState<HouseholdSummaryGet | null>(null);
  const [exceptions, setExceptions] = useState<ExceptionRecord[]>([]);
  const [pendingBatchId, setPendingBatchId] = useState<string | null>(null);
  const [pendingBatchStatus, setPendingBatchStatus] = useState<string>("none");
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
  const [wizRisk, setWizRisk] = useState("");
  const [wizFreq, setWizFreq] = useState("");
  const [wizSecurityId, setWizSecurityId] = useState("");
  const [wizPrice, setWizPrice] = useState("");
  const [wizPriceSource, setWizPriceSource] = useState("public");
  const [wizDeclSource, setWizDeclSource] = useState("");
  const [wizLookback, setWizLookback] = useState("12");
  const [wizAnalytics, setWizAnalytics] = useState<{
    htmlReturned?: boolean;
    fundPage?: boolean;
    tableOnGet?: boolean;
    tableRowCount?: number;
    jsLikely?: boolean;
  } | null>(null);
  const [wizFutureStrategy, setWizFutureStrategy] = useState("");
  const [wizDeclAmounts, setWizDeclAmounts] = useState("");
  const [wizAttempts, setWizAttempts] = useState<
    Array<{ vendor: string; url: string; found: boolean; note: string }>
  >([]);
  const [wizMiss, setWizMiss] = useState("");
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
  const [addLotQty, setAddLotQty] = useState("1");
  const [addLotCost, setAddLotCost] = useState("");
  const [addLotBaseline, setAddLotBaseline] = useState(() => JSON.stringify(emptyAddLot()));
  const [wizRemaining, setWizRemaining] = useState<RemainingYearIncomeGet | null>(null);
  const [wizPayDraft, setWizPayDraft] = useState<Array<{ originalPayOn: string; payOn: string }>>(
    [],
  );
  const [wizNextPay, setWizNextPay] = useState("");
  const [addLotRemaining, setAddLotRemaining] = useState<RemainingYearIncomeGet | null>(null);
  const [addLotPayDraft, setAddLotPayDraft] = useState<
    Array<{ originalPayOn: string; payOn: string }>
  >([]);
  const [addLotNextPay, setAddLotNextPay] = useState("");
  const [positionDetails, setPositionDetails] = useState<PositionDetailsGet | null>(null);
  const [positionMaster, setPositionMaster] = useState<PositionMasterGet | null>(null);
  const [issuerCoverage, setIssuerCoverage] = useState<PositionDetailsCoverageGet | null>(null);
  const [pdRemaining, setPdRemaining] = useState<RemainingYearIncomeGet | null>(null);
  const [positionSymbol, setPositionSymbol] = useState("");
  const [investment, setInvestment] = useState<InvestmentGet | null>(null);
  const [wizPart1Stored, setWizPart1Stored] = useState(false);
  const [wizPlanStored, setWizPlanStored] = useState(false);
  const [wizLotStored, setWizLotStored] = useState(false);
  const [wizStep, setWizStep] = useState(1);
  const [wizRocPct, setWizRocPct] = useState("");
  const [wizRoc, setWizRoc] = useState<RocResearchGet | null>(null);
  const [wizBullStart, setWizBullStart] = useState("");
  const [wizBullEnd, setWizBullEnd] = useState("");
  const [wizBearStart, setWizBearStart] = useState("");
  const [wizBearEnd, setWizBearEnd] = useState("");
  const [wizBullStored, setWizBullStored] = useState(false);
  const [wizBearStored, setWizBearStored] = useState(false);
  const [wizBaseline, setWizBaseline] = useState(() => JSON.stringify(emptyWizEdit()));
  const [pdDraft, setPdDraft] = useState<PdDraft | null>(null);
  const [pdBaseline, setPdBaseline] = useState("");
  const [pdPeriod, setPdPeriod] = useState<PdPeriodDraft>(() => emptyPeriod());
  const [pdPeriodBaseline, setPdPeriodBaseline] = useState(() => JSON.stringify(emptyPeriod()));
  const [savedPeriodId, setSavedPeriodId] = useState("");
  const [lastPriceBusy, setLastPriceBusy] = useState(false);
  const lastPriceKickoff = useRef(false);

  const refreshHousehold = useCallback(async (asOf: string) => {
    const [weekResult, burnResult, holdingsResult, exceptionResult, summaryResult, calcResult, accountResult, securityResult, positionResult, masterResult, coverageResult] =
      await Promise.all([
        client.executeQuery("IncomePlanWeekGet", { asOfDate: asOf }),
        client.executeQuery("DashboardBurndownGet", { asOfDate: asOf }),
        client.executeQuery("HoldingsGet"),
        client.executeQuery("ExceptionList"),
        client.executeQuery("HouseholdSummaryGet"),
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
      holdingsResult,
      exceptionResult,
      summaryResult,
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
    setHoldings(parse<HoldingsGet>(holdingsResult.bodyJson));
    setCalculator(parse<CalculatorGet>(calcResult.bodyJson));
    setSummary(parse<HouseholdSummaryGet>(summaryResult.bodyJson));
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
      await refreshHousehold(asOfDate);
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
      await refreshHousehold(asOfDate);
    } catch (err: unknown) {
      setActionMessage(String(err));
    }
  }, [asOfDate, refreshHousehold]);

  const applyIssuerSources = async () => {
    setBusy(true);
    try {
      const result = await client.executeCommand("ProviderDeclarationSourcesApply", {});
      if (!result.ok) {
        setActionMessage(
          `Apply issuer sources failed: ${result.errorCode ?? "error"}`,
        );
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
      setActionMessage(
        `Applied issuer sources from provider: ${formatCount(updated)} updated. Empty, public, and unassigned only.`,
      );
      await refreshHousehold(asOfDate);
      if (positionSymbol) {
        await loadInvestment(positionSymbol);
      }
    } catch (err: unknown) {
      setActionMessage(String(err));
    } finally {
      setBusy(false);
    }
  };

  const loadInvestment = useCallback(async (symbol: string) => {
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
    const result = await client.executeQuery("InvestmentGet", {
      symbol,
      asOfDate: asOfDate || new Date().toISOString().slice(0, 10),
    });
    if (!result.ok || !result.bodyJson) {
      setInvestment(null);
      setActionMessage(`InvestmentGet failed: ${result.errorCode ?? "not found"}`);
      return;
    }
    try {
      const body = JSON.parse(result.bodyJson) as InvestmentGet;
      const draft = draftFromInvestment(body);
      setInvestment(body);
      setPdDraft(draft);
      setPdBaseline(JSON.stringify(draft));
      setPdPeriod(emptyPeriod());
      setPdPeriodBaseline(JSON.stringify(emptyPeriod()));
      setSavedPeriodId(body.periods?.at(-1)?.periodId ?? "");
    } catch {
      setInvestment(null);
      setPdDraft(null);
      setPdBaseline("");
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
        const result = await client.executeQuery("HouseholdSummaryGet");
        if (cancelled) return;
        if (!result.ok || !result.bodyJson) {
          setAsOfDate("");
          return;
        }
        const body = JSON.parse(result.bodyJson) as HouseholdSummaryGet;
        setSummary(body);
        setAsOfDate(body.latestYieldOn ?? "");
      } catch {
        if (!cancelled) setAsOfDate("");
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    refreshHousehold(asOfDate).catch((err: unknown) => {
      setActionMessage(String(err));
    });
  }, [asOfDate, refreshHousehold]);

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
    const qty = Number(wizQty);
    const asOf = asOfDate || new Date().toISOString().slice(0, 10);
    void client
      .executeQuery("RemainingYearIncomeGet", {
        securityId: wizSecurityId,
        asOfDate: asOf,
        thisLotQuantityMinor: Number.isFinite(qty) ? qty : null,
        thisLotOpenedOn: asOf,
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
  }, [screen, wizSecurityId, wizQty, wizPlanStored, asOfDate]);

  useEffect(() => {
    if (screen !== "add-lot" || !addLotSecurityId) {
      return;
    }
    let cancelled = false;
    const qty = Number(addLotQty);
    const asOf = asOfDate || new Date().toISOString().slice(0, 10);
    void client
      .executeQuery("RemainingYearIncomeGet", {
        securityId: addLotSecurityId,
        asOfDate: asOf,
        thisLotQuantityMinor: Number.isFinite(qty) ? qty : null,
        thisLotOpenedOn: asOf,
      })
      .then((result) => {
        if (cancelled || !result.ok || !result.bodyJson) return;
        const body = JSON.parse(result.bodyJson) as RemainingYearIncomeGet;
        const next = body.payments.map((p) => ({
          originalPayOn: p.originalPayOn,
          payOn: p.payOn,
        }));
        setAddLotRemaining(body);
        setAddLotPayDraft((prev) => {
          if (prev.some((p) => p.payOn !== p.originalPayOn)) return prev;
          return next;
        });
        setAddLotBaseline((prev) => {
          const draft = JSON.parse(prev) as AddLotDraft;
          const parsed = parseRemainingPaysKey(draft.remainingPays ?? "");
          if (parsed.pays.some((p) => p.payOn !== p.originalPayOn)) return prev;
          draft.remainingPays = remainingPaysKey(next, draft.nextPayDate ?? "");
          return JSON.stringify(draft);
        });
      });
    return () => {
      cancelled = true;
    };
  }, [screen, addLotSecurityId, addLotQty, asOfDate]);

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
      await refreshHousehold(asOfDate);
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

  const currentWizEdit = (): WizEditDraft => ({
    symbol: wizSymbol,
    name: wizName,
    provider: wizProvider,
    underlying: wizUnderlying,
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
    qty: addLotQty,
    cost: addLotCost,
    remainingPays: remainingPaysKey(addLotPayDraft, addLotNextPay),
    nextPayDate: addLotNextPay,
  });
  const addLotDirty = JSON.stringify(currentAddLot()) !== addLotBaseline;
  const cancelAddLotEdits = () => {
    const draft = JSON.parse(addLotBaseline) as AddLotDraft;
    setAddLotSecurityId(draft.securityId);
    setAddLotAccountId(draft.accountId);
    setAddLotQty(draft.qty);
    setAddLotCost(draft.cost);
    const remaining = parseRemainingPaysKey(draft.remainingPays ?? "");
    setAddLotPayDraft(remaining.pays);
    setAddLotNextPay(draft.nextPayDate ?? remaining.nextPay);
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

  const saveAddLotRemainingDates = async () => {
    if (!addLotSecurityId) {
      setActionMessage("Choose an existing symbol first.");
      return;
    }
    setBusy(true);
    try {
      const result = await persistRemainingDates(addLotSecurityId, addLotPayDraft, addLotNextPay);
      if (!result.ok) {
        setActionMessage(`Remaining dates not stored: ${result.errorCode ?? "error"}`);
        return;
      }
      setAddLotBaseline(JSON.stringify(currentAddLot()));
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
        "Research filled provider, source analytics, future-declaration method, and underlying when found. Confirm the standing template, then Next retrieves. Yahoo is last price only.",
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
          "Issuer page empty — not using Yahoo.",
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
        "Choose issuer calendar or derived walk. The standing order needs one calendar policy; there is no default.",
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
      await refreshHousehold(asOfDate);
    } catch (err: unknown) {
      setActionMessage(String(err));
    } finally {
      setBusy(false);
    }
  };

  const confirmPlan = async () => {
    if (!wizSecurityId) {
      setActionMessage("Complete Part 1 first.");
      return;
    }
    if (!wizPlanReason) {
      setActionMessage("Choose a Plan reason.");
      return;
    }
    setBusy(true);
    setActionMessage(null);
    try {
      const cadence = parseCadence(wizFreq);
      if (!cadence) {
        setActionMessage(
          "Choose Weekly (52), Monthly (12), Quarterly (4), or None before Confirm Plan. There is no default.",
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
      setWizStep(7);
      const rocBody = await loadRocResearch();
      snapshotWiz();
      const rocNote =
        rocBody?.rocPctMinor != null
          ? ` System ROC ${(rocBody.rocPctMinor / 10 ** rocBody.scale).toFixed(rocBody.scale)}% from ${rocBody.method || rocBody.source}${rocBody.sourceUrl ? ` (${rocBody.sourceUrl})` : ""}. Override if needed, then open the first lot.`
          : " ROC unknown — paste a 19a-1 percent. Unknown is not 0%.";
      setActionMessage(
        `Stored Plan ${wizPlan}/share (${wizPlanReason}).${rocNote}`,
      );
      await refreshHousehold(asOfDate);
    } catch (err: unknown) {
      setActionMessage(String(err));
    } finally {
      setBusy(false);
    }
  };

  const loadRocResearch = async (): Promise<RocResearchGet | null> => {
    const retrieved = await client.executeCommand("RocResearchRetrieve", {
      symbol: wizSymbol.trim().toUpperCase(),
      securityId: wizSecurityId || undefined,
      asOfDate: asOfDate || new Date().toISOString().slice(0, 10),
    });
    const candidates =
      retrieved.ok && retrieved.bodyJson
        ? ((JSON.parse(retrieved.bodyJson) as { candidates?: unknown[] }).candidates ?? [])
        : [];
    const result = await client.executeQuery("RocResearchGet", {
      securityId: wizSecurityId,
      accountId: wizAccountId || undefined,
      asOfDate: asOfDate || new Date().toISOString().slice(0, 10),
      candidates,
    });
    if (!result.ok || !result.bodyJson) {
      return null;
    }
    const body = JSON.parse(result.bodyJson) as RocResearchGet;
    setWizRoc(body);
    if (body.rocPctMinor != null) {
      setWizRocPct((body.rocPctMinor / 10 ** body.scale).toFixed(body.scale));
    }
    return body;
  };

  const researchRoc = async () => {
    if (!wizSecurityId) {
      setActionMessage("Save Part 1 before ROC research.");
      return;
    }
    setBusy(true);
    setActionMessage(null);
    try {
      const body = await loadRocResearch();
      if (!body) {
        setActionMessage("ROC research missed.");
        return;
      }
      setActionMessage(
        body.complete
          ? `System ROC ${body.rocPctMinor == null ? "unknown" : `${(body.rocPctMinor / 10 ** body.scale).toFixed(body.scale)}%`} from ${body.method || body.source}${body.sourceUrl ? ` at ${body.sourceUrl}` : ""}. You may override before MAGI uses it.`
          : "ROC unknown — paste a 19a-1 or 1099 percent. Unknown is not 0%.",
      );
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
      await refreshHousehold(asOfDate);
      await loadInvestment(symbol);
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
    wizDirty ? "New Investment" : null,
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
      await refreshHousehold(asOfDate);
      await loadInvestment(investment.symbol);
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
        riskTier: pdDraft.risk,
        provider: pdDraft.provider.trim(),
        underlying: pdDraft.underlying.trim(),
        notes: pdDraft.notes,
        divType: pdDraft.divType.trim(),
        needsRocResearch: pdDraft.needsRoc,
        isActive: pdDraft.isActive,
        rocPct2024ActualMinor: rocMinor(pdDraft.roc2024),
        rocPct2025ActualMinor: rocMinor(pdDraft.roc2025),
        rocPct2026EstimateMinor: rocMinor(pdDraft.roc2026e),
        rocPct2026ActualMinor: rocMinor(pdDraft.roc2026a),
        rocScale: 2,
      });
      if (!chars.ok) {
        setActionMessage(`Characteristics not stored: ${chars.errorCode ?? "error"}`);
        return;
      }
      const template = await client.executeCommand("RetrievalTemplateSet", {
        securityId: investment.securityId,
        priceSource: pdDraft.priceSource.trim() || "public",
        sourceSymbol: pdDraft.sourceSymbol.trim() || investment.symbol,
        declarationSource: pdDraft.declarationSource.trim(),
        lookbackCount: Number(pdDraft.lookbackCount) || 12,
        sourceUrl: pdDraft.sourceUrl.trim(),
        calendarPolicy: pdDraft.calendarPolicy.trim(),
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
          await refreshHousehold(asOfDate);
          await loadInvestment(investment.symbol);
          return;
        }
        planNote = `; Plan ${pdDraft.plan}/share`;
      }
      setActionMessage(`Stored ${investment.symbol}${planNote}.`);
      await refreshHousehold(asOfDate);
      await loadInvestment(investment.symbol);
    } catch (err: unknown) {
      setActionMessage(String(err));
    } finally {
      setBusy(false);
    }
  };

  const refreshPdLastPrice = async () => {
    if (!investment) {
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
        await client.executeCommand("LastPriceRefresh", {
          quotes: [],
          misses: [{
            securityId: investment.securityId,
            symbol: investment.symbol,
            code: "price_retrieve_miss",
            reason: `Last price miss for ${investment.symbol}. Stored price still displays; never $0.`,
          }],
        });
        setActionMessage("Last price retrieve missed. Unknown stays unknown — not $0.");
        await refreshHousehold(asOfDate);
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
      await refreshHousehold(asOfDate);
      await loadInvestment(investment.symbol);
    } catch (err: unknown) {
      setActionMessage(String(err));
    } finally {
      setBusy(false);
    }
  };

  const refreshPdDeclarations = async () => {
    if (!investment) {
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
      const retrieved = (body.candidates ?? []).filter(
        (d) => d.source && d.amountPerShareMinor != null && d.amountPerShareMinor > 0,
      );
      if (retrieved.length === 0) {
        await client.executeCommand("DeclarationRefresh", {
          misses: [{
            securityId: investment.securityId,
            symbol: investment.symbol,
            code: "declaration_retrieve_miss",
            reason: body.missExplanation || "Issuer page empty — not using Yahoo.",
          }],
        });
        setActionMessage("Issuer page empty — not using Yahoo.");
        await refreshHousehold(asOfDate);
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
      await refreshHousehold(asOfDate);
      await loadInvestment(investment.symbol);
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
      const retrieved = await client.executeCommand("RocResearchRetrieve", {
        symbol: investment.symbol,
        securityId: investment.securityId,
        asOfDate: asOfDate || new Date().toISOString().slice(0, 10),
      });
      const candidates =
        retrieved.ok && retrieved.bodyJson
          ? ((JSON.parse(retrieved.bodyJson) as { candidates?: unknown[] }).candidates ?? [])
          : [];
      const result = await client.executeQuery("RocResearchGet", {
        securityId: investment.securityId,
        asOfDate: asOfDate || new Date().toISOString().slice(0, 10),
        candidates,
      });
      if (!result.ok || !result.bodyJson) {
        setActionMessage("ROC research missed. Unknown is not 0%.");
        return;
      }
      const body = JSON.parse(result.bodyJson) as RocResearchGet;
      setActionMessage(
        body.complete
          ? `System ROC ${body.rocPctMinor == null ? "unknown" : `${(body.rocPctMinor / 10 ** body.scale).toFixed(body.scale)}%`} from ${body.method || body.source}${body.sourceUrl ? ` at ${body.sourceUrl}` : ""}. Confirm in the ROC year boxes if you accept it.`
          : "ROC unknown — paste a 19a-1 percent. Unknown is not 0%.",
      );
      await loadInvestment(investment.symbol);
    } catch (err: unknown) {
      setActionMessage(String(err));
    } finally {
      setBusy(false);
    }
  };

  const openAddLot = async () => {
    if (!addLotSecurityId || !addLotAccountId) {
      setActionMessage("Choose an existing symbol and account.");
      return;
    }
    const dates = await persistRemainingDates(addLotSecurityId, addLotPayDraft, addLotNextPay);
    if (!dates.ok) {
      setActionMessage(`Remaining dates not stored: ${dates.errorCode ?? "error"}`);
      return;
    }
    await runCommand("LotOpen", {
      accountId: addLotAccountId,
      securityId: addLotSecurityId,
      openedOn: new Date().toISOString().slice(0, 10),
      origin: "purchase",
      quantityMinor: Number(addLotQty),
      quantityScale: 0,
      performanceBasisMinor: dollarsToMinor(addLotCost),
      taxBasisMinor: dollarsToMinor(addLotCost),
      scale: 2,
      isOpen: true,
    });
    setAddLotBaseline(JSON.stringify(currentAddLot()));
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
        setActionMessage(`ImportStage failed: ${staged.errorCode ?? "error"}`);
        return;
      }
      const batchId = (JSON.parse(staged.bodyJson) as { batchId?: string }).batchId;
      if (!batchId) {
        setActionMessage("ImportStage failed: missing batchId");
        return;
      }
      setPendingBatchId(batchId);
      const validated = await client.executeCommand("ImportValidate", { batchId });
      const status =
        validated.bodyJson != null
          ? (JSON.parse(validated.bodyJson) as { status?: string }).status ?? "unknown"
          : "unknown";
      setPendingBatchStatus(status);
      await refreshHousehold(asOfDate);
      if (!validated.ok || status !== "validated") {
        setActionMessage(
          `Staged ${batchId}; status ${status}. Review exceptions before approve/post.`,
        );
        return;
      }
      setActionMessage(`Staged and validated ${batchId}. Approve, then post.`);
    } catch (err: unknown) {
      setActionMessage(String(err));
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
    try {
      const result = await client.executeCommand(name, { batchId: pendingBatchId });
      if (result.bodyJson) {
        try {
          const body = JSON.parse(result.bodyJson) as { status?: string };
          if (body.status) setPendingBatchStatus(body.status);
        } catch {
          /* ignore */
        }
      }
      setActionMessage(
        result.ok ? `${name} ok` : `${name} failed: ${result.errorCode ?? "error"}`,
      );
      await refreshHandoff();
      await refreshHousehold(asOfDate);
    } catch (err: unknown) {
      setActionMessage(String(err));
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

  const wizardResearched = wizAttempts.length > 0 || Boolean(wizDeclSource) || Boolean(wizMiss);
  const goWizardNext = async () => {
    if (wizStep === 1) {
      if (!wizSymbol.trim()) {
        setActionMessage("Type the ticker, then Next drafts how this position will be maintained.");
        return;
      }
      setWizStep(2);
      return;
    }
    if (wizStep === 2) {
      const ok = await researchSource();
      if (ok) setWizStep(3);
      return;
    }
    if (wizStep === 3) {
      if (!wizardResearched) {
        const ok = await researchSource();
        if (!ok) return;
      }
      if (!wizDeclSource.trim()) {
        setActionMessage(
          "No issuer source yet. Choose Roundhill or another provider site, or Next will retrieve last price only and leave declarations unknown.",
        );
      }
      const ok = await retrieveFromMarket();
      if (ok) setWizStep(4);
      return;
    }
    if (wizStep === 4) {
      setWizStep(5);
      return;
    }
    if (wizStep === 5) {
      if (!RISK_TIERS.includes(wizRisk)) {
        setActionMessage(
          "Choose Foundation, Core, or Risk On. The form does not assume Risk On.",
        );
        return;
      }
      await newInvestmentPart1();
      return;
    }
    if (wizStep === 6) {
      if (!wizPlanStored) {
        setActionMessage(
          "Type Plan and a reason, then Confirm Plan. Most Current / Avg 6 stay calculated; Plan is yours.",
        );
        return;
      }
      setWizStep(7);
      return;
    }
    if (wizStep === 7) {
      if (!wizSecurityId) {
        setActionMessage("Save Part 1 before ROC research.");
        return;
      }
      if (!wizRoc) {
        await researchRoc();
        return;
      }
      setWizStep(8);
      return;
    }
    if (wizStep === 8) {
      if (!wizLotStored) {
        setActionMessage(
          "Choose account, quantity, and original cost, then Open first lot.",
        );
        return;
      }
      setWizStep(9);
    }
  };
  const showWiz = {
    symbol: wizStep === 1,
    strategy: wizStep === 1 || wizStep === 2,
    template: wizStep === 2,
    research: wizStep === 3,
    price: wizStep === 4,
    review: wizStep === 4 || wizStep === 6,
    identity: wizStep === 5,
    risk: wizStep === 5,
    plan: wizStep === 6,
    roc: wizStep === 7,
    lot: wizStep === 8,
    regime: wizStep === 9,
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
      <p>
        Household stays on this machine after seed. Last prices refresh on open; a stored last
        price still displays when it is not from today.
        {summary
          ? ` Loaded: ${formatCount(summary.accountCount)} accounts, ${formatCount(summary.symbolCount ?? 0)} symbols, ${formatCount(summary.openLotCount)} open lots, ${formatCount(summary.yieldCount)} yield, ${formatCount(summary.disbursementCount)} disbursement, ${formatCount(summary.planCount)} Calculator plans${summary.latestYieldOn ? `, last yield ${summary.latestYieldOn}` : ""}. Original cost ${formatUsd(summary.openPerformanceMinor ?? 0, summary.scale ?? 2)}. Tax ${formatUsd(summary.openTaxMinor ?? 0, summary.scale ?? 2)}. Last prices ${formatCount(summary.lastPriceCount ?? 0)} of ${formatCount(summary.symbolCount ?? 0)}. Market value ${
              summary.marketValueMinor == null
                ? "unknown"
                : `${formatUsd(summary.marketValueMinor, summary.scale ?? 2)}${summary.marketValueComplete ? "" : " (incomplete)"}`
            }.`
          : " Checking household file…"}
      </p>
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
      <nav className="nav" aria-label="Household screens">
        {navButton("income-plan", "Income Plan")}
        {navButton("calculator", "Calculator")}
        {navButton("position-details", "Position Details")}
        {navButton("dashboard", "Dashboard")}
        {navButton("holdings", "Holdings")}
        {navButton("new-investment", "New Investment")}
        {navButton("add-lot", "Add Lot")}
        {navButton("import", "Import")}
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
                      ? "New Investment"
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
          <div className="buttons">
            <button
              type="button"
              aria-label="Previous week"
              disabled={!asOfDate}
              onClick={() => setAsOfDate(shiftIso(asOfDate, -7))}
            >
              Previous week
            </button>
            <button
              type="button"
              aria-label="Next week"
              disabled={!asOfDate}
              onClick={() => setAsOfDate(shiftIso(asOfDate, 7))}
            >
              Next week
            </button>
          </div>
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
          <IncomePlanWeekPanel week={incomeWeek} selectedAccount={drillAccount} />
        </section>
      ) : null}

      {screen === "calculator" ? (
        <section aria-label="Calculator">
          <h2>Calculator</h2>
          <p>
            Plan × quantity for completed investments. Open Position Details to see price,
            declarations, Most Current, and Avg 6 for one symbol.
          </p>
          <CalculatorPanel rows={calculator?.rows ?? null} />
        </section>
      ) : null}

      {screen === "position-details" ? (
        <section aria-label="Position Details">
          <h2>Position Details</h2>
          <p>
            One row per symbol with joined quantity, cost, last price, yields, and completeness.
            Click a symbol to open the dossier. Owner settings save in place. Calculated rows stay
            calculated. Save or Cancel; other screens stay blocked while edits are unsaved.
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
          <section aria-label="Issuer retrieve miss summary">
            <ExceptionList exceptions={exceptions} />
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
                        ? row.lastRunMessage || "Issuer page empty — not using Yahoo."
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
                setPositionSymbol(symbol);
                void loadInvestment(symbol);
              });
            }}
          />
          <h3>Account splits</h3>
          <p>Account × symbol quantity and cost. Market value is qty × last price when valid.</p>
          <PositionDetailsTable
            positions={positionDetails}
            filter={positionSymbol}
          />
          <label>
            Symbol
            <select
              aria-label="Position symbol"
              value={positionSymbol}
              onChange={(e) => {
                const symbol = e.target.value;
                leaveWithoutSaving(() => {
                  setPositionSymbol(symbol);
                  void loadInvestment(symbol);
                });
              }}
            >
              <option value="">Choose a holding</option>
              {(securities.length > 0 ? securities : (calculator?.rows ?? []).map((row) => ({
                symbol: row.symbol,
              }))).map((row) => (
                <option key={row.symbol} value={row.symbol}>
                  {row.symbol}
                </option>
              ))}
            </select>
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
                  aria-label="Research ROC for this symbol"
                  disabled={busy || writesBlocked}
                  onClick={() => void researchPdRoc()}
                >
                  Research ROC
                </button>
              </div>
              <p aria-label="Issuer retrieve status">
                Source: {investment.template?.declarationSource || "unassigned"}. Freshness:{" "}
                {investment.declarationFreshness || "unavailable"}. Last run:{" "}
                {investment.template?.lastRunAt || "never"}
                {investment.template?.lastRunOk === true
                  ? " success"
                  : investment.template?.lastRunOk === false
                    ? ` miss: ${investment.template?.lastRunMessage || "Issuer page empty — not using Yahoo."}`
                    : ""}
                .
              </p>
              <p aria-label="Position ROC research status">
                ROC research: {investment.rocResearchStatus || "not-in-scope"}
                {investment.rocEstimateMethod
                  ? `. Estimate ${investment.rocEstimateMethod}${
                      investment.rocEstimateSourceUrl
                        ? ` from ${investment.rocEstimateSourceUrl}`
                        : ""
                    }${investment.rocEstimateAsOf ? ` as of ${investment.rocEstimateAsOf}` : ""}${
                      investment.rocEstimateEstablishedHow
                        ? ` (${investment.rocEstimateEstablishedHow})`
                        : ""
                    }`
                  : ""}
                .
              </p>
              <p aria-label="Lifetime distributions">
                Total distributions:{" "}
                {investment.distributionsScope === "incomplete" ||
                investment.totalDistributionsReceivedMinor == null
                  ? "unknown"
                  : formatUsd(investment.totalDistributionsReceivedMinor, investment.scale)}
                . ROC component:{" "}
                {investment.distributionsScope === "incomplete" ||
                investment.rocDistributionsMinor == null
                  ? "unknown"
                  : formatUsd(investment.rocDistributionsMinor, investment.scale)}
                . Cost recovery: {formatBps(investment.costRecoveryBps ?? null)}.
              </p>
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
                      <td>{formatBps(investment.mostCurrentVsPlanBps)}</td>
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
                        <input
                          aria-label="ROC 2025 actual"
                          value={pdDraft.roc2025}
                          onChange={(e) => patchDraft({ roc2025: e.target.value })}
                          disabled={busy || writesBlocked}
                        />
                      </td>
                      <td>editable</td>
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
                        <input
                          aria-label="ROC 2026 actual"
                          value={pdDraft.roc2026a}
                          onChange={(e) => patchDraft({ roc2026a: e.target.value })}
                          disabled={busy || writesBlocked}
                        />
                      </td>
                      <td>editable</td>
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
                        <label>
                          Price source
                          <input
                            aria-label="Price source"
                            value={pdDraft.priceSource}
                            onChange={(e) => patchDraft({ priceSource: e.target.value })}
                            disabled={busy || writesBlocked}
                          />
                        </label>
                        <label>
                          Source symbol
                          <input
                            aria-label="Source symbol"
                            value={pdDraft.sourceSymbol}
                            onChange={(e) => patchDraft({ sourceSymbol: e.target.value })}
                            disabled={busy || writesBlocked}
                          />
                        </label>
                        <label>
                          Declaration source
                          <input
                            aria-label="Declaration source"
                            value={pdDraft.declarationSource}
                            onChange={(e) => patchDraft({ declarationSource: e.target.value })}
                            disabled={busy || writesBlocked}
                          />
                        </label>
                        <label>
                          Lookback
                          <input
                            aria-label="Lookback count"
                            value={pdDraft.lookbackCount}
                            onChange={(e) => patchDraft({ lookbackCount: e.target.value })}
                            disabled={busy || writesBlocked}
                          />
                        </label>
                        <label>
                          Source URL
                          <input
                            aria-label="Source URL"
                            value={pdDraft.sourceUrl}
                            onChange={(e) => patchDraft({ sourceUrl: e.target.value })}
                            disabled={busy || writesBlocked}
                          />
                        </label>
                        <label>
                          Calendar policy
                          <select
                            aria-label="Calendar policy"
                            value={pdDraft.calendarPolicy}
                            onChange={(e) => patchDraft({ calendarPolicy: e.target.value })}
                            disabled={busy || writesBlocked}
                          >
                            <option value="">Choose policy</option>
                            <option value="issuer_calendar">issuer_calendar — published year dates</option>
                            <option value="derived_walk">derived_walk — from last pay + cadence</option>
                            <option value="none">none — does not pay</option>
                          </select>
                        </label>
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
                      </td>
                      <td>editable</td>
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
              <h3>Last 12 paid distributions</h3>
              <p>Paid $/share from the issuer page. Blank stays unknown, not $0.</p>
              {(investment.declarations ?? []).length === 0 ? (
                <p>No issuer declarations stored.</p>
              ) : (
                <div className="table-wrap">
                  <table aria-label="Stored declarations">
                    <thead>
                      <tr>
                        <th scope="col">Period</th>
                        <th className="numeric" scope="col">Per share</th>
                        <th scope="col">Source</th>
                      </tr>
                    </thead>
                    <tbody>
                      {(investment.declarations ?? []).map((row, i) => (
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
              <h3>Remaining-year income</h3>
              <p>
                Dates × Plan × quantity. Blank issuer amount is unknown, not $0. Do not mix
                with paid history.
              </p>
              {pdRemaining == null ? (
                <p>Loading remaining-year dates…</p>
              ) : (
                <div className="table-wrap">
                  <table aria-label="Remaining-year payment dates">
                    <thead>
                      <tr>
                        <th scope="col">Item</th>
                        <th scope="col">Value</th>
                      </tr>
                    </thead>
                    <tbody>
                      <tr>
                        <th scope="row">Schedule</th>
                        <td>
                          {pdRemaining.known
                            ? pdRemaining.provenance
                            : `${pdRemaining.provenance}. Unknown stays unknown.`}
                          {pdRemaining.calendarPolicy
                            ? ` Policy ${pdRemaining.calendarPolicy}.`
                            : ""}
                          {(pdRemaining.orphanedOverrides ?? []).length > 0
                            ? ` Orphaned owner date overrides (issuer calendar no longer has that original date): ${(pdRemaining.orphanedOverrides ?? [])
                                .map((o) => `${o.originalPayOn}→${o.payOn}`)
                                .join(", ")}.`
                            : ""}
                        </td>
                      </tr>
                      <tr>
                        <th scope="row">Year-to-go</th>
                        <td>
                          {pdRemaining.yearToGoMinor == null
                            ? "unknown"
                            : formatUsd(pdRemaining.yearToGoMinor, pdRemaining.scale)}
                        </td>
                      </tr>
                      {pdRemaining.months.map((m) => (
                        <tr key={m.month}>
                          <th scope="row">{m.month}</th>
                          <td>
                            {m.cashMinor == null
                              ? "unknown"
                              : formatUsd(m.cashMinor, pdRemaining.scale)}
                          </td>
                        </tr>
                      ))}
                      {pdRemaining.payments.map((pay) => (
                        <tr key={pay.originalPayOn}>
                          <th scope="row">{pay.payOn}</th>
                          <td>
                            {pay.cashMinor == null
                              ? "unknown"
                              : formatUsd(pay.cashMinor, pdRemaining.scale)}
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              )}
              <h3>This symbol</h3>
              <p>Lots and cost for the chosen ticker only. Household totals above stay put.</p>
              {investment.lots.length === 0 ? (
                <p>No open lots. Calculator omits this symbol until the first lot.</p>
              ) : (
                <SymbolLotsTable lots={investment.lots} />
              )}
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
            </>
          ) : (
            <p>Choose a symbol to load stored facts from FinanceClient.</p>
          )}
        </section>
      ) : null}

      {screen === "dashboard" ? (
        <section aria-label="Dashboard">
          <h2>Dashboard</h2>
          <DashboardBurndownPanel burndown={burndown} />
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
          <HoldingsPanel lots={holdings?.lots ?? null} filter={holdingsFilter} />
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
        <section aria-label="New Investment">
          <h2>New Investment</h2>
          <p>
            Step {wizStep} of 9. First define how this position will be maintained.
            Research then fills provider, source analytics, future-declaration method, and underlying.
            Retrieve runs after that. Yahoo is last price only. Missing stays unknown, never $0.
          </p>
          <div className="buttons">
            <button
              type="button"
              aria-label="Previous wizard step"
              disabled={busy || wizStep <= 1}
              onClick={() => setWizStep((s) => Math.max(1, s - 1))}
            >
              Back
            </button>
            <button
              type="button"
              aria-label="Next wizard step"
              disabled={busy || wizStep >= 9}
              onClick={() => void goWizardNext()}
            >
              Next
            </button>
          </div>
          <p aria-label="Wizard step guidance">
            {wizStep === 1
              ? "Name the ticker and confirm what this identity must keep current. Next does not retrieve."
              : wizStep === 2
                ? "Draft the standing template: price source, declaration source, lookback. Next researches issuer sites to fill provider, analytics, future-declaration method, and underlying."
                : wizStep === 3
                  ? "Review research. Confirm or override the issuer source. Next runs the first retrieve using this strategy. Yahoo is last price only."
                  : wizStep === 4
                    ? "Review issuer declarations, future pay dates, and Yahoo last price. Missing rows stay unknown. Then Next."
                    : wizStep === 5
                      ? "Confirm name, provider, underlying, frequency, and Risk. Next saves identity, the retrieval template, last price, and declarations. Not Plan, not ROC, not the lot."
                      : wizStep === 6
                        ? "Most Current / Avg 6 are calculated from issuer declarations. Type Plan and a reason, then Confirm Plan."
                        : wizStep === 7
                          ? "Next researches current-year ROC from the issuer 19a-1 when found. Unknown is not 0%."
                          : wizStep === 8
                            ? "Choose account, quantity, and original cost, then Open first lot. Remaining-year dates use issuer pay dates when retrieved."
                            : "Name Bull and Bear windows. The system does not pick dates."}
          </p>
          {wizDirty ? (
            <p className="blocked" role="status">
              Unsaved edits. Save or Cancel — other screens stay blocked.
            </p>
          ) : null}
          <div className="buttons dossier-actions">
            {wizStep === 3 || wizStep === 4 ? (
            <button
              type="button"
              aria-label="Retrieve from market"
              disabled={busy || writesBlocked}
              onClick={() => void retrieveFromMarket()}
            >
              Retrieve from market
            </button>
            ) : null}
            {wizStep === 5 ? (
            <button
              type="button"
              aria-label="Save new investment facts"
              disabled={busy || writesBlocked}
              onClick={() => void newInvestmentPart1()}
            >
              Save
            </button>
            ) : null}
            <button
              type="button"
              aria-label="Cancel new investment edits"
              disabled={busy || !wizDirty}
              onClick={() => cancelWizEdits()}
            >
              Cancel
            </button>
            {wizStep === 6 ? (
            <>
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
              disabled={busy || writesBlocked || !wizSecurityId}
              onClick={() => void confirmPlan()}
            >
              Confirm Plan
            </button>
            </>
            ) : null}
            {wizStep === 7 ? (
            <>
            <button
              type="button"
              aria-label="Research ROC"
              disabled={busy || writesBlocked || !wizSecurityId}
              onClick={() => void researchRoc()}
            >
              Research ROC
            </button>
            <button
              type="button"
              aria-label="Confirm ROC plan"
              disabled={busy || writesBlocked || !wizSecurityId}
              onClick={() => void confirmRocPlan()}
            >
              Confirm ROC plan
            </button>
            </>
            ) : null}
            {wizStep === 8 ? (
            <>
            <button
              type="button"
              aria-label="Open first lot"
              disabled={busy || writesBlocked || !wizSecurityId}
              onClick={() => void openFirstLot()}
            >
              Open first lot
            </button>
            <button
              type="button"
              aria-label="Save remaining payment dates"
              disabled={busy || writesBlocked || !wizSecurityId}
              onClick={() => void saveWizRemainingDates()}
            >
              Save remaining dates
            </button>
            </>
            ) : null}
            {wizStep === 9 ? (
            <>
            <button
              type="button"
              aria-label="Record bull period"
              disabled={busy || writesBlocked}
              onClick={() => void recordWizardPeriod("Bull")}
            >
              Record Bull dates
            </button>
            <button
              type="button"
              aria-label="Record bear period"
              disabled={busy || writesBlocked}
              onClick={() => void recordWizardPeriod("Bear")}
            >
              Record Bear dates
            </button>
            </>
            ) : null}
          </div>
          {wizRetrieveNote && showWiz.price ? <p>{wizRetrieveNote}</p> : null}
          {showWiz.strategy ? (
            <section aria-label="What we maintain">
              <h3>What this position must keep current</h3>
              <p>
                Define the maintenance streams before any retrieve. Research comes next and fills
                provider, issuer-page analytics, the future-declaration method, and underlying.
              </p>
              <table aria-label="Maintenance strategy">
                <thead>
                  <tr>
                    <th scope="col">Stream</th>
                    <th scope="col">How it stays current</th>
                  </tr>
                </thead>
                <tbody>
                  <tr>
                    <th scope="row">Last price</th>
                    <td>Public quote (Yahoo). Never used for declarations or provider.</td>
                  </tr>
                  <tr>
                    <th scope="row">Issuer declarations</th>
                    <td>Issuer site named by research. Lookback is observation count, not Yahoo dividends.</td>
                  </tr>
                  <tr>
                    <th scope="row">Future pay dates</th>
                    <td>Remaining-year calendar from issuer pay dates when the GET table is readable.</td>
                  </tr>
                  <tr>
                    <th scope="row">ROC</th>
                    <td>Current-year 19a-1 when published. Blank is unknown, not 0%.</td>
                  </tr>
                  <tr>
                    <th scope="row">Characteristics</th>
                    <td>Provider, underlying, frequency — researched, then owner-confirmed.</td>
                  </tr>
                  <tr>
                    <th scope="row">Broker actuals</th>
                    <td>From the account that holds the lot (import). Not a template field.</td>
                  </tr>
                </tbody>
              </table>
            </section>
          ) : null}
          {showWiz.template ? (
            <section aria-label="Standing retrieval template">
              <h3>Standing retrieval template</h3>
              <p>
                This is the how. Next researches issuer sites to fill blanks. It does not pull last
                price or post declarations yet.
              </p>
              <label>
                Price source
                <input
                  aria-label="Price source"
                  value={wizPriceSource}
                  onChange={(e) => setWizPriceSource(e.target.value)}
                  disabled={busy || writesBlocked}
                />
              </label>
              <label>
                Declaration source
                <select
                  aria-label="Declaration source"
                  value={wizDeclSource}
                  onChange={(e) => setWizDeclSource(e.target.value)}
                  disabled={busy || writesBlocked}
                >
                  <option value="">Unassigned — research will propose one</option>
                  <option value="roundhill">roundhill</option>
                  <option value="amplify">amplify</option>
                  <option value="neos">neos</option>
                  <option value="yieldmax">yieldmax</option>
                </select>
              </label>
              <label>
                Payment cadence
                <select
                  aria-label="New investment frequency"
                  value={wizFreq}
                  onChange={(e) => setWizFreq(e.target.value)}
                  disabled={busy || writesBlocked}
                >
                  <option value="">Choose cadence</option>
                  <option value="Weekly">Weekly (52)</option>
                  <option value="Monthly">Monthly (12)</option>
                  <option value="Quarterly">Quarterly (4)</option>
                  <option value="None">None (does not pay)</option>
                </select>
              </label>
              <label>
                Lookback (observations)
                <input
                  aria-label="Lookback count"
                  value={wizLookback}
                  onChange={(e) => setWizLookback(e.target.value)}
                  disabled={busy || writesBlocked}
                />
              </label>
              <label>
                Calendar policy
                <select
                  aria-label="Calendar policy"
                  value={wizCalendarPolicy}
                  onChange={(e) => setWizCalendarPolicy(e.target.value)}
                  disabled={busy || writesBlocked}
                >
                  <option value="">Choose how remaining-year dates are built</option>
                  <option value="issuer_calendar">issuer_calendar — published year dates</option>
                  <option value="derived_walk">derived_walk — from last pay + cadence</option>
                  <option value="none">none — does not pay</option>
                </select>
              </label>
            </section>
          ) : null}
          {showWiz.research ? (
            <section aria-label="Source research">
              <h3>Source research</h3>
              <p>
                Provider lookup, issuer-page analytics, future-declaration strategy, and
                characteristics. Yahoo is last price only. If a site cannot fill a fact, it stays
                unknown and the note explains why.
              </p>
              {wizAttempts.length === 0 ? (
                <p>No issuer attempts yet. Next on step 2 runs this research.</p>
              ) : (
                <table aria-label="Issuer site attempts">
                  <thead>
                    <tr>
                      <th scope="col">Site</th>
                      <th scope="col">Found</th>
                      <th scope="col">Note</th>
                    </tr>
                  </thead>
                  <tbody>
                    {wizAttempts.map((row) => (
                      <tr key={`${row.vendor}-${row.url}`}>
                        <td>
                          {row.vendor}{" "}
                          <a href={row.url} target="_blank" rel="noreferrer">
                            {row.url}
                          </a>
                        </td>
                        <td>{row.found ? "yes" : "no"}</td>
                        <td>{row.note}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}
              {wizAnalytics ? (
                <table aria-label="Source analytics">
                  <thead>
                    <tr>
                      <th scope="col">Issuer GET</th>
                      <th scope="col">Result</th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr>
                      <th scope="row">HTML returned</th>
                      <td>{wizAnalytics.htmlReturned ? "yes" : "no"}</td>
                    </tr>
                    <tr>
                      <th scope="row">Fund page</th>
                      <td>{wizAnalytics.fundPage ? "yes" : "no"}</td>
                    </tr>
                    <tr>
                      <th scope="row">Distribution table on GET</th>
                      <td>
                        {wizAnalytics.tableOnGet
                          ? `${formatCount(wizAnalytics.tableRowCount ?? 0)} rows`
                          : "empty"}
                      </td>
                    </tr>
                    <tr>
                      <th scope="row">JS-filled grid likely</th>
                      <td>{wizAnalytics.jsLikely ? "yes" : "no"}</td>
                    </tr>
                  </tbody>
                </table>
              ) : null}
              {wizFutureStrategy ? (
                <p aria-label="Future declaration strategy">{wizFutureStrategy}</p>
              ) : null}
              {wizMiss ? <p role="status">{wizMiss}</p> : null}
              {wizSourceUrl ? <p>Chosen page: {wizSourceUrl}</p> : null}
              <label>
                Declaration source
                <select
                  aria-label="Declaration source"
                  value={wizDeclSource}
                  onChange={(e) => setWizDeclSource(e.target.value)}
                  disabled={busy || writesBlocked}
                >
                  <option value="">Unassigned — do not use Yahoo dividends</option>
                  <option value="roundhill">roundhill</option>
                  <option value="amplify">amplify</option>
                  <option value="neos">neos</option>
                  <option value="yieldmax">yieldmax</option>
                </select>
              </label>
              <label>
                Provider
                <input
                  aria-label="New investment provider"
                  value={wizProvider}
                  onChange={(e) => setWizProvider(e.target.value)}
                  disabled={busy || writesBlocked}
                  placeholder="issuer/sponsor from the chosen site"
                />
              </label>
              <label>
                Underlying
                <input
                  aria-label="New investment underlying"
                  value={wizUnderlying}
                  onChange={(e) => setWizUnderlying(e.target.value)}
                  disabled={busy || writesBlocked}
                  placeholder="researched exposure ticker, blank if unknown"
                />
              </label>
            </section>
          ) : null}
          {wizRisk === "Risk On" ? (
            <p role="status">
              Risk On is your manual setting. High-distribution ETFs often return capital: treat
              ROC as unknown until researched, and do not auto-fill Plan from Avg 6.
            </p>
          ) : null}
          <div className="table-wrap">
          <table aria-label="Mandatory data checklist">
            <thead>
              <tr>
                <th scope="col">Fact</th>
                <th scope="col">Value</th>
                <th scope="col">Status</th>
              </tr>
            </thead>
            <tbody>
              {showWiz.symbol ? (
              <tr>
                <th scope="row">Symbol</th>
                <td>
                  <input
                    aria-label="New investment symbol"
                    value={wizSymbol}
                    onChange={(e) => setWizSymbol(e.target.value)}
                    disabled={busy || writesBlocked}
                  />
                </td>
                <td>{wizPart1Stored ? "stored" : wizSymbol ? "editable" : "missing"}</td>
              </tr>
              ) : null}
              {showWiz.identity ? (
              <>
              <tr>
                <th scope="row">Name</th>
                <td>
                  <input
                    aria-label="New investment name"
                    value={wizName}
                    onChange={(e) => setWizName(e.target.value)}
                    disabled={busy || writesBlocked}
                  />
                </td>
                <td>{wizPart1Stored ? "stored" : wizName ? "retrieved" : "missing"}</td>
              </tr>
              <tr>
                <th scope="row">Provider</th>
                <td>
                  <input
                    aria-label="New investment provider"
                    value={wizProvider}
                    onChange={(e) => setWizProvider(e.target.value)}
                    disabled={busy || writesBlocked}
                    placeholder="issuer/sponsor — not the instrument name"
                  />
                </td>
                <td>{wizProvider ? (wizPart1Stored ? "stored" : "retrieved") : "missing"}</td>
              </tr>
              <tr>
                <th scope="row">Underlying</th>
                <td>
                  <input
                    aria-label="New investment underlying"
                    value={wizUnderlying}
                    onChange={(e) => setWizUnderlying(e.target.value)}
                    disabled={busy || writesBlocked}
                    placeholder="exposure ticker — blank if unknown"
                  />
                </td>
                <td>{wizUnderlying ? (wizPart1Stored ? "stored" : "researched") : "unknown"}</td>
              </tr>
              </>
              ) : null}
              {showWiz.risk ? (
              <>
              <tr>
                <th scope="row">Risk</th>
                <td>
                  <select
                    aria-label="New investment risk"
                    value={wizRisk}
                    onChange={(e) => setWizRisk(e.target.value)}
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
                <td>{wizPart1Stored ? "stored" : wizRisk ? "owner" : "missing — owner must choose"}</td>
              </tr>
              <tr>
                <th scope="row">Frequency</th>
                <td>
                  <select
                    aria-label="New investment frequency"
                    value={wizFreq}
                    onChange={(e) => setWizFreq(e.target.value)}
                    disabled={busy || writesBlocked}
                  >
                    <option value="">Choose cadence</option>
                    <option value="Weekly">Weekly (52)</option>
                    <option value="Monthly">Monthly (12)</option>
                    <option value="Quarterly">Quarterly (4)</option>
                    <option value="None">None (does not pay)</option>
                  </select>
                </td>
                <td>
                  {wizPart1Stored
                    ? "stored"
                    : parseCadence(wizFreq)?.label === "None"
                      ? "None (does not pay)"
                      : parseCadence(wizFreq)
                        ? `${parseCadence(wizFreq)?.label} (${parseCadence(wizFreq)?.periods})`
                        : "missing — required to add"}
                </td>
              </tr>
              </>
              ) : null}
              {showWiz.price ? (
              <>
              <tr>
                <th scope="row">CurrentPrice</th>
                <td>
                  <input
                    aria-label="Record last price"
                    value={wizPrice}
                    onChange={(e) => setWizPrice(e.target.value)}
                    disabled={busy || writesBlocked}
                    placeholder="Yahoo last price only, or type override"
                  />
                </td>
                <td>
                  {wizPriceState?.priceMinor != null
                    ? `${wizPriceState.freshness} stored`
                    : wizPrice
                      ? "retrieved — not stored yet"
                      : "missing"}
                </td>
              </tr>
              <tr>
                <th scope="row">Declarations (12 lookback)</th>
                <td>
                  {formatCount(wizReview?.observationCount ?? wizDecls.length)} / 12
                  {wizDecls.length === 0 ? (
                    <>
                      <input
                        aria-label="Declaration source"
                        value={wizDeclSource}
                        onChange={(e) => setWizDeclSource(e.target.value)}
                        disabled={busy || writesBlocked}
                        placeholder="source"
                      />
                      <input
                        aria-label="Retrieve declarations"
                        value={wizDeclAmounts}
                        onChange={(e) => setWizDeclAmounts(e.target.value)}
                        disabled={busy || writesBlocked}
                        placeholder="up to 12 amounts, newest first"
                      />
                    </>
                  ) : null}
                </td>
                <td>
                  {(wizReview?.observationCount ?? wizDecls.length) > 0
                    ? wizPart1Stored
                      ? "stored"
                      : "retrieved"
                    : "missing — paste in this row"}
                </td>
              </tr>
              <tr>
                <th scope="row">Future pay dates</th>
                <td>
                  {wizUpcomingPays.length === 0
                    ? "none from issuer page — remaining-year stays unknown until researched"
                    : wizUpcomingPays.map((p) => (
                        <div key={p.payOn}>
                          {p.payOn}
                          {p.amountPerShareMinor == null
                            ? ""
                            : ` $${formatScaled(p.amountPerShareMinor, p.amountScale ?? 4)}`}
                        </div>
                      ))}
                </td>
                <td>
                  {wizUpcomingPays.length === 0
                    ? "unknown — not invented"
                    : "from issuer site"}
                </td>
              </tr>
              </>
              ) : null}
              {showWiz.review ? (
              <>
              <tr>
                <th scope="row">Most Current</th>
                <td>
                  {(wizReview?.mostCurrentMinor ?? wizDecls[0]?.amountPerShareMinor) == null
                    ? "N/A"
                    : `$${formatScaled(
                        wizReview?.mostCurrentMinor ?? wizDecls[0]?.amountPerShareMinor ?? 0,
                        wizReview?.amountScale ?? wizDecls[0]?.amountScale ?? 4,
                      )}`}
                </td>
                <td>
                  {(wizReview?.mostCurrentMinor ?? wizDecls[0]?.amountPerShareMinor) == null
                    ? "missing"
                    : "calculated"}
                </td>
              </tr>
              <tr>
                <th scope="row">Avg 6</th>
                <td>
                  {wizReview?.avg6Minor == null
                    ? "incomplete"
                    : `$${formatScaled(wizReview.avg6Minor, wizReview.amountScale)}`}
                </td>
                <td>{wizReview?.avg6Complete ? "calculated" : "incomplete"}</td>
              </tr>
              <tr>
                <th scope="row">Min / max / 80% of avg</th>
                <td>
                  {wizReview?.minMinor == null
                    ? "N/A until retrieve/save"
                    : `$${formatScaled(wizReview.minMinor, wizReview.amountScale)} / $${formatScaled(wizReview.maxMinor ?? 0, wizReview.amountScale)} / ${
                        wizReview.eightyPctOfAvgMinor == null
                          ? "N/A"
                          : `$${formatScaled(wizReview.eightyPctOfAvgMinor, wizReview.amountScale)}`
                      }`}
                </td>
                <td>{wizReview?.fullAnalysisPossible ? "calculated" : "blocked or incomplete"}</td>
              </tr>
              </>
              ) : null}
              {showWiz.plan ? (
              <>
              <tr>
                <th scope="row">Plan / share</th>
                <td>
                  <input
                    aria-label="Plan per share"
                    value={wizPlan}
                    onChange={(e) => setWizPlan(e.target.value)}
                    disabled={busy || writesBlocked}
                    placeholder={
                      wizDecls[0]
                        ? `Most Current ${formatScaled(wizDecls[0].amountPerShareMinor, wizDecls[0].amountScale ?? 4)} — type Plan`
                        : "Owner-controlled; not filled from Avg 6"
                    }
                  />
                </td>
                <td>{wizPlanStored ? "stored" : "editable"}</td>
              </tr>
              <tr>
                <th scope="row">Plan reason</th>
                <td>
                  <select
                    aria-label="Plan decision reason"
                    value={wizPlanReason}
                    onChange={(e) => setWizPlanReason(e.target.value)}
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
                <td>{wizPlanStored ? "stored" : "editable"}</td>
              </tr>
              {(wizReview?.incompleteReasonRequired ||
                (wizDecls.length > 0 && wizDecls.length < 6)) && (
                <tr>
                  <th scope="row">Incomplete analysis</th>
                  <td>
                    <select
                      aria-label="Incomplete analysis reason"
                      value={wizIncomplete}
                      onChange={(e) => setWizIncomplete(e.target.value)}
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
              )}
              </>
              ) : null}
              {showWiz.roc ? (
              <>
              <tr>
                <th scope="row">ROC percent</th>
                <td>
                  <input
                    aria-label="ROC percent"
                    value={wizRocPct}
                    onChange={(e) => setWizRocPct(e.target.value)}
                    disabled={busy || writesBlocked}
                    placeholder="system-filled when researched; override if needed"
                  />
                </td>
                <td>
                  {wizRoc?.complete
                    ? wizRoc.ownerOverride
                      ? "owner override"
                      : "system"
                    : "unknown is not 0%"}
                </td>
              </tr>
              <tr>
                <th scope="row">ROC how / where</th>
                <td>
                  {wizRoc
                    ? `${wizRoc.establishedHow || wizRoc.reason}${wizRoc.method ? ` via ${wizRoc.method}` : ""}${wizRoc.asOf ? ` as of ${wizRoc.asOf}` : ""}${wizRoc.sourceUrl ? ` ${wizRoc.sourceUrl}` : ""}`
                    : "not researched"}
                </td>
                <td>calculated</td>
              </tr>
              </>
              ) : null}
              {showWiz.lot ? (
              <>
              <tr>
                <th scope="row">First lot account</th>
                <td>
                  <select
                    aria-label="First lot account"
                    value={wizAccountId}
                    onChange={(e) => setWizAccountId(e.target.value)}
                    disabled={busy || writesBlocked}
                  >
                    <option value="">Choose account</option>
                    {accounts.map((a) => (
                      <option key={a.accountId} value={a.accountId}>
                        {a.name}
                      </option>
                    ))}
                  </select>
                </td>
                <td>{wizLotStored ? "stored" : "editable"}</td>
              </tr>
              <tr>
                <th scope="row">Quantity</th>
                <td>
                  <input
                    aria-label="First lot quantity"
                    value={wizQty}
                    onChange={(e) => setWizQty(e.target.value)}
                    disabled={busy || writesBlocked}
                  />
                </td>
                <td>{wizLotStored ? "stored" : "editable"}</td>
              </tr>
              <tr>
                <th scope="row">Original cost</th>
                <td>
                  <input
                    aria-label="First lot original cost"
                    value={wizCost}
                    onChange={(e) => setWizCost(e.target.value)}
                    disabled={busy || writesBlocked}
                  />
                </td>
                <td>{wizLotStored ? "stored" : "editable"}</td>
              </tr>
              <tr>
                <th scope="row">Remaining-year schedule</th>
                <td>
                  {!wizSecurityId
                    ? "Save Part 1 first."
                    : wizRemaining == null
                      ? "loading"
                      : wizRemaining.known
                        ? `${wizRemaining.provenance}${wizRemaining.hypothetical ? " — planned if this quantity is opened" : ""}`
                        : `${wizRemaining.provenance}. Name the next payment date; do not invent $0.`}
                </td>
                <td>{wizRemaining?.known ? "calculated" : "unknown"}</td>
              </tr>
              <tr>
                <th scope="row">Year-to-go planned income</th>
                <td>
                  {wizRemaining?.yearToGoMinor == null
                    ? "unknown"
                    : formatUsd(wizRemaining.yearToGoMinor, wizRemaining.scale)}
                </td>
                <td>{wizRemaining?.planKnown ? "Plan × qty" : "need confirmed Plan"}</td>
              </tr>
              <tr>
                <th scope="row">Monthly planned income</th>
                <td>
                  {monthTotalsFromPays(
                    wizPayDraft.map((d) => {
                      const src = wizRemaining?.payments.find(
                        (p) => p.originalPayOn === d.originalPayOn,
                      );
                      return {
                        payOn: d.payOn,
                        cashMinor: src?.cashMinor ?? null,
                      };
                    }),
                  ).map((m) => (
                    <div key={m.month}>
                      {m.month}: {m.cashMinor == null ? "unknown" : formatUsd(m.cashMinor, 2)}
                    </div>
                  ))}
                  {wizRemaining?.known && wizPayDraft.length === 0 ? "unknown" : null}
                </td>
                <td>months with a payment only</td>
              </tr>
              <tr>
                <th scope="row">Remaining payment dates</th>
                <td>
                  {wizRemaining != null && !wizRemaining.known ? (
                    <input
                      aria-label="Next payment date"
                      type="date"
                      value={wizNextPay}
                      onChange={(e) => setWizNextPay(e.target.value)}
                      disabled={busy || writesBlocked || !wizSecurityId}
                    />
                  ) : (
                    <table aria-label="Remaining-year payment dates">
                      <thead>
                        <tr>
                          <th scope="col">Pay date</th>
                          <th scope="col">Cash</th>
                        </tr>
                      </thead>
                      <tbody>
                        {wizPayDraft.map((pay) => {
                          const src = wizRemaining?.payments.find(
                            (p) => p.originalPayOn === pay.originalPayOn,
                          );
                          return (
                            <tr key={pay.originalPayOn}>
                              <td>
                                <input
                                  type="date"
                                  aria-label={`Remaining pay date ${pay.originalPayOn}`}
                                  value={pay.payOn}
                                  onChange={(e) =>
                                    setWizPayDraft((prev) =>
                                      prev.map((row) =>
                                        row.originalPayOn === pay.originalPayOn
                                          ? { ...row, payOn: e.target.value }
                                          : row,
                                      ),
                                    )
                                  }
                                  disabled={busy || writesBlocked}
                                />
                              </td>
                              <td>
                                {src?.cashMinor == null
                                  ? "unknown"
                                  : formatUsd(src.cashMinor, wizRemaining?.scale ?? 2)}
                              </td>
                            </tr>
                          );
                        })}
                      </tbody>
                      <tfoot>
                        <tr>
                          <td>Year-to-go</td>
                          <td>
                            {wizRemaining?.yearToGoMinor == null
                              ? "unknown"
                              : formatUsd(wizRemaining.yearToGoMinor, wizRemaining.scale)}
                          </td>
                        </tr>
                      </tfoot>
                    </table>
                  )}
                </td>
                <td>editable in the same row</td>
              </tr>
              </>
              ) : null}
              {showWiz.regime ? (
              <>
              <tr>
                <th scope="row">Bull start / end</th>
                <td>
                  <input
                    type="date"
                    aria-label="Bull start"
                    value={wizBullStart}
                    onChange={(e) => setWizBullStart(e.target.value)}
                    disabled={busy || writesBlocked}
                  />
                  <input
                    type="date"
                    aria-label="Bull end"
                    value={wizBullEnd}
                    onChange={(e) => setWizBullEnd(e.target.value)}
                    disabled={busy || writesBlocked}
                  />
                </td>
                <td>{wizBullStored ? "stored" : "owner names dates"}</td>
              </tr>
              <tr>
                <th scope="row">Bear start / end</th>
                <td>
                  <input
                    type="date"
                    aria-label="Bear start"
                    value={wizBearStart}
                    onChange={(e) => setWizBearStart(e.target.value)}
                    disabled={busy || writesBlocked}
                  />
                  <input
                    type="date"
                    aria-label="Bear end"
                    value={wizBearEnd}
                    onChange={(e) => setWizBearEnd(e.target.value)}
                    disabled={busy || writesBlocked}
                  />
                </td>
                <td>{wizBearStored ? "stored" : "owner names dates"}</td>
              </tr>
              </>
              ) : null}
            </tbody>
          </table>
          </div>
          {wizDecls.length > 0 && (showWiz.price || showWiz.review) ? (
            <div className="table-wrap">
              <p>Last {formatCount(wizDecls.length)} declarations from the market (newest first).</p>
              <table>
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
        </section>
      ) : null}

      {screen === "add-lot" ? (
        <section aria-label="Add Lot">
          <h2>Add Lot</h2>
          <p>
            Existing symbol only. Quantity and cost are edited in the same row that shows them.
            Remaining-year payment dates and monthly/year-to-go Plan cash use this lot and the
            position after add. Save or Cancel; other screens stay blocked while this row is dirty.
          </p>
          {addLotDirty ? (
            <p className="blocked" role="status">
              Unsaved edits. Save or Cancel — other screens stay blocked.
            </p>
          ) : null}
          <div className="buttons dossier-actions">
            <button
              type="button"
              aria-label="Add Lot"
              disabled={busy || writesBlocked}
              onClick={() => void openAddLot()}
            >
              Save
            </button>
            <button
              type="button"
              aria-label="Cancel add lot edits"
              disabled={busy || !addLotDirty}
              onClick={() => cancelAddLotEdits()}
            >
              Cancel
            </button>
            <button
              type="button"
              aria-label="Save remaining payment dates"
              disabled={busy || writesBlocked || !addLotSecurityId}
              onClick={() => void saveAddLotRemainingDates()}
            >
              Save remaining dates
            </button>
          </div>
          <div className="table-wrap">
            <table aria-label="Add lot dossier">
              <thead>
                <tr>
                  <th scope="col">Fact</th>
                  <th scope="col">Value</th>
                  <th scope="col">Status</th>
                </tr>
              </thead>
              <tbody>
                <tr>
                  <th scope="row">Symbol</th>
                  <td>
                    <select
                      aria-label="Add lot symbol"
                      value={addLotSecurityId}
                      onChange={(e) => setAddLotSecurityId(e.target.value)}
                      disabled={busy || writesBlocked}
                    >
                      <option value="">Choose existing holding</option>
                      {securities
                        .filter((s) => (calculator?.rows ?? []).some((r) => r.symbol === s.symbol))
                        .map((s) => (
                          <option key={s.securityId} value={s.securityId}>
                            {s.symbol}
                          </option>
                        ))}
                    </select>
                  </td>
                  <td>editable</td>
                </tr>
                <tr>
                  <th scope="row">Account</th>
                  <td>
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
                  </td>
                  <td>editable</td>
                </tr>
                <tr>
                  <th scope="row">Quantity</th>
                  <td>
                    <input
                      aria-label="Add lot quantity"
                      value={addLotQty}
                      onChange={(e) => setAddLotQty(e.target.value)}
                      disabled={busy || writesBlocked}
                    />
                  </td>
                  <td>editable</td>
                </tr>
                <tr>
                  <th scope="row">Original cost</th>
                  <td>
                    <input
                      aria-label="Add lot original cost"
                      value={addLotCost}
                      onChange={(e) => setAddLotCost(e.target.value)}
                      disabled={busy || writesBlocked}
                    />
                  </td>
                  <td>editable</td>
                </tr>
                <tr>
                  <th scope="row">Remaining-year schedule</th>
                  <td>
                    {!addLotSecurityId
                      ? "Choose a holding first."
                      : addLotRemaining == null
                        ? "loading"
                        : addLotRemaining.known
                          ? `${addLotRemaining.provenance}${addLotRemaining.hypothetical ? " — planned if this quantity is opened" : ""}`
                          : `${addLotRemaining.provenance}. Name the next payment date; do not invent $0.`}
                  </td>
                  <td>{addLotRemaining?.known ? "calculated" : "unknown"}</td>
                </tr>
                <tr>
                  <th scope="row">Year-to-go planned income</th>
                  <td>
                    {addLotRemaining?.thisLotYearToGoMinor == null
                      ? "unknown"
                      : `This lot ${formatUsd(addLotRemaining.thisLotYearToGoMinor, addLotRemaining.scale)}`}
                    {addLotRemaining?.positionAfterYearToGoMinor == null
                      ? ""
                      : `; position after add ${formatUsd(addLotRemaining.positionAfterYearToGoMinor, addLotRemaining.scale)}`}
                  </td>
                  <td>this lot vs position after add</td>
                </tr>
                <tr>
                  <th scope="row">Monthly planned income</th>
                  <td>
                    {monthTotalsFromPays(
                      addLotPayDraft.map((d) => {
                        const src = addLotRemaining?.payments.find(
                          (p) => p.originalPayOn === d.originalPayOn,
                        );
                        return {
                          payOn: d.payOn,
                          cashMinor: src?.positionAfterCashMinor ?? src?.cashMinor ?? null,
                          thisLotCashMinor: src?.thisLotCashMinor ?? null,
                          positionAfterCashMinor: src?.positionAfterCashMinor ?? null,
                        };
                      }),
                    ).map((m) => (
                      <div key={m.month}>
                        {m.month}: this lot{" "}
                        {m.thisLotCashMinor == null ? "unknown" : formatUsd(m.thisLotCashMinor, 2)}
                        {"; after add "}
                        {m.positionAfterCashMinor == null
                          ? "unknown"
                          : formatUsd(m.positionAfterCashMinor, 2)}
                      </div>
                    ))}
                  </td>
                  <td>months with a payment only</td>
                </tr>
                <tr>
                  <th scope="row">Remaining payment dates</th>
                  <td>
                    {addLotRemaining != null && !addLotRemaining.known ? (
                      <input
                        aria-label="Next payment date"
                        type="date"
                        value={addLotNextPay}
                        onChange={(e) => setAddLotNextPay(e.target.value)}
                        disabled={busy || writesBlocked || !addLotSecurityId}
                      />
                    ) : (
                      <table aria-label="Remaining-year payment dates">
                        <thead>
                          <tr>
                            <th scope="col">Pay date</th>
                            <th scope="col">This lot</th>
                            <th scope="col">After add</th>
                          </tr>
                        </thead>
                        <tbody>
                          {addLotPayDraft.map((pay) => {
                            const src = addLotRemaining?.payments.find(
                              (p) => p.originalPayOn === pay.originalPayOn,
                            );
                            return (
                              <tr key={pay.originalPayOn}>
                                <td>
                                  <input
                                    type="date"
                                    aria-label={`Remaining pay date ${pay.originalPayOn}`}
                                    value={pay.payOn}
                                    onChange={(e) =>
                                      setAddLotPayDraft((prev) =>
                                        prev.map((row) =>
                                          row.originalPayOn === pay.originalPayOn
                                            ? { ...row, payOn: e.target.value }
                                            : row,
                                        ),
                                      )
                                    }
                                    disabled={busy || writesBlocked}
                                  />
                                </td>
                                <td>
                                  {src?.thisLotCashMinor == null
                                    ? "unknown"
                                    : formatUsd(src.thisLotCashMinor, addLotRemaining?.scale ?? 2)}
                                </td>
                                <td>
                                  {src?.positionAfterCashMinor == null
                                    ? "unknown"
                                    : formatUsd(
                                        src.positionAfterCashMinor,
                                        addLotRemaining?.scale ?? 2,
                                      )}
                                </td>
                              </tr>
                            );
                          })}
                        </tbody>
                        <tfoot>
                          <tr>
                            <td>Year-to-go</td>
                            <td>
                              {addLotRemaining?.thisLotYearToGoMinor == null
                                ? "unknown"
                                : formatUsd(
                                    addLotRemaining.thisLotYearToGoMinor,
                                    addLotRemaining.scale,
                                  )}
                            </td>
                            <td>
                              {addLotRemaining?.positionAfterYearToGoMinor == null
                                ? "unknown"
                                : formatUsd(
                                    addLotRemaining.positionAfterYearToGoMinor,
                                    addLotRemaining.scale,
                                  )}
                            </td>
                          </tr>
                        </tfoot>
                      </table>
                    )}
                  </td>
                  <td>editable in the same row</td>
                </tr>
              </tbody>
            </table>
          </div>
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
            <ExceptionList exceptions={exceptions} />
          </section>
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
