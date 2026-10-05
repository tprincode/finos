/** Twice monthly is 24. Frequency wins over a stale planningPeriodsPerYear of 12. */
export function periodsForPlan(frequency: string, storedPeriods: number): number {
  const key = frequency.trim().toLowerCase();
  if (key === "weekly" || key === "52") return 52;
  if (
    key === "twice monthly" ||
    key === "twice-monthly" ||
    key === "semimonthly" ||
    key === "semi-monthly" ||
    key === "24"
  ) {
    return 24;
  }
  if (key === "monthly" || key === "12") return 12;
  if (key === "quarterly" || key === "4") return 4;
  return storedPeriods > 0 ? storedPeriods : 0;
}

/** Plan $/period → scale-2 annual. HAKY 3800 at scale 4 is $0.38/sh/mo, not $38. */
export function planAnnualCents(
  planPerShareMinor: number,
  planScale: number,
  periods: number,
  qtyWhole: number,
): number | null {
  if (planPerShareMinor <= 0 || periods <= 0 || qtyWhole <= 0) return null;
  const perPeriodCents = Math.round((planPerShareMinor * 100) / 10 ** planScale);
  if (perPeriodCents <= 0) return null;
  return perPeriodCents * periods * qtyWhole;
}

/** The two numbers behind every cart rate: "Plan $0.2300 x 24". Declarations never appear. */
export function planBasisText(
  planPerShareMinor: number,
  planScale: number,
  periods: number,
): string {
  if (planPerShareMinor <= 0 || periods <= 0) return "";
  const places = planScale > 4 ? planScale : 4;
  return `Plan $${(planPerShareMinor / 10 ** planScale).toFixed(places)} x ${periods}`;
}

export function planEachAnnualCents(
  planPerShareMinor: number,
  planScale: number,
  periods: number,
): number | null {
  return planAnnualCents(planPerShareMinor, planScale, periods, 1);
}

export function incomeWeekMonth(yearMinor: number | null): {
  weekMinor: number | null;
  monthMinor: number | null;
} {
  if (yearMinor == null) return { weekMinor: null, monthMinor: null };
  return { weekMinor: Math.trunc(yearMinor / 52), monthMinor: Math.trunc(yearMinor / 12) };
}
