import { useEffect, useMemo, useState } from "react";
import { LocalTauriFinanceClient } from "../../financeClient";

const client = new LocalTauriFinanceClient();

const FREQUENCIES = [
  ["weekly", "Weekly"],
  ["monthly", "Monthly"],
  ["quarterly", "Quarterly"],
  ["annual", "Annual"],
] as const;

type RegisterLine = {
  lineId: string;
  payType: string;
  occurredOn?: string | null;
  amountMinor: number;
  category: string;
  vendor: string;
  description: string;
  completed?: boolean;
  stepTransfer?: boolean;
  stepBillpay?: boolean;
  stepBillpayDeposit?: boolean;
  stepPay?: boolean;
  stepWithdrawal?: boolean;
};

type LoanVendorLine = {
  lineId: string;
  occurredOn: string;
  amountMinor: number;
  dueOn: string;
  description: string;
  principalMinor: number;
  interestMinor: number;
  escrowMinor: number;
  lateMinor: number;
  principalBalanceMinor: number;
  escrowBalanceMinor: number;
};

type LoanVendor = {
  termPayments: number;
  paymentsMade: number;
  principalPaidMinor: number;
  interestPaidMinor: number;
  escrowPaidMinor: number;
  principalBalanceMinor: number;
  escrowBalanceMinor: number;
  note?: string | null;
  lines: LoanVendorLine[];
};

type ManagedAccount = {
  accountId: string;
  name: string;
  kind: string;
  chargesInterest: boolean;
  startingMinor: number | null;
  currentMinor: number | null;
  paymentMinor: number | null;
  reductionMinor: number | null;
  pendingMinor: number;
  financeMinor: number | null;
  dueOn?: string | null;
  aprPpm?: number | null;
  frequency?: string | null;
  paidThrough?: string | null;
  registerKey: string;
  payProcess?: string | null;
  linkedElementId?: string | null;
  lines: RegisterLine[];
  vendor?: LoanVendor | null;
};

type ManagerBody = {
  accounts: ManagedAccount[];
};

type ElementChoice = {
  elementId: string;
  account: string;
  kind: string;
  note: string;
  amountMinor: number;
};

const PROCESSES = [
  ["week_ahead", "Week Ahead loan"],
  ["register", "Checking and Credit"],
  ["element", "Linked element"],
] as const;

type SetupDraft = {
  name: string;
  process: string;
  elementId: string;
  starting: string;
  current: string;
  payment: string;
  rate: string;
  frequency: string;
  due: string;
};

function blankDraft(): SetupDraft {
  return {
    name: "",
    process: "",
    elementId: "",
    starting: "",
    current: "",
    payment: "",
    rate: "",
    frequency: "",
    due: "",
  };
}

function processLabel(account: ManagedAccount, elements: ElementChoice[]): string {
  if (account.payProcess === "week_ahead") return "Week Ahead";
  if (account.payProcess === "register") return "Checking and Credit";
  if (account.payProcess === "element") {
    const linked = elements.find((element) => element.elementId === account.linkedElementId);
    const name = linked?.note || linked?.account;
    return name ? `Linked element · ${name}` : "Linked element · choose one";
  }
  return "Choose a process";
}

function money(minor: number | null | undefined): string {
  if (minor == null) return "—";
  return (minor / 100).toLocaleString("en-US", { style: "currency", currency: "USD" });
}

function dollarsInput(minor: number | null | undefined): string {
  if (minor == null) return "";
  return (minor / 100).toFixed(2);
}

function parseDollars(text: string): number | null {
  const trimmed = text.trim();
  if (!trimmed) return null;
  const value = Number(trimmed.replace(/[$,]/g, ""));
  if (!Number.isFinite(value)) return null;
  return Math.round(value * 100);
}

function rateText(ppm: number | null | undefined): string {
  if (ppm == null) return "";
  const percent = ppm / 10000;
  return String(Math.round(percent * 1000) / 1000);
}

