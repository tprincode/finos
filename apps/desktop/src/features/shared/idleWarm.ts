/** Run `work` after the page is idle. Cancel on navigate, write, or unmount. */
export function scheduleIdleWarm(work: () => void, delayMs = 1200): () => void {
  const id = window.setTimeout(work, delayMs);
  return () => window.clearTimeout(id);
}
