import type {
  AccountListItem,
  CalculatorGet,
  CartBuyLine,
  CartExecutedList,
  CartExecutedRow,
  CurrentPriceGet,
  DeclarationHistoryGet,
  CartScenario,
  CartScenarioList,
  CartSellLine,
  CashPileGet,
  HoldingsGet,
  InvestmentGet,
  PositionMasterGet,
} from "@finos/app-contracts";
import {
  CALCULATOR_CADENCE_FILTERS,
  CALCULATOR_PERFORMANCE_VIEWS,
  formatBps,
  formatUsd,
  matchesCalculatorPerformanceView,
} from "@finos/ui-components";
import { useEffect, useMemo, useRef, useState } from "react";
import { rankLowestCost } from "../shared/pickers";
import type { ResearchedSymbolOption } from "../shared/pickers";
import {
  CartStartWizard,
  wizardRailStep,
  type CartWizardPrompt,
  type SavedCartPlan,
} from "./CartStartWizard";
import { periodsForPlan, planAnnualCents, planBasisText } from "./cartPlan";
import { ExecutePlanPanel } from "./ExecutePlanPanel";
import { IncomeCompare } from "./IncomeCompare";
import { PlanSheetTable, type PlanSheetRow } from "./PlanSheets";
import { ScenarioDelta } from "./ScenarioDelta";
import {
  accountCashSymbol,
  compareCritique,
  isCashSymbol,
  pctOf,
  roundedWeekMonth,
  scenarioCritique,
} from "./planMath";
import { cashConfirmPreview, lotCostCents } from "./cartCashConfirm";
import {
  CART_UNIT_SCALE,
  executeUnitMinor,
  priceCents,
  rescaleMinor,
  spendCentsFromPrice,
} from "./cartPrice";
import { clearCartResume, readCartResume, writeCartResume } from "./cartResume";
import { cashRemainderRow, withSymbolSubtotals } from "./planSheetRollups";
import { StepRail } from "./StepRail";

export type CartClient = {
  executeCommand: (
    name: string,
    body: Record<string, unknown>,
  ) => Promise<{ ok: boolean; errorCode?: string; bodyJson?: string }>;
  executeQuery: (
    name: string,
    body?: Record<string, unknown>,
  ) => Promise<{ ok: boolean; errorCode?: string; bodyJson?: string }>;
};

export type CartBuyPrefill = {
  scenarioId: string;
  accountId: string;
  planId: string;
  securityId: string;
  symbol: string;
  qtyWhole: number;
  lastMinor: number;
  priceScale?: number;
  openedOn: string;
};

function parseScenario(json?: string): CartScenario | null {
  if (!json) return null;
  try {
    return JSON.parse(json) as CartScenario;
  } catch {
    return null;
  }
}

function masterRow(master: PositionMasterGet | null, symbol: string) {
  return master?.rows.find((row) => row.symbol.toUpperCase() === symbol.toUpperCase());
}

function normalizeTier(raw: string): string {
  const key = raw.trim().toLowerCase().replace(/_/g, " ");
  if (key === "foundation") return "Foundation";
  if (key === "core") return "Core";
  if (key === "risk on" || key === "riskon" || key === "high risk" || key === "highrisk" || key === "high-risk") {
    return "Risk On";
  }
  return "";
}

function tierOf(master: PositionMasterGet | null, symbol: string): string {
  return normalizeTier(masterRow(master, symbol)?.riskTier ?? "");
}

type SymbolFacts = {
  tier: string;
  planPerShareMinor: number;
  planScale: number;
  periods: number;
  remainingQuantityMinor: number;
  quantityScale: number;
  remainingPerformanceMinor: number;
  remainingTaxMinor: number;
};

function factsAnnual(facts: SymbolFacts | undefined, qty: number): number | null {
  if (!facts || !(qty > 0) || facts.periods <= 0) return null;
  return planAnnualCents(facts.planPerShareMinor, facts.planScale, facts.periods, qty);
}

async function loadSymbolFacts(
  client: CartClient,
  securityId: string,
): Promise<SymbolFacts | null> {
  const result = await client.executeQuery("InvestmentGet", { securityId });
  if (!result.ok || !result.bodyJson) return null;
  const body = JSON.parse(result.bodyJson) as InvestmentGet;
  return {
    tier: normalizeTier(body.riskTier ?? ""),
    planPerShareMinor: body.planPerShareMinor,
    planScale: body.planScale,
    periods: periodsForPlan(body.paymentFrequency, body.planningPeriodsPerYear),
    remainingQuantityMinor: body.remainingQuantityMinor,
    quantityScale: body.quantityScale,
    remainingPerformanceMinor: body.remainingPerformanceMinor,
    remainingTaxMinor: body.remainingTaxMinor,
  };
}

function agreeStopMessage(code: string | undefined): string {
  switch (code) {
    case "insufficient_lot_qty":
      return "Execute stopped: a sell quantity is larger than the shares still in that lot.";
    case "mix_override_required":
      return "Execute stopped: this plan would worsen the target mix, and Agree requires a typed reason.";
    case "scenario_already_agreed":
      return "Execute stopped: another scenario on this account is already agreed.";
    case "cash_floor_override_required":
      return "Execute stopped at the cash-floor check. That check should not block a sell-and-rebuy — reload the desktop and try again.";
    default:
      return `Execute stopped at agree (${code ?? "error"}).`;
  }
}

function priceToCents(minor: number, scale: number | null | undefined): number {
  const places = scale ?? 2;
  if (places === 2) return minor;
  if (places > 2) return Math.round(minor / 10 ** (places - 2));
  return Math.round(minor * 10 ** (2 - places));
}

function lastOf(
  calculator: CalculatorGet | null,
  master: PositionMasterGet | null,
  symbol: string,
): number | null {
  const calc = calculator?.rows.find(
    (row) => row.symbol.toUpperCase() === symbol.toUpperCase(),
  );
  if (calc?.lastPriceMinor != null && calc.lastPriceMinor > 0) {
    return priceToCents(calc.lastPriceMinor, calc.lastPriceScale);
  }
  const row = masterRow(master, symbol);
  if (row?.lastPriceMinor != null && row.lastPriceMinor > 0) {
    return priceToCents(row.lastPriceMinor, row.lastPriceScale);
  }
  return null;
}

function cashYieldBps(
  calculator: CalculatorGet | null,
  master: PositionMasterGet | null,
  symbol: string,
): number | null {
  const row = masterRow(master, symbol);
  if (row?.planFwdYieldBps != null) return row.planFwdYieldBps;
  const calc = calculator?.rows.find((item) => item.symbol === symbol);
  if (!calc?.planKnown || !calc.planPerShareMinor) return null;
  const periods = periodsForPlan(
    row?.paymentFrequency ?? "",
    calc.planningPeriodsPerYear ?? 0,
  );
  if (!periods) return null;
  const priceCents = isCashSymbol(symbol) ? 100 : calc.lastPriceMinor;
  if (priceCents == null || priceCents <= 0) return null;
  const denom = 10 ** calc.planScale * priceCents;
  if (denom === 0) return null;
  return Math.round((calc.planPerShareMinor * periods * 1_000_000) / denom);
}

function planAnnualForSymbol(
  calculator: CalculatorGet | null,
  master: PositionMasterGet | null,
  symbol: string,
  qtyWhole: number,
): number | null {
  const calc = calculator?.rows.find(
    (row) => row.symbol.toUpperCase() === symbol.toUpperCase(),
  );
  const row = masterRow(master, symbol);
  // Frequency wins over a stale Calculator planningPeriodsPerYear (Twice monthly = 24).
  const periods = periodsForPlan(
    row?.paymentFrequency ?? calc?.paymentFrequency ?? "",
    calc?.planningPeriodsPerYear ?? 0,
  );
  if (calc?.planKnown && calc.planPerShareMinor && periods) {
    const annual = planAnnualCents(
      calc.planPerShareMinor,
      calc.planScale,
      periods,
      qtyWhole,
    );
    if (annual != null) return annual;
  }
  if (row?.planPerShareMinor && periods) {
    const annual = planAnnualCents(row.planPerShareMinor, row.planScale, periods, qtyWhole);
    if (annual != null) return annual;
  }
  return null;
}

/** Same Plan $ and periods the rate came from, so a wrong rate is readable on the sheet. */
function planBasisForSymbol(
  calculator: CalculatorGet | null,
  master: PositionMasterGet | null,
  symbol: string,
): string {
  const calc = calculator?.rows.find(
    (row) => row.symbol.toUpperCase() === symbol.toUpperCase(),
  );
  const row = masterRow(master, symbol);
  const periods = periodsForPlan(
    row?.paymentFrequency ?? calc?.paymentFrequency ?? "",
    calc?.planningPeriodsPerYear ?? 0,
  );
  if (calc?.planKnown && calc.planPerShareMinor) {
    const text = planBasisText(calc.planPerShareMinor, calc.planScale, periods);
    if (text) return text;
  }
  if (row?.planPerShareMinor) {
    return planBasisText(row.planPerShareMinor, row.planScale, periods);
  }
  return "";
}

function annualForQty(
  calculator: CalculatorGet | null,
  master: PositionMasterGet | null,
  symbol: string,
  qty: number,
): number | null {
  if (!(qty > 0)) return null;
  const one = planAnnualForSymbol(calculator, master, symbol, 1);
  if (one == null) return null;
  return Math.round(one * qty);
}

/** Account - M/D/YY -  so the owner can type the last name. */
export function cartPlanDefaultName(accountName: string, asOfIso: string): string {
  const day = (asOfIso || "").slice(0, 10);
  const parts = day.split("-");
  let short = day;
  if (parts.length === 3 && parts[0].length === 4) {
    const month = Number(parts[1]);
    const date = Number(parts[2]);
    if (month && date) short = `${month}/${date}/${parts[0].slice(2)}`;
  }
  const acct = accountName.trim() || "Account";
  return `${acct} - ${short} - `;
}

export function cartPlanStoredName(raw: string): string {
  const trimmed = raw.trim().replace(/\s+-\s*$/, "").trim();
  return trimmed || "Draft";
}

function qtyOf(line: CartSellLine): number {
  const scale = line.qtyScale ?? 0;
  return line.qtyMinor / 10 ** scale;
}

function sharesOf(qtyMinor: number, qtyScale: number): string {
  return (qtyMinor / 10 ** qtyScale).toFixed(qtyScale);
}

function dollarsFromQty(qtyMinor: number, qtyScale: number, unitMinor: number): number {
  const denom = 10 ** qtyScale;
  if (!Number.isFinite(denom) || denom <= 0) return 0;
  return Math.round((qtyMinor * unitMinor) / denom);
}

function proportionalBasis(basisMinor: number, remainingQty: number, takeQty: number): number {
  if (remainingQty <= 0 || takeQty <= 0) return 0;
  const take = Math.min(takeQty, remainingQty);
  return Math.trunc((basisMinor * take) / remainingQty);
}

function moneyText(minor: number | null): string {
  if (minor == null || !Number.isFinite(minor)) return "";
  return formatUsd(minor, 2);
}

