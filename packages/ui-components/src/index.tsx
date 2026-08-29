import { useState } from "react";
import {
  ClearFiltersButton,
  colFilter,
  ListFilter,
  matchesColFilters,
  sortHead,
  sortRows,
  textMatches,
  uniqueFilterValues,
  useListSort,
  type CheckedFilters,
} from "./listTable";

export { ClearFiltersButton, ListFilter, SortTh, sortHead, sortRows, textMatches, useListSort } from "./listTable";
export type { CheckedFilters, SortDir } from "./listTable";

/** Design tokens and Radix-based primitives used by the local desktop data screens. */
export const DESIGN_SYSTEM_VERSION = "0.1.0-draft";

const USD_SCALE = 2;

function groupInt(digits: string): string {
  return digits.replace(/\B(?=(\d{3})+(?!\d))/g, ",");
}

/** Counts, lot numbers, versions — grouping only, no currency. */
export function formatCount(n: number): string {
  const sign = n < 0 ? "-" : "";
  return sign + groupInt(Math.abs(Math.trunc(n)).toString());
}

/** Fixed-scale quantity or other non-money amount (ADR-0004), with grouping. */
export function formatScaled(minor: number, scale = 0): string {
  const places = Number.isFinite(scale) ? Math.max(0, Math.trunc(scale)) : 0;
  const sign = minor < 0 ? "-" : "";
  const digits = Math.abs(Math.trunc(minor))
    .toString()
    .padStart(places + 1, "0");
  if (places <= 0) {
    return sign + groupInt(digits);
  }
  const i = digits.length - places;
  return `${sign}${groupInt(digits.slice(0, i))}.${digits.slice(i)}`;
}

function toUsdCents(minor: number, scale: number): number {
  const places = Number.isFinite(scale) ? Math.max(0, Math.trunc(scale)) : USD_SCALE;
  const n = Math.trunc(minor);
  if (places === USD_SCALE) {
    return n;
  }
  if (places > USD_SCALE) {
    const factor = 10 ** (places - USD_SCALE);
    const abs = Math.abs(n);
    const rounded = Math.trunc((abs + Math.floor(factor / 2)) / factor);
    return n < 0 ? -rounded : rounded;
  }
  return n * 10 ** (USD_SCALE - places);
}

/** Typical USD: $3,715.87. Stored scale may be 4 from unit costs; display is always cents. */
export function formatUsd(minor: number, scale = USD_SCALE): string {
  const cents = toUsdCents(minor, scale);
  const sign = cents < 0 ? "-" : "";
  return `${sign}$${formatScaled(Math.abs(cents), USD_SCALE)}`;
}

export function formatPercentScaled(minor: number, scale = USD_SCALE): string {
  const places = Number.isFinite(scale) ? scale : USD_SCALE;
  return `${formatScaled(minor, places)}%`;
}

export function formatBps(bps: number | null | undefined): string {
  if (bps == null) return "unknown";
  return `${(bps / 100).toFixed(2)}%`;
}

function moneyTotal(
  values: Array<
    | number
    | null
    | undefined
    | { minor: number | null | undefined; scale?: number }
  >,
  fallbackScale?: number,
): string {
  let cents = 0;
  let known = 0;
  let missing = 0;
  for (const value of values) {
    const minor =
      typeof value === "object" && value != null ? value.minor : value;
    const scale =
      typeof value === "object" && value != null && value.scale != null
        ? value.scale
        : (fallbackScale ?? USD_SCALE);
    if (minor == null) {
      missing += 1;
    } else {
      cents += toUsdCents(minor, scale);
      known += 1;
    }
  }
  if (known === 0) return "N/A";
  const usd = formatUsd(cents, USD_SCALE);
  return missing > 0 ? `${usd} (partial)` : usd;
}

function qtyTotal(
  rows: Array<{ remainingQuantityMinor: number; quantityScale?: number }>,
): string {
  if (rows.length === 0) return formatScaled(0, 0);
  const scale = rows.reduce(
    (max, row) => Math.max(max, row.quantityScale ?? 0),
    0,
  );
  const sum = rows.reduce((total, row) => {
    const rowScale = row.quantityScale ?? 0;
    return total + row.remainingQuantityMinor * 10 ** (scale - rowScale);
  }, 0);
  return formatScaled(sum, scale);
}

export type PositionLineView = {
  accountName: string;
  symbol: string;
  remainingQuantityMinor: number;
  remainingPerformanceMinor: number;
  remainingTaxMinor: number;
  lotCount: number;
  quantityScale?: number;
  marketValueMinor?: number | null;
  scale?: number;
};

export type AccountPositionTotalView = {
  accountId: string;
  accountName: string;
  symbolCount: number;
  openLotCount: number;
  openPerformanceMinor: number;
  openTaxMinor: number;
  marketValueMinor?: number | null;
  scale?: number;
};

export type PositionDetailsView = {
  positions: PositionLineView[];
  accountTotals?: AccountPositionTotalView[];
  symbolCount?: number;
  accountCount?: number;
  openLotCount?: number;
  openPerformanceMinor: number;
  openTaxMinor: number;
  marketValueMinor?: number | null;
  marketValueComplete?: boolean;
  scale?: number;
};

export type TaxProjectionView = {
  sourceQuery: string;
  decisionState: string;
  actualIncludedYtd: { amountMinor: number; scale?: number };
  applicableThreshold: { amountMinor: number; scale?: number };
  dataCompleteness: string;
};

export type AccountView = {
  accountId?: string;
  name: string;
  kind: string;
};

export type ExceptionView = {
  code: string;
  message: string;
  acknowledged?: boolean;
  createdAt?: string;
};

export function issuerRetrieveMissSummary(
  exceptions: ExceptionView[],
  today = new Date().toISOString().slice(0, 10),
): string | null {
  const misses = exceptions.filter(
    (e) =>
      (e.code === "declaration_retrieve_miss" || e.code === "div1_adapter_missing") &&
      !e.acknowledged &&
      (e.createdAt ?? "").startsWith(today),
  );
  if (misses.length === 0) {
    return null;
  }
  const symbols = misses
    .map((e) => {
      const idx = e.message.indexOf(":");
      if (idx > 0 && idx <= 8) {
        return e.message.slice(0, idx).trim();
      }
      return "";
    })
    .filter(Boolean);
  const listed = symbols.slice(0, 8).join(", ");
  const extra = symbols.length > 8 ? ", …" : "";
  if (listed) {
    return `${misses.length} issuer retrieves missed (${listed}${extra})`;
  }
  return `${misses.length} issuer retrieves missed`;
}

export type LotView = {
  lotId?: string;
  accountId?: string;
  openedOn?: string;
  origin?: string;
  remainingQuantityMinor: number;
  remainingPerformanceMinor: number;
  remainingTaxMinor: number;
  crfZeroCost: boolean;
  quantityScale?: number;
  scale?: number;
};

export type BasisView = {
  lots: LotView[];
  openPerformanceMinor: number;
  openTaxMinor: number;
  scale?: number;
};

export type RoiView = {
  performanceGainMinor: number;
  taxGainMinor: number;
  scale?: number;
};

export type DividendActualView = {
  occurredOn: string;
  amountMinor: number;
  scale?: number;
};

export type DividendView = {
  actuals: DividendActualView[];
  actualTotalMinor: number;
  scale?: number;
};

export type ActivityView = {
  activityId: string;
  activityType: string;
  amountMinor: number;
  occurredOn: string;
  scale?: number;
};

export type MagiView = {
  decisionState: string;
  actualIncludedYtd: { amountMinor: number; scale?: number };
  protectedHeadroom: { amountMinor: number; scale?: number };
  dataCompleteness: string;
};

export type PlanView = {
  remainingMinor: number;
  version: number;
  scale?: number;
};

export type BurndownView = {
  cashMinor: number;
  obligationMinor: number;
  sufficient: boolean;
  scale?: number;
};

export type AllocationView = {
  targets: Array<{ name: string; targetMinor: number; scale?: number }>;
  openPerformanceMinor: number;
  openTaxMinor: number;
  scale?: number;
};

function dash(value: string | null | undefined): string {
  const t = (value ?? "").trim();
  return t || "—";
}

function unknownMoney(minor: number | null | undefined, scale: number): string {
  return minor == null ? "unknown" : formatUsd(minor, scale);
}

function lineColValues(line: PositionLineView, scale: number): Record<string, string> {
  const s = line.scale ?? scale;
  return {
    account: line.accountName,
    symbol: line.symbol,
    qty: formatScaled(line.remainingQuantityMinor, line.quantityScale ?? 0),
    lots: formatCount(line.lotCount),
    perf: formatUsd(line.remainingPerformanceMinor, s),
    tax: formatUsd(line.remainingTaxMinor, s),
    mv: unknownMoney(line.marketValueMinor, s),
  };
}

function accountColValues(
  acct: AccountPositionTotalView,
  scale: number,
): Record<string, string> {
  const s = acct.scale ?? scale;
  return {
    account: acct.accountName,
    symbols: formatCount(acct.symbolCount),
    lots: formatCount(acct.openLotCount),
    cost: formatUsd(acct.openPerformanceMinor, s),
    tax: formatUsd(acct.openTaxMinor, s),
    mv: unknownMoney(acct.marketValueMinor, s),
  };
}

