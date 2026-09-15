/** Mix share vs dollar share must stay within 1 percentage point. */

export type RiskSharePoint = {
  foundationMinor: number | null;
  coreMinor: number | null;
  riskOnMinor: number | null;
  totalMinor: number | null;
};

export type RiskDayShares = {
  foundationPct: number | null;
  corePct: number | null;
  riskOnPct: number | null;
};

function pctOfTotal(minor: number | null, total: number): number | null {
  if (minor == null) return null;
  return (minor / total) * 100;
}

export function riskDayShares(point: RiskSharePoint): RiskDayShares {
  const total = point.totalMinor;
  if (total == null || total <= 0) {
    return { foundationPct: null, corePct: null, riskOnPct: null };
  }
  return {
    foundationPct: pctOfTotal(point.foundationMinor, total),
    corePct: pctOfTotal(point.coreMinor, total),
    riskOnPct: pctOfTotal(point.riskOnMinor, total),
  };
}

export function sharesMatchDollarShare(
  point: RiskSharePoint,
  maxPts = 1,
): boolean {
  const shares = riskDayShares(point);
  const total = point.totalMinor;
  if (total == null || total <= 0) {
    return true;
  }
  const check = (minor: number | null, share: number | null) => {
    if (minor == null || share == null) return true;
    return Math.abs((minor / total) * 100 - share) <= maxPts;
  };
  return (
    check(point.foundationMinor, shares.foundationPct) &&
    check(point.coreMinor, shares.corePct) &&
    check(point.riskOnMinor, shares.riskOnPct)
  );
}

/** Authority 11 Sep 2026 hover: Foundation ~45%, Core ~28%, Risk On ~27%. */
export const RISK_SHARE_GOLDEN: RiskSharePoint = {
  foundationMinor: 16_147_952,
  coreMinor: 10_135_312,
  riskOnMinor: 9_552_360,
  totalMinor: 35_862_244,
};

if (!sharesMatchDollarShare(RISK_SHARE_GOLDEN, 1)) {
  throw new Error("riskDayShares drifted from dollar share");
}