function parseRate(text: string): number | null {
  const trimmed = text.trim().replace(/%/g, "");
  if (!trimmed) return null;
  const value = Number(trimmed);
  if (!Number.isFinite(value) || value < 0) return null;
  return Math.round(value * 10000);
}

function frequencyLabel(value: string | null | undefined): string {
  const found = FREQUENCIES.find(([key]) => key === value);
  return found ? found[1] : "—";
}

function rateLabel(account: ManagedAccount): string {
  if (account.aprPpm != null) return `${rateText(account.aprPpm)}%`;
  if (!account.chargesInterest) return "0%";
  return "—";
}

function shortDate(iso: string | null | undefined): string {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec((iso ?? "").slice(0, 10));
  if (!match) return iso || "—";
  return `${Number(match[2])}/${Number(match[3])}/${match[1].slice(2)}`;
}

const VENDOR_YEAR = new Date().getFullYear();

const VENDOR_SLICES = [
  { key: "principal", label: "Principal Paid", color: "#22d3ee" },
  { key: "interest", label: "Interest Amount", color: "#1e3a5f" },
  { key: "escrow", label: "Escrow Amount", color: "#f97316" },
  { key: "balance", label: "Principal Balance", color: "#a78bfa" },
] as const;

function VendorRing({ vendor }: { vendor: LoanVendor }) {
  const values = [
    vendor.principalPaidMinor,
    vendor.interestPaidMinor,
    vendor.escrowPaidMinor,
    vendor.principalBalanceMinor,
  ];
  const total = values.reduce((sum, value) => sum + Math.max(0, value), 0);
  const radius = 34;
  const circumference = 2 * Math.PI * radius;
  let cursor = 0;
  return (
    <svg className="loan-vendor-ring" viewBox="0 0 100 100" aria-hidden="true">
      <circle cx="50" cy="50" r={radius} fill="none" stroke="#eef2f6" strokeWidth="14" />
      {values.map((value, index) => {
        if (total <= 0 || value <= 0) return null;
        const length = (value / total) * circumference;
        const segment = (
          <circle
            key={VENDOR_SLICES[index].key}
            cx="50"
            cy="50"
            r={radius}
            fill="none"
            stroke={VENDOR_SLICES[index].color}
            strokeWidth="14"
            strokeDasharray={`${length} ${circumference - length}`}
            strokeDashoffset={-cursor}
            transform="rotate(-90 50 50)"
          />
        );
        cursor += length;
        return segment;
      })}
    </svg>
  );
}

function yearView(vendor: LoanVendor): { ring: LoanVendor; lines: LoanVendorLine[] } {
  const lines = vendor.lines.filter((line) => line.occurredOn.startsWith(`${VENDOR_YEAR}-`));
  const paid = lines.filter((line) => line.amountMinor > 0);
  const sum = (rows: LoanVendorLine[], pick: (line: LoanVendorLine) => number) =>
    rows.reduce((total, line) => total + pick(line), 0);
  return {
    lines,
    ring: {
      ...vendor,
      principalPaidMinor: sum(paid, (line) => line.principalMinor),
      interestPaidMinor: sum(paid, (line) => line.interestMinor),
      escrowPaidMinor: sum(paid, (line) => line.escrowMinor),
    },
  };
}

function factRows(vendor: LoanVendor, remaining: number | null): { label: string; value: string }[] {
  const parsed = (vendor.note ?? "")
    .split("\n")
    .map((line) => line.split("|"))
    .filter((parts) => parts.length === 2 && parts[0].trim() && parts[1].trim())
    .map(([label, value]) => ({ label: label.trim(), value: value.trim() }));
  if (parsed.length > 0) {
    if (remaining != null) parsed.push({ label: "Remaining", value: remaining.toLocaleString("en-US") });
    return parsed;
  }
  const left = remaining ?? vendor.termPayments - vendor.paymentsMade;
  return [
    { label: "Payments made", value: vendor.paymentsMade.toLocaleString("en-US") },
    { label: "Term", value: vendor.termPayments.toLocaleString("en-US") },
    { label: "Remaining", value: left.toLocaleString("en-US") },
  ];
}

