/** Survive Add Lot remount: ShoppingCartScreen unmounts when screen leaves shopping-cart. */
export const CART_RESUME_KEY = "finos.cart.resume";

export type CartResume = {
  scenarioId: string;
  accountId: string;
  planId: string;
};

export function writeCartResume(resume: CartResume): void {
  try {
    sessionStorage.setItem(CART_RESUME_KEY, JSON.stringify(resume));
  } catch {
    /* private mode */
  }
}

export function readCartResume(): CartResume | null {
  try {
    const raw = sessionStorage.getItem(CART_RESUME_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as Partial<CartResume>;
    if (
      typeof parsed.scenarioId !== "string" ||
      !parsed.scenarioId ||
      typeof parsed.accountId !== "string" ||
      !parsed.accountId
    ) {
      return null;
    }
    return {
      scenarioId: parsed.scenarioId,
      accountId: parsed.accountId,
      planId:
        typeof parsed.planId === "string" && parsed.planId
          ? parsed.planId
          : parsed.scenarioId,
    };
  } catch {
    return null;
  }
}

export function clearCartResume(): void {
  try {
    sessionStorage.removeItem(CART_RESUME_KEY);
  } catch {
    /* private mode */
  }
}
