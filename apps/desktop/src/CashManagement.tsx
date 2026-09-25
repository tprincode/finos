import { formatUsd } from "@finos/ui-components";
import type {
  AccountListItem,
  CarRocPlanGet,
  CashManagementMonthGet,
  CashManagementRemindersGet,
  CashManagementWeekGet,
  DividendPerformanceGet,
  MagiProjection,
  TaxPlanningGet,
  TrendsWeekPoint,
} from "@finos/app-contracts";
import { useEffect, useRef, useState, type ReactNode } from "react";
import {
  CashWeekDesk,
  type CashWeekOverview,
  type OpenWeekIncome,
  type TrendIncomePoint,
} from "./features/cash/CashWeekDesk";

export type CashDistributionYtd = {
  grossMinor: number;
  federalWithholdingMinor?: number;
  stateWithholdingMinor?: number;
  netMinor?: number;
  lines: Array<{
    activityType: string;
    accountName: string;
    amountMinor: number;
    occurredOn: string;
    scale: number;
    federalWithholdingMinor?: number;
    stateWithholdingMinor?: number;
    netMinor?: number;
    accountKind?: string;
    taxSection?: string;
  }>;
  accountTotals?: Array<{
    accountName: string;
    accountKind: string;
    taxSection: string;
    grossMinor: number;
    netMinor: number;
  }>;
  sections?: Array<{
    id: string;
    label: string;
    taxNote: string;
    grossMinor: number;
    netMinor: number;
  }>;
  scale: number;
};

export type CashTaxAcaMonitor = {
  federalWithholdingMinor: number;
  projectedLiabilityMinor?: number | null;
  gapMinor?: number | null;
  warning: boolean;
  acaThresholdMinor?: number | null;
  acaCoverageYear?: number | null;
  note: string;
  scale: number;
};

type CashActivity = "chooser" | "distribution" | "ssa" | "withdrawal" | null;

const DIST_TYPES = [
  { value: "IRA_Distribution", label: "IRA distribution" },
  { value: "Roth_Distribution", label: "Roth distribution" },
];

const DIST_STEPS = ["Type", "Account", "Amounts", "Review"] as const;
const SSA_STEPS = ["Payee", "Account", "Received", "Review"] as const;
const WITHDRAW_STEPS = ["Account", "Amount", "Review"] as const;

function formatPlanUsd(
  minor: number | null | undefined,
  scale: number,
): string {
  if (minor == null) return "unknown";
  return formatUsd(minor, scale);
}

function formatCarUsd(
  minor: number | null | undefined,
  scale: number,
): string {
  return formatUsd(minor ?? 0, scale);
}

function formatRocCell(
  minor: number | null | undefined,
  scale: number,
  reason?: string | null,
): string {
  if (minor == null) {
    const text = reason?.trim();
    return text ? text : "unknown";
  }
  return formatUsd(minor, scale);
}

function addKnown(left: number | null, right: number | null): number | null {
  if (left == null || right == null) return null;
  return left + right;
}

