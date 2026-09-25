import { useEffect, useMemo, useState } from "react";
import ReactECharts from "echarts-for-react";
import type { CashRegisterGet, HoldingsGet, TrendsWeekPoint } from "@finos/app-contracts";
import {
  addUtcDays,
  formatFridayEnding,
  formatUsd,
  weekIdContaining,
} from "@finos/ui-components";
import { TRENDS_CASH_ACCOUNT_CHARTS } from "../graphing/TrendsCharts";
import { GRAPH_PERIOD_OPTIONS, type GraphPeriod } from "../graphing/graphPeriod";
import { LocalTauriFinanceClient } from "../../financeClient";

/** One Account cash flow projection for Home and Cash Management Register. Grid/Trends stay on 6m / All data. */
export const HOME_FOCUS_DEFAULT_PERIOD: GraphPeriod = "2m";
export const HOME_FOCUS_DEFAULT_ACCOUNT = "healthCashMinor";

const client = new LocalTauriFinanceClient();
export const HOME_REGISTER_TIMEOUT_MS = 30_000;

function withTimeout<T>(promise: Promise<T>, ms: number): Promise<T> {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error("timeout")), ms);
    promise.then(
      (value) => {
        clearTimeout(timer);
        resolve(value);
      },
      (error) => {
        clearTimeout(timer);
        reject(error);
      },
    );
  });
}

type CashFlowEvent = {
  occurredOn: string;
  name: string;
  amountMinor: number;
  scale: number;
  source: string;
  cashY: number | null;
};

type HomeCashPoint = {
  periodStart: string;
  periodEnd: string;
  cashMinor: number | null;
  scale: number;
};

function startingPlottedCash(
  points: HomeCashPoint[],
  prior?: { minor: number; scale: number; asOf: string } | null,
): { minor: number; scale: number } | null {
  const first = points.find((point) => point.cashMinor != null);
  if (first?.cashMinor != null) {
    return { minor: first.cashMinor, scale: first.scale };
  }
  if (prior) return { minor: prior.minor, scale: prior.scale };
  return null;
}

function endingPlottedCash(
  points: HomeCashPoint[],
): { minor: number; scale: number; periodEnd: string } | null {
  const last = points.at(-1);
  if (last?.cashMinor != null) {
    return { minor: last.cashMinor, scale: last.scale, periodEnd: last.periodEnd };
  }
  for (let i = points.length - 1; i >= 0; i -= 1) {
    if (points[i].cashMinor != null) {
      return {
        minor: points[i].cashMinor as number,
        scale: points[i].scale,
        periodEnd: points[i].periodEnd,
      };
    }
  }
  return null;
}

/** Same Register book for every Home / CM cash-flow dropdown key. */
export const CASH_FLOW_REGISTER_BOOK: Partial<Record<keyof TrendsWeekPoint, string>> = {
  incomeCashMinor: "Income",
  incomeBalanceMinor: "Income",
  rothCashMinor: "FI Roth",
  rothBalanceMinor: "FI Roth",
  carCashMinor: "Car",
  carBalanceMinor: "Car",
  healthCashMinor: "Health",
  healthBalanceMinor: "Health",
  speculationCashMinor: "Speculation",
  speculationBalanceMinor: "Speculation",
  acct9CashMinor: "Account 9",
};

function registerBook(key: keyof TrendsWeekPoint): string | null {
  return CASH_FLOW_REGISTER_BOOK[key] ?? null;
}

if (TRENDS_CASH_ACCOUNT_CHARTS.some((row) => registerBook(row.key) == null)) {
  throw new Error("every cash-flow account must use the same Register fold");
}

function firstOfMonth(iso: string): string {
  return `${iso.slice(0, 7)}-01`;
}

/** Today + N calendar months, same day-of-month (clamped). 20 Sep + 1m = 20 Oct. */
function addMonthsFromDay(iso: string, months: number): string {
  const [year, month, day] = iso.slice(0, 10).split("-").map(Number);
  const cursor = new Date(Date.UTC(year ?? 2026, (month ?? 1) - 1, 1));
  cursor.setUTCMonth(cursor.getUTCMonth() + months);
  const last = new Date(
    Date.UTC(cursor.getUTCFullYear(), cursor.getUTCMonth() + 1, 0),
  ).getUTCDate();
  const useDay = Math.min(day ?? 1, last);
  return `${cursor.getUTCFullYear()}-${String(cursor.getUTCMonth() + 1).padStart(2, "0")}-${String(useDay).padStart(2, "0")}`;
}

const FORWARD_MONTHS: Record<Exclude<GraphPeriod, "ytd" | "all">, number> = {
  "1m": 1,
  "2m": 2,
  "3m": 3,
  "6m": 6,
  "12m": 12,
};

/**
 * Always opens on the 1st of the current month. N months is as-of + N months
 * (1 month from today), not last-of-calendar-month.
 */
export function homeCashWindow(
  asOfIso: string,
  period: GraphPeriod,
): { start: string; end: string } {
  const asOf = asOfIso || new Date().toISOString().slice(0, 10);
  const start = firstOfMonth(asOf);
  if (period === "all") {
    return { start, end: "9999-12-31" };
  }
  if (period === "ytd") {
    return { start, end: `${start.slice(0, 4)}-12-31` };
  }
  return { start, end: addMonthsFromDay(asOf, FORWARD_MONTHS[period]) };
}