function addMonthsIso(iso: string, months: number): string {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(iso);
  if (!match) return iso;
  const day = Number(match[3]);
  const base = new Date(Number(match[1]), Number(match[2]) - 1 + months, 1);
  const last = new Date(base.getFullYear(), base.getMonth() + 1, 0).getDate();
  const dd = Math.min(day, last);
  return `${base.getFullYear()}-${String(base.getMonth() + 1).padStart(2, "0")}-${String(dd).padStart(2, "0")}`;
}

type AmortRow = {
  due: string;
  payment: number;
  principal: number;
  interest: number;
  balance: number;
};

function amortize(account: ManagedAccount): AmortRow[] {
  if (!account.chargesInterest || periodsOf(account.frequency) !== 12) return [];
  const payment = account.paymentMinor;
  let balance = account.currentMinor;
  const apr = account.aprPpm;
  const dueOn = account.dueOn?.slice(0, 10);
  if (
    balance == null ||
    payment == null ||
    apr == null ||
    !dueOn ||
    balance <= 0 ||
    payment <= 0 ||
    apr <= 0
  ) {
    return [];
  }
  const rows: AmortRow[] = [];
  let due = dueOn;
  for (let i = 0; i < 480 && balance > 0; i += 1) {
    const interest = Math.round((balance * apr) / (1_000_000 * 12));
    let principal = payment - interest;
    if (principal <= 0) break;
    if (principal > balance) principal = balance;
    balance -= principal;
    rows.push({ due, payment: principal + interest, principal, interest, balance });
    due = addMonthsIso(due, 1);
  }
  return rows;
}

function dueDay(iso: string | null | undefined): string {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec((iso ?? "").slice(0, 10));
  return match ? String(Number(match[3])) : "";
}

function ordinal(day: number): string {
  const teen = day % 100;
  if (teen >= 11 && teen <= 13) return `${day}th`;
  const last = day % 10;
  if (last === 1) return `${day}st`;
  if (last === 2) return `${day}nd`;
  if (last === 3) return `${day}rd`;
  return `${day}th`;
}

function dueDayLabel(iso: string | null | undefined): string {
  const day = Number(dueDay(iso));
  if (!day) return "—";
  return ordinal(day);
}

function nextDueOn(day: number, from = new Date()): string | null {
  if (day < 1 || day > 31) return null;
  const clamp = (year: number, month: number) => {
    const last = new Date(year, month + 1, 0).getDate();
    return Math.min(day, last);
  };
  let year = from.getFullYear();
  let month = from.getMonth();
  let date = clamp(year, month);
  if (date < from.getDate()) {
    month += 1;
    if (month > 11) {
      month = 0;
      year += 1;
    }
    date = clamp(year, month);
  }
  const mm = String(month + 1).padStart(2, "0");
  const dd = String(date).padStart(2, "0");
  return `${year}-${mm}-${dd}`;
}

function periodsOf(frequency: string | null | undefined): number | null {
  if (frequency === "weekly") return 52;
  if (frequency === "monthly") return 12;
  if (frequency === "quarterly") return 4;
  if (frequency === "annual") return 1;
  return null;
}

function paymentsRemaining(account: ManagedAccount): number | null {
  const current = account.currentMinor;
  const payment = account.paymentMinor;
  if (current == null || current <= 0 || payment == null || payment <= 0) return null;
  if (!account.chargesInterest) return Math.ceil(current / payment);
  if (account.aprPpm == null || account.aprPpm <= 0) return null;
  const periods = periodsOf(account.frequency);
  if (periods == null) return null;
  const rate = account.aprPpm / 1_000_000 / periods;
  if (payment <= rate * current) return null;
  const count = Math.log(payment / (payment - rate * current)) / Math.log(1 + rate);
  if (!Number.isFinite(count) || count <= 0) return null;
  return Math.ceil(count);
}

