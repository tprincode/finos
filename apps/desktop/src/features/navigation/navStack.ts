export type NavFrame = {
  id?: string;
  screen?: string;
  cmDesk?: string;
  restore: () => void;
};

let frames: NavFrame[] = [];
const listeners = new Set<() => void>();

function emit() {
  listeners.forEach((listener) => listener());
}

export function pushNav(frame: NavFrame) {
  frames.push(frame);
  emit();
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
