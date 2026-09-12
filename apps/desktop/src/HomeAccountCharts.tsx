import { useEffect, useMemo, useState } from "react";
import ReactECharts from "echarts-for-react";
import type {
  AccountValueHomeGet,
  AccountValuePoint,
  AccountValueSeries,
  RiskGroupValue,
  RiskValueHome,
  RiskValuePoint,
} from "@finos/app-contracts";
import { formatUsd } from "@finos/ui-components";
import {
  GRAPH_PERIOD_OPTIONS,
  graphPeriodStartIso,
  inGraphPeriod,
  type GraphPeriod,
} from "./graphPeriod";

const COLORS = [
  "#1b6b4a",
  "#2a5f8f",
  "#8a5a12",
  "#5b3d8a",
  "#8a3d4a",
  "#3d6b8a",
  "#4a7a3d",
  "#7a5a3d",
];

const RISK_COLORS: Record<string, string> = {
  Foundation: "#1b6b4a",
  Core: "#2a5f8f",
  "Risk On": "#8a5a12",
};

function colorFor(name: string, index: number, custodian?: string): string {
  if (custodian === "Direct" || name.toLowerCase().includes("energyx")) return "#8a5a12";
  if (name.toLowerCase().includes("fidelity") || custodian === "Fidelity") return "#3b6d11";
  if (name.toLowerCase().includes("schwab") || custodian === "Schwab") return "#00a0df";
  if (custodian === "Robinhood") return "#4a7a3d";
  return COLORS[index % COLORS.length];
}

function dollars(minor: number | null | undefined, scale: number): number | null {
  if (minor == null) return null;
  return minor / 10 ** scale;
}

function filterPoints<T extends { asOf: string }>(
  points: T[] | undefined,
  asOf: string,
  period: GraphPeriod,
): T[] {
  return (points ?? []).filter((p) => inGraphPeriod(p.asOf, asOf, period));
}

function filterSeries(series: AccountValueSeries, asOf: string, period: GraphPeriod): AccountValueSeries {
  return {
    ...series,
    points: filterPoints(series.points, asOf, period),
    trendsPoints: filterPoints(series.trendsPoints, asOf, period),
  };
}

function chartOption(series: AccountValueSeries, color: string) {
  const scale = series.scale ?? 2;
  const live = series.points ?? [];
  const trends = series.trendsPoints ?? [];
  const categories = [
    ...new Set([...live.map((p) => p.asOf), ...trends.map((p) => p.asOf)]),
  ].sort();
  const liveByDay = new Map(live.map((p) => [p.asOf, p.marketValueMinor]));
  const trendsByDay = new Map(trends.map((p) => [p.asOf, p.marketValueMinor]));
  const liveData = categories.map((day) => dollars(liveByDay.get(day) ?? null, scale));
  const trendsData = categories.map((day) => dollars(trendsByDay.get(day) ?? null, scale));
  const hasTrends = trends.some((p) => p.marketValueMinor != null);
  const chartSeries: Record<string, unknown>[] = [
    {
      name: "Live",
      type: "line",
      data: liveData,
      showSymbol: true,
      symbolSize: 5,
      smooth: false,
      connectNulls: false,
      lineStyle: { width: 2.2, color },
      itemStyle: { color },
      areaStyle: {
        color: {
          type: "linear",
          x: 0,
          y: 0,
          x2: 0,
          y2: 1,
          colorStops: [
            { offset: 0, color },
            { offset: 1, color: "rgba(255,255,255,0)" },
          ],
        },
      },
    },
  ];
  if (hasTrends) {
    chartSeries.push({
      name: "Trends",
      type: "line",
      data: trendsData,
      showSymbol: true,
      symbolSize: 5,
      smooth: false,
      connectNulls: false,
      lineStyle: { width: 1.8, type: "dashed", color: "#5a6a78" },
      itemStyle: { color: "#5a6a78" },
    });
  }
  return {
    tooltip: {
      trigger: "axis",
      valueFormatter: (v: number | string) =>
        typeof v === "number" ? formatUsd(Math.round(v * 100), 2) : String(v),
    },
    legend: hasTrends
      ? {
          top: 4,
          right: 8,
          itemWidth: 10,
          itemHeight: 8,
          textStyle: { fontSize: 10 },
        }
      : undefined,
    grid: { left: 12, right: 18, top: hasTrends ? 28 : 10, bottom: 8, containLabel: true },
    xAxis: {
      type: "category",
      data: categories,
      boundaryGap: false,
      axisLabel: {
        hideOverlap: false,
        interval: "auto",
        fontSize: 10,
        color: "#5a6a78",
        formatter: (value: string) => value.slice(5),
      },
      axisLine: { lineStyle: { color: "rgba(26, 43, 60, 0.18)" } },
    },
    yAxis: {
      type: "value",
      scale: true,
      splitLine: { lineStyle: { color: "rgba(26, 43, 60, 0.08)" } },
      axisLabel: { show: false },
    },
    series: chartSeries,
  };
}

