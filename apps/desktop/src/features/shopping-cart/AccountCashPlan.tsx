import type { CartBuyLine, CashLedgerGet, CashPileGet } from "@finos/app-contracts";
import { formatUsd } from "@finos/ui-components";

export function AccountCashPlan({
  pile,
  ledger,
  buyLines,
  agreed,
  filled,
  depositAmount,
  fills,
  busy,
  writesBlocked,
  onDepositAmount,
  onFill,
  onDeposit,
  onConfirmFill,
}: {
  pile: CashPileGet | null;
  ledger: CashLedgerGet | null;
  buyLines: CartBuyLine[];
  agreed: boolean;
  filled: boolean;
  depositAmount: string;
  fills: Record<string, string>;
  busy?: boolean;
  writesBlocked?: boolean;
  onDepositAmount: (value: string) => void;
  onFill: (lineId: string, value: string) => void;
  onDeposit: () => void;
  onConfirmFill: () => void;
}) {
  const dollars = pile?.found ? pile.dollarsMinor : null;
  return (
    <section aria-label="Account cash pile">
      <p aria-label="Cart cash pile">
        {pile?.found
          ? `${pile.symbol} ${formatUsd(pile.dollarsMinor, 2)} — qty at $1.00. Draft does not spend this.`
          : "This account has no cash position."}
      </p>
      {ledger && ledger.entries.length > 0 ? (
        <table aria-label="Cash deposit ledger">
          <thead>
            <tr>
              <th scope="col">Date</th>
              <th scope="col">Type</th>
              <th scope="col">Amount</th>
            </tr>
          </thead>
          <tbody>
            {ledger.entries.map((row) => (
              <tr key={row.activityId}>
                <td>{row.occurredOn}</td>
                <td>{row.activityType}</td>
                <td>{formatUsd(row.amountMinor, row.scale)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      ) : null}
      {dollars != null && !agreed ? (
        <div className="form-grid">
          <label>
            Deposit to cover
            <input
              aria-label="Cart cash deposit"
              value={depositAmount}
              onChange={(e) => onDepositAmount(e.target.value)}
              disabled={busy || writesBlocked}
            />
          </label>
          <div className="buttons">
            <button
              type="button"
              aria-label="Post cash deposit"
              disabled={busy || writesBlocked || !depositAmount.trim()}
              onClick={onDeposit}
            >
              Post deposit
            </button>
          </div>
        </div>
      ) : null}
      {agreed && buyLines.length > 0 ? (
        <div aria-label="Confirm fill prices">
          <p>Confirm the price paid. Cost is deducted from {pile?.symbol ?? "cash"} only after this.</p>
          {buyLines.map((line) => (
            <label key={line.lineId}>
              {line.symbol} fill $/sh
              <input
                aria-label={`Cart fill price ${line.symbol}`}
                value={fills[line.lineId] ?? (line.lastMinor / 100).toFixed(2)}
                onChange={(e) => onFill(line.lineId, e.target.value)}
                disabled={busy || writesBlocked || filled}
              />
            </label>
          ))}
          <div className="buttons">
            <button
              type="button"
              aria-label="Confirm fill"
              disabled={busy || writesBlocked || filled}
              onClick={onConfirmFill}
            >
              Confirm fill
            </button>
          </div>
        </div>
      ) : null}
    </section>
  );
}
