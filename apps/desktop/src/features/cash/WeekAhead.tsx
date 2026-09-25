import { formatUsd } from "@finos/ui-components";
import type { WeekAheadGet } from "@finos/app-contracts";

export function WeekAheadPanel({
  week,
  busy,
  pendingId,
  onConfirm,
  onOpenEditor,
  onDefer,
}: {
  week: WeekAheadGet | null;
  busy?: boolean;
  pendingId?: string | null;
  onConfirm: (occurrenceId: string) => void;
  onOpenEditor: (row: {
    occurrenceId: string;
    elementId: string;
    account: string;
    note: string;
  }) => void;
  onDefer: (occurrenceId: string) => void;
}) {
  if (!week) return null;
  const scale = week.scale ?? 2;

  return (
    <section className="week-ahead" aria-label="Week ahead">
      <h3>Week ahead</h3>
      <div className="table-wrap">
        <table aria-label="Week ahead">
          <thead>
            <tr>
              <th>Date</th>
              <th>Account</th>
              <th>Transaction</th>
              <th className="numeric">Amount</th>
              <th>Confirm</th>
              <th>Edit</th>
              <th>Check tomorrow</th>
            </tr>
          </thead>
          <tbody>
            {week.rows.map((row) => {
              const pending = pendingId === row.occurrenceId;
              return (
                <tr key={row.occurrenceId}>
                  <td>{row.occurredOn}</td>
                  <td>{row.account}</td>
                  <td>{row.transaction}</td>
                  <td className="numeric">
                    {formatUsd(row.amountMinor, row.scale ?? scale)}
                  </td>
                  <td>
                    <button
                      type="button"
                      aria-label={`Confirm ${row.account} ${row.note || row.transaction}`}
                      disabled={busy || pending}
                      className={pending ? "is-unsaved" : undefined}
                      onClick={() => onConfirm(row.occurrenceId)}
                    >
                      Confirm
                    </button>
                  </td>
                  <td>
                    <button
                      type="button"
                      aria-label={`Edit ${row.account} ${row.note || row.transaction}`}
                      disabled={busy}
                      onClick={() =>
                        onOpenEditor({
                          occurrenceId: row.occurrenceId,
                          elementId: row.elementId,
                          account: row.account,
                          note: row.note,
                        })
                      }
                    >
                      Edit
                    </button>
                  </td>
                  <td>
                    <button
                      type="button"
                      aria-label={`Check tomorrow ${row.account} ${row.note || row.transaction}`}
                      disabled={busy}
                      onClick={() => onDefer(row.occurrenceId)}
                    >
                      Check tomorrow
                    </button>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </section>
  );
}
