import type { WeekAheadGet } from "@finos/app-contracts";

import type { LocalTauriFinanceClient } from "../../financeClient";

type LoanConfirm = {
  accountId: string;
  dueOn: string;
  principalMinor: number;
  interestMinor: number;
};

/**
 * Week Ahead task buttons. These handlers were dropped from App.tsx while WeekAheadPanel
 * still called them, so Resolve, Ignore, and Confirm loan threw on click. The bodies are
 * the ones last shipped, from the 2026-09-30 archive.
 */
export function weekAheadTaskHandlers(deps: {
  client: LocalTauriFinanceClient;
  asOf: string;
  periodStart: string | undefined;
  setPending: (id: string | null) => void;
  setBusy: (busy: boolean) => void;
  setMessage: (message: string) => void;
  setWeek: (week: WeekAheadGet) => void;
}) {
  const reload = async () => {
    const ahead = await deps.client.executeQuery("WeekAheadGet", { asOfDate: deps.asOf });
    if (ahead.ok && ahead.bodyJson) {
      deps.setWeek(JSON.parse(ahead.bodyJson) as WeekAheadGet);
    }
  };
  return {
    onMagiPosted: () => {
      void reload();
    },
    onResolveTask: (taskId: string) => {
      void (async () => {
        deps.setPending(taskId);
        deps.setBusy(true);
        try {
          const r = await deps.client.executeCommand("TaskResolve", { taskId, how: "done" });
          if (!r.ok) {
            deps.setMessage(`Task resolve failed: ${r.errorCode ?? "error"}`);
            return;
          }
          await reload();
        } finally {
          deps.setPending(null);
          deps.setBusy(false);
        }
      })();
    },
    onIgnoreTask: (taskId: string) => {
      void (async () => {
        deps.setPending(taskId);
        deps.setBusy(true);
        try {
          const start = deps.periodStart || deps.asOf;
          const next = new Date(Date.parse(`${start.slice(0, 10)}T00:00:00Z`) + 7 * 86400000)
            .toISOString()
            .slice(0, 10);
          const r = await deps.client.executeCommand("TaskResolve", {
            taskId,
            how: "ignored_until",
            ignoreUntil: next,
          });
          if (!r.ok) {
            deps.setMessage(`Task ignore failed: ${r.errorCode ?? "error"}`);
            return;
          }
          await reload();
        } finally {
          deps.setPending(null);
          deps.setBusy(false);
        }
      })();
    },
    onConfirmLoan: (loan: LoanConfirm) => {
      void (async () => {
        deps.setPending(loan.accountId);
        deps.setBusy(true);
        try {
          const r = await deps.client.executeCommand("LoanPaymentConfirm", {
            accountId: loan.accountId,
            dueOn: loan.dueOn,
            principalMinor: loan.principalMinor,
            interestMinor: loan.interestMinor,
          });
          if (!r.ok) {
            deps.setMessage(`Loan payment confirm failed: ${r.errorCode ?? "error"}`);
            return;
          }
          await reload();
        } finally {
          deps.setPending(null);
          deps.setBusy(false);
        }
      })();
    },
  };
}
