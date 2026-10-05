import { useEffect, useState, type ReactNode } from "react";
import { subscribePageActivity, type PageActivityLine } from "./pageActivity";

function openLabel(lines: PageActivityLine[]): string {
  const open = lines.filter((line) => !line.done);
  return open[open.length - 1]?.label ?? "Working";
}

export function BusySurface({
  busy,
  children,
}: {
  busy: boolean;
  children: ReactNode;
}) {
  const [label, setLabel] = useState("Working");

  useEffect(
    () => subscribePageActivity((lines) => setLabel(openLabel(lines))),
    [],
  );

  return (
    <div
      className={busy ? "busy-surface is-busy" : "busy-surface"}
      aria-busy={busy || undefined}
    >
      <div className="busy-surface-body">{children}</div>
      {busy ? (
        <p className="busy-surface-status" role="status">
          {label}
        </p>
      ) : null}
    </div>
  );
}
