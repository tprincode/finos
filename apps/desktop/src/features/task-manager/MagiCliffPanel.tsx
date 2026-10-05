import { useEffect, useState } from "react";
import { formatUsd } from "@finos/ui-components";
import type { AccountListItem, TaskListGet, TaskRecord } from "@finos/app-contracts";

const CONTRIBUTION_ACCOUNTS = ["Income", "Speculation", "Account 9"] as const;

type Client = {
  executeQuery: (name: string, body?: unknown) => Promise<{ ok: boolean; bodyJson?: string }>;
  executeCommand: (
    name: string,
    body?: unknown,
  ) => Promise<{ ok: boolean; errorCode?: string }>;
};

type MagiPayload = {
  overageMinor?: number;
};

function parseDollars(raw: string): number | null {
  const t = raw.trim().replace(/[$,]/g, "");
  if (!t) return null;
  const n = Number(t);
  if (!Number.isFinite(n) || n <= 0) return null;
  return Math.round(n * 100);
}

export function MagiCliffPanel({
  client,
  taskId,
  asOfDate,
  accounts,
  busy,
  onCutDraws,
  onClose,
  onPosted,
}: {
  client: Client;
  taskId: string;
  asOfDate: string;
  accounts: AccountListItem[];
  busy?: boolean;
  onCutDraws: () => void;
  onClose: () => void;
  onPosted: () => void;
}) {
  const [task, setTask] = useState<TaskRecord | null>(null);
  const [amount, setAmount] = useState("");
  const [seeded, setSeeded] = useState(false);
  const [accountName, setAccountName] = useState("");
  const [occurredOn, setOccurredOn] = useState(asOfDate.slice(0, 10));
  const [pending, setPending] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      const listed = await client.executeQuery("TaskList");
      if (!listed.ok || !listed.bodyJson || cancelled) return;
      const body = JSON.parse(listed.bodyJson) as TaskListGet;
      const found = body.items.find((row) => row.taskId === taskId) ?? null;
      if (!cancelled) setTask(found);
    })();
    return () => {
      cancelled = true;
    };
  }, [client, taskId]);

  let overageMinor: number | null = null;
  if (task) {
    try {
      const payload = JSON.parse(task.payloadJson) as MagiPayload;
      if (typeof payload.overageMinor === "number") overageMinor = payload.overageMinor;
    } catch {
      overageMinor = null;
    }
  }

  useEffect(() => {
    if (!task || seeded || overageMinor == null) return;
    setAmount((overageMinor / 100).toFixed(2));
    setSeeded(true);
  }, [task, seeded, overageMinor]);

  const offered = accounts.filter((row) =>
    CONTRIBUTION_ACCOUNTS.some((name) => name.toLowerCase() === row.name.toLowerCase()),
  );
  const selected = accountName || offered[0]?.name || "";

  const confirmContribution = async () => {
    const amountMinor = parseDollars(amount);
    if (amountMinor == null || !selected || !occurredOn) return;
    setPending(true);
    setError("");
    try {
      const result = await client.executeCommand("MagiIraContributionPost", {
        taskId,
        accountName: selected,
        amountMinor,
        occurredOn,
      });
      if (!result.ok) {
        setError(result.errorCode || "contribution failed");
        return;
      }
      onPosted();
      onClose();
    } finally {
      setPending(false);
    }
  };

  return (
    <section className="magi-cliff-ticket" aria-label="MAGI ticket">
      <h3>
        MAGI over cliff
        {overageMinor == null ? "" : ` ${formatUsd(overageMinor)}`}
      </h3>
      <form
        className="magi-cliff-row"
        aria-label="Traditional IRA contribution"
        onSubmit={(event) => {
          event.preventDefault();
          void confirmContribution();
        }}
      >
        <span>IRA Contribution</span>
        <input
          aria-label="Contribution amount"
          size={8}
          value={amount}
          onChange={(event) => setAmount(event.target.value)}
          inputMode="decimal"
        />
        <select
          aria-label="Contribution account"
          value={selected}
          onChange={(event) => setAccountName(event.target.value)}
        >
          {offered.map((row) => (
            <option key={row.accountId} value={row.name}>
              {row.name}
            </option>
          ))}
        </select>
        <input
          aria-label="Contribution date"
          type="date"
          value={occurredOn}
          onChange={(event) => setOccurredOn(event.target.value)}
        />
        <button
          type="submit"
          aria-label="Confirm Traditional IRA contribution"
          disabled={busy || pending || parseDollars(amount) == null || !selected}
        >
          Confirm
        </button>
        <button type="button" onClick={onClose}>
          Close
        </button>
      </form>
      {error ? <p role="alert">{error}</p> : null}
      <div className="magi-cliff-row">
        <span>Cut remaining IRA draws</span>
        <button type="button" aria-label="Cut remaining Traditional IRA draws" onClick={onCutDraws}>
          Cut draws
        </button>
      </div>
    </section>
  );
}

export function MagiDrawHead({
  taskId,
  client,
  asOfDate,
  accounts,
  busy,
  onCutDraws,
  onClose,
  onPosted,
}: {
  taskId: string | null;
  client: Client;
  asOfDate: string;
  accounts: AccountListItem[];
  busy?: boolean;
  onCutDraws: () => void;
  onClose: () => void;
  onPosted: () => void;
}) {
  if (!taskId) return null;
  return (
    <MagiCliffPanel
      client={client}
      taskId={taskId}
      asOfDate={asOfDate}
      accounts={accounts}
      busy={busy}
      onCutDraws={onCutDraws}
      onClose={onClose}
      onPosted={onPosted}
    />
  );
}
