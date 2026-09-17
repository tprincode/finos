import { useMemo, useState } from "react";
import ReactECharts from "echarts-for-react";
import type {
  AccountValueHomeGet,
  DividendPerformanceGet,
  RiskValueHome,
  TrendsWeekPoint,
} from "@finos/app-contracts";
import { formatUsd, formatWeekShort, weekIdContaining } from "@finos/ui-components";
import {
  weekIncomeMinor,
  type TrendIncomePoint,
} from "../cash/CashWeekDesk";
import {
  DEFAULT_GRAPH_PERIOD,
  GRAPH_PERIOD_OPTIONS,
  graphPeriodStartIso,
  type GraphPeriod,
} from "./graphPeriod";
import { weeklyDeclVsPlanOption } from "./DividendWeeks";
import {
  accountChartOption,
  colorFor,
  filterSeries,
  isTrendsPlacedAccount,
  LiveByRiskCharts,
} from "./HomeAccountCharts";

type SeriesSpec = {
  title: string;
  key: keyof TrendsWeekPoint;
  color: string;
};

const PERIOD_OPTIONS = GRAPH_PERIOD_OPTIONS;

const METRIC_CHARTS: SeriesSpec[] = [
  { title: "Cash", key: "totalCashMinor", color: "#2a5f8f" },
  { title: "Monthly Dividends", key: "monthlyDivsMinor", color: "#8a5a12" },
  { title: "Total Fidelity & Schwab", key: "fidSchCombinedMinor", color: "#5b3d8a" },
];

const ACCOUNT_CHARTS: SeriesSpec[] = [
  { title: "Roth", key: "rothBalanceMinor", color: "#8a3d4a" },
  { title: "Car", key: "carBalanceMinor", color: "#3d6b8a" },
  { title: "Income", key: "incomeBalanceMinor", color: "#4a7a3d" },
  { title: "Health", key: "healthBalanceMinor", color: "#3d7a6b" },
  { title: "Speculation", key: "speculationBalanceMinor", color: "#7a5a3d" },
];

export function filterWeeksByPeriod(
  weeks: TrendsWeekPoint[],
  period: GraphPeriod,
  asOfIso?: string,
): TrendsWeekPoint[] {
  if (period === "all" || weeks.length === 0) return weeks;
  const asOf = (asOfIso || weeks[weeks.length - 1]?.periodEnd || "").slice(0, 10);
  const startIso = graphPeriodStartIso(asOf, period);
  if (!startIso) return weeks;
  return weeks.filter((w) => w.periodEnd >= startIso && w.periodEnd <= asOf);
}

function filterPerfByPeriod(
  perf: DividendPerformanceGet | null | undefined,
  period: GraphPeriod,
  asOfIso: string,
): DividendPerformanceGet | null {
  if (!perf) return null;
  const asOf = asOfIso.slice(0, 10);
  const startIso = graphPeriodStartIso(asOf, period);
  const weeks = perf.weeks.filter((w) => {
    const end = w.end.slice(0, 10);
    if (end > asOf) return false;
    if (startIso && end < startIso) return false;
    return true;
  });
  return { ...perf, weeks };
}

function weekIncomeValues(
  weeks: TrendsWeekPoint[],
  points: TrendIncomePoint[] | undefined,
  perf: DividendPerformanceGet | null | undefined,
): (number | null)[] {
  return weeks.map((w) => {
    const id = weekIdContaining(w.periodStart || w.periodEnd);
    const minor = weekIncomeMinor(id.start, id.end, points, perf);
    if (minor == null) return null;
    return minor / 10 ** (w.scale ?? 2);
  });
}

function seriesValues(weeks: TrendsWeekPoint[], key: keyof TrendsWeekPoint): (number | null)[] {
  return weeks.map((w) => {
    const raw = w[key];
    if (raw == null || typeof raw !== "number") return null;
    return raw / 10 ** (w.scale ?? 2);
  });
}

function linearTrend(data: (number | null)[]): (number | null)[] {
  const pts: Array<{ i: number; y: number }> = [];
  data.forEach((y, i) => {
    if (y != null) pts.push({ i, y });
  });
  if (pts.length < 2) return data.map(() => null);
  const n = pts.length;
  const sumX = pts.reduce((s, p) => s + p.i, 0);
  const sumY = pts.reduce((s, p) => s + p.y, 0);
  const sumXY = pts.reduce((s, p) => s + p.i * p.y, 0);
  const sumXX = pts.reduce((s, p) => s + p.i * p.i, 0);
  const den = n * sumXX - sumX * sumX;
  if (den === 0) return data.map(() => null);
  const slope = (n * sumXY - sumX * sumY) / den;
  const intercept = (sumY - slope * sumX) / n;
  return data.map((_, i) => intercept + slope * i);
}

