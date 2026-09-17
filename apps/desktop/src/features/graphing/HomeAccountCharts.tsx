import { useEffect, useMemo, useState } from "react";
import ReactECharts from "echarts-for-react";
import type {
  AccountValueHomeGet,
  AccountValueSeries,
  RiskGroupValue,
  RiskValueHome,
  RiskValuePoint,
} from "@finos/app-contracts";
import { formatUsd } from "@finos/ui-components";
import {
  DEFAULT_GRAPH_PERIOD,
  GRAPH_PERIOD_OPTIONS,
  graphPeriodStartIso,
  inGraphPeriod,
  type GraphPeriod,
} from "./graphPeriod";
import { riskDayShares } from "./riskChart";

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

const INCOME_COLOR = "#8a3d6b";

export function isTrendsPlacedAccount(name: string, custodian?: string): boolean {
  const n = name.trim().toLowerCase();
  const c = (custodian ?? "").trim().toLowerCase();
  if (c === "robinhood" || n.includes("robinhood")) return true;
  return c === "direct" || n.includes("energyx") || n === "energy";
}

export function colorFor(name: string, index: number, custodian?: string): string {
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

function hexToRgba(hex: string, alpha: number): string {
  const raw = hex.replace("#", "");
  const full = raw.length === 3 ? raw.split("").map((c) => c + c).join("") : raw;
  const n = Number.parseInt(full, 16);
  return `rgba(${(n >> 16) & 255},${(n >> 8) & 255},${n & 255},${alpha})`;
}

function filterPoints<T extends { asOf: string }>(
  points: T[] | undefined,
  asOf: string,
  period: GraphPeriod,
): T[] {
  return (points ?? []).filter((p) => inGraphPeriod(p.asOf, asOf, period));
}

export function filterSeries(series: AccountValueSeries, asOf: string, period: GraphPeriod): AccountValueSeries {
  return {
    ...series,
    points: filterPoints(series.points, asOf, period),
    trendsPoints: filterPoints(series.trendsPoints, asOf, period),
    incomePoints: filterPoints(series.incomePoints, asOf, period),
  };
}

export function accountChartOption(
  series: AccountValueSeries,
  color: string,
  chrome: "home" | "trends" = "home",
) {
  const scale = series.scale ?? 2;
  const live = series.points ?? [];
  const trends = series.trendsPoints ?? [];
  const income = series.incomePoints ?? [];
  const categories = [
    ...new Set([
      ...live.map((p) => p.asOf),
      ...trends.map((p) => p.asOf),
      ...income.map((p) => p.asOf),
    ]),
  ].sort();
  const liveByDay = new Map(live.map((p) => [p.asOf, p.marketValueMinor]));
  const trendsByDay = new Map(trends.map((p) => [p.asOf, p.marketValueMinor]));
  const incomeByDay = new Map(income.map((p) => [p.asOf, p.incomeMinor]));
  const liveData = categories.map((day) => dollars(liveByDay.get(day) ?? null, scale));
  const trendsData = categories.map((day) => dollars(trendsByDay.get(day) ?? null, scale));
  const incomeData = categories.map((day) => dollars(incomeByDay.get(day) ?? null, scale));
  const hasTrends = trends.some((p) => p.marketValueMinor != null);
  const hasIncome =
    series.accountName.trim().toLowerCase() !== "speculation" &&
    income.some((p) => p.incomeMinor != null);
  const chartSeries: Record<string, unknown>[] = [
    {
      name: "Live",
      type: "line",
      data: liveData,
      showSymbol: chrome === "home",
      symbolSize: 5,
      smooth: false,
      connectNulls: false,
      z: 1,
      lineStyle: { width: 2.2, color },
      itemStyle: { color },
      areaStyle:
        chrome === "home"
          ? {
              opacity: 0.35,
              color: {
                type: "linear",
                x: 0,
                y: 0,
                x2: 0,
                y2: 1,
                colorStops: [
                  { offset: 0, color: hexToRgba(color, 0.28) },
                  { offset: 1, color: "rgba(255,255,255,0)" },
                ],
              },
            }
          : undefined,
    },
  ];
  if (hasTrends) {
    chartSeries.push({
      name: "Trends",
      type: "line",
      data: trendsData,
      showSymbol: chrome === "home",
      symbolSize: 5,
      smooth: false,
      connectNulls: true,
      z: 2,
      lineStyle: { width: 2, type: "dashed", color: "#5a6a78" },
      itemStyle: { color: "#5a6a78" },
    });
  }
  if (hasIncome) {
    chartSeries.push({
      name: "Weekly actuals",
      type: "line",
      yAxisIndex: 1,
      data: incomeData,
      showSymbol: true,
      symbol: "circle",
      symbolSize: 7,
      smooth: false,
      connectNulls: true,
      z: 8,
      lineStyle: { width: 2.2, type: "dotted", color: INCOME_COLOR },
      itemStyle: { color: INCOME_COLOR, borderColor: "#fff", borderWidth: 1.5 },
    });
  }
  const showLegend = chrome === "trends" && (hasTrends || hasIncome);
  return {
    tooltip: {
      trigger: "axis",
      formatter: (
        items: Array<{ seriesName?: string; value?: number | null; axisValue?: string; color?: string }>,
      ) => {
        const day = String(items[0]?.axisValue ?? "");
        const rows = items
          .filter((item) => item.value != null)
          .map((item) => {
            const amount =
              typeof item.value === "number"
                ? formatUsd(Math.round(item.value * 100), scale)
                : "unknown";
            return `<div><span style="display:inline-block;width:0.7rem;height:0.7rem;background:${item.color};margin-right:0.35rem"></span><strong>${item.seriesName}</strong> ${amount}</div>`;
          });
        return `<div><div><strong>${day}</strong></div>${rows.join("")}</div>`;
      },
    },
    title:
      chrome === "trends"
        ? { text: series.accountName, left: 0, textStyle: { fontSize: 13, fontWeight: 600 } }
        : undefined,
    legend: showLegend
      ? {
          top: 4,
          right: 8,
          itemWidth: 10,
          itemHeight: 8,
          textStyle: { fontSize: 10 },
        }
      : undefined,
    grid:
      chrome === "trends"
        ? { left: 48, right: 20, top: showLegend ? 52 : 36, bottom: 28 }
        : { left: 12, right: 36, top: 14, bottom: 8, containLabel: true },
    xAxis: {
      type: "category",
      data: categories,
      boundaryGap: false,
      axisLabel: {
        hideOverlap: false,
        interval: "auto",
        showMaxLabel: true,
        fontSize: 10,
        color: "#5a6a78",
        formatter: (value: string) => value.slice(5),
      },
      axisLine: { lineStyle: { color: "rgba(26, 43, 60, 0.18)" } },
    },
    yAxis: [
      {
        type: "value",
        scale: true,
        boundaryGap: ["4%", "8%"],
        splitLine: { lineStyle: { color: "rgba(26, 43, 60, 0.08)" } },
        axisLabel: chrome === "trends" ? { fontSize: 10 } : { show: false },
      },
      {
        type: "value",
        scale: true,
        boundaryGap: ["4%", "8%"],
        splitLine: { show: false },
        axisLabel: { show: false },
      },
    ],
    series: chartSeries,
  };
}

const RISK_SERIES = [
  { key: "foundationMinor" as const, share: "foundationPct" as const, name: "Foundation", color: RISK_COLORS.Foundation },
  { key: "coreMinor" as const, share: "corePct" as const, name: "Core", color: RISK_COLORS.Core },
  { key: "riskOnMinor" as const, share: "riskOnPct" as const, name: "Risk On", color: RISK_COLORS["Risk On"] },
];

function riskHoverHtml(
  points: RiskValuePoint[],
  groups: RiskGroupValue[],
  scale: number,
  asOf: string,
  day: string,
): string {
  const point = points.find((p) => p.asOf === day);
  const shares = point ? riskDayShares(point) : null;
  const symbolsByTier = new Map(groups.map((g) => [g.riskTier, g.symbols]));
  const rows = RISK_SERIES.map((spec) => {
    const minor = point?.[spec.key] ?? null;
    const share = shares?.[spec.share] ?? null;
    const amount = minor == null ? "unknown" : formatUsd(minor, scale);
    const pct = share == null ? "unknown" : `${share.toFixed(1)}%`;
    const symbols = day === asOf ? (symbolsByTier.get(spec.name) ?? []) : [];
    const names = symbols.map((s) => s.symbol).join(", ");
    return `<div><span style="display:inline-block;width:0.7rem;height:0.7rem;background:${spec.color};margin-right:0.35rem"></span>${spec.name} ${amount} (${pct})${names ? `<div style="margin-left:1.05rem;color:#5a6a78">${names}</div>` : ""}</div>`;
  });
  const total =
    point?.totalMinor == null ? "unknown" : formatUsd(point.totalMinor, scale);
  return `<div><div>${day}</div><div>Daily total ${total}</div>${rows.join("")}</div>`;
}

function riskDonutOption(
  groups: RiskGroupValue[],
  scale: number,
  currentTotalMinor: number | null,
) {
  const data = RISK_SERIES.flatMap((spec) => {
    const group = groups.find((g) => g.riskTier === spec.name);
    const value = dollars(group?.currentMinor, scale);
    if (value == null) return [];
    return [
      {
        name: spec.name,
        value,
        itemStyle: { color: spec.color },
      },
    ];
  });
  const centerTotal =
    currentTotalMinor == null ? "unknown" : formatUsd(currentTotalMinor, scale);
  return {
    tooltip: {
      trigger: "item",
      formatter: (item: { name?: string; value?: number; percent?: number }) => {
        const amount =
          item.value == null
            ? "unknown"
            : formatUsd(Math.round(item.value * 10 ** scale), scale);
        const pct = item.percent == null ? "unknown" : `${item.percent.toFixed(1)}%`;
        return `${item.name ?? ""} ${amount} (${pct})`;
      },
    },
    legend: {
      bottom: 0,
      itemWidth: 10,
      itemHeight: 10,
      textStyle: { fontSize: 10 },
    },
    title: {
      text: centerTotal,
      subtext: "Current allocation",
      left: "center",
      top: "38%",
      textStyle: { fontSize: 13, fontWeight: 600 },
      subtextStyle: { fontSize: 10, color: "#5a6a78" },
    },
    series: [
      {
        type: "pie",
        radius: ["50%", "72%"],
        center: ["50%", "44%"],
        avoidLabelOverlap: true,
        label: {
          formatter: "{d}%",
          fontSize: 11,
        },
        data,
      },
    ],
  };
}

function riskLevelOption(
  points: RiskValuePoint[],
  groups: RiskGroupValue[],
  scale: number,
  asOf: string,
  totalLabel: string,
) {
  const categories = points.map((p) => p.asOf);
  return {
    tooltip: {
      trigger: "axis",
      formatter: (items: Array<{ axisValue?: string }>) =>
        riskHoverHtml(points, groups, scale, asOf, String(items[0]?.axisValue ?? "")),
    },
    title: {
      text: totalLabel,
      right: 8,
      top: 2,
      textStyle: { fontSize: 13, fontWeight: 700 },
    },
    legend: {
      top: 4,
      left: 8,
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
      min: 0,
      scale: false,
      splitLine: { lineStyle: { color: "rgba(26, 43, 60, 0.08)" } },
      axisLabel: { show: false },
    },
    series: RISK_SERIES.map((spec) => ({
      name: spec.name,
      type: "line",
      data: points.map((p) => dollars(p[spec.key], scale)),
      showSymbol: true,
      symbolSize: 5,
      smooth: false,
      connectNulls: false,
      lineStyle: { width: 2, color: spec.color },
      itemStyle: { color: spec.color },
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
    (series.trendsPoints ?? []).some((p) => p.marketValueMinor != null) ||
    (series.incomePoints ?? []).some((p) => p.incomeMinor != null);
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
      <header className="home-av-card-head">
        <h3>{series.accountName}</h3>
        <p className="home-av-custodian">{series.custodian}</p>
        <p className="home-av-value">{value}</p>
      </header>
      {!series.currentComplete ? (
        <p className="home-av-note">Incomplete — a last price is missing</p>
      ) : null}
      {!hasChart ? (
        <p className="home-av-empty">No live or stored points yet.</p>
      ) : (
        <ReactECharts
          option={accountChartOption(series, color)}
          style={{ height: compact ? 148 : 200, width: "100%" }}
          opts={{ renderer: "canvas" }}
          notMerge
        />
      )}
    </article>
  );
}

export function LiveByRiskCharts({
  risk,
  asOf,
  period,
}: {
  risk: RiskValueHome | null;
  asOf: string;
  period: GraphPeriod;
}) {
  const data = risk ?? {
    currentTotalMinor: null,
    currentComplete: false,
    groups: [],
    points: [],
    scale: 2,
  };
  const points = useMemo(
    () => filterPoints(data.points, asOf, period),
    [data.points, asOf, period],
  );
  const scale = data.scale ?? 2;
  const total =
    data.currentTotalMinor == null ? "unknown" : formatUsd(data.currentTotalMinor, scale);
  const visibleGroups = data.groups.filter((g) => g.riskTier !== "Undecided");
  const hasChart = points.length > 0;
  const hasDonut = visibleGroups.some((g) => g.currentMinor != null);
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
    <article className="home-av-card home-av-risk" aria-label="Risk Profile">
      <header className="home-av-card-head">
        <h3>Risk Profile</h3>
      </header>
      {!data.currentComplete ? (
        <p className="home-av-note">Incomplete — a last price is missing</p>
      ) : null}
      {!hasChart && !hasDonut ? (
        <p className="home-av-empty">No live points yet.</p>
      ) : (
        <div className="live-risk-pair">
          <section aria-label="Live by risk level">
            {!hasChart ? (
              <p className="home-av-empty">No live points yet.</p>
            ) : (
              <ReactECharts
                option={riskLevelOption(points, data.groups, scale, asOf, total)}
                style={{ height: 180, width: "100%" }}
                opts={{ renderer: "canvas" }}
                notMerge
                onEvents={{ dblclick: () => setSymbolOpen(true) }}
              />
            )}
          </section>
          <section aria-label="Live by risk allocation">
            {!hasDonut ? (
              <p className="home-av-empty">Current allocation unknown.</p>
            ) : (
              <ReactECharts
                option={riskDonutOption(data.groups, scale, data.currentTotalMinor)}
                style={{ height: 180, width: "100%" }}
                opts={{ renderer: "canvas" }}
                notMerge
                onEvents={{ dblclick: () => setSymbolOpen(true) }}
              />
            )}
          </section>
        </div>
      )}
      {hasChart || hasDonut ? (
        <p className="home-av-dblclick-hint">
          Double-click the chart for the symbol list.
        </p>
      ) : null}
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
  const add = (points?: Array<{ asOf: string }>) => {
    for (const p of points ?? []) days.add(p.asOf);
  };
  add(values.fidelity.points);
  add(values.fidelity.trendsPoints);
  add(values.fidelity.incomePoints);
  add(values.schwab?.points);
  add(values.schwab?.trendsPoints);
  add(values.schwab?.incomePoints);
  for (const acct of values.accounts) {
    add(acct.points);
    add(acct.trendsPoints);
    add(acct.incomePoints);
  }
  return [...days].sort();
}

export function HomeAccountCharts({
  values,
}: {
  values: AccountValueHomeGet | null;
}) {
  const [period, setPeriod] = useState<GraphPeriod>(DEFAULT_GRAPH_PERIOD);
  const asOf = values?.asOf ?? "";
  const filtered = useMemo(() => {
    if (!values) return null;
    return {
      fidelity: filterSeries(values.fidelity, asOf, period),
      schwab: values.schwab ? filterSeries(values.schwab, asOf, period) : null,
      accounts: values.accounts.map((acct) => filterSeries(acct, asOf, period)),
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
  }).length;

  return (
    <section className="home-account-values" aria-label="Account values">
      <header className="home-av-heading">
        <h2>Account values</h2>
        <div className="home-av-toolbar">
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
            <p>
              <span className="home-av-swatch is-income" aria-hidden="true" />
              <strong>Weekly actuals</strong> — paid Sat–Fri, plotted on Friday
            </p>
          </div>
        </div>
      </header>
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
        {filtered.accounts
          .filter((acct) => !isTrendsPlacedAccount(acct.accountName, acct.custodian))
          .map((acct, i) => (
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
