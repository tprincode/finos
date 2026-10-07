export function RoadmapScreen() {
  return (
    <section className="interest-rate-page" aria-label="Roadmap">
      <h2>Roadmap</h2>
      <article aria-label="cash-covered short puts">
        <h3>cash-covered short puts</h3>
        <p>
          A later hold reserves strike times 100 times quantity against that account's cash-symbol
          balance. Cash does not move until assignment. This screen does not reserve cash.
        </p>
      </article>
      <article aria-label="Checking account">
        <h3>Checking account</h3>
        <p>
          Checking stays off the v1 cash critical path until Car, Income, and Health books tie out.
          Do not credit net-to-checking to another book. External remains the live desk for other
          accounts; this item is backlog only.
        </p>
      </article>
      <article aria-label="Inter-account transfers">
        <h3>Inter-account transfers</h3>
        <p>
          Household budget moves and transfers between managed books are not scoped for v1 cash.
          Keep them on Roadmap until a later lock names the transfer journal and register books.
        </p>
      </article>
    </section>
  );
}
