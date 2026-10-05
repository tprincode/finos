import { useMemo, useState } from "react";
import ReactECharts from "echarts-for-react";
import { ACCOUNT_CASH_SYMBOL } from "@finos/app-contracts";
import type {
  AccountValueHomeGet,
  DividendPerformanceGet,
  DividendPlanHomeGet,
  HoldingsGet,
  RiskValueHome,
  TrendsWeekPoint,
} from "@finos/app-contracts";
import { formatUsd, formatWeekShort, weekIdContaining } from "@finos/ui-components";
import {
  plannedWeekIncomeMinor,
  reportedWeekIncomeMinor,
  type TrendIncomePoint,
} from "../cash/CashWeekDesk";
import {
  DEFAULT_GRAPH_PERIOD,
  GRAPH_PERIOD_OPTIONS,
  graphPeriodStartIso,
  type GraphPeriod,
} from "./graphPeriod";
import { DefaultTick, initialChartDefault } from "./chartDefault";
import { weeklyDeclVsPlanOption } from "./DividendWeeks";
import {
  accountChartOption,
  colorFor,
  filterSeries,
  LiveByRiskCharts,
} from "./HomeAccountCharts";
import { atlasChartExtras } from "../screen-atlas/atlasSession";

export type TrendsCashAccountSpec = {
  title: string;
  key: keyof TrendsWeekPoint;
  color: string;
};

const PERIOD_OPTIONS = GRAPH_PERIOD_OPTIONS;

const METRIC_CHARTS: TrendsCashAccountSpec[] = [
  { title: "All Cash", key: "totalCashMinor", color: "#2a5f8f" },
  { title: "Monthly Dividends", key: "monthlyDivsMinor", color: "#8a5a12" },
  { title: "Total Fidelity & Schwab", key: "fidSchCombinedMinor", color: "#5b3d8a" },
];

/** Cash-flow account list. The Trends Accounts grid charts market value, not these keys. */
export const TRENDS_CASH_ACCOUNT_CHARTS: TrendsCashAccountSpec[] = [
  { title: "Income", key: "incomeCashMinor", color: "#4a7a3d" },
  { title: "FI Roth", key: "rothCashMinor", color: "#8a3d4a" },
  { title: "Car", key: "carCashMinor", color: "#3d6b8a" },
  { title: "Health", key: "healthCashMinor", color: "#3d7a6b" },
  { title: "Speculation", key: "speculationCashMinor", color: "#7a5a3d" },
  { title: "Account 9", key: "acct9CashMinor", color: "#5b3d8a" },
];

function latestIso(...days: Array<string | undefined>): string {
  return days
    .map((d) => (d ?? "").slice(0, 10))
    .filter((d) => d.length >= 10)
    .sort()
    .at(-1) ?? "";
}

/** Include the newest saved Friday even when that week is still open (Friday after today). */
export function filterWeeksByPeriod(
  weeks: TrendsWeekPoint[],
  period: GraphPeriod,
  asOfIso?: string,
): TrendsWeekPoint[] {
  if (weeks.length === 0) return weeks;
  const latestSaved = weeks.reduce(
    (max, w) => (w.periodEnd > max ? w.periodEnd : max),
    weeks[0].periodEnd,
  );
  if (period === "all") return weeks;
  const asOf = latestIso(asOfIso, latestSaved);
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
  const latestSaved = perf.weeks.reduce(
    (max, w) => (w.end > max ? w.end : max),
    asOfIso.slice(0, 10),
  );
  const asOf = latestIso(asOfIso, latestSaved);
  const startIso = graphPeriodStartIso(asOf, period);
  const weeks = perf.weeks.filter((w) => {
    const end = w.end.slice(0, 10);
    if (end > asOf) return false;
    if (startIso && end < startIso) return false;
    return true;
  });
  return { ...perf, weeks };
}

/** Home cash grid lots. Account 9's book name on the lot is "9". */
const OPEN_WEEK_CASH_LOTS: Array<[string, string]> = Object.entries(ACCOUNT_CASH_SYMBOL);

function cashLotCents(
  lots: HoldingsGet["lots"],
  account: string,
  symbol: string,
): number {
  return lots
    .filter(
      (lot) =>
        lot.accountName === account &&
        lot.symbol.toUpperCase() === symbol &&
        lot.remainingQuantityMinor > 0,
    )
    .reduce((sum, lot) => {
      const denom = 10 ** lot.quantityScale;
      return sum + (denom ? Math.round((lot.remainingQuantityMinor * 100) / denom) : 0);
    }, 0);
}