function CarTaxPlanTable({ plan }: { plan: CarRocPlanGet }) {
  const scale = plan.scale;
  const rocReason = plan.ytdRocUnknownReason;
  const rocYtd = plan.ytdRocMinor;
  const rocPlanned = plan.remainingRocMinor;
  const rows = [
    {
      key: "ordinary",
      label: "Ordinary",
      ytd: plan.ytdOrdinaryMinor ?? 0,
      planned: plan.remainingOrdinaryMinor ?? 0,
      ytdText: formatCarUsd(plan.ytdOrdinaryMinor ?? 0, scale),
      plannedText: formatCarUsd(plan.remainingOrdinaryMinor ?? 0, scale),
      totalText: formatCarUsd(
        (plan.ytdOrdinaryMinor ?? 0) + (plan.remainingOrdinaryMinor ?? 0),
        scale,
      ),
    },
    {
      key: "roc",
      label: "ROC",
      ytd: rocYtd,
      planned: rocPlanned,
      ytdText: formatRocCell(rocYtd, scale, rocReason),
      plannedText: formatRocCell(rocPlanned, scale, rocPlanned == null ? rocReason : null),
      totalText: formatRocCell(addKnown(rocYtd, rocPlanned), scale, rocReason),
    },
    {
      key: "lt",
      label: "Long Term Capital Gains",
      ytd: plan.ytdLongTermGainMinor ?? 0,
      planned: 0,
      ytdText: formatCarUsd(plan.ytdLongTermGainMinor ?? 0, scale),
      plannedText: formatCarUsd(0, scale),
      totalText: formatCarUsd(plan.ytdLongTermGainMinor ?? 0, scale),
    },
    {
      key: "st",
      label: "Short Term Capital Gains",
      ytd: plan.ytdShortTermGainMinor ?? 0,
      planned: 0,
      ytdText: formatCarUsd(plan.ytdShortTermGainMinor ?? 0, scale),
      plannedText: formatCarUsd(0, scale),
      totalText: formatCarUsd(plan.ytdShortTermGainMinor ?? 0, scale),
    },
  ];
  const knownYtd = rows.every((row) => row.ytd != null)
    ? rows.reduce((sum, row) => sum + (row.ytd as number), 0)
    : null;
  const knownPlanned = rows.every((row) => row.planned != null)
    ? rows.reduce((sum, row) => sum + (row.planned as number), 0)
    : null;
  return (
    <div className="table-wrap car-tax-table-wrap">
      <table aria-label="Car account tax planning">
        <thead>
          <tr>
            <th scope="col"> </th>
            <th scope="col" className="numeric">
              YTD
            </th>
            <th scope="col" className="numeric">
              Planned
            </th>
            <th scope="col" className="numeric">
              Total YTD + Planned
            </th>
          </tr>
        </thead>
        <tbody>
          {rows.map((row) => (
            <tr key={row.key} className={`car-tax-row car-tax-${row.key}`}>
              <th scope="row">{row.label}</th>
              <td className="numeric">{row.ytdText}</td>
              <td className="numeric">{row.plannedText}</td>
              <td className="numeric">{row.totalText}</td>
            </tr>
          ))}
        </tbody>
        <tfoot>
          <tr>
            <th scope="row">Total</th>
            <td className="numeric">{formatRocCell(knownYtd, scale, rocReason)}</td>
            <td className="numeric">{formatRocCell(knownPlanned, scale, rocReason)}</td>
            <td className="numeric">
              {formatRocCell(addKnown(knownYtd, knownPlanned), scale, rocReason)}
            </td>
          </tr>
        </tfoot>
      </table>
    </div>
  );
}

function magiImpactLabel(impact: string): string {
  if (impact === "none") {
    return "None";
  }
  if (impact === "magi_ltcg") {
    return "MAGI · LTCG rate";
  }
  return "MAGI";
}

function formatPlanCell(
  minor: number | null | undefined,
  scale: number,
  reason?: string | null,
): string {
  if (minor == null) {
    return reason?.trim() ? `— (${reason})` : "—";
  }
  return formatUsd(minor, scale);
}

function HouseholdTaxTable({ plan }: { plan: TaxPlanningGet }) {
  const scale = plan.scale;
  const groups = [plan.magiIncluded, plan.notMagi, plan.allSources];
  return (
    <div className="table-wrap car-tax-table-wrap">
      <table aria-label="Tax Planning income">
        <thead>
          <tr>
            <th scope="col"> </th>
            <th scope="col" className="numeric">
              YTD
            </th>
            <th scope="col" className="numeric">
              Projected
            </th>
            <th scope="col" className="numeric">
              Total
            </th>
            <th scope="col">Tax impact</th>
          </tr>
        </thead>
        <tbody>
          {plan.rows.map((row) => (
            <tr key={row.key}>
              <th scope="row">{row.label}</th>
              <td className="numeric">{formatPlanCell(row.ytdMinor, scale)}</td>
              <td className="numeric">{formatPlanCell(row.projectedMinor, scale)}</td>
              <td className="numeric">{formatPlanCell(row.totalMinor, scale)}</td>
              <td>{magiImpactLabel(row.magiImpact)}</td>
            </tr>
          ))}
        </tbody>
        <tfoot>
          {groups.map((group) => (
            <tr key={group.label}>
              <th scope="row">{group.label}</th>
              <td className="numeric">{formatPlanCell(group.ytdMinor, scale)}</td>
              <td className="numeric">{formatPlanCell(group.projectedMinor, scale)}</td>
              <td className="numeric">{formatPlanCell(group.totalMinor, scale)}</td>
              <td> </td>
            </tr>
          ))}
        </tfoot>
      </table>
    </div>
  );
}

