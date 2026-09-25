const CASH = new Set(["SPAXX", "CASH", "FDRXX", "SWVXX"]);

export function isCashSymbol(symbol: string): boolean {
  return CASH.has(symbol.trim().toUpperCase());
}

/** Account name picks the money-market symbol. A held symbol is only a fallback. */
export function accountCashSymbol(accountName: string, heldSymbols: string[] = []): string {
  const name = accountName.trim();
  if (name === "Health") return "FDRXX";
  if (name === "9") return "SWVXX";
  if (name === "Income" || name === "FI Roth" || name === "Speculation" || name === "Car") {
    return "SPAXX";
  }
  const held = heldSymbols.find((symbol) => isCashSymbol(symbol));
  return held ?? "SPAXX";
}

export function roundedWeekMonth(yearMinor: number | null): {
  weekMinor: number | null;
  monthMinor: number | null;
} {
  if (yearMinor == null) return { weekMinor: null, monthMinor: null };
  return {
    weekMinor: Math.round(yearMinor / 52),
    monthMinor: Math.round(yearMinor / 12),
  };
}

export function pctOf(part: number | null, total: number | null, digits: number): string {
  if (part == null || total == null || total <= 0) return "";
  return `${((part / total) * 100).toFixed(digits)}%`;
}

export function usd(minor: number): string {
  if (!Number.isFinite(minor)) return "";
  return (minor / 100).toLocaleString("en-US", {
    style: "currency",
    currency: "USD",
  });
}

export function sumOrBlank(values: Array<number | null>): number | null {
  if (values.some((value) => value == null)) return null;
  return values.reduce<number>((sum, value) => sum + (value ?? 0), 0);
}

/** Blank tier omits the clause. One shared type says maintain. Otherwise name the change. */
export function positionTypeClause(
  sellTiers: string[],
  buyTiers: string[],
): string | null {
  if (sellTiers.length === 0 || buyTiers.length === 0) return null;
  if (sellTiers.some((tier) => tier.trim() === "") || buyTiers.some((tier) => tier.trim() === "")) {
    return null;
  }
  const sell = [...new Set(sellTiers)];
  const buy = [...new Set(buyTiers)];
  if (sell.length === 1 && buy.length === 1 && sell[0] === buy[0]) {
    return `maintain the ${sell[0]} position type`;
  }
  return `change the position type from ${sell.join(" and ")} to ${buy.join(" and ")}`;
}

export function scenarioCritique(input: {
  slot: string;
  sellMonth: number | null;
  buyMonth: number | null;
  sellTiers: string[];
  buyTiers: string[];
  sellDollars: number | null;
  buyDollars: number | null;
  depositMinor: number;
  offerAnother: boolean;
}): string {
  const typeClause = positionTypeClause(input.sellTiers, input.buyTiers);
  let text: string;
  if (input.sellMonth != null && input.buyMonth != null) {
    const delta = input.buyMonth - input.sellMonth;
    const word = delta < 0 ? "decreased" : "increased";
    text = `Scenario ${input.slot} will generate ${usd(Math.abs(delta))} ${word} monthly income`;
    text += typeClause ? ` and ${typeClause}.` : ".";
  } else if (typeClause) {
    text = `Scenario ${input.slot} will ${typeClause}.`;
  } else {
    text = `Scenario ${input.slot} is waiting on price or income for one or more rows.`;
  }
  if (input.sellDollars != null && input.buyDollars != null) {
    const enough = input.buyDollars <= input.sellDollars + Math.max(input.depositMinor, 0);
    text += ` Currently the plan ${enough ? "does" : "does not"} have sufficient cash to execute.`;
  }
  if (input.offerAnother) {
    text += " Do you want to add a new scenario?";
  }
  return text;
}

export function compareCritique(input: {
  sellDollars: number | null;
  aMonth: number | null;
  bMonth: number | null;
  aYear: number | null;
  bYear: number | null;
  aUnspent: number | null;
  bUnspent: number | null;
  aTypes: string;
  bTypes: string;
  aEnough: boolean | null;
  bEnough: boolean | null;
}): string {
  const month =
    input.aMonth != null && input.bMonth != null
      ? `Scenario A monthly income is ${usd(input.aMonth)}. Scenario B monthly income is ${usd(input.bMonth)}.`
      : "Monthly income is blank until both scenarios have a year amount on every row.";
  const ret = (year: number | null) =>
    input.sellDollars != null && input.sellDollars > 0 && year != null
      ? pctOf(year, input.sellDollars, 2)
      : "";
  const aRet = ret(input.aYear);
  const bRet = ret(input.bYear);
  const returns =
    aRet && bRet
      ? ` New return is ${aRet} for Scenario A and ${bRet} for Scenario B.`
      : "";
  const types =
    input.aTypes || input.bTypes
      ? ` Position type is ${input.aTypes || "blank"} for Scenario A and ${input.bTypes || "blank"} for Scenario B.`
      : "";
  const unspent =
    input.aUnspent != null && input.bUnspent != null
      ? ` Unspent dollars are ${usd(input.aUnspent)} for Scenario A and ${usd(input.bUnspent)} for Scenario B.`
      : "";
  const cash = (label: string, enough: boolean | null) =>
    enough == null ? "" : `${label} ${enough ? "does" : "does not"} have sufficient cash`;
  const aCash = cash("Scenario A", input.aEnough);
  const bCash = cash("Scenario B", input.bEnough);
  const which = aCash && bCash ? ` ${aCash}. ${bCash}.` : "";
  return `${month}${returns}${types}${unspent}${which}`;
}
