import type { CartEval } from "@finos/app-contracts";

export function TradeoffCallout({
  eval: ev,
  mixWorse,
  incomeRises,
  overrideReason,
  onOverrideReason,
}: {
  eval: CartEval | null;
  mixWorse: boolean;
  incomeRises: boolean;
  overrideReason: string;
  onOverrideReason: (reason: string) => void;
}) {
  if (!ev) return null;
  const needsReason = ev.cashFloorWarn || mixWorse;
  if (!needsReason && !ev.insufficientLotQty) return null;
  return (
    <aside className="cart-tradeoff" aria-label="Tradeoff">
      {ev.insufficientLotQty ? (
        <p className="blocked" role="status">
          Named remaining does not cover spend. Math is intent only. Agree stays blocked.
        </p>
      ) : null}
      {ev.cashFloorWarn ? (
        <p role="status">Cash leftover is below the account floor. Typed reason required to Agree.</p>
      ) : null}
      {incomeRises && mixWorse ? (
        <>
          <p role="status">Income would rise versus keeping the cash in the money-market plan.</p>
          <p role="status">
            Mix would worsen versus target (more Risk On or less Foundation). Typed reason required
            to Agree.
          </p>
        </>
      ) : mixWorse ? (
        <p role="status">
          Mix would worsen versus target (more Risk On or less Foundation). Typed reason required to
          Agree.
        </p>
      ) : null}
      {needsReason && !ev.insufficientLotQty ? (
        <label>
          Override reason
          <input
            aria-label="Cart override reason"
            value={overrideReason}
            onChange={(e) => onOverrideReason(e.target.value)}
          />
        </label>
      ) : null}
    </aside>
  );
}
