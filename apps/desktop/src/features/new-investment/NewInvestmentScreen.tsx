import {
  formatCount,
  formatPerShare,
} from "@finos/ui-components";
import type {
  CurrentPriceGet,
  LookthroughResearch,
  PlanReviewGet,
  RemainingYearIncomeGet,
  RocResearchGet,
  SecurityListItem,
} from "@finos/app-contracts";
import { ReadinessChecklist, type ReadinessRow } from "./ReadinessChecklist";

export type WizardDecl = {
  amountPerShareMinor: number;
  amountScale?: number;
  paymentPeriod?: string;
  source?: string;
};

export type NewInvestmentScreenProps = {
  busy: boolean;
  writesBlocked: boolean;
  wizDirty: boolean;
  wizSecurityId: string;
  wizCollectorGaps: string[];
  wizProcessASaved: boolean;
  wizCollectorComplete: boolean;
  wizPlanStored: boolean;
  wizSymbol: string;
  wizName: string;
  wizInceptionOn: string;
  wizSourceUrl: string;
  wizDeclSource: string;
  wizStoredUrlOffer: string;
  wizResearchDone: boolean;
  wizRetrieveNote: string;
  wizEstablishComplete: boolean;
  wizEstablishOpenLabels: string[];
  wizEstablishChecklist: ReadinessRow[];
  wizAskSecondUrl: boolean;
  wizSecondUrlTried: boolean;
  wizSecondUrl: string;
  wizAskInception: boolean;
  wizInceptionCandidate: string;
  wizExpectedPaid: number | null;
  wizProvider: string;
  wizFieldDecision: Record<string, string>;
  wizUnderlying: string;
  wizLookthrough: LookthroughResearch;
  wizFreq: string;
  wizDecls: WizardDecl[];
  wizRemaining: RemainingYearIncomeGet | null;
  wizUpcomingPays: Array<{
    payOn: string;
    amountPerShareMinor?: number | null;
    amountScale?: number;
  }>;
  wizReview: PlanReviewGet | null;
  wizPriceState: CurrentPriceGet | null;
  wizRoc: RocResearchGet | null;
  wizTierSuggestion: {
    suggestedTier: string;
    ruleset: string;
    reason: string;
    complete: boolean;
  } | null;
  wizRisk: string;
  wizBacktestNeeded: boolean;
  wizDivType: string;
  wizPlan: string;
  wizPlanReason: string;
  wizIncomplete: string;
  wizStoredPlan: { minor: number; scale: number } | null;
  wizRocUrl: string;
  wizRocPct: string;
  wizAiThesis: string | null;
  researchNotes: {
    overview: string;
    suggestedTier: string;
    suggestedReason: string;
    source: string;
  } | null;
  researchActivity: {
    running: boolean;
    step: number;
    total: number;
    label: string;
    resultLine: string | null;
  } | null;
  securities: SecurityListItem[];
  processANeedsRemaining: boolean;
  processALotCount: number;
  processANeedsInception: boolean;
  processAPlanBlockReason: string;
  processAFieldStatus: {
    frequency: boolean;
    declarations: boolean;
    roc: boolean;
    price: boolean;
  };
  DIV_TYPES: readonly string[];
  RISK_TIERS: readonly string[];
  PLAN_REASONS: readonly string[];
  INCOMPLETE_REASONS: readonly string[];
  scaledDollars: (minor: number, scale: number) => string;
  dateProvenanceLabel: (provenance: string | null | undefined) => string;
  strategyCharacteristics: (lt: LookthroughResearch, underlying?: string) => string;
  parseCadence: (raw: string) => unknown;
  normalizeDivType: (raw: string) => string;
  continueProcessARemaining: () => void;
  goProcessAToPositionDetails: () => void;
  researchRoc: () => void | Promise<void>;
  goProcessAToAddLot: () => void;
  startAnotherProcessA: () => void;
  offerStoredTemplatesForSymbol: (rawSymbol: string) => void | Promise<void>;
  runProcessAResearch: () => void | Promise<void>;
  saveProcessAResearch: () => void;
  retryProcessASecondUrl: () => void | Promise<void>;
  confirmProcessAInception: (yes: boolean) => void | Promise<void>;
  decideProcessAField: (field: string, decision: "accept" | "skip") => void | Promise<void>;
  saveProcessAOwnerIdentity: () => void | Promise<void>;
  applyOwnerRiskTier: () => void | Promise<void>;
  confirmPlan: () => void | Promise<void>;
  applyWizSuggestedTier: () => void | Promise<void>;
  submitProcessARocUrl: () => void | Promise<void>;
  submitProcessAManualRoc: () => void | Promise<void>;
  acceptProcessARoc: () => void | Promise<void>;
  setWizSymbol: (value: string) => void;
  setWizResearchDone: (value: boolean) => void;
  setWizProcessASaved: (value: boolean) => void;
  setWizStoredUrlOffer: (value: string) => void;
  setWizSourceUrl: (value: string) => void;
  setWizSecondUrl: (value: string) => void;
  setWizInceptionOn: (value: string) => void;
  setWizProvider: (value: string) => void;
  setWizFreq: (value: string) => void;
  setWizDivType: (value: string) => void;
  setWizUnderlying: (value: string) => void;
  setWizRisk: (value: string) => void;
  setWizPlan: (value: string) => void;
  setWizPlanReason: (value: string) => void;
  setWizIncomplete: (value: string) => void;
  setWizRocUrl: (value: string) => void;
  setWizRocPct: (value: string) => void;
};

