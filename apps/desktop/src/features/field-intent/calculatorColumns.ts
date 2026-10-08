/** Calculator column contract. Sentences stay with this screen, not in App.tsx. */

export type ColumnIntentStatus = "matches" | "still wrong";

/** Excel / display kind. Week-date columns are cash (decl per share). */
export type CalculatorColumnKind = "cash" | "percent" | "shares" | "text" | "count";

export type CalculatorColumnIntent = {
  name: string;
  kind: CalculatorColumnKind;
  intent: string;
  formula: string;
  status: ColumnIntentStatus;
};

export const CALCULATOR_COLUMN_INTENT: readonly CalculatorColumnIntent[] = [
  {
    name: "Symbol",
    kind: "text",
    intent:
      "Stored position key. This row documents that position. Not a lot, not an account, not a 60-day slice. The button opens that position.",
    formula: "position.symbol",
    status: "matches",
  },
  {
    name: "Price",
    kind: "cash",
    intent:
      "Last accepted price of one share, shared with the rest of the app. Cash symbols are $1.00. A failed quote does not become $0.",
    formula: "acceptedPrice. Cash symbol → 1.00.",
    status: "matches",
  },
  {
    name: "Port %",
    kind: "percent",
    intent:
      "This position's open-lot market value as a percent of household market value, on the same price set.",
    formula: "positionMarketValue / householdMarketValue",
    status: "matches",
  },
  {
    name: "Risk",
    kind: "text",
    intent: "Stored risk tier for this position. Canonical values are Foundation, Core, Risk On.",
    formula: "position.riskTier, else —",
    status: "matches",
  },
  {
    name: "Account",
    kind: "text",
    intent: "Books that still hold this symbol, so the household row can be traced to open lots.",
    formula: "distinct account names where remaining shares > 0",
    status: "matches",
  },
  {
    name: "Shares",
    kind: "shares",
    intent: "Shares still open on this position. Denominator for plan pay and current pay.",
    formula: "sum of open-lot remaining quantity",
    status: "matches",
  },
  {
    name: "Cost",
    kind: "cash",
    intent:
      "Remaining original economic cost of the open shares. Not tax basis. ROC does not reduce it. A stored zero stays zero.",
    formula: "sum of remaining original economic cost",
    status: "matches",
  },
  {
    name: "Avg px",
    kind: "cash",
    intent: "Average original unit cost of the shares still open.",
    formula: "cost / shares",
    status: "matches",
  },
  {
    name: "Gain $",
    kind: "cash",
    intent: "Unrealized gain on original cost, using the row's price.",
    formula: "marketValue − cost",
    status: "matches",
  },
  {
    name: "Gain %",
    kind: "percent",
    intent: "Unrealized gain as a percent of original cost. One input to legacy Calculator TVAL.",
    formula: "(marketValue − cost) / cost",
    status: "matches",
  },
  {
    name: "YOC",
    kind: "percent",
    intent: "Plan yield on original cost. Forward plan, not cash received and not the 60-day grid.",
    formula: "(planPerShare × periodsPerYear) / average original unit cost",
    status: "matches",
  },
  {
    name: "FWD",
    kind: "percent",
    intent: "Forward yield on price from the in-force Plan. Not the latest dividend and not the visible weeks.",
    formula: "(planPerShare × periodsPerYear) / lastPrice",
    status: "matches",
  },
  {
    name: "MC FWD",
    kind: "percent",
    intent:
      "Forward yield on price from the newest in-force issuer declaration. Not broker cash. Not the older end of the 60-day columns.",
    formula: "(newestDeclarationPerShare × periodsPerYear) / lastPrice",
    status: "matches",
  },
  {
    name: "Annual",
    kind: "cash",
    intent: "Full-year Plan dollars on the open shares. This is what planning inherits.",
    formula: "planPerShare × shares × periodsPerYear",
    status: "matches",
  },
  {
    name: "Plan",
    kind: "cash",
    intent:
      "Owner-set dollars per share per period. Never a formula. Never auto-posted from Most Current or a collector.",
    formula: "effective PlanHistory amount per share, else N/A",
    status: "matches",
  },
  {
    name: "Type",
    kind: "text",
    intent: "Stored dividend type. Calculator lists DIV-1 only. Empty is a ticket, not a guess.",
    formula: "position.divType, else —",
    status: "still wrong",
  },
  {
    name: "Sched",
    kind: "text",
    intent: "Stored payment frequency. It drives periods per year. Not inferred from the 60-day window.",
    formula: "position.paymentFrequency, else —",
    status: "matches",
  },
  {
    name: "Div recv",
    kind: "cash",
    intent:
      "Broker cash received for this symbol, all accounts, all loaded years. Not the history grid and not the Period window.",
    formula: "sum of ledger dividend amounts for this symbol",
    status: "matches",
  },
  {
    name: "ROC $",
    kind: "cash",
    intent:
      "Current-year Car dividend cash times the ROC percent shown in ROC %. Other accounts are excluded. A missing percent is not 0%.",
    formula: "carDividendsThisYear × rocPercentDisplayed",
    status: "matches",
  },
  {
    name: "Cost rec",
    kind: "percent",
    intent: "Broker dividends divided by original economic cost. ROC does not reduce the denominator.",
    formula: "totalDividendsReceived / originalEconomicCost",
    status: "matches",
  },
  {
    name: "ROC %",
    kind: "percent",
    intent:
      "Percent used for ROC $, in that same order. Label year and actual vs estimate. Stored zero is allowed only when a notice or the owner stored zero.",
    formula: "current-year actual, else current-year estimate, else prior-year actual",
    status: "matches",
  },
  {
    name: "Declares",
    kind: "text",
    intent: "Stored declaration weekday. Not calculated from pay dates.",
    formula: "position.declarationWeekday, else —",
    status: "matches",
  },
  {
    name: "Ex-date",
    kind: "text",
    intent: "Stored ex-date weekday. Not inferred.",
    formula: "position.exdateWeekday, else —",
    status: "matches",
  },
  {
    name: "Payday",
    kind: "text",
    intent:
      "Stored payday weekday. The history grid still uses the expected Sat–Fri pay week, not this label.",
    formula: "position.paydayWeekday, else —",
    status: "matches",
  },
  {
    name: "Decl freshness",
    kind: "text",
    intent:
      "Whether the declaration for the current expected pay week is in, not yet due, or missing after that week closed.",
    formula:
      "not due, or current, or stale, or unknown, from expected pay week vs in-force declarations",
    status: "matches",
  },
  {
    name: "Decl count",
    kind: "count",
    intent: "How many issuer declarations are still in force, all dates.",
    formula: "count of in-force declarations",
    status: "matches",
  },
  {
    name: "Ann / share",
    kind: "cash",
    intent: "Plan annualized to dollars per share.",
    formula: "planPerShare × periodsPerYear",
    status: "matches",
  },
  {
    name: "Recent / share",
    kind: "cash",
    intent: "Newest in-force issuer declaration, annualized. Not an older week column and not broker cash.",
    formula: "newestDeclarationPerShare × periodsPerYear",
    status: "matches",
  },
  {
    name: "Recent total",
    kind: "cash",
    intent: "Newest declaration annualized, times open shares.",
    formula: "recentPerShare × shares",
    status: "matches",
  },
  {
    name: "Realized %",
    kind: "percent",
    intent: "Same measure as Cost rec. One calculation, two labels.",
    formula: "same as Cost rec",
    status: "matches",
  },
  {
    name: "Calculator blend",
    kind: "percent",
    intent:
      "Legacy Calculator TVAL V3. Blend of unrealized gain and Plan yield on price. Not Holdings TVAL. TVAL 2.1 is not this column.",
    formula: "(Gain% × 2 + FWD) / 2",
    status: "still wrong",
  },
  {
    name: "MC TVAL",
    kind: "percent",
    intent: "Same blend, using the newest-declaration yield.",
    formula: "(Gain% × 2 + MC FWD) / 2",
    status: "still wrong",
  },
  {
    name: "TVAL Δ",
    kind: "percent",
    intent: "How far the declaration score has moved from the Plan score.",
    formula: "MC TVAL − TVAL",
    status: "still wrong",
  },
  {
    name: "3-pay yield",
    kind: "percent",
    intent:
      "Yield on price from the three newest in-force issuer declarations. Not a screen-window statistic.",
    formula: "(mean of the 3 newest in-force declarations × periodsPerYear) / lastPrice",
    status: "matches",
  },
  {
    name: "Plan Δ",
    kind: "cash",
    intent:
      "Newest declaration versus Plan, in dollars for the open shares. The owner uses this to decide whether to edit Plan.",
    formula: "(newestDeclarationPerShare − planPerShare) × shares",
    status: "matches",
  },
  {
    name: "Over/Under",
    kind: "percent",
    intent: "Same comparison as a percent of Plan.",
    formula: "(newestDeclarationPerShare − planPerShare) / planPerShare",
    status: "matches",
  },
  {
    name: "Plan pay",
    kind: "cash",
    intent: "One period of Plan dollars on the open shares. Not the annual figure.",
    formula: "planPerShare × shares",
    status: "matches",
  },
  {
    name: "Current pay",
    kind: "cash",
    intent: "One period of the newest issuer declaration on the open shares.",
    formula: "newestDeclarationPerShare × shares",
    status: "matches",
  },
  {
    name: "Most current",
    kind: "cash",
    intent:
      "Newest in-force issuer declaration per share. A blank is not a declaration. An explicit stored zero is a declaration and counts.",
    formula: "latest in-force declaration.amountPerShare by payDate",
    status: "matches",
  },
  {
    name: "Avg 3",
    kind: "cash",
    intent:
      "Mean of the three newest complete in-force declarations at scale 5 (0.00000). Computed once on DeclarationHistory. Week columns are not an input. Fewer than three stays unknown with the count.",
    formula: "DeclarationHistory.avg3Minor / 10^avg3Scale",
    status: "matches",
  },
  {
    name: "Avg 6",
    kind: "cash",
    intent:
      "Mean of up to six newest complete in-force declarations at scale 5 (0.00000). Computed once on DeclarationHistory. Show n of 6. Week columns are not an input.",
    formula: "DeclarationHistory.avg6Minor / 10^avg6Scale",
    status: "matches",
  },
  {
    name: "Paid",
    kind: "cash",
    intent:
      "Sum of issuer declarations in the week columns the owner is looking at. A view total only. Not lifetime cash and not an average input.",
    formula: "sum of declaration per share in the visible Sat–Fri weeks",
    status: "matches",
  },
  {
    name: "Plan check",
    kind: "text",
    intent:
      "Count in-force declarations above, equal, and below Plan. Blanks are not counted. Green only when at least one is counted and none is below.",
    formula: "above / equal / below vs planPerShare; percent = above / counted",
    status: "matches",
  },
  {
    name: "Week columns (dated)",
    kind: "cash",
    intent:
      "Declaration history grid. Each header is one Saturday–Friday expected pay week. The cell is the issuer declaration per share whose pay date falls in that week. Newest week on the left.",
    formula: "in-force declaration per share whose pay date is in that Saturday–Friday week",
    status: "matches",
  },
];
