import type {
  CalculatorGet,
  HoldingsGet,
  HoldingsUnassignedSell,
} from "@finos/app-contracts";
import { HoldingsPanel } from "@finos/ui-components";
import {
  LotCostTable,
  UnassignedSellTable,
  type LotSortMode,
} from "../shared/pickers";

type HoldingsLot = HoldingsGet["lots"][number];

export type HoldingsScreenProps = {
  busy: boolean;
  writesBlocked: boolean;
  holdings: HoldingsGet | null;
  holdingsFilter: string;
  setHoldingsFilter: (v: string) => void;
  openPositionHub: (symbol: string, focus?: "lots" | "income" | "declarations" | "ledger" | "") => void;
  lotId: string;
  setLotId: (id: string) => void;
  setAssignSellId: (id: string) => void;
  setAssignQty: (q: string) => void;
  assignSellId: string;
  assignQty: string;
  calculator: CalculatorGet | null;
  holdingsLotSort: LotSortMode;
  setHoldingsLotSort: (mode: LotSortMode) => void;
  selectedLot: HoldingsLot | undefined;
  selectedSell: HoldingsUnassignedSell | undefined;
  lotUnassignedSells: HoldingsUnassignedSell[];
  runCommand: (name: string, body: Record<string, unknown>) => void | Promise<void>;
};

export function HoldingsScreen({
  busy,
  writesBlocked,
  holdings,
  holdingsFilter,
  setHoldingsFilter,
  openPositionHub,
  lotId,
  setLotId,
  setAssignSellId,
  setAssignQty,
  assignSellId,
  assignQty,
  calculator,
  holdingsLotSort,
  setHoldingsLotSort,
  selectedLot,
  selectedSell,
  lotUnassignedSells,
  runCommand,
}: HoldingsScreenProps) {
  return (
    <section aria-label="Holdings">
      <h2>Holdings</h2>
      <p>Open lots. Owner assigns sales; no FIFO.</p>
      <label>
        Filter holdings
        <input
          value={holdingsFilter}
          onChange={(e) => setHoldingsFilter(e.target.value)}
          aria-label="Filter holdings"
        />
      </label>
      <HoldingsPanel
        lots={holdings?.lots ?? null}
        filter={holdingsFilter}
        onOpenSymbol={(symbol) => openPositionHub(symbol, "lots")}
      />
      <p>
        Cart Confirm sell assigns the lot in the same step. This list is leftover posted
        sells only. Owner assigns; no FIFO.
      </p>
      <LotCostTable
        lots={(holdings?.lots ?? []).map((l) => ({
          lotId: l.lotId,
          symbol: l.symbol,
          accountName: l.accountName,
          remainingQuantityMinor: l.remainingQuantityMinor,
          quantityScale: l.quantityScale,
          openedOn: l.openedOn,
          remainingPerformanceMinor: l.remainingPerformanceMinor,
          remainingTaxMinor: l.remainingTaxMinor,
          scale: l.scale,
        }))}
        value={lotId}
        onChange={(nextLotId) => {
          setLotId(nextLotId);
          setAssignSellId("");
          const nextLot = holdings?.lots.find((lot) => lot.lotId === nextLotId);
          if (nextLot) {
            setAssignQty(String(nextLot.remainingQuantityMinor));
          }
        }}
        lastBySymbol={Object.fromEntries(
          (calculator?.rows ?? []).map((r) => [r.symbol, r.lastPriceMinor]),
        )}
        sortMode={holdingsLotSort}
        onSortModeChange={setHoldingsLotSort}
        disabled={busy || writesBlocked}
        ariaLabel="Open lots"
      />
      {selectedLot ? (
        <>
          {lotUnassignedSells.length === 0 ? (
            <p>No unassigned sell for this symbol</p>
          ) : (
            <UnassignedSellTable
              sells={lotUnassignedSells}
              value={assignSellId}
              onChange={(activityId) => {
                setAssignSellId(activityId);
                const sell = lotUnassignedSells.find((row) => row.activityId === activityId);
                const qty = sell?.quantityMinor ?? selectedLot.remainingQuantityMinor;
                setAssignQty(String(qty));
              }}
              disabled={busy || writesBlocked}
            />
          )}
          <label>
            Quantity
            <input
              value={assignQty}
              onChange={(e) => setAssignQty(e.target.value)}
              disabled={busy || writesBlocked || !selectedSell}
            />
          </label>
        </>
      ) : (
        <p>Pick an open lot, then a leftover sell.</p>
      )}
      <button
        type="button"
        aria-label="Assign lot"
        disabled={busy || writesBlocked || !lotId || !selectedSell}
        onClick={() => {
          if (!lotId || !selectedSell) return;
          void runCommand("LotAssign", {
            lotId,
            activityId: selectedSell.activityId,
            quantityMinor: Number(assignQty),
            quantityScale: selectedLot?.quantityScale ?? 0,
          });
        }}
      >
        Assign lot
      </button>
    </section>
  );
}
