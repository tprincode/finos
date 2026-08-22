/** Design tokens and Radix-based primitives used by the local desktop household screens. */
export const DESIGN_SYSTEM_VERSION = "0.1.0-draft";

const USD_SCALE = 2;

function groupInt(digits: string): string {
  return digits.replace(/\B(?=(\d{3})+(?!\d))/g, ",");
}

/** Counts, lot numbers, versions — grouping only, no currency. */
export function formatCount(n: number): string {
  const sign = n < 0 ? "-" : "";
  return sign + groupInt(Math.abs(Math.trunc(n)).toString());
}

/** Fixed-scale quantity or other non-money amount (ADR-0004), with grouping. */
export function formatScaled(minor: number, scale = 0): string {
  const places = Number.isFinite(scale) ? Math.max(0, Math.trunc(scale)) : 0;
  const sign = minor < 0 ? "-" : "";
  const digits = Math.abs(Math.trunc(minor))
    .toString()
    .padStart(places + 1, "0");
  if (places <= 0) {
    return sign + groupInt(digits);
  }
  const i = digits.length - places;
  return `${sign}${groupInt(digits.slice(0, i))}.${digits.slice(i)}`;
}

function toUsdCents(minor: number, scale: number): number {
  const places = Number.isFinite(scale) ? Math.max(0, Math.trunc(scale)) : USD_SCALE;
  const n = Math.trunc(minor);
  if (places === USD_SCALE) {
    return n;
  }
  if (places > USD_SCALE) {
    const factor = 10 ** (places - USD_SCALE);
    const abs = Math.abs(n);
    const rounded = Math.trunc((abs + Math.floor(factor / 2)) / factor);
    return n < 0 ? -rounded : rounded;
  }
  return n * 10 ** (USD_SCALE - places);
}

/** Typical USD: $3,715.87. Stored scale may be 4 from unit costs; display is always cents. */
export function formatUsd(minor: number, scale = USD_SCALE): string {
  const cents = toUsdCents(minor, scale);
  const sign = cents < 0 ? "-" : "";
  return `${sign}$${formatScaled(Math.abs(cents), USD_SCALE)}`;
}

export function formatPercentScaled(minor: number, scale = USD_SCALE): string {
  const places = Number.isFinite(scale) ? scale : USD_SCALE;
  return `${formatScaled(minor, places)}%`;
}

export type PositionLineView = {
  accountName: string;
  symbol: string;
  remainingQuantityMinor: number;
  remainingPerformanceMinor: number;
  remainingTaxMinor: number;
  lotCount: number;
  quantityScale?: number;
  scale?: number;
};

export type PositionDetailsView = {
  positions: PositionLineView[];
  openPerformanceMinor: number;
  openTaxMinor: number;
  scale?: number;
};

export type TaxProjectionView = {
  sourceQuery: string;
  decisionState: string;
  actualIncludedYtd: { amountMinor: number; scale?: number };
  applicableThreshold: { amountMinor: number; scale?: number };
  dataCompleteness: string;
};

export type AccountView = {
  accountId?: string;
  name: string;
  kind: string;
};

export type ExceptionView = {
  code: string;
  message: string;
};

export type LotView = {
  lotId?: string;
  accountId?: string;
  openedOn?: string;
  origin?: string;
  remainingQuantityMinor: number;
  remainingPerformanceMinor: number;
  remainingTaxMinor: number;
  crfZeroCost: boolean;
  quantityScale?: number;
  scale?: number;
};

export type BasisView = {
  lots: LotView[];
  openPerformanceMinor: number;
  openTaxMinor: number;
  scale?: number;
};

export type RoiView = {
  performanceGainMinor: number;
  taxGainMinor: number;
  scale?: number;
};

export type DividendActualView = {
  occurredOn: string;
  amountMinor: number;
  scale?: number;
};

export type DividendView = {
  actuals: DividendActualView[];
  actualTotalMinor: number;
  scale?: number;
};

export type ActivityView = {
  activityId: string;
  activityType: string;
  amountMinor: number;
  occurredOn: string;
  scale?: number;
};

export type MagiView = {
  decisionState: string;
  actualIncludedYtd: { amountMinor: number; scale?: number };
  protectedHeadroom: { amountMinor: number; scale?: number };
  dataCompleteness: string;
};

