import { useCallback, useEffect, useState } from "react";
import {
  FINANCE_CLIENT_CONTRACT_VERSION,
  type BasisGet,
  type BrokerLotReconcileGet,
  type DashboardGet,
  type DividendGet,
  type HandoffStatus,
  type IncomePlanGet,
  type MagiProjection,
  type PlanGet,
  type BurndownGet,
  type AllocationGet,
  type CartGet,
  type BacktestGet,
  type ClassificationReviewGet,
  type AnalysisRunList,
  type PositionDetailsGet,
  type TaxProjectionGet,
  type RoiGet,
  type TrendsGet,
} from "@finos/app-contracts";
import { invoke } from "@tauri-apps/api/core";
import { check } from "@tauri-apps/plugin-updater";
import { LocalTauriFinanceClient } from "./financeClient";
import "./App.css";

const client = new LocalTauriFinanceClient();

type HealthView = {
  ok: boolean;
  status: string;
  contractVersion: string;
  error?: string;
};

function parseHandoff(bodyJson?: string): HandoffStatus | null {
  if (!bodyJson) return null;
  try {
    return JSON.parse(bodyJson) as HandoffStatus;
  } catch {
    return null;
  }
}

export default function App() {
  const [health, setHealth] = useState<HealthView | null>(null);
  const [handoff, setHandoff] = useState<HandoffStatus | null>(null);
  const [handoffError, setHandoffError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [actionMessage, setActionMessage] = useState<string | null>(null);
  const [deviceName, setDeviceName] = useState("");
  const [week, setWeek] = useState<{ start?: string; end?: string; asOfDate?: string } | null>(
    null,
  );
  const [accounts, setAccounts] = useState<Array<{ name: string; kind: string }>>([]);
  const [exceptions, setExceptions] = useState<Array<{ code: string; message: string }>>([]);
  const [importStatus, setImportStatus] = useState<string>("none");
  const [dividend, setDividend] = useState<DividendGet | null>(null);
  const [incomePlan, setIncomePlan] = useState<IncomePlanGet | null>(null);
  const [dashboard, setDashboard] = useState<DashboardGet | null>(null);
  const [trends, setTrends] = useState<TrendsGet | null>(null);
  const [basis, setBasis] = useState<BasisGet | null>(null);
  const [roi, setRoi] = useState<RoiGet | null>(null);
  const [lotRecon, setLotRecon] = useState<BrokerLotReconcileGet | null>(null);
  const [positions, setPositions] = useState<PositionDetailsGet | null>(null);
  const [taxProjection, setTaxProjection] = useState<TaxProjectionGet | null>(null);
  const [magi, setMagi] = useState<MagiProjection | null>(null);
  const [calcPlan, setCalcPlan] = useState<PlanGet | null>(null);
  const [burndown, setBurndown] = useState<BurndownGet | null>(null);
  const [allocation, setAllocation] = useState<AllocationGet | null>(null);
  const [cart, setCart] = useState<CartGet | null>(null);
  const [backtest, setBacktest] = useState<BacktestGet | null>(null);
  const [classification, setClassification] = useState<ClassificationReviewGet | null>(null);
  const [aiRuns, setAiRuns] = useState<AnalysisRunList | null>(null);
  const [updateStatus, setUpdateStatus] = useState<string>("not checked");

  const refreshCanonical = useCallback(async () => {
    const [weekResult, accountResult, exceptionResult] = await Promise.all([
      client.executeQuery("CanonicalWeekGet", { asOfDate: "2026-08-18" }),
      client.executeQuery("AccountList"),
      client.executeQuery("ExceptionList"),
    ]);
    if (weekResult.bodyJson) {
      try {
        setWeek(JSON.parse(weekResult.bodyJson));
      } catch {
        /* ignore */
      }
    }
    if (accountResult.bodyJson) {
      try {
        setAccounts(JSON.parse(accountResult.bodyJson));
      } catch {
        setAccounts([]);
      }
    }
    if (exceptionResult.bodyJson) {
      try {
        setExceptions(JSON.parse(exceptionResult.bodyJson));
      } catch {
        setExceptions([]);
      }
    }
    const [batches, dividendResult, incomeResult, dashboardResult, trendsResult, basisResult, roiResult, reconResult, positionResult, magiResult, taxResult, planResult, burndownResult, allocationResult, cartResult, backtestResult, classificationResult, aiResult] =
      await Promise.all([
        client.executeQuery("ReconcileCountsGet"),
        client.executeQuery("DividendGet"),
        client.executeQuery("IncomePlanGet"),
        client.executeQuery("DashboardGet"),
        client.executeQuery("TrendsGet"),
        client.executeQuery("BasisGet"),
        client.executeQuery("RoiGet"),
        client.executeQuery("BrokerLotReconcileGet"),
        client.executeQuery("PositionDetailsGet"),
        client.executeQuery("MagiProjectionGet"),
        client.executeQuery("TaxProjectionGet"),
        client.executeQuery("PlanGet"),
        client.executeQuery("BurndownGet"),
        client.executeQuery("AllocationGet"),
        client.executeQuery("CartGet"),
        client.executeQuery("BacktestGet"),
        client.executeQuery("ClassificationReviewGet"),
        client.executeQuery("AnalysisRunList"),
      ]);
    if (batches.bodyJson) {
      try {
        const counts = JSON.parse(batches.bodyJson) as { importBatches?: number };
        setImportStatus(
          counts.importBatches ? `${counts.importBatches} batch(es)` : "none",
        );
      } catch {
        setImportStatus("unknown");
      }
    }
    const parseView = <T,>(raw?: string): T | null => {
      if (!raw) return null;
      try {
        return JSON.parse(raw) as T;
      } catch {
        return null;
      }
    };
    setDividend(parseView<DividendGet>(dividendResult.bodyJson));
    setIncomePlan(parseView<IncomePlanGet>(incomeResult.bodyJson));
    setDashboard(parseView<DashboardGet>(dashboardResult.bodyJson));
    setTrends(parseView<TrendsGet>(trendsResult.bodyJson));
    setBasis(parseView<BasisGet>(basisResult.bodyJson));
    setRoi(parseView<RoiGet>(roiResult.bodyJson));
    setLotRecon(parseView<BrokerLotReconcileGet>(reconResult.bodyJson));
    setPositions(parseView<PositionDetailsGet>(positionResult.bodyJson));
    setMagi(parseView<MagiProjection>(magiResult.bodyJson));
    setTaxProjection(parseView<TaxProjectionGet>(taxResult.bodyJson));
    setCalcPlan(parseView<PlanGet>(planResult.bodyJson));
    setBurndown(parseView<BurndownGet>(burndownResult.bodyJson));
    setAllocation(parseView<AllocationGet>(allocationResult.bodyJson));
    setCart(parseView<CartGet>(cartResult.bodyJson));
    setBacktest(parseView<BacktestGet>(backtestResult.bodyJson));
    setClassification(parseView<ClassificationReviewGet>(classificationResult.bodyJson));
    setAiRuns(parseView<AnalysisRunList>(aiResult.bodyJson));
  }, []);

  const refreshHandoff = useCallback(async () => {
    const result = await client.executeQuery("HandoffStatusGet");
    if (!result.ok) {
      setHandoff(null);
      setHandoffError(result.errorCode ?? "handoff_failed");
      return;
    }
    setHandoffError(null);
    setHandoff(parseHandoff(result.bodyJson ?? undefined));
  }, []);

  useEffect(() => {
    let cancelled = false;
    client
      .executeQuery("HealthGet")
      .then((result) => {
        if (cancelled) return;
        let status = "unknown";
        let contractVersion = FINANCE_CLIENT_CONTRACT_VERSION;
        if (result.bodyJson) {
          try {
            const body = JSON.parse(result.bodyJson) as {
              status?: string;
              contractVersion?: string;
            };
            status = body.status ?? status;
            contractVersion = body.contractVersion ?? contractVersion;
          } catch {
            status = "invalid-body";
          }
        }
        setHealth({
          ok: result.ok,
          status,
          contractVersion,
          error: result.errorCode,
        });
      })
      .catch((err: unknown) => {
        if (cancelled) return;
        setHealth({
          ok: false,
          status: "invoke-failed",
          contractVersion: FINANCE_CLIENT_CONTRACT_VERSION,
          error: String(err),
        });
      });
    client.executeQuery("ConfigGet").then((result) => {
      if (cancelled || !result.bodyJson) return;
      try {
        const body = JSON.parse(result.bodyJson) as { deviceName?: string };
        if (body.deviceName) setDeviceName(body.deviceName);
      } catch {
        /* ignore */
      }
    });
    refreshHandoff().catch((err: unknown) => {
      if (!cancelled) setHandoffError(String(err));
    });
    refreshCanonical().catch(() => {
      /* ignore until host is ready */
    });
    return () => {
      cancelled = true;
    };
  }, [refreshHandoff, refreshCanonical]);

  const runCommand = async (name: string, body?: unknown) => {
    setBusy(true);
    setActionMessage(null);
    try {
      const result = await client.executeCommand(name, body);
      setActionMessage(
        result.ok ? `${name} ok` : `${name} failed: ${result.errorCode ?? "error"}`,
      );
      await refreshHandoff();
      await refreshCanonical();
      if (name === "ConfigSet" && result.bodyJson) {
        const cfg = JSON.parse(result.bodyJson) as { deviceName?: string };
        if (cfg.deviceName) setDeviceName(cfg.deviceName);
      }
    } catch (err: unknown) {
      setActionMessage(String(err));
    } finally {
      setBusy(false);
    }
  };

  const importSampleFidelity = async () => {
    setBusy(true);
    setActionMessage(null);
    try {
      const csv =
        "Run Date,Account,Action,Symbol,Security Description,Amount\n01/17/2026,Taxable Brokerage,DIVIDEND RECEIVED,CASH,USD Cash,500.00\n";
      const staged = await client.executeCommand("ImportStage", {
        sourceId: "ui-sample-fidelity-div",
        filename: "fidelity-dividend.csv",
        content: csv,
        accountName: "Taxable Brokerage",
      });
      if (!staged.ok || !staged.bodyJson) {
        setActionMessage(
          `ImportStage failed: ${staged.errorCode ?? "error"} (register sample account first)`,
        );
        return;
      }
      const batchId = (JSON.parse(staged.bodyJson) as { batchId?: string }).batchId;
      if (!batchId) {
        setActionMessage("ImportStage failed: missing batchId");
        return;
      }
      for (const name of ["ImportValidate", "ImportApprove", "ImportPost"] as const) {
        const result = await client.executeCommand(name, { batchId });
        if (!result.ok) {
          setActionMessage(`${name} failed: ${result.errorCode ?? "error"}`);
          return;
        }
      }
      setActionMessage("ImportPost ok");
      await refreshHandoff();
      await refreshCanonical();
    } catch (err: unknown) {
      setActionMessage(String(err));
    } finally {
      setBusy(false);
    }
  };

  const checkForUpdates = async () => {
    setBusy(true);
    setActionMessage(null);
    try {
      const update = await check();
      if (update) {
        setUpdateStatus(`available ${update.version} (not applied)`);
        setActionMessage(`Update ${update.version} available; not applied`);
      } else {
        setUpdateStatus("none");
        setActionMessage("No update available");
      }
    } catch (err: unknown) {
      setUpdateStatus("fail-closed");
      setActionMessage(`Update check failed closed: ${String(err)}`);
    } finally {
      setBusy(false);
    }
  };

  const writesBlocked = handoff !== null && !handoff.writesAllowed;

  return (
    <main className="container" aria-label="finos">
      <h1>finos</h1>
      <p>Local-first finance desktop (Milestone 4)</p>
      {health === null ? (
        <p>Checking HealthGet…</p>
      ) : (
        <section>
          <h2>HealthGet</h2>
          <dl className="health">
            <dt>ok</dt>
            <dd>{health.ok ? "true" : "false"}</dd>
            <dt>status</dt>
            <dd>{health.status}</dd>
            <dt>contract</dt>
            <dd>{health.contractVersion}</dd>
            {health.error ? (
              <>
                <dt>error</dt>
                <dd>{health.error}</dd>
              </>
            ) : null}
          </dl>
        </section>
      )}

      <section>
        <h2>HandoffStatusGet</h2>
        {handoffError ? <p className="blocked">{handoffError}</p> : null}
        {handoff === null && !handoffError ? (
          <p>Loading handoff status…</p>
        ) : handoff ? (
          <dl className="health">
            <dt>decision</dt>
            <dd>{handoff.decision}</dd>
            <dt>writes</dt>
            <dd className={writesBlocked ? "blocked" : undefined}>
              {handoff.writesAllowed ? "allowed" : "blocked"}
            </dd>
            <dt>message</dt>
            <dd>{handoff.message}</dd>
            <dt>local</dt>
            <dd>{handoff.localHead?.snapshotId ?? "none"}</dd>
            <dt>published</dt>
            <dd>{handoff.publishedHead?.snapshotId ?? "none"}</dd>
          </dl>
        ) : null}
      </section>

      <section className="actions">
        <h2>Platform</h2>
        <label>
          Device name
          <input
            value={deviceName}
            onChange={(e) => setDeviceName(e.target.value)}
            disabled={busy || writesBlocked}
          />
        </label>
        <div className="buttons">
          <button
            type="button"
            aria-label="Save device name"
            disabled={busy || writesBlocked}
            onClick={() => runCommand("ConfigSet", { deviceName })}
          >
            Save device name
          </button>
          <button
            type="button"
            aria-label="Create snapshot"
            disabled={busy}
            onClick={() => runCommand("SnapshotCreate")}
          >
            Create snapshot
          </button>
          <button
            type="button"
            aria-label="Restore published"
            disabled={busy}
            onClick={() => runCommand("SnapshotRestore")}
          >
            Restore published
          </button>
          <button
            type="button"
            aria-label="Acknowledge review"
            disabled={busy || handoff?.decision !== "block_until_restore"}
            onClick={() => runCommand("HandoffResolve", { action: "review" })}
          >
            Acknowledge review
          </button>
          <button
            type="button"
            aria-label="Exit"
            onClick={() => {
              void invoke("app_exit");
            }}
          >
            Exit
          </button>
          <button
            type="button"
            aria-label="Check for updates"
            disabled={busy}
            onClick={() => void checkForUpdates()}
          >
            Check for updates
          </button>
        </div>
        {writesBlocked ? (
          <p className="blocked">
            Ordinary writes are blocked until restore or explicit review.
          </p>
        ) : null}
        {actionMessage ? <p>{actionMessage}</p> : null}
        <p>Update check: {updateStatus}. Missing releases fail closed and do not post.</p>
      </section>

      <section>
        <h2>Canonical foundation</h2>
        <dl className="health">
          <dt>week</dt>
          <dd>
            {week ? `${week.start ?? "?"} to ${week.end ?? "?"} (Sat–Fri)` : "loading…"}
          </dd>
          <dt>import</dt>
          <dd>{importStatus}</dd>
        </dl>
        <p>Accounts</p>
        {accounts.length === 0 ? (
          <p>None yet</p>
        ) : (
          <ul>
            {accounts.map((a) => (
              <li key={a.name}>
                {a.name} ({a.kind})
              </li>
            ))}
          </ul>
        )}
        <p>Exceptions</p>
        {exceptions.length === 0 ? (
          <p>None</p>
        ) : (
          <ul>
            {exceptions.map((e) => (
              <li key={e.message}>
                {e.code}: {e.message}
              </li>
            ))}
          </ul>
        )}
        <div className="buttons">
          <button
            type="button"
            aria-label="Register sample account"
            disabled={busy || writesBlocked}
            onClick={() =>
              runCommand("AccountRegister", {
                name: "Taxable Brokerage",
                kind: "taxable",
              })
            }
          >
            Register sample account
          </button>
          <button
            type="button"
            aria-label="Import sample Fidelity dividend"
            disabled={busy || writesBlocked}
            onClick={() => void importSampleFidelity()}
          >
            Import sample Fidelity dividend
          </button>
        </div>
      </section>

      <section>
        <h2>Dividend actuals</h2>
        <p>
          Income Plan, Dashboard, and Trends read the same posted actual total.
          Declarations never mix in.
        </p>
        <dl className="health">
          <dt>DividendGet</dt>
          <dd>{dividend ? dividend.actualTotalMinor : "loading…"}</dd>
          <dt>IncomePlanGet</dt>
          <dd>{incomePlan ? incomePlan.actualMinor : "loading…"}</dd>
          <dt>DashboardGet</dt>
          <dd>{dashboard ? dashboard.actualDividendMinor : "loading…"}</dd>
          <dt>TrendsGet</dt>
          <dd>{trends ? trends.totalMinor : "loading…"}</dd>
        </dl>
      </section>

      <section>
        <h2>Lots and ROI</h2>
        <p>Dual basis stays separate. Sales are unmatched until an explicit lot is assigned.</p>
        <dl className="health">
          <dt>performance</dt>
          <dd>{basis ? basis.openPerformanceMinor : "loading…"}</dd>
          <dt>tax</dt>
          <dd>{basis ? basis.openTaxMinor : "loading…"}</dd>
          <dt>perf gain</dt>
          <dd>{roi ? roi.performanceGainMinor : "loading…"}</dd>
          <dt>tax gain</dt>
          <dd>{roi ? roi.taxGainMinor : "loading…"}</dd>
          <dt>lot recon</dt>
          <dd>
            {lotRecon
              ? lotRecon.matched
                ? "matched"
                : `${lotRecon.unmatchedSells} unmatched`
              : "loading…"}
          </dd>
          <dt>positions</dt>
          <dd>
            {positions
              ? positions.positions.length === 0
                ? "none"
                : positions.positions
                    .map((p) => `${p.symbol} × ${p.remainingQuantityMinor}`)
                    .join(", ")
              : "loading…"}
          </dd>
        </dl>
      </section>

      <section>
        <h2>Marketplace MAGI</h2>
        <p>Projection comes from MagiProjectionGet. Incomplete or pending-review facts never display SAFE.</p>
        <dl className="health">
          <dt>decision</dt>
          <dd>{magi ? magi.decisionState : "not set"}</dd>
          <dt>actual included</dt>
          <dd>{magi ? magi.actualIncludedYtd.amountMinor : "—"}</dd>
          <dt>protected headroom</dt>
          <dd>{magi ? magi.protectedHeadroom.amountMinor : "—"}</dd>
          <dt>TaxProjectionGet</dt>
          <dd>
            {taxProjection
              ? `${taxProjection.decisionState} (from ${taxProjection.sourceQuery})`
              : "not set"}
          </dd>
        </dl>
      </section>

      <section>
        <h2>Calculator Plan</h2>
        <p>Approved Plan remaining is not actual cash. Burndown uses Plan remaining as the obligation.</p>
        <dl className="health">
          <dt>PlanGet remaining</dt>
          <dd>{calcPlan ? calcPlan.remainingMinor : "—"}</dd>
          <dt>version</dt>
          <dd>{calcPlan ? calcPlan.version : "—"}</dd>
          <dt>cash</dt>
          <dd>{burndown ? burndown.cashMinor : "—"}</dd>
          <dt>obligation</dt>
          <dd>{burndown ? burndown.obligationMinor : "—"}</dd>
          <dt>sufficient</dt>
          <dd>{burndown ? (burndown.sufficient ? "yes" : "no") : "—"}</dd>
        </dl>
      </section>

      <section>
        <h2>Allocation</h2>
        <p>
          Targets are decision support. Open performance and tax amounts are lot
          cost basis, not market value. They do not post cash or MAGI facts.
        </p>
        <dl className="health">
          <dt>targets</dt>
          <dd>{allocation ? allocation.targets.length : "—"}</dd>
          <dt>open performance (basis)</dt>
          <dd>{allocation ? allocation.openPerformanceMinor : "—"}</dd>
          <dt>open tax (basis)</dt>
          <dd>{allocation ? allocation.openTaxMinor : "—"}</dd>
        </dl>
      </section>

      <section>
        <h2>Shopping cart</h2>
        <p>Cart lines are not fills. They do not post lots, cash, or MAGI facts.</p>
        <dl className="health">
          <dt>items</dt>
          <dd>{cart ? cart.items.length : "—"}</dd>
        </dl>
      </section>

      <section>
        <h2>Backtest</h2>
        <p>Hypothetical P&amp;L is not a posted activity.</p>
        <dl className="health">
          <dt>runs</dt>
          <dd>{backtest ? backtest.runs.length : "—"}</dd>
        </dl>
      </section>

      <section>
        <h2>Classification review</h2>
        <p>A review record is not a MAGI oracle change.</p>
        <dl className="health">
          <dt>reviews</dt>
          <dd>{classification ? classification.reviews.length : "—"}</dd>
        </dl>
      </section>

      <section>
        <h2>AI advisory</h2>
        <p>Recommendations only. Does not post ledger, lots, or MAGI facts. Requires XAI_API_KEY.</p>
        <dl className="health">
          <dt>runs</dt>
          <dd>{aiRuns ? aiRuns.runs.length : "—"}</dd>
          <dt>last</dt>
          <dd>
            {aiRuns && aiRuns.runs.length
              ? aiRuns.runs[aiRuns.runs.length - 1].recommendation
              : "—"}
          </dd>
        </dl>
      </section>
    </main>
  );
}
