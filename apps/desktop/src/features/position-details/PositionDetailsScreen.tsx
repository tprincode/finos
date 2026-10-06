/**
 * Position Details hub — markup extracted from App (binder props; logic stays in App).
 */
// @ts-nocheck — binder extract; props typed loosely until Next pass.

const EVIDENCE_ROWS = [
  {
    key: "incomeReliability",
    label: "Plan achieved % in period",
    title:
      "Cash received in this window divided by the cash the plan expected. 120% means the window paid 1.20 for each 1.00 the plan expected.",
  },
  {
    key: "downsideResilience",
    label: "Downside resilience",
    title:
      "The deepest fall in the share price during this window, from a high point to a later lower point. 100 means the price never fell. A lower score means a deeper fall. 0 means the price fell by its whole value or more.",
  },
  {
    key: "recoveryUpside",
    label: "Recovery / upside",
    title:
      "After the low, how much of the drop was gained back. Unknown when the low is the last price, so there is no bounce to measure. Unknown is not written as 0.",
  },
  {
    key: "navPersistence",
    label: "NAV persistence",
    title:
      "Whether the share price held up once this window’s cash cushion is included. The score runs from 0 to 100.",
  },
];

const COMPARISON_TIP =
  "Price change from the first day to the last day of this window. This is one episode. It does not say what the next downturn will be.";

const CASH_CUSHION_TIP =
  "Cash the broker paid on one share during this window, divided by that share's price on the first day. 5% means 5 cents of cash for each dollar of the starting price. It is not the whole holding's cash, not yield on cost, and not a Plan figure. A large cushion does not cancel a price drop; read it next to price return.";

const COMPARISON_ROWS = [
  {
    key: "cashCushion",
    label: "Cash cushion",
    title: CASH_CUSHION_TIP,
  },
  {
    key: "days",
    label: "Days",
    title:
      "How many days this window covers, including the first and last day. A longer window is still one episode.",
  },
  {
    key: "underlying",
    label: "Underlying",
    title: COMPARISON_TIP,
  },
  {
    key: "spy",
    label: "SPY",
    title: COMPARISON_TIP,
  },
  {
    key: "nasdaq",
    label: "Nasdaq-100",
    title: COMPARISON_TIP,
  },
  {
    key: "versusUnderlying",
    label: "Versus underlying",
    title:
      "This window's price change minus the underlying's price change for the same days. A negative number means the position fell further than the underlying.",
  },
];

function inclusiveDays(start, end) {
  if (!start || !end) return null;
  const a = Date.parse(`${String(start).slice(0, 10)}T00:00:00Z`);
  const b = Date.parse(`${String(end).slice(0, 10)}T00:00:00Z`);
  if (!Number.isFinite(a) || !Number.isFinite(b)) return null;
  return Math.round((b - a) / 86400000) + 1;
}

function percentFace(bps) {
  if (bps == null) return "unknown";
  return `${(bps / 100).toFixed(2)}%`;
}

function evidenceColumn(investment, kind) {
  const periods = (investment.periods ?? []).filter(
    (period) => String(period.kind).toLowerCase() === kind.toLowerCase(),
  );
  const period =
    periods
      .slice()
      .sort((a, b) => (a.endOn < b.endOn ? 1 : a.endOn > b.endOn ? -1 : 0))[0] ?? null;
  const evidence = period
    ? [...(investment.windowEvidence ?? [])]
        .reverse()
        .find((row) => row.periodId === period.periodId) ?? null
    : null;
  const result = period
    ? [...(investment.results ?? [])]
        .reverse()
        .find((row) => row.periodId === period.periodId) ?? null
    : null;
  const label = period ? `${period.kind} ${period.startOn} – ${period.endOn}` : kind;
  return {
    evidence,
    label,
    period,
    result,
    planAchievedBps: result?.incomeReliabilityBps ?? null,
  };
}

function evidenceFace(row, column) {
  if (!column.evidence) return "";
  if (row.key === "incomeReliability") {
    return column.planAchievedBps == null
      ? "unknown"
      : `${(column.planAchievedBps / 100).toFixed(2)}%`;
  }
  const value = column.evidence[row.key];
  return value == null ? "unknown" : String(value);
}

function comparisonFace(row, column, investment) {
  if (!column.evidence) return "";
  if (row.key === "cashCushion") return percentFace(column.result?.cushionBps);
  if (row.key === "days") {
    const days = inclusiveDays(column.period?.startOn, column.period?.endOn);
    return days == null ? "" : String(days);
  }
  const underlying = String(investment.underlying ?? "").trim();
  if (row.key === "underlying") {
    return underlying ? percentFace(column.result?.underlyingReturnBps) : "";
  }
  if (row.key === "spy") return percentFace(column.result?.spyReturnBps);
  if (row.key === "nasdaq") return percentFace(column.result?.nasdaqReturnBps);
  if (!underlying) return "";
  const position = column.result?.priceReturnBps;
  const base = column.result?.underlyingReturnBps;
  if (position == null || base == null) return "unknown";
  return percentFace(position - base);
}

