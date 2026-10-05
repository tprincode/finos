import type { ReactNode } from "react";
import { formatUsd } from "@finos/ui-components";
import { pctOf, usd } from "./planMath";
import { rowCountsInSheetTotal } from "./planSheetRollups";

export type PlanSheetRow = {
  key: string;
  priceMinor: number | null;
  symbol: string;
  alloc: string;
  qty: string;
  qtyNode?: ReactNode;
  marketMinor: number | null;
  weekMinor: number | null;
  monthMinor: number | null;
  yearMinor: number | null;
  eachMinor: number | null;
  yieldText: string;
  /** "Plan $0.2300 x 24" — the Plan $ and periods this rate came from. */
  planBasis?: string;
  tier: string;
  taxGainMinor?: number | null;
  performanceGainMinor?: number | null;
};

function cellMoney(minor: number | null | undefined): string {
  if (minor == null) return "";
  return formatUsd(minor, 2);
}

function paren(minor: number | null): string {
  if (minor == null) return "";
  return `(${formatUsd(Math.abs(minor), 2)})`;
}

/** Sum a column when some rows are still blank (e.g. sell cash qty not entered yet). */
function sumKnown(rows: PlanSheetRow[], pick: (row: PlanSheetRow) => number | null): number | null {
  let sum = 0;
  let any = false;
  for (const row of rows) {
    if (!rowCountsInSheetTotal(row)) continue;
    const value = pick(row);
    if (value == null) continue;
    sum += value;
    any = true;
  }
  return any ? sum : null;
}

