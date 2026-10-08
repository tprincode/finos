import { useEffect, useMemo, useRef, useState } from "react";
import type { CashElementRecord } from "@finos/app-contracts";
import { LocalTauriFinanceClient } from "../../financeClient";
import { CashElementExceptions } from "./CashElementExceptions";
import type { EditorOccurrence } from "./CashElementEditor";

const client = new LocalTauriFinanceClient();

const FREQUENCIES = [
  ["weekly", "Weekly"],
  ["monthly", "Monthly"],
  ["quarterly", "Quarterly"],
  ["annual", "Annual"],
] as const;

const NEW_LOAN_ELEMENT = "__new_loan_element__";

const ELEMENT_CADENCES = [
  ["weekly", "Weekly"],
  ["monthly", "Monthly"],
  ["annual", "Annual"],
  ["one-time", "One-time"],
] as const;

function todayIso(): string {
  const now = new Date();
  const month = String(now.getMonth() + 1).padStart(2, "0");
  const day = String(now.getDate()).padStart(2, "0");
  return `${now.getFullYear()}-${month}-${day}`;
}

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
  accountName?: string;
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
  inactive?: boolean;
  lines: RegisterLine[];
  vendor?: LoanVendor | null;
};

type ManagerBody = {
  accounts: ManagedAccount[];
};

type BudgetBucket = {
  bucketId: string;
  name: string;
  bank: string;
  description: string;
  budgetCategory: string;
};

type BucketDraft = {
  bucketId: string | null;
  name: string;
  bank: string;
  description: string;
  budgetCategory: string;
};

type ElementChoice = {
  elementId: string;
  account: string;
  kind: string;
  note: string;
  amountMinor: number;
  cadence?: string;
  weekdayOrMonthDay?: string;
  startOn?: string;
  stopOn?: string;
  exceptionCount?: number;
  exceptions?: EditorOccurrence[];
  upcoming?: EditorOccurrence[];
};

/** Debts: interest loans use Week Ahead verify; zero-interest may use Scheduled element → CCT. */
const DEBT_PROCESSES = [
  [
    "week_ahead",
    "Week Ahead",
    "Interest projection and verify on Week Ahead; Confirm reduces Current by principal only.",
  ],
  [
    "element",
    "Scheduled element",
    "Week Ahead confirm drafts a CCT Open row; balance moves when CCT settles (full amount).",
  ],
] as const;

/** Escrow Mom only. */
const ESCROW_PROCESS = [
  "register",
  "Bucket Transaction Managed",
  "As-needed CCT; Bucket Mom moves the balance.",
] as const;

const BANK_OPTIONS = [
  "UCARD",
  "CAP",
  "Capital One",
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
] as const;

const ADD_BANK_VALUE = "__add_bank__";

function mergeBankOptions(
  seed: readonly string[],
  fromBuckets: string[],
  extras: string[],
): string[] {
  const seen = new Set<string>();
  const out: string[] = [];
  for (const name of [...seed, ...fromBuckets, ...extras]) {
    const trimmed = name.trim();
    if (!trimmed) continue;
    const key = trimmed.toLowerCase();
    if (seen.has(key)) continue;
    seen.add(key);
    out.push(trimmed);
  }
  return out;
}

function BucketBankField({
  value,
  options,
  onChange,
  onAdded,
}: {
  value: string;
  options: string[];
  onChange: (bank: string) => void;
  onAdded: (bank: string) => void;
}) {
  const [adding, setAdding] = useState(false);
  const [draftName, setDraftName] = useState("");
  const priorRef = useRef(value);

  const commitAdd = () => {
    const name = draftName.trim();
    if (name) {
      onAdded(name);
      onChange(name);
    } else {
      onChange(priorRef.current);
    }
    setAdding(false);
    setDraftName("");
  };

  if (adding) {
    return (
      <input
        aria-label="New bank name"
        value={draftName}
        autoFocus
        onChange={(event) => setDraftName(event.target.value)}
        onBlur={commitAdd}
        onKeyDown={(event) => {
          if (event.key === "Enter") {
            event.preventDefault();
            commitAdd();
          }
          if (event.key === "Escape") {
            event.preventDefault();
            event.stopPropagation();
            setAdding(false);
            setDraftName("");
            onChange(priorRef.current);
          }
        }}
      />
    );
  }

  const known = options.some((name) => name === value);
  return (
    <select
      className="debt-bucket-bank"
      aria-label="Bucket bank"
      value={value}
      onChange={(event) => {
        if (event.target.value === ADD_BANK_VALUE) {
          priorRef.current = value;
          setAdding(true);
          setDraftName("");
          return;
        }
        onChange(event.target.value);
      }}
    >
      <option value="">(none)</option>
      {options.map((name) => (
        <option key={name} value={name}>
          {name}
        </option>
      ))}
      {value && !known ? <option value={value}>{value}</option> : null}
      <option value={ADD_BANK_VALUE}>Add bank…</option>
    </select>
  );
}

const BUDGET_CATEGORY_OPTIONS = [
  "Food",
  "Cash",
  "Bills",
  "Pets",
  "Medical",
  "HSA",
  "Cash acct",
  "Bill acct",
  "Home",
  "House",
  "Work",
  "Gas",
  "Auto",
  "Insurance",
  "Other",
] as const;

function blankBucketDraft(): BucketDraft {
  return {
    bucketId: null,
    name: "",
    bank: "",
    description: "",
    budgetCategory: "",
  };
}

function draftFromBucket(bucket: BudgetBucket): BucketDraft {
  return {
    bucketId: bucket.bucketId,
    name: bucket.name,
    bank: bucket.bank,
    description: bucket.description,
    budgetCategory: bucket.budgetCategory,
  };
}

