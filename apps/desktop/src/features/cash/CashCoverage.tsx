import { formatUsd } from "@finos/ui-components";
import type {
  CashCoverageExpenseLine,
  CashCoverageGet,
  CashCoverageIncomeLine,
  CashCoverageRow,
} from "@finos/app-contracts";

export type CoveragePeriod = "week" | "month" | "year";

function comparisonTitle(period: CoveragePeriod): string {
  if (period === "month") {
    return "Monthly comparison";
  }
  if (period === "year") {
    return "Annual comparison";
  }
  return "Weekly comparison";
}

function MoneyRows({
  rows,
  scale,
  cells,
}: {
  rows: CashCoverageRow[];
  scale: number;
  cells: (row: CashCoverageRow) => Array<number | null | undefined>;
}) {
  const money = (minor: number | null | undefined) =>
    minor == null ? "—" : formatUsd(minor, scale);
  return (
    <tbody>
      {rows.map((row) => (
        <tr key={row.account}>
          <th scope="row">{row.account}</th>
          {cells(row).map((minor, i) => (
            <td key={`${row.account}-${i}`} className="numeric">
              {money(minor)}
            </td>
          ))}
        </tr>
      ))}
    </tbody>
  );
}

function IncomeMath({
  lines,
  scale,
}: {
  lines: CashCoverageIncomeLine[];
  scale: number;
}) {
  const money = (minor: number | null | undefined) =>
    minor == null ? "—" : formatUsd(minor, scale);
  return (
    <div
      className="table-wrap cash-coverage-table-wrap"
      id="coverage-income-math"
      data-section="coverage-income-math"
      data-part="coverage-income"
    >
      <p className="cash-coverage-caption">
        Amount per payment × periods for the next 12 months. Week above = year ÷ 52; Month = year ÷ 12.
      </p>
      <table aria-label="Coverage income math">
        <thead>
          <tr>
            <th>Account</th>
            <th>Symbol</th>
            <th className="numeric">Per payment</th>
            <th className="numeric">Periods</th>
            <th className="numeric">Year</th>
          </tr>
        </thead>
        <tbody>
          {lines.length === 0 ? (
            <tr>
              <td colSpan={5}>No planned dividends.</td>
            </tr>
          ) : (
            lines.map((line) => (
              <tr key={`${line.account}:${line.symbol}`}>
                <th scope="row">{line.account}</th>
                <td>{line.symbol}</td>
                <td className="numeric">{money(line.perPeriodMinor)}</td>
                <td className="numeric">{line.periods ?? "—"}</td>
                <td className="numeric">{money(line.yearMinor)}</td>
              </tr>
            ))
          )}
        </tbody>
      </table>
    </div>
  );
}

function ExpenseMath({
  lines,
  scale,
}: {
  lines: CashCoverageExpenseLine[];
  scale: number;
}) {
  return (
    <div
      className="table-wrap cash-coverage-table-wrap"
      id="coverage-expense-math"
      data-section="coverage-expense-math"
      data-part="coverage-expense"
    >
      <p className="cash-coverage-caption">
        Scheduled amount × remaining occurrences in the next 12 months. Week above = year ÷ 52; Month = year ÷ 12. Schedules past their stop date are left out.
      </p>
      <table aria-label="Coverage expense math">
        <thead>
          <tr>
            <th>Account</th>
            <th>Name</th>
            <th className="numeric">Per</th>
            <th className="numeric">Periods</th>
            <th className="numeric">Year</th>
          </tr>
        </thead>
        <tbody>
          {lines.length === 0 ? (
            <tr>
              <td colSpan={5}>No planned withdrawals.</td>
            </tr>
          ) : (
            lines.map((line) => (
              <tr key={`${line.account}:${line.name}:${line.cadence}`}>
                <th scope="row">{line.account}</th>
                <td>{line.name}</td>
                <td className="numeric">{formatUsd(line.perPeriodMinor, scale)}</td>
                <td className="numeric">{line.periods}</td>
                <td className="numeric">{formatUsd(line.yearMinor, scale)}</td>
              </tr>
            ))
          )}
        </tbody>
      </table>
    </div>
  );
}

export function CashCoveragePanel({
  coverage,
  period,
  busy,
  onPeriod,
}: {
  coverage: CashCoverageGet | null;
  period: CoveragePeriod;
  busy?: boolean;
  onPeriod: (period: CoveragePeriod) => void;
}) {
  const scale = coverage?.scale ?? 2;
  const rows = coverage?.rows ?? [];
  const title = comparisonTitle(period);
  const periodChip =
    period === "month" ? "Month" : period === "year" ? "Year" : "Week";
  const loading = Boolean(busy) || coverage == null;
  const window = loading
    ? `Loading ${periodChip}…`
    : `next 12 months · ${coverage?.periodStart ?? ""} – ${coverage?.periodEnd ?? ""}`;
  const divisor =
    period === "month" ? "Month = year ÷ 12." : period === "year" ? "Year total." : "Week = year ÷ 52.";

  return (
    <section
      className="cash-coverage"
      id="cash-coverage"
      data-section="coverage-summary"
      aria-label="Income vs Expense planner"
    >
      <header className="cash-coverage-head">
        <h2>{title}</h2>
        <p className="cash-coverage-window">{window}</p>
      </header>
      <div
        className="cash-coverage-periods"
        role="group"
        aria-label="Coverage period"
        data-part="coverage-period"
      >
        {(
          [
            ["week", "Week"],
            ["month", "Month"],
            ["year", "Year"],
          ] as const
        ).map(([id, label]) => (
          <button
            key={id}
            type="button"
            aria-label={`Coverage ${label}`}
            aria-pressed={period === id}
            disabled={busy}
            onClick={() => onPeriod(id)}
          >
            {label}
          </button>
        ))}
      </div>
      {loading ? (
        <p className="cash-coverage-loading" role="status">
          Loading {periodChip}…
        </p>
      ) : null}
      {loading ? null : (
      <>
      <div className="table-wrap cash-coverage-table-wrap" data-part="coverage-plan">
        <p className="cash-coverage-caption">
          {title} · planned income vs planned withdrawals · {divisor}
        </p>
        <table aria-label="Coverage plan">
          <thead>
            <tr>
              <th>Account</th>
              <th className="numeric">Income</th>
              <th className="numeric">Expenses</th>
              <th className="numeric">Δ</th>
            </tr>
          </thead>
          <MoneyRows
            rows={rows}
            scale={scale}
            cells={(row) => [
              row.planIncomeMinor,
              row.planExpenseMinor,
              row.planMinor,
            ]}
          />
        </table>
      </div>
      <IncomeMath lines={coverage?.incomeLines ?? []} scale={scale} />
      <ExpenseMath lines={coverage?.expenseLines ?? []} scale={scale} />
      </>
      )}
    </section>
  );
}
