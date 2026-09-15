export type GraphPeriod = "1m" | "2m" | "3m" | "6m" | "12m" | "ytd" | "all";

export const GRAPH_PERIOD_OPTIONS: Array<{ value: GraphPeriod; label: string }> = [
  { value: "1m", label: "1 month" },
  { value: "2m", label: "2 months" },
  { value: "3m", label: "3 months" },
  { value: "6m", label: "6 months" },
  { value: "12m", label: "12 months" },
  { value: "ytd", label: "YTD" },
  { value: "all", label: "All data" },
];

/** Calendar-month windows. 2026-09-11 1m starts 2026-08-11; 2m starts 2026-07-11. */
export const GRAPH_PERIOD_START_EXAMPLES = [
  { asOf: "2026-09-11", period: "1m" as const, startOn: "2026-08-11" },
  { asOf: "2026-09-11", period: "2m" as const, startOn: "2026-07-11" },
] as const;

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

const MONTHS_BACK: Record<Exclude<GraphPeriod, "ytd" | "all">, number> = {
  "1m": 1,
  "2m": 2,
  "3m": 3,
  "6m": 6,
  "12m": 12,
};

export function graphPeriodStartIso(asOfIso: string, period: GraphPeriod): string | null {
  if (period === "all") return null;
  const asOf = parseIsoDate(asOfIso);
  if (!asOf) return null;
  if (period === "ytd") return `${asOf.getFullYear()}-01-01`;
  return toIsoDate(addMonths(asOf, -MONTHS_BACK[period]));
}

export function inGraphPeriod(iso: string, asOfIso: string, period: GraphPeriod): boolean {
  const day = iso.slice(0, 10);
  const asOf = asOfIso.slice(0, 10);
  if (day > asOf) return false;
  const start = graphPeriodStartIso(asOf, period);
  if (!start) return true;
  return day >= start;
}

if (
  GRAPH_PERIOD_START_EXAMPLES.some(
    (row) => graphPeriodStartIso(row.asOf, row.period) !== row.startOn,
  )
) {
  throw new Error("graphPeriodStartIso examples drifted");
}
