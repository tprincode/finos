export type LotOption = {
  lotId: string;
  symbol: string;
  accountName: string;
  remainingQuantityMinor: number;
  quantityScale: number;
};

function formatQty(minor: number, scale: number): string {
  return (minor / 10 ** scale).toFixed(scale);
}

export function LotSelect({
  lots,
  value,
  onChange,
  ariaLabel,
  disabled,
  accountName,
}: {
  lots: LotOption[];
  value: string;
  onChange: (lotId: string) => void;
  ariaLabel: string;
  disabled?: boolean;
  accountName?: string;
}) {
  const visible = accountName
    ? lots.filter((lot) => lot.accountName === accountName)
    : lots;
  return (
    <label>
      Lot
      <select
        aria-label={ariaLabel}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        disabled={disabled}
      >
        <option value="">Choose lot</option>
        {visible.map((lot) => (
          <option key={lot.lotId} value={lot.lotId}>
            {lot.symbol} · {formatQty(lot.remainingQuantityMinor, lot.quantityScale)} remaining ·{" "}
            {lot.accountName}
          </option>
        ))}
      </select>
    </label>
  );
}
