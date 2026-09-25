import type { CartScenario } from "@finos/app-contracts";
import { formatUsd } from "@finos/ui-components";

function money(minor: number | null | undefined): string {
  if (minor == null) return "unknown";
  return formatUsd(minor, 2);
}

export function ComparePlans({
  keepMonthlyMinor,
  keepAnnualMinor,
  drafts,
  activeId,
  onSelect,
}: {
  keepMonthlyMinor: number | null;
  keepAnnualMinor: number | null;
  drafts: CartScenario[];
  activeId: string;
  onSelect: (scenarioId: string) => void;
}) {
  const shown = drafts.slice(0, 4);
  return (
    <section aria-label="Compare plans">
      <h3>Compare drafts</h3>
      <table>
        <thead>
          <tr>
            <th scope="col"> </th>
            <th scope="col">Cash on traded</th>
            {shown.map((d) => (
              <th key={d.scenarioId} scope="col">
                <button
                  type="button"
                  aria-label={`Select draft ${d.name || "Draft"}`}
                  aria-pressed={d.scenarioId === activeId}
                  onClick={() => onSelect(d.scenarioId)}
                >
                  {d.name || "Draft"}
                </button>
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          <tr>
            <th scope="row">Week</th>
            <td>
              {keepAnnualMinor == null ? "unknown" : money(Math.trunc(keepAnnualMinor / 52))}
            </td>
            {shown.map((d) => (
              <td key={d.scenarioId}>{money(d.eval?.netWeeklyMinor)}</td>
            ))}
          </tr>
          <tr>
            <th scope="row">Month</th>
            <td>{money(keepMonthlyMinor)}</td>
            {shown.map((d) => (
              <td key={d.scenarioId}>{money(d.eval?.netMonthlyMinor)}</td>
            ))}
          </tr>
          <tr>
            <th scope="row">Year</th>
            <td>{money(keepAnnualMinor)}</td>
            {shown.map((d) => (
              <td key={d.scenarioId}>{money(d.eval?.netAnnualMinor)}</td>
            ))}
          </tr>
          <tr>
            <th scope="row">Tax P/L if sells execute</th>
            <td>—</td>
            {shown.map((d) => {
              const known = d.sellLines.every((l) => l.isCash || l.taxGainMinor != null);
              const sum = d.sellLines.reduce((s, l) => s + (l.taxGainMinor ?? 0), 0);
              const any = d.sellLines.some((l) => !l.isCash);
              return (
                <td key={d.scenarioId}>
                  {!any ? "—" : known ? money(sum) : "unknown"}
                </td>
              );
            })}
          </tr>
        </tbody>
      </table>
    </section>
  );
}