export type PlanView = {
  remainingMinor: number;
  version: number;
  scale?: number;
};

export type BurndownView = {
  cashMinor: number;
  obligationMinor: number;
  sufficient: boolean;
  scale?: number;
};

export type AllocationView = {
  targets: Array<{ name: string; targetMinor: number; scale?: number }>;
  openPerformanceMinor: number;
  openTaxMinor: number;
  scale?: number;
};

export function PositionDetailsTable({
  positions,
  filter = "",
}: {
  positions: PositionDetailsView | null;
  filter?: string;
}) {
  if (!positions) {
    return <p>Loading position details…</p>;
  }
  if (positions.positions.length === 0) {
    return (
      <p>
        No open positions yet. Complete New Investment including the first lot.
      </p>
    );
  }
  const needle = filter.trim().toLowerCase();
  const lines = needle
    ? positions.positions.filter(
        (line) =>
          line.accountName.toLowerCase().includes(needle) ||
          line.symbol.toLowerCase().includes(needle),
      )
    : positions.positions;
  return (
    <div>
      <p>
        Showing {formatCount(lines.length)} of {formatCount(positions.positions.length)} open
        position lines.
      </p>
      <div className="table-wrap">
      <table>
        <thead>
          <tr>
            <th scope="col">Account</th>
            <th scope="col">Symbol</th>
            <th className="numeric" scope="col">Qty</th>
            <th className="numeric" scope="col">Lots</th>
            <th className="numeric" scope="col">Perf basis</th>
            <th className="numeric" scope="col">Tax basis</th>
          </tr>
        </thead>
        <tbody>
          {lines.map((line) => (
            <tr key={`${line.accountName}-${line.symbol}`}>
              <td>{line.accountName}</td>
              <td>{line.symbol}</td>
              <td className="numeric">
                {formatScaled(line.remainingQuantityMinor, line.quantityScale ?? 0)}
              </td>
              <td className="numeric">{formatCount(line.lotCount)}</td>
              <td className="numeric">
                {formatUsd(line.remainingPerformanceMinor, line.scale ?? positions.scale)}
              </td>
              <td className="numeric">
                {formatUsd(line.remainingTaxMinor, line.scale ?? positions.scale)}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      </div>
      <dl className="health">
        <dt>open performance</dt>
        <dd>{formatUsd(positions.openPerformanceMinor, positions.scale)}</dd>
        <dt>open tax</dt>
        <dd>{formatUsd(positions.openTaxMinor, positions.scale)}</dd>
      </dl>
    </div>
  );
}

export function TaxProjectionCard({ tax }: { tax: TaxProjectionView | null }) {
  if (!tax) {
    return <p>Loading tax projection…</p>;
  }
  return (
    <dl className="health">
      <dt>decision</dt>
      <dd>{tax.decisionState}</dd>
      <dt>source</dt>
      <dd>{tax.sourceQuery}</dd>
      <dt>actual YTD</dt>
      <dd>{formatUsd(tax.actualIncludedYtd.amountMinor, tax.actualIncludedYtd.scale)}</dd>
      <dt>threshold</dt>
      <dd>{formatUsd(tax.applicableThreshold.amountMinor, tax.applicableThreshold.scale)}</dd>
      <dt>completeness</dt>
      <dd>{tax.dataCompleteness}</dd>
    </dl>
  );
}

export function AccountList({ accounts }: { accounts: AccountView[] }) {
  if (accounts.length === 0) {
    return <p>None yet</p>;
  }
  return (
    <ul>
      {accounts.map((a) => (
        <li key={a.accountId ?? a.name}>
          {a.name} ({a.kind})
        </li>
      ))}
    </ul>
  );
}

export function ExceptionList({ exceptions }: { exceptions: ExceptionView[] }) {
  if (exceptions.length === 0) {
    return <p>None</p>;
  }
  return (
    <ul>
      {exceptions.map((e) => (
        <li key={`${e.code}:${e.message}`}>
          {e.code}: {e.message}
        </li>
      ))}
    </ul>
  );
}

export function LotsRoiPanel({
  basis,
  roi,
  filter = "",
}: {
  basis: BasisView | null;
  roi: RoiView | null;
  filter?: string;
}) {
  if (!basis) {
    return <p>Loading lots…</p>;
  }
  const openLots = basis.lots.filter((lot) => lot.remainingQuantityMinor > 0);
  const needle = filter.trim().toLowerCase();
  const shown = needle
    ? openLots.filter((lot) =>
        [lot.lotId, lot.accountId, lot.origin, lot.openedOn]
          .filter(Boolean)
          .join(" ")
          .toLowerCase()
          .includes(needle),
      )
    : openLots;
  return (
    <div>
      <dl className="health">
        <dt>open lots</dt>
        <dd>{formatCount(openLots.length)}</dd>
        <dt>shown</dt>
        <dd>{formatCount(shown.length)}</dd>
        <dt>performance</dt>
        <dd>{formatUsd(basis.openPerformanceMinor, basis.scale)}</dd>
        <dt>tax</dt>
        <dd>{formatUsd(basis.openTaxMinor, basis.scale)}</dd>
        <dt>perf gain</dt>
        <dd>{roi ? formatUsd(roi.performanceGainMinor, roi.scale ?? basis.scale) : "—"}</dd>
        <dt>tax gain</dt>
        <dd>{roi ? formatUsd(roi.taxGainMinor, roi.scale ?? basis.scale) : "—"}</dd>
      </dl>
      {openLots.length === 0 ? (
        <p>No open lots.</p>
      ) : (
        <div className="table-wrap">
        <table>
          <thead>
            <tr>
              <th scope="col">Lot</th>
              <th scope="col">Opened</th>
              <th scope="col">Origin</th>
              <th className="numeric" scope="col">Qty remaining</th>
              <th className="numeric" scope="col">Perf basis</th>
              <th className="numeric" scope="col">Tax basis</th>
              <th scope="col">CRF zero-cost</th>
            </tr>
          </thead>
          <tbody>
            {shown.map((lot, i) => (
              <tr key={lot.lotId ?? `${lot.remainingQuantityMinor}-${i}`}>
                <td>{lot.lotId ?? "—"}</td>
                <td>{lot.openedOn ?? "—"}</td>
                <td>{lot.origin ?? "—"}</td>
                <td className="numeric">
                  {formatScaled(lot.remainingQuantityMinor, lot.quantityScale ?? 0)}
                </td>
                <td className="numeric">
                  {formatUsd(lot.remainingPerformanceMinor, lot.scale ?? basis.scale)}
                </td>
                <td className="numeric">
                  {formatUsd(lot.remainingTaxMinor, lot.scale ?? basis.scale)}
                </td>
                <td>{lot.crfZeroCost ? "yes" : "no"}</td>
              </tr>
            ))}
          </tbody>
        </table>
        </div>
      )}
    </div>
  );
}

export function YieldPanel({
  dividend,
  activities,
  filter = "",
}: {
  dividend: DividendView | null;
  activities: ActivityView[];
  filter?: string;
}) {
  const yieldActs = activities.filter(
    (a) => a.activityType.toLowerCase() === "dividend",
  );
  const needle = filter.trim().toLowerCase();
  const shown = needle
    ? yieldActs.filter((row) => {
        const usd = formatUsd(row.amountMinor, row.scale ?? dividend?.scale);
        return (
          row.occurredOn.toLowerCase().includes(needle) ||
          String(row.amountMinor).includes(needle) ||
          usd.toLowerCase().includes(needle)
        );
      })
    : yieldActs;
  return (
    <div>
      <dl className="health">
        <dt>DividendGet</dt>
        <dd>
          {dividend ? formatUsd(dividend.actualTotalMinor, dividend.scale) : "loading…"}
        </dd>
        <dt>posted rows</dt>
        <dd>{formatCount(yieldActs.length)}</dd>
        <dt>shown</dt>
        <dd>{formatCount(shown.length)}</dd>
      </dl>
      {yieldActs.length === 0 ? (
        <p>No yield activity yet.</p>
      ) : (
        <div className="table-wrap">
        <table>
          <thead>
            <tr>
              <th scope="col">Date</th>
              <th className="numeric" scope="col">Amount</th>
              <th scope="col">Activity</th>
            </tr>
          </thead>
          <tbody>
            {shown.map((row) => (
              <tr key={row.activityId}>
                <td>{row.occurredOn}</td>
                <td className="numeric">
                  {formatUsd(row.amountMinor, row.scale ?? dividend?.scale)}
                </td>
                <td>{row.activityId}</td>
              </tr>
            ))}
          </tbody>
        </table>
        </div>
      )}
    </div>
  );
}

const DISBURSEMENT_TYPES = new Set([
  "IRA_Distribution",
  "Withdrawal",
  "Form_1099",
  "SSA",
  "Roth_Distribution",
]);

export function DisbursementPanel({
  activities,
  filter = "",
}: {
  activities: ActivityView[];
  filter?: string;
}) {
  const rows = activities.filter((a) => DISBURSEMENT_TYPES.has(a.activityType));
  const needle = filter.trim().toLowerCase();
  const shown = needle
    ? rows.filter((row) => {
        const usd = formatUsd(row.amountMinor, row.scale);
        return (
          row.activityType.toLowerCase().includes(needle) ||
          row.occurredOn.toLowerCase().includes(needle) ||
          String(row.amountMinor).includes(needle) ||
          usd.toLowerCase().includes(needle)
        );
      })
    : rows;
  const gross = rows.reduce((sum, row) => sum + row.amountMinor, 0);
  const grossScale = rows[0]?.scale ?? USD_SCALE;
  return (
    <div>
      <dl className="health">
        <dt>posted</dt>
        <dd>{formatCount(rows.length)}</dd>
        <dt>shown</dt>
        <dd>{formatCount(shown.length)}</dd>
        <dt>gross</dt>
        <dd>{formatUsd(gross, grossScale)}</dd>
      </dl>
      {rows.length === 0 ? (
        <p>No disbursements yet.</p>
      ) : (
        <div className="table-wrap">
        <table>
          <thead>
            <tr>
              <th scope="col">Date</th>
              <th scope="col">Type</th>
              <th className="numeric" scope="col">Amount</th>
            </tr>
          </thead>
          <tbody>
            {shown.map((row) => (
              <tr key={row.activityId}>
                <td>{row.occurredOn}</td>
                <td>{row.activityType}</td>
                <td className="numeric">{formatUsd(row.amountMinor, row.scale)}</td>
              </tr>
            ))}
          </tbody>
        </table>
        </div>
      )}
    </div>
  );
}

export function MagiCard({ magi }: { magi: MagiView | null }) {
  if (!magi) {
    return <p>MAGI not set. Load household facts through FinanceClient.</p>;
  }
  return (
    <dl className="health">
      <dt>decision</dt>
      <dd>{magi.decisionState}</dd>
      <dt>actual included</dt>
      <dd>{formatUsd(magi.actualIncludedYtd.amountMinor, magi.actualIncludedYtd.scale)}</dd>
      <dt>protected headroom</dt>
      <dd>{formatUsd(magi.protectedHeadroom.amountMinor, magi.protectedHeadroom.scale)}</dd>
      <dt>completeness</dt>
      <dd>{magi.dataCompleteness}</dd>
    </dl>
  );
}

export function PlanBurndownCard({
  plan,
  burndown,
}: {
  plan: PlanView | null;
  burndown: BurndownView | null;
}) {
  return (
    <dl className="health">
      <dt>PlanGet remaining</dt>
      <dd>{plan ? formatUsd(plan.remainingMinor, plan.scale) : "—"}</dd>
      <dt>version</dt>
      <dd>{plan ? formatCount(plan.version) : "—"}</dd>
      <dt>cash</dt>
      <dd>{burndown ? formatUsd(burndown.cashMinor, burndown.scale) : "—"}</dd>
      <dt>obligation</dt>
      <dd>{burndown ? formatUsd(burndown.obligationMinor, burndown.scale) : "—"}</dd>
      <dt>sufficient</dt>
      <dd>{burndown ? (burndown.sufficient ? "yes" : "no") : "—"}</dd>
    </dl>
  );
}

export function AllocationVsPositions({
  allocation,
  positions,
}: {
  allocation: AllocationView | null;
  positions: PositionDetailsView | null;
}) {
  if (!allocation || !positions) {
    return <p>Loading allocation versus positions…</p>;
  }
  const openPerf = allocation.openPerformanceMinor;
  return (
    <div>
      <p>
        Targets are decision support. Open amounts are lot cost basis, not market
        value. They do not post cash or MAGI facts.
      </p>
      <dl className="health">
        <dt>targets</dt>
        <dd>{formatCount(allocation.targets.length)}</dd>
        <dt>open performance (basis)</dt>
        <dd>{formatUsd(allocation.openPerformanceMinor, allocation.scale)}</dd>
        <dt>open tax (basis)</dt>
        <dd>{formatUsd(allocation.openTaxMinor, allocation.scale)}</dd>
      </dl>
      {allocation.targets.length > 0 ? (
        <ul>
          {allocation.targets.map((t) => (
            <li key={t.name}>
              {t.name}: {formatPercentScaled(t.targetMinor, t.scale ?? allocation.scale)}
            </li>
          ))}
        </ul>
      ) : (
        <p>No targets yet. Set a household target below.</p>
      )}
      {positions.positions.length === 0 ? (
        <p>No open positions to compare.</p>
      ) : (
        <table>
          <thead>
            <tr>
              <th scope="col">Account</th>
              <th scope="col">Symbol</th>
              <th className="numeric" scope="col">Perf basis</th>
              <th className="numeric" scope="col">Share of open</th>
            </tr>
          </thead>
          <tbody>
            {positions.positions.map((line) => {
              const share =
                openPerf === 0
                  ? 0
                  : (line.remainingPerformanceMinor * 10000) / openPerf;
              return (
                <tr key={`${line.accountName}-${line.symbol}`}>
                  <td>{line.accountName}</td>
                  <td>{line.symbol}</td>
                  <td className="numeric">
                    {formatUsd(line.remainingPerformanceMinor, line.scale ?? positions.scale)}
                  </td>
                  <td className="numeric">
                    {formatPercentScaled(Math.round(share), 2)}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      )}
    </div>
  );
}

export type IncomePlanWeekView = {
  asOfDate: string;
  start: string;
  end: string;
  status: string;
  lines: Array<{
    accountName: string;
    actualMinor: number;
    plannedMinor?: number;
    planKnown: boolean;
    scale: number;
  }>;
  drilldown: Array<{
    accountName: string;
    symbol: string;
    occurredOn: string;
    amountMinor: number;
    scale: number;
  }>;
  latestActualOn?: string | null;
  yieldCount?: number;
  scale?: number;
};

export function IncomePlanWeekPanel({
  week,
  selectedAccount,
}: {
  week: IncomePlanWeekView | null;
  selectedAccount?: string | null;
}) {
  if (!week) {
    return <p>Loading week…</p>;
  }
  const totalActual = week.lines.reduce((sum, line) => sum + line.actualMinor, 0);
  const drill = selectedAccount
    ? week.drilldown.filter((row) => row.accountName === selectedAccount)
    : week.drilldown;
  return (
    <div>
      <p>
        Saturday {week.start} through Friday {week.end}. Status {week.status}. Plan is
        unknown until Calculator exists — not shown as $0.00.
        {week.yieldCount != null ? ` Household yield rows: ${formatCount(week.yieldCount)}.` : ""}
        {week.latestActualOn ? ` Last yield ${week.latestActualOn}.` : ""}
      </p>
      {totalActual === 0 ? (
        <p>
          No dividend cash in this week. That is not unpaid and not an empty database — use
          previous week if the last yield is earlier.
        </p>
      ) : null}
      <div className="table-wrap">
        <table>
          <thead>
            <tr>
              <th scope="col">Account</th>
              <th className="numeric" scope="col">Plan</th>
              <th className="numeric" scope="col">Actual</th>
              <th className="numeric" scope="col">Variance</th>
            </tr>
          </thead>
          <tbody>
            {week.lines.map((line) => (
              <tr key={line.accountName}>
                <td>{line.accountName}</td>
                <td className="numeric">
                  {line.planKnown
                    ? formatUsd(line.plannedMinor ?? 0, line.scale)
                    : "N/A"}
                </td>
                <td className="numeric">{formatUsd(line.actualMinor, line.scale)}</td>
                <td className="numeric">
                  {line.planKnown
                    ? formatUsd(line.actualMinor - (line.plannedMinor ?? 0), line.scale)
                    : "N/A"}
                </td>
              </tr>
            ))}
            <tr>
              <td>Total</td>
              <td className="numeric">N/A</td>
              <td className="numeric">{formatUsd(totalActual, week.scale)}</td>
              <td className="numeric">N/A</td>
            </tr>
          </tbody>
        </table>
      </div>
      <h3>Symbol drilldown{selectedAccount ? ` — ${selectedAccount}` : ""}</h3>
      {drill.length === 0 ? (
        <p>No dividend actuals in this week for the selected scope.</p>
      ) : (
        <div className="table-wrap">
          <table>
            <thead>
              <tr>
                <th scope="col">Account</th>
                <th scope="col">Symbol</th>
                <th scope="col">Date</th>
                <th className="numeric" scope="col">Amount</th>
              </tr>
            </thead>
            <tbody>
              {drill.map((row, i) => (
                <tr key={`${row.accountName}-${row.symbol}-${row.occurredOn}-${i}`}>
                  <td>{row.accountName}</td>
                  <td>{row.symbol}</td>
                  <td>{row.occurredOn}</td>
                  <td className="numeric">{formatUsd(row.amountMinor, row.scale)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}

export type DashboardBurndownView = {
  start: string;
  end: string;
  status: string;
  note: string;
  lines: Array<{
    accountName: string;
    inflowMinor: number;
    outflowMinor: number;
    floorKnown: boolean;
    scale: number;
  }>;
  scale?: number;
};

export function DashboardBurndownPanel({
  burndown,
}: {
  burndown: DashboardBurndownView | null;
}) {
  if (!burndown) {
    return <p>Loading burndown…</p>;
  }
  return (
    <div>
      <p>
        Read-only. Week {burndown.start} to {burndown.end}. {burndown.note}
      </p>
      <div className="table-wrap">
        <table>
          <thead>
            <tr>
              <th scope="col">Account</th>
              <th className="numeric" scope="col">Dividend inflow</th>
              <th className="numeric" scope="col">Disbursement outflow</th>
              <th className="numeric" scope="col">Net</th>
              <th className="numeric" scope="col">Floor</th>
            </tr>
          </thead>
          <tbody>
            {burndown.lines.map((line) => (
              <tr key={line.accountName}>
                <td>{line.accountName}</td>
                <td className="numeric">{formatUsd(line.inflowMinor, line.scale)}</td>
                <td className="numeric">{formatUsd(line.outflowMinor, line.scale)}</td>
                <td className="numeric">
                  {formatUsd(line.inflowMinor - line.outflowMinor, line.scale)}
                </td>
                <td className="numeric">{line.floorKnown ? formatUsd(0, line.scale) : "N/A"}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

export type HoldingsLotView = {
  lotId: string;
  accountName: string;
  symbol: string;
  openedOn: string;
  remainingQuantityMinor: number;
  quantityScale: number;
  remainingPerformanceMinor: number;
  remainingTaxMinor: number;
  scale: number;
};

function unitCostMinor(lot: HoldingsLotView): number | null {
  if (lot.remainingQuantityMinor <= 0) {
    return null;
  }
  const factor = 10 ** lot.quantityScale;
  return Math.trunc((lot.remainingPerformanceMinor * factor) / lot.remainingQuantityMinor);
}

function unitTaxMinor(lot: HoldingsLotView): number | null {
  if (lot.remainingQuantityMinor <= 0) {
    return null;
  }
  const factor = 10 ** lot.quantityScale;
  return Math.trunc((lot.remainingTaxMinor * factor) / lot.remainingQuantityMinor);
}

export function HoldingsPanel({
  lots,
  filter = "",
}: {
  lots: HoldingsLotView[] | null;
  filter?: string;
}) {
  if (!lots) {
    return <p>Loading holdings…</p>;
  }
  const needle = filter.trim().toLowerCase();
  const shown = needle
    ? lots.filter(
        (lot) =>
          lot.accountName.toLowerCase().includes(needle) ||
          lot.symbol.toLowerCase().includes(needle),
      )
    : lots;
  if (lots.length === 0) {
    return <p>No open lots.</p>;
  }
  return (
    <div>
      <p>
        Showing {formatCount(shown.length)} of {formatCount(lots.length)} open lots. Dual
        cost stays separate. Market value is parked (Last Price).
      </p>
      <div className="table-wrap">
        <table>
          <thead>
            <tr>
              <th scope="col">Account</th>
              <th scope="col">Symbol</th>
              <th scope="col">Opened</th>
              <th className="numeric" scope="col">Qty</th>
              <th className="numeric" scope="col">Unit orig</th>
              <th className="numeric" scope="col">Unit tax</th>
              <th className="numeric" scope="col">Perf basis</th>
              <th className="numeric" scope="col">Tax basis</th>
            </tr>
          </thead>
          <tbody>
            {shown.map((lot) => {
              const orig = unitCostMinor(lot);
              const tax = unitTaxMinor(lot);
              return (
                <tr key={lot.lotId}>
                  <td>{lot.accountName}</td>
                  <td>{lot.symbol}</td>
                  <td>{lot.openedOn}</td>
                  <td className="numeric">
                    {formatScaled(lot.remainingQuantityMinor, lot.quantityScale)}
                  </td>
                  <td className="numeric">
                    {orig == null ? "N/A" : formatUsd(orig, lot.scale)}
                  </td>
                  <td className="numeric">
                    {tax == null ? "N/A" : formatUsd(tax, lot.scale)}
                  </td>
                  <td className="numeric">
                    {formatUsd(lot.remainingPerformanceMinor, lot.scale)}
                  </td>
                  <td className="numeric">{formatUsd(lot.remainingTaxMinor, lot.scale)}</td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </div>
  );
}

export type CalculatorRowView = {
  symbol: string;
  paymentFrequency: string;
  planKnown: boolean;
  planPerShareMinor: number;
  planScale: number;
  planningPeriodsPerYear: number;
  remainingQuantityMinor: number;
  quantityScale: number;
  planPaymentMinor: number;
  remainingPerformanceMinor: number;
  rocPct2025ActualMinor: number | null;
  rocScale: number | null;
  scale: number;
};

export function CalculatorPanel({ rows }: { rows: CalculatorRowView[] | null }) {
  if (!rows) {
    return <p>Loading Calculator…</p>;
  }
  if (rows.length === 0) {
    return <p>No Calculator positions yet. Complete New Investment including the first lot.</p>;
  }
  return (
    <div>
      <p>
        Plan is owner-controlled per share (Calculator AE extract). Declarations and broker
        cash never change it. Most Current and Avg 6 are on Position Details after
        declarations are stored. Missing Plan is N/A, not $0.00.
      </p>
      <div className="table-wrap">
        <table>
          <thead>
            <tr>
              <th scope="col">Symbol</th>
              <th scope="col">Frequency</th>
              <th className="numeric" scope="col">Plan / share</th>
              <th className="numeric" scope="col">Periods</th>
              <th className="numeric" scope="col">Qty</th>
              <th className="numeric" scope="col">Plan payment</th>
              <th className="numeric" scope="col">Cost</th>
              <th className="numeric" scope="col">ROC 2025</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((row) => (
              <tr key={row.symbol}>
                <td>{row.symbol}</td>
                <td>{row.paymentFrequency || "—"}</td>
                <td className="numeric">
                  {row.planKnown
                    ? `$${formatScaled(row.planPerShareMinor, row.planScale)}`
                    : "N/A"}
                </td>
                <td className="numeric">
                  {row.planningPeriodsPerYear > 0
                    ? formatCount(row.planningPeriodsPerYear)
                    : "N/A"}
                </td>
                <td className="numeric">
                  {formatScaled(row.remainingQuantityMinor, row.quantityScale)}
                </td>
                <td className="numeric">
                  {row.planKnown ? formatUsd(row.planPaymentMinor, row.scale) : "N/A"}
                </td>
                <td className="numeric">
                  {formatUsd(row.remainingPerformanceMinor, row.scale)}
                </td>
                <td className="numeric">
                  {row.rocPct2025ActualMinor == null || row.rocScale == null
                    ? "N/A"
                    : formatPercentScaled(row.rocPct2025ActualMinor, row.rocScale)}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

