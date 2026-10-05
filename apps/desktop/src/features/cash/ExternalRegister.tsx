import { useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
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
  stepBillpayDeposit?: boolean;
  stepPay?: boolean;
  stepWithdrawal?: boolean;
  stepTransferOn?: string | null;
  stepBillpayOn?: string | null;
  stepBillpayDepositOn?: string | null;
  stepPayOn?: string | null;
  stepWithdrawalOn?: string | null;
};

type RegisterBody = {
  lines: RegisterLine[];
  payTypes: string[];
  categories: string[];
  vendors: string[];
  totalCount: number;
};

type ExportPreview = {
  printHtml: string;
  defaultFileName: string;
  bytesBase64: string;
  format: string;
};

type EntryDraft = {
  payType: string;
  occurredOn: string;
  amount: string;
  category: string;
  vendor: string;
  description: string;
};

const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

function monthDay(iso: string | null | undefined): string {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec((iso ?? "").trim());
  if (!match) return "";
  const month = MONTHS[Number(match[2]) - 1];
  if (!month) return "";
  return `${month} ${Number(match[3])}`;
}

type OpenSortKey =
  | "payType"
  | "occurredOn"
  | "amount"
  | "category"
  | "vendor"
  | "description"
  | "transfer"
  | "billpayDeposit"
  | "pay"
  | "withdrawal";

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

