const TIERS = [
  ["Foundation", "foundation"],
  ["Core", "core"],
  ["Risk On", "riskOn"],
] as const;

export type MixSlice = {
  foundation: number;
  core: number;
  riskOn: number;
};

function pct(part: number, total: number): string {
  if (total <= 0) return "unknown";
  return `${((part / total) * 100).toFixed(1)}%`;
}

function points(before: number, after: number, beforeTotal: number, afterTotal: number): string {
  if (beforeTotal <= 0 || afterTotal <= 0) return "unknown";
  const gap = (after / afterTotal - before / beforeTotal) * 100;
  const sign = gap > 0 ? "+" : "";
  return `${sign}${gap.toFixed(1)} pp`;
}

function dollars(minor: number): string {
  return (minor / 100).toLocaleString("en-US", {
    style: "currency",
    currency: "USD",
  });
}

function signedDollars(minor: number): string {
  if (minor === 0) return dollars(0);
  const body = dollars(Math.abs(minor));
  return minor > 0 ? `+${body}` : `-${body}`;
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
    <section aria-label="Allocation gap">
      <h3>Allocation gap</h3>
      <table>
        <thead>
          <tr>
            <th scope="col">Tier</th>
            <th scope="col">Before</th>
            <th scope="col">After</th>
            <th scope="col">Gap</th>
          </tr>
        </thead>
        <tbody>
          {TIERS.map(([label, key]) => {
            const before = current[key];
            const next = after[key];
            return (
              <tr key={key}>
                <th scope="row">{label}</th>
                <td>
                  {dollars(before)} · {pct(before, curTotal)}
                </td>
                <td>
                  {dollars(next)} · {pct(next, afterTotal)}
                </td>
                <td>
                  {signedDollars(next - before)} · {points(before, next, curTotal, afterTotal)}
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
      <p className="cart-mix-note">
        Cash {dollars(cashCurrentMinor)} before · {dollars(cashAfterMinor)} after. Cash stays off
        the mix.
      </p>
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
