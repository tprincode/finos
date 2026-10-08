/** Income Plan week KPIs: unknown plan/declaration stays incomplete (—), never summed as $0. */

export type WeekMoneyRow = {
  symbol: string;
  /** Absent is unknown, the same as false. */
  planKnown?: boolean;
  plannedMinor?: number | null;
  declarationKnown?: boolean;
  declarationMinor?: number | null;
};

export type WeekKpi = {
  /** True when every payer is known for this KPI (empty payer list is complete at $0). */
  complete: boolean;
  minor: number;
  missingSymbols: string[];
};

export function weekPlanKpi(payers: WeekMoneyRow[]): WeekKpi {
  const missingSymbols = payers
    .filter((row) => !row.planKnown)
    .map((row) => row.symbol);
  const complete = missingSymbols.length === 0;
  const minor = complete
    ? payers.reduce((sum, row) => sum + (row.plannedMinor ?? 0), 0)
    : 0;
  return { complete, minor, missingSymbols };
}

export function weekDeclaredKpi(payers: WeekMoneyRow[]): WeekKpi {
  const missingSymbols = payers
    .filter((row) => !row.declarationKnown)
    .map((row) => row.symbol);
  const complete = missingSymbols.length === 0;
  const minor = complete
    ? payers.reduce((sum, row) => sum + (row.declarationMinor ?? 0), 0)
    : 0;
  return { complete, minor, missingSymbols };
}
