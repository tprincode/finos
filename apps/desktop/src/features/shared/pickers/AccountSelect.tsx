export type AccountOption = {
  accountId: string;
  name: string;
};

export function AccountSelect({
  accounts,
  value,
  onChange,
  ariaLabel,
  disabled,
}: {
  accounts: AccountOption[];
  value: string;
  onChange: (accountId: string) => void;
  ariaLabel: string;
  disabled?: boolean;
}) {
  return (
    <label>
      Account
      <select
        aria-label={ariaLabel}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        disabled={disabled}
      >
        <option value="">Choose account</option>
        {accounts.map((a) => (
          <option key={a.accountId} value={a.accountId}>
            {a.name}
          </option>
        ))}
      </select>
    </label>
  );
}