function chartOptionFromValues(
  title: string,
  weeks: TrendsWeekPoint[],
  data: (number | null)[],
  color: string,
) {
  const categories = weeks.map((w) => formatWeekShort(w.periodStart || w.periodEnd));
  const trend = linearTrend(data);
  return {
    title: { text: title, left: 0, textStyle: { fontSize: 13, fontWeight: 600 } },
    tooltip: {
      trigger: "axis",
      valueFormatter: (v: number | string) =>
        typeof v === "number" ? formatUsd(Math.round(v * 100), 2) : String(v),
    },
    grid: { left: 48, right: 12, top: 36, bottom: 28 },
    xAxis: { type: "category", data: categories, axisLabel: { hideOverlap: true, fontSize: 10 } },
    yAxis: { type: "value", scale: true, axisLabel: { fontSize: 10 } },
    series: [
      {
        name: title,
        type: "line",
        data,
        showSymbol: false,
        connectNulls: false,
        lineStyle: { width: 2, color },
        itemStyle: { color },
      },
      {
        name: "Trend",
        type: "line",
        data: trend,
        showSymbol: false,
        lineStyle: { width: 1, type: "dashed", color: "#888" },
      },
    ],
  };
}

function chartOption(title: string, weeks: TrendsWeekPoint[], key: keyof TrendsWeekPoint, color: string) {
  return chartOptionFromValues(title, weeks, seriesValues(weeks, key), color);
}