function projected(account: ManagedAccount): { principal: number; interest: number } | null {
  const balance = account.currentMinor;
  const payment = account.paymentMinor;
  if (balance == null || balance <= 0 || payment == null || payment <= 0) return null;
  if (!account.chargesInterest) return { principal: Math.min(payment, balance), interest: 0 };
  const periods = periodsOf(account.frequency);
  if (account.aprPpm == null || periods == null) return null;
  const interest = Math.round((balance * account.aprPpm) / (1_000_000 * periods));
  const principal = payment - interest;
  if (principal <= 0) return null;
  return { principal, interest };
}

function lineReady(account: ManagedAccount, line: RegisterLine): boolean {
  if (account.kind === "credit") return Boolean(line.completed) || Boolean(line.stepTransfer);
  return (
    Boolean(line.completed) ||
    (Boolean(line.stepTransfer) &&
      Boolean(line.stepBillpay) &&
      Boolean(line.stepBillpayDeposit) &&
      Boolean(line.stepPay) &&
      Boolean(line.stepWithdrawal))
  );
}

function draftFrom(account: ManagedAccount): SetupDraft {
  return {
    name: account.name,
    process: account.kind === "credit" ? "register" : account.payProcess ?? "",
    elementId: account.linkedElementId ?? "",
    starting: dollarsInput(account.startingMinor),
    current: dollarsInput(account.currentMinor),
    payment: dollarsInput(account.paymentMinor),
    rate: account.aprPpm != null ? rateText(account.aprPpm) : account.chargesInterest ? "" : "0",
    frequency: account.frequency ?? "",
    due: dueDay(account.dueOn),
  };
}

function sameDraft(left: SetupDraft, right: SetupDraft): boolean {
  return (
    left.name === right.name &&
    left.process === right.process &&
    left.elementId === right.elementId &&
    left.starting === right.starting &&
    left.current === right.current &&
    left.payment === right.payment &&
    left.rate === right.rate &&
    left.frequency === right.frequency &&
    left.due === right.due
  );
}

async function readBody(result: { ok: boolean; errorCode?: string; bodyJson?: string }) {
  if (!result.ok || !result.bodyJson) {
    throw new Error(result.errorCode || "Debt planner request failed");
  }
  return JSON.parse(result.bodyJson) as ManagerBody;
}

