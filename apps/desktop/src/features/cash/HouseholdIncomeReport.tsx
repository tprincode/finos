import { formatUsd } from "@finos/ui-components";
import type { MagiProjection, TaxPlanningGet } from "@finos/app-contracts";
import { useEffect, useState } from "react";
import {
  APPLICATION_APTC_MINOR,
  BARBARA_LTCG_MINOR,
  COMPUTER_SCHEDULE_C_MINOR,
  HALF_SE_EST_MINOR,
  HSA_CONTRIBUTION_MINOR,
  NET_CAPITAL_LOSS_LIMIT_MINOR,
  forecastMagi,
  planParts,
} from "./magiForecast";

const TAX_YEAR = 2026;
const STORAGE_KEY = `finos.tax.aptc.${TAX_YEAR}`;
const MEDICAL_MINOR = 600_000;
const EST_FED_OWED_MINOR = 620_000;
const EST_VA_OWED_MINOR = 77_000;

/** Marketplace application MAGI that produced the 2026 credit. */
const APPLICATION_MAGI_MINOR = 8_084_400;
/** A month counts toward APTC YTD on this calendar day. */
const APTC_ACCRUAL_DAY = 25;

type AptcFacts = {
  slcspMinor: number | null;
  applicationMagiMinor: number;
  applicationAptcMinor: number;
};

function money(minor: number, scale = 2): string {
  return formatUsd(minor, scale);
}

function parseDollars(raw: string): number | null {
  const t = raw.trim().replace(/[$,]/g, "");
  if (!t) return null;
  const n = Number(t);
  if (!Number.isFinite(n) || n < 0) return null;
  return Math.round(n * 100);
}

function dollarsField(minor: number | null): string {
  if (minor == null) return "";
  return (minor / 100).toFixed(2);
}

function emptyAptc(): AptcFacts {
  return {
    slcspMinor: null,
    applicationMagiMinor: APPLICATION_MAGI_MINOR,
    applicationAptcMinor: APPLICATION_APTC_MINOR,
  };
}

function loadAptc(): AptcFacts {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return emptyAptc();
    const body = JSON.parse(raw) as Partial<AptcFacts>;
    return {
      slcspMinor: body.slcspMinor ?? null,
      applicationMagiMinor: body.applicationMagiMinor ?? APPLICATION_MAGI_MINOR,
      applicationAptcMinor: body.applicationAptcMinor ?? APPLICATION_APTC_MINOR,
    };
  } catch {
    return emptyAptc();
  }
}

function parseIsoDate(iso: string): { year: number; month: number; day: number } {
  const [year, month, day] = iso.slice(0, 10).split("-").map(Number);
  return { year, month, day };
}

/** Months whose 25th has already passed in the tax year. Sep 25 → 9. */
function aptcCompletedMonths(asOfIso: string, taxYear: number): number {
  const { year, month, day } = parseIsoDate(asOfIso);
  if (!year || !month || !day) return 0;
  if (year < taxYear) return 0;
  if (year > taxYear) return 12;
  const completed = day >= APTC_ACCRUAL_DAY ? month : month - 1;
  return Math.max(0, Math.min(12, completed));
}

/** Nine months of $18,054 is $13,540 — floor to whole dollars. */
function aptcYtdMinor(yearAptcMinor: number, completedMonths: number): number {
  return Math.floor((yearAptcMinor * completedMonths) / 12 / 100) * 100;
}

/** Over 400% FPL, repay the APTC that was awarded on the application. */
function creditAtRisk(
  overCliff: boolean,
  applicationAptc: number,
): { label: string; tone: "ok" | "over"; minor: number } {
  if (!overCliff) {
    return { label: money(0), tone: "ok", minor: 0 };
  }
  return { label: money(applicationAptc), tone: "over", minor: applicationAptc };
}

