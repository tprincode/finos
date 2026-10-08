import { useMemo, useState } from "react";
import {
  RATE_PERIODS,
  PERIODS_PER_YEAR,
  convertPeriodRate,
  formatHold,
  formatPct,
  holdDays,
  parseMoneyInput,
  parsePercentInput,
  projectHold,
  todayIso,
  type RatePeriod,
} from "./math";

type ContractRow = {
  id: number;
  notional: string;
  premium: string;
  startOn: string;
  endOn: string;
};

function emptyContract(id: number): ContractRow {
  return {
    id,
    notional: "",
    premium: "",
    startOn: todayIso(),
    endOn: "",
  };
}

export function InterestRateCalculator() {
  const [periodInputs, setPeriodInputs] = useState<Record<RatePeriod, string>>({
    daily: "",
    weekly: "",
    monthly: "",
    quarterly: "",
    annual: "",
  });
  const [nextId, setNextId] = useState(2);
  const [contracts, setContracts] = useState<ContractRow[]>([emptyContract(1)]);

  const periodRows = useMemo(
    () =>
      RATE_PERIODS.map((period) => {
        const rate = parsePercentInput(periodInputs[period.id]);
        const from = PERIODS_PER_YEAR[period.id];
        return {
          ...period,
          rate,
          daily:
            rate == null ? null : convertPeriodRate(rate, from, PERIODS_PER_YEAR.daily),
          weekly:
            rate == null ? null : convertPeriodRate(rate, from, PERIODS_PER_YEAR.weekly),
          monthly:
            rate == null ? null : convertPeriodRate(rate, from, PERIODS_PER_YEAR.monthly),
          annual:
            rate == null ? null : convertPeriodRate(rate, from, PERIODS_PER_YEAR.annual),
        };
      }),
    [periodInputs],
  );

  const patchContract = (id: number, patch: Partial<ContractRow>) => {
    setContracts((rows) =>
      rows.map((row) => (row.id === id ? { ...row, ...patch } : row)),
    );
  };

  return (
    <section className="interest-rate-page" aria-label="Interest rate calculator">
      <h2>Interest rate calculator</h2>
      <p className="interest-rate-sub">
        Compound equivalents on a 365-day year. Type a period return — the other
        columns fill. A monthly 3.50% is 51.11% annual, same as the spreadsheet.
      </p>

      <h3 id="interest-period" data-section="interest-period">
        Period conversion
      </h3>
      <div className="table-wrap" data-part="period-conversion">
        <table aria-label="Period conversion">
          <thead>
            <tr>
              <th scope="col">Period</th>
              <th scope="col" className="numeric">
                Period return %
              </th>
              <th scope="col" className="numeric">
                Daily
              </th>
              <th scope="col" className="numeric">
                Weekly
              </th>
              <th scope="col" className="numeric">
                Monthly
              </th>
              <th scope="col" className="numeric">
                Annual
              </th>
            </tr>
          </thead>
          <tbody>
            {periodRows.map((row) => (
              <tr key={row.id}>
                <th scope="row">{row.label}</th>
                <td className="numeric">
                  <input
                    type="text"
                    inputMode="decimal"
                    aria-label={`${row.label} period return percent`}
                    value={periodInputs[row.id]}
                    onChange={(e) =>
                      setPeriodInputs((cur) => ({ ...cur, [row.id]: e.target.value }))
                    }
                  />
                </td>
                <td className="numeric">{formatPct(row.daily)}</td>
                <td className="numeric">{formatPct(row.weekly)}</td>
                <td className="numeric">{formatPct(row.monthly)}</td>
                <td className="numeric">{formatPct(row.annual)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <h3 id="interest-contract" data-section="interest-contract">
        Contract or premium
      </h3>
      <p className="interest-rate-sub">
        Start is today. Enter the end date so hold days drive the monthly and
        annual projection. Premium ÷ contract is the return for that hold.
      </p>
      <div className="table-wrap" data-part="contract-return">
        <table aria-label="Contract return projection">
          <thead>
            <tr>
              <th scope="col">Contract $</th>
              <th scope="col">CC or put $</th>
              <th scope="col">Start</th>
              <th scope="col">End</th>
              <th scope="col">Hold</th>
              <th scope="col" className="numeric">
                Period %
              </th>
              <th scope="col" className="numeric">
                Monthly
              </th>
              <th scope="col" className="numeric">
                Annual
              </th>
            </tr>
          </thead>
          <tbody>
            {contracts.map((row) => {
              const days = holdDays(row.startOn, row.endOn);
              const notional = parseMoneyInput(row.notional);
              const premium = parseMoneyInput(row.premium);
              const period =
                notional != null && notional !== 0 && premium != null
                  ? premium / notional
                  : null;
              const proj =
                period != null && days != null && days > 0
                  ? projectHold(period, days)
                  : null;
              const holdHint =
                !row.endOn
                  ? "Enter the end date"
                  : days != null && days <= 0
                    ? "End must be after start"
                    : formatHold(days);
              return (
                <tr key={row.id}>
                  <td>
                    <input
                      type="text"
                      inputMode="decimal"
                      aria-label="Contract amount"
                      value={row.notional}
                      onChange={(e) =>
                        patchContract(row.id, { notional: e.target.value })
                      }
                    />
                  </td>
                  <td>
                    <input
                      type="text"
                      inputMode="decimal"
                      aria-label="Covered call or put premium"
                      value={row.premium}
                      onChange={(e) =>
                        patchContract(row.id, { premium: e.target.value })
                      }
                    />
                  </td>
                  <td>
                    <input
                      type="date"
                      aria-label="Hold start date"
                      value={row.startOn}
                      onChange={(e) =>
                        patchContract(row.id, { startOn: e.target.value })
                      }
                    />
                  </td>
                  <td>
                    <input
                      type="date"
                      aria-label="Hold end date"
                      value={row.endOn}
                      onChange={(e) =>
                        patchContract(row.id, { endOn: e.target.value })
                      }
                    />
                  </td>
                  <td>{holdHint}</td>
                  <td className="numeric">{formatPct(period)}</td>
                  <td className="numeric">{formatPct(proj?.monthly)}</td>
                  <td className="numeric">{formatPct(proj?.annual)}</td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
      <p>
        <button
          type="button"
          onClick={() => {
            setContracts((rows) => [...rows, emptyContract(nextId)]);
            setNextId((n) => n + 1);
          }}
        >
          Add another contract
        </button>
      </p>
    </section>
  );
}