function riskChartOption(
  points: RiskValuePoint[],
  groups: RiskGroupValue[],
  scale: number,
  asOf: string,
) {
  const categories = points.map((p) => p.asOf);
  const seriesSpecs = [
    { key: "foundationMinor" as const, name: "Foundation", color: RISK_COLORS.Foundation },
    { key: "coreMinor" as const, name: "Core", color: RISK_COLORS.Core },
    { key: "riskOnMinor" as const, name: "Risk On", color: RISK_COLORS["Risk On"] },
  ];
  const symbolsByTier = new Map(groups.map((g) => [g.riskTier, g.symbols]));
  return {
    tooltip: {
      trigger: "axis",
      formatter: (items: Array<{ axisValue?: string; seriesName?: string; value?: number | null; color?: string }>) => {
        const day = String(items[0]?.axisValue ?? "");
        const point = points.find((p) => p.asOf === day);
        const rows = items
          .filter((item) => item.value != null)
          .map((item) => {
            const symbols = day === asOf ? (symbolsByTier.get(item.seriesName ?? "") ?? []) : [];
            const names = symbols.map((s) => s.symbol).join(", ");
            const amount =
              typeof item.value === "number" ? formatUsd(Math.round(item.value * 100), scale) : "unknown";
            return `<div><span style="display:inline-block;width:0.7rem;height:0.7rem;background:${item.color};margin-right:0.35rem"></span><strong>${item.seriesName}</strong> ${amount}${names ? `<div style="margin-left:1.05rem;color:#5a6a78">${names}</div>` : ""}</div>`;
          });
        const total =
          point?.totalMinor == null ? "unknown" : formatUsd(point.totalMinor, scale);
        return `<div><div><strong>${day}</strong></div><div>Daily total ${total}</div>${rows.join("")}</div>`;
      },
    },
    legend: {
      top: 4,
      right: 8,
      itemWidth: 10,
      itemHeight: 10,
      textStyle: { fontSize: 10 },
    },
    grid: { left: 16, right: 18, top: 36, bottom: 8, containLabel: true },
    xAxis: {
      type: "category",
      data: categories,
      boundaryGap: false,
      axisLabel: {
        hideOverlap: false,
        fontSize: 10,
        color: "#5a6a78",
        formatter: (value: string) => value.slice(5),
      },
      axisLine: { lineStyle: { color: "rgba(26, 43, 60, 0.18)" } },
    },
    yAxis: {
      type: "value",
      scale: true,
      splitLine: { lineStyle: { color: "rgba(26, 43, 60, 0.08)" } },
      axisLabel: { show: false },
    },
    series: seriesSpecs.map((spec) => ({
      name: spec.name,
      type: "line",
      stack: "risk",
      data: points.map((p) => dollars(p[spec.key], scale)),
      showSymbol: true,
      symbolSize: 5,
      smooth: false,
      connectNulls: false,
      lineStyle: { width: 1.2, color: spec.color },
      itemStyle: { color: spec.color },
      areaStyle: { color: spec.color, opacity: 0.72 },
    })),
  };
}

