import { useEffect, useRef, useState } from "react";
import { formatUsd } from "@finos/ui-components";
import type { CashElementRecord } from "@finos/app-contracts";
import type { EditorOccurrence } from "./CashElementEditor";
import { CashElementExceptionEdit } from "./CashElementExceptionEdit";

function formatUpcomingDate(iso: string): string {
  const day = new Date(`${iso}T00:00:00`);
  if (Number.isNaN(day.getTime())) return iso;
  return day.toLocaleDateString("en-US", {
    weekday: "short",
    month: "short",
    day: "numeric",
    year: "numeric",
  });
}

function cadenceLabel(cadence: string): string {
  switch (cadence) {
    case "weekly":
      return "Weekly";
    case "monthly":
      return "Monthly";
    case "annual":
      return "Annual";
    case "one-time":
      return "One-time";
    default:
      return cadence;
  }
}

function exceptionLabel(count: number): string {
  return count === 1 ? "1 Exception" : `${count} Exceptions`;
}

export function CashElementExceptions({
  account,
  element,
  occurrences,
  busy,
  onSave,
  onClose,
  onDirtyChange,
}: {
  account: string;
  element: CashElementRecord;
  occurrences: EditorOccurrence[];
  busy?: boolean;
  onSave: (body: {
    elementId: string;
    occurrenceId: string;
    cancel: boolean;
    occurredOn: string;
    amountMinor: number;
  }) => boolean | Promise<boolean>;
  onClose: () => void;
  onDirtyChange?: (dirty: boolean) => void;
}) {
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [editOpen, setEditOpen] = useState(false);
  const box = useRef<HTMLElement>(null);
  const rows = occurrences;
  const selected =
    rows.find((row) => (row.occurrenceId || row.occurredOn) === selectedId) ??
    null;
  const exceptionCount =
    element.exceptionCount ??
    rows.filter((row) => row.isException || row.isCancelled).length;

  useEffect(() => {
    setSelectedId(null);
    setEditOpen(false);
    onDirtyChange?.(false);
    box.current?.scrollIntoView({ block: "nearest" });
  }, [element.elementId]);

  const openRow = (row: EditorOccurrence) => {
    setSelectedId(row.occurrenceId || row.occurredOn);
    setEditOpen(true);
  };

  if (editOpen && selected) {
    return (
      <CashElementExceptionEdit
        account={account}
        element={element}
        occurrence={selected}
        busy={busy}
        onSave={async (body) => {
          const ok = await onSave(body);
          if (ok) setEditOpen(false);
          return ok;
        }}
        onClose={() => setEditOpen(false)}
        onDirtyChange={onDirtyChange}
      />
    );
  }

  return (
    <section
      ref={box}
      id="cash-element-exceptions"
      className="cash-element-editor cash-element-exceptions"
      aria-label="Element exceptions"
      aria-busy={busy || undefined}
    >
      {busy ? (
        <p aria-live="polite" aria-label="Element exceptions working" role="status">
          Working…
        </p>
      ) : null}
      <p className="element-editor-account">Account {account}</p>
      <h3>Upcoming Transactions</h3>
      <p className="element-exceptions-series">
        {formatUsd(element.amountMinor, 2)} {cadenceLabel(element.cadence)} —{" "}
        {exceptionLabel(exceptionCount)}
      </p>
      <table aria-label="Upcoming transactions">
        <thead>
          <tr>
            <th>Date</th>
            <th className="numeric">Amount</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((row, i) => {
            const key = row.occurrenceId || `${row.occurredOn}-${i}`;
            const selectedRow = selectedId === (row.occurrenceId || row.occurredOn);
            return (
              <tr
                key={key}
                className={[
                  "element-upcoming-row",
                  selectedRow ? "is-selected" : "",
                  row.isCancelled ? "is-cancelled" : "",
                ]
                  .filter(Boolean)
                  .join(" ")}
                onClick={() => openRow(row)}
              >
                <td>{formatUpcomingDate(row.occurredOn)}</td>
                <td className="numeric">{formatUsd(row.amountMinor, 2)}</td>
              </tr>
            );
          })}
        </tbody>
      </table>
      <div className="buttons">
        <button
          type="button"
          aria-label="Close exceptions"
          disabled={busy}
          onClick={onClose}
        >
          Close
        </button>
      </div>
    </section>
  );
}
