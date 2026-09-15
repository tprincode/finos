const TIERS = ["Foundation", "Core", "Risk On"] as const;

export type MixSlice = {
  foundation: number;
  core: number;
  riskOn: number;
};

function pct(part: number, total: number): string {
  if (total <= 0) return "unknown";
  return `${((part / total) * 100).toFixed(1)}%`;
}

function dollars(minor: number): string {
  return (minor / 100).toLocaleString("en-US", {
    style: "currency",
    currency: "USD",
  });
}

export function MixBars({
  current,
  after,
  cashCurrentMinor,
  cashAfterMinor,
}: {
  current: MixSlice;
  after: MixSlice;
  cashCurrentMinor: number;
  cashAfterMinor: number;
}) {
  const curTotal = current.foundation + current.core + current.riskOn;
  const afterTotal = after.foundation + after.core + after.riskOn;
  return (
    <section aria-label="Mix">
      <h3>Mix (cash off the bar)</h3>
      <p>
        Current: Foundation {pct(current.foundation, curTotal)} · Core{" "}
        {pct(current.core, curTotal)} · Risk On {pct(current.riskOn, curTotal)}
        <span className="cart-cash-label"> · Cash {dollars(cashCurrentMinor)} (off mix)</span>
      </p>
      <p>
        After: Foundation {pct(after.foundation, afterTotal)} · Core {pct(after.core, afterTotal)} ·
        Risk On {pct(after.riskOn, afterTotal)}
        <span className="cart-cash-label"> · Cash {dollars(cashAfterMinor)} (off mix)</span>
      </p>
      <p className="cart-mix-note">Tiers: {TIERS.join(", ")}. Cash is a label beside the bar, not a slice.</p>
    </section>
  );
}

export function mixWorsens(current: MixSlice, after: MixSlice): boolean {
  const curTotal = current.foundation + current.core + current.riskOn;
  const afterTotal = after.foundation + after.core + after.riskOn;
  if (curTotal <= 0 || afterTotal <= 0) return false;
  const curRisk = current.riskOn / curTotal;
  const afterRisk = after.riskOn / afterTotal;
  const curFound = current.foundation / curTotal;
  const afterFound = after.foundation / afterTotal;
  return afterRisk > curRisk + 0.0001 || afterFound + 0.0001 < curFound;
}
