import { useEffect, useState } from "react";
import type { AccountRecord } from "@finos/app-contracts";
import { LocalTauriFinanceClient } from "../../financeClient";

const client = new LocalTauriFinanceClient();

type ManagedLoan = {
  accountId: string;
  name: string;
  accountName?: string;
  kind: string;
  currentMinor: number | null;
  startingMinor?: number | null;
  paymentMinor?: number | null;
  inactive?: boolean;
  registerKey: string;
  payProcess?: string | null;
  linkedElementId?: string | null;
};

type Tab = "brokerage" | "loans" | "escrow";

function money(minor: number | null | undefined): string {
  if (minor == null) return "—";
  return (minor / 100).toLocaleString("en-US", { style: "currency", currency: "USD" });
}

function dollarsInput(minor: number | null | undefined): string {
  if (minor == null) return "";
  return (minor / 100).toFixed(2);
}

function parseDollars(text: string): number | null {
  const trimmed = text.trim();
  if (!trimmed) return null;
  const value = Number(trimmed.replace(/[$,]/g, ""));
  if (!Number.isFinite(value)) return null;
  return Math.round(value * 100);
}

export function AccountManagement() {
  const [tab, setTab] = useState<Tab>("brokerage");
  const [brokerage, setBrokerage] = useState<AccountRecord[]>([]);
  const [loans, setLoans] = useState<ManagedLoan[]>([]);
  const [escrow, setEscrow] = useState<ManagedLoan[]>([]);
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState<string | null>(null);
  const [editId, setEditId] = useState<string | null>(null);
  const [cashSymbol, setCashSymbol] = useState("");
  const [brokerNumber, setBrokerNumber] = useState("");
  const [minBalance, setMinBalance] = useState("");

  const load = async () => {
    setBusy(true);
    try {
      const [acct, managed] = await Promise.all([
        client.executeQuery("AccountList", {}),
        client.executeQuery("ExternalAccountManagerGet", {}),
      ]);
      if (acct.ok && acct.bodyJson) {
        const parsed = JSON.parse(acct.bodyJson) as AccountRecord[];
        setBrokerage(Array.isArray(parsed) ? parsed : []);
      }
      if (managed.ok && managed.bodyJson) {
        const body = JSON.parse(managed.bodyJson) as { accounts?: ManagedLoan[] };
        const rows = body.accounts ?? [];
        setLoans(rows.filter((row) => row.kind !== "credit"));
        setEscrow(rows.filter((row) => row.kind === "credit"));
      }
      setStatus(null);
    } catch (err: unknown) {
      setStatus(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  };

  useEffect(() => {
    void load();
  }, []);

  const startEdit = (account: AccountRecord) => {
    setEditId(account.accountId);
    setCashSymbol(account.cashSymbol ?? "");
    setBrokerNumber(account.brokerAccountNumber ?? "");
    setMinBalance(dollarsInput(account.minBalanceTargetMinor ?? null));
  };

  const saveBrokerage = async () => {
    if (!editId) return;
    const target =
      minBalance.trim() === "" ? null : parseDollars(minBalance);
    if (minBalance.trim() && target == null) {
      setStatus("Enter min balance as dollars, for example 2500.00.");
      return;
    }
    setBusy(true);
    try {
      const result = await client.executeCommand("AccountUpdate", {
        accountId: editId,
        cashSymbol: cashSymbol.trim(),
        brokerAccountNumber: brokerNumber.trim(),
        minBalanceTargetMinor: target,
      });
      if (!result.ok) {
        setStatus(`Save failed: ${result.errorCode ?? "error"}`);
        return;
      }
      setEditId(null);
      await load();
      setStatus("Brokerage account saved.");
    } finally {
      setBusy(false);
    }
  };

  const setLoanActive = async (loan: ManagedLoan, active: boolean) => {
    setBusy(true);
    try {
      const result = await client.executeCommand("ExternalAccountManagerSave", {
        accounts: [
          {
            accountId: loan.accountId,
            name: loan.name,
            accountName: loan.accountName ?? loan.name,
            startingMinor: loan.startingMinor ?? null,
            currentMinor: loan.currentMinor,
            paymentMinor: loan.paymentMinor ?? null,
            registerKey: loan.registerKey || loan.name,
            payProcess: loan.payProcess || "element",
            linkedElementId: loan.linkedElementId ?? null,
            inactive: !active,
          },
        ],
      });
      if (!result.ok) {
        setStatus(`Loan update failed: ${result.errorCode ?? "error"}`);
        return;
      }
      await load();
      setStatus(active ? `${loan.name} is active on Debt planner.` : `${loan.name} marked inactive.`);
    } finally {
      setBusy(false);
    }
  };

  return (
    <section className="account-management" aria-label="Account Management">
      <h2>Account Management</h2>
      <p className="account-management-lead">
        Brokerage, loan, and escrow lists stay separate. Inactive loans leave the Debt planner list
        but remain here so you can turn Active back on.
      </p>
      {status ? (
        <p role="status" aria-live="polite">
          {status}
        </p>
      ) : null}
      <div
        className="account-management-tabs"
        role="tablist"
        aria-label="Account lists"
        id="accounts-tabs"
        data-section="accounts-tabs"
        data-part="account-lists"
      >
        {(
          [
            ["brokerage", "Brokerage"],
            ["loans", "Loans"],
            ["escrow", "Escrow"],
          ] as const
        ).map(([id, label]) => (
          <button
            key={id}
            type="button"
            role="tab"
            aria-selected={tab === id}
            disabled={busy}
            onClick={() => setTab(id)}
          >
            {label}
          </button>
        ))}
      </div>

      {tab === "brokerage" ? (
        <div
          className="table-wrap"
          id="accounts-brokerage"
          data-section="accounts-brokerage"
          data-part="brokerage-accounts"
        >
          <table aria-label="Brokerage accounts">
            <thead>
              <tr>
                <th>Name</th>
                <th>Kind</th>
                <th>Cash symbol</th>
                <th>Broker account #</th>
                <th className="num">Min balance target</th>
                <th />
              </tr>
            </thead>
            <tbody>
              {brokerage.map((account) => {
                const editing = editId === account.accountId;
                return (
                  <tr key={account.accountId}>
                    <td>{account.name}</td>
                    <td>{account.kind}</td>
                    <td>
                      {editing ? (
                        <input
                          aria-label={`Cash symbol ${account.name}`}
                          value={cashSymbol}
                          onChange={(event) => setCashSymbol(event.target.value)}
                        />
                      ) : (
                        account.cashSymbol || "—"
                      )}
                    </td>
                    <td>
                      {editing ? (
                        <input
                          aria-label={`Broker account number ${account.name}`}
                          value={brokerNumber}
                          onChange={(event) => setBrokerNumber(event.target.value)}
                        />
                      ) : (
                        account.brokerAccountNumber || "—"
                      )}
                    </td>
                    <td className="num">
                      {editing ? (
                        <input
                          aria-label={`Min balance target ${account.name}`}
                          inputMode="decimal"
                          value={minBalance}
                          onChange={(event) => setMinBalance(event.target.value)}
                        />
                      ) : (
                        money(account.minBalanceTargetMinor ?? null)
                      )}
                    </td>
                    <td>
                      {editing ? (
                        <>
                          <button type="button" disabled={busy} onClick={() => void saveBrokerage()}>
                            Save
                          </button>
                          <button
                            type="button"
                            disabled={busy}
                            onClick={() => setEditId(null)}
                          >
                            Cancel
                          </button>
                        </>
                      ) : (
                        <button
                          type="button"
                          disabled={busy}
                          aria-label={`Edit ${account.name}`}
                          onClick={() => startEdit(account)}
                        >
                          Edit
                        </button>
                      )}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      ) : null}

      {tab === "loans" ? (
        <div
          className="table-wrap"
          id="accounts-loan"
          data-section="accounts-loan"
          data-part="loan-accounts"
        >
          <table aria-label="Loan accounts">
            <thead>
              <tr>
                <th>Account Name</th>
                <th>Loan Name</th>
                <th className="num">Current</th>
                <th>Active</th>
              </tr>
            </thead>
            <tbody>
              {loans.map((loan) => {
                const active = !loan.inactive;
                return (
                  <tr key={loan.accountId}>
                    <td>{(loan.accountName ?? "").trim() || loan.name}</td>
                    <td>{loan.name}</td>
                    <td className="num">{money(loan.currentMinor)}</td>
                    <td>
                      <label>
                        <input
                          type="checkbox"
                          aria-label={`Active loan ${loan.name}`}
                          checked={active}
                          disabled={busy}
                          onChange={(event) => void setLoanActive(loan, event.target.checked)}
                        />
                        Active
                      </label>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      ) : null}

      {tab === "escrow" ? (
        <div
          className="table-wrap"
          id="accounts-escrow"
          data-section="accounts-escrow"
          data-part="escrow-accounts"
        >
          <table aria-label="Escrow accounts">
            <thead>
              <tr>
                <th>Name</th>
                <th>Register key</th>
                <th className="num">Current</th>
              </tr>
            </thead>
            <tbody>
              {escrow.map((row) => (
                <tr key={row.accountId}>
                  <td>{row.name}</td>
                  <td>{row.registerKey}</td>
                  <td className="num">{money(row.currentMinor)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      ) : null}
    </section>
  );
}