/** 19 Sep: 1m is 1 Sep–19 Oct; 2m is 1 Sep–19 Nov. Axis never starts next month. */
export const HOME_CASH_WINDOW_EXAMPLES = [
  { asOf: "2026-09-19", period: "1m" as const, startOn: "2026-09-01", endOn: "2026-10-19" },
  { asOf: "2026-09-19", period: "2m" as const, startOn: "2026-09-01", endOn: "2026-11-19" },
] as const;

if (
  HOME_CASH_WINDOW_EXAMPLES.some((row) => {
    const window = homeCashWindow(row.asOf, row.period);
    return window.start !== row.startOn || window.end !== row.endOn;
  })
) {
  throw new Error("homeCashWindow examples drifted");
}

/** Cash symbol whose open lot is the current total for that Register book. */
export const CASH_FLOW_LOT: Record<string, { account: string; symbol: string }> = {
  Income: { account: "Income", symbol: "SPAXX" },
  "FI Roth": { account: "FI Roth", symbol: "SPAXX" },
  Car: { account: "Car", symbol: "SPAXX" },
  Health: { account: "Health", symbol: "FDRXX" },
  Speculation: { account: "Speculation", symbol: "SPAXX" },
  "Account 9": { account: "9", symbol: "SWVXX" },
};

export function cashLotCents(
  lots: Array<{
    accountName: string;
    symbol: string;
    remainingQuantityMinor: number;
    quantityScale: number;
  }>,
  book: string,
): number | null {
  const spec = CASH_FLOW_LOT[book];
  if (!spec) return null;
  const rows = lots.filter(
    (lot) =>
      lot.accountName === spec.account &&
      lot.symbol.toUpperCase() === spec.symbol &&
      lot.remainingQuantityMinor > 0,
  );
  if (rows.length === 0) return null;
  return rows.reduce((sum, lot) => {
    const denom = 10 ** lot.quantityScale;
    return sum + (denom ? Math.round((lot.remainingQuantityMinor * 100) / denom) : 0);
  }, 0);
}

/**
 * Line starts at the cash lot on as-of. Only deposits and withdrawals after
 * that day move later Fridays. Weeks before as-of are omitted.
 */
export function cashFlowFromLot(
  lotMinor: number,
  asOf: string,
  windowEnd: string,
  events: Array<{ occurredOn: string; amountMinor: number }>,
): HomeCashPoint[] {
  const start = asOf.slice(0, 10);
  const end = windowEnd.slice(0, 10);
  const points: HomeCashPoint[] = [
    { periodStart: start, periodEnd: start, cashMinor: lotMinor, scale: 2 },
  ];
  if (!start || end <= start) return points;
  const future = events.filter((event) => {
    const day = event.occurredOn.slice(0, 10);
    return day > start && day <= end;
  });
  let sat = weekIdContaining(start).start;
  for (let i = 0; i < 80; i += 1) {
    const id = weekIdContaining(sat);
    if (id.end > end) break;
    if (id.end > start) {
      const net = future
        .filter((event) => event.occurredOn.slice(0, 10) <= id.end)
        .reduce((sum, event) => sum + event.amountMinor, 0);
      points.push({
        periodStart: id.start,
        periodEnd: id.end,
        cashMinor: lotMinor + net,
        scale: 2,
      });
    }
    sat = addUtcDays(sat, 7);
  }
  return points;
}

/** Lot $1,000 on 19 Sep. A 19 Sep debit stays out. 25 Sep is $1,000 + $50 − $200. */
export const CASH_FLOW_FROM_LOT_EXAMPLE = {
  asOf: "2026-09-19",
  lotMinor: 100_000,
  windowEnd: "2026-11-19",
  sameDayDebit: { occurredOn: "2026-09-19", amountMinor: -20_000 },
  deposit: { occurredOn: "2026-09-25", amountMinor: 5_000 },
  debit: { occurredOn: "2026-09-24", amountMinor: -20_000 },
  friday: "2026-09-25",
  fridayMinor: 85_000,
} as const;

if (
  (() => {
    const ex = CASH_FLOW_FROM_LOT_EXAMPLE;
    const points = cashFlowFromLot(ex.lotMinor, ex.asOf, ex.windowEnd, [
      ex.sameDayDebit,
      ex.deposit,
      ex.debit,
    ]);
    const open = points[0];
    const friday = points.find((point) => point.periodEnd === ex.friday);
    return (
      open?.cashMinor !== ex.lotMinor ||
      open.periodEnd !== ex.asOf ||
      friday?.cashMinor !== ex.fridayMinor ||
      points.some((point) => point.periodEnd < ex.asOf)
    );
  })()
) {
  throw new Error("cashFlowFromLot example drifted");
}

