import { useEffect, useRef, useState } from "react";
import { formatUsd, formatWeekChooserLabel } from "@finos/ui-components";

export type TrendsCashReference = {
  accountId: string;
  accountName: string;
  displayName: string;
  cashSymbol: string;
  referenceMinor: number | null;
};

export type TrendsWeekCapture = {
  periodStart: string;
  periodEnd: string;
  weekYear?: number;
  weekNumber?: number;
  capturedAt: string;
  closed: boolean;
  exists: boolean;
  prior?: {
    profitMinor: number;
    monthlyDivsMinor: number;
    fidelityTotalMinor: number;
    schwabTotalMinor: number;
    incomeCashMinor: number;
    acct9CashMinor: number;
    acct9EtfValueMinor: number;
  } | null;
  current?: {
    profitMinor: number;
    monthlyDivsMinor: number;
    fidelityTotalMinor: number;
    schwabTotalMinor: number;
    incomeCashMinor: number;
    acct9CashMinor: number;
    acct9EtfValueMinor: number;
    closed?: boolean;
  } | null;
  carBalanceMinor?: number | null;
  incomeBalanceMinor?: number | null;
  healthBalanceMinor?: number | null;
  rothBalanceMinor?: number | null;
  speculationBalanceMinor?: number | null;
  acct9BalanceMinor?: number | null;
  carCashMinor?: number | null;
  healthCashMinor?: number | null;
  rothCashMinor?: number | null;
  speculationCashMinor?: number | null;
  suggestedProfitMinor: number;
  suggestedMonthlyDivsMinor: number;
  suggestedAcct9EtfProxyMinor?: number | null;
  firstUnpopulatedStart?: string;
  chooserSaturdays?: string[];
  populatedPeriodEnds?: string[];
  missingRequired: string[];
  scale: number;
  cashReferences?: TrendsCashReference[];
};

type Draft = {
  incomeBalanceMinor: string;
  incomeCashMinor: string;
  rothBalanceMinor: string;
  rothCashMinor: string;
  speculationBalanceMinor: string;
  speculationCashMinor: string;
  healthBalanceMinor: string;
  healthCashMinor: string;
  carBalanceMinor: string;
  carCashMinor: string;
  acct9BalanceMinor: string;
  acct9CashMinor: string;
  /** Typed Account 9 ETF total; blank means 70% is — and excluded from totals. */
  acct9EtfTotalMinor: string;
};

/** Slice 1b: Week → one capture grid → Review (+ recon). Speculation is a grid row. */
const STEPS = ["Week", "Capture", "Review"] as const;

type Step = (typeof STEPS)[number];

const ACCOUNT_ROWS: Array<{
  label: string;
  totalKey: keyof Draft;
  cashKey: keyof Draft;
  accountName: string;
  totalLabel: string;
  cashLabel: string;
}> = [
  {
    label: "Income",
    totalKey: "incomeBalanceMinor",
    cashKey: "incomeCashMinor",
    accountName: "Income",
    totalLabel: "Income Total Balance",
    cashLabel: "Income Cash Balance",
  },
  {
    label: "FI Roth",
    totalKey: "rothBalanceMinor",
    cashKey: "rothCashMinor",
    accountName: "FI Roth",
    totalLabel: "FI Roth Total Balance",
    cashLabel: "FI Roth Cash Balance",
  },
  {
    label: "Speculation",
    totalKey: "speculationBalanceMinor",
    cashKey: "speculationCashMinor",
    accountName: "Speculation",
    totalLabel: "Speculation Total Balance",
    cashLabel: "Speculation Cash Balance",
  },
  {
    label: "Health",
    totalKey: "healthBalanceMinor",
    cashKey: "healthCashMinor",
    accountName: "Health",
    totalLabel: "Health Total Balance",
    cashLabel: "Health Cash Balance",
  },
  {
    label: "Car",
    totalKey: "carBalanceMinor",
    cashKey: "carCashMinor",
    accountName: "Car",
    totalLabel: "Car Total Balance",
    cashLabel: "Car Cash Balance",
  },
  {
    label: "Account 9",
    totalKey: "acct9BalanceMinor",
    cashKey: "acct9CashMinor",
    accountName: "9",
    totalLabel: "Account 9 Total Balance",
    cashLabel: "Account 9 Cash Balance",
  },
];

