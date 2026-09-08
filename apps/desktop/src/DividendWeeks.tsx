import { Fragment, useState } from "react";
import ReactECharts from "echarts-for-react";
import type {
  DividendPerformanceGet,
  DividendPerformanceRange,
} from "@finos/app-contracts";
import {
  formatPctOfPlan,
  formatUsd,
  formatWeekNumber,
  formatWeekShort,
  sortHead,
  sortRows,
  useListSort,
  weekIdContaining,
} from "@finos/ui-components";

const RANGE_OPTIONS: Array<{ value: DividendPerformanceRange; label: string }> = [
  { value: "30d", label: "30 days" },
  { value: "60d", label: "60 days" },
  { value: "90d", label: "90 days" },
  { value: "1m", label: "1 month" },
  { value: "2m", label: "2 months" },
  { value: "3m", label: "3 months" },
  { value: "ytd", label: "YTD" },
  { value: "all", label: "All" },
];

function dollars(minor: number, scale: number): number {
  const places = Number.isFinite(scale) ? Math.max(0, Math.trunc(scale)) : 2;
  return minor / 10 ** places;
}

function actualVsPlanOption(perf: DividendPerformanceGet) {
  const scale = perf.scale ?? 2;
  const categories = perf.weeks.map((w) => formatWeekShort(w.start || w.end));
  const actual = perf.weeks.map((w) => dollars(w.actualMinor, scale));
  const plan = perf.weeks.map((w) =>
    w.planKnown ? dollars(w.plannedMinor, scale) : null,
  );
  return {
    title: {
      text: "Weekly actual vs plan",
      left: 0,
      textStyle: { fontSize: 13, fontWeight: 600 },
    },
    tooltip: {
      trigger: "axis",
      valueFormatter: (v: number | string) =>
        typeof v === "number" ? formatUsd(Math.round(v * 100), 2) : String(v),
    },
    legend: { data: ["Actual", "Plan"], top: 20 },
    grid: { left: 48, right: 12, top: 52, bottom: 28 },
    xAxis: {
      type: "category",
      data: categories,
      axisLabel: { hideOverlap: true, fontSize: 10 },
    },
    yAxis: { type: "value", scale: true, axisLabel: { fontSize: 10 } },
    series: [
      {
        name: "Actual",
        type: "line",
        data: actual,
        showSymbol: false,
        connectNulls: false,
        lineStyle: { width: 2, color: "#1b6b4a" },
        itemStyle: { color: "#1b6b4a" },
      },
      {
        name: "Plan",
        type: "line",
        data: plan,
        showSymbol: false,
        connectNulls: false,
        lineStyle: { width: 2, color: "#2a5f8f" },
        itemStyle: { color: "#2a5f8f" },
      },
    ],
  };
}

