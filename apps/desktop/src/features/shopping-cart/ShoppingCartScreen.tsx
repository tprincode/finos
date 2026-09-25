import type {
  AccountListItem,
  CalculatorGet,
  CartBuyLine,
  CurrentPriceGet,
  CartScenario,
  CartScenarioList,
  CartSellLine,
  CashPileGet,
  HoldingsGet,
  InvestmentGet,
  PositionMasterGet,
} from "@finos/app-contracts";
import { formatUsd } from "@finos/ui-components";
import { useEffect, useMemo, useRef, useState } from "react";
import { rankLowestCost, type LotCostOption } from "../shared/pickers";
import type { ResearchedSymbolOption } from "../shared/pickers";
import {
  CartStartWizard,
  wizardRailStep,
  type CartWizardPrompt,
  type SavedCartPlan,
} from "./CartStartWizard";
import { planAnnualCents } from "./cartPlan";
import { ExecutePanel } from "./ExecutePanel";
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
  securityId: string;
  symbol: string;
  qtyWhole: number;
  lastMinor: number;
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
    periods: body.planningPeriodsPerYear || periodsPerYear(body.paymentFrequency) || 0,
    remainingQuantityMinor: body.remainingQuantityMinor,
    quantityScale: body.quantityScale,
    remainingPerformanceMinor: body.remainingPerformanceMinor,
    remainingTaxMinor: body.remainingTaxMinor,
  };
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
  if (!calc?.planKnown || !calc.planPerShareMinor || !calc.planningPeriodsPerYear) return null;
  const priceCents = isCashSymbol(symbol) ? 100 : calc.lastPriceMinor;
  if (priceCents == null || priceCents <= 0) return null;
  const denom = 10 ** calc.planScale * priceCents;
  if (denom === 0) return null;
  return Math.round((calc.planPerShareMinor * calc.planningPeriodsPerYear * 1_000_000) / denom);
}

function periodsPerYear(freq: string): number | null {
  const key = freq.trim().toLowerCase();
  if (key === "weekly") return 52;
  if (key === "monthly") return 12;
  if (key === "quarterly") return 4;
  return null;
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
  if (calc?.planKnown && calc.planPerShareMinor && calc.planningPeriodsPerYear) {
    const annual = planAnnualCents(
      calc.planPerShareMinor,
      calc.planScale,
      calc.planningPeriodsPerYear,
      qtyWhole,
    );
    if (annual != null) return annual;
  }
  const row = masterRow(master, symbol);
  const periods = (row ? periodsPerYear(row.paymentFrequency) : null) ?? calc?.planningPeriodsPerYear ?? null;
  if (row?.planPerShareMinor && periods) {
    const annual = planAnnualCents(row.planPerShareMinor, row.planScale, periods, qtyWhole);
    if (annual != null) return annual;
  }
  return null;
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
  const qtyMinor = finiteMinor(line.qtyMinor);
  const unit = finiteMinor(line.unitMinor);
  const scale = finiteMinor(line.qtyScale) ?? 0;
  if (qtyMinor != null && unit != null && unit > 0) {
    const denom = 10 ** scale;
    if (Number.isFinite(denom) && denom > 0) return Math.round((qtyMinor * unit) / denom);
  }
  return finiteMinor(line.proceedsMinor) ?? 0;
}

/** Same dollars the open buy table shows: whole shares times last price. */
function buyLineMinor(line: CartBuyLine): number {
  const qty = finiteMinor(line.qtyWhole);
  const last = finiteMinor(line.lastMinor);
  if (qty != null && last != null && last > 0) return Math.round(qty * last);
  return finiteMinor(line.spendMinor) ?? 0;
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
    const priceMinor = line.unitMinor > 0 ? line.unitMinor : null;
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
    const priceMinor = line.lastMinor > 0 ? line.lastMinor : null;
    const known = facts[line.symbol.toUpperCase()];
    const yearMinor =
      planAnnualForSymbol(calculator, master, line.symbol, line.qtyWhole) ??
      line.planAnnualMinor ??
      factsAnnual(known, line.qtyWhole);
    const parts = roundedWeekMonth(yearMinor);
    const eachMinor =
      yearMinor == null || line.qtyWhole <= 0 ? null : Math.round(yearMinor / line.qtyWhole);
    const marketMinor = priceMinor == null ? null : line.qtyWhole * priceMinor;
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
    periods: periodsPerYear(row.paymentFrequency) ?? 0,
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
  if (rows.length === 0 || rows.some((row) => row.monthMinor == null)) return null;
  return rows.reduce((sum, row) => sum + (row.monthMinor ?? 0), 0);
}

