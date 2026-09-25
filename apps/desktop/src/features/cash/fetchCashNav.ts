import type {
  CashElementListGet,
  CashRegisterGet,
  CashYtdGet,
  TaxPlanningGet,
} from "@finos/app-contracts";
import { LocalTauriFinanceClient } from "../../financeClient";

export function magiQualifyingType(activityType?: string | null): boolean {
  return (
    activityType === "IRA_Distribution" || activityType === "Roth_Distribution"
  );
}

export async function fetchCashNav(
  client: LocalTauriFinanceClient,
  asOf: string,
  book: string,
  period: string,
  ytdView: string,
): Promise<{
  register: CashRegisterGet | null;
  elements: CashElementListGet | null;
  ytd: CashYtdGet | null;
}> {
  const [reg, els, ytd] = await Promise.all([
    client.executeQuery("CashRegisterGet", {
      asOfDate: asOf,
      account: book,
      period,
    }),
    client.executeQuery("CashElementListGet", {
      account: "all",
      asOfDate: asOf,
    }),
    client.executeQuery("CashYtdGet", {
      asOfDate: asOf,
      view: ytdView,
    }),
  ]);
  return {
    register:
      reg.ok && reg.bodyJson
        ? (JSON.parse(reg.bodyJson) as CashRegisterGet)
        : null,
    elements:
      els.ok && els.bodyJson
        ? (JSON.parse(els.bodyJson) as CashElementListGet)
        : null,
    ytd:
      ytd.ok && ytd.bodyJson
        ? (JSON.parse(ytd.bodyJson) as CashYtdGet)
        : null,
  };
}

export async function fetchElementList(
  client: LocalTauriFinanceClient,
  asOf: string,
): Promise<CashElementListGet | null> {
  const els = await client.executeQuery("CashElementListGet", {
    account: "all",
    asOfDate: asOf,
  });
  return els.ok && els.bodyJson
    ? (JSON.parse(els.bodyJson) as CashElementListGet)
    : null;
}

export async function fetchTaxPlanning(
  client: LocalTauriFinanceClient,
  asOf: string,
): Promise<TaxPlanningGet | null> {
  const result = await client.executeQuery("TaxPlanningGet", { asOfDate: asOf });
  return result.ok && result.bodyJson
    ? (JSON.parse(result.bodyJson) as TaxPlanningGet)
    : null;
}