export function ExternalAccountManager({
  resetToken,
  onDirtyChange,
}: {
  resetToken: number;
  onDirtyChange: (dirty: boolean) => void;
}) {
  const [body, setBody] = useState<ManagerBody | null>(null);
  const [elements, setElements] = useState<ElementChoice[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);
  const [newId, setNewId] = useState<string | null>(null);
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState<SetupDraft | null>(null);
  const [status, setStatus] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const accounts = body?.accounts ?? [];
  const selected = accounts.find((account) => account.accountId === selectedId) ?? null;
  const dirty =
    draft != null &&
    editing &&
    (creating ? !sameDraft(draft, blankDraft()) : selected != null && !sameDraft(draft, draftFrom(selected)));

  useEffect(() => {
    onDirtyChange(dirty);
  }, [dirty, onDirtyChange]);

  const closeSetup = () => {
    setEditing(false);
    setCreating(false);
    setNewId(null);
    setDraft(null);
  };

  useEffect(() => {
    if (!editing) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") closeSetup();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [editing]);

  const applyBody = (next: ManagerBody) => {
    setBody(next);
    setSelectedId((current) =>
      current && next.accounts.some((account) => account.accountId === current) ? current : null,
    );
  };

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const [managed, elementList] = await Promise.all([
          client.executeQuery("ExternalAccountManagerGet"),
          client.executeQuery("CashElementListGet", { account: "all" }),
        ]);
        const next = await readBody(managed);
        if (!cancelled) {
          applyBody(next);
          if (elementList.ok && elementList.bodyJson) {
            const parsed = JSON.parse(elementList.bodyJson) as { items?: ElementChoice[] };
            setElements(parsed.items ?? []);
          }
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

  const totals = useMemo(() => {
    let starting = 0;
    let startingAny = false;
    let current = 0;
    let currentAny = false;
    let payment = 0;
    for (const account of accounts) {
      if (account.startingMinor != null) {
        starting += account.startingMinor;
        startingAny = true;
      }
      if (account.currentMinor != null) {
        current += account.currentMinor;
        currentAny = true;
      }
      if (account.kind !== "credit" && account.paymentMinor != null) payment += account.paymentMinor;
    }
    return { starting, startingAny, current, currentAny, payment };
  }, [accounts]);

  const openSetup = (account: ManagedAccount) => {
    setCreating(false);
    setNewId(null);
    setSelectedId(account.accountId);
    setDraft(draftFrom(account));
    setEditing(true);
    setStatus(null);
  };

  const openNew = () => {
    setSelectedId(null);
    setCreating(true);
    setNewId(crypto.randomUUID());
    setDraft(blankDraft());
    setEditing(true);
    setStatus(null);
  };

  const save = async () => {
    if (!draft || (!creating && !selected)) return;
    const name = draft.name.trim();
    if (!name) {
      setStatus("Enter a name for this loan.");
      return;
    }
    const process = selected?.kind === "credit" && !creating ? "register" : draft.process;
    if (!process) {
      setStatus("Choose how this loan is paid.");
      return;
    }
    const credit = selected?.kind === "credit" && !creating;
    const startingMinor = parseDollars(draft.starting);
    const currentMinor = parseDollars(draft.current);
    const paymentMinor = credit ? null : parseDollars(draft.payment);
    if (draft.starting.trim() && startingMinor == null) {
      setStatus("Enter the starting balance as dollars, for example 300000.00.");
      return;
    }
    if (draft.current.trim() && currentMinor == null) {
      setStatus("Enter the current balance as dollars, for example 298489.00.");
      return;
    }
    if (!credit && draft.payment.trim() && paymentMinor == null) {
      setStatus("Enter the payment as dollars, for example 1796.72.");
      return;
    }
    if (draft.rate.trim() && parseRate(draft.rate) == null) {
      setStatus("Enter the interest rate as a percent, for example 5.99.");
      return;
    }
    const dueDayNumber = Number(draft.due);
    const dueOn = draft.due.trim() ? nextDueOn(dueDayNumber) : null;
    if (draft.due.trim() && dueOn == null) {
      setStatus("Enter the due day as a day of the month, for example 10.");
      return;
    }
    const accountId = creating ? newId : selected?.accountId;
    if (!accountId) return;
    setBusy(true);
    try {
      const result = await client.executeCommand("ExternalAccountManagerSave", {
        accounts: [
          {
            accountId,
            name,
            startingMinor,
            currentMinor,
            paymentMinor,
            reductionMinor: selected?.reductionMinor ?? null,
            financeMinor: selected?.financeMinor ?? null,
            dueOn,
            aprPpm: parseRate(draft.rate),
            frequency: draft.frequency || null,
            registerKey: credit ? selected?.registerKey ?? "Mom" : name,
            payProcess: process,
            linkedElementId: process === "element" && draft.elementId ? draft.elementId : null,
          },
        ],
      });
      const next = await readBody(result);
      applyBody(next);
      setSelectedId(accountId);
      setCreating(false);
      setNewId(null);
      setEditing(false);
      setDraft(null);
      setStatus("Saved.");
    } catch (err: unknown) {
      const code = err instanceof Error ? err.message : String(err);
      const known: Record<string, string> = {
        missing_process: "Choose how this loan is paid.",
        unknown_element: "That element is not on the element list.",
        duplicate_name: "A loan with that name is already on the list.",
        bad_process: "Choose Week Ahead, Checking and Credit, or a linked element.",
      };
      setStatus(known[code] ?? code);
    } finally {
      setBusy(false);
    }
  };

  return (
    <section className="managed-accounts" aria-label="Debt planner">
      <div className="managed-toolbar">
        <p>
          Choose the process in Loan setup. Week Ahead confirms the bank loan. Checking and Credit
          uses a register line. A linked element keeps its existing Week Ahead row and reduces this
          balance when that element is confirmed.
        </p>
        <div className="buttons">
          <button type="button" aria-label="Add loan" disabled={busy} onClick={openNew}>
            Add loan
          </button>
          <button
            type="button"
            aria-label="Edit selected loan"
            disabled={!selected || busy}
            onClick={() => selected && openSetup(selected)}
          >
            Edit loan
          </button>
        </div>
      </div>
      {status ? <p role="status">{status}</p> : null}
      <div className="table-wrap managed-wrap">
        <table aria-label="Debt planner">
          <thead>
            <tr>
              <th>Account</th>
              <th>Starting balance</th>
              <th>Current balance</th>
              <th>Interest rate</th>
              <th>Frequency</th>
              <th>Due day</th>
              <th>Payment</th>
              <th>Payments remaining</th>
              <th>Paid through</th>
            </tr>
          </thead>
          <tbody>
            {accounts.map((account) => {
              const remaining = paymentsRemaining(account);
              const credit = account.kind === "credit";
              return (
                <tr
                  key={account.accountId}
                  className={selectedId === account.accountId ? "is-selected" : undefined}
                  aria-selected={selectedId === account.accountId}
                  onClick={() => {
                    if (editing) return;
                    setSelectedId(account.accountId);
                  }}
                >
                  <td>
                    {account.name}
                    <div className="managed-key">
                      {processLabel(account, elements)}
                      {credit ? ` · ${account.registerKey}` : ""}
                    </div>
                  </td>
                  <td>{money(account.startingMinor)}</td>
                  <td className="managed-current">{money(account.currentMinor)}</td>
                  <td>{credit ? "—" : rateLabel(account)}</td>
                  <td>{credit ? "—" : frequencyLabel(account.frequency)}</td>
                  <td>{dueDayLabel(account.dueOn)}</td>
                  <td>{credit ? "—" : money(account.paymentMinor)}</td>
                  <td>{credit || remaining == null ? "—" : remaining.toLocaleString("en-US")}</td>
                  <td>{account.paidThrough ?? "—"}</td>
                </tr>
              );
            })}
          </tbody>
          <tfoot>
            <tr className="managed-annual">
              <th scope="row">Total</th>
              <td>{totals.startingAny ? money(totals.starting) : "—"}</td>
              <td className="managed-current">{totals.currentAny ? money(totals.current) : "—"}</td>
              <td>—</td>
              <td>—</td>
              <td>—</td>
              <td>
                <div>{money(totals.payment)}</div>
                <div className="managed-year">Year {money(totals.payment * 12)}</div>
              </td>
              <td>—</td>
              <td>—</td>
            </tr>
          </tfoot>
        </table>
      </div>
      {editing && draft && (creating || selected) ? (
        <div className="home-av-dialog-backdrop" onClick={closeSetup}>
          <form
            role="dialog"
            aria-modal="true"
            aria-label="Loan setup"
            className="home-av-dialog managed-setup"
            onClick={(event) => event.stopPropagation()}
            onSubmit={(event) => {
              event.preventDefault();
              void save();
            }}
          >
            <h3>Loan setup · {draft.name.trim() || "New loan"}</h3>
            <label>
              Name
              <input
                aria-label="Loan name"
                value={draft.name}
                onChange={(event) => setDraft({ ...draft, name: event.target.value })}
              />
            </label>
            <label>
              Process
              <select
                aria-label="Loan process"
                value={draft.process}
                disabled={selected?.kind === "credit" && !creating}
                onChange={(event) => {
                  const process = event.target.value;
                  setDraft({
                    ...draft,
                    process,
                    elementId: process === "element" ? draft.elementId : "",
                  });
                }}
              >
                {draft.process === "" ? <option value="">Choose how this loan is paid</option> : null}
                {PROCESSES.map(([key, label]) => (
                  <option key={key} value={key}>
                    {label}
                  </option>
                ))}
              </select>
            </label>
            {draft.process === "element" ? (
              <label>
                Element
                <select
                  aria-label="Linked element"
                  value={draft.elementId}
                  onChange={(event) => setDraft({ ...draft, elementId: event.target.value })}
                >
                  <option value="">Choose an element</option>
                  {elements.map((element) => (
                    <option key={element.elementId} value={element.elementId}>
                      {(element.note || element.account) + " · " + element.account + " · " + money(element.amountMinor)}
                    </option>
                  ))}
                </select>
              </label>
            ) : null}
            <label>
              Starting balance
              <input
                aria-label="Starting balance"
                inputMode="decimal"
                value={draft.starting}
                onChange={(event) => setDraft({ ...draft, starting: event.target.value })}
              />
            </label>
            <label>
              Current balance
              <input
                aria-label="Current balance"
                inputMode="decimal"
                value={draft.current}
                onChange={(event) => setDraft({ ...draft, current: event.target.value })}
              />
            </label>
            <label>
              Interest rate
              <input
                aria-label="Interest rate"
                inputMode="decimal"
                value={draft.rate}
                placeholder="5.99"
                onChange={(event) => setDraft({ ...draft, rate: event.target.value })}
              />
            </label>
            <label>
              Frequency
              <select
                aria-label="Frequency"
                value={draft.frequency}
                onChange={(event) => setDraft({ ...draft, frequency: event.target.value })}
              >
                <option value="">—</option>
                {FREQUENCIES.map(([key, label]) => (
                  <option key={key} value={key}>
                    {label}
                  </option>
                ))}
              </select>
            </label>
            {draft.process === "register" ? null : (
              <label>
                Due day
                <input
                  aria-label="Due day"
                  inputMode="numeric"
                  value={draft.due}
                  placeholder="10"
                  onChange={(event) => setDraft({ ...draft, due: event.target.value })}
                />
              </label>
            )}
            {selected?.kind === "credit" && !creating ? null : (
              <label>
                Payment
                <input
                  aria-label="Payment"
                  inputMode="decimal"
                  value={draft.payment}
                  onChange={(event) => setDraft({ ...draft, payment: event.target.value })}
                />
              </label>
            )}
            <div className="buttons">
              <button type="submit" className={dirty ? "is-unsaved" : undefined} disabled={!dirty || busy}>
                Save
              </button>
              <button type="button" disabled={busy} onClick={closeSetup}>
                Cancel
              </button>
            </div>
          </form>
        </div>
      ) : (
        <p>Select a loan, then Edit loan, or add a loan and choose how it is paid.</p>
      )}
      {selected ? (
        <div className="loan-detail">
          <section className="loan-vendor" aria-label="Loan Vendor Data">
            <h3>Loan Vendor Data · {VENDOR_YEAR}</h3>
            {selected.vendor ? (
              <>
                <div className="loan-vendor-summary">
                  <VendorRing vendor={yearView(selected.vendor).ring} />
                  <dl className="loan-vendor-legend">
                    {VENDOR_SLICES.map((slice) => {
                      const view = yearView(selected.vendor!).ring;
                      const amount =
                        slice.key === "principal"
                          ? view.principalPaidMinor
                          : slice.key === "interest"
                            ? view.interestPaidMinor
                            : slice.key === "escrow"
                              ? view.escrowPaidMinor
                              : view.principalBalanceMinor;
                      return (
                        <div key={slice.key}>
                          <dt>
                            <span className="loan-vendor-swatch" style={{ background: slice.color }} />
                            {slice.label}
                          </dt>
                          <dd>{money(amount)}</dd>
                        </div>
                      );
                    })}
                  </dl>
                  <table className="loan-vendor-facts" aria-label="Loan facts">
                    <tbody>
                      {factRows(selected.vendor, amortize(selected).length || null).map((row) => (
                        <tr key={row.label}>
                          <th scope="row">{row.label}</th>
                          <td>{row.value}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
                <div className="loan-vendor-tables">
                <div className="table-wrap loan-vendor-lines">
                  <table aria-label="Loan vendor history">
                    <thead>
                      <tr>
                        <th>Date</th>
                        <th className="num">Amount</th>
                        <th>Due</th>
                        <th>Description</th>
                        <th className="num">Principal</th>
                        <th className="num">Interest</th>
                        <th className="num">Escrow</th>
                        <th className="num">Late</th>
                        <th className="num">Balance</th>
                        <th className="num">Escrow bal.</th>
                      </tr>
                    </thead>
                    <tbody>
                      {yearView(selected.vendor).lines.length === 0 ? (
                        <tr>
                          <td colSpan={10}>No {VENDOR_YEAR} vendor payments yet.</td>
                        </tr>
                      ) : (
                        yearView(selected.vendor).lines.map((line) => (
                          <tr key={line.lineId}>
                            <td>{shortDate(line.occurredOn)}</td>
                            <td className="num">{money(line.amountMinor)}</td>
                            <td>{shortDate(line.dueOn)}</td>
                            <td>{line.description}</td>
                            <td className="num">{money(line.principalMinor)}</td>
                            <td className="num">{money(line.interestMinor)}</td>
                            <td className="num">{money(line.escrowMinor)}</td>
                            <td className="num">{money(line.lateMinor)}</td>
                            <td className="num">{money(line.principalBalanceMinor)}</td>
                            <td className="num">{money(line.escrowBalanceMinor)}</td>
                          </tr>
                        ))
                      )}
                    </tbody>
                  </table>
                </div>
                {amortize(selected).length > 0 ? (
                  <div className="table-wrap loan-vendor-lines loan-amort">
                    <table aria-label="Remaining payments">
                      <caption>Remaining payments</caption>
                      <thead>
                        <tr>
                          <th>Due</th>
                          <th className="num">Payment</th>
                          <th className="num">Principal</th>
                          <th className="num">Interest</th>
                          <th className="num">Balance</th>
                        </tr>
                      </thead>
                      <tbody>
                        {amortize(selected).map((row) => (
                          <tr key={row.due}>
                            <td>{shortDate(row.due)}</td>
                            <td className="num">{money(row.payment)}</td>
                            <td className="num">{money(row.principal)}</td>
                            <td className="num">{money(row.interest)}</td>
                            <td className="num">{money(row.balance)}</td>
                          </tr>
                        ))}
                      </tbody>
                      <tfoot>
                        <tr>
                          <th scope="row">Interest</th>
                          <td />
                          <td />
                          <td className="num">
                            {money(amortize(selected).reduce((sum, row) => sum + row.interest, 0))}
                          </td>
                          <td />
                        </tr>
                      </tfoot>
                    </table>
                  </div>
                ) : null}
                </div>
              </>
            ) : (
              <p className="loan-vendor-meta">No loan vendor history yet.</p>
            )}
          </section>
          <div className="managed-register">
            <h3>
              Our transactions
              {projected(selected)
                ? ` · next split ${money(projected(selected)?.principal)} principal, ${money(projected(selected)?.interest)} interest`
                : ""}
            </h3>
            <div className="table-wrap managed-lines">
              <table>
                <thead>
                  <tr>
                    <th>Date</th>
                    <th>Pay type</th>
                    <th className="num">Total spent</th>
                    <th>Category</th>
                    <th>Vendor</th>
                    <th>Description</th>
                    <th>Status</th>
                  </tr>
                </thead>
                <tbody>
                  {selected.lines.length === 0 ? (
                    <tr>
                      <td colSpan={7}>No matching charges in the external register yet.</td>
                    </tr>
                  ) : (
                    selected.lines.map((line) => (
                      <tr key={line.lineId}>
                        <td>{line.occurredOn ?? "—"}</td>
                        <td>{line.payType}</td>
                        <td className="num">{money(line.amountMinor)}</td>
                        <td>{line.category}</td>
                        <td>{line.vendor}</td>
                        <td>{line.description}</td>
                        <td>{lineReady(selected, line) ? "Paid" : "Pending"}</td>
                      </tr>
                    ))
                  )}
                </tbody>
              </table>
            </div>
          </div>
        </div>
      ) : null}
    </section>
  );
}
