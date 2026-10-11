export type NavFrame = {
  id?: string;
  screen?: string;
  cmDesk?: string;
  restore: () => void;
};

/**
 * Caller-based Return: push a frame when opening a child screen; shell Return and
 * child Confirm/Cancel call `returnToCaller(id)` (or `popNav`) so restore runs.
 * Never hardcode the destination screen on exit — the caller's `restore` owns it.
 */
let frames: NavFrame[] = [];
const listeners = new Set<() => void>();

function emit() {
  listeners.forEach((listener) => listener());
}

export function pushNav(frame: NavFrame) {
  frames.push(frame);
  emit();
}

/**
 * Record the calling place before `setScreen` to the child. Replaces a top frame
 * with the same `id` without running its restore (re-open of the same child).
 */
export function openFromCaller(frame: NavFrame & { id: string }) {
  if (frames[frames.length - 1]?.id === frame.id) {
    frames.pop();
  }
  pushNav(frame);
}

/** Pop and restore when the top frame is this child; otherwise leave the stack alone. */
export function returnToCaller(id: string): boolean {
  return popNavIf(id);
}

export function peekNav(): NavFrame | null {
  return frames.length > 0 ? frames[frames.length - 1] : null;
}

export function popNav() {
  const frame = frames.pop();
  emit();
  frame?.restore();
}

export function popNavIf(id: string): boolean {
  if (frames[frames.length - 1]?.id !== id) return false;
  popNav();
  return true;
}

export function navDepth(): number {
  return frames.length;
}

export function subscribeNav(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}
