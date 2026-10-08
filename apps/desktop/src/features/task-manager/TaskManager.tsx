import type {
  AccountListItem,
  MagiProjection,
  TaskListGet,
  TaskRecord,
  TaskRule,
  TaskRuleListGet,
  TaxPlanningGet,
} from "@finos/app-contracts";
import { useCallback, useEffect, useState } from "react";
import {
  APPLICATION_APTC_MINOR,
  syncMagiCliffTask,
} from "../cash/magiForecast";
import { MagiCliffPanel } from "./MagiCliffPanel";
import { domainLabel, ReminderTable, taskReminderRows } from "./ReminderTable";

function saturdayOf(iso: string): string {
  const [y, m, d] = iso.slice(0, 10).split("-").map(Number);
  const date = new Date(Date.UTC(y, m - 1, d));
  const day = date.getUTCDay();
  const back = (day + 1) % 7;
  date.setUTCDate(date.getUTCDate() - back);
  return date.toISOString().slice(0, 10);
}

function taskRuleText(code: string): string {
  if (code.startsWith("plan_under:")) return "Under Plan for 3 periods in a row.";
  if (code.startsWith("plan_over:")) return "Above Plan for 5 periods in a row.";
  if (code === "magi_cliff_over") return "MAGI cliff.";
  return "Reminder. No rule.";
}

const TICKET_FUNCTIONS = [
  {
    code: "declaration_retrieve_miss",
    tool: "retry_retrieve",
    purpose: "The issuer page did not return a declaration.",
  },
  {
    code: "declaration_retrieve_timeout",
    tool: "retry_retrieve",
    purpose: "The issuer page timed out.",
  },
  {
    code: "declaration_parse_unstable",
    tool: "retry_retrieve",
    purpose: "The page parse did not settle.",
  },
  {
    code: "parse_miss",
    tool: "retry_retrieve",
    purpose: "The page did not parse.",
  },
  {
    code: "declaration_stored_mismatch",
    tool: "retry_retrieve",
    purpose: "The page disagrees with the stored declaration.",
  },
  {
    code: "declaration_history_dropped",
    tool: "retry_retrieve",
    purpose: "Stored declaration history was dropped.",
  },
  {
    code: "declaration_lookback_short",
    tool: "set_inception",
    purpose: "The lookback is shorter than inception.",
  },
  {
    code: "declaration_cadence_mismatch",
    tool: "lock_cadence",
    purpose: "The pay cadence does not match the stored frequency.",
  },
  {
    code: "declaration_cadence_spacing",
    tool: "lock_cadence",
    purpose: "Pay dates are spaced off the stored frequency.",
  },
  {
    code: "declaration_amount_variation",
    tool: "amount_confirm",
    purpose: "The newest pay is more than 30% from the previous pay. Except keeps the issuer amount. Reject leaves the stored amount.",
  },
  {
    code: "declaration_plan_mismatch",
    tool: "plan_vs_declaration",
    purpose: "The newest pay is more than 30% from Plan. Over closes when the deposit matches. Under asks for a new Plan.",
  },
  {
    code: "div_type",
    tool: "set_div_type",
    purpose: "DIV-1 is empty.",
  },
  {
    code: "frequency",
    tool: "lock_cadence",
    purpose: "Payment frequency is empty.",
  },
  {
    code: "underlying",
    tool: "set_underlying",
    purpose: "Underlying is empty.",
  },
  {
    code: "provider",
    tool: "set_provider",
    purpose: "Provider is empty.",
  },
  {
    code: "risk_tier",
    tool: "set_risk",
    purpose: "Risk tier is empty.",
  },
  {
    code: "roc_estimate",
    tool: "run_roc",
    purpose: "Current-year ROC percent is missing.",
  },
  {
    code: "roc_pct_change",
    tool: "roc_confirm",
    purpose: "The live current-year ROC percent differs from the stored percent.",
  },
  {
    code: "remaining_year",
    tool: "fix_remaining_year",
    purpose: "Remaining-year pay dates need a fix.",
  },
  {
    code: "paid_payable_supersede",
    tool: "fix_remaining_year",
    purpose: "A payable date was superseded.",
  },
  {
    code: "payable_date_moved",
    tool: "fix_remaining_year",
    purpose: "The payable date moved.",
  },
  {
    code: "sec_403",
    tool: "retry_retrieve",
    purpose: "The SEC page returned 403.",
  },
  {
    code: "mlp_sec_owner_amount",
    tool: "enter_declared_amount",
    purpose: "The owner must enter the declared amount.",
  },
  {
    code: "adapter_url_mismatch",
    tool: "retry_retrieve",
    purpose: "The adapter URL does not match the template.",
  },
  {
    code: "missing_seed_url",
    tool: "retry_retrieve",
    purpose: "The seed URL is missing.",
  },
  {
    code: "collector_establish_incomplete",
    tool: "establish_recertify",
    purpose: "Establish has not finished.",
  },
];