/** Header totals: as-of → duration end. Hits before today stay off Planned Income. */
export function periodCashFlowTotals(
  events: Array<{
    occurredOn: string;
    amountMinor: number;
    scale: number;
    source: string;
  }>,
  fromDay: string,
  toDay: string,
): { incomeMinor: number; withdrawalMinor: number; scale: number } {
  const from = fromDay.slice(0, 10);
  const to = toDay.slice(0, 10);
  let incomeMinor = 0;
  let withdrawalMinor = 0;
  let scale = 2;
  for (const event of events) {
    const day = event.occurredOn.slice(0, 10);
    if (day < from || day > to) continue;
    scale = event.scale || scale;
    if (event.amountMinor > 0 && event.source === "income-plan") {
      incomeMinor += event.amountMinor;
    } else if (event.amountMinor < 0) {
      withdrawalMinor += Math.abs(event.amountMinor);
    }
  }
  return { incomeMinor, withdrawalMinor, scale };
}

/** 20 Sep 12m: Sep 5 dividend is before as-of; Oct income and Oct debit count. */
export const HOME_CASH_PERIOD_TOTALS_EXAMPLE = {
  asOf: "2026-09-20",
  end: "2027-09-20",
  before: {
    occurredOn: "2026-09-05",
    amountMinor: 10_000,
    scale: 2,
    source: "income-plan",
  },
  income: {
    occurredOn: "2026-10-15",
    amountMinor: 50_000,
    scale: 2,
    source: "income-plan",
  },
  withdraw: {
    occurredOn: "2026-10-01",
    amountMinor: -20_000,
    scale: 2,
    source: "element",
  },
  incomeMinor: 50_000,
  withdrawalMinor: 20_000,
} as const;

if (
  (() => {
    const ex = HOME_CASH_PERIOD_TOTALS_EXAMPLE;
    const got = periodCashFlowTotals(
      [ex.before, ex.income, ex.withdraw],
      ex.asOf,
      ex.end,
    );
    return got.incomeMinor !== ex.incomeMinor || got.withdrawalMinor !== ex.withdrawalMinor;
  })()
) {
  throw new Error("periodCashFlowTotals example drifted");
}

/** Snapshot for this Friday: exact Friday first, then Saturday (or any day) in that Sat–Fri week. */
export function weekSnapshot(
  weeks: TrendsWeekPoint[],
  friday: string,
): TrendsWeekPoint | undefined {
  const id = weekIdContaining(friday);
  const exact = weeks.find((week) => week.periodEnd.slice(0, 10) === id.end);
  if (exact) return exact;
  return weeks.find((week) => {
    const end = week.periodEnd.slice(0, 10);
    const start = (week.periodStart ?? "").slice(0, 10);
    if (start === id.start) return true;
    return weekIdContaining(end).end === id.end;
  });
}

function cashMinorOnWeek(
  week: TrendsWeekPoint | undefined,
  key: keyof TrendsWeekPoint,
): number | null {
  if (!week) return null;
  const raw = week[key];
  // rust i64 / blank Accept is 0. That is not a confirmed Friday cash.
  if (typeof raw !== "number" || raw === 0) return null;
  return raw;
}

/** One Sat–Fri slot per week that overlaps the window. Seed cash sits on Friday; Saturday elements map into that week. */
export function homeCashPoints(
  weeks: TrendsWeekPoint[],
  key: keyof TrendsWeekPoint,
  period: GraphPeriod,
  asOfIso?: string,
): HomeCashPoint[] {
  const { start, end } = homeCashWindow(asOfIso ?? "", period);
  const points: HomeCashPoint[] = [];
  let sat = weekIdContaining(start).start;
  for (let i = 0; i < 80; i += 1) {
    const id = weekIdContaining(sat);
    if (id.start > end) break;
    if (id.end >= start && id.start <= end) {
      const saved = weekSnapshot(weeks, id.end);
      points.push({
        periodStart: id.start,
        periodEnd: id.end,
        cashMinor: cashMinorOnWeek(saved, key),
        scale: saved?.scale ?? 2,
      });
    }
    sat = addUtcDays(sat, 7);
  }
  return points;
}

/** Last snapshot Friday before `beforeDay` — opening cash when the visible month has no Friday fact yet. */
export function lastKnownCashFact(
  weeks: TrendsWeekPoint[],
  key: keyof TrendsWeekPoint,
  beforeDay: string,
): { minor: number; scale: number; asOf: string } | null {
  const known = weeks
    .map((week) => {
      const minor = cashMinorOnWeek(week, key);
      if (minor == null) return null;
      const friday = weekIdContaining(week.periodEnd.slice(0, 10)).end;
      if (friday >= beforeDay) return null;
      return { minor, scale: week.scale ?? 2, asOf: friday };
    })
    .filter((row): row is { minor: number; scale: number; asOf: string } => row != null)
    .sort((a, b) => a.asOf.localeCompare(b.asOf));
  return known.at(-1) ?? null;
}

export function homeCashAxisEnd(windowEnd: string, points: HomeCashPoint[]): string {
  const last = points.at(-1)?.periodEnd;
  return last && last > windowEnd ? last : windowEnd;
}

/** Latest known cash for the book — used when the payable week has no snapshot yet. */
export function lastKnownCashY(
  weeks: TrendsWeekPoint[],
  key: keyof TrendsWeekPoint,
): number | null {
  const known = weeks
    .map((week) => {
      const minor = cashMinorOnWeek(week, key);
      if (minor == null) return null;
      return { periodEnd: week.periodEnd, minor, scale: week.scale ?? 2 };
    })
    .filter((row): row is { periodEnd: string; minor: number; scale: number } => row != null)
    .sort((a, b) => a.periodEnd.localeCompare(b.periodEnd));
  const last = known.at(-1);
  if (!last) return null;
  return last.minor / 10 ** last.scale;
}

