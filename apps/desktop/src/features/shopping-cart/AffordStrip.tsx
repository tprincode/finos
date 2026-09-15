import type { CartEval } from "@finos/app-contracts";
import { formatUsd } from "@finos/ui-components";

function money(minor: number | null | undefined): string {
  if (minor == null) return "unknown";
  return formatUsd(minor, 2);
}

export function AffordStrip({ eval: ev }: { eval: CartEval | null }) {
  if (!ev) {
    return <p role="status">Evaluate to see remaining, spend, and leftover.</p>;
  }
  return (
    <dl className="cart-afford" aria-label="Afford">
      <div>
        <dt>Named remaining</dt>
        <dd>{money(ev.remainingMinor)}</dd>
      </div>
      <div>
        <dt>Spend</dt>
        <dd>{money(ev.spendMinor)}</dd>
      </div>
      <div>
        <dt>Leftover (still earns)</dt>
        <dd>{money(ev.leftoverMinor)}</dd>
      </div>
      {ev.insufficientLotQty ? (
        <p className="blocked" role="status">
          Spend exceeds live remaining — Agree is blocked (insufficient_lot_qty).
        </p>
      ) : null}
    </dl>
  );
}
