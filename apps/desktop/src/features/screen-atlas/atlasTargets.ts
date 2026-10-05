/** Runtime capture list — the same App screens and Cash Management desks as the component catalog. */

export type AtlasScreen =
  | "home"
  | "income-plan"
  | "calculator"
  | "market-impact"
  | "dashboard"
  | "trends"
  | "cash-management"
  | "shopping-cart"
  | "holdings"
  | "import"
  | "settings"
  | "new-investment"
  | "add-lot"
  | "position-details"
  | "collectors"
  | "tickets"
  | "collector-establish"
  | "task-manager"
  | "interest-rate"
  | "contract-positions"
  | "components"
  | "screen-atlas";

export type AtlasCmDesk =
  | "elements"
  | "cashflow"
  | "weekly"
  | "car"
  | "coverage"
  | "external";

export type AtlasTarget = {
  /** File stem under the dated atlas folder. */
  id: string;
  /** App Screen id to navigate to. */
  screen: AtlasScreen;
  /** Cash Management desk when screen is cash-management. */
  cmDesk?: AtlasCmDesk;
  /** Menubar path for index.md. */
  menuPath: string;
  label: string;
  /**
   * Extra settle after busy + Loading… clear (ms). Heavy screens need longer
   * so charts/calendars finish painting before html-to-image.
   */
  settleMs?: number;
};

/** Default settle after a screen looks idle (ms). */
export const ATLAS_DEFAULT_SETTLE_MS = 2000;

/** Every surface the atlas walks. Keep in sync with App.tsx SCREENS + CmDesk. */
export const ATLAS_TARGETS: readonly AtlasTarget[] = [
  {
    id: "home",
    screen: "home",
    menuPath: "Home",
    label: "Home",
    settleMs: 4500,
  },
  {
    id: "income-plan",
    screen: "income-plan",
    menuPath: "Income Plan",
    label: "Income Plan",
    settleMs: 5000,
  },
  {
    id: "trends",
    screen: "trends",
    menuPath: "Trends",
    label: "Trends",
    settleMs: 4000,
  },
  {
    id: "cm-elements",
    screen: "cash-management",
    cmDesk: "elements",
    menuPath: "Cash Management → Element Management",
    label: "Element Management",
    settleMs: 3000,
  },
  {
    id: "cm-cashflow",
    screen: "cash-management",
    cmDesk: "cashflow",
    menuPath: "Cash Management → Cashflow Manager",
    label: "Cash Flow Manager",
    settleMs: 4500,
  },
  {
    id: "cm-weekly",
    screen: "cash-management",
    cmDesk: "weekly",
    menuPath: "Cash Management → Week ahead planner",
    label: "System update tasks",
    settleMs: 3000,
  },
  {
    id: "cm-car",
    screen: "cash-management",
    cmDesk: "car",
    menuPath: "Cash Management → Tax Planning",
    label: "Tax Planning",
    settleMs: 3500,
  },
  {
    id: "cm-coverage",
    screen: "cash-management",
    cmDesk: "coverage",
    menuPath: "Cash Management → Income vs Expense planner",
    label: "Income vs Expense planner",
    settleMs: 3500,
  },
  {
    id: "cm-external",
    screen: "cash-management",
    cmDesk: "external",
    menuPath: "Cash Management → External accounts",
    label: "External accounts",
    settleMs: 3000,
  },
  {
    id: "calculator",
    screen: "calculator",
    menuPath: "Plan → Calculator",
    label: "Calculator",
    settleMs: 5000,
  },
  {
    id: "dashboard",
    screen: "dashboard",
    menuPath: "Plan → Dashboard",
    label: "Dashboard",
    settleMs: 3000,
  },
  {
    id: "market-impact",
    screen: "market-impact",
    menuPath: "Plan → Market impact planner",
    label: "Market impact planner",
  },
  {
    id: "cash-management-default",
    screen: "cash-management",
    cmDesk: "weekly",
    menuPath: "Plan → Cash Management",
    label: "Cash Management (Plan entry)",
    settleMs: 3000,
  },
  {
    id: "shopping-cart",
    screen: "shopping-cart",
    menuPath: "Plan → Shopping Cart",
    label: "Shopping Cart",
  },
  {
    id: "position-details",
    screen: "position-details",
    menuPath: "Positions → Position Details",
    label: "Position Details",
  },
  {
    id: "holdings",
    screen: "holdings",
    menuPath: "Positions → Holdings",
    label: "Holdings",
    settleMs: 4000,
  },
  {
    id: "new-investment",
    screen: "new-investment",
    menuPath: "Positions → Add Investment",
    label: "Add Investment",
  },
  {
    id: "add-lot",
    screen: "add-lot",
    menuPath: "Positions → Add Lot",
    label: "Add Lot",
  },
  { id: "import", screen: "import", menuPath: "Data → Import", label: "Import" },
  {
    id: "collectors",
    screen: "collectors",
    menuPath: "Data → Collectors",
    label: "Collectors",
    settleMs: 3500,
  },
  {
    id: "tickets",
    screen: "tickets",
    menuPath: "Data → Tickets",
    label: "Tickets",
    settleMs: 3000,
  },
  {
    id: "collector-establish",
    screen: "collector-establish",
    menuPath: "Tools → Reevaluate collector",
    label: "Reevaluate collector",
  },
  {
    id: "task-manager",
    screen: "task-manager",
    menuPath: "Tools → Task Manager",
    label: "Task Manager",
  },
  {
    id: "interest-rate",
    screen: "interest-rate",
    menuPath: "Tools → Interest rate calculator",
    label: "Interest rate calculator",
  },
  {
    id: "contract-positions",
    screen: "contract-positions",
    menuPath: "Tools → Contract positions",
    label: "Contract positions",
  },
  {
    id: "components",
    screen: "components",
    menuPath: "Tools → Components",
    label: "Components",
  },
  {
    id: "settings",
    screen: "settings",
    menuPath: "Tools → Settings",
    label: "Settings",
  },
  {
    id: "screen-atlas",
    screen: "screen-atlas",
    menuPath: "Tools → Screen Atlas",
    label: "Screen Atlas",
  },
];

export const ATLAS_SCREEN_IDS: readonly AtlasScreen[] = [
  "home",
  "income-plan",
  "calculator",
  "market-impact",
  "dashboard",
  "trends",
  "cash-management",
  "shopping-cart",
  "holdings",
  "import",
  "settings",
  "new-investment",
  "add-lot",
  "position-details",
  "collectors",
  "tickets",
  "collector-establish",
  "task-manager",
  "interest-rate",
  "contract-positions",
  "components",
  "screen-atlas",
];

export const ATLAS_CM_DESKS: readonly AtlasCmDesk[] = [
  "elements",
  "cashflow",
  "weekly",
  "car",
  "coverage",
  "external",
];