/**
 * Confirmed Friday cash is the base. Each later week is a projection:
 * prior cash + Income Plan planned dividends − future debits (elements,
 * posted actuals). A week without Accept is still a number. Skip Adjust only.
 * Unknown only when there has never been a confirmed cash to start from.
 */
export function projectHomeCashPoints(
  points: HomeCashPoint[],
  events: CashFlowEvent[],
  prior?: { minor: number; scale: number; asOf: string } | null,
): HomeCashPoint[] {
  let running: number | null = null;
  let cursor = "";
  let scale = 2;
  if (prior) {
    running = prior.minor;
    cursor = prior.asOf;
    scale = prior.scale;
  }
  return points.map((point) => {
    if (point.cashMinor != null) {
      running = point.cashMinor;
      cursor = point.periodEnd;
      scale = point.scale;
      return point;
    }
    if (running == null) return point;
    const net = events
      .filter((event) => {
        const day = event.occurredOn.slice(0, 10);
        return day > cursor && day <= point.periodEnd;
      })
      .reduce((sum, event) => sum + event.amountMinor, 0);
    running += net;
    cursor = point.periodEnd;
    return { ...point, cashMinor: running, scale };
  });
}

/** Dot Y is this week's Friday line cash. Do not invent a second series. */
export function eventProjectedCashY(
  points: HomeCashPoint[],
  events: CashFlowEvent[],
  occurredOn: string,
): number | null {
  const day = occurredOn.slice(0, 10);
  const friday = weekIdContaining(day).end;
  const weekPoint = points.find((point) => point.periodEnd === friday);
  if (weekPoint?.cashMinor != null) {
    return weekPoint.cashMinor / 10 ** weekPoint.scale;
  }
  const fact = [...points]
    .reverse()
    .find((point) => point.cashMinor != null && point.periodEnd < friday);
  if (!fact || fact.cashMinor == null) return lastKnownCashYFromPoints(points);
  const add = events
    .filter((event) => {
      const on = event.occurredOn.slice(0, 10);
      return on > fact.periodEnd && on <= day;
    })
    .reduce((sum, event) => sum + event.amountMinor, 0);
  return (fact.cashMinor + add) / 10 ** fact.scale;
}

export function formatProjectedCash(cashY: number): string {
  return `Projected cash ${formatUsd(Math.round(cashY * 100), 2)}`;
}

/** Line points are `[day, cash]` or `{ value: [day, cash] }`. Missing value is not unknown when cash is present. */
export function cashLineHoverValue(
  data: unknown,
): { day: string; cash: number } | null {
  const raw = data as { value?: [string, number] } | [string, number] | undefined;
  const pair = Array.isArray(raw) ? raw : raw?.value;
  const day = String(pair?.[0] ?? "").slice(0, 10);
  const cash = pair?.[1];
  if (!day || typeof cash !== "number") return null;
  return { day, cash };
}

function lastKnownCashYFromPoints(points: HomeCashPoint[]): number | null {
  for (let i = points.length - 1; i >= 0; i -= 1) {
    if (points[i].cashMinor != null) {
      return (points[i].cashMinor as number) / 10 ** points[i].scale;
    }
  }
  return null;
}

/** Sep 18 $1,000 fact + Sep 25 $50 Income Plan → Friday 25 Sep is $1,050, not $1,000. */
export const HOME_CASH_PROJECTION_EXAMPLE = {
  factFriday: "2026-09-18",
  factMinor: 100_000,
  payOn: "2026-09-25",
  depositMinor: 5_000,
  projectedFriday: "2026-09-25",
  projectedMinor: 105_000,
} as const;

/** Same week: planned dividend adds, Saturday debit reduces. $1,000 + $50 − $200 = $850. */
export const HOME_CASH_NET_PROJECTION_EXAMPLE = {
  factFriday: "2026-09-18",
  factMinor: 100_000,
  payOn: "2026-09-25",
  depositMinor: 5_000,
  debitOn: "2026-09-19",
  debitMinor: -20_000,
  projectedFriday: "2026-09-25",
  projectedMinor: 85_000,
} as const;

/** 2m window last Friday is a dollar amount, not unknown. Later weeks carry $850. */
export const HOME_CASH_CHART_END_EXAMPLE = {
  asOf: "2026-09-19",
  period: "2m" as const,
  lastFriday: "2026-11-20",
  endingMinor: 85_000,
  debitProjectedY: 850,
} as const;

/** 1m from 19 Sep is 1 Sep–19 Oct; Saturday-keyed cash and Fri 2 Oct stay in. */
export const HOME_CASH_1M_ENDING_EXAMPLE = {
  asOf: "2026-09-19",
  period: "1m" as const,
  saturdayKeyedEnd: "2026-09-19",
  mapsToFriday: "2026-09-25",
  overlappingFriday: "2026-10-02",
  priorFriday: "2026-08-28",
  priorMinor: 100_000,
} as const;

