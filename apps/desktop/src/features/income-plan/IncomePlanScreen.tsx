import type {
  DividendPerformanceGet,
  DividendPerformanceRange,
  IncomePlanGridGet,
  IncomePlanWeekGet,
} from "@finos/app-contracts";
import {
  AccountTickPicker,
  INCOME_PLAN_DEFAULT_ACCOUNTS,
  INCOME_PLAN_OPTIONAL_ACCOUNTS,
  IncomePlanGridPanel,
  IncomePlanWeekPanel,
  saturdayOfWeek,
} from "@finos/ui-components";
import { DividendWeeksPanel } from "../graphing/DividendWeeks";
import { incomeGridMemoryMatches } from "../shared/backgroundReads";
import {
  incomePlanWeekChoices,
  incomePlanWeekOptionLabel,
  shiftIso,
} from "./weekChoices";

export type IncomePlanScreenProps = {
  incomeWeekLoading: boolean;
  incomeWeek: IncomePlanWeekGet | null;
  incomePattern: "A" | "B";
  setIncomePattern: (next: "A" | "B") => void;
  incomeGrid: IncomePlanGridGet | null;
  asOfDate: string;
  incomeHistWeeks: number;
  setIncomeHistWeeks: (n: number) => void;
  incomeFutWeeks: number;
  setIncomeFutWeeks: (n: number) => void;
  drillAccounts: string[];
  setDrillAccounts: (next: string[]) => void;
  withIncomeLoading: (work: () => Promise<void>) => Promise<void>;
  loadIncomeGrid: (
    asOf: string,
    hist: number,
    fut: number,
    accounts: string[],
    weekEnding?: string,
  ) => Promise<void>;
  loadIncomeWeek: (saturday: string) => Promise<void>;
  requestIncomeExport: () => Promise<void>;
  incomeExportLoading: boolean;
  incomeWeekNav: "prev" | "next" | null;
  setIncomeWeekNav: (next: "prev" | "next") => void;
  openPositionHub: (symbol: string, focus?: "lots" | "income" | "declarations" | "ledger" | "") => void;
  dividendPerf: DividendPerformanceGet | null;
  perfRange: DividendPerformanceRange;
  onPerfRangeChange: (next: DividendPerformanceRange) => void;
};

