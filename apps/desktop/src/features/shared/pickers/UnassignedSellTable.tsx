import type { HoldingsUnassignedSell } from "@finos/app-contracts";

function formatQty(minor: number, scale: number): string {
  return (minor / 10 ** scale).toFixed(scale);
}

function formatMoney(minor: number, scale: number): string {
  return (minor / 10 ** scale).toLocaleString("en-US", {
    style: "currency",
    currency: "USD",
  });
}

export function UnassignedSellTable({
  sells,
  value,
  onChange,
  disabled,
}: {
  sells: HoldingsUnassignedSell[];
  value: string;
  onChange: (activityId: string) => void;
  disabled?: boolean;
}) {
  return (
    <div className="lot-cost-table">
      <table aria-label="Unassigned sells">
        <thead>
          <tr>
            <th scope="col">Date</th>
            <th scope="col">Account</th>
            <th scope="col">Qty</th>
            <th scope="col">Amount</th>
            <th scope="col">Source</th>
          </tr>
        </thead>
        <tbody>
          {sells.length === 0 ? (
            <tr>
              <td colSpan={5}>No unassigned sell for this symbol</td>
            </tr>
          ) : (
            sells.map((sell) => {
              const qty =
                sell.quantityMinor != null && sell.quantityScale != null
                  ? formatQty(sell.quantityMinor, sell.quantityScale)
                  : "—";
              const source = sell.source === "cart" ? "Cart leftover" : "";
              return (
                <tr
                  key={sell.activityId}
                  aria-selected={sell.activityId === value}
                  className={sell.activityId === value ? "is-selected" : undefined}
                >
                  <td>
                    <button
                      type="button"
                      aria-label={`Choose sell ${sell.occurredOn} ${sell.accountName} ${formatMoney(sell.amountMinor, sell.scale)}`}
                      disabled={disabled}
                      onClick={() => onChange(sell.activityId)}
                    >
                      {sell.occurredOn}
                    </button>
                  </td>
                  <td>{sell.accountName}</td>
                  <td>{qty}</td>
                  <td>{formatMoney(sell.amountMinor, sell.scale)}</td>
                  <td>{source}</td>
                </tr>
              );
            })
          )}
        </tbody>
      </table>
    </div>
  );
}
