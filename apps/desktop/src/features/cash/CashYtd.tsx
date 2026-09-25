import { formatUsd } from "@finos/ui-components";
import type { CashYtdGet } from "@finos/app-contracts";

export function CashYtdPanel({
  ytd,
  view,
  busy,
  onView,
}: {
  ytd: CashYtdGet | null;
  view: "account" | "tax";
  busy?: boolean;
  onView: (view: "account" | "tax") => void;
}) {
  const scale = ytd?.scale ?? 2;
  const money = (minor: number | null | undefined) =>
    minor == null ? "—" : formatUsd(minor, scale);

  return (
    <section className="cash-ytd" id="cash-ytd" aria-label="Cash YTD">
      <h3>YTD</h3>
      <div className="buttons" aria-label="YTD view">
        <button
          type="button"
          aria-label="YTD account"
          aria-pressed={view === "account"}
          disabled={busy}
          onClick={() => onView("account")}
        >
          Account
        </button>
        <button
          type="button"
          aria-label="YTD tax type"
          aria-pressed={view === "tax"}
          disabled={busy}
          onClick={() => onView("tax")}
        >
          Tax type
        </button>
      </div>
      <div className="table-wrap">
        <table aria-label="Cash YTD">
          <thead>
            <tr>
              <th>{view === "tax" ? "Tax type" : "Account"}</th>
              <th className="numeric">YTD actual</th>
              <th className="numeric">Remaining plan</th>
              <th className="numeric">EOY projected</th>
            </tr>
          </thead>
          <tbody>
            {(ytd?.rows ?? []).map((row) => (
              <tr key={row.label}>
                <td>{row.label}</td>
                <td className="numeric">{money(row.actualMinor)}</td>
                <td className="numeric">{money(row.remainingMinor)}</td>
                <td className="numeric">{money(row.eoyMinor)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </section>
  );
}
