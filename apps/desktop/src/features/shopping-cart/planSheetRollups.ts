import type { PlanSheetRow } from "./PlanSheets";
import { pctOf, roundedWeekMonth } from "./planMath";

export function rowCountsInSheetTotal(row: PlanSheetRow): boolean {
  if (row.key.startsWith("subtotal:") || row.key === "cash-remainder") return false;
  return true;
}

function sumQtyLabels(rows: PlanSheetRow[]): string {
  let sum = 0;
  let any = false;
  let fractional = false;
  for (const row of rows) {
    const n = Number(String(row.qty).trim());
    if (!Number.isFinite(n)) continue;
    sum += n;
    any = true;
    if (!Number.isInteger(n)) fractional = true;
  }
  if (!any) return "";
  return fractional ? String(Math.round(sum * 100) / 100) : String(sum);
}

function sumColumn(
  rows: PlanSheetRow[],
  pick: (row: PlanSheetRow) => number | null | undefined,
): number | null {
  let sum = 0;
  let any = false;
  for (const row of rows) {
    const value = pick(row);
    if (value == null) continue;
    sum += value;
    any = true;
  }
  return any ? sum : null;
}

export function withSymbolSubtotals(rows: PlanSheetRow[]): PlanSheetRow[] {
  const out: PlanSheetRow[] = [];
  let i = 0;
  while (i < rows.length) {
    const symbol = rows[i].symbol;
    const group: PlanSheetRow[] = [];
    while (i < rows.length && rows[i].symbol === symbol) {
      group.push(rows[i]);
      i += 1;
    }
    out.push(...group);
    if (group.length <= 1) continue;
    const marketMinor = sumColumn(group, (r) => r.marketMinor);
    const yearMinor = sumColumn(group, (r) => r.yearMinor);
    const parts = roundedWeekMonth(yearMinor);
    const taxGainMinor = sumColumn(group, (r) => r.taxGainMinor ?? null);
    const performanceGainMinor = sumColumn(group, (r) => r.performanceGainMinor ?? null);
    out.push({
      key: `subtotal:${symbol}`,
      priceMinor: null,
      symbol: `${symbol} subtotal`,
      alloc: "",
      qty: sumQtyLabels(group),
      marketMinor,
      weekMinor: parts.weekMinor,
      monthMinor: parts.monthMinor,
      yearMinor,
      eachMinor: null,
      yieldText: pctOf(yearMinor, marketMinor, 2),
      tier: group[0]?.tier ?? "",
      taxGainMinor,
      performanceGainMinor,
    });
  }
  return out;
}

/** Starting SPAXX + position proceeds − scenario buys (no SPAXX trade line). */
export function endingCashMinor(
  startingCashMinor: number | null,
  positionProceedsMinor: number | null,
  buySpendMinor: number | null,
): number | null {
  if (startingCashMinor == null && positionProceedsMinor == null && buySpendMinor == null) {
    return null;
  }
  return (startingCashMinor ?? 0) + (positionProceedsMinor ?? 0) - (buySpendMinor ?? 0);
}

export function cashRemainderRow(input: {
  cashSymbol: string;
  startingCashMinor: number | null;
  positionProceedsMinor: number | null;
  buySpendMinor: number | null;
  budgetMinor: number | null;
  tier: string;
  yearMinorOnCash: (qtyWhole: number) => number | null;
}): PlanSheetRow | null {
  const marketMinor = endingCashMinor(
    input.startingCashMinor,
    input.positionProceedsMinor,
    input.buySpendMinor,
  );
  if (marketMinor == null) return null;
  const qty = marketMinor / 100;
  const yearMinor = input.yearMinorOnCash(Math.max(0, qty));
  const parts = roundedWeekMonth(yearMinor);
  return {
    key: "cash-remainder",
    priceMinor: 100,
    symbol: `${input.cashSymbol} (remainder)`,
    alloc: pctOf(marketMinor, input.budgetMinor, 1),
    qty: String(Math.round(qty * 100) / 100),
    marketMinor,
    weekMinor: parts.weekMinor,
    monthMinor: parts.monthMinor,
    yearMinor,
    eachMinor: yearMinor == null || qty <= 0 ? null : Math.round(yearMinor / qty),
    yieldText: pctOf(yearMinor, marketMinor, 2),
    tier: input.tier,
    taxGainMinor: null,
    performanceGainMinor: null,
  };
}
