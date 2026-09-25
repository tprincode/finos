export type AccountTickMode = "allOrOne" | "anyCombination" | "exactlyOne";

export function allAccountsSelected(
  accounts: readonly string[],
  selected: readonly string[],
): boolean {
  return accounts.length > 0 && accounts.every((name) => selected.includes(name));
}

export function nextAccountTicks(
  mode: AccountTickMode,
  accounts: readonly string[],
  selected: readonly string[],
  action: { type: "all" } | { type: "account"; name: string },
  allAccounts?: readonly string[],
): string[] {
  const allTarget = allAccounts ?? accounts;
  const allOn = allAccounts
    ? allTarget.length > 0 &&
      allTarget.every((name) => selected.includes(name)) &&
      selected.every((name) => allTarget.includes(name))
    : allAccountsSelected(accounts, selected);
  if (action.type === "all") {
    return allOn ? [] : [...allTarget];
  }
  if (mode === "exactlyOne") {
    return [action.name];
  }
  if (mode === "allOrOne") {
    if (allOn) return [...selected];
    if (selected.length === 1 && selected[0] === action.name) return [];
    return [action.name];
  }
  return selected.includes(action.name)
    ? selected.filter((name) => name !== action.name)
    : [...selected, action.name];
}

export function AccountTickPicker({
  legend = "Accounts",
  allLabel,
  accounts,
  allAccounts,
  selected,
  mode,
  showAll = true,
  onSelected,
}: {
  legend?: string;
  allLabel?: string;
  accounts: readonly string[];
  /** Names the All control turns on. Defaults to every listed account. */
  allAccounts?: readonly string[];
  selected: readonly string[];
  mode: AccountTickMode;
  showAll?: boolean;
  onSelected: (next: string[]) => void;
}) {
  const allTarget = allAccounts ?? accounts;
  const allOn = allAccounts
    ? allTarget.length > 0 &&
      allTarget.every((name) => selected.includes(name)) &&
      selected.every((name) => allTarget.includes(name))
    : allAccountsSelected(accounts, selected);
  const waiting = selected.length === 0 && mode !== "exactlyOne";
  const lockAccounts = mode === "allOrOne" && allOn;
  const renderAll = showAll && Boolean(allLabel);

  return (
    <fieldset className="income-account-picker" aria-label={legend}>
      <legend className="income-week-label">{legend}</legend>
      <div className="income-account-ticks">
        {renderAll ? (
        <label className={`income-account-tick${allOn ? "" : " off"}`}>
          <input
            type="checkbox"
            aria-label={allLabel}
            checked={allOn}
            onChange={() =>
              onSelected(
                nextAccountTicks(mode, accounts, selected, { type: "all" }, allAccounts),
              )
            }
          />
          {allLabel}
        </label>
        ) : null}
        {accounts.map((name) => (
          <label
            key={name}
            className={`income-account-tick${
              selected.includes(name) ? "" : " off"
            }${lockAccounts ? " is-locked" : ""}`}
          >
            <input
              type="checkbox"
              aria-label={`Filter ${name}`}
              checked={selected.includes(name)}
              disabled={lockAccounts}
              onChange={() =>
                onSelected(
                  nextAccountTicks(mode, accounts, selected, {
                    type: "account",
                    name,
                  }),
                )
              }
            />
            {name}
          </label>
        ))}
      </div>
      {waiting ? (
        <p role="status" className="account-tick-must-pick">
          Pick at least one account
        </p>
      ) : null}
    </fieldset>
  );
}
