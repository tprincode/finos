/** Contract field intent. One row per control. Page and component are the key. */

export type ContractFieldIntent = {
  name: string;
  intent: string;
  formula: string;
  status: "matches";
  pageId: string;
  componentId: string;
  owner: string;
};

const pageId = "contract-positions";

function row(
  name: string,
  componentId: string,
  owner: string,
  intent: string,
  formula: string,
): ContractFieldIntent {
  return { name, intent, formula, status: "matches", pageId, componentId, owner };
}

export const CONTRACT_FIELD_INTENT: readonly ContractFieldIntent[] = [
  row(
    "OCC symbol",
    "contract-create",
    "Create contract",
    "The pasted OCC symbol. Call or put is read from it.",
    "parseOcc(symbol)",
  ),
  row(
    "side",
    "contract-create",
    "Create contract",
    "Long, or Short Cover. The stored value stays long or short. It does not choose call or put.",
    "side",
  ),
  row(
    "call or put",
    "contract-create",
    "Create contract",
    "Read from the OCC symbol. Not a second control.",
    "putCall",
  ),
  row(
    "quantity",
    "contract-create",
    "Create contract",
    "Number of contracts. A short call reserves 100 shares each.",
    "quantity",
  ),
  row(
    "account",
    "contract-create",
    "Create contract",
    "Income, Speculation, or Account 9. Other names are refused.",
    "account.name",
  ),
  row(
    "fragment list",
    "contract-create",
    "Create contract",
    "Cheapest free shares in that account that cover this short call.",
    "option_cover_lot",
  ),
  row(
    "block average",
    "contract-create",
    "Create contract",
    "Total reserved cost divided by reserved shares.",
    "sum(cost) / sum(shares)",
  ),
  row(
    "open premium",
    "contract-create",
    "Create contract",
    "Optional cash for this leg. Blank posts nothing and is not $0. A typed short credit posts in the week it arrives. A typed long debit posts in the week it leaves.",
    "open_premium_minor",
  ),
  row(
    "Previously realized P/L $",
    "contract-create",
    "Create contract",
    "Optional history typed at entry. It posts to no week.",
    "prior_balance_minor",
  ),
  row(
    "open date",
    "contract-create",
    "Create contract",
    "The day the open cash moved. Week is Saturday through Friday.",
    "open_on",
  ),
  row(
    "live underlying",
    "contract-open",
    "Open contracts",
    "Last accepted price of the underlying. A missing price is the word unknown.",
    "price_quote last accepted",
  ),
  row(
    "strike",
    "contract-open",
    "Open contracts",
    "Strike from the OCC symbol, in cents.",
    "strike_minor",
  ),
  row(
    "DTE",
    "contract-open",
    "Open contracts",
    "Calendar days from today to expiry.",
    "expiry − today",
  ),
  row(
    "ITM",
    "contract-open",
    "Open contracts",
    "A short call is in the money when last is at or above the strike.",
    "last >= strike",
  ),
  row(
    "assign risk",
    "contract-open",
    "Open contracts",
    "In the money and seven or fewer days to expiry.",
    "ITM and DTE <= 7",
  ),
  row(
    "period %",
    "contract-open",
    "Open contracts",
    "This leg's open premium divided by strike times 100 times quantity.",
    "open premium / (strike × 100 × quantity)",
  ),
  row(
    "annual",
    "contract-open",
    "Open contracts",
    "The period percent compounded from the open date to expiry.",
    "compound(period, open, expiry)",
  ),
  row(
    "week category",
    "contract-open",
    "Open contracts",
    "Covered Call, Call, or Put. The week is the week the cash moved.",
    "week_category(side, putCall)",
  ),
  row(
    "option result",
    "contract-closed",
    "Closed contracts",
    "Realized option premium in the week the close cash moved. A short result is open minus close.",
    "open premium − close premium",
  ),
  row(
    "stock result",
    "contract-closed",
    "Closed contracts",
    "On assignment, strike minus the reserved piece cost, times shares. Separate from the option result.",
    "strike − piece cost",
  ),
];