export function PositionDetailsTable({
  positions,
  filter = "",
}: {
  positions: PositionDetailsView | null;
  filter?: string;
}) {
  const [lineFilter, setLineFilter] = useState<CheckedFilters>({});
  const [accountFilter, setAccountFilter] = useState<CheckedFilters>({});
  const lineSort = useListSort("account");
  const accountSort = useListSort("account");
  if (!positions) {
    return <p>Loading position details…</p>;
  }
  if (positions.positions.length === 0) {
    return (
      <p>
        No open positions yet. Complete New Investment including the first lot.
      </p>
    );
  }
  const scale = positions.scale ?? 2;
  const symbolNeedle = filter.trim().toLowerCase();
  const afterSymbol = positions.positions.filter((line) => {
    return (
      !symbolNeedle ||
      line.accountName.toLowerCase().includes(symbolNeedle) ||
      line.symbol.toLowerCase().includes(symbolNeedle)
    );
  });
  const lineValueMaps = afterSymbol.map((line) => lineColValues(line, scale));
  const lineHead = (label: string, key: string, numeric = false) =>
    sortHead(
      lineSort,
      label,
      key,
      numeric,
      colFilter(
        lineFilter,
        setLineFilter,
        key,
        uniqueFilterValues(lineValueMaps, lineFilter, key),
      ),
    );
  const matched = afterSymbol.filter((_line, index) =>
    matchesColFilters(lineFilter, lineValueMaps[index] ?? {}),
  );
  const lines = sortRows(matched, lineSort.sortKey, lineSort.sortDir, (line, key) => {
    switch (key) {
      case "account":
        return line.accountName;
      case "symbol":
        return line.symbol;
      case "qty":
        return line.remainingQuantityMinor;
      case "lots":
        return line.lotCount;
      case "perf":
        return line.remainingPerformanceMinor;
      case "tax":
        return line.remainingTaxMinor;
      case "mv":
        return line.marketValueMinor ?? null;
      default:
        return line.accountName;
    }
  });
  const accountValueMaps = (positions.accountTotals ?? []).map((acct) =>
    accountColValues(acct, scale),
  );
  const accountHead = (label: string, key: string, numeric = false) =>
    sortHead(
      accountSort,
      label,
      key,
      numeric,
      colFilter(
        accountFilter,
        setAccountFilter,
        key,
        uniqueFilterValues(accountValueMaps, accountFilter, key),
      ),
    );
  const accountRows = sortRows(
    (positions.accountTotals ?? []).filter((_acct, index) =>
      matchesColFilters(accountFilter, accountValueMaps[index] ?? {}),
    ),
    accountSort.sortKey,
    accountSort.sortDir,
    (acct, key) => {
      switch (key) {
        case "account":
          return acct.accountName;
        case "symbols":
          return acct.symbolCount;
        case "lots":
          return acct.openLotCount;
        case "cost":
          return acct.openPerformanceMinor;
        case "tax":
          return acct.openTaxMinor;
        case "mv":
          return acct.marketValueMinor ?? null;
        default:
          return acct.accountName;
      }
    },
  );
  const dataMv =
    positions.marketValueMinor != null
      ? positions.marketValueComplete
        ? formatUsd(positions.marketValueMinor, scale)
        : `${formatUsd(positions.marketValueMinor, scale)} (partial)`
      : "unknown";
  return (
    <div>
      <h3>Data</h3>
      <p>These totals do not change when you choose one symbol.</p>
      <dl className="health" aria-label="Data position totals">
        <dt>open symbols</dt>
        <dd>{formatCount(positions.symbolCount ?? 0)}</dd>
        <dt>open lots</dt>
        <dd>{formatCount(positions.openLotCount ?? 0)}</dd>
        <dt>accounts</dt>
        <dd>{formatCount(positions.accountCount ?? 0)}</dd>
        <dt>original cost</dt>
        <dd>{formatUsd(positions.openPerformanceMinor, scale)}</dd>
        <dt>tax basis</dt>
        <dd>{formatUsd(positions.openTaxMinor, scale)}</dd>
        <dt>market value</dt>
        <dd>{dataMv}</dd>
      </dl>
      <h3>Per account</h3>
      <p>
        Market value is open quantity × last public price. It is not the Fidelity account total
        (cash, money-market, and broker marks stay on the statement).
      </p>
      <p className="filter-toolbar">
        Use a column heading drop-down to check values.
        <ClearFiltersButton filters={accountFilter} onClear={() => setAccountFilter({})} />
      </p>
      {(positions.accountTotals ?? []).length === 0 ? (
        <p>No account subtotals yet.</p>
      ) : (
        <div className="table-wrap">
          <table aria-label="Per-account position totals">
            <thead>
              <tr>
                {accountHead("Account", "account")}
                {accountHead("Symbols", "symbols", true)}
                {accountHead("Lots", "lots", true)}
                {accountHead("Cost", "cost", true)}
                {accountHead("Tax", "tax", true)}
                {accountHead("Market value", "mv", true)}
              </tr>
            </thead>
            <tbody>
              {accountRows.map((acct) => (
                <tr key={acct.accountId}>
                  <td>{acct.accountName}</td>
                  <td className="numeric">{formatCount(acct.symbolCount)}</td>
                  <td className="numeric">{formatCount(acct.openLotCount)}</td>
                  <td className="numeric">
                    {formatUsd(acct.openPerformanceMinor, acct.scale ?? scale)}
                  </td>
                  <td className="numeric">
                    {formatUsd(acct.openTaxMinor, acct.scale ?? scale)}
                  </td>
                  <td className="numeric">
                    {acct.marketValueMinor == null
                      ? "unknown"
                      : formatUsd(acct.marketValueMinor, acct.scale ?? scale)}
                  </td>
                </tr>
              ))}
            </tbody>
            <tfoot>
              <tr>
                <td>Total (shown subset)</td>
                <td className="numeric">
                  {formatCount(accountRows.reduce((sum, acct) => sum + acct.symbolCount, 0))}
                </td>
                <td className="numeric">
                  {formatCount(accountRows.reduce((sum, acct) => sum + acct.openLotCount, 0))}
                </td>
                <td className="numeric">
                  {moneyTotal(
                    accountRows.map((acct) => ({
                      minor: acct.openPerformanceMinor,
                      scale: acct.scale ?? scale,
                    })),
                  )}
                </td>
                <td className="numeric">
                  {moneyTotal(
                    accountRows.map((acct) => ({
                      minor: acct.openTaxMinor,
                      scale: acct.scale ?? scale,
                    })),
                  )}
                </td>
                <td className="numeric">
                  {moneyTotal(
                    accountRows.map((acct) => ({
                      minor: acct.marketValueMinor,
                      scale: acct.scale ?? scale,
                    })),
                  )}
                </td>
              </tr>
            </tfoot>
          </table>
        </div>
      )}
      <p className="filter-toolbar">
        Showing {formatCount(lines.length)} of {formatCount(positions.positions.length)} open
        position lines. Use a column heading drop-down to check values. Footer totals are the
        shown rows.
        <ClearFiltersButton filters={lineFilter} onClear={() => setLineFilter({})} />
      </p>
      <div className="table-wrap">
      <table aria-label="Open position lines">
        <thead>
          <tr>
            {lineHead("Account", "account")}
            {lineHead("Symbol", "symbol")}
            {lineHead("Qty", "qty", true)}
            {lineHead("Lots", "lots", true)}
            {lineHead("Perf basis", "perf", true)}
            {lineHead("Tax basis", "tax", true)}
            {lineHead("Market value", "mv", true)}
          </tr>
        </thead>
        <tbody>
          {lines.map((line) => (
            <tr
              key={`${line.accountName}-${line.symbol}`}
              aria-current={
                symbolNeedle && line.symbol.toLowerCase() === symbolNeedle ? "true" : undefined
              }
            >
              <td>{line.accountName}</td>
              <td>{line.symbol}</td>
              <td className="numeric">
                {formatScaled(line.remainingQuantityMinor, line.quantityScale ?? 0)}
              </td>
              <td className="numeric">{formatCount(line.lotCount)}</td>
              <td className="numeric">
                {formatUsd(line.remainingPerformanceMinor, line.scale ?? positions.scale)}
              </td>
              <td className="numeric">
                {formatUsd(line.remainingTaxMinor, line.scale ?? positions.scale)}
              </td>
              <td className="numeric">
                {line.marketValueMinor == null
                  ? "unknown"
                  : formatUsd(line.marketValueMinor, line.scale ?? positions.scale)}
              </td>
            </tr>
          ))}
        </tbody>
        <tfoot>
          <tr>
            <td>Total (shown subset)</td>
            <td>{formatCount(lines.length)}</td>
            <td className="numeric">{qtyTotal(lines)}</td>
            <td className="numeric">
              {formatCount(lines.reduce((sum, line) => sum + line.lotCount, 0))}
            </td>
            <td className="numeric">
              {moneyTotal(
                lines.map((line) => ({
                  minor: line.remainingPerformanceMinor,
                  scale: line.scale ?? positions.scale,
                })),
              )}
            </td>
              <td className="numeric">
              {moneyTotal(
                lines.map((line) => ({
                  minor: line.remainingTaxMinor,
                  scale: line.scale ?? positions.scale,
                })),
              )}
              </td>
              <td className="numeric">
              {moneyTotal(
                lines.map((line) => ({
                  minor: line.marketValueMinor ?? null,
                  scale: line.scale ?? positions.scale,
                })),
              )}
              </td>
          </tr>
        </tfoot>
      </table>
      </div>
    </div>
  );
}