export function PlanSheetTable({
  title,
  ariaLabel,
  budgetMinor,
  rows,
  showPnl,
  unspentMinor,
  returnText,
  returnLabel,
  editor,
  footerRows,
  onRemoveRow,
  canRemoveRow,
  removeDisabled,
}: {
  title: string;
  ariaLabel: string;
  budgetMinor: number | null;
  rows: PlanSheetRow[];
  showPnl?: boolean;
  unspentMinor?: number | null;
  returnText: string;
  returnLabel: string;
  editor?: ReactNode;
  /** Calculated rows after the total. They stay out of the total. */
  footerRows?: PlanSheetRow[];
  onRemoveRow?: (key: string) => void;
  canRemoveRow?: (row: PlanSheetRow) => boolean;
  removeDisabled?: boolean;
}) {
  const showRemove = Boolean(onRemoveRow);
  const removable = (row: PlanSheetRow) =>
    showRemove &&
    rowCountsInSheetTotal(row) &&
    (canRemoveRow ? canRemoveRow(row) : true);
  const week = sumKnown(rows, (row) => row.weekMinor);
  const month = sumKnown(rows, (row) => row.monthMinor);
  const year = sumKnown(rows, (row) => row.yearMinor);
  const dollars = sumKnown(rows, (row) => row.marketMinor);
  return (
    <section aria-label={ariaLabel} className="plan-sheet">
      <h3>{title}</h3>
      <table>
        <thead>
          <tr>
            <th scope="col">{budgetMinor == null ? "" : usd(budgetMinor)}</th>
            <th scope="col">Symbol</th>
            <th scope="col">% Allocated</th>
            <th scope="col">Qty</th>
            <th scope="col">Market value</th>
            <th scope="col">Plan $</th>
            <th scope="col">Week</th>
            <th scope="col">Month</th>
            <th scope="col">Year</th>
            <th scope="col">Annual each</th>
            <th scope="col">Yield</th>
            <th scope="col">Position type</th>
            {showPnl ? <th scope="col">Tax P/L</th> : null}
            {showPnl ? <th scope="col">Performance P/L</th> : null}
            {showRemove ? <th scope="col" aria-label="Row actions" /> : null}
          </tr>
        </thead>
        <tbody>
          {rows.map((row) => (
            <tr
              key={row.key}
              className={
                row.key.startsWith("subtotal:")
                  ? "plan-sheet-subtotal"
                  : row.key === "cash-remainder"
                    ? "plan-sheet-cash-remainder"
                    : undefined
              }
            >
              <td>{cellMoney(row.priceMinor)}</td>
              <td>{row.symbol}</td>
              <td>{row.alloc}</td>
              <td>{row.qtyNode ?? row.qty}</td>
              <td>{cellMoney(row.marketMinor)}</td>
              <td>{paren(row.marketMinor)}</td>
              <td>{cellMoney(row.weekMinor)}</td>
              <td>{cellMoney(row.monthMinor)}</td>
              <td>{cellMoney(row.yearMinor)}</td>
              <td>{cellMoney(row.eachMinor)}</td>
              <td>
                {row.yieldText}
                {row.planBasis ? <span className="plan-basis">{row.planBasis}</span> : null}
              </td>
              <td>{row.tier}</td>
              {showPnl ? <td>{cellMoney(row.taxGainMinor)}</td> : null}
              {showPnl ? <td>{cellMoney(row.performanceGainMinor)}</td> : null}
              {showRemove ? (
                <td>
                  {removable(row) ? (
                    <button
                      type="button"
                      aria-label={`Delete ${row.symbol} row`}
                      disabled={removeDisabled}
                      onClick={() => onRemoveRow?.(row.key)}
                    >
                      Delete
                    </button>
                  ) : null}
                </td>
              ) : null}
            </tr>
          ))}
          {editor}
          <tr>
            <td colSpan={4}>Total</td>
            <td>{cellMoney(dollars)}</td>
            <td>{paren(dollars)}</td>
            <td>{cellMoney(week)}</td>
            <td>{cellMoney(month)}</td>
            <td>{cellMoney(year)}</td>
            <td />
            <td>{budgetMinor != null && year != null ? pctOf(year, budgetMinor, 2) : ""}</td>
            <td>{returnLabel}</td>
            {showPnl ? <td /> : null}
            {showPnl ? <td /> : null}
            {showRemove ? <td /> : null}
          </tr>
          {footerRows?.map((row) => (
            <tr
              key={row.key}
              className={
                row.key === "cash-remainder" ? "plan-sheet-cash-remainder" : undefined
              }
            >
              <td>{cellMoney(row.priceMinor)}</td>
              <td>{row.symbol}</td>
              <td>{row.alloc}</td>
              <td>{row.qtyNode ?? row.qty}</td>
              <td>{cellMoney(row.marketMinor)}</td>
              <td>{paren(row.marketMinor)}</td>
              <td>{cellMoney(row.weekMinor)}</td>
              <td>{cellMoney(row.monthMinor)}</td>
              <td>{cellMoney(row.yearMinor)}</td>
              <td>{cellMoney(row.eachMinor)}</td>
              <td>
                {row.yieldText}
                {row.planBasis ? <span className="plan-basis">{row.planBasis}</span> : null}
              </td>
              <td>{row.tier}</td>
              {showPnl ? <td>{cellMoney(row.taxGainMinor)}</td> : null}
              {showPnl ? <td>{cellMoney(row.performanceGainMinor)}</td> : null}
              {showRemove ? <td /> : null}
            </tr>
          ))}
          {unspentMinor !== undefined ? (
            <tr>
              <td colSpan={4}>Unspent</td>
              <td>{cellMoney(unspentMinor)}</td>
              <td colSpan={(showPnl ? 10 : 8) + (showRemove ? 1 : 0)} />
            </tr>
          ) : null}
          <tr>
            <td colSpan={6} />
            <td />
            <td>{cellMoney(month)}</td>
            <td>{cellMoney(year)}</td>
            <td />
            <td>{returnText}</td>
            <td>{returnLabel}</td>
            {showPnl ? <td /> : null}
            {showPnl ? <td /> : null}
            {showRemove ? <td /> : null}
          </tr>
        </tbody>
      </table>
    </section>
  );
}