if (
  (() => {
    const satWeek = {
      periodEnd: HOME_CASH_1M_ENDING_EXAMPLE.saturdayKeyedEnd,
      periodStart: "2026-09-19",
      incomeCashMinor: 88_000,
      scale: 2,
    } as TrendsWeekPoint;
    const satPoints = homeCashPoints(
      [satWeek],
      "incomeCashMinor",
      HOME_CASH_1M_ENDING_EXAMPLE.period,
      HOME_CASH_1M_ENDING_EXAMPLE.asOf,
    );
    if (
      satPoints.find((point) => point.periodEnd === HOME_CASH_1M_ENDING_EXAMPLE.mapsToFriday)
        ?.cashMinor !== 88_000
    ) {
      return true;
    }
    if (
      !satPoints.some(
        (point) => point.periodEnd === HOME_CASH_1M_ENDING_EXAMPLE.overlappingFriday,
      )
    ) {
      return true;
    }
    const octWeek = {
      periodEnd: HOME_CASH_1M_ENDING_EXAMPLE.overlappingFriday,
      periodStart: "2026-09-26",
      incomeCashMinor: 99_000,
      scale: 2,
    } as TrendsWeekPoint;
    const octPoints = homeCashPoints(
      [octWeek],
      "incomeCashMinor",
      HOME_CASH_1M_ENDING_EXAMPLE.period,
      HOME_CASH_1M_ENDING_EXAMPLE.asOf,
    );
    if (
      octPoints.find(
        (point) => point.periodEnd === HOME_CASH_1M_ENDING_EXAMPLE.overlappingFriday,
      )?.cashMinor !== 99_000
    ) {
      return true;
    }
    const carried = projectHomeCashPoints(
      [
        { periodStart: "2026-08-29", periodEnd: "2026-09-04", cashMinor: null, scale: 2 },
        { periodStart: "2026-09-19", periodEnd: "2026-09-25", cashMinor: null, scale: 2 },
      ],
      [],
      {
        minor: HOME_CASH_1M_ENDING_EXAMPLE.priorMinor,
        scale: 2,
        asOf: HOME_CASH_1M_ENDING_EXAMPLE.priorFriday,
      },
    );
    if (carried[1]?.cashMinor !== HOME_CASH_1M_ENDING_EXAMPLE.priorMinor) return true;
    const blankLater = homeCashPoints(
      [
        {
          periodEnd: HOME_CASH_PROJECTION_EXAMPLE.factFriday,
          periodStart: "2026-09-12",
          incomeCashMinor: HOME_CASH_PROJECTION_EXAMPLE.factMinor,
          scale: 2,
        } as TrendsWeekPoint,
        {
          periodEnd: HOME_CASH_PROJECTION_EXAMPLE.projectedFriday,
          periodStart: "2026-09-19",
          incomeCashMinor: 0,
          scale: 2,
        } as TrendsWeekPoint,
      ],
      "incomeCashMinor",
      "2m",
      HOME_CASH_WINDOW_EXAMPLES[1].asOf,
    );
    if (
      blankLater.find(
        (point) => point.periodEnd === HOME_CASH_PROJECTION_EXAMPLE.projectedFriday,
      )?.cashMinor != null
    ) {
      return true;
    }
    const points: HomeCashPoint[] = [
      {
        periodStart: "2026-09-12",
        periodEnd: HOME_CASH_PROJECTION_EXAMPLE.factFriday,
        cashMinor: HOME_CASH_PROJECTION_EXAMPLE.factMinor,
        scale: 2,
      },
      {
        periodStart: "2026-09-19",
        periodEnd: HOME_CASH_PROJECTION_EXAMPLE.projectedFriday,
        cashMinor: null,
        scale: 2,
      },
    ];
    const events: CashFlowEvent[] = [
      {
        occurredOn: HOME_CASH_PROJECTION_EXAMPLE.payOn,
        name: "QYLD",
        amountMinor: HOME_CASH_PROJECTION_EXAMPLE.depositMinor,
        scale: 2,
        source: "income-plan",
        cashY: null,
      },
    ];
    const projected = projectHomeCashPoints(points, events);
    if (projected[1]?.cashMinor !== HOME_CASH_PROJECTION_EXAMPLE.projectedMinor) {
      return true;
    }
    const netted = projectHomeCashPoints(points, [
      ...events,
      {
        occurredOn: HOME_CASH_NET_PROJECTION_EXAMPLE.debitOn,
        name: "net",
        amountMinor: HOME_CASH_NET_PROJECTION_EXAMPLE.debitMinor,
        scale: 2,
        source: "element",
        cashY: null,
      },
    ]);
    if (netted[1]?.cashMinor !== HOME_CASH_NET_PROJECTION_EXAMPLE.projectedMinor) {
      return true;
    }
    const fromBlank = projectHomeCashPoints(blankLater, events);
    if (
      fromBlank.find(
        (point) => point.periodEnd === HOME_CASH_PROJECTION_EXAMPLE.projectedFriday,
      )?.cashMinor !== HOME_CASH_PROJECTION_EXAMPLE.projectedMinor
    ) {
      return true;
    }
    const windowPoints = homeCashPoints(
      [
        {
          periodEnd: HOME_CASH_NET_PROJECTION_EXAMPLE.factFriday,
          periodStart: "2026-09-12",
          incomeCashMinor: HOME_CASH_NET_PROJECTION_EXAMPLE.factMinor,
          scale: 2,
        } as TrendsWeekPoint,
      ],
      "incomeCashMinor",
      HOME_CASH_CHART_END_EXAMPLE.period,
      HOME_CASH_CHART_END_EXAMPLE.asOf,
    );
    const chartEnd = projectHomeCashPoints(windowPoints, [
      ...events,
      {
        occurredOn: HOME_CASH_NET_PROJECTION_EXAMPLE.debitOn,
        name: "net",
        amountMinor: HOME_CASH_NET_PROJECTION_EXAMPLE.debitMinor,
        scale: 2,
        source: "element",
        cashY: null,
      },
    ]);
    const last = endingPlottedCash(chartEnd);
    if (
      last == null ||
      last.periodEnd !== HOME_CASH_CHART_END_EXAMPLE.lastFriday ||
      last.minor !== HOME_CASH_CHART_END_EXAMPLE.endingMinor
    ) {
      return true;
    }
    const firstFact = chartEnd.findIndex((point) => point.cashMinor != null);
    if (firstFact < 0 || chartEnd.slice(firstFact).some((point) => point.cashMinor == null)) {
      return true;
    }
    const debitY = eventProjectedCashY(chartEnd, [
      ...events,
      {
        occurredOn: HOME_CASH_NET_PROJECTION_EXAMPLE.debitOn,
        name: "net",
        amountMinor: HOME_CASH_NET_PROJECTION_EXAMPLE.debitMinor,
        scale: 2,
        source: "element",
        cashY: null,
      },
    ], HOME_CASH_NET_PROJECTION_EXAMPLE.debitOn);
    if (debitY !== HOME_CASH_CHART_END_EXAMPLE.debitProjectedY) return true;
    const hoverTuple = cashLineHoverValue([
      HOME_CASH_CHART_END_EXAMPLE.lastFriday,
      HOME_CASH_CHART_END_EXAMPLE.endingMinor / 100,
    ]);
    const hoverObj = cashLineHoverValue({
      value: [
        HOME_CASH_CHART_END_EXAMPLE.lastFriday,
        HOME_CASH_CHART_END_EXAMPLE.endingMinor / 100,
      ],
    });
    return (
      hoverTuple?.cash !== HOME_CASH_CHART_END_EXAMPLE.debitProjectedY ||
      hoverObj?.cash !== HOME_CASH_CHART_END_EXAMPLE.debitProjectedY ||
      hoverTuple?.day !== HOME_CASH_CHART_END_EXAMPLE.lastFriday
    );
  })()
) {
  throw new Error("projectHomeCashPoints example drifted");
}