function paidDividendsThisMonth(
  points: TrendIncomePoint[] | undefined,
  todayIso: string,
): number {
  const month = todayIso.slice(0, 7);
  let paid = 0;
  for (const point of points ?? []) {
    const on = point.occurredOn.slice(0, 10);
    if (on.slice(0, 7) === month && on <= todayIso) paid += point.amountMinor;
  }
  return paid;
}

/** Home plan month: annual plan ÷ 12, including Health. */
export function planMonthMinor(
  plan: DividendPlanHomeGet | null | undefined,
): number | null {
  const income = plan?.total.monthlyIncomeMinor;
  const medical = plan?.total.monthlyMedicalMinor;
  if (income == null && medical == null) return null;
  return (income ?? 0) + (medical ?? 0);
}

/**
 * Each week is that week's plan, stated as what the month would be.
 * Saved Fridays keep the figure stored with the week. A saved figure that is
 * only that week's income, and the open week, use the current plan month.
 */
export function monthlyDividendWeeks(
  weeks: TrendsWeekPoint[],
  todayIso: string,
  planMonth: number | null,
): TrendsWeekPoint[] {
  const today = todayIso.slice(0, 10);
  const open = weekIdContaining(today);
  const saved = [...weeks].sort((a, b) => a.periodEnd.localeCompare(b.periodEnd));
  const history = saved.filter((week) => week.periodEnd.slice(0, 10) < open.end);
  let priorMonthly: number | null = null;
  const plotted = history.map((week) => {
    const stored = week.monthlyDivsMinor;
    const profit = week.profitMinor;
    const weekSized = profit > 0 && stored > 0 && stored <= profit;
    const monthly = weekSized
      ? planMonth ?? priorMonthly ?? stored
      : stored;
    priorMonthly = monthly;
    if (monthly === stored) return week;
    return { ...week, monthlyDivsMinor: monthly };
  });
  const closedOpen = saved.find(
    (week) => week.periodEnd.slice(0, 10) === open.end && week.closed,
  );
  if (closedOpen) return [...plotted, closedOpen];
  const monthly = planMonth ?? priorMonthly;
  if (monthly == null) return plotted;
  const template = history.at(-1);
  plotted.push({
    periodEnd: open.end,
    periodStart: open.start,
    weekYear: open.year,
    weekNumber: open.number,
    profitMinor: 0,
    monthlyDivsMinor: monthly,
    divDeltaMinor: monthly - (priorMonthly ?? monthly),
    fidelityTotalMinor: template?.fidelityTotalMinor ?? 0,
    schwabTotalMinor: template?.schwabTotalMinor ?? 0,
    fidSchCombinedMinor: template?.fidSchCombinedMinor ?? 0,
    wkToWkChangeMinor: 0,
    incomeCashMinor: template?.incomeCashMinor ?? 0,
    acct9CashMinor: template?.acct9CashMinor ?? 0,
    acct9EtfProxyMinor: template?.acct9EtfProxyMinor ?? 0,
    totalCashMinor: template?.totalCashMinor ?? 0,
    carBalanceMinor: template?.carBalanceMinor ?? null,
    incomeBalanceMinor: template?.incomeBalanceMinor ?? null,
    healthBalanceMinor: template?.healthBalanceMinor ?? null,
    rothBalanceMinor: template?.rothBalanceMinor ?? null,
    speculationBalanceMinor: template?.speculationBalanceMinor ?? null,
    closed: false,
    scale: template?.scale ?? 2,
  });
  return plotted;
}

/**
 * Chart-only Sat–Fri week for today. Not written to trends_week, so Cash
 * Management still treats an unsaved Friday as unsaved. Live cash matches the
 * home cash grid. Fidelity and Schwab match the home account totals. Monthly
 * Dividends does not use this open week.
 */
