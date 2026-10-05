import { priceCents } from "./cartPrice";

export type CashConfirmPreview = {
  baselineMinor: number;
  sellProceedsMinor: number;
  buySpendMinor: number;
  targetMinor: number;
  currentMinor: number | null;
  deltaMinor: number | null;
};

/** Same leftover rust Confirm cash writes: baseline + sale proceeds − purchase costs. */
export function cashConfirmPreview(input: {
  baselineMinor: number | null | undefined;
  sellProceedsMinor: number;
  buySpendMinor: number;
  currentMinor: number | null | undefined;
}): CashConfirmPreview | null {
  if (input.baselineMinor == null || !Number.isFinite(input.baselineMinor)) return null;
  const targetMinor = input.baselineMinor + input.sellProceedsMinor - input.buySpendMinor;
  const currentMinor =
    input.currentMinor != null && Number.isFinite(input.currentMinor) ? input.currentMinor : null;
  return {
    baselineMinor: input.baselineMinor,
    sellProceedsMinor: input.sellProceedsMinor,
    buySpendMinor: input.buySpendMinor,
    targetMinor,
    currentMinor,
    deltaMinor: currentMinor == null ? null : targetMinor - currentMinor,
  };
}

export function lotCostCents(lot: {
  remainingPerformanceMinor: number;
  scale: number;
}): number {
  return priceCents(lot.remainingPerformanceMinor, lot.scale);
}
