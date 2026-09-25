import { Fragment, type Dispatch, type SetStateAction } from "react";
import {
  ExceptionList,
  WorkTicketQueue,
  formatCount,
  formatUsd,
} from "@finos/ui-components";
import type { ExceptionRecord, WorkTicketRecord } from "@finos/app-contracts";
import {
  collectorIsCash,
  collectorNeedsOwnerUrl,
  fleetRocText,
  formatCollectorClock,
} from "./helpers";
import type {
  CollectorRunProgress,
  CollectorSetItem,
  CollectorStats,
  ResearchActivity,
  RetrieveRunRow,
} from "./types";

export type CollectorsScreenProps = {
  busy: boolean;
  writesBlocked: boolean;
  collectorItems: CollectorSetItem[];
  collectorStats: CollectorStats | null;
  collectorAction: string | null;
  collectorStatusAt: string;
  collectorRunProgress: CollectorRunProgress | null;
  researchActivity: ResearchActivity | null;
  missingUrlDrafts: Record<string, { sourceUrl: string; rocSourceUrl: string }>;
  setMissingUrlDrafts: Dispatch<
    SetStateAction<Record<string, { sourceUrl: string; rocSourceUrl: string }>>
  >;
  fleetDetailId: string | null;
  setFleetDetailId: (id: string | null) => void;
  collectorSymbol: string;
  collectorRuns: RetrieveRunRow[];
  collectorPayload: Record<string, unknown> | null;
  collectorPlan: {
    known: boolean;
    perShareMinor: number;
    scale: number;
    reason: string;
  } | null;
  exceptions: ExceptionRecord[];
  workTickets: WorkTicketRecord[];
  retryingTicket: { ticketId: string; symbol: string } | null;
  ticketDecision: { ticketId: string; action: "accept" | "reject" | "except" } | null;
  onRunEnabled: () => void;
  onRunMisses: () => void;
  onFillResearchGaps: () => void;
  onApplyIssuerSources: () => void;
  onApplyMissingUrls: () => void;
  onToggleEnabled: (row: CollectorSetItem, enabled: boolean) => void;
  onForceRefresh: (row: CollectorSetItem) => void;
  onOpenPosition: (symbol: string) => void;
  onOpenTickets: (symbol: string) => void;
  onOpenPositionHub: (symbol: string) => void;
  onOpenExceptionLog: () => void;
  onRetryTicket: (ticket: WorkTicketRecord) => void;
  onRecreateAdapter: (ticket: WorkTicketRecord) => void;
  onExceptTicket: (ticket: WorkTicketRecord) => void;
  onRejectTicket: (ticket: WorkTicketRecord) => void;
  onEnterAmount: (ticket: WorkTicketRecord, amount: string) => void;
  onFileTicket: (ticket: WorkTicketRecord) => void;
};

