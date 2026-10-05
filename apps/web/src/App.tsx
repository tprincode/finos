import { useCallback, useState } from "react";
import type {
  MobileHead,
  MobileOutboxPut,
  WeekAheadRow,
  TaskRecord,
} from "@finos/app-contracts";
import { RemoteHttpFinanceClient } from "./financeClient";
import "./App.css";

type MagiSummary = {
  applicableThresholdMinor?: number;
  baseForecastMinor?: number;
  actualIncludedYtdMinor?: number;
  overageMinor?: number;
  decisionState?: string;
  scale?: number;
};

function formatUsd(minor: number, scale = 2): string {
  const neg = minor < 0;
  const abs = Math.abs(minor);
  const div = 10 ** scale;
  const whole = Math.floor(abs / div);
  const frac = String(abs % div).padStart(scale, "0");
  return `${neg ? "-" : ""}$${whole.toLocaleString("en-US")}.${frac}`;
}

export default function App() {
  const [baseUrl, setBaseUrl] = useState("http://127.0.0.1:8787");
  const [accessToken, setAccessToken] = useState("");
  const [headUrl, setHeadUrl] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [status, setStatus] = useState<string | null>(null);
  const [head, setHead] = useState<MobileHead | null>(null);
  const [pendingId, setPendingId] = useState<string | null>(null);

  const client = () => new RemoteHttpFinanceClient(baseUrl, accessToken);

  const loadFromQuery = useCallback(async () => {
    setError(null);
    setStatus(null);
    try {
      const result = await client().executeQuery("MobileHeadGet");
      if (!result.ok || !result.bodyJson) {
        throw new Error(result.errorCode ?? "MobileHeadGet failed");
      }
      setHead(JSON.parse(result.bodyJson) as MobileHead);
      setStatus("Loaded published head from server / primary desktop publish folder.");
    } catch (err) {
      setHead(null);
      setError(err instanceof Error ? err.message : String(err));
    }
  }, [accessToken, baseUrl]);

  const loadFromUrl = useCallback(async () => {
    setError(null);
    setStatus(null);
    try {
      if (!headUrl.trim()) {
        throw new Error("Paste a head.json URL (cloud-synced publish folder) or use Load from server.");
      }
      const res = await fetch(headUrl.trim());
      if (!res.ok) {
        throw new Error(`head.json HTTP ${res.status}`);
      }
      setHead((await res.json()) as MobileHead);
      setStatus("Loaded published head from URL.");
    } catch (err) {
      setHead(null);
      setError(err instanceof Error ? err.message : String(err));
    }
  }, [headUrl]);

  const confirmRow = useCallback(
    async (row: WeekAheadRow) => {
      setPendingId(row.occurrenceId);
      setError(null);
      setStatus(null);
      try {
        const result = await client().executeCommand("MobileOutboxPut", {
          occurrenceId: row.occurrenceId,
          asOfDate: head?.asOf,
          note: "phone week ahead confirm",
        });
        if (!result.ok || !result.bodyJson) {
          throw new Error(result.errorCode ?? "MobileOutboxPut failed");
        }
        const body = JSON.parse(result.bodyJson) as MobileOutboxPut;
        setStatus(
          `Queued confirm for ${row.transaction || row.note || row.occurrenceId}. Primary desktop Apply mobile outbox (intent ${body.intentId.slice(0, 8)}…).`,
        );
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
      } finally {
        setPendingId(null);
      }
    },
    [accessToken, baseUrl, head?.asOf],
  );

  let magi: MagiSummary = {};
  if (head?.magiJson) {
    try {
      magi = JSON.parse(head.magiJson) as MagiSummary;
    } catch {
      magi = {};
    }
  }
  const scale = magi.scale ?? head?.weekAhead?.scale ?? 2;
  const rows = head?.weekAhead?.rows ?? [];
  const tasks: TaskRecord[] = head?.openTasks ?? [];

  return (
    <main className="container phone">
      <h1>finos mobile</h1>
      <p>
        Read-only Week Ahead, open tasks, and MAGI from the primary desktop publish.
        Point <code>FINOS_MOBILE_PUBLISH_DIR</code> at a cloud sync folder on the PC. Confirm
        queues an outbox intent — the desktop applies it.
      </p>

      <label>
        Server
        <input value={baseUrl} onChange={(e) => setBaseUrl(e.target.value)} />
      </label>
      <label>
        Bearer token
        <input
          value={accessToken}
          onChange={(e) => setAccessToken(e.target.value)}
          type="password"
        />
      </label>
      <label>
        head.json URL (optional)
        <input
          value={headUrl}
          onChange={(e) => setHeadUrl(e.target.value)}
          placeholder="https://…/mobile-publish/head.json"
        />
      </label>
      <div className="buttons">
        <button type="button" onClick={() => void loadFromQuery()}>
          Load from server
        </button>
        <button type="button" onClick={() => void loadFromUrl()}>
          Load from URL
        </button>
      </div>

      {error ? <p className="error">{error}</p> : null}
      {status ? <p role="status">{status}</p> : null}

      {head ? (
        <>
          <p className="meta">
            As of {head.asOf} · published {head.publishedAt}
          </p>

          <section aria-label="Mobile MAGI summary">
            <h2>MAGI</h2>
            <dl>
              <dt>Forecast</dt>
              <dd>{formatUsd(magi.baseForecastMinor ?? 0, scale)}</dd>
              <dt>Cliff</dt>
              <dd>{formatUsd(magi.applicableThresholdMinor ?? 0, scale)}</dd>
              <dt>Overage</dt>
              <dd>{formatUsd(magi.overageMinor ?? 0, scale)}</dd>
              <dt>State</dt>
              <dd>{String(magi.decisionState ?? "—")}</dd>
            </dl>
          </section>

          <section aria-label="Mobile open tasks">
            <h2>Open tasks · {tasks.length}</h2>
            {tasks.length === 0 ? (
              <p>None</p>
            ) : (
              <ul>
                {tasks.map((t) => (
                  <li key={t.taskId}>
                    {t.title} <span className="due">due {t.dueOn}</span>
                  </li>
                ))}
              </ul>
            )}
          </section>

          <section aria-label="Mobile week ahead">
            <h2>
              Week Ahead · {head.weekAhead.periodStart} – {head.weekAhead.periodEnd}
            </h2>
            {rows.length === 0 ? (
              <p>No open charges this week.</p>
            ) : (
              <ul className="week-ahead">
                {rows.map((row) => (
                  <li key={row.occurrenceId}>
                    <div>
                      <strong>{row.transaction || row.note}</strong>
                      <span>
                        {" "}
                        {row.account} · {row.occurredOn} ·{" "}
                        {formatUsd(row.amountMinor, row.scale ?? scale)}
                      </span>
                    </div>
                    <button
                      type="button"
                      disabled={pendingId === row.occurrenceId}
                      onClick={() => void confirmRow(row)}
                    >
                      Confirm
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </section>
        </>
      ) : null}
    </main>
  );
}