export type PositionMasterRowView = {
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
  rocPct2025ActualMinor: number | null;
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
    dataConfidence: number;
  } | null;
  bearCushionBps?: number | null;
  bullPriceReturnBps?: number | null;
  bullCushionBps?: number | null;
  carMarketValueMinor?: number | null;
  carShareOfSymbolBps?: number | null;
  carShareOfDataBps?: number | null;
  rocResearchStatus?: string;
  declarationFreshness?: string;
};

function unknownOrBps(value: number | null | undefined): string {
  return value == null ? "unknown" : formatBps(value);
}

function masterColValues(row: PositionMasterRowView): Record<string, string> {
  return {
    symbol: row.symbol,
    active: row.isActive ? "yes" : "no",
    risk: dash(row.riskTier),
    provider: dash(row.provider),
    underlying: dash(row.underlying),
    freq: dash(row.paymentFrequency),
    div: dash(row.divType),
    rocNeed: row.needsRocResearch ? "yes" : "no",
    rocStatus: dash(row.rocResearchStatus),
    declFresh: dash(row.declarationFreshness),
    qty: formatScaled(row.remainingQuantityMinor, row.quantityScale),
    unit: row.unitCostMinor == null ? "unknown" : formatUsd(row.unitCostMinor, 2),
    cost: formatUsd(row.remainingPerformanceMinor, row.scale),
    tax: formatUsd(row.remainingTaxMinor, row.scale),
    price:
      row.lastPriceMinor == null
        ? "unknown"
        : `${formatUsd(row.lastPriceMinor, row.lastPriceScale ?? 2)} (${row.priceFreshness})`,
    mv: unknownMoney(row.marketValueMinor, row.scale),
    alloc: formatBps(row.allocationBps),
    carMv: unknownMoney(row.carMarketValueMinor ?? null, row.scale),
    carSym: unknownOrBps(row.carShareOfSymbolBps ?? null),
    carData: unknownOrBps(row.carShareOfDataBps ?? null),
    plan: row.planKnown ? `$${formatScaled(row.planPerShareMinor, row.planScale)}` : "N/A",
    annual: unknownMoney(row.annualPlanMinor, row.scale),
    yoc: formatBps(row.planYocBps),
    planFwd: formatBps(row.planFwdYieldBps),
    mcFwd: formatBps(row.mostCurrentFwdYieldBps),
    pnl: formatBps(row.unrealizedPnlBps),
    dist:
      row.distributionsScope === "incomplete" || row.totalDistributionsReceivedMinor == null
        ? "unknown"
        : formatUsd(row.totalDistributionsReceivedMinor, row.scale),
    rocComp:
      row.distributionsScope === "incomplete" || row.rocDistributionsMinor == null
        ? "unknown"
        : formatUsd(row.rocDistributionsMinor, row.scale),
    costRec: unknownOrBps(row.costRecoveryBps ?? null),
    roc:
      row.rocPct2025ActualMinor == null || row.rocScale == null
        ? "N/A"
        : formatPercentScaled(row.rocPct2025ActualMinor, row.rocScale),
    incomeRel: unknownOrBps(row.evidence?.incomeReliability ?? null),
    downside: unknownOrBps(row.evidence?.downsideResilience ?? null),
    recovery: unknownOrBps(row.evidence?.recoveryUpside ?? null),
    nav: unknownOrBps(row.evidence?.navPersistence ?? null),
    dataConf: row.evidence == null ? "unknown" : formatCount(row.evidence.dataConfidence),
    bearPrice: row.periodDated ? unknownOrBps(row.bearPriceReturnBps) : "unknown",
    bearCushion: row.periodDated ? unknownOrBps(row.bearCushionBps ?? null) : "unknown",
    bear: row.periodDated ? formatBps(row.bearTotalReturnBps) : "unknown",
    bullPrice: row.periodDated ? unknownOrBps(row.bullPriceReturnBps ?? null) : "unknown",
    bullCushion: row.periodDated ? unknownOrBps(row.bullCushionBps ?? null) : "unknown",
    bull: row.periodDated ? formatBps(row.bullTotalReturnBps) : "unknown",
    complete: row.completeness,
    notes: dash(row.notes),
  };
}

