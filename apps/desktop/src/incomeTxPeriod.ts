/** Home Income through list windows. Calendar days — not 30/60-day graphing periods. */

export type IncomeTxPeriod = "today" | "month" | "ytd" | "custom";

export const INCOME_TX_PERIOD_OPTIONS: Array<{
  value: IncomeTxPeriod;
  label: string;
}> = [
  { value: "today", label: "Today" },
  { value: "month", label: "Current month" },
  { value: "ytd", label: "Year to date" },
  { value: "custom", label: "Custom date range" },
];

export type IncomeTxRange = { startOn: string; endOn: string };

export function localIsoDate(now = new Date()): string {
  const y = now.getFullYear();
  const m = String(now.getMonth() + 1).padStart(2, "0");
  const d = String(now.getDate()).padStart(2, "0");
  return `${y}-${m}-${d}`;
}

export function incomeTxRange(
  period: IncomeTxPeriod,
  todayIso: string,
  customStart: string,
  customEnd: string,
): IncomeTxRange | null {
  const today = todayIso.slice(0, 10);
  if (period === "today") return { startOn: today, endOn: today };
  if (period === "month") return { startOn: `${today.slice(0, 7)}-01`, endOn: today };
  if (period === "ytd") return { startOn: `${today.slice(0, 4)}-01-01`, endOn: today };
  const start = customStart.trim().slice(0, 10);
  const end = customEnd.trim().slice(0, 10);
  if (!start || !end) return null;
  return start <= end ? { startOn: start, endOn: end } : { startOn: end, endOn: start };
}

export function inIncomeTxRange(day: string, range: IncomeTxRange | null): boolean {
  if (!range) return false;
  const iso = day.slice(0, 10);
  return iso >= range.startOn && iso <= range.endOn;
}

/** Authority 11 Sep 2026. */
export const INCOME_TX_RANGE_EXAMPLES = [
  { period: "today" as const, today: "2026-09-11", startOn: "2026-09-11", endOn: "2026-09-11" },
  { period: "month" as const, today: "2026-09-11", startOn: "2026-09-01", endOn: "2026-09-11" },
  { period: "ytd" as const, today: "2026-09-11", startOn: "2026-01-01", endOn: "2026-09-11" },
] as const;

if (
  INCOME_TX_RANGE_EXAMPLES.some((row) => {
    const range = incomeTxRange(row.period, row.today, "", "");
    return !range || range.startOn !== row.startOn || range.endOn !== row.endOn;
  })
) {
  throw new Error("incomeTxRange examples drifted");
}
