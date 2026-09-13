import type {
  AccountListItem,
  CalculatorGet,
  CartScenario,
  CartScenarioList,
  CashLedgerGet,
  CashPileGet,
  CurrentPriceGet,
  HoldingsGet,
  PositionMasterGet,
} from "@finos/app-contracts";
import { formatBps } from "@finos/ui-components";
import { useEffect, useMemo, useState } from "react";
import {
  LotCostTable,
  ResearchedSymbolCombobox,
  type LotSortMode,
  type ResearchedSymbolOption,
} from "../shared/pickers";
import { AccountCashPlan } from "./AccountCashPlan";
import { AffordStrip } from "./AffordStrip";
import { CartBlendTable, type BlendRow } from "./CartBlendTable";
import {
  CartStartWizard,
  noCashAccount,
  wizardRailStep,
  type CartFunding,
  type CartWizardPrompt,
} from "./CartStartWizard";
import { ComparePlans } from "./ComparePlans";
import { incomeWeekMonth, planAnnualCents } from "./cartPlan";
import { ExecutePanel } from "./ExecutePanel";
import { IncomeCompare } from "./IncomeCompare";
import { MixBars, mixWorsens, type MixSlice } from "./MixBars";
import { StepRail } from "./StepRail";
import { TradeoffCallout } from "./TradeoffCallout";

const CASH = new Set(["SPAXX", "CASH", "FDRXX", "SWVXX"]);

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

function emptyMix(): MixSlice {
  return { foundation: 0, core: 0, riskOn: 0 };
}

function addTier(mix: MixSlice, tier: string, value: number) {
  const key = tier.trim();
  if (key === "Foundation") mix.foundation += value;
  else if (key === "Risk On") mix.riskOn += value;
  else mix.core += value;
}

function masterRow(master: PositionMasterGet | null, symbol: string) {
  return master?.rows.find((r) => r.symbol.toUpperCase() === symbol.toUpperCase());
}

function riskOf(master: PositionMasterGet | null, symbol: string): string {
  const tier = masterRow(master, symbol)?.riskTier?.trim();
  return tier || "Core";
}

function lastOf(
  calculator: CalculatorGet | null,
  master: PositionMasterGet | null,
  symbol: string,
): number | null {
  const calc = calculator?.rows.find((r) => r.symbol === symbol);
  if (calc?.lastPriceMinor != null) return calc.lastPriceMinor;
  const row = masterRow(master, symbol);
  return row?.lastPriceMinor ?? null;
}

function cashYieldBps(
  calculator: CalculatorGet | null,
  master: PositionMasterGet | null,
  symbol: string,
): number | null {
  const row = masterRow(master, symbol);
  if (row?.planFwdYieldBps != null) return row.planFwdYieldBps;
  const calc = calculator?.rows.find((r) => r.symbol === symbol);
  if (!calc?.planKnown || !calc.planPerShareMinor || !calc.planningPeriodsPerYear) return null;
  const priceCents = CASH.has(symbol.toUpperCase()) ? 100 : calc.lastPriceMinor;
  if (priceCents == null || priceCents <= 0) return null;
  const denom = 10 ** calc.planScale * priceCents;
  if (denom === 0) return null;
  return Math.round(
    (calc.planPerShareMinor * calc.planningPeriodsPerYear * 1_000_000) / denom,
  );
}

function lotDollars(qtyMinor: number, qtyScale: number, unitMinor: number): number {
  const denom = 10 ** qtyScale;
  if (denom === 0) return 0;
  return Math.round((qtyMinor * unitMinor) / denom);
}