function MagiThresholdBoard({ magi }: { magi: MagiProjection | null }) {
  if (!magi) {
    return <p role="status">Loading MAGI…</p>;
  }
  const scale = magi.applicableThreshold.scale;
  const money = (m: { amountMinor: number; scale: number }) =>
    formatUsd(m.amountMinor, m.scale ?? scale);
  return (
    <div className="table-wrap car-tax-table-wrap">
      <table aria-label="Tax Planning MAGI">
        <thead>
          <tr>
            <th scope="col">MAGI</th>
            <th scope="col" className="numeric">
              Amount
            </th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <th scope="row">Threshold</th>
            <td className="numeric">{money(magi.applicableThreshold)}</td>
          </tr>
          <tr>
            <th scope="row">YTD included</th>
            <td className="numeric">{money(magi.actualIncludedYtd)}</td>
          </tr>
          <tr>
            <th scope="row">Known remaining</th>
            <td className="numeric">{money(magi.knownRemaining)}</td>
          </tr>
          <tr>
            <th scope="row">Forecast</th>
            <td className="numeric">{money(magi.baseForecast)}</td>
          </tr>
          <tr>
            <th scope="row">Conservative forecast</th>
            <td className="numeric">{money(magi.conservativeForecast)}</td>
          </tr>
          <tr>
            <th scope="row">Headroom</th>
            <td className="numeric">{money(magi.protectedHeadroom)}</td>
          </tr>
          <tr>
            <th scope="row">Decision</th>
            <td>{magi.decisionState}</td>
          </tr>
        </tbody>
      </table>
    </div>
  );
}

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

function nextOf<T extends string>(steps: readonly T[], step: T): T {
  const i = steps.indexOf(step);
  return steps[Math.min(i + 1, steps.length - 1)] ?? step;
}

function prevOf<T extends string>(steps: readonly T[], step: T): T {
  const i = steps.indexOf(step);
  return steps[Math.max(i - 1, 0)] ?? step;
}

function isSsaAccountName(name: string): boolean {
  return name.trim().toLowerCase() === "external";
}

/** Same rule the post uses: owner only sees accounts that can take this cash type. */
function accountsForCashType(
  accounts: AccountListItem[],
  activityType: string,
): AccountListItem[] {
  return accounts.filter((a) => {
    const kind = a.kind.trim().toLowerCase();
    if (activityType === "SSA") return isSsaAccountName(a.name);
    if (activityType === "IRA_Distribution") return kind === "ira";
    if (activityType === "Roth_Distribution")
      return kind === "roth" || kind === "fi_roth";
    if (activityType === "Withdrawal")
      return kind === "taxable" && !isSsaAccountName(a.name);
    return false;
  });
}

