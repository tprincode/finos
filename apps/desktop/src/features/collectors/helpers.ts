import type { CollectorSetItem } from "./types";

export function formatCollectorClock(raw?: string | null): string {
  const opts: Intl.DateTimeFormatOptions = {
    weekday: "long",
    year: "numeric",
    month: "long",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
    second: "2-digit",
  };
  const stamp = (raw ?? "").trim();
  if (stamp.length >= 19) {
    const parsed = new Date(stamp.includes("T") ? stamp : stamp.replace(" ", "T"));
    if (!Number.isNaN(parsed.getTime())) {
      return parsed.toLocaleString("en-US", opts);
    }
  }
  if (stamp.length >= 10) {
    return `${stamp.slice(0, 10)} (run stored date only) · screen ${new Date().toLocaleString("en-US", opts)}`;
  }
  return new Date().toLocaleString("en-US", opts);
}

export function fleetRocText(row: {
  rocEstimateMinor?: number | null;
  rocScale?: number;
  rocTaxYear?: string;
}): string {
  if (row.rocEstimateMinor == null) return "unknown";
  const scale = row.rocScale ?? 2;
  const pct = (row.rocEstimateMinor / 10 ** scale).toFixed(scale);
  return row.rocTaxYear?.trim()
    ? `${pct}% (${row.rocTaxYear})`
    : `${pct}%`;
}

export function isDeclarationWeekdayToday(asOfDate: string, weekday?: string): boolean {
  const raw = (weekday ?? "").trim();
  if (!raw || asOfDate.trim().length < 10) {
    return false;
  }
  const d = new Date(`${asOfDate.trim().slice(0, 10)}T12:00:00`);
  if (Number.isNaN(d.getTime())) {
    return false;
  }
  const names = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
  ];
  const full = names[d.getDay()] ?? "";
  const short = full.slice(0, 3);
  return (
    raw.localeCompare(full, undefined, { sensitivity: "accent" }) === 0 ||
    raw.localeCompare(short, undefined, { sensitivity: "accent" }) === 0
  );
}

export function collectorRetrieveNeedsRun(row: CollectorSetItem, asOfDate: string): boolean {
  if (!row.openLots || !row.collectorEnabled || !row.declarationSource.trim()) {
    return false;
  }
  if (isDeclarationWeekdayToday(asOfDate, row.declarationWeekday)) {
    return true;
  }
  if (row.lastRunOk !== true) {
    return true;
  }
  const ran = (row.lastRunAt ?? "").trim();
  if (!ran) {
    return true;
  }
  return !ran.startsWith(asOfDate);
}

export function collectorMissIsTimeout(message: string, code?: string) {
  const t = `${code ?? ""} ${message}`.toLowerCase();
  return (
    t.includes("declaration_retrieve_timeout") ||
    t.includes("timed out") ||
    t.includes("timeout")
  );
}

export function collectorCommandBody(
  row: CollectorSetItem,
  forceRefresh: boolean,
  establish = false,
  extra?: { timeoutAttempt?: number; progressiveRetry?: boolean },
) {
  return {
    securityId: row.securityId,
    symbol: row.symbol,
    declarationSource: row.declarationSource,
    sourceUrl: row.sourceUrl ?? "",
    divType: row.divType ?? "",
    lastContentHash: row.lastContentHash ?? "",
    lastRunOk: row.lastRunOk === true,
    lastRunAt: row.lastRunAt ?? "",
    paymentFrequency: row.paymentFrequency ?? "",
    inceptionOn: row.inceptionOn ?? "",
    forceRefresh,
    establish,
    timeoutAttempt: extra?.timeoutAttempt ?? 0,
    progressiveRetry: extra?.progressiveRetry === true,
  };
}

/** Fleet shows income names only — matches storage collector_symbol_pays. */
export function collectorItemPays(row: CollectorSetItem): boolean {
  const sym = row.symbol.trim().toUpperCase();
  const div = (row.divType || "").trim().toUpperCase().replace(/\s+/g, "-");
  if (div === "CASH" || sym === "SPAXX" || sym === "FDRXX" || sym === "SWVXX") {
    return true;
  }
  if (div === "DIV-1" || div === "DIV1") {
    return true;
  }
  const freq = (row.paymentFrequency || "").trim().toLowerCase();
  return (
    freq === "weekly" ||
    freq === "52" ||
    freq === "monthly" ||
    freq === "12" ||
    freq === "quarterly" ||
    freq === "4"
  );
}

export function collectorIsCash(row: CollectorSetItem): boolean {
  const sym = row.symbol.trim().toUpperCase();
  const div = (row.divType || "").trim().toUpperCase().replace(/\s+/g, "-");
  return div === "CASH" || sym === "SPAXX" || sym === "FDRXX" || sym === "SWVXX";
}

export function collectorIsDiv1OrCash(row: CollectorSetItem): boolean {
  const div = (row.divType || "").trim().toUpperCase().replace(/\s+/g, "-");
  return collectorIsCash(row) || div === "DIV-1" || div === "DIV1";
}

/** DIV-1/CASH only. Failing or never-run with empty seed URL, or DIV-1 ROC hole. */
export function collectorNeedsOwnerUrl(row: CollectorSetItem): boolean {
  if (!collectorIsDiv1OrCash(row)) {
    return false;
  }
  const emptySeed = !(row.sourceUrl ?? "").trim();
  const failingOrNever = row.lastRunOk !== true;
  if (emptySeed && failingOrNever) {
    return true;
  }
  if (collectorIsCash(row)) {
    return false;
  }
  return Boolean(row.fillGapsRocBlank) && !(row.rocSourceUrl ?? "").trim();
}
