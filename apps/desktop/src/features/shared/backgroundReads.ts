import type {
  CashElementListGet,
  DividendPerformanceGet,
  IncomePlanGridGet,
  TaxPlanningGet,
} from "@finos/app-contracts";
import { LocalTauriFinanceClient } from "../../financeClient";

export type BackgroundReadMemory = {
  grid?: IncomePlanGridGet;
  perf?: DividendPerformanceGet;
  elements?: CashElementListGet;
  tax?: TaxPlanningGet;
};

function sameAccounts(left: string[], right: string[]): boolean {
  if (left.length !== right.length) return false;
  const a = [...left].sort();
  const b = [...right].sort();
  return a.every((name, i) => name === b[i]);
}

export function incomeGridMemoryMatches(
  grid: IncomePlanGridGet | null,
  asOf: string,
  hist: number,
  fut: number,
  accounts: string[],
): boolean {
  if (!grid) return false;
  return (
    grid.asOfDate === asOf
    && grid.historicalWeeks === hist
    && grid.futureWeeks === fut
    && sameAccounts(grid.selectedAccounts ?? [], accounts)
  );
}

function parseBody<T>(ok: boolean, bodyJson?: string): T | undefined {
  if (!ok || !bodyJson) return undefined;
  try {
    return JSON.parse(bodyJson) as T;
  } catch {
    return undefined;
  }
}

/** One read at a time. Week Ahead, Register, Coverage, and Cash YTD stay off this list. */
export async function runBackgroundReads(
  client: LocalTauriFinanceClient,
  args: {
    asOf: string;
    hist: number;
    fut: number;
    accounts: string[];
    range: string;
    haveGrid: boolean;
    havePerf: boolean;
    haveElements: boolean;
    haveTax: boolean;
  },
  cancelled: () => boolean,
): Promise<BackgroundReadMemory> {
  const out: BackgroundReadMemory = {};
  if (!args.haveGrid) {
    if (cancelled()) return out;
    const grid = await client.executeQuery("IncomePlanGridGet", {
      asOfDate: args.asOf,
      historicalWeeks: args.hist,
      futureWeeks: args.fut,
      accounts: args.accounts,
      weekEnding: args.asOf,
    });
    out.grid = parseBody<IncomePlanGridGet>(grid.ok, grid.bodyJson);
  }
  if (!args.havePerf) {
    if (cancelled()) return out;
    const perf = await client.executeQuery("DividendPerformanceGet", {
      asOfDate: args.asOf,
      range: args.range,
    });
    out.perf = parseBody<DividendPerformanceGet>(perf.ok, perf.bodyJson);
  }
  if (!args.haveElements) {
    if (cancelled()) return out;
    const elements = await client.executeQuery("CashElementListGet", {
      account: "all",
      asOfDate: args.asOf,
    });
    out.elements = parseBody<CashElementListGet>(elements.ok, elements.bodyJson);
  }
  if (!args.haveTax) {
    if (cancelled()) return out;
    const tax = await client.executeQuery("TaxPlanningGet", {
      asOfDate: args.asOf,
    });
    out.tax = parseBody<TaxPlanningGet>(tax.ok, tax.bodyJson);
  }
  return out;
}
