import { formatUsd } from "@finos/ui-components";
import type { CashYtdGet } from "@finos/app-contracts";

function YtdTable({
  title,
  ariaLabel,
  ytd,
}: {
  title: string;
  ariaLabel: string;
  ytd: CashYtdGet | null;
}) {
  const scale = ytd?.scale ?? 2;
  const money = (minor: number | null | undefined) =>
    minor == null ? "—" : formatUsd(minor, scale);
  return (
    <div className="table-wrap">
      <h4>{title}</h4>
      <table aria-label={ariaLabel}>
        <thead>
          <tr>
            <th>{title}</th>
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
  );
}

export function CashYtdPanel({
  account,
  tax,
}: {
  account: CashYtdGet | null;
  tax: CashYtdGet | null;
  busy?: boolean;
}) {
  return (
    <section className="cash-ytd" id="cash-ytd" aria-label="Cash YTD">
      <h3>YTD</h3>
      <div className="cash-ytd-tables">
        <YtdTable title="Account" ariaLabel="Cash YTD by account" ytd={account} />
        <YtdTable title="Tax type" ariaLabel="Cash YTD by tax type" ytd={tax} />
      </div>
    </section>
  );
}
