import { useEffect, useState } from "react";
import { type NavSection, sectionsFor } from "./pageSections";

/** One shared empty list, so "nothing is live" never counts as a new state. */
const NONE_LIVE: readonly string[] = [];

function sameList(a: readonly string[], b: readonly string[]): boolean {
  return a.length === b.length && a.every((id, i) => id === b[i]);
}

function mounted(sections: readonly NavSection[]): string[] {
  return sections.filter((s) => document.getElementById(s.anchor)).map((s) => s.id);
}

/**
 * Jump links for the sections of this screen. Several sections render only when
 * they have data, so a link appears once its anchor is on the page — a shortcut
 * to nothing is worse than no shortcut.
 */
export function SectionNav({ screen, cmDesk }: { screen: string; cmDesk?: string }) {
  const sections = sectionsFor(screen, cmDesk);
  const [live, setLive] = useState<readonly string[]>(NONE_LIVE);

  useEffect(() => {
    if (sections.length === 0) {
      setLive((prev) => (prev.length === 0 ? prev : NONE_LIVE));
      return;
    }
    let frame = 0;
    const sync = () => {
      frame = 0;
      setLive((prev) => {
        const next = mounted(sections);
        return sameList(prev, next) ? prev : next;
      });
    };
    sync();
    const observer = new MutationObserver(() => {
      if (frame === 0) frame = requestAnimationFrame(sync);
    });
    observer.observe(document.body, { childList: true, subtree: true });
    return () => {
      observer.disconnect();
      if (frame !== 0) cancelAnimationFrame(frame);
    };
  }, [sections]);

  const shown = sections.filter((s) => live.includes(s.id));
  if (shown.length < 2) return null;
  return (
    <nav className="page-nav-sections" aria-label="Sections on this page">
      {shown.map((s) => (
        <a key={s.id} href={`#${s.anchor}`}>
          {s.label}
        </a>
      ))}
    </nav>
  );
}
