export type PageActivityLine = {
  id: string;
  label: string;
  done?: boolean;
};

const LOG_KEEP = 10;

type Listener = (lines: PageActivityLine[]) => void;

let currentPage = "";
let lines: PageActivityLine[] = [];
const listeners = new Set<Listener>();

function emit() {
  const snapshot = lines.slice();
  listeners.forEach((fn) => fn(snapshot));
}

export function setPageActivityPage(page: string) {
  currentPage = page.trim();
}

export function pageActivityPage() {
  return currentPage;
}

export function subscribePageActivity(fn: Listener): () => void {
  listeners.add(fn);
  fn(lines.slice());
  return () => {
    listeners.delete(fn);
  };
}

export function activityLabel(name: string): string {
  const trimmed = name.trim();
  if (!trimmed) return "Working";
  if (trimmed.endsWith("Get")) {
    return `Reading ${trimmed.slice(0, -3)}`;
  }
  if (
    trimmed.endsWith("Save") ||
    trimmed.endsWith("Confirm") ||
    trimmed.endsWith("Post") ||
    trimmed.endsWith("Delete") ||
    trimmed.endsWith("Resolve") ||
    trimmed.endsWith("File")
  ) {
    return `Writing ${trimmed}`;
  }
  return trimmed;
}

export function beginPageActivity(label: string): string | null {
  const id = crypto.randomUUID();
  lines = [...lines, { id, label }].slice(-LOG_KEEP);
  emit();
  return id;
}

export function endPageActivity(id: string | null) {
  if (!id) return;
  lines = lines.map((line) => (line.id === id ? { ...line, done: true } : line));
  emit();
}