export function PositionMasterTable({
  rows,
  selectedSymbol = "",
  onOpenSymbol,
}: {
  rows: PositionMasterRowView[] | null;
  selectedSymbol?: string;
  onOpenSymbol: (symbol: string) => void;
}) {
  const [colFilters, setColFilters] = useState<CheckedFilters>({});
  const sort = useListSort("symbol");
  if (!rows) {
    return <p>Loading position master…</p>;
  }
  if (rows.length === 0) {
    return <p>No positions yet. Complete New Investment including the first lot.</p>;
  }
  const valueMaps = rows.map(masterColValues);
  const f = (key: string) =>
    colFilter(colFilters, setColFilters, key, uniqueFilterValues(valueMaps, colFilters, key));
  const matched = rows.filter((_, index) =>
    matchesColFilters(colFilters, valueMaps[index] ?? {}),
  );
  const shown = sortRows(matched, sort.sortKey, sort.sortDir, (row, key) => {
    switch (key) {
      case "symbol":
        return row.symbol;
      case "risk":
        return row.riskTier;
      case "provider":
        return row.provider;
      case "underlying":
        return row.underlying;
      case "freq":
        return row.paymentFrequency;
      case "div":
        return row.divType;
      case "rocNeed":
        return row.needsRocResearch ? 1 : 0;
      case "rocStatus":
        return row.rocResearchStatus ?? "";
      case "declFresh":
        return row.declarationFreshness ?? "";
      case "active":
        return row.isActive ? 1 : 0;
      case "notes":
        return row.notes;
      case "qty":
        return row.remainingQuantityMinor;
      case "unit":
        return row.unitCostMinor;
      case "cost":
        return row.remainingPerformanceMinor;
      case "tax":
        return row.remainingTaxMinor;
      case "price":
        return row.lastPriceMinor;
      case "mv":
        return row.marketValueMinor;
      case "alloc":
        return row.allocationBps;
      case "carMv":
        return row.carMarketValueMinor ?? null;
      case "carSym":
        return row.carShareOfSymbolBps ?? null;
      case "carData":
        return row.carShareOfDataBps ?? null;
      case "plan":
        return row.planKnown ? row.planPerShareMinor : null;
      case "annual":
        return row.annualPlanMinor;
      case "yoc":
        return row.planYocBps;
      case "planFwd":
        return row.planFwdYieldBps;
      case "mcFwd":
        return row.mostCurrentFwdYieldBps;
      case "pnl":
        return row.unrealizedPnlBps;
      case "dist":
        return row.totalDistributionsReceivedMinor ?? null;
      case "rocComp":
        return row.rocDistributionsMinor ?? null;
      case "costRec":
        return row.costRecoveryBps ?? null;
      case "roc":
        return row.rocPct2025ActualMinor;
      case "incomeRel":
        return row.evidence?.incomeReliability ?? null;
      case "downside":
        return row.evidence?.downsideResilience ?? null;
      case "recovery":
        return row.evidence?.recoveryUpside ?? null;
      case "nav":
        return row.evidence?.navPersistence ?? null;
      case "dataConf":
        return row.evidence?.dataConfidence ?? null;
      case "bearPrice":
        return row.bearPriceReturnBps;
      case "bearCushion":
        return row.bearCushionBps ?? null;
      case "bear":
        return row.bearTotalReturnBps;
      case "bullPrice":
        return row.bullPriceReturnBps ?? null;
      case "bullCushion":
        return row.bullCushionBps ?? null;
      case "bull":
        return row.bullTotalReturnBps;
      default:
        return row.symbol;
    }
  });
  const scale = shown[0]?.scale ?? 2;
  return (
    <div>
      <h3>All positions</h3>
      <p>
        One row per symbol. Quantity, cost, price, and yields are joined views, not
        stored on the position. Click a symbol to open the dossier. Use a column heading
        drop-down to check values. Footer totals are the shown subset.
      </p>
      <p className="filter-toolbar">
        Showing {formatCount(shown.length)} of {formatCount(rows.length)} symbols.
        <ClearFiltersButton filters={colFilters} onClear={() => setColFilters({})} />
      </p>
      <div className="table-wrap">
        <table aria-label="Position master">
          <thead>
            <tr>
              {sortHead(sort, "Symbol", "symbol", false, f("symbol"))}
              {sortHead(sort, "Active", "active", false, f("active"))}
              {sortHead(sort, "Risk", "risk", false, f("risk"))}
              {sortHead(sort, "Provider", "provider", false, f("provider"))}
              {sortHead(sort, "Underlying", "underlying", false, f("underlying"))}
              {sortHead(sort, "Frequency", "freq", false, f("freq"))}
              {sortHead(sort, "Type", "div", false, f("div"))}
              {sortHead(sort, "ROC research", "rocNeed", false, f("rocNeed"))}
              {sortHead(sort, "ROC status", "rocStatus", false, f("rocStatus"))}
              {sortHead(sort, "Decl freshness", "declFresh", false, f("declFresh"))}
              {sortHead(sort, "Qty", "qty", true, f("qty"))}
              {sortHead(sort, "Avg cost", "unit", true, f("unit"))}
              {sortHead(sort, "Original cost", "cost", true, f("cost"))}
              {sortHead(sort, "Tax", "tax", true, f("tax"))}
              {sortHead(sort, "Last price", "price", true, f("price"))}
              {sortHead(sort, "Market value", "mv", true, f("mv"))}
              {sortHead(sort, "Alloc %", "alloc", true, f("alloc"))}
              {sortHead(sort, "Car MV", "carMv", true, f("carMv"))}
              {sortHead(sort, "Car % symbol", "carSym", true, f("carSym"))}
              {sortHead(sort, "Car % data", "carData", true, f("carData"))}
              {sortHead(sort, "Plan / share", "plan", true, f("plan"))}
              {sortHead(sort, "Annual Plan", "annual", true, f("annual"))}
              {sortHead(sort, "Plan YOC", "yoc", true, f("yoc"))}
              {sortHead(sort, "Plan FWD", "planFwd", true, f("planFwd"))}
              {sortHead(sort, "Most Current FWD", "mcFwd", true, f("mcFwd"))}
              {sortHead(sort, "Unrealized %", "pnl", true, f("pnl"))}
              {sortHead(sort, "Total dist", "dist", true, f("dist"))}
              {sortHead(sort, "ROC component", "rocComp", true, f("rocComp"))}
              {sortHead(sort, "Cost recovery", "costRec", true, f("costRec"))}
              {sortHead(sort, "ROC 2025", "roc", true, f("roc"))}
              {sortHead(sort, "Income reliability", "incomeRel", true, f("incomeRel"))}
              {sortHead(sort, "Downside", "downside", true, f("downside"))}
              {sortHead(sort, "Recovery", "recovery", true, f("recovery"))}
              {sortHead(sort, "NAV persistence", "nav", true, f("nav"))}
              {sortHead(sort, "Data confidence", "dataConf", true, f("dataConf"))}
              {sortHead(sort, "Bear price", "bearPrice", true, f("bearPrice"))}
              {sortHead(sort, "Bear cushion", "bearCushion", true, f("bearCushion"))}
              {sortHead(sort, "Bear total", "bear", true, f("bear"))}
              {sortHead(sort, "Bull price", "bullPrice", true, f("bullPrice"))}
              {sortHead(sort, "Bull cushion", "bullCushion", true, f("bullCushion"))}
              {sortHead(sort, "Bull total", "bull", true, f("bull"))}
              {sortHead(sort, "Complete", "complete", false, f("complete"))}
              {sortHead(sort, "Notes", "notes", false, f("notes"))}
            </tr>
          </thead>
          <tbody>
            {shown.map((row) => (
              <tr key={row.securityId}>
                <td>
                  <button
                    type="button"
                    aria-label={`Open ${row.symbol} dossier`}
                    aria-current={
                      selectedSymbol.toUpperCase() === row.symbol.toUpperCase()
                        ? "true"
                        : undefined
                    }
                    onClick={() => onOpenSymbol(row.symbol)}
                  >
                    {row.symbol}
                  </button>
                </td>
                <td>{row.isActive ? "yes" : "no"}</td>
                <td>{row.riskTier || "—"}</td>
                <td>{row.provider || "—"}</td>
                <td>{row.underlying || "—"}</td>
                <td>{row.paymentFrequency || "—"}</td>
                <td>{row.divType || "—"}</td>
                <td>{row.needsRocResearch ? "yes" : "no"}</td>
                <td>{row.rocResearchStatus || "—"}</td>
                <td>{row.declarationFreshness || "—"}</td>
                <td className="numeric">
                  {formatScaled(row.remainingQuantityMinor, row.quantityScale)}
                </td>
                <td className="numeric">
                  {row.unitCostMinor == null ? "unknown" : formatUsd(row.unitCostMinor, 2)}
                </td>
                <td className="numeric">
                  {formatUsd(row.remainingPerformanceMinor, row.scale)}
                </td>
                <td className="numeric">{formatUsd(row.remainingTaxMinor, row.scale)}</td>
                <td className="numeric">
                  {row.lastPriceMinor == null
                    ? "unknown"
                    : `${formatUsd(row.lastPriceMinor, row.lastPriceScale ?? 2)} (${row.priceFreshness})`}
                </td>
                <td className="numeric">
                  {row.marketValueMinor == null
                    ? "unknown"
                    : formatUsd(row.marketValueMinor, row.scale)}
                </td>
                <td className="numeric">{formatBps(row.allocationBps)}</td>
                <td className="numeric">
                  {unknownMoney(row.carMarketValueMinor ?? null, row.scale)}
                </td>
                <td className="numeric">{unknownOrBps(row.carShareOfSymbolBps ?? null)}</td>
                <td className="numeric">{unknownOrBps(row.carShareOfDataBps ?? null)}</td>
                <td className="numeric">
                  {row.planKnown
                    ? `$${formatScaled(row.planPerShareMinor, row.planScale)}`
                    : "N/A"}
                </td>
                <td className="numeric">
                  {row.annualPlanMinor == null
                    ? "unknown"
                    : formatUsd(row.annualPlanMinor, row.scale)}
                </td>
                <td className="numeric">{formatBps(row.planYocBps)}</td>
                <td className="numeric">{formatBps(row.planFwdYieldBps)}</td>
                <td className="numeric">{formatBps(row.mostCurrentFwdYieldBps)}</td>
                <td className="numeric">{formatBps(row.unrealizedPnlBps)}</td>
                <td className="numeric">
                  {row.distributionsScope === "incomplete" ||
                  row.totalDistributionsReceivedMinor == null
                    ? "unknown"
                    : formatUsd(row.totalDistributionsReceivedMinor, row.scale)}
                </td>
                <td className="numeric">
                  {row.distributionsScope === "incomplete" || row.rocDistributionsMinor == null
                    ? "unknown"
                    : formatUsd(row.rocDistributionsMinor, row.scale)}
                </td>
                <td className="numeric">{unknownOrBps(row.costRecoveryBps ?? null)}</td>
                <td className="numeric">
                  {row.rocPct2025ActualMinor == null || row.rocScale == null
                    ? "N/A"
                    : formatPercentScaled(row.rocPct2025ActualMinor, row.rocScale)}
                </td>
                <td className="numeric">
                  {unknownOrBps(row.evidence?.incomeReliability ?? null)}
                </td>
                <td className="numeric">
                  {unknownOrBps(row.evidence?.downsideResilience ?? null)}
                </td>
                <td className="numeric">{unknownOrBps(row.evidence?.recoveryUpside ?? null)}</td>
                <td className="numeric">{unknownOrBps(row.evidence?.navPersistence ?? null)}</td>
                <td className="numeric">
                  {row.evidence == null ? "unknown" : formatCount(row.evidence.dataConfidence)}
                </td>
                <td className="numeric">
                  {row.periodDated ? unknownOrBps(row.bearPriceReturnBps) : "unknown"}
                </td>
                <td className="numeric">
                  {row.periodDated ? unknownOrBps(row.bearCushionBps ?? null) : "unknown"}
                </td>
                <td className="numeric">
                  {row.periodDated ? formatBps(row.bearTotalReturnBps) : "unknown"}
                </td>
                <td className="numeric">
                  {row.periodDated ? unknownOrBps(row.bullPriceReturnBps ?? null) : "unknown"}
                </td>
                <td className="numeric">
                  {row.periodDated ? unknownOrBps(row.bullCushionBps ?? null) : "unknown"}
                </td>
                <td className="numeric">
                  {row.periodDated ? formatBps(row.bullTotalReturnBps) : "unknown"}
                </td>
                <td>{row.completeness}</td>
                <td>{row.notes || "—"}</td>
              </tr>
            ))}
          </tbody>
          <tfoot>
            <tr>
              <td>Total (shown subset)</td>
              <td>{formatCount(shown.length)}</td>
              <td colSpan={8}>—</td>
              <td className="numeric">{qtyTotal(shown)}</td>
              <td className="numeric">—</td>
              <td className="numeric">
                {moneyTotal(
                  shown.map((row) => ({
                    minor: row.remainingPerformanceMinor,
                    scale: row.scale,
                  })),
                  scale,
                )}
              </td>
              <td className="numeric">
                {moneyTotal(
                  shown.map((row) => ({
                    minor: row.remainingTaxMinor,
                    scale: row.scale,
                  })),
                  scale,
                )}
              </td>
              <td className="numeric">—</td>
              <td className="numeric">
                {moneyTotal(
                  shown.map((row) => ({
                    minor: row.marketValueMinor,
                    scale: row.scale,
                  })),
                  scale,
                )}
              </td>
              <td colSpan={27}>—</td>
            </tr>
          </tfoot>
        </table>
      </div>
    </div>
  );
}

