import { formatUsd } from "@finos/ui-components";
import type { ReactNode } from "react";

const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

export function monthDay(iso: string): string {
  const [y, m, d] = iso.slice(0, 10).split("-").map(Number);
  if (!y || !m || !d || m < 1 || m > 12) return iso;
  return `${MONTHS[m - 1]} ${d}`;
}

export function domainLabel(domain: string): string {
  if (domain === "plan") return "Dividend Plan";
  return domain;
}

function statusLabel(status: string): string {
  if (status === "ignored_until") return "Snoozed";
  if (status === "done") return "Done";
  return "Open";
}

export type ReminderRow = {
  id: string;
  title: ReactNode;
  domain: string;
  due: string;
  status: string;
  actions: ReactNode;
  selected?: boolean;
  onSelect?: () => void;
};

export function ReminderTable({
  rows,
  label,
}: {
  rows: ReminderRow[];
  label?: string;
}) {
  return (
    <div className="table-wrap reminder-table">
      <table aria-label={label}>
        <thead>
          <tr>
            <th>Title</th>
            <th>Domain</th>
            <th>Due</th>
            <th>Status</th>
            <th>Actions</th>
          </tr>
        </thead>
        <tbody>
          {rows.length === 0 ? (
            <tr>
              <td colSpan={5}>None</td>
            </tr>
          ) : (
            rows.map((row) => (
              <tr
                key={row.id}
                className={row.selected ? "is-selected" : undefined}
                onClick={row.onSelect}
              >
                <td>{row.title}</td>
                <td>{row.domain}</td>
                <td>{row.due}</td>
                <td>{row.status}</td>
                <td className="task-actions">{row.actions}</td>
              </tr>
            ))
          )}
        </tbody>
      </table>
    </div>
  );
}

export type TaskReminderSource = {
  taskId: string;
  code: string;
  title: string;
  domain: string;
  dueOn: string;
  status: string;
  payloadJson: string;
};

export function taskReminderRows({
  rows,
  actions,
  pendingId,
  selectedId,
  onSelect,
  onOpenSymbol,
  onOpenMagi,
  onResolve,
  onSnooze,
  snoozeAria,
}: {
  rows: TaskReminderSource[];
  actions: boolean;
  pendingId: string | null;
  selectedId?: string | null;
  onSelect?: (taskId: string) => void;
  onOpenSymbol?: (symbol: string) => void;
  onOpenMagi?: (taskId: string) => void;
  onResolve: (taskId: string) => void;
  onSnooze: (taskId: string) => void;
  snoozeAria?: (title: string) => string;
}): ReminderRow[] {
  return rows.map((row) => {
    let title = row.title;
    let symbol = "";
    if (row.code === "magi_cliff_over") {
      try {
        const payload = JSON.parse(row.payloadJson) as { overageMinor?: number };
        if (payload.overageMinor) {
          title = `MAGI over cliff by ${formatUsd(payload.overageMinor)} — resolve or snooze till next plan week.`;
        }
      } catch {
        /* keep stored title */
      }
    } else if (row.code.startsWith("plan_under:") || row.code.startsWith("plan_over:")) {
      try {
        const payload = JSON.parse(row.payloadJson) as { symbol?: string };
        if (payload.symbol) symbol = payload.symbol;
      } catch {
        /* keep stored title */
      }
    }
    const pending = pendingId === row.taskId;
    const magi = row.code === "magi_cliff_over";
    const ticketEnabled = magi ? Boolean(onOpenMagi) : Boolean(symbol && onOpenSymbol);
    return {
      id: row.taskId,
      selected: selectedId === row.taskId,
      onSelect: onSelect ? () => onSelect(row.taskId) : undefined,
      domain: domainLabel(row.domain),
      due: monthDay(row.dueOn),
      status: statusLabel(row.status),
      title: (
        <>
          {symbol && onOpenSymbol ? (
            <button
              type="button"
              aria-label={`Open ${symbol} in Position Details`}
              onClick={(event) => {
                event.stopPropagation();
                onOpenSymbol(symbol);
              }}
            >
              {symbol}
            </button>
          ) : null}
          {symbol ? ` — ${title}` : title}
        </>
      ),
      actions: (
        <>
          {actions ? (
            <>
              <button
                type="button"
                aria-label={`Resolve ${row.title}`}
                disabled={pending}
                className={pending ? "is-unsaved" : undefined}
                onClick={(event) => {
                  event.stopPropagation();
                  onResolve(row.taskId);
                }}
              >
                Resolve
              </button>
              <button
                type="button"
                aria-label={snoozeAria ? snoozeAria(row.title) : `Snooze ${row.title}`}
                disabled={pending}
                className={pending ? "is-unsaved" : undefined}
                onClick={(event) => {
                  event.stopPropagation();
                  onSnooze(row.taskId);
                }}
              >
                Snooze
              </button>
            </>
          ) : null}
          <button
            type="button"
            aria-label={`Work ticket ${symbol || row.title}`}
            disabled={!ticketEnabled}
            onClick={(event) => {
              event.stopPropagation();
              if (magi && onOpenMagi) onOpenMagi(row.taskId);
              else if (symbol && onOpenSymbol) onOpenSymbol(symbol);
            }}
          >
            Work ticket
          </button>
        </>
      ),
    };
  });
}