function yearOf(rows: PlanSheetRow[]): number | null {
  if (rows.length === 0 || rows.some((row) => row.yearMinor == null)) return null;
  return rows.reduce((sum, row) => sum + (row.yearMinor ?? 0), 0);
}

export function ShoppingCartScreen({
  client,
  accounts,
  holdings,
  calculator,
  researched,
  positionMaster,
  asOfDate,
  busy,
  writesBlocked,
  onMessage,
  onBusy,
  onDirtyChange,
  onOpenBuyLot,
}: {
  client: CartClient;
  accounts: AccountListItem[];
  holdings: HoldingsGet | null;
  calculator: CalculatorGet | null;
  researched: ResearchedSymbolOption[];
  positionMaster: PositionMasterGet | null;
  asOfDate: string;
  busy?: boolean;
  writesBlocked?: boolean;
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
  const [accountCashMinor, setAccountCashMinor] = useState<number | null>(null);
  const [cashPile, setCashPile] = useState<CashPileGet | null>(null);
  const cashAlignedPlan = useRef<string | null>(null);
  const loadedFacts = useRef(new Set<string>());
  const [wizardPrompt, setWizardPrompt] = useState<CartWizardPrompt>("account");
  const [savedPlans, setSavedPlans] = useState<SavedCartPlan[]>([]);
  const [planId, setPlanId] = useState("");
  const [scenarioA, setScenarioA] = useState<CartScenario | null>(null);
  const [scenarioB, setScenarioB] = useState<CartScenario | null>(null);
  const [cashQty, setCashQty] = useState("");
  const [sellSymbol, setSellSymbol] = useState("");
  const [lotSellQty, setLotSellQty] = useState<Record<string, string>>({});
  const [quotedSellMinor, setQuotedSellMinor] = useState<number | null>(null);
  const [sellPlan, setSellPlan] = useState<{
    symbol: string;
    tier: string;
    annualPerShare: number | null;
  } | null>(null);
  const [sellNote, setSellNote] = useState("");
  const [cashTier, setCashTier] = useState("");
  const [buyA, setBuyA] = useState({ securityId: "", qty: "" });
  const [buyB, setBuyB] = useState({ securityId: "", qty: "" });
  const [depositAmount, setDepositAmount] = useState("");
  const [sold, setSold] = useState(false);
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

  const loadSavedPlans = async (account: string) => {
    const result = await client.executeQuery("CartScenarioList", { accountId: account });
    if (!result.ok || !result.bodyJson) {
      setSavedPlans([]);
      return;
    }
    const body = JSON.parse(result.bodyJson) as CartScenarioList;
    const grouped = new Map<string, CartScenario[]>();
    for (const item of body.items) {
      const id = item.planId || item.scenarioId;
      const current = grouped.get(id) ?? [];
      current.push(item);
      grouped.set(id, current);
    }
    setSavedPlans(
      [...grouped.entries()]
        .map(([id, scenes]) => {
          const slotA = scenes.find((item) => (item.slot ?? "A") === "A") ?? scenes[0];
          const slotB = scenes.find((item) => item.slot === "B") ?? null;
          const discardOrder = [slotB, slotA].filter((item): item is CartScenario => item != null);
          return {
            planId: id,
            name: slotA.name?.trim() || "Draft",
            asOf: slotA.asOf,
            status:
              slotB && slotB.status !== slotA.status
                ? `A ${slotA.status} · B ${slotB.status}`
                : slotA.status,
            scenarioCount: slotB ? 2 : 1,
            sellMinor: (slotA.sellLines ?? []).reduce((sum, line) => sum + sellLineMinor(line), 0),
            buyAMinor: (slotA.buyLines ?? []).reduce((sum, line) => sum + buyLineMinor(line), 0),
            buyBMinor: slotB
              ? (slotB.buyLines ?? []).reduce((sum, line) => sum + buyLineMinor(line), 0)
              : null,
            scenarioIds: [...new Set(discardOrder.map((item) => item.scenarioId))],
          };
        })
        .reverse(),
    );
  };

  useEffect(() => {
    if (wizardPrompt !== "plans" || !accountId) return;
    void loadSavedPlans(accountId);
  }, [wizardPrompt, accountId]);

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
    const pile = await client.executeQuery("CashPileGet", { accountId });
    if (pile.ok && pile.bodyJson) {
      const body = JSON.parse(pile.bodyJson) as CashPileGet;
      if (body.found && body.lotId && body.remainingQtyMinor > 0) {
        const cleared = await run("CartSellSymbolClear", {
          scenarioId: next.scenarioId,
          symbol: body.symbol || cashSymbol,
        });
        if (cleared) {
          const added = await run("CartSellLineAdd", {
            scenarioId: next.scenarioId,
            lotId: body.lotId,
            qtyMinor: body.remainingQtyMinor,
            unitMinor: 100,
            isCash: true,
          });
          if (added) await refreshPlan(next.planId, accountId);
        }
      }
    }
    onMessage("Plan opened.");
  };

  const cashLine = scenarioA?.sellLines.find((line) => line.isCash);
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
      const periods = body.planningPeriodsPerYear || periodsPerYear(body.paymentFrequency) || 0;
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
    const pending = lines.filter(
      (line) => line.securityId && !loadedFacts.current.has(line.securityId),
    );
    if (pending.length === 0) return;
    for (const line of pending) loadedFacts.current.add(line.securityId);
    void (async () => {
      const next: Record<string, SymbolFacts> = {};
      for (const line of pending) {
        const facts = await loadSymbolFacts(client, line.securityId);
        if (facts) next[line.symbol.toUpperCase()] = facts;
        else loadedFacts.current.delete(line.securityId);
      }
      if (Object.keys(next).length > 0) {
        setSymbolFacts((current) => ({ ...current, ...next }));
      }
    })();
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
      if (body.found && body.dollarsMinor > 0) setAccountCashMinor(body.dollarsMinor);
    });
    return () => {
      cancelled = true;
    };
  }, [accountId, client, scenarioA]);

  useEffect(() => {
    cashAlignedPlan.current = null;
  }, [planId]);

  useEffect(() => {
    if (!scenarioA || !planId || !accountId || !cashPile?.found || !cashPile.lotId) return;
    if (cashPile.dollarsMinor <= 0) return;
    if (cashAlignedPlan.current === planId) return;
    const lineDollars = cashLine ? Math.round(qtyOf(cashLine) * 100) : null;
    if (lineDollars === cashPile.dollarsMinor) {
      cashAlignedPlan.current = planId;
      return;
    }
    cashAlignedPlan.current = planId;
    const scenarioId = scenarioA.scenarioId;
    const pile = cashPile;
    void (async () => {
      const cleared = await run("CartSellSymbolClear", {
        scenarioId,
        symbol: pile.symbol || cashSymbol,
      });
      if (!cleared) return;
      const added = await run("CartSellLineAdd", {
        scenarioId,
        lotId: pile.lotId,
        qtyMinor: pile.remainingQtyMinor,
        unitMinor: 100,
        isCash: true,
      });
      if (added) await refreshPlan(planId, accountId);
    })();
  }, [scenarioA, planId, accountId, cashPile, cashLine, cashSymbol]);

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
    let unitMinor = lastOf(calculator, positionMaster, sellSymbol) ?? quotedSellMinor;
    if (unitMinor == null || unitMinor <= 0) {
      const securityId = masterRow(positionMaster, sellSymbol)?.securityId;
      if (securityId) {
        const priced = await client.executeQuery("CurrentPriceGet", { securityId });
        if (priced.ok && priced.bodyJson) {
          const price = JSON.parse(priced.bodyJson) as CurrentPriceGet;
          if (price.priceMinor != null && price.priceMinor > 0) {
            unitMinor = priceToCents(price.priceMinor, price.scale);
          }
        }
      }
    }
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

  const removeSellSymbol = async (symbol: string) => {
    if (!scenarioA || !planId || !accountId || isCashSymbol(symbol)) return;
    const next = await run("CartSellSymbolClear", {
      scenarioId: scenarioA.scenarioId,
      symbol,
    });
    if (next) {
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
      let known = symbolFacts[row.symbol.toUpperCase()];
      if (!known) {
        known = (await loadSymbolFacts(client, row.securityId)) ?? undefined;
        if (known) {
          setSymbolFacts((current) => ({ ...current, [row.symbol.toUpperCase()]: known as SymbolFacts }));
        }
      }
      let lastMinor = lastOf(calculator, positionMaster, row.symbol);
      if (lastMinor == null) {
        const priced = await client.executeQuery("CurrentPriceGet", { securityId: row.securityId });
        if (priced.ok && priced.bodyJson) {
          const price = JSON.parse(priced.bodyJson) as CurrentPriceGet;
          if (price.priceMinor != null && price.priceMinor > 0) lastMinor = price.priceMinor;
        }
      }
      if (lastMinor == null) {
        onMessage("Last price unknown — will not invent $0.");
        return;
      }
      const next = await run("CartBuyLineAdd", {
        scenarioId: scene.scenarioId,
        securityId: row.securityId,
        qtyWhole,
        lastMinor,
        planAnnualMinor:
          planAnnualForSymbol(calculator, positionMaster, row.symbol, qtyWhole) ??
          factsAnnual(known, qtyWhole),
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
    const lastMinor = lastOf(calculator, positionMaster, line.symbol) ?? line.lastMinor;
    const next = await run("CartBuyLineQtySet", {
      scenarioId: scene.scenarioId,
      lineId,
      qtyWhole,
      lastMinor,
      planAnnualMinor:
        planAnnualForSymbol(calculator, positionMaster, line.symbol, qtyWhole) ??
        factsAnnual(symbolFacts[line.symbol.toUpperCase()], qtyWhole),
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

  const confirmSell = async () => {
    const scene = [scenarioA, scenarioB].find(
      (item) => item && (item.status === "agreed" || item.status === "executing"),
    );
    if (!scene) return;
    const next = await run("CartExecuteSell", {
      scenarioId: scene.scenarioId,
      occurredOn: asOfDate,
    });
    if (next && planId && accountId) {
      setSold(true);
      await refreshPlan(planId, accountId);
      onMessage("Sell posted and assigned. Open Add Lot to buy.");
    }
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
      setWizardPrompt("plans");
      markDirty(false);
      if (accountId) await loadSavedPlans(accountId);
      onMessage("Plan discarded.");
    } finally {
      onBusy(false);
    }
  };

  const openLot = () => {
    const scene = [scenarioA, scenarioB].find(
      (item) => item && (item.status === "agreed" || item.status === "executing"),
    );
    const buy = scene?.buyLines[0];
    if (!scene || !buy) return;
    onOpenBuyLot({
      scenarioId: scene.scenarioId,
      accountId: scene.accountId,
      securityId: buy.securityId,
      symbol: buy.symbol,
      qtyWhole: buy.qtyWhole,
      lastMinor: buy.lastMinor,
      openedOn: asOfDate,
    });
  };

  const sellLines = scenarioA?.sellLines ?? [];
  const positionLines = sellLines.filter((line) => !line.isCash && !isCashSymbol(line.symbol));
  const yieldBps =
    scenarioA && scenarioA.cashYieldBps > 0
      ? scenarioA.cashYieldBps
      : cashYieldBps(calculator, positionMaster, cashSymbol);
  const cashQtyNumber = Number(cashQty);
  const cashMarketMinor =
    Number.isFinite(cashQtyNumber) && cashQtyNumber > 0 ? Math.round(cashQtyNumber * 100) : null;
  const cashYearMinor = (() => {
    if (cashMarketMinor == null) return null;
    const planned = annualForQty(calculator, positionMaster, cashSymbol, cashQtyNumber);
    if (planned != null) return planned;
    if (yieldBps != null && yieldBps > 0) return Math.round((cashMarketMinor * yieldBps) / 10_000);
    return null;
  })();
  const positionRows = sellSheetRows(positionLines, null, calculator, positionMaster, yieldBps);
  const positionDollars = dollarsOf(positionRows);
  const sellTotal =
    cashMarketMinor == null && positionDollars == null
      ? null
      : (cashMarketMinor ?? 0) + (positionDollars ?? 0);
  const cashParts = roundedWeekMonth(cashYearMinor);
  const cashRow: PlanSheetRow = {
    key: "account-cash",
    priceMinor: 100,
    symbol: cashSymbol,
    alloc: pctOf(cashMarketMinor, sellTotal, 1),
    qty: cashQty,
    qtyNode: (
      <input
        aria-label="Sell cash qty"
        value={cashQty}
        disabled={busy || writesBlocked}
        onChange={(event) => setCashQty(event.target.value)}
        onBlur={() => void commitCashQty()}
      />
    ),
    marketMinor: cashMarketMinor,
    weekMinor: cashParts.weekMinor,
    monthMinor: cashParts.monthMinor,
    yearMinor: cashYearMinor,
    eachMinor:
      cashYearMinor == null || !(cashQtyNumber > 0) ? null : Math.round(cashYearMinor / cashQtyNumber),
    yieldText: pctOf(cashYearMinor, cashMarketMinor, 2),
    tier: tierOf(positionMaster, cashSymbol) || cashTier,
    taxGainMinor: null,
    performanceGainMinor: null,
  };
  const sellRowsAllocated = [
    cashRow,
    ...sellSheetRows(positionLines, sellTotal, calculator, positionMaster, yieldBps),
  ];
  const accountCash = accountCashMinor ?? cashMarketMinor;
  const planCash = cashMarketMinor;
  const unsoldMinor =
    accountCash != null && planCash != null ? Math.max(0, accountCash - planCash) : 0;
  const cashStillHeld = (buySpend: number | null): number | null => {
    if (planCash == null || buySpend == null) return null;
    const fromPositions = positionDollars ?? 0;
    const cashSpent = Math.min(planCash, Math.max(0, buySpend - fromPositions));
    return Math.max(0, planCash - cashSpent);
  };
  const cashHeldRow = (key: string, marketMinor: number | null): PlanSheetRow | null => {
    if (marketMinor == null || marketMinor <= 0) return null;
    const qty = marketMinor / 100;
    const planned = annualForQty(calculator, positionMaster, cashSymbol, qty);
    const yearMinor =
      planned ?? (yieldBps != null && yieldBps > 0 ? Math.round((marketMinor * yieldBps) / 10_000) : null);
    const parts = roundedWeekMonth(yearMinor);
    return {
      key,
      priceMinor: 100,
      symbol: `Remaining ${cashSymbol}`,
      alloc: pctOf(marketMinor, sellTotal, 1),
      qty: String(Math.round(qty * 100) / 100),
      marketMinor,
      weekMinor: parts.weekMinor,
      monthMinor: parts.monthMinor,
      yearMinor,
      eachMinor: yearMinor == null || qty <= 0 ? null : Math.round(yearMinor / qty),
      yieldText: pctOf(yearMinor, marketMinor, 2),
      tier: tierOf(positionMaster, cashSymbol) || cashTier,
    };
  };
  const unsoldRow = cashHeldRow("unsold-cash", unsoldMinor > 0 ? unsoldMinor : null);
  const purchasesA = scenarioA
    ? buySheetRows(scenarioA.buyLines, sellTotal, calculator, positionMaster, symbolFacts)
    : [];
  const purchasesB = scenarioB
    ? buySheetRows(scenarioB.buyLines, sellTotal, calculator, positionMaster, symbolFacts)
    : [];
  const withQty = (
    slot: "A" | "B",
    rows: PlanSheetRow[],
    lines: CartScenario["buyLines"],
    onQty: (lineId: string, qtyWhole: number) => void,
    onRemove: (lineId: string) => void,
  ): PlanSheetRow[] =>
    rows.map((row) => {
      const line = lines.find((item) => item.lineId === row.key);
      if (!line) return row;
      return {
        ...row,
        qtyNode: (
          <>
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
            <button
              type="button"
              aria-label={`Remove scenario ${slot} ${line.symbol}`}
              disabled={busy || writesBlocked}
              onClick={() => onRemove(line.lineId)}
            >
              Remove
            </button>
          </>
        ),
      };
    });
  const remainA = cashHeldRow("remaining-cash-a", cashStillHeld(dollarsOf(purchasesA)));
  const remainB = scenarioB ? cashHeldRow("remaining-cash-b", cashStillHeld(dollarsOf(purchasesB))) : null;
  const buyRowsA = scenarioA
    ? withQty(
        "A",
        purchasesA,
        scenarioA.buyLines,
        (lineId, qty) => void setBuyQty(scenarioA, lineId, qty),
        (lineId) => void removeBuy(scenarioA, lineId),
      )
    : [];
  const buyRowsB = scenarioB
    ? withQty(
        "B",
        purchasesB,
        scenarioB.buyLines,
        (lineId, qty) => void setBuyQty(scenarioB, lineId, qty),
        (lineId) => void removeBuy(scenarioB, lineId),
      )
    : [];
  const depositMinor = scenarioA?.depositMinor ?? 0;
  const agreedScene = [scenarioA, scenarioB].find(
    (item) => item && (item.status === "agreed" || item.status === "executing"),
  );
  const step = !scenarioA
    ? wizardRailStep(wizardPrompt)
    : !agreedScene
      ? "Evaluate"
      : !sold
        ? "Confirm sell"
        : "Open lot";

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

  const short =
    sellTotal != null &&
    ((spendOf(buyRowsA) ?? 0) > sellTotal + depositMinor ||
      (scenarioB != null && (spendOf(buyRowsB) ?? 0) > sellTotal + depositMinor));

  const returnText = (rows: PlanSheetRow[]) => {
    const year = yearOf(rows);
    return sellTotal != null && year != null ? pctOf(year, sellTotal, 2) : "";
  };

  const sellSymbols = [
    ...new Set(sellLines.filter((line) => !line.isCash).map((line) => line.symbol)),
  ];

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
      <p>
        One sell table funds Scenario A and, if you add it, Scenario B. Drafts do not change
        Holdings or Income Plan.
      </p>
      <StepRail current={step} />
      {!scenarioA ? (
        <CartStartWizard
          prompt={wizardPrompt}
          accounts={accounts}
          accountId={accountId}
          accountName={accountName || "this account"}
          planName={draftName}
          savedPlans={savedPlans}
          busy={busy}
          writesBlocked={writesBlocked}
          onAccountId={(id) => {
            setAccountId(id);
            suggestPlanName(id);
          }}
          onPlanName={setDraftName}
          onOpenPlan={(id) => void openSavedPlan(id)}
          onDeletePlan={(plan) => void deleteSavedPlan(plan)}
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
          {sellNote ? <p aria-label="Sell note">{sellNote}</p> : null}
          <PlanSheetTable
            title="Sell"
            ariaLabel="Sell plan"
            budgetMinor={sellTotal}
            rows={sellRowsAllocated}
            footerRows={unsoldRow ? [unsoldRow] : undefined}
            showPnl
            returnText={returnText(sellRowsAllocated)}
            returnLabel="Return"
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
                </tr>
              </>
            }
          />
          <div className="form-grid">
            {sellSymbols.map((symbol) => (
              <button
                key={symbol}
                type="button"
                aria-label={`Remove sell ${symbol}`}
                disabled={busy || writesBlocked}
                onClick={() => void removeSellSymbol(symbol)}
              >
                Remove {symbol}
              </button>
            ))}
          </div>
          <BuyBlock
            slot="A"
            rows={buyRowsA}
            budgetMinor={sellTotal}
            returnText={returnText(buyRowsA)}
            choices={buyChoices}
            client={client}
            calculator={calculator}
            positionMaster={positionMaster}
            symbolFacts={symbolFacts}
            onFacts={(symbol, facts) =>
              setSymbolFacts((current) => ({ ...current, [symbol.toUpperCase()]: facts }))
            }
            form={buyA}
            onForm={setBuyA}
            footerRows={remainA ? [remainA] : undefined}
            busy={busy}
            writesBlocked={writesBlocked}
            onAdd={() => void addBuy("A")}
          />
          {scenarioB ? (
            <BuyBlock
              slot="B"
              rows={buyRowsB}
              budgetMinor={sellTotal}
              returnText={returnText(buyRowsB)}
            choices={buyChoices}
            client={client}
            calculator={calculator}
            positionMaster={positionMaster}
            symbolFacts={symbolFacts}
            onFacts={(symbol, facts) =>
              setSymbolFacts((current) => ({ ...current, [symbol.toUpperCase()]: facts }))
            }
              form={buyB}
              onForm={setBuyB}
              footerRows={remainB ? [remainB] : undefined}
              busy={busy}
              writesBlocked={writesBlocked}
              onAdd={() => void addBuy("B")}
            />
          ) : null}
          <ScenarioDelta
            sellRows={sellRowsAllocated}
            rowsA={buyRowsA}
            rowsB={scenarioB ? buyRowsB : null}
            cashBefore={planCash}
            cashAfterA={cashStillHeld(spendOf(buyRowsA))}
            cashAfterB={scenarioB ? cashStillHeld(spendOf(buyRowsB)) : null}
          />
          <section aria-label="Plan critique">
            <p>{critiqueFor("A", buyRowsA, scenarioB == null)}</p>
            {scenarioB ? (
              <>
                <p>{critiqueFor("B", buyRowsB, false)}</p>
                <p>
                  {compareCritique({
                    sellDollars: sellTotal,
                    aMonth: monthOf(buyRowsA),
                    bMonth: monthOf(buyRowsB),
                    aYear: yearOf(buyRowsA),
                    bYear: yearOf(buyRowsB),
                    aUnspent:
                      sellTotal == null || spendOf(buyRowsA) == null
                        ? null
                        : sellTotal - (spendOf(buyRowsA) ?? 0),
                    bUnspent:
                      sellTotal == null || spendOf(buyRowsB) == null
                        ? null
                        : sellTotal - (spendOf(buyRowsB) ?? 0),
                    aTypes: [...new Set(buyRowsA.map((row) => row.tier).filter(Boolean))].join(" and "),
                    bTypes: [...new Set(buyRowsB.map((row) => row.tier).filter(Boolean))].join(" and "),
                    aEnough:
                      sellTotal == null || spendOf(buyRowsA) == null
                        ? null
                        : (spendOf(buyRowsA) ?? 0) <= sellTotal + depositMinor,
                    bEnough:
                      sellTotal == null || spendOf(buyRowsB) == null
                        ? null
                        : (spendOf(buyRowsB) ?? 0) <= sellTotal + depositMinor,
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
          <ExecutePanel
            agreed={Boolean(agreedScene)}
            sold={sold}
            busy={busy}
            writesBlocked={writesBlocked}
            onConfirmSell={() => void confirmSell()}
            onOpenLot={openLot}
          />
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
  symbolFacts,
  onFacts,
  form,
  onForm,
  busy,
  writesBlocked,
  onAdd,
  footerRows,
}: {
  slot: "A" | "B";
  rows: PlanSheetRow[];
  budgetMinor: number | null;
  returnText: string;
  choices: ResearchedSymbolOption[];
  client: CartClient;
  calculator: CalculatorGet | null;
  positionMaster: PositionMasterGet | null;
  symbolFacts: Record<string, SymbolFacts>;
  onFacts: (symbol: string, facts: SymbolFacts) => void;
  form: { securityId: string; qty: string };
  onForm: (next: { securityId: string; qty: string }) => void;
  busy?: boolean;
  writesBlocked?: boolean;
  onAdd: () => void;
  footerRows?: PlanSheetRow[];
}) {
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
        editor={
          <tr>
            <td>{lastMinor == null ? "" : formatUsd(lastMinor, 2)}</td>
            <td>
              <select
                aria-label={`Scenario ${slot} symbol`}
                value={form.securityId}
                disabled={busy || writesBlocked}
                onChange={(event) => onForm({ ...form, securityId: event.target.value })}
              >
                <option value="">Add position</option>
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
          </tr>
        }
      />
    </>
  );
}