export function TaxProjectionCard({ tax }: { tax: TaxProjectionView | null }) {
  if (!tax) {
    return <p>Loading tax projection…</p>;
  }
  return (
    <dl className="health">
      <dt>decision</dt>
      <dd>{tax.decisionState}</dd>
      <dt>source</dt>
      <dd>{tax.sourceQuery}</dd>
      <dt>actual YTD</dt>
      <dd>{formatUsd(tax.actualIncludedYtd.amountMinor, tax.actualIncludedYtd.scale)}</dd>
      <dt>threshold</dt>
      <dd>{formatUsd(tax.applicableThreshold.amountMinor, tax.applicableThreshold.scale)}</dd>
      <dt>completeness</dt>
      <dd>{tax.dataCompleteness}</dd>
    </dl>
  );
}

export function AccountList({ accounts }: { accounts: AccountView[] }) {
  if (accounts.length === 0) {
    return <p>None yet</p>;
  }
  return (
    <ul>
      {accounts.map((a) => (
        <li key={a.accountId ?? a.name}>
          {a.name} ({a.kind})
        </li>
      ))}
    </ul>
  );
}

export function ExceptionList({
  exceptions,
  onOpenLog,
}: {
  exceptions: ExceptionView[];
  onOpenLog?: () => void;
}) {
  const open = exceptions.filter((e) => !e.acknowledged);
  const summary = issuerRetrieveMissSummary(exceptions);
  if (open.length === 0 && exceptions.length === 0) {
    return <p aria-label="Exception summary">No open exceptions.</p>;
  }
  const countLine =
    open.length === 0
      ? `${exceptions.length} acknowledged exception${exceptions.length === 1 ? "" : "s"}`
      : `${open.length} open exception${open.length === 1 ? "" : "s"}`;
  return (
    <div aria-label="Exception summary">
      {summary ? <p aria-label="Issuer retrieve miss summary">{summary}</p> : null}
      <p>
        {countLine}
        {onOpenLog ? (
          <>
            {" "}
            <button type="button" aria-label="Open exception log" onClick={onOpenLog}>
              Open exception log
            </button>
          </>
        ) : null}
      </p>
    </div>
  );
}

export function LotsRoiPanel({
  basis,
  roi,
  filter = "",
}: {
  basis: BasisView | null;
  roi: RoiView | null;
  filter?: string;
}) {
  if (!basis) {
    return <p>Loading lots…</p>;
  }
  const openLots = basis.lots.filter((lot) => lot.remainingQuantityMinor > 0);
  const needle = filter.trim().toLowerCase();
  const shown = needle
    ? openLots.filter((lot) =>
        [lot.lotId, lot.accountId, lot.origin, lot.openedOn]
          .filter(Boolean)
          .join(" ")
          .toLowerCase()
          .includes(needle),
      )
    : openLots;
  return (
    <div>
      <dl className="health">
        <dt>open lots</dt>
        <dd>{formatCount(openLots.length)}</dd>
        <dt>shown</dt>
        <dd>{formatCount(shown.length)}</dd>
        <dt>performance</dt>
        <dd>{formatUsd(basis.openPerformanceMinor, basis.scale)}</dd>
        <dt>tax</dt>
        <dd>{formatUsd(basis.openTaxMinor, basis.scale)}</dd>
        <dt>perf gain</dt>
        <dd>{roi ? formatUsd(roi.performanceGainMinor, roi.scale ?? basis.scale) : "—"}</dd>
        <dt>tax gain</dt>
        <dd>{roi ? formatUsd(roi.taxGainMinor, roi.scale ?? basis.scale) : "—"}</dd>
      </dl>
      {openLots.length === 0 ? (
        <p>No open lots.</p>
      ) : (
        <div className="table-wrap">
        <table>
          <thead>
            <tr>
              <th scope="col">Lot</th>
              <th scope="col">Opened</th>
              <th scope="col">Origin</th>
              <th className="numeric" scope="col">Qty remaining</th>
              <th className="numeric" scope="col">Perf basis</th>
              <th className="numeric" scope="col">Tax basis</th>
              <th scope="col">CRF zero-cost</th>
            </tr>
          </thead>
          <tbody>
            {shown.map((lot, i) => (
              <tr key={lot.lotId ?? `${lot.remainingQuantityMinor}-${i}`}>
                <td>{lot.lotId ?? "—"}</td>
                <td>{lot.openedOn ?? "—"}</td>
                <td>{lot.origin ?? "—"}</td>
                <td className="numeric">
                  {formatScaled(lot.remainingQuantityMinor, lot.quantityScale ?? 0)}
                </td>
                <td className="numeric">
                  {formatUsd(lot.remainingPerformanceMinor, lot.scale ?? basis.scale)}
                </td>
                <td className="numeric">
                  {formatUsd(lot.remainingTaxMinor, lot.scale ?? basis.scale)}
                </td>
                <td>{lot.crfZeroCost ? "yes" : "no"}</td>
              </tr>
            ))}
          </tbody>
        </table>
        </div>
      )}
    </div>
  );
}

export function YieldPanel({
  dividend,
  activities,
  filter = "",
}: {
  dividend: DividendView | null;
  activities: ActivityView[];
  filter?: string;
}) {
  const yieldActs = activities.filter(
    (a) => a.activityType.toLowerCase() === "dividend",
  );
  const needle = filter.trim().toLowerCase();
  const shown = needle
    ? yieldActs.filter((row) => {
        const usd = formatUsd(row.amountMinor, row.scale ?? dividend?.scale);
        return (
          row.occurredOn.toLowerCase().includes(needle) ||
          String(row.amountMinor).includes(needle) ||
          usd.toLowerCase().includes(needle)
        );
      })
    : yieldActs;
  return (
    <div>
      <dl className="health">
        <dt>DividendGet</dt>
        <dd>
          {dividend ? formatUsd(dividend.actualTotalMinor, dividend.scale) : "loading…"}
        </dd>
        <dt>posted rows</dt>
        <dd>{formatCount(yieldActs.length)}</dd>
        <dt>shown</dt>
        <dd>{formatCount(shown.length)}</dd>
      </dl>
      {yieldActs.length === 0 ? (
        <p>No yield activity yet.</p>
      ) : (
        <div className="table-wrap">
        <table>
          <thead>
            <tr>
              <th scope="col">Date</th>
              <th className="numeric" scope="col">Amount</th>
              <th scope="col">Activity</th>
            </tr>
          </thead>
          <tbody>
            {shown.map((row) => (
              <tr key={row.activityId}>
                <td>{row.occurredOn}</td>
                <td className="numeric">
                  {formatUsd(row.amountMinor, row.scale ?? dividend?.scale)}
                </td>
                <td>{row.activityId}</td>
              </tr>
            ))}
          </tbody>
        </table>
        </div>
      )}
    </div>
  );
}

const DISBURSEMENT_TYPES = new Set([
  "IRA_Distribution",
  "Withdrawal",
  "Form_1099",
  "SSA",
  "Roth_Distribution",
]);

export function DisbursementPanel({
  activities,
  filter = "",
}: {
  activities: ActivityView[];
  filter?: string;
}) {
  const rows = activities.filter((a) => DISBURSEMENT_TYPES.has(a.activityType));
  const needle = filter.trim().toLowerCase();
  const shown = needle
    ? rows.filter((row) => {
        const usd = formatUsd(row.amountMinor, row.scale);
        return (
          row.activityType.toLowerCase().includes(needle) ||
          row.occurredOn.toLowerCase().includes(needle) ||
          String(row.amountMinor).includes(needle) ||
          usd.toLowerCase().includes(needle)
        );
      })
    : rows;
  const gross = rows.reduce((sum, row) => sum + row.amountMinor, 0);
  const grossScale = rows[0]?.scale ?? USD_SCALE;
  return (
    <div>
      <dl className="health">
        <dt>posted</dt>
        <dd>{formatCount(rows.length)}</dd>
        <dt>shown</dt>
        <dd>{formatCount(shown.length)}</dd>
        <dt>gross</dt>
        <dd>{formatUsd(gross, grossScale)}</dd>
      </dl>
      {rows.length === 0 ? (
        <p>No disbursements yet.</p>
      ) : (
        <div className="table-wrap">
        <table>
          <thead>
            <tr>
              <th scope="col">Date</th>
              <th scope="col">Type</th>
              <th className="numeric" scope="col">Amount</th>
            </tr>
          </thead>
          <tbody>
            {shown.map((row) => (
              <tr key={row.activityId}>
                <td>{row.occurredOn}</td>
                <td>{row.activityType}</td>
                <td className="numeric">{formatUsd(row.amountMinor, row.scale)}</td>
              </tr>
            ))}
          </tbody>
        </table>
        </div>
      )}
    </div>
  );
}

