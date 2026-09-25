import { useEffect, useMemo, useState } from "react";
import { LocalTauriFinanceClient } from "../../financeClient";

const client = new LocalTauriFinanceClient();

type RegisterLine = {
  lineId: string;
  sourceRow?: number | null;
  payType: string;
  occurredOn?: string | null;
  amountMinor: number;
  scale: number;
  category: string;
  vendor: string;
  description: string;
  trueUpOn?: string | null;
  completed?: boolean;
  stepTransfer?: boolean;
  stepBillpay?: boolean;
  stepPay?: boolean;
};

type RegisterBody = {
  lines: RegisterLine[];
  payTypes: string[];
  categories: string[];
  vendors: string[];
  totalCount: number;
};

type EntryDraft = {
  payType: string;
  occurredOn: string;
  amount: string;
  category: string;
  vendor: string;
  description: string;
};

function todayIso(): string {
  const now = new Date();
  const month = String(now.getMonth() + 1).padStart(2, "0");
  const day = String(now.getDate()).padStart(2, "0");
  return `${now.getFullYear()}-${month}-${day}`;
}

function shiftIsoDay(iso: string, days: number): string {
  const source = /^\d{4}-\d{2}-\d{2}$/.test(iso) ? iso : todayIso();
  const [year, month, day] = source.split("-").map(Number);
  const date = new Date(Date.UTC(year, month - 1, day));
  date.setUTCDate(date.getUTCDate() + days);
  const nextMonth = String(date.getUTCMonth() + 1).padStart(2, "0");
  const nextDay = String(date.getUTCDate()).padStart(2, "0");
  return `${date.getUTCFullYear()}-${nextMonth}-${nextDay}`;
}

function dateStepKey(
  event: { key: string; preventDefault: () => void },
  current: string,
  apply: (next: string) => void,
): boolean {
  if (event.key === "+" || event.key === "Add") {
    event.preventDefault();
    apply(shiftIsoDay(current, 1));
    return true;
  }
  if (event.key === "-" || event.key === "Subtract") {
    event.preventDefault();
    apply(shiftIsoDay(current, -1));
    return true;
  }
  return false;
}

function DateStep({
  value,
  ariaLabel,
  onChange,
  onKeyDown,
}: {
  value: string;
  ariaLabel: string;
  onChange: (next: string) => void;
  onKeyDown?: (event: { key: string; preventDefault: () => void }) => void;
}) {
  return (
    <span className="external-date-step">
      <button
        type="button"
        aria-label="Previous day"
        onClick={(event) => {
          event.stopPropagation();
          onChange(shiftIsoDay(value, -1));
        }}
      >
        −
      </button>
      <input
        aria-label={ariaLabel}
        type="date"
        value={value}
        onClick={(event) => event.stopPropagation()}
        onChange={(event) => onChange(event.target.value)}
        onKeyDown={onKeyDown}
      />
      <button
        type="button"
        aria-label="Next day"
        onClick={(event) => {
          event.stopPropagation();
          onChange(shiftIsoDay(value, 1));
        }}
      >
        +
      </button>
    </span>
  );
}

function emptyEntry(): EntryDraft {
  return {
    payType: "",
    occurredOn: todayIso(),
    amount: "",
    category: "",
    vendor: "",
    description: "",
  };
}

function amountText(minor: number, scale = 2): string {
  const sign = minor < 0 ? "-" : "";
  const abs = Math.abs(minor);
  const div = 10 ** scale;
  const whole = Math.floor(abs / div);
  const frac = String(abs % div).padStart(scale, "0");
  return `${sign}${whole}.${frac}`;
}

function amountSearchText(minor: number, scale = 2): string {
  const text = amountText(minor, scale);
  const frac = Math.abs(minor) % 10 ** scale;
  if (frac === 0) {
    return `${text} ${Math.trunc(minor / 10 ** scale)}`;
  }
  return text;
}

function parseAmount(raw: string): number | null {
  const cleaned = raw.trim().replace(/[$,]/g, "");
  if (!cleaned) return null;
  const value = Number(cleaned);
  if (!Number.isFinite(value)) return null;
  return Math.round(value * 100);
}

