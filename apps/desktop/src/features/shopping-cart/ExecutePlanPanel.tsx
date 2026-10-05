import { formatUsd } from "@finos/ui-components";
import type { CashConfirmPreview } from "./cartCashConfirm";
import { cartPriceInput, parseCartPrice } from "./cartPrice";

export type ExecuteSellRow = {
  lineId: string;
  symbol: string;
  qtyLabel: string;
  priceMinor: number;
  unitScale: number;
  proceedsMinor: number;
  taxGainMinor: number | null;
  performanceGainMinor: number | null;
};

export type ExecuteBuyRow = {
  lineId: string;
  symbol: string;
  qtyWhole: number;
  lastMinor: number;
  priceScale: number;
  spendMinor: number;
  confirmed: boolean;
};

function money(minor: number | null | undefined): string {
  if (minor == null) return "";
  return formatUsd(minor, 2);
}

function signedGain(minor: number | null): string {
  if (minor == null) return "";
  if (minor === 0) return formatUsd(0, 2);
  const sign = minor > 0 ? "+" : "-";
  return `${sign}${formatUsd(Math.abs(minor), 2)}`;
}

function unitPriceDisplay(minor: number, scale: number): string {
  return cartPriceInput(minor, scale);
}

export function ExecutePlanPanel({
  scenarioLabel,
  scenarioStatus,
  cashSymbol,
  spaxxQtyLabel,
  sellRows,
  buyRows,
  startingCashMinor,
  sellBudgetMinor,
  buySpendMinor,
  budgetBalanceMinor,
  canExecute,
  blockReason,
  sellsPosted,
  busy,
  writesBlocked,
  statusMessage,
  onExecuteSells,
  onOpenBuyLot,
  onSellPriceCommit,
  onBuyPriceCommit,
  cashAligned,
  canAlignCash,
  alignBlockReason,
  executeWarning,
  cashPreview,
  accountName,
  purchaseCostMinor,
  netDividendMinor,
}: {
  scenarioLabel: string;
  scenarioStatus: string;
  cashSymbol: string;
  spaxxQtyLabel: string | null;
  sellRows: ExecuteSellRow[];
  buyRows: ExecuteBuyRow[];
  startingCashMinor: number | null;
  sellBudgetMinor: number | null;
  buySpendMinor: number | null;
  budgetBalanceMinor: number | null;
  canExecute: boolean;
  blockReason: string | null;
  sellsPosted: boolean;
  busy?: boolean;
  writesBlocked?: boolean;
  statusMessage?: string | null;
  onExecuteSells: () => void;
  onOpenBuyLot: (lineId: string) => void;
  onSellPriceCommit: (lineId: string, unitMinor: number) => void;
  onBuyPriceCommit: (lineId: string, lastMinor: number) => void;
  buysConfirmed: boolean;
  cashAligned: boolean;
  canAlignCash: boolean;
  alignBlockReason: string | null;
  executeWarning: string | null;
  cashPreview: CashConfirmPreview | null;
  accountName: string;
  purchaseCostMinor: number | null;
  netDividendMinor: number | null;
}) {
  /** A completed cart stays on screen for the close summary, but nothing is editable. */
  const readOnly = scenarioStatus === "complete" || scenarioStatus === "discarded";
  const frozen =
    readOnly || scenarioStatus === "agreed" || scenarioStatus === "executing";
  const disabled = busy || writesBlocked;
  const executeBlocked = disabled || !canExecute;
  const cashReady = canAlignCash && !cashAligned;
  const cashMath = cashPreview ? (
    <table aria-label="Confirm cash calculation">
      <tbody>
        <tr>
          <th scope="row">Starting {cashSymbol}</th>
          <td>{money(cashPreview.baselineMinor)}</td>
        </tr>
        <tr>
          <th scope="row">Sale proceeds</th>
          <td>{money(cashPreview.sellProceedsMinor)}</td>
        </tr>
        <tr>
          <th scope="row">Purchases</th>
          <td>{money(cashPreview.buySpendMinor)}</td>
        </tr>
        <tr>
          <th scope="row">{cashSymbol} after this confirm</th>
          <td>{money(cashPreview.targetMinor)}</td>
        </tr>
      </tbody>
    </table>
  ) : null;
  const closeSummary = (
    <table aria-label="Completed register summary">
      <thead>
        <tr>
          <th scope="col">Account</th>
          <th scope="col">Purchases</th>
          <th scope="col">Net dividend change</th>
        </tr>
      </thead>
      <tbody>
        <tr>
          <td>{accountName}</td>
          <td>{money(purchaseCostMinor)}</td>
          <td>{signedGain(netDividendMinor)}</td>
        </tr>
      </tbody>
    </table>
  );
  const cashBlock = (
    <>
      <h3>{cashAligned ? "Cash confirmed" : "Confirm cash"}</h3>
      {cashAligned ? (
        <div className="cart-confirm-cash" aria-label="Confirm cash preview">
          <p role="status">
            {cashSymbol} matches actual sale proceeds and purchase dollars
            {cashPreview ? ` — ${money(cashPreview.targetMinor)}.` : "."}
          </p>
          {cashMath}
          {closeSummary}
        </div>
      ) : cashReady ? (
        <div className="cart-confirm-cash" aria-label="Confirm cash preview">
          <p role="status">
            Aligning leftover {cashSymbol}
            {cashPreview ? ` to ${money(cashPreview.targetMinor)}` : ""}.
          </p>
          {cashMath}
          {alignBlockReason ? (
            <p className="blocked" role="status">
              {alignBlockReason}
            </p>
          ) : null}
        </div>
      ) : (
        <p role="status">
          {sellsPosted
            ? "Confirm each purchase. Leftover cash aligns after the last one."
            : "Confirm sells first. Leftover cash aligns after the last purchase."}
        </p>
      )}
    </>
  );

  return (
    <section aria-label="Execute plan" className="cart-execute-plan">
      {cashReady || cashAligned ? cashBlock : null}
      {cashReady || cashAligned ? null : <h3>Execute plan</h3>}
      {cashReady || cashAligned ? null : (
      <p className="cart-execute-summary">
        <span>
          {cashSymbol} {startingCashMinor == null ? "—" : money(startingCashMinor)}
        </span>
        <span>Budget {sellBudgetMinor == null ? "—" : money(sellBudgetMinor)}</span>
        <span>Buys {buySpendMinor == null ? "—" : money(buySpendMinor)}</span>
        <span>
          Left{" "}
          {budgetBalanceMinor == null
            ? "—"
            : budgetBalanceMinor >= 0
              ? money(budgetBalanceMinor)
              : `short ${money(Math.abs(budgetBalanceMinor))}`}
        </span>
        <span>{scenarioStatus}</span>
      </p>
      )}

      {statusMessage ? (
        <p className="cart-execute-status" role="status">
          {statusMessage}
        </p>
      ) : null}

      {writesBlocked ? (
        <p className="blocked" role="status">
          Writes are blocked until the open handoff is cleared.
        </p>
      ) : null}

      {!sellsPosted && blockReason ? (
        <p className="blocked" role="status">
          {blockReason}
        </p>
      ) : null}

      {!sellsPosted && executeWarning ? (
        <p className="cart-execute-warning" role="note">
          {executeWarning}
        </p>
      ) : null}

      <h4>Sells</h4>
      {spaxxQtyLabel != null && startingCashMinor != null ? (
        <p className="cart-execute-note" role="note">
          {cashSymbol} {spaxxQtyLabel} sh, {money(startingCashMinor)} — not sold here
        </p>
      ) : null}
      {sellRows.length === 0 ? (
        <p role="status">No position sells in this plan (SPAXX-only funding is OK).</p>
      ) : (
        <table>
          <thead>
            <tr>
              <th scope="col">Symbol</th>
              <th scope="col">Qty</th>
              <th scope="col">Price / share</th>
              <th scope="col">Total proceeds</th>
              <th scope="col">Tax P/L</th>
              <th scope="col">Performance P/L</th>
            </tr>
          </thead>
          <tbody>
            {sellRows.map((row) => (
              <tr key={row.lineId}>
                <td>{row.symbol}</td>
                <td>{row.qtyLabel}</td>
                <td>
                  {sellsPosted ? (
                    unitPriceDisplay(row.priceMinor, row.unitScale)
                  ) : (
                    <input
                      aria-label={`${row.symbol} sell price per share`}
                      key={`${row.lineId}-${row.priceMinor}-${row.unitScale}`}
                      defaultValue={unitPriceDisplay(row.priceMinor, row.unitScale)}
                      step="0.0001"
                      inputMode="decimal"
                      disabled={disabled}
                      onBlur={(event) => {
                        const minor = parseCartPrice(event.target.value);
                        if (minor != null && minor !== row.priceMinor) {
                          onSellPriceCommit(row.lineId, minor);
                        }
                      }}
                    />
                  )}
                </td>
                <td>{money(row.proceedsMinor)}</td>
                <td>{signedGain(row.taxGainMinor)}</td>
                <td>{signedGain(row.performanceGainMinor)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}

      {!sellsPosted ? (
        <div className="buttons">
          <button
            type="button"
            aria-label={`Execute plan sells for ${scenarioLabel}`}
            disabled={executeBlocked}
            onClick={onExecuteSells}
          >
            Execute plan — confirm sells
          </button>
          {executeBlocked && !blockReason && !writesBlocked && canExecute ? (
            <p role="status">Working…</p>
          ) : null}
        </div>
      ) : (
        <p className="cart-execute-note" role="status">
          Sells posted.
        </p>
      )}

      <h4>Buys</h4>
      {buyRows.length === 0 ? (
        <p role="status">Add buy rows on the scenario table first.</p>
      ) : (
        <table>
          <thead>
            <tr>
              <th scope="col">Symbol</th>
              <th scope="col">Qty</th>
              <th scope="col">Plan $ / share</th>
              <th scope="col">Plan total</th>
              <th scope="col">Action</th>
            </tr>
          </thead>
          <tbody>
            {buyRows.map((row) => (
              <tr key={row.lineId}>
                <td>{row.symbol}</td>
                <td>{row.qtyWhole}</td>
                <td>
                  {row.confirmed ? (
                    unitPriceDisplay(row.lastMinor, row.priceScale)
                  ) : (
                    <input
                      aria-label={`${row.symbol} plan price per share`}
                      key={`${row.lineId}-${row.lastMinor}-${row.priceScale}`}
                      defaultValue={unitPriceDisplay(row.lastMinor, row.priceScale)}
                      step="0.0001"
                      inputMode="decimal"
                      disabled={disabled}
                      onBlur={(event) => {
                        const minor = parseCartPrice(event.target.value);
                        if (minor != null && minor !== row.lastMinor) {
                          onBuyPriceCommit(row.lineId, minor);
                        }
                      }}
                    />
                  )}
                </td>
                <td>{money(row.spendMinor)}</td>
                <td>
                  {row.confirmed ? (
                    <span>Recorded in Add Lot</span>
                  ) : (
                    <button
                      type="button"
                      aria-label={`Confirm purchase ${row.symbol}`}
                      disabled={disabled || readOnly || !sellsPosted || !frozen}
                      onClick={() => onOpenBuyLot(row.lineId)}
                    >
                      Confirm purchase
                    </button>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
      {!cashReady && !cashAligned ? cashBlock : null}
    </section>
  );
}
