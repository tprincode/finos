import { Fragment, useState, type CSSProperties } from "react";
import type {
  DividendPerformanceGet,
  DividendPerformanceRange,
} from "@finos/app-contracts";
import {
  formatFridayEnding,
  formatPctOfPlan,
  formatUsd,
  formatWeekNumber,
  formatWeekShort,
  pctOfPlanMinor,
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

function weekLabel(start: string, end: string): string {
  const id = weekIdContaining(start || end);
  return `${formatWeekNumber(id)} · ${formatFridayEnding(id.start)} – ${formatFridayEnding(id.end)}`;
}

function moneyOrEmpty(
  known: boolean | undefined,
  minor: number | undefined,
  scale: number,
): string {
  if (!known) return "";
  return formatUsd(minor ?? 0, scale);
}

function pctOrEmpty(pct: number | null | undefined): string {
  if (pct == null) return "";
  return formatPctOfPlan(pct);
}

/** Diverging heat for % of Plan (10000 = 100%). Miss → red/amber, beat → green. */
const PCT_HEAT_STOPS: Array<{ at: number; bg: [number, number, number]; fg: string }> = [
  { at: 5000, bg: [155, 28, 28], fg: "#fff" },
  { at: 8000, bg: [194, 65, 12], fg: "#fff" },
  { at: 9200, bg: [202, 138, 4], fg: "#1a1a1a" },
  { at: 10000, bg: [77, 124, 15], fg: "#fff" },
  { at: 11500, bg: [22, 101, 52], fg: "#fff" },
  { at: 13000, bg: [20, 83, 45], fg: "#fff" },
];

function lerp(a: number, b: number, t: number): number {
  return Math.round(a + (b - a) * t);
}

export function pctOfPlanHeat(
  pct: number | null | undefined,
): CSSProperties | undefined {
  if (pct == null) return undefined;
  const clamped = Math.min(13000, Math.max(5000, pct));
  let lo = PCT_HEAT_STOPS[0];
  let hi = PCT_HEAT_STOPS[PCT_HEAT_STOPS.length - 1];
  for (let i = 0; i < PCT_HEAT_STOPS.length - 1; i += 1) {
    if (clamped >= PCT_HEAT_STOPS[i].at && clamped <= PCT_HEAT_STOPS[i + 1].at) {
      lo = PCT_HEAT_STOPS[i];
      hi = PCT_HEAT_STOPS[i + 1];
      break;
    }
  }
  const t = (clamped - lo.at) / (hi.at - lo.at || 1);
  return {
    background: `rgb(${lerp(lo.bg[0], hi.bg[0], t)}, ${lerp(lo.bg[1], hi.bg[1], t)}, ${lerp(lo.bg[2], hi.bg[2], t)})`,
    color: t < 0.45 ? lo.fg : hi.fg,
  };
}

function periodWeekSpan(perf: DividendPerformanceGet): string | null {
  if (perf.weeks.length === 0) return null;
  const oldest = perf.weeks.reduce((a, b) => (a.start < b.start ? a : b));
  const newest = perf.weeks.reduce((a, b) => (a.end > b.end ? a : b));
  const first = weekIdContaining(oldest.start || oldest.end);
  const last = weekIdContaining(newest.end || newest.start);
  return `${formatWeekNumber(first)} · ${formatFridayEnding(first.start)} – ${formatWeekNumber(last)} · ${formatFridayEnding(last.end)}`;
}

function declPct(
  planKnown: boolean | undefined,
  plannedMinor: number | undefined,
  declarationKnown: boolean | undefined,
  declarationMinor: number | undefined,
): number | null {
  return pctOfPlanMinor(
    !!planKnown,
    plannedMinor ?? 0,
    declarationMinor ?? 0,
    !declarationKnown,
  );
}

export function weeklyDeclVsPlanOption(perf: DividendPerformanceGet) {
  const scale = perf.scale ?? 2;
  const categories = perf.weeks.map((w) => formatWeekShort(w.start || w.end));
  const declared = perf.weeks.map((w) =>
    w.declarationKnown ? dollars(w.declarationMinor ?? 0, scale) : null,
  );
  const plan = perf.weeks.map((w) =>
    w.planKnown ? dollars(w.plannedMinor, scale) : null,
  );
  const declColor = "#0d6b3d";
  const planColor = "#d9480f";
  return {
    title: {
      text: "Weekly Decl vs Plan",
      left: 0,
      textStyle: { fontSize: 13, fontWeight: 600 },
    },
    tooltip: {
      trigger: "axis",
      valueFormatter: (v: number | string) =>
        typeof v === "number" ? formatUsd(Math.round(v * 100), 2) : String(v),
    },
    legend: {
      data: [
        { name: "Decl — solid", icon: "path://M0 0H18V3H0Z" },
        { name: "Plan — dashed", icon: "path://M0 0H5V3H0ZM8 0H13V3H8ZM16 0H21V3H16Z" },
      ],
      top: 20,
    },
    grid: { left: 48, right: 12, top: 52, bottom: 28 },
    xAxis: {
      type: "category",
      data: categories,
      axisLabel: { hideOverlap: true, fontSize: 10 },
    },
    yAxis: { type: "value", scale: true, axisLabel: { fontSize: 10 } },
    series: [
      {
        name: "Decl — solid",
        type: "line",
        data: declared,
        symbol: "none",
        showSymbol: false,
        connectNulls: false,
        lineStyle: { width: 3, type: "solid", color: declColor },
        itemStyle: { color: declColor },
        emphasis: { scale: false },
      },
      {
        name: "Plan — dashed",
        type: "line",
        data: plan,
        symbol: "none",
        showSymbol: false,
        connectNulls: false,
        lineStyle: { width: 3, type: "dashed", color: planColor },
        itemStyle: { color: planColor },
        emphasis: { scale: false },
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
  const weekSort = useListSort("end", "desc");
  const [expanded, setExpanded] = useState<string | null>(null);
  if (!perf) {
    return (
      <section className="dividend-weeks" aria-label="Plan versus declaration">
        <header className="dividend-weeks-head">
          <h3>Plan vs Decl</h3>
          <p>Declared pay versus locked Plan for each Sat–Fri week in the selected period.</p>
        </header>
        <p role="status">Loading dividend weeks…</p>
      </section>
    );
  }
  const scale = perf.scale ?? 2;
  const rows = sortRows(perf.weeks, weekSort.sortKey, weekSort.sortDir, (week, key) => {
    switch (key) {
      case "week":
        return week.weekNumber ?? weekIdContaining(week.start || week.end).number;
      case "end":
        return week.end;
      case "declaration":
        return week.declarationKnown ? week.declarationMinor ?? 0 : null;
      case "plan":
        return week.planKnown ? week.plannedMinor : null;
      case "pct":
        return declPct(
          week.planKnown,
          week.plannedMinor,
          week.declarationKnown,
          week.declarationMinor,
        );
      default:
        return week.end;
    }
  });
  const summary = perf.summary;
  const declaredWeeks = perf.weeks.filter((w) => w.declarationKnown);
  const summaryDecl = declaredWeeks.reduce(
    (s, w) => s + (w.declarationMinor ?? 0),
    0,
  );
  const summaryPct = declPct(
    summary.planKnown,
    summary.plannedMinor,
    declaredWeeks.length > 0,
    summaryDecl,
  );
  const rangeLabel =
    RANGE_OPTIONS.find((opt) => opt.value === range)?.label ?? range;
  const weekSpan = periodWeekSpan(perf);
  return (
    <section className="dividend-weeks" aria-label="Plan versus declaration">
      <header className="dividend-weeks-head">
        <h3>Plan vs Decl</h3>
        <p>Declared pay versus locked Plan for each Sat–Fri week in the selected period.</p>
      </header>
      <div className="dividend-weeks-period">
        <label className="trends-period-label">
          Period
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
        <p className="dividend-weeks-period-caption" aria-label="Weeks in selected period">
          {weekSpan
            ? `${rangeLabel} · ${weekSpan}`
            : `${rangeLabel} · no weeks in this period`}
        </p>
      </div>
      <table
        className="dividend-weeks-summary-table"
        aria-label="Plan versus declaration for selected period"
      >
        <thead>
          <tr>
            <th>Selected weeks</th>
            <th className="numeric">Plan</th>
            <th className="numeric">Decl</th>
            <th className="numeric">% of Plan</th>
            <th className="numeric">Weeks</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <td>
              {weekSpan ? `${rangeLabel} · ${weekSpan}` : rangeLabel}
            </td>
            <td className="numeric">
              {summary.planKnown ? formatUsd(summary.plannedMinor, scale) : ""}
            </td>
            <td className="numeric">
              {declaredWeeks.length > 0 ? formatUsd(summaryDecl, scale) : ""}
            </td>
            <td className="numeric pct-heat" style={pctOfPlanHeat(summaryPct)}>
              {pctOrEmpty(summaryPct)}
            </td>
            <td className="numeric">{summary.weekCount}</td>
          </tr>
        </tbody>
      </table>
      <h4 className="dividend-weeks-weeks-label">Weeks</h4>
      {rows.length === 0 ? (
        <p role="status">No past dividend weeks in this range.</p>
      ) : (
        <div className="table-wrap">
          <table aria-label="Dividend week table">
            <thead>
              <tr>
                <th></th>
                {sortHead(weekSort, "Week", "week")}
                {sortHead(weekSort, "Plan", "plan", true)}
                {sortHead(weekSort, "Decl", "declaration", true)}
                {sortHead(weekSort, "% of Plan", "pct", true)}
              </tr>
            </thead>
            <tbody>
              {rows.map((week) => {
                const open = expanded === week.end;
                const label = weekLabel(week.start, week.end);
                const rowPct = declPct(
                  week.planKnown,
                  week.plannedMinor,
                  week.declarationKnown,
                  week.declarationMinor,
                );
                return (
                  <Fragment key={week.end}>
                    <tr>
                      <td>
                        <button
                          type="button"
                          aria-label={
                            open ? `Collapse ${label}` : `Expand ${label}`
                          }
                          aria-expanded={open}
                          onClick={() =>
                            setExpanded(open ? null : week.end)
                          }
                        >
                          {open ? "−" : "+"}
                        </button>
                      </td>
                      <td>{label}</td>
                      <td className="numeric">
                        {moneyOrEmpty(week.planKnown, week.plannedMinor, week.scale)}
                      </td>
                      <td className="numeric">
                        {moneyOrEmpty(
                          week.declarationKnown,
                          week.declarationMinor,
                          week.scale,
                        )}
                      </td>
                      <td className="numeric pct-heat" style={pctOfPlanHeat(rowPct)}>
                        {pctOrEmpty(rowPct)}
                      </td>
                    </tr>
                    {open ? (
                      <tr>
                        <td colSpan={5}>
                          <table aria-label={`Positions for ${label}`}>
                            <thead>
                              <tr>
                                <th>Symbol</th>
                                <th className="numeric">Plan</th>
                                <th className="numeric">Decl</th>
                                <th className="numeric">% of Plan</th>
                              </tr>
                            </thead>
                            <tbody>
                              {week.positions.map((p) => {
                                const posPct = declPct(
                                  p.planKnown,
                                  p.plannedMinor,
                                  p.declarationKnown,
                                  p.declarationMinor,
                                );
                                return (
                                  <tr key={p.symbol}>
                                    <td>{p.symbol}</td>
                                    <td className="numeric">
                                      {moneyOrEmpty(p.planKnown, p.plannedMinor, p.scale)}
                                    </td>
                                    <td className="numeric">
                                      {moneyOrEmpty(
                                        p.declarationKnown,
                                        p.declarationMinor,
                                        p.scale,
                                      )}
                                    </td>
                                    <td
                                      className="numeric pct-heat"
                                      style={pctOfPlanHeat(posPct)}
                                    >
                                      {pctOrEmpty(posPct)}
                                    </td>
                                  </tr>
                                );
                              })}
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
      )}
      <p className="pct-heat-legend" aria-hidden="true">
        <span>Miss</span>
        <span className="pct-heat-ramp" />
        <span>Exceed</span>
      </p>
    </section>
  );
}
