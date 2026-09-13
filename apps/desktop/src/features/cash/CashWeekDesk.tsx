import { useMemo } from "react";
import type { DividendPerformanceGet, TrendsWeekPoint } from "@finos/app-contracts";
import {
  formatFridayEnding,
  formatUsd,
  formatWeekNumber,
  saturdaysFromTo,
  weekIdContaining,
} from "@finos/ui-components";

export type TrendIncomePoint = { occurredOn: string; amountMinor: number };

export type CashWeekOverview = {
  fidSchCombinedMinor?: number | null;
  wkToWkChangeMinor?: number | null;
  profitMinor?: number | null;
  monthlyDivsMinor?: number | null;
  divDeltaMinor?: number | null;
  totalCashMinor?: number | null;
  scale: number;
};

/** Every Sat–Fri from the first saved week through today, including unsaved gaps. */
export function trendsTableSaturdays(
  weeks: TrendsWeekPoint[],
  todayIso: string,
): string[] {
  const today = todayIso.slice(0, 10);
  if (weeks.length === 0) return saturdaysFromTo(today, today);
  const first = weeks[0].periodStart || weeks[0].periodEnd;
  return saturdaysFromTo(first, today);
}

export function weekIncomeMinor(
  start: string,
  end: string,
  points: TrendIncomePoint[] | undefined,
  perf: DividendPerformanceGet | null | undefined,
): number | null {
  let paid = 0;
  let anyPaid = false;
  for (const p of points ?? []) {
    const on = p.occurredOn.slice(0, 10);
    if (on >= start && on <= end) {
      paid += p.amountMinor;
      anyPaid = true;
    }
  }
  if (anyPaid && paid !== 0) return paid;
  const row = perf?.weeks.find((w) => w.start === start || w.end === end);
  if (row?.declarationKnown && (row.declarationMinor ?? 0) !== 0) {
    return row.declarationMinor ?? 0;
  }
  return null;
}

function moneyOrBlank(
  minor: number | null | undefined,
  scale: number,
): string {
  if (minor == null) return "";
  return formatUsd(minor, scale);
}

export function CashWeekDesk({
  weeks,
  points,
  dividendPerf,
  overview,
}: {
  weeks: TrendsWeekPoint[] | null | undefined;
  points?: TrendIncomePoint[];
  dividendPerf?: DividendPerformanceGet | null;
  overview?: CashWeekOverview | null;
}) {
  const todayIso = new Date().toISOString().slice(0, 10);
  const tableSaturdays = useMemo(
    () => (weeks && weeks.length > 0 ? trendsTableSaturdays(weeks, todayIso) : []),
    [weeks, todayIso],
  );
  const savedByStart = useMemo(() => {
    const map = new Map<string, TrendsWeekPoint>();
    for (const week of weeks ?? []) {
      const start = week.periodStart || weekIdContaining(week.periodEnd).start;
      map.set(start, week);
    }
    return map;
  }, [weeks]);
  const scale = overview?.scale ?? weeks?.[0]?.scale ?? 2;

  if (weeks == null) return null;

  return (
    <section className="cash-week-desk" aria-label="Cash week desk">
      {overview ? (
        <div className="trends-overview" aria-label="Cash week overview">
          <span>FID+SCH {formatUsd(overview.fidSchCombinedMinor ?? 0, scale)}</span>
          <span>WkΔ {formatUsd(overview.wkToWkChangeMinor ?? 0, scale)}</span>
          <span>Profit {formatUsd(overview.profitMinor ?? 0, scale)}</span>
          <span>
            DIVS {formatUsd(overview.monthlyDivsMinor ?? 0, scale)} (Δ{" "}
            {formatUsd(overview.divDeltaMinor ?? 0, scale)})
          </span>
          <span>Cash {formatUsd(overview.totalCashMinor ?? 0, scale)}</span>
        </div>
      ) : null}
      {tableSaturdays.length > 0 ? (
        <div className="table-wrap trends-week-table-wrap">
          <table aria-label="Saved cash weeks">
            <thead>
              <tr>
                <th>Week</th>
                <th className="numeric">Week income</th>
                <th className="numeric">Profit</th>
                <th className="numeric">Cash</th>
                <th className="numeric">Fidelity</th>
                <th className="numeric">Schwab</th>
                <th className="numeric">Wk to wk</th>
              </tr>
            </thead>
            <tbody>
              {[...tableSaturdays].reverse().map((start) => {
                const id = weekIdContaining(start);
                const saved = savedByStart.get(id.start);
                const rowScale = saved?.scale ?? scale;
                const income = weekIncomeMinor(
                  id.start,
                  id.end,
                  points,
                  dividendPerf,
                );
                return (
                  <tr key={id.end}>
                    <td>
                      {formatWeekNumber(id)} · {formatFridayEnding(id.start)} –{" "}
                      {formatFridayEnding(id.end)}
                    </td>
                    <td className="numeric">{moneyOrBlank(income, rowScale)}</td>
                    <td className="numeric">
                      {saved ? formatUsd(saved.profitMinor, rowScale) : ""}
                    </td>
                    <td className="numeric">
                      {saved ? formatUsd(saved.totalCashMinor, rowScale) : ""}
                    </td>
                    <td className="numeric">
                      {saved ? formatUsd(saved.fidelityTotalMinor, rowScale) : ""}
                    </td>
                    <td className="numeric">
                      {saved ? formatUsd(saved.schwabTotalMinor, rowScale) : ""}
                    </td>
                    <td className="numeric">
                      {saved ? formatUsd(saved.wkToWkChangeMinor, rowScale) : ""}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      ) : null}
    </section>
  );
}
