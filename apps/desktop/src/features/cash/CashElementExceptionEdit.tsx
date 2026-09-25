import { useEffect, useRef, useState } from "react";
import type { CashElementRecord } from "@finos/app-contracts";
import type { EditorOccurrence } from "./CashElementEditor";

function formatLongDate(iso: string): string {
  const day = new Date(`${iso}T00:00:00`);
  if (Number.isNaN(day.getTime())) return iso;
  return day.toLocaleDateString("en-US", {
    weekday: "long",
    month: "long",
    day: "numeric",
    year: "numeric",
  });
}

export function CashElementExceptionEdit({
  account,
  element,
  occurrence,
  busy,
  onSave,
  onClose,
  onDirtyChange,
}: {
  account: string;
  element: CashElementRecord;
  occurrence: EditorOccurrence;
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
  const [mode, setMode] = useState<"modify" | "cancel">("modify");
  const [occurredOn, setOccurredOn] = useState(occurrence.occurredOn);
  const [amount, setAmount] = useState((occurrence.amountMinor / 100).toFixed(2));
  const [dirty, setDirty] = useState(false);
  const box = useRef<HTMLElement>(null);

  useEffect(() => {
    setMode(occurrence.isCancelled ? "cancel" : "modify");
    setOccurredOn(occurrence.occurredOn);
    setAmount((occurrence.amountMinor / 100).toFixed(2));
    setDirty(false);
    onDirtyChange?.(false);
    box.current?.scrollIntoView({ block: "nearest" });
  }, [occurrence.occurrenceId, occurrence.occurredOn]);

  const mark = () => {
    setDirty(true);
    onDirtyChange?.(true);
  };

  const ownerActive = dirty || !!busy;

  return (
    <section
      ref={box}
      id="cash-element-exception-edit"
      className={
        ownerActive
          ? "cash-element-editor cash-element-exception-edit is-owner-edit-active"
          : "cash-element-editor cash-element-exception-edit"
      }
      aria-label="Edit transaction"
      aria-busy={busy || undefined}
    >
      {busy ? (
        <p aria-live="polite" aria-label="Edit transaction working" role="status">
          Working…
        </p>
      ) : null}
      <p className="element-editor-account">Account {account}</p>
      <h3>Edit transaction</h3>
      <p className="element-exceptions-series">{element.note || "Element"}</p>
      <p className="element-exception-original">
        Originally {formatLongDate(occurrence.occurredOn)}
      </p>
      <fieldset className="element-exception-mode">
        <legend>Change</legend>
        <label>
          <input
            type="radio"
            name="exception-mode"
            aria-label="Cancel this transaction"
            checked={mode === "cancel"}
            disabled={busy}
            onChange={() => {
              setMode("cancel");
              mark();
            }}
          />
          Cancel this transaction
        </label>
        <label>
          <input
            type="radio"
            name="exception-mode"
            aria-label="Modify this transaction"
            checked={mode === "modify"}
            disabled={busy}
            onChange={() => {
              setMode("modify");
              mark();
            }}
          />
          Modify this transaction
        </label>
      </fieldset>
      {mode === "modify" ? (
        <>
          <label>
            Date
            <input
              type="date"
              aria-label="Exception date"
              value={occurredOn}
              disabled={busy}
              onChange={(e) => {
                setOccurredOn(e.target.value);
                mark();
              }}
            />
          </label>
          <label>
            Amount
            <input
              inputMode="decimal"
              aria-label="Exception amount"
              value={amount}
              disabled={busy}
              onChange={(e) => {
                setAmount(e.target.value);
                mark();
              }}
            />
          </label>
        </>
      ) : null}
      <div className="buttons">
        <button
          type="button"
          aria-label="Save exception"
          disabled={busy || !dirty || !occurrence.occurrenceId}
          className={dirty ? "is-unsaved" : undefined}
          onClick={() => {
            const dollars = Number(amount);
            void (async () => {
              const ok = await onSave({
                elementId: element.elementId,
                occurrenceId: occurrence.occurrenceId!,
                cancel: mode === "cancel",
                occurredOn,
                amountMinor: Number.isFinite(dollars)
                  ? Math.round(dollars * 100)
                  : occurrence.amountMinor,
              });
              if (ok) {
                setDirty(false);
                onDirtyChange?.(false);
              }
            })();
          }}
        >
          Save
        </button>
        <button
          type="button"
          aria-label="Close edit transaction"
          disabled={busy}
          onClick={onClose}
        >
          Close
        </button>
      </div>
    </section>
  );
}
