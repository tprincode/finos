import { useMemo, useState } from "react";
import ReactECharts from "echarts-for-react";

export type DeclarationPaymentPoint = {
  paymentPeriod: string;
  amountPerShareMinor: number;
  amountScale: number;
};

type GraphPeriod = "1m" | "3m" | "ytd" | "1y" | "all";

const PERIOD_OPTIONS: Array<{ value: GraphPeriod; label: string }> = [
  { value: "1m", label: "1 month" },
  { value: "3m", label: "3 months" },
  { value: "ytd", label: "YTD" },
  { value: "1y", label: "1 year" },
  { value: "all", label: "All" },
];

function parsePeriodDate(period: string): Date | null {
  const raw = period.trim().slice(0, 10);
  if (!/^\d{4}-\d{2}-\d{2}$/.test(raw)) return null;
  const d = new Date(`${raw}T00:00:00`);
  return Number.isNaN(d.getTime()) ? null : d;
}

function formatPerShare(minor: number, scale: number): string {
  const places = Number.isFinite(scale) ? scale : 4;
  const factor = 10 ** places;
  return `$${(minor / factor).toFixed(places > 4 ? 4 : places)}`;
}

function filterByPeriod(
  points: DeclarationPaymentPoint[],
  period: GraphPeriod,
  asOf: string,
): DeclarationPaymentPoint[] {
  if (period === "all") return points;
  const end = parsePeriodDate(asOf) ?? new Date();
  const start = new Date(end);
  if (period === "1m") {
    start.setMonth(start.getMonth() - 1);
  } else if (period === "3m") {
    start.setMonth(start.getMonth() - 3);
  } else if (period === "1y") {
    start.setFullYear(start.getFullYear() - 1);
  } else {
    start.setMonth(0, 1);
  }
  return points.filter((p) => {
    const d = parsePeriodDate(p.paymentPeriod);
    return d != null && d >= start && d <= end;
  });
}

export function DeclarationPaymentsChart({
  declarations,
  asOfDate,
}: {
  declarations: DeclarationPaymentPoint[];
  asOfDate: string;
}) {
  const [period, setPeriod] = useState<GraphPeriod>("1y");
  const paid = useMemo(
    () =>
      declarations
        .filter((d) => d.amountPerShareMinor != null && d.amountPerShareMinor > 0)
        .map((d) => ({
          paymentPeriod: d.paymentPeriod,
          amountPerShareMinor: d.amountPerShareMinor,
          amountScale: d.amountScale,
        }))
        .sort((a, b) => a.paymentPeriod.localeCompare(b.paymentPeriod)),
    [declarations],
  );
  const filtered = useMemo(
    () => filterByPeriod(paid, period, asOfDate),
    [paid, period, asOfDate],
  );
  const option = useMemo(() => {
    const labels = filtered.map((p) => p.paymentPeriod.slice(0, 10));
    const values = filtered.map((p) => p.amountPerShareMinor / 10 ** p.amountScale);
    return {
      tooltip: {
        trigger: "axis",
        formatter: (params: Array<{ dataIndex: number }>) => {
          const idx = params[0]?.dataIndex ?? 0;
          const row = filtered[idx];
          if (!row) return "";
          return `${row.paymentPeriod}<br/>${formatPerShare(row.amountPerShareMinor, row.amountScale)}`;
        },
      },
      grid: { left: 48, right: 16, top: 24, bottom: 48 },
      xAxis: {
        type: "category",
        data: labels,
        axisLabel: { rotate: labels.length > 8 ? 45 : 0 },
      },
      yAxis: {
        type: "value",
        name: "$/share",
        axisLabel: { formatter: (v: number) => `$${v.toFixed(4)}` },
      },
      series: [
        {
          type: "bar",
          data: values,
          itemStyle: { color: "#2a6f4a" },
        },
      ],
    };
  }, [filtered]);

  if (paid.length === 0) {
    return <p>No paid declaration history to chart yet.</p>;
  }

  return (
    <div className="declaration-payments-chart" aria-label="Declaration payment history">
      <div className="trends-period-bar">
        <span className="trends-period-label">
          Period
          <select
            aria-label="Declaration graphing period"
            value={period}
            onChange={(e) => setPeriod(e.target.value as GraphPeriod)}
          >
            {PERIOD_OPTIONS.map((opt) => (
              <option key={opt.value} value={opt.value}>
                {opt.label}
              </option>
            ))}
          </select>
        </span>
        <span>
          {formatCount(filtered.length)} of {formatCount(paid.length)} payments shown
        </span>
      </div>
      <ReactECharts option={option} style={{ height: 280, width: "100%" }} />
    </div>
  );
}

function formatCount(n: number): string {
  return n.toLocaleString("en-US");
}
