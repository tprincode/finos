import { useEffect, useRef, useState } from "react";
import { formatUsd, formatWeekChooserLabel } from "@finos/ui-components";

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
};

const STEPS = [
  "Week",
  "Income",
  "FI Roth",
  "Speculation",
  "Health",
  "Car",
  "Account 9",
  "Review",
] as const;

type Step = (typeof STEPS)[number];

const ACCOUNT_STEPS: Array<{
  step: Step;
  totalKey: keyof Draft;
  cashKey: keyof Draft;
  totalLabel: string;
  cashLabel: string;
}> = [
  {
    step: "Income",
    totalKey: "incomeBalanceMinor",
    cashKey: "incomeCashMinor",
    totalLabel: "Income Total Balance",
    cashLabel: "Income Cash Balance",
  },
  {
    step: "FI Roth",
    totalKey: "rothBalanceMinor",
    cashKey: "rothCashMinor",
    totalLabel: "FI Roth Total Balance",
    cashLabel: "FI Roth Cash Balance",
  },
  {
    step: "Speculation",
    totalKey: "speculationBalanceMinor",
    cashKey: "speculationCashMinor",
    totalLabel: "Speculation Total Balance",
    cashLabel: "Speculation Cash Balance",
  },
  {
    step: "Health",
    totalKey: "healthBalanceMinor",
    cashKey: "healthCashMinor",
    totalLabel: "Health Total Balance",
    cashLabel: "Health Cash Balance",
  },
  {
    step: "Car",
    totalKey: "carBalanceMinor",
    cashKey: "carCashMinor",
    totalLabel: "Car Total Balance",
    cashLabel: "Car Cash Balance",
  },
  {
    step: "Account 9",
    totalKey: "acct9BalanceMinor",
    cashKey: "acct9CashMinor",
    totalLabel: "Account 9 Total Balance",
    cashLabel: "Account 9 Cash Balance",
  },
];

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

function draftFromCapture(c: TrendsWeekCapture): Draft {
  const cur = c.current;
  const scale = c.scale ?? 2;
  return {
    incomeBalanceMinor: minorToInput(c.incomeBalanceMinor, scale),
    incomeCashMinor: minorToInput(cur?.incomeCashMinor, scale),
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
  const openedGap = useRef(false);

  useEffect(() => {
    if (capture) {
      setDraft(draftFromCapture(capture));
      setDirty(false);
    }
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
    setDraft((d) => (d ? { ...d, [key]: value } : d));
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
  const etf = capture.suggestedAcct9EtfProxyMinor;
  const etfValue = etf ?? 0;
  const fid = incomeBal + rothBal + specBal + healthBal + carBal;
  const schwab = acct9Bal;
  const totalCash = incomeCash + rothCash + specCash + healthCash + carCash + acct9Cash + etfValue;
  const fidChange = fid - (capture.prior?.fidelityTotalMinor ?? 0);
  const schChange = schwab - (capture.prior?.schwabTotalMinor ?? 0);
  const weekIncome = capture.suggestedMonthlyDivsMinor;
  const profit = capture.suggestedProfitMinor;

  const accountFilled = (s: Step) => {
    const row = ACCOUNT_STEPS.find((a) => a.step === s);
    if (!row) return true;
    return draft[row.totalKey].trim() !== "" && draft[row.cashKey].trim() !== "";
  };

  const buildBody = () => ({
    periodStart: capture.periodStart,
    periodEnd: capture.periodEnd,
    capturedAt: new Date().toISOString(),
    profitMinor: profit,
    monthlyDivsMinor: weekIncome,
    fidelityTotalMinor: fid,
    schwabTotalMinor: schwab,
    incomeCashMinor: incomeCash,
    acct9CashMinor: acct9Cash,
    acct9EtfValueMinor: etf ?? 0,
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
  });

  const currentAccount = ACCOUNT_STEPS.find((a) => a.step === step);

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
      {currentAccount ? (
        <div className="trends-capture-grid">
          <label>
            {currentAccount.totalLabel}
            <input
              aria-label={currentAccount.totalLabel}
              inputMode="decimal"
              value={draft[currentAccount.totalKey]}
              disabled={busy || (capture.closed && !dirty)}
              onChange={(e) => setField(currentAccount.totalKey, e.target.value)}
            />
          </label>
          <label>
            {currentAccount.cashLabel}
            <input
              aria-label={currentAccount.cashLabel}
              inputMode="decimal"
              value={draft[currentAccount.cashKey]}
              disabled={busy || (capture.closed && !dirty)}
              onChange={(e) => setField(currentAccount.cashKey, e.target.value)}
            />
          </label>
          {step === "Account 9" ? (
            <p aria-label="Account 9 70% ETF">
              70% ETF{" "}
              {etf == null ? "unknown" : formatUsd(etf, scale)}
            </p>
          ) : null}
        </div>
      ) : null}
      {step === "Review" ? (
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
        </dl>
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
            disabled={
              busy ||
              (step !== "Week" && !accountFilled(step))
            }
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
              disabled={busy || capture.closed}
              onClick={() => {
                void onSave(buildBody(), false).then(() => setStep("Week"));
              }}
            >
              Accept
            </button>
            <button
              type="button"
              aria-label="Edit Trends week"
              disabled={busy}
              onClick={() => setStep("Income")}
            >
              Edit
            </button>
            <button
              type="button"
              aria-label="Correct Trends week"
              disabled={busy || !capture.exists}
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
          disabled={!dirty}
          onClick={() => {
            setDraft(draftFromCapture(capture));
            setDirty(false);
          }}
        >
          Cancel
        </button>
      </div>
    </div>
  );
}
