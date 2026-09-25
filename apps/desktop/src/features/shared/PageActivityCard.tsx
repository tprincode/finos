import { useEffect } from "react";
import { setPageActivityPage, subscribePageActivity } from "./pageActivity";

function markControl(el: HTMLElement) {
  el.setAttribute("data-page-activity", "1");
  el.classList.add("is-unsaved");
}

function clearMarked(root: ParentNode) {
  root.querySelectorAll("[data-page-activity]").forEach((node) => {
    node.classList.remove("is-unsaved");
    node.removeAttribute("data-page-activity");
  });
}

export function PageActivityCard({
  page,
  rootId,
}: {
  page: string;
  rootId?: string;
}) {
  setPageActivityPage(page);

  useEffect(() => {
    setPageActivityPage(page);
    return () => {
      setPageActivityPage("");
    };
  }, [page]);

  useEffect(() => {
    if (!rootId) return;
    const root = document.getElementById(rootId);
    if (!root) return;
    const onPointer = (event: Event) => {
      const target = event.target;
      if (!(target instanceof Element)) return;
      const control = target.closest("button, select");
      if (!(control instanceof HTMLElement) || !root.contains(control)) return;
      markControl(control);
    };
    root.addEventListener("click", onPointer, true);
    root.addEventListener("change", onPointer, true);
    return () => {
      root.removeEventListener("click", onPointer, true);
      root.removeEventListener("change", onPointer, true);
    };
  }, [rootId]);

  useEffect(() => {
    if (!rootId) return;
    const root = document.getElementById(rootId);
    if (!root) return;
    return subscribePageActivity((next) => {
      if (next.some((line) => !line.done)) return;
      clearMarked(root);
    });
  }, [rootId]);

  return null;
}
