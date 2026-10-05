/** Cart per-share prices use scale 4 (1/10000 USD). Proceeds/spend stay USD cents. */
export const CART_UNIT_SCALE = 4;

export function rescaleMinor(minor: number, fromScale: number, toScale: number): number {
  if (fromScale === toScale) return minor;
  if (fromScale > toScale) return Math.round(minor / 10 ** (fromScale - toScale));
  return minor * 10 ** (toScale - fromScale);
}

/** Per-share minor → USD cents. Scale 4 last 1014703 is $101.47, not $10,147. */
export function priceCents(minor: number, scale: number | null | undefined): number {
  return rescaleMinor(minor, scale ?? 2, 2);
}

export function cartPriceInput(minor: number, scale: number): string {
  const s = Math.max(0, Math.trunc(scale));
  const digits = Math.abs(Math.trunc(minor)).toString().padStart(s + 1, "0");
  const i = digits.length - s;
  const whole = digits.slice(0, i);
  const frac = digits.slice(i).padEnd(s, "0");
  const text = s > 0 ? `${whole}.${frac}` : whole;
  return (minor < 0 ? "-" : "") + text;
}

export function parseCartPrice(raw: string): number | null {
  const dollars = Number(raw.trim());
  if (!Number.isFinite(dollars) || dollars <= 0) return null;
  return Math.round(dollars * 10 ** CART_UNIT_SCALE);
}

/** USD cents for whole-share qty × per-share minor at `priceScale`. */
export function spendCentsFromPrice(qtyWhole: number, priceMinor: number, priceScale: number): number {
  const p = 10 ** Math.max(0, Math.trunc(priceScale));
  if (!Number.isFinite(p) || p === 0) return 0;
  return Math.round((qtyWhole * priceMinor * 100) / p);
}

/**
 * Per-share minor at scale 4 for the execute table.
 * If the stored scale does not reproduce the stored dollar total, use scale 2
 * (cent prices that were labeled scale 4 display 100× too small).
 */
export function executeUnitMinor(
  storedMinor: number,
  storedScale: number | undefined,
  qtyWhole: number,
  totalCents: number,
): number {
  const scale = storedScale ?? 2;
  const matches = (s: number) =>
    qtyWhole > 0 && spendCentsFromPrice(qtyWhole, storedMinor, s) === totalCents;
  const from = matches(scale) ? scale : matches(2) ? 2 : scale;
  return rescaleMinor(storedMinor, from, CART_UNIT_SCALE);
}