export function NewInvestmentScreen(props: NewInvestmentScreenProps) {
  const {
    busy,
    writesBlocked,
    wizDirty,
    wizSecurityId,
    wizCollectorGaps,
    wizProcessASaved,
    wizCollectorComplete,
    wizPlanStored,
    wizSymbol,
    wizName,
    wizInceptionOn,
    wizSourceUrl,
    wizDeclSource,
    wizStoredUrlOffer,
    wizResearchDone,
    wizRetrieveNote,
    wizEstablishComplete,
    wizEstablishOpenLabels,
    wizEstablishChecklist,
    wizAskSecondUrl,
    wizSecondUrlTried,
    wizSecondUrl,
    wizAskInception,
    wizInceptionCandidate,
    wizExpectedPaid,
    wizProvider,
    wizFieldDecision,
    wizUnderlying,
    wizLookthrough,
    wizFreq,
    wizDecls,
    wizRemaining,
    wizUpcomingPays,
    wizReview,
    wizPriceState,
    wizRoc,
    wizTierSuggestion,
    wizRisk,
    wizBacktestNeeded,
    wizDivType,
    wizPlan,
    wizPlanReason,
    wizIncomplete,
    wizStoredPlan,
    wizRocUrl,
    wizRocPct,
    wizAiThesis,
    researchNotes,
    researchActivity,
    securities,
    processANeedsRemaining,
    processALotCount,
    processANeedsInception,
    processAPlanBlockReason,
    processAFieldStatus,
    DIV_TYPES,
    RISK_TIERS,
    PLAN_REASONS,
    INCOMPLETE_REASONS,
    scaledDollars,
    dateProvenanceLabel,
    strategyCharacteristics,
    parseCadence,
    normalizeDivType,
    continueProcessARemaining,
    goProcessAToPositionDetails,
    researchRoc,
    goProcessAToAddLot,
    startAnotherProcessA,
    offerStoredTemplatesForSymbol,
    runProcessAResearch,
    saveProcessAResearch,
    retryProcessASecondUrl,
    confirmProcessAInception,
    decideProcessAField,
    saveProcessAOwnerIdentity,
    applyOwnerRiskTier,
    confirmPlan,
    applyWizSuggestedTier,
    submitProcessARocUrl,
    submitProcessAManualRoc,
    acceptProcessARoc,
    setWizSymbol,
    setWizResearchDone,
    setWizProcessASaved,
    setWizStoredUrlOffer,
    setWizSourceUrl,
    setWizSecondUrl,
    setWizInceptionOn,
    setWizProvider,
    setWizFreq,
    setWizDivType,
    setWizUnderlying,
    setWizRisk,
    setWizPlan,
    setWizPlanReason,
    setWizIncomplete,
    setWizRocUrl,
    setWizRocPct,
  } = props;
  return (
<section aria-label="Add Investment">
  <h2>Add Investment</h2>
  <p
    aria-label="Collector gaps"
    id="add-investment-gaps"
    data-section="add-investment-gaps"
    data-part="collector-gaps"
  >
    {wizSecurityId
      ? wizCollectorGaps.length
        ? `Open: ${wizCollectorGaps.join(", ")}`
        : "No collector gaps."
      : "Open gaps appear once this symbol is saved."}
  </p>
  {wizProcessASaved && wizSecurityId ? (
    <section
      aria-label="Process A completion"
      id="add-investment-saved"
      data-section="add-investment-saved"
    >
      <h3>Research saved</h3>
      <p
        data-part="completion-status"
        role="status"
        aria-label="Add Investment completion status"
      >
        {wizCollectorComplete && wizPlanStored
          ? `${wizSymbol.trim().toUpperCase() || "This symbol"}: research and Plan are saved in the book. Nothing left to Confirm or Accept on this screen.`
          : processANeedsRemaining
            ? "Identity retrieval was saved, but Plan and/or inception are still open. Finish Confirm Plan / inception on this screen — Complete research only re-runs retrieval; it does not store Plan."
            : "Process A finished for this identity. Choose a next action."}
      </p>
      <ul aria-label="What is saved versus optional">
        <li>
          Research (frequency, DIV type, provider, underlying, ROC, pays, inception):{" "}
          {wizCollectorComplete ? "saved" : "not complete yet"}
        </li>
        <li>
          Plan / share: {wizPlanStored ? "saved — Shopping Cart can buy" : "not saved — Confirm Plan required"}
        </li>
        <li>
          Open lot:{" "}
          {processALotCount > 0
            ? `saved (${formatCount(processALotCount)}) — Income Plan / P/L connected`
            : "optional — Add lots when you hold shares (Calculator already lists Plan at 0 shares)"}
        </li>
      </ul>
      <dl
        className="process-a-completion-summary"
        aria-label="Saved identity summary"
        data-part="saved-identity-summary"
      >
        <div>
          <dt>Symbol</dt>
          <dd>{wizSymbol.trim().toUpperCase() || "—"}</dd>
        </div>
        <div>
          <dt>Name</dt>
          <dd>{wizName.trim() || "unknown"}</dd>
        </div>
        <div>
          <dt>Lot count</dt>
          <dd>{formatCount(processALotCount)}</dd>
        </div>
        <div>
          <dt>Plan / share</dt>
          <dd>{wizPlanStored ? "stored" : "not stored — Confirm Plan still required"}</dd>
        </div>
        <div>
          <dt>Inception</dt>
          <dd>
            {wizInceptionOn.trim()
              ? wizInceptionOn.trim()
              : processANeedsInception
                ? "needed (under 12 pays)"
                : "n/a"}
          </dd>
        </div>
        <div>
          <dt>Overall</dt>
          <dd>
            {processANeedsRemaining
              ? "incomplete"
              : processAFieldStatus.frequency &&
                  processAFieldStatus.declarations &&
                  processAFieldStatus.roc &&
                  processAFieldStatus.price
                ? "researched"
                : "incomplete"}
          </dd>
        </div>
      </dl>
      <table aria-label="Researched versus incomplete" data-part="researched-vs-incomplete">
        <thead>
          <tr>
            <th scope="col">Field</th>
            <th scope="col">Status</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <th scope="row">Frequency</th>
            <td>{processAFieldStatus.frequency ? "filled" : "unknown"}</td>
          </tr>
          <tr>
            <th scope="row">Declarations</th>
            <td>
              {processAFieldStatus.declarations
                ? `filled (${formatCount(wizDecls.length)})`
                : "unknown"}
            </td>
          </tr>
          <tr>
            <th scope="row">ROC estimate</th>
            <td>
              {processAFieldStatus.roc
                ? `filled (${((wizRoc!.rocPctMinor as number) / 10 ** wizRoc!.scale).toFixed(wizRoc!.scale)}%)`
                : "unknown"}
            </td>
          </tr>
          <tr>
            <th scope="row">Price</th>
            <td>
              {processAFieldStatus.price
                ? `filled (${scaledDollars(wizPriceState!.priceMinor as number, wizPriceState!.scale)})`
                : "unknown"}
            </td>
          </tr>
          <tr>
            <th scope="row">Plan / share</th>
            <td>{wizPlanStored ? "stored" : "not stored"}</td>
          </tr>
          <tr>
            <th scope="row">Calculator / Income Plan</th>
            <td>
              {processALotCount > 0
                ? `connected (${formatCount(processALotCount)} open lot${processALotCount === 1 ? "" : "s"})`
                : wizPlanStored
                  ? "Calculator lists Plan with 0 shares; Income Plan / daily prices wait on Add lots"
                  : "omitted until Confirm Plan (then Calculator at 0 shares; Add lots for Income Plan)"}
            </td>
          </tr>
        </tbody>
      </table>
      <div className="buttons dossier-actions" data-part="saved-actions">
        {processANeedsRemaining ? (
          <button
            type="button"
            aria-label="Complete remaining details"
            disabled={busy}
            onClick={() => continueProcessARemaining()}
          >
            Complete remaining details
          </button>
        ) : (
          <button
            type="button"
            aria-label="Open Position Details"
            disabled={busy}
            onClick={() => goProcessAToPositionDetails()}
          >
            Open Position Details
          </button>
        )}
        <button
          type="button"
          aria-label="Re-run research retrieval"
          disabled={
            busy ||
            writesBlocked ||
            Boolean(researchActivity?.running) ||
            !(wizSourceUrl.trim() || wizDeclSource.trim())
          }
          onClick={() => void researchRoc()}
        >
          {researchActivity?.running
            ? "Re-running research…"
            : "Re-run research retrieval"}
        </button>
        <button
          type="button"
          aria-label="Add lots"
          disabled={busy || (!wizCollectorComplete && processALotCount === 0)}
          onClick={() => goProcessAToAddLot()}
        >
          Add lots
        </button>
        <button
          type="button"
          aria-label="Research another position"
          disabled={busy || writesBlocked}
          onClick={() => startAnotherProcessA()}
        >
          Research another
        </button>
      </div>
    </section>
  ) : null}
  <p>
    Process A: enter symbol — if a Template Dividend URL is already on file it
    is offered as the default. Confirm or paste the issuer URL, then Research.
    Retrieved facts show below; missing stays unknown — never $0. Confirm Plan
    and Apply tier are explicit owner actions. Lots are opened on Add Lot, not here.
    Save keeps this research on screen — it does not wipe fields.
  </p>
  <div
    className="form-grid process-a-inputs"
    id="add-investment-inputs"
    data-section="add-investment-inputs"
    data-part="research-inputs"
  >
    <label>
      Symbol
      <input
        aria-label="Symbol"
        value={wizSymbol}
        onChange={(e) => {
          setWizSymbol(e.target.value.toUpperCase());
        }}
        onBlur={() => {
          const next = wizSymbol.trim().toUpperCase();
          const held = (
            securities.find((s) => s.securityId === wizSecurityId)?.symbol ||
            ""
          )
            .trim()
            .toUpperCase();
          if (wizSecurityId && next && held && next !== held) {
            setWizResearchDone(false);
            setWizProcessASaved(false);
            setWizStoredUrlOffer("");
          }
          void offerStoredTemplatesForSymbol(wizSymbol);
        }}
        onKeyDown={(e) => {
          if (e.key === "Enter") {
            e.preventDefault();
            void offerStoredTemplatesForSymbol(wizSymbol);
          }
        }}
        disabled={busy || writesBlocked}
        autoComplete="off"
      />
    </label>
    <label>
      Template Dividend
      <input
        aria-label="Template Dividend"
        value={wizSourceUrl}
        onChange={(e) => {
          setWizSourceUrl(e.target.value);
        }}
        disabled={busy || writesBlocked}
        placeholder="https://…/#distributions"
        autoComplete="off"
      />
    </label>
  </div>
  {wizStoredUrlOffer ? (
    <p role="status" aria-label="Stored Template Dividend offer">
      {wizStoredUrlOffer}
    </p>
  ) : null}
  <div className="buttons dossier-actions" data-part="research-actions">
    <button
      type="button"
      aria-label="Research"
      disabled={
        busy ||
        Boolean(researchActivity?.running) ||
        writesBlocked ||
        !wizSymbol.trim() ||
        !wizSourceUrl.trim()
      }
      onClick={() => void runProcessAResearch()}
    >
      Research
    </button>
    {wizResearchDone ? (
      <button
        type="button"
        aria-label="Save Process A research"
        className={wizDirty ? "is-unsaved" : undefined}
        disabled={busy || writesBlocked || !wizSecurityId}
        onClick={() => saveProcessAResearch()}
      >
        Save
      </button>
    ) : null}
  </div>
  {researchActivity?.running ? (
    <section
      className="process-a-research-progress"
      aria-label="Research progress"
      aria-busy="true"
      id="add-investment-progress"
      data-section="add-investment-progress"
      data-part="research-progress"
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
  {researchActivity?.resultLine && !researchActivity.running ? (
    <p data-part="research-result" role="status" aria-label="Research result">
      {researchActivity.resultLine}
    </p>
  ) : null}
  {wizRetrieveNote && !researchActivity?.resultLine ? (
    <p role="status">{wizRetrieveNote}</p>
  ) : null}
  {wizResearchDone && !researchActivity?.running ? (
    <section
      aria-label="Research results"
      className="process-a-results"
      id="add-investment-results"
      data-section="add-investment-results"
    >
      <h3>Research results</h3>
      {(() => {
        const complete = wizEstablishComplete || (wizCollectorComplete && wizPlanStored);
        const label = complete
          ? "Complete"
          : wizEstablishOpenLabels.length
            ? `Information still needed: ${wizEstablishOpenLabels.join(", ")}`
            : !wizPlanStored
              ? "Information still needed — Confirm Plan (Shopping Cart needs stored Plan / share)"
              : "Information still needed";
        return (
          <p
            role="status"
            aria-label="Investment details status"
            className={
              complete
                ? "investment-details-status is-complete"
                : "investment-details-status is-missing"
            }
          >
            {label}
          </p>
        );
      })()}
      {wizAskSecondUrl && !wizSecondUrlTried ? (
        <section
          aria-label="Second distribution URL"
          className="process-a-second-url"
          data-part="second-distribution-url"
        >
          <p role="status">
            First history parse failed. Paste a second issuer URL once (same
            adapter). A second miss is a loud fail — no half adapter. Manual
            adapter stays parked.
          </p>
          <label>
            Second distribution URL
            <input
              aria-label="Second distribution URL"
              value={wizSecondUrl}
              onChange={(e) => setWizSecondUrl(e.target.value)}
              disabled={busy || writesBlocked}
            />
          </label>
          <button
            type="button"
            aria-label="Retry with second URL"
            disabled={busy || writesBlocked || !wizSecondUrl.trim()}
            onClick={() => void retryProcessASecondUrl()}
          >
            Retry with this URL
          </button>
        </section>
      ) : null}
      {wizSecondUrlTried ? (
        <p role="alert">
          Second distribution URL failed. Adapter not built. Manual adapter
          is parked. Collector stays incomplete.
        </p>
      ) : null}
      {wizAskInception || processANeedsInception ? (
        <section
          aria-label="Inception confirm"
          className="process-a-inception"
          data-part="inception-confirm"
        >
          <p role="status">
            Paid history is under 12
            {wizDecls.length
              ? ` (${formatCount(wizDecls.filter((d) => d.amountPerShareMinor > 0).length)} on file)`
              : ""}
            .
            {wizInceptionCandidate
              ? ` Search found inception ${wizInceptionCandidate}${
                  wizExpectedPaid != null
                    ? ` — expected ${formatCount(wizExpectedPaid)} paid periods since then`
                    : ""
                }. Is this name too new for 12 paid declarations?`
              : " Type the fund inception / start date if you know it, then Yes. No opens a ticket and leaves the collector incomplete. Confirm Plan with an Incomplete analysis reason can still store Plan for Shopping Cart either way."}
          </p>
          <label>
            Inception date
            <input
              aria-label="Inception date"
              type="date"
              value={wizInceptionOn}
              onChange={(e) => setWizInceptionOn(e.target.value)}
              disabled={busy || writesBlocked}
            />
          </label>
          <div className="buttons">
            <button
              type="button"
              aria-label="Inception yes"
              disabled={busy || writesBlocked || !wizInceptionOn.trim()}
              onClick={() => void confirmProcessAInception(true)}
            >
              Yes — store inception
            </button>
            <button
              type="button"
              aria-label="Inception no"
              disabled={busy || writesBlocked}
              onClick={() => void confirmProcessAInception(false)}
            >
              No
            </button>
          </div>
        </section>
      ) : null}
      <section
        aria-label="Mandatory data checklist"
        id="add-investment-checklist"
        data-section="add-investment-checklist"
      >
      <table aria-label="Retrieved versus unknown" data-part="retrieved-vs-unknown">
        <thead>
          <tr>
            <th scope="col">Field</th>
            <th scope="col">Value</th>
            <th scope="col">Status</th>
            <th scope="col">Checklist</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <th scope="row">Provider / type</th>
            <td>
              <input
                aria-label="Provider"
                value={wizProvider}
                onChange={(e) => setWizProvider(e.target.value)}
                disabled={busy || writesBlocked}
                placeholder="Provider"
              />
              {wizDeclSource.trim() ? (
                <span aria-label="Declaration source"> · {wizDeclSource}</span>
              ) : null}
            </td>
            <td>{wizProvider.trim() || wizDeclSource.trim() ? "retrieved" : "unknown"}</td>
            <td>
              {wizFieldDecision.provider ? (
                <span className="process-a-checklist-decision" aria-label="Provider decision">
                  {wizFieldDecision.provider === "accept" ? "Accepted" : "Skipped"}
                </span>
              ) : null}
              <button type="button" aria-label="Accept provider" disabled={busy || !wizProvider.trim()} onClick={() => void decideProcessAField("provider", "accept")}>Accept</button>
              <button type="button" aria-label="Skip provider" disabled={busy} onClick={() => void decideProcessAField("provider", "skip")}>Skip</button>
            </td>
          </tr>
            <tr>
              <th scope="row">Underlying</th>
              <td aria-label="Underlying">
                {wizUnderlying.trim() || "—"}
                <span role="note"> · set in Owner actions</span>
              </td>
              <td>{wizUnderlying.trim() ? "retrieved" : "unknown"}</td>
              <td>
                {wizFieldDecision.underlying ? (
                  <span className="process-a-checklist-decision" aria-label="Underlying decision">
                    {wizFieldDecision.underlying === "accept" ? "Accepted" : "Skipped"}
                  </span>
                ) : (
                  "Owner actions"
                )}
              </td>
            </tr>
            <tr>
              <th scope="row">Strategy characteristics</th>
              <td aria-label="Strategy characteristics">
                {strategyCharacteristics(wizLookthrough, wizUnderlying) || "—"}
              </td>
              <td>
                {strategyCharacteristics(wizLookthrough, wizUnderlying)
                  ? "retrieved"
                  : "unknown"}
              </td>
              <td />
            </tr>
          <tr>
            <th scope="row">Frequency</th>
            <td>
              <select
                aria-label="Payment frequency"
                value={wizFreq}
                onChange={(e) => setWizFreq(e.target.value)}
                disabled={busy || writesBlocked}
              >
                <option value="">Select frequency</option>
                {["Weekly", "Twice monthly", "Monthly", "Quarterly"].map((f) => (
                  <option key={f} value={f}>
                    {f}
                  </option>
                ))}
              </select>
            </td>
            <td>{parseCadence(wizFreq) ? "retrieved" : "unknown"}</td>
            <td>
              {wizFieldDecision.frequency ? (
                <span className="process-a-checklist-decision" aria-label="Frequency decision">
                  {wizFieldDecision.frequency === "accept" ? "Accepted" : "Skipped"}
                </span>
              ) : null}
              <button type="button" aria-label="Accept frequency" disabled={busy || !parseCadence(wizFreq)} onClick={() => void decideProcessAField("frequency", "accept")}>Accept</button>
              <button type="button" aria-label="Skip frequency" disabled={busy} onClick={() => void decideProcessAField("frequency", "skip")}>Skip</button>
            </td>
          </tr>
          <tr>
            <th scope="row">Last 12 declarations</th>
            <td>
              {wizDecls.length > 0
                ? `${formatCount(wizDecls.length)} paid`
                : "—"}
            </td>
            <td>{wizDecls.length > 0 ? "retrieved" : "unknown"}</td>
            <td />
          </tr>
          <tr>
            <th scope="row">Remaining-year dates</th>
            <td>
              {(wizRemaining?.payments?.length ?? wizUpcomingPays.length) > 0
                ? `${formatCount(wizRemaining?.payments?.length ?? wizUpcomingPays.length)} dates`
                : "—"}
            </td>
            <td>
              {(wizRemaining?.payments?.length ?? wizUpcomingPays.length) > 0
                ? "retrieved"
                : "unknown"}
            </td>
            <td>
              {wizFieldDecision.remaining_year ? (
                <span className="process-a-checklist-decision" aria-label="Remaining year decision">
                  {wizFieldDecision.remaining_year === "accept" ? "Accepted" : "Skipped"}
                </span>
              ) : null}
              <button type="button" aria-label="Accept remaining year" disabled={busy} onClick={() => void decideProcessAField("remaining_year", "accept")}>Accept</button>
              <button type="button" aria-label="Skip remaining year" disabled={busy} onClick={() => void decideProcessAField("remaining_year", "skip")}>Skip</button>
            </td>
          </tr>
          <tr>
            <th scope="row">Dividend basis</th>
            <td aria-label="Dividend basis summary">
              {wizReview && (wizReview.minMinor != null || wizReview.mostCurrentMinor != null) ? (
                <>
                  {wizReview.minMinor != null
                    ? `Min ${formatPerShare(wizReview.minMinor, wizReview.amountScale)}`
                    : null}
                  {wizReview.maxMinor != null
                    ? ` · Max ${formatPerShare(wizReview.maxMinor, wizReview.amountScale)}`
                    : null}
                  {wizReview.averageMinor != null
                    ? ` · Avg ${formatPerShare(wizReview.averageMinor, wizReview.amountScale)}`
                    : null}
                  {wizReview.mostCurrentMinor != null
                    ? ` · Most Current ${formatPerShare(wizReview.mostCurrentMinor, wizReview.amountScale)}`
                    : null}
                  {wizReview.avg6Minor != null
                    ? ` · Avg 6 ${formatPerShare(wizReview.avg6Minor, wizReview.amountScale)}`
                    : null}
                </>
              ) : (
                "—"
              )}
            </td>
            <td>
              {wizReview?.mostCurrentMinor != null || wizReview?.averageMinor != null
                ? "retrieved"
                : "unknown"}
            </td>
            <td />
          </tr>
          <tr>
            <th scope="row">Last price</th>
            <td>
              {wizPriceState?.priceMinor != null && wizPriceState.priceMinor > 0
                ? `${scaledDollars(wizPriceState.priceMinor, wizPriceState.scale)} (${wizPriceState.freshness || "unknown"})`
                : "—"}
            </td>
            <td>
              {wizPriceState?.priceMinor != null && wizPriceState.priceMinor > 0
                ? "retrieved"
                : "unknown"}
            </td>
            <td />
          </tr>
          <tr>
            <th scope="row">Suggested tier</th>
            <td>
              {(wizTierSuggestion?.suggestedTier || wizLookthrough.riskTierSuggestion || "").trim()
                ? `${wizTierSuggestion?.suggestedTier || wizLookthrough.riskTierSuggestion}${
                    (wizTierSuggestion?.reason || wizLookthrough.riskTierSuggestionReason)
                      ? ` — ${wizTierSuggestion?.reason || wizLookthrough.riskTierSuggestionReason}`
                      : ""
                  }`
                : "—"}
              {wizRisk.trim() ? (
                <span role="note"> · owner Risk: {wizRisk.trim()}</span>
              ) : (
                <span role="note"> · set Risk in Owner actions</span>
              )}
              {wizBacktestNeeded ? (
                <p role="status" aria-label="Backtest dates required for tier suggest">
                  Backtest dates required for ClassificationSuggest. Risk tier is owner-only.
                </p>
              ) : null}
            </td>
            <td>
              {wizRisk.trim()
                ? "owner set"
                : (wizTierSuggestion?.suggestedTier || wizLookthrough.riskTierSuggestion || "").trim()
                  ? "suggested"
                  : wizBacktestNeeded
                    ? "needs backtest"
                    : "unknown"}
            </td>
            <td>
              {wizFieldDecision.risk_tier ? (
                <span className="process-a-checklist-decision" aria-label="Risk decision">
                  {wizFieldDecision.risk_tier === "accept" ? "Accepted" : "Skipped"}
                </span>
              ) : (
                "Owner actions"
              )}
              {wizBacktestNeeded ? (
                <button type="button" aria-label="Skip backtest" disabled={busy} onClick={() => void decideProcessAField("backtest", "skip")}>Skip backtest</button>
              ) : null}
            </td>
          </tr>
          <tr>
            <th scope="row">Investment type</th>
            <td>
              <select
                aria-label="Investment type"
                value={normalizeDivType(wizDivType)}
                onChange={(e) => setWizDivType(e.target.value)}
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
            <td>
              {normalizeDivType(wizDivType)
                ? wizCollectorGaps.includes("div_type")
                  ? "chosen"
                  : "retrieved"
                : "unknown"}
            </td>
            <td>
              {wizFieldDecision.div_type ? (
                <span className="process-a-checklist-decision" aria-label="Div type decision">
                  {wizFieldDecision.div_type === "accept" ? "Accepted" : "Skipped"}
                </span>
              ) : null}
              <button
                type="button"
                aria-label="Accept div type"
                disabled={busy || !normalizeDivType(wizDivType)}
                onClick={() => void decideProcessAField("div_type", "accept")}
              >
                Accept
              </button>
              <button
                type="button"
                aria-label="Skip div type"
                disabled={busy}
                onClick={() => void decideProcessAField("div_type", "skip")}
              >
                Skip
              </button>
            </td>
          </tr>
        </tbody>
      </table>
      </section>

      {(researchNotes || wizAiThesis) ? (
        <section
          className="research-notes-panel"
          aria-label="Research notes"
          id="add-investment-notes"
          data-section="add-investment-notes"
          data-part="research-notes"
        >
          <h4>Research notes</h4>
          <p className="research-notes-overview">
            {researchNotes?.overview || wizAiThesis}
          </p>
          {(researchNotes?.suggestedTier ||
            wizTierSuggestion?.suggestedTier ||
            wizLookthrough.riskTierSuggestion) ? (
            <p role="note" aria-label="Suggested risk tier">
              Suggested risk (not applied):{" "}
              {researchNotes?.suggestedTier ||
                wizTierSuggestion?.suggestedTier ||
                wizLookthrough.riskTierSuggestion}
              . Owner sets Risk manually — never auto-applied.
            </p>
          ) : null}
        </section>
      ) : null}

      {wizDecls.length > 0 ? (
        <div className="table-wrap" data-part="last-declarations">
          <p>Last {formatCount(wizDecls.length)} declarations (newest first).</p>
          <table aria-label="Last declarations">
            <thead>
              <tr>
                <th scope="col">Period</th>
                <th className="numeric" scope="col">Per share</th>
                <th scope="col">Source</th>
              </tr>
            </thead>
            <tbody>
              {wizDecls.map((d, i) => (
                <tr key={`${d.paymentPeriod ?? i}`}>
                  <td>{d.paymentPeriod ?? "—"}</td>
                  <td className="numeric">
                    {formatPerShare(d.amountPerShareMinor, d.amountScale ?? 4)}
                  </td>
                  <td>{d.source ?? "—"}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      ) : null}

      {(wizRemaining?.payments?.length ?? 0) > 0 ? (
        <div className="table-wrap" data-part="remaining-year-dates">
          <p>Remaining-year pay dates.</p>
          <table aria-label="Remaining year dates">
            <thead>
              <tr>
                <th scope="col">Pay on</th>
                <th scope="col">Source</th>
              </tr>
            </thead>
            <tbody>
              {wizRemaining!.payments.map((p) => (
                <tr key={p.payOn}>
                  <td>{p.payOn}</td>
                  <td>{dateProvenanceLabel(p.dateProvenance) || "—"}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      ) : null}

      <section
        aria-label="Owner plan and tier actions"
        id="add-investment-owner-actions"
        data-section="add-investment-owner-actions"
      >
        <h4>Owner actions</h4>
        <p
          className="process-a-dividend-basis"
          aria-label="Dividend statistics"
          data-part="dividend-statistics"
        >
          {wizReview &&
          (wizReview.minMinor != null ||
            wizReview.maxMinor != null ||
            wizReview.averageMinor != null) ? (
            <>
              Min{" "}
              {wizReview.minMinor != null
                ? formatPerShare(wizReview.minMinor, wizReview.amountScale)
                : "—"}
              {" · "}
              Max{" "}
              {wizReview.maxMinor != null
                ? formatPerShare(wizReview.maxMinor, wizReview.amountScale)
                : "—"}
              {" · "}
              Average{" "}
              {wizReview.averageMinor != null
                ? formatPerShare(wizReview.averageMinor, wizReview.amountScale)
                : "—"}
              {wizReview.mostCurrentMinor != null
                ? ` · Most Current ${formatPerShare(wizReview.mostCurrentMinor, wizReview.amountScale)}`
                : null}
              {wizReview.avg6Minor != null
                ? ` · Avg 6 ${formatPerShare(wizReview.avg6Minor, wizReview.amountScale)}`
                : null}
            </>
          ) : (
            "Min / Max / Average unavailable until declarations retrieve."
          )}
        </p>
        <p>
          {processALotCount > 0
            ? "Recreate validates stored facts. It does not re-import pays, identity, ROC, or Plan. Confirm Plan is not required unless you change Plan."
            : "Enter Plan / share from the dividend basis above, then Confirm Plan. Confirm Plan writes Plan / share only. It does not replace stored paid history, lots, or remaining-year dates."}
          {parseCadence(wizFreq) ? "" : " Frequency research is still required."}
        </p>
        {(() => {
          const planScale = 4;
          const proposedMinor = Number(wizPlan);
          const proposedOk = Number.isFinite(proposedMinor);
          const proposedLabel = proposedOk
            ? formatPerShare(Math.round(proposedMinor * 10 ** planScale), planScale)
            : "—";
          const storedLabel = wizStoredPlan
            ? formatPerShare(wizStoredPlan.minor, wizStoredPlan.scale)
            : "none";
          const planChanges =
            wizStoredPlan != null &&
            proposedOk &&
            storedLabel !== proposedLabel;
          return (
            <div
              className="process-a-plan-diff"
              aria-label="Proposed plan changes"
              data-part="proposed-plan-changes"
            >
              <p>
                Stored Plan {storedLabel} → this button {proposedLabel}
                {planChanges
                  ? `. This will change the stored Plan from ${storedLabel} to ${proposedLabel}.`
                  : ". No Plan amount change."}
              </p>
              <p>
                {processALotCount > 0 && !planChanges
                  ? "Proposed writes: none. Stored facts stay. Validation only."
                  : "Proposed writes: Plan / share only. Not written: stored paid history, lots, remaining-year dates, last price, ROC, or tier (Apply tier is a separate button)."}
              </p>
              {processALotCount > 0 ? (
                <p>
                  Recreate lists stored pays at five decimal places so
                  0.13000 is not shown as 0.13. It does not re-import
                  them.
                </p>
              ) : null}
            </div>
          );
        })()}
        <div className="form-grid" data-part="owner-plan-inputs">
          <label>
            Underlying
            <input
              aria-label="Owner underlying"
              value={wizUnderlying}
              onChange={(e) => setWizUnderlying(e.target.value)}
              disabled={busy || writesBlocked}
              placeholder="e.g. MSTR"
            />
          </label>
          <label>
            Risk tier
            <select
              aria-label="Owner risk tier"
              value={wizRisk}
              onChange={(e) => setWizRisk(e.target.value)}
              disabled={busy || writesBlocked}
            >
              <option value="">Select tier</option>
              {RISK_TIERS.map((t) => (
                <option key={t} value={t}>
                  {t}
                </option>
              ))}
            </select>
          </label>
          <label>
            Plan / share
            <input
              aria-label="Plan per share"
              className="per-share-input"
              inputMode="decimal"
              value={wizPlan}
              onChange={(e) => setWizPlan(e.target.value)}
              disabled={busy || writesBlocked}
              placeholder="Owner-entered — not auto-filled"
            />
          </label>
          <label>
            Plan reason
            <select
              aria-label="Plan reason"
              value={wizPlanReason}
              onChange={(e) => setWizPlanReason(e.target.value)}
              disabled={busy || writesBlocked}
            >
              <option value="">Select reason</option>
              {PLAN_REASONS.map((r) => (
                <option key={r} value={r}>
                  {r}
                </option>
              ))}
            </select>
          </label>
          {(() => {
            const obs =
              wizReview?.observationCount ??
              wizDecls.filter((d) => d.amountPerShareMinor > 0).length;
            const shortHistory =
              Boolean(wizReview?.incompleteReasonRequired) ||
              (obs > 0 && obs < 6);
            if (!shortHistory) return null;
            if (wizInceptionOn.trim()) {
              return (
                <p role="status" aria-label="Incomplete analysis covered by inception">
                  Incomplete analysis reason not needed — inception {wizInceptionOn.trim()} covers short paid history (Confirm Plan will record “Recent inception date”).
                </p>
              );
            }
            return (
              <label>
                Incomplete analysis reason
                <select
                  aria-label="Incomplete analysis reason"
                  value={wizIncomplete}
                  onChange={(e) => setWizIncomplete(e.target.value)}
                  disabled={busy || writesBlocked}
                >
                  <option value="">Select reason</option>
                  {INCOMPLETE_REASONS.map((r) => (
                    <option key={r} value={r}>
                      {r}
                    </option>
                  ))}
                </select>
              </label>
            );
          })()}
        </div>
        {processAPlanBlockReason ? (
          <p role="status" aria-label="Confirm Plan blocked reason">
            Confirm Plan needs: {processAPlanBlockReason}
          </p>
        ) : (
          <p role="status" aria-label="Confirm Plan ready">
            Confirm Plan is ready — click it to write Plan into history for Shopping Cart.
          </p>
        )}
        <div className="buttons" data-part="owner-plan-actions">
          <button
            type="button"
            aria-label="Save identity"
            className={wizDirty ? "is-unsaved" : undefined}
            disabled={busy || writesBlocked || !wizSecurityId}
            onClick={() => void saveProcessAOwnerIdentity()}
          >
            Save underlying / tier
          </button>
          <button
            type="button"
            aria-label="Apply owner risk tier"
            disabled={
              busy ||
              writesBlocked ||
              !wizSecurityId ||
              !RISK_TIERS.includes(wizRisk.trim())
            }
            onClick={() => void applyOwnerRiskTier()}
          >
            Apply tier
          </button>
          <button
            type="button"
            aria-label="Use Most Current as Plan"
            disabled={
              busy ||
              writesBlocked ||
              (wizReview?.mostCurrentMinor == null && wizDecls[0] == null)
            }
            onClick={() => {
              const minor = wizReview?.mostCurrentMinor ?? wizDecls[0]?.amountPerShareMinor;
              const scale = wizReview?.amountScale ?? wizDecls[0]?.amountScale ?? 4;
              if (minor == null) return;
              setWizPlan(formatPerShare(minor, scale));
              setWizPlanReason("Match Most Current");
            }}
          >
            Use Most Current as Plan
          </button>
          <button
            type="button"
            aria-label="Use Avg 6 as Plan"
            disabled={busy || writesBlocked || wizReview?.avg6Minor == null}
            onClick={() => {
              if (wizReview?.avg6Minor == null) return;
              setWizPlan(formatPerShare(wizReview.avg6Minor, wizReview.amountScale));
              setWizPlanReason("Match Avg 6 (owner typed)");
            }}
          >
            Use Avg 6 as Plan
          </button>
          <button
            type="button"
            aria-label="Confirm Plan"
            className={wizDirty ? "is-unsaved" : undefined}
            disabled={
              busy ||
              writesBlocked ||
              Boolean(processAPlanBlockReason)
            }
            onClick={() => void confirmPlan()}
          >
            {processALotCount > 0
              ? "Update Plan / share"
              : "Confirm Plan"}
          </button>
          <button
            type="button"
            aria-label="Apply suggested tier"
            disabled={
              busy ||
              writesBlocked ||
              !wizSecurityId ||
              !RISK_TIERS.includes(
                (wizTierSuggestion?.suggestedTier ||
                  wizLookthrough.riskTierSuggestion ||
                  "").trim(),
              )
            }
            onClick={() => void applyWizSuggestedTier()}
          >
            Apply tier
          </button>
        </div>
        {wizPlanStored ? (
          <p role="status">Plan confirmed — stored in plan history for Shopping Cart.</p>
        ) : wizPlan.trim() ? (
          <p role="status" aria-label="Plan typed but not stored">
            Plan is typed here but not stored yet. Shopping Cart week/month/year stay blank until Confirm Plan succeeds.
          </p>
        ) : null}
        {wizRisk.trim() ? <p role="status">Applied tier: {wizRisk}</p> : null}
        {(() => {
          const complete =
            wizEstablishComplete || (wizCollectorComplete && wizPlanStored);
          const label = complete
            ? "Complete"
            : wizEstablishOpenLabels.length
              ? `Information still needed: ${wizEstablishOpenLabels.join(", ")}`
              : !wizPlanStored
                ? "Information still needed — Confirm Plan (Shopping Cart needs stored Plan / share)"
                : "Information still needed";
          return (
            <p
              role="status"
              aria-label="Investment details status"
              className={
                complete
                  ? "investment-details-status is-complete"
                  : "investment-details-status is-missing"
              }
            >
              {label}
            </p>
          );
        })()}
      </section>

      <section
        aria-label="ROC research strip"
        className="roc-research-strip"
        id="add-investment-roc"
        data-section="add-investment-roc"
        data-part="roc-research"
      >
        <h4>ROC research</h4>
        <p>
          {wizRoc && wizRoc.rocPctMinor != null
            ? `Proposed ${((wizRoc.rocPctMinor as number) / 10 ** wizRoc.scale).toFixed(wizRoc.scale)}% (2026 estimate, ${wizRoc.kind || "estimate"})${
                wizRoc.sourceUrl ? ` — ${wizRoc.sourceUrl}` : ""
              }. Not research-complete.`
            : "2026 estimate unknown — not 0%. 2025 actual stays N/A when the position was not held that year."}
        </p>
        <label>
          Template ROC
          <input
            aria-label="Template ROC"
            value={wizRocUrl}
            onChange={(e) => setWizRocUrl(e.target.value)}
            disabled={busy || writesBlocked}
            placeholder="https://…19a-1…"
          />
        </label>
        <button
          type="button"
          aria-label="Store ROC URL"
          disabled={busy || writesBlocked || !wizRocUrl.trim()}
          onClick={() => void submitProcessARocUrl()}
        >
          Store ROC URL and parse
        </button>
        <label>
          Manual ROC %
          <input
            aria-label="Manual ROC percent"
            inputMode="decimal"
            value={wizRocPct}
            onChange={(e) => setWizRocPct(e.target.value)}
            disabled={busy || writesBlocked}
            placeholder="e.g. 80 or 99.70"
          />
        </label>
        <button
          type="button"
          aria-label="Store manual ROC percent"
          className={wizRocPct.trim() ? "is-unsaved" : undefined}
          disabled={busy || writesBlocked || !wizRocPct.trim()}
          onClick={() => void submitProcessAManualRoc()}
        >
          Store manual ROC %
        </button>
        <div className="buttons">
          <button
            type="button"
            aria-label="Accept ROC estimate"
            disabled={busy}
            onClick={() => void acceptProcessARoc()}
          >
            Accept ROC
          </button>
          <button type="button" aria-label="Skip ROC estimate" disabled={busy} onClick={() => void decideProcessAField("roc_estimate", "skip")}>
            Skip ROC
          </button>
        </div>
      </section>
    </section>
  ) : null}
  <ReadinessChecklist
    rows={wizEstablishChecklist}
    complete={wizEstablishComplete}
    openLabels={wizEstablishOpenLabels}
  />
</section>
  );
}
