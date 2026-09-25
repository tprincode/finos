import { useEffect, useState } from "react";
import { AccountTickPicker, formatUsd } from "@finos/ui-components";
import type { CashElementListGet, CashElementRecord } from "@finos/app-contracts";
import { CashElementEditor, type EditorOccurrence } from "./CashElementEditor";
import { CashElementExceptions } from "./CashElementExceptions";
import { CashElementHistory } from "./CashElementHistory";
import { PageActivityCard } from "../shared/PageActivityCard";

const MANAGED_ELEMENT_ACCOUNTS = [
  "Income",
  "FI Roth",
  "Health",
  "Car",
  "Account 9",
  "SSA_2026",
] as const;

function selectedForBook(book: string): string[] {
  if (book === "all") return [...MANAGED_ELEMENT_ACCOUNTS];
  if (!book) return [];
  return [book];
}

function bookFromSelected(next: string[]): string {
  if (next.length === MANAGED_ELEMENT_ACCOUNTS.length) return "all";
  if (next.length === 1) return next[0];
  return "";
}

function isDeposit(kind: string): boolean {
  return kind.toLowerCase() === "deposit";
}

export function CashElementsCatalog({
  catalog,
  book,
  editorOpen,
  editorAccount,
  editorElement,
  editorOccurrences,
  editorOccurrenceId,
  busy,
  onBook,
  onAddElement,
  onOpenElement,
  onCloseEditor,
  onSaveElement,
  onSaveExceptions,
  onDeleteSeries,
  onDirtyChange,
  asOfDate,
}: {
  catalog: CashElementListGet | null;
  book: string;
  editorOpen: boolean;
  editorAccount: string;
  editorElement?: CashElementRecord | null;
  editorOccurrences: EditorOccurrence[];
  editorOccurrenceId?: string | null;
  busy?: boolean;
  onBook: (book: string) => void;
  onAddElement: () => void;
  onOpenElement: (element: CashElementRecord) => void;
  onCloseEditor: () => void;
  onSaveElement: (body: {
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
  onSaveExceptions: (body: {
    elementId: string;
    occurrenceId: string;
    cancel: boolean;
    occurredOn: string;
    amountMinor: number;
  }) => boolean | Promise<boolean>;
  onDeleteSeries: (elementId: string) => void;
  onDirtyChange?: (dirty: boolean) => void;
  asOfDate: string;
}) {
  const [addKind, setAddKind] = useState("Withdrawal");
  const [exceptionsOpen, setExceptionsOpen] = useState(false);

  useEffect(() => {
    setExceptionsOpen(false);
  }, [editorElement?.elementId, editorOpen]);
  const items = (catalog?.items ?? []).filter(
    (el) => book === "all" || el.account === book,
  );
  const deposits = items.filter((el) => isDeposit(el.kind));
  const withdrawals = items.filter((el) => !isDeposit(el.kind));
  const money = (minor: number | null | undefined) =>
    minor == null ? "—" : formatUsd(minor, 2);
  const canAdd = Boolean(book) && book !== "all" && !busy;
  const showAccount = book === "all";

  const startAdd = (kind: "Deposit" | "Withdrawal") => {
    if (!canAdd) return;
    setAddKind(kind);
    onAddElement();
  };

  const renderGroup = (
    title: "Deposits" | "Withdrawals",
    addLabel: "Add Deposit" | "Add Withdrawal",
    kind: "Deposit" | "Withdrawal",
    rows: CashElementRecord[],
  ) => (
    <div className="elements-group">
      <div className="elements-group-head">
        {title === "Deposits" ? <h4>Deposits</h4> : <h4>Withdrawals</h4>}
        <button
          type="button"
          aria-label={addLabel}
          className={busy ? "is-unsaved" : undefined}
          disabled={!canAdd}
          onClick={() => startAdd(kind)}
        >
          +
        </button>
      </div>
      <div className="table-wrap">
        <table aria-label={title}>
          <thead>
            <tr>
              <th>Name</th>
              {showAccount ? <th>Account</th> : null}
              <th>Frequency</th>
              <th>Next date</th>
              <th className="numeric">Next amount</th>
              <th>Edit</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((el) => (
              <tr
                key={el.elementId}
                className="elements-catalog-row"
                onClick={() => onOpenElement(el)}
              >
                <td>{el.note || "—"}</td>
                {showAccount ? <td>{el.account}</td> : null}
                <td>{el.cadence}</td>
                <td>{el.nextOccurredOn || "—"}</td>
                <td className="numeric">{money(el.nextAmountMinor)}</td>
                <td>
                  <button
                    type="button"
                    aria-label={`Edit element ${el.note || el.kind}`}
                    className={busy ? "is-unsaved" : undefined}
                    disabled={busy}
                    onClick={(event) => {
                      event.stopPropagation();
                      onOpenElement(el);
                    }}
                  >
                    Edit</button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );

  return (
    <section className="cash-elements-catalog" id="cash-elements" aria-label="Elements catalog">
      <PageActivityCard page="elements" rootId="cash-elements" />
      <h3>Elements</h3>
      {editorOpen && exceptionsOpen && editorElement ? (
        <CashElementExceptions
          account={editorAccount}
          element={editorElement}
          occurrences={editorOccurrences}
          busy={busy}
          onSave={onSaveExceptions}
          onClose={() => setExceptionsOpen(false)}
          onDirtyChange={onDirtyChange}
        />
      ) : editorOpen ? (
        <CashElementEditor
          account={editorAccount}
          defaultKind={addKind}
          focus={{ element: editorElement, occurrenceId: editorOccurrenceId }}
          busy={busy}
          onSave={onSaveElement}
          onDeleteSeries={onDeleteSeries}
          onOpenExceptions={() => setExceptionsOpen(true)}
          onClose={() => {
            setExceptionsOpen(false);
            onCloseEditor();
          }}
          onDirtyChange={onDirtyChange}
        />
      ) : (
        <>
          <AccountTickPicker
            legend="Managed accounts"
            allLabel="All managed accounts"
            mode="allOrOne"
            accounts={MANAGED_ELEMENT_ACCOUNTS}
            selected={selectedForBook(book)}
            onSelected={(next) => onBook(bookFromSelected(next))}
          />
          {renderGroup("Deposits", "Add Deposit", "Deposit", deposits)}
          {renderGroup("Withdrawals", "Add Withdrawal", "Withdrawal", withdrawals)}
        </>
      )}
      <CashElementHistory
        elements={catalog?.items ?? []}
        asOfDate={asOfDate}
      />
    </section>
  );
}
