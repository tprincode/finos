import { useCallback, useState } from "react";
import type { DividendGet, MagiProjection } from "@finos/app-contracts";
import { RemoteHttpFinanceClient } from "./financeClient";

type HealthView = {
  status: string;
  contractVersion: string;
};

export default function App() {
  const [baseUrl, setBaseUrl] = useState("http://127.0.0.1:8787");
  const [accessToken, setAccessToken] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [health, setHealth] = useState<HealthView | null>(null);
  const [dividend, setDividend] = useState<DividendGet | null>(null);
  const [magi, setMagi] = useState<MagiProjection | null>(null);

  const load = useCallback(async () => {
    setError(null);
    const client = new RemoteHttpFinanceClient(baseUrl, accessToken);
    try {
      const healthResult = await client.executeQuery("HealthGet");
      if (!healthResult.ok) {
        throw new Error(healthResult.errorCode ?? "HealthGet failed");
      }
      const healthBody = JSON.parse(healthResult.bodyJson ?? "{}") as {
        status?: string;
      };
      setHealth({
        status: healthBody.status ?? "unknown",
        contractVersion: client.contractVersion(),
      });

      const dividendResult = await client.executeQuery("DividendGet");
      if (!dividendResult.ok) {
        throw new Error(dividendResult.errorCode ?? "DividendGet failed");
      }
      setDividend(JSON.parse(dividendResult.bodyJson ?? "{}") as DividendGet);

      const magiResult = await client.executeQuery("MagiProjectionGet");
      if (!magiResult.ok) {
        throw new Error(magiResult.errorCode ?? "MagiProjectionGet failed");
      }
      setMagi(JSON.parse(magiResult.bodyJson ?? "{}") as MagiProjection);
    } catch (err) {
      setHealth(null);
      setDividend(null);
      setMagi(null);
      setError(err instanceof Error ? err.message : String(err));
    }
  }, [accessToken, baseUrl]);

  return (
    <main className="container">
      <h1>finos web</h1>
      <p>Authenticated Health, Dividend, and MAGI against the central server. No SQL in this UI.</p>
      <label>
        Server
        <input value={baseUrl} onChange={(e) => setBaseUrl(e.target.value)} />
      </label>
      <label>
        Bearer token
        <input
          value={accessToken}
          onChange={(e) => setAccessToken(e.target.value)}
          type="password"
        />
      </label>
      <button type="button" onClick={() => void load()}>
        Load
      </button>
      {error ? <p className="error">{error}</p> : null}
      {health ? (
        <dl className="health">
          <dt>Health</dt>
          <dd>{health.status}</dd>
          <dt>Contract</dt>
          <dd>{health.contractVersion}</dd>
        </dl>
      ) : null}
      {dividend ? (
        <p>Dividend actual total (minor): {dividend.actualTotalMinor}</p>
      ) : null}
      {magi ? (
        <p>
          MAGI {magi.decisionState}; included {magi.actualIncludedYtd.amountMinor}; headroom{" "}
          {magi.protectedHeadroom.amountMinor}
        </p>
      ) : null}
    </main>
  );
}
