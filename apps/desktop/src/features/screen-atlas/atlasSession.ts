/** Session flags while Screen Atlas is capturing (docs mode, not a golden oracle). */

let frozen = false;
let pinnedAsOf: string | null = null;
/** Survives React StrictMode remounts in the same JS context. */
let autoStartClaimed = false;

export function beginAtlasFreeze(asOf: string): void {
  frozen = true;
  pinnedAsOf = asOf.slice(0, 10);
  document.documentElement.classList.add("screen-atlas-freeze");
}

export function endAtlasFreeze(): void {
  frozen = false;
  pinnedAsOf = null;
  document.documentElement.classList.remove("screen-atlas-freeze");
}

export function isAtlasFrozen(): boolean {
  return frozen;
}

/** When frozen, App should prefer this asOf over calendar drift. */
export function atlasPinnedAsOf(): string | null {
  return pinnedAsOf;
}

/** Spread into ECharts option objects during atlas capture. */
export function atlasChartExtras(): { animation: boolean } | Record<string, never> {
  return frozen ? { animation: false } : {};
}

/** Claim the one-shot auto-start (token file). False if already claimed this session. */
export function claimAtlasAutoStart(): boolean {
  if (autoStartClaimed) {
    return false;
  }
  autoStartClaimed = true;
  return true;
}
