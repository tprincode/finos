import { useState } from "react";

const PREFIX = "finos.chart-default.";

export function readChartDefault(key: string): string | null {
  try {
    const value = localStorage.getItem(PREFIX + key);
    return value && value.length > 0 ? value : null;
  } catch {
    return null;
  }
}

export function writeChartDefault(key: string, value: string): void {
  try {
    localStorage.setItem(PREFIX + key, value);
  } catch {
    /* private mode */
  }
}

export function initialChartDefault<T extends string>(
  key: string,
  fallback: T,
  allowed: readonly T[],
): T {
  const stored = readChartDefault(key);
  if (stored && (allowed as readonly string[]).includes(stored)) {
    return stored as T;
  }
  return fallback;
}

/** Small tick. No caption. Click stores the current choice as the next default. */
export function DefaultTick({
  storageKey,
  value,
}: {
  storageKey: string;
  value: string;
}) {
  const [saved, setSaved] = useState(() => readChartDefault(storageKey));
  const current = saved === value;
  return (
    <button
      type="button"
      className={current ? "chart-default-tick is-current" : "chart-default-tick"}
      aria-label="Set default"
      onClick={() => {
        writeChartDefault(storageKey, value);
        setSaved(value);
      }}
    >
      {current ? "✓" : ""}
    </button>
  );
}