export function DividendWeeksPanel({
  perf,
  range,
  onRangeChange,
}: {
  perf: DividendPerformanceGet | null;
  range: DividendPerformanceRange;
  onRangeChange: (range: DividendPerformanceRange) => void;
}) {
  const weekSort = useListSort("end");
  const [expanded, setExpanded] = useState<string | null>(null);
  if (!perf) {
    return <p>Loading dividend weeks…</p>;
  }
  const scale = perf.scale ?? 2;
  const rows = sortRows(perf.weeks, weekSort.sortKey, weekSort.sortDir, (week, key) => {
    switch (key) {
      case "week":
        return week.weekNumber ?? weekIdContaining(week.start || week.end).number;
      case "start":
        return week.start;
      case "end":
        return week.end;
      case "actual":
        return week.actualKnown === false ? null : week.actualMinor;
      case "declaration":
        return week.declarationKnown ? week.declarationMinor ?? 0 : null;
      case "plan":
        return week.planKnown ? week.plannedMinor : null;
      case "pct":
        return week.pctOfPlanMinor;
      default:
        return week.end;
    }
  });
  const summary = perf.summary;
  return (
    <section className="dividend-weeks" aria-label="Dividend weeks">
      <h3>Dividend weeks</h3>
      <p>
        Past Saturday–Friday weeks only. This week and future Plan stay on Income Plan. Unknown
        Plan is N/A, not 0%.
      </p>
      <div className="trends-period-bar">
        <label className="trends-period-label">
          Performance range
          <select
            aria-label="Dividend performance period"
            value={range}
            onChange={(e) => onRangeChange(e.target.value as DividendPerformanceRange)}
          >
            {RANGE_OPTIONS.map((opt) => (
              <option key={opt.value} value={opt.value}>
                {opt.label}
              </option>
            ))}
          </select>
        </label>
        <p className="trends-period-caption">
          {perf.rangeStart
            ? `${formatWeekShort(perf.rangeStart)} through last complete week before ${formatWeekShort(perf.thisWeekStart)}.`
            : `All past weeks before ${formatWeekShort(perf.thisWeekStart)}.`}
        </p>
      </div>
      <div className="dividend-weeks-summary" aria-label="Dividend performance summary">
        <span>Actual {formatUsd(summary.actualMinor, scale)}</span>
        <span>
          Plan {summary.planKnown ? formatUsd(summary.plannedMinor, scale) : "N/A"}
        </span>
        <span>
          Declaration is on each week row — missing stays N/A, never $0.
        </span>
        <span>% of Plan {formatPctOfPlan(summary.pctOfPlanMinor)}</span>
        <span>
          Avg / week{" "}
          {summary.avgWeeklyActualMinor == null
            ? "N/A"
            : formatUsd(summary.avgWeeklyActualMinor, scale)}
        </span>
        <span>
          Avg plan / week{" "}
          {summary.avgWeeklyPlanMinor == null
            ? "N/A"
            : formatUsd(summary.avgWeeklyPlanMinor, scale)}
        </span>
        <span>
          {summary.weekCount} weeks ({summary.knownPlanWeekCount} with Plan)
        </span>
      </div>
      {rows.length === 0 ? (
        <p role="status">No past dividend weeks in this range.</p>
      ) : (
        <>
          <div className="table-wrap">
            <table aria-label="Dividend week table">
              <thead>
                <tr>
                  <th></th>
                  {sortHead(weekSort, "Week", "week")}
                  {sortHead(weekSort, "Week start", "start")}
                  {sortHead(weekSort, "Week ending", "end")}
                  {sortHead(weekSort, "Plan", "plan", true)}
                  {sortHead(weekSort, "Declaration", "declaration", true)}
                  {sortHead(weekSort, "Actual", "actual", true)}
                  {sortHead(weekSort, "% of Plan", "pct", true)}
                </tr>
              </thead>
              <tbody>
                {rows.map((week) => {
                  const open = expanded === week.end;
                  return (
                    <Fragment key={week.end}>
                      <tr>
                        <td>
                          <button
                            type="button"
                            aria-label={
                              open
                                ? `Collapse ${formatWeekShort(week.start || week.end)}`
                                : `Expand ${formatWeekShort(week.start || week.end)}`
                            }
                            aria-expanded={open}
                            onClick={() =>
                              setExpanded(open ? null : week.end)
                            }
                          >
                            {open ? "Hide" : "Show"}
                          </button>
                        </td>
                        <td>{formatWeekNumber(weekIdContaining(week.start || week.end))}</td>
                        <td>{week.start}</td>
                        <td>{week.end}</td>
                        <td className="numeric">
                          {week.planKnown
                            ? formatUsd(week.plannedMinor, week.scale)
                            : "N/A"}
                        </td>
                        <td className="numeric">
                          {week.declarationKnown
                            ? formatUsd(week.declarationMinor ?? 0, week.scale)
                            : "N/A"}
                        </td>
                        <td className="numeric">
                          {week.actualKnown === false
                            ? "N/A"
                            : formatUsd(week.actualMinor, week.scale)}
                        </td>
                        <td className="numeric">
                          {formatPctOfPlan(week.pctOfPlanMinor)}
                        </td>
                      </tr>
                      {open ? (
                        <tr>
                          <td colSpan={8}>
                            <table aria-label={`Positions for ${formatWeekShort(week.start || week.end)}`}>
                              <thead>
                                <tr>
                                  <th>Symbol</th>
                                  <th className="numeric">Plan</th>
                                  <th className="numeric">Declaration</th>
                                  <th className="numeric">Actual</th>
                                  <th className="numeric">% of Plan</th>
                                </tr>
                              </thead>
                              <tbody>
                                {week.positions.map((p) => (
                                  <tr key={p.symbol}>
                                    <td>{p.symbol}</td>
                                    <td className="numeric">
                                      {p.planKnown
                                        ? formatUsd(p.plannedMinor, p.scale)
                                        : "N/A"}
                                    </td>
                                    <td className="numeric">
                                      {p.declarationKnown
                                        ? formatUsd(p.declarationMinor ?? 0, p.scale)
                                        : "N/A"}
                                    </td>
                                    <td className="numeric">
                                      {p.actualKnown === false
                                        ? "N/A"
                                        : formatUsd(p.actualMinor, p.scale)}
                                    </td>
                                    <td className="numeric">
                                      {formatPctOfPlan(p.pctOfPlanMinor)}
                                    </td>
                                  </tr>
                                ))}
                              </tbody>
                            </table>
                          </td>
                        </tr>
                      ) : null}
                    </Fragment>
                  );
                })}
              </tbody>
            </table>
          </div>
          <div
            className="trends-chart-card dividend-weeks-chart"
            aria-label="Actual versus plan"
          >
            <ReactECharts
              option={actualVsPlanOption(perf)}
              style={{ height: 280, width: "100%" }}
              opts={{ renderer: "canvas" }}
              notMerge
              lazyUpdate
            />
          </div>
        </>
      )}
    </section>
  );
}
