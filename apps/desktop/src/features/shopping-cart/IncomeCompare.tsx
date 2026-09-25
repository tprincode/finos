import type { CartEval } from "@finos/app-contracts";
import { formatUsd } from "@finos/ui-components";

function money(minor: number | null | undefined): string {
  if (minor == null) return "unknown";
  return formatUsd(minor, 2);
}

function slice(annual: number | null | undefined, divisor: number): number | null {
  if (annual == null) return null;
  return Math.trunc(annual / divisor);
}

export function IncomeCompare({ eval: ev }: { eval: CartEval | null }) {
  if (!ev) return null;
  const cashYear = ev.surrenderedAnnualMinor;
  const positionYear = ev.buyAnnualMinor;
  const deltaYear = ev.netAnnualMinor;
  return (
    <section aria-label="Traded dollars income">
      <h3>Traded dollars</h3>
      <p>
        Trading {money(ev.spendMinor)} changes the cash income on that amount from{" "}
        {money(cashYear)} a year to {money(positionYear)}. Delta {money(deltaYear)}.
      </p>
      <table>
        <thead>
          <tr>
            <th scope="col"> </th>
            <th scope="col">Cash on traded</th>
            <th scope="col">New position</th>
            <th scope="col">Delta</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <th scope="row">Week</th>
            <td>{money(slice(cashYear, 52))}</td>
            <td>{money(slice(positionYear, 52))}</td>
            <td>{money(ev.netWeeklyMinor)}</td>
          </tr>
          <tr>
            <th scope="row">Month</th>
            <td>{money(slice(cashYear, 12))}</td>
            <td>{money(slice(positionYear, 12))}</td>
            <td>{money(ev.netMonthlyMinor)}</td>
          </tr>
          <tr>
            <th scope="row">Year</th>
            <td>{money(cashYear)}</td>
            <td>{money(positionYear)}</td>
            <td>{money(deltaYear)}</td>
          </tr>
        </tbody>
      </table>
    </section>
  );
}
