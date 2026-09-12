export type GraphPeriod = "3m" | "6m" | "12m" | "ytd" | "all";

export const GRAPH_PERIOD_OPTIONS: Array<{ value: GraphPeriod; label: string }> = [
  { value: "3m", label: "3 months" },
  { value: "6m", label: "6 months" },
  { value: "12m", label: "12 months" },
  { value: "ytd", label: "YTD" },
  { value: "all", label: "All data" },
];

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

export function graphPeriodStartIso(asOfIso: string, period: GraphPeriod): string | null {
  if (period === "all") return null;
  const asOf = parseIsoDate(asOfIso);
  if (!asOf) return null;
  if (period === "ytd") return `${asOf.getFullYear()}-01-01`;
  const months = period === "3m" ? 3 : period === "6m" ? 6 : 12;
  return toIsoDate(addMonths(asOf, -months));
}

export function inGraphPeriod(iso: string, asOfIso: string, period: GraphPeriod): boolean {
  const day = iso.slice(0, 10);
  const asOf = asOfIso.slice(0, 10);
  if (day > asOf) return false;
  const start = graphPeriodStartIso(asOf, period);
  if (!start) return true;
  return day >= start;
}