function EvidenceWindows({ investment }) {
  const bull = evidenceColumn(investment, "Bull");
  const bear = evidenceColumn(investment, "Bear");
  if (!bull.evidence && !bear.evidence) {
    return <p>No evidence dimensions until a window is calculated.</p>;
  }
  return (
    <div className="table-wrap">
      <table aria-label="Evidence dimensions">
        <thead>
          <tr>
            <th></th>
            <th>{bull.label}</th>
            <th>{bear.label}</th>
          </tr>
        </thead>
        <tbody>
          {EVIDENCE_ROWS.map((row) => (
            <tr key={row.key}>
              <th scope="row" title={row.title}>
                {row.label}
              </th>
              <td>{evidenceFace(row, bull)}</td>
              <td>{evidenceFace(row, bear)}</td>
            </tr>
          ))}
          {COMPARISON_ROWS.map((row) => (
            <tr key={row.key}>
              <th scope="row" title={row.title}>
                {row.key === "underlying" && String(investment.underlying ?? "").trim()
                  ? `Underlying ${String(investment.underlying).trim()}`
                  : row.label}
              </th>
              <td>{comparisonFace(row, bull, investment)}</td>
              <td>{comparisonFace(row, bear, investment)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

import type { WorkTicketRecord } from "@finos/app-contracts";
import {
  SymbolLotsTable,
  WorkTicketQueue,
  dividendScore,
  formatCount,
  formatPercentScaled,
  formatScaled,
  formatUsd,
  formatScale6,
  meanNewestPays,
  minPaidDeclaration,
  newestStoredPay,
  parseTypedPlan,
  planCheck,
  planDecisionImpact,
  planFwdAtAmountBps,
} from "@finos/ui-components";
import { DeclarationPaymentsChart } from "../graphing/DeclarationPaymentsChart";
import { SymbolWindowTable } from "../market-impact/SymbolWindowRow";

export type PositionDetailsScreenProps = {
  DECLARATION_LOOKBACK_TARGET: any;
  INCOMPLETE_REASONS: any;
  PERIOD_KINDS: any;
  PLAN_REASONS: any;
  RISK_TIERS: any;
  DIV_TYPES?: readonly string[];
  TAX_HANDLING: any;
  WEEKDAYS: any;
  applyIssuerSources: any;
  applySuggestedTier: any;
  asOfDate: any;
  busy: any;
  calculateWindow: any;
  windowClient: any;
  windowDiscardEpoch: any;
  onWindowsDirty: any;
  onWindowsSaved: any;
  calculator: any;
  declHistory: any;
  calendarPolicyLabel: any;
  cancelPositionEdits: any;
  concentrationSummary: any;
  dateProvenanceLabel: any;
  expectedDeclarationLookback: any;
  factsDirty: any;
  fileWorkTicket: any;
  fixRemainingYearTicket: any;
  filteredPositionSymbols: any;
  formatBps: any;
  formatHubPrice: any;
  investment: any;
  lastPriceBusy: any;
  lastPriceProgress: any;
  lastPriceRefreshLabel: any;
  leaveWithoutSaving: any;
  loadInvestment: any;
  metricHint: any;
  mostCurrentVsPlanFace: any;
  openRecreateAdapter: any;
  ownerRiskChoice: any;
  patchDraft: any;
  patchPeriod: any;
  pdDirty: any;
  pdDraft: any;
  pdLedger: any;
  pdPeriod: any;
  pdRemaining: any;
  periodDirty: any;
  periodReady: any;
  positionDetails: any;
  positionFocusPanel: any;
  positionMaster: any;
  positionSymbol: any;
  positionSymbolOpen: any;
  positionSymbolQuery: any;
  receivedByYear: any;
  refreshLastPrices: any;
  refreshPdDeclarations: any;
  refreshPdLastPrice: any;
  researchActivity: any;
  researchNotes: any;
  researchPdRoc: any;
  resolveConfirmTicket: any;
  resolveEnterDeclaredAmount: any;
  resolveTicketRetry: any;
  resolveTicketRocContext: any;
  storeTicketRocUrl: any;
  storeTicketManualRoc: any;
  openTicketRocResearch: any;
  retryingTicket: any;
  rocActualUnavailable: any;
  rocResearchLabel: any;
  rocResearchUpdated: any;
  saveOwnerPeriod: any;
  saveStoredFacts: any;
  savedPeriodId: any;
  scaledDollars: any;
  selectPositionSymbol: any;
  setOwnerRisk: any;
  setOwnerRiskChoice: any;
  setPositionFocusPanel: any;
  setPositionSymbol: any;
  setPositionSymbolOpen: any;
  setPositionSymbolQuery: any;
  setScreen: any;
  strategyCharacteristics: any;
  ticketDecision: any;
  workTickets: any;
  writesBlocked: any;
};

export function PositionDetailsScreen({
  DECLARATION_LOOKBACK_TARGET,
  INCOMPLETE_REASONS,
  PERIOD_KINDS,
  PLAN_REASONS,
  RISK_TIERS,
  DIV_TYPES = ["DIV-1", "CASH"],
  TAX_HANDLING,
  WEEKDAYS,
  applyIssuerSources,
  applySuggestedTier,
  asOfDate,
  busy,
  calculateWindow: _calculateWindow,
  windowClient,
  windowDiscardEpoch,
  onWindowsDirty,
  onWindowsSaved,
  calculator,
  declHistory,
  calendarPolicyLabel,
  cancelPositionEdits,
  concentrationSummary,
  dateProvenanceLabel,
  expectedDeclarationLookback,
  factsDirty,
  fileWorkTicket,
  fixRemainingYearTicket,
  filteredPositionSymbols,
  formatBps,
  formatHubPrice,
  investment,
  lastPriceBusy,
  lastPriceProgress,
  lastPriceRefreshLabel,
  leaveWithoutSaving,
  loadInvestment,
  metricHint,
  mostCurrentVsPlanFace,
  openRecreateAdapter,
  ownerRiskChoice,
  patchDraft,
  patchPeriod,
  pdDirty,
  pdDraft,
  pdLedger,
  pdPeriod,
  pdRemaining,
  periodDirty,
  periodReady,
  positionDetails,
  positionFocusPanel,
  positionMaster,
  positionSymbol,
  positionSymbolOpen,
  positionSymbolQuery,
  receivedByYear,
  refreshLastPrices,
  refreshPdDeclarations,
  refreshPdLastPrice,
  researchActivity,
  researchNotes,
  researchPdRoc,
  resolveConfirmTicket,
  resolveEnterDeclaredAmount,
  resolveTicketRetry,
  resolveTicketRocContext,
  storeTicketRocUrl,
  storeTicketManualRoc,
  openTicketRocResearch,
  retryingTicket,
  rocActualUnavailable,
  rocResearchLabel,
  rocResearchUpdated,
  saveOwnerPeriod,
  saveStoredFacts,
  savedPeriodId,
  scaledDollars,
  selectPositionSymbol,
  setOwnerRisk,
  setOwnerRiskChoice,
  setPositionFocusPanel,
  setPositionSymbol,
  setPositionSymbolOpen,
  setPositionSymbolQuery,
  setScreen,
  strategyCharacteristics,
  ticketDecision,
  workTickets,
  writesBlocked,
}: PositionDetailsScreenProps) {
  return (
      <section aria-label="Position Details">
        <h2>Position Details</h2>
        {investment ? (
          <p aria-label="Collector gaps">
            {(investment.collectorGaps ?? []).length === 0
              ? "No collector gaps."
              : `Open: ${investment.collectorGaps.join(", ")}`}
          </p>
        ) : null}
        <p>Symbol, Plan, and the pays that support it.</p>
        <div className="buttons">
          <button
            type="button"
            aria-label="Refresh last prices"
            aria-busy={lastPriceBusy}
            disabled={lastPriceBusy || busy}
            onClick={() => void refreshLastPrices(true)}
          >
            {lastPriceBusy
              ? lastPriceRefreshLabel(lastPriceProgress)
              : "Refresh last prices"}
          </button>
          <button
            type="button"
            aria-label="Apply issuer sources from provider"
            disabled={busy || writesBlocked}
            onClick={() => void applyIssuerSources()}
          >
            Apply issuer sources from provider
          </button>
        </div>
        <p className="field-caption">
          Fills an empty, public, or unassigned declaration source from the stored
          provider. A registered issuer is left alone.
        </p>
        <label className="symbol-combobox">
          Symbol
          <input
            aria-label="Position symbol"
            aria-expanded={positionSymbolOpen}
            aria-controls="position-symbol-list"
            aria-autocomplete="list"
            role="combobox"
            value={positionSymbolQuery}
            onChange={(e) => {
              setPositionSymbolQuery(e.target.value.toUpperCase());
              setPositionSymbolOpen(true);
            }}
            onFocus={() => setPositionSymbolOpen(true)}
            onBlur={() => {
              window.setTimeout(() => setPositionSymbolOpen(false), 150);
            }}
            onKeyDown={(e) => {
              if (e.key === "Enter" && filteredPositionSymbols[0]) {
                e.preventDefault();
                selectPositionSymbol(filteredPositionSymbols[0]);
              }
              if (e.key === "Escape") {
                setPositionSymbolOpen(false);
              }
            }}
          />
          {positionSymbolOpen && filteredPositionSymbols.length > 0 ? (
            <ul
              id="position-symbol-list"
              className="symbol-combobox-list"
              role="listbox"
              aria-label="Position symbols"
            >
              {filteredPositionSymbols.slice(0, 30).map((sym) => (
                <li
                  key={sym}
                  role="option"
                  aria-selected={sym === positionSymbol}
                  onMouseDown={(e) => {
                    e.preventDefault();
                    selectPositionSymbol(sym);
                  }}
                >
                  {sym}
                </li>
              ))}
            </ul>
          ) : null}
        </label>
        {investment && pdDraft ? (
          <>
            {pdDirty ? (
              <p className="blocked" role="status">
                Unsaved edits. Save or Cancel — other screens stay blocked.
              </p>
            ) : null}
            <div className="buttons dossier-actions">
              <button
                type="button"
                aria-label="Save stored facts"
                className={pdDirty ? "is-unsaved" : undefined}
                disabled={busy || writesBlocked || !pdDirty}
                onClick={() => void saveStoredFacts()}
              >
                Save
              </button>
              <button
                type="button"
                aria-label="Cancel position edits"
                disabled={busy || !pdDirty}
                onClick={() => cancelPositionEdits()}
              >
                Cancel
              </button>
              <button
                type="button"
                aria-label="Refresh last price for this symbol"
                disabled={busy || writesBlocked}
                onClick={() => void refreshPdLastPrice()}
              >
                Refresh last price
              </button>
              <button
                type="button"
                aria-label="Retrieve declarations for this symbol"
                disabled={busy || writesBlocked}
                onClick={() => void refreshPdDeclarations()}
              >
                Retrieve declarations
              </button>
              <button
                type="button"
                aria-label="Complete research"
                disabled={
                  busy ||
                  writesBlocked ||
                  !investment.securityId ||
                  Boolean(researchActivity?.running)
                }
                onClick={() => void researchPdRoc()}
              >
                {researchActivity?.running ? "Completing research…" : "Complete research"}
              </button>
            </div>
            <p className="field-caption">
              Retrieves declarations and drafts a tier suggestion for a position that
              already exists. It does not store the Plan.
            </p>

            <section
              className="hub-panel"
              id="hub-plan"
              aria-label="Plan Management"
              data-focus={positionFocusPanel === "plan" ? "1" : undefined}
            >
              <h3>Plan Management</h3>
              {(() => {
                const master = (positionMaster?.rows ?? []).find(
                  (row) => row.symbol === investment.symbol,
                );
                const historyRow = (declHistory?.rows ?? []).find(
                  (row) => row.symbol === investment.symbol,
                );
                const cells = historyRow?.cells ?? [];
                const pays = historyRow?.recentPays ?? [];
                const inForce = historyRow?.inForcePays ?? [];
                const payCount = pays.filter((pay) => pay.amountPerShareMinor != null).length;
                const avg = meanNewestPays(pays, 6, true);
                const avg3 = meanNewestPays(pays, 3);
                const low = minPaidDeclaration(cells);
                const current = newestStoredPay(pays);
                const score = dividendScore(master, cells, current, inForce);
                const holdingQty = (positionDetails?.positions ?? [])
                  .filter((row) => row.symbol === investment.symbol)
                  .reduce(
                    (sum, row) => sum + Math.max(0, row.remainingQuantityMinor ?? 0),
                    0,
                  );
                const sharesMinor =
                  (master?.remainingQuantityMinor ?? investment.remainingQuantityMinor ?? 0) > 0
                    ? (master?.remainingQuantityMinor ?? investment.remainingQuantityMinor)
                    : holdingQty;
                const planKnown = master?.planKnown ?? investment.planKnown;
                const planMinor = master?.planPerShareMinor ?? investment.planPerShareMinor ?? 0;
                const planScale = master?.planScale ?? investment.planScale ?? 2;
                const check = planCheck(inForce, planKnown, planMinor, planScale);
                const typed = parseTypedPlan(pdDraft?.plan ?? "");
                const periods =
                  master?.planningPeriodsPerYear ?? investment.planningPeriodsPerYear ?? 0;
                const price = investment.price;
                const fwd = planFwdAtAmountBps(
                  typed?.minor ?? null,
                  typed?.scale ?? planScale,
                  periods,
                  price?.priceDerivedValid ? price.priceMinor : null,
                  price?.scale ?? 2,
                );
                const impact = planDecisionImpact({
                  currentMinor: planKnown ? planMinor : null,
                  currentScale: planScale,
                  nextMinor: typed?.minor ?? null,
                  nextScale: typed?.scale ?? planScale,
                  quantityMinor: sharesMinor,
                  quantityScale: master?.quantityScale ?? investment.quantityScale ?? 0,
                  periods,
                  cells: inForce,
                });
                const money = (cents) =>
                  cents == null ? "unknown" : formatUsd(cents, 2);
                return (
                  <>
                    {master || sharesMinor > 0 ? null : (
                      <p>No Calculator row yet (needs Plan + first lot).</p>
                    )}
                    <p className="field-caption">Save stored facts stores this amount.</p>
                    <div className="fact-grid" aria-label="Plan Management figures">
                      <div>
                        <span className="fact-label">Plan amount</span>
                        <input
                          aria-label="Plan amount"
                          value={pdDraft.plan}
                          onChange={(e) => patchDraft({ plan: e.target.value })}
                          disabled={busy || writesBlocked}
                        />
                      </div>
                      <div>
                        <span className="fact-label">Plan reason</span>
                        <select
                          aria-label="Stored Plan decision reason"
                          value={pdDraft.planReason}
                          onChange={(e) => {
                            const planReason = e.target.value;
                            if (
                              planReason === "Match Most Current" &&
                              investment.review.mostCurrentMinor != null
                            ) {
                              patchDraft({
                                planReason,
                                plan: scaledDollars(
                                  investment.review.mostCurrentMinor,
                                  investment.review.amountScale,
                                ),
                              });
                              return;
                            }
                            if (planReason === "Match Avg 6 (owner typed)" && avg != null) {
                              patchDraft({
                                planReason,
                                plan: scaledDollars(avg, 2),
                              });
                              return;
                            }
                            patchDraft({ planReason });
                          }}
                          disabled={busy || writesBlocked}
                        >
                          <option value="">Choose reason</option>
                          {PLAN_REASONS.map((reason) => (
                            <option key={reason} value={reason}>
                              {reason}
                            </option>
                          ))}
                        </select>
                      </div>
                      {investment.review.incompleteReasonRequired ? (
                        <div>
                          <span className="fact-label">Incomplete analysis</span>
                          <select
                            aria-label="Stored incomplete analysis reason"
                            value={pdDraft.incomplete}
                            onChange={(e) => patchDraft({ incomplete: e.target.value })}
                            disabled={busy || writesBlocked}
                          >
                            <option value="">Choose why full analysis is not possible</option>
                            {INCOMPLETE_REASONS.map((reason) => (
                              <option key={reason} value={reason}>
                                {reason}
                              </option>
                            ))}
                          </select>
                        </div>
                      ) : null}
                      <div>
                        <span className="fact-label">FWD at this amount</span>
                        <span className="fact-value">{formatBps(fwd)}</span>
                      </div>
                      <div>
                        <span className="fact-label">Schedule</span>
                        <span className="fact-value">
                          {master?.paymentFrequency ||
                            investment.paymentFrequency ||
                            historyRow?.paymentFrequency ||
                            "—"}
                        </span>
                      </div>
                      <div>
                        <span className="fact-label">Plan / share</span>
                        <span className="fact-value">
                          {planKnown ? `$${formatScaled(planMinor, planScale)}` : "N/A"}
                        </span>
                      </div>
                      <div>
                        <span className="fact-label">Most current</span>
                        <span className="fact-value">
                          {current?.amountPerShareMinor == null
                            ? "unknown"
                            : `$${formatScaled(current.amountPerShareMinor, current.amountScale)}`}
                        </span>
                      </div>
                      <div>
                        <span className="fact-label">Avg 3</span>
                        <span className="fact-value">
                          {avg3 == null
                            ? `${Math.min(payCount, 3)} of 3, unknown`
                            : formatUsd(avg3, 2)}
                        </span>
                      </div>
                      <div>
                        <span className="fact-label">Avg 6</span>
                        <span className="fact-value">
                          {avg == null
                            ? "unknown"
                            : `${formatUsd(avg, 2)} (${Math.min(payCount, 6)} of 6)`}
                        </span>
                      </div>
                      <div>
                        <span className="fact-label">Min pay</span>
                        <span className="fact-value">{low == null ? "unknown" : formatUsd(low, 2)}</span>
                      </div>
                      <div>
                        <span className="fact-label">Plan pay</span>
                        <span className="fact-value">{money(score.planPay)}</span>
                      </div>
                      <div>
                        <span className="fact-label">Current pay</span>
                        <span className="fact-value">{money(score.currentPay)}</span>
                      </div>
                      <div>
                        <span className="fact-label">Plan Δ</span>
                        <span className="fact-value">{money(score.planDelta)}</span>
                      </div>
                      <div>
                        <span className="fact-label">Over/Under</span>
                        <span className="fact-value">{formatBps(score.overUnder)}</span>
                      </div>
                      <div>
                        <span className="fact-label">Plan check</span>
                        <span className="fact-value">{check.text}</span>
                      </div>
                      <div>
                        <span className="fact-label">Per-share change</span>
                        <span className="fact-value">{formatScale6(impact.perShareDeltaUnits)}</span>
                      </div>
                      <div>
                        <span className="fact-label">One-payment change</span>
                        <span className="fact-value">{formatScale6(impact.paymentDeltaUnits)}</span>
                      </div>
                      <div>
                        <span className="fact-label">Annual change</span>
                        <span className="fact-value">{formatScale6(impact.annualDeltaUnits)}</span>
                      </div>
                      <div>
                        <span className="fact-label">Plan check for typed amount</span>
                        <span className="fact-value">{impact.checkText}</span>
                      </div>
                      <div className="fact-wide">
                        <span className="fact-label">Last 6 declarations</span>
                        <span className="fact-value">
                          {pays.length === 0
                            ? "unknown"
                            : pays
                                .map(
                                  (pay) =>
                                    `$${formatScaled(pay.amountPerShareMinor, pay.amountScale)}`,
                                )
                                .join(", ")}
                        </span>
                      </div>
                    </div>
                  </>
                );
              })()}
            </section>
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
                    value={Math.min(
                      researchActivity.step,
                      researchActivity.total,
                    )}
                  />
                ) : (
                  <div
                    className="process-a-research-spinner"
                    aria-hidden="true"
                  />
                )}
              </section>
            ) : null}
            {researchActivity?.resultLine && !researchActivity.running ? (
              <p role="status" aria-label="Research result">
                {researchActivity.resultLine}
              </p>
            ) : null}
            {(() => {
              const finished =
                Boolean(researchActivity?.resultLine) &&
                !researchActivity?.running &&
                !(researchActivity?.resultLine || "").startsWith(
                  "Complete research failed",
                );
              const rawNotes = (
                pdDraft?.notes ||
                investment.notes ||
                ""
              ).trim();
              const notesAreRocStub = /^roc source:/i.test(rawNotes);
              const storedNotes = notesAreRocStub ? "" : rawNotes;
              const storedTier = RISK_TIERS.includes(investment.riskTier)
                ? investment.riskTier
                : "";
              const rocUrl = (investment.rocEstimateSourceUrl || "").trim();
              const rocPct =
                investment.rocPct2026EstimateMinor != null &&
                investment.rocScale != null
                  ? formatPercentScaled(
                      investment.rocPct2026EstimateMinor,
                      investment.rocScale,
                    )
                  : "unknown";
              const rocMethod = (investment.rocEstimateMethod || "").trim();
              const ownerInput = rocMethod
                ? rocPct === "unknown"
                  ? rocMethod
                  : `${rocMethod} · ${rocPct}`
                : rocPct;
              const asOfYear = Number(String(asOfDate || "").slice(0, 4));
              const rocMinorFor = (year, kind) => {
                if (kind === "actual" && year === 2024) return investment.rocPct2024ActualMinor;
                if (kind === "actual" && year === 2025) return investment.rocPct2025ActualMinor;
                if (kind === "actual" && year === 2026) return investment.rocPct2026ActualMinor;
                if (kind === "estimate" && year === 2026) return investment.rocPct2026EstimateMinor;
                return null;
              };
              const rocYearFace = (year, minor) => {
                if (!Number.isFinite(year) || minor == null || investment.rocScale == null) {
                  return "unknown";
                }
                return `${year} ${formatPercentScaled(minor, investment.rocScale)}`;
              };
              const previousYear = asOfYear - 1;
              const currentMinor =
                rocMinorFor(asOfYear, "actual") ?? rocMinorFor(asOfYear, "estimate");
              const previousFace = rocYearFace(previousYear, rocMinorFor(previousYear, "actual"));
              const currentFace = rocYearFace(asOfYear, currentMinor);
              if (
                !researchNotes &&
                !storedNotes &&
                !finished &&
                !storedTier &&
                !rocUrl &&
                !rocMethod &&
                investment.rocPct2026EstimateMinor == null &&
                investment.suggestion == null
              ) {
                return null;
              }
              return (
              <section
                className="hub-panel research-notes-panel"
                aria-label="Research notes"
              >
                <div className="fact-strip" aria-label="Assigned tier">
                  <div>
                    <span className="fact-label">Assigned tier</span>
                    <select
                      aria-label="Owner risk choice"
                      value={ownerRiskChoice}
                      onChange={(e) => {
                        const tier = e.target.value;
                        setOwnerRiskChoice(tier);
                        if (RISK_TIERS.includes(tier)) void setOwnerRisk(tier);
                      }}
                      disabled={busy || writesBlocked}
                    >
                      <option value="">Choose owner tier</option>
                      {RISK_TIERS.map((tier) => (
                        <option key={tier} value={tier}>
                          {tier}
                        </option>
                      ))}
                    </select>
                  </div>
                </div>
                <p className="field-caption">
                  {storedTier
                    ? `Tier is stored as ${storedTier}.`
                    : "Choose Foundation, Core, or Risk On."}
                </p>
                <p>
                  {investment.suggestion == null || !investment.suggestion.complete
                    ? investment.suggestion?.reason ?? "No complete suggestion."
                    : `${investment.suggestion.suggestedTier} (${investment.suggestion.ruleset}): ${investment.suggestion.reason}`}
                </p>
                <div className="fact-strip roc-facts" aria-label="ROC">
                  <div>
                    <span className="fact-label">ROC %</span>
                    <span className="fact-value">{rocPct}</span>
                  </div>
                  <div>
                    <span className="fact-label">ROC link</span>
                    <span className="fact-value">
                      {rocUrl ? (
                        <a href={rocUrl}>ROC URL Data</a>
                      ) : (
                        "unknown"
                      )}
                    </span>
                  </div>
                  <div>
                    <span className="fact-label">Source</span>
                    <span className="fact-value">
                      {(investment.rocEstimateEstablishedHow || ownerInput || "").trim() ||
                        "unknown"}
                    </span>
                  </div>
                </div>
                <div className="fact-strip roc-facts" aria-label="ROC years">
                  <div>
                    <span className="fact-label">ROC previous year</span>
                    <span className="fact-value">{previousFace}</span>
                  </div>
                  <div>
                    <span className="fact-label">ROC current year</span>
                    <span className="fact-value">{currentFace}</span>
                  </div>
                </div>
                {researchNotes?.source && researchNotes.source !== "stored notes" ? (
                  <p className="muted">Source: {researchNotes.source}</p>
                ) : null}
              </section>
              );
            })()}
            <dl className="hub-hero" aria-label="Position hub summary">
              <div>
                <dt>Symbol</dt>
                <dd>{investment.symbol}</dd>
              </div>
              <div>
                <dt>Market value</dt>
                <dd>
                  {investment.marketValueMinor == null
                    ? "unknown"
                    : formatUsd(investment.marketValueMinor, investment.scale)}
                </dd>
              </div>
              <div>
                <dt>Plan annual</dt>
                <dd>
                  {investment.annualPlanMinor == null
                    ? "unknown"
                    : formatUsd(investment.annualPlanMinor, investment.scale)}
                </dd>
              </div>
              <div>
                <dt>Underlying</dt>
                <dd>{investment.underlying?.trim() || "—"}</dd>
              </div>
              <div>
                <dt>Plan YOC</dt>
                <dd>{formatBps(investment.planYocBps ?? null)}</dd>
              </div>
              <div>
                <dt>Price</dt>
                <dd>{formatHubPrice(investment.price, investment.scale)}</dd>
              </div>
              <div>
                <dt>Total distributions</dt>
                <dd>
                  {investment.distributionsScope === "incomplete" ||
                  investment.totalDistributionsReceivedMinor == null
                    ? "unknown"
                    : formatUsd(
                        investment.totalDistributionsReceivedMinor,
                        investment.scale,
                      )}
                </dd>
              </div>
              <div>
                <dt>ROC component (Car, current year)</dt>
                <dd>
                  {investment.rocDistributionsMinor == null
                    ? "unknown"
                    : formatUsd(investment.rocDistributionsMinor, investment.scale)}
                </dd>
              </div>
              <div>
                <dt>Cost recovery</dt>
                <dd>{formatBps(investment.costRecoveryBps ?? null)}</dd>
              </div>
              {(() => {
                const master = (positionMaster?.rows ?? []).find(
                  (r) => r.symbol === investment.symbol,
                );
                if (!master) return null;
                return (
                  <div>
                    <dt title="Share of total data portfolio market value">Portfolio %</dt>
                    <dd>{formatBps(master.allocationBps)}</dd>
                  </div>
                );
              })()}
            </dl>
            <div id="hub-tickets" data-focus={positionFocusPanel === "tickets" ? "1" : undefined}>
            <WorkTicketQueue
              tickets={workTickets}
              filterSymbol={investment.symbol}
              retryingTicketId={retryingTicket?.ticketId}
              retryingSymbol={retryingTicket?.symbol}
              pendingTicketId={ticketDecision?.ticketId}
              pendingAction={ticketDecision?.action}
              resolveRocContext={(t) =>
                resolveTicketRocContext(t as WorkTicketRecord)
              }
              onStoreRocUrl={(t, url) =>
                storeTicketRocUrl(t as WorkTicketRecord, url)
              }
              onStoreManualRoc={(t, pct) =>
                void storeTicketManualRoc(t as WorkTicketRecord, pct)
              }
              onOpenRocResearch={(t) =>
                openTicketRocResearch(t as WorkTicketRecord)
              }
              onRetry={(t) => void resolveTicketRetry(t as WorkTicketRecord)}
              onRecreateAdapter={(t) =>
                openRecreateAdapter(t as WorkTicketRecord)
              }
              onFixRemainingYear={(t) =>
                void fixRemainingYearTicket(t as WorkTicketRecord)
              }
              onExcept={(t) =>
                resolveConfirmTicket(t as WorkTicketRecord, "positive")
              }
              onReject={(t) =>
                resolveConfirmTicket(t as WorkTicketRecord, "reject")
              }
              onEnterAmount={(t, amount) =>
                void resolveEnterDeclaredAmount(t as WorkTicketRecord, amount)
              }
              onFile={(t) => void fileWorkTicket(t as WorkTicketRecord)}
            />
            </div>
            <nav className="hub-toc" aria-label="Position hub sections">
              <a href="#hub-plan">Plan Management</a>
              <a href="#hub-identity">Position information</a>
              <a href="#hub-calculator">Calculator</a>
              <a href="#hub-plan-yields">Plan</a>
              <a href="#hub-income">Payment summary</a>
              <a href="#hub-pay-dates">Pay dates</a>
              <a href="#hub-received">Received</a>
              <a href="#hub-declarations">Declarations</a>
              <a href="#hub-accounts">By account</a>
              <a href="#hub-lots">Lots</a>
              <a href="#hub-ledger">Ledger</a>
            </nav>
            <section className="hub-panel" id="hub-identity" aria-label="Position information">
              <h3>Position information</h3>
              <p>
                Owner facts edit in this table. Risk is mandatory: Foundation, Core,
                or Risk On. Frequency is required. Save or Cancel.
              </p>
              <div className="table-wrap">
                <table className="two-col-facts" aria-label="Position information">
                  <thead>
                    <tr>
                      <th scope="col">Fact</th>
                      <th scope="col">Value</th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr>
                      <th scope="row">Symbol</th>
                      <td>{investment.symbol}</td>
                    </tr>
                    <tr>
                      <th scope="row">Name</th>
                      <td>
                        <input
                          aria-label="Position name"
                          value={pdDraft.name}
                          onChange={(e) => patchDraft({ name: e.target.value })}
                          disabled={busy || writesBlocked}
                        />
                      </td>
                    </tr>
                    <tr>
                      <th scope="row">Provider</th>
                      <td>
                        <input
                          aria-label="Position provider"
                          value={pdDraft.provider}
                          onChange={(e) => patchDraft({ provider: e.target.value })}
                          disabled={busy || writesBlocked}
                        />
                      </td>
                    </tr>
                    <tr>
                      <th scope="row">Underlying</th>
                      <td>
                        <input
                          aria-label="Position underlying"
                          value={pdDraft.underlying}
                          onChange={(e) => patchDraft({ underlying: e.target.value })}
                          disabled={busy || writesBlocked}
                        />
                      </td>
                    </tr>
                    <tr>
                      <th scope="row">Frequency</th>
                      <td>
                        <select
                          aria-label="Position frequency"
                          value={pdDraft.freq}
                          onChange={(e) => patchDraft({ freq: e.target.value })}
                          disabled={busy || writesBlocked}
                        >
                          <option value="">Choose cadence</option>
                          <option value="Weekly">Weekly (52)</option>
                          <option value="Twice monthly">Twice monthly (24)</option>
                          <option value="Monthly">Monthly (12)</option>
                          <option value="Quarterly">Quarterly (4)</option>
                          <option value="None">None (does not pay)</option>
                        </select>
                      </td>
                    </tr>
                    <tr>
                      <th scope="row">ROC research</th>
                      <td aria-label="Position ROC research status">
                        {rocResearchLabel(investment)}
                      </td>
                    </tr>
                    <tr>
                      <th scope="row">ROC last update</th>
                      <td>{rocResearchUpdated(investment)}</td>
                    </tr>
                  </tbody>
                </table>
              </div>
            </section>
            <section className="hub-panel" id="hub-calculator" aria-label="Calculator snapshot">
              <h3>Calculator snapshot</h3>
              {(() => {
                const calc = (calculator?.rows ?? []).find(
                  (r) => r.symbol === investment.symbol,
                );
                const holdingQty = (positionDetails?.positions ?? [])
                  .filter((row) => row.symbol === investment.symbol)
                  .reduce(
                    (sum, row) => sum + Math.max(0, row.remainingQuantityMinor ?? 0),
                    0,
                  );
                const sharesMinor =
                  investment.remainingQuantityMinor > 0
                    ? investment.remainingQuantityMinor
                    : holdingQty;
                if (!calc) {
                  const freq = (investment.paymentFrequency || "").trim().toLowerCase();
                  if (freq === "none") {
                    return (
                      <p>Cadence is does not pay, so this position stays off the Calculator.</p>
                    );
                  }
                  if (!(sharesMinor > 0) && !investment.planKnown) {
                    return (
                      <p>The Calculator query has neither open shares nor a Plan.</p>
                    );
                  }
                  if (!(sharesMinor > 0)) {
                    return (
                      <p>
                        No Calculator row yet (needs Plan + first lot). Holdings and
                        remaining-year panels below still apply.
                      </p>
                    );
                  }
                  return (
                    <p>
                      Open shares or a Plan, and the cadence pays, so this position
                      belongs on the Calculator. The loaded Calculator rows do not include it.
                    </p>
                  );
                }
                return (
                  <div className="table-wrap">
                    <table aria-label="Calculator snapshot">
                      <thead>
                        <tr>
                          <th scope="col">Metric</th>
                          <th className="numeric" scope="col">Value</th>
                        </tr>
                      </thead>
                      <tbody>
                        <tr>
                          <th scope="row">Frequency</th>
                          <td className="numeric">{calc.paymentFrequency || "—"}</td>
                        </tr>
                        <tr>
                          <th scope="row">Plan / share</th>
                          <td className="numeric">
                            {calc.planKnown
                              ? `$${formatScaled(calc.planPerShareMinor, calc.planScale)}`
                              : "N/A"}
                          </td>
                        </tr>
                        <tr>
                          <th scope="row">Periods / year</th>
                          <td className="numeric">
                            {calc.planningPeriodsPerYear > 0
                              ? formatCount(calc.planningPeriodsPerYear)
                              : "N/A"}
                          </td>
                        </tr>
                        <tr>
                          <th scope="row">Open quantity</th>
                          <td className="numeric">
                            {formatScaled(calc.remainingQuantityMinor, calc.quantityScale)}
                          </td>
                        </tr>
                        <tr>
                          <th scope="row">Plan payment</th>
                          <td className="numeric">
                            {calc.planKnown
                              ? formatUsd(calc.planPaymentMinor, calc.scale)
                              : "N/A"}
                          </td>
                        </tr>
                        <tr>
                          <th scope="row">Original cost</th>
                          <td className="numeric">
                            {formatUsd(calc.remainingPerformanceMinor, calc.scale)}
                          </td>
                        </tr>
                        <tr>
                          <th scope="row">Last price</th>
                          <td className="numeric">
                            {calc.lastPriceMinor == null
                              ? "unknown"
                              : formatUsd(
                                  calc.lastPriceMinor,
                                  calc.lastPriceScale ?? calc.scale,
                                )}
                          </td>
                        </tr>
                        <tr>
                          <th scope="row">Price freshness</th>
                          <td className="numeric">{calc.priceFreshness || "unavailable"}</td>
                        </tr>
                        <tr>
                          <th scope="row">Market value</th>
                          <td className="numeric">
                            {calc.marketValueMinor == null
                              ? "unknown"
                              : formatUsd(calc.marketValueMinor, calc.scale)}
                          </td>
                        </tr>
                        <tr>
                          <th scope="row">ROC 2025</th>
                          <td className="numeric">
                            {rocActualUnavailable(
                              investment.lots,
                              2025,
                              asOfDate,
                            ) ??
                              (calc.rocPct2025ActualMinor == null ||
                              calc.rocScale == null
                                ? "missing-1099"
                                : formatPercentScaled(
                                    calc.rocPct2025ActualMinor,
                                    calc.rocScale,
                                  ))}
                          </td>
                        </tr>
                        <tr>
                          <th scope="row">ROC 2026 estimate</th>
                          <td className="numeric">
                            {calc.rocPct2026EstimateMinor == null || calc.rocScale == null
                              ? "unknown"
                              : formatPercentScaled(
                                  calc.rocPct2026EstimateMinor,
                                  calc.rocScale,
                                )}
                          </td>
                        </tr>
                        <tr>
                          <th scope="row">ROC 2026 actual</th>
                          <td className="numeric">
                            {rocActualUnavailable(
                              investment.lots,
                              2026,
                              asOfDate,
                            ) ??
                              (calc.rocPct2026ActualMinor == null ||
                              calc.rocScale == null
                                ? "missing-1099"
                                : formatPercentScaled(
                                    calc.rocPct2026ActualMinor,
                                    calc.rocScale,
                                  ))}
                          </td>
                        </tr>
                      </tbody>
                    </table>
                  </div>
                );
              })()}
            </section>
            <section className="hub-panel" id="hub-plan-yields" aria-label="Plan and yields">
              <h3>Plan and yields</h3>
              <div className="table-wrap">
                <table className="two-col-facts" aria-label="Plan and yields">
                  <thead>
                    <tr>
                      <th scope="col">Metric</th>
                      <th scope="col">Value</th>
                    </tr>
                  </thead>
                  <tbody>
                    {(() => {
                      const planShare = investment.planKnown
                        ? `$${formatScaled(investment.planPerShareMinor, investment.planScale)}`
                        : "unknown";
                      const periods =
                        investment.planningPeriodsPerYear > 0
                          ? formatCount(investment.planningPeriodsPerYear)
                          : "unknown";
                      const qty = formatScaled(
                        investment.remainingQuantityMinor,
                        investment.quantityScale,
                      );
                      const cost = formatUsd(
                        investment.remainingPerformanceMinor,
                        investment.scale,
                      );
                      const price =
                        investment.price?.priceMinor == null
                          ? "unknown"
                          : formatUsd(
                              investment.price.priceMinor,
                              investment.price.scale ?? investment.scale,
                            );
                      const mostCurrent =
                        investment.review.mostCurrentMinor == null
                          ? "unknown"
                          : `$${formatScaled(
                              investment.review.mostCurrentMinor,
                              investment.review.amountScale,
                            )}`;
                      const annual =
                        investment.annualPlanMinor == null
                          ? "unknown"
                          : formatUsd(investment.annualPlanMinor, investment.scale);
                      return (
                        <>
                          <tr>
                            <th
                              scope="row"
                              className="metric-hint"
                              {...metricHint(
                                "Plan / share",
                                `Owner PlanHistory $/share (never auto from declarations). Last adjusted ${
                                  investment.planEffectiveFrom?.trim() || "unknown"
                                }.`,
                              )}
                            >
                              Plan / share
                            </th>
                            <td>
                              {investment.planKnown
                                ? `$${formatScaled(investment.planPerShareMinor, investment.planScale)}`
                                : "unknown"}
                              {investment.planEffectiveFrom?.trim()
                                ? ` — last adjusted ${investment.planEffectiveFrom}`
                                : ""}
                            </td>
                          </tr>
                          <tr>
                            <th
                              scope="row"
                              className="metric-hint"
                              {...metricHint(
                                "Annual plan",
                                `Plan/share × open qty × periods/year. Inputs: ${planShare} × ${qty} × ${periods} → ${annual}.`,
                              )}
                            >
                              Annual plan
                            </th>
                            <td>
                              {investment.annualPlanMinor == null
                                ? "unknown"
                                : formatUsd(
                                    investment.annualPlanMinor,
                                    investment.scale,
                                  )}
                            </td>
                          </tr>
                          <tr>
                            <th
                              scope="row"
                              className="metric-hint"
                              {...metricHint(
                                "Plan YOC",
                                `Annual plan $ ÷ original economic cost. Inputs: ${annual} ÷ ${cost}.`,
                              )}
                            >
                              Plan YOC
                            </th>
                            <td>{formatBps(investment.planYocBps ?? null)}</td>
                          </tr>
                          <tr>
                            <th
                              scope="row"
                              className="metric-hint"
                              {...metricHint(
                                "Plan FWD",
                                `(Plan/share × periods) ÷ last price. Inputs: (${planShare} × ${periods}) ÷ ${price}.`,
                              )}
                            >
                              Plan FWD
                            </th>
                            <td>
                              {formatBps(investment.planFwdYieldBps ?? null)}
                            </td>
                          </tr>
                          <tr>
                            <th
                              scope="row"
                              className="metric-hint"
                              {...metricHint(
                                "Most Current",
                                `Latest paid issuer declaration $/share (Plan-review). This position: ${mostCurrent}.`,
                              )}
                            >
                              Most Current
                            </th>
                            <td>
                              {investment.review.mostCurrentMinor == null
                                ? "unknown"
                                : `$${formatScaled(
                                    investment.review.mostCurrentMinor,
                                    investment.review.amountScale,
                                  )}`}
                            </td>
                          </tr>
                          <tr>
                            <th
                              scope="row"
                              className="metric-hint"
                              {...metricHint(
                                "Avg 6",
                                "Mean of up to 6 most recent paid declarations (blank weeks are not $0).",
                              )}
                            >
                              Avg 6
                            </th>
                            <td>
                              {investment.review.avg6Minor == null
                                ? "unknown"
                                : `$${formatScaled(
                                    investment.review.avg6Minor,
                                    investment.review.amountScale,
                                  )}`}
                            </td>
                          </tr>
                          <tr>
                            <th
                              scope="row"
                              className="metric-hint"
                              {...metricHint(
                                "Most Current FWD",
                                `(Most Current × periods) ÷ last price. Inputs: (${mostCurrent} × ${periods}) ÷ ${price}.`,
                              )}
                            >
                              Most Current FWD
                            </th>
                            <td>
                              {formatBps(
                                investment.mostCurrentFwdYieldBps ?? null,
                              )}
                            </td>
                          </tr>
                          <tr>
                            <th
                              scope="row"
                              className="metric-hint"
                              {...metricHint(
                                "Most Current vs Plan",
                                `(Most Current − Plan) ÷ Plan → Above / Equal / Below. Inputs: (${mostCurrent} − ${planShare}) ÷ ${planShare}.`,
                              )}
                            >
                              Most Current vs Plan
                            </th>
                            <td>
                              {mostCurrentVsPlanFace(
                                investment.mostCurrentVsPlanBps ?? null,
                              )}
                            </td>
                          </tr>
                        </>
                      );
                    })()}
                  </tbody>
                </table>
              </div>
            </section>
            <section className="hub-panel" id="hub-accounts" aria-label="Holdings by account">
              <h3>Holdings by account</h3>
              <p>
                Open quantity and cost for this symbol by control account. Market value is
                qty × last price when the price is valid. P&amp;L is market value minus tax
                basis.
              </p>
              {(() => {
                const rows = (positionDetails?.positions ?? []).filter(
                  (r) => r.symbol === investment.symbol,
                );
                if (rows.length === 0) {
                  return <p>No open account splits for this symbol.</p>;
                }
                const price =
                  investment.price?.priceDerivedValid && investment.price.priceMinor != null
                    ? {
                        minor: investment.price.priceMinor,
                        scale: investment.price.scale ?? investment.scale,
                      }
                    : null;
                return (
                  <div className="table-wrap">
                    <table aria-label="Holdings by account">
                      <thead>
                        <tr>
                          <th scope="col">Account</th>
                          <th className="numeric" scope="col">Qty</th>
                          <th className="numeric" scope="col">Lots</th>
                          <th className="numeric" scope="col">Original cost</th>
                          <th className="numeric" scope="col">Tax basis</th>
                          <th className="numeric" scope="col">Market value</th>
                          <th className="numeric" scope="col">P&amp;L</th>
                        </tr>
                      </thead>
                      <tbody>
                        {rows.map((row) => {
                          let mv: string = "unknown";
                          let pnl: string = "unknown";
                          if (price && row.remainingQuantityMinor > 0) {
                            const qScale = row.quantityScale ?? 0;
                            const mvMinor = Math.trunc(
                              (row.remainingQuantityMinor * price.minor) /
                                10 ** qScale,
                            );
                            mv = formatUsd(mvMinor, price.scale);
                            pnl = formatUsd(
                              mvMinor - row.remainingTaxMinor,
                              row.scale,
                            );
                          }
                          return (
                            <tr key={`${row.accountId}-${row.symbol}`}>
                              <td>{row.accountName}</td>
                              <td className="numeric">
                                {formatScaled(
                                  row.remainingQuantityMinor,
                                  row.quantityScale,
                                )}
                              </td>
                              <td className="numeric">{formatCount(row.lotCount)}</td>
                              <td className="numeric">
                                {formatUsd(row.remainingPerformanceMinor, row.scale)}
                              </td>
                              <td className="numeric">
                                {formatUsd(row.remainingTaxMinor, row.scale)}
                              </td>
                              <td className="numeric">{mv}</td>
                              <td className="numeric">{pnl}</td>
                            </tr>
                          );
                        })}
                      </tbody>
                    </table>
                  </div>
                );
              })()}
            </section>
            <details className="hub-panel" aria-label="Identity and holdings facts">
            <summary>Identity and edit facts (name, provider, template)</summary>
            <h3>Identity and economics</h3>
            <div className="table-wrap">
              <table className="two-col-facts" aria-label="Position dossier">
                <thead>
                  <tr>
                    <th scope="col">Fact</th>
                    <th scope="col">Value</th>
                    <th scope="col">Kind</th>
                  </tr>
                </thead>
                <tbody>
                  <tr>
                    <th scope="row">Symbol</th>
                    <td>{investment.symbol}</td>
                    <td>identity</td>
                  </tr>
                  <tr>
                    <th scope="row">Name</th>
                    <td>
                      <input
                        aria-label="Position name"
                        value={pdDraft.name}
                        onChange={(e) => patchDraft({ name: e.target.value })}
                        disabled={busy || writesBlocked}
                      />
                    </td>
                    <td>editable</td>
                  </tr>
                  <tr>
                    <th scope="row">Provider</th>
                    <td>
                      <input
                        aria-label="Position provider"
                        value={pdDraft.provider}
                        onChange={(e) => patchDraft({ provider: e.target.value })}
                        disabled={busy || writesBlocked}
                      />
                    </td>
                    <td>editable</td>
                  </tr>
                  <tr>
                    <th scope="row">Frequency</th>
                    <td>
                      <select
                        aria-label="Position frequency"
                        value={pdDraft.freq}
                        onChange={(e) => patchDraft({ freq: e.target.value })}
                        disabled={busy || writesBlocked}
                      >
                        <option value="">Choose cadence</option>
                        <option value="Weekly">Weekly (52)</option>
                        <option value="Twice monthly">Twice monthly (24)</option>
                        <option value="Monthly">Monthly (12)</option>
                        <option value="Quarterly">Quarterly (4)</option>
                        <option value="None">None (does not pay)</option>
                      </select>
                    </td>
                    <td>editable</td>
                  </tr>
                  <tr>
                    <th scope="row">Underlying</th>
                    <td>
                      <input
                        aria-label="Position underlying"
                        value={pdDraft.underlying}
                        onChange={(e) => patchDraft({ underlying: e.target.value })}
                        disabled={busy || writesBlocked}
                      />
                    </td>
                    <td>editable</td>
                  </tr>
                  <tr>
                    <th scope="row">Strategy characteristics</th>
                    <td aria-label="Strategy characteristics">
                      {strategyCharacteristics(
                        pdDraft.lookthrough,
                        pdDraft.underlying,
                      ) || "—"}
                    </td>
                    <td>
                      {strategyCharacteristics(
                        pdDraft.lookthrough,
                        pdDraft.underlying,
                      )
                        ? "researched"
                        : "unknown"}
                    </td>
                  </tr>
                  <tr>
                    <th scope="row">Theme / strategy</th>
                    <td>
                      <input
                        aria-label="Position theme strategy"
                        value={pdDraft.lookthrough.themeStrategy ?? ""}
                        onChange={(e) =>
                          patchDraft({
                            lookthrough: {
                              ...pdDraft.lookthrough,
                              themeStrategy: e.target.value,
                            },
                          })
                        }
                        disabled={busy || writesBlocked}
                      />
                    </td>
                    <td>editable</td>
                  </tr>
                  <tr>
                    <th scope="row">Primary risk driver</th>
                    <td>
                      <input
                        aria-label="Position primary risk driver"
                        value={pdDraft.lookthrough.primaryRiskDriver ?? ""}
                        onChange={(e) =>
                          patchDraft({
                            lookthrough: {
                              ...pdDraft.lookthrough,
                              primaryRiskDriver: e.target.value,
                            },
                          })
                        }
                        disabled={busy || writesBlocked}
                      />
                    </td>
                    <td>editable</td>
                  </tr>
                  <tr>
                    <th scope="row">Concentration</th>
                    <td aria-label="Position concentration">
                      {concentrationSummary(pdDraft.lookthrough)}
                    </td>
                    <td>
                      {(pdDraft.lookthrough.concentrationStatus ?? "unknown") === "unknown"
                        ? "unknown"
                        : "researched"}
                    </td>
                  </tr>
                  <tr>
                    <th scope="row">Volatility / beta proxy</th>
                    <td>
                      <input
                        aria-label="Position volatility proxy"
                        value={pdDraft.lookthrough.volProxy ?? ""}
                        onChange={(e) =>
                          patchDraft({
                            lookthrough: { ...pdDraft.lookthrough, volProxy: e.target.value },
                          })
                        }
                        disabled={busy || writesBlocked}
                      />
                    </td>
                    <td>editable</td>
                  </tr>
                  <tr>
                    <th scope="row">Tax character note</th>
                    <td>
                      <input
                        aria-label="Position tax character"
                        value={pdDraft.lookthrough.taxCharacter ?? ""}
                        onChange={(e) =>
                          patchDraft({
                            lookthrough: {
                              ...pdDraft.lookthrough,
                              taxCharacter: e.target.value,
                            },
                          })
                        }
                        disabled={busy || writesBlocked}
                      />
                    </td>
                    <td>editable</td>
                  </tr>
                  <tr>
                    <th scope="row">Type / div_type</th>
                    <td>
                      <select
                        aria-label="Position div type"
                        value={
                          (() => {
                            const n = (pdDraft.divType || "")
                              .trim()
                              .toUpperCase()
                              .replace(/\s+/g, "-");
                            if (n === "DIV1" || n === "DIV-1") return "DIV-1";
                            if (n === "CASH" || n === "DIV-2" || n === "DIV2") return "CASH";
                            return "";
                          })()
                        }
                        onChange={(e) => patchDraft({ divType: e.target.value })}
                        disabled={busy || writesBlocked}
                      >
                        <option value="">Select type</option>
                        {DIV_TYPES.map((t) => (
                          <option key={t} value={t}>
                            {t === "CASH" ? "CASH (money market / DIV-2 path)" : t}
                          </option>
                        ))}
                      </select>
                    </td>
                    <td>editable</td>
                  </tr>
                  <tr>
                    <th scope="row">Needs ROC research</th>
                    <td>
                      <label>
                        <input
                          type="checkbox"
                          aria-label="Needs ROC research"
                          checked={pdDraft.needsRoc}
                          onChange={(e) => patchDraft({ needsRoc: e.target.checked })}
                          disabled={busy || writesBlocked}
                        />{" "}
                        flag for 19a-1 research
                      </label>
                    </td>
                    <td>editable</td>
                  </tr>
                  <tr>
                    <th scope="row">Active</th>
                    <td>
                      <label>
                        <input
                          type="checkbox"
                          aria-label="Position is active"
                          checked={pdDraft.isActive}
                          onChange={(e) => patchDraft({ isActive: e.target.checked })}
                          disabled={busy || writesBlocked}
                        />{" "}
                        include in daily last-price set
                      </label>
                    </td>
                    <td>editable</td>
                  </tr>
                  <tr>
                    <th scope="row">Expected tax handling</th>
                    <td>
                      <select
                        aria-label="Expected tax handling"
                        value={pdDraft.taxHandling}
                        onChange={(e) => patchDraft({ taxHandling: e.target.value })}
                        disabled={busy || writesBlocked}
                      >
                        {TAX_HANDLING.map((item) => (
                          <option key={item || "blank"} value={item}>
                            {item || "unknown"}
                          </option>
                        ))}
                      </select>
                    </td>
                    <td>editable</td>
                  </tr>
                  <tr>
                    <th scope="row">Declaration weekday</th>
                    <td>
                      <select
                        aria-label="Declaration weekday"
                        value={pdDraft.declarationWeekday}
                        onChange={(e) => patchDraft({ declarationWeekday: e.target.value })}
                        disabled={busy || writesBlocked}
                      >
                        {WEEKDAYS.map((day) => (
                          <option key={day || "blank"} value={day}>
                            {day || "unknown"}
                          </option>
                        ))}
                      </select>
                    </td>
                    <td>editable</td>
                  </tr>
                  <tr>
                    <th scope="row">Ex-date weekday</th>
                    <td>
                      <select
                        aria-label="Ex-date weekday"
                        value={pdDraft.exdateWeekday}
                        onChange={(e) => patchDraft({ exdateWeekday: e.target.value })}
                        disabled={busy || writesBlocked}
                      >
                        {WEEKDAYS.map((day) => (
                          <option key={day || "blank"} value={day}>
                            {day || "unknown"}
                          </option>
                        ))}
                      </select>
                    </td>
                    <td>editable</td>
                  </tr>
                  <tr>
                    <th scope="row">Payday weekday</th>
                    <td>
                      <select
                        aria-label="Payday weekday"
                        value={pdDraft.paydayWeekday}
                        onChange={(e) => patchDraft({ paydayWeekday: e.target.value })}
                        disabled={busy || writesBlocked}
                      >
                        {WEEKDAYS.map((day) => (
                          <option key={day || "blank"} value={day}>
                            {day || "unknown"}
                          </option>
                        ))}
                      </select>
                    </td>
                    <td>editable</td>
                  </tr>
                  <tr>
                    <th scope="row">Completeness</th>
                    <td aria-label="Position completeness">
                      {positionMaster?.rows.find(
                        (row) => row.symbol.toUpperCase() === investment.symbol.toUpperCase(),
                      )?.completeness ?? "unknown"}
                    </td>
                    <td>joined</td>
                  </tr>
                  <tr>
                    <th scope="row">CurrentPrice</th>
                    <td>
                      {investment.price.priceMinor != null
                        ? `${formatUsd(investment.price.priceMinor, investment.price.scale)} (${investment.price.freshness})`
                        : `unavailable (${investment.price.freshness})`}
                    </td>
                    <td>calculated</td>
                  </tr>
                  <tr>
                    <th scope="row">Price-derived valid</th>
                    <td>{investment.price.priceDerivedValid ? "yes" : "no"}</td>
                    <td>calculated</td>
                  </tr>
                  <tr>
                    <th scope="row">Open qty</th>
                    <td>
                      {formatScaled(
                        investment.remainingQuantityMinor,
                        investment.quantityScale,
                      )}
                    </td>
                    <td>from lots</td>
                  </tr>
                  <tr>
                    <th scope="row">Original cost</th>
                    <td>
                      {formatUsd(investment.remainingPerformanceMinor, investment.scale)}
                    </td>
                    <td>from lots</td>
                  </tr>
                  <tr>
                    <th scope="row">Avg unit cost</th>
                    <td>
                      {investment.unitCostMinor == null
                        ? "unknown"
                        : formatUsd(investment.unitCostMinor, 2)}
                    </td>
                    <td>calculated</td>
                  </tr>
                  <tr>
                    <th scope="row">Tax basis</th>
                    <td>{formatUsd(investment.remainingTaxMinor, investment.scale)}</td>
                    <td>from lots</td>
                  </tr>
                  <tr>
                    <th scope="row">Market value</th>
                    <td>
                      {investment.marketValueMinor == null
                        ? "unknown — needs a valid CurrentPrice"
                        : formatUsd(investment.marketValueMinor, investment.scale)}
                    </td>
                    <td>calculated</td>
                  </tr>
                  <tr>
                    <th scope="row">Unrealized vs cost</th>
                    <td>
                      {investment.unrealizedPerformanceMinor == null
                        ? "unknown"
                        : formatUsd(
                            investment.unrealizedPerformanceMinor,
                            investment.scale,
                          )}
                    </td>
                    <td>calculated</td>
                  </tr>
                  <tr>
                    <th scope="row">Unrealized %</th>
                    <td>{formatBps(investment.unrealizedPnlBps)}</td>
                    <td>calculated</td>
                  </tr>
                  <tr>
                    <th scope="row">Plan last adjusted</th>
                    <td>
                      {investment.planEffectiveFrom?.trim() || "unknown"}
                    </td>
                    <td>from PlanHistory</td>
                  </tr>
                  <tr>
                    <th scope="row">Declarations</th>
                    <td>{formatCount(investment.declarationCount)} / 12 stored</td>
                    <td>observations</td>
                  </tr>
                  <tr>
                    <th scope="row">ROC 2024 actual %</th>
                    <td>
                      <input
                        aria-label="ROC 2024 actual"
                        value={pdDraft.roc2024}
                        onChange={(e) => patchDraft({ roc2024: e.target.value })}
                        disabled={busy || writesBlocked}
                      />
                    </td>
                    <td>editable</td>
                  </tr>
                  <tr>
                    <th scope="row">ROC 2025 actual %</th>
                    <td>
                      {rocActualUnavailable(investment.lots, 2025, asOfDate) ? (
                        <p aria-label="ROC 2025 actual">
                          {rocActualUnavailable(
                            investment.lots,
                            2025,
                            asOfDate,
                          )}
                        </p>
                      ) : (
                        <input
                          aria-label="ROC 2025 actual"
                          value={pdDraft.roc2025}
                          onChange={(e) =>
                            patchDraft({ roc2025: e.target.value })
                          }
                          disabled={busy || writesBlocked}
                        />
                      )}
                    </td>
                    <td>
                      {rocActualUnavailable(investment.lots, 2025, asOfDate) ||
                        "editable after 2025 1099"}
                    </td>
                  </tr>
                  <tr>
                    <th scope="row">ROC 2026 estimate %</th>
                    <td>
                      <input
                        aria-label="ROC 2026 estimate"
                        value={pdDraft.roc2026e}
                        onChange={(e) => patchDraft({ roc2026e: e.target.value })}
                        disabled={busy || writesBlocked}
                      />
                    </td>
                    <td>editable</td>
                  </tr>
                  <tr>
                    <th scope="row">ROC 2026 actual %</th>
                    <td>
                      {rocActualUnavailable(investment.lots, 2026, asOfDate) ? (
                        <p aria-label="ROC 2026 actual">
                          {rocActualUnavailable(
                            investment.lots,
                            2026,
                            asOfDate,
                          )}
                        </p>
                      ) : (
                        <input
                          aria-label="ROC 2026 actual"
                          value={pdDraft.roc2026a}
                          onChange={(e) =>
                            patchDraft({ roc2026a: e.target.value })
                          }
                          disabled={busy || writesBlocked}
                        />
                      )}
                    </td>
                    <td>
                      {rocActualUnavailable(investment.lots, 2026, asOfDate) ||
                        "editable after 2026 1099"}
                    </td>
                  </tr>
                  <tr className="fact-span">
                    <th scope="row">Notes</th>
                    <td>
                      <input
                        aria-label="Position notes"
                        value={pdDraft.notes}
                        onChange={(e) => patchDraft({ notes: e.target.value })}
                        disabled={busy || writesBlocked}
                      />
                    </td>
                    <td>editable</td>
                  </tr>
                  <tr className="fact-span">
                    <th scope="row">Retrieval template</th>
                    <td>
                      <p aria-label="Adapter source">
                        Adapter:{" "}
                        {investment.template?.declarationSource ||
                          "unassigned"}
                      </p>
                      <p>
                        Schedule:{" "}
                        {calendarPolicyLabel(
                          investment.template?.calendarPolicy,
                        )}
                      </p>
                      <p aria-label="Template Dividend">
                        Template Dividend:{" "}
                        {investment.template?.sourceUrl?.trim()
                          ? investment.template.sourceUrl
                          : "—"}
                      </p>
                      <p aria-label="Template ROC">
                        Template ROC:{" "}
                        {investment.template?.rocSourceUrl?.trim()
                          ? investment.template.rocSourceUrl
                          : "—"}
                      </p>
                      <p aria-label="Template content hash">
                        Content hash:{" "}
                        {investment.template?.lastContentHash?.trim()
                          ? investment.template.lastContentHash
                          : "—"}
                      </p>
                      <p>
                        Lookback:{" "}
                        {formatCount(
                          investment.template?.lookbackCount ??
                            DECLARATION_LOOKBACK_TARGET,
                        )}
                      </p>
                      <p>
                        Inception (optional):{" "}
                        {investment.template?.inceptionOn?.trim()
                          ? investment.template.inceptionOn
                          : "— (only used when paid decls are under 12)"}
                      </p>
                      {investment.template?.lastRunAt ? (
                        <p>
                          Last run {investment.template.lastRunAt}
                          {investment.template.lastRunOk == null
                            ? ""
                            : investment.template.lastRunOk
                              ? " succeeded"
                              : " missed"}
                          {investment.template.lastRunMessage
                            ? `: ${investment.template.lastRunMessage}`
                            : ""}
                        </p>
                      ) : (
                        <p>Last run: never</p>
                      )}
                      <p>
                        Edit retrieval templates on Settings. Hub face source is
                        the adapter, not the stored template knobs.
                      </p>
                      <button
                        type="button"
                        aria-label="Open Settings retrieval templates"
                        onClick={() => setScreen("settings")}
                      >
                        Open Settings
                      </button>
                    </td>
                    <td>Settings</td>
                  </tr>
                  <tr>
                    <th scope="row">Backtest / scores</th>
                    <td>
                      {(investment.results ?? []).length === 0
                        ? "unknown — dated bull/bear windows required before scores. Missing evidence stays missing; it does not invent a rank."
                        : `${formatCount(investment.results.length)} stored window result${investment.results.length === 1 ? "" : "s"}. Suggestion does not change Risk until you Apply.`}
                    </td>
                    <td>
                      {(investment.results ?? []).length === 0 ? "unknown" : "calculated"}
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
            </details>
            <section className="hub-panel" id="hub-declarations" aria-label="Declarations" data-focus={positionFocusPanel === "declarations" ? "1" : undefined}>
            <h3>Declarations</h3>
            {(() => {
              const decls = investment.declarations ?? [];
              const paid = decls.filter((d) => d.amountPerShareMinor != null).length;
              const stored = decls.length;
              const asOf = asOfDate || new Date().toISOString().slice(0, 10);
              const inception = investment.template?.inceptionOn?.trim() || "";
              let short: string | null = null;
              let needLabel = `need ${formatCount(DECLARATION_LOOKBACK_TARGET)}`;
              if (paid >= DECLARATION_LOOKBACK_TARGET) {
                needLabel = `${formatCount(DECLARATION_LOOKBACK_TARGET)}+ paid — inception N/A`;
              } else if (inception) {
                const expected = expectedDeclarationLookback(
                  inception,
                  asOf,
                  pdDraft.freq || investment.paymentFrequency,
                );
                needLabel = `need ${formatCount(expected)} (inception-limited; full is ${formatCount(DECLARATION_LOOKBACK_TARGET)})`;
                if (paid < expected) {
                  short = `Adapter returned ${formatCount(paid)} of ${formatCount(expected)} paid declarations expected since inception ${inception}.`;
                }
              } else {
                short = `Adapter returned ${formatCount(paid)} of ${formatCount(DECLARATION_LOOKBACK_TARGET)} required paid declarations. Confirm inception Yes/No on Add Investment — do not defer to Settings.`;
              }
              return (
                <>
                  <h4>Latest paid ({needLabel})</h4>
                  <p>
                    Paid $/share from the issuer adapter. Period is the payable date,
                    not the declaration date. Blank stays unknown, not $0.
                    Stored: {formatCount(stored)}. Retrieve first; inception is
                    consulted only when under{" "}
                    {formatCount(DECLARATION_LOOKBACK_TARGET)} paid points.
                  </p>
                  {short ? (
                    <p role="status" aria-label="Declaration shortfall">
                      {short}
                    </p>
                  ) : null}
                  <DeclarationPaymentsChart
                    declarations={(investment.declarations ?? []).map((d) => ({
                      paymentPeriod: d.paymentPeriod,
                      amountPerShareMinor: d.amountPerShareMinor ?? 0,
                      amountScale: d.amountScale,
                    }))}
                    asOfDate={asOf}
                  />
                  {stored === 0 ? (
                    <p>No issuer declarations stored.</p>
                  ) : (
                    <div className="table-wrap">
                      <table aria-label="Stored declarations">
                        <thead>
                          <tr>
                            <th scope="col">Pay date</th>
                            <th className="numeric" scope="col">
                              Per share
                            </th>
                            <th scope="col">Source</th>
                          </tr>
                        </thead>
                        <tbody>
                          {decls.map((row, i) => (
                            <tr key={`${row.paymentPeriod}-${i}`}>
                              <td>{row.paymentPeriod || "—"}</td>
                              <td className="numeric">
                                {row.amountPerShareMinor == null
                                  ? "—"
                                  : `$${formatScaled(row.amountPerShareMinor, row.amountScale)}`}
                              </td>
                              <td>{row.source || "—"}</td>
                            </tr>
                          ))}
                        </tbody>
                      </table>
                    </div>
                  )}
                </>
              );
            })()}
            </section>
            <section className="hub-panel" id="hub-income" aria-label="Plan payment summary" data-focus={positionFocusPanel === "income" ? "1" : undefined}>
            <h3>Plan payment summary</h3>
            <p>
              Remaining-year Plan cash (Plan × quantity). Blank stays unknown, not $0.
              Separate from paid declaration history and broker ledger.
            </p>
            {pdRemaining == null ? (
              <p>Loading plan payment summary…</p>
            ) : (
              <>
                <p aria-label="Remaining year schedule provenance">
                  {pdRemaining.known
                    ? dateProvenanceLabel(pdRemaining.provenance)
                    : `${dateProvenanceLabel(pdRemaining.provenance)}. Unknown stays unknown.`}
                  {pdRemaining.calendarPolicy
                    ? ` Schedule: ${calendarPolicyLabel(pdRemaining.calendarPolicy)}.`
                    : ""}
                </p>
                <div className="table-wrap">
                  <table aria-label="Plan payment summary">
                    <thead>
                      <tr>
                        <th scope="col">Period</th>
                        <th className="numeric" scope="col">Plan cash</th>
                      </tr>
                    </thead>
                    <tbody>
                      <tr>
                        <th scope="row">Remaining this year</th>
                        <td className="numeric">
                          {pdRemaining.yearToGoMinor == null
                            ? "unknown"
                            : formatUsd(pdRemaining.yearToGoMinor, pdRemaining.scale)}
                        </td>
                      </tr>
                      {pdRemaining.months.map((m) => (
                        <tr key={m.month}>
                          <th scope="row">{m.month}</th>
                          <td className="numeric">
                            {m.cashMinor == null
                              ? "unknown"
                              : formatUsd(m.cashMinor, pdRemaining.scale)}
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </>
            )}
            </section>
            <section className="hub-panel" id="hub-pay-dates" aria-label="Future plan payment dates">
            <h3>Future plan payment dates</h3>
            <p>
              Each remaining pay date × Plan × open quantity for this symbol.
              Date source shows whether the day is vendor-published or a derived
              projection (derived is overwritten when the issuer posts that period).
            </p>
            {pdRemaining == null ? (
              <p>Loading pay dates…</p>
            ) : (pdRemaining.payments ?? []).length === 0 ? (
              <p>No remaining-year pay dates yet.</p>
            ) : (
              <div className="table-wrap">
                <table aria-label="Future plan payment dates">
                  <thead>
                    <tr>
                      <th scope="col">Pay on</th>
                      <th className="numeric" scope="col">Plan cash</th>
                      <th scope="col">Date source</th>
                    </tr>
                  </thead>
                  <tbody>
                    {pdRemaining.payments.map((pay) => (
                      <tr key={pay.originalPayOn}>
                        <td>{pay.payOn}</td>
                        <td className="numeric">
                          {pay.cashMinor == null
                            ? "unknown"
                            : formatUsd(pay.cashMinor, pdRemaining.scale)}
                        </td>
                        <td>
                          {dateProvenanceLabel(
                            pay.dateProvenance || pay.originalPayOn,
                          )}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
            </section>
            <section
              className="hub-panel"
              id="hub-received"
              aria-label="Received payments"
            >
              <h3>Received payments</h3>
              <p>
                Broker cash already received for this symbol (Investment Activity
                Ledger). All-years and per calendar year. Unknown stays unknown.
              </p>
              {(() => {
                const actuals = pdLedger?.actuals ?? [];
                const scale = pdLedger?.scale ?? investment.scale;
                const byYear = receivedByYear(actuals);
                const allYears =
                  investment.distributionsScope === "incomplete" ||
                  investment.totalDistributionsReceivedMinor == null
                    ? null
                    : investment.totalDistributionsReceivedMinor;
                const ledgerSum = actuals.reduce((s, a) => s + a.amountMinor, 0);
                return (
                  <>
                    <div className="table-wrap">
                      <table aria-label="Received payment totals">
                        <thead>
                          <tr>
                            <th scope="col">Scope</th>
                            <th className="numeric" scope="col">
                              Received
                            </th>
                          </tr>
                        </thead>
                        <tbody>
                          <tr>
                            <th scope="row">All years</th>
                            <td className="numeric">
                              {allYears == null
                                ? actuals.length === 0
                                  ? "unknown"
                                  : formatUsd(ledgerSum, scale)
                                : formatUsd(allYears, investment.scale)}
                            </td>
                          </tr>
                          <tr>
                            <th scope="row">ROC component (Car, current year)</th>
                            <td className="numeric">
                              {investment.rocDistributionsMinor == null
                                ? "unknown"
                                : formatUsd(
                                    investment.rocDistributionsMinor,
                                    investment.scale,
                                  )}
                            </td>
                          </tr>
                        </tbody>
                      </table>
                    </div>
                    {byYear.length === 0 ? (
                      <p>No ledger dividends stored for this symbol yet.</p>
                    ) : (
                      <div className="table-wrap">
                        <table aria-label="Received payments by year">
                          <thead>
                            <tr>
                              <th scope="col">Year</th>
                              <th className="numeric" scope="col">
                                Received
                              </th>
                            </tr>
                          </thead>
                          <tbody>
                            {byYear.map((row) => (
                              <tr key={row.year}>
                                <td>{row.year}</td>
                                <td className="numeric">
                                  {formatUsd(row.amountMinor, scale)}
                                </td>
                              </tr>
                            ))}
                          </tbody>
                        </table>
                      </div>
                    )}
                  </>
                );
              })()}
            </section>
            <section
              className="hub-panel"
              id="hub-ledger"
              aria-label="Ledger income"
              data-focus={positionFocusPanel === "ledger" ? "1" : undefined}
            >
              <h3>Ledger income</h3>
              <p>
                Broker-paid dividends for this symbol (Investment Activity Ledger).
                Unknown stays unknown.
              </p>
              {(pdLedger?.actuals ?? []).length === 0 ? (
                <p>No ledger dividends stored for this symbol yet.</p>
              ) : (
                <div className="table-wrap">
                  <table aria-label="Ledger dividends">
                    <thead>
                      <tr>
                        <th scope="col">Occurred</th>
                        <th className="numeric" scope="col">
                          Amount
                        </th>
                      </tr>
                    </thead>
                    <tbody>
                      {(pdLedger?.actuals ?? []).map((row) => (
                        <tr key={row.actualId}>
                          <td>{row.occurredOn}</td>
                          <td className="numeric">
                            {formatUsd(row.amountMinor, row.scale)}
                          </td>
                        </tr>
                      ))}
                    </tbody>
                    <tfoot>
                      <tr>
                        <td>Total</td>
                        <td className="numeric">
                          {formatUsd(
                            (pdLedger?.actuals ?? []).reduce(
                              (s, a) => s + a.amountMinor,
                              0,
                            ),
                            pdLedger?.scale ?? 2,
                          )}
                        </td>
                      </tr>
                    </tfoot>
                  </table>
                </div>
              )}
            </section>
            <section
              className="hub-panel"
              id="hub-lots"
              aria-label="Lots by account"
              data-focus={positionFocusPanel === "lots" ? "1" : undefined}
            >
              <h3>Lots by account</h3>
              <p>Lots and cost for the chosen ticker only. Data totals above stay put.</p>
              {investment.lots.length === 0 ? (
                <p>No open lots. Calculator omits this symbol until the first lot.</p>
              ) : (
                <SymbolLotsTable lots={investment.lots} />
              )}
            </section>
            <section className="hub-panel" aria-label="Backtests">
            <h3>Owner period</h3>
            <p>
              Bull and Bear dates for this symbol. The same row as Market impact.
              A higher bear return lost less. A higher bull return made more.
            </p>
            <SymbolWindowTable
              client={windowClient}
              symbol={investment.symbol}
              discardEpoch={windowDiscardEpoch}
              onDirtyChange={onWindowsDirty}
              onSaved={onWindowsSaved}
              showCashCushion={false}
            />
            <h3>Evidence</h3>
            <EvidenceWindows investment={investment} />
            </section>
          </>

        ) : (
          <p>Choose a symbol to open the position hub.</p>
        )}
      </section>
  );
}
