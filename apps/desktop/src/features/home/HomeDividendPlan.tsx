import type { DividendPlanHomeGet, DividendPlanRow } from "@finos/app-contracts";
import { formatBps, formatUsd } from "@finos/ui-components";

function moneyKnown(minor: number | null | undefined, scale: number): string {
  if (minor == null) return "unknown";
  return formatUsd(minor, scale);
}

function moneyBlank(minor: number | null | undefined, scale: number): string {
  if (minor == null) return "";
  return formatUsd(minor, scale);
}

function PlanCells({
  row,
  scale,
  total,
}: {
  row: DividendPlanRow;
  scale: number;
  total?: boolean;
}) {
  const medicalAccount = row.accountName === "Health";
  const mark = total ? " dp-total" : "";
  return (
    <>
      <div className={`dp-name${mark}`}>{row.accountName}</div>
      <div className={`numeric${mark}`}>{moneyKnown(row.annualDividendMinor, scale)}</div>
      <div className={`numeric${mark}`}>{moneyKnown(row.marketValueMinor, scale)}</div>
      <div className={`numeric${mark}`}>
        {medicalAccount
          ? moneyBlank(row.monthlyIncomeMinor, scale)
          : moneyKnown(row.monthlyIncomeMinor, scale)}
      </div>
      <div className={`numeric${mark}`}>
        {medicalAccount || total
          ? moneyKnown(row.monthlyMedicalMinor, scale)
          : moneyBlank(row.monthlyMedicalMinor, scale)}
      </div>
      <div className={`numeric${mark}`}>{moneyKnown(row.weeklyMinor, scale)}</div>
      <div className={`numeric${mark}`}>{formatBps(row.effectiveAnnualBps)}</div>
    </>
  );
}

export function HomeDividendPlan({
  plan,
}: {
  plan: DividendPlanHomeGet | null;
}) {
  if (!plan) {
    return <p role="status">Loading Dividend Plan…</p>;
  }
  const scale = plan.scale ?? 2;
  return (
    <section className="home-dividend-plan" aria-label="Dividend Plan">
      <h2>Dividend Plan</h2>
      <div className="dp-grid" role="table">
        <div className="dp-head" role="columnheader">
          Account
        </div>
        <div className="dp-head numeric" role="columnheader">
          Annual dividend
        </div>
        <div className="dp-head numeric" role="columnheader">
          Market value
        </div>
        <div className="dp-head numeric" role="columnheader">
          Monthly Income
        </div>
        <div className="dp-head numeric" role="columnheader">
          Monthly Medical
        </div>
        <div className="dp-head numeric" role="columnheader">
          Weekly
        </div>
        <div
          className="dp-head numeric"
          role="columnheader"
          title="Plan annual dividend ÷ account market value"
        >
          Effective annual return
        </div>
        {plan.rows.map((row) => (
          <PlanCells key={row.accountName} row={row} scale={scale} />
        ))}
        <PlanCells row={plan.total} scale={scale} total />
      </div>
    </section>
  );
}
