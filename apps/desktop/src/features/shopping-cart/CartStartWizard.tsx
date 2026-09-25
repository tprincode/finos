import { AccountSelect, type AccountOption } from "../shared/pickers";
import { usd } from "./planMath";

export type CartFunding = "sellLots" | "accountCash" | "newDeposit";
export type CartWizardPrompt = "account" | "plans" | "planName";

export type SavedCartPlan = {
  planId: string;
  name: string;
  asOf: string;
  status: string;
  scenarioCount: number;
  sellMinor: number;
  buyAMinor: number;
  buyBMinor: number | null;
  scenarioIds: string[];
};

function buySummary(plan: SavedCartPlan): string {
  if (plan.buyBMinor == null) return usd(plan.buyAMinor);
  return `A ${usd(plan.buyAMinor)} · B ${usd(plan.buyBMinor)}`;
}

export function noCashAccount(name: string): boolean {
  const n = name.trim().toLowerCase();
  return n.includes("energy") || n.includes("robinhood");
}

export function wizardRailStep(prompt: CartWizardPrompt): "Account" | "Plan name" {
  if (prompt === "account") return "Account";
  return "Plan name";
}

export function CartStartWizard({
  prompt,
  accounts,
  accountId,
  accountName,
  planName,
  savedPlans,
  busy,
  writesBlocked,
  onAccountId,
  onPlanName,
  onOpenPlan,
  onDeletePlan,
  onNewPlan,
  onBack,
  onNext,
}: {
  prompt: CartWizardPrompt;
  accounts: AccountOption[];
  accountId: string;
  accountName: string;
  planName: string;
  savedPlans: SavedCartPlan[];
  busy?: boolean;
  writesBlocked?: boolean;
  onAccountId: (id: string) => void;
  onPlanName: (name: string) => void;
  onOpenPlan: (planId: string) => void;
  onDeletePlan: (plan: SavedCartPlan) => void;
  onNewPlan: () => void;
  onBack: () => void;
  onNext: () => void;
}) {
  const nextDisabled =
    busy ||
    writesBlocked ||
    prompt === "plans" ||
    (prompt === "account" && !accountId);
  return (
    <section aria-label="Cart start wizard">
      {prompt === "account" ? (
        <AccountSelect
          accounts={accounts}
          value={accountId}
          onChange={onAccountId}
          ariaLabel="Cart account"
          disabled={busy || writesBlocked}
        />
      ) : null}
      {prompt === "plans" ? (
        <section aria-label="Saved plans" className="plan-sheet">
          <h3>Saved plans for {accountName}</h3>
          <table>
            <thead>
              <tr>
                <th scope="col">Plan</th>
                <th scope="col">As of</th>
                <th scope="col">Status</th>
                <th scope="col">Scenarios</th>
                <th scope="col">Total sell</th>
                <th scope="col">Total buy</th>
                <th scope="col">Open</th>
                <th scope="col">Delete</th>
              </tr>
            </thead>
            <tbody>
              {savedPlans.length === 0 ? (
                <tr>
                  <td colSpan={8}>No saved plans for this account.</td>
                </tr>
              ) : (
                savedPlans.map((plan) => (
                  <tr key={plan.planId}>
                    <td>{plan.name}</td>
                    <td>{plan.asOf.slice(0, 10)}</td>
                    <td>{plan.status}</td>
                    <td>{plan.scenarioCount}</td>
                    <td>{usd(plan.sellMinor)}</td>
                    <td>{buySummary(plan)}</td>
                    <td>
                      <button
                        type="button"
                        aria-label={`Open plan ${plan.name}`}
                        disabled={busy || writesBlocked}
                        onClick={() => onOpenPlan(plan.planId)}
                      >
                        Open
                      </button>
                    </td>
                    <td>
                      <button
                        type="button"
                        aria-label={`Delete plan ${plan.name}`}
                        disabled={busy || writesBlocked}
                        onClick={() => onDeletePlan(plan)}
                      >
                        Delete
                      </button>
                    </td>
                  </tr>
                ))
              )}
            </tbody>
          </table>
          <button
            type="button"
            aria-label="New cart plan"
            disabled={busy || writesBlocked}
            onClick={onNewPlan}
          >
            New plan
          </button>
        </section>
      ) : null}
      {prompt === "planName" ? (
        <label>
          Plan name
          <input
            aria-label="Cart plan name"
            value={planName}
            onChange={(e) => onPlanName(e.target.value)}
            disabled={busy || writesBlocked}
          />
        </label>
      ) : null}
      <div className="buttons">
        <button
          type="button"
          aria-label="Previous cart step"
          disabled={busy || prompt === "account"}
          onClick={onBack}
        >
          Back
        </button>
        {prompt === "plans" ? null : (
          <button
            type="button"
            aria-label="Next cart step"
            disabled={nextDisabled}
            onClick={onNext}
          >
            Next
          </button>
        )}
      </div>
    </section>
  );
}