function AccountValueCard({
  series,
  color,
  featured,
  compact,
}: {
  series: AccountValueSeries;
  color: string;
  featured?: "fidelity" | "schwab";
  compact?: boolean;
}) {
  const scale = series.scale ?? 2;
  const value =
    series.currentMinor == null ? "unknown" : formatUsd(series.currentMinor, scale);
  const featuredClass =
    featured === "fidelity"
      ? " is-featured"
      : featured === "schwab"
        ? " is-featured is-featured-schwab"
        : "";
  const hasChart =
    series.points.length > 0 ||
    (series.trendsPoints ?? []).some((p) => p.marketValueMinor != null);
  return (
    <article
      className={`home-av-card${featuredClass}`}
      aria-label={
        featured === "fidelity"
          ? "Fidelity total"
          : featured === "schwab"
            ? "Schwab total"
            : `Account value ${series.accountName}`
      }
    >
      <header>
        <h3>{series.accountName}</h3>
        <p className="home-av-custodian">{series.custodian}</p>
      </header>
      <p className="home-av-value">{value}</p>
      {!series.currentComplete ? (
        <p className="home-av-note">Incomplete — a last price is missing</p>
      ) : null}
      {!hasChart ? (
        <p className="home-av-empty">No live or stored points yet.</p>
      ) : (
        <ReactECharts
          option={chartOption(series, color)}
          style={{ height: compact ? 110 : 160, width: "100%" }}
          opts={{ renderer: "canvas" }}
          notMerge
        />
      )}
    </article>
  );
}

