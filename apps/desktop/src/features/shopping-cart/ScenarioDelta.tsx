import type { PlanSheetRow } from "./PlanSheets";
import { usd } from "./planMath";

const TIERS = ["Foundation", "Core", "Risk On"] as const;

function sumMinor(rows: PlanSheetRow[], key: "weekMinor" | "monthMinor" | "yearMinor"): number | null {
  if (rows.length === 0 || rows.some((row) => row[key] == null)) return null;
  return rows.reduce((sum, row) => sum + (row[key] ?? 0), 0);
}

function signed(minor: number | null): string {
  if (minor == null) return "";
  if (minor === 0) return usd(0);
  return minor > 0 ? `+${usd(minor)}` : `-${usd(Math.abs(minor))}`;
}

function tierMix(rows: PlanSheetRow[]): Record<(typeof TIERS)[number], number> {
  const mix = { Foundation: 0, Core: 0, "Risk On": 0 };
  for (const row of rows) {
    if (row.marketMinor == null) continue;
    if (row.tier === "Foundation" || row.tier === "Core" || row.tier === "Risk On") {
      mix[row.tier] += row.marketMinor;
    }
  }
  return mix;
}

function mixTotal(mix: Record<(typeof TIERS)[number], number>): number {
  return TIERS.reduce((sum, tier) => sum + mix[tier], 0);
}

function pct(part: number, total: number): string {
  if (total <= 0) return "";
  return `${((part / total) * 100).toFixed(1)}%`;
}

function points(before: number, after: number, beforeTotal: number, afterTotal: number): string {
  const beforePct = beforeTotal <= 0 ? 0 : before / beforeTotal;
  const afterPct = afterTotal <= 0 ? 0 : after / afterTotal;
  const gap = (afterPct - beforePct) * 100;
  const sign = gap > 0 ? "+" : "";
  return `${sign}${gap.toFixed(1)} pp`;
}

function typesOf(rows: PlanSheetRow[]): string {
  const names = [...new Set(rows.map((row) => row.tier).filter(Boolean))];
  return names.length === 0 ? "" : names.join(", ");
}

function characteristicDelta(sell: string, buy: string): string {
  if (!sell || !buy) return "";
  return sell === buy ? "unchanged" : `${sell} to ${buy}`;
}

export function ScenarioDelta({
  sellRows,
  rowsA,
  rowsB,
  cashBefore,
  cashAfterA,
  cashAfterB,
}: {
  sellRows: PlanSheetRow[];
  rowsA: PlanSheetRow[];
  rowsB: PlanSheetRow[] | null;
  cashBefore: number | null;
  cashAfterA: number | null;
  cashAfterB: number | null;
}) {
  const sellMix = tierMix(sellRows);
  const mixA = tierMix(rowsA);
  const mixB = rowsB ? tierMix(rowsB) : null;
  const sellMixTotal = mixTotal(sellMix);
  const mixTotalA = mixTotal(mixA);
  const mixTotalB = mixB ? mixTotal(mixB) : 0;
  const sellTypes = typesOf(sellRows);
  const typesA = typesOf(rowsA);
  const typesB = rowsB ? typesOf(rowsB) : "";
  const pair = (
    sell: number | null,
    after: number | null,
  ): { text: string; delta: string } => ({
    text: after == null ? "" : usd(after),
    delta: signed(sell == null || after == null ? null : after - sell),
  });
  const weekSell = sumMinor(sellRows, "weekMinor");
  const monthSell = sumMinor(sellRows, "monthMinor");
  const yearSell = sumMinor(sellRows, "yearMinor");
  const weekA = pair(weekSell, sumMinor(rowsA, "weekMinor"));
  const monthA = pair(monthSell, sumMinor(rowsA, "monthMinor"));
  const yearA = pair(yearSell, sumMinor(rowsA, "yearMinor"));
  const weekB = rowsB ? pair(weekSell, sumMinor(rowsB, "weekMinor")) : null;
  const monthB = rowsB ? pair(monthSell, sumMinor(rowsB, "monthMinor")) : null;
  const yearB = rowsB ? pair(yearSell, sumMinor(rowsB, "yearMinor")) : null;
  const cashA = pair(cashBefore, cashAfterA);
  const cashB = rowsB ? pair(cashBefore, cashAfterB) : null;
  const line = (
    label: string,
    sell: string,
    afterA: string,
    deltaA: string,
    afterB?: string,
    deltaB?: string,
  ) => (
    <tr key={label}>
      <th scope="row">{label}</th>
      <td>{sell}</td>
      <td>{afterA}</td>
      <td>{deltaA}</td>
      {rowsB ? <td>{afterB ?? ""}</td> : null}
      {rowsB ? <td>{deltaB ?? ""}</td> : null}
    </tr>
  );
  return (
    <section aria-label="Scenario delta" className="plan-sheet">
      <h3>Changes</h3>
      <table>
        <thead>
          <tr>
            <th scope="col">Change</th>
            <th scope="col">Sell</th>
            <th scope="col">Scenario A</th>
            <th scope="col">Delta</th>
            {rowsB ? <th scope="col">Scenario B</th> : null}
            {rowsB ? <th scope="col">Delta</th> : null}
          </tr>
        </thead>
        <tbody>
          {line(
            "Investment characteristic",
            sellTypes,
            typesA,
            characteristicDelta(sellTypes, typesA),
            typesB,
            rowsB ? characteristicDelta(sellTypes, typesB) : "",
          )}
          {TIERS.map((tier) =>
            line(
              `${tier} allocation`,
              pct(sellMix[tier], sellMixTotal),
              pct(mixA[tier], mixTotalA),
              points(sellMix[tier], mixA[tier], sellMixTotal, mixTotalA),
              mixB ? pct(mixB[tier], mixTotalB) : "",
              mixB ? points(sellMix[tier], mixB[tier], sellMixTotal, mixTotalB) : "",
            ),
          )}
          {line("Week", weekSell == null ? "" : usd(weekSell), weekA.text, weekA.delta, weekB?.text, weekB?.delta)}
          {line("Month", monthSell == null ? "" : usd(monthSell), monthA.text, monthA.delta, monthB?.text, monthB?.delta)}
          {line("Year", yearSell == null ? "" : usd(yearSell), yearA.text, yearA.delta, yearB?.text, yearB?.delta)}
          {line(
            "Remaining cash",
            cashBefore == null ? "" : usd(cashBefore),
            cashA.text,
            cashA.delta,
            cashB?.text,
            cashB?.delta,
          )}
        </tbody>
      </table>
    </section>
  );
}