const BUILT_PLAN_RULES = [
  {
    code: "plan_under",
    title: "Plan amount is wrong and must be adjusted.",
    detail: "3 periods",
    domain: "Dividend Plan",
  },
  {
    code: "plan_over",
    title: "Validate Plan amount — paid above Plan for 5 periods.",
    detail: "5 periods",
    domain: "Dividend Plan",
  },
];

export function TaskManager({
  client,
  asOfDate,
  accounts,
  onOpenSymbol,
  magiTaskId,
  onOpenMagi,
  onCloseMagi,
  onCutDraws,
}: {
  client: {
    executeQuery: (name: string, body?: unknown) => Promise<{ ok: boolean; bodyJson?: string }>;
    executeCommand: (name: string, body?: unknown) => Promise<{ ok: boolean; errorCode?: string }>;
  };
  asOfDate: string;
  accounts?: AccountListItem[];
  onOpenSymbol?: (symbol: string) => void;
  magiTaskId?: string | null;
  onOpenMagi?: (taskId: string) => void;
  onCloseMagi?: () => void;
  onCutDraws?: () => void;
}) {
  const weekStart = saturdayOf(asOfDate);
  const [open, setOpen] = useState<TaskRecord[]>([]);
  const [ignored, setIgnored] = useState<TaskRecord[]>([]);
  const [done, setDone] = useState<TaskRecord[]>([]);
  const [title, setTitle] = useState("");
  const [dueOn, setDueOn] = useState(asOfDate.slice(0, 10));
  const [pending, setPending] = useState<string | null>(null);
  const [addDirty, setAddDirty] = useState(false);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [rules, setRules] = useState<TaskRule[]>([]);

  const reload = useCallback(async () => {
    const [all, ruleList] = await Promise.all([
      client.executeQuery("TaskList"),
      client.executeQuery("TaskRuleList"),
    ]);
    if (ruleList.ok && ruleList.bodyJson) {
      const body = JSON.parse(ruleList.bodyJson) as TaskRuleListGet;
      setRules(body.items);
    }
    if (all.ok && all.bodyJson) {
      const body = JSON.parse(all.bodyJson) as TaskListGet;
      setOpen(
        body.items.filter((t) => t.status === "open" && t.weekStart === weekStart),
      );
      setIgnored(body.items.filter((t) => t.status === "ignored_until"));
      setDone(
        body.items.filter((t) => t.status === "done" && t.weekStart === weekStart),
      );
    }
  }, [client, weekStart]);

  useEffect(() => {
    void (async () => {
      const [tax, magi] = await Promise.all([
        client.executeQuery("TaxPlanningGet", { asOfDate }),
        client.executeQuery("MagiProjectionGet"),
      ]);
      if (tax.ok && tax.bodyJson) {
        const plan = JSON.parse(tax.bodyJson) as TaxPlanningGet;
        const projection = magi.ok && magi.bodyJson
          ? (JSON.parse(magi.bodyJson) as MagiProjection)
          : null;
        await syncMagiCliffTask(client, plan, projection, APPLICATION_APTC_MINOR);
      }
      await client.executeCommand("PlanSaturdayTaskSync", { asOfDate });
      await reload();
    })();
  }, [asOfDate, client, reload]);

  const addTask = async () => {
    const trimmed = title.trim();
    if (!trimmed || !dueOn) return;
    setPending("add");
    try {
      const r = await client.executeCommand("TaskAdd", {
        title: trimmed,
        domain: "manual",
        dueOn,
        note: "",
      });
      if (r.ok) {
        setTitle("");
        setAddDirty(false);
        await reload();
      }
    } finally {
      setPending(null);
    }
  };

  const resolve = async (taskId: string, how: "done" | "ignored_until") => {
    setPending(taskId);
    try {
      const ignoreUntil =
        how === "ignored_until"
          ? saturdayOf(
              new Date(
                Date.parse(`${weekStart}T00:00:00Z`) + 7 * 86400000,
              )
                .toISOString()
                .slice(0, 10),
            )
          : undefined;
      await client.executeCommand("TaskResolve", {
        taskId,
        how,
        ignoreUntil,
      });
      await reload();
    } finally {
      setPending(null);
    }
  };

  const renderRows = (rows: TaskRecord[], actions: boolean) => (
    <ReminderTable
      rows={taskReminderRows({
        rows,
        actions,
        pendingId: pending,
        selectedId,
        onSelect: setSelectedId,
        onOpenSymbol,
        onOpenMagi,
        onResolve: (taskId) => void resolve(taskId, "done"),
        onSnooze: (taskId) => void resolve(taskId, "ignored_until"),
      })}
    />
  );

  return (
    <section className="task-manager" aria-label="Task Manager">
      <h2>Task Manager</h2>
      <p>
        Tasks stay open and reminded until Resolve. Snooze hides a task until the next plan week
        (Sat–Fri). Rules can open tasks; owner can add tasks. Week Ahead lists open tasks for this
        plan week.
      </p>

      <h3 id="tasks-open" data-section="tasks-open">Open this week</h3>
      {magiTaskId && onCutDraws && onCloseMagi ? (
        <div data-part="magi-cliff-panel">
        <MagiCliffPanel
          client={client}
          taskId={magiTaskId}
          asOfDate={asOfDate}
          accounts={accounts ?? []}
          busy={pending != null}
          onCutDraws={onCutDraws}
          onClose={onCloseMagi}
          onPosted={() => void reload()}
        />
        </div>
      ) : null}
      <div data-part="open-tasks">{renderRows(open, true)}</div>

      <h3 id="tasks-snoozed" data-section="tasks-snoozed">Snoozed</h3>
      <div data-part="snoozed-tasks">{renderRows(ignored, false)}</div>

      <h3 id="tasks-done" data-section="tasks-done">Done (this week)</h3>
      <div data-part="done-tasks">{renderRows(done, false)}</div>

      <h3 id="tasks-rules" data-section="tasks-rules">Add/View Task</h3>
      <ul aria-label="Built task rules" data-part="built-task-rules">
        {rules
          .filter((rule) => rule.code === "magi_cliff_over")
          .map((rule) => (
            <li key={rule.code}>
              {rule.code}: {rule.title}. {rule.cadence}. {domainLabel(rule.domain)}.
            </li>
          ))}
        {rules.some((rule) => rule.code === "magi_cliff_over") ? null : (
          <li>magi_cliff_over: Resolve MAGI cliff gap. weekly. magi.</li>
        )}
        {BUILT_PLAN_RULES.map((rule) => (
          <li key={rule.code}>
            {rule.code}: {rule.title} {rule.detail}. {rule.domain}.
          </li>
        ))}
      </ul>
      <h3>Ticket functions</h3>
      <p>Tickets are collector exceptions. Tasks are weekly reminders.</p>
      <ul aria-label="Ticket functions" data-part="ticket-functions">
        {TICKET_FUNCTIONS.map((row) => (
          <li key={row.code}>
            {row.code} ({row.tool}): {row.purpose}
          </li>
        ))}
      </ul>
      {(() => {
        const selected = [...open, ...ignored, ...done].find((row) => row.taskId === selectedId);
        if (!selected) return null;
        let name = selected.title;
        if (selected.code.startsWith("plan_under:") || selected.code.startsWith("plan_over:")) {
          try {
            const payload = JSON.parse(selected.payloadJson) as { symbol?: string };
            if (payload.symbol) name = `${payload.symbol} — ${selected.title}`;
          } catch {
            /* stored title */
          }
        }
        return (
          <div className="task-view" aria-label="View task" data-part="view-task">
            <p>
              <span className="fact-label">Name</span> {name}
            </p>
            <p>
              <span className="fact-label">Rule</span> {taskRuleText(selected.code)}
            </p>
          </div>
        );
      })()}
      <div className="task-manager-add" data-part="add-task">
        <label>
          Title
          <input
            aria-label="Task title"
            value={title}
            onChange={(event) => {
              setTitle(event.target.value);
              setAddDirty(true);
            }}
          />
        </label>
        <label>
          Due date
          <input
            aria-label="Task due date"
            type="date"
            value={dueOn}
            onChange={(event) => {
              setDueOn(event.target.value);
              setAddDirty(true);
            }}
          />
        </label>
        <button
          type="button"
          aria-label="Add task"
          className={addDirty ? "is-unsaved" : undefined}
          disabled={!title.trim() || !dueOn || pending === "add"}
          onClick={() => void addTask()}
        >
          Add task
        </button>
      </div>
    </section>
  );
}