type CombinedCashHit = {
  day: string;
  weekStart: string;
  debit: boolean;
  cashY: number;
  scale: number;
  hits: CashFlowEvent[];
};

/** One green and/or one red dot per Sat–Fri week. Hover lists every hit of that color plus Total. */
export function combineLikeColorDots(
  events: CashFlowEvent[],
  fallbackY?: number | null,
): CombinedCashHit[] {
  const groups = new Map<string, CombinedCashHit>();
  for (const event of events) {
    const cashY = event.cashY ?? fallbackY ?? null;
    if (cashY == null) continue;
    const week = weekIdContaining(event.occurredOn);
    const debit = event.amountMinor < 0;
    const key = `${week.end}|${debit ? "debit" : "credit"}`;
    const existing = groups.get(key);
    if (existing) {
      existing.hits.push(event);
      existing.cashY = cashY;
      continue;
    }
    groups.set(key, {
      day: week.end,
      weekStart: week.start,
      debit,
      cashY,
      scale: event.scale,
      hits: [event],
    });
  }
  for (const group of groups.values()) {
    group.hits.sort((a, b) =>
      a.occurredOn === b.occurredOn
        ? a.name.localeCompare(b.name)
        : a.occurredOn.localeCompare(b.occurredOn),
    );
  }
  return [...groups.values()].sort((a, b) =>
    a.day === b.day ? Number(a.debit) - Number(b.debit) : a.day.localeCompare(b.day),
  );
}

