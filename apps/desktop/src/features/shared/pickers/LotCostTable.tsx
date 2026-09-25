import type { LotOption } from "./LotSelect";

export type LotSortMode = "lowest-cost" | "largest-tax-loss";

export type LotCostOption = LotOption & {
  openedOn?: string;
  remainingPerformanceMinor: number;
  remainingTaxMinor: number;
  /** Lot money scale. Basis is not cents unless this is 2. */
  scale?: number;
};

function toCents(minor: number, scale: number | undefined): number {
  const places = scale ?? 2;
  if (places === 2) return minor;
  if (places > 2) return Math.round(minor / 10 ** (places - 2));
  return Math.round(minor * 10 ** (2 - places));
}

const CASH = new Set(["SPAXX", "CASH", "FDRXX", "SWVXX"]);

function isCashSymbol(symbol: string): boolean {
  return CASH.has(symbol.toUpperCase());
}

function formatQty(minor: number, scale: number): string {
  return (minor / 10 ** scale).toFixed(scale);
}

function money(minor: number | null | undefined): string {
  if (minor == null) return "";
  return (minor / 100).toLocaleString("en-US", {
    style: "currency",
    currency: "USD",
  });
}

function lotDollars(qtyMinor: number, qtyScale: number, unitMinor: number): number {
  const denom = 10 ** qtyScale;
  if (denom === 0) return 0;
  return Math.round((qtyMinor * unitMinor) / denom);
}

function unitCost(basisMinor: number, qtyMinor: number, qtyScale: number): number | null {
  if (qtyMinor <= 0) return null;
  const qty = qtyMinor / 10 ** qtyScale;
  if (qty <= 0) return null;
  return Math.round(basisMinor / qty);
}

export function rankLowestCost(lots: LotCostOption[]): string[] {
  return [...lots]
    .filter((l) => l.remainingQuantityMinor > 0)
    .sort((a, b) => {
      const aKey = BigInt(a.remainingPerformanceMinor) * BigInt(b.remainingQuantityMinor);
      const bKey = BigInt(b.remainingPerformanceMinor) * BigInt(a.remainingQuantityMinor);
      if (aKey < bKey) return -1;
      if (aKey > bKey) return 1;
      return a.lotId.localeCompare(b.lotId);
    })
    .map((l) => l.lotId);
}

export function rankLargestTaxLoss(
  lots: LotCostOption[],
  lastBySymbol: Record<string, number | null | undefined>,
): string[] {
  const rows = lots.map((lot) => {
    const last = lastBySymbol[lot.symbol];
    const taxGain =
      isCashSymbol(lot.symbol) || last == null
        ? null
        : lotDollars(lot.remainingQuantityMinor, lot.quantityScale, last) -
          toCents(lot.remainingTaxMinor, lot.scale);
    return { lotId: lot.lotId, taxGain };
  });
  return rows
    .sort((a, b) => {
      if (a.taxGain != null && b.taxGain != null) {
        if (a.taxGain !== b.taxGain) return a.taxGain - b.taxGain;
        return a.lotId.localeCompare(b.lotId);
      }
      if (a.taxGain != null) return -1;
      if (b.taxGain != null) return 1;
      return a.lotId.localeCompare(b.lotId);
    })
    .map((r) => r.lotId);
}

export function LotCostTable({
  lots,
  value,
  onChange,
  lastBySymbol,
  sortMode,
  onSortModeChange,
  accountName,
  disabled,
  ariaLabel,
}: {
  lots: LotCostOption[];
  value: string;
  onChange: (lotId: string) => void;
  lastBySymbol: Record<string, number | null | undefined>;
  sortMode: LotSortMode;
  onSortModeChange: (mode: LotSortMode) => void;
  accountName?: string;
  disabled?: boolean;
  ariaLabel: string;
}) {
  const visible = accountName
    ? lots.filter((lot) => lot.accountName === accountName)
    : lots;
  const order =
    sortMode === "largest-tax-loss"
      ? rankLargestTaxLoss(visible, lastBySymbol)
      : rankLowestCost(visible);
  const ranked = order
    .map((id) => visible.find((l) => l.lotId === id))
    .filter((l): l is LotCostOption => l != null);
  const topId = ranked[0]?.lotId;
  const badge = sortMode === "largest-tax-loss" ? "largest tax loss" : "lowest cost";

  return (
    <div className="lot-cost-table">
      <div className="form-grid">
        <label>
          Lot rank
          <select
            aria-label="Lot rank mode"
            value={sortMode}
            onChange={(e) => onSortModeChange(e.target.value as LotSortMode)}
            disabled={disabled}
          >
            <option value="lowest-cost">Lowest performance cost</option>
            <option value="largest-tax-loss">Largest tax loss</option>
          </select>
        </label>
      </div>
      <table aria-label={ariaLabel}>
        <thead>
          <tr>
            <th scope="col">Rank</th>
            <th scope="col">Symbol</th>
            <th scope="col">Opened</th>
            <th scope="col">Remaining</th>
            <th scope="col">Unit cost (perf)</th>
            <th scope="col">Unit cost (tax)</th>
            <th scope="col">Last</th>
            <th scope="col">Perf P/L if sold</th>
            <th scope="col">Tax P/L if sold</th>
          </tr>
        </thead>
        <tbody>
          {ranked.length === 0 ? (
            <tr>
              <td colSpan={9}>No open lots.</td>
            </tr>
          ) : (
            ranked.map((lot) => {
              const cash = isCashSymbol(lot.symbol);
              const last = cash ? 100 : (lastBySymbol[lot.symbol] ?? null);
              const proceeds =
                last == null
                  ? null
                  : lotDollars(lot.remainingQuantityMinor, lot.quantityScale, last);
              const perfPl =
                cash || proceeds == null
                  ? null
                  : proceeds - toCents(lot.remainingPerformanceMinor, lot.scale);
              const taxPl =
                cash || proceeds == null ? null : proceeds - toCents(lot.remainingTaxMinor, lot.scale);
              const perfUnit = cash
                ? null
                : unitCost(
                    toCents(lot.remainingPerformanceMinor, lot.scale),
                    lot.remainingQuantityMinor,
                    lot.quantityScale,
                  );
              const taxUnit = cash
                ? null
                : unitCost(
                    toCents(lot.remainingTaxMinor, lot.scale),
                    lot.remainingQuantityMinor,
                    lot.quantityScale,
                  );
              return (
                <tr
                  key={lot.lotId}
                  aria-selected={lot.lotId === value}
                  className={lot.lotId === value ? "is-selected" : undefined}
                >
                  <td>{lot.lotId === topId ? badge : ""}</td>
                  <td>
                    <button
                      type="button"
                      aria-label={`Choose lot ${lot.symbol} ${lot.lotId}`}
                      disabled={disabled}
                      onClick={() => onChange(lot.lotId)}
                    >
                      {lot.symbol}
                    </button>
                  </td>
                  <td>{lot.openedOn ?? ""}</td>
                  <td>{formatQty(lot.remainingQuantityMinor, lot.quantityScale)}</td>
                  <td>{money(perfUnit)}</td>
                  <td>{money(taxUnit)}</td>
                  <td>{cash ? "" : money(last)}</td>
                  <td>{money(perfPl)}</td>
                  <td>{money(taxPl)}</td>
                </tr>
              );
            })
          )}
        </tbody>
      </table>
    </div>
  );
}