const REASON_KINDS = ["fee", "split", "other"] as const;

function minorToInput(v: number | null | undefined, scale: number): string {
  if (v == null) return "";
  return (v / 10 ** scale).toFixed(scale);
}

function inputToMinor(raw: string, scale: number): number | null {
  const t = raw.trim();
  if (!t) return null;
  const n = Number(t);
  if (!Number.isFinite(n)) return null;
  return Math.round(n * 10 ** scale);
}

/** Stored value is already the 70% amount; invert for the typed ETF total field. */
function etfTotalFromStoredSeventy(seventyMinor: number | null | undefined, scale: number): string {
  if (seventyMinor == null || seventyMinor === 0) return "";
  const total = Math.round((seventyMinor * 100) / 70);
  return minorToInput(total, scale);
}

/** Owner types ETF total → system 70% (integer cents). Blank → null (UI —). */
function seventyFromEtfTotal(raw: string, scale: number): number | null {
  const total = inputToMinor(raw, scale);
  if (total == null) return null;
  return Math.round((total * 70) / 100);
}

function draftFromCapture(c: TrendsWeekCapture): Draft {
  const cur = c.current;
  const scale = c.scale ?? 2;
  const savedSeventy = cur?.acct9EtfValueMinor;
  return {
    incomeBalanceMinor: minorToInput(c.incomeBalanceMinor, scale),
    incomeCashMinor: minorToInput(cur?.incomeCashMinor ?? null, scale),
    rothBalanceMinor: minorToInput(c.rothBalanceMinor, scale),
    rothCashMinor: minorToInput(c.rothCashMinor, scale),
    speculationBalanceMinor: minorToInput(c.speculationBalanceMinor, scale),
    speculationCashMinor: minorToInput(c.speculationCashMinor, scale),
    healthBalanceMinor: minorToInput(c.healthBalanceMinor, scale),
    healthCashMinor: minorToInput(c.healthCashMinor, scale),
    carBalanceMinor: minorToInput(c.carBalanceMinor, scale),
    carCashMinor: minorToInput(c.carCashMinor, scale),
    acct9BalanceMinor: minorToInput(
      c.acct9BalanceMinor ?? cur?.schwabTotalMinor,
      scale,
    ),
    acct9CashMinor: minorToInput(cur?.acct9CashMinor, scale),
    acct9EtfTotalMinor: etfTotalFromStoredSeventy(savedSeventy, scale),
  };
}

function nextStep(step: Step): Step {
  const i = STEPS.indexOf(step);
  return STEPS[Math.min(i + 1, STEPS.length - 1)] ?? "Review";
}

function prevStep(step: Step): Step {
  const i = STEPS.indexOf(step);
  return STEPS[Math.max(i - 1, 0)] ?? "Week";
}

function formatRef(ref: number | null, scale: number): string {
  if (ref == null) return "—";
  return formatUsd(ref, scale);
}

function weekHydrateKey(c: TrendsWeekCapture): string {
  return [
    c.periodStart,
    c.periodEnd,
    String(c.exists),
    String(c.closed),
    String(c.current?.incomeCashMinor ?? ""),
    String(c.current?.acct9CashMinor ?? ""),
    String(c.current?.acct9EtfValueMinor ?? ""),
    String(c.incomeBalanceMinor ?? ""),
    String(c.rothCashMinor ?? ""),
    String(c.carCashMinor ?? ""),
  ].join("|");
}

