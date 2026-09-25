import { useEffect, useRef, useState } from "react";
import type { CashElementRecord } from "@finos/app-contracts";

export type EditorOccurrence = {
  occurrenceId?: string;
  occurredOn: string;
  amountMinor: number;
  isException?: boolean;
  isCancelled?: boolean;
};

export function CashElementEditor({
  account,
  defaultKind = "Withdrawal",
  focus,
  busy,
  onSave,
  onDeleteSeries,
  onOpenExceptions,
  onClose,
  onDirtyChange,
}: {
  account: string;
  defaultKind?: string;
  focus?: { element?: CashElementRecord | null; occurrenceId?: string | null };
  busy?: boolean;
  onSave: (body: {
    elementId?: string;
    name: string;
    account: string;
    kind: string;
    cadence: string;
    weekdayOrMonthDay: string;
    startOn: string;
    stopOn: string;
    amountMinor: number;
    occurrences: EditorOccurrence[];
  }) => boolean | Promise<boolean>;
  onDeleteSeries?: (elementId: string) => void;
  onOpenExceptions?: () => void;
  onClose: () => void;
  onDirtyChange?: (dirty: boolean) => void;
}) {
  const element = focus?.element ?? null;
  const [name, setName] = useState(element?.note ?? "");
  const [kind, setKind] = useState(element?.kind ?? defaultKind);
  const [cadence, setCadence] = useState(element?.cadence ?? "monthly");
  const [day, setDay] = useState(element?.weekdayOrMonthDay || "1");
  const [startOn, setStartOn] = useState(element?.startOn ?? "");
  const [stopOn, setStopOn] = useState(element?.stopOn ?? "");
  const [amount, setAmount] = useState(
    element ? (element.amountMinor / 100).toFixed(2) : "",
  );
  const [dirty, setDirty] = useState(false);
  const box = useRef<HTMLElement>(null);
  const exceptionCount = element?.exceptionCount ?? 0;

  useEffect(() => {
    setName(element?.note ?? "");
    setKind(element?.kind ?? defaultKind);
    setCadence(element?.cadence ?? "monthly");
    setDay(element?.weekdayOrMonthDay || "1");
    setStartOn(element?.startOn ?? "");
    setStopOn(element?.stopOn ?? "");
    setAmount(element ? (element.amountMinor / 100).toFixed(2) : "");
    setDirty(false);
    onDirtyChange?.(false);
    box.current?.scrollIntoView({ block: "nearest" });
  }, [element?.elementId, defaultKind]);

  const mark = () => {
    setDirty(true);
    onDirtyChange?.(true);
  };

  const ownerActive = dirty || !!busy;

  return (
    <section
      ref={box}
      id="cash-element-editor"
      className={
        ownerActive ? "cash-element-editor is-owner-edit-active" : "cash-element-editor"
      }
      aria-label="Element editor"
      aria-busy={busy || undefined}
    >
      {busy ? (
        <p aria-live="polite" aria-label="Element editor working" role="status">
          Working…
        </p>
      ) : null}
      <p className="element-editor-account">Account {account}</p>
      <h3>{element ? "Edit element" : "Add Element"}</h3>
      <label>
        Type
        <select
          aria-label="Element type"
          value={kind}
          disabled={busy}
          onChange={(e) => {
            setKind(e.target.value);
            mark();
          }}
        >
          <option value="Deposit">Deposit</option>
          <option value="Withdrawal">Withdrawal</option>
        </select>
      </label>
      <label>
        Name
        <input
          aria-label="Element name"
          value={name}
          disabled={busy}
          onChange={(e) => {
            setName(e.target.value);
            mark();
          }}
        />
      </label>
      <label>
        Amount
        <input
          inputMode="decimal"
          aria-label="Element amount"
          value={amount}
          disabled={busy}
          onChange={(e) => {
            setAmount(e.target.value);
            mark();
          }}
        />
      </label>
      <label>
        Frequency
        <select
          aria-label="Element frequency"
          value={cadence}
          disabled={busy}
          onChange={(e) => {
            setCadence(e.target.value);
            mark();
          }}
        >
          <option value="weekly">Weekly</option>
          <option value="monthly">Monthly</option>
          <option value="annual">Annual</option>
          <option value="one-time">One-time</option>
        </select>
      </label>
      <label>
        Schedule Date
        <input
          aria-label="Element schedule date"
          value={day}
          disabled={busy}
          onChange={(e) => {
            setDay(e.target.value);
            mark();
          }}
        />
      </label>
      <label>
        Start date
        <input
          type="date"
          aria-label="Element start date"
          value={startOn}
          disabled={busy}
          onChange={(e) => {
            setStartOn(e.target.value);
            mark();
          }}
        />
      </label>
      <label>
        Expiration
        <input
          type="date"
          aria-label="Element stop date"
          value={stopOn}
          disabled={busy}
          onChange={(e) => {
            setStopOn(e.target.value);
            mark();
          }}
        />
      </label>
      {stopOn ? null : (
        <p className="element-never-expires">Never expires</p>
      )}
      <button
        type="button"
        className="element-exceptions-link"
        aria-label="Open exceptions"
        disabled={busy || !element || !onOpenExceptions}
        onClick={() => onOpenExceptions?.()}
      >
        {exceptionCount} Exceptions
      </button>
      <div className="buttons">
        <button
          type="button"
          aria-label="Save element"
          disabled={busy || !dirty}
          className={dirty ? "is-unsaved" : undefined}
          onClick={() => {
            const dollars = Number(amount);
            void (async () => {
              const ok = await onSave({
                elementId: element?.elementId,
                name,
                account,
                kind,
                cadence,
                weekdayOrMonthDay: day,
                startOn,
                stopOn,
                amountMinor: Number.isFinite(dollars) ? Math.round(dollars * 100) : 0,
                occurrences: [],
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
        {element && onDeleteSeries ? (
          <button
            type="button"
            aria-label="Delete element series"
            disabled={busy}
            onClick={() => onDeleteSeries(element.elementId)}
          >
            Delete series
          </button>
        ) : null}
        <button
          type="button"
          aria-label="Close element editor"
          disabled={busy}
          onClick={onClose}
        >
          Close
        </button>
      </div>
    </section>
  );
}
