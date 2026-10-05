import { useState } from "react";
import type { CartExecutedRow } from "@finos/app-contracts";

import { sumOrBlank, usd } from "./planMath";

function signed(minor: number | null): string {
  if (minor == null) return "—";
  return minor < 0 ? `-${usd(Math.abs(minor))}` : usd(minor);
}

type SortKey = "name" | "account" | "asOf" | "sales" | "pl" | "invested" | "income";

const COLUMNS: { key: SortKey; label: string }[] = [
  { key: "name", label: "Cart" },
  { key: "account", label: "Account" },
  { key: "asOf", label: "As of" },
  { key: "sales", label: "Sales (non-cash)" },
  { key: "pl", label: "Realized P/L" },
  { key: "invested", label: "Invested" },
  { key: "income", label: "Monthly income change" },
];

function sortValue(row: CartExecutedRow, key: SortKey): string | number | null {
  switch (key) {
    case "name":
      return (row.name?.trim() || "Cart").toLowerCase();
    case "account":
      return row.accountName.toLowerCase();
    case "asOf":
      return row.asOf;
    case "sales":
      return row.nonCashSalesMinor;
    case "pl":
      return row.realizedPlMinor;
    case "invested":
      return row.investedMinor;
    case "income":
      return row.deltaMonthlyIncomeMinor;
  }
}

function compareRows(a: CartExecutedRow, b: CartExecutedRow, key: SortKey): number {
  const left = sortValue(a, key);
  const right = sortValue(b, key);
  if (left == null && right == null) return 0;
  if (left == null) return 1;
  if (right == null) return -1;
  if (typeof left === "number" && typeof right === "number") return left - right;
  return String(left).localeCompare(String(right));
}

/**
 * Completed carts for every account. The list arrives newest first and stays that way until a
 * column header is clicked. Sales, P/L, and dollars invested are actual; the monthly income
 * delta is the one stored when the cart was agreed.
 */
export function ExecutedCarts({
  rows,
  busy,
  onOpen,
}: {
  rows: CartExecutedRow[];
  busy?: boolean;
  onOpen: (row: CartExecutedRow) => void;
}) {
  const [sort, setSort] = useState<{ key: SortKey; dir: "asc" | "desc" } | null>(null);
  const shown = sort
    ? [...rows].sort((a, b) => {
        const left = sortValue(a, sort.key);
        const right = sortValue(b, sort.key);
        if (left == null || right == null) return compareRows(a, b, sort.key);
        const gap = compareRows(a, b, sort.key);
        return sort.dir === "asc" ? gap : -gap;
      })
    : rows;
  const toggle = (key: SortKey) => {
    setSort((current) => {
      if (current?.key !== key) return { key, dir: "asc" };
      return { key, dir: current.dir === "asc" ? "desc" : "asc" };
    });
  };
  const salesTotal = rows.reduce((sum, row) => sum + row.nonCashSalesMinor, 0);
  const plTotal = rows.reduce((sum, row) => sum + row.realizedPlMinor, 0);
  const investedTotal = rows.reduce((sum, row) => sum + row.investedMinor, 0);
  const incomeTotal = sumOrBlank(rows.map((row) => row.deltaMonthlyIncomeMinor));
  return (
    <section aria-label="Executed carts" className="plan-sheet">
      <h3>Executed carts</h3>
      <table>
        <thead>
          <tr>
            {COLUMNS.map((column) => {
              const active = sort?.key === column.key;
              const ariaSort = !active ? "none" : sort.dir === "asc" ? "ascending" : "descending";
              return (
                <th key={column.key} scope="col" aria-sort={ariaSort}>
                  <button type="button" onClick={() => toggle(column.key)}>
                    {column.label}
                  </button>
                </th>
              );
            })}
            <th scope="col">Open</th>
          </tr>
        </thead>
        <tbody>
          {shown.length === 0 ? (
            <tr>
              <td colSpan={8}>No executed carts yet.</td>
            </tr>
          ) : (
            <>
              <tr className="executed-cart-total" aria-label="Total of all executed carts">
                <th scope="row">Total</th>
                <td />
                <td />
                <td>{usd(salesTotal)}</td>
                <td>{signed(plTotal)}</td>
                <td>{usd(investedTotal)}</td>
                <td>{signed(incomeTotal)}</td>
                <td />
              </tr>
              {shown.map((row) => (
                <tr key={row.scenarioId}>
                  <td>{row.name?.trim() || "Cart"}</td>
                  <td>{row.accountName}</td>
                  <td>{row.asOf.slice(0, 10)}</td>
                  <td>{usd(row.nonCashSalesMinor)}</td>
                  <td>{signed(row.realizedPlMinor)}</td>
                  <td>{usd(row.investedMinor)}</td>
                  <td>{signed(row.deltaMonthlyIncomeMinor)}</td>
                  <td>
                    <button
                      type="button"
                      aria-label={`Open ${row.name?.trim() || row.asOf}`}
                      disabled={busy}
                      onClick={() => onOpen(row)}
                    >
                      Open
                    </button>
                  </td>
                </tr>
              ))}
            </>
          )}
        </tbody>
      </table>
    </section>
  );
}