export function IncomePlanScreen({
  incomeWeekLoading,
  incomeWeek,
  incomePattern,
  setIncomePattern,
  incomeGrid,
  asOfDate,
  incomeHistWeeks,
  setIncomeHistWeeks,
  incomeFutWeeks,
  setIncomeFutWeeks,
  drillAccounts,
  setDrillAccounts,
  withIncomeLoading,
  loadIncomeGrid,
  loadIncomeWeek,
  requestIncomeExport,
  incomeExportLoading,
  incomeWeekNav,
  setIncomeWeekNav,
  openPositionHub,
  dividendPerf,
  perfRange,
  onPerfRangeChange,
}: IncomePlanScreenProps) {
  return (
    <section
      className="income-plan-page"
      aria-label="Income Plan"
      aria-busy={incomeWeekLoading}
    >
      <header className="income-plan-appbar">
        <div>
          <h2>Income Plan</h2>
          <p className="income-plan-sub">Sat–Fri week labeled by Friday ending</p>
        </div>
        {incomeWeekLoading ? (
          <p role="status" aria-live="polite">
            Loading the week…
          </p>
        ) : null}
        <div className="income-plan-report-tabs" role="tablist" aria-label="Report type">
          <button
            type="button"
            role="tab"
            aria-selected={incomePattern === "A"}
            className={incomePattern === "A" ? "is-active" : undefined}
            onClick={() => {
              setIncomePattern("A");
              if (
                incomeGridMemoryMatches(
                  incomeGrid,
                  asOfDate,
                  incomeHistWeeks,
                  incomeFutWeeks,
                  drillAccounts,
                )
              ) {
                return;
              }
              void withIncomeLoading(() =>
                loadIncomeGrid(
                  asOfDate,
                  incomeHistWeeks,
                  incomeFutWeeks,
                  drillAccounts,
                  incomeGrid?.selectedWeekEnd || incomeWeek?.end,
                ),
              );
            }}
          >
            Weekly grid
          </button>
          <button
            type="button"
            role="tab"
            aria-selected={incomePattern === "B"}
            className={incomePattern === "B" ? "is-active" : undefined}
            onClick={() => {
              setIncomePattern("B");
              const weekEnd = incomeGrid?.selectedWeekEnd ?? incomeWeek?.end;
              if (weekEnd) {
                void loadIncomeWeek(saturdayOfWeek(weekEnd));
              }
            }}
          >
            Weekly report
          </button>
        </div>
      </header>
      {incomePattern === "A" ? (
        <IncomePlanGridPanel
          grid={incomeGrid}
          selectedAccounts={drillAccounts}
          onSelectedAccounts={(next) => {
            setDrillAccounts(next);
            void loadIncomeGrid(
              asOfDate,
              incomeHistWeeks,
              incomeFutWeeks,
              next,
              incomeGrid?.selectedWeekEnd,
            );
          }}
          historicalWeeks={incomeHistWeeks}
          futureWeeks={incomeFutWeeks}
          onHistoricalWeeks={(n) => {
            setIncomeHistWeeks(n);
            void withIncomeLoading(() =>
              loadIncomeGrid(
                asOfDate,
                n,
                incomeFutWeeks,
                drillAccounts,
                incomeGrid?.selectedWeekEnd,
              ),
            );
          }}
          onFutureWeeks={(n) => {
            setIncomeFutWeeks(n);
            void withIncomeLoading(() =>
              loadIncomeGrid(
                asOfDate,
                incomeHistWeeks,
                n,
                drillAccounts,
                incomeGrid?.selectedWeekEnd,
              ),
            );
          }}
          onOpenWeek={(weekEnd) => {
            setIncomePattern("B");
            void loadIncomeWeek(saturdayOfWeek(weekEnd));
          }}
          onPrintExport={() => void requestIncomeExport()}
          exportLoading={incomeExportLoading}
        />
      ) : (
        <>
          {(() => {
            const todaySat = saturdayOfWeek(new Date().toISOString().slice(0, 10));
            const selectedSat = saturdayOfWeek(
              incomeWeek?.start || asOfDate || todaySat,
            );
            const selectedFri = incomeWeek?.end || shiftIso(selectedSat, 6);
            const weekChoices = incomePlanWeekChoices(
              selectedSat,
              incomeWeek?.latestActualOn,
            );
            const chips = [
              ...INCOME_PLAN_DEFAULT_ACCOUNTS,
              ...INCOME_PLAN_OPTIONAL_ACCOUNTS,
            ];
            return (
              <>
                <div className="income-week-bar">
                  <label className="income-week-label">
                    Week
                    <select
                      aria-label="Select week"
                      value={selectedSat}
                      disabled={incomeWeekLoading}
                      onChange={(e) => void loadIncomeWeek(e.target.value)}
                    >
                      {weekChoices.map((sat) => (
                        <option key={sat} value={sat}>
                          {incomePlanWeekOptionLabel(sat, todaySat)}
                        </option>
                      ))}
                    </select>
                  </label>
                  <button
                    type="button"
                    aria-label="Previous week"
                    aria-busy={incomeWeekNav === "prev" && incomeWeekLoading}
                    className={
                      incomeWeekNav === "prev" && incomeWeekLoading
                        ? "is-loading"
                        : undefined
                    }
                    disabled={!selectedSat || incomeWeekLoading}
                    onClick={() => {
                      setIncomeWeekNav("prev");
                      void loadIncomeWeek(shiftIso(selectedSat, -7));
                    }}
                  >
                    Previous week
                  </button>
                  <button
                    type="button"
                    aria-label="Next week"
                    aria-busy={incomeWeekNav === "next" && incomeWeekLoading}
                    className={
                      incomeWeekNav === "next" && incomeWeekLoading
                        ? "is-loading"
                        : undefined
                    }
                    disabled={!selectedSat || incomeWeekLoading}
                    onClick={() => {
                      setIncomeWeekNav("next");
                      void loadIncomeWeek(shiftIso(selectedSat, 7));
                    }}
                  >
                    Next week
                  </button>
                  <button
                    type="button"
                    aria-label="Back to Weekly grid"
                    onClick={() => {
                      setIncomePattern("A");
                      void withIncomeLoading(() =>
                        loadIncomeGrid(
                          asOfDate,
                          incomeHistWeeks,
                          incomeFutWeeks,
                          drillAccounts,
                          incomeWeek?.end,
                        ),
                      );
                    }}
                  >
                    Back to Weekly grid
                  </button>
                  <div className="income-print-export">
                    <button
                      type="button"
                      aria-label="Print Export"
                      aria-busy={incomeExportLoading}
                      className={incomeExportLoading ? "is-loading" : undefined}
                      disabled={incomeWeekLoading || incomeExportLoading}
                      onClick={() => void requestIncomeExport()}
                    >
                      Print Export
                    </button>
                  </div>
                </div>
                <p className="income-current-week" aria-label="Current week">
                  Current week: Saturday {selectedSat} through Friday {selectedFri}
                  {selectedSat === todaySat ? (
                    <span className="this-week-mark"> · this week</span>
                  ) : null}
                </p>
                <AccountTickPicker
                  legend="Accounts"
                  allLabel="All Dividend accounts"
                  mode="anyCombination"
                  accounts={chips}
                  allAccounts={INCOME_PLAN_DEFAULT_ACCOUNTS}
                  selected={drillAccounts}
                  onSelected={setDrillAccounts}
                />
              </>
            );
          })()}
          <IncomePlanWeekPanel
            week={incomeWeek}
            selectedAccounts={drillAccounts}
            onOpenSymbol={(symbol) => openPositionHub(symbol, "income")}
          />
        </>
      )}
      <DividendWeeksPanel
        perf={dividendPerf}
        range={perfRange}
        onRangeChange={onPerfRangeChange}
      />
    </section>
  );
}
