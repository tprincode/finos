function sleep(ms: number): Promise<void> {
  return new Promise((r) => window.setTimeout(r, ms));
}

function dataUrlToBytes(dataUrl: string): Uint8Array {
  const comma = dataUrl.indexOf(",");
  const b64 = comma >= 0 ? dataUrl.slice(comma + 1) : dataUrl;
  const bin = atob(b64);
  const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i += 1) {
    out[i] = bin.charCodeAt(i);
  }
  return out;
}

type StylePatch = {
  el: HTMLElement;
  overflow: string;
  maxHeight: string;
  height: string;
};

/**
 * Unlock nested scroll/clip so html-to-image sees the full document height
 * (Income Plan panels, calculator sheet, calendar wraps, etc.).
 */
export function expandForFullCapture(root: HTMLElement): () => void {
  const patches: StylePatch[] = [];
  const seen = new Set<HTMLElement>();

  const patch = (el: HTMLElement) => {
    if (seen.has(el)) return;
    seen.add(el);
    patches.push({
      el,
      overflow: el.style.overflow,
      maxHeight: el.style.maxHeight,
      height: el.style.height,
    });
    el.style.overflow = "visible";
    el.style.maxHeight = "none";
    el.style.height = "auto";
  };

  patch(document.documentElement);
  patch(document.body);
  const appRoot = document.getElementById("root");
  if (appRoot) patch(appRoot);
  patch(root);

  const nodes = root.querySelectorAll<HTMLElement>("*");
  for (const el of nodes) {
    const style = window.getComputedStyle(el);
    const overflowY = style.overflowY;
    const clips =
      overflowY === "auto" ||
      overflowY === "scroll" ||
      overflowY === "hidden" ||
      style.overflow === "hidden";
    const constrained =
      style.maxHeight !== "none" && style.maxHeight !== "0px"
        ? true
        : false;
    if (clips || constrained) {
      patch(el);
    }
  }

  window.scrollTo(0, 0);
  root.scrollTop = 0;

  return () => {
    for (const p of patches) {
      p.el.style.overflow = p.overflow;
      p.el.style.maxHeight = p.maxHeight;
      p.el.style.height = p.height;
    }
  };
}

function captureFilter(node: HTMLElement): boolean {
  if (node.classList.contains("screen-atlas-progress")) return false;
  if (node.classList.contains("menubar-activity-list")) return false;
  if (
    node.matches?.("main.container > p") &&
    /Screen Atlas|Saved |Working…/i.test(node.textContent ?? "")
  ) {
    return false;
  }
  return true;
}

/**
 * Capture the full scrollable app surface (not just the visible window).
 * Expands overflow, measures full height, then html-to-image at that size.
 * Width stays at the viewport so wide sheets stay readable.
 */
export async function captureMainPngBytes(): Promise<Uint8Array> {
  const { toPng } = await import("html-to-image");
  const root =
    (document.getElementById("root") as HTMLElement | null) ??
    (document.querySelector("main.container") as HTMLElement | null) ??
    document.body;

  const restore = expandForFullCapture(root);
  try {
    await sleep(250);
    await new Promise<void>((resolve) => {
      requestAnimationFrame(() => requestAnimationFrame(() => resolve()));
    });

    const viewportW = Math.max(window.innerWidth || 0, 800);
    const width = Math.min(
      Math.max(root.scrollWidth, root.clientWidth, viewportW),
      1920,
    );
    // Prefer layout height after expand — not the clipped viewport.
    const rawHeight = Math.max(
      root.scrollHeight,
      root.offsetHeight,
      document.documentElement.scrollHeight,
      document.body.scrollHeight,
      600,
    );
    const height = Math.min(rawHeight, 16000);
    const pixelRatio = Math.min(2, Math.max(1, window.devicePixelRatio || 1));

    const opts = {
      cacheBust: true,
      pixelRatio,
      width,
      height,
      backgroundColor: "#f6f6f6",
      style: {
        overflow: "visible",
        height: `${height}px`,
        maxHeight: "none",
      },
      filter: (node: Element) => {
        if (!(node instanceof HTMLElement)) return true;
        return captureFilter(node);
      },
    };

    const dataUrl = await toPng(root, opts);
    return dataUrlToBytes(dataUrl);
  } finally {
    restore();
  }
}
