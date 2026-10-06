import type { OccRight } from "./parseOcc";

export type AssignmentFlags = {
  dte: number | null;
  itm: boolean;
  assignmentRisk: boolean;
};

function calendarDays(asOfIso: string, expiryIso: string): number | null {
  if (!asOfIso || !expiryIso) return null;
  const start = Date.parse(`${asOfIso.slice(0, 10)}T00:00:00`);
  const end = Date.parse(`${expiryIso.slice(0, 10)}T00:00:00`);
  if (!Number.isFinite(start) || !Number.isFinite(end)) return null;
  return Math.round((end - start) / 86_400_000);
}

/** Local heuristic: ITM vs strike (both cents); assignment risk when ITM and DTE ≤ 7. */
export function assignmentFlags(input: {
  right: OccRight;
  strikeMinor: number;
  asOfIso: string;
  expiryIso: string;
  underlyingLastMinor: number | null;
}): AssignmentFlags {
  const dte = calendarDays(input.asOfIso, input.expiryIso);
  const last = input.underlyingLastMinor;
  let itm = false;
  if (last != null && Number.isFinite(last) && Number.isFinite(input.strikeMinor)) {
    if (input.right === "call") {
      itm = last >= input.strikeMinor;
    } else {
      itm = last <= input.strikeMinor;
    }
  }
  const assignmentRisk = itm && dte != null && dte >= 0 && dte <= 7;
  return { dte, itm, assignmentRisk };
}

/** Yellow when last and strike both exist and the gap is at most 10% of last. */
export function nearStrike(lastMinor: number | null, strikeMinor: number): boolean {
  if (lastMinor == null || lastMinor === 0) return false;
  if (!Number.isFinite(lastMinor) || !Number.isFinite(strikeMinor)) return false;
  return Math.abs(lastMinor - strikeMinor) / Math.abs(lastMinor) <= 0.1;
}
