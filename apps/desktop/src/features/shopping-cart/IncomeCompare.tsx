import type { CartEval } from "@finos/app-contracts";
import { formatUsd } from "@finos/ui-components";

function money(minor: number | null | undefined): string {
  if (minor == null) return "unknown";
  return formatUsd(minor, 2);
}

export function IncomeCompare({ eval: ev }: { eval: CartEval | null }) {
  if (!ev) return null;
  return (
    <section aria-label="Keep vs swap income">
      <h3>Keep vs swap</h3>
      <table>
        <tbody>
          <tr>
            <th scope="row">Surrendered (plan on dollars spent)</th>
            <td>{money(ev.surrenderedAnnualMinor)}</td>
          </tr>
          <tr>
            <th scope="row">Buy plan</th>
            <td>{money(ev.buyAnnualMinor)}</td>
          </tr>
          <tr>
            <th scope="row">Leftover cash still earns</th>
            <td>{money(ev.leftoverAnnualMinor)}</td>
          </tr>
          <tr>
            <th scope="row">Week</th>
            <td>{money(ev.netWeeklyMinor)}</td>
          </tr>
          <tr>
            <th scope="row">Month</th>
            <td>{money(ev.netMonthlyMinor)}</td>
          </tr>
          <tr>
            <th scope="row">Year</th>
            <td>{money(ev.netAnnualMinor)}</td>
          </tr>
        </tbody>
      </table>
    </section>
  );
}
