import { useEffect, useState } from "react";
import { formatUsd } from "@finos/ui-components";

export type TrendsWeekCapture = {
  periodStart: string;
  periodEnd: string;
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
  suggestedProfitMinor: number;
  suggestedMonthlyDivsMinor: number;
  suggestedAcct9EtfProxyMinor?: number | null;
  missingRequired: string[];
  scale: number;
};

type Draft = {
  profitMinor: string;
  monthlyDivsMinor: string;
  fidelityTotalMinor: string;
  schwabTotalMinor: string;
  incomeCashMinor: string;
  acct9CashMinor: string;
  acct9EtfValueMinor: string;
  carBalanceMinor: string;
  incomeBalanceMinor: string;
  healthBalanceMinor: string;
  rothBalanceMinor: string;
  speculationBalanceMinor: string;
};

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
    profitMinor: minorToInput(cur?.profitMinor ?? c.suggestedProfitMinor, scale),
    monthlyDivsMinor: minorToInput(
      cur?.monthlyDivsMinor ?? c.suggestedMonthlyDivsMinor,
      scale,
    ),
    fidelityTotalMinor: minorToInput(cur?.fidelityTotalMinor, scale),
    schwabTotalMinor: minorToInput(cur?.schwabTotalMinor, scale),
    incomeCashMinor: minorToInput(cur?.incomeCashMinor, scale),
    acct9CashMinor: minorToInput(cur?.acct9CashMinor, scale),
    acct9EtfValueMinor: minorToInput(
      cur?.acct9EtfValueMinor ?? c.suggestedAcct9EtfProxyMinor,
      scale,
    ),
    carBalanceMinor: minorToInput(c.carBalanceMinor, scale),
    incomeBalanceMinor: minorToInput(c.incomeBalanceMinor, scale),
    healthBalanceMinor: minorToInput(c.healthBalanceMinor, scale),
    rothBalanceMinor: minorToInput(c.rothBalanceMinor, scale),
    speculationBalanceMinor: minorToInput(c.speculationBalanceMinor, scale),
  };
}

export function TrendsCapturePanel({
  capture,
  busy,
  onReload,
  onSave,
  onClose,
  onCopyPrior,
}: {
  capture: TrendsWeekCapture | null;
  busy?: boolean;
  onReload: (asOf: string) => void;
  onSave: (body: Record<string, unknown>, correct: boolean) => Promise<void>;
  onClose: (periodEnd: string) => Promise<void>;
  onCopyPrior: () => void;
}) {
  const [asOf, setAsOf] = useState(capture?.periodEnd ?? "");
  const [draft, setDraft] = useState<Draft | null>(null);
  const [dirty, setDirty] = useState(false);

  useEffect(() => {
    if (capture) {
      setAsOf(capture.periodEnd);
      setDraft(draftFromCapture(capture));
      setDirty(false);
    }
  }, [capture]);

  if (!capture || !draft) {
    return <p>Loading weekly capture…</p>;
  }

  const scale = capture.scale ?? 2;
  const setField = (key: keyof Draft, value: string) => {
    setDraft((d) => (d ? { ...d, [key]: value } : d));
    setDirty(true);
  };

  const fid = inputToMinor(draft.fidelityTotalMinor, scale) ?? 0;
  const schwab = inputToMinor(draft.schwabTotalMinor, scale) ?? 0;
  const incomeCash = inputToMinor(draft.incomeCashMinor, scale) ?? 0;
  const acct9 = inputToMinor(draft.acct9CashMinor, scale) ?? 0;
  const etf = inputToMinor(draft.acct9EtfValueMinor, scale) ?? 0;
  const combined = fid + schwab;
  const totalCash = incomeCash + acct9 + etf;

  const buildBody = () => ({
    periodStart: capture.periodStart,
    periodEnd: capture.periodEnd,
    capturedAt: new Date().toISOString(),
    profitMinor: inputToMinor(draft.profitMinor, scale) ?? 0,
    monthlyDivsMinor: inputToMinor(draft.monthlyDivsMinor, scale) ?? 0,
    fidelityTotalMinor: fid,
    schwabTotalMinor: schwab,
    incomeCashMinor: incomeCash,
    acct9CashMinor: acct9,
    acct9EtfValueMinor: etf,
    carBalanceMinor: inputToMinor(draft.carBalanceMinor, scale),
    incomeBalanceMinor: inputToMinor(draft.incomeBalanceMinor, scale),
    healthBalanceMinor: inputToMinor(draft.healthBalanceMinor, scale),
    rothBalanceMinor: inputToMinor(draft.rothBalanceMinor, scale),
    speculationBalanceMinor: inputToMinor(draft.speculationBalanceMinor, scale),
    scale,
    closed: capture.closed,
  });

  return (
    <div className="trends-capture" aria-label="Trends weekly capture">
      <div className="trends-period-bar">
        <label className="trends-period-label">
          Week ending (Friday)
          <input
            type="date"
            aria-label="Trends capture as-of date"
            value={asOf}
            onChange={(e) => setAsOf(e.target.value)}
          />
        </label>
        <button
          type="button"
          aria-label="Open Trends week"
          disabled={busy}
          onClick={() => onReload(asOf)}
        >
          Open week
        </button>
        <button
          type="button"
          aria-label="Copy prior Trends week"
          disabled={busy || !capture.prior}
          onClick={onCopyPrior}
        >
          Copy prior
        </button>
        <span>
          {capture.periodStart} → {capture.periodEnd}
          {capture.closed ? " (closed)" : " (open)"}
        </span>
      </div>
      <p className="trends-period-caption">
        Enter source facts once. Profit / Monthly DIVS use ledger / Income Plan suggestions when left
        at suggested values. Calculated: FID+SCH {formatUsd(combined, scale)}, Total Cash{" "}
        {formatUsd(totalCash, scale)}.
      </p>
      <div className="trends-capture-grid">
        {(
          [
            ["profitMinor", "Profit (temp until IAL-only)", true],
            ["monthlyDivsMinor", "Monthly DIVS (frozen on save)", true],
            ["fidelityTotalMinor", "Total Fidelity", false],
            ["schwabTotalMinor", "Total Schwab", false],
            ["incomeCashMinor", "Income Acct cash", false],
            ["acct9CashMinor", "Acct 9 cash", false],
            ["acct9EtfValueMinor", "Acct 9 70% ETF", false],
            ["carBalanceMinor", "Car balance", false],
            ["incomeBalanceMinor", "Income balance", false],
            ["healthBalanceMinor", "Health balance", false],
            ["rothBalanceMinor", "Roth balance", false],
            ["speculationBalanceMinor", "Speculation balance", false],
          ] as Array<[keyof Draft, string, boolean]>
        ).map(([key, label, suggested]) => (
          <label key={key}>
            {label}
            <input
              aria-label={label}
              inputMode="decimal"
              value={draft[key]}
              disabled={capture.closed && !dirty}
              onChange={(e) => setField(key, e.target.value)}
              placeholder={suggested ? "suggested" : ""}
            />
          </label>
        ))}
      </div>
      <div className="buttons">
        <button
          type="button"
          aria-label="Save Trends week"
          disabled={busy || capture.closed}
          onClick={() => void onSave(buildBody(), false)}
        >
          Save week
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
