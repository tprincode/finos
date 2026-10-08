import { AccountSelect, type AccountOption } from "../shared/pickers/AccountSelect";
import { ResearchedSymbolCombobox } from "../shared/pickers/ResearchedSymbolCombobox";

const LOT_ORIGINS = ["purchase", "drip", "transfer"] as const;

export type AddLotScreenProps = {
  busy: boolean;
  writesBlocked: boolean;
  addLotDirty: boolean;
  addLotSecurityId: string;
  addLotAccountId: string;
  addLotOpenedOn: string;
  addLotQty: string;
  addLotCost: string;
  addLotTaxCost: string;
  addLotTaxDifferent: boolean;
  addLotTaxEquals: boolean;
  addLotOrigin: string;
  addLotQuery: string;
  addLotSymbolOpen: boolean;
  accounts: AccountOption[];
  securitiesLength: number;
  filteredAddLotSecurities: Array<{
    securityId: string;
    symbol: string;
    name: string;
  }>;
  formatScaled: (minor: number, scale: number) => string;
  dollarsToMinor: (raw: string) => number;
  lotTotalFromUnitCents: (qtyMinor: number, quantityScale: number, unitCents: number) => number;
  formatMoneyInput: (raw: string) => string;
  openAddLot: () => void | Promise<void>;
  cancelAddLotEdits: () => void;
  setAddLotQuery: (q: string) => void;
  setAddLotSecurityId: (id: string) => void;
  setAddLotSymbolOpen: (open: boolean) => void;
  selectAddLotSecurity: (row: { securityId: string; symbol: string; name: string }) => void;
  setAddLotAccountId: (id: string) => void;
  setAddLotOpenedOn: (v: string) => void;
  setAddLotQty: (v: string) => void;
  setAddLotCost: (v: string) => void;
  setAddLotTaxDifferent: (v: boolean) => void;
  setAddLotTaxEquals: (v: boolean) => void;
  setAddLotTaxCost: (v: string) => void;
  setAddLotOrigin: (v: string) => void;
};

