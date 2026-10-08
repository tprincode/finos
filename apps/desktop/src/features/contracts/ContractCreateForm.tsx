import type { AccountRecord } from "@finos/app-contracts";
import { useEffect, useState } from "react";
import { todayIso } from "../interest-rate/math";
import { parseOcc } from "./parseOcc";

type Client = {
  executeQuery: (
    name: string,
    body?: unknown,
  ) => Promise<{ ok: boolean; bodyJson?: string; errorCode?: string }>;
  executeCommand: (
    name: string,
    body?: unknown,
  ) => Promise<{ ok: boolean; bodyJson?: string; errorCode?: string; errorMessage?: string }>;
};

function dollarsToMinor(raw: string): number | null {
  const text = raw.trim().replace(/[$,]/g, "");
  if (text === "") return null;
  const n = Number(text);
  if (!Number.isFinite(n)) return null;
  return Math.round(n * 100);
}

function iraAccount(name: string): boolean {
  return name === "Income" || name === "Speculation" || name === "9" || name === "Account 9";
}

type Preview = {
  pieces: { lotId: string; shares: number; costMinor: number }[];
  averageMinor: number | null;
  refused: string;
};

export function ContractCreateForm({
  client,
  pending,
  onCreate,
}: {
  client: Client;
  pending: string | null;
  onCreate: (body: Record<string, unknown>) => void;
}) {
  const [occSymbol, setOccSymbol] = useState("");
  const [side, setSide] = useState<"short" | "long">("short");
  const [qty, setQty] = useState("1");
  const [premium, setPremium] = useState("");
  const [prior, setPrior] = useState("");
  const [openOn, setOpenOn] = useState(todayIso());
  const [accountId, setAccountId] = useState("");
  const [accounts, setAccounts] = useState<AccountRecord[]>([]);
  const [preview, setPreview] = useState<Preview | null>(null);
  const [note, setNote] = useState<string | null>(null);

  const parsed = parseOcc(occSymbol);
  const right = parsed?.putCall === "P" ? "Put" : parsed?.putCall === "C" ? "Call" : "";

  useEffect(() => {
    void client.executeQuery("AccountList").then((result) => {
      if (!result.ok || !result.bodyJson) return;
      const rows = JSON.parse(result.bodyJson) as AccountRecord[];
      setAccounts(rows.filter((row) => iraAccount(row.name)));
    });
  }, [client]);

  const underlying = parsed?.underlying ?? "";
  const putCall = parsed?.putCall ?? "";
  useEffect(() => {
    if (!underlying || side !== "short" || putCall !== "C" || !accountId) {
      setPreview(null);
      return;
    }
    const quantity = Number(qty);
    if (!Number.isInteger(quantity) || quantity <= 0) return;
    void client
      .executeQuery("ContractCoverPreview", {
        accountId,
        symbol: underlying,
        quantity,
      })
      .then((result) => {
        if (!result.ok || !result.bodyJson) return;
        setPreview(JSON.parse(result.bodyJson) as Preview);
      });
  }, [client, underlying, putCall, side, accountId, qty]);

  const priorMinor = dollarsToMinor(prior);
  const premiumMinor = dollarsToMinor(premium);
  const warnBoth = priorMinor != null && priorMinor !== 0 && premiumMinor != null;

  return (
    <section
      aria-label="Create contract"
      id="contract-create"
      data-section="contract-create"
      data-part="contract-create"
    >
      <h3>Create contract</h3>
      <div className="table-wrap">
        <table aria-label="Create contract" className="contract-create">
          <thead>
            <tr>
              <th scope="col">OCC symbol</th>
              <th scope="col">Side</th>
              <th scope="col">Call or put</th>
              <th scope="col">Account</th>
              <th scope="col">Qty</th>
              <th scope="col">Open premium $</th>
              <th scope="col">Previously realized P/L $</th>
              <th scope="col">Open</th>
              <th scope="col"> </th>
            </tr>
          </thead>
          <tbody>
            <tr>
              <td>
                <input
                  aria-label="OCC symbol"
                  spellCheck={false}
                  value={occSymbol}
                  onChange={(e) => setOccSymbol(e.target.value.toUpperCase())}
                  placeholder=".TSLL1270115C20.7"
                />
              </td>
              <td>
                <select
                  aria-label="Contract side"
                  value={side}
                  onChange={(e) => setSide(e.target.value as "short" | "long")}
                >
                  <option value="short">Short Cover</option>
                  <option value="long">Long</option>
                </select>
              </td>
              <td aria-label="Call or put">{right}</td>
              <td>
                <select
                  aria-label="Contract account"
                  value={accountId}
                  onChange={(e) => setAccountId(e.target.value)}
                >
                  <option value="">Account</option>
                  {accounts.map((row) => (
                    <option key={row.accountId} value={row.accountId}>
                      {row.name}
                    </option>
                  ))}
                </select>
              </td>
              <td>
                <input
                  aria-label="Contract quantity"
                  className="contract-money"
                  inputMode="numeric"
                  value={qty}
                  onChange={(e) => setQty(e.target.value)}
                />
              </td>
              <td>
                <input
                  aria-label="Open premium dollars"
                  className="contract-money"
                  inputMode="decimal"
                  value={premium}
                  onChange={(e) => setPremium(e.target.value)}
                />
              </td>
              <td>
                <input
                  aria-label="Previously realized P/L dollars"
                  className="contract-money"
                  inputMode="decimal"
                  value={prior}
                  onChange={(e) => setPrior(e.target.value)}
                />
              </td>
              <td>
                <input
                  type="date"
                  aria-label="Open date"
                  className="contract-date"
                  value={openOn}
                  onChange={(e) => setOpenOn(e.target.value)}
                />
              </td>
              <td>
                <button
                  type="button"
                  aria-label="Create contract"
                  className={pending === "create" ? "is-unsaved" : undefined}
                  disabled={pending != null}
                  onClick={() => {
                    if (!parsed) {
                      setNote("Not an OCC symbol.");
                      return;
                    }
                    const quantity = Number(qty);
                    const openPremiumMinor = dollarsToMinor(premium);
                    if (!accountId || !Number.isInteger(quantity) || quantity <= 0) {
                      setNote("Account and quantity are required.");
                      return;
                    }
                    setNote(null);
                    onCreate({
                      occSymbol,
                      side,
                      quantity,
                      accountId,
                      ...(openPremiumMinor == null ? {} : { openPremiumMinor }),
                      priorBalanceMinor: priorMinor ?? 0,
                      openOn,
                    });
                    setOccSymbol("");
                    setPremium("");
                    setPrior("");
                  }}
                >
                  Create
                </button>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
      {warnBoth ? (
        <p role="status">Previously realized P/L does not post to this week.</p>
      ) : null}
      {note ? <p role="status">{note}</p> : null}
      {preview ? (
        <div aria-label="Cover preview">
          <p>
            Block average{" "}
            {preview.averageMinor != null ? `$${(preview.averageMinor / 100).toFixed(2)}` : "unknown"}
          </p>
          {preview.refused ? <p role="alert">{preview.refused}</p> : null}
          <ul>
            {preview.pieces.map((piece) => (
              <li key={`${piece.lotId}-${piece.shares}`}>
                {piece.shares} shares, cost ${(piece.costMinor / 100).toFixed(2)}
              </li>
            ))}
          </ul>
        </div>
      ) : null}
    </section>
  );
}
