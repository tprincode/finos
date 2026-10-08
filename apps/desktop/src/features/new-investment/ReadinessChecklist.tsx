/**
 * Every step a new investment must satisfy before it is wired into the plans and reports,
 * read straight off `InvestmentGet.establishChecklist` so the screen and the goldens judge
 * the same list. Adding a step is a domain change, never a change here.
 */
export type ReadinessRow = {
  id: string;
  label: string;
  status: string;
  blocksComplete?: boolean;
};

const WHY: Record<string, string> = {
  template_dividend: "Without it no declaration is ever retrieved.",
  template_roc: "Needed before a 19a-1 ROC estimate can be confirmed.",
  div_type: "Decides whether this name is a payer or cash.",
  frequency: "Sets the period count every rate and projection uses.",
  provider: "Picks the issuer adapter.",
  underlying: "Owner-entered value stops the collector re-asking.",
  risk_tier: "Groups the position on the plan boards.",
  paid_history: "The series the 30% amount rule compares against.",
  remaining_year: "The pay dates the rest of the year is planned from.",
  roc_estimate: "Owner-accepted; a miss stays unknown, never 0%.",
  plan: "Shopping Cart and Income Plan read Plan, not declarations.",
  calculator: "Makes the position visible on the Calculator and Cart.",
  last_price: "A $0 price silently zeroes market value and yields.",
  declaration_adapter: "An unregistered issuer means no declarations, ever.",
  collector_enabled: "Run enabled skips the name until this is on.",
  plan_periods: "Zero periods is how a rate comes out wrong.",
  plan_window: "A Plan starting after the pay date leaves the week blank.",
  first_lot: "Optional. Supplies the quantity behind Income Plan dollars.",
};

export function ReadinessChecklist({
  rows,
  complete,
  openLabels,
}: {
  rows: ReadinessRow[];
  complete: boolean;
  openLabels: string[];
}) {
  if (!rows.length) {
    return null;
  }
  return (
    <section
      aria-label="Establish checklist"
      className="process-a-establish-checklist"
      id="establish-checklist"
      data-section="establish-checklist"
      data-part="establish-checklist"
    >
      <h4>Establish checklist</h4>
      <table>
        <thead>
          <tr>
            <th scope="col">Step</th>
            <th scope="col">Status</th>
            <th scope="col">Why it blocks</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((row) => (
            <tr key={row.id} className={`readiness-${row.status}`}>
              <td>{row.label}</td>
              <td>
                {row.status === "done"
                  ? "Done"
                  : row.status === "na"
                    ? "N/A"
                    : "Open"}
              </td>
              <td>{WHY[row.id] ?? ""}</td>
            </tr>
          ))}
        </tbody>
      </table>
      <p
        role="status"
        aria-label="Establish checklist footer"
        className={
          complete
            ? "investment-details-status is-complete"
            : "investment-details-status is-missing"
        }
      >
        {complete
          ? "Investment details complete"
          : `Information still needed${
              openLabels.length ? `: ${openLabels.join(", ")}` : ""
            }`}
      </p>
    </section>
  );
}
