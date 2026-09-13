import type { CartBuyLine } from "@finos/app-contracts";
import { formatBps, formatUsd } from "@finos/ui-components";

export type BlendRow = {
  line: CartBuyLine;
  lastMinor: number;
  spendMinor: number;
  yearMinor: number | null;
  monthMinor: number | null;
  weekMinor: number | null;
  eachAnnualMinor: number | null;
  yieldBps: number | null;
  allocBps: number | null;
  risk: string;
};

function money(minor: number | null | undefined): string {
  if (minor == null) return "unknown";
  return formatUsd(minor, 2);
}

export function CartBlendTable({
  allocatedMinor,
  leftoverMinor,
  leftoverWeekMinor,
  leftoverMonthMinor,
  leftoverYearMinor,
  rows,
  overBudget,
  agreed,
  busy,
  onQty,
}: {
  allocatedMinor: number;
  leftoverMinor: number;
  leftoverWeekMinor: number | null;
  leftoverMonthMinor: number | null;
  leftoverYearMinor: number | null;
  rows: BlendRow[];
  overBudget: boolean;
  agreed: boolean;
  busy?: boolean;
  onQty: (lineId: string, qtyWhole: number) => void;
}) {
  const spend = rows.reduce((s, r) => s + r.spendMinor, 0);
  const year = rows.every((r) => r.yearMinor != null)
    ? rows.reduce((s, r) => s + (r.yearMinor ?? 0), 0)
    : null;
  const month = year == null ? null : Math.trunc(year / 12);
  const week = year == null ? null : Math.trunc(year / 52);
  const blendBps =
    spend > 0 && year != null ? Math.round((year * 10_000) / spend) : null;
  return (
    <section aria-label="Cart blend" className="cart-blend">
      <p>
        Allocated {formatUsd(allocatedMinor, 2)}
        {overBudget ? " — over budget (experimenting)" : ""}
      </p>
      <table>
        <thead>
          <tr>
            <th scope="col">$</th>
            <th scope="col">Symbol</th>
            <th scope="col">%</th>
            <th scope="col">Qty</th>
            <th scope="col">Spend</th>
            <th scope="col">Week</th>
            <th scope="col">Month</th>
            <th scope="col">Year</th>
            <th scope="col">Annual each</th>
            <th scope="col">Yield</th>
            <th scope="col">Risk</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((row) => (
            <tr key={row.line.lineId}>
              <td>{money(row.lastMinor)}</td>
              <td>{row.line.symbol}</td>
              <td>{formatBps(row.allocBps)}</td>
              <td>
                <input
                  aria-label={`Cart blend qty ${row.line.symbol}`}
                  type="number"
                  min={1}
                  step={1}
                  value={row.line.qtyWhole}
                  disabled={busy || agreed}
                  onChange={(e) => {
                    const n = Number(e.target.value);
                    if (Number.isInteger(n) && n > 0) onQty(row.line.lineId, n);
                  }}
                />
              </td>
              <td>{money(row.spendMinor)}</td>
              <td>{money(row.weekMinor)}</td>
              <td>{money(row.monthMinor)}</td>
              <td>{money(row.yearMinor)}</td>
              <td>{money(row.eachAnnualMinor)}</td>
              <td>{formatBps(row.yieldBps)}</td>
              <td>{row.risk}</td>
            </tr>
          ))}
          <tr>
            <td colSpan={4}>Leftover cash</td>
            <td>{money(leftoverMinor)}</td>
            <td>{money(leftoverWeekMinor)}</td>
            <td>{money(leftoverMonthMinor)}</td>
            <td>{money(leftoverYearMinor)}</td>
            <td />
            <td>{formatBps(blendBps)}</td>
            <td>New return</td>
          </tr>
          <tr>
            <td colSpan={5}>Blend</td>
            <td>{money(week)}</td>
            <td>{money(month)}</td>
            <td>{money(year)}</td>
            <td />
            <td>{formatBps(blendBps)}</td>
            <td />
          </tr>
        </tbody>
      </table>
    </section>
  );
}
