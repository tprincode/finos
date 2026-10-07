import { formatCount, formatUsd } from "@finos/ui-components";
import type {
  AccountValueHomeGet,
  DataSummaryGet,
  DividendGet,
  DividendPlanHomeGet,
  TrendsGet,
} from "@finos/app-contracts";
import { HomePaintMark } from "../shell/HomePaintMark";
import { HomeAccountCharts } from "../graphing/HomeAccountCharts";
import { AccountCashFlow } from "../cash/AccountCashFlow";
import { HomeDividendPlan } from "./HomeDividendPlan";
import {
  INCOME_TX_PERIOD_OPTIONS,
  incomeTxRange,
  type IncomeTxPeriod,
} from "../../incomeTxPeriod";

type RefreshProgress = {
  current: number;
  total: number;
  symbol: string;
};

export type HomeCashRow = {
  name: string;
  symbol: string;
  cents: number;
  found: boolean;
};

export type HomeIncomeTxRow = {
  id: string;
  date: string;
  account: string;
  symbol: string;
  amountMinor: number;
  scale: number;
};

export type HomeScreenProps = {
  summary: DataSummaryGet | null;
  accountCashRows: HomeCashRow[];
  cashTotalCents: number;
  dividendActual: DividendGet | null; // Home host passes DividendGet (paid cash)
  incomeThroughOn: string;
  incomeWeekTotal: number | null;
  incomeWeekDeclared: number | null;
  declarationBusy: boolean;
  lastPriceBusy: boolean;
  busy: boolean;
  declarationProgress: RefreshProgress | null;
  lastPriceProgress: RefreshProgress | null;
  refreshDeclarations: (force: boolean) => void | Promise<void>;
  refreshLastPrices: (force: boolean) => void | Promise<void>;
  onWorkTickets: () => void;
  dividendPlan: DividendPlanHomeGet | null;
  accountValues: AccountValueHomeGet | null;
  trends: TrendsGet | null;
  asOfDate: string;
  incomeTxOpen: boolean;
  setIncomeTxOpen: (open: boolean) => void;
  incomeTxPeriod: IncomeTxPeriod;
  setIncomeTxPeriod: (p: IncomeTxPeriod) => void;
  incomeTxCustomStart: string;
  setIncomeTxCustomStart: (v: string) => void;
  incomeTxCustomEnd: string;
  setIncomeTxCustomEnd: (v: string) => void;
  incomeTxAccountId: string;
  setIncomeTxAccountId: (id: string) => void;
  incomeTxToday: string;
  incomeTxWindow: { startOn: string; endOn: string } | null;
  incomeTxAccountOptions: Array<{ accountId: string; name: string }>;
  incomeThroughRows: HomeIncomeTxRow[];
  declarationRefreshLabel: (p: RefreshProgress | null) => string;
  lastPriceRefreshLabel: (p: RefreshProgress | null) => string;
};

