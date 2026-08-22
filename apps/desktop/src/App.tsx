import { useCallback, useEffect, useState } from "react";
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
  type PlanReviewGet,
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
  formatCount,
  formatScaled,
  formatUsd,
} from "@finos/ui-components";
import "./App.css";

const RISK_TIERS = ["Foundation", "Core", "Risk On"];
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
  const [wizRisk, setWizRisk] = useState("");
  const [wizFreq, setWizFreq] = useState("Weekly");
  const [wizSecurityId, setWizSecurityId] = useState("");
  const [wizPrice, setWizPrice] = useState("");
  const [wizDeclSource, setWizDeclSource] = useState("provider-site");
  const [wizDeclAmounts, setWizDeclAmounts] = useState("");
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
  const [positionDetails, setPositionDetails] = useState<PositionDetailsGet | null>(null);
  const [positionSymbol, setPositionSymbol] = useState("");
  const [investment, setInvestment] = useState<InvestmentGet | null>(null);
  const [wizPart1Stored, setWizPart1Stored] = useState(false);
  const [wizPlanStored, setWizPlanStored] = useState(false);
  const [wizLotStored, setWizLotStored] = useState(false);
  const [pdName, setPdName] = useState("");
  const [pdProvider, setPdProvider] = useState("");
  const [pdRisk, setPdRisk] = useState("");
  const [pdFreq, setPdFreq] = useState("");
  const [pdPlan, setPdPlan] = useState("");
  const [pdPlanReason, setPdPlanReason] = useState("");
  const [pdIncomplete, setPdIncomplete] = useState("");

  const refreshHousehold = useCallback(async (asOf: string) => {
    const [weekResult, burnResult, holdingsResult, exceptionResult, summaryResult, calcResult, accountResult, securityResult, positionResult] =
      await Promise.all([
        client.executeQuery("IncomePlanWeekGet", { asOfDate: asOf }),
        client.executeQuery("DashboardBurndownGet", { asOfDate: asOf }),
        client.executeQuery("HoldingsGet"),
        client.executeQuery("ExceptionList"),
        client.executeQuery("HouseholdSummaryGet"),
        client.executeQuery("CalculatorGet"),
        client.executeQuery("AccountList"),
        client.executeQuery("SecurityList"),
        client.executeQuery("PositionDetailsGet"),
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

  const loadInvestment = useCallback(async (symbol: string) => {
    if (!symbol) {
      setInvestment(null);
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
      setInvestment(body);
      setPdName(body.name);
      setPdProvider(body.provider);
      setPdRisk(body.riskTier);
      setPdFreq(body.paymentFrequency);
      setPdPlan(
        body.planKnown ? scaledDollars(body.planPerShareMinor, body.planScale) : "",
      );
      setPdPlanReason(body.planReason);
      setPdIncomplete("");
    } catch {
      setInvestment(null);
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

  const periodsForFreq = (freq: string) => {
    const n = freq.toLowerCase();
    if (n === "weekly") return 52;
    if (n === "quarterly") return 4;
    return 12;
  };

  const applyMarketBody = (raw: string) => {
    const body = JSON.parse(raw) as {
      name?: string;
      suggestedFrequency?: string;
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
    if (body.suggestedFrequency) {
      setWizFreq(body.suggestedFrequency);
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
    if (decls[0]?.source) {
      setWizDeclSource(decls[0].source);
    }
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
        ? `price ${formatUsd(body.quote.priceMinor, body.quote.scale ?? 2)} from ${body.quote.source}`
        : "no live price";
    const note = `Retrieved ${formatCount(n)} declarations; ${quoteBit}. Plan is still yours to confirm.`;
    setWizRetrieveNote(note);
    return { body, n, hasQuote: Boolean(body.quote?.priceMinor && body.quote.priceMinor > 0) };
  };

  const retrieveFromMarket = async () => {
    const symbol = wizSymbol.trim().toUpperCase();
    if (!symbol) {
      setActionMessage("Enter a ticker, then retrieve from the market.");
      return;
    }
    setBusy(true);
    setActionMessage(null);
    try {
      const result = await client.executeCommand("MarketRetrieve", { symbol });
      if (!result.ok || !result.bodyJson) {
        setActionMessage(`Market retrieve failed: ${result.errorCode ?? "error"}`);
        return;
      }
      const { n, hasQuote } = applyMarketBody(result.bodyJson);
      if (!hasQuote && n === 0) {
        setActionMessage("Market retrieve missed. Enter current price and declarations below.");
      } else {
        setActionMessage(
          `Retrieved ${formatCount(n)} declarations${hasQuote ? " and a live price" : ""}. Confirm Plan, then open the first lot.`,
        );
      }
    } catch (err: unknown) {
      setActionMessage(String(err));
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
        suggestedFrequency?: string;
      } = {};
      const marketResult = await client.executeCommand("MarketRetrieve", {
        symbol: wizSymbol.trim().toUpperCase(),
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
        priceSource: "public",
        sourceSymbol: wizSymbol.trim().toUpperCase(),
        declarationSource: market.candidates?.[0]?.source ?? wizDeclSource,
        lookbackCount: 12,
        paymentSource: "import",
      });
      await client.executeCommand("PositionCharacteristicUpsert", {
        securityId,
        paymentFrequency: market.suggestedFrequency ?? wizFreq,
        riskTier: wizRisk,
        provider: wizProvider,
        underlying: "",
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
      const planScale = 4;
      const planMinor = Math.round(Number(wizPlan) * 10 ** planScale);
      const result = await client.executeCommand("PlanHistoryConfirm", {
        securityId: wizSecurityId,
        amountPerShareMinor: planMinor,
        amountScale: planScale,
        planningPeriodsPerYear: periodsForFreq(wizFreq),
        effectiveFrom: new Date().toISOString().slice(0, 10),
        decisionReason: wizPlanReason,
        incompleteAnalysisReason: wizIncomplete,
      });
      if (!result.ok) {
        setActionMessage(`Plan not stored: ${result.errorCode ?? "error"}`);
        return;
      }
      setWizPlanStored(true);
      setActionMessage(
        `Stored Plan ${wizPlan}/share (${wizPlanReason}). Open the first lot so Calculator can show this symbol.`,
      );
      await refreshHousehold(asOfDate);
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
      const symbol = wizSymbol.trim().toUpperCase();
      setActionMessage(
        `Stored first lot for ${symbol}. Open Position Details or Calculator to review.`,
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

  const saveStoredFacts = async () => {
    if (!investment) {
      setActionMessage("Choose a symbol first.");
      return;
    }
    if (!RISK_TIERS.includes(pdRisk)) {
      setActionMessage("Choose Foundation, Core, or Risk On before saving.");
      return;
    }
    setBusy(true);
    setActionMessage(null);
    try {
      const renamed = await client.executeCommand("SecurityUpdate", {
        securityId: investment.securityId,
        name: pdName.trim(),
      });
      if (!renamed.ok) {
        setActionMessage(`Name not stored: ${renamed.errorCode ?? "error"}`);
        return;
      }
      const chars = await client.executeCommand("PositionCharacteristicUpsert", {
        securityId: investment.securityId,
        paymentFrequency: pdFreq,
        riskTier: pdRisk,
        provider: pdProvider.trim(),
        underlying: investment.underlying,
      });
      if (!chars.ok) {
        setActionMessage(`Characteristics not stored: ${chars.errorCode ?? "error"}`);
        return;
      }
      setActionMessage(
        `Stored ${investment.symbol}: name, provider ${pdProvider.trim() || "(blank)"}, ${pdRisk}, ${pdFreq}.`,
      );
      await refreshHousehold(asOfDate);
      await loadInvestment(investment.symbol);
    } catch (err: unknown) {
      setActionMessage(String(err));
    } finally {
      setBusy(false);
    }
  };

  const confirmStoredPlan = async () => {
    if (!investment) {
      setActionMessage("Choose a symbol first.");
      return;
    }
    if (!pdPlanReason) {
      setActionMessage("Choose a Plan reason.");
      return;
    }
    setBusy(true);
    setActionMessage(null);
    try {
      const planScale = 4;
      const planMinor = Math.round(Number(pdPlan) * 10 ** planScale);
      const result = await client.executeCommand("PlanHistoryConfirm", {
        securityId: investment.securityId,
        amountPerShareMinor: planMinor,
        amountScale: planScale,
        planningPeriodsPerYear: periodsForFreq(pdFreq || investment.paymentFrequency),
        effectiveFrom: new Date().toISOString().slice(0, 10),
        decisionReason: pdPlanReason,
        incompleteAnalysisReason: pdIncomplete,
      });
      if (!result.ok) {
        setActionMessage(`Plan not stored: ${result.errorCode ?? "error"}`);
        return;
      }
      setActionMessage(`Stored Plan ${pdPlan}/share for ${investment.symbol} (${pdPlanReason}).`);
      await refreshHousehold(asOfDate);
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

  const navButton = (id: Screen, label: string) => (
    <button
      type="button"
      aria-label={label}
      aria-current={screen === id ? "page" : undefined}
      onClick={() => setScreen(id)}
    >
      {label}
    </button>
  );

  return (
    <main className="container" aria-label="finos">
      <h1>finos</h1>
      <p>
        Household stays on this machine after seed. No daily reload.
        {summary
          ? ` Loaded: ${formatCount(summary.accountCount)} accounts, ${formatCount(summary.openLotCount)} open lots, ${formatCount(summary.yieldCount)} yield, ${formatCount(summary.disbursementCount)} disbursement, ${formatCount(summary.planCount)} Calculator plans${summary.latestYieldOn ? `, last yield ${summary.latestYieldOn}` : ""}.`
          : " Checking household file…"}
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
            Identity is ticker plus instrument name (what retrieve stored). Provider is a
            separate owner fact and stays blank until you type it. Risk was not confirmed on
            the first wizard; change it here. Quantity and cost come from lots.
          </p>
          <label>
            Symbol
            <select
              aria-label="Position symbol"
              value={positionSymbol}
              onChange={(e) => {
                const symbol = e.target.value;
                setPositionSymbol(symbol);
                void loadInvestment(symbol);
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
          {investment ? (
            <>
            <dl className="health" aria-label="Stored investment facts">
              <dt>Identity</dt>
              <dd>
                {investment.symbol} — {investment.name || "unnamed"}
              </dd>
              <dt>Risk</dt>
              <dd>{investment.riskTier || "missing"}</dd>
              <dt>Frequency</dt>
              <dd>{investment.paymentFrequency || "missing"}</dd>
              <dt>Provider</dt>
              <dd>{investment.provider || "not stored"}</dd>
              <dt>CurrentPrice</dt>
              <dd>
                {investment.price.priceMinor != null
                  ? `${formatUsd(investment.price.priceMinor, investment.price.scale)} (${investment.price.freshness})`
                  : `unavailable (${investment.price.freshness})`}
              </dd>
              <dt>Price-derived valid</dt>
              <dd>{investment.price.priceDerivedValid ? "yes" : "no"}</dd>
              <dt>Declarations stored</dt>
              <dd>{formatCount(investment.declarationCount)} / 12</dd>
              <dt>Most Current</dt>
              <dd>
                {investment.review.mostCurrentMinor == null
                  ? "N/A"
                  : `$${formatScaled(investment.review.mostCurrentMinor, investment.review.amountScale)}`}
              </dd>
              <dt>Avg 6</dt>
              <dd>
                {investment.review.avg6Minor == null
                  ? "incomplete"
                  : `$${formatScaled(investment.review.avg6Minor, investment.review.amountScale)}`}
              </dd>
              <dt>Min / max / 80% of avg</dt>
              <dd>
                {investment.review.minMinor == null
                  ? "N/A"
                  : `$${formatScaled(investment.review.minMinor, investment.review.amountScale)} / $${formatScaled(investment.review.maxMinor ?? 0, investment.review.amountScale)} / ${
                      investment.review.eightyPctOfAvgMinor == null
                        ? "N/A"
                        : `$${formatScaled(investment.review.eightyPctOfAvgMinor, investment.review.amountScale)}`
                    }`}
              </dd>
              <dt>Plan</dt>
              <dd>
                {investment.planKnown
                  ? `$${formatScaled(investment.planPerShareMinor, investment.planScale)} (${investment.planReason || "no reason"}) from ${investment.planEffectiveFrom}`
                  : "unknown — not confirmed"}
              </dd>
              <dt>Open qty / original cost</dt>
              <dd>
                {formatScaled(investment.remainingQuantityMinor, investment.quantityScale)} /{" "}
                {formatUsd(investment.remainingPerformanceMinor, investment.scale)}
              </dd>
              <dt>Retrieval template</dt>
              <dd>
                {investment.template
                  ? `price ${investment.template.priceSource || "—"}, declarations ${investment.template.declarationSource || "—"}, lookback ${formatCount(investment.template.lookbackCount)}`
                  : "missing"}
              </dd>
              <dt>First lot</dt>
              <dd>{investment.firstLotComplete ? "stored" : "missing — Calculator omits this symbol"}</dd>
              <dt>ROC 2025</dt>
              <dd>
                {investment.rocPct2025ActualMinor == null || investment.rocScale == null
                  ? "unknown"
                  : `${formatScaled(investment.rocPct2025ActualMinor, investment.rocScale)}%`}
              </dd>
            </dl>
            <h3>Correct stored facts</h3>
            <label>
              Name
              <input
                aria-label="Position name"
                value={pdName}
                onChange={(e) => setPdName(e.target.value)}
                disabled={busy || writesBlocked}
              />
            </label>
            <label>
              Provider
              <input
                aria-label="Position provider"
                value={pdProvider}
                onChange={(e) => setPdProvider(e.target.value)}
                disabled={busy || writesBlocked}
              />
            </label>
            <label>
              Risk
              <select
                aria-label="Position risk"
                value={pdRisk}
                onChange={(e) => setPdRisk(e.target.value)}
                disabled={busy || writesBlocked}
              >
                <option value="">Choose owner tier</option>
                {RISK_TIERS.map((tier) => (
                  <option key={tier} value={tier}>
                    {tier}
                  </option>
                ))}
              </select>
            </label>
            <label>
              Frequency
              <select
                aria-label="Position frequency"
                value={pdFreq}
                onChange={(e) => setPdFreq(e.target.value)}
                disabled={busy || writesBlocked}
              >
                <option value="">Choose frequency</option>
                <option>Weekly</option>
                <option>Monthly</option>
                <option>Quarterly</option>
              </select>
            </label>
            <button
              type="button"
              aria-label="Save stored facts"
              disabled={busy || writesBlocked}
              onClick={() => void saveStoredFacts()}
            >
              Save identity and characteristics
            </button>
            <div className="buttons">
              <button
                type="button"
                aria-label="Use Most Current as stored Plan"
                disabled={
                  busy || writesBlocked || investment.review.mostCurrentMinor == null
                }
                onClick={() => {
                  if (investment.review.mostCurrentMinor == null) return;
                  setPdPlan(
                    scaledDollars(
                      investment.review.mostCurrentMinor,
                      investment.review.amountScale,
                    ),
                  );
                  setPdPlanReason("Match Most Current");
                }}
              >
                Use Most Current as Plan
              </button>
              <button
                type="button"
                aria-label="Use Avg 6 as stored Plan"
                disabled={busy || writesBlocked || investment.review.avg6Minor == null}
                onClick={() => {
                  if (investment.review.avg6Minor == null) return;
                  setPdPlan(
                    scaledDollars(investment.review.avg6Minor, investment.review.amountScale),
                  );
                  setPdPlanReason("Match Avg 6 (owner typed)");
                }}
              >
                Use Avg 6 as Plan
              </button>
            </div>
            <label>
              Plan per share
              <input
                aria-label="Stored Plan per share"
                value={pdPlan}
                onChange={(e) => setPdPlan(e.target.value)}
                disabled={busy || writesBlocked}
              />
            </label>
            <label>
              Plan reason
              <select
                aria-label="Stored Plan decision reason"
                value={pdPlanReason}
                onChange={(e) => setPdPlanReason(e.target.value)}
                disabled={busy || writesBlocked}
              >
                <option value="">Choose reason</option>
                {PLAN_REASONS.map((reason) => (
                  <option key={reason} value={reason}>
                    {reason}
                  </option>
                ))}
              </select>
            </label>
            {investment.review.incompleteReasonRequired ? (
              <label>
                Incomplete analysis reason
                <select
                  aria-label="Stored incomplete analysis reason"
                  value={pdIncomplete}
                  onChange={(e) => setPdIncomplete(e.target.value)}
                  disabled={busy || writesBlocked}
                >
                  <option value="">Choose why full analysis is not possible</option>
                  {INCOMPLETE_REASONS.map((reason) => (
                    <option key={reason} value={reason}>
                      {reason}
                    </option>
                  ))}
                </select>
              </label>
            ) : null}
            <button
              type="button"
              aria-label="Confirm stored Plan"
              disabled={busy || writesBlocked}
              onClick={() => void confirmStoredPlan()}
            >
              Confirm Plan
            </button>
            </>
          ) : (
            <p>Choose a symbol to load stored facts from FinanceClient.</p>
          )}
          <h3>Open position lines</h3>
          <PositionDetailsTable positions={positionDetails} />
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
            Enter the ticker and retrieve from the public market. Name, last price, last 12
            declarations, and a suggested frequency come back automatically. You still confirm
            Plan, risk, and the first lot. Nothing is posted until you save.
          </p>
          <label>
            Symbol
            <input
              aria-label="New investment symbol"
              value={wizSymbol}
              onChange={(e) => setWizSymbol(e.target.value)}
              disabled={busy || writesBlocked}
            />
          </label>
          <button
            type="button"
            aria-label="Retrieve from market"
            disabled={busy || writesBlocked}
            onClick={() => void retrieveFromMarket()}
          >
            Retrieve from market
          </button>
          {wizRetrieveNote ? <p>{wizRetrieveNote}</p> : null}
          <table aria-label="Mandatory data checklist">
            <thead>
              <tr>
                <th scope="col">Mandatory fact</th>
                <th scope="col">Value</th>
                <th scope="col">Status</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td>Symbol / name</td>
                <td>
                  {wizSymbol || "—"} {wizName ? `/ ${wizName}` : ""}
                </td>
                <td>{wizPart1Stored ? "stored" : wizName ? "retrieved" : "missing"}</td>
              </tr>
              <tr>
                <td>Risk</td>
                <td>{wizRisk || "not chosen"}</td>
                <td>{wizPart1Stored ? "stored" : wizRisk ? "owner" : "missing — owner must choose"}</td>
              </tr>
              <tr>
                <td>Frequency</td>
                <td>{wizFreq}</td>
                <td>{wizPart1Stored ? "stored" : wizRetrieveNote ? "retrieved" : "owner"}</td>
              </tr>
              <tr>
                <td>Provider</td>
                <td>{wizProvider || "not retrieved"}</td>
                <td>{wizProvider ? (wizPart1Stored ? "stored" : "present") : "missing"}</td>
              </tr>
              <tr>
                <td>CurrentPrice</td>
                <td>
                  {wizPriceState?.priceMinor != null
                    ? `${formatUsd(wizPriceState.priceMinor, wizPriceState.scale)} (${wizPriceState.freshness})`
                    : wizPrice
                      ? `$${wizPrice} (not stored yet)`
                      : "unavailable"}
                </td>
                <td>
                  {wizPriceState?.priceMinor != null
                    ? "stored"
                    : wizPrice
                      ? "retrieved"
                      : "missing"}
                </td>
              </tr>
              <tr>
                <td>Declarations (12 lookback)</td>
                <td>
                  {formatCount(wizReview?.observationCount ?? wizDecls.length)} / 12
                </td>
                <td>
                  {(wizReview?.observationCount ?? wizDecls.length) > 0
                    ? wizPart1Stored
                      ? "stored"
                      : "retrieved"
                    : "missing"}
                </td>
              </tr>
              <tr>
                <td>Most Current</td>
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
                    : "ready"}
                </td>
              </tr>
              <tr>
                <td>Avg 6</td>
                <td>
                  {wizReview?.avg6Minor == null
                    ? "incomplete"
                    : `$${formatScaled(wizReview.avg6Minor, wizReview.amountScale)}`}
                </td>
                <td>{wizReview?.avg6Complete ? "ready" : "incomplete"}</td>
              </tr>
              <tr>
                <td>Min / max / 80% of avg</td>
                <td>
                  {wizReview?.minMinor == null
                    ? "N/A until retrieve/save"
                    : `$${formatScaled(wizReview.minMinor, wizReview.amountScale)} / $${formatScaled(wizReview.maxMinor ?? 0, wizReview.amountScale)} / ${
                        wizReview.eightyPctOfAvgMinor == null
                          ? "N/A"
                          : `$${formatScaled(wizReview.eightyPctOfAvgMinor, wizReview.amountScale)}`
                      }`}
                </td>
                <td>{wizReview?.fullAnalysisPossible ? "ready" : "blocked or incomplete"}</td>
              </tr>
              <tr>
                <td>Plan confirmed</td>
                <td>{wizPlan || "not entered"}</td>
                <td>{wizPlanStored ? "stored" : "owner"}</td>
              </tr>
              <tr>
                <td>First lot</td>
                <td>
                  {wizLotStored
                    ? "open"
                    : wizQty && wizCost
                      ? `qty ${wizQty}, cost ${wizCost}`
                      : "required"}
                </td>
                <td>{wizLotStored ? "stored" : "owner"}</td>
              </tr>
            </tbody>
          </table>
          <label>
            Name
            <input
              aria-label="New investment name"
              value={wizName}
              onChange={(e) => setWizName(e.target.value)}
              disabled={busy || writesBlocked}
            />
          </label>
          <label>
            Provider (issuer/sponsor — not the instrument name)
            <input
              aria-label="New investment provider"
              value={wizProvider}
              onChange={(e) => setWizProvider(e.target.value)}
              disabled={busy || writesBlocked}
            />
          </label>
          <label>
            Risk
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
          </label>
          <label>
            Frequency
            <select
              aria-label="New investment frequency"
              value={wizFreq}
              onChange={(e) => setWizFreq(e.target.value)}
              disabled={busy || writesBlocked}
            >
              <option>Weekly</option>
              <option>Monthly</option>
              <option>Quarterly</option>
            </select>
          </label>
          <label>
            Current price (if live quote misses)
            <input
              aria-label="Record last price"
              value={wizPrice}
              onChange={(e) => setWizPrice(e.target.value)}
              disabled={busy || writesBlocked}
            />
          </label>
          {wizDecls.length > 0 ? (
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
          <label>
            Declaration source
            <input
              aria-label="Declaration source"
              value={wizDeclSource}
              onChange={(e) => setWizDeclSource(e.target.value)}
              disabled={busy || writesBlocked}
            />
          </label>
          <label>
            Declaration amounts (up to 12, newest first)
            <input
              aria-label="Retrieve declarations"
              value={wizDeclAmounts}
              onChange={(e) => setWizDeclAmounts(e.target.value)}
              disabled={busy || writesBlocked}
            />
          </label>
          <button
            type="button"
            aria-label="Save new investment facts"
            disabled={busy || writesBlocked}
            onClick={() => void newInvestmentPart1()}
          >
            Save Part 1
          </button>
          {wizPriceState ? (
            <p>
              Price {wizPriceState.freshness}
              {wizPriceState.priceMinor != null ? ` ${wizPriceState.priceMinor}` : " unavailable"}.
              Price-derived valid: {wizPriceState.priceDerivedValid ? "yes" : "no"}.
            </p>
          ) : null}
          {wizReview ? (
            <p>
              Plan-review stored: {formatCount(wizReview.observationCount)} observations. Avg 6{" "}
              {wizReview.avg6Complete ? "complete" : "incomplete"}. Full analysis{" "}
              {wizReview.fullAnalysisPossible ? "possible" : "not possible"}.
            </p>
          ) : null}
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
          </div>
          <label>
            Plan per share
            <input
              aria-label="Plan per share"
              value={wizPlan}
              onChange={(e) => setWizPlan(e.target.value)}
              disabled={busy || writesBlocked}
              placeholder={
                wizDecls[0]
                  ? `Most Current ${formatScaled(wizDecls[0].amountPerShareMinor, wizDecls[0].amountScale ?? 4)} — type Plan (not auto-filled)`
                  : "Owner-controlled; not filled from Avg 6"
              }
            />
          </label>
          <label>
            Plan reason
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
          </label>
          {(wizReview?.incompleteReasonRequired ||
            (wizDecls.length > 0 && wizDecls.length < 6)) && (
            <label>
              Incomplete analysis reason
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
            </label>
          )}
          <button
            type="button"
            aria-label="Confirm Plan"
            disabled={busy || writesBlocked || !wizSecurityId}
            onClick={() => void confirmPlan()}
          >
            Confirm Plan
          </button>
          <label>
            First lot account
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
          </label>
          <label>
            Quantity
            <input
              aria-label="First lot quantity"
              value={wizQty}
              onChange={(e) => setWizQty(e.target.value)}
              disabled={busy || writesBlocked}
            />
          </label>
          <label>
            Original cost
            <input
              aria-label="First lot original cost"
              value={wizCost}
              onChange={(e) => setWizCost(e.target.value)}
              disabled={busy || writesBlocked}
            />
          </label>
          <button
            type="button"
            aria-label="Open first lot"
            disabled={busy || writesBlocked || !wizSecurityId}
            onClick={() => void openFirstLot()}
          >
            Open first lot
          </button>
        </section>
      ) : null}

      {screen === "add-lot" ? (
        <section aria-label="Add Lot">
          <h2>Add Lot</h2>
          <p>
            Existing symbol only. Opens another lot; upcoming Income Plan uses Plan × new
            quantity. Does not change Plan or re-fetch declarations.
          </p>
          <label>
            Symbol
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
            Quantity
            <input
              aria-label="Add lot quantity"
              value={addLotQty}
              onChange={(e) => setAddLotQty(e.target.value)}
              disabled={busy || writesBlocked}
            />
          </label>
          <label>
            Original cost
            <input
              aria-label="Add lot original cost"
              value={addLotCost}
              onChange={(e) => setAddLotCost(e.target.value)}
              disabled={busy || writesBlocked}
            />
          </label>
          <button
            type="button"
            aria-label="Add Lot"
            disabled={busy || writesBlocked}
            onClick={() => void openAddLot()}
          >
            Add Lot
          </button>
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