function sameBucketDraft(left: BucketDraft, right: BucketDraft): boolean {
  return (
    left.bucketId === right.bucketId &&
    left.name === right.name &&
    left.bank === right.bank &&
    left.description === right.description &&
    left.budgetCategory === right.budgetCategory
  );
}

type SetupDraft = {
  accountName: string;
  name: string;
  process: string;
  elementId: string;
  elementNew: boolean;
  elementName: string;
  elementAmount: string;
  elementCadence: string;
  elementDay: string;
  elementStartOn: string;
  elementStopOn: string;
  starting: string;
  current: string;
  payment: string;
  rate: string;
  frequency: string;
  due: string;
  inactive: boolean;
};

function blankElementFields(loanName = ""): Pick<
  SetupDraft,
  | "elementId"
  | "elementNew"
  | "elementName"
  | "elementAmount"
  | "elementCadence"
  | "elementDay"
  | "elementStartOn"
  | "elementStopOn"
> {
  return {
    elementId: "",
    elementNew: false,
    elementName: loanName,
    elementAmount: "",
    elementCadence: "monthly",
    elementDay: "1",
    elementStartOn: "",
    elementStopOn: "",
  };
}

function elementFieldsFrom(
  element: ElementChoice | null | undefined,
  loanName: string,
): ReturnType<typeof blankElementFields> {
  if (!element) {
    return { ...blankElementFields(loanName), elementNew: true };
  }
  return {
    elementId: element.elementId,
    elementNew: false,
    elementName: (element.note || loanName).trim(),
    elementAmount: dollarsInput(element.amountMinor),
    elementCadence: element.cadence || "monthly",
    elementDay: element.weekdayOrMonthDay || "1",
    elementStartOn: element.startOn ?? "",
    elementStopOn: element.stopOn ?? "",
  };
}

function blankDraft(): SetupDraft {
  return {
    accountName: "",
    name: "",
    process: "week_ahead",
    ...blankElementFields(),
    starting: "",
    current: "",
    payment: "",
    rate: "",
    frequency: "monthly",
    due: "",
    inactive: false,
  };
}

function processLabel(account: ManagedAccount, elements: ElementChoice[]): string {
  // Bucket Transaction Managed is escrow (Mom shopping) only.
  if (account.kind === "credit" || account.payProcess === "register") {
    return account.kind === "credit" ? "Bucket Transaction Managed" : "Register";
  }
  if (account.payProcess === "element") {
    const linked = elements.find((element) => element.elementId === account.linkedElementId);
    const name = linked?.note || linked?.account;
    return name ? `Scheduled element · ${name}` : "Scheduled element · choose one";
  }
  if (account.payProcess === "week_ahead") return "Week Ahead";
  return "Scheduled element";
}