export function TrendsCapturePanel({
  capture,
  busy,
  onReload,
  onSave,
  onClose,
  onWizardActive,
}: {
  capture: TrendsWeekCapture | null;
  busy?: boolean;
  onReload: (asOf: string) => void;
  onSave: (body: Record<string, unknown>, correct: boolean) => Promise<void>;
  onClose: (periodEnd: string) => Promise<void>;
  onWizardActive?: (active: boolean) => void;
}) {
  const [step, setStep] = useState<Step>("Week");
  const [draft, setDraft] = useState<Draft | null>(null);
  const [dirty, setDirty] = useState(false);
  const [weekPicked, setWeekPicked] = useState(false);
  const [reasons, setReasons] = useState<Record<string, { kind: string; detail: string }>>({});
  const openedGap = useRef(false);
  const lastHydrateKey = useRef<string>("");
  /** Keeps last typed draft for the week so Edit never blanks an in-progress form. */
  const typedDraftRef = useRef<Draft | null>(null);

  useEffect(() => {
    if (!capture) return;
    const key = weekHydrateKey(capture);
    if (key === lastHydrateKey.current && typedDraftRef.current) {
      // Same week payload — keep what the owner typed (Edit / re-render safe).
      setDraft(typedDraftRef.current);
      return;
    }
    lastHydrateKey.current = key;
    const next = draftFromCapture(capture);
    typedDraftRef.current = next;
    setDraft(next);
    setDirty(false);
    setReasons({});
  }, [capture]);

  useEffect(() => {
    if (
      openedGap.current ||
      weekPicked ||
      !capture?.firstUnpopulatedStart ||
      capture.periodStart === capture.firstUnpopulatedStart
    ) {
      return;
    }
    openedGap.current = true;
    onReload(capture.firstUnpopulatedStart);
  }, [capture, onReload, weekPicked]);

  useEffect(() => {
    onWizardActive?.(step !== "Week" || dirty);
  }, [step, dirty, onWizardActive]);

  if (!capture || !draft) {
    return (
      <div className="trends-capture" aria-label="Trends weekly capture">
        <p>Loading weekly capture…</p>
        {busy ? (
          <div className="trends-capture-busy" aria-busy="true" role="status">
            <div className="process-a-research-spinner" aria-hidden="true" />
            <span>Working…</span>
          </div>
        ) : null}
      </div>
    );
  }

  const scale = capture.scale ?? 2;
  const todaySat = new Date().toISOString().slice(0, 10);
  const chooser =
    capture.chooserSaturdays && capture.chooserSaturdays.length > 0
      ? capture.chooserSaturdays
      : [capture.periodStart];
  const selectedSat =
    (weekPicked ? capture.periodStart : capture.firstUnpopulatedStart) ||
    capture.periodStart ||
    "";

  const setField = (key: keyof Draft, value: string) => {
    setDraft((d) => {
      if (!d) return d;
      const next = { ...d, [key]: value };
      typedDraftRef.current = next;
      return next;
    });
    setDirty(true);
  };

  const incomeBal = inputToMinor(draft.incomeBalanceMinor, scale) ?? 0;
  const rothBal = inputToMinor(draft.rothBalanceMinor, scale) ?? 0;
  const specBal = inputToMinor(draft.speculationBalanceMinor, scale) ?? 0;
  const healthBal = inputToMinor(draft.healthBalanceMinor, scale) ?? 0;
  const carBal = inputToMinor(draft.carBalanceMinor, scale) ?? 0;
  const acct9Bal = inputToMinor(draft.acct9BalanceMinor, scale) ?? 0;
  const incomeCash = inputToMinor(draft.incomeCashMinor, scale) ?? 0;
  const rothCash = inputToMinor(draft.rothCashMinor, scale) ?? 0;
  const specCash = inputToMinor(draft.speculationCashMinor, scale) ?? 0;
  const healthCash = inputToMinor(draft.healthCashMinor, scale) ?? 0;
  const carCash = inputToMinor(draft.carCashMinor, scale) ?? 0;
  const acct9Cash = inputToMinor(draft.acct9CashMinor, scale) ?? 0;
  const etfSeventy = seventyFromEtfTotal(draft.acct9EtfTotalMinor, scale);
  const etfValueForTotals = etfSeventy ?? 0;
  const fid = incomeBal + rothBal + specBal + healthBal + carBal;
  const schwab = acct9Bal;
  const totalCash =
    incomeCash +
    rothCash +
    specCash +
    healthCash +
    carCash +
    acct9Cash +
    etfValueForTotals;
  const fidChange = fid - (capture.prior?.fidelityTotalMinor ?? 0);
  const schChange = schwab - (capture.prior?.schwabTotalMinor ?? 0);
  const weekIncome = capture.suggestedMonthlyDivsMinor;
  const profit = capture.suggestedProfitMinor;

  const typedByAccount: Record<string, number> = {
    Income: incomeCash,
    "FI Roth": rothCash,
    Speculation: specCash,
    Health: healthCash,
    Car: carCash,
    "9": acct9Cash,
  };

  const reconRows = (capture.cashReferences ?? []).map((ref) => {
    const typed = typedByAccount[ref.accountName] ?? 0;
    const gap =
      ref.referenceMinor == null ? null : typed - ref.referenceMinor;
    const material = gap != null && Math.abs(gap) >= 1;
    return { ref, typed, gap, material };
  });

  const materialGaps = reconRows.filter((r) => r.material);
  const reasonsReady = materialGaps.every((r) => {
    const row = reasons[r.ref.accountId];
    return row && REASON_KINDS.includes(row.kind as (typeof REASON_KINDS)[number]);
  });

  const gridFilled = ACCOUNT_ROWS.every(
    (row) => draft[row.totalKey].trim() !== "" && draft[row.cashKey].trim() !== "",
  );

  const buildBody = () => {
    const adjusts = materialGaps.map((r) => {
      const row = reasons[r.ref.accountId] ?? { kind: "fee", detail: "" };
      const reason =
        row.detail.trim() !== "" ? `${row.kind}: ${row.detail.trim()}` : row.kind;
      return {
        accountId: r.ref.accountId,
        amountMinor: r.gap as number,
        reason,
      };
    });
    return {
      periodStart: capture.periodStart,
      periodEnd: capture.periodEnd,
      capturedAt: new Date().toISOString(),
      profitMinor: profit,
      monthlyDivsMinor: weekIncome,
      fidelityTotalMinor: fid,
      schwabTotalMinor: schwab,
      incomeCashMinor: incomeCash,
      acct9CashMinor: acct9Cash,
      // Blank ETF total → store 0 (not suggested proxy); UI shows — and excludes from totals.
      acct9EtfValueMinor: etfSeventy ?? 0,
      carBalanceMinor: inputToMinor(draft.carBalanceMinor, scale),
      incomeBalanceMinor: inputToMinor(draft.incomeBalanceMinor, scale),
      healthBalanceMinor: inputToMinor(draft.healthBalanceMinor, scale),
      rothBalanceMinor: inputToMinor(draft.rothBalanceMinor, scale),
      speculationBalanceMinor: inputToMinor(draft.speculationBalanceMinor, scale),
      acct9BalanceMinor: inputToMinor(draft.acct9BalanceMinor, scale),
      carCashMinor: inputToMinor(draft.carCashMinor, scale),
      healthCashMinor: inputToMinor(draft.healthCashMinor, scale),
      rothCashMinor: inputToMinor(draft.rothCashMinor, scale),
      speculationCashMinor: inputToMinor(draft.speculationCashMinor, scale),
      scale,
      closed: capture.closed,
      adjusts,
    };
  };

  const acceptBlocked =
    busy ||
    capture.closed ||
    (materialGaps.length > 0 && !reasonsReady);

  const reloadWeekDraft = (from: TrendsWeekCapture) => {
    const next = draftFromCapture(from);
    typedDraftRef.current = next;
    lastHydrateKey.current = weekHydrateKey(from);
    setDraft(next);
    setDirty(false);
    setReasons({});
  };

  return (
    <div className="trends-capture" aria-label="Trends weekly capture">
      {busy ? (
        <div className="trends-capture-busy" aria-busy="true" role="status">
          <div className="process-a-research-spinner" aria-hidden="true" />
          <span>Working…</span>
        </div>
      ) : null}
      <ol className="cart-step-rail" aria-label="Trends capture steps">
        {STEPS.map((name) => (
          <li key={name} aria-current={name === step ? "step" : undefined}>
            {name}
          </li>
        ))}
      </ol>
      {step === "Week" ? (
        <div className="trends-period-bar">
          <label className="income-week-label">
            Week
            <select
              aria-label="Select week"
              value={selectedSat}
              disabled={busy}
              onChange={(e) => {
                setWeekPicked(true);
                setStep("Week");
                typedDraftRef.current = null;
                lastHydrateKey.current = "";
                onReload(e.target.value);
              }}
            >
              {chooser.map((sat) => (
                <option key={sat} value={sat}>
                  {formatWeekChooserLabel(sat, todaySat)}
                </option>
              ))}
            </select>
          </label>
          <span>
            {formatWeekChooserLabel(capture.periodStart || selectedSat, todaySat)}
            {capture.closed ? " (closed)" : capture.exists ? " (saved)" : " (open)"}
          </span>
        </div>
      ) : null}
      {step === "Capture" ? (
        <div className="trends-capture-entry" aria-label="Week capture grid">
          <table className="trends-capture-table">
            <thead>
              <tr>
                <th>Account</th>
                <th>Total Balance</th>
                <th>Cash Balance</th>
                <th>ETF total → 70%</th>
              </tr>
            </thead>
            <tbody>
              {ACCOUNT_ROWS.map((row) => {
                const isAcct9 = row.accountName === "9";
                return (
                  <tr key={row.accountName}>
                    <td>{row.label}</td>
                    <td>
                      <input
                        aria-label={row.totalLabel}
                        inputMode="decimal"
                        value={draft[row.totalKey]}
                        disabled={busy || (capture.closed && !dirty)}
                        onChange={(e) => setField(row.totalKey, e.target.value)}
                      />
                    </td>
                    <td>
                      <input
                        aria-label={row.cashLabel}
                        inputMode="decimal"
                        value={draft[row.cashKey]}
                        disabled={busy || (capture.closed && !dirty)}
                        onChange={(e) => setField(row.cashKey, e.target.value)}
                      />
                    </td>
                    <td>
                      {isAcct9 ? (
                        <span className="trends-capture-etf">
                          <input
                            aria-label="Account 9 ETF total"
                            inputMode="decimal"
                            value={draft.acct9EtfTotalMinor}
                            disabled={busy || (capture.closed && !dirty)}
                            onChange={(e) => setField("acct9EtfTotalMinor", e.target.value)}
                          />
                          <span aria-label="Account 9 70% ETF">
                            {etfSeventy == null ? "—" : formatUsd(etfSeventy, scale)}
                          </span>
                        </span>
                      ) : (
                        "—"
                      )}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      ) : null}
      {step === "Review" ? (
        <>
          <dl className="trends-capture-review" aria-label="Trends week review">
            <div>
              <dt>Week income</dt>
              <dd>{formatUsd(weekIncome, scale)}</dd>
            </div>
            <div>
              <dt>Profit</dt>
              <dd>{formatUsd(profit, scale)}</dd>
            </div>
            <div>
              <dt>Total cash</dt>
              <dd>{formatUsd(totalCash, scale)}</dd>
            </div>
            <div>
              <dt>Total Fidelity</dt>
              <dd>{formatUsd(fid, scale)}</dd>
            </div>
            <div>
              <dt>Total Schwab</dt>
              <dd>{formatUsd(schwab, scale)}</dd>
            </div>
            <div>
              <dt>Fidelity week-to-week</dt>
              <dd>{formatUsd(fidChange, scale)}</dd>
            </div>
            <div>
              <dt>Schwab week-to-week</dt>
              <dd>{formatUsd(schChange, scale)}</dd>
            </div>
            <div>
              <dt>Account 9 70% ETF</dt>
              <dd aria-label="Account 9 70% ETF review">
                {etfSeventy == null ? "—" : formatUsd(etfSeventy, scale)}
              </dd>
            </div>
          </dl>
          <div className="trends-cash-recon" aria-label="Week cash recon">
            <table>
              <thead>
                <tr>
                  <th>Account</th>
                  <th>Typed</th>
                  <th>Reference</th>
                  <th>Gap</th>
                  <th>Reason</th>
                </tr>
              </thead>
              <tbody>
                {reconRows.map(({ ref, typed, gap, material }) => (
                  <tr key={ref.accountId}>
                    <td>{ref.displayName}</td>
                    <td>{formatUsd(typed, scale)}</td>
                    <td>{formatRef(ref.referenceMinor, scale)}</td>
                    <td>
                      {gap == null ? "—" : formatUsd(gap, scale)}
                    </td>
                    <td>
                      {material ? (
                        <span className="trends-cash-recon-reason">
                          <select
                            aria-label={`${ref.displayName} cash adjust reason`}
                            value={reasons[ref.accountId]?.kind ?? ""}
                            disabled={busy}
                            onChange={(e) => {
                              const kind = e.target.value;
                              setReasons((prev) => ({
                                ...prev,
                                [ref.accountId]: {
                                  kind,
                                  detail: prev[ref.accountId]?.detail ?? "",
                                },
                              }));
                              setDirty(true);
                            }}
                          >
                            <option value="">Select</option>
                            {REASON_KINDS.map((k) => (
                              <option key={k} value={k}>
                                {k}
                              </option>
                            ))}
                          </select>
                          <input
                            aria-label={`${ref.displayName} cash adjust detail`}
                            placeholder="optional detail"
                            value={reasons[ref.accountId]?.detail ?? ""}
                            disabled={busy}
                            onChange={(e) => {
                              setReasons((prev) => ({
                                ...prev,
                                [ref.accountId]: {
                                  kind: prev[ref.accountId]?.kind ?? "fee",
                                  detail: e.target.value,
                                },
                              }));
                              setDirty(true);
                            }}
                          />
                        </span>
                      ) : (
                        "—"
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </>
      ) : null}
      <div className="buttons">
        {step !== "Week" ? (
          <button
            type="button"
            aria-label="Previous Trends step"
            disabled={busy}
            onClick={() => setStep(prevStep(step))}
          >
            Back
          </button>
        ) : null}
        {step !== "Review" ? (
          <button
            type="button"
            aria-label="Next Trends step"
            disabled={busy || (step === "Capture" && !gridFilled)}
            onClick={() => setStep(nextStep(step))}
          >
            Next
          </button>
        ) : null}
        {step === "Review" ? (
          <>
            <button
              type="button"
              aria-label="Save Trends week"
              className={dirty ? "is-unsaved" : undefined}
              disabled={acceptBlocked}
              onClick={() => {
                void onSave(buildBody(), false).then(() => {
                  typedDraftRef.current = null;
                  lastHydrateKey.current = "";
                  setStep("Week");
                });
              }}
            >
              Accept
            </button>
            <button
              type="button"
              aria-label="Edit Trends week"
              disabled={busy}
              onClick={() => {
                // Never blank restart: restore last typed draft for this week.
                if (typedDraftRef.current) {
                  setDraft(typedDraftRef.current);
                }
                setStep("Capture");
              }}
            >
              Edit
            </button>
            <button
              type="button"
              aria-label="Correct Trends week"
              disabled={busy || !capture.exists || (materialGaps.length > 0 && !reasonsReady)}
              onClick={() => void onSave(buildBody(), true)}
            >
              Correct week
            </button>
            <button
              type="button"
              aria-label="Close Trends week"
              disabled={busy || !capture.exists || capture.closed}
              onClick={() => void onClose(capture.periodEnd)}
            >
              Close week
            </button>
          </>
        ) : null}
        <button
          type="button"
          aria-label="Cancel Trends edits"
          disabled={busy || (step === "Week" && !dirty)}
          onClick={() => {
            // Abort capture completely: no WeekCaptureAccept / Cash_Adjust,
            // clear draft + recon reasons, return to clean Week (last committed).
            setReasons({});
            setDirty(false);
            setStep("Week");
            reloadWeekDraft(capture);
            onWizardActive?.(false);
          }}
        >
          Cancel
        </button>
      </div>
    </div>
  );
}
