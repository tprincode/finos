import type { CashElementRecord } from "@finos/app-contracts";

export const MAGI_DRAW_ACCOUNTS = ["Income", "Speculation", "Account 9"] as const;

function isWithdrawal(kind: string): boolean {
  return kind.toLowerCase() !== "deposit";
}

export function magiFutureUnconfirmedWithdrawal(
  element: CashElementRecord,
  asOfDate: string,
): boolean {
  if (!isWithdrawal(element.kind)) return false;
  if (!MAGI_DRAW_ACCOUNTS.some((name) => name === element.account)) return false;
  const asOf = asOfDate.slice(0, 10);
  const upcoming = element.upcoming ?? [];
  if (upcoming.length > 0) {
    return upcoming.some(
      (row) => !row.isCancelled && row.occurredOn.slice(0, 10) >= asOf,
    );
  }
  const next = element.nextOccurredOn ?? "";
  return next.slice(0, 10) >= asOf && next.length >= 10;
}
