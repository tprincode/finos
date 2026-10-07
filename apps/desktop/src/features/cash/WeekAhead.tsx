import { useState } from "react";
import { formatUsd } from "@finos/ui-components";
import type { AccountListItem, LoanWeekRow, WeekAheadGet } from "@finos/app-contracts";
import { MagiCliffPanel } from "../task-manager/MagiCliffPanel";
import { ReminderTable, taskReminderRows } from "../task-manager/ReminderTable";
import { BusySurface } from "../shared/BusySurface";

export function WeekAheadPanel({
  week,
  busy,
  pendingId,
  onConfirm,
  onOpenEditor,
  onDefer,
  onConfirmLoan,
  onResolveTask,
  onIgnoreTask,
  onOpenSymbol,
  onOpenMagi,
  magiTaskId,
  magiClient,
  magiAccounts,
  asOfDate,
  onCutDraws,
  onCloseMagi,
  onMagiPosted,
  onAddElement,
}: {
  week: WeekAheadGet | null;
  busy?: boolean;
  pendingId?: string | null;
  onConfirm: (occurrenceId: string) => void;
  onOpenEditor: (row: {
    occurrenceId: string;
    elementId: string;
    account: string;
    note: string;
  }) => void;
  onDefer: (occurrenceId: string) => void;
  onConfirmLoan: (loan: {
    accountId: string;
    dueOn: string;
    principalMinor: number;
    interestMinor: number;
  }) => void;
  onResolveTask: (taskId: string) => void;
  onIgnoreTask: (taskId: string) => void;
  onOpenSymbol?: (symbol: string) => void;
  onOpenMagi?: (taskId: string) => void;
  magiTaskId?: string | null;
  magiClient?: {
    executeQuery: (name: string, body?: unknown) => Promise<{ ok: boolean; bodyJson?: string }>;
    executeCommand: (name: string, body?: unknown) => Promise<{ ok: boolean; errorCode?: string }>;
  };
  magiAccounts?: AccountListItem[];
  asOfDate?: string;
  onCutDraws?: () => void;
  onCloseMagi?: () => void;
  onMagiPosted?: () => void;
  onAddElement: () => void;
}) {
  if (!week) return null;
  const scale = week.scale ?? 2;
  const tasks = week.tasks ?? [];

  return (
    <section className="week-ahead" aria-label="Week ahead">
      <h3>Week ahead</h3>
      <BusySurface busy={!!busy}>
      <h3>Tasks</h3>
      {magiTaskId && magiClient && onCutDraws && onCloseMagi ? (
        <MagiCliffPanel
          client={magiClient}
          taskId={magiTaskId}
          asOfDate={asOfDate || week.periodEnd}
          accounts={magiAccounts ?? []}
          busy={!!busy}
          onCutDraws={onCutDraws}
          onClose={onCloseMagi}
          onPosted={onMagiPosted ?? (() => {})}
        />
      ) : null}
      <ReminderTable
        label="Tasks"
        rows={taskReminderRows({
          rows: tasks,
          actions: true,
          pendingId: pendingId ?? null,
          onOpenSymbol,
          onOpenMagi,
          onResolve: onResolveTask,
          onSnooze: onIgnoreTask,
          snoozeAria: (title) => `Snooze ${title} till next plan week`,
        })}
      />
      <div className="week-ahead-section-head">
        <h3>Elements</h3>
        <button type="button" aria-label="Add element" onClick={onAddElement}>
          Add element
        </button>
      </div>
      <div className="table-wrap">
        <table aria-label="Week ahead elements">
          <thead>
            <tr>
              <th>Date</th>
              <th>Account</th>
              <th>Transaction</th>
              <th>Amount</th>
              <th>Confirm</th>
              <th>Edit</th>
              <th>Snooze</th>
            </tr>
          </thead>
          <tbody>
            {week.rows.map((row) => {
              const pending = pendingId === row.occurrenceId;
              const name = row.note || row.transaction;
              return (
                <tr key={row.occurrenceId}>
                  <td>{row.occurredOn}</td>
                  <td>{row.account}</td>
                  <td>{row.transaction}</td>
                  <td className="numeric">
                    {formatUsd(row.amountMinor, row.scale ?? scale)}
                  </td>
                  <td>
                    <button
                      type="button"
                      aria-label={`Confirm ${row.account} ${name}`}
                      disabled={busy || pending}
                      className={pending ? "is-unsaved" : undefined}
                      onClick={() => onConfirm(row.occurrenceId)}
                    >
                      Confirm
                    </button>
                  </td>
                  <td>
                    <button
                      type="button"
                      aria-label={`Edit ${row.account} ${name}`}
                      disabled={busy}
                      onClick={() =>
                        onOpenEditor({
                          occurrenceId: row.occurrenceId,
                          elementId: row.elementId,
                          account: row.account,
                          note: row.note,
                        })
                      }
                    >
                      Edit
                    </button>
                  </td>
                  <td>
                    <button
                      type="button"
                      aria-label={`Snooze till tomorrow ${row.account} ${name}`}
                      disabled={busy}
                      onClick={() => onDefer(row.occurrenceId)}
                    >
                      Snooze
                    </button>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
      {(week.loans ?? []).length > 0 ? (
        <div className="table-wrap">
          <h3>Loan payments</h3>
          <table aria-label="Week ahead loan payments">
            <thead>
              <tr>
                <th>Date</th>
                <th>Loan</th>
                <th className="numeric">Payment</th>
                <th className="numeric">Principal</th>
                <th className="numeric">Interest</th>
                <th>Confirm</th>
              </tr>
            </thead>
            <tbody>
              {(week.loans ?? []).map((loan) => (
                <LoanPayRow
                  key={`${loan.accountId}-${loan.dueOn}`}
                  loan={loan}
                  scale={scale}
                  busy={busy}
                  pending={pendingId === loan.accountId}
                  onConfirm={onConfirmLoan}
                />
              ))}
            </tbody>
          </table>
        </div>
      ) : null}
      </BusySurface>
    </section>
  );
}

function LoanPayRow({
  loan,
  scale,
  busy,
  pending,
  onConfirm,
}: {
  loan: LoanWeekRow;
  scale: number;
  busy?: boolean;
  pending: boolean;
  onConfirm: (loan: {
    accountId: string;
    dueOn: string;
    principalMinor: number;
    interestMinor: number;
  }) => void;
}) {
  const paymentMinor = loan.paymentMinor;
  const projectedInterest = formatUsd(loan.interestMinor, loan.scale ?? scale).replace(
    /[$,]/g,
    "",
  );
  const [interest, setInterest] = useState(projectedInterest);
  const parse = (text: string) => {
    const value = Number(text.replace(/[$,]/g, ""));
    if (!Number.isFinite(value) || value < 0) return null;
    return Math.round(value * 100);
  };
  const interestMinor = parse(interest);
  const principalMinor = interestMinor == null ? null : paymentMinor - interestMinor;
  const ready = principalMinor != null && principalMinor >= 0;
  return (
    <tr>
      <td>{loan.dueOn}</td>
      <td>{loan.name}</td>
      <td className="numeric">{formatUsd(paymentMinor, loan.scale ?? scale)}</td>
      <td className="numeric" aria-label={`${loan.name} principal`}>
        {ready ? formatUsd(principalMinor, loan.scale ?? scale) : "—"}
      </td>
      <td>
        <input
          aria-label={`${loan.name} interest`}
          inputMode="decimal"
          value={interest}
          onChange={(event) => setInterest(event.target.value)}
        />
      </td>
      <td>
        <button
          type="button"
          aria-label={`Confirm ${loan.name} loan payment`}
          disabled={busy || pending || !ready}
          className={interest !== projectedInterest ? "is-unsaved" : undefined}
          onClick={() => {
            if (interestMinor == null || principalMinor == null || principalMinor < 0) return;
            onConfirm({
              accountId: loan.accountId,
              dueOn: loan.dueOn,
              principalMinor,
              interestMinor,
            });
          }}
        >
          Confirm
        </button>
      </td>
    </tr>
  );
}