function fundingOf(scene: CartScenario | null): CartFunding {
  const raw = scene?.fundingSource;
  if (raw === "accountCash" || raw === "newDeposit") return raw;
  return "sellLots";
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
  const calc = calculator?.rows.find((r) => r.symbol === symbol);
  if (calc?.planKnown && calc.planPerShareMinor && calc.planningPeriodsPerYear) {
    return planAnnualCents(
      calc.planPerShareMinor,
      calc.planScale,
      calc.planningPeriodsPerYear,
      qtyWhole,
    );
  }
  const row = masterRow(master, symbol);
  const periods = row ? periodsPerYear(row.paymentFrequency) : null;
  if (row?.planKnown && row.planPerShareMinor && periods) {
    return planAnnualCents(row.planPerShareMinor, row.planScale, periods, qtyWhole);
  }
  return null;
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
  const [wizardPrompt, setWizardPrompt] = useState<CartWizardPrompt>("account");
  const [funding, setFunding] = useState<CartFunding>("sellLots");
  const [sortMode, setSortMode] = useState<LotSortMode>("lowest-cost");
  const [drafts, setDrafts] = useState<CartScenario[]>([]);
  const [scenario, setScenario] = useState<CartScenario | null>(null);
  const [lotId, setLotId] = useState("");
  const [sellQty, setSellQty] = useState("");
  const [buyQuery, setBuyQuery] = useState("");
  const [buyOpen, setBuyOpen] = useState(false);
  const [buySecurityId, setBuySecurityId] = useState("");
  const [overrideReason, setOverrideReason] = useState("");
  const [sold, setSold] = useState(false);
  const [cashFilled, setCashFilled] = useState(false);
  const [depositAmount, setDepositAmount] = useState("");
  const [fills, setFills] = useState<Record<string, string>>({});
  const [cashPile, setCashPile] = useState<CashPileGet | null>(null);
  const [cashLedger, setCashLedger] = useState<CashLedgerGet | null>(null);
  const [dirty, setDirty] = useState(false);

  const accountName = accounts.find((a) => a.accountId === accountId)?.name ?? "";
  const lots = holdings?.lots ?? [];
  const selectedLot = lots.find((l) => l.lotId === lotId);
  const accountLots = accountName
    ? lots.filter((l) => l.accountName === accountName)
    : lots;
  const cashLots = accountLots.filter((l) => CASH.has(l.symbol.toUpperCase()));
  const lastBySymbol = useMemo(() => {
    const map: Record<string, number | null> = {};
    for (const row of calculator?.rows ?? []) {
      map[row.symbol] = row.lastPriceMinor;
    }
    for (const row of positionMaster?.rows ?? []) {
      if (map[row.symbol] == null) map[row.symbol] = row.lastPriceMinor;
    }
    return map;
  }, [calculator, positionMaster]);

  const accountCashOffered = Boolean(accountName) && !noCashAccount(accountName);

  const refreshCash = async (id: string) => {
    const pile = await client.executeQuery("CashPileGet", { accountId: id });
    if (pile.ok && pile.bodyJson) {
      setCashPile(JSON.parse(pile.bodyJson) as CashPileGet);
    }
    const ledger = await client.executeQuery("CashLedgerGet", { accountId: id });
    if (ledger.ok && ledger.bodyJson) {
      setCashLedger(JSON.parse(ledger.bodyJson) as CashLedgerGet);
    }
  };

  useEffect(() => {
    if (!accountId) return;
    void refreshCash(accountId);
  }, [accountId]);

  useEffect(() => {
    if (!accountCashOffered && funding === "accountCash") {
      setFunding("sellLots");
    }
  }, [accountCashOffered, funding]);

  const refreshDrafts = async (id: string) => {
    const result = await client.executeQuery("CartScenarioList", { accountId: id });
    if (result.ok && result.bodyJson) {
      const body = JSON.parse(result.bodyJson) as CartScenarioList;
      setDrafts(body.items);
    }
  };

  const filteredBuys = useMemo(() => {
    const q = buyQuery.trim().toUpperCase();
    const rows = researched.filter((r) => !CASH.has(r.symbol.toUpperCase()));
    if (!q) return rows.slice(0, 12);
    return rows
      .filter((r) => r.symbol.toUpperCase().includes(q) || r.name.toUpperCase().includes(q))
      .slice(0, 12);
  }, [researched, buyQuery]);

  const markDirty = (next: boolean) => {
    setDirty(next);
    onDirtyChange(next);
  };

  const applyScene = (next: CartScenario | null) => {
    setScenario(next);
    markDirty(Boolean(next) && next?.status === "draft");
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

  const create = async () => {
    if (!accountId) {
      onMessage("Choose an account.");
      return;
    }
    const next = await run("CartScenarioCreate", {
      accountId,
      asOf: asOfDate,
      name: draftName.trim() || "Draft",
      fundingSource: funding,
    });
    if (next) {
      applyScene(next);
      await refreshDrafts(accountId);
      onMessage("Draft swap created.");
    }
  };

  const addSell = async () => {
    if (!scenario || !lotId || !selectedLot) {
      onMessage("Choose a named lot.");
      return;
    }
    const qty = Number(sellQty);
    if (!Number.isFinite(qty) || qty <= 0) {
      onMessage("Sell qty must be positive.");
      return;
    }
    const qtyMinor = Math.round(qty * 10 ** selectedLot.quantityScale);
    const isCash = CASH.has(selectedLot.symbol.toUpperCase());
    const unitMinor = isCash
      ? 100
      : lastOf(calculator, positionMaster, selectedLot.symbol);
    if (unitMinor == null) {
      onMessage("Last price unknown — will not invent $0.");
      return;
    }
    const next = await run("CartSellLineAdd", {
      scenarioId: scenario.scenarioId,
      lotId,
      qtyMinor,
      unitMinor,
      isCash,
    });
    if (next) {
      applyScene(next);
      onMessage(`Sell line ${selectedLot.symbol} added.`);
    }
  };

  const addBuy = async () => {
    if (!scenario || !buySecurityId) {
      onMessage("Choose a researched buy.");
      return;
    }
    const qtyWhole = 1;
    const row = researched.find((r) => r.securityId === buySecurityId);
    const symbol = row?.symbol ?? "";
    if (scenario.buyLines.some((l) => l.symbol.toUpperCase() === symbol.toUpperCase())) {
      onMessage(`${symbol} is already in the blend. Change qty on that row.`);
      return;
    }
    let lastMinor = lastOf(calculator, positionMaster, symbol);
    if (lastMinor == null) {
      const priced = await client.executeQuery("CurrentPriceGet", {
        securityId: buySecurityId,
      });
      if (priced.ok && priced.bodyJson) {
        const body = JSON.parse(priced.bodyJson) as CurrentPriceGet;
        lastMinor = body.priceMinor;
      }
    }
    if (lastMinor == null) {
      onMessage("Last price unknown — will not invent $0.");
      return;
    }
    const planAnnual = planAnnualForSymbol(calculator, positionMaster, symbol, qtyWhole);
    const next = await run("CartBuyLineAdd", {
      scenarioId: scenario.scenarioId,
      securityId: buySecurityId,
      qtyWhole,
      lastMinor,
      planAnnualMinor: planAnnual,
    });
    if (next) {
      applyScene(next);
      setBuySecurityId("");
      setBuyQuery("");
      onMessage(`${row?.symbol ?? ""} added to the blend.`);
    }
  };

  const setBuyQty = async (lineId: string, qtyWhole: number) => {
    if (!scenario) return;
    const line = scenario.buyLines.find((l) => l.lineId === lineId);
    if (!line) return;
    const lastMinor = lastOf(calculator, positionMaster, line.symbol) ?? line.lastMinor;
    const planAnnual = planAnnualForSymbol(calculator, positionMaster, line.symbol, qtyWhole);
    const next = await run("CartBuyLineQtySet", {
      scenarioId: scenario.scenarioId,
      lineId,
      qtyWhole,
      lastMinor,
      planAnnualMinor: planAnnual,
    });
    if (next) {
      applyScene(next);
    }
  };

  const evaluate = async () => {
    if (!scenario) return;
    onBusy(true);
    try {
      const result = await client.executeQuery("CartScenarioEvaluate", {
        scenarioId: scenario.scenarioId,
      });
      if (!result.ok) {
        onMessage(`Evaluate failed: ${result.errorCode ?? "error"}`);
        return;
      }
      const next = parseScenario(result.bodyJson);
      if (next) {
        applyScene(next);
        if (accountId) await refreshDrafts(accountId);
        onMessage("Evaluated. Draft does not change Holdings or Income Plan.");
      }
    } finally {
      onBusy(false);
    }
  };

  const save = async () => {
    if (!scenario) return;
    const next = await run("CartScenarioSave", { scenarioId: scenario.scenarioId });
    if (next) {
      applyScene(next);
      markDirty(false);
      onMessage("Draft saved.");
    }
  };

  const agree = async () => {
    if (!scenario) return;
    const next = await run("CartScenarioAgree", {
      scenarioId: scenario.scenarioId,
      overrideReason: overrideReason.trim() || undefined,
      mixWorsens: worse,
    });
    if (next) {
      setScenario(next);
      markDirty(false);
      onMessage("Swap agreed. Confirm sell, then Open lot.");
    }
  };

  const confirmSell = async () => {
    if (!scenario) return;
    const next = await run("CartExecuteSell", {
      scenarioId: scenario.scenarioId,
      occurredOn: asOfDate,
    });
    if (next) {
      setScenario(next);
      setSold(true);
      onMessage("Sell posted and assigned. Open Add Lot to buy.");
    }
  };

  const postDeposit = async () => {
    if (!accountId) return;
    const dollars = Number(depositAmount);
    if (!Number.isFinite(dollars) || dollars <= 0) {
      onMessage("Deposit must be a positive dollar amount.");
      return;
    }
    const amountMinor = Math.round(dollars * 100);
    onBusy(true);
    try {
      const result = await client.executeCommand("CashDeposit", {
        accountId,
        amountMinor,
        occurredOn: asOfDate,
      });
      if (!result.ok) {
        onMessage(`CashDeposit failed: ${result.errorCode ?? "error"}`);
        return;
      }
      setDepositAmount("");
      await refreshCash(accountId);
      onMessage("Deposit posted. Cash qty updated. Draft did not spend it.");
    } finally {
      onBusy(false);
    }
  };

  const confirmFill = async () => {
    if (!scenario) return;
    const nextFills = scenario.buyLines.map((line) => {
      const typed = fills[line.lineId];
      const dollars = typed == null || typed.trim() === ""
        ? line.lastMinor / 100
        : Number(typed);
      return {
        lineId: line.lineId,
        fillMinor: Number.isFinite(dollars) ? Math.round(dollars * 100) : 0,
      };
    });
    if (nextFills.some((f) => f.fillMinor <= 0)) {
      onMessage("Confirm the price paid for each buy.");
      return;
    }
    const next = await run("CartExecuteFill", {
      scenarioId: scenario.scenarioId,
      occurredOn: asOfDate,
      fills: nextFills,
    });
    if (next) {
      setScenario(next);
      setCashFilled(true);
      await refreshCash(accountId);
      onMessage("Fill confirmed. Cash deducted. Open Add Lot to buy.");
    }
  };

  const discard = async () => {
    if (!scenario || scenario.status !== "draft") {
      applyScene(null);
      setSold(false);
      setCashFilled(false);
      setWizardPrompt("account");
      return;
    }
    onBusy(true);
    try {
      const result = await client.executeCommand("CartScenarioDiscard", {
        scenarioId: scenario.scenarioId,
      });
      if (!result.ok) {
        onMessage(`Discard failed: ${result.errorCode ?? "error"}`);
        return;
      }
      applyScene(null);
      setSold(false);
      setCashFilled(false);
      setWizardPrompt("account");
      markDirty(false);
      onMessage("Draft discarded.");
      if (accountId) await refreshDrafts(accountId);
    } finally {
      onBusy(false);
    }
  };

  const duplicate = async () => {
    if (!scenario) return;
    const next = await run("CartScenarioDuplicate", { scenarioId: scenario.scenarioId });
    if (next) {
      applyScene(next);
      setDraftName(next.name || "Draft (copy)");
      if (accountId) await refreshDrafts(accountId);
      onMessage("Draft duplicated.");
    }
  };

  const renameDraft = async () => {
    if (!scenario) return;
    const next = await run("CartScenarioRename", {
      scenarioId: scenario.scenarioId,
      name: draftName.trim() || "Draft",
    });
    if (next) {
      applyScene(next);
      if (accountId) await refreshDrafts(accountId);
      onMessage("Draft renamed.");
    }
  };

  const openLot = () => {
    const buy = scenario?.buyLines[0];
    if (!scenario || !buy) return;
    onOpenBuyLot({
      scenarioId: scenario.scenarioId,
      accountId: scenario.accountId,
      securityId: buy.securityId,
      symbol: buy.symbol,
      qtyWhole: buy.qtyWhole,
      lastMinor: buy.lastMinor,
      openedOn: asOfDate,
    });
  };

  const currentMix = useMemo(() => {
    const mix = emptyMix();
    for (const lot of accountLots) {
      if (CASH.has(lot.symbol.toUpperCase())) continue;
      const last = lastOf(calculator, positionMaster, lot.symbol);
      if (last == null) continue;
      addTier(
        mix,
        riskOf(positionMaster, lot.symbol),
        lotDollars(lot.remainingQuantityMinor, lot.quantityScale, last),
      );
    }
    return mix;
  }, [accountLots, calculator, positionMaster]);

  const afterMix = useMemo(() => {
    const mix = { ...currentMix };
    for (const buy of scenario?.buyLines ?? []) {
      addTier(mix, riskOf(positionMaster, buy.symbol), buy.spendMinor);
    }
    return mix;
  }, [currentMix, scenario, positionMaster]);

  const cashCurrentMinor = cashLots.reduce(
    (sum, lot) => sum + lotDollars(lot.remainingQuantityMinor, lot.quantityScale, 100),
    0,
  );
  const worse = mixWorsens(currentMix, afterMix);
  const ev = scenario?.eval ?? null;
  const cashAfterMinor = ev ? Math.max(ev.leftoverMinor, 0) : cashCurrentMinor;
  const cashSymbol = cashLots[0]?.symbol ?? "";
  const collectorYieldBps =
    scenario && scenario.cashYieldBps > 0
      ? scenario.cashYieldBps
      : cashSymbol
        ? cashYieldBps(calculator, positionMaster, cashSymbol)
        : null;
  const keepAnnualMinor =
    cashCurrentMinor > 0 && collectorYieldBps != null && collectorYieldBps > 0
      ? Math.trunc((cashCurrentMinor * collectorYieldBps) / 10_000)
      : null;
  const sceneFunding = fundingOf(scenario);
  const parkedFunding = Boolean(scenario) && sceneFunding === "newDeposit";
  const accountCashFlow = Boolean(scenario) && sceneFunding === "accountCash";
  const allocatedMinor = accountCashFlow
    ? cashPile?.dollarsMinor ?? cashCurrentMinor
    : ev
      ? ev.remainingMinor
      : scenario
        ? scenario.sellLines.reduce((s, l) => s + l.proceedsMinor, 0) || cashCurrentMinor
        : cashCurrentMinor;
  const blendRows: BlendRow[] = (scenario?.buyLines ?? []).map((line) => {
    const lastMinor = lastOf(calculator, positionMaster, line.symbol) ?? line.lastMinor;
    const spendMinor = line.qtyWhole * lastMinor;
    const yearMinor =
      planAnnualForSymbol(calculator, positionMaster, line.symbol, line.qtyWhole) ??
      line.planAnnualMinor;
    const { weekMinor, monthMinor } = incomeWeekMonth(yearMinor);
    const eachAnnualMinor =
      yearMinor == null || line.qtyWhole <= 0 ? null : Math.round(yearMinor / line.qtyWhole);
    const yieldBps =
      lastMinor > 0 && eachAnnualMinor != null
        ? Math.round((eachAnnualMinor * 10_000) / lastMinor)
        : null;
    const allocBps =
      allocatedMinor > 0 ? Math.round((spendMinor * 10_000) / allocatedMinor) : null;
    return {
      line,
      lastMinor,
      spendMinor,
      yearMinor,
      monthMinor,
      weekMinor,
      eachAnnualMinor,
      yieldBps,
      allocBps,
      risk: riskOf(positionMaster, line.symbol),
    };
  });
  const blendSpend = blendRows.reduce((s, r) => s + r.spendMinor, 0);
  const leftoverMinor = allocatedMinor - blendSpend;
  const leftoverYearMinor =
    leftoverMinor > 0 && collectorYieldBps != null && collectorYieldBps > 0
      ? Math.trunc((leftoverMinor * collectorYieldBps) / 10_000)
      : leftoverMinor <= 0
        ? 0
        : null;
  const leftoverParts = incomeWeekMonth(leftoverYearMinor);
  const agreed = scenario?.status === "agreed" || scenario?.status === "executing";
  const incomeRises = (ev?.netAnnualMinor ?? 0) > 0;
  const step = !scenario
    ? wizardRailStep(wizardPrompt)
    : parkedFunding
      ? "How funded"
      : !ev
        ? "Evaluate"
        : !agreed
          ? "Agree"
          : !sold
            ? "Confirm sell"
            : "Open lot";

  const nextWizard = () => {
    if (wizardPrompt === "account") {
      if (!accountId) {
        onMessage("Choose an account.");
        return;
      }
      setWizardPrompt("planName");
      return;
    }
    if (wizardPrompt === "planName") {
      setWizardPrompt("funding");
      return;
    }
    void create();
  };

  const unitHint = selectedLot
    ? selectedLot.quantityScale === 2
      ? "dollars at par for cash"
      : "shares"
    : "";

  return (
    <section aria-label="Shopping Cart" className="shopping-cart">
      <h2>Shopping Cart</h2>
      <p>
        Cash deploy is a swap. Pick named lots (no FIFO). Leftover cash still earns the
        money-market plan. Drafts do not change Holdings or Income Plan. Evaluate never
        opens a lot.
      </p>
      <StepRail current={step} />
      {!scenario ? (
        <CartStartWizard
          prompt={wizardPrompt}
          accounts={accounts}
          accountId={accountId}
          planName={draftName}
          funding={funding}
          accountCashOffered={accountCashOffered}
          busy={busy}
          writesBlocked={writesBlocked}
          onAccountId={(id) => {
            setAccountId(id);
            setLotId("");
          }}
          onPlanName={setDraftName}
          onFunding={setFunding}
          onBack={() => {
            if (wizardPrompt === "funding") setWizardPrompt("planName");
            else if (wizardPrompt === "planName") setWizardPrompt("account");
          }}
          onNext={nextWizard}
        />
      ) : parkedFunding ? (
        <section aria-label="Cart funding next step">
          <p>
            Next step depends on this funding choice (new deposit is hypothetical
            and does not post). Continues in the next slice.
          </p>
          <div className="buttons">
            <button
              type="button"
              aria-label="Discard cart draft"
              disabled={busy || writesBlocked || agreed}
              onClick={() => void discard()}
            >
              Cancel
            </button>
          </div>
        </section>
      ) : (
        <>
          <p aria-label="Cart cash yield from collector">
            {collectorYieldBps != null
              ? `Leftover cash uses the ${cashSymbol || "money-market"} collector plan (${formatBps(collectorYieldBps)}).`
              : "Cash yield unknown — no collector plan."}
          </p>
          {accountCashFlow ? (
            <AccountCashPlan
              pile={cashPile}
              ledger={cashLedger}
              buyLines={scenario.buyLines}
              agreed={agreed}
              filled={cashFilled}
              depositAmount={depositAmount}
              fills={fills}
              busy={busy}
              writesBlocked={writesBlocked}
              onDepositAmount={setDepositAmount}
              onFill={(lineId, value) => setFills((prev) => ({ ...prev, [lineId]: value }))}
              onDeposit={() => void postDeposit()}
              onConfirmFill={() => void confirmFill()}
            />
          ) : (
            <>
          <h3>Sell named lots</h3>
          <div className="form-grid">
            <LotCostTable
              lots={accountLots.map((l) => ({
                lotId: l.lotId,
                symbol: l.symbol,
                accountName: l.accountName,
                remainingQuantityMinor: l.remainingQuantityMinor,
                quantityScale: l.quantityScale,
                openedOn: l.openedOn,
                remainingPerformanceMinor: l.remainingPerformanceMinor,
                remainingTaxMinor: l.remainingTaxMinor,
              }))}
              value={lotId}
              onChange={(id) => {
                setLotId(id);
                const lot = lots.find((l) => l.lotId === id);
                if (!lot) return;
                setSellQty(
                  (lot.remainingQuantityMinor / 10 ** lot.quantityScale).toFixed(
                    lot.quantityScale,
                  ),
                );
              }}
              lastBySymbol={lastBySymbol}
              sortMode={sortMode}
              onSortModeChange={setSortMode}
              accountName={accountName}
              disabled={busy || writesBlocked || agreed}
              ariaLabel="Cart lot"
            />
            <label>
              Qty to sell {unitHint ? `(${unitHint})` : ""}
              <input
                aria-label="Cart sell qty"
                value={sellQty}
                onChange={(e) => setSellQty(e.target.value)}
                disabled={busy || writesBlocked || agreed}
              />
            </label>
          </div>
          <div className="buttons">
            <button
              type="button"
              aria-label="Add sell line"
              disabled={busy || writesBlocked || agreed || !lotId}
              onClick={() => void addSell()}
            >
              Add sell
            </button>
          </div>
          {scenario.sellLines.length > 0 ? (
            <ul aria-label="Cart sell lines">
              {scenario.sellLines.map((line) => (
                <li key={line.lineId}>
                  {line.symbol} · qty {line.qtyMinor / 10 ** line.qtyScale} · proceeds{" "}
                  {(line.proceedsMinor / 100).toFixed(2)}
                  {line.isCash
                    ? ""
                    : line.taxGainMinor == null
                      ? " · tax P/L unknown"
                      : ` · tax P/L ${(line.taxGainMinor / 100).toFixed(2)}`}
                  {line.isCash || line.performanceGainMinor == null
                    ? ""
                    : ` · perf P/L ${(line.performanceGainMinor / 100).toFixed(2)}`}
                </li>
              ))}
            </ul>
          ) : null}
            </>
          )}

          <h3>Blend</h3>
          <div className="form-grid">
            <ResearchedSymbolCombobox
              options={filteredBuys}
              query={buyQuery}
              onQueryChange={(q) => {
                setBuyQuery(q);
                setBuySecurityId("");
              }}
              open={buyOpen}
              onOpenChange={setBuyOpen}
              selectedId={buySecurityId}
              onSelect={(row) => {
                setBuySecurityId(row.securityId);
                setBuyQuery(`${row.symbol}${row.name ? ` — ${row.name}` : ""}`);
                setBuyOpen(false);
              }}
              disabled={busy || writesBlocked || agreed}
              inputAriaLabel="Cart buy symbol"
              listId="cart-buy-symbol-list"
              listAriaLabel="Cart buy symbol matches"
            />
          </div>
          <div className="buttons">
            <button
              type="button"
              aria-label="Add buy line"
              disabled={busy || writesBlocked || agreed || !buySecurityId}
              onClick={() => void addBuy()}
            >
              Add position
            </button>
          </div>
          <CartBlendTable
            allocatedMinor={allocatedMinor}
            leftoverMinor={leftoverMinor}
            leftoverWeekMinor={leftoverParts.weekMinor}
            leftoverMonthMinor={leftoverParts.monthMinor}
            leftoverYearMinor={leftoverYearMinor}
            rows={blendRows}
            overBudget={leftoverMinor < 0}
            agreed={agreed}
            busy={busy}
            onQty={(lineId, qty) => void setBuyQty(lineId, qty)}
          />

          <div className="buttons dossier-actions">
            <button
              type="button"
              aria-label="Evaluate swap"
              disabled={
                busy ||
                scenario.buyLines.length === 0 ||
                (!accountCashFlow && scenario.sellLines.length === 0)
              }
              onClick={() => void evaluate()}
            >
              Evaluate
            </button>
            <button
              type="button"
              aria-label="Save cart draft"
              className={dirty ? "is-unsaved" : undefined}
              disabled={busy || writesBlocked || !dirty}
              onClick={() => void save()}
            >
              Save
            </button>
            <button
              type="button"
              aria-label="Discard cart draft"
              disabled={busy || writesBlocked || agreed}
              onClick={() => void discard()}
            >
              Cancel
            </button>
            <button
              type="button"
              aria-label="Duplicate cart draft"
              disabled={busy || writesBlocked || !scenario || agreed}
              onClick={() => void duplicate()}
            >
              Duplicate
            </button>
            <button
              type="button"
              aria-label="Rename cart draft"
              disabled={busy || writesBlocked || !scenario || agreed}
              onClick={() => void renameDraft()}
            >
              Rename
            </button>
            <button
              type="button"
              aria-label="Agree swap"
              disabled={
                busy ||
                writesBlocked ||
                !ev ||
                ev.insufficientLotQty ||
                agreed ||
                ((ev.cashFloorWarn || worse) && !overrideReason.trim())
              }
              onClick={() => void agree()}
            >
              Agree
            </button>
          </div>
          <ComparePlans
            keepMonthlyMinor={
              keepAnnualMinor == null ? null : Math.trunc(keepAnnualMinor / 12)
            }
            keepAnnualMinor={keepAnnualMinor}
            drafts={drafts}
            activeId={scenario.scenarioId}
            onSelect={(id) => {
              const next = drafts.find((d) => d.scenarioId === id);
              if (next) applyScene(next);
            }}
          />
          <AffordStrip eval={ev} />
          <IncomeCompare eval={ev} />
          <MixBars
            current={currentMix}
            after={afterMix}
            cashCurrentMinor={cashCurrentMinor}
            cashAfterMinor={cashAfterMinor}
          />
          <TradeoffCallout
            eval={ev}
            mixWorse={worse}
            incomeRises={incomeRises}
            overrideReason={overrideReason}
            onOverrideReason={setOverrideReason}
          />
          {accountCashFlow ? (
            <div className="buttons">
              <button
                type="button"
                aria-label="Open prefilled Add Lot"
                disabled={busy || !cashFilled}
                onClick={openLot}
              >
                Add Lot (prefilled)
              </button>
            </div>
          ) : (
            <ExecutePanel
              agreed={agreed}
              sold={sold}
              busy={busy}
              writesBlocked={writesBlocked}
              onConfirmSell={() => void confirmSell()}
              onOpenLot={openLot}
            />
          )}
        </>
      )}
    </section>
  );
}
