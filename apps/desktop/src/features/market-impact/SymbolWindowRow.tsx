import { useCallback, useEffect, useState } from "react";
import type { MarketImpactGet } from "@finos/app-contracts";

export type WindowRow = MarketImpactGet["rows"][number];

export type WindowClient = {
  executeQuery: (name: string, body?: unknown) => Promise<{ ok: boolean; bodyJson?: string }>;
  executeCommand: (
    name: string,
    body?: unknown,
  ) => Promise<{ ok: boolean; bodyJson?: string; errorCode?: string }>;
};

function returnFace(bps: number | null): string {
  if (bps == null) return "";
  return `${(bps / 100).toFixed(2)}%`;
}

async function storeWindow(
  client: WindowClient,
  row: WindowRow,
  kind: "Bull" | "Bear",
  start: string,
  end: string,
): Promise<string | null> {
  const startOn = start.trim();
  const endOn = end.trim();
  if (!startOn && !endOn) return null;
  if (!startOn || !endOn) return `${kind} needs the other date.`;
  const recorded = await client.executeCommand("BacktestPeriodRecord", {
    kind,
    name: `${kind} ${row.symbol}`,
    startOn,
    endOn,
  });
  if (!recorded.ok || !recorded.bodyJson) {
    return `${kind} not stored: ${recorded.errorCode ?? "error"}`;
  }
  const period = JSON.parse(recorded.bodyJson) as { periodId?: string };
  if (!period.periodId) return `${kind} not stored.`;
  const series = await client.executeCommand("PeriodSeriesRetrieve", {
    symbol: row.symbol,
    startOn,
    endOn,
  });
  if (!series.ok || !series.bodyJson) return null;
  const retrieved = JSON.parse(series.bodyJson) as {
    candidates?: unknown[];
    benchmarkCandidates?: unknown[];
    posted?: boolean;
  };
  if (retrieved.posted) return null;
  const underlying = (row.underlying ?? "").trim();
  const [underlyingCandidates, spyCandidates, nasdaqCandidates] = await Promise.all([
    comparisonCandidates(client, underlying, startOn, endOn),
    comparisonCandidates(client, "SPY", startOn, endOn),
    comparisonCandidates(client, "QQQ", startOn, endOn),
  ]);
  await client.executeCommand("PositionBacktestCalculate", {
    securityId: row.securityId,
    periodId: period.periodId,
    candidates: retrieved.candidates ?? [],
    benchmarkCandidates: retrieved.benchmarkCandidates ?? [],
    underlyingCandidates,
    spyCandidates,
    nasdaqCandidates,
  });
  return null;
}

async function comparisonCandidates(
  client: WindowClient,
  symbol: string,
  startOn: string,
  endOn: string,
): Promise<unknown[]> {
  if (!symbol) return [];
  const series = await client.executeCommand("PeriodSeriesRetrieve", {
    symbol,
    startOn,
    endOn,
  });
  if (!series.ok || !series.bodyJson) return [];
  const retrieved = JSON.parse(series.bodyJson) as { candidates?: unknown[]; posted?: boolean };
  if (retrieved.posted) return [];
  return retrieved.candidates ?? [];
}

function SymbolWindowRow({
  row,
  client,
  onOpenSymbol,
  onRowDirty,
  onSaved,
  showCashCushion = true,
}: {
  row: WindowRow;
  client: WindowClient;
  onOpenSymbol?: (symbol: string) => void;
  onRowDirty: (symbol: string, dirty: boolean) => void;
  onSaved: () => void;
  showCashCushion?: boolean;
}) {
  const [bullStart, setBullStart] = useState(row.bullStart);
  const [bullEnd, setBullEnd] = useState(row.bullEnd);
  const [bearStart, setBearStart] = useState(row.bearStart);
  const [bearEnd, setBearEnd] = useState(row.bearEnd);
  const [note, setNote] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const dirty =
    bullStart !== row.bullStart ||
    bullEnd !== row.bullEnd ||
    bearStart !== row.bearStart ||
    bearEnd !== row.bearEnd;

  useEffect(() => {
    setBullStart(row.bullStart);
    setBullEnd(row.bullEnd);
    setBearStart(row.bearStart);
    setBearEnd(row.bearEnd);
  }, [row.bullStart, row.bullEnd, row.bearStart, row.bearEnd]);

  useEffect(() => {
    onRowDirty(row.symbol, dirty);
    return () => onRowDirty(row.symbol, false);
  }, [dirty, onRowDirty, row.symbol]);

  const save = async () => {
    setBusy(true);
    setNote(null);
    try {
      const notes = [
        await storeWindow(client, row, "Bull", bullStart, bullEnd),
        await storeWindow(client, row, "Bear", bearStart, bearEnd),
      ].filter((line): line is string => line != null);
      setNote(notes.length > 0 ? notes.join(" ") : null);
      onSaved();
    } catch (err: unknown) {
      setNote(String(err));
    } finally {
      setBusy(false);
    }
  };

  return (
    <tr>
      <td>
        {onOpenSymbol ? (
          <button
            type="button"
            aria-label={`Open ${row.symbol} position`}
            onClick={() => onOpenSymbol(row.symbol)}
          >
            {row.symbol}
          </button>
        ) : (
          row.symbol
        )}
      </td>
      <td>
        <input
          type="date"
          className="market-impact-date"
          aria-label={`Bull start ${row.symbol}`}
          value={bullStart}
          onChange={(e) => setBullStart(e.target.value)}
          disabled={busy}
        />
      </td>
      <td>
        <input
          type="date"
          className="market-impact-date"
          aria-label={`Bull stop ${row.symbol}`}
          value={bullEnd}
          onChange={(e) => setBullEnd(e.target.value)}
          disabled={busy}
        />
      </td>
      <td className="market-impact-return">{returnFace(row.bullPriceReturnBps)}</td>
      {showCashCushion ? (
        <td className="market-impact-return">{returnFace(row.bullCushionBps)}</td>
      ) : null}
      <td className="market-impact-return">{returnFace(row.bullTotalReturnBps)}</td>
      <td>
        <input
          type="date"
          className="market-impact-date"
          aria-label={`Bear start ${row.symbol}`}
          value={bearStart}
          onChange={(e) => setBearStart(e.target.value)}
          disabled={busy}
        />
      </td>
      <td>
        <input
          type="date"
          className="market-impact-date"
          aria-label={`Bear stop ${row.symbol}`}
          value={bearEnd}
          onChange={(e) => setBearEnd(e.target.value)}
          disabled={busy}
        />
      </td>
      <td className="market-impact-return">{returnFace(row.bearPriceReturnBps)}</td>
      {showCashCushion ? (
        <td className="market-impact-return">{returnFace(row.bearCushionBps)}</td>
      ) : null}
      <td className="market-impact-return">{returnFace(row.bearTotalReturnBps)}</td>
      <td>
        <button
          type="button"
          aria-label={`Save ${row.symbol} windows`}
          className={dirty ? "is-unsaved" : undefined}
          onClick={() => void save()}
          disabled={busy}
        >
          Save/Update
        </button>
        {note ? <span role="status">{note}</span> : null}
      </td>
    </tr>
  );
}