function RiskStackCard({
  risk,
  points,
  asOf,
}: {
  risk: RiskValueHome;
  points: RiskValuePoint[];
  asOf: string;
}) {
  const scale = risk.scale ?? 2;
  const total =
    risk.currentTotalMinor == null ? "unknown" : formatUsd(risk.currentTotalMinor, scale);
  const visibleGroups = risk.groups.filter((g) => g.riskTier !== "Undecided");
  const hasChart = points.length > 0;
  const hasSymbols = visibleGroups.some((g) => g.symbols.length > 0);
  const [symbolOpen, setSymbolOpen] = useState(false);
  useEffect(() => {
    if (!symbolOpen) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") setSymbolOpen(false);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [symbolOpen]);
  return (
    <article className="home-av-card home-av-risk" aria-label="Live value by risk">
      <header>
        <h3>Live by risk</h3>
        <p className="home-av-custodian">Core, Foundation, and Risk On</p>
      </header>
      <p className="home-av-value">{total}</p>
      {!risk.currentComplete ? (
        <p className="home-av-note">Incomplete — a last price is missing</p>
      ) : null}
      {!hasChart ? (
        <p className="home-av-empty">No live points yet.</p>
      ) : (
        <>
          <ReactECharts
            option={riskChartOption(points, risk.groups, scale, asOf)}
            style={{ height: 240, width: "100%" }}
            opts={{ renderer: "canvas" }}
            notMerge
            onEvents={{ dblclick: () => setSymbolOpen(true) }}
          />
          <p className="home-av-dblclick-hint">
            Double-click the chart for the symbol list.
          </p>
        </>
      )}
      {hasSymbols ? (
        <button
          type="button"
          aria-label="Symbol totals"
          onClick={() => setSymbolOpen(true)}
        >
          Symbol list
        </button>
      ) : null}
      {symbolOpen ? (
        <div
          className="home-av-dialog-backdrop"
          onClick={() => setSymbolOpen(false)}
        >
          <div
            role="dialog"
            aria-modal="true"
            aria-label="Symbol totals"
            className="home-av-dialog"
            onClick={(event) => event.stopPropagation()}
          >
            <header>
              <h3>Symbols by risk</h3>
              <button
                type="button"
                aria-label="Exit symbol totals"
                onClick={() => setSymbolOpen(false)}
              >
                Exit
              </button>
            </header>
            <div className="home-av-risk-groups" aria-label="Risk symbol groups">
              {visibleGroups.map((group) => (
                <section key={group.riskTier}>
                  <h4>{group.riskTier}</h4>
                  <p>
                    {group.currentMinor == null
                      ? "unknown"
                      : formatUsd(group.currentMinor, scale)}
                  </p>
                  {group.symbols.length === 0 ? (
                    <p className="home-av-empty">No symbols</p>
                  ) : (
                    <ul>
                      {group.symbols.map((row) => (
                        <li key={row.symbol}>
                          {row.symbol}{" "}
                          {row.marketValueMinor == null
                            ? "unknown"
                            : formatUsd(row.marketValueMinor, scale)}
                        </li>
                      ))}
                    </ul>
                  )}
                </section>
              ))}
            </div>
          </div>
        </div>
      ) : null}
    </article>
  );
}

function rangeCaption(
  asOf: string,
  period: GraphPeriod,
  visibleDays: number,
  storedDays: number,
): string {
  const start = graphPeriodStartIso(asOf, period);
  const from = start ?? "first stored day";
  return `Calendar days (${from} to ${asOf}; ${visibleDays} of ${storedDays} in view).`;
}

function uniqueDays(values: AccountValueHomeGet): string[] {
  const days = new Set<string>();
  const add = (points?: AccountValuePoint[]) => {
    for (const p of points ?? []) days.add(p.asOf);
  };
  add(values.fidelity.points);
  add(values.fidelity.trendsPoints);
  add(values.schwab?.points);
  add(values.schwab?.trendsPoints);
  for (const acct of values.accounts) {
    add(acct.points);
    add(acct.trendsPoints);
  }
  for (const p of values.risk?.points ?? []) days.add(p.asOf);
  return [...days].sort();
}

export function HomeAccountCharts({
  values,
}: {
  values: AccountValueHomeGet | null;
}) {
  const [period, setPeriod] = useState<GraphPeriod>("12m");
  const asOf = values?.asOf ?? "";
  const filtered = useMemo(() => {
    if (!values) return null;
    return {
      fidelity: filterSeries(values.fidelity, asOf, period),
      schwab: values.schwab ? filterSeries(values.schwab, asOf, period) : null,
      accounts: values.accounts.map((acct) => filterSeries(acct, asOf, period)),
      riskPoints: filterPoints(values.risk?.points, asOf, period),
    };
  }, [values, asOf, period]);

  if (!values || !filtered) {
    return <p role="status">Loading account values…</p>;
  }
  const schwab = filtered.schwab ?? {
    accountId: "schwab-total",
    accountName: "Schwab Total",
    custodian: "Schwab",
    currentMinor: null,
    currentComplete: false,
    points: [],
    trendsPoints: [],
    scale: values.scale ?? 2,
  };
  const storedDays = uniqueDays(values).length;
  const visibleDays = uniqueDays({
    ...values,
    fidelity: filtered.fidelity,
    schwab,
    accounts: filtered.accounts,
    risk: values.risk
      ? { ...values.risk, points: filtered.riskPoints }
      : values.risk,
  }).length;
  const emptyRisk: RiskValueHome = {
    currentTotalMinor: null,
    currentComplete: false,
    groups: [],
    points: [],
    scale: values.scale ?? 2,
  };

  return (
    <section className="home-account-values" aria-label="Account values">
      <header className="home-av-heading">
        <h2>Account values</h2>
        <div className="trends-period-bar">
          <label className="trends-period-label">
            Graphing period
            <select
              aria-label="Home graphing period"
              value={period}
              onChange={(e) => setPeriod(e.target.value as GraphPeriod)}
            >
              {GRAPH_PERIOD_OPTIONS.map((opt) => (
                <option key={opt.value} value={opt.value}>
                  {opt.label}
                </option>
              ))}
            </select>
          </label>
          <p className="trends-period-caption">
            {rangeCaption(asOf, period, visibleDays, storedDays)}
          </p>
        </div>
        <div className="home-av-legend" aria-label="Account value legend">
          <p>
            <span className="home-av-swatch is-live" aria-hidden="true" />
            <strong>Live</strong> — qty × last price, including today
          </p>
          <p>
            <span className="home-av-swatch is-trends" aria-hidden="true" />
            <strong>Trends</strong> — stored week close (dashed)
          </p>
        </div>
      </header>
      <RiskStackCard risk={values.risk ?? emptyRisk} points={filtered.riskPoints} asOf={asOf} />
      <div className="home-av-totals">
        <AccountValueCard
          series={filtered.fidelity}
          color={colorFor(values.fidelity.accountName, 0, values.fidelity.custodian)}
          featured="fidelity"
          compact
        />
        <AccountValueCard
          series={schwab}
          color={colorFor(schwab.accountName, 1, schwab.custodian)}
          featured="schwab"
          compact
        />
      </div>
      <div className="home-av-grid">
        {filtered.accounts.map((acct, i) => (
          <AccountValueCard
            key={acct.accountId}
            series={acct}
            color={colorFor(acct.accountName, i, acct.custodian)}
          />
        ))}
      </div>
    </section>
  );
}