export function MagiCard({ magi }: { magi: MagiView | null }) {
  if (!magi) {
    return <p>MAGI not set. Load MAGI facts through FinanceClient.</p>;
  }
  return (
    <dl className="health">
      <dt>decision</dt>
      <dd>{magi.decisionState}</dd>
      <dt>actual included</dt>
      <dd>{formatUsd(magi.actualIncludedYtd.amountMinor, magi.actualIncludedYtd.scale)}</dd>
      <dt>protected headroom</dt>
      <dd>{formatUsd(magi.protectedHeadroom.amountMinor, magi.protectedHeadroom.scale)}</dd>
      <dt>completeness</dt>
      <dd>{magi.dataCompleteness}</dd>
    </dl>
  );
}

export function PlanBurndownCard({
  plan,
  burndown,
}: {
  plan: PlanView | null;
  burndown: BurndownView | null;
}) {
  return (
    <dl className="health">
      <dt>PlanGet remaining</dt>
      <dd>{plan ? formatUsd(plan.remainingMinor, plan.scale) : "—"}</dd>
      <dt>version</dt>
      <dd>{plan ? formatCount(plan.version) : "—"}</dd>
      <dt>cash</dt>
      <dd>{burndown ? formatUsd(burndown.cashMinor, burndown.scale) : "—"}</dd>
      <dt>obligation</dt>
      <dd>{burndown ? formatUsd(burndown.obligationMinor, burndown.scale) : "—"}</dd>
      <dt>sufficient</dt>
      <dd>{burndown ? (burndown.sufficient ? "yes" : "no") : "—"}</dd>
    </dl>
  );
}

