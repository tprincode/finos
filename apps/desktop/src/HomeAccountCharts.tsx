import ReactECharts from "echarts-for-react";
import type { AccountValueHomeGet, AccountValueSeries } from "@finos/app-contracts";
import { formatUsd } from "@finos/ui-components";

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
    grid: { left: 8, right: 8, top: 8, bottom: 22, containLabel: false },
    xAxis: {
      type: "category",
      data: categories,
      boundaryGap: false,
      axisLabel: { hideOverlap: true, fontSize: 10, color: "#5a6a78" },
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
      ) : (
        <p className="home-av-note">Live last price · dashed Trends</p>
      )}
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

export function HomeAccountCharts({
  values,
}: {
  values: AccountValueHomeGet | null;
}) {
  if (!values) {
    return <p role="status">Loading account values…</p>;
  }
  const schwab = values.schwab ?? {
    accountId: "schwab-total",
    accountName: "Schwab Total",
    custodian: "Schwab",
    currentMinor: null,
    currentComplete: false,
    points: [],
    trendsPoints: [],
    scale: values.scale ?? 2,
  };
  return (
    <section className="home-account-values" aria-label="Account values">
      <header className="home-av-heading">
        <h2>Account values</h2>
        <p>{values.note}</p>
      </header>
      <div className="home-av-totals">
        <AccountValueCard
          series={values.fidelity}
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
        {values.accounts.map((acct, i) => (
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
