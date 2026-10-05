const STEPS = [
  "Account",
  "Plan name",
  "Evaluate",
  "Agree",
  "Confirm sell",
  "Confirm purchase",
  "Confirm cash",
  "Done",
] as const;

export type CartRailStep = (typeof STEPS)[number];

export function StepRail({ current }: { current: CartRailStep }) {
  const idx = STEPS.indexOf(current);
  return (
    <ol className="cart-step-rail" aria-label="Shopping cart steps">
      {STEPS.map((step, i) => (
        <li key={step} aria-current={i === idx ? "step" : undefined}>
          {step}
        </li>
      ))}
    </ol>
  );
}