export function TrendsChartsPanel({
  weeks,
  points,
  dividendPerf,
  note,
  error,
  missingRequired,
  accountValues,
  risk,
  asOf: _asOf,
  onGraphPeriodChange,
}: {
  weeks: TrendsWeekPoint[] | null | undefined;
  points?: TrendIncomePoint[];
  dividendPerf?: DividendPerformanceGet | null;
  note?: string;
  error?: string | null;
  missingRequired?: string[];
  accountValues?: AccountValueHomeGet | null;
  risk?: RiskValueHome | null;
  asOf?: string;
  onGraphPeriodChange?: (period: GraphPeriod) => void;
}) {
  const [period, setPeriod] = useState<GraphPeriod>(DEFAULT_GRAPH_PERIOD);
  const todayIso = new Date().toISOString().slice(0, 10);
  const setGraphPeriod = (next: GraphPeriod) => {
    setPeriod(next);
    onGraphPeriodChange?.(next);
  };
  const visible = useMemo(
    () => (weeks && weeks.length > 0 ? filterWeeksByPeriod(weeks, period, todayIso) : []),
    [weeks, period, todayIso],
  );
  const chartPerf = useMemo(
    () => filterPerfByPeriod(dividendPerf, period, todayIso),
    [dividendPerf, period, todayIso],
  );
  const placedAccounts = useMemo(() => {
    if (!accountValues) return [];
    return accountValues.accounts
      .filter((acct) => isTrendsPlacedAccount(acct.accountName, acct.custodian))
      .map((acct) => filterSeries(acct, todayIso, period))
      .sort((a, b) => a.accountName.localeCompare(b.accountName));
  }, [accountValues, period, todayIso]);

  if (error) {
    return (
      <p role="alert">
        Trends could not load: {error}. Fully quit the app and rebuild if the host is stale.
      </p>
    );
  }
  if (weeks == null) return <p>Loading Trends…</p>;
  if (weeks.length === 0) {
    return (
      <div className="trends-charts" aria-label="Trends weekly charts">
        <p role="status">No weekly snapshots yet. Enter the week on Cash Management or run data-seed.</p>
        <div className="trends-period-bar">
          <label className="trends-period-label">
            Graphing period
            <select
              aria-label="Trends graphing period"
              value={period}
              onChange={(e) => setGraphPeriod(e.target.value as GraphPeriod)}
            >
              {PERIOD_OPTIONS.map((opt) => (
                <option key={opt.value} value={opt.value}>
                  {opt.label}
                </option>
              ))}
            </select>
          </label>
        </div>
        {chartPerf && chartPerf.weeks.length > 0 ? (
          <div
            className="trends-chart-card dividend-weeks-chart"
            aria-label="Weekly Decl vs Plan"
          >
            <ReactECharts
              option={weeklyDeclVsPlanOption(chartPerf)}
              style={{ height: 220, width: "100%" }}
              opts={{ renderer: "canvas" }}
              notMerge
              lazyUpdate
            />
          </div>
        ) : null}
        <LiveByRiskCharts
          risk={risk ?? accountValues?.risk ?? null}
          asOf={todayIso}
          period={period}
        />
      </div>
    );
  }

  const rangeLabel =
    visible.length > 0
      ? `${formatWeekShort(visible[0].periodStart || visible[0].periodEnd)} to ${formatWeekShort(visible[visible.length - 1].periodStart || visible[visible.length - 1].periodEnd)}`
      : "no weeks in this period";

  return (
    <div className="trends-charts" aria-label="Trends weekly charts">
      {missingRequired &&
      missingRequired.filter((code) => code !== "week_not_saved").length > 0 ? (
        <p className="trends-quality" role="status" aria-label="Trends data quality">
          Data quality:{" "}
          {missingRequired.filter((code) => code !== "week_not_saved").join(", ")}
        </p>
      ) : null}
      <div className="trends-period-bar">
        <label className="trends-period-label">
          Graphing period
          <select
            aria-label="Trends graphing period"
            value={period}
            onChange={(e) => setGraphPeriod(e.target.value as GraphPeriod)}
          >
            {PERIOD_OPTIONS.map((opt) => (
              <option key={opt.value} value={opt.value}>
                {opt.label}
              </option>
            ))}
          </select>
        </label>
        <p className="trends-period-caption">
          Sat–Fri weeks ({rangeLabel}; {visible.length} of {weeks.length}). {note}
        </p>
      </div>
      {visible.length === 0 && !chartPerf?.weeks.length ? (
        <p role="status">No weeks fall in the selected graphing period.</p>
      ) : (
        <>
          {chartPerf && chartPerf.weeks.length > 0 ? (
            <div
              className="trends-chart-card dividend-weeks-chart"
              aria-label="Weekly Decl vs Plan"
            >
              <ReactECharts
                option={weeklyDeclVsPlanOption(chartPerf)}
                style={{ height: 220, width: "100%" }}
                opts={{ renderer: "canvas" }}
                notMerge
                lazyUpdate
              />
            </div>
          ) : null}
          <div className="trends-chart-grid">
            <div className="trends-chart-card" aria-label="Week income">
              <ReactECharts
                option={chartOptionFromValues(
                  "Week income",
                  visible,
                  weekIncomeValues(visible, points, chartPerf),
                  "#1b6b4a",
                )}
                style={{ height: 220, width: "100%" }}
                opts={{ renderer: "canvas" }}
                notMerge
                lazyUpdate
              />
            </div>
            {METRIC_CHARTS.map((spec) => (
              <div key={spec.key} className="trends-chart-card" aria-label={spec.title}>
                <ReactECharts
                  option={chartOption(spec.title, visible, spec.key, spec.color)}
                  style={{ height: 220, width: "100%" }}
                  opts={{ renderer: "canvas" }}
                  notMerge
                  lazyUpdate
                />
              </div>
            ))}
          </div>
          <h3 className="trends-account-heading">Accounts</h3>
          <div className="trends-chart-grid" aria-label="Trends account charts">
            {ACCOUNT_CHARTS.map((spec) => (
              <div key={spec.key} className="trends-chart-card" aria-label={spec.title}>
                <ReactECharts
                  option={chartOption(spec.title, visible, spec.key, spec.color)}
                  style={{ height: 220, width: "100%" }}
                  opts={{ renderer: "canvas" }}
                  notMerge
                  lazyUpdate
                />
              </div>
            ))}
            {placedAccounts.map((series, i) => (
              <div
                key={series.accountId}
                className="trends-chart-card"
                aria-label={series.accountName}
              >
                <ReactECharts
                  option={accountChartOption(
                    series,
                    colorFor(series.accountName, i, series.custodian),
                    "trends",
                  )}
                  style={{ height: 220, width: "100%" }}
                  opts={{ renderer: "canvas" }}
                  notMerge
                  lazyUpdate
                />
              </div>
            ))}
          </div>
        </>
      )}
      <LiveByRiskCharts
        risk={risk ?? accountValues?.risk ?? null}
        asOf={todayIso}
        period={period}
      />
    </div>
  );
}