/** Completed column: stored true-up date, else today until the host stamps it. */
function completedDateOf(line: RegisterLine): string {
  const stored = line.trueUpOn?.trim();
  return stored || todayIso();
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

function lineYear(line: RegisterLine): string | null {
  const match = /^(\d{4})-/.exec((line.occurredOn ?? "").trim());
  return match ? match[1] : null;
}

const KNOWN_PAY_ACCOUNTS = new Set([
  "UCARD",
  "CAP",
  "Checking",
  "SAB",
  "PPMC",
  "PPALMC",
  "PPAL",
  "PAYPAL",
  "PM",
  "BJS",
  "BOA",
  "AMZ",
  "HSA",
  "UP",
  "EBAY",
  "OPTIM",
  "Check-U",
  "Check-C",
]);

function ranked2026(
  lines: RegisterLine[],
  field: "payType" | "category" | "vendor",
  allow?: (name: string) => boolean,
): string[] {
  const counts = new Map<string, number>();
  for (const line of lines) {
    if (!(line.occurredOn ?? "").startsWith("2026")) continue;
    const name = line[field].trim();
    if (!name || (allow && !allow(name))) continue;
    counts.set(name, (counts.get(name) ?? 0) + 1);
  }
  return [...counts.keys()].sort(byFrequency(counts));
}

function SuggestInput({
  value,
  options,
  ariaLabel,
  onChange,
  onKeyDown,
}: {
  value: string;
  options: string[];
  ariaLabel: string;
  onChange: (next: string) => void;
  onKeyDown?: (event: { key: string; preventDefault: () => void }) => void;
}) {
  const inputRef = useRef<HTMLInputElement>(null);
  const [open, setOpen] = useState(false);
  const [active, setActive] = useState(0);
  const [place, setPlace] = useState({ top: 0, left: 0, width: 0 });
  const query = value.trim().toLowerCase();
  const matches = options
    .filter((name) => {
      const key = name.toLowerCase();
      if (!query) return true;
      return key.startsWith(query) && key !== query;
    })
    .slice(0, 12);

  const placeList = () => {
    const rect = inputRef.current?.getBoundingClientRect();
    if (!rect) return;
    setPlace({ top: rect.bottom + 2, left: rect.left, width: Math.max(rect.width, 8 * 16) });
  };

  useEffect(() => {
    setActive(0);
  }, [value]);

  useEffect(() => {
    if (!open) return;
    placeList();
    const move = () => placeList();
    window.addEventListener("scroll", move, true);
    window.addEventListener("resize", move);
    return () => {
      window.removeEventListener("scroll", move, true);
      window.removeEventListener("resize", move);
    };
  }, [open, value, options]);

  return (
    <>
      <input
        ref={inputRef}
        aria-label={ariaLabel}
        aria-autocomplete="list"
        aria-expanded={open && matches.length > 0}
        autoComplete="off"
        value={value}
        onFocus={() => setOpen(true)}
        onBlur={() => setOpen(false)}
        onChange={(event) => {
          onChange(event.target.value);
          setOpen(true);
        }}
        onKeyDown={(event) => {
          if (open && matches.length > 0) {
            if (event.key === "ArrowDown") {
              event.preventDefault();
              setActive((index) => Math.min(index + 1, matches.length - 1));
              return;
            }
            if (event.key === "ArrowUp") {
              event.preventDefault();
              setActive((index) => Math.max(index - 1, 0));
              return;
            }
            if (event.key === "Escape") {
              event.preventDefault();
              setOpen(false);
              return;
            }
            if (event.key === "Enter" && query) {
              event.preventDefault();
              onChange(matches[Math.min(active, matches.length - 1)]);
              setOpen(false);
              return;
            }
          }
          onKeyDown?.(event);
        }}
      />
      {open && matches.length > 0 ? (
        <div
          className="external-suggest"
          role="listbox"
          style={{ top: place.top, left: place.left, width: place.width }}
        >
          {matches.map((name, index) => (
            <div
              key={name}
              role="option"
              aria-selected={index === active}
              className={index === active ? "is-active" : undefined}
              onMouseDown={(event) => {
                event.preventDefault();
                event.stopPropagation();
                onChange(name);
                setOpen(false);
              }}
            >
              {name}
            </div>
          ))}
        </div>
      ) : null}
    </>
  );
}

function bump(counts: Map<string, number>, value: string) {
  const text = value.trim();
  if (!text) return;
  counts.set(text, (counts.get(text) ?? 0) + 1);
}

function byFrequency(counts: Map<string, number>) {
  return (a: string, b: string) => {
    const diff = (counts.get(b) ?? 0) - (counts.get(a) ?? 0);
    if (diff !== 0) return diff;
    return a.localeCompare(b, undefined, { numeric: true, sensitivity: "base" });
  };
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
  const [checkedYears, setCheckedYears] = useState<string[]>(["2026"]);
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
  const [openSort, setOpenSort] = useState<{ key: OpenSortKey; dir: "asc" | "desc" } | null>(null);
  const editRowRef = useRef<HTMLTableRowElement>(null);
  const [transferIds, setTransferIds] = useState<string[]>([]);
  const [billpayDepositIds, setBillpayDepositIds] = useState<string[]>([]);
  const [payIds, setPayIds] = useState<string[]>([]);
  const [withdrawalIds, setWithdrawalIds] = useState<string[]>([]);
  const [status, setStatus] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [exportPreview, setExportPreview] = useState<ExportPreview | null>(null);
  const [exportLoading, setExportLoading] = useState(false);

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

  const years = useMemo(() => {
    const found = new Set<string>(["2026"]);
    for (const line of body?.lines ?? []) {
      const year = lineYear(line);
      if (year) found.add(year);
    }
    return [...found].sort((a, b) => b.localeCompare(a));
  }, [body]);

  const valueFrequency = useMemo(() => {
    const counts = {
      payType: new Map<string, number>(),
      occurredOn: new Map<string, number>(),
      amount: new Map<string, number>(),
      category: new Map<string, number>(),
      vendor: new Map<string, number>(),
      description: new Map<string, number>(),
      trueUpOn: new Map<string, number>(),
    };
    for (const line of body?.lines ?? []) {
      bump(counts.payType, line.payType);
      bump(counts.occurredOn, line.occurredOn ?? "");
      bump(counts.amount, amountText(line.amountMinor, line.scale));
      bump(counts.category, line.category);
      bump(counts.vendor, line.vendor);
      bump(counts.description, line.description);
      bump(counts.trueUpOn, line.trueUpOn ?? "");
    }
    return counts;
  }, [body]);

  const suggestions = useMemo(
    () => ({
      payType: ranked2026(body?.lines ?? [], "payType", (name) => KNOWN_PAY_ACCOUNTS.has(name)),
      category: ranked2026(body?.lines ?? [], "category"),
      vendor: ranked2026(body?.lines ?? [], "vendor"),
    }),
    [body],
  );

  const shown = useMemo(() => {
    const lines = body?.lines ?? [];
    const allowed = new Set(checkedYears);
    return lines.filter((line) => {
      const year = lineYear(line);
      if (year && !allowed.has(year)) return false;
      return lineMatches(line, search);
    });
  }, [body, search, checkedYears]);

  const patchEntry = (patch: Partial<EntryDraft>) => {
    setEntry((current) => ({ ...current, ...patch }));
  };

  useEffect(() => {
    if (!editingId) return;
    const close = (event: MouseEvent) => {
      const target = event.target;
      if (!(target instanceof Node)) return;
      if (editRowRef.current?.contains(target)) return;
      setEditingId(null);
    };
    document.addEventListener("mousedown", close);
    return () => document.removeEventListener("mousedown", close);
  }, [editingId]);

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
        tickedOn: todayIso(),
      });
      const next = await readBody(result);
      setBody(next);
      if (step === "transfer") setTransferIds((current) => current.filter((id) => !ids.includes(id)));
      if (step === "billpay_deposit") {
        setBillpayDepositIds((current) => current.filter((id) => !ids.includes(id)));
      }
      if (step === "pay") setPayIds((current) => current.filter((id) => !ids.includes(id)));
      if (step === "withdrawal") setWithdrawalIds((current) => current.filter((id) => !ids.includes(id)));
      setStatus(`${label} marked.`);
    } catch (err: unknown) {
      setStatus(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  };

  const exportLinesPayload = () =>
    completedVisible.map((line) => {
      const view = viewOf(line);
      return {
        payType: view.payType,
        occurredOn: view.occurredOn ?? null,
        amountMinor: view.amountMinor,
        scale: view.scale,
        category: view.category,
        vendor: view.vendor,
        description: view.description,
        trueUpOn: completedDateOf(view),
      };
    });

  const requestCompletedExport = async () => {
    if (exportLoading || completedVisible.length === 0) return;
    setExportLoading(true);
    setBusy(true);
    setStatus(null);
    try {
      const result = await client.executeQuery("ExternalRegisterExportGet", {
        format: "html",
        printedAt: new Date().toISOString().slice(0, 16) + "Z",
        lines: exportLinesPayload(),
      });
      if (!result.ok || !result.bodyJson) {
        setStatus(`Print / Export failed: ${result.errorCode ?? "error"}`);
        return;
      }
      setExportPreview(JSON.parse(result.bodyJson) as ExportPreview);
    } catch (err: unknown) {
      setStatus(err instanceof Error ? err.message : String(err));
    } finally {
      setExportLoading(false);
      setBusy(false);
    }
  };

  const commitCompletedExport = async (action: "print" | "pdf" | "excel") => {
    if (!exportPreview) return;
    if (action === "print") {
      const html = exportPreview.printHtml;
      setExportPreview(null);
      const frame = document.createElement("iframe");
      frame.setAttribute("aria-hidden", "true");
      frame.style.position = "fixed";
      frame.style.right = "0";
      frame.style.bottom = "0";
      frame.style.width = "0";
      frame.style.height = "0";
      frame.style.border = "0";
      document.body.appendChild(frame);
      const doc = frame.contentDocument;
      if (doc) {
        doc.open();
        doc.write(html);
        doc.close();
        frame.contentWindow?.focus();
        frame.contentWindow?.print();
      }
      window.setTimeout(() => frame.remove(), 1000);
      return;
    }
    setExportLoading(true);
    setBusy(true);
    try {
      const result = await client.executeQuery("ExternalRegisterExportGet", {
        format: action === "excel" ? "xlsx" : "pdf",
        printedAt: new Date().toISOString().slice(0, 16) + "Z",
        lines: exportLinesPayload(),
      });
      if (!result.ok || !result.bodyJson) {
        setStatus(`Print / Export failed: ${result.errorCode ?? "error"}`);
        return;
      }
      const body = JSON.parse(result.bodyJson) as ExportPreview;
      const bytes = Uint8Array.from(atob(body.bytesBase64), (c) => c.charCodeAt(0));
      await invoke("save_local_bytes", {
        defaultFileName: body.defaultFileName,
        bytes: Array.from(bytes),
      });
      setExportPreview(null);
      setStatus(`Saved ${body.defaultFileName} on this computer.`);
    } catch (err: unknown) {
      setStatus(`Save failed: ${String(err)}`);
    } finally {
      setExportLoading(false);
      setBusy(false);
    }
  };

  const viewOf = (line: RegisterLine) => drafts[line.lineId] ?? line;
  const stepsDone = (line: RegisterLine) =>
    Boolean(
      line.stepTransfer &&
        line.stepBillpayDeposit &&
        line.stepPay &&
        line.stepWithdrawal,
    );
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
      columnHit(completedDateOf(view).trim(), colFilters.trueUpOn)
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
    if (completedDateOf(view).trim()) completedChoices.trueUpOn.add(completedDateOf(view).trim());
  }
  const rank = {
    payType: byFrequency(valueFrequency.payType),
    occurredOn: byFrequency(valueFrequency.occurredOn),
    amount: byFrequency(valueFrequency.amount),
    category: byFrequency(valueFrequency.category),
    vendor: byFrequency(valueFrequency.vendor),
    description: byFrequency(valueFrequency.description),
    trueUpOn: byFrequency(valueFrequency.trueUpOn),
  };
  const choiceLists = {
    payType: [...completedChoices.payType].sort(rank.payType),
    occurredOn: [...completedChoices.occurredOn].sort(rank.occurredOn),
    amount: [...completedChoices.amount].sort(rank.amount),
    category: [...completedChoices.category].sort(rank.category),
    vendor: [...completedChoices.vendor].sort(rank.vendor),
    description: [...completedChoices.description].sort(rank.description),
    trueUpOn: [...completedChoices.trueUpOn].sort(rank.trueUpOn),
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
    storedOn: string | null | undefined,
  ) => {
    const selected = ids.includes(line.lineId);
    const stamp = done ? monthDay(storedOn) : selected ? monthDay(todayIso()) : "";
    return (
      <td onClick={(event) => event.stopPropagation()}>
        <span className="external-step-tick">
          <input
            type="checkbox"
            aria-label={`${label} ${view.payType} ${view.occurredOn ?? ""} ${amountText(view.amountMinor, view.scale)}${stamp ? ` ticked ${stamp}` : ""}`}
            checked={done || selected}
            disabled={done || busy}
            onClick={(event) => event.stopPropagation()}
            onChange={() => {
              if (!done) toggleId(line.lineId, ids, setIds);
            }}
          />
          {stamp ? <span className="external-step-date">{stamp}</span> : null}
        </span>
      </td>
    );
  };
  const toggleOpenSort = (key: OpenSortKey) => {
    setOpenSort((current) => {
      if (!current || current.key !== key) return { key, dir: "asc" };
      return { key, dir: current.dir === "asc" ? "desc" : "asc" };
    });
  };
  const stepSortValue = (line: RegisterLine, ids: string[], done: boolean, storedOn: string | null | undefined) => {
    if (done) return `2${storedOn ?? ""}`;
    if (ids.includes(line.lineId)) return "1";
    return "0";
  };
  const openRows = !openSort
    ? pending
    : [...pending].sort((left, right) => {
        const a = viewOf(left);
        const b = viewOf(right);
        const cmp = (() => {
          switch (openSort.key) {
            case "amount":
              return a.amountMinor - b.amountMinor;
            case "occurredOn":
              return (a.occurredOn ?? "").localeCompare(b.occurredOn ?? "");
            case "transfer":
              return stepSortValue(left, transferIds, Boolean(left.stepTransfer), left.stepTransferOn).localeCompare(
                stepSortValue(right, transferIds, Boolean(right.stepTransfer), right.stepTransferOn),
              );
            case "billpayDeposit":
              return stepSortValue(
                left,
                billpayDepositIds,
                Boolean(left.stepBillpayDeposit),
                left.stepBillpayDepositOn,
              ).localeCompare(
                stepSortValue(
                  right,
                  billpayDepositIds,
                  Boolean(right.stepBillpayDeposit),
                  right.stepBillpayDepositOn,
                ),
              );
            case "pay":
              return stepSortValue(left, payIds, Boolean(left.stepPay), left.stepPayOn).localeCompare(
                stepSortValue(right, payIds, Boolean(right.stepPay), right.stepPayOn),
              );
            case "withdrawal":
              return stepSortValue(
                left,
                withdrawalIds,
                Boolean(left.stepWithdrawal),
                left.stepWithdrawalOn,
              ).localeCompare(
                stepSortValue(
                  right,
                  withdrawalIds,
                  Boolean(right.stepWithdrawal),
                  right.stepWithdrawalOn,
                ),
              );
            case "payType":
              return a.payType.localeCompare(b.payType, undefined, { numeric: true, sensitivity: "base" });
            case "category":
              return a.category.localeCompare(b.category, undefined, { numeric: true, sensitivity: "base" });
            case "vendor":
              return a.vendor.localeCompare(b.vendor, undefined, { numeric: true, sensitivity: "base" });
            case "description":
              return a.description.localeCompare(b.description, undefined, { numeric: true, sensitivity: "base" });
          }
        })();
        return openSort.dir === "asc" ? cmp : -cmp;
      });
  const sortHead = (label: string, key: OpenSortKey) => {
    const active = openSort?.key === key;
    return (
      <th aria-sort={active ? (openSort.dir === "asc" ? "ascending" : "descending") : "none"} className="sortable">
        <button type="button" onClick={() => toggleOpenSort(key)}>
          {label}
          {active ? (openSort.dir === "asc" ? " ↑" : " ↓") : ""}
        </button>
      </th>
    );
  };
  const sortStepHead = (line1: string, line2: string, key: OpenSortKey) => {
    const active = openSort?.key === key;
    const label = `${line1} ${line2}`.trim();
    const sortMark = active ? (openSort!.dir === "asc" ? " ↑" : " ↓") : "";
    return (
      <th
        aria-sort={active ? (openSort.dir === "asc" ? "ascending" : "descending") : "none"}
        className="sortable open-step-head"
      >
        <button type="button" aria-label={label} onClick={() => toggleOpenSort(key)}>
          <span className="open-step-head-label">
            <span className="open-step-head-line">{line1}</span>
            <span className="open-step-head-line">
              {line2}
              {sortMark}
            </span>
          </span>
        </button>
      </th>
    );
  };
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

  const openPayTypeTotals = (() => {
    const map = new Map<string, { amountMinor: number; scale: number; count: number }>();
    for (const line of pending) {
      const view = viewOf(line);
      const key = view.payType.trim() || "(no pay type)";
      const cur = map.get(key) ?? {
        amountMinor: 0,
        scale: view.scale || 2,
        count: 0,
      };
      cur.amountMinor += view.amountMinor;
      cur.count += 1;
      if ((view.scale || 2) > cur.scale) cur.scale = view.scale || 2;
      map.set(key, cur);
    }
    return [...map.entries()].sort(([a], [b]) =>
      a.localeCompare(b, undefined, { numeric: true, sensitivity: "base" }),
    );
  })();

  return (
    <section className="external-register" aria-label="Checking and Credit Transactions -CCT">
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
          {years.map((year) => (
            <span key={year} className="external-year-tick">
              <input
                type="checkbox"
                aria-label={`Include ${year}`}
                checked={checkedYears.includes(year)}
                onChange={() => {
                  setCheckedYears((current) =>
                    current.includes(year) ? current.filter((item) => item !== year) : [...current, year],
                  );
                }}
              />
              {year}
            </span>
          ))}
        </div>
      </div>
      <div className="external-steps-row">
        <ol className="external-steps">
          <li>Fill the row and press Add. It stays open.</li>
          <li>Tick rows to complete steps</li>
          <li>Open charges move to Completed when all steps are done</li>
        </ol>
        <div
          className="external-open-paytype-totals"
          aria-label="Open totals by pay type"
        >
          {openPayTypeTotals.length === 0 ? (
            <span className="external-open-paytype-empty">No open accounts</span>
          ) : (
            openPayTypeTotals.map(([payType, tot]) => (
              <span key={payType} className="external-open-paytype-total">
                <span className="external-open-paytype-name">{payType}</span>
                <span className="external-open-paytype-amt">
                  {amountText(tot.amountMinor, tot.scale)}
                </span>
                {tot.count > 1 ? (
                  <span className="external-open-paytype-count">· {tot.count}</span>
                ) : null}
              </span>
            ))
          )}
        </div>
      </div>
      {status ? <p role="status">{status}</p> : null}
      <h3 className="external-open-head">
        Open · {pending.length}
        {stepTotal("Transfer", "transfer", transferIds)}
        {stepTotal("Bill Pay Deposit", "billpay_deposit", billpayDepositIds)}
        {stepTotal("Pay Bill", "pay", payIds)}
        {stepTotal("Bill Pay Withdrawal", "withdrawal", withdrawalIds)}
      </h3>
      <div className="table-wrap external-register-wrap external-open-wrap">
        <table aria-label="Pending external charges">
          <colgroup>
            <col className="open-paytype-col" />
            <col className="open-date-col" />
            <col className="open-spent-col" />
            <col className="open-category-col" />
            <col className="open-vendor-col" />
            <col className="open-desc-col" />
            <col className="open-tick-col" />
            <col className="open-tick-col" />
            <col className="open-tick-col" />
            <col className="open-tick-col" />
          </colgroup>
          <thead>
            <tr>
              {sortHead("Pay type", "payType")}
              {sortHead("Date", "occurredOn")}
              {sortHead("Total spent", "amount")}
              {sortHead("Category", "category")}
              {sortHead("Vendor", "vendor")}
              {sortHead("Description", "description")}
              {sortHead("Transfer", "transfer")}
              {sortStepHead("Bill Pay", "Deposit", "billpayDeposit")}
              {sortStepHead("Pay", "Bill", "pay")}
              {sortStepHead("Bill Pay", "Withdrawal", "withdrawal")}
            </tr>
          </thead>
          <tbody>
            <tr className="external-entry">
              <td>
                <SuggestInput
                  ariaLabel="New pay type"
                  options={suggestions.payType}
                  value={entry.payType}
                  onChange={(payType) => patchEntry({ payType })}
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
                <SuggestInput
                  ariaLabel="New category"
                  options={suggestions.category}
                  value={entry.category}
                  onChange={(category) => patchEntry({ category })}
                  onKeyDown={addOnEnter}
                />
              </td>
              <td>
                <SuggestInput
                  ariaLabel="New vendor"
                  options={suggestions.vendor}
                  value={entry.vendor}
                  onChange={(vendor) => patchEntry({ vendor })}
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
              <td colSpan={4} className="external-entry-actions">
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
            {openRows.map((line) => {
              const draft = drafts[line.lineId];
              const view = draft ?? line;
              const editing = editingId === line.lineId;
              return (
                <tr
                  key={line.lineId}
                  ref={editing ? editRowRef : undefined}
                  onClick={() => startEdit(line)}
                  onBlur={(event) => {
                    if (!editing) return;
                    const next = event.relatedTarget;
                    if (next instanceof Node && editRowRef.current?.contains(next)) return;
                    if (next == null) return;
                    setEditingId(null);
                  }}
                >
                  {editing && draft ? (
                    <>
                      <td>
                        <SuggestInput
                          ariaLabel={`Pay type ${line.lineId}`}
                          options={suggestions.payType}
                          value={draft.payType}
                          onChange={(payType) => patchDraft(line.lineId, { payType })}
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
                        <SuggestInput
                          ariaLabel={`Category ${line.lineId}`}
                          options={suggestions.category}
                          value={draft.category}
                          onChange={(category) => patchDraft(line.lineId, { category })}
                        />
                      </td>
                      <td>
                        <SuggestInput
                          ariaLabel={`Vendor ${line.lineId}`}
                          options={suggestions.vendor}
                          value={draft.vendor}
                          onChange={(vendor) => patchDraft(line.lineId, { vendor })}
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
                      {stepCheck(line, view, Boolean(line.stepTransfer), transferIds, setTransferIds, "Transfer", line.stepTransferOn)}
                      {stepCheck(
                        line,
                        view,
                        Boolean(line.stepBillpayDeposit),
                        billpayDepositIds,
                        setBillpayDepositIds,
                        "Bill Pay Deposit",
                        line.stepBillpayDepositOn,
                      )}
                      {stepCheck(line, view, Boolean(line.stepPay), payIds, setPayIds, "Pay Bill", line.stepPayOn)}
                      {stepCheck(
                        line,
                        view,
                        Boolean(line.stepWithdrawal),
                        withdrawalIds,
                        setWithdrawalIds,
                        "Bill Pay Withdrawal",
                        line.stepWithdrawalOn,
                      )}
                    </>
                  ) : (
                    <>
                      <td title={view.payType}>{view.payType}</td>
                      <td>{view.occurredOn ?? ""}</td>
                      <td className="numeric">{amountText(view.amountMinor, view.scale)}</td>
                      <td title={view.category}>{view.category}</td>
                      <td title={view.vendor}>{view.vendor}</td>
                      <td title={view.description}>{view.description}</td>
                      {stepCheck(line, view, Boolean(line.stepTransfer), transferIds, setTransferIds, "Transfer", line.stepTransferOn)}
                      {stepCheck(
                        line,
                        view,
                        Boolean(line.stepBillpayDeposit),
                        billpayDepositIds,
                        setBillpayDepositIds,
                        "Bill Pay Deposit",
                        line.stepBillpayDepositOn,
                      )}
                      {stepCheck(line, view, Boolean(line.stepPay), payIds, setPayIds, "Pay Bill", line.stepPayOn)}
                      {stepCheck(
                        line,
                        view,
                        Boolean(line.stepWithdrawal),
                        withdrawalIds,
                        setWithdrawalIds,
                        "Bill Pay Withdrawal",
                        line.stepWithdrawalOn,
                      )}
                    </>
                  )}
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
      <h3 className="external-open-head">
        Completed · {completedVisible.length}
        <button
          type="button"
          aria-label="Export"
          disabled={busy || exportLoading || completedVisible.length === 0}
          onClick={() => void requestCompletedExport()}
        >
          {exportLoading ? "Export…" : "Export"}
        </button>
      </h3>
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
                  <td className="external-green-date">{completedDateOf(view)}</td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
      {exportPreview ? (
        <div
          className="home-av-dialog-backdrop"
          onClick={() => setExportPreview(null)}
        >
          <div
            role="dialog"
            aria-modal="true"
            aria-label="CCT Completed export preview"
            className="home-av-dialog income-export-preview-dialog"
            onClick={(event) => event.stopPropagation()}
          >
            <header>
              <h3>Print / Export preview</h3>
              <button
                type="button"
                aria-label="Cancel CCT export"
                onClick={() => setExportPreview(null)}
              >
                Cancel
              </button>
            </header>
            <iframe
              className="income-export-preview-frame"
              title="CCT Completed export preview"
              srcDoc={exportPreview.printHtml}
            />
            <p>Review the completed charges, then choose print, PDF, or Excel.</p>
            <div
              className="income-export-preview-actions"
              aria-label="Confirm CCT Completed export"
            >
              <button
                type="button"
                aria-label="Print to page"
                disabled={busy || exportLoading}
                onClick={() => void commitCompletedExport("print")}
              >
                Print
              </button>
              <button
                type="button"
                aria-label="Save PDF"
                disabled={busy || exportLoading}
                onClick={() => void commitCompletedExport("pdf")}
              >
                Save PDF
              </button>
              <button
                type="button"
                aria-label="Export Excel"
                disabled={busy || exportLoading}
                onClick={() => void commitCompletedExport("excel")}
              >
                Export Excel
              </button>
              <button
                type="button"
                aria-label="Cancel CCT export"
                disabled={busy || exportLoading}
                onClick={() => setExportPreview(null)}
              >
                Cancel
              </button>
            </div>
          </div>
        </div>
      ) : null}
    </section>
  );
}
