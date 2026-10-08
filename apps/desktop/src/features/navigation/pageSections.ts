/**
 * Section shortcuts for the page nav row. The same set the component catalog
 * carries as `sections`, proven equal by `ui_modules`.
 *
 * Named `pageSections`, not `sectionNav`: Windows resolves a case-insensitive
 * import, so a `sectionNav.ts` beside `SectionNav.tsx` loads the wrong file.
 */

export type NavSection = {
  /** Catalog section id. */
  id: string;
  /** Short label for the row. The owner reads these, not the ids. */
  label: string;
  /** Element id already on the page. Shipped deep links keep using it. */
  anchor: string;
};

/** Key is the App `Screen` id, or `screen/cmDesk` for a Cash Management desk. */
export const SCREEN_SECTIONS: Record<string, readonly NavSection[]> = {
  "position-details": [
    { id: "plan-management", label: "Plan Management", anchor: "hub-plan" },
    { id: "research-notes", label: "Research notes", anchor: "hub-research" },
    { id: "work-tickets", label: "Work tickets", anchor: "hub-tickets" },
    { id: "position-information", label: "Position information", anchor: "hub-identity" },
    { id: "calculator-snapshot", label: "Calculator", anchor: "hub-calculator" },
    { id: "plan-and-yields", label: "Plan", anchor: "hub-plan-yields" },
    { id: "holdings-by-account", label: "By account", anchor: "hub-accounts" },
    { id: "identity-and-economics", label: "Identity facts", anchor: "hub-facts" },
    { id: "declarations", label: "Declarations", anchor: "hub-declarations" },
    { id: "plan-payment-summary", label: "Payment summary", anchor: "hub-income" },
    { id: "pay-dates", label: "Pay dates", anchor: "hub-pay-dates" },
    { id: "received-payments", label: "Received", anchor: "hub-received" },
    { id: "ledger-income", label: "Ledger", anchor: "hub-ledger" },
    { id: "lots-by-account", label: "Lots", anchor: "hub-lots" },
    { id: "owner-period", label: "Owner period", anchor: "hub-periods" },
  ],
  "home": [
    { id: "home-overview", label: "Overview", anchor: "home-top-row" },
    { id: "home-account-values", label: "Account values", anchor: "home-account-values" },
    { id: "home-account-charts", label: "Charts", anchor: "home-account-charts" },
  ],
  "trends": [
    { id: "trends-period", label: "Period", anchor: "trends-charts" },
    { id: "trends-declared", label: "Declared vs Plan", anchor: "trends-declared" },
    { id: "trends-weekly-income", label: "Weekly income", anchor: "trends-weekly-income" },
    { id: "trends-accounts", label: "Accounts", anchor: "trends-accounts" },
    { id: "trends-risk", label: "Risk", anchor: "trends-risk" },
    { id: "trends-dividends-by-month", label: "Dividends by month", anchor: "trends-dividends-by-month" },
  ],
  "income-plan": [
    { id: "income-plan-controls", label: "Controls", anchor: "income-plan-page" },
    { id: "income-week-grid", label: "Week grid", anchor: "income-week-grid" },
    { id: "income-dividend-weeks", label: "Plan vs Decl", anchor: "income-dividend-weeks" },
  ],
  "calculator": [
    { id: "calculator-sheet", label: "Calculator", anchor: "calculator-page" },
  ],
  "dashboard": [
    { id: "dashboard-burndown", label: "Burndown", anchor: "dashboard-burndown" },
  ],
  "cash-management/weekly": [
    { id: "cash-week-desk", label: "Week desk", anchor: "cash-week-desk" },
    { id: "week-ahead", label: "Week ahead", anchor: "week-ahead" },
    { id: "week-activity", label: "Activity", anchor: "cash-week-activity" },
    { id: "week-selected", label: "This week", anchor: "cash-week-selected" },
    { id: "week-distributions", label: "Distributions", anchor: "cash-week-distributions" },
    { id: "week-tax-monitor", label: "Tax monitor", anchor: "cash-week-tax-monitor" },
  ],
  "cash-management/car": [
    { id: "magi-forecast-tiles", label: "MAGI forecast", anchor: "magi-forecast" },
    { id: "magi-application", label: "Application", anchor: "magi-application" },
    { id: "household-income-detail", label: "Income", anchor: "household-income-detail" },
    { id: "magi-estimates", label: "Estimates", anchor: "magi-estimates" },
    { id: "household-deductions", label: "Deductions", anchor: "household-deductions" },
    { id: "tax-money", label: "Tax money", anchor: "tax-money" },
    { id: "magi-suggestions", label: "Suggestions", anchor: "magi-suggestions" },
    { id: "household-aptc", label: "APTC", anchor: "household-aptc" },
    { id: "car-lot-detail", label: "Car lot detail", anchor: "car-lot-detail" },
  ],
  "cash-management/elements": [
    { id: "elements-catalog", label: "Elements", anchor: "cash-elements" },
  ],
  "cash-management/cashflow": [
    { id: "register-controls", label: "Totals", anchor: "cash-register" },
    { id: "register-trend", label: "Trend", anchor: "register-trend" },
    { id: "register-calendar", label: "Calendar", anchor: "register-calendar" },
    { id: "register-day", label: "Day", anchor: "register-day" },
  ],
  "cash-management/coverage": [
    { id: "coverage-summary", label: "Plan", anchor: "cash-coverage" },
    { id: "coverage-income-math", label: "Income math", anchor: "coverage-income-math" },
    { id: "coverage-expense-math", label: "Expense math", anchor: "coverage-expense-math" },
  ],
  "cash-management/external": [
    { id: "external-find", label: "Search", anchor: "external-find" },
    { id: "external-open", label: "Open", anchor: "external-open" },
    { id: "external-completed", label: "Completed", anchor: "external-completed" },
  ],
  "cash-management/manager": [
    { id: "debt-accounts-list", label: "Loans", anchor: "managed-accounts" },
    { id: "bucket-manager", label: "Buckets", anchor: "debt-bucket-panel" },
    { id: "loan-vendor", label: "Loan vendor", anchor: "loan-vendor" },
    { id: "loan-register", label: "Our transactions", anchor: "managed-register" },
  ],
  "holdings": [
    { id: "holdings-list", label: "Open lots", anchor: "holdings-list" },
    { id: "holdings-assign", label: "Assign", anchor: "holdings-assign" },
  ],
  "add-lot": [
    { id: "add-lot-actions", label: "Actions", anchor: "add-lot-actions" },
    { id: "add-lot-form", label: "Lot details", anchor: "add-lot-form" },
  ],
  "market-impact": [
    { id: "market-impact-planner", label: "Windows", anchor: "market-impact-planner" },
  ],
  "new-investment": [
    { id: "add-investment-gaps", label: "Gaps", anchor: "add-investment-gaps" },
    { id: "add-investment-saved", label: "Research saved", anchor: "add-investment-saved" },
    { id: "add-investment-inputs", label: "Inputs", anchor: "add-investment-inputs" },
    { id: "add-investment-progress", label: "Progress", anchor: "add-investment-progress" },
    { id: "add-investment-results", label: "Results", anchor: "add-investment-results" },
    { id: "add-investment-checklist", label: "Checklist", anchor: "add-investment-checklist" },
    { id: "establish-checklist", label: "Establish checklist", anchor: "establish-checklist" },
    { id: "add-investment-notes", label: "Notes", anchor: "add-investment-notes" },
    { id: "add-investment-owner-actions", label: "Owner actions", anchor: "add-investment-owner-actions" },
    { id: "add-investment-roc", label: "ROC", anchor: "add-investment-roc" },
  ],
  "shopping-cart": [
    { id: "cart-start", label: "Start", anchor: "cart-start" },
    { id: "cart-plan", label: "Plan", anchor: "cart-plan" },
    { id: "cart-execute", label: "Execute", anchor: "cart-execute" },
    { id: "cart-sell", label: "Sell", anchor: "cart-sell" },
    { id: "cart-buy", label: "Buy", anchor: "cart-buy" },
    { id: "cart-critique", label: "Critique", anchor: "cart-critique" },
    { id: "cart-actions", label: "Actions", anchor: "cart-actions" },
  ],
  "interest-rate": [
    { id: "interest-period", label: "Period conversion", anchor: "interest-period" },
    { id: "interest-contract", label: "Contract or premium", anchor: "interest-contract" },
  ],
  "account-management": [
    { id: "accounts-tabs", label: "Lists", anchor: "accounts-tabs" },
    { id: "accounts-brokerage", label: "Brokerage", anchor: "accounts-brokerage" },
    { id: "accounts-loan", label: "Loans", anchor: "accounts-loan" },
    { id: "accounts-escrow", label: "Escrow", anchor: "accounts-escrow" },
  ],
  "contract-positions": [
    { id: "contract-create", label: "Create", anchor: "contract-create" },
    { id: "contract-open", label: "Open", anchor: "contract-open" },
    { id: "contract-closed", label: "Closed", anchor: "contract-closed" },
  ],
  "roadmap": [
    { id: "roadmap-puts", label: "Cash-covered puts", anchor: "roadmap-puts" },
    { id: "roadmap-checking", label: "Checking", anchor: "roadmap-checking" },
    { id: "roadmap-transfers", label: "Transfers", anchor: "roadmap-transfers" },
  ],
  "field-intent": [
    { id: "field-intent-controls", label: "Filters", anchor: "field-intent-controls" },
    { id: "field-intent-table", label: "Fields", anchor: "field-intent-table" },
  ],
  "task-manager": [
    { id: "tasks-open", label: "Open this week", anchor: "tasks-open" },
    { id: "tasks-snoozed", label: "Snoozed", anchor: "tasks-snoozed" },
    { id: "tasks-done", label: "Done", anchor: "tasks-done" },
    { id: "tasks-rules", label: "Add/View Task", anchor: "tasks-rules" },
  ],
  "collector-establish": [
    { id: "establish-fleet", label: "Fleet", anchor: "establish-fleet" },
  ],
  "tickets": [
    { id: "tickets-queue", label: "Work queue", anchor: "tickets-queue" },
  ],
  "collectors": [
    { id: "collectors-run", label: "Run", anchor: "collectors-run" },
    { id: "collectors-today", label: "Today", anchor: "collectors-today" },
    { id: "collectors-missing-urls", label: "Missing URLs", anchor: "collectors-missing-urls" },
    { id: "collectors-fleet", label: "Fleet", anchor: "collectors-fleet" },
    { id: "collectors-work-queue", label: "Work queue", anchor: "collectors-work-queue" },
    { id: "collectors-symbol", label: "Symbol", anchor: "collectors-symbol" },
  ],
  "import": [
    { id: "import-step", label: "Import", anchor: "import-step" },
    { id: "validate-step", label: "Validate", anchor: "validate-step" },
    { id: "loaded-step", label: "Loaded", anchor: "loaded-step" },
  ],
  "settings": [
    { id: "settings-core-functions", label: "Core functions", anchor: "settings-core-functions" },
    { id: "settings-templates", label: "Retrieval templates", anchor: "settings-templates" },
    { id: "settings-handoff", label: "Handoff", anchor: "settings-handoff" },
    { id: "settings-actions", label: "Actions", anchor: "settings-actions" },
  ],
  "components": [
    { id: "registry-pages", label: "Pages", anchor: "registry-pages" },
    { id: "registry-also", label: "Also registered", anchor: "registry-also" },
  ],
  "screen-atlas": [
    { id: "atlas-capture", label: "Capture", anchor: "atlas-capture" },
    { id: "atlas-viewer", label: "Viewer", anchor: "atlas-viewer" },
    { id: "atlas-targets", label: "Targets", anchor: "atlas-targets" },
  ],
};

/** One shared empty list. A fresh `[]` per call would re-fire the nav effect every render. */
export const NO_SECTIONS: readonly NavSection[] = [];

export function sectionsFor(screen: string, cmDesk?: string): readonly NavSection[] {
  return SCREEN_SECTIONS[cmDesk ? `${screen}/${cmDesk}` : screen] ?? NO_SECTIONS;
}