export function SymbolWindowTable({
  client,
  symbol,
  discardEpoch = 0,
  onDirtyChange,
  onOpenSymbol,
  onSaved,
  showCashCushion = true,
}: {
  client: WindowClient;
  symbol?: string;
  discardEpoch?: number;
  onDirtyChange?: (dirty: boolean) => void;
  onOpenSymbol?: (symbol: string) => void;
  onSaved?: () => void;
  showCashCushion?: boolean;
}) {
  const [rows, setRows] = useState<MarketImpactGet["rows"] | null>(null);
  const [reload, setReload] = useState(0);
  const [rowDirty, setRowDirty] = useState<Record<string, boolean>>({});
  const markRowDirty = useCallback((name: string, dirty: boolean) => {
    setRowDirty((current) => {
      if (current[name] === dirty) return current;
      return { ...current, [name]: dirty };
    });
  }, []);
  const anyDirty = Object.values(rowDirty).some(Boolean);

  useEffect(() => {
    onDirtyChange?.(anyDirty);
  }, [anyDirty, onDirtyChange]);

  useEffect(() => {
    let cancelled = false;
    void client.executeQuery("MarketImpactGet").then((result) => {
      if (cancelled) return;
      if (!result.ok || !result.bodyJson) {
        setRows([]);
        return;
      }
      try {
        const parsed = (JSON.parse(result.bodyJson) as MarketImpactGet).rows ?? [];
        const wanted = symbol?.trim().toUpperCase();
        setRows(wanted ? parsed.filter((row) => row.symbol.toUpperCase() === wanted) : parsed);
      } catch {
        setRows([]);
      }
    });
    return () => {
      cancelled = true;
    };
  }, [client, reload, symbol]);

  return (
    <div className="table-wrap" data-part="market-impact-windows">
      <table className="market-impact-windows" aria-label="Market impact windows">
        <thead>
          <tr>
            <th>Symbol</th>
            <th>Bull start</th>
            <th>Bull stop</th>
            <th
              className="market-impact-return"
              title="Change in the share price from the first day to the last day of this window. Distributions are not included."
            >
              Bull return
            </th>
            {showCashCushion ? (
            <th
              className="market-impact-return"
              title="Cash the broker paid on one share during this window, divided by that share's price on the first day. 5% means 5 cents of cash for each dollar of the starting price. It is not the whole holding's cash, not yield on cost, and not a Plan figure. A large cushion does not cancel a price drop; read it next to price return."
            >
              Bull cash cushion
            </th>
            ) : null}
            <th
              className="market-impact-return"
              title="Price return plus cash cushion for the same days, both per share. This is not a reinvested total-return index. If either input is unknown, the total is unknown."
            >
              Bull total
            </th>
            <th>Bear start</th>
            <th>Bear stop</th>
            <th
              className="market-impact-return"
              title="Change in the share price from the first day to the last day of this window. Distributions are not included."
            >
              Bear return
            </th>
            {showCashCushion ? (
            <th
              className="market-impact-return"
              title="Cash the broker paid on one share during this window, divided by that share's price on the first day. 5% means 5 cents of cash for each dollar of the starting price. It is not the whole holding's cash, not yield on cost, and not a Plan figure. A large cushion does not cancel a price drop; read it next to price return."
            >
              Bear cash cushion
            </th>
            ) : null}
            <th
              className="market-impact-return"
              title="Price return plus cash cushion for the same days, both per share. This is not a reinvested total-return index. If either input is unknown, the total is unknown."
            >
              Bear total
            </th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          {(rows ?? []).map((row) => (
            <SymbolWindowRow
              key={`${row.symbol}-${discardEpoch}`}
              row={row}
              client={client}
              onOpenSymbol={onOpenSymbol}
              showCashCushion={showCashCushion}
              onRowDirty={markRowDirty}
              onSaved={() => {
                setReload((n) => n + 1);
                onSaved?.();
              }}
            />
          ))}
        </tbody>
      </table>
      {rows == null ? <p role="status">Loading market impact…</p> : null}
      {rows != null && rows.length === 0 ? <p role="status">No calculator symbols.</p> : null}
    </div>
  );
}
