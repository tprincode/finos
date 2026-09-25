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

function cadenceLabel(periods: number | null | undefined): string {
  if (periods === 52) return "Weekly";
  if (periods === 12) return "Monthly";
  if (periods === 4) return "Quarterly";
  if (periods === 1) return "Annual";
  if (periods == null) return "No plan";
  return `${periods} periods`;
}

function incomeRule(periods: number | null | undefined): string {
  if (periods === 52) return "Current plan payment × 52";
  if (periods === 12) return "Current plan payment × 12";
  if (periods === 4) return "Current plan payment × 4";
  if (periods === 1) return "Current plan payment × 1";
  if (periods == null) return "Held, but the current plan amount is missing";
  return `Current plan payment × ${periods}`;
}

function expenseRule(cadence: string): string {
  const key = cadence.trim().toLowerCase();
  if (key === "weekly") return "Scheduled amount × weeks still ahead, 52 if it runs all year";
  if (key === "monthly") return "Scheduled amount × months still ahead, 12 if it runs all year";
  if (key === "annual" || key === "yearly") return "Scheduled amount × 1 if that payment is still ahead";
  if (key === "one-time") return "Counted once, only if the date is still ahead";
  return "Scheduled amount × occurrences still ahead";
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
  const groups = new Map<string, { count: number; payment: number | null; year: number | null; periods: number | null }>();
  for (const line of lines) {
    const key = String(line.periods ?? "none");
    const group = groups.get(key) ?? { count: 0, payment: null, year: null, periods: line.periods };
    group.count += 1;
    if (line.perPeriodMinor != null) group.payment = (group.payment ?? 0) + line.perPeriodMinor;
    if (line.yearMinor != null) group.year = (group.year ?? 0) + line.yearMinor;
    groups.set(key, group);
  }
  const rows = [...groups.values()];
  return (
    <div className="table-wrap cash-coverage-table-wrap">
      <table aria-label="Coverage income math">
        <caption className="cash-coverage-caption">
          Dividends use the current plan amount per payment, annualized for the next 12 months. Amount per payment × periods. Week above is this year ÷ 52. Month above is this year ÷ 12.
        </caption>
        <thead>
          <tr>
            <th>Plan</th>
            <th className="numeric">Positions</th>
            <th>How the year is made</th>
            <th className="numeric">Current payments</th>
            <th className="numeric">Annualized dividends</th>
          </tr>
        </thead>
        <tbody>
          {rows.length === 0 ? (
            <tr>
              <td colSpan={5}>No planned dividends.</td>
            </tr>
          ) : (
            rows.map((row) => (
              <tr key={String(row.periods)}>
                <th scope="row">{cadenceLabel(row.periods)}</th>
                <td className="numeric">{row.count}</td>
                <td>{incomeRule(row.periods)}</td>
                <td className="numeric">{money(row.payment)}</td>
                <td className="numeric">{money(row.year)}</td>
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
  const groups = new Map<string, { count: number; year: number }>();
  for (const line of lines) {
    const key = line.cadence.trim().toLowerCase() || "scheduled";
    const group = groups.get(key) ?? { count: 0, year: 0 };
    group.count += 1;
    group.year += line.yearMinor;
    groups.set(key, group);
  }
  const rows = [...groups.entries()];
  return (
    <div className="table-wrap cash-coverage-table-wrap">
      <table aria-label="Coverage expense math">
        <caption className="cash-coverage-caption">
          Withdrawals use the scheduled amount, counted for the occurrences still ahead in the next 12 months. Week above is this year ÷ 52. Month above is this year ÷ 12. A withdrawal that has already ended is omitted.
        </caption>
        <thead>
          <tr>
            <th>Schedule</th>
            <th className="numeric">Withdrawals</th>
            <th>How the year is made</th>
            <th className="numeric">Annualized withdrawals</th>
          </tr>
        </thead>
        <tbody>
          {rows.length === 0 ? (
            <tr>
              <td colSpan={4}>No planned withdrawals.</td>
            </tr>
          ) : (
            rows.map(([cadence, row]) => (
              <tr key={cadence}>
                <th scope="row">{cadence.charAt(0).toUpperCase() + cadence.slice(1)}</th>
                <td className="numeric">{row.count}</td>
                <td>{expenseRule(cadence)}</td>
                <td className="numeric">{formatUsd(row.year, scale)}</td>
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
      aria-label="Cash Management Coverage"
    >
      <header className="cash-coverage-head">
        <h2>{title}</h2>
        <p className="cash-coverage-window">{window}</p>
      </header>
      <div className="cash-coverage-periods" role="group" aria-label="Coverage period">
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
      <div className="table-wrap cash-coverage-table-wrap">
        <table aria-label="Coverage plan">
          <caption className="cash-coverage-caption">
            {title} · planned income vs planned withdrawals · {divisor}
          </caption>
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
