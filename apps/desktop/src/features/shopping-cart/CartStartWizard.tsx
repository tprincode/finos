import { AccountSelect, type AccountOption } from "../shared/pickers";

export type CartFunding = "sellLots" | "accountCash" | "newDeposit";
export type CartWizardPrompt = "account" | "planName" | "funding";

export function noCashAccount(name: string): boolean {
  const n = name.trim().toLowerCase();
  return n.includes("energy") || n.includes("robinhood");
}

export function wizardRailStep(
  prompt: CartWizardPrompt,
): "Account" | "Plan name" | "How funded" {
  if (prompt === "account") return "Account";
  if (prompt === "planName") return "Plan name";
  return "How funded";
}

export function CartStartWizard({
  prompt,
  accounts,
  accountId,
  planName,
  funding,
  accountCashOffered,
  busy,
  writesBlocked,
  onAccountId,
  onPlanName,
  onFunding,
  onBack,
  onNext,
}: {
  prompt: CartWizardPrompt;
  accounts: AccountOption[];
  accountId: string;
  planName: string;
  funding: CartFunding;
  accountCashOffered: boolean;
  busy?: boolean;
  writesBlocked?: boolean;
  onAccountId: (id: string) => void;
  onPlanName: (name: string) => void;
  onFunding: (funding: CartFunding) => void;
  onBack: () => void;
  onNext: () => void;
}) {
  const nextDisabled =
    busy ||
    writesBlocked ||
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
      {prompt === "funding" ? (
        <fieldset>
          <legend>How funded</legend>
          <div role="radiogroup" aria-label="Cart funding">
            <label>
              <input
                type="radio"
                name="cart-funding"
                value="sellLots"
                checked={funding === "sellLots"}
                onChange={() => onFunding("sellLots")}
                disabled={busy || writesBlocked}
              />
              Sell lots
            </label>
            {accountCashOffered ? (
              <label>
                <input
                  type="radio"
                  name="cart-funding"
                  value="accountCash"
                  checked={funding === "accountCash"}
                  onChange={() => onFunding("accountCash")}
                  disabled={busy || writesBlocked}
                />
                Account cash
              </label>
            ) : null}
            <label>
              <input
                type="radio"
                name="cart-funding"
                value="newDeposit"
                checked={funding === "newDeposit"}
                onChange={() => onFunding("newDeposit")}
                disabled={busy || writesBlocked}
              />
              New deposit
            </label>
          </div>
        </fieldset>
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
        <button
          type="button"
          aria-label="Next cart step"
          disabled={nextDisabled}
          onClick={onNext}
        >
          Next
        </button>
      </div>
    </section>
  );
}
