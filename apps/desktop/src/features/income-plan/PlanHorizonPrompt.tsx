import { useEffect, useState } from "react";
import { LocalTauriFinanceClient } from "../../financeClient";

const client = new LocalTauriFinanceClient();

type HorizonName = {
  symbol: string;
  holes: string[];
};

type HorizonGet = {
  assumeYear: number;
  holeCount: number;
  names: HorizonName[];
};

export function PlanHorizonPrompt({
  asOfDate,
  disabled,
  onConfirmed,
}: {
  asOfDate: string;
  disabled?: boolean;
  onConfirmed?: () => void;
}) {
  const [horizon, setHorizon] = useState<HorizonGet | null>(null);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    void client
      .executeQuery("PlanHorizonGet", { asOfDate })
      .then((result) => {
        if (cancelled || !result.ok || !result.bodyJson) {
          return;
        }
        const body = JSON.parse(result.bodyJson) as HorizonGet;
        setHorizon(body);
      })
      .catch(() => {
        if (!cancelled) {
          setHorizon(null);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [asOfDate]);

  if (!horizon || horizon.holeCount < 1) {
    return null;
  }

  const names = horizon.names
    .map((n) => n.symbol)
    .slice(0, 8)
    .join(", ");
  const extra =
    horizon.names.length > 8 ? ` +${horizon.names.length - 8}` : "";

  async function confirm() {
    if (pending || disabled) {
      return;
    }
    setPending(true);
    setError(null);
    try {
      const result = await client.executeCommand("PlanHorizonAssumeNextYear", {
        asOfDate,
      });
      if (!result.ok) {
        setError(result.errorCode ?? "assume_failed");
        return;
      }
      setHorizon(null);
      onConfirmed?.();
    } catch (err) {
      setError(err instanceof Error ? err.message : "assume_failed");
    } finally {
      setPending(false);
    }
  }

  return (
    <aside className="plan-horizon-prompt" aria-label="Next-year plan dates">
      <p>
        Confirm {horizon.assumeYear} pay dates for {horizon.names.length} planned
        names ({horizon.holeCount} holes). Same Plan $ / share. Vendor dates
        replace assumed holes. {names}
        {extra}.
      </p>
      {error ? <p className="blocked">{error}</p> : null}
      <button
        type="button"
        aria-label="Confirm next-year plan dates"
        disabled={disabled || pending}
        className="is-unsaved"
        onClick={() => void confirm()}
      >
        Confirm
      </button>
    </aside>
  );
}