export function HomeScreen({
  summary,
  accountCashRows,
  cashTotalCents,
  dividendActual,
  incomeThroughOn,
  incomeWeekTotal,
  incomeWeekDeclared,
  declarationBusy,
  lastPriceBusy,
  busy,
  declarationProgress,
  lastPriceProgress,
  refreshDeclarations,
  refreshLastPrices,
  onWorkTickets,
  dividendPlan,
  accountValues,
  trends,
  asOfDate,
  incomeTxOpen,
  setIncomeTxOpen,
  incomeTxPeriod,
  setIncomeTxPeriod,
  incomeTxCustomStart,
  setIncomeTxCustomStart,
  incomeTxCustomEnd,
  setIncomeTxCustomEnd,
  incomeTxAccountId,
  setIncomeTxAccountId,
  incomeTxToday,
  incomeTxWindow,
  incomeTxAccountOptions,
  incomeThroughRows,
  declarationRefreshLabel,
  lastPriceRefreshLabel,
}: HomeScreenProps) {
  return (
    <>
        <HomePaintMark />
        <div className="home-top-row">
        {summary ? (
        <section className="home-portfolio-pane" aria-label="Portfolio summary">
          <dl className="portfolio-summary">
            <div className="ps-head-row">
            <div className="ps-cell ps-mv">
              <dt>Market value</dt>
              <dd>
                {summary.marketValueMinor == null
                  ? "unknown"
                  : `${formatUsd(summary.marketValueMinor, summary.scale ?? 2)}${
                      summary.marketValueComplete ? "" : " (incomplete)"
                    }`}
              </dd>
            </div>
            <div className="ps-cell ps-cost">
              <dt>Original cost</dt>
              <dd>{formatUsd(summary.openPerformanceMinor ?? 0, summary.scale ?? 2)}</dd>
            </div>
            <div className="ps-cell ps-tax">
              <dt>Tax basis</dt>
              <dd>{formatUsd(summary.openTaxMinor ?? 0, summary.scale ?? 2)}</dd>
            </div>
            <div
              className={`ps-cell ps-unrealized${
                summary.marketValueMinor == null
                  ? ""
                  : summary.marketValueMinor - (summary.openPerformanceMinor ?? 0) >= 0
                    ? " ps-gain"
                    : " ps-loss"
              }`}
            >
              <dt>Unrealized</dt>
              <dd>
                {summary.marketValueMinor == null
                  ? "unknown"
                  : formatUsd(
                      summary.marketValueMinor - (summary.openPerformanceMinor ?? 0),
                      summary.scale ?? 2,
                    )}
              </dd>
            </div>
            </div>
            <div className="ps-cell ps-cash" aria-label="Account cash balances">
              <dt>
                Cash
                <span className="ps-cash-total">{formatUsd(cashTotalCents, 2)}</span>
              </dt>
              <dd>
                <ul>
                  {accountCashRows.map((row) => (
                    <li key={row.name}>
                      <span className="ps-cash-name">{row.name}</span>
                      <span className="ps-cash-amt">
                        {row.found ? formatUsd(row.cents, 2) : "none"}
                      </span>
                    </li>
                  ))}
                </ul>
              </dd>
              <p className="ps-note">Stored cash lot quantity</p>
            </div>
            <div className="ps-cell ps-income">
              <dt>Income earned</dt>
              <dd>
                {formatUsd(
                  dividendActual?.actualTotalMinor ??
                    summary.incomeEarnedMinor ??
                    0,
                  dividendActual?.scale ?? summary.scale ?? 2,
                )}
              </dd>
              <p className="ps-note">Lifetime paid dividends (cash)</p>
            </div>
            <div className="ps-cell ps-date">
              <dt>Income through</dt>
              <dd>
                {incomeThroughOn ? (
                  <button
                    type="button"
                    className="ps-date-link"
                    aria-label="Income through transactions"
                    onClick={() => setIncomeTxOpen(true)}
                  >
                    {incomeThroughOn}
                  </button>
                ) : (
                  "none"
                )}
              </dd>
              {incomeWeekTotal == null ? null : (
                <p className="ps-note">
                  {incomeWeekDeclared == null ? "This week imported" : "This week declared"}{" "}
                  {formatUsd(incomeWeekTotal, 2)}
                </p>
              )}
            </div>
            <div className="ps-pair" aria-label="Average monthly income">
              <div className="ps-cell ps-income">
                <dt>
                  Average <strong>previous</strong> 12 months
                  <span className="ps-avg-income">Income</span>
                </dt>
                <dd>
                  {summary.avgMonthlyActualIncomeMinor == null
                    ? "unknown"
                    : `${formatUsd(
                        summary.avgMonthlyActualIncomeMinor,
                        summary.scale ?? 2,
                      )} (${formatUsd(
                        summary.avgMonthlyActualIncomeMinor * 12,
                        summary.scale ?? 2,
                      )})`}
                </dd>
                <p className="ps-note">Actual paid dividends, same accounts</p>
              </div>
              <div className="ps-cell ps-income">
                <dt>
                  Average Monthly Plan
                  <span className="ps-avg-income">Income</span>
                </dt>
                <dd>
                  {summary.avgMonthlyPlanIncomeMinor == null
                    ? "unknown"
                    : `${formatUsd(
                        summary.avgMonthlyPlanIncomeMinor,
                        summary.scale ?? 2,
                      )} (${formatUsd(
                        summary.avgMonthlyPlanIncomeMinor * 12,
                        summary.scale ?? 2,
                      )})`}
                </dd>
                <p className="ps-note">Account 9, Income, FI Roth, Car — annual ÷ 12</p>
              </div>
            </div>
            <div className="ps-cell ps-coverage">
              <dt>Dividend Managed positions</dt>
              <dd>
                {formatCount(summary.declarationCount ?? 0)} of{" "}
                {formatCount(summary.declarationCollectorCount ?? 0)}
              </dd>
              <p className="ps-note">
                Last update{" "}
                {declarationBusy
                  ? "…"
                  : summary.declarationRefreshedOn?.trim().slice(0, 10) ||
                    "none"}
              </p>
              <button
                type="button"
                aria-label="Refresh declarations"
                aria-busy={declarationBusy}
                disabled={declarationBusy || busy}
                onClick={() => {
                  void refreshDeclarations(true);
                }}
              >
                {declarationBusy ? (
                  <span
                    className="ps-refresh-count"
                    role="status"
                    aria-live="polite"
                    aria-label="Declaration refresh progress"
                  >
                    {declarationRefreshLabel(declarationProgress)}
                  </span>
                ) : (
                  "Refresh declarations"
                )}
              </button>
            </div>
            <div className="ps-cell ps-coverage">
              <dt>Open tickets</dt>
              <dd>{formatCount(summary.openTicketCount ?? 0)}</dd>
              <p className="ps-note">
                Stay until filed — not today&apos;s miss count
              </p>
              <button
                type="button"
                aria-label="Work Tickets"
                onClick={() => {
                  onWorkTickets();
                }}
              >
                Work Tickets
              </button>
            </div>
            <div className="ps-cell ps-coverage">
              <dt>Last Price all symbols</dt>
              <dd>
                {formatCount(summary.lastPriceCount ?? 0)} of{" "}
                {formatCount(summary.symbolCount ?? 0)}
              </dd>
              <p className="ps-note">
                Last refresh{" "}
                {lastPriceBusy
                  ? "…"
                  : summary.lastPriceRefreshedOn?.trim().slice(0, 10) || "none"}
              </p>
              <button
                type="button"
                aria-label="Refresh last prices"
                aria-busy={lastPriceBusy}
                disabled={lastPriceBusy || busy}
                onClick={() => {
                  void refreshLastPrices(true);
                }}
              >
                {lastPriceBusy ? (
                  <span
                    className="ps-refresh-count"
                    role="status"
                    aria-live="polite"
                    aria-label="Last price refresh progress"
                  >
                    {lastPriceRefreshLabel(lastPriceProgress)}
                  </span>
                ) : (
                  "Refresh last prices"
                )}
              </button>
            </div>
          </dl>
        </section>
        ) : (
          <p role="status">Loading portfolio summary…</p>
        )}
        <div className="home-top-right">
        <HomeDividendPlan plan={dividendPlan} />
        <AccountCashFlow
          weeks={accountValues?.weeks ?? trends?.weeks}
          asOf={asOfDate}
        />
        </div>
        </div>
        {incomeTxOpen ? (
          <div
            className="home-av-dialog-backdrop"
            onClick={() => setIncomeTxOpen(false)}
          >
            <div
              role="dialog"
              aria-modal="true"
              aria-label="Income through transactions"
              className="home-av-dialog income-tx-dialog"
              onClick={(event) => event.stopPropagation()}
            >
              <header>
                <h3>
                  Income through {incomeThroughOn || "none"}
                </h3>
                <button
                  type="button"
                  aria-label="Exit income through transactions"
                  onClick={() => setIncomeTxOpen(false)}
                >
                  Exit
                </button>
              </header>
              <div className="income-tx-period-bar">
                <label>
                  Period
                  <select
                    aria-label="Income transaction period"
                    value={incomeTxPeriod}
                    onChange={(e) => {
                      const next = e.target.value as IncomeTxPeriod;
                      setIncomeTxPeriod(next);
                      if (next === "custom" && (!incomeTxCustomStart || !incomeTxCustomEnd)) {
                        const month = incomeTxRange("month", incomeTxToday, "", "");
                        setIncomeTxCustomStart(month?.startOn ?? incomeTxToday);
                        setIncomeTxCustomEnd(month?.endOn ?? incomeTxToday);
                      }
                    }}
                  >
                    {INCOME_TX_PERIOD_OPTIONS.map((opt) => (
                      <option key={opt.value} value={opt.value}>
                        {opt.label}
                      </option>
                    ))}
                  </select>
                </label>
                <label>
                  Account
                  <select
                    aria-label="Income transaction account"
                    value={incomeTxAccountId}
                    onChange={(e) => setIncomeTxAccountId(e.target.value)}
                  >
                    <option value="">All accounts</option>
                    {incomeTxAccountOptions.map((acct) => (
                      <option key={acct.accountId} value={acct.accountId}>
                        {acct.name}
                      </option>
                    ))}
                  </select>
                </label>
                {incomeTxPeriod === "custom" ? (
                  <>
                    <label>
                      From
                      <input
                        type="date"
                        aria-label="Income transaction start"
                        value={incomeTxCustomStart}
                        onChange={(e) => setIncomeTxCustomStart(e.target.value)}
                      />
                    </label>
                    <label>
                      To
                      <input
                        type="date"
                        aria-label="Income transaction end"
                        value={incomeTxCustomEnd}
                        onChange={(e) => setIncomeTxCustomEnd(e.target.value)}
                      />
                    </label>
                  </>
                ) : null}
                <p className="income-tx-range-note">
                  {incomeTxWindow
                    ? `${incomeTxWindow.startOn} to ${incomeTxWindow.endOn}`
                    : "Choose a start and end date"}
                </p>
              </div>
              {incomeThroughRows.length === 0 ? (
                <p className="home-av-empty">No paid dividends in this range.</p>
              ) : (
                <div className="income-tx-scroll">
                  <table className="income-tx-table">
                    <thead>
                      <tr>
                        <th>Date</th>
                        <th>Acct</th>
                        <th>Symbol</th>
                        <th className="numeric">Amount</th>
                      </tr>
                    </thead>
                    <tbody>
                      {incomeThroughRows.map((row) => (
                        <tr key={row.id}>
                          <td>{row.date}</td>
                          <td>{row.account}</td>
                          <td>{row.symbol}</td>
                          <td className="numeric">
                            {formatUsd(row.amountMinor, row.scale)}
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              )}
            </div>
          </div>
        ) : null}
        <HomeAccountCharts values={accountValues} />

    </>
  );
}
