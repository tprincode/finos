import {
  holdDays,
  projectHold,
} from "../interest-rate/math";

export type RollYieldProjection = {
  period: number;
  days: number;
  daily: number;
  weekly: number;
  monthly: number;
  annual: number;
};

/** Premium ÷ collateral for the hold window; compound via projectHold. */
export function rollYield(input: {
  premium: number;
  collateral: number;
  startIso: string;
  endIso: string;
}): RollYieldProjection | null {
  const { premium, collateral, startIso, endIso } = input;
  if (!Number.isFinite(premium) || !Number.isFinite(collateral) || collateral === 0) {
    return null;
  }
  const days = holdDays(startIso, endIso);
  if (days == null || days <= 0) return null;
  const period = premium / collateral;
  const proj = projectHold(period, days);
  if (!proj) return null;
  return { period, days, ...proj };
}
