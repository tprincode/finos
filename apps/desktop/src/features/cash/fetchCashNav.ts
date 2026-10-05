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

async function parseYtd(
  client: LocalTauriFinanceClient,
  asOf: string,
  view: "account" | "tax",
): Promise<CashYtdGet | null> {
  const ytd = await client.executeQuery("CashYtdGet", {
    asOfDate: asOf,
    view,
  });
  return ytd.ok && ytd.bodyJson
    ? (JSON.parse(ytd.bodyJson) as CashYtdGet)
    : null;
}

export async function fetchCashYtdBoth(
  client: LocalTauriFinanceClient,
  asOf: string,
): Promise<{ ytdAccount: CashYtdGet | null; ytdTax: CashYtdGet | null }> {
  const [ytdAccount, ytdTax] = await Promise.all([
    parseYtd(client, asOf, "account"),
    parseYtd(client, asOf, "tax"),
  ]);
  return { ytdAccount, ytdTax };
}

export async function fetchCashNav(
  client: LocalTauriFinanceClient,
  asOf: string,
  book: string,
  period: string,
): Promise<{
  register: CashRegisterGet | null;
  elements: CashElementListGet | null;
  ytdAccount: CashYtdGet | null;
  ytdTax: CashYtdGet | null;
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
    fetchCashYtdBoth(client, asOf),
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
    ytdAccount: ytd.ytdAccount,
    ytdTax: ytd.ytdTax,
  };
}

/** Write a cash-nav pack into the screen. YTD is both views; a missing view stays blank, not $0. */
export function applyCashNav(
  pack: {
    register: CashRegisterGet | null;
    elements: CashElementListGet | null;
    ytdAccount: CashYtdGet | null;
    ytdTax: CashYtdGet | null;
  },
  setRegister: (value: CashRegisterGet) => void,
  setElements: (value: CashElementListGet) => void,
  setYtdAccount: (value: CashYtdGet | null) => void,
  setYtdTax: (value: CashYtdGet | null) => void,
) {
  if (pack.register) setRegister(pack.register);
  if (pack.elements) setElements(pack.elements);
  setYtdAccount(pack.ytdAccount);
  setYtdTax(pack.ytdTax);
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
