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
