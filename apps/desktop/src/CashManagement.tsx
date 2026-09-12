import { formatUsd } from "@finos/ui-components";
import type {
  AccountListItem,
  CashManagementMonthGet,
  CashManagementRemindersGet,
  CashManagementWeekGet,
  MagiProjection,
} from "@finos/app-contracts";
import { useEffect, useState } from "react";

const TYPES = [
  { value: "IRA_Distribution", label: "IRA distribution" },
  { value: "Roth_Distribution", label: "Roth distribution" },
];

function dollarsToMinor(raw: string, scale: number): number | null {
  const t = raw.trim();
  if (!t) return null;
  const n = Number(t);
  if (!Number.isFinite(n)) return null;
  return Math.round(n * 10 ** scale);
}

function magiAddMinor(
  activityType: string,
  grossMinor: number | null,
): number | null {
  if (activityType === "IRA_Distribution") return grossMinor;
  if (activityType === "Roth_Distribution") return 0;
  return null;
}

export function CashManagementPanel({
  week,
  month,
  reminders,
  magi,
  accounts,
  busy,
  onReload,
  onSave,
  onSsaConfirm,
  onDirtyChange,
}: {
  week: CashManagementWeekGet | null;
  month: CashManagementMonthGet | null;
  reminders: CashManagementRemindersGet | null;
  magi: MagiProjection | null;
  accounts: AccountListItem[];
  busy?: boolean;
  onReload: (asOf: string) => void;
  onSave: (body: Record<string, unknown>) => Promise<boolean>;
  onSsaConfirm: (body: Record<string, unknown>) => Promise<boolean>;
  onDirtyChange?: (dirty: boolean) => void;
}) {
  const [asOf, setAsOf] = useState(week?.periodEnd ?? "");
  const [accountId, setAccountId] = useState("");
  const [activityType, setActivityType] = useState("IRA_Distribution");
  const [occurredOn, setOccurredOn] = useState(week?.periodEnd ?? "");
  const [gross, setGross] = useState("");
  const [fed, setFed] = useState("0.00");
  const [state, setState] = useState("0.00");
  const [dirty, setDirty] = useState(false);
  const [ssaAccountId, setSsaAccountId] = useState("");
  const [ssaOccurredOn, setSsaOccurredOn] = useState("");
  const [ssaReceived, setSsaReceived] = useState("");
  const [ssaDirty, setSsaDirty] = useState(false);

  useEffect(() => {
    if (week) {
      setAsOf(week.periodEnd);
      if (!dirty) {
        setOccurredOn(
          reminders?.saturdayDraft.open
            ? reminders.saturdayDraft.occurredOn
            : week.periodEnd,
        );
      }
    }
  }, [week, reminders, dirty]);

  useEffect(() => {
    if (!reminders || dirty) {
      return;
    }
    if (reminders.saturdayDraft.open) {
      setActivityType(reminders.saturdayDraft.activityType);
      if (reminders.saturdayDraft.suggestedAccountId) {
        setAccountId(reminders.saturdayDraft.suggestedAccountId);
      }
      setOccurredOn(reminders.saturdayDraft.occurredOn);
    }
  }, [reminders, dirty]);

  useEffect(() => {
    if (!reminders || ssaDirty) {
      return;
    }
    if (reminders.tomSsa.suggestedAccountId) {
      setSsaAccountId(reminders.tomSsa.suggestedAccountId);
    }
    if (!ssaOccurredOn) {
      setSsaOccurredOn(reminders.asOfDate);
    }
  }, [reminders, ssaDirty, ssaOccurredOn]);

  useEffect(() => {
    onDirtyChange?.(dirty || ssaDirty);
    return () => onDirtyChange?.(false);
  }, [dirty, ssaDirty, onDirtyChange]);

  const scale = week?.scale ?? reminders?.scale ?? 2;
  const grossMinor = dollarsToMinor(gross, scale);
  const fedMinor = dollarsToMinor(fed, scale) ?? 0;
  const stateMinor = dollarsToMinor(state, scale) ?? 0;
  const netMinor =
    grossMinor == null ? null : grossMinor - fedMinor - stateMinor;
  const identityOk = netMinor != null && netMinor >= 0;
  const rothBlocked =
    activityType === "Roth_Distribution" && (fedMinor !== 0 || stateMinor !== 0);
  const ssaReceivedMinor = dollarsToMinor(ssaReceived, scale);
  const tomExpected = reminders?.tomSsa.expectedMinor ?? 286500;
  const ssaVariance =
    ssaReceivedMinor != null && ssaReceivedMinor !== tomExpected;
  const magiAdd = magiAddMinor(activityType, grossMinor);
  const taxPaymentCredit = fedMinor + stateMinor;

  const mark = (fn: () => void) => {
    fn();
    setDirty(true);
  };

  const markSsa = (fn: () => void) => {
    fn();
    setSsaDirty(true);
  };

  const reset = () => {
    setGross("");
    setFed("0.00");
    setState("0.00");
    setActivityType(
      reminders?.saturdayDraft.activityType ?? "IRA_Distribution",
    );
    setOccurredOn(
      reminders?.saturdayDraft.occurredOn ?? week?.periodEnd ?? occurredOn,
    );
    if (reminders?.saturdayDraft.suggestedAccountId) {
      setAccountId(reminders.saturdayDraft.suggestedAccountId);
    }
    setDirty(false);
  };

  const resetSsa = () => {
    setSsaReceived("");
    setSsaOccurredOn(reminders?.asOfDate ?? ssaOccurredOn);
    if (reminders?.tomSsa.suggestedAccountId) {
      setSsaAccountId(reminders.tomSsa.suggestedAccountId);
    }
    setSsaDirty(false);
  };

  if (!week) {
    return <p>Loading Cash Management…</p>;
  }

  return (
    <div className="cash-management" aria-label="Cash Management">
      <div className="trends-period-bar">
        <label className="trends-period-label">
          Week
          <input
            type="date"
            aria-label="Cash management as-of date"
            value={asOf}
            onChange={(e) => setAsOf(e.target.value)}
          />
        </label>
        <button
          type="button"
          aria-label="Open cash management week"
          disabled={busy}
          onClick={() => onReload(asOf)}
        >
          Open week
        </button>
        <span>
          {week.periodStart} to {week.periodEnd}
        </span>
      </div>
      {reminders?.saturdayDraft.open ? (
        <p className="trends-period-caption" aria-label="Saturday income draft">
          Saturday planned draft: Income IRA for{" "}
          {reminders.saturdayDraft.suggestedAccountName || "Income"} on{" "}
          {reminders.saturdayDraft.occurredOn}. Gross stays blank until entered.
        </p>
      ) : (
        <p className="trends-period-caption">
          Select account, enter gross, then federal and state withholding. Net
          must equal gross minus withholding. Gross includes withholding.
        </p>
      )}
      <div className="trends-capture-grid">
        <label>
          Type
          <select
            aria-label="Distribution type"
            value={activityType}
            onChange={(e) => mark(() => setActivityType(e.target.value))}
          >
            {TYPES.map((t) => (
              <option key={t.value} value={t.value}>
                {t.label}
              </option>
            ))}
          </select>
        </label>
        <label>
          Account
          <select
            aria-label="Distribution account"
            value={accountId}
            onChange={(e) => mark(() => setAccountId(e.target.value))}
          >
            <option value="">Select account</option>
            {accounts.map((a) => (
              <option key={a.accountId} value={a.accountId}>
                {a.name}
              </option>
            ))}
          </select>
        </label>
        <label>
          Date
          <input
            type="date"
            aria-label="Distribution date"
            value={occurredOn}
            onChange={(e) => mark(() => setOccurredOn(e.target.value))}
          />
        </label>
        <label>
          Gross
          <input
            aria-label="Distribution gross"
            inputMode="decimal"
            value={gross}
            onChange={(e) => mark(() => setGross(e.target.value))}
          />
        </label>
        <label>
          Federal withholding
          <input
            aria-label="Federal withholding"
            inputMode="decimal"
            value={fed}
            onChange={(e) => mark(() => setFed(e.target.value))}
          />
        </label>
        <label>
          State withholding
          <input
            aria-label="State withholding"
            inputMode="decimal"
            value={state}
            onChange={(e) => mark(() => setState(e.target.value))}
          />
        </label>
      </div>
      <p>
        Net {netMinor == null ? "—" : formatUsd(netMinor, scale)}
        {identityOk ? "" : " — net cannot be negative"}
        {rothBlocked ? " — Roth cannot have withholding" : ""}
      </p>
      <p aria-label="Cash MAGI preview">
        MAGI add{" "}
        {magiAdd == null ? "unknown (year-level)" : formatUsd(magiAdd, scale)}{" "}
        from taxable gross. Withholding {formatUsd(taxPaymentCredit, scale)}{" "}
        changes tax-payment, not Marketplace MAGI.
        {magi ? ` Current MAGI ${magi.decisionState}.` : ""}
      </p>
      <div className="buttons">
        <button
          type="button"
          aria-label="Save cash distribution"
          className={dirty ? "is-unsaved" : undefined}
          disabled={
            busy ||
            !accountId ||
            grossMinor == null ||
            !identityOk ||
            rothBlocked
          }
          onClick={() => {
            void onSave({
              accountId,
              activityType,
              occurredOn,
              grossMinor,
              federalWithholdingMinor: fedMinor,
              stateWithholdingMinor: stateMinor,
              scale,
            }).then((ok) => {
              if (ok) reset();
            });
          }}
        >
          Save
        </button>
        <button
          type="button"
          aria-label="Cancel cash distribution"
          disabled={!dirty}
          onClick={reset}
        >
          Cancel
        </button>
      </div>
      <h3>Confirm Tom Social Security retirement</h3>
      <p>
        Expected {formatUsd(tomExpected, scale)} each month. Label is Social
        Security retirement. A missed month stays unknown, never $0.
        {reminders?.tomSsa.extraAudit
          ? " Extra $2,865 in the same month is audit, not next month."
          : ""}
      </p>
      <p>
        {reminders
          ? `${reminders.tomSsa.yearMonth} is ${reminders.tomSsa.status}${
              reminders.tomSsa.postedMinor == null
                ? ""
                : ` at ${formatUsd(reminders.tomSsa.postedMinor, scale)}`
            }.`
          : "Loading Social Security retirement…"}
      </p>
      <div className="trends-capture-grid">
        <label>
          Account
          <select
            aria-label="Tom SSA account"
            value={ssaAccountId}
            onChange={(e) => markSsa(() => setSsaAccountId(e.target.value))}
          >
            <option value="">Select account</option>
            {accounts.map((a) => (
              <option key={a.accountId} value={a.accountId}>
                {a.name}
              </option>
            ))}
          </select>
        </label>
        <label>
          Date
          <input
            type="date"
            aria-label="Tom SSA date"
            value={ssaOccurredOn}
            onChange={(e) => markSsa(() => setSsaOccurredOn(e.target.value))}
          />
        </label>
        <label>
          Received
          <input
            aria-label="Tom SSA received"
            inputMode="decimal"
            value={ssaReceived}
            onChange={(e) => markSsa(() => setSsaReceived(e.target.value))}
          />
        </label>
      </div>
      <p>
        {ssaReceivedMinor == null
          ? "Received stays blank until entered."
          : ssaVariance
            ? `Variance ${formatUsd(ssaReceivedMinor - tomExpected, scale)} — exception stays open.`
            : "Matches expected."}
      </p>
      <div className="buttons">
        <button
          type="button"
          aria-label="Confirm Tom Social Security retirement"
          className={ssaDirty ? "is-unsaved" : undefined}
          disabled={busy || !ssaAccountId || ssaReceivedMinor == null}
          onClick={() => {
            void onSsaConfirm({
              accountId: ssaAccountId,
              occurredOn: ssaOccurredOn,
              receivedMinor: ssaReceivedMinor,
              scale,
            }).then((ok) => {
              if (ok) resetSsa();
            });
          }}
        >
          Confirm
        </button>
        <button
          type="button"
          aria-label="Cancel Tom SSA confirm"
          disabled={!ssaDirty}
          onClick={resetSsa}
        >
          Cancel
        </button>
      </div>
      {reminders && reminders.tomSsa.recent.length > 0 ? (
        <div className="table-wrap">
          <table aria-label="Tom Social Security retirement history">
            <thead>
              <tr>
                <th>Paid</th>
                <th>Amount</th>
                <th>Account</th>
                <th>Note</th>
              </tr>
            </thead>
            <tbody>
              {reminders.tomSsa.recent.map((row) => (
                <tr key={`${row.occurredOn}-${row.amountMinor}`}>
                  <td>{row.occurredOn}</td>
                  <td className="numeric">
                    {formatUsd(row.amountMinor, scale)}
                  </td>
                  <td>{row.accountName}</td>
                  <td>{row.extraAudit ? "extra — audit" : "match"}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      ) : null}
      <h3>This week</h3>
      <p>
        Gross {formatUsd(week.weekGrossMinor, scale)}. Withholding{" "}
        {formatUsd(week.weekWithholdingMinor, scale)}. Net{" "}
        {formatUsd(week.weekNetMinor, scale)}.
      </p>
      <div className="table-wrap">
        <table aria-label="Cash management week">
          <thead>
            <tr>
              <th>Date</th>
              <th>Account</th>
              <th>Type</th>
              <th>Gross</th>
              <th>Fed WH</th>
              <th>State WH</th>
              <th>Net</th>
            </tr>
          </thead>
          <tbody>
            {week.rows.map((row) => (
              <tr key={row.activityId}>
                <td>{row.occurredOn}</td>
                <td>{row.accountName}</td>
                <td>{row.activityType}</td>
                <td className="numeric">{formatUsd(row.grossMinor, row.scale)}</td>
                <td className="numeric">
                  {formatUsd(row.federalWithholdingMinor, row.scale)}
                </td>
                <td className="numeric">
                  {formatUsd(row.stateWithholdingMinor, row.scale)}
                </td>
                <td className="numeric">{formatUsd(row.netMinor, row.scale)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {week.rows.length === 0 ? (
        <p>No non-ROI cash events in this week yet.</p>
      ) : null}
      {month ? (
        <>
          <h3>This month ({month.yearMonth})</h3>
          <p>
            Calendar month uses event date, not a sum of week cells.{" "}
            {month.periodStart} to {month.periodEnd}. Gross{" "}
            {formatUsd(month.monthGrossMinor, scale)}. Withholding{" "}
            {formatUsd(month.monthWithholdingMinor, scale)}. Net{" "}
            {formatUsd(month.monthNetMinor, scale)}.
          </p>
          <div className="table-wrap">
            <table aria-label="Cash management month">
              <thead>
                <tr>
                  <th>Account</th>
                  <th>Type</th>
                  <th>Count</th>
                  <th>Gross</th>
                  <th>Fed WH</th>
                  <th>State WH</th>
                  <th>Net</th>
                </tr>
              </thead>
              <tbody>
                {month.rows.map((row) => (
                  <tr key={`${row.accountId}-${row.activityType}`}>
                    <td>{row.accountName}</td>
                    <td>{row.activityType}</td>
                    <td className="numeric">{row.count}</td>
                    <td className="numeric">
                      {formatUsd(row.grossMinor, row.scale)}
                    </td>
                    <td className="numeric">
                      {formatUsd(row.federalWithholdingMinor, row.scale)}
                    </td>
                    <td className="numeric">
                      {formatUsd(row.stateWithholdingMinor, row.scale)}
                    </td>
                    <td className="numeric">
                      {formatUsd(row.netMinor, row.scale)}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </>
      ) : null}
    </div>
  );
}