function columnHit(value: string, filter: string): boolean {
  if (!filter) return true;
  return value === filter;
}

function sortedValues(values: Set<string>): string[] {
  return [...values].sort((a, b) => a.localeCompare(b, undefined, { numeric: true, sensitivity: "base" }));
}

function lineMatches(line: RegisterLine, query: string): boolean {
  const q = query.trim().toLowerCase();
  if (!q) return true;
  const fields = [
    line.payType,
    line.occurredOn ?? "",
    amountSearchText(line.amountMinor, line.scale ?? 2),
    line.category,
    line.vendor,
    line.description,
    line.trueUpOn ?? "",
  ];
  return fields.some((field) => field.toLowerCase().includes(q));
}

function lineChanged(original: RegisterLine, draft: RegisterLine): boolean {
  return (
    original.payType !== draft.payType ||
    (original.occurredOn ?? "") !== (draft.occurredOn ?? "") ||
    original.amountMinor !== draft.amountMinor ||
    original.category !== draft.category ||
    original.vendor !== draft.vendor ||
    original.description !== draft.description ||
    (original.trueUpOn ?? "") !== (draft.trueUpOn ?? "")
  );
}

function entryReady(entry: EntryDraft): boolean {
  return (
    entry.payType.trim() !== "" ||
    entry.amount.trim() !== "" ||
    entry.category.trim() !== "" ||
    entry.vendor.trim() !== "" ||
    entry.description.trim() !== ""
  );
}

async function readBody(result: { ok: boolean; errorCode?: string; bodyJson?: string }) {
  if (!result.ok || !result.bodyJson) {
    throw new Error(result.errorCode || "External register request failed");
  }
  return JSON.parse(result.bodyJson) as RegisterBody;
}