export function CashManagementPanel({
  desk = "weekly",
  week,
  month,
  reminders,
  magi,
  accounts,
  busy,
  distributions,
  taxMonitor,
  carRocPlan,
  taxPlanning,
  children,
  weekAhead,
  cashElements,
  cashRegister,
  cashYtd,
  weekDesk,
  weekJustSavedAt,
  onReload,
  onSave,
  onSsaConfirm,
  onDirtyChange,
}: {
  desk?: "elements" | "weekly" | "car" | "cashflow";
  week: CashManagementWeekGet | null;
  month: CashManagementMonthGet | null;
  reminders: CashManagementRemindersGet | null;
  magi: MagiProjection | null;
  accounts: AccountListItem[];
  busy?: boolean;
  distributions?: CashDistributionYtd | null;
  taxMonitor?: CashTaxAcaMonitor | null;
  carRocPlan?: CarRocPlanGet | null;
  taxPlanning?: TaxPlanningGet | null;
  children?: ReactNode;
  weekAhead?: ReactNode;
  cashElements?: ReactNode;
  cashRegister?: ReactNode;
  cashYtd?: ReactNode;
  weekDesk?: {
    weeks?: TrendsWeekPoint[] | null;
    points?: TrendIncomePoint[];
    dividendPerf?: DividendPerformanceGet | null;
    overview?: CashWeekOverview | null;
    openWeek?: OpenWeekIncome | null;
    incomePlanWeek?: OpenWeekIncome | null;
  };
  weekJustSavedAt?: number;
  onReload: (asOf: string) => void;
  onSave: (body: Record<string, unknown>) => Promise<boolean>;
  onSsaConfirm: (body: Record<string, unknown>) => Promise<boolean>;
  onDirtyChange?: (dirty: boolean) => void;
}) {
  const [asOf, setAsOf] = useState(week?.periodEnd ?? "");
  const [activity, setActivity] = useState<CashActivity>(null);
  const [distStep, setDistStep] = useState<(typeof DIST_STEPS)[number]>("Type");
  const [ssaStep, setSsaStep] = useState<(typeof SSA_STEPS)[number]>("Payee");
  const [withdrawStep, setWithdrawStep] =
    useState<(typeof WITHDRAW_STEPS)[number]>("Account");
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
  const [ssaPayee, setSsaPayee] = useState<"barbara" | "tom">("tom");
  const [ssaDirty, setSsaDirty] = useState(false);
  const [distAccount, setDistAccount] = useState("all");
  const handledSaveAt = useRef(0);

  useEffect(() => {
    if (weekJustSavedAt && weekJustSavedAt !== handledSaveAt.current) {
      handledSaveAt.current = weekJustSavedAt;
      setActivity("chooser");
    }
  }, [weekJustSavedAt]);

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
    if (!ssaDirty) {
      const firstOpen = reminders.ssaPayees?.find((p) => p.status === "unconfirmed");
      if (firstOpen?.payee === "barbara" || firstOpen?.payee === "tom") {
        setSsaPayee(firstOpen.payee);
      }
    }
  }, [reminders, ssaDirty, ssaOccurredOn]);

  useEffect(() => {
    onDirtyChange?.(dirty || ssaDirty);
    return () => onDirtyChange?.(false);
  }, [dirty, ssaDirty, onDirtyChange]);

  const scale = week?.scale ?? reminders?.scale ?? 2;
  const postingType =
    activity === "withdrawal" ? "Withdrawal" : activityType;
  const grossMinor = dollarsToMinor(gross, scale);
  const fedMinor = dollarsToMinor(fed, scale) ?? 0;
  const stateMinor = dollarsToMinor(state, scale) ?? 0;
  const netMinor =
    grossMinor == null ? null : grossMinor - fedMinor - stateMinor;
  const identityOk = netMinor != null && netMinor >= 0;
  const rothBlocked =
    postingType === "Roth_Distribution" && (fedMinor !== 0 || stateMinor !== 0);
  const ssaReceivedMinor = dollarsToMinor(ssaReceived, scale);
  const ssaExpected =
    ssaPayee === "barbara"
      ? (reminders?.ssaPayees?.find((p) => p.payee === "barbara")?.expectedMinor ??
        133100)
      : (reminders?.tomSsa.expectedMinor ?? 286500);
  const ssaVariance =
    ssaReceivedMinor != null && ssaReceivedMinor !== ssaExpected;
  const magiAdd = magiAddMinor(postingType, grossMinor);
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
    setDistStep("Type");
    setWithdrawStep("Account");
  };

  const resetSsa = () => {
    setSsaReceived("");
    setSsaOccurredOn(reminders?.asOfDate ?? ssaOccurredOn);
    if (reminders?.tomSsa.suggestedAccountId) {
      setSsaAccountId(reminders.tomSsa.suggestedAccountId);
    }
    setSsaDirty(false);
    setSsaStep("Payee");
  };

  const backToChooser = () => {
    reset();
    resetSsa();
    setActivity("chooser");
  };

  const openActivity = (next: CashActivity) => {
    if (next === "distribution") {
      const nextType =
        reminders?.saturdayDraft.activityType ?? "IRA_Distribution";
      setActivityType(nextType);
      setDistStep("Type");
      if (
        !accountsForCashType(accounts, nextType).some(
          (a) => a.accountId === accountId,
        )
      ) {
        setAccountId("");
      }
    }
    if (next === "withdrawal") {
      setActivityType("Withdrawal");
      setWithdrawStep("Account");
      if (
        !accountsForCashType(accounts, "Withdrawal").some(
          (a) => a.accountId === accountId,
        )
      ) {
        setAccountId("");
      }
    }
    if (next === "ssa") {
      setSsaStep("Payee");
    }
    setActivity(next);
  };

  if (desk === "car") {
    return (
      <div className="cash-management" aria-label="Cash Management">
        <section className="car-tax-plan" aria-label="Cash Management Tax Planning">
          <h3>MAGI threshold</h3>
          <MagiThresholdBoard magi={magi} />
          {taxPlanning ? (
            <>
              <h3>Income by tax type</h3>
              <HouseholdTaxTable plan={taxPlanning} />
            </>
          ) : (
            <p role="status">Loading Tax Planning…</p>
          )}
          {carRocPlan ? (
            <>
              <h3>Car</h3>
              <CarTaxPlanTable plan={carRocPlan} />
            </>
          ) : null}
          {cashYtd}
        </section>
      </div>
    );
  }

  if (desk === "elements") {
    return (
      <div className="cash-management" aria-label="Cash Management">
        {cashElements}
      </div>
    );
  }

  if (desk === "cashflow") {
    return (
      <div className="cash-management" aria-label="Cash Management">
        {cashRegister}
      </div>
    );
  }

  if (!week) {
    return (
      <div className="cash-management" aria-label="Cash Management">
        {children}
        {weekAhead}
        {children || weekAhead ? null : <p>Loading Cash Management…</p>}
      </div>
    );
  }

  const distCanAdvance =
    distStep === "Type"
      ? activityType !== ""
      : distStep === "Account"
        ? accountId !== "" && occurredOn !== ""
        : distStep === "Amounts"
          ? grossMinor != null && identityOk && !rothBlocked
          : true;
  const withdrawCanAdvance =
    withdrawStep === "Account"
      ? accountId !== "" && occurredOn !== ""
      : withdrawStep === "Amount"
        ? grossMinor != null && identityOk
        : true;
  const ssaCanAdvance =
    ssaStep === "Payee"
      ? ssaPayee === "barbara" || ssaPayee === "tom"
      : ssaStep === "Account"
        ? ssaAccountId !== "" && ssaOccurredOn !== ""
        : ssaStep === "Received"
          ? ssaReceivedMinor != null
          : true;

  return (
    <div className="cash-management" aria-label="Cash Management">
      {desk === "weekly" ? children : null}
      {desk === "weekly" ? weekAhead : null}
      {desk === "weekly" && activity === "chooser" ? (
        <div className="cash-follow-up" aria-label="Cash week follow-up">
          <p>
            Do you have any distributions, withdrawals, or cash payments to
            declare this week?
          </p>
          <div className="buttons">
            <button
              type="button"
              aria-label="Confirm Tom Social Security"
              disabled={busy}
              onClick={() => openActivity("ssa")}
            >
              Tom SSA
            </button>
            <button
              type="button"
              aria-label="Enter a distribution"
              disabled={busy}
              onClick={() => openActivity("distribution")}
            >
              Distribution
            </button>
            <button
              type="button"
              aria-label="Enter a withdrawal"
              disabled={busy}
              onClick={() => openActivity("withdrawal")}
            >
              Withdrawal
            </button>
            <button
              type="button"
              aria-label="Done with this week"
              disabled={busy}
              onClick={() => setActivity(null)}
            >
              None
            </button>
          </div>
        </div>
      ) : null}
      {desk === "weekly" && activity == null ? (
        <div className="buttons">
          <button
            type="button"
            aria-label="Add cash activity"
            disabled={busy}
            onClick={() => setActivity("chooser")}
          >
            Add cash activity
          </button>
        </div>
      ) : null}
      {desk === "weekly" && activity === "distribution" ? (
        <section aria-label="Distribution wizard">
          <ol className="cart-step-rail" aria-label="Distribution steps">
            {DIST_STEPS.map((name) => (
              <li key={name} aria-current={name === distStep ? "step" : undefined}>
                {name}
              </li>
            ))}
          </ol>
          {reminders?.saturdayDraft.open ? (
            <p className="trends-period-caption" aria-label="Saturday income draft">
              Saturday planned draft: Income IRA for{" "}
              {reminders.saturdayDraft.suggestedAccountName || "Income"} on{" "}
              {reminders.saturdayDraft.occurredOn}. Gross stays blank until entered.
            </p>
          ) : null}
          {distStep === "Type" ? (
            <div className="trends-capture-grid">
              <label>
                Type
                <select
                  aria-label="Distribution type"
                  value={activityType}
                  onChange={(e) =>
                    mark(() => {
                      const next = e.target.value;
                      setActivityType(next);
                      if (
                        !accountsForCashType(accounts, next).some(
                          (a) => a.accountId === accountId,
                        )
                      ) {
                        setAccountId("");
                      }
                    })
                  }
                >
                  {DIST_TYPES.map((t) => (
                    <option key={t.value} value={t.value}>
                      {t.label}
                    </option>
                  ))}
                </select>
              </label>
            </div>
          ) : null}
          {distStep === "Account" ? (
            <div className="trends-capture-grid">
              <label>
                Account
                <select
                  aria-label="Distribution account"
                  value={accountId}
                  onChange={(e) => mark(() => setAccountId(e.target.value))}
                >
                  <option value="">Select account</option>
                  {accountsForCashType(accounts, activityType).map((a) => (
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
            </div>
          ) : null}
          {distStep === "Amounts" || distStep === "Review" ? (
            <div className="trends-capture-grid">
              <label>
                Gross
                <input
                  aria-label="Distribution gross"
                  inputMode="decimal"
                  value={gross}
                  disabled={distStep === "Review"}
                  onChange={(e) => mark(() => setGross(e.target.value))}
                />
              </label>
              <label>
                Federal withholding
                <input
                  aria-label="Federal withholding"
                  inputMode="decimal"
                  value={fed}
                  disabled={distStep === "Review"}
                  onChange={(e) => mark(() => setFed(e.target.value))}
                />
              </label>
              <label>
                State withholding
                <input
                  aria-label="State withholding"
                  inputMode="decimal"
                  value={state}
                  disabled={distStep === "Review"}
                  onChange={(e) => mark(() => setState(e.target.value))}
                />
              </label>
            </div>
          ) : null}
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
            {distStep !== "Type" ? (
              <button
                type="button"
                aria-label="Previous distribution step"
                disabled={busy}
                onClick={() => setDistStep(prevOf(DIST_STEPS, distStep))}
              >
                Back
              </button>
            ) : null}
            {distStep !== "Review" ? (
              <button
                type="button"
                aria-label="Next distribution step"
                disabled={busy || !distCanAdvance}
                onClick={() => setDistStep(nextOf(DIST_STEPS, distStep))}
              >
                Next
              </button>
            ) : (
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
                    if (ok) {
                      reset();
                      setActivity("chooser");
                    }
                  });
                }}
              >
                Save
              </button>
            )}
            <button
              type="button"
              aria-label="Cancel cash distribution"
              onClick={backToChooser}
            >
              Cancel
            </button>
          </div>
        </section>
      ) : null}
      {desk === "weekly" && activity === "withdrawal" ? (
        <section aria-label="Withdrawal wizard">
          <ol className="cart-step-rail" aria-label="Withdrawal steps">
            {WITHDRAW_STEPS.map((name) => (
              <li
                key={name}
                aria-current={name === withdrawStep ? "step" : undefined}
              >
                {name}
              </li>
            ))}
          </ol>
          {withdrawStep === "Account" ? (
            <div className="trends-capture-grid">
              <label>
                Account
                <select
                  aria-label="Withdrawal account"
                  value={accountId}
                  onChange={(e) => mark(() => setAccountId(e.target.value))}
                >
                  <option value="">Select account</option>
                  {accountsForCashType(accounts, "Withdrawal").map((a) => (
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
                  aria-label="Withdrawal date"
                  value={occurredOn}
                  onChange={(e) => mark(() => setOccurredOn(e.target.value))}
                />
              </label>
            </div>
          ) : null}
          {withdrawStep === "Amount" || withdrawStep === "Review" ? (
            <div className="trends-capture-grid">
              <label>
                Amount
                <input
                  aria-label="Withdrawal amount"
                  inputMode="decimal"
                  value={gross}
                  disabled={withdrawStep === "Review"}
                  onChange={(e) => mark(() => setGross(e.target.value))}
                />
              </label>
              <label>
                Federal withholding
                <input
                  aria-label="Federal withholding"
                  inputMode="decimal"
                  value={fed}
                  disabled={withdrawStep === "Review"}
                  onChange={(e) => mark(() => setFed(e.target.value))}
                />
              </label>
              <label>
                State withholding
                <input
                  aria-label="State withholding"
                  inputMode="decimal"
                  value={state}
                  disabled={withdrawStep === "Review"}
                  onChange={(e) => mark(() => setState(e.target.value))}
                />
              </label>
            </div>
          ) : null}
          <p>
            Net {netMinor == null ? "—" : formatUsd(netMinor, scale)}
            {identityOk ? "" : " — net cannot be negative"}
          </p>
          <p aria-label="Cash MAGI preview">
            MAGI add{" "}
            {magiAdd == null ? "unknown (year-level)" : formatUsd(magiAdd, scale)}{" "}
            from taxable gross. Withholding {formatUsd(taxPaymentCredit, scale)}{" "}
            changes tax-payment, not Marketplace MAGI.
            {magi ? ` Current MAGI ${magi.decisionState}.` : ""}
          </p>
          <div className="buttons">
            {withdrawStep !== "Account" ? (
              <button
                type="button"
                aria-label="Previous withdrawal step"
                disabled={busy}
                onClick={() => setWithdrawStep(prevOf(WITHDRAW_STEPS, withdrawStep))}
              >
                Back
              </button>
            ) : null}
            {withdrawStep !== "Review" ? (
              <button
                type="button"
                aria-label="Next withdrawal step"
                disabled={busy || !withdrawCanAdvance}
                onClick={() => setWithdrawStep(nextOf(WITHDRAW_STEPS, withdrawStep))}
              >
                Next
              </button>
            ) : (
              <button
                type="button"
                aria-label="Save cash distribution"
                className={dirty ? "is-unsaved" : undefined}
                disabled={busy || !accountId || grossMinor == null || !identityOk}
                onClick={() => {
                  void onSave({
                    accountId,
                    activityType: "Withdrawal",
                    occurredOn,
                    grossMinor,
                    federalWithholdingMinor: fedMinor,
                    stateWithholdingMinor: stateMinor,
                    scale,
                  }).then((ok) => {
                    if (ok) {
                      reset();
                      setActivity("chooser");
                    }
                  });
                }}
              >
                Save
              </button>
            )}
            <button
              type="button"
              aria-label="Cancel cash distribution"
              onClick={backToChooser}
            >
              Cancel
            </button>
          </div>
        </section>
      ) : null}
      {desk === "weekly" && activity === "ssa" ? (
        <section aria-label="Tom SSA wizard">
          <ol className="cart-step-rail" aria-label="Tom SSA steps">
            {SSA_STEPS.map((name) => (
              <li key={name} aria-current={name === ssaStep ? "step" : undefined}>
                {name}
              </li>
            ))}
          </ol>
          <h3>
            {ssaPayee === "barbara"
              ? "Confirm Barbara Social Security retirement"
              : "Confirm Tom Social Security retirement"}
          </h3>
          <p>
            Barbara {formatUsd(133100, scale)} and Tom {formatUsd(286500, scale)}{" "}
            each month, two confirms. A missed payee stays unknown, never $0.
            {reminders?.tomSsa.extraAudit
              ? " An unexpected SSA amount needs a review; June’s two Tom pays are facts."
              : ""}
          </p>
          <p>
            {reminders
              ? reminders.ssaPayees?.length
                ? reminders.ssaPayees
                    .map(
                      (p) =>
                        `${p.payee} ${p.status}${
                          p.postedMinor == null
                            ? ""
                            : ` at ${formatUsd(p.postedMinor, scale)}`
                        }`,
                    )
                    .join("; ")
                : `${reminders.tomSsa.yearMonth} is ${reminders.tomSsa.status}${
                    reminders.tomSsa.postedMinor == null
                      ? ""
                      : ` at ${formatUsd(reminders.tomSsa.postedMinor, scale)}`
                  }.`
              : "Loading Social Security retirement…"}
          </p>
          {ssaStep === "Payee" ? (
            <div className="trends-capture-grid">
              <label>
                Payee
                <select
                  aria-label="SSA payee"
                  value={ssaPayee}
                  onChange={(e) =>
                    markSsa(() =>
                      setSsaPayee(e.target.value === "barbara" ? "barbara" : "tom"),
                    )
                  }
                >
                  <option value="barbara">Barbara — {formatUsd(133100, scale)}</option>
                  <option value="tom">Tom — {formatUsd(286500, scale)}</option>
                </select>
              </label>
            </div>
          ) : null}
          {ssaStep === "Account" ? (
            <div className="trends-capture-grid">
              <label>
                Account
                <select
                  aria-label="Tom SSA account"
                  value={ssaAccountId}
                  onChange={(e) => markSsa(() => setSsaAccountId(e.target.value))}
                >
                  <option value="">Select account</option>
                  {accountsForCashType(accounts, "SSA").map((a) => (
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
            </div>
          ) : null}
          {ssaStep === "Received" || ssaStep === "Review" ? (
            <div className="trends-capture-grid">
              <label>
                Received
                <input
                  aria-label="Tom SSA received"
                  inputMode="decimal"
                  value={ssaReceived}
                  disabled={ssaStep === "Review"}
                  onChange={(e) => markSsa(() => setSsaReceived(e.target.value))}
                />
              </label>
            </div>
          ) : null}
          <p>
            {ssaReceivedMinor == null
              ? "Received stays blank until entered."
              : ssaVariance
                ? `Variance ${formatUsd(ssaReceivedMinor - ssaExpected, scale)} — exception stays open.`
                : "Matches expected."}
          </p>
          <div className="buttons">
            {ssaStep !== "Payee" ? (
              <button
                type="button"
                aria-label="Previous Tom SSA step"
                disabled={busy}
                onClick={() => setSsaStep(prevOf(SSA_STEPS, ssaStep))}
              >
                Back
              </button>
            ) : null}
            {ssaStep !== "Review" ? (
              <button
                type="button"
                aria-label="Next Tom SSA step"
                disabled={busy || !ssaCanAdvance}
                onClick={() => setSsaStep(nextOf(SSA_STEPS, ssaStep))}
              >
                Next
              </button>
            ) : (
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
                    payee: ssaPayee,
                  }).then((ok) => {
                    if (ok) {
                      resetSsa();
                      setActivity("chooser");
                    }
                  });
                }}
              >
                Confirm
              </button>
            )}
            <button
              type="button"
              aria-label="Cancel Tom SSA confirm"
              onClick={backToChooser}
            >
              Cancel
            </button>
          </div>
        </section>
      ) : null}
      {desk === "weekly" ? (
      <CashWeekDesk
        weeks={weekDesk?.weeks}
        points={weekDesk?.points}
        dividendPerf={weekDesk?.dividendPerf}
        overview={weekDesk?.overview}
        openWeek={weekDesk?.openWeek}
        incomePlanWeek={weekDesk?.incomePlanWeek}
      />
      ) : null}
      {desk === "weekly" ? (
      <>
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
      {reminders && reminders.tomSsa.recent.length > 0 ? (
        <div className="table-wrap">
          <table aria-label="Social Security retirement history">
            <thead>
              <tr>
                <th>Paid</th>
                <th>Payee</th>
                <th>Amount</th>
                <th>Account</th>
                <th>Note</th>
              </tr>
            </thead>
            <tbody>
              {reminders.tomSsa.recent.map((row) => (
                <tr key={`${row.occurredOn}-${row.amountMinor}-${row.payee}`}>
                  <td>{row.occurredOn}</td>
                  <td>{row.payee === "barbara" ? "Barbara" : row.payee === "tom" ? "Tom" : row.payee || "—"}</td>
                  <td className="numeric">
                    {formatUsd(row.amountMinor, scale)}
                  </td>
                  <td>{row.accountName}</td>
                  <td>{row.extraAudit ? "unexpected amount" : ""}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      ) : null}
      </>
      ) : null}
      {desk === "weekly" && distributions ? (
        <section aria-label="Cash Management distributions YTD">
          <h3>Distributions (YTD)</h3>
          <div
            className="cm-dist-bar"
            aria-label="Distribution account totals"
          >
            <button
              type="button"
              aria-label="All distribution accounts"
              aria-pressed={distAccount === "all"}
              className={distAccount === "all" ? "is-selected" : undefined}
              onClick={() => setDistAccount("all")}
            >
              <strong>All</strong>
              <span>{formatUsd(distributions.grossMinor, distributions.scale)}</span>
            </button>
            {(distributions.accountTotals ?? []).map((acct) => (
              <button
                type="button"
                key={acct.accountName}
                aria-label={`${acct.accountName} distribution total`}
                aria-pressed={distAccount === acct.accountName}
                className={distAccount === acct.accountName ? "is-selected" : undefined}
                onClick={() =>
                  setDistAccount((prev) =>
                    prev === acct.accountName ? "all" : acct.accountName,
                  )
                }
              >
                <strong>{acct.accountName}</strong>
                <span>{formatUsd(acct.grossMinor, distributions.scale)}</span>
              </button>
            ))}
          </div>
          {(distributions.sections ?? []).length > 0 ? (
            <div className="cm-dist-sections" aria-label="Distribution tax sections">
              {(distributions.sections ?? []).map((section) => (
                <div key={section.id} className="cm-dist-section">
                  <p>
                    <strong>{section.label}</strong>{" "}
                    {formatUsd(section.grossMinor, distributions.scale)}
                  </p>
                </div>
              ))}
            </div>
          ) : null}
          <div className="table-wrap">
            <table>
              <thead>
                <tr>
                  <th>Date</th>
                  <th>Type</th>
                  <th>Account</th>
                  <th>Gross</th>
                  <th>Fed WH</th>
                  <th>State WH</th>
                  <th>Net</th>
                </tr>
              </thead>
              <tbody>
                {distributions.lines
                  .filter(
                    (line) =>
                      distAccount === "all" || line.accountName === distAccount,
                  )
                  .slice(-40)
                  .map((line, i) => (
                  <tr key={`${line.occurredOn}-${line.activityType}-${i}`}>
                    <td>{line.occurredOn}</td>
                    <td>{line.activityType}</td>
                    <td>{line.accountName}</td>
                    <td className="numeric">{formatUsd(line.amountMinor, line.scale)}</td>
                    <td className="numeric">
                      {formatUsd(line.federalWithholdingMinor ?? 0, line.scale)}
                    </td>
                    <td className="numeric">
                      {formatUsd(line.stateWithholdingMinor ?? 0, line.scale)}
                    </td>
                    <td className="numeric">
                      {formatUsd(
                        line.netMinor ??
                          line.amountMinor -
                            (line.federalWithholdingMinor ?? 0) -
                            (line.stateWithholdingMinor ?? 0),
                        line.scale,
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </section>
      ) : null}
      {desk === "weekly" && taxMonitor ? (
        <section aria-label="Cash Management tax and ACA monitor">
          <h3>Tax / ACA monitor</h3>
          <p>
            Federal withholding{" "}
            {formatUsd(taxMonitor.federalWithholdingMinor, taxMonitor.scale)}. YTD
            included {formatUsd(taxMonitor.projectedLiabilityMinor ?? 0, taxMonitor.scale)}
            {taxMonitor.acaThresholdMinor != null
              ? `; ACA ${taxMonitor.acaCoverageYear} threshold ${formatUsd(taxMonitor.acaThresholdMinor, taxMonitor.scale)}`
              : ""}
            {taxMonitor.warning ? " — warning: above threshold" : ""}
          </p>
        </section>
      ) : null}
    </div>
  );
}
