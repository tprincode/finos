import { formatUsd } from "@finos/ui-components";
import type { MagiProjection, TaxPlanningGet } from "@finos/app-contracts";

export const JOINT_TWO_PERSON_CLIFF_MINOR = 8_460_000;
export const HSA_CONTRIBUTION_MINOR: number = 975_000;
export const COMPUTER_SCHEDULE_C_MINOR: number = 160_000;
export const HALF_SE_EST_MINOR: number = 92_000;
export const BARBARA_LTCG_MINOR = 0;
export const APPLICATION_APTC_MINOR = 1_805_400;

export function planParts(
  plan: TaxPlanningGet,
  key: string,
): { ytd: number; remaining: number; eoy: number } {
  const row = plan.rows.find((item) => item.key === key);
  const ytd = row?.ytdMinor ?? 0;
  const remaining = row?.projectedMinor ?? 0;
  const eoy = row?.totalMinor ?? ytd + remaining;
  return { ytd, remaining, eoy };
}

/** 1040 caps the net capital loss deducted against income at $3,000 a year. */
export const NET_CAPITAL_LOSS_LIMIT_MINOR = 300_000;

export type CapitalGainMagi = {
  /** Every capital gain part added up, gains and losses, uncapped. */
  netMinor: number;
  /** The slice MAGI takes. A net loss stops at the 1040 limit. */
  magiMinor: number;
  /** Loss left over after the limit, negative. Zero when the net is a gain. */
  carryforwardMinor: number;
  /** A lot sale the server could not place in a long or short term. */
  unplaceableSale: boolean;
};

/** Car long and short term plus Barbara net first, then the limit applies once. */
export function capitalGainForMagi(plan: TaxPlanningGet): CapitalGainMagi {
  const carLt = planParts(plan, "ltcg");
  const carSt = planParts(plan, "stcg");
  const netMinor =
    (plan.netCapitalGainMinor ?? carLt.eoy + carSt.eoy) + BARBARA_LTCG_MINOR;
  const magiMinor = Math.max(netMinor, -NET_CAPITAL_LOSS_LIMIT_MINOR);
  return {
    netMinor,
    magiMinor,
    carryforwardMinor: netMinor - magiMinor,
    unplaceableSale: plan.netCapitalGainMinor === null,
  };
}

export type MagiForecast = {
  after: number;
  cliff: number;
  overage: number;
  over: boolean;
  hole: number;
  iraRemaining: number;
  iraCut: number;
  splitCut: number;
  splitContrib: number;
  isIndeterminate: boolean;
  isEstimate: boolean;
  suggestionLines: string[];
  creditAtRiskMinor: number;
  gains: CapitalGainMagi;
};

export function forecastMagi(
  plan: TaxPlanningGet,
  magi: MagiProjection | null,
  yearAptcMinor = APPLICATION_APTC_MINOR,
): MagiForecast {
  const scale = plan.scale;
  const ira = planParts(plan, "ira");
  const job = planParts(plan, "job1099");
  const ordinary = planParts(plan, "ordinary");
  const ssa = planParts(plan, "ssa");
  const gains = capitalGainForMagi(plan);
  const magiEoy = ira.eoy + job.eoy + ordinary.eoy + ssa.eoy + gains.magiMinor;
  const iraContributionMinor = plan.iraContributionMinor ?? 0;
  const deductions =
    HSA_CONTRIBUTION_MINOR +
    COMPUTER_SCHEDULE_C_MINOR +
    HALF_SE_EST_MINOR +
    iraContributionMinor;
  const after = magiEoy - deductions;
  const cliff = magi?.applicableThreshold.amountMinor ?? JOINT_TWO_PERSON_CLIFF_MINOR;
  const overage = after - cliff;
  const over = overage > 0;
  const hole = Math.max(0, overage);
  const isIndeterminate = magi?.decisionState === "INDETERMINATE";
  const estimatesFeedForecast =
    HSA_CONTRIBUTION_MINOR !== 0 ||
    COMPUTER_SCHEDULE_C_MINOR !== 0 ||
    HALF_SE_EST_MINOR !== 0 ||
    iraContributionMinor !== 0;
  const isEstimate =
    isIndeterminate ||
    magi == null ||
    magi.dataCompleteness !== "complete" ||
    estimatesFeedForecast ||
    gains.unplaceableSale;
  const iraCut = Math.min(ira.remaining, hole);
  const splitCut =
    hole > 0 && ira.remaining > 0
      ? hole <= ira.remaining
        ? Math.floor(hole / 2 / 100) * 100
        : iraCut
      : 0;
  const splitContrib = hole > 0 ? hole - splitCut : 0;
  const suggestionLines: string[] = [];
  if (isIndeterminate) {
    suggestionLines.push("Book the missing fact: Barbara LTCG, Schedule C, APTC.");
  }
  if (gains.unplaceableSale) {
    suggestionLines.push(
      "A lot sale is dated before its lot, so capital gains are missing from this forecast.",
    );
  }
  if (hole > 0 && iraCut > 0) {
    suggestionLines.push(
      `Cut remaining Traditional IRA draws by ${formatUsd(iraCut, scale)}.`,
    );
  }
  if (hole > 0) {
    suggestionLines.push(
      `Or book a Traditional IRA contribution of ${formatUsd(hole, scale)}.`,
    );
  }
  if (hole > 0 && splitCut > 0 && splitContrib > 0) {
    suggestionLines.push(
      `Or both, split: cut remaining Traditional IRA draws by ${formatUsd(splitCut, scale)} and book a Traditional IRA contribution of ${formatUsd(splitContrib, scale)}.`,
    );
  }
  if (hole <= 0 && !isIndeterminate) {
    suggestionLines.push(
      "Forecast is under the cliff on this estimate. Book the estimates to confirm.",
    );
  }
  return {
    after,
    cliff,
    overage,
    over,
    hole,
    iraRemaining: ira.remaining,
    iraCut,
    splitCut,
    splitContrib,
    isIndeterminate,
    isEstimate,
    suggestionLines,
    creditAtRiskMinor: over ? yearAptcMinor : 0,
    gains,
  };
}

export async function syncMagiCliffTask(
  client: {
    executeCommand: (name: string, body?: unknown) => Promise<{ ok: boolean }>;
  },
  plan: TaxPlanningGet,
  magi: MagiProjection | null,
  yearAptcMinor = APPLICATION_APTC_MINOR,
): Promise<void> {
  const forecast = forecastMagi(plan, magi, yearAptcMinor);
  await client.executeCommand("MagiCliffTaskSync", {
    asOfDate: plan.asOfDate,
    overageMinor: forecast.overage,
    creditAtRiskMinor: forecast.creditAtRiskMinor,
    suggestions: forecast.suggestionLines.filter(
      (line) =>
        line.startsWith("Cut remaining") ||
        line.startsWith("Or book") ||
        line.startsWith("Or both"),
    ),
  });
}
