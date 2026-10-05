import { SymbolWindowTable, type WindowClient } from "./SymbolWindowRow";

export function MarketImpactPlanner({
  client,
  discardEpoch = 0,
  onDirtyChange,
  onOpenSymbol,
}: {
  client: WindowClient;
  discardEpoch?: number;
  onDirtyChange?: (dirty: boolean) => void;
  onOpenSymbol?: (symbol: string) => void;
}) {
  return (
    <section aria-label="Market impact planner">
      <h2>Market impact planner</h2>
      <p>
        One row per Calculator symbol. Bull and Bear dates are that symbol’s owner
        periods. Bear return is the price change in the bear window; a higher number
        lost less. Bull return is the price change in the bull window; a higher number
        made more.
      </p>
      <SymbolWindowTable
        client={client}
        discardEpoch={discardEpoch}
        onDirtyChange={onDirtyChange}
        onOpenSymbol={onOpenSymbol}
      />
    </section>
  );
}
