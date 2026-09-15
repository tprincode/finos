export function ExecutePanel({
  agreed,
  sold,
  busy,
  writesBlocked,
  onConfirmSell,
  onOpenLot,
}: {
  agreed: boolean;
  sold: boolean;
  busy?: boolean;
  writesBlocked?: boolean;
  onConfirmSell: () => void;
  onOpenLot: () => void;
}) {
  if (!agreed) {
    return (
      <p role="status">Agree to freeze the swap before confirm sell and Add Lot.</p>
    );
  }
  return (
    <div className="buttons" aria-label="Execute swap">
      <button
        type="button"
        aria-label="Confirm sell"
        disabled={busy || writesBlocked || sold}
        onClick={onConfirmSell}
      >
        Confirm sell
      </button>
      <button
        type="button"
        aria-label="Open prefilled Add Lot"
        disabled={busy || !sold}
        onClick={onOpenLot}
      >
        Add Lot (prefilled)
      </button>
    </div>
  );
}