function isEscrowAccount(account: ManagedAccount): boolean {
  return account.kind === "credit";
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

function draftFrom(account: ManagedAccount, elements: ElementChoice[]): SetupDraft {
  const linked = elements.find((element) => element.elementId === account.linkedElementId);
  const loanName = account.name;
  const elementPart = account.linkedElementId
    ? elementFieldsFrom(linked, loanName)
    : blankElementFields(loanName);
  const weekAhead = account.payProcess === "week_ahead";
  return {
    accountName: (account.accountName ?? "").trim() || account.name,
    name: account.name,
    process: account.kind === "credit" ? "register" : account.payProcess || "week_ahead",
    ...elementPart,
    starting: dollarsInput(account.startingMinor),
    current: dollarsInput(account.currentMinor),
    payment: dollarsInput(account.paymentMinor),
    rate: account.aprPpm != null ? rateText(account.aprPpm) : account.chargesInterest ? "" : "0",
    // Week Ahead interest path owns due day / frequency on the loan row.
    frequency: weekAhead
      ? account.frequency || linked?.cadence || "monthly"
      : linked?.cadence || account.frequency || "",
    due: weekAhead
      ? dueDay(account.dueOn) || linked?.weekdayOrMonthDay || ""
      : linked?.weekdayOrMonthDay || dueDay(account.dueOn),
    inactive: Boolean(account.inactive),
  };
}

function sameDraft(left: SetupDraft, right: SetupDraft): boolean {
  return (
    left.accountName === right.accountName &&
    left.name === right.name &&
    left.process === right.process &&
    left.elementId === right.elementId &&
    left.elementNew === right.elementNew &&
    left.elementName === right.elementName &&
    left.elementAmount === right.elementAmount &&
    left.elementCadence === right.elementCadence &&
    left.elementDay === right.elementDay &&
    left.elementStartOn === right.elementStartOn &&
    left.elementStopOn === right.elementStopOn &&
    left.starting === right.starting &&
    left.current === right.current &&
    left.payment === right.payment &&
    left.rate === right.rate &&
    left.frequency === right.frequency &&
    left.due === right.due &&
    left.inactive === right.inactive
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
  const [buckets, setBuckets] = useState<BudgetBucket[]>([]);
  const [elements, setElements] = useState<ElementChoice[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [selectedBucketId, setSelectedBucketId] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);
  const [newId, setNewId] = useState<string | null>(null);
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState<SetupDraft | null>(null);
  const [bucketEditing, setBucketEditing] = useState(false);
  const [bucketCreating, setBucketCreating] = useState(false);
  const [bucketDraft, setBucketDraft] = useState<BucketDraft | null>(null);
  const [extraBanks, setExtraBanks] = useState<string[]>([]);
  const [status, setStatus] = useState<string | null>(null);
  const [loadState, setLoadState] = useState<"loading" | "ready" | "error">("loading");
  const [busy, setBusy] = useState(false);
  const [exceptionsOpen, setExceptionsOpen] = useState(false);

  const accounts = body?.accounts ?? [];
  const selected = accounts.find((account) => account.accountId === selectedId) ?? null;
  const selectedBucket = buckets.find((bucket) => bucket.bucketId === selectedBucketId) ?? null;
  const bankOptions = useMemo(
    () =>
      mergeBankOptions(
        BANK_OPTIONS,
        buckets.map((bucket) => bucket.bank),
        extraBanks,
      ),
    [buckets, extraBanks],
  );
  const rememberBank = (bank: string) => {
    const trimmed = bank.trim();
    if (!trimmed) return;
    setExtraBanks((current) => mergeBankOptions([], current, [trimmed]));
  };
  const dirty =
    draft != null &&
    editing &&
    (creating
      ? !sameDraft(draft, blankDraft())
      : selected != null && !sameDraft(draft, draftFrom(selected, elements)));
  const bucketDirty =
    bucketDraft != null &&
    bucketEditing &&
    (bucketCreating
      ? !sameBucketDraft(bucketDraft, blankBucketDraft())
      : selectedBucket != null && !sameBucketDraft(bucketDraft, draftFromBucket(selectedBucket)));

  useEffect(() => {
    onDirtyChange(dirty || bucketDirty);
  }, [dirty, bucketDirty, onDirtyChange]);

  useEffect(() => {
    if (!editing || creating || !selected?.linkedElementId) return;
    setDraft((current) => {
      if (!current || current.elementNew || current.elementName.trim()) return current;
      const found = elements.find((element) => element.elementId === selected.linkedElementId);
      if (!found) return current;
      return { ...current, ...elementFieldsFrom(found, current.name) };
    });
  }, [elements, editing, creating, selected?.linkedElementId, selected?.accountId]);

  const closeSetup = () => {
    setExceptionsOpen(false);
    setEditing(false);
    setCreating(false);
    setNewId(null);
    setDraft(null);
  };

  const closeBucketSetup = () => {
    setBucketEditing(false);
    setBucketCreating(false);
    setBucketDraft(null);
  };

  useEffect(() => {
    if (!editing && !bucketEditing) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        if (exceptionsOpen) setExceptionsOpen(false);
        else if (bucketEditing) closeBucketSetup();
        else closeSetup();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [editing, bucketEditing, exceptionsOpen]);

  const applyBody = (next: ManagerBody) => {
    setBody(next);
    setSelectedId((current) =>
      current && next.accounts.some((account) => account.accountId === current) ? current : null,
    );
  };

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      // Load independently so one failing query cannot blank Loans, Escrow, and buckets.
      if (!cancelled) setLoadState("loading");
      try {
        const managed = await client.executeQuery("ExternalAccountManagerGet");
        if (!managed.ok || !managed.bodyJson) {
          if (!cancelled) {
            setLoadState("error");
            setStatus(
              `Debt planner load failed: ${managed.errorCode || "ExternalAccountManagerGet"}. Restart after migrations (inactive / account columns).`,
            );
          }
        } else {
          const next = await readBody(managed);
          if (!cancelled) {
            applyBody(next);
            setLoadState("ready");
            const activeLoans = (next.accounts ?? []).filter(
              (account) => account.kind !== "credit" && account.inactive !== true,
            ).length;
            if ((next.accounts?.length ?? 0) === 0) {
              setStatus("Debt planner returned no loans. Check migrations / restart.");
            } else if (activeLoans === 0) {
              setStatus("No active loans (all inactive or escrow-only). Use Account Management to reactivate.");
            } else {
              setStatus(null);
            }
          }
        }
      } catch (err: unknown) {
        if (!cancelled) {
          setLoadState("error");
          setStatus(err instanceof Error ? err.message : String(err));
        }
      }
      try {
        const elementList = await client.executeQuery("CashElementListGet", {
          account: "all",
          asOfDate: todayIso(),
        });
        if (!cancelled && elementList.ok && elementList.bodyJson) {
          const parsed = JSON.parse(elementList.bodyJson) as { items?: ElementChoice[] };
          setElements(parsed.items ?? []);
        }
      } catch {
        /* keep prior elements */
      }
      try {
        const bucketList = await client.executeQuery("ExternalBucketListGet");
        if (!cancelled && bucketList.ok && bucketList.bodyJson) {
          const parsed = JSON.parse(bucketList.bodyJson) as { buckets?: BudgetBucket[] };
          setBuckets(parsed.buckets ?? []);
        }
      } catch {
        /* keep prior buckets */
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [resetToken]);

  const sumAccounts = (rows: ManagedAccount[]) => {
    let starting = 0;
    let startingAny = false;
    let current = 0;
    let currentAny = false;
    let payment = 0;
    for (const account of rows) {
      if (account.startingMinor != null) {
        starting += account.startingMinor;
        startingAny = true;
      }
      if (account.currentMinor != null) {
        current += account.currentMinor;
        currentAny = true;
      }
      if (account.kind !== "credit" && account.paymentMinor != null) {
        payment += account.paymentMinor;
      }
    }
    return { starting, startingAny, current, currentAny, payment };
  };

  const openSetup = (account: ManagedAccount) => {
    setCreating(false);
    setNewId(null);
    setSelectedId(account.accountId);
    setDraft(draftFrom(account, elements));
    setExceptionsOpen(false);
    setEditing(true);
    setStatus(null);
  };

  const openNew = () => {
    setSelectedId(null);
    setCreating(true);
    setNewId(crypto.randomUUID());
    setDraft({ ...blankDraft(), elementNew: true });
    setExceptionsOpen(false);
    setEditing(true);
    setStatus(null);
  };

  const refreshElements = async () => {
    const elementList = await client.executeQuery("CashElementListGet", {
      account: "all",
      asOfDate: todayIso(),
    });
    if (elementList.ok && elementList.bodyJson) {
      const parsed = JSON.parse(elementList.bodyJson) as { items?: ElementChoice[] };
      setElements(parsed.items ?? []);
      return parsed.items ?? [];
    }
    return elements;
  };

  const pickLoanElement = (value: string) => {
    if (!draft) return;
    if (value === NEW_LOAN_ELEMENT) {
      setDraft({
        ...draft,
        ...blankElementFields(draft.name.trim() || draft.elementName),
        elementNew: true,
        elementAmount: draft.payment || draft.elementAmount,
      });
      return;
    }
    if (!value) {
      setDraft({ ...draft, ...blankElementFields(draft.name.trim()) });
      return;
    }
    const found = elements.find((element) => element.elementId === value);
    setDraft({
      ...draft,
      ...elementFieldsFrom(found, draft.name.trim() || found?.note || ""),
    });
  };

  const save = async () => {
    if (!draft || (!creating && !selected)) return;
    const name = draft.name.trim();
    if (!name) {
      setStatus("Enter a loan name.");
      return;
    }
    const accountName = draft.accountName.trim() || name;
    const credit = selected?.kind === "credit" && !creating;
    const process = credit
      ? "register"
      : draft.process === "week_ahead" || draft.process === "element"
        ? draft.process
        : "week_ahead";
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
    let linkedElementId: string | null = null;
    let frequency: string | null = draft.frequency || null;
    let dueOn: string | null = draft.due.trim() ? nextDueOn(Number(draft.due)) : null;
    const needsElement = !credit && !draft.inactive && process === "element";
    if (needsElement) {
      const elementName = draft.elementName.trim();
      if (!elementName) {
        setStatus("Enter a Loan Element name.");
        return;
      }
      if (!draft.elementId && !draft.elementNew) {
        setStatus("Choose a Loan Element or create a new one.");
        return;
      }
      const elementAmountMinor = parseDollars(draft.elementAmount);
      if (elementAmountMinor == null || elementAmountMinor <= 0) {
        setStatus("Enter the Loan Element amount as dollars, for example 236.00.");
        return;
      }
      const scheduleDay = draft.elementDay.trim() || "1";
      const dueDayNumber = Number(scheduleDay);
      dueOn = Number.isFinite(dueDayNumber) ? nextDueOn(dueDayNumber) : null;
      if (scheduleDay && Number.isFinite(dueDayNumber) && dueOn == null) {
        setStatus("Enter the schedule date as a day of the month, for example 1.");
        return;
      }
      frequency = draft.elementCadence || "monthly";
    } else if (!credit && process === "week_ahead") {
      frequency = draft.frequency || draft.elementCadence || "monthly";
      const dueDayNumber = Number(draft.due.trim() || draft.elementDay.trim());
      if (Number.isFinite(dueDayNumber) && dueDayNumber > 0) {
        dueOn = nextDueOn(dueDayNumber);
      }
      if (!dueOn) {
        setStatus("Enter the due day of the month for Week Ahead, for example 7.");
        return;
      }
      // Keep an existing Loan Element link for history, but Week Ahead Loan payments drive paydown.
      linkedElementId = draft.elementId && !draft.elementNew ? draft.elementId : selected?.linkedElementId ?? null;
    } else if (!credit && draft.elementId) {
      linkedElementId = draft.elementId;
      frequency = draft.elementCadence || draft.frequency || null;
      const dueDayNumber = Number(draft.elementDay.trim() || draft.due);
      if (Number.isFinite(dueDayNumber)) dueOn = nextDueOn(dueDayNumber);
    }
    const accountId = creating ? newId : selected?.accountId;
    if (!accountId) return;
    setBusy(true);
    try {
      if (needsElement) {
        const elementAmountMinor = parseDollars(draft.elementAmount)!;
        const elementSave = await client.executeCommand("CashElementSave", {
          ...(draft.elementId && !draft.elementNew ? { elementId: draft.elementId } : {}),
          name: draft.elementName.trim(),
          account: "Loan",
          kind: "Withdrawal",
          cadence: draft.elementCadence || "monthly",
          weekdayOrMonthDay: draft.elementDay.trim() || "1",
          startOn: draft.elementStartOn.trim(),
          stopOn: draft.elementStopOn.trim(),
          amountMinor: elementAmountMinor,
          occurrences: [],
          asOfDate: todayIso(),
        });
        if (!elementSave.ok) {
          setStatus(`Loan Element save failed: ${elementSave.errorCode ?? "error"}`);
          return;
        }
        const saved = elementSave.bodyJson
          ? (JSON.parse(elementSave.bodyJson) as { element?: { elementId?: string } })
          : {};
        linkedElementId =
          draft.elementId && !draft.elementNew
            ? draft.elementId
            : saved.element?.elementId ?? null;
        if (!linkedElementId) {
          setStatus("Loan Element save did not return an id.");
          return;
        }
        await refreshElements();
      }
      const result = await client.executeCommand("ExternalAccountManagerSave", {
        accounts: [
          {
            accountId,
            name,
            accountName,
            startingMinor,
            currentMinor,
            paymentMinor,
            reductionMinor: selected?.reductionMinor ?? null,
            financeMinor: selected?.financeMinor ?? null,
            dueOn,
            aprPpm: parseRate(draft.rate),
            frequency,
            registerKey: credit ? selected?.registerKey ?? "Mom" : name,
            payProcess: process,
            linkedElementId: process === "element" ? linkedElementId : null,
            inactive: credit ? false : draft.inactive,
          },
        ],
      });
      const next = await readBody(result);
      applyBody(next);
      setSelectedId(draft.inactive && !credit ? null : accountId);
      setCreating(false);
      setNewId(null);
      setExceptionsOpen(false);
      setEditing(false);
      setDraft(null);
      setStatus(draft.inactive && !credit ? "Saved as inactive (hidden from loan list)." : "Saved.");
    } catch (err: unknown) {
      const code = err instanceof Error ? err.message : String(err);
      const known: Record<string, string> = {
        missing_process: "Choose how this loan is paid.",
        unknown_element: "That element is not on the element list.",
        duplicate_name: "A loan with that name is already on the list.",
        bad_process: "Choose Week Ahead, Bucket Transaction Managed (escrow), or a linked element.",
        unknown_amount: "Enter the Loan Element amount as dollars, for example 236.00.",
        cash_account_kind: "Loan Element must stay on the Loan book.",
      };
      setStatus(known[code] ?? code);
    } finally {
      setBusy(false);
    }
  };

  const saveBucket = async () => {
    if (!bucketDraft) return;
    const name = bucketDraft.name.trim();
    if (!name) {
      setStatus("Enter a bucket name.");
      return;
    }
    setBusy(true);
    try {
      const result = await client.executeCommand("ExternalBucketSave", {
        bucketId: bucketDraft.bucketId,
        name,
        bank: bucketDraft.bank.trim(),
        description: bucketDraft.description.trim(),
        budgetCategory: bucketDraft.budgetCategory.trim() || name,
      });
      if (!result.ok || !result.bodyJson) {
        throw new Error(result.errorCode || "Bucket save failed");
      }
      const parsed = JSON.parse(result.bodyJson) as { buckets?: BudgetBucket[] };
      const next = parsed.buckets ?? [];
      setBuckets(next);
      const saved =
        next.find((bucket) => bucket.name.toLowerCase() === name.toLowerCase()) ?? null;
      setSelectedBucketId(saved?.bucketId ?? null);
      closeBucketSetup();
      setStatus("Bucket saved.");
    } catch (err: unknown) {
      const code = err instanceof Error ? err.message : String(err);
      const known: Record<string, string> = {
        bad_bucket: "Enter a bucket name.",
        duplicate_bucket: "A bucket with that name already exists.",
      };
      setStatus(known[code] ?? code);
    } finally {
      setBusy(false);
    }
  };

  /** Active debts only — inactive loans stay in DB / Account Management. */
  const loanAccounts = accounts.filter(
    (account) => !isEscrowAccount(account) && account.inactive !== true,
  );
  const escrowAccounts = accounts.filter((account) => isEscrowAccount(account));
  const loanTotals = sumAccounts(loanAccounts);
  const escrowTotals = sumAccounts(escrowAccounts);

  const renderAccountRow = (account: ManagedAccount) => {
    const remaining = paymentsRemaining(account);
    const escrow = isEscrowAccount(account);
    const accountName = (account.accountName ?? "").trim() || account.name;
    const loanName = escrow ? "—" : account.name;
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
        <td className="managed-col-account" title={accountName}>
          {accountName}
        </td>
        <td className="managed-col-type" title={processLabel(account, elements)}>
          {processLabel(account, elements)}
        </td>
        <td className="managed-col-bucket" title={loanName === "—" ? undefined : loanName}>
          {loanName}
        </td>
        <td className="managed-col-money">{money(account.startingMinor)}</td>
        <td className="managed-col-money managed-current">{money(account.currentMinor)}</td>
        <td className="managed-col-rate">{escrow ? "—" : rateLabel(account)}</td>
        <td className="managed-col-freq">{escrow ? "—" : frequencyLabel(account.frequency)}</td>
        <td className="managed-col-due">{dueDayLabel(account.dueOn)}</td>
        <td className="managed-col-money">{escrow ? "—" : money(account.paymentMinor)}</td>
        <td className="managed-col-rem">
          {escrow || remaining == null ? "—" : remaining.toLocaleString("en-US")}
        </td>
        <td className="managed-col-paid">{account.paidThrough ?? "—"}</td>
      </tr>
    );
  };

  const renderSectionTotal = (
    label: string,
    aria: string,
    section: ReturnType<typeof sumAccounts>,
    includePayment: boolean,
  ) => (
    <tr className="managed-annual" aria-label={aria}>
      <th scope="row" colSpan={2}>
        {label}
      </th>
      <td className="managed-col-bucket">—</td>
      <td className="managed-col-money">{section.startingAny ? money(section.starting) : "—"}</td>
      <td className="managed-col-money managed-current">
        {section.currentAny ? money(section.current) : "—"}
      </td>
      <td className="managed-col-rate">—</td>
      <td className="managed-col-freq">—</td>
      <td className="managed-col-due">—</td>
      <td className="managed-col-money">
        {includePayment ? (
          <>
            <div>{money(section.payment)}</div>
            <div className="managed-year">Year {money(section.payment * 12)}</div>
          </>
        ) : (
          "—"
        )}
      </td>
      <td className="managed-col-rem">—</td>
      <td className="managed-col-paid">—</td>
    </tr>
  );

  return (
    <section
      className="managed-accounts"
      id="managed-accounts"
      data-section="debt-accounts-list"
      aria-label="Debt planner"
    >
      <div className="managed-toolbar">
        <div className="buttons">
          <button
            type="button"
            aria-label="Add loan"
            title="Add a loan. Choose how it is paid in Loan setup."
            disabled={busy || bucketEditing}
            onClick={openNew}
          >
            Add loan
          </button>
          <button
            type="button"
            aria-label="Edit selected loan"
            title="Edit the selected loan or escrow account."
            disabled={!selected || busy || bucketEditing}
            onClick={() => selected && openSetup(selected)}
          >
            Edit loan
          </button>
        </div>
      </div>
      {status ? <p role="status">{status}</p> : null}
      <div className="table-wrap managed-wrap" data-part="debt-accounts">
        <table aria-label="Debt planner" className="managed-debt-table">
          <thead>
            <tr>
              <th className="managed-col-account" title="Account Name">
                Account
                <br />
                Name
              </th>
              <th className="managed-col-type" title="Loan type">
                Loan
                <br />
                type
              </th>
              <th className="managed-col-bucket" title="Loan Name">
                Loan
                <br />
                Name
              </th>
              <th className="managed-col-money" title="Starting balance">
                Starting
                <br />
                balance
              </th>
              <th className="managed-col-money" title="Current balance">
                Current
                <br />
                balance
              </th>
              <th className="managed-col-rate" title="Interest rate">
                Interest
                <br />
                rate
              </th>
              <th className="managed-col-freq" title="Frequency">
                Freq
              </th>
              <th className="managed-col-due" title="Due day">
                Due
                <br />
                day
              </th>
              <th className="managed-col-money" title="Payment">
                Payment
              </th>
              <th className="managed-col-rem" title="Payments remaining">
                Payments
                <br />
                remaining
              </th>
              <th className="managed-col-paid" title="Paid through">
                Paid
                <br />
                through
              </th>
            </tr>
          </thead>
          <tbody>
            <tr className="managed-section">
              <th scope="colgroup" colSpan={11} aria-label="Loans">
                Loans
              </th>
            </tr>
            {loanAccounts.length === 0 ? (
              <tr>
                <td colSpan={11}>
                  {loadState === "loading"
                    ? "Loading loans…"
                    : loadState === "error"
                      ? "Loans unavailable — see status above (usually a migration / restart)."
                      : "No loans loaded."}
                </td>
              </tr>
            ) : (
              loanAccounts.map(renderAccountRow)
            )}
            {renderSectionTotal("Loans total", "Loans total", loanTotals, true)}
            <tr className="managed-section">
              <th scope="colgroup" colSpan={11} aria-label="Escrow account">
                Escrow account
              </th>
            </tr>
            {escrowAccounts.length === 0 ? (
              <tr>
                <td colSpan={11}>No escrow account loaded.</td>
              </tr>
            ) : (
              escrowAccounts.map(renderAccountRow)
            )}
            {renderSectionTotal("Escrow total", "Escrow total", escrowTotals, false)}
          </tbody>
        </table>
      </div>
      <section
        className="debt-bucket-panel"
        id="debt-bucket-panel"
        data-section="bucket-manager"
        aria-label="Bucket manager"
      >
        <h3>Bucket manager</h3>
        <div className="debt-bucket-toolbar buttons">
          <button
            type="button"
            aria-label="Add bucket"
            disabled={busy || editing}
            onClick={() => {
              setSelectedBucketId(null);
              setBucketCreating(true);
              setBucketDraft(blankBucketDraft());
              setBucketEditing(true);
              setStatus(null);
            }}
          >
            Add bucket
          </button>
          <button
            type="button"
            aria-label="Save bucket"
            className={bucketDirty ? "is-unsaved" : undefined}
            disabled={!bucketDirty || busy || editing}
            onClick={() => void saveBucket()}
          >
            Save
          </button>
          <button
            type="button"
            aria-label="Cancel bucket edit"
            disabled={!bucketEditing || busy}
            onClick={closeBucketSetup}
          >
            Cancel
          </button>
        </div>
        <div className="table-wrap managed-wrap" data-part="debt-buckets">
          <table aria-label="Budget buckets">
            <thead>
              <tr>
                <th>Bucket Name</th>
                <th>Budget category</th>
                <th>Associated bank</th>
                <th>Description</th>
              </tr>
            </thead>
            <tbody>
              {bucketCreating && bucketDraft ? (
                <tr className="is-selected" aria-selected={true}>
                  <td>
                    <input
                      aria-label="Bucket name"
                      value={bucketDraft.name}
                      onChange={(event) =>
                        setBucketDraft({ ...bucketDraft, name: event.target.value })
                      }
                    />
                  </td>
                  <td>
                    <select
                      aria-label="Bucket budget category"
                      value={bucketDraft.budgetCategory}
                      onChange={(event) =>
                        setBucketDraft({ ...bucketDraft, budgetCategory: event.target.value })
                      }
                    >
                      <option value="">(same as name)</option>
                      {BUDGET_CATEGORY_OPTIONS.map((name) => (
                        <option key={name} value={name}>
                          {name}
                        </option>
                      ))}
                    </select>
                  </td>
                  <td>
                    <BucketBankField
                      key="new-bucket-bank"
                      value={bucketDraft.bank}
                      options={bankOptions}
                      onChange={(bank) => setBucketDraft({ ...bucketDraft, bank })}
                      onAdded={rememberBank}
                    />
                  </td>
                  <td>
                    <input
                      aria-label="Bucket description"
                      value={bucketDraft.description}
                      onChange={(event) =>
                        setBucketDraft({ ...bucketDraft, description: event.target.value })
                      }
                    />
                  </td>
                </tr>
              ) : null}
              {buckets.map((bucket) => {
                const rowEditing =
                  !bucketCreating &&
                  bucketEditing &&
                  bucketDraft != null &&
                  selectedBucketId === bucket.bucketId;
                return (
                  <tr
                    key={bucket.bucketId}
                    className={selectedBucketId === bucket.bucketId ? "is-selected" : undefined}
                    aria-selected={selectedBucketId === bucket.bucketId}
                    onClick={() => {
                      if (editing || bucketCreating) return;
                      if (bucketEditing && bucketDirty && selectedBucketId !== bucket.bucketId) {
                        return;
                      }
                      setSelectedBucketId(bucket.bucketId);
                      setBucketCreating(false);
                      setBucketDraft(draftFromBucket(bucket));
                      setBucketEditing(true);
                      setStatus(null);
                    }}
                  >
                    {rowEditing && bucketDraft ? (
                      <>
                        <td onClick={(event) => event.stopPropagation()}>
                          <input
                            aria-label="Bucket name"
                            value={bucketDraft.name}
                            onChange={(event) =>
                              setBucketDraft({ ...bucketDraft, name: event.target.value })
                            }
                          />
                        </td>
                        <td onClick={(event) => event.stopPropagation()}>
                          <select
                            aria-label="Bucket budget category"
                            value={bucketDraft.budgetCategory}
                            onChange={(event) =>
                              setBucketDraft({
                                ...bucketDraft,
                                budgetCategory: event.target.value,
                              })
                            }
                          >
                            <option value="">(same as name)</option>
                            {BUDGET_CATEGORY_OPTIONS.map((name) => (
                              <option key={name} value={name}>
                                {name}
                              </option>
                            ))}
                            {bucketDraft.budgetCategory &&
                            !(BUDGET_CATEGORY_OPTIONS as readonly string[]).includes(
                              bucketDraft.budgetCategory,
                            ) ? (
                              <option value={bucketDraft.budgetCategory}>
                                {bucketDraft.budgetCategory}
                              </option>
                            ) : null}
                          </select>
                        </td>
                        <td onClick={(event) => event.stopPropagation()}>
                          <BucketBankField
                            key={bucket.bucketId}
                            value={bucketDraft.bank}
                            options={bankOptions}
                            onChange={(bank) => setBucketDraft({ ...bucketDraft, bank })}
                            onAdded={rememberBank}
                          />
                        </td>
                        <td onClick={(event) => event.stopPropagation()}>
                          <input
                            aria-label="Bucket description"
                            value={bucketDraft.description}
                            onChange={(event) =>
                              setBucketDraft({
                                ...bucketDraft,
                                description: event.target.value,
                              })
                            }
                          />
                        </td>
                      </>
                    ) : (
                      <>
                        <td>{bucket.name}</td>
                        <td>{bucket.budgetCategory || "—"}</td>
                        <td>{bucket.bank || "—"}</td>
                        <td>{bucket.description || "—"}</td>
                      </>
                    )}
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      </section>
      {editing && draft && (creating || selected) ? (
        <div className="home-av-dialog-backdrop" onClick={closeSetup}>
          <form
            role="dialog"
            aria-modal="true"
            aria-label="Loan setup"
            data-part="loan-setup"
            className="home-av-dialog managed-setup"
            onClick={(event) => event.stopPropagation()}
            onSubmit={(event) => {
              event.preventDefault();
              void save();
            }}
          >
            <h3>Loan setup · {draft.name.trim() || "New loan"}</h3>
            <label>
              Account Name
              <input
                aria-label="Account name"
                value={draft.accountName}
                onChange={(event) => setDraft({ ...draft, accountName: event.target.value })}
              />
            </label>
            <label>
              Loan Name
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
                value={
                  selected?.kind === "credit" && !creating
                    ? "register"
                    : draft.process || "week_ahead"
                }
                disabled={selected?.kind === "credit" && !creating}
                onChange={(event) => {
                  setDraft({ ...draft, process: event.target.value });
                }}
              >
                {selected?.kind === "credit" && !creating ? (
                  <option value="register" title={ESCROW_PROCESS[2]}>
                    {ESCROW_PROCESS[1]}
                  </option>
                ) : (
                  DEBT_PROCESSES.map(([key, label, tip]) => (
                    <option key={key} value={key} title={tip}>
                      {label}
                    </option>
                  ))
                )}
              </select>
            </label>
            {selected?.kind !== "credit" || creating ? (
              draft.process === "week_ahead" || !draft.process ? (
                <>
                  <label>
                    Frequency
                    <select
                      aria-label="Loan frequency"
                      value={draft.frequency || "monthly"}
                      onChange={(event) =>
                        setDraft({ ...draft, frequency: event.target.value })
                      }
                    >
                      {FREQUENCIES.map(([key, label]) => (
                        <option key={key} value={key}>
                          {label}
                        </option>
                      ))}
                    </select>
                  </label>
                  <label>
                    Due day
                    <input
                      aria-label="Loan due day"
                      value={draft.due}
                      placeholder="7"
                      title="Day of month for Week Ahead interest verify"
                      onChange={(event) => setDraft({ ...draft, due: event.target.value })}
                    />
                  </label>
                </>
              ) : (
              <>
                <label>
                  Loan Element
                  <select
                    aria-label="Loan Element"
                    value={
                      draft.elementNew
                        ? NEW_LOAN_ELEMENT
                        : draft.elementId || ""
                    }
                    onChange={(event) => pickLoanElement(event.target.value)}
                  >
                    <option value="">Choose a Loan Element</option>
                    <option value={NEW_LOAN_ELEMENT}>New Loan Element…</option>
                    {elements
                      .filter((element) => element.account === "Loan")
                      .map((element) => (
                        <option key={element.elementId} value={element.elementId}>
                          {element.note || element.account}
                        </option>
                      ))}
                  </select>
                </label>
                {draft.elementId || draft.elementNew ? (
                  <fieldset className="loan-element-fields" aria-label="Loan Element details">
                    <legend>Loan Element details</legend>
                    <label>
                      Name
                      <input
                        aria-label="Element name"
                        value={draft.elementName}
                        onChange={(event) =>
                          setDraft({ ...draft, elementName: event.target.value })
                        }
                      />
                    </label>
                    <label>
                      Amount
                      <input
                        aria-label="Element amount"
                        inputMode="decimal"
                        value={draft.elementAmount}
                        onChange={(event) =>
                          setDraft({ ...draft, elementAmount: event.target.value })
                        }
                      />
                    </label>
                    <label>
                      Frequency
                      <select
                        aria-label="Element frequency"
                        value={draft.elementCadence}
                        onChange={(event) =>
                          setDraft({ ...draft, elementCadence: event.target.value })
                        }
                      >
                        {ELEMENT_CADENCES.map(([key, label]) => (
                          <option key={key} value={key}>
                            {label}
                          </option>
                        ))}
                      </select>
                    </label>
                    <label>
                      Schedule Date
                      <input
                        aria-label="Element schedule date"
                        value={draft.elementDay}
                        onChange={(event) =>
                          setDraft({ ...draft, elementDay: event.target.value })
                        }
                      />
                    </label>
                    <label>
                      Start date
                      <input
                        type="date"
                        aria-label="Element start date"
                        value={draft.elementStartOn}
                        onChange={(event) =>
                          setDraft({ ...draft, elementStartOn: event.target.value })
                        }
                      />
                    </label>
                    <label>
                      Expiration
                      <input
                        type="date"
                        aria-label="Element stop date"
                        value={draft.elementStopOn}
                        onChange={(event) =>
                          setDraft({ ...draft, elementStopOn: event.target.value })
                        }
                      />
                    </label>
                    {draft.elementStopOn ? null : (
                      <p className="element-never-expires">Never expires</p>
                    )}
                    <button
                      type="button"
                      className="element-exceptions-link"
                      aria-label="Open exceptions"
                      disabled={busy || !draft.elementId || draft.elementNew}
                      onClick={() => setExceptionsOpen(true)}
                    >
                      {(elements.find((element) => element.elementId === draft.elementId)
                        ?.exceptionCount ?? 0) + " Exceptions"}
                    </button>
                  </fieldset>
                ) : null}
              </>
              )
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
            {selected?.kind === "credit" && !creating ? null : (
              <label>
                Payment
                <input
                  aria-label="Payment"
                  inputMode="decimal"
                  value={draft.payment}
                  title="Loan payment applied to balance; may differ from the Loan Element amount"
                  onChange={(event) => setDraft({ ...draft, payment: event.target.value })}
                />
              </label>
            )}
            {selected?.kind === "credit" && !creating ? null : (
              <label className="loan-inactive-check">
                <input
                  type="checkbox"
                  aria-label="Inactive loan"
                  checked={draft.inactive}
                  onChange={(event) =>
                    setDraft({ ...draft, inactive: event.target.checked })
                  }
                />
                Inactive (hide from loan list; keep history)
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
          {exceptionsOpen && draft.elementId && !draft.elementNew
            ? (() => {
                const found = elements.find((element) => element.elementId === draft.elementId);
                if (!found) return null;
                const record: CashElementRecord = {
                  elementId: found.elementId,
                  account: "Loan",
                  kind: found.kind || "Withdrawal",
                  cadence: draft.elementCadence || found.cadence || "monthly",
                  amountMinor: parseDollars(draft.elementAmount) ?? found.amountMinor,
                  note: draft.elementName.trim() || found.note,
                  weekdayOrMonthDay: draft.elementDay || found.weekdayOrMonthDay || "1",
                  startOn: draft.elementStartOn || found.startOn,
                  stopOn: draft.elementStopOn || found.stopOn,
                  exceptionCount: found.exceptionCount,
                  exceptions: found.exceptions,
                  upcoming: found.upcoming,
                };
                const occurrences: EditorOccurrence[] = (
                  record.upcoming ??
                  record.exceptions ??
                  []
                ).map((row) => ({
                  occurrenceId: row.occurrenceId,
                  occurredOn: row.occurredOn,
                  amountMinor: row.amountMinor,
                  isException: row.isException,
                  isCancelled: row.isCancelled,
                }));
                return (
                  <div
                    className="home-av-dialog-backdrop loan-exceptions-backdrop"
                    onClick={(event) => {
                      event.stopPropagation();
                      setExceptionsOpen(false);
                    }}
                  >
                    <div
                      className="home-av-dialog managed-setup loan-exceptions-dialog"
                      onClick={(event) => event.stopPropagation()}
                    >
                      <CashElementExceptions
                        account="Loan"
                        element={record}
                        occurrences={occurrences}
                        busy={busy}
                        onSave={async (body) => {
                          setBusy(true);
                          try {
                            const result = await client.executeCommand("PlannedOccurrenceSave", {
                              elementId: body.elementId,
                              occurrenceId: body.occurrenceId,
                              asOfDate: todayIso(),
                              cancel: body.cancel,
                              occurredOn: body.occurredOn,
                              amountMinor: body.amountMinor,
                            });
                            if (!result.ok) {
                              setStatus(
                                `Exception save failed: ${result.errorCode ?? "error"}`,
                              );
                              return false;
                            }
                            await refreshElements();
                            setStatus("Exception saved.");
                            return true;
                          } finally {
                            setBusy(false);
                          }
                        }}
                        onClose={() => setExceptionsOpen(false)}
                      />
                    </div>
                  </div>
                );
              })()
            : null}
        </div>
      ) : (
        <p>Select a loan, then Edit loan, or add a loan and choose how it is paid.</p>
      )}
      {selected ? (
        <div className="loan-detail">
          <section
            className="loan-vendor"
            id="loan-vendor"
            data-section="loan-vendor"
            aria-label="Loan Vendor Data"
          >
            <h3>Loan Vendor Data · {VENDOR_YEAR}</h3>
            {selected.vendor ? (
              <>
                <div className="loan-vendor-summary" data-part="loan-vendor-ring">
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
                  <table
                    className="loan-vendor-facts"
                    aria-label="Loan facts"
                    data-part="loan-facts"
                  >
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
                <div className="table-wrap loan-vendor-lines" data-part="loan-vendor-history">
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
                  <div
                    className="table-wrap loan-vendor-lines loan-amort"
                    data-part="loan-remaining-payments"
                  >
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
          <div
            className="managed-register"
            id="managed-register"
            data-section="loan-register"
          >
            <h3>
              Our transactions
              {projected(selected)
                ? ` · next split ${money(projected(selected)?.principal)} principal, ${money(projected(selected)?.interest)} interest`
                : ""}
            </h3>
            <div className="table-wrap managed-lines" data-part="loan-our-transactions">
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
