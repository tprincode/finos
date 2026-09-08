import { useEffect, useMemo, useState } from "react";
import {
  type HandoffStatus,
  type ImportBatchRecord,
  type ImportCandidate,
} from "@finos/app-contracts";
import { formatUsd } from "@finos/ui-components";
import { LocalTauriFinanceClient } from "./financeClient";
import "./App.css";

const client = new LocalTauriFinanceClient();

function notesLabel(activityType: string): string {
  const t = activityType.trim().toLowerCase();
  if (t === "dividend") return "Dividend";
  if (t === "drip") return "DRIP";
  if (t === "interest") return "Interest";
  if (t === "roc") return "ROC";
  return activityType.trim() || "—";
}

function statusLabel(row: ImportCandidate): string {
  if (row.validation === "blocked") return row.issue ? `Cannot load (${row.issue})` : "Cannot load";
  if (row.validation === "duplicate") return "Already in Finos";
  return "Ready";
}

function amountText(row: ImportCandidate): string {
  if (row.amountMinor == null) return "unknown";
  return formatUsd(row.amountMinor, row.scale ?? 2);
}

async function closeWizardWindow() {
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().close();
  } catch {
    window.close();
  }
}

export function ImportWizard() {
  const params = new URLSearchParams(window.location.search);
  const batchId = params.get("batchId") ?? "";
  const filenameHint = params.get("filename") ?? "";
  const [step, setStep] = useState<"imported" | "validate" | "result">("imported");
  const [batch, setBatch] = useState<ImportBatchRecord | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [writesAllowed, setWritesAllowed] = useState(true);
  const [resultLines, setResultLines] = useState<string[]>([]);
  const [resultSummary, setResultSummary] = useState("");

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const handoff = await client.executeQuery("HandoffStatusGet");
        if (!cancelled && handoff.ok && handoff.bodyJson) {
          const body = JSON.parse(handoff.bodyJson) as HandoffStatus;
          setWritesAllowed(body.writesAllowed);
        }
      } catch {
        /* keep allowed */
      }
      if (!batchId) {
        setError("No import batch. Close this window and choose a CSV on Import.");
        return;
      }
      const result = await client.executeQuery("ImportBatchGet", { batchId });
      if (cancelled) return;
      if (!result.ok || !result.bodyJson) {
        setError(`Could not load import: ${result.errorCode ?? "error"}`);
        return;
      }
      let body = JSON.parse(result.bodyJson) as ImportBatchRecord;
      if (body.status === "staged") {
        const validated = await client.executeCommand("ImportValidate", { batchId });
        if (validated.ok) {
          const again = await client.executeQuery("ImportBatchGet", { batchId });
          if (again.ok && again.bodyJson) {
            body = JSON.parse(again.bodyJson) as ImportBatchRecord;
          }
        }
      }
      setBatch(body);
    })();
    return () => {
      cancelled = true;
    };
  }, [batchId]);

  const rows = batch?.candidates ?? [];
  const filename = batch?.filename?.trim() || filenameHint || "CSV";
  const counts = useMemo(() => {
    let ready = 0;
    let duplicate = 0;
    let blocked = 0;
    for (const row of rows) {
      if (row.validation === "blocked") blocked += 1;
      else if (row.validation === "duplicate") duplicate += 1;
      else ready += 1;
    }
    return { ready, duplicate, blocked };
  }, [rows]);
  const loadOk = counts.blocked === 0 && writesAllowed && !busy && Boolean(batchId);

  const load = async () => {
    if (!batchId || !loadOk) return;
    setBusy(true);
    setError(null);
    try {
      const approved = await client.executeCommand("ImportApprove", { batchId });
      if (!approved.ok) {
        setError(`Could not load: ${approved.errorCode ?? "error"}`);
        return;
      }
      const posted = await client.executeCommand("ImportPost", { batchId });
      let body: Partial<ImportBatchRecord> = {};
      if (posted.bodyJson) {
        try {
          body = JSON.parse(posted.bodyJson) as ImportBatchRecord;
        } catch {
          body = {};
        }
      }
      if (!posted.ok) {
        setError(`Could not load: ${posted.errorCode ?? "error"}`);
        return;
      }
      const postedN = body.postedCount ?? 0;
      const skipped = body.skippedDuplicateCount ?? 0;
      const errors = body.errorCount ?? 0;
      setResultLines(body.processLines ?? []);
      setResultSummary(
        `Loaded ${postedN}, already in Finos ${skipped}, ${errors} error${errors === 1 ? "" : "s"}.`,
      );
      setStep("result");
    } catch (err: unknown) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  };

  return (
    <main className="import-wizard" aria-label="Import wizard">
      <h1>Import</h1>
      {error ? (
        <p role="alert">{error}</p>
      ) : null}

      {step === "imported" ? (
        <section aria-label="Import step">
          <h2>1. Import</h2>
          <p>
            {batch
              ? `${rows.length} transaction${rows.length === 1 ? "" : "s"} read from ${filename}.`
              : `Reading ${filename}…`}
          </p>
          <p>Nothing has been written to the household yet.</p>
          <div className="buttons">
            <button
              type="button"
              aria-label="Continue to validate"
              disabled={!batch || busy}
              onClick={() => setStep("validate")}
            >
              Continue to validate
            </button>
            <button type="button" aria-label="Cancel" onClick={() => void closeWizardWindow()}>
              Cancel
            </button>
          </div>
        </section>
      ) : null}

      {step === "validate" ? (
        <section aria-label="Validate step">
          <h2>2. Validate</h2>
          <p>
            {filename}: {counts.ready} ready, {counts.duplicate} already in Finos,{" "}
            {counts.blocked} cannot load. Qty is 1. Amount is the cash total.
          </p>
          {counts.blocked > 0 ? (
            <p role="status">
              Load stays off until every row is Ready or Already in Finos. Unknown amounts stay
              unknown.
            </p>
          ) : null}
          <div className="table-wrap">
            <table aria-label="Import transactions">
              <thead>
                <tr>
                  <th scope="col">Account</th>
                  <th scope="col">Symbol</th>
                  <th scope="col">Notes</th>
                  <th scope="col">Date</th>
                  <th scope="col">Qty</th>
                  <th scope="col">Amount</th>
                  <th scope="col">Status</th>
                </tr>
              </thead>
              <tbody>
                {rows.map((row, index) => (
                  <tr key={row.candidateId || `${row.accountName}-${row.symbol}-${row.occurredOn}-${index}`}>
                    <td>{row.accountName}</td>
                    <td>{row.symbol?.trim() || "—"}</td>
                    <td>{notesLabel(row.activityType)}</td>
                    <td>{row.occurredOn}</td>
                    <td className="numeric">1</td>
                    <td className="numeric">{amountText(row)}</td>
                    <td>{statusLabel(row)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <div className="buttons">
            <button
              type="button"
              aria-label="Load"
              disabled={!loadOk}
              onClick={() => void load()}
            >
              {busy ? "Loading…" : "Load"}
            </button>
            <button
              type="button"
              aria-label="Cancel"
              disabled={busy}
              onClick={() => void closeWizardWindow()}
            >
              Cancel
            </button>
          </div>
        </section>
      ) : null}

      {step === "result" ? (
        <section aria-label="Import result">
          <h2>Loaded</h2>
          <p role="status">{resultSummary}</p>
          {resultLines.length > 0 ? (
            <pre aria-label="Import result lines">{resultLines.join("\n")}</pre>
          ) : null}
          <div className="buttons">
            <button type="button" aria-label="Close import" onClick={() => void closeWizardWindow()}>
              Close
            </button>
          </div>
        </section>
      ) : null}
    </main>
  );
}

export async function openImportWizardWindow(batchId: string, filename: string) {
  const qs = new URLSearchParams({
    wizard: "import",
    batchId,
    filename,
  });
  const url = `/?${qs.toString()}`;
  try {
    const { WebviewWindow } = await import("@tauri-apps/api/webviewWindow");
    const existing = await WebviewWindow.getByLabel("import-wizard");
    if (existing) {
      await existing.close();
    }
    new WebviewWindow("import-wizard", {
      url,
      title: "Import",
      width: 1100,
      height: 740,
      focus: true,
    });
  } catch {
    window.open(url, "import-wizard");
  }
}