export function trendsWeeksWithOpenWeek(
  weeks: TrendsWeekPoint[],
  todayIso: string,
  holdings: HoldingsGet | null | undefined,
  accountValues: AccountValueHomeGet | null | undefined,
  points: TrendIncomePoint[] | undefined,
): TrendsWeekPoint[] {
  const today = todayIso.slice(0, 10);
  if (today.length < 10) return weeks;
  const id = weekIdContaining(today);
  const existing = weeks.find(
    (week) =>
      week.periodEnd.slice(0, 10) === id.end ||
      (week.periodStart ?? "").slice(0, 10) === id.start,
  );
  if (existing?.closed) return weeks;

  const prior = [...weeks]
    .reverse()
    .find((week) => week.periodEnd.slice(0, 10) < id.end);
  const scale = existing?.scale ?? prior?.scale ?? 2;
  const liveCash = holdings
    ? OPEN_WEEK_CASH_LOTS.map(([account, symbol]) =>
        cashLotCents(holdings.lots, account, symbol),
      )
    : null;
  const totalCash = liveCash
    ? liveCash.reduce((sum, cents) => sum + cents, 0)
    : (existing?.totalCashMinor ?? prior?.totalCashMinor ?? 0);
  const fidLive = accountValues?.fidelity.currentMinor;
  const schLive = accountValues?.schwab?.currentMinor;
  const fidelityTotal =
    fidLive != null
      ? fidLive
      : (existing?.fidelityTotalMinor ?? prior?.fidelityTotalMinor ?? 0);
  const schwabTotal =
    schLive != null
      ? schLive
      : (existing?.schwabTotalMinor ?? prior?.schwabTotalMinor ?? 0);
  const combined = fidelityTotal + schwabTotal;
  const prevCombined = prior?.fidSchCombinedMinor ?? combined;
  const monthly = paidDividendsThisMonth(points, today);
  const prevMonthly = prior?.monthlyDivsMinor ?? 0;
  const point: TrendsWeekPoint = {
    periodEnd: id.end,
    periodStart: id.start,
    weekYear: id.year,
    weekNumber: id.number,
    profitMinor: existing?.profitMinor ?? 0,
    monthlyDivsMinor: monthly,
    divDeltaMinor: monthly - prevMonthly,
    fidelityTotalMinor: fidelityTotal,
    schwabTotalMinor: schwabTotal,
    fidSchCombinedMinor: combined,
    wkToWkChangeMinor: combined - prevCombined,
    incomeCashMinor: liveCash ? liveCash[0] : (existing?.incomeCashMinor ?? prior?.incomeCashMinor ?? 0),
    acct9CashMinor: liveCash ? liveCash[5] : (existing?.acct9CashMinor ?? prior?.acct9CashMinor ?? 0),
    acct9EtfProxyMinor: liveCash ? 0 : (existing?.acct9EtfProxyMinor ?? 0),
    totalCashMinor: totalCash,
    carBalanceMinor: existing?.carBalanceMinor ?? prior?.carBalanceMinor ?? null,
    incomeBalanceMinor: existing?.incomeBalanceMinor ?? prior?.incomeBalanceMinor ?? null,
    healthBalanceMinor: existing?.healthBalanceMinor ?? prior?.healthBalanceMinor ?? null,
    rothBalanceMinor: existing?.rothBalanceMinor ?? prior?.rothBalanceMinor ?? null,
    speculationBalanceMinor:
      existing?.speculationBalanceMinor ?? prior?.speculationBalanceMinor ?? null,
    carCashMinor: liveCash ? liveCash[4] : (existing?.carCashMinor ?? prior?.carCashMinor),
    healthCashMinor: liveCash ? liveCash[3] : (existing?.healthCashMinor ?? prior?.healthCashMinor),
    rothCashMinor: liveCash ? liveCash[1] : (existing?.rothCashMinor ?? prior?.rothCashMinor),
    speculationCashMinor: liveCash
      ? liveCash[2]
      : (existing?.speculationCashMinor ?? prior?.speculationCashMinor),
    fidelityWkChangeMinor: fidelityTotal - (prior?.fidelityTotalMinor ?? fidelityTotal),
    schwabWkChangeMinor: schwabTotal - (prior?.schwabTotalMinor ?? schwabTotal),
    closed: false,
    scale,
  };
  const rest = existing ? weeks.filter((week) => week !== existing) : weeks;
  return [...rest, point].sort((a, b) => a.periodEnd.localeCompare(b.periodEnd));
}

function plannedWeekIncomeValues(
  weeks: TrendsWeekPoint[],
  perf: DividendPerformanceGet | null | undefined,
): (number | null)[] {
  return weeks.map((w) => {
    const id = weekIdContaining(w.periodStart || w.periodEnd);
    const minor = plannedWeekIncomeMinor(id.start, id.end, perf);
    if (minor == null) return null;
    return minor / 10 ** (w.scale ?? 2);
  });
}

