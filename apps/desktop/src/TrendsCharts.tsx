import { useMemo, useState } from "react";
import ReactECharts from "echarts-for-react";
import type { TrendsWeekPoint } from "@finos/app-contracts";
import { formatUsd } from "@finos/ui-components";

type SeriesSpec = {
  title: string;
  key: keyof TrendsWeekPoint;
  color: string;
};

type GraphPeriod = "3m" | "6m" | "12m" | "ytd" | "all";

const PERIOD_OPTIONS: Array<{ value: GraphPeriod; label: string }> = [
  { value: "3m", label: "3 months" },
  { value: "6m", label: "6 months" },
  { value: "12m", label: "12 months" },
  { value: "ytd", label: "YTD" },
  { value: "all", label: "All data" },
];

const CHARTS: SeriesSpec[] = [
  { title: "Weekly Gross", key: "profitMinor", color: "#1b6b4a" },
  { title: "Cash", key: "totalCashMinor", color: "#2a5f8f" },
  { title: "Monthly Dividends", key: "monthlyDivsMinor", color: "#8a5a12" },
  { title: "Total Fidelity & Schwab", key: "fidSchCombinedMinor", color: "#5b3d8a" },
  { title: "Roth", key: "rothBalanceMinor", color: "#8a3d4a" },
  { title: "Car", key: "carBalanceMinor", color: "#3d6b8a" },
  { title: "Income", key: "incomeBalanceMinor", color: "#4a7a3d" },
  { title: "Speculation", key: "speculationBalanceMinor", color: "#7a5a3d" },
];

export type TrendsOverview = {
  fidSchCombinedMinor?: number | null;
  wkToWkChangeMinor?: number | null;
  profitMinor?: number | null;
  monthlyDivsMinor?: number | null;
  divDeltaMinor?: number | null;
  totalCashMinor?: number | null;
  scale: number;
};

export type TrendsDistribution = {
  grossMinor: number;
  lines: Array<{
    activityType: string;
    accountName: string;
    amountMinor: number;
    occurredOn: string;
    scale: number;
  }>;
  scale: number;
};

export type TrendsTaxMonitor = {
  federalWithholdingMinor: number;
  projectedLiabilityMinor?: number | null;
  gapMinor?: number | null;
  warning: boolean;
  acaThresholdMinor?: number | null;
  acaCoverageYear?: number | null;
  note: string;
  scale: number;
};

function parseIsoDate(iso: string): Date | null {
  if (!iso || iso.length < 10) return null;
  const d = new Date(`${iso.slice(0, 10)}T12:00:00`);
  return Number.isNaN(d.getTime()) ? null : d;
}

function addMonths(date: Date, months: number): Date {
  const d = new Date(date.getTime());
  const day = d.getDate();
  d.setMonth(d.getMonth() + months);
  if (d.getDate() < day) d.setDate(0);
  return d;
}

function toIsoDate(d: Date): string {
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${y}-${m}-${day}`;
}

export function filterWeeksByPeriod(
  weeks: TrendsWeekPoint[],
  period: GraphPeriod,
): TrendsWeekPoint[] {
  if (period === "all" || weeks.length === 0) return weeks;
  const asOfIso = weeks[weeks.length - 1]?.periodEnd ?? "";
  const asOf = parseIsoDate(asOfIso);
  if (!asOf) return weeks;
  let startIso: string;
  if (period === "ytd") {
    startIso = `${asOf.getFullYear()}-01-01`;
  } else {
    const months = period === "3m" ? 3 : period === "6m" ? 6 : 12;
    startIso = toIsoDate(addMonths(asOf, -months));
  }
  return weeks.filter((w) => w.periodEnd >= startIso && w.periodEnd <= asOfIso);
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

function chartOption(title: string, weeks: TrendsWeekPoint[], key: keyof TrendsWeekPoint, color: string) {
  const categories = weeks.map((w) => w.periodEnd);
  const data = seriesValues(weeks, key);
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

export function TrendsChartsPanel({
  weeks,
  note,
  error,
  overview,
  distributions,
  taxMonitor,
  missingRequired,
}: {
  weeks: TrendsWeekPoint[] | null | undefined;
  note?: string;
  error?: string | null;
  overview?: TrendsOverview | null;
  distributions?: TrendsDistribution | null;
  taxMonitor?: TrendsTaxMonitor | null;
  missingRequired?: string[];
}) {
  const [period, setPeriod] = useState<GraphPeriod>("12m");
  const visible = useMemo(
    () => (weeks && weeks.length > 0 ? filterWeeksByPeriod(weeks, period) : []),
    [weeks, period],
  );

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
      <div role="status">
        <p>No weekly Trends snapshots yet. Use weekly capture above or run data-seed.</p>
      </div>
    );
  }

  const scale = overview?.scale ?? visible[0]?.scale ?? 2;
  const rangeLabel =
    visible.length > 0
      ? `${visible[0].periodEnd} to ${visible[visible.length - 1].periodEnd}`
      : "no weeks in this period";

  return (
    <div className="trends-charts" aria-label="Trends weekly charts">
      {missingRequired && missingRequired.length > 0 ? (
        <p className="trends-quality" role="status" aria-label="Trends data quality">
          Data quality: {missingRequired.join(", ")}
        </p>
      ) : null}
      {overview ? (
        <div className="trends-overview" aria-label="Trends overview">
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
      <div className="trends-period-bar">
        <label className="trends-period-label">
          Graphing period
          <select
            aria-label="Trends graphing period"
            value={period}
            onChange={(e) => setPeriod(e.target.value as GraphPeriod)}
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
      {visible.length === 0 ? (
        <p role="status">No weeks fall in the selected graphing period.</p>
      ) : (
        <div className="trends-chart-grid">
          {CHARTS.map((spec) => (
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
      )}
      {distributions ? (
        <section aria-label="Trends distribution history">
          <h3>Distributions (YTD)</h3>
          <p>Gross {formatUsd(distributions.grossMinor, distributions.scale)}. Non-ROI ledger only.</p>
          <div className="table-wrap">
            <table>
              <thead>
                <tr>
                  <th>Date</th>
                  <th>Type</th>
                  <th>Account</th>
                  <th>Amount</th>
                </tr>
              </thead>
              <tbody>
                {distributions.lines.slice(-40).map((line, i) => (
                  <tr key={`${line.occurredOn}-${line.activityType}-${i}`}>
                    <td>{line.occurredOn}</td>
                    <td>{line.activityType}</td>
                    <td>{line.accountName}</td>
                    <td className="numeric">{formatUsd(line.amountMinor, line.scale)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </section>
      ) : null}
      {taxMonitor ? (
        <section aria-label="Trends tax and ACA monitor">
          <h3>Tax / ACA monitor</h3>
          <p>{taxMonitor.note}</p>
          <p>
            YTD included {formatUsd(taxMonitor.projectedLiabilityMinor ?? 0, taxMonitor.scale)}
            {taxMonitor.acaThresholdMinor != null
              ? `; ACA ${taxMonitor.acaCoverageYear} threshold ${formatUsd(taxMonitor.acaThresholdMinor, taxMonitor.scale)}`
              : ""}
            {taxMonitor.warning ? " — warning: above threshold" : ""}
          </p>
        </section>
      ) : null}
    </div>
  );
}
