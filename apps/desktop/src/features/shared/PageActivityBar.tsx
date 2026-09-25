import { useEffect, useRef, useState } from "react";
import {
  subscribePageActivity,
  type PageActivityLine,
} from "./pageActivity";

export function PageActivityBar() {
  const rootRef = useRef<HTMLDivElement>(null);
  const [lines, setLines] = useState<PageActivityLine[]>([]);
  const [open, setOpen] = useState(false);

  useEffect(() => subscribePageActivity(setLines), []);

  useEffect(() => {
    if (!open) return;
    const onDown = (event: PointerEvent) => {
      const root = rootRef.current;
      if (root && event.target instanceof Node && !root.contains(event.target)) {
        setOpen(false);
      }
    };
    document.addEventListener("pointerdown", onDown);
    return () => document.removeEventListener("pointerdown", onDown);
  }, [open]);

  const busy = lines.some((line) => !line.done);
  const last = lines[lines.length - 1];
  const summary = last?.label ?? "Idle";

  return (
    <div className="menubar-activity" ref={rootRef}>
      <button
        type="button"
        className={
          busy ? "menubar-activity-chip is-busy" : "menubar-activity-chip"
        }
        aria-label="Page activity"
        aria-expanded={open}
        aria-busy={busy ? "true" : "false"}
        aria-haspopup="dialog"
        onClick={() => setOpen((next) => !next)}
      >
        <span className="menubar-activity-last">{summary}</span>
      </button>
      {open ? (
        <div
          className="menubar-activity-list"
          role="dialog"
          aria-label="Page activity log"
        >
          {lines.length === 0 ? (
            <p className="page-activity-idle">
              Idle. Reads and writes for this session appear here.
            </p>
          ) : (
            <ul>
              {lines.map((line) => (
                <li key={line.id} data-done={line.done ? "1" : "0"}>
                  {line.label}
                </li>
              ))}
            </ul>
          )}
        </div>
      ) : null}
    </div>
  );
}