export function HouseholdIncomeReport({
  plan,
  magi,
}: {
  plan: TaxPlanningGet;
  magi: MagiProjection | null;
}) {
  const scale = plan.scale;
  const ira = planParts(plan, "ira");
  const job = planParts(plan, "job1099");
  const ordinary = planParts(plan, "ordinary");
  const ssa = planParts(plan, "ssa");
  const roth = planParts(plan, "roth");
  const hsaWd = planParts(plan, "hsa");
  const roc = planParts(plan, "roc");
  const carLt = planParts(plan, "ltcg");
  const carSt = planParts(plan, "stcg");

  const [aptc, setAptc] = useState<AptcFacts>(loadAptc);
  const [slcspText, setSlcspText] = useState(() => dollarsField(loadAptc().slcspMinor));
  const [appMagiText, setAppMagiText] = useState(() =>
    dollarsField(loadAptc().applicationMagiMinor),
  );
  const [appAptcText, setAppAptcText] = useState(() =>
    dollarsField(loadAptc().applicationAptcMinor),
  );
  const [dirty, setDirty] = useState(false);

  useEffect(() => {
    const next = loadAptc();
    setAptc(next);
    setSlcspText(dollarsField(next.slcspMinor));
    setAppMagiText(dollarsField(next.applicationMagiMinor));
    setAppAptcText(dollarsField(next.applicationAptcMinor));
  }, []);

  const yearAptc =
    (dirty ? parseDollars(appAptcText) : null) ?? aptc.applicationAptcMinor;
  const completedMonths = aptcCompletedMonths(plan.asOfDate, TAX_YEAR);
  const aptcYtd = aptcYtdMinor(yearAptc, completedMonths);
  const aptcRemain = yearAptc - aptcYtd;
  const forecast = forecastMagi(plan, magi, yearAptc);
  const {
    after,
    cliff,
    overage,
    over,
    hole,
    isEstimate,
    suggestionLines,
    gains,
  } = forecast;
  const magiEoy = ira.eoy + job.eoy + ordinary.eoy + ssa.eoy + gains.magiMinor;
  const confidence = isEstimate ? "Estimate" : "Booked";
  const leadSuggestions = suggestionLines.slice(0, 2);

  const risk = creditAtRisk(over, yearAptc);
  const appVsCliff = cliff - aptc.applicationMagiMinor;
  const fed = plan.withholding?.find((row) => row.key === "fed");
  const state = plan.withholding?.find((row) => row.key === "state");
  const fedYtd = fed?.ytdMinor ?? 0;
  const fedRemain = fed?.remainingMinor ?? 0;
  const fedEoy = fedYtd + fedRemain;
  const vaYtd = state?.ytdMinor ?? 0;
  const vaRemain = state?.remainingMinor ?? 0;
  const vaEoy = vaYtd + vaRemain;
  const showSuggestions = over || isEstimate;

  const saveAptc = () => {
    const next: AptcFacts = {
      slcspMinor: parseDollars(slcspText),
      applicationMagiMinor: parseDollars(appMagiText) ?? APPLICATION_MAGI_MINOR,
      applicationAptcMinor: parseDollars(appAptcText) ?? APPLICATION_APTC_MINOR,
    };
    localStorage.setItem(STORAGE_KEY, JSON.stringify(next));
    setAptc(next);
    setDirty(false);
  };

  const incomeRows: Array<{
    key: string;
    label: string;
    ytd: number;
    remaining: number;
    eoy: number;
    magi: string;
    form1040: string;
    note?: string;
  }> = [
    {
      key: "ira",
      label: "Traditional IRA withdrawals",
      ...ira,
      magi: "100%",
      form1040: "Ordinary",
    },
    {
      key: "job",
      label: "1099 consulting",
      ...job,
      magi: "100%",
      form1040: "Sch C + SE tax",
    },
    {
      key: "ordinary",
      label: "Ordinary income",
      ...ordinary,
      magi: "100%",
      form1040: "Ordinary",
    },
    {
      key: "ssa",
      label: "Social Security",
      ...ssa,
      magi: "100%",
      form1040: "85% taxable",
    },
    {
      key: "roth",
      label: "Roth withdrawals",
      ...roth,
      magi: "0%",
      form1040: "None",
    },
    {
      key: "hsa",
      label: "HSA withdrawals",
      ...hsaWd,
      magi: "0%",
      form1040: "None",
    },
    {
      key: "roc",
      label: "ROC",
      ...roc,
      magi: "0%",
      form1040: "None",
    },
    {
      key: "ltcg",
      label: "Long-term capital gains",
      ...carLt,
      magi: "100%",
      form1040: "LTCG rate",
      note: "Car lot sales. 1040 net capital-loss limit is $3,000.",
    },
    {
      key: "stcg",
      label: "Short-term capital gains",
      ...carSt,
      magi: "100%",
      form1040: "Ordinary",
      note: "Car lot sales.",
    },
  ];

  return (
    <div aria-label="Tax Planning MAGI" className="household-income">
      <header className="household-income-head">
        <h3 aria-label="Tax Planning income">{TAX_YEAR} household income</h3>
        <p>Joint · 2-person · Virginia · Gross dollars</p>
      </header>

      <div className="household-income-top">
      <div className="household-income-lead">
      <div
        className="tax-cliff household-income-tiles"
        id="magi-forecast"
        data-section="magi-forecast-tiles"
        data-part="magi-tiles"
        aria-label="MAGI forecast"
      >
        <div>
          <span>MAGI after estimates vs {money(cliff, scale)}</span>
          <strong className={over ? "is-over" : "is-under"}>{money(after, scale)}</strong>
        </div>
        <div>
          <span>{over ? "Overage" : "Cushion"}</span>
          <strong className={over ? "is-over" : "is-under"}>
            {money(Math.abs(overage), scale)}
          </strong>
        </div>
        <div>
          <span>Confidence</span>
          <strong
            aria-label="MAGI confidence"
            className={isEstimate ? "is-estimate" : "is-under"}
          >
            {confidence}
          </strong>
        </div>
        <div>
          <span>Credit at risk for {TAX_YEAR}</span>
          <strong className={risk.tone === "ok" ? "is-under" : "is-over"}>{risk.label}</strong>
        </div>
      </div>
      {showSuggestions && leadSuggestions.length > 0 ? (
        <section
          className="household-income-lead-suggest"
          data-part="magi-suggestions-preview"
          aria-label="MAGI suggestions preview"
        >
          <h4>
            Suggestions{hole > 0 ? ` that close ${money(hole, scale)}` : ""}
          </h4>
          <ul>
            {leadSuggestions.map((line) => (
              <li key={line}>{line}</li>
            ))}
          </ul>
        </section>
      ) : null}
      </div>

      <div
        className="household-income-compare"
        id="magi-application"
        data-section="magi-application"
      >
      <h4>What that means against this year’s projection</h4>
      <div className="table-wrap car-tax-table-wrap" data-part="magi-application-compare">
        <table aria-label="Application versus current MAGI">
          <thead>
            <tr>
              <th scope="col" />
              <th scope="col" className="numeric">
                Amount
              </th>
            </tr>
          </thead>
          <tbody>
            <tr>
              <th scope="row">Income on the application</th>
              <td className="numeric">{money(aptc.applicationMagiMinor, scale)}</td>
            </tr>
            <tr>
              <th scope="row">MAGI after estimates</th>
              <td className="numeric">{money(after, scale)}</td>
            </tr>
            <tr>
              <th scope="row">Cliff</th>
              <td className="numeric">{money(cliff, scale)}</td>
            </tr>
            <tr>
              <th scope="row">Application vs cliff</th>
              <td className="numeric">
                {money(Math.abs(appVsCliff), scale)} {appVsCliff >= 0 ? "under" : "over"}
                {appVsCliff >= 0 ? " (why you got the credit)" : ""}
              </td>
            </tr>
            <tr>
              <th scope="row">Current forecast vs cliff</th>
              <td className="numeric">
                {money(Math.abs(overage), scale)} {over ? "over" : "under"}
              </td>
            </tr>
            <tr className="tax-magi-sum">
              <th scope="row">Credit at risk for {TAX_YEAR}</th>
              <td className="numeric">{risk.label}</td>
            </tr>
          </tbody>
        </table>
      </div>
      </div>
      </div>
      <h4 id="household-income-detail" data-section="household-income-detail">Income</h4>
      <div className="table-wrap car-tax-table-wrap" data-part="household-income-table">
        <table aria-label="Household income">
          <thead>
            <tr>
              <th scope="col">Source</th>
              <th scope="col" className="numeric">
                YTD
              </th>
              <th scope="col" className="numeric">
                Remaining
              </th>
              <th scope="col" className="numeric">
                EOY
              </th>
              <th scope="col">MAGI</th>
              <th scope="col">1040</th>
            </tr>
          </thead>
          <tbody>
            {incomeRows.map((row) => (
              <tr key={row.key} className={row.magi === "0%" ? undefined : "tax-in-magi"}>
                <th scope="row">
                  {row.label}
                  {row.note ? <span className="household-row-note"> {row.note}</span> : null}
                </th>
                <td className="numeric">{money(row.ytd, scale)}</td>
                <td className="numeric">{money(row.remaining, scale)}</td>
                <td className="numeric">{money(row.eoy, scale)}</td>
                <td>{row.magi}</td>
                <td>{row.form1040}</td>
              </tr>
            ))}
            {gains.carryforwardMinor < 0 ? (
              <tr className="tax-in-magi">
                <th scope="row">
                  Capital loss over the 1040 limit
                  <span className="household-row-note">
                    {" "}
                    Net loss is {money(Math.abs(gains.netMinor), scale)}. MAGI takes{" "}
                    {money(NET_CAPITAL_LOSS_LIMIT_MINOR, scale)} this year;{" "}
                    {money(Math.abs(gains.carryforwardMinor), scale)} carries forward.
                  </span>
                </th>
                <td colSpan={2} />
                <td className="numeric">{money(-gains.carryforwardMinor, scale)}</td>
                <td>100%</td>
                <td>Carryforward</td>
              </tr>
            ) : null}
            {gains.unplaceableSale ? (
              <tr className="tax-in-magi">
                <th scope="row" colSpan={6}>
                  A lot sale is dated before its lot, so capital gains are missing
                  from this total.
                </th>
              </tr>
            ) : null}
            <tr className="tax-magi-sum">
              <th scope="row">MAGI-included total</th>
              <td colSpan={2} />
              <td className="numeric">{money(magiEoy, scale)}</td>
              <td colSpan={2} />
            </tr>
          </tbody>
        </table>
      </div>

      <h4 id="magi-estimates" data-section="magi-estimates">Estimates</h4>
      <div className="table-wrap car-tax-table-wrap" data-part="magi-estimates-table">
        <table aria-label="MAGI estimates">
          <thead>
            <tr>
              <th scope="col">Item</th>
              <th scope="col">Kind</th>
              <th scope="col" className="numeric">
                Amount
              </th>
              <th scope="col">MAGI</th>
            </tr>
          </thead>
          <tbody>
            <tr className="tax-estimate">
              <th scope="row">HSA contribution</th>
              <td>
                <span className="household-estimate-tag">Estimate</span>
              </td>
              <td className="numeric">{money(HSA_CONTRIBUTION_MINOR, scale)}</td>
              <td>Reduces</td>
            </tr>
            <tr className="tax-estimate">
              <th scope="row">Computer / laptop</th>
              <td>
                <span className="household-estimate-tag">Estimate</span>
              </td>
              <td className="numeric">{money(COMPUTER_SCHEDULE_C_MINOR, scale)}</td>
              <td>Reduces</td>
            </tr>
            <tr className="tax-estimate">
              <th scope="row">Half of SE tax</th>
              <td>
                <span className="household-estimate-tag">Estimate</span>
              </td>
              <td className="numeric">{money(HALF_SE_EST_MINOR, scale)}</td>
              <td>Reduces</td>
            </tr>
            <tr className="tax-estimate">
              <th scope="row">Traditional IRA contribution</th>
              <td>
                <span className="household-estimate-tag">Estimate</span>
              </td>
              <td className="numeric">{money(plan.iraContributionMinor ?? 0, scale)}</td>
              <td>Reduces if &gt; 0</td>
            </tr>
            <tr className="tax-estimate">
              <th scope="row">Barbara LTCG</th>
              <td>
                <span className="household-estimate-tag">Estimate</span>
              </td>
              <td className="numeric">{money(BARBARA_LTCG_MINOR, scale)}</td>
              <td>Adds if &gt; 0</td>
            </tr>
            <tr className="tax-magi-sum">
              <th scope="row">MAGI after estimates</th>
              <td />
              <td className="numeric">{money(after, scale)}</td>
              <td />
            </tr>
          </tbody>
        </table>
      </div>
      <p className="tax-car-note">
        Estimate rows feed the forecast tile. They are not booked ledger facts.
      </p>

      <h4 id="household-deductions" data-section="household-deductions">Deductions</h4>
      <div className="table-wrap car-tax-table-wrap" data-part="household-deductions-table">
        <table aria-label="Household deductions">
          <thead>
            <tr>
              <th scope="col">Item</th>
              <th scope="col" className="numeric">
                Amount
              </th>
              <th scope="col">MAGI</th>
              <th scope="col">1040</th>
            </tr>
          </thead>
          <tbody>
            <tr>
              <th scope="row">Medical expenses</th>
              <td className="numeric">{money(MEDICAL_MINOR, scale)}</td>
              <td>No effect</td>
              <td>Schedule A</td>
            </tr>
          </tbody>
        </table>
      </div>

      <h4 id="tax-money" data-section="tax-money">Tax money</h4>
      <div className="table-wrap car-tax-table-wrap" data-part="tax-withholding-table">
        <table aria-label="Tax withholding">
          <thead>
            <tr>
              <th scope="col">Bucket</th>
              <th scope="col" className="numeric">
                YTD collected
              </th>
              <th scope="col" className="numeric">
                Remaining
              </th>
              <th scope="col" className="numeric">
                EOY collected
              </th>
              <th scope="col" className="numeric">
                Est. owed
              </th>
              <th scope="col" className="numeric">
                Gap
              </th>
            </tr>
          </thead>
          <tbody>
            <tr>
              <th scope="row">Federal income + SE tax</th>
              <td className="numeric">{money(fedYtd, scale)}</td>
              <td className="numeric">{money(fedRemain, scale)}</td>
              <td className="numeric">{money(fedEoy, scale)}</td>
              <td className="numeric">{money(EST_FED_OWED_MINOR, scale)}</td>
              <td className="numeric">{money(fedEoy - EST_FED_OWED_MINOR, scale)}</td>
            </tr>
            <tr>
              <th scope="row">Virginia</th>
              <td className="numeric">{money(vaYtd, scale)}</td>
              <td className="numeric">{money(vaRemain, scale)}</td>
              <td className="numeric">{money(vaEoy, scale)}</td>
              <td className="numeric">{money(EST_VA_OWED_MINOR, scale)}</td>
              <td className="numeric">{money(vaEoy - EST_VA_OWED_MINOR, scale)}</td>
            </tr>
          </tbody>
        </table>
      </div>
      <p className="tax-car-note">
        Est. owed is the 26 Sep note, not a calculated return. Collected is Income withholding
        YTD plus remaining planned weeks.
      </p>

      {showSuggestions && suggestionLines.length > 0 ? (
        <section
          id="magi-suggestions"
          data-section="magi-suggestions"
          data-part="magi-suggestion-list"
          aria-label="MAGI suggestions"
        >
          <h4>Suggestions{hole > 0 ? ` that close ${money(hole, scale)}` : ""}</h4>
          <ul>
            {suggestionLines.map((line) => (
              <li key={line}>{line}</li>
            ))}
          </ul>
        </section>
      ) : null}

      <section
        className="household-aptc"
        id="household-aptc"
        data-section="household-aptc"
        data-part="aptc-facts"
        aria-label="APTC from 1095-A"
      >
        <h4>Marketplace application and 1095-A</h4>
        <div className="household-aptc-grid">
          <label>
            Income on the application
            <input
              aria-label="Income on the application"
              inputMode="decimal"
              value={appMagiText}
              onChange={(event) => {
                setAppMagiText(event.target.value);
                setDirty(true);
              }}
            />
          </label>
          <label>
            APTC on the application
            <input
              aria-label="APTC on the application"
              inputMode="decimal"
              value={appAptcText}
              onChange={(event) => {
                setAppAptcText(event.target.value);
                setDirty(true);
              }}
            />
          </label>
          <div className="household-aptc-fact">
            <span>APTC received YTD</span>
            <strong aria-label="APTC received YTD">{money(aptcYtd, scale)}</strong>
            <span>
              {completedMonths} completed month{completedMonths === 1 ? "" : "s"} (25th)
            </span>
          </div>
          <div className="household-aptc-fact">
            <span>APTC remaining this year</span>
            <strong aria-label="APTC remaining this year">{money(aptcRemain, scale)}</strong>
          </div>
          <label>
            SLCSP annual (optional)
            <input
              aria-label="SLCSP annual"
              inputMode="decimal"
              value={slcspText}
              onChange={(event) => {
                setSlcspText(event.target.value);
                setDirty(true);
              }}
            />
          </label>
          <button
            type="button"
            aria-label="Save APTC facts"
            className={dirty ? "is-unsaved" : undefined}
            disabled={!dirty}
            onClick={saveAptc}
          >
            Save APTC
          </button>
        </div>
        <p role="status">
          {aptc.slcspMinor != null
            ? `SLCSP stored ${money(aptc.slcspMinor)}. Under-cliff PTC is not computed here.`
            : `${completedMonths} completed months on the 25th. YTD is the year credit × months ÷ 12.`}
        </p>
      </section>
    </div>
  );
}
