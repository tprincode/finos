import { ATLAS_DEFAULT_SETTLE_MS } from "./atlasTargets";

function sleep(ms: number): Promise<void> {
  return new Promise((r) => window.setTimeout(r, ms));
}

/** Any in-pane Loading… copy — including "Loading dividend weeks…". */
const LOADING_RE = /\bLoading\b/i;

function mainSurface(): HTMLElement {
  return (
    (document.querySelector("main.container") as HTMLElement | null) ??
    (document.getElementById("root") as HTMLElement | null) ??
    document.body
  );
}

/** Menubar chip — Page activity while invoke/query is in flight. */
function pageActivityBusy(): boolean {
  const chip = document.querySelector(
    ".menubar-activity-chip[aria-busy='true'], .menubar-activity-chip.is-busy",
  );
  return chip != null;
}

function menuWorkingVisible(): boolean {
  return document.querySelector('.menu-working[aria-busy="true"]') != null;
}

function hasLoadingCopy(root: HTMLElement): boolean {
  const nodes = root.querySelectorAll(
    '[role="status"], [aria-busy="true"], p, .income-plan-loading',
  );
  for (const node of nodes) {
    if (node.closest?.(".menubar-activity, .screen-atlas-progress")) continue;
    const text = (node.textContent ?? "").trim();
    if (!text) continue;
    if (LOADING_RE.test(text)) return true;
  }
  return false;
}

export function screenStillWorking(isBusy: () => boolean): boolean {
  if (isBusy()) return true;
  if (pageActivityBusy()) return true;
  if (menuWorkingVisible()) return true;
  if (hasLoadingCopy(mainSurface())) return true;
  return false;
}

/**
 * Wait until Page activity is idle, App busy clears, and Loading… copy
 * stays gone for a quiet window (nested loads like Plan vs Decl start after
 * the parent table). Then settle so charts/calendars can paint.
 */
export async function waitForScreenReady(args: {
  isBusy: () => boolean;
  settleMs?: number;
  busyTimeoutMs?: number;
  /** How long Loading… / activity must stay clear before we call it ready. */
  quietMs?: number;
}): Promise<void> {
  const busyTimeout = args.busyTimeoutMs ?? 90000;
  const quietMs = args.quietMs ?? 1200;
  const settle = args.settleMs ?? ATLAS_DEFAULT_SETTLE_MS;
  const start = Date.now();

  // Let React commit the navigated screen (first Loading… may appear late).
  await sleep(500);

  let quietSince: number | null = null;
  while (Date.now() - start < busyTimeout) {
    if (screenStillWorking(args.isBusy)) {
      quietSince = null;
      await sleep(200);
      continue;
    }
    if (quietSince == null) quietSince = Date.now();
    if (Date.now() - quietSince >= quietMs) break;
    await sleep(150);
  }

  await new Promise<void>((resolve) => {
    requestAnimationFrame(() => requestAnimationFrame(() => resolve()));
  });
  await sleep(settle);
}