function finiteMinor(value: unknown): number | null {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

/** Same dollars the open sell table shows: quantity times last price. */
function sellLineMinor(line: CartSellLine): number {
  const proceeds = finiteMinor(line.proceedsMinor);
  if (proceeds != null && proceeds > 0) return proceeds;
  const qtyMinor = finiteMinor(line.qtyMinor);
  const unit = finiteMinor(line.unitMinor);
  const qtyScale = finiteMinor(line.qtyScale) ?? 0;
  if (qtyMinor != null && unit != null && unit > 0) {
    const shares = qtyMinor / 10 ** qtyScale;
    if (Number.isFinite(shares) && shares > 0) {
      return spendCentsFromPrice(shares, unit, line.unitScale ?? 2);
    }
  }
  return 0;
}

/** Same dollars the open buy table shows: whole shares times last price. */
function buyLineMinor(line: CartBuyLine): number {
  const spend = finiteMinor(line.spendMinor);
  if (spend != null && spend > 0) return spend;
  const qty = finiteMinor(line.qtyWhole);
  const last = finiteMinor(line.lastMinor);
  if (qty != null && last != null && last > 0) {
    return spendCentsFromPrice(qty, last, line.priceScale ?? 2);
  }
  return 0;
}

function sellSheetRows(
  lines: CartSellLine[],
  sellTotal: number | null,
  calculator: CalculatorGet | null,
  master: PositionMasterGet | null,
  yieldBps: number | null,
): PlanSheetRow[] {
  return lines.map((line) => {
    const qty = qtyOf(line);
    const priceMinor = line.unitMinor > 0 ? priceCents(line.unitMinor, line.unitScale) : null;
    const marketMinor = priceMinor == null ? null : line.proceedsMinor;
    const planned = annualForQty(calculator, master, line.symbol, qty);
    const yearMinor = line.isCash
      ? planned ??
        (yieldBps != null && marketMinor != null && marketMinor > 0
          ? Math.round((marketMinor * yieldBps) / 10_000)
          : null)
      : planned;
    const parts = roundedWeekMonth(yearMinor);
    const eachMinor = yearMinor == null || qty <= 0 ? null : Math.round(yearMinor / qty);
    return {
      key: line.lineId,
      priceMinor,
      symbol: line.symbol,
      alloc: pctOf(marketMinor, sellTotal, 1),
      qty: qty ? String(qty) : "",
      marketMinor,
      weekMinor: parts.weekMinor,
      monthMinor: parts.monthMinor,
      yearMinor,
      eachMinor,
      yieldText: pctOf(yearMinor, marketMinor, 2),
      planBasis: planBasisForSymbol(calculator, master, line.symbol),
      tier: tierOf(master, line.symbol),
      taxGainMinor: line.isCash ? null : line.taxGainMinor ?? null,
      performanceGainMinor: line.isCash ? null : line.performanceGainMinor ?? null,
    };
  });
}

function buySheetRows(
  lines: CartBuyLine[],
  sellTotal: number | null,
  calculator: CalculatorGet | null,
  master: PositionMasterGet | null,
  facts: Record<string, SymbolFacts>,
): PlanSheetRow[] {
  return lines.map((line) => {
    const priceMinor = line.lastMinor > 0 ? priceCents(line.lastMinor, line.priceScale) : null;
    const known = facts[line.symbol.toUpperCase()];
    const yearMinor =
      planAnnualForSymbol(calculator, master, line.symbol, line.qtyWhole) ??
      line.planAnnualMinor ??
      factsAnnual(known, line.qtyWhole);
    const parts = roundedWeekMonth(yearMinor);
    const eachMinor =
      yearMinor == null || line.qtyWhole <= 0 ? null : Math.round(yearMinor / line.qtyWhole);
    const marketMinor =
      priceMinor == null
        ? null
        : line.spendMinor > 0
          ? line.spendMinor
          : spendCentsFromPrice(line.qtyWhole, line.lastMinor, line.priceScale ?? 2);
    return {
      key: line.lineId,
      priceMinor,
      symbol: line.symbol,
      alloc: pctOf(marketMinor, sellTotal, 1),
      qty: line.qtyWhole ? String(line.qtyWhole) : "",
      marketMinor,
      weekMinor: parts.weekMinor,
      monthMinor: parts.monthMinor,
      yearMinor,
      eachMinor,
      yieldText: pctOf(yearMinor, marketMinor, 2),
      planBasis:
        planBasisForSymbol(calculator, master, line.symbol) ||
        planBasisText(known?.planPerShareMinor ?? 0, known?.planScale ?? 0, known?.periods ?? 0),
      tier: tierOf(master, line.symbol) || known?.tier || "",
    };
  });
}

function factsFromMaster(master: PositionMasterGet | null, symbol: string): SymbolFacts | undefined {
  const row = masterRow(master, symbol);
  if (!row) return undefined;
  return {
    tier: normalizeTier(row.riskTier),
    planPerShareMinor: row.planPerShareMinor,
    planScale: row.planScale,
    periods: periodsForPlan(row.paymentFrequency, 0),
    remainingQuantityMinor: row.remainingQuantityMinor,
    quantityScale: row.quantityScale,
    remainingPerformanceMinor: row.remainingPerformanceMinor,
    remainingTaxMinor: row.remainingTaxMinor,
  };
}

function spendOf(rows: PlanSheetRow[]): number | null {
  return dollarsOf(rows.filter((row) => !row.key.startsWith("remaining-cash")));
}

function dollarsOf(rows: PlanSheetRow[]): number | null {
  const known = rows.filter((row) => row.marketMinor != null);
  if (known.length === 0) return null;
  return known.reduce((sum, row) => sum + (row.marketMinor ?? 0), 0);
}

function monthOf(rows: PlanSheetRow[]): number | null {
  if (rows.length === 0) return null;
  let sum = 0;
  let any = false;
  for (const row of rows) {
    if (row.monthMinor == null) continue;
    sum += row.monthMinor;
    any = true;
  }
  return any ? sum : null;
}

function yearOf(rows: PlanSheetRow[]): number | null {
  if (rows.length === 0) return null;
  let sum = 0;
  let any = false;
  for (const row of rows) {
    if (row.yearMinor == null) continue;
    sum += row.yearMinor;
    any = true;
  }
  return any ? sum : null;
}

export function ShoppingCartScreen({
  client,
  accounts,
  holdings,
  calculator,
  researched,
  positionMaster,
  declHistory,
  asOfDate,
  busy,
  writesBlocked,
  onMessage,
  onBusy,
  onDirtyChange,
  onOpenBuyLot,
  cartRefreshKey = 0,
  resumeScenarioId = null,
  onCartResumed,
}: {
  client: CartClient;
  accounts: AccountListItem[];
  holdings: HoldingsGet | null;
  calculator: CalculatorGet | null;
  researched: ResearchedSymbolOption[];
  positionMaster: PositionMasterGet | null;
  declHistory: DeclarationHistoryGet | null;
  asOfDate: string;
  busy?: boolean;
  writesBlocked?: boolean;
  cartRefreshKey?: number;
  /** Survives Add Lot remount — host keeps this while screen === add-lot. */
  resumeScenarioId?: string | null;
  onCartResumed?: () => void;
  onMessage: (message: string) => void;
  onBusy: (busy: boolean) => void;
  onDirtyChange: (dirty: boolean) => void;
  onOpenBuyLot: (prefill: CartBuyPrefill) => void;
}) {
  const [accountId, setAccountId] = useState("");
  const [draftName, setDraftName] = useState("Draft");
  const suggestedName = useRef("Draft");
  const buyLock = useRef(false);
  const [symbolFacts, setSymbolFacts] = useState<Record<string, SymbolFacts>>({});
  const [cashPile, setCashPile] = useState<CashPileGet | null>(null);
  const loadedFacts = useRef(new Set<string>());
  const [wizardPrompt, setWizardPrompt] = useState<CartWizardPrompt>("account");
  const [savedPlans, setSavedPlans] = useState<SavedCartPlan[]>([]);
  const [executedCarts, setExecutedCarts] = useState<CartExecutedRow[]>([]);
  const [planId, setPlanId] = useState("");
  const [scenarioA, setScenarioA] = useState<CartScenario | null>(null);
  const [scenarioB, setScenarioB] = useState<CartScenario | null>(null);
  const [resumingCart, setResumingCart] = useState(
    () => Boolean(resumeScenarioId) || readCartResume() != null,
  );
  const [sellSymbol, setSellSymbol] = useState("");
  const [lotSellQty, setLotSellQty] = useState<Record<string, string>>({});
  const [quotedSellMinor, setQuotedSellMinor] = useState<number | null>(null);
  const [sellPlan, setSellPlan] = useState<{
    symbol: string;
    tier: string;
    annualPerShare: number | null;
  } | null>(null);
  const [sellNote, setSellNote] = useState("");
  const [cashQty, setCashQty] = useState("");
  const [cashTier, setCashTier] = useState("");
  const [buyA, setBuyA] = useState({ securityId: "", qty: "" });
  const [buyB, setBuyB] = useState({ securityId: "", qty: "" });
  const [depositAmount, setDepositAmount] = useState("");
  const [sold, setSold] = useState(false);
  const [executeStatusMessage, setExecuteStatusMessage] = useState<string | null>(null);
  const [dirty, setDirty] = useState(false);

  const accountName = accounts.find((account) => account.accountId === accountId)?.name ?? "";
  const lots = useMemo(() => holdings?.lots ?? [], [holdings]);
  const accountLots = useMemo(
    () => (accountName ? lots.filter((lot) => lot.accountName === accountName) : lots),
    [accountName, lots],
  );
  const cashSymbol = accountCashSymbol(
    accountName,
    accountLots.map((lot) => lot.symbol),
  );
  const heldSymbols = [
    ...new Set(
      accountLots
        .filter((lot) => !isCashSymbol(lot.symbol) && lot.remainingQuantityMinor > 0)
        .map((lot) => lot.symbol),
    ),
  ].sort();
  const buyChoices = researched.filter((row) => !isCashSymbol(row.symbol));

  const markDirty = (next: boolean) => {
    setDirty(next);
    onDirtyChange(next);
  };
  const edited = () => markDirty(true);

  const bindPlan = (items: CartScenario[], id: string) => {
    const mine = items.filter((item) => (item.planId ?? "") === id);
    if (mine.length === 0) return;
    const slotA = mine.find((item) => (item.slot ?? "A") === "A");
    if (slotA) setScenarioA(slotA);
    setScenarioB(mine.find((item) => item.slot === "B") ?? null);
  };

  const refreshPlan = async (id: string, account: string) => {
    const result = await client.executeQuery("CartScenarioList", { accountId: account });
    if (!result.ok || !result.bodyJson) return;
    const body = JSON.parse(result.bodyJson) as CartScenarioList;
    bindPlan(body.items, id);
  };

  useEffect(() => {
    if (planId && accountId) void refreshPlan(planId, accountId);
  }, [cartRefreshKey, planId, accountId]);

  /** After Add Lot, this screen remounts empty — reload the in-progress execute scenario. */
  useEffect(() => {
    let cancelled = false;
    setResumingCart(true);
    void (async () => {
      const stored = readCartResume();
      let scenarioId = resumeScenarioId || stored?.scenarioId || "";
      if (!scenarioId) {
        // Always recover an executing plan (sells posted / buys in flight).
        // After Add Lot (cartRefreshKey > 0) also recover agreed-not-yet-executing.
        for (const account of accounts) {
          const listed = await client.executeQuery("CartScenarioList", {
            accountId: account.accountId,
          });
          if (cancelled) return;
          if (!listed.ok || !listed.bodyJson) continue;
          const body = JSON.parse(listed.bodyJson) as CartScenarioList;
          const items = body.items ?? [];
          const executing = items.find((item) => item.status === "executing");
          const agreed =
            cartRefreshKey > 0
              ? items.find((item) => item.status === "agreed")
              : null;
          const pick = executing ?? agreed ?? null;
          if (pick) {
            scenarioId = pick.scenarioId;
            break;
          }
        }
      }
      if (!scenarioId) {
        setResumingCart(false);
        return;
      }
      const got = await client.executeQuery("CartScenarioGet", { scenarioId });
      if (cancelled) return;
      if (!got.ok || !got.bodyJson) {
        clearCartResume();
        onCartResumed?.();
        setResumingCart(false);
        onMessage("In-progress cart could not be restored — pick it from Saved plans.");
        return;
      }
      const scene = JSON.parse(got.bodyJson) as CartScenario;
      if (scene.status === "discarded") {
        clearCartResume();
        onCartResumed?.();
        setResumingCart(false);
        return;
      }
      const nextPlanId = scene.planId || scene.scenarioId;
      const nextAccountId = scene.accountId;
      setAccountId(nextAccountId);
      setPlanId(nextPlanId);
      if (scene.status === "complete") {
        // Cash already aligned — land on the close summary read-only, not an empty sell
        // sheet. The key is dropped so the next visit starts a new cart.
        clearCartResume();
      } else {
        writeCartResume({
          scenarioId: scene.scenarioId,
          accountId: nextAccountId,
          planId: nextPlanId,
        });
      }
      const listed = await client.executeQuery("CartScenarioList", {
        accountId: nextAccountId,
      });
      if (cancelled) return;
      if (listed.ok && listed.bodyJson) {
        const body = JSON.parse(listed.bodyJson) as CartScenarioList;
        bindPlan(body.items, nextPlanId);
      } else {
        setScenarioA(scene);
        setScenarioB(null);
      }
      onCartResumed?.();
      setResumingCart(false);
    })();
    return () => {
      cancelled = true;
    };
    // Intentionally omit onMessage / onCartResumed — parent passes unstable lambdas.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [resumeScenarioId, cartRefreshKey, client, accounts]);

  /**
   * Completed carts are history, not drafts, and they belong on the Shopping Cart home screen
   * rather than inside the build flow. No accountId is sent on purpose: home lists every
   * account's completed carts, which is what the table's Account column is for.
   */
  const loadExecutedCarts = async () => {
    const result = await client.executeQuery("CartExecutedList", {});
    if (!result.ok || !result.bodyJson) {
      setExecutedCarts([]);
      return;
    }
    const body = JSON.parse(result.bodyJson) as CartExecutedList;
    setExecutedCarts(body.items ?? []);
  };

  const loadSavedPlans = async (account: string) => {
    const result = await client.executeQuery("CartScenarioList", { accountId: account });
    if (!result.ok || !result.bodyJson) {
      setSavedPlans([]);
      return;
    }
    const body = JSON.parse(result.bodyJson) as CartScenarioList;
    // A plan with an executed slot is not an open cart. Filtering only the complete scenario
    // left its sibling behind, so one cart appeared in Open carts and Executed carts at once.
    const executedPlans = new Set(
      body.items
        .filter((scene) => scene.status === "complete")
        .map((scene) => scene.planId || scene.scenarioId),
    );
    const grouped = new Map<string, CartScenario[]>();
    for (const item of body.items) {
      const id = item.planId || item.scenarioId;
      if (executedPlans.has(id)) continue;
      if (item.status === "complete" || item.status === "superseded") continue;
      const current = grouped.get(id) ?? [];
      current.push(item);
      grouped.set(id, current);
    }
    setSavedPlans(
      [...grouped.entries()]
        .map(([id, scenes]) => {
          // No `?? scenes[0]` fallback. When slot A was filtered out that let the surviving
          // slot B stand in as slot A and be counted as both, which is how a single empty
          // draft read "Scenarios 2" with "A $0.00 · B $0.00".
          const slotA = scenes.find((item) => (item.slot ?? "A") === "A") ?? null;
          const slotB = scenes.find((item) => item.slot === "B") ?? null;
          const slots = [slotB, slotA].filter((item): item is CartScenario => item != null);
          const head = slotA ?? slotB;
          if (!head) return null;
          return {
            planId: id,
            name: head.name?.trim() || "Draft",
            asOf: head.asOf,
            status:
              slotA && slotB && slotB.status !== slotA.status
                ? `A ${slotA.status} · B ${slotB.status}`
                : head.status,
            scenarioCount: slots.length,
            // Sell lines are keyed by plan, so either slot reports the same sale.
            sellMinor: (head.sellLines ?? []).reduce((sum, line) => sum + sellLineMinor(line), 0),
            buyAMinor: slotA
              ? (slotA.buyLines ?? []).reduce((sum, line) => sum + buyLineMinor(line), 0)
              : 0,
            buyBMinor: slotB
              ? (slotB.buyLines ?? []).reduce((sum, line) => sum + buyLineMinor(line), 0)
              : null,
            scenarioIds: [...new Set(slots.map((item) => item.scenarioId))],
          };
        })
        .filter((plan): plan is SavedCartPlan => plan !== null)
        .reverse(),
    );
  };

  useEffect(() => {
    // Home: completed carts for every account, under the account picker.
    if (wizardPrompt === "account") {
      void loadExecutedCarts();
      return;
    }
    // Picking an account leaves home for that account's open carts.
    if (wizardPrompt !== "plans" || !accountId) return;
    void loadSavedPlans(accountId);
  }, [wizardPrompt, accountId, cartRefreshKey]);

  useEffect(() => {
    if (!planId) {
      setSold(false);
      return;
    }
    try {
      setSold(sessionStorage.getItem(`cart-sells-posted-${planId}`) === "1");
    } catch {
      setSold(false);
    }
  }, [planId]);

  const deleteSavedPlan = async (plan: SavedCartPlan) => {
    if (!accountId) return;
    const ok = window.confirm(
      `Delete ${plan.name}? The sell table and its scenarios are removed.`,
    );
    if (!ok) return;
    onBusy(true);
    try {
      for (const scenarioId of plan.scenarioIds) {
        const result = await client.executeCommand("CartScenarioDiscard", { scenarioId });
        if (!result.ok) {
          onMessage(`Delete failed: ${result.errorCode ?? "error"}`);
          await loadSavedPlans(accountId);
          return;
        }
      }
      await loadSavedPlans(accountId);
      onMessage("Plan deleted.");
    } finally {
      onBusy(false);
    }
  };

  const openSavedPlan = async (id: string) => {
    if (!accountId) return;
    const result = await client.executeQuery("CartScenarioList", { accountId });
    if (!result.ok || !result.bodyJson) {
      onMessage("Saved plans could not be loaded.");
      return;
    }
    const body = JSON.parse(result.bodyJson) as CartScenarioList;
    if (!body.items.some((item) => (item.planId || item.scenarioId) === id)) {
      onMessage("That plan is no longer saved.");
      await loadSavedPlans(accountId);
      return;
    }
    setPlanId(id);
    bindPlan(body.items, id);
    markDirty(false);
    onMessage("Plan opened.");
  };

  /**
   * Home lists every account and the account picker is empty, so the draft opener
   * (`if (!accountId) return`) never ran. Open the scenario on the row itself.
   */
  const openExecutedCart = async (row: CartExecutedRow) => {
    const account = row.accountId;
    if (!account || !row.scenarioId) {
      onMessage("That cart could not be opened.");
      return;
    }
    const got = await client.executeQuery("CartScenarioGet", { scenarioId: row.scenarioId });
    if (!got.ok || !got.bodyJson) {
      onMessage("That cart could not be opened.");
      return;
    }
    const scene = JSON.parse(got.bodyJson) as CartScenario;
    setAccountId(scene.accountId || account);
    setPlanId(scene.planId || scene.scenarioId);
    setScenarioA(scene);
    setScenarioB(null);
    clearCartResume();
    markDirty(false);
    onMessage("Cart opened.");
  };

  const run = async (name: string, body: Record<string, unknown>) => {
    onBusy(true);
    try {
      const result = await client.executeCommand(name, body);
      if (!result.ok) {
        onMessage(`${name} failed: ${result.errorCode ?? "error"}`);
        return null;
      }
      return parseScenario(result.bodyJson);
    } finally {
      onBusy(false);
    }
  };

  const suggestPlanName = (id: string) => {
    const acct = accounts.find((account) => account.accountId === id)?.name ?? "";
    const next = cartPlanDefaultName(acct, asOfDate);
    setDraftName((current) => {
      if (current.trim() !== "" && current !== "Draft" && current !== suggestedName.current) {
        return current;
      }
      suggestedName.current = next;
      return next;
    });
  };

  const create = async () => {
    if (!accountId) {
      onMessage("Choose an account.");
      return;
    }
    const next = await run("CartScenarioCreate", {
      accountId,
      asOf: asOfDate,
      name: cartPlanStoredName(draftName),
      fundingSource: "sellLots",
    });
    if (!next?.planId) return;
    setPlanId(next.planId);
    setScenarioA(next);
    setScenarioB(null);
    markDirty(true);
    onMessage("Plan opened.");
  };

  const cashLine = scenarioA?.sellLines.find((line) => line.isCash || isCashSymbol(line.symbol));
  useEffect(() => {
    if (!cashLine) {
      setCashQty("");
      return;
    }
    setCashQty(String(qtyOf(cashLine)));
  }, [cashLine?.lineId, cashLine?.qtyMinor, cashLine?.qtyScale]);

  useEffect(() => {
    if (!sellSymbol) {
      setQuotedSellMinor(null);
      setSellPlan(null);
      setLotSellQty({});
      return;
    }
    const seeded: Record<string, string> = {};
    for (const lot of accountLots) {
      if (lot.symbol.toUpperCase() !== sellSymbol.toUpperCase() || lot.remainingQuantityMinor <= 0) {
        continue;
      }
      seeded[lot.lotId] = String(lot.remainingQuantityMinor / 10 ** lot.quantityScale);
    }
    setLotSellQty(seeded);
    const symbol = sellSymbol;
    setQuotedSellMinor(null);
    setSellPlan(null);
    let cancel = false;
    void client.executeQuery("InvestmentGet", { symbol }).then((result) => {
      if (cancel || !result.ok || !result.bodyJson) return;
      const body = JSON.parse(result.bodyJson) as InvestmentGet;
      if (body.symbol.toUpperCase() !== symbol.toUpperCase()) return;
      const px = body.price?.priceMinor;
      if (px != null && px > 0) setQuotedSellMinor(priceToCents(px, body.price?.scale));
      // Frequency wins over a stale Calculator planningPeriodsPerYear (Twice monthly = 24).
      const periods = periodsForPlan(body.paymentFrequency, body.planningPeriodsPerYear);
      setSellPlan({
        symbol: symbol.toUpperCase(),
        tier: normalizeTier(body.riskTier ?? ""),
        annualPerShare: planAnnualCents(body.planPerShareMinor, body.planScale, periods, 1),
      });
    });
    return () => {
      cancel = true;
    };
  }, [sellSymbol, client]);

  useEffect(() => {
    const lines = [...(scenarioA?.buyLines ?? []), ...(scenarioB?.buyLines ?? [])];
    const ids = [
      ...new Set(
        lines
          .map((line) => line.securityId)
          .filter((id): id is string => Boolean(id)),
      ),
    ];
    if (ids.length === 0) return;
    let cancel = false;
    void (async () => {
      const next: Record<string, SymbolFacts> = {};
      for (const securityId of ids) {
        const line = lines.find((row) => row.securityId === securityId);
        if (!line) continue;
        const facts = await loadSymbolFacts(client, securityId);
        if (cancel) return;
        if (facts) {
          next[line.symbol.toUpperCase()] = facts;
          loadedFacts.current.add(securityId);
        } else {
          loadedFacts.current.delete(securityId);
        }
      }
      if (cancel || Object.keys(next).length === 0) return;
      setSymbolFacts((current) => ({ ...current, ...next }));
    })();
    return () => {
      cancel = true;
    };
  }, [client, scenarioA, scenarioB]);

  useEffect(() => {
    const known = tierOf(positionMaster, cashSymbol);
    if (known) {
      setCashTier(known);
      return;
    }
    if (!cashSymbol) {
      setCashTier("");
      return;
    }
    let cancel = false;
    void client.executeQuery("InvestmentGet", { symbol: cashSymbol }).then((result) => {
      if (cancel || !result.ok || !result.bodyJson) return;
      const body = JSON.parse(result.bodyJson) as InvestmentGet;
      if (body.symbol.toUpperCase() !== cashSymbol.toUpperCase()) return;
      setCashTier(normalizeTier(body.riskTier ?? ""));
    });
    return () => {
      cancel = true;
    };
  }, [cashSymbol, client, positionMaster]);

  useEffect(() => {
    if (!accountId || !scenarioA) return;
    let cancelled = false;
    void client.executeQuery("CashPileGet", { accountId }).then((result) => {
      if (cancelled || !result.ok || !result.bodyJson) return;
      const body = JSON.parse(result.bodyJson) as CashPileGet;
      setCashPile(body.found ? body : null);
    });
    return () => {
      cancelled = true;
    };
  }, [accountId, client, scenarioA, cartRefreshKey]);

  const commitCashQty = async () => {
    if (!scenarioA || !planId || !accountId) return;
    const qty = Number(cashQty);
    if (!Number.isFinite(qty) || qty < 0) {
      onMessage("Cash qty must be a number.");
      return;
    }
    const cashSymbols = new Set<string>([
      cashSymbol,
      ...sellLines.filter((line) => line.isCash || isCashSymbol(line.symbol)).map((line) => line.symbol),
    ]);
    for (const symbol of cashSymbols) {
      const cleared = await run("CartSellSymbolClear", {
        scenarioId: scenarioA.scenarioId,
        symbol,
      });
      if (!cleared) return;
    }
    if (qty === 0) {
      await refreshPlan(planId, accountId);
      edited();
      return;
    }
    const cashLots = accountLots.filter(
      (lot) => lot.symbol.toUpperCase() === cashSymbol.toUpperCase() && lot.remainingQuantityMinor > 0,
    );
    const lot = [...cashLots].sort((a, b) => b.remainingQuantityMinor - a.remainingQuantityMinor)[0];
    if (!lot) {
      onMessage(`No ${cashSymbol} lot is open on this account.`);
      await refreshPlan(planId, accountId);
      return;
    }
    const qtyMinor = Math.round(qty * 10 ** lot.quantityScale);
    if (qtyMinor > lot.remainingQuantityMinor) {
      onMessage(`${cashSymbol} qty is above the open lot.`);
      await refreshPlan(planId, accountId);
      return;
    }
    const next = await run("CartSellLineAdd", {
      scenarioId: scenarioA.scenarioId,
      lotId: lot.lotId,
      qtyMinor,
      unitMinor: 100,
      isCash: true,
    });
    if (next) {
      await refreshPlan(planId, accountId);
      edited();
    }
  };

  const addSellSymbol = async () => {
    if (!scenarioA || !planId || !accountId || !sellSymbol) {
      const message = "Choose a symbol to sell.";
      setSellNote(message);
      onMessage(message);
      return;
    }
    const matching = accountLots.filter(
      (lot) => lot.symbol.toUpperCase() === sellSymbol.toUpperCase() && lot.remainingQuantityMinor > 0,
    );
    if (matching.length === 0) {
      const message = `${sellSymbol} has no open lot.`;
      setSellNote(message);
      onMessage(message);
      return;
    }
    const takes: Array<{ lotId: string; qtyMinor: number }> = [];
    for (const lot of matching) {
      const qty = Number(lotSellQty[lot.lotId] ?? "");
      if (!Number.isFinite(qty) || qty <= 0) continue;
      const qtyMinor = Math.round(qty * 10 ** lot.quantityScale);
      if (qtyMinor > lot.remainingQuantityMinor) {
        const message = `${sellSymbol} qty is above the ${lot.openedOn} lot.`;
        setSellNote(message);
        onMessage(message);
        return;
      }
      takes.push({ lotId: lot.lotId, qtyMinor });
    }
    if (takes.length === 0) {
      const message = "Enter the shares to sell on the lot row.";
      setSellNote(message);
      onMessage(message);
      return;
    }
    let unitMinorRaw: number | null =
      calculator?.rows.find((row) => row.symbol.toUpperCase() === sellSymbol.toUpperCase())
        ?.lastPriceMinor ?? null;
    let unitFromScale =
      calculator?.rows.find((row) => row.symbol.toUpperCase() === sellSymbol.toUpperCase())
        ?.lastPriceScale ?? 2;
    if (unitMinorRaw == null || unitMinorRaw <= 0) {
      const master = masterRow(positionMaster, sellSymbol);
      if (master?.lastPriceMinor != null && master.lastPriceMinor > 0) {
        unitMinorRaw = master.lastPriceMinor;
        unitFromScale = master.lastPriceScale ?? 2;
      }
    }
    if (unitMinorRaw == null || unitMinorRaw <= 0) {
      unitMinorRaw = quotedSellMinor;
      unitFromScale = 2;
    }
    if (unitMinorRaw == null || unitMinorRaw <= 0) {
      const securityId = masterRow(positionMaster, sellSymbol)?.securityId;
      if (securityId) {
        const priced = await client.executeQuery("CurrentPriceGet", { securityId });
        if (priced.ok && priced.bodyJson) {
          const price = JSON.parse(priced.bodyJson) as CurrentPriceGet;
          if (price.priceMinor != null && price.priceMinor > 0) {
            unitMinorRaw = price.priceMinor;
            unitFromScale = price.scale ?? 2;
          }
        }
      }
    }
    const unitMinor =
      unitMinorRaw != null && unitMinorRaw > 0
        ? rescaleMinor(unitMinorRaw, unitFromScale, CART_UNIT_SCALE)
        : null;
    if (unitMinor == null || unitMinor <= 0) {
      const message = "Last price unknown — will not invent $0.";
      setSellNote(message);
      onMessage(message);
      return;
    }
    const cleared = await run("CartSellSymbolClear", {
      scenarioId: scenarioA.scenarioId,
      symbol: sellSymbol,
    });
    if (!cleared) {
      setSellNote("The sell row was not added.");
      return;
    }
    for (const take of takes) {
      const next = await run("CartSellLineAdd", {
        scenarioId: scenarioA.scenarioId,
        lotId: take.lotId,
        qtyMinor: take.qtyMinor,
        unitMinor,
        unitScale: CART_UNIT_SCALE,
        isCash: false,
      });
      if (!next) {
        setSellNote("The sell row was not added.");
        return;
      }
    }
    setSellSymbol("");
    setSellNote("");
    await refreshPlan(planId, accountId);
    edited();
    onMessage(`${sellSymbol} split across lowest-cost lots.`);
  };

  const removeSellLine = async (lineId: string) => {
    if (!scenarioA || !planId || !accountId) return;
    const line = scenarioA.sellLines.find((item) => item.lineId === lineId);
    if (!line) return;
    const next = await run("CartSellLineRemove", {
      scenarioId: scenarioA.scenarioId,
      lineId,
    });
    if (next) {
      if (line.isCash || isCashSymbol(line.symbol)) setCashQty("");
      await refreshPlan(planId, accountId);
      edited();
    }
  };

  const addBuy = async (slot: "A" | "B") => {
    if (buyLock.current) return;
    const scene = slot === "A" ? scenarioA : scenarioB;
    const form = slot === "A" ? buyA : buyB;
    if (!scene || !planId || !accountId || !form.securityId || form.qty.trim() === "") return;
    const qtyWhole = Number(form.qty.trim());
    if (!Number.isInteger(qtyWhole) || qtyWhole <= 0) {
      onMessage("Buy qty must be whole shares.");
      return;
    }
    const row = buyChoices.find((item) => item.securityId === form.securityId);
    if (!row) return;
    if (scene.buyLines.some((line) => line.symbol.toUpperCase() === row.symbol.toUpperCase())) {
      onMessage(`${row.symbol} is already on this scenario. Change its qty.`);
      return;
    }
    buyLock.current = true;
    try {
      // Always re-read Plan — a line added before Confirm Plan must not keep a blank year.
      const known =
        (await loadSymbolFacts(client, row.securityId)) ??
        symbolFacts[row.symbol.toUpperCase()];
      if (known) {
        setSymbolFacts((current) => ({
          ...current,
          [row.symbol.toUpperCase()]: known,
        }));
        loadedFacts.current.add(row.securityId);
      }
      let quoteMinor = lastOf(calculator, positionMaster, row.symbol);
      let quoteScale = 2;
      if (quoteMinor == null) {
        const priced = await client.executeQuery("CurrentPriceGet", { securityId: row.securityId });
        if (priced.ok && priced.bodyJson) {
          const price = JSON.parse(priced.bodyJson) as CurrentPriceGet;
          if (price.priceMinor != null && price.priceMinor > 0) {
            quoteMinor = price.priceMinor;
            quoteScale = price.scale ?? 2;
          }
        }
      }
      if (quoteMinor == null || quoteMinor <= 0) {
        onMessage("Last price unknown — will not invent $0.");
        return;
      }
      const lastMinor = rescaleMinor(quoteMinor, quoteScale, CART_UNIT_SCALE);
      const planAnnual =
        planAnnualForSymbol(calculator, positionMaster, row.symbol, qtyWhole) ??
        factsAnnual(known, qtyWhole);
      if (planAnnual == null) {
        onMessage(
          `${row.symbol}: week/month/year need a stored Plan / share. Confirm Plan on Add Investment, then add this buy again.`,
        );
      }
      const next = await run("CartBuyLineAdd", {
        scenarioId: scene.scenarioId,
        securityId: row.securityId,
        qtyWhole,
        lastMinor,
        priceScale: CART_UNIT_SCALE,
        planAnnualMinor: planAnnual,
      });
      if (!next) return;
      if (slot === "A") {
        setScenarioA(next);
        setBuyA({ securityId: "", qty: "" });
      } else {
        setScenarioB(next);
        setBuyB({ securityId: "", qty: "" });
      }
      await refreshPlan(planId, accountId);
      edited();
    } finally {
      buyLock.current = false;
    }
  };

  const setBuyQty = async (scene: CartScenario, lineId: string, qtyWhole: number) => {
    if (!planId || !accountId) return;
    const line = scene.buyLines.find((item) => item.lineId === lineId);
    if (!line || !Number.isInteger(qtyWhole) || qtyWhole <= 0) return;
    const cents = lastOf(calculator, positionMaster, line.symbol);
    const lastMinor =
      cents != null
        ? rescaleMinor(cents, 2, line.priceScale ?? 2)
        : line.lastMinor;
    const known =
      (await loadSymbolFacts(client, line.securityId)) ??
      symbolFacts[line.symbol.toUpperCase()];
    if (known) {
      setSymbolFacts((current) => ({
        ...current,
        [line.symbol.toUpperCase()]: known,
      }));
    }
    const next = await run("CartBuyLineQtySet", {
      scenarioId: scene.scenarioId,
      lineId,
      qtyWhole,
      lastMinor,
      planAnnualMinor:
        planAnnualForSymbol(calculator, positionMaster, line.symbol, qtyWhole) ??
        factsAnnual(known, qtyWhole),
    });
    if (next) {
      await refreshPlan(planId, accountId);
      edited();
    }
  };

  const removeBuy = async (scene: CartScenario, lineId: string) => {
    if (!planId || !accountId) return;
    const next = await run("CartBuyLineRemove", {
      scenarioId: scene.scenarioId,
      lineId,
    });
    if (next) {
      await refreshPlan(planId, accountId);
      edited();
    }
  };

  const addScenarioB = async () => {
    if (!scenarioA || !planId || !accountId) return;
    const next = await run("CartScenarioSlotAdd", { scenarioId: scenarioA.scenarioId });
    if (next) {
      await refreshPlan(planId, accountId);
      edited();
    }
  };

  const commitDeposit = async () => {
    if (!scenarioA || !planId || !accountId) return;
    const dollars = depositAmount.trim() === "" ? 0 : Number(depositAmount);
    if (!Number.isFinite(dollars) || dollars < 0) {
      onMessage("Deposit must be zero or a positive dollar amount.");
      return;
    }
    const next = await run("CartPlanDepositSet", {
      scenarioId: scenarioA.scenarioId,
      depositMinor: Math.round(dollars * 100),
    });
    if (next) {
      await refreshPlan(planId, accountId);
      edited();
    }
  };

  const evaluate = async (scene: CartScenario) => {
    onBusy(true);
    try {
      const result = await client.executeQuery("CartScenarioEvaluate", {
        scenarioId: scene.scenarioId,
      });
      if (!result.ok) {
        onMessage(`Evaluate failed: ${result.errorCode ?? "error"}`);
        return;
      }
      if (planId && accountId) await refreshPlan(planId, accountId);
      onMessage(`Scenario ${scene.slot ?? "A"} evaluated.`);
    } finally {
      onBusy(false);
    }
  };

  const agree = async (scene: CartScenario) => {
    const next = await run("CartScenarioAgree", {
      scenarioId: scene.scenarioId,
      mixWorsens: false,
    });
    if (next && planId && accountId) {
      await refreshPlan(planId, accountId);
      markDirty(false);
      onMessage(`Scenario ${scene.slot ?? "A"} agreed.`);
    }
  };

  const confirmSell = async (scene: CartScenario) => {
    const next = await run("CartExecuteSell", {
      scenarioId: scene.scenarioId,
      occurredOn: asOfDate,
    });
    if (next && planId && accountId) {
      setSold(true);
      try {
        sessionStorage.setItem(`cart-sells-posted-${planId}`, "1");
      } catch {
        /* ignore */
      }
      await refreshPlan(planId, accountId);
      const msg =
        "Position sells posted (if any). SPAXX in the plan is not sold here — leftover cash aligns after the last purchase.";
      setExecuteStatusMessage(msg);
      onMessage(msg);
    }
  };

  const alignCash = async (scene: CartScenario, endingCashMinor: number | null) => {
    const next = await run("CartExecuteCashAlign", {
      scenarioId: scene.scenarioId,
      occurredOn: asOfDate,
      ...(endingCashMinor != null ? { targetMinor: endingCashMinor } : {}),
    });
    if (next && planId && accountId) {
      await refreshPlan(planId, accountId);
      if (next.status === "complete") {
        clearCartResume();
      }
      onMessage("Cash aligned to actual sale proceeds and purchase dollars.");
      return true;
    }
    return false;
  };

  const executePlanSells = async () => {
    if (!scenarioA || !planId || !accountId) {
      const msg = "Plan is not ready — reopen this plan from Saved plans.";
      setExecuteStatusMessage(msg);
      onMessage(msg);
      return;
    }
    onBusy(true);
    setExecuteStatusMessage("Running Evaluate and Agree, then posting sells…");
    try {
      const evaluated = await client.executeQuery("CartScenarioEvaluate", {
        scenarioId: scenarioA.scenarioId,
      });
      if (!evaluated.ok) {
        const msg = `Evaluate failed: ${evaluated.errorCode ?? "error"}`;
        setExecuteStatusMessage(msg);
        onMessage(msg);
        return;
      }
      await refreshPlan(planId, accountId);
      if (scenarioA.status === "draft") {
        const agreed = await client.executeCommand("CartScenarioAgree", {
          scenarioId: scenarioA.scenarioId,
          mixWorsens: false,
        });
        if (!agreed.ok) {
          const msg = agreeStopMessage(agreed.errorCode);
          setExecuteStatusMessage(msg);
          onMessage(msg);
          return;
        }
        await refreshPlan(planId, accountId);
        const scene = parseScenario(agreed.bodyJson);
        if (!scene) {
          const msg = "Agree succeeded but scenario could not be read.";
          setExecuteStatusMessage(msg);
          onMessage(msg);
          return;
        }
        markDirty(false);
        await confirmSell(scene);
        return;
      }
      if (scenarioA.status === "agreed" || scenarioA.status === "executing") {
        markDirty(false);
        await confirmSell(scenarioA);
      }
    } finally {
      onBusy(false);
    }
  };

  const openBuyLine = (lineId: string) => {
    const scene = [scenarioA, scenarioB].find(
      (item) => item && (item.status === "agreed" || item.status === "executing"),
    );
    const buy = scene?.buyLines.find((line) => line.lineId === lineId);
    if (!scene || !buy) {
      onMessage("Post sells first, then confirm each purchase.");
      return;
    }
    onOpenBuyLot({
      scenarioId: scene.scenarioId,
      accountId: scene.accountId,
      planId: planId || scene.planId || scene.scenarioId,
      securityId: buy.securityId,
      symbol: buy.symbol,
      qtyWhole: buy.qtyWhole,
      lastMinor: executeUnitMinor(buy.lastMinor, buy.priceScale, buy.qtyWhole, buy.spendMinor),
      priceScale: CART_UNIT_SCALE,
      // Confirm on a later calendar day must not backdate the lot to a stale cart asOf.
      openedOn: (() => {
        const today = new Date().toISOString().slice(0, 10);
        const asOf = (asOfDate || "").slice(0, 10);
        return asOf && asOf < today ? today : asOf || today;
      })(),
    });
    writeCartResume({
      scenarioId: scene.scenarioId,
      accountId: scene.accountId,
      planId: planId || scene.planId || scene.scenarioId,
    });
  };

  const discard = async () => {
    const scenes = [scenarioB, scenarioA].filter((item): item is CartScenario => item != null);
    onBusy(true);
    try {
      for (const scene of scenes) {
        if (scene.status !== "draft") continue;
        const result = await client.executeCommand("CartScenarioDiscard", {
          scenarioId: scene.scenarioId,
        });
        if (!result.ok) {
          onMessage(`Discard failed: ${result.errorCode ?? "error"}`);
          return;
        }
      }
      setScenarioA(null);
      setScenarioB(null);
      setPlanId("");
      setSold(false);
      clearCartResume();
      setWizardPrompt("plans");
      markDirty(false);
      if (accountId) await loadSavedPlans(accountId);
      onMessage("Plan discarded.");
    } finally {
      onBusy(false);
    }
  };

  /**
   * Back to the Shopping Cart home screen from an open cart, a new cart, or a completed cart
   * opened read-only. There was no way out before: once a scenario loaded, the wizard stopped
   * rendering. `clearCartResume` is the part that cannot be skipped — the resume effect would
   * otherwise pull the same cart straight back open and leaving would look broken.
   */
  const returnToCartHome = () => {
    setScenarioA(null);
    setScenarioB(null);
    setPlanId("");
    setSold(false);
    clearCartResume();
    markDirty(false);
    setWizardPrompt("account");
    onMessage("Shopping Cart home.");
  };

  const sellLines = scenarioA?.sellLines ?? [];
  const positionLines = sellLines.filter((line) => !line.isCash && !isCashSymbol(line.symbol));
  const yieldBps =
    scenarioA && scenarioA.cashYieldBps > 0
      ? scenarioA.cashYieldBps
      : cashYieldBps(calculator, positionMaster, cashSymbol);
  const depositMinor = scenarioA?.depositMinor ?? 0;
  const positionRows = sellSheetRows(positionLines, null, calculator, positionMaster, yieldBps);
  const positionDollars = dollarsOf(positionRows);
  const cashQtyNumber = Number(cashQty);
  const plannedCashMinor =
    cashLine != null
      ? Math.round(qtyOf(cashLine) * 100)
      : Number.isFinite(cashQtyNumber) && cashQtyNumber > 0
        ? Math.round(cashQtyNumber * 100)
        : null;
  const cashYearMinor = (() => {
    if (plannedCashMinor == null) return null;
    const qty = plannedCashMinor / 100;
    const planned = annualForQty(calculator, positionMaster, cashSymbol, qty);
    if (planned != null) return planned;
    if (yieldBps != null && yieldBps > 0) return Math.round((plannedCashMinor * yieldBps) / 10_000);
    return null;
  })();
  const cashParts = roundedWeekMonth(cashYearMinor);
  const sellTotal =
    plannedCashMinor == null && positionDollars == null
      ? null
      : (plannedCashMinor ?? 0) + (positionDollars ?? 0);
  const cashRow: PlanSheetRow = {
    key: cashLine?.lineId ?? "account-cash",
    priceMinor: 100,
    symbol: cashSymbol,
    alloc: pctOf(plannedCashMinor, sellTotal, 1),
    qty: cashQty,
    qtyNode: (
      <input
        aria-label={`${cashSymbol} qty to use in this plan`}
        value={cashQty}
        disabled={busy || writesBlocked}
        onChange={(event) => setCashQty(event.target.value)}
        onBlur={() => void commitCashQty()}
      />
    ),
    marketMinor: plannedCashMinor,
    weekMinor: cashParts.weekMinor,
    monthMinor: cashParts.monthMinor,
    yearMinor: cashYearMinor,
    eachMinor:
      cashYearMinor == null || !(plannedCashMinor != null && plannedCashMinor > 0)
        ? null
        : Math.round(cashYearMinor / (plannedCashMinor / 100)),
    yieldText: pctOf(cashYearMinor, plannedCashMinor, 2),
    tier: tierOf(positionMaster, cashSymbol) || cashTier,
    taxGainMinor: null,
    performanceGainMinor: null,
  };
  const sellRowsAllocated = [
    cashRow,
    ...withSymbolSubtotals(
      sellSheetRows(positionLines, sellTotal, calculator, positionMaster, yieldBps),
    ),
  ];
  const budgetBalance = (buySpend: number | null): number | null => {
    if (sellTotal == null || buySpend == null) return null;
    return sellTotal - buySpend;
  };
  const buyBudgetMinor = sellTotal;
  const purchasesA = scenarioA
    ? buySheetRows(scenarioA.buyLines, buyBudgetMinor, calculator, positionMaster, symbolFacts)
    : [];
  const purchasesB = scenarioB
    ? buySheetRows(scenarioB.buyLines, buyBudgetMinor, calculator, positionMaster, symbolFacts)
    : [];
  const withQty = (
    slot: "A" | "B",
    rows: PlanSheetRow[],
    lines: CartScenario["buyLines"],
    onQty: (lineId: string, qtyWhole: number) => void,
  ): PlanSheetRow[] =>
    rows.map((row) => {
      const line = lines.find((item) => item.lineId === row.key);
      if (!line) return row;
      return {
        ...row,
        qtyNode: (
          <input
            aria-label={`Scenario ${slot} qty ${line.symbol}`}
            key={`${line.lineId}-${line.qtyWhole}`}
            defaultValue={String(line.qtyWhole)}
            disabled={busy || writesBlocked}
            onBlur={(event) => {
              const qty = Number(event.target.value);
              if (Number.isInteger(qty) && qty > 0 && qty !== line.qtyWhole) onQty(line.lineId, qty);
            }}
          />
        ),
      };
    });
  const buyRowsA = scenarioA
    ? withQty(
        "A",
        purchasesA,
        scenarioA.buyLines,
        (lineId, qty) => void setBuyQty(scenarioA, lineId, qty),
      )
    : [];
  const buyRowsB = scenarioB
    ? withQty(
        "B",
        purchasesB,
        scenarioB.buyLines,
        (lineId, qty) => void setBuyQty(scenarioB, lineId, qty),
      )
    : [];
  /** "complete" stays here so the close summary is visible after cash aligns. */
  const agreedScene = [scenarioA, scenarioB].find(
    (item) =>
      item &&
      (item.status === "agreed" ||
        item.status === "executing" ||
        item.status === "complete"),
  );
  const executeSteps = scenarioA?.executeSteps ?? [];
  const sellsPosted =
    sold ||
    scenarioA?.executeCashBaselineMinor != null ||
    scenarioA?.status === "executing" ||
    scenarioA?.status === "complete" ||
    executeSteps.some((step) => step.kind === "sell" || step.kind === "cash_align");
  const buySpendA = spendOf(buyRowsA);
  const sellFooterRows = (() => {
    const remainder = cashRemainderRow({
      cashSymbol,
      startingCashMinor: plannedCashMinor,
      positionProceedsMinor: positionDollars,
      buySpendMinor: buySpendA,
      budgetMinor: sellTotal,
      tier: tierOf(positionMaster, cashSymbol) || cashTier,
      yearMinorOnCash: (qtyWhole) => {
        if (!(qtyWhole > 0)) return null;
        const planned = annualForQty(calculator, positionMaster, cashSymbol, qtyWhole);
        if (planned != null) return planned;
        if (yieldBps != null && yieldBps > 0) {
          return Math.round(qtyWhole * 100 * yieldBps) / 10_000;
        }
        return null;
      },
    });
    return remainder ? [remainder] : [];
  })();
  const executeBlockReason = (() => {
    if (!scenarioA || scenarioA.buyLines.length === 0) {
      return "Add at least one buy row on Scenario A.";
    }
    if (sellLines.length === 0) {
      return "Add at least one sell line.";
    }
    if (sellTotal == null || buySpendA == null) {
      return "Enter sell and buy amounts so totals are known.";
    }
    if (!scenarioA.eval) {
      return "Run Evaluate A first — Execute plan needs a fresh rollup.";
    }
    const ev = scenarioA.eval;
    if (ev?.insufficientLotQty) {
      return "Spend exceeds live lot remaining (Evaluate shows insufficient_lot_qty).";
    }
    return null;
  })();
  const executePlanWarning = (() => {
    if (sellTotal == null || buySpendA == null) return null;
    const parts: string[] = [];
    if (buySpendA > sellTotal + depositMinor) {
      parts.push(
        `Plan buy ${formatUsd(buySpendA, 2)} exceeds plan budget ${formatUsd(sellTotal + depositMinor, 2)} (${cashSymbol} + sale proceeds).`,
      );
    }
    const ev = scenarioA?.eval;
    if (ev != null && ev.leftoverMinor < 0) {
      parts.push("Evaluate shows plan cash short at plan prices.");
    }
    if (parts.length === 0) return null;
    return `${parts.join(" ")} You can still confirm sells — SPAXX is reconciled on Confirm cash after actual prices.`;
  })();
  const canExecutePlan = executeBlockReason == null && !sellsPosted;
  const executeSellRows = sellLines
    .filter((line) => !line.isCash && !isCashSymbol(line.symbol))
    .map((line) => ({
      lineId: line.lineId,
      symbol: line.symbol,
      qtyLabel: sharesOf(line.qtyMinor, line.qtyScale ?? 0),
      priceMinor: rescaleMinor(line.unitMinor, line.unitScale ?? 2, CART_UNIT_SCALE),
      unitScale: CART_UNIT_SCALE,
      proceedsMinor: line.proceedsMinor,
      taxGainMinor: line.taxGainMinor ?? null,
      performanceGainMinor: line.performanceGainMinor ?? null,
    }));
  /** Confirmed by the lot’s symbol on the buy step — never by buy-line index (HAKY≠MUIB). */
  const confirmedBuySymbols = new Set(
    executeSteps
      .filter((step) => step.kind === "buy" && step.lotId)
      .map((step) => {
        const lot = holdings?.lots.find((item) => item.lotId === step.lotId);
        return lot?.symbol.trim().toUpperCase() ?? "";
      })
      .filter((symbol) => symbol.length > 0),
  );
  const executeBuyRows = (scenarioA?.buyLines ?? []).map((line) => ({
    lineId: line.lineId,
    symbol: line.symbol,
    qtyWhole: line.qtyWhole,
    lastMinor: executeUnitMinor(line.lastMinor, line.priceScale, line.qtyWhole, line.spendMinor),
    priceScale: CART_UNIT_SCALE,
    spendMinor: line.spendMinor,
    confirmed:
      sellsPosted && confirmedBuySymbols.has(line.symbol.trim().toUpperCase()),
  }));
  const buysConfirmed =
    (scenarioA?.buyLines.length ?? 0) > 0 &&
    (scenarioA?.buyLines ?? []).every((line) =>
      confirmedBuySymbols.has(line.symbol.trim().toUpperCase()),
    );
  const cashAligned = executeSteps.some((step) => step.kind === "cash_align");
  const canAlignCash = sellsPosted && buysConfirmed && !cashAligned;
  const cashPreview = (() => {
    const buyLotIds = executeSteps
      .filter((step) => step.kind === "buy" && step.lotId)
      .map((step) => step.lotId as string);
    let lotSpend = 0;
    let found = 0;
    for (const id of buyLotIds) {
      const lot = holdings?.lots.find((item) => item.lotId === id);
      if (!lot) continue;
      lotSpend += lotCostCents(lot);
      found += 1;
    }
    const buySpendMinor =
      buyLotIds.length > 0 && found === buyLotIds.length
        ? lotSpend
        : executeBuyRows.filter((row) => row.confirmed).reduce((sum, row) => sum + row.spendMinor, 0);
    return cashConfirmPreview({
      baselineMinor: scenarioA?.executeCashBaselineMinor,
      sellProceedsMinor: executeSellRows.reduce((sum, row) => sum + row.proceedsMinor, 0),
      buySpendMinor,
      currentMinor: cashPile?.dollarsMinor,
    });
  })();
  const purchaseCostMinor = cashPreview?.buySpendMinor ?? null;
  /**
   * The monthly income change the owner agreed to, read straight off the scenario's stored
   * evaluation. This used to be re-derived from the live calculator, and on a reopened cart
   * `annualForQty` returned null for every buy; the `null ? sum` skip collapsed the buy term to
   * zero and left only the cash interest on the dollars spent, so the Income cart's close
   * summary read -$31.36 where its own snapshot said +$7.13. Blank stays blank, not $0.
   */
  const netDividendMinor = scenarioA?.eval?.netMonthlyMinor ?? null;
  const spaxxQtyLabel =
    plannedCashMinor != null && plannedCashMinor > 0
      ? (plannedCashMinor / 100).toLocaleString("en-US", { maximumFractionDigits: 2 })
      : null;

  const commitSellExecutePrice = async (lineId: string, unitMinor: number) => {
    if (!scenarioA || !planId || !accountId || sellsPosted) return;
    const next = await run("CartSellLineUnitSet", {
      scenarioId: scenarioA.scenarioId,
      lineId,
      unitMinor,
    });
    if (next) {
      setScenarioA(next);
      await refreshPlan(planId, accountId);
      edited();
      setExecuteStatusMessage("Sell price updated for that line.");
    }
  };

  const commitBuyExecutePrice = async (lineId: string, lastMinor: number) => {
    if (!scenarioA || !planId || !accountId) return;
    if (executeBuyRows.some((row) => row.lineId === lineId && row.confirmed)) return;
    const next = await run("CartBuyLineLastSet", {
      scenarioId: scenarioA.scenarioId,
      lineId,
      lastMinor,
      priceScale: CART_UNIT_SCALE,
    });
    if (next) {
      setScenarioA(next);
      await refreshPlan(planId, accountId);
      edited();
      setExecuteStatusMessage("Buy plan price updated.");
    }
  };

  const autoAlignKey = useRef<string | null>(null);
  useEffect(() => {
    if (!canAlignCash || !scenarioA || busy || writesBlocked) return;
    if (scenarioA.fundingSource === "accountCash") return;
    const key = scenarioA.scenarioId;
    if (autoAlignKey.current === key) return;
    if (cashPreview && cashPreview.targetMinor < 0) {
      autoAlignKey.current = key;
      setExecuteStatusMessage("Leftover cash is negative — purchases exceed sale proceeds.");
      return;
    }
    autoAlignKey.current = key;
    // align leftover cash after the last purchase — computed SPAXX target, no owner click
    void alignCash(scenarioA, null).then((ok) => {
      if (!ok) {
        setExecuteStatusMessage("Leftover cash did not align.");
      }
    });
  }, [canAlignCash, scenarioA, busy, writesBlocked, cashPreview]);

  const step = !scenarioA
    ? wizardRailStep(wizardPrompt)
    : !agreedScene
      ? "Evaluate"
      : !sellsPosted
        ? "Confirm sell"
        : cashAligned
          ? "Done"
          : !buysConfirmed
            ? "Confirm purchase"
            : "Confirm cash";

  const critiqueFor = (slot: string, rows: PlanSheetRow[], offerAnother: boolean) =>
    scenarioCritique({
      slot,
      sellMonth: monthOf(sellRowsAllocated),
      buyMonth: monthOf(rows),
      sellTiers: sellRowsAllocated.map((row) => row.tier),
      buyTiers: rows.map((row) => row.tier),
      sellDollars: sellTotal,
      buyDollars: spendOf(rows),
      depositMinor,
      offerAnother,
    });

  const availableForBuy = (sellTotal ?? 0) + depositMinor;
  const short =
    (spendOf(buyRowsA) ?? 0) > availableForBuy ||
    (scenarioB != null && (spendOf(buyRowsB) ?? 0) > availableForBuy);

  const returnText = (rows: PlanSheetRow[]) => {
    const year = yearOf(rows);
    return sellTotal != null && year != null ? pctOf(year, sellTotal, 2) : "";
  };

  const planFor = sellPlan?.symbol === sellSymbol.toUpperCase() ? sellPlan : null;
  const sellLastMinor = sellSymbol
    ? (quotedSellMinor ?? lastOf(calculator, positionMaster, sellSymbol))
    : null;
  const yearFor = (symbol: string, qty: number): number | null => {
    if (!(qty > 0)) return null;
    const perShare =
      planFor && planFor.symbol === symbol.toUpperCase()
        ? planFor.annualPerShare
        : planAnnualForSymbol(calculator, positionMaster, symbol, 1);
    if (perShare == null) return null;
    return Math.round(perShare * qty);
  };
  const tierFor = (symbol: string): string =>
    (planFor && planFor.symbol === symbol.toUpperCase() ? planFor.tier : "") ||
    tierOf(positionMaster, symbol);
  const sellLots = (() => {
    if (!sellSymbol) return [];
    const matching = accountLots.filter(
      (lot) =>
        lot.symbol.toUpperCase() === sellSymbol.toUpperCase() && lot.remainingQuantityMinor > 0,
    );
    const order = rankLowestCost(matching);
    return order
      .map((lotId) => matching.find((lot) => lot.lotId === lotId))
      .filter((lot): lot is (typeof matching)[number] => lot != null);
  })();

  return (
    <section aria-label="Shopping Cart" className="shopping-cart">
      <h2>Shopping Cart</h2>
      {!agreedScene ? (
        <p>
          One sell table funds Scenario A and, if you add it, Scenario B. Enter{" "}
          <strong>{cashSymbol} qty to use</strong> (tab out to save) plus position sells; scenario buys
          compare to that total. Drafts do not change Holdings or Income Plan.
        </p>
      ) : null}
      <StepRail current={step} />
      {resumingCart ? (
        <p role="status">Returning to in-progress cart…</p>
      ) : !scenarioA ? (
        <CartStartWizard
          prompt={wizardPrompt}
          accounts={accounts}
          accountId={accountId}
          accountName={accountName || "this account"}
          planName={draftName}
          savedPlans={savedPlans}
          executedCarts={executedCarts}
          busy={busy}
          writesBlocked={writesBlocked}
          onAccountId={(id) => {
            setAccountId(id);
            suggestPlanName(id);
          }}
          onPlanName={setDraftName}
          onOpenPlan={(id) => void openSavedPlan(id)}
          onDeletePlan={(plan) => void deleteSavedPlan(plan)}
          onOpenExecuted={(row) => void openExecutedCart(row)}
          onNewPlan={() => {
            suggestPlanName(accountId);
            setWizardPrompt("planName");
          }}
          onBack={() => {
            if (wizardPrompt === "planName") setWizardPrompt("plans");
            else if (wizardPrompt === "plans") setWizardPrompt("account");
          }}
          onNext={() => {
            if (wizardPrompt === "account") {
              if (!accountId) {
                onMessage("Choose an account.");
                return;
              }
              suggestPlanName(accountId);
              void loadSavedPlans(accountId);
              setWizardPrompt("plans");
              return;
            }
            void create();
          }}
        />
      ) : (
        <>
          <p aria-label="Open cart plan">
            {accountName} · {scenarioA.name?.trim() || "Draft"}
          </p>
          <button
            type="button"
            aria-label="Shopping Cart home"
            disabled={busy}
            onClick={returnToCartHome}
          >
            Back to carts
          </button>
          {agreedScene ? (
            <>
            <ExecutePlanPanel
              scenarioLabel={`Scenario ${scenarioA.slot ?? "A"}`}
              scenarioStatus={scenarioA.status}
              cashSymbol={cashSymbol}
              spaxxQtyLabel={spaxxQtyLabel}
              sellRows={executeSellRows}
              buyRows={executeBuyRows}
              startingCashMinor={plannedCashMinor}
              sellBudgetMinor={sellTotal}
              buySpendMinor={buySpendA}
              budgetBalanceMinor={budgetBalance(buySpendA)}
              canExecute={canExecutePlan}
              blockReason={executeBlockReason}
              sellsPosted={sellsPosted}
              busy={busy}
              writesBlocked={writesBlocked}
              statusMessage={executeStatusMessage}
              onExecuteSells={() => void executePlanSells()}
              onOpenBuyLot={openBuyLine}
              onSellPriceCommit={(lineId, unitMinor) => void commitSellExecutePrice(lineId, unitMinor)}
              onBuyPriceCommit={(lineId, lastMinor) => void commitBuyExecutePrice(lineId, lastMinor)}
              buysConfirmed={buysConfirmed}
              cashAligned={cashAligned}
              canAlignCash={canAlignCash}
              alignBlockReason={null}
              executeWarning={executePlanWarning}
              cashPreview={cashPreview}
              accountName={accountName || "Account"}
              purchaseCostMinor={purchaseCostMinor}
              netDividendMinor={netDividendMinor}
            />
            {/*
              What the scenario change was, off the evaluation stored when the owner agreed.
              An agreed or completed cart used to show the close summary alone, so the week /
              month / year detail that the decision was made on disappeared the moment the cart
              closed. IncomeCompare reads `eval` directly, so nothing here is recomputed.
            */}
            <IncomeCompare eval={scenarioA.eval ?? null} />
            </>
          ) : (
            <>
          {sellNote ? <p aria-label="Sell note">{sellNote}</p> : null}
          <PlanSheetTable
            title="Sell"
            ariaLabel="Sell plan"
            budgetMinor={sellTotal}
            rows={sellRowsAllocated}
            showPnl
            footerRows={sellFooterRows}
            returnText={returnText(sellRowsAllocated)}
            returnLabel="Return"
            onRemoveRow={(lineId) => void removeSellLine(lineId)}
            canRemoveRow={(row) => row.key !== "account-cash" || cashLine != null}
            removeDisabled={busy || writesBlocked}
            editor={
              <>
                {sellLots.map((lot) => {
                  const held = lot.remainingQuantityMinor / 10 ** lot.quantityScale;
                  const typed = Number(lotSellQty[lot.lotId] ?? "");
                  const sellShares = Number.isFinite(typed) && typed > 0 ? typed : 0;
                  const takeMinor =
                    sellShares > 0 ? Math.round(sellShares * 10 ** lot.quantityScale) : 0;
                  const market =
                    sellLastMinor == null || takeMinor <= 0
                      ? null
                      : dollarsFromQty(takeMinor, lot.quantityScale, sellLastMinor);
                  const taxBasis =
                    takeMinor <= 0
                      ? null
                      : priceToCents(
                          proportionalBasis(lot.remainingTaxMinor, lot.remainingQuantityMinor, takeMinor),
                          lot.scale,
                        );
                  const perfBasis =
                    takeMinor <= 0
                      ? null
                      : priceToCents(
                          proportionalBasis(
                            lot.remainingPerformanceMinor,
                            lot.remainingQuantityMinor,
                            takeMinor,
                          ),
                          lot.scale,
                        );
                  const year = yearFor(lot.symbol, sellShares);
                  const parts = roundedWeekMonth(year);
                  return (
                    <tr key={lot.lotId}>
                      <td>{moneyText(sellLastMinor)}</td>
                      <td>
                        {lot.symbol} {lot.openedOn}
                      </td>
                      <td>{pctOf(market, sellTotal, 1)}</td>
                      <td>
                        <input
                          aria-label={`Sell qty ${lot.symbol} ${lot.openedOn}`}
                          value={lotSellQty[lot.lotId] ?? ""}
                          placeholder={String(held)}
                          disabled={busy || writesBlocked}
                          onChange={(event) => {
                            setLotSellQty((current) => ({
                              ...current,
                              [lot.lotId]: event.target.value,
                            }));
                            setSellNote("");
                          }}
                        />
                      </td>
                      <td>{moneyText(market)}</td>
                      <td>{market == null ? "" : `(${formatUsd(Math.abs(market), 2)})`}</td>
                      <td>{moneyText(parts.weekMinor)}</td>
                      <td>{moneyText(parts.monthMinor)}</td>
                      <td>{moneyText(year)}</td>
                      <td>{year == null || sellShares <= 0 ? "" : moneyText(Math.round(year / sellShares))}</td>
                      <td>{pctOf(year, market, 2)}</td>
                      <td>{tierFor(lot.symbol)}</td>
                      <td>{market == null || taxBasis == null ? "" : moneyText(market - taxBasis)}</td>
                      <td>{market == null || perfBasis == null ? "" : moneyText(market - perfBasis)}</td>
                      <td />
                    </tr>
                  );
                })}
                <tr>
                  <td>{moneyText(sellLastMinor)}</td>
                  <td>
                    <select
                      aria-label="Sell symbol"
                      value={sellSymbol}
                      disabled={busy || writesBlocked}
                      onChange={(event) => {
                        setSellSymbol(event.target.value);
                        setSellNote("");
                      }}
                    >
                      <option value="">Add position</option>
                      {heldSymbols.map((symbol) => {
                        const price = lastOf(calculator, positionMaster, symbol);
                        return (
                          <option key={symbol} value={symbol}>
                            {price == null ? symbol : `${symbol} ${formatUsd(price, 2)}`}
                          </option>
                        );
                      })}
                    </select>
                  </td>
                  <td />
                  <td>
                    <button
                      type="button"
                      aria-label="Add sell row"
                      disabled={busy || writesBlocked || !sellSymbol}
                      onClick={() => void addSellSymbol()}
                    >
                      Add row
                    </button>
                  </td>
                  <td />
                  <td />
                  <td />
                  <td />
                  <td />
                  <td />
                  <td />
                  <td />
                  <td />
                  <td />
                  <td />
                </tr>
              </>
            }
          />
          <BuyBlock
            slot="A"
            rows={buyRowsA}
            budgetMinor={buyBudgetMinor}
            returnText={returnText(buyRowsA)}
            choices={buyChoices}
            client={client}
            calculator={calculator}
            positionMaster={positionMaster}
            declHistory={declHistory}
            symbolFacts={symbolFacts}
            onFacts={(symbol, facts) =>
              setSymbolFacts((current) => ({ ...current, [symbol.toUpperCase()]: facts }))
            }
            form={buyA}
            onForm={setBuyA}
            busy={busy}
            writesBlocked={writesBlocked}
            onAdd={() => void addBuy("A")}
            onRemoveRow={(lineId) => void removeBuy(scenarioA, lineId)}
          />
          {buyRowsA.some((row) => row.yearMinor == null && !isCashSymbol(row.symbol)) ? (
            <p role="status" aria-label="Buy plan missing">
              Week / month / year stay blank when Plan / share is not stored yet. Open Add
              Investment → Confirm Plan (Incomplete analysis reason required under 6 pays), then
              re-add or refresh this buy row.
            </p>
          ) : null}
          {scenarioB ? (
            <BuyBlock
              slot="B"
              rows={buyRowsB}
              budgetMinor={buyBudgetMinor}
              returnText={returnText(buyRowsB)}
            choices={buyChoices}
            client={client}
            calculator={calculator}
            positionMaster={positionMaster}
            declHistory={declHistory}
            symbolFacts={symbolFacts}
            onFacts={(symbol, facts) =>
              setSymbolFacts((current) => ({ ...current, [symbol.toUpperCase()]: facts }))
            }
              form={buyB}
              onForm={setBuyB}
              busy={busy}
              writesBlocked={writesBlocked}
              onAdd={() => void addBuy("B")}
              onRemoveRow={(lineId) => scenarioB && void removeBuy(scenarioB, lineId)}
            />
          ) : null}
          <ScenarioDelta
            sellRows={sellRowsAllocated}
            rowsA={buyRowsA}
            rowsB={scenarioB ? buyRowsB : null}
            cashBefore={sellTotal}
            cashAfterA={budgetBalance(spendOf(buyRowsA))}
            cashAfterB={scenarioB ? budgetBalance(spendOf(buyRowsB)) : null}
          />
          <section aria-label="Plan critique">
            <p>{critiqueFor("A", buyRowsA, scenarioB == null)}</p>
            {scenarioB ? (
              <>
                <p>{critiqueFor("B", buyRowsB, false)}</p>
                <p>
                  {compareCritique({
                    sellDollars: buyBudgetMinor,
                    aMonth: monthOf(buyRowsA),
                    bMonth: monthOf(buyRowsB),
                    aYear: yearOf(buyRowsA),
                    bYear: yearOf(buyRowsB),
                    aUnspent: budgetBalance(spendOf(buyRowsA)),
                    bUnspent: budgetBalance(spendOf(buyRowsB)),
                    aTypes: [...new Set(buyRowsA.map((row) => row.tier).filter(Boolean))].join(" and "),
                    bTypes: [...new Set(buyRowsB.map((row) => row.tier).filter(Boolean))].join(" and "),
                    aEnough:
                      spendOf(buyRowsA) == null
                        ? null
                        : (spendOf(buyRowsA) ?? 0) <= availableForBuy,
                    bEnough:
                      spendOf(buyRowsB) == null
                        ? null
                        : (spendOf(buyRowsB) ?? 0) <= availableForBuy,
                  })}
                </p>
              </>
            ) : (
              <button
                type="button"
                aria-label="Add scenario B"
                disabled={busy || writesBlocked}
                onClick={() => void addScenarioB()}
              >
                Add scenario B
              </button>
            )}
            {short ? (
              <label>
                Deposit toward this plan
                <input
                  aria-label="Plan deposit"
                  value={depositAmount}
                  disabled={busy || writesBlocked}
                  onChange={(event) => setDepositAmount(event.target.value)}
                  onBlur={() => void commitDeposit()}
                />
              </label>
            ) : null}
          </section>
          <div className="buttons">
            <button
              type="button"
              aria-label="Evaluate scenario A"
              disabled={busy || scenarioA.buyLines.length === 0}
              onClick={() => void evaluate(scenarioA)}
            >
              Evaluate A
            </button>
            <button
              type="button"
              aria-label="Agree scenario A"
              disabled={busy || writesBlocked || scenarioA.status !== "draft"}
              onClick={() => void agree(scenarioA)}
            >
              Agree A
            </button>
            {scenarioB ? (
              <>
                <button
                  type="button"
                  aria-label="Evaluate scenario B"
                  disabled={busy || scenarioB.buyLines.length === 0}
                  onClick={() => void evaluate(scenarioB)}
                >
                  Evaluate B
                </button>
                <button
                  type="button"
                  aria-label="Agree scenario B"
                  disabled={busy || writesBlocked || scenarioB.status !== "draft"}
                  onClick={() => void agree(scenarioB)}
                >
                  Agree B
                </button>
              </>
            ) : null}
            <button
              type="button"
              aria-label="Save cart draft"
              className={dirty ? "is-unsaved" : undefined}
              disabled={busy || writesBlocked || !dirty}
              onClick={() => {
                if (scenarioA) void evaluate(scenarioA);
                markDirty(false);
              }}
            >
              Save
            </button>
            <button
              type="button"
              aria-label="Discard cart draft"
              disabled={busy || writesBlocked}
              onClick={() => void discard()}
            >
              Cancel
            </button>
          </div>
            </>
          )}
        </>
      )}
    </section>
  );
}

function BuyBlock({
  slot,
  rows,
  budgetMinor,
  returnText,
  choices,
  client,
  calculator,
  positionMaster,
  declHistory,
  symbolFacts,
  onFacts,
  form,
  onForm,
  busy,
  writesBlocked,
  onAdd,
  footerRows,
  onRemoveRow,
}: {
  slot: "A" | "B";
  rows: PlanSheetRow[];
  budgetMinor: number | null;
  returnText: string;
  choices: ResearchedSymbolOption[];
  client: CartClient;
  calculator: CalculatorGet | null;
  positionMaster: PositionMasterGet | null;
  declHistory: DeclarationHistoryGet | null;
  symbolFacts: Record<string, SymbolFacts>;
  onFacts: (symbol: string, facts: SymbolFacts) => void;
  form: { securityId: string; qty: string };
  onForm: (next: { securityId: string; qty: string }) => void;
  busy?: boolean;
  writesBlocked?: boolean;
  onAdd: () => void;
  footerRows?: PlanSheetRow[];
  onRemoveRow?: (lineId: string) => void;
}) {
  const [filterView, setFilterView] = useState<string | null>(null);
  const [filterCadence, setFilterCadence] = useState("all");
  const selected = choices.find((choice) => choice.securityId === form.securityId);
  const knownMinor = selected ? lastOf(calculator, positionMaster, selected.symbol) : null;
  const [quotedMinor, setQuotedMinor] = useState<number | null>(null);
  const selectedId = selected?.securityId ?? "";
  useEffect(() => {
    if (!selectedId || knownMinor != null) {
      setQuotedMinor(null);
      return;
    }
    let cancelled = false;
    void client.executeQuery("CurrentPriceGet", { securityId: selectedId }).then((priced) => {
      if (cancelled || !priced.ok || !priced.bodyJson) return;
      const price = JSON.parse(priced.bodyJson) as CurrentPriceGet;
      setQuotedMinor(price.priceMinor != null && price.priceMinor > 0 ? price.priceMinor : null);
    });
    return () => {
      cancelled = true;
    };
  }, [client, selectedId, knownMinor]);
  const lastMinor = knownMinor ?? quotedMinor;
  const askedFacts = useRef(new Set<string>());
  useEffect(() => {
    if (!selectedId || !selected) return;
    if (symbolFacts[selected.symbol.toUpperCase()] || askedFacts.current.has(selectedId)) return;
    askedFacts.current.add(selectedId);
    const symbol = selected.symbol;
    void loadSymbolFacts(client, selectedId).then((facts) => {
      if (facts) onFacts(symbol, facts);
      else askedFacts.current.delete(selectedId);
    });
  }, [client, onFacts, selected, selectedId, symbolFacts]);
  const spent = rows.length === 0 ? 0 : dollarsOf(rows);
  const room = budgetMinor == null || spent == null ? null : budgetMinor - spent;
  const possibleQty =
    lastMinor != null && lastMinor > 0 && room != null && room > 0
      ? Math.floor(room / lastMinor)
      : 0;
  const qtyWhole = Number(form.qty.trim());
  const draftQty = Number.isInteger(qtyWhole) && qtyWhole > 0 ? qtyWhole : null;
  const draftMarket = selected && lastMinor != null && draftQty != null ? draftQty * lastMinor : null;
  const selectedFacts = selected
    ? symbolFacts[selected.symbol.toUpperCase()] ?? factsFromMaster(positionMaster, selected.symbol)
    : undefined;
  const draftYear =
    selected && draftQty != null
      ? planAnnualForSymbol(calculator, positionMaster, selected.symbol, draftQty) ??
        factsAnnual(selectedFacts, draftQty)
      : null;
  const draftTier = selected
    ? tierOf(positionMaster, selected.symbol) || selectedFacts?.tier || ""
    : "";
  const draftParts = roundedWeekMonth(draftYear);
  const draftEach =
    draftYear == null || draftQty == null ? null : Math.round(draftYear / draftQty);
  const filterRows = useMemo(() => {
    if (!filterView || !declHistory) return [];
    const masters = positionMaster?.rows ?? [];
    const matched = [];
    for (const history of declHistory.rows) {
      const master = masters.find(
        (row) => row.symbol.toUpperCase() === history.symbol.toUpperCase(),
      );
      if (!matchesCalculatorPerformanceView(history, master, filterView, filterCadence)) {
        continue;
      }
      const choice = choices.find(
        (item) => item.symbol.toUpperCase() === history.symbol.toUpperCase(),
      );
      if (!choice) continue;
      matched.push({ choice, master });
    }
    return matched;
  }, [choices, declHistory, filterCadence, filterView, positionMaster]);
  const commit = () => {
    if (!form.securityId || form.qty.trim() === "") return;
    onAdd();
  };
  return (
    <>
      <PlanSheetTable
        title={`Scenario ${slot}`}
        ariaLabel={`Scenario ${slot} buy plan`}
        budgetMinor={budgetMinor}
        rows={rows}
        returnText={returnText}
        returnLabel="New Return"
        footerRows={footerRows}
        onRemoveRow={onRemoveRow}
        removeDisabled={busy || writesBlocked}
        editor={
          <tr>
            <td>{lastMinor == null ? "" : formatUsd(lastMinor, 2)}</td>
            <td>
              <select
                aria-label={`Scenario ${slot} symbol`}
                value={form.securityId}
                disabled={busy || writesBlocked}
                onChange={(event) => {
                  const value = event.target.value;
                  const view = CALCULATOR_PERFORMANCE_VIEWS.find((item) => item.id === value);
                  if (view) {
                    setFilterView(view.id);
                    setFilterCadence("all");
                    return;
                  }
                  onForm({ ...form, securityId: value });
                }}
              >
                <option value="">Add position</option>
                {CALCULATOR_PERFORMANCE_VIEWS.map((view) => (
                  <option key={view.id} value={view.id}>
                    {view.label}
                  </option>
                ))}
                {choices.map((choice) => {
                  const price = lastOf(calculator, positionMaster, choice.symbol);
                  return (
                    <option key={choice.securityId} value={choice.securityId}>
                      {price == null ? choice.symbol : `${choice.symbol} ${formatUsd(price, 2)}`}
                    </option>
                  );
                })}
              </select>
            </td>
            <td>{pctOf(draftMarket, budgetMinor, 1)}</td>
            <td>
              <input
                aria-label={`Scenario ${slot} qty`}
                value={form.qty}
                placeholder={possibleQty > 0 ? String(possibleQty) : ""}
                disabled={busy || writesBlocked}
                onChange={(event) => onForm({ ...form, qty: event.target.value })}
                onBlur={commit}
                onKeyDown={(event) => {
                  if (event.key !== "Enter") return;
                  event.preventDefault();
                  commit();
                }}
              />
            </td>
            <td>{draftMarket == null ? "" : formatUsd(draftMarket, 2)}</td>
            <td>{draftMarket == null ? "" : `(${formatUsd(draftMarket, 2)})`}</td>
            <td>{draftParts.weekMinor == null ? "" : formatUsd(draftParts.weekMinor, 2)}</td>
            <td>{draftParts.monthMinor == null ? "" : formatUsd(draftParts.monthMinor, 2)}</td>
            <td>{draftYear == null ? "" : formatUsd(draftYear, 2)}</td>
            <td>{draftEach == null ? "" : formatUsd(draftEach, 2)}</td>
            <td>{pctOf(draftYear, draftMarket, 2)}</td>
            <td>
              {draftTier}
              <button
                type="button"
                aria-label={`Add scenario ${slot} row`}
                disabled={busy || writesBlocked || !form.securityId || form.qty.trim() === ""}
                onMouseDown={(event) => event.preventDefault()}
                onClick={onAdd}
              >
                Add row
              </button>
            </td>
            {onRemoveRow ? <td /> : null}
          </tr>
        }
      />
      {filterView ? (
        <div className="home-av-dialog-backdrop" onClick={() => setFilterView(null)}>
          <div
            role="dialog"
            aria-modal="true"
            aria-label="Filter view symbols"
            className="home-av-dialog"
            onClick={(event) => event.stopPropagation()}
          >
            <header>
              <h3>
                {CALCULATOR_PERFORMANCE_VIEWS.find((view) => view.id === filterView)?.label}
              </h3>
              <label>
                Payment frequency
                <select
                  aria-label="Filter view frequency"
                  value={filterCadence}
                  onChange={(event) => setFilterCadence(event.target.value)}
                >
                  {CALCULATOR_CADENCE_FILTERS.map((item) => (
                    <option key={item.id} value={item.id}>
                      {item.label}
                    </option>
                  ))}
                </select>
              </label>
              <button type="button" aria-label="Exit filter view" onClick={() => setFilterView(null)}>
                Exit
              </button>
            </header>
            <table aria-label="Filter view symbols">
              <thead>
                <tr>
                  <th scope="col">Symbol</th>
                  <th scope="col">FWD</th>
                  <th scope="col">MC FWD</th>
                  <th scope="col">YOC</th>
                  <th scope="col">Gain %</th>
                </tr>
              </thead>
              <tbody>
                {filterRows.length === 0 ? (
                  <tr>
                    <td colSpan={5}>No symbols</td>
                  </tr>
                ) : (
                  filterRows.map(({ choice, master }) => (
                    <tr key={choice.securityId}>
                      <td>
                        <button
                          type="button"
                          onClick={() => {
                            onForm({ ...form, securityId: choice.securityId });
                            setFilterView(null);
                          }}
                        >
                          {choice.symbol}
                        </button>
                      </td>
                      <td>{bpsFace(master?.planFwdYieldBps)}</td>
                      <td>{bpsFace(master?.mostCurrentFwdYieldBps)}</td>
                      <td>{bpsFace(master?.planYocBps)}</td>
                      <td>{bpsFace(master?.unrealizedPnlBps)}</td>
                    </tr>
                  ))
                )}
              </tbody>
            </table>
          </div>
        </div>
      ) : null}
    </>
  );
}

function bpsFace(bps: number | null | undefined): string {
  if (bps == null) return "";
  return formatBps(bps);
}
