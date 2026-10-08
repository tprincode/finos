/** Same screen order as App `SCREENS` and `CmDesk`. Components, Screen Atlas, and the snapshot share it. */

export const SCREEN_ORDER = [
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
  "account-management",
  "contract-positions",
  "field-intent",
  "roadmap",
  "components",
  "screen-atlas",
] as const;

export const SCREEN_LABEL: Record<(typeof SCREEN_ORDER)[number], string> = {
  home: "Home",
  "income-plan": "Income Plan",
  calculator: "Calculator",
  "market-impact": "Market impact planner",
  dashboard: "Dashboard",
  trends: "Trends",
  "cash-management": "Cash Management",
  "shopping-cart": "Shopping Cart",
  holdings: "Holdings",
  import: "Import",
  settings: "Settings",
  "new-investment": "Add Investment",
  "add-lot": "Add Lot",
  "position-details": "Position Details",
  collectors: "Collectors",
  tickets: "Tickets",
  "collector-establish": "Reevaluate collector",
  "task-manager": "Task Manager",
  "interest-rate": "Interest rate calculator",
  "account-management": "Account Management",
  "contract-positions": "Contract positions",
  "field-intent": "Field intent",
  roadmap: "Roadmap",
  components: "Component Registry",
  "screen-atlas": "Screen Atlas",
};

const REGISTRY_SCREENS = new Set(["components", "screen-atlas"]);

/** One owner page for Component Registry. Screen Atlas stays a screen id, not a second heading. */
export function ownerPageGroups<T extends ScreenKeyed>(
  items: readonly T[],
): { groups: ScreenGroup<T>[]; also: T[] } {
  const grouped = groupByScreen(items);
  const merged = grouped.groups
    .filter((group) => REGISTRY_SCREENS.has(group.screen))
    .flatMap((group) => group.direct);
  const groups = grouped.groups.flatMap((group) => {
    if (group.screen === "screen-atlas") return [];
    if (group.screen === "components") {
      return [{ ...group, label: SCREEN_LABEL.components, direct: merged }];
    }
    return [group];
  });
  return { groups, also: grouped.also };
}

export const DESK_ORDER = [
  "elements",
  "cashflow",
  "weekly",
  "car",
  "coverage",
  "external",
  "manager",
] as const;

export const DESK_LABEL: Record<(typeof DESK_ORDER)[number], string> = {
  elements: "Element Management",
  cashflow: "Cashflow Manager",
  weekly: "Week ahead planner",
  car: "Tax Planning",
  coverage: "Income vs Expense planner",
  external: "External accounts",
  manager: "Debt planner",
};

export type ScreenKeyed = {
  screen?: string;
  cmDesk?: string;
};

export type DeskGroup<T> = {
  desk: string;
  label: string;
  items: T[];
};

export type ScreenGroup<T> = {
  screen: string;
  label: string;
  direct: T[];
  desks: DeskGroup<T>[];
};

export function groupByScreen<T extends ScreenKeyed>(
  items: readonly T[],
): { groups: ScreenGroup<T>[]; also: T[] } {
  const also = items.filter((item) => !item.screen);
  const groups = SCREEN_ORDER.map((screen) => {
    const mine = items.filter((item) => item.screen === screen);
    if (screen === "cash-management") {
      const direct = mine.filter((item) => !item.cmDesk);
      const desks = DESK_ORDER.map((desk) => ({
        desk,
        label: DESK_LABEL[desk],
        items: mine.filter((item) => item.cmDesk === desk),
      })).filter((desk) => desk.items.length > 0);
      return { screen, label: SCREEN_LABEL[screen], direct, desks };
    }
    return {
      screen,
      label: SCREEN_LABEL[screen],
      direct: mine,
      desks: [] as DeskGroup<T>[],
    };
  }).filter((group) => group.direct.length > 0 || group.desks.length > 0);
  return { groups, also };
}