function cashFlowChartOption(
  points: HomeCashPoint[],
  events: CashFlowEvent[],
  color: string,
  windowStart: string,
  windowEnd: string,
) {
  const line = points
    .filter((point) => point.cashMinor != null)
    .map((point) => [
      point.periodEnd,
      (point.cashMinor as number) / 10 ** point.scale,
    ]);
  const fallbackY = endingPlottedCash(points);
  const scatter = combineLikeColorDots(
    events,
    fallbackY == null ? null : fallbackY.minor / 10 ** fallbackY.scale,
  ).map((group) => ({
    value: [`${group.day}T${group.debit ? "15" : "12"}:00:00Z`, group.cashY],
    occurredOn: group.day,
    weekStart: group.weekStart,
    debit: group.debit,
    hits: group.hits,
    projectedCash: formatProjectedCash(group.cashY),
    itemStyle: { color: group.debit ? "#b42318" : "#1b6b4a" },
  }));
  return {
    tooltip: {
      trigger: "item",
      formatter: (param: {
        seriesName?: string;
        value?: [string, number];
        data?: {
          occurredOn?: string;
          weekStart?: string;
          debit?: boolean;
          hits?: CashFlowEvent[];
          projectedCash?: string;
          value?: [string, number];
        } | [string, number];
      }) => {
        if (param.seriesName === "Cash") {
          const hover =
            cashLineHoverValue(param.data) ?? cashLineHoverValue(param.value);
          const day = hover?.day ?? "";
          const point = points.find((row) => row.periodEnd === day);
          const cashText =
            hover == null
              ? "unknown"
              : formatUsd(Math.round(hover.cash * 100), 2);
          return [
            point
              ? `${formatFridayEnding(day)} · Sat ${point.periodStart} – Fri ${point.periodEnd}`
              : day,
            `Ending cash ${cashText}`,
          ].join("<br/>");
        }
        const group = param.data;
        const friday = group?.occurredOn ?? "";
        const sat = group?.weekStart ?? weekIdContaining(friday).start;
        const hits = group?.hits ?? [];
        const tone = group?.debit ? "Withdrawals" : "Deposits";
        const scale = hits[0]?.scale ?? 2;
        const totalMinor = hits.reduce((sum, event) => sum + event.amountMinor, 0);
        const lines = [
          `${formatFridayEnding(friday)} · Sat ${sat} – Fri ${friday} · ${tone} (${hits.length})`,
          ...hits.map(
            (event) =>
              `${event.occurredOn.slice(0, 10)} ${event.name} ${formatUsd(event.amountMinor, event.scale)}`,
          ),
          `Total ${formatUsd(totalMinor, scale)}`,
          group?.projectedCash ?? formatProjectedCash(group?.value?.[1] ?? 0),
        ];
        return lines.filter(Boolean).join("<br/>");
      },
    },
    grid: { left: 48, right: 12, top: 12, bottom: 28 },
    xAxis: {
      type: "time",
      min: `${windowStart}T00:00:00Z`,
      max: `${windowEnd}T23:59:59Z`,
      axisLabel: {
        hideOverlap: true,
        fontSize: 10,
        showMinLabel: true,
        showMaxLabel: true,
      },
    },
    yAxis: { type: "value", scale: true, axisLabel: { fontSize: 10 } },
    series: [
      {
        name: "Cash",
        type: "line",
        data: line,
        showSymbol: true,
        symbolSize: 6,
        connectNulls: false,
        z: 1,
        lineStyle: { width: 2, color },
        itemStyle: { color },
      },
      {
        name: "Cash hits",
        type: "scatter",
        symbolSize: 10,
        z: 3,
        data: scatter,
      },
    ],
  };
}

