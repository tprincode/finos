import { useEffect, useState } from "react";
import { formatUsd } from "@finos/ui-components";
import type {
  CashElementHistoryGet,
  CashElementRecord,
} from "@finos/app-contracts";
import { LocalTauriFinanceClient } from "../../financeClient";

const historyClient = new LocalTauriFinanceClient();

const ELEMENT_HISTORY_DURATIONS = [
  { value: "all", label: "All" },
  { value: "ytd", label: "YTD" },
  { value: "6m", label: "6 months" },
  { value: "3m", label: "3 months" },
  { value: "1m", label: "1 month" },
] as const;

function money(minor: number | null | undefined) {
  return minor == null ? "—" : formatUsd(minor, 2);
}

function optionLabel(el: CashElementRecord, asOf: string) {
  const name = el.note || el.kind;
  const retired =
    Boolean(el.stopOn) && el.stopOn! < asOf ? " (retired)" : "";
  return `${el.account} · ${name}${retired}`;
}

export function CashElementHistory({
  elements,
  asOfDate,
}: {
  elements: CashElementRecord[];
  asOfDate: string;
}) {
  const [elementId, setElementId] = useState("");
  const [duration, setDuration] = useState("ytd");
  const [history, setHistory] = useState<CashElementHistoryGet | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    if (elementId && !elements.some((el) => el.elementId === elementId)) {
      setElementId("");
      setHistory(null);
    }
  }, [elements, elementId]);

  useEffect(() => {
    if (!elementId) {
      setHistory(null);
      setError("");
      return;
    }
    let alive = true;
    setBusy(true);
    void historyClient
      .executeQuery("CashElementHistoryGet", {
        elementId,
        duration,
        asOfDate,
      })
      .then((result) => {
        if (!alive) return;
        if (result.ok && result.bodyJson) {
          setHistory(JSON.parse(result.bodyJson) as CashElementHistoryGet);
          setError("");
        } else {
          setHistory(null);
          setError(result.errorCode ?? "error");
        }
      })
      .catch((err: unknown) => {
        if (!alive) return;
        setHistory(null);
        setError(String(err));
      })
      .finally(() => {
        if (alive) setBusy(false);
      });
    return () => {
      alive = false;
    };
  }, [elementId, duration, asOfDate]);

  const choices = [...elements].sort((a, b) => {
    const left = `${a.account} ${a.note || a.kind}`;
    const right = `${b.account} ${b.note || b.kind}`;
    return left.localeCompare(right);
  });
  const windowLabel =
    history && (history.periodStart || history.periodEnd)
      ? `${history.periodStart || "start"} – ${history.periodEnd || "open"}`
      : history
        ? "All dates"
        : "";
  const totals = history?.totals;

  return (
    <section className="element-history" aria-label="Element history">
      <div className="element-history-break" aria-hidden="true" />
      <p className="element-history-kicker">Element history</p>
      <h3>Transaction register</h3>
      <div className="element-history-controls">
        <label>
          Element
          <select
            aria-label="History element"
            value={elementId}
            disabled={busy}
            onChange={(event) => setElementId(event.target.value)}
          >
            <option value="">Select an element</option>
            {choices.map((el) => (
              <option key={el.elementId} value={el.elementId}>
                {optionLabel(el, asOfDate)}
              </option>
            ))}
          </select>
        </label>
        <label>
          Duration
          <select
            aria-label="History duration"
            value={duration}
            disabled={busy || !elementId}
            onChange={(event) => setDuration(event.target.value)}
          >
            {ELEMENT_HISTORY_DURATIONS.map((row) => (
              <option key={row.value} value={row.value}>
                {row.label}
              </option>
            ))}
          </select>
        </label>
      </div>
      {!elementId ? (
        <p>Pick a current or retired element to load its transactions.</p>
      ) : error ? (
        <p>Element history failed: {error}</p>
      ) : !history ? (
        <p>{busy ? "Loading register…" : "No register for this element."}</p>
      ) : (
        <>
          <p className="element-history-meta">
            {history.elementName} · {history.account} · {history.kind}
            {history.retired ? " · retired" : ""} · {history.cadence}
            {windowLabel ? ` · ${windowLabel}` : ""}
          </p>
          <div className="table-wrap">
            <table aria-label="Element transaction register">
              <thead>
                <tr>
                  <th>Date</th>
                  <th>Element</th>
                  <th>Account</th>
                  <th>Status</th>
                  <th>Side</th>
                  <th className="numeric">Amount</th>
                  <th>Kind</th>
                  <th>Cadence</th>
                  <th>Activity</th>
                  <th>Posted key</th>
                  <th>Exception</th>
                </tr>
              </thead>
              <tbody>
                {history.rows.length === 0 ? (
                  <tr>
                    <td colSpan={11}>No transactions in this duration.</td>
                  </tr>
                ) : (
                  history.rows.map((row) => (
                    <tr key={row.occurrenceId}>
                      <td>{row.occurredOn}</td>
                      <td>{row.elementName}</td>
                      <td>{row.account}</td>
                      <td>{row.status}</td>
                      <td
                        className={
                          row.side === "Debit"
                            ? "element-history-debit"
                            : "element-history-credit"
                        }
                      >
                        {row.side}
                      </td>
                      <td className="numeric">{money(row.amountMinor)}</td>
                      <td>{row.kind}</td>
                      <td>{row.cadence}</td>
                      <td>{row.activityType || "—"}</td>
                      <td>{row.postedKey || "—"}</td>
                      <td>{row.isException ? "Yes" : "—"}</td>
                    </tr>
                  ))
                )}
              </tbody>
            </table>
          </div>
          {totals ? (
            <p className="element-history-totals" aria-label="Element history totals">
              Actual {totals.actualCount} · Debits {money(totals.actualDebitMinor)} ·
              Credits {money(totals.actualCreditMinor)} · Net {money(totals.actualNetMinor)}
              {totals.plannedCount
                ? ` · Planned ${totals.plannedCount} · Debits ${money(totals.plannedDebitMinor)} · Credits ${money(totals.plannedCreditMinor)}`
                : ""}
            </p>
          ) : null}
        </>
      )}
    </section>
  );
}