export function AllocationVsPositions({
  allocation,
  positions,
}: {
  allocation: AllocationView | null;
  positions: PositionDetailsView | null;
}) {
  if (!allocation || !positions) {
    return <p>Loading allocation versus positions…</p>;
  }
  const openPerf = allocation.openPerformanceMinor;
  return (
    <div>
      <p>
        Targets are decision support. Open amounts are lot cost basis, not market
        value. They do not post cash or MAGI facts.
      </p>
      <dl className="health">
        <dt>targets</dt>
        <dd>{formatCount(allocation.targets.length)}</dd>
        <dt>open performance (basis)</dt>
        <dd>{formatUsd(allocation.openPerformanceMinor, allocation.scale)}</dd>
        <dt>open tax (basis)</dt>
        <dd>{formatUsd(allocation.openTaxMinor, allocation.scale)}</dd>
      </dl>
      {allocation.targets.length > 0 ? (
        <ul>
          {allocation.targets.map((t) => (
            <li key={t.name}>
              {t.name}: {formatPercentScaled(t.targetMinor, t.scale ?? allocation.scale)}
            </li>
          ))}
        </ul>
      ) : (
        <p>No targets yet. Set a data target below.</p>
      )}
      {positions.positions.length === 0 ? (
        <p>No open positions to compare.</p>
      ) : (
        <table>
          <thead>
            <tr>
              <th scope="col">Account</th>
              <th scope="col">Symbol</th>
              <th className="numeric" scope="col">Perf basis</th>
              <th className="numeric" scope="col">Share of open</th>
            </tr>
          </thead>
          <tbody>
            {positions.positions.map((line) => {
              const share =
                openPerf === 0
                  ? 0
                  : (line.remainingPerformanceMinor * 10000) / openPerf;
              return (
                <tr key={`${line.accountName}-${line.symbol}`}>
                  <td>{line.accountName}</td>
                  <td>{line.symbol}</td>
                  <td className="numeric">
                    {formatUsd(line.remainingPerformanceMinor, line.scale ?? positions.scale)}
                  </td>
                  <td className="numeric">
                    {formatPercentScaled(Math.round(share), 2)}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      )}
    </div>
  );
}

export type IncomePlanWeekView = {
  asOfDate: string;
  start: string;
  end: string;
  status: string;
  lines: Array<{
    accountName: string;
    actualMinor: number;
    plannedMinor?: number;
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
  latestActualOn?: string | null;
  yieldCount?: number;
  scale?: number;
};

export function IncomePlanWeekPanel({
  week,
  selectedAccount,
  onOpenSymbol,
}: {
  week: IncomePlanWeekView | null;
  selectedAccount?: string | null;
  onOpenSymbol?: (symbol: string) => void;
}) {
  const [drillFilter, setDrillFilter] = useState("");
  const weekSort = useListSort("account");
  const drillSort = useListSort("date");
  if (!week) {
    return <p>Loading week…</p>;
  }
  const weekLines = sortRows(week.lines, weekSort.sortKey, weekSort.sortDir, (line, key) => {
    switch (key) {
      case "account":
        return line.accountName;
      case "plan":
        return line.planKnown ? (line.plannedMinor ?? 0) : null;
      case "actual":
        return line.actualMinor;
      case "variance":
        return line.planKnown ? line.actualMinor - (line.plannedMinor ?? 0) : null;
      default:
        return line.accountName;
    }
  });
  const totalActual = week.lines.reduce((sum, line) => sum + line.actualMinor, 0);
  const scoped = selectedAccount
    ? week.drilldown.filter((row) => row.accountName === selectedAccount)
    : week.drilldown;
  const matched = scoped.filter((row) =>
    textMatches(
      drillFilter,
      row.accountName,
      row.symbol,
      row.occurredOn,
      formatUsd(row.amountMinor, row.scale),
    ),
  );
  const drill = sortRows(matched, drillSort.sortKey, drillSort.sortDir, (row, key) => {
    switch (key) {
      case "account":
        return row.accountName;
      case "symbol":
        return row.symbol;
      case "date":
        return row.occurredOn;
      case "amount":
        return row.amountMinor;
      default:
        return row.occurredOn;
    }
  });
  return (
    <div>
      <p>
        Saturday {week.start} through Friday {week.end}. Status {week.status}. Plan is
        unknown until Calculator exists — not shown as $0.00.
        {week.yieldCount != null ? ` Data yield rows: ${formatCount(week.yieldCount)}.` : ""}
        {week.latestActualOn ? ` Last yield ${week.latestActualOn}.` : ""}
      </p>
      {totalActual === 0 ? (
        <p>
          No dividend cash in this week. That is not unpaid and not an empty database — use
          previous week if the last yield is earlier.
        </p>
      ) : null}
      <div className="table-wrap">
        <table aria-label="Income plan week">
          <thead>
            <tr>
              {sortHead(weekSort, "Account", "account")}
              {sortHead(weekSort, "Plan", "plan", true)}
              {sortHead(weekSort, "Actual", "actual", true)}
              {sortHead(weekSort, "Variance", "variance", true)}
            </tr>
          </thead>
          <tbody>
            {weekLines.map((line) => (
              <tr key={line.accountName}>
                <td>{line.accountName}</td>
                <td className="numeric">
                  {line.planKnown
                    ? formatUsd(line.plannedMinor ?? 0, line.scale)
                    : "N/A"}
                </td>
                <td className="numeric">{formatUsd(line.actualMinor, line.scale)}</td>
                <td className="numeric">
                  {line.planKnown
                    ? formatUsd(line.actualMinor - (line.plannedMinor ?? 0), line.scale)
                    : "N/A"}
                </td>
              </tr>
            ))}
          </tbody>
          <tfoot>
            <tr>
              <td>Total</td>
              <td className="numeric">
                {moneyTotal(
                  week.lines.map((line) => (line.planKnown ? (line.plannedMinor ?? 0) : null)),
                  week.scale,
                )}
              </td>
              <td className="numeric">{formatUsd(totalActual, week.scale)}</td>
              <td className="numeric">
                {moneyTotal(
                  week.lines.map((line) =>
                    line.planKnown ? line.actualMinor - (line.plannedMinor ?? 0) : null,
                  ),
                  week.scale,
                )}
              </td>
            </tr>
          </tfoot>
        </table>
      </div>
      <h3>Symbol drilldown{selectedAccount ? ` — ${selectedAccount}` : ""}</h3>
      <ListFilter
        label="Filter income plan drilldown"
        value={drillFilter}
        onChange={setDrillFilter}
      />
      {scoped.length === 0 ? (
        <p>No dividend actuals in this week for the selected scope.</p>
      ) : (
        <div className="table-wrap">
          <p>
            Showing {formatCount(drill.length)} of {formatCount(scoped.length)} yield rows.
            Click a column heading to sort. Footer totals are the shown rows.
          </p>
          <table aria-label="Income plan drilldown">
            <thead>
              <tr>
                {sortHead(drillSort, "Account", "account")}
                {sortHead(drillSort, "Symbol", "symbol")}
                {sortHead(drillSort, "Date", "date")}
                {sortHead(drillSort, "Amount", "amount", true)}
              </tr>
            </thead>
            <tbody>
              {drill.map((row, i) => (
                <tr key={`${row.accountName}-${row.symbol}-${row.occurredOn}-${i}`}>
                  <td>{row.accountName}</td>
                  <td>
                    {onOpenSymbol ? (
                      <button
                        type="button"
                        aria-label={`Open ${row.symbol} position`}
                        onClick={() => onOpenSymbol(row.symbol)}
                      >
                        {row.symbol}
                      </button>
                    ) : (
                      row.symbol
                    )}
                  </td>
                  <td>{row.occurredOn}</td>
                  <td className="numeric">{formatUsd(row.amountMinor, row.scale)}</td>
                </tr>
              ))}
            </tbody>
            <tfoot>
              <tr>
                <td>Total</td>
                <td>{formatCount(drill.length)}</td>
                <td></td>
                <td className="numeric">
                  {moneyTotal(
                    drill.map((row) => row.amountMinor),
                    week.scale,
                  )}
                </td>
              </tr>
            </tfoot>
          </table>
        </div>
      )}
    </div>
  );
}

export type DashboardBurndownView = {
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
  scale?: number;
};

export function DashboardBurndownPanel({
  burndown,
}: {
  burndown: DashboardBurndownView | null;
}) {
  const [listFilter, setListFilter] = useState("");
  const sort = useListSort("account");
  if (!burndown) {
    return <p>Loading burndown…</p>;
  }
  const matched = burndown.lines.filter((line) =>
    textMatches(listFilter, line.accountName),
  );
  const shown = sortRows(matched, sort.sortKey, sort.sortDir, (line, key) => {
    switch (key) {
      case "account":
        return line.accountName;
      case "inflow":
        return line.inflowMinor;
      case "outflow":
        return line.outflowMinor;
      case "net":
        return line.inflowMinor - line.outflowMinor;
      case "floor":
        return line.floorKnown ? 0 : null;
      case "ending":
        return line.endingBalanceKnown ? (line.endingBalanceMinor ?? null) : null;
      default:
        return line.accountName;
    }
  });
  const scale = burndown.scale ?? shown[0]?.scale;
  return (
    <div>
      <p>
        Read-only. Week {burndown.start} to {burndown.end}. {burndown.note}
      </p>
      <ListFilter label="Filter dashboard" value={listFilter} onChange={setListFilter} />
      <p>
        Showing {formatCount(shown.length)} of {formatCount(burndown.lines.length)} accounts.
        Click a column heading to sort. Footer totals are the shown rows.
      </p>
      <div className="table-wrap">
        <table aria-label="Dashboard burndown">
          <thead>
            <tr>
              {sortHead(sort, "Account", "account")}
              {sortHead(sort, "Dividend inflow", "inflow", true)}
              {sortHead(sort, "Disbursement outflow", "outflow", true)}
              {sortHead(sort, "Net", "net", true)}
              {sortHead(sort, "Ending balance", "ending", true)}
              {sortHead(sort, "Floor", "floor", true)}
            </tr>
          </thead>
          <tbody>
            {shown.map((line) => (
              <tr key={line.accountName}>
                <td>{line.accountName}</td>
                <td className="numeric">{formatUsd(line.inflowMinor, line.scale)}</td>
                <td className="numeric">{formatUsd(line.outflowMinor, line.scale)}</td>
                <td className="numeric">
                  {formatUsd(line.inflowMinor - line.outflowMinor, line.scale)}
                </td>
                <td className="numeric">
                  {line.endingBalanceKnown
                    ? formatUsd(line.endingBalanceMinor ?? 0, line.scale)
                    : "unknown"}
                </td>
                <td className="numeric">{line.floorKnown ? formatUsd(0, line.scale) : "N/A"}</td>
              </tr>
            ))}
          </tbody>
          <tfoot>
            <tr>
              <td>Total</td>
              <td className="numeric">
                {moneyTotal(
                  shown.map((line) => line.inflowMinor),
                  scale,
                )}
              </td>
              <td className="numeric">
                {moneyTotal(
                  shown.map((line) => line.outflowMinor),
                  scale,
                )}
              </td>
              <td className="numeric">
                {moneyTotal(
                  shown.map((line) => line.inflowMinor - line.outflowMinor),
                  scale,
                )}
              </td>
              <td className="numeric">
                {shown.every((line) => line.endingBalanceKnown)
                  ? moneyTotal(
                      shown.map((line) => line.endingBalanceMinor ?? 0),
                      scale,
                    )
                  : "unknown"}
              </td>
              <td className="numeric">
                {shown.some((line) => !line.floorKnown) ? "N/A" : formatUsd(0, scale)}
              </td>
            </tr>
          </tfoot>
        </table>
      </div>
    </div>
  );
}

export type HoldingsLotView = {
  lotId: string;
  accountName: string;
  symbol: string;
  openedOn: string;
  remainingQuantityMinor: number;
  quantityScale: number;
  remainingPerformanceMinor: number;
  remainingTaxMinor: number;
  scale: number;
};

function unitCostMinor(lot: HoldingsLotView): number | null {
  if (lot.remainingQuantityMinor <= 0) {
    return null;
  }
  const factor = 10 ** lot.quantityScale;
  return Math.trunc((lot.remainingPerformanceMinor * factor) / lot.remainingQuantityMinor);
}

function unitTaxMinor(lot: HoldingsLotView): number | null {
  if (lot.remainingQuantityMinor <= 0) {
    return null;
  }
  const factor = 10 ** lot.quantityScale;
  return Math.trunc((lot.remainingTaxMinor * factor) / lot.remainingQuantityMinor);
}

export function HoldingsPanel({
  lots,
  filter = "",
  onOpenSymbol,
}: {
  lots: HoldingsLotView[] | null;
  filter?: string;
  onOpenSymbol?: (symbol: string) => void;
}) {
  const sort = useListSort("symbol");
  if (!lots) {
    return <p>Loading holdings…</p>;
  }
  if (lots.length === 0) {
    return <p>No open lots.</p>;
  }
  const matched = lots.filter((lot) =>
    textMatches(
      filter,
      lot.accountName,
      lot.symbol,
      lot.openedOn,
      lot.lotId,
      formatUsd(lot.remainingPerformanceMinor, lot.scale),
      formatUsd(lot.remainingTaxMinor, lot.scale),
    ),
  );
  const shown = sortRows(matched, sort.sortKey, sort.sortDir, (lot, key) => {
    switch (key) {
      case "account":
        return lot.accountName;
      case "symbol":
        return lot.symbol;
      case "opened":
        return lot.openedOn;
      case "qty":
        return lot.remainingQuantityMinor;
      case "unitOrig":
        return unitCostMinor(lot);
      case "unitTax":
        return unitTaxMinor(lot);
      case "perf":
        return lot.remainingPerformanceMinor;
      case "tax":
        return lot.remainingTaxMinor;
      default:
        return lot.symbol;
    }
  });
  return (
    <div>
      <p>
        Showing {formatCount(shown.length)} of {formatCount(lots.length)} open lots. Click a
        column heading to sort. Footer totals are the shown rows. Dual cost stays separate.
        Last price and market value are on Calculator and Position Details, including a
        stale last price. Unknown stays unknown.
      </p>
      <div className="table-wrap">
        <table aria-label="Holdings lots">
          <thead>
            <tr>
              {sortHead(sort, "Account", "account")}
              {sortHead(sort, "Symbol", "symbol")}
              {sortHead(sort, "Opened", "opened")}
              {sortHead(sort, "Qty", "qty", true)}
              {sortHead(sort, "Unit orig", "unitOrig", true)}
              {sortHead(sort, "Unit tax", "unitTax", true)}
              {sortHead(sort, "Perf basis", "perf", true)}
              {sortHead(sort, "Tax basis", "tax", true)}
            </tr>
          </thead>
          <tbody>
            {shown.map((lot) => {
              const orig = unitCostMinor(lot);
              const tax = unitTaxMinor(lot);
              return (
                <tr key={lot.lotId}>
                  <td>{lot.accountName}</td>
                  <td>
                    {onOpenSymbol ? (
                      <button
                        type="button"
                        aria-label={`Open ${lot.symbol} position`}
                        onClick={() => onOpenSymbol(lot.symbol)}
                      >
                        {lot.symbol}
                      </button>
                    ) : (
                      lot.symbol
                    )}
                  </td>
                  <td>{lot.openedOn}</td>
                  <td className="numeric">
                    {formatScaled(lot.remainingQuantityMinor, lot.quantityScale)}
                  </td>
                  <td className="numeric">
                    {orig == null ? "N/A" : formatUsd(orig, lot.scale)}
                  </td>
                  <td className="numeric">
                    {tax == null ? "N/A" : formatUsd(tax, lot.scale)}
                  </td>
                  <td className="numeric">
                    {formatUsd(lot.remainingPerformanceMinor, lot.scale)}
                  </td>
                  <td className="numeric">{formatUsd(lot.remainingTaxMinor, lot.scale)}</td>
                </tr>
              );
            })}
          </tbody>
          <tfoot>
            <tr>
              <td>Total</td>
              <td>{formatCount(shown.length)} lots</td>
              <td></td>
              <td className="numeric">{qtyTotal(shown)}</td>
              <td className="numeric">—</td>
              <td className="numeric">—</td>
              <td className="numeric">
                {moneyTotal(
                  shown.map((lot) => ({
                    minor: lot.remainingPerformanceMinor,
                    scale: lot.scale,
                  })),
                )}
              </td>
              <td className="numeric">
                {moneyTotal(
                  shown.map((lot) => ({
                    minor: lot.remainingTaxMinor,
                    scale: lot.scale,
                  })),
                )}
              </td>
            </tr>
          </tfoot>
        </table>
      </div>
    </div>
  );
}

export type CalculatorRowView = {
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
  lastPriceMinor?: number | null;
  lastPriceScale?: number | null;
  priceFreshness?: string;
  marketValueMinor?: number | null;
  scale: number;
};

export type SymbolLotView = {
  lotId: string;
  accountName: string;
  openedOn: string;
  remainingQuantityMinor: number;
  quantityScale: number;
  remainingPerformanceMinor: number;
  remainingTaxMinor: number;
  scale: number;
};

export function SymbolLotsTable({ lots }: { lots: SymbolLotView[] }) {
  const [listFilter, setListFilter] = useState("");
  const sort = useListSort("opened");
  const matched = lots.filter((lot) =>
    textMatches(listFilter, lot.accountName, lot.openedOn, lot.lotId),
  );
  const shown = sortRows(matched, sort.sortKey, sort.sortDir, (lot, key) => {
    switch (key) {
      case "account":
        return lot.accountName;
      case "opened":
        return lot.openedOn;
      case "qty":
        return lot.remainingQuantityMinor;
      case "cost":
        return lot.remainingPerformanceMinor;
      case "tax":
        return lot.remainingTaxMinor;
      default:
        return lot.openedOn;
    }
  });
  return (
    <div>
      <ListFilter label="Filter position lots" value={listFilter} onChange={setListFilter} />
      <p>
        Showing {formatCount(shown.length)} of {formatCount(lots.length)} lots. Click a
        column heading to sort. Footer totals are the shown rows.
      </p>
      <div className="table-wrap">
        <table aria-label="Position lots">
          <thead>
            <tr>
              {sortHead(sort, "Account", "account")}
              {sortHead(sort, "Opened", "opened")}
              {sortHead(sort, "Qty", "qty", true)}
              {sortHead(sort, "Cost", "cost", true)}
              {sortHead(sort, "Tax", "tax", true)}
            </tr>
          </thead>
          <tbody>
            {shown.map((lot) => (
              <tr key={lot.lotId}>
                <td>{lot.accountName}</td>
                <td>{lot.openedOn}</td>
                <td className="numeric">
                  {formatScaled(lot.remainingQuantityMinor, lot.quantityScale)}
                </td>
                <td className="numeric">
                  {formatUsd(lot.remainingPerformanceMinor, lot.scale)}
                </td>
                <td className="numeric">{formatUsd(lot.remainingTaxMinor, lot.scale)}</td>
              </tr>
            ))}
          </tbody>
          <tfoot>
            <tr>
              <td>Total</td>
              <td>{formatCount(shown.length)}</td>
              <td className="numeric">{qtyTotal(shown)}</td>
              <td className="numeric">
                {moneyTotal(
                  shown.map((lot) => ({
                    minor: lot.remainingPerformanceMinor,
                    scale: lot.scale,
                  })),
                )}
              </td>
              <td className="numeric">
                {moneyTotal(
                  shown.map((lot) => ({
                    minor: lot.remainingTaxMinor,
                    scale: lot.scale,
                  })),
                )}
              </td>
            </tr>
          </tfoot>
        </table>
      </div>
    </div>
  );
}

export function CalculatorPanel({
  rows,
  onOpenSymbol,
}: {
  rows: CalculatorRowView[] | null;
  onOpenSymbol?: (symbol: string) => void;
}) {
  const [listFilter, setListFilter] = useState("");
  const sort = useListSort("symbol");
  if (!rows) {
    return <p>Loading Calculator…</p>;
  }
  if (rows.length === 0) {
    return <p>No Calculator positions yet. Complete New Investment including the first lot.</p>;
  }
  const needle = listFilter.trim();
  const matched = rows.filter((row) =>
    textMatches(
      needle,
      row.symbol,
      row.paymentFrequency,
      row.planKnown ? formatScaled(row.planPerShareMinor, row.planScale) : "N/A",
      formatUsd(row.remainingPerformanceMinor, row.scale),
      row.priceFreshness ?? "",
      row.lastPriceMinor != null
        ? formatUsd(row.lastPriceMinor, row.lastPriceScale ?? row.scale)
        : "",
    ),
  );
  const shown = sortRows(matched, sort.sortKey, sort.sortDir, (row, key) => {
    switch (key) {
      case "symbol":
        return row.symbol;
      case "freq":
        return row.paymentFrequency;
      case "planShare":
        return row.planKnown ? row.planPerShareMinor : null;
      case "periods":
        return row.planningPeriodsPerYear > 0 ? row.planningPeriodsPerYear : null;
      case "qty":
        return row.remainingQuantityMinor;
      case "planPay":
        return row.planKnown ? row.planPaymentMinor : null;
      case "cost":
        return row.remainingPerformanceMinor;
      case "lastPrice":
        return row.lastPriceMinor ?? null;
      case "freshness":
        return row.priceFreshness ?? "";
      case "mv":
        return row.marketValueMinor ?? null;
      case "roc":
        return row.rocPct2025ActualMinor;
      default:
        return row.symbol;
    }
  });
  return (
    <div>
      <p>
        Plan is owner-controlled per share (Calculator AE extract). Declarations and broker
        cash never change it. Most Current and Avg 6 are on Position Details after
        declarations are stored. Missing Plan is N/A, not $0.00.
      </p>
      <ListFilter label="Filter calculator" value={listFilter} onChange={setListFilter} />
      <p>
        Showing {formatCount(shown.length)} of {formatCount(rows.length)} positions. Click a
        column heading to sort. Footer totals are the shown rows.
      </p>
      <div className="table-wrap">
        <table aria-label="Calculator positions">
          <thead>
            <tr>
              {sortHead(sort, "Symbol", "symbol")}
              {sortHead(sort, "Frequency", "freq")}
              {sortHead(sort, "Plan / share", "planShare", true)}
              {sortHead(sort, "Periods", "periods", true)}
              {sortHead(sort, "Qty", "qty", true)}
              {sortHead(sort, "Plan payment", "planPay", true)}
              {sortHead(sort, "Cost", "cost", true)}
              {sortHead(sort, "Last price", "lastPrice", true)}
              {sortHead(sort, "Freshness", "freshness")}
              {sortHead(sort, "Market value", "mv", true)}
              {sortHead(sort, "ROC 2025", "roc", true)}
            </tr>
          </thead>
          <tbody>
            {shown.map((row) => (
              <tr key={row.symbol}>
                <td>
                  {onOpenSymbol ? (
                    <button
                      type="button"
                      aria-label={`Open ${row.symbol} position`}
                      onClick={() => onOpenSymbol(row.symbol)}
                    >
                      {row.symbol}
                    </button>
                  ) : (
                    row.symbol
                  )}
                </td>
                <td>{row.paymentFrequency || "—"}</td>
                <td className="numeric">
                  {row.planKnown
                    ? `$${formatScaled(row.planPerShareMinor, row.planScale)}`
                    : "N/A"}
                </td>
                <td className="numeric">
                  {row.planningPeriodsPerYear > 0
                    ? formatCount(row.planningPeriodsPerYear)
                    : "N/A"}
                </td>
                <td className="numeric">
                  {formatScaled(row.remainingQuantityMinor, row.quantityScale)}
                </td>
                <td className="numeric">
                  {row.planKnown ? formatUsd(row.planPaymentMinor, row.scale) : "N/A"}
                </td>
                <td className="numeric">
                  {formatUsd(row.remainingPerformanceMinor, row.scale)}
                </td>
                <td className="numeric">
                  {row.lastPriceMinor == null
                    ? "unknown"
                    : formatUsd(row.lastPriceMinor, row.lastPriceScale ?? row.scale)}
                </td>
                <td>{row.priceFreshness || "unavailable"}</td>
                <td className="numeric">
                  {row.marketValueMinor == null
                    ? "unknown"
                    : formatUsd(row.marketValueMinor, row.scale)}
                </td>
                <td className="numeric">
                  {row.rocPct2025ActualMinor == null || row.rocScale == null
                    ? "N/A"
                    : formatPercentScaled(row.rocPct2025ActualMinor, row.rocScale)}
                </td>
              </tr>
            ))}
          </tbody>
          <tfoot>
            <tr>
              <td>Total</td>
              <td>{formatCount(shown.length)}</td>
              <td className="numeric">—</td>
              <td className="numeric">—</td>
              <td className="numeric">{qtyTotal(shown)}</td>
              <td className="numeric">
                {moneyTotal(
                  shown.map((row) => ({
                    minor: row.planKnown ? row.planPaymentMinor : null,
                    scale: row.scale,
                  })),
                )}
              </td>
              <td className="numeric">
                {moneyTotal(
                  shown.map((row) => ({
                    minor: row.remainingPerformanceMinor,
                    scale: row.scale,
                  })),
                )}
              </td>
              <td className="numeric">—</td>
              <td className="numeric">—</td>
              <td className="numeric">
                {moneyTotal(
                  shown.map((row) => ({
                    minor: row.marketValueMinor ?? null,
                    scale: row.scale,
                  })),
                )}
              </td>
              <td className="numeric">—</td>
            </tr>
          </tfoot>
        </table>
      </div>
    </div>
  );
}