export function AccountCashFlow({
  weeks,
  asOf,
}: {
  weeks: TrendsWeekPoint[] | null | undefined;
  asOf?: string;
}) {
  const [accountKey, setAccountKey] = useState<keyof TrendsWeekPoint>(
    HOME_FOCUS_DEFAULT_ACCOUNT,
  );
  const [period, setPeriod] = useState<GraphPeriod>(HOME_FOCUS_DEFAULT_PERIOD);
  const [events, setEvents] = useState<CashFlowEvent[]>([]);
  const [lotByBook, setLotByBook] = useState<Record<string, number | null> | null>(null);
  const selected =
    TRENDS_CASH_ACCOUNT_CHARTS.find((row) => row.key === accountKey) ??
    TRENDS_CASH_ACCOUNT_CHARTS.find((row) => row.key === HOME_FOCUS_DEFAULT_ACCOUNT) ??
    TRENDS_CASH_ACCOUNT_CHARTS[0];
  const window = useMemo(
    () => homeCashWindow(asOf ?? "", period),
    [asOf, period],
  );
  const asOfDay = (asOf ?? "").slice(0, 10);
  const book = registerBook(selected.key);
  const lotMinor = book == null || lotByBook == null ? null : (lotByBook[book] ?? null);
  const forwardEvents = useMemo(
    () => events.filter((event) => event.occurredOn.slice(0, 10) > asOfDay),
    [events, asOfDay],
  );
  const visible = useMemo(() => {
    if (lotMinor == null || !asOfDay) return [];
    return cashFlowFromLot(lotMinor, asOfDay, window.end, forwardEvents);
  }, [lotMinor, asOfDay, window.end, forwardEvents]);
  const axisEnd = window.end;
  const plottedEvents = useMemo(
    () =>
      forwardEvents.map((event) => ({
        ...event,
        cashY: eventProjectedCashY(visible, forwardEvents, event.occurredOn),
      })),
    [forwardEvents, visible],
  );
  const starting = lotMinor == null ? null : { minor: lotMinor, scale: 2 };
  const ending = endingPlottedCash(visible);
  const startValue =
    starting == null ? "unknown" : formatUsd(starting.minor, starting.scale);
  const endValue = ending == null ? "unknown" : formatUsd(ending.minor, ending.scale);
  const totals = useMemo(
    () => periodCashFlowTotals(forwardEvents, asOfDay, window.end),
    [forwardEvents, asOfDay, window.end],
  );
  const data = visible.map((point) =>
    point.cashMinor == null ? null : point.cashMinor / 10 ** point.scale,
  );
  const hasChart = data.some((v) => v != null);

  useEffect(() => {
    let cancelled = false;
    void client.executeQuery("HoldingsGet").then((result) => {
      if (cancelled) return;
      if (!result.ok || !result.bodyJson) {
        setLotByBook({});
        return;
      }
      const body = JSON.parse(result.bodyJson) as HoldingsGet;
      const next: Record<string, number | null> = {};
      for (const name of Object.keys(CASH_FLOW_LOT)) {
        next[name] = cashLotCents(body.lots ?? [], name);
      }
      setLotByBook(next);
    }).catch(() => {
      if (!cancelled) setLotByBook({});
    });
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (!book || !asOf) {
      if (weeks == null) setEvents([]);
      setEvents([]);
      return;
    }
    let cancelled = false;
    void withTimeout(
      client.executeQuery("CashRegisterGet", {
        asOfDate: asOf,
        account: book,
        period: "1Y",
        periodStart: window.start,
        periodEnd: axisEnd,
        includeUnconfirmedPast: true,
        hitsOnly: true,
      }),
      HOME_REGISTER_TIMEOUT_MS,
    )
      .then((result) => {
        if (cancelled) return;
        if (!result.ok || !result.bodyJson) {
          setEvents([]);
          return;
        }
        const body = JSON.parse(result.bodyJson) as CashRegisterGet;
        const scale = body.scale ?? 2;
        const next: CashFlowEvent[] = [];
        for (const row of body.rows ?? []) {
          // Keep every Register cash hit: elements, Income Plan planned dividends,
          // posted dividend actuals. Skip Adjust only.
          const adjust =
            row.source === "adjust" ||
            row.transaction === "Cash_Adjust" ||
            row.label.trim().toLowerCase().startsWith("adjust");
          if (adjust) continue;
          if (!row.label && !row.transaction) continue;
          const amount =
            (row.depositMinor ?? 0) > 0
              ? row.depositMinor
              : -Math.abs(row.withdrawalMinor ?? 0);
          if (amount === 0) continue;
          const day = row.occurredOn.slice(0, 10);
          if (day < window.start || day > axisEnd) continue;
          next.push({
            occurredOn: row.occurredOn,
            name: row.label.trim() || row.transaction,
            amountMinor: amount,
            scale: row.scale ?? scale,
            source: row.source ?? "",
            cashY: null,
          });
        }
        setEvents(next);
      })
      .catch(() => {
        if (!cancelled) setEvents([]);
      });
    return () => {
      cancelled = true;
    };
  }, [book, asOf, weeks, period, selected.key, window.start, axisEnd]);

  if (lotByBook == null) {
    return (
      <section className="home-trend-focus" aria-label="Account cash flow projection">
        <h2>Account cash flow projection</h2>
        <p role="status">Loading account trend…</p>
      </section>
    );
  }

  return (
    <section className="home-trend-focus" aria-label="Account cash flow projection">
      <header className="home-trend-focus-title">
        <div className="home-trend-focus-top">
        <h2>Account cash flow projection</h2>
        <div className="home-trend-focus-controls">
        <select
          aria-label="Account trend account"
          value={String(selected.key)}
          onChange={(e) => setAccountKey(e.target.value as keyof TrendsWeekPoint)}
        >
          {TRENDS_CASH_ACCOUNT_CHARTS.map((row) => (
            <option key={String(row.key)} value={String(row.key)}>
              {row.title}
            </option>
          ))}
        </select>
        <select
          aria-label="Account trend duration"
          value={period}
          onChange={(e) => setPeriod(e.target.value as GraphPeriod)}
        >
          {GRAPH_PERIOD_OPTIONS.map((opt) => (
            <option key={opt.value} value={opt.value}>
              {opt.label}
            </option>
          ))}
        </select>
        </div>
        </div>
        <div className="home-trend-focus-flow">
          <span className="home-trend-focus-label">Planned Income</span>
          <span className="home-av-value is-income" aria-label="Planned income for period">
            {formatUsd(totals.incomeMinor, totals.scale)}
          </span>
          <span className="home-trend-focus-label">Starting Balance</span>
          <span className="home-av-value" aria-label="Starting balance for period">
            {startValue}
          </span>
          <span className="home-trend-focus-label">Planned withdrawals</span>
          <span className="home-av-value is-withdraw" aria-label="Planned withdrawals for period">
            {formatUsd(totals.withdrawalMinor, totals.scale)}
          </span>
          <span className="home-trend-focus-label">Ending balance</span>
          <span className="home-av-value" aria-label="Ending plotted cash">
            {endValue}
          </span>
        </div>
      </header>
      {lotMinor == null || !hasChart ? (
        <p className="home-av-empty">
          {lotMinor == null
            ? "No cash lot for this account."
            : "No cash points in this duration."}
        </p>
      ) : (
        <div className="home-trend-focus-chart">
          <ReactECharts
            option={cashFlowChartOption(
              visible,
              plottedEvents,
              selected.color,
              window.start,
              axisEnd,
            )}
            style={{ height: "100%", width: "100%" }}
            opts={{ renderer: "canvas" }}
            notMerge
          />
        </div>
      )}
    </section>
  );
}

/** Home and older imports keep this name; implementation is AccountCashFlow. */
export const HomeAccountTrendFocus = AccountCashFlow;