function reportedWeekIncomeValues(
  weeks: TrendsWeekPoint[],
  points: TrendIncomePoint[] | undefined,
): (number | null)[] {
  return weeks.map((w) => {
    const id = weekIdContaining(w.periodStart || w.periodEnd);
    const minor = reportedWeekIncomeMinor(id.start, id.end, points);
    if (minor == null) return null;
    return minor / 10 ** (w.scale ?? 2);
  });
}

export function seriesValues(weeks: TrendsWeekPoint[], key: keyof TrendsWeekPoint): (number | null)[] {
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

export function chartOptionFromValues(
  title: string,
  weeks: TrendsWeekPoint[],
  data: (number | null)[],
  color: string,
  showTitle = true,
) {
  const categories = weeks.map((w) => formatWeekShort(w.periodStart || w.periodEnd));
  const trend = linearTrend(data);
  return {
    ...atlasChartExtras(),
    title: showTitle
      ? { text: title, left: 0, textStyle: { fontSize: 13, fontWeight: 600 } }
      : { show: false },
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

const DIVIDEND_MONTH_LABELS = [
  "Jan",
  "Feb",
  "Mar",
  "Apr",
  "May",
  "Jun",
  "Jul",
  "Aug",
  "Sep",
  "Oct",
  "Nov",
  "Dec",
];

const DIVIDEND_YEAR_COLORS = ["#5b7c99", "#1b6b4a"];

export type DividendYearMonthCompare = {
  years: number[];
  /** Dollars. Null when that calendar month has no paid dividend. */
  series: Array<Array<number | null>>;
  totalsMinor: number[];
};

/** Two latest calendar years of paid dividends, grouped by month. */
export function dividendYearMonthCompare(
  points: Array<{ occurredOn: string; amountMinor: number }> | undefined,
  scale = 2,
): DividendYearMonthCompare | null {
  if (!points || points.length === 0) return null;
  const buckets = new Map<string, number>();
  for (const point of points) {
    const day = (point.occurredOn ?? "").slice(0, 10);
    if (day.length < 7) continue;
    const key = day.slice(0, 7);
    buckets.set(key, (buckets.get(key) ?? 0) + point.amountMinor);
  }
  const years = [...new Set([...buckets.keys()].map((key) => Number(key.slice(0, 4))))]
    .filter((year) => Number.isFinite(year))
    .sort((a, b) => b - a)
    .slice(0, 2)
    .sort((a, b) => a - b);
  if (years.length === 0) return null;
  const divisor = 10 ** scale;
  const series = years.map((year) =>
    DIVIDEND_MONTH_LABELS.map((_, monthIndex) => {
      const key = `${year}-${String(monthIndex + 1).padStart(2, "0")}`;
      const minor = buckets.get(key);
      if (minor == null) return null;
      return minor / divisor;
    }),
  );
  const totalsMinor = years.map((year) => {
    let sum = 0;
    for (let month = 1; month <= 12; month += 1) {
      const key = `${year}-${String(month).padStart(2, "0")}`;
      sum += buckets.get(key) ?? 0;
    }
    return sum;
  });
  return { years, series, totalsMinor };
}

export function dividendYearCompareOption(compare: DividendYearMonthCompare) {
  return {
    legend: {
      top: 0,
      left: 0,
      itemWidth: 12,
      itemHeight: 8,
      textStyle: { fontSize: 12 },
    },
    tooltip: {
      trigger: "axis",
      valueFormatter: (value: unknown) =>
        typeof value === "number" && Number.isFinite(value)
          ? formatUsd(Math.round(value * 100), 2)
          : "—",
    },
    grid: { left: 64, right: 16, top: 32, bottom: 28 },
    xAxis: {
      type: "category",
      data: DIVIDEND_MONTH_LABELS,
      axisLabel: { fontSize: 11 },
    },
    yAxis: { type: "value", axisLabel: { fontSize: 10 } },
    series: compare.years.map((year, index) => ({
      name: String(year),
      type: "bar",
      data: compare.series[index],
      itemStyle: { color: DIVIDEND_YEAR_COLORS[index % DIVIDEND_YEAR_COLORS.length] },
      barMaxWidth: 22,
      barGap: "30%",
    })),
  };
}

function DividendYearCompareChart({ points }: { points?: TrendIncomePoint[] }) {
  const compare = useMemo(() => dividendYearMonthCompare(points), [points]);
  if (!compare) return null;
  const caption = compare.years
    .map((year, index) => `${year} ${formatUsd(compare.totalsMinor[index], 2)}`)
    .join(". ");
  return (
    <>
      <h3 className="trends-account-heading">Dividends paid by month</h3>
      <div className="trends-chart-card dividend-year-compare" aria-label="Dividends paid by month">
        <ReactECharts
          option={dividendYearCompareOption(compare)}
          style={{ height: 300, width: "100%" }}
          opts={{ renderer: "canvas" }}
          notMerge
          lazyUpdate
        />
        <p className="trends-period-caption">Dividend only. {caption}.</p>
      </div>
    </>
  );
}

export function TrendsChartsPanel({
  weeks,
  points,
  dividendPerf,
  note,
  error,
  missingRequired,
  accountValues,
  holdings,
  dividendPlan,
  risk,
  asOf,
  onGraphPeriodChange,
}: {
  weeks: TrendsWeekPoint[] | null | undefined;
  points?: TrendIncomePoint[];
  dividendPerf?: DividendPerformanceGet | null;
  note?: string;
  error?: string | null;
  missingRequired?: string[];
  accountValues?: AccountValueHomeGet | null;
  holdings?: HoldingsGet | null;
  dividendPlan?: DividendPlanHomeGet | null;
  risk?: RiskValueHome | null;
  asOf?: string;
  onGraphPeriodChange?: (period: GraphPeriod) => void;
}) {
  const [period, setPeriod] = useState<GraphPeriod>(() =>
    initialChartDefault(
      "trends-period",
      DEFAULT_GRAPH_PERIOD,
      GRAPH_PERIOD_OPTIONS.map((opt) => opt.value),
    ),
  );
  const todayIso = new Date().toISOString().slice(0, 10);
  const chartAsOf = latestIso(asOf, todayIso);
  const setGraphPeriod = (next: GraphPeriod) => {
    setPeriod(next);
    onGraphPeriodChange?.(next);
  };
  const chartWeeks = useMemo(
    () =>
      weeks
        ? trendsWeeksWithOpenWeek(weeks, todayIso, holdings, accountValues, points)
        : [],
    [weeks, todayIso, holdings, accountValues, points],
  );
  const visible = useMemo(
    () =>
      chartWeeks.length > 0 ? filterWeeksByPeriod(chartWeeks, period, chartAsOf) : [],
    [chartWeeks, period, chartAsOf],
  );
  const monthlyVisible = useMemo(() => {
    const series = monthlyDividendWeeks(weeks ?? [], todayIso, planMonthMinor(dividendPlan));
    return series.length > 0 ? filterWeeksByPeriod(series, period, chartAsOf) : [];
  }, [weeks, todayIso, dividendPlan, period, chartAsOf]);
  const chartPerf = useMemo(
    () => filterPerfByPeriod(dividendPerf, period, chartAsOf),
    [dividendPerf, period, chartAsOf],
  );
  const accountCharts = useMemo(() => {
    if (!accountValues) return [];
    return accountValues.accounts
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
          <span className="chart-default-choice">
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
            <DefaultTick storageKey="trends-period" value={period} />
          </span>
        </label>
        <p className="trends-period-caption">
          Sat–Fri weeks ({rangeLabel}; {visible.length} of {chartWeeks.length}). {note}
        </p>
      </div>
      {visible.length === 0 && !chartPerf?.weeks.length ? (
        <p role="status">No weeks fall in the selected graphing period.</p>
      ) : (
        <>
          {chartPerf && chartPerf.weeks.length > 0 ? (
            <div
              className="trends-chart-card dividend-weeks-chart"
              aria-label="Declared vs Plan"
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
            <div className="trends-chart-card" aria-label="Planned weekly income">
              <ReactECharts
                option={chartOptionFromValues(
                  "Planned weekly income",
                  visible,
                  plannedWeekIncomeValues(visible, chartPerf),
                  "#1b6b4a",
                )}
                style={{ height: 220, width: "100%" }}
                opts={{ renderer: "canvas" }}
                notMerge
                lazyUpdate
              />
            </div>
            <div className="trends-chart-card" aria-label="Reported weekly income">
              <ReactECharts
                option={chartOptionFromValues(
                  "Reported weekly income",
                  visible,
                  reportedWeekIncomeValues(visible, points),
                  "#6b3d1b",
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
                  option={chartOption(
                    spec.title,
                    spec.key === "monthlyDivsMinor" ? monthlyVisible : visible,
                    spec.key,
                    spec.color,
                  )}
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
            {accountCharts.map((series, i) => (
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
      <DividendYearCompareChart points={points} />
    </div>
  );
}