export function ExternalRegister({
  resetToken,
  onDirtyChange,
}: {
  resetToken: number;
  onDirtyChange: (dirty: boolean) => void;
}) {
  const [body, setBody] = useState<RegisterBody | null>(null);
  const [search, setSearch] = useState("");
  const [colFilters, setColFilters] = useState({
    payType: "",
    occurredOn: "",
    amount: "",
    category: "",
    vendor: "",
    description: "",
    trueUpOn: "",
  });
  const [entry, setEntry] = useState<EntryDraft>(emptyEntry);
  const [drafts, setDrafts] = useState<Record<string, RegisterLine>>({});
  const [amountInputs, setAmountInputs] = useState<Record<string, string>>({});
  const [editingId, setEditingId] = useState<string | null>(null);
  const [transferIds, setTransferIds] = useState<string[]>([]);
  const [billpayIds, setBillpayIds] = useState<string[]>([]);
  const [payIds, setPayIds] = useState<string[]>([]);
  const [status, setStatus] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const dirty =
    entryReady(entry) ||
    (body?.lines ?? []).some(
      (line) => drafts[line.lineId] && lineChanged(line, drafts[line.lineId]),
    );

  useEffect(() => {
    onDirtyChange(dirty);
  }, [dirty, onDirtyChange]);

  const load = async () => {
    const result = await client.executeQuery("ExternalRegisterGet");
    const next = await readBody(result);
    setBody(next);
    setDrafts({});
    setAmountInputs({});
    setEditingId(null);
    setEntry(emptyEntry());
  };

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const result = await client.executeQuery("ExternalRegisterGet");
        const next = await readBody(result);
        if (!cancelled) {
          setBody(next);
          setDrafts({});
          setAmountInputs({});
          setEditingId(null);
          setEntry(emptyEntry());
          setStatus(null);
        }
      } catch (err: unknown) {
        if (!cancelled) setStatus(err instanceof Error ? err.message : String(err));
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [resetToken]);

  const shown = useMemo(() => {
    const lines = body?.lines ?? [];
    return lines.filter((line) => lineMatches(line, search));
  }, [body, search]);

  const patchEntry = (patch: Partial<EntryDraft>) => {
    setEntry((current) => ({ ...current, ...patch }));
  };

  const startEdit = (line: RegisterLine) => {
    setEditingId(line.lineId);
    setDrafts((current) =>
      current[line.lineId] ? current : { ...current, [line.lineId]: { ...line } },
    );
    setAmountInputs((current) =>
      current[line.lineId]
        ? current
        : { ...current, [line.lineId]: amountText(line.amountMinor, line.scale) },
    );
  };

  const patchDraft = (lineId: string, patch: Partial<RegisterLine>) => {
    setDrafts((current) => {
      const base = current[lineId];
      if (!base) return current;
      return { ...current, [lineId]: { ...base, ...patch } };
    });
  };

  const save = async () => {
    const lines: RegisterLine[] = (body?.lines ?? [])
      .filter((line) => drafts[line.lineId] && lineChanged(line, drafts[line.lineId]))
      .map((line) => drafts[line.lineId]);
    if (entryReady(entry)) {
      const amountMinor = parseAmount(entry.amount);
      if (amountMinor == null) {
        setStatus("Enter the amount as dollars, for example 9.47 or -2.23.");
        return;
      }
      lines.push({
        lineId: crypto.randomUUID(),
        sourceRow: null,
        payType: entry.payType.trim(),
        occurredOn: entry.occurredOn.trim() || null,
        amountMinor,
        scale: 2,
        category: entry.category.trim(),
        vendor: entry.vendor.trim(),
        description: entry.description.trim(),
        trueUpOn: null,
      });
    }
    if (lines.length === 0) return;
    setBusy(true);
    setStatus(null);
    try {
      const result = await client.executeCommand("ExternalRegisterSave", { lines });
      const next = await readBody(result);
      setBody(next);
      setDrafts({});
      setAmountInputs({});
      setEditingId(null);
      setEntry(emptyEntry());
      setStatus("Saved.");
    } catch (err: unknown) {
      setStatus(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  };

  const addOnEnter = (event: { key: string; preventDefault: () => void }) => {
    if (event.key === "Enter" && entryReady(entry)) {
      event.preventDefault();
      void save();
    }
  };

  const markStep = async (ids: string[], step: string, label: string) => {
    if (dirty) {
      setStatus("Add or cancel the open edits before marking a step.");
      return;
    }
    if (ids.length === 0) return;
    setBusy(true);
    setStatus(null);
    try {
      const result = await client.executeCommand("ExternalRegisterMarkStep", {
        lineIds: ids,
        step,
      });
      const next = await readBody(result);
      setBody(next);
      if (step === "transfer") setTransferIds((current) => current.filter((id) => !ids.includes(id)));
      if (step === "billpay") setBillpayIds((current) => current.filter((id) => !ids.includes(id)));
      if (step === "pay") setPayIds((current) => current.filter((id) => !ids.includes(id)));
      setStatus(`${label} marked.`);
    } catch (err: unknown) {
      setStatus(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  };

  const viewOf = (line: RegisterLine) => drafts[line.lineId] ?? line;
  const stepsDone = (line: RegisterLine) =>
    Boolean(line.stepTransfer && line.stepBillpay && line.stepPay);
  const isComplete = (line: RegisterLine) => Boolean(line.completed || stepsDone(line));
  const pending = shown.filter((line) => !isComplete(line));
  const completed = shown.filter((line) => isComplete(line));
  const completedVisible = completed.filter((line) => {
    const view = viewOf(line);
    return (
      columnHit(view.payType.trim(), colFilters.payType) &&
      columnHit((view.occurredOn ?? "").trim(), colFilters.occurredOn) &&
      columnHit(amountText(view.amountMinor, view.scale), colFilters.amount) &&
      columnHit(view.category.trim(), colFilters.category) &&
      columnHit(view.vendor.trim(), colFilters.vendor) &&
      columnHit(view.description.trim(), colFilters.description) &&
      columnHit((view.trueUpOn ?? "").trim(), colFilters.trueUpOn)
    );
  });
  const setFilter = (key: keyof typeof colFilters, value: string) => {
    setColFilters((current) => ({ ...current, [key]: value }));
  };
  const completedChoices = {
    payType: new Set<string>(),
    occurredOn: new Set<string>(),
    amount: new Set<string>(),
    category: new Set<string>(),
    vendor: new Set<string>(),
    description: new Set<string>(),
    trueUpOn: new Set<string>(),
  };
  for (const line of completed) {
    const view = viewOf(line);
    if (view.payType.trim()) completedChoices.payType.add(view.payType.trim());
    if (view.occurredOn?.trim()) completedChoices.occurredOn.add(view.occurredOn.trim());
    completedChoices.amount.add(amountText(view.amountMinor, view.scale));
    if (view.category.trim()) completedChoices.category.add(view.category.trim());
    if (view.vendor.trim()) completedChoices.vendor.add(view.vendor.trim());
    if (view.description.trim()) completedChoices.description.add(view.description.trim());
    if (view.trueUpOn?.trim()) completedChoices.trueUpOn.add(view.trueUpOn.trim());
  }
  const choiceLists = {
    payType: sortedValues(completedChoices.payType),
    occurredOn: sortedValues(completedChoices.occurredOn),
    amount: sortedValues(completedChoices.amount),
    category: sortedValues(completedChoices.category),
    vendor: sortedValues(completedChoices.vendor),
    description: sortedValues(completedChoices.description),
    trueUpOn: sortedValues(completedChoices.trueUpOn),
  };
  const sumSelected = (ids: string[]) =>
    pending
      .filter((line) => ids.includes(line.lineId))
      .reduce((sum, line) => sum + viewOf(line).amountMinor, 0);
  const toggleId = (
    lineId: string,
    ids: string[],
    setIds: (next: string[]) => void,
  ) => {
    setIds(ids.includes(lineId) ? ids.filter((id) => id !== lineId) : [...ids, lineId]);
  };
  const stepCheck = (
    line: RegisterLine,
    view: RegisterLine,
    done: boolean,
    ids: string[],
    setIds: (next: string[]) => void,
    label: string,
  ) => (
    <td>
      <input
        type="checkbox"
        aria-label={`${label} ${view.payType} ${view.occurredOn ?? ""} ${amountText(view.amountMinor, view.scale)}`}
        checked={done || ids.includes(line.lineId)}
        disabled={done || busy}
        onClick={(event) => event.stopPropagation()}
        onChange={() => {
          if (!done) toggleId(line.lineId, ids, setIds);
        }}
      />
    </td>
  );
  const stepTotal = (label: string, step: string, ids: string[]) => (
    <span className="external-transfer-total">
      {label} {amountText(sumSelected(ids))}
      {ids.length ? ` · ${ids.length} selected` : ""}
      <button
        type="button"
        aria-label={`Mark ${label}`}
        disabled={busy || ids.length === 0}
        onClick={() => void markStep(ids, step, label)}
      >
        Mark
      </button>
    </span>
  );

  const lists = body ?? { payTypes: [], categories: [], vendors: [], lines: [], totalCount: 0 };

  return (
    <section className="external-register" aria-label="External accounts">
      <div className="external-find">
        <div className="external-find-grid" role="group" aria-label="Search">
          <span className="external-find-label">Search</span>
          <input
            aria-label="Search external register"
            value={search}
            onChange={(event) => setSearch(event.target.value)}
            placeholder="Any column"
          />
          <span className="external-find-label">Pending</span>
          <span className="external-find-count">{pending.length}</span>
          <span className="external-find-label">Complete</span>
          <span className="external-find-count">{completed.length}</span>
        </div>
      </div>
      <ol className="external-steps">
        <li>Fill the row and press Add. It stays open.</li>
        <li>Tick the rows in one transfer, mark Transfer, then do the same for Enter into bill pay and Pay bill.</li>
        <li>The charge moves to Completed only after all three steps are marked.</li>
      </ol>
      {status ? <p role="status">{status}</p> : null}
      <h3 className="external-open-head">
        Open · {pending.length}
        {stepTotal("Transfer", "transfer", transferIds)}
        {stepTotal("Enter into bill pay", "billpay", billpayIds)}
        {stepTotal("Pay bill", "pay", payIds)}
      </h3>
      <div className="table-wrap external-register-wrap external-open-wrap">
        <table aria-label="Pending external charges">
          <thead>
            <tr>
              <th>Pay type</th>
              <th>Date</th>
              <th>Total spent</th>
              <th>Category</th>
              <th>Vendor</th>
              <th>Description</th>
              <th>Transfer</th>
              <th>Bill pay</th>
              <th>Pay bill</th>
            </tr>
          </thead>
          <tbody>
            <tr className="external-entry">
              <td>
                <input
                  aria-label="New pay type"
                  list="external-pay-types"
                  value={entry.payType}
                  onChange={(event) => patchEntry({ payType: event.target.value })}
                  onKeyDown={addOnEnter}
                />
              </td>
              <td>
                <DateStep
                  ariaLabel="New date"
                  value={entry.occurredOn}
                  onChange={(occurredOn) => patchEntry({ occurredOn })}
                  onKeyDown={(event) => {
                    if (dateStepKey(event, entry.occurredOn, (occurredOn) => patchEntry({ occurredOn }))) return;
                    addOnEnter(event);
                  }}
                />
              </td>
              <td>
                <input
                  aria-label="New total spent"
                  value={entry.amount}
                  onChange={(event) => patchEntry({ amount: event.target.value })}
                  onKeyDown={addOnEnter}
                />
              </td>
              <td>
                <input
                  aria-label="New category"
                  list="external-categories"
                  value={entry.category}
                  onChange={(event) => patchEntry({ category: event.target.value })}
                  onKeyDown={addOnEnter}
                />
              </td>
              <td>
                <input
                  aria-label="New vendor"
                  list="external-vendors"
                  value={entry.vendor}
                  onChange={(event) => patchEntry({ vendor: event.target.value })}
                  onKeyDown={addOnEnter}
                />
              </td>
              <td>
                <input
                  aria-label="New description"
                  value={entry.description}
                  onChange={(event) => patchEntry({ description: event.target.value })}
                  onKeyDown={addOnEnter}
                />
              </td>
              <td colSpan={3} className="external-entry-actions">
                <button
                  type="button"
                  aria-label="Add charge"
                  className={dirty ? "is-unsaved" : undefined}
                  disabled={busy || !dirty}
                  onClick={() => void save()}
                >
                  Add
                </button>
                <button
                  type="button"
                  aria-label="Cancel external register edits"
                  disabled={busy || !dirty}
                  onClick={() => void load()}
                >
                  Cancel
                </button>
              </td>
            </tr>
            {pending.map((line) => {
              const draft = drafts[line.lineId];
              const view = draft ?? line;
              const editing = editingId === line.lineId;
              return (
                <tr key={line.lineId} onClick={() => startEdit(line)}>
                  {editing && draft ? (
                    <>
                      <td>
                        <input
                          aria-label={`Pay type ${line.lineId}`}
                          list="external-pay-types"
                          value={draft.payType}
                          onChange={(event) =>
                            patchDraft(line.lineId, { payType: event.target.value })
                          }
                        />
                      </td>
                      <td>
                        <DateStep
                          ariaLabel={`Date ${line.lineId}`}
                          value={draft.occurredOn ?? ""}
                          onChange={(occurredOn) => patchDraft(line.lineId, { occurredOn: occurredOn || null })}
                          onKeyDown={(event) => {
                            dateStepKey(event, draft.occurredOn ?? "", (occurredOn) =>
                              patchDraft(line.lineId, { occurredOn }),
                            );
                          }}
                        />
                      </td>
                      <td>
                        <input
                          aria-label={`Total spent ${line.lineId}`}
                          value={amountInputs[line.lineId] ?? amountText(draft.amountMinor, draft.scale)}
                          onChange={(event) => {
                            const text = event.target.value;
                            setAmountInputs((current) => ({ ...current, [line.lineId]: text }));
                            const amountMinor = parseAmount(text);
                            if (amountMinor == null) return;
                            patchDraft(line.lineId, { amountMinor });
                          }}
                        />
                      </td>
                      <td>
                        <input
                          aria-label={`Category ${line.lineId}`}
                          list="external-categories"
                          value={draft.category}
                          onChange={(event) =>
                            patchDraft(line.lineId, { category: event.target.value })
                          }
                        />
                      </td>
                      <td>
                        <input
                          aria-label={`Vendor ${line.lineId}`}
                          list="external-vendors"
                          value={draft.vendor}
                          onChange={(event) =>
                            patchDraft(line.lineId, { vendor: event.target.value })
                          }
                        />
                      </td>
                      <td>
                        <input
                          aria-label={`Description ${line.lineId}`}
                          value={draft.description}
                          onChange={(event) =>
                            patchDraft(line.lineId, { description: event.target.value })
                          }
                        />
                      </td>
                      {stepCheck(line, view, Boolean(line.stepTransfer), transferIds, setTransferIds, "Transfer")}
                      {stepCheck(line, view, Boolean(line.stepBillpay), billpayIds, setBillpayIds, "Enter into bill pay")}
                      {stepCheck(line, view, Boolean(line.stepPay), payIds, setPayIds, "Pay bill")}
                    </>
                  ) : (
                    <>
                      <td>{view.payType}</td>
                      <td>{view.occurredOn ?? ""}</td>
                      <td className="numeric">{amountText(view.amountMinor, view.scale)}</td>
                      <td>{view.category}</td>
                      <td>{view.vendor}</td>
                      <td>{view.description}</td>
                      {stepCheck(line, view, Boolean(line.stepTransfer), transferIds, setTransferIds, "Transfer")}
                      {stepCheck(line, view, Boolean(line.stepBillpay), billpayIds, setBillpayIds, "Enter into bill pay")}
                      {stepCheck(line, view, Boolean(line.stepPay), payIds, setPayIds, "Pay bill")}
                    </>
                  )}
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
      <h3>Completed · {completedVisible.length}</h3>
      <div className="table-wrap external-register-wrap external-completed-wrap">
        <table aria-label="Completed external charges">
          <thead>
            <tr>
              {(
                [
                  ["Pay type", "payType", "Filter pay type"],
                  ["Date", "occurredOn", "Filter date"],
                  ["Total spent", "amount", "Filter total spent"],
                  ["Category", "category", "Filter category"],
                  ["Vendor", "vendor", "Filter vendor"],
                  ["Description", "description", "Filter description"],
                  ["Completed", "trueUpOn", "Filter completed date"],
                ] as const
              ).map(([label, key, aria]) => (
                <th key={key}>
                  <div>{label}</div>
                  <select
                    className="external-col-filter"
                    aria-label={aria}
                    value={colFilters[key]}
                    onChange={(event) => setFilter(key, event.target.value)}
                  >
                    <option value="">All</option>
                    {choiceLists[key].map((value) => (
                      <option key={value} value={value}>
                        {value}
                      </option>
                    ))}
                  </select>
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {completedVisible.map((line) => {
              const view = viewOf(line);
              return (
                <tr key={line.lineId} className="is-complete">
                  <td>{view.payType}</td>
                  <td>{view.occurredOn ?? ""}</td>
                  <td className="numeric">{amountText(view.amountMinor, view.scale)}</td>
                  <td>{view.category}</td>
                  <td>{view.vendor}</td>
                  <td>{view.description}</td>
                  <td className="external-green-date">{view.trueUpOn ?? ""}</td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
      <datalist id="external-pay-types">
        {lists.payTypes.map((name) => (
          <option key={name} value={name} />
        ))}
      </datalist>
      <datalist id="external-categories">
        {lists.categories.map((name) => (
          <option key={name} value={name} />
        ))}
      </datalist>
      <datalist id="external-vendors">
        {lists.vendors.map((name) => (
          <option key={name} value={name} />
        ))}
      </datalist>
    </section>
  );
}
