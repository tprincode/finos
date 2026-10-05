import { useSyncExternalStore } from "react";
import { navDepth, subscribeNav } from "./navStack";

export function ReturnToPrevious({ onReturn }: { onReturn: () => void }) {
  const depth = useSyncExternalStore(subscribeNav, navDepth, navDepth);
  if (depth === 0) return null;
  return (
    <button
      type="button"
      className="return-previous"
      aria-label="Return to previous menu"
      onClick={onReturn}
    >
      Return
    </button>
  );
}
