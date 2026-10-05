/** Compound periods in a 365-day year. Matches the sheet: 3.50% monthly → 51.11% annual. */
export const PERIODS_PER_YEAR = {
  daily: 365,
  weekly: 52,
  monthly: 12,
  quarterly: 4,
  annual: 1,
} as const;

export type RatePeriod = keyof typeof PERIODS_PER_YEAR;

export const RATE_PERIODS: { id: RatePeriod; label: string }[] = [
  { id: "daily", label: "Daily" },
  { id: "weekly", label: "Weekly" },
  { id: "monthly", label: "Monthly" },
  { id: "quarterly", label: "Quarterly" },
  { id: "annual", label: "Annual" },
];

export function todayIso(now = new Date()): string {
  const y = now.getFullYear();
  const m = String(now.getMonth() + 1).padStart(2, "0");
  const d = String(now.getDate()).padStart(2, "0");
  return `${y}-${m}-${d}`;
}

export function parsePercentInput(raw: string): number | null {
  const text = raw.trim().replace(/%/g, "");
  if (text === "") return null;
  const n = Number(text);
  if (!Number.isFinite(n)) return null;
  return n / 100;
}

export function parseMoneyInput(raw: string): number | null {
  const text = raw.trim().replace(/[$,]/g, "");
  if (text === "") return null;
  const n = Number(text);
  return Number.isFinite(n) ? n : null;
}

export function holdDays(startIso: string, endIso: string): number | null {
  if (!startIso || !endIso) return null;
  const start = Date.parse(`${startIso}T00:00:00`);
  const end = Date.parse(`${endIso}T00:00:00`);
  if (!Number.isFinite(start) || !Number.isFinite(end)) return null;
  return Math.round((end - start) / 86_400_000);
}

export function convertPeriodRate(
  rate: number,
  fromPerYear: number,
  toPerYear: number,
): number {
  const annual = (1 + rate) ** fromPerYear - 1;
  return (1 + annual) ** (1 / toPerYear) - 1;
}

export function projectHold(periodReturn: number, days: number): {
  daily: number;
  weekly: number;
  monthly: number;
  annual: number;
} | null {
  if (days <= 0 || !Number.isFinite(periodReturn)) return null;
  const annual = (1 + periodReturn) ** (365 / days) - 1;
  return {
    daily: (1 + annual) ** (1 / 365) - 1,
    weekly: (1 + annual) ** (1 / 52) - 1,
    monthly: (1 + annual) ** (1 / 12) - 1,
    annual,
  };
}

export function formatPct(rate: number | null | undefined): string {
  if (rate == null || !Number.isFinite(rate)) return "—";
  return `${(rate * 100).toFixed(2)}%`;
}

export function formatHold(days: number | null): string {
  if (days == null) return "—";
  if (days <= 0) return "—";
  if (days === 1) return "1 day";
  return `${days} days`;
}
