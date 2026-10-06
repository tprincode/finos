import { formatUsd } from "@finos/ui-components";
import type { OptionContractListGet, OptionContractRecord } from "@finos/app-contracts";
import { useCallback, useEffect, useState } from "react";
import { todayIso } from "../interest-rate/math";
import { assignmentFlags, nearStrike } from "./assignment";
import { ContractCreateForm } from "./ContractCreateForm";
import { parseOcc } from "./parseOcc";
import { rollYield } from "./rollYield";

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

function sideLabel(side: string, putCall: string): string {
  if (side === "short") return "Short Cover";
  const right = putCall === "P" ? "put" : "call";
  return `Long ${right}`;
}

export function ContractPositions({ client }: { client?: Client | null }) {
  const [items, setItems] = useState<OptionContractRecord[]>([]);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [status, setStatus] = useState<string | null>(null);
  const [pending, setPending] = useState<string | null>(null);

  const [rollForId, setRollForId] = useState<string | null>(null);
  const [rollOcc, setRollOcc] = useState("");
  const [rollClosePremium, setRollClosePremium] = useState("");
  const [rollOpenPremium, setRollOpenPremium] = useState("");
  const [rollOpenOn, setRollOpenOn] = useState(todayIso());

  const [closeForId, setCloseForId] = useState<string | null>(null);
  const [closePremium, setClosePremium] = useState("");
  const [closeOn, setCloseOn] = useState(todayIso());
  const [closeHow, setCloseHow] = useState<"closed" | "assigned">("closed");

  const asOf = todayIso();

  const reload = useCallback(async () => {
    if (!client) {
      setLoadError("Finance client is missing. Open Contract positions from the desktop host.");
      setItems([]);
      return;
    }
    const result = await client.executeQuery("ContractList", {});
    if (!result.ok || !result.bodyJson) {
      setLoadError(`ContractList failed: ${result.errorCode ?? "error"}`);
      setItems([]);
      return;
    }
    const body = JSON.parse(result.bodyJson) as OptionContractListGet;
    setItems(body.items ?? []);
    setLoadError(null);
  }, [client]);

  useEffect(() => {
    void reload();
  }, [reload]);

  const runCommand = async (key: string, name: string, body: unknown) => {
    if (!client) {
      setStatus("Finance client is missing.");
      return;
    }
    setPending(key);
    setStatus(null);
    try {
      const result = await client.executeCommand(name, body);
      if (!result.ok) {
        setStatus(result.errorCode || `${name} failed`);
        return;
      }
      await reload();
      setRollForId(null);
      setCloseForId(null);
    } catch (err: unknown) {
      setStatus(err instanceof Error ? err.message : String(err));
    } finally {
      setPending(null);
    }
  };

  const openRows = items.filter((r) => r.status === "open");
  const closedRows = items.filter((r) => r.status !== "open");

  if (!client) {
    return (
      <section className="interest-rate-page" aria-label="Contract positions">
        <h2>Contract positions</h2>
        <p role="alert">
          Finance client is missing. Open Contract positions from the desktop host.
        </p>
      </section>
    );
  }

  return (
    <section className="interest-rate-page" aria-label="Contract positions">
      <h2>Contract positions</h2>
      <p className="interest-rate-sub">
        Paste an OCC symbol, book the open premium in dollars (stored as cents), and keep
        quotes for DTE / assignment. Rows survive restart.
      </p>
      {loadError ? <p role="alert">{loadError}</p> : null}
      {status ? <p role="status">{status}</p> : null}

      <ContractCreateForm
        client={client}
        pending={pending}
        onCreate={(body) => void runCommand("create", "ContractCreate", body)}
      />

      <h3>Open contracts · {openRows.length}</h3>
      <div className="table-wrap">
        <table aria-label="Open contracts">
          <thead>
            <tr>
              <th scope="col">Symbol</th>
              <th scope="col">Underlying</th>
              <th scope="col">Expiry</th>
              <th scope="col">Side</th>
              <th scope="col">Qty</th>
              <th scope="col">Strike</th>
              <th scope="col">Open premium</th>
              <th scope="col">DTE</th>
              <th scope="col">ITM</th>
              <th scope="col">Assign risk</th>
              <th scope="col">Period %</th>
              <th scope="col">Annual</th>
              <th scope="col">Actions</th>
            </tr>
          </thead>
          <tbody>
            {openRows.map((row) => {
              const right = row.putCall === "P" ? "put" : "call";
              const live = row.liveUnderlyingMinor ?? null;
              const flags = assignmentFlags({
                right,
                strikeMinor: row.strikeMinor,
                asOfIso: asOf,
                expiryIso: row.expiryOn,
                underlyingLastMinor: live,
              });
              const yellow = nearStrike(live, row.strikeMinor);
              const collateralMinor = row.strikeMinor * 100 * row.quantity;
              const yieldProj = rollYield({
                premium: row.openPremiumMinor,
                collateral: collateralMinor,
                startIso: row.openOn,
                endIso: row.expiryOn,
              });
              return (
                <tr key={row.contractId} className={yellow ? "contract-near" : undefined}>
                  <td>{row.occSymbol}</td>
                  <td>{live != null ? formatUsd(live, 2) : "unknown"}</td>
                  <td>{row.expiryOn}</td>
                  <td>{sideLabel(row.side, row.putCall)}</td>
                  <td>{row.quantity}</td>
                  <td>{formatUsd(row.strikeMinor, row.scale)}</td>
                  <td>{row.openPremiumBlank ? "" : formatUsd(row.openPremiumMinor, 2)}</td>
                  <td>{flags.dte != null ? String(flags.dte) : "—"}</td>
                  <td>{flags.itm ? "Yes" : "No"}</td>
                  <td>{flags.assignmentRisk ? "Yes" : "No"}</td>
                  <td>
                    {yieldProj
                      ? `${(yieldProj.period * 100).toFixed(2)}%`
                      : "—"}
                  </td>
                  <td>
                    {yieldProj
                      ? `${(yieldProj.annual * 100).toFixed(2)}%`
                      : "—"}
                  </td>
                  <td>
                    <button
                      type="button"
                      aria-label={`Roll ${row.occSymbol}`}
                      className={pending === `roll-${row.contractId}` ? "is-unsaved" : undefined}
                      disabled={pending != null}
                      onClick={() => {
                        setRollForId(row.contractId);
                        setRollOcc("");
                        setRollClosePremium("");
                        setRollOpenPremium("");
                        setRollOpenOn(todayIso());
                      }}
                    >
                      Roll
                    </button>{" "}
                    <button
                      type="button"
                      aria-label={`Close ${row.occSymbol}`}
                      className={pending === `close-${row.contractId}` ? "is-unsaved" : undefined}
                      disabled={pending != null}
                      onClick={() => {
                        setCloseForId(row.contractId);
                        setClosePremium("");
                        setCloseOn(todayIso());
                        setCloseHow("closed");
                      }}
                    >
                      Close
                    </button>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>

      {rollForId ? (
        <div className="buttons" aria-label="Roll contract">
          <input
            aria-label="Roll new OCC symbol"
            value={rollOcc}
            onChange={(e) => setRollOcc(e.target.value.toUpperCase())}
            placeholder="New OCC"
          />
          <input
            aria-label="Roll close premium dollars"
            value={rollClosePremium}
            onChange={(e) => setRollClosePremium(e.target.value)}
            placeholder="Close premium $"
          />
          <input
            aria-label="Roll new open premium dollars"
            value={rollOpenPremium}
            onChange={(e) => setRollOpenPremium(e.target.value)}
            placeholder="New open premium $"
          />
          <input
            type="date"
            aria-label="Roll open date"
            value={rollOpenOn}
            onChange={(e) => setRollOpenOn(e.target.value)}
          />
          <button
            type="button"
            aria-label="Confirm roll"
            className={pending === `roll-${rollForId}` ? "is-unsaved" : undefined}
            disabled={pending != null}
            onClick={() => {
              if (!parseOcc(rollOcc)) {
                setStatus("Not an OCC symbol.");
                return;
              }
              const closePremiumMinor = dollarsToMinor(rollClosePremium);
              const newOpenPremiumMinor = dollarsToMinor(rollOpenPremium);
              if (closePremiumMinor == null || newOpenPremiumMinor == null) {
                setStatus("Roll premiums are required.");
                return;
              }
              void runCommand(`roll-${rollForId}`, "ContractRoll", {
                contractId: rollForId,
                newOccSymbol: rollOcc,
                closePremiumMinor,
                newOpenPremiumMinor,
                newOpenOn: rollOpenOn,
              });
            }}
          >
            Confirm roll
          </button>
          <button type="button" onClick={() => setRollForId(null)}>
            Cancel
          </button>
        </div>
      ) : null}

      {closeForId ? (
        <div className="buttons" aria-label="Close contract">
          <select
            aria-label="Close how"
            value={closeHow}
            onChange={(e) => setCloseHow(e.target.value as "closed" | "assigned")}
          >
            <option value="closed">closed</option>
            <option value="assigned">assigned</option>
          </select>
          <input
            aria-label="Close premium dollars"
            value={closePremium}
            onChange={(e) => setClosePremium(e.target.value)}
            placeholder="Close premium $"
          />
          <input
            type="date"
            aria-label="Closed on"
            value={closeOn}
            onChange={(e) => setCloseOn(e.target.value)}
          />
          <button
            type="button"
            aria-label="Confirm close"
            className={pending === `close-${closeForId}` ? "is-unsaved" : undefined}
            disabled={pending != null}
            onClick={() =>
              void runCommand(`close-${closeForId}`, "ContractClose", {
                contractId: closeForId,
                closePremiumMinor: dollarsToMinor(closePremium),
                closedOn: closeOn,
                how: closeHow,
              })
            }
          >
            Confirm close
          </button>
          <button type="button" onClick={() => setCloseForId(null)}>
            Cancel
          </button>
        </div>
      ) : null}

      <h3>Closed contracts · {closedRows.length}</h3>
      <div className="table-wrap">
        <table aria-label="Closed contracts">
          <thead>
            <tr>
              <th scope="col">Symbol</th>
              <th scope="col">Status</th>
              <th scope="col">Underlying</th>
              <th scope="col">Expiry</th>
              <th scope="col">Side</th>
              <th scope="col">Qty</th>
              <th scope="col">Open premium</th>
              <th scope="col">Close premium</th>
              <th scope="col">Closed on</th>
            </tr>
          </thead>
          <tbody>
            {closedRows.map((row) => (
              <tr key={row.contractId}>
                <td>{row.occSymbol}</td>
                <td>{row.status}</td>
                <td>{row.underlying}</td>
                <td>{row.expiryOn}</td>
                <td>{sideLabel(row.side, row.putCall)}</td>
                <td>{row.quantity}</td>
                <td>{row.openPremiumBlank ? "" : formatUsd(row.openPremiumMinor, 2)}</td>
                <td>
                  {row.closePremiumMinor != null
                    ? formatUsd(row.closePremiumMinor, 2)
                    : "—"}
                </td>
                <td>{row.closedOn || "—"}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </section>
  );
}