export function CollectorsScreen(props: CollectorsScreenProps) {
  const {
    busy,
    writesBlocked,
    collectorItems,
    collectorStats,
    collectorAction,
    collectorStatusAt,
    collectorRunProgress,
    researchActivity,
    missingUrlDrafts,
    setMissingUrlDrafts,
    fleetDetailId,
    setFleetDetailId,
    collectorSymbol,
    collectorRuns,
    collectorPayload,
    collectorPlan,
    exceptions,
    workTickets,
    retryingTicket,
    ticketDecision,
    onRunEnabled,
    onRunMisses,
    onFillResearchGaps,
    onApplyIssuerSources,
    onApplyMissingUrls,
    onToggleEnabled,
    onForceRefresh,
    onOpenPosition,
    onOpenTickets,
    onOpenPositionHub,
    onOpenExceptionLog,
    onRetryTicket,
    onRecreateAdapter,
    onExceptTicket,
    onRejectTicket,
    onEnterAmount,
    onFileTicket,
  } = props;
  return (
    <section aria-label="Collectors">
              <h2>Collectors</h2>
              <p>
                Income names only (DIV-1, CASH, Weekly/Monthly/Quarterly). Non-payers
                are excluded. Collect fresh distribution data is the daily run —
                declarations only. Establish and Reevaluate collector are on Tools.
                Yahoo last price is separate and does not fill this page. Yahoo is
                never a declaration source.
              </p>
              <div className="buttons">
                <button
                  type="button"
                  aria-label="Run enabled collectors"
                  disabled={busy || writesBlocked}
                  onClick={() => onRunEnabled()}
                >
                  {collectorRunProgress?.running
                    ? `Running ${formatCount(collectorRunProgress.current)} of ${formatCount(collectorRunProgress.total)}${
                        collectorRunProgress.symbol
                          ? `: ${collectorRunProgress.symbol}`
                          : ""
                      }…`
                    : "Run enabled collectors"}
                </button>
                <button
                  type="button"
                  aria-label="Run misses only"
                  disabled={busy || writesBlocked}
                  onClick={() => onRunMisses()}
                >
                  Run misses only
                </button>
                <button
                  type="button"
                  aria-label="Fill research gaps"
                  disabled={busy || writesBlocked || Boolean(researchActivity?.running)}
                  onClick={() => onFillResearchGaps()}
                >
                  {researchActivity?.running
                    ? "Filling research gaps…"
                    : "Fill research gaps"}
                </button>
                <button
                  type="button"
                  aria-label="Apply issuer sources from provider"
                  disabled={busy || writesBlocked}
                  onClick={() => onApplyIssuerSources()}
                >
                  Apply issuer sources from provider
                </button>
              </div>
              {researchActivity?.running ? (
                <section
                  className="process-a-research-progress"
                  aria-label="Research progress"
                  aria-busy="true"
                >
                  <p role="status" aria-live="polite">
                    {researchActivity.label}
                  </p>
                  {researchActivity.total > 0 ? (
                    <progress
                      max={researchActivity.total}
                      value={Math.min(researchActivity.step, researchActivity.total)}
                    />
                  ) : (
                    <div className="process-a-research-spinner" aria-hidden="true" />
                  )}
                </section>
              ) : null}
              {researchActivity?.resultLine &&
              !researchActivity.running ? (
                <p role="status" aria-label="Research result">
                  {researchActivity.resultLine}
                </p>
              ) : null}
              {writesBlocked ? (
                <p role="status">
                  Writes are blocked on this device, so Apply and Run enabled are
                  disabled.
                </p>
              ) : null}
              {collectorAction ? (
                <p aria-label="Collector action status" role="status" aria-live="polite">
                  {collectorAction}
                </p>
              ) : (
                <p aria-label="Collector action status" role="status">
                  Daily fleet
                  is open lots only — saved-but-incomplete names stay out. Run
                  enabled and Run misses only collect declarations (not prices).
                  Collect fresh distribution data is one symbol, declarations
                  only. Stored pays are never overwritten. Amount changes ticket
                  Except or Reject. Empty / 403 / JS parks the name — do not
                  Establish-replace. Identity and Accept ROC are on Tools. Fill
                  research gaps fills missing provider, frequency, DIV-1, or ROC
                  estimate (underlying is extra). Apply fills empty templates
                  from provider (0 updated means already assigned).
                </p>
              )}
              {collectorRunProgress ? (
                <section
                  aria-label="Collector run progress"
                  aria-busy={collectorRunProgress.running}
                >
                  <h3>
                    {collectorRunProgress.running ? "Collecting…" : "Last run"}
                  </h3>
                  <p>
                    {collectorRunProgress.running
                      ? "In progress"
                      : `Finished ${formatCollectorClock(collectorRunProgress.finishedAt)}`}
                    . Progress {formatCount(collectorRunProgress.current)} /{" "}
                    {formatCount(collectorRunProgress.total)}
                    {collectorRunProgress.symbol
                      ? ` — ${collectorRunProgress.symbol}`
                      : ""}
                    . Ok {formatCount(collectorRunProgress.ok)}. Miss{" "}
                    {formatCount(collectorRunProgress.miss)}.
                  </p>
                  <progress
                    max={Math.max(collectorRunProgress.total, 1)}
                    value={collectorRunProgress.current}
                  />
                  <div className="table-wrap">
                    <table aria-label="Collector run log">
                      <thead>
                        <tr>
                          <th scope="col">Result</th>
                        </tr>
                      </thead>
                      <tbody>
                        {collectorRunProgress.lines.length === 0 ? (
                          <tr>
                            <td>
                              {collectorRunProgress.running
                                ? "Waiting for first symbol…"
                                : "No lines."}
                            </td>
                          </tr>
                        ) : (
                          [...collectorRunProgress.lines].reverse().map((line, idx) => (
                            <tr key={`${idx}-${line}`}>
                              <td>{line}</td>
                            </tr>
                          ))
                        )}
                      </tbody>
                    </table>
                  </div>
                </section>
              ) : null}
              {collectorStats ? (
                <p aria-label="Collector statistics">
                  Assigned {formatCount(collectorStats.assigned)}. Enabled{" "}
                  {formatCount(collectorStats.enabled)}. We collect these{" "}
                  {formatCount(collectorStats.enabled)} income names. Ran{" "}
                  {formatCount(collectorStats.ranToday)} of those{" "}
                  {formatCount(collectorStats.enabled)} we asked the issuer about
                  today
                  {(collectorStats.ranOutsideFleet ?? []).length > 0
                    ? ` (also retrieved ${(collectorStats.ranOutsideFleet ?? []).join(", ")} — not income fleet)`
                    : ""}
                  . Still miss {formatCount(collectorStats.stillMiss ?? collectorStats.missToday)}
                  — last ask today is not OK. Fail{" "}
                  {formatCount(collectorStats.failCount ?? collectorStats.stillMiss ?? 0)}{" "}
                  must equal miss tickets{" "}
                  {formatCount(collectorStats.failTicketCount ?? 0)}
                  {collectorStats.failTicketParity === false
                    ? " — gate broken"
                    : ""}
                  . Had a miss today{" "}
                  {formatCount(collectorStats.hadMissToday ?? 0)} — failed at least
                  once today; a later retry may have worked. Unchanged{" "}
                  {formatCount(collectorStats.unchangedToday)} — issuer page matched
                  the last copy; we did not rewrite Plan $. Price current{" "}
                  {formatCount(collectorStats.priceCurrent)} / stale{" "}
                  {formatCount(collectorStats.priceStale)} — share price quotes, not
                  dividend pages. Open tickets{" "}
                  {formatCount(collectorStats.openExceptions)} — owner work still
                  open, including older days.
                </p>
              ) : null}
              {(() => {
                const today = new Date().toISOString().slice(0, 10);
                const fleetEnabled = collectorItems.filter(
                  (row) =>
                    row.collectorEnabled && (row.declarationSource || "").trim(),
                );
                const todayOk = fleetEnabled.filter(
                  (row) =>
                    row.lastRunOk === true &&
                    (row.lastRunAt || "").startsWith(today),
                );
                const todayMisses = fleetEnabled.filter(
                  (row) =>
                    !(
                      row.lastRunOk === true &&
                      (row.lastRunAt || "").startsWith(today)
                    ),
                );
                const latestStored = [...todayOk, ...todayMisses]
                  .map((row) => row.lastRunAt || "")
                  .filter((at) => at.length >= 10)
                  .sort()
                  .at(-1);
                return (
                  <section aria-label="Today collector status">
                    <h3>Today&apos;s declaration status</h3>
                    <p>
                      As of {formatCollectorClock(latestStored || collectorStatusAt)}.{" "}
                      {formatCount(todayOk.length)} last retrieve today OK ·{" "}
                      {formatCount(todayMisses.length)} still miss of{" "}
                      {formatCount(fleetEnabled.length)} enabled.
                      Older tickets below are a work queue, not this run.
                    </p>
                    {todayMisses.length > 0 ? (
                      <div className="table-wrap">
                        <table aria-label="Today declaration misses">
                          <thead>
                            <tr>
                              <th scope="col">Symbol</th>
                              <th scope="col">Source</th>
                              <th scope="col">Last run</th>
                              <th scope="col">Why this run missed</th>
                            </tr>
                          </thead>
                          <tbody>
                            {todayMisses.map((row) => (
                              <tr key={`miss-${row.securityId}`}>
                                <td>{row.symbol}</td>
                                <td>{row.declarationSource || "unassigned"}</td>
                                <td>{formatCollectorClock(row.lastRunAt)}</td>
                                <td>{row.lastRunMessage || "Issuer page empty."}</td>
                              </tr>
                            ))}
                          </tbody>
                        </table>
                      </div>
                    ) : (
                      <p>No declaration misses recorded today.</p>
                    )}
                  </section>
                );
              })()}
              {(() => {
                const missing = collectorItems.filter(collectorNeedsOwnerUrl);
                if (missing.length === 0) {
                  return null;
                }
                return (
                  <section aria-label="Missing collector URLs">
                    <h3>Missing issuer URLs</h3>
                    <p>
                      Paste Template Dividend (seed URL) for each failing DIV-1 or
                      CASH collector that has none. Apply before collect will succeed.
                      CASH does not take Template ROC. Empty cells stay empty.
                    </p>
                    <div className="table-wrap">
                      <table aria-label="Missing collector URLs">
                        <thead>
                          <tr>
                            <th scope="col">Symbol</th>
                            <th scope="col">Adapter</th>
                            <th scope="col">Template Dividend</th>
                            <th scope="col">Template ROC</th>
                          </tr>
                        </thead>
                        <tbody>
                          {missing.map((row) => {
                            const draft = missingUrlDrafts[row.securityId] ?? {
                              sourceUrl: row.sourceUrl ?? "",
                              rocSourceUrl: row.rocSourceUrl ?? "",
                            };
                            const cash = collectorIsCash(row);
                            return (
                              <tr key={`url-${row.securityId}`}>
                                <td>{row.symbol}</td>
                                <td>{row.declarationSource || "unassigned"}</td>
                                <td>
                                  <input
                                    aria-label={`Template Dividend for ${row.symbol}`}
                                    value={draft.sourceUrl}
                                    onChange={(e) =>
                                      setMissingUrlDrafts((prev) => ({
                                        ...prev,
                                        [row.securityId]: {
                                          ...draft,
                                          sourceUrl: e.target.value,
                                        },
                                      }))
                                    }
                                    disabled={busy || writesBlocked}
                                  />
                                </td>
                                <td>
                                  <input
                                    aria-label={`Template ROC for ${row.symbol}`}
                                    value={cash ? "" : draft.rocSourceUrl}
                                    onChange={(e) =>
                                      setMissingUrlDrafts((prev) => ({
                                        ...prev,
                                        [row.securityId]: {
                                          ...draft,
                                          rocSourceUrl: e.target.value,
                                        },
                                      }))
                                    }
                                    disabled={busy || writesBlocked || cash}
                                  />
                                </td>
                              </tr>
                            );
                          })}
                        </tbody>
                      </table>
                    </div>
                    <div className="buttons">
                      <button
                        type="button"
                        aria-label="Apply URLs"
                        disabled={busy || writesBlocked}
                        onClick={() => onApplyMissingUrls()}
                      >
                        Apply URLs
                      </button>
                    </div>
                  </section>
                );
              })()}
              <section aria-label="Collector footer grid">
              <h3>Fleet footer</h3>
              <p>
                One row per open-lot collector — the same set as Run enabled.
                Adapter is the face source. Missing ROC, declaration, price, or
                actual is N/A, never $0.
              </p>
              <div className="table-wrap">
                <table aria-label="Collector fleet">
                  <thead>
                    <tr>
                      <th scope="col">Symbol</th>
                      <th scope="col">Adapter</th>
                      <th scope="col">Enabled</th>
                      <th scope="col">Complete</th>
                      <th scope="col">div_type</th>
                      <th scope="col">Frequency</th>
                      <th scope="col">Remaining</th>
                      <th scope="col">Paid decls</th>
                      <th scope="col">Inception</th>
                      <th scope="col">Last run</th>
                      <th scope="col">Runs ok / fail</th>
                      <th scope="col">Template Dividend</th>
                      <th scope="col">Template ROC</th>
                      <th scope="col">Tickets</th>
                      <th scope="col">Last price</th>
                      <th scope="col">Actions</th>
                    </tr>
                  </thead>
                  <tbody>
                    {collectorItems.map((row) => {
                      const complete = row.complete === true;
                      const gaps = row.gaps ?? [];
                      const remaining =
                        row.remainingPlanned == null && row.remainingExpected == null
                          ? "—"
                          : `${formatCount(row.remainingPlanned ?? 0)} / ${formatCount(row.remainingExpected ?? 0)}`;
                      const open = fleetDetailId === row.securityId;
                      return (
                        <Fragment key={row.securityId}>
                      <tr>
                        <td>
                          <button
                            type="button"
                            aria-label={`Open ${row.symbol} position`}
                            onClick={() => {
                              onOpenPosition(row.symbol);
                            }}
                          >
                            {row.symbol}
                          </button>
                        </td>
                        <td>{row.declarationSource || "unassigned"}</td>
                        <td>{row.collectorEnabled ? "yes" : "no"}</td>
                        <td>
                          {complete ? "Y" : "N"}
                          {!complete && gaps.length > 0
                            ? ` — ${gaps.join(", ")}`
                            : ""}
                        </td>
                        <td>{(row.divType ?? "").trim() ? row.divType : "blank"}</td>
                        <td>{row.paymentFrequency || "—"}</td>
                        <td>{remaining}</td>
                        <td>{formatCount(row.paidDeclarationCount ?? 0)}</td>
                        <td>{row.inceptionOn?.trim() || "—"}</td>
                        <td>
                          {row.lastRunAt || "never"}
                          {row.lastRunOk === true
                            ? " ok"
                            : row.lastRunOk === false
                              ? " miss"
                              : ""}
                        </td>
                        <td>
                          {formatCount(row.successfulRunCount ?? 0)} /{" "}
                          {formatCount(row.failureCount ?? 0)}
                        </td>
                        <td>{row.sourceUrl?.trim() || "—"}</td>
                        <td>{row.rocSourceUrl?.trim() || "—"}</td>
                        <td>
                          <button
                            type="button"
                            aria-label={`Open tickets for ${row.symbol}`}
                            disabled={(row.openTicketCount ?? 0) === 0}
                            onClick={() => {
                              onOpenTickets(row.symbol);
                            }}
                          >
                            {formatCount(row.openTicketCount ?? 0)}
                            {row.latestTicketField
                              ? ` ${row.latestTicketField}`
                              : ""}
                          </button>
                        </td>
                        <td>
                          {row.lastPriceFreshness === "Unavailable" ||
                          !row.lastPriceFreshness
                            ? "N/A — Unavailable"
                            : `${row.lastPriceAsOf?.trim() || "N/A"} ${row.lastPriceFreshness}`}
                        </td>
                        <td>
                          <button
                            type="button"
                            aria-label={
                              row.collectorEnabled
                                ? `Disable collector for ${row.symbol}`
                                : `Enable collector for ${row.symbol}`
                            }
                            disabled={busy || writesBlocked || !row.declarationSource}
                            onClick={() =>
                              onToggleEnabled(row, !row.collectorEnabled)
                            }
                          >
                            {row.collectorEnabled ? "Disable" : "Enable"}
                          </button>{" "}
                          <button
                            type="button"
                            aria-label="Collect fresh distribution data for this symbol"
                            disabled={busy || writesBlocked}
                            onClick={() => onForceRefresh(row)}
                          >
                            Collect fresh distribution data
                          </button>{" "}
                          <button
                            type="button"
                            aria-label={`Toggle details for ${row.symbol}`}
                            aria-expanded={open}
                            onClick={() =>
                              setFleetDetailId(open ? null : row.securityId)
                            }
                          >
                            {open ? "Hide detail" : "Detail"}
                          </button>
                        </td>
                      </tr>
                      {open ? (
                        <tr>
                          <td colSpan={16}>
                            <dl aria-label={`${row.symbol} collector detail`}>
                              <div>
                                <dt>Provider</dt>
                                <dd>{row.provider || "—"}</dd>
                              </div>
                              <div>
                                <dt>Underlying</dt>
                                <dd>{row.underlying?.trim() || "—"}</dd>
                              </div>
                              <div>
                                <dt>ROC estimate</dt>
                                <dd>{fleetRocText(row)}</dd>
                              </div>
                              <div>
                                <dt>Content hash</dt>
                                <dd>
                                  {row.lastContentHash?.trim()
                                    ? row.lastContentHash.slice(0, 8)
                                    : "—"}
                                </dd>
                              </div>
                              <div>
                                <dt>Open lots</dt>
                                <dd>{formatCount(row.openLotCount ?? 0)}</dd>
                              </div>
                              <div>
                                <dt>Last payable</dt>
                                <dd>{row.lastPayableOn?.trim() || "—"}</dd>
                              </div>
                            </dl>
                          </td>
                        </tr>
                      ) : null}
                        </Fragment>
                      );
                    })}
                  </tbody>
                </table>
              </div>
              </section>
              <section aria-label="Collector work queue">
                <h3>Work queue (not today&apos;s run log)</h3>
                <p>
                  Open tickets stay until filed. Use Today&apos;s declaration status
                  above for the current run.
                </p>
                <ExceptionList exceptions={exceptions} onOpenLog={() => onOpenExceptionLog()} />
                <WorkTicketQueue
                  tickets={workTickets}
                  retryingTicketId={retryingTicket?.ticketId}
                  retryingSymbol={retryingTicket?.symbol}
                  pendingTicketId={ticketDecision?.ticketId}
                  pendingAction={ticketDecision?.action}
                  onRetry={(t) => onRetryTicket(t as WorkTicketRecord)}
                  onRecreateAdapter={(t) => onRecreateAdapter(t as WorkTicketRecord)}
                  onExcept={(t) => onExceptTicket(t as WorkTicketRecord)}
                  onReject={(t) => onRejectTicket(t as WorkTicketRecord)}
                  onEnterAmount={(t, amount) => onEnterAmount(t as WorkTicketRecord, amount)}
                  onFile={(t) => onFileTicket(t as WorkTicketRecord)}
                />
              </section>
              {collectorSymbol ? (
                <section aria-label="Collector symbol page">
                  <h3>{collectorSymbol}</h3>
                  <div className="buttons">
                    <button
                      type="button"
                      aria-label="Open in Position Details"
                      onClick={() => onOpenPositionHub(collectorSymbol)}
                    >
                      Open in Position Details
                    </button>
                  </div>
                  <p>
                    Collectors is ops only — last retrieve runs and payload. Position
                    data (declarations, plan pays, received totals) lives on Position
                    Details.
                  </p>
                  {collectorPlan ? (
                    <p aria-label="Collector plan">
                      Plan check:{" "}
                      {collectorPlan.known
                        ? `${formatUsd(collectorPlan.perShareMinor, collectorPlan.scale)}${
                            collectorPlan.reason ? ` — ${collectorPlan.reason}` : ""
                          }`
                        : "unknown (not $0)"}
                    </p>
                  ) : null}
                  <div className="table-wrap">
                    <table aria-label="Retrieve runs">
                      <thead>
                        <tr>
                          <th scope="col">When</th>
                          <th scope="col">Kind</th>
                          <th scope="col">Ok</th>
                          <th scope="col">Recorded</th>
                          <th scope="col">Skipped</th>
                          <th scope="col">Message</th>
                        </tr>
                      </thead>
                      <tbody>
                        {collectorRuns
                          .filter((run) => run.kind !== "price")
                          .concat(collectorRuns.filter((run) => run.kind === "price").slice(0, 3))
                          .map((run) => (
                          <tr key={run.runId}>
                            <td>{run.requestedAt}</td>
                            <td>{run.kind}</td>
                            <td>{run.ok ? "ok" : "miss"}</td>
                            <td>{formatCount(run.recorded)}</td>
                            <td>{formatCount(run.skipped)}</td>
                            <td>
                              {run.code ? `${run.code}: ` : ""}
                              {run.message || "—"}
                            </td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                  {collectorPayload ? (
                    <div className="table-wrap">
                      <table aria-label="Collector retrieve payload">
                        <thead>
                          <tr>
                            <th scope="col">Field</th>
                            <th scope="col">Value</th>
                          </tr>
                        </thead>
                        <tbody>
                          {Object.entries(collectorPayload).map(([key, value]) => (
                            <tr key={key}>
                              <td>{key}</td>
                              <td>
                                <code>
                                  {typeof value === "string"
                                    ? value
                                    : JSON.stringify(value)}
                                </code>
                              </td>
                            </tr>
                          ))}
                        </tbody>
                      </table>
                    </div>
                  ) : (
                    <p>
                      No declaration retrieve payload yet. Daily open and Force
                      refresh write declaration runs here; last-price-only runs are
                      not shown as the payload.
                    </p>
                  )}
                </section>
              ) : null}
            </section>
  );
}