export function AddLotScreen(props: AddLotScreenProps) {
  const {
    busy,
    writesBlocked,
    addLotDirty,
    addLotSecurityId,
    addLotAccountId,
    addLotOpenedOn,
    addLotQty,
    addLotCost,
    addLotTaxCost,
    addLotTaxDifferent,
    addLotTaxEquals,
    addLotOrigin,
    addLotQuery,
    addLotSymbolOpen,
    accounts,
    securitiesLength,
    filteredAddLotSecurities,
    formatScaled,
    dollarsToMinor,
    lotTotalFromUnitCents,
    formatMoneyInput,
    openAddLot,
    cancelAddLotEdits,
    setAddLotQuery,
    setAddLotSecurityId,
    setAddLotSymbolOpen,
    selectAddLotSecurity,
    setAddLotAccountId,
    setAddLotOpenedOn,
    setAddLotQty,
    setAddLotCost,
    setAddLotTaxDifferent,
    setAddLotTaxEquals,
    setAddLotTaxCost,
    setAddLotOrigin,
  } = props;

  const carBuy =
    accounts.find((a) => a.accountId === addLotAccountId)?.name === "Car";
  const carTaxReady = !carBuy || addLotTaxEquals || !!addLotTaxCost.trim();

  return (
    <section aria-label="Add Lot">
      <h2>Add Lot</h2>
      <p>
        Confirm writes this lot. A cart purchase returns to Shopping Cart. The symbol must
        already be on Position Details. No FIFO. No collector from this screen.
      </p>
      {addLotDirty ? (
        <p className="blocked" role="status">
          Unsaved edits. Confirm or Cancel — other screens stay blocked.
        </p>
      ) : null}
      <div
        className="buttons dossier-actions"
        id="add-lot-actions"
        data-section="add-lot-actions"
        data-part="add-lot-actions"
      >
        <button
          type="button"
          aria-label="Confirm add lot"
          className={addLotDirty ? "is-unsaved" : undefined}
          disabled={
            busy ||
            writesBlocked ||
            !addLotSecurityId ||
            !addLotAccountId ||
            !addLotOpenedOn.trim() ||
            !addLotQty.trim() ||
            !addLotCost.trim() ||
            (addLotTaxDifferent && !addLotTaxCost.trim()) ||
            !carTaxReady
          }
          onClick={() => void openAddLot()}
        >
          Confirm
        </button>
        <button
          type="button"
          aria-label="Cancel add lot edits"
          disabled={busy || !addLotDirty}
          onClick={() => cancelAddLotEdits()}
        >
          Cancel
        </button>
      </div>
      <div className="form-grid" id="add-lot-form" data-section="add-lot-form">
        <div data-part="add-lot-symbol">
          <ResearchedSymbolCombobox
            options={filteredAddLotSecurities}
            query={addLotQuery}
            onQueryChange={(q) => {
              setAddLotQuery(q);
              setAddLotSecurityId("");
            }}
            open={addLotSymbolOpen}
            onOpenChange={setAddLotSymbolOpen}
            selectedId={addLotSecurityId}
            onSelect={selectAddLotSecurity}
            disabled={busy || writesBlocked}
            inputAriaLabel="Add lot symbol"
            listId="add-lot-symbol-list"
            listAriaLabel="Add lot symbol matches"
          />
        </div>
        <div data-part="add-lot-account">
          <AccountSelect
            accounts={accounts}
            value={addLotAccountId}
            onChange={setAddLotAccountId}
            ariaLabel="Add lot account"
            disabled={busy || writesBlocked}
          />
        </div>
        <label data-part="add-lot-opened-on">
          Opened on
          <input
            aria-label="Add lot opened on"
            type="date"
            value={addLotOpenedOn}
            onChange={(e) => setAddLotOpenedOn(e.target.value)}
            disabled={busy || writesBlocked}
          />
        </label>
        <label data-part="add-lot-quantity">
          Quantity
          <input
            aria-label="Add lot quantity"
            value={addLotQty}
            onChange={(e) => setAddLotQty(e.target.value)}
            disabled={busy || writesBlocked}
          />
        </label>
        <label data-part="add-lot-unit-cost">
          Unit original $
          <input
            aria-label="Add lot unit original cost"
            value={addLotCost}
            onChange={(e) => setAddLotCost(e.target.value)}
            disabled={busy || writesBlocked}
            placeholder="29.46"
          />
        </label>
        <label data-part="add-lot-tax-cost">
          {carBuy ? (
            <span>
              <input
                type="checkbox"
                aria-label="Tax equals performance"
                checked={addLotTaxEquals}
                onChange={(e) => {
                  setAddLotTaxEquals(e.target.checked);
                  if (e.target.checked) {
                    setAddLotTaxDifferent(false);
                    setAddLotTaxCost("");
                  }
                }}
                disabled={busy || writesBlocked}
              />{" "}
              Tax equals performance
            </span>
          ) : (
            <span>
              <input
                type="checkbox"
                aria-label="Tax cost different"
                checked={addLotTaxDifferent}
                onChange={(e) => setAddLotTaxDifferent(e.target.checked)}
                disabled={busy || writesBlocked}
              />{" "}
              Unit tax $ different
            </span>
          )}
          <input
            aria-label="Add lot unit tax cost"
            value={
              carBuy
                ? addLotTaxEquals
                  ? addLotCost
                  : addLotTaxCost
                : addLotTaxDifferent
                  ? addLotTaxCost
                  : addLotCost
            }
            onChange={(e) => setAddLotTaxCost(e.target.value)}
            disabled={
              busy ||
              writesBlocked ||
              (carBuy ? addLotTaxEquals : !addLotTaxDifferent)
            }
            placeholder={carBuy && !addLotTaxEquals ? "required" : undefined}
          />
        </label>
        {(() => {
          const qty = Number(addLotQty);
          const unit = Number(addLotCost);
          if (
            !Number.isFinite(qty) ||
            qty <= 0 ||
            !Number.isFinite(unit) ||
            unit < 0 ||
            !addLotQty.trim() ||
            !addLotCost.trim()
          ) {
            return (
              <p data-part="add-lot-total" role="status" aria-label="Add lot total confirmation">
                Enter qty and unit original $ — lot total = qty × unit.
              </p>
            );
          }
          const totalCents = lotTotalFromUnitCents(qty, 0, dollarsToMinor(addLotCost));
          const totalLabel = (totalCents / 100).toLocaleString("en-US", {
            minimumFractionDigits: 2,
            maximumFractionDigits: 2,
          });
          const unitLabel = formatMoneyInput(addLotCost);
          return (
            <p role="status" aria-label="Add lot total confirmation">
              Confirm: {formatScaled(qty, 0)} × ${unitLabel} = ${totalLabel} lot original
              (saved as performance basis total).
              {carBuy
                ? addLotTaxEquals
                  ? " Tax equals performance."
                  : addLotTaxCost.trim()
                    ? (() => {
                        const taxUnit = Number(addLotTaxCost);
                        if (!Number.isFinite(taxUnit) || taxUnit < 0) return "";
                        const taxTotal = lotTotalFromUnitCents(
                          qty,
                          0,
                          dollarsToMinor(addLotTaxCost),
                        );
                        return ` Tax: ${formatScaled(qty, 0)} × $${formatMoneyInput(addLotTaxCost)} = $${(taxTotal / 100).toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 })}.`;
                      })()
                    : " Enter unit tax $, or check Tax equals performance."
                : addLotTaxDifferent && addLotTaxCost.trim()
                  ? (() => {
                      const taxUnit = Number(addLotTaxCost);
                      if (!Number.isFinite(taxUnit) || taxUnit < 0) return "";
                      const taxTotal = lotTotalFromUnitCents(
                        qty,
                        0,
                        dollarsToMinor(addLotTaxCost),
                      );
                      return ` Tax: ${formatScaled(qty, 0)} × $${formatMoneyInput(addLotTaxCost)} = $${(taxTotal / 100).toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 })}.`;
                    })()
                  : ""}
            </p>
          );
        })()}
        <label data-part="add-lot-origin">
          Origin
          <select
            aria-label="Add lot origin"
            value={addLotOrigin}
            onChange={(e) => setAddLotOrigin(e.target.value)}
            disabled={busy || writesBlocked}
          >
            {LOT_ORIGINS.map((o) => (
              <option key={o} value={o}>
                {o}
              </option>
            ))}
          </select>
        </label>
      </div>
      {securitiesLength === 0 ? (
        <p role="status">
          No securities yet. Use Add Investment (symbol + distribution URL), then return here
          to open a lot. Researched names with zero lots are included.
        </p>
      ) : addLotQuery.trim() && !addLotSecurityId && filteredAddLotSecurities.length === 0 ? (
        <p role="status">
          No matching researched symbol. Cannot create a new ticker here — research it on
          Add Investment first.
        </p>
      ) : null}
    </section>
  );
}
