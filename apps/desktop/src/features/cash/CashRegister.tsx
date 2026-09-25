import { useEffect, useState } from "react";
import { AccountTickPicker, formatUsd } from "@finos/ui-components";
import type { CashRegisterGet, TrendsWeekPoint } from "@finos/app-contracts";
import { AccountCashFlow } from "./AccountCashFlow";

const BOOKS = [
  "Income",
  "FI Roth",
  "Health",
  "Car",
  "Account 9",
  "SSA_2026",
] as const;

const WEEKDAYS = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"] as const;
const MONTHS = [
  "January",
  "February",
  "March",
  "April",
  "May",
  "June",
  "July",
  "August",
  "September",
  "October",
  "November",
  "December",
] as const;

function utcDate(iso: string) {
  const [y, m, d] = iso.split("-").map(Number);
  return new Date(Date.UTC(y, (m ?? 1) - 1, d ?? 1));
}

function toIso(day: Date) {
  return day.toISOString().slice(0, 10);
}

function addDays(iso: string, days: number) {
  const day = utcDate(iso);
  day.setUTCDate(day.getUTCDate() + days);
  return toIso(day);
}

function firstOfMonth(iso: string) {
  const day = utcDate(iso);
  return toIso(new Date(Date.UTC(day.getUTCFullYear(), day.getUTCMonth(), 1)));
}

function lastOfMonth(iso: string) {
  const day = utcDate(iso);
  return toIso(new Date(Date.UTC(day.getUTCFullYear(), day.getUTCMonth() + 1, 0)));
}

function addMonths(iso: string, months: number) {
  const day = utcDate(iso);
  return toIso(new Date(Date.UTC(day.getUTCFullYear(), day.getUTCMonth() + months, 1)));
}

function monthCaption(iso: string) {
  const day = utcDate(iso);
  return `${MONTHS[day.getUTCMonth()]} ${day.getUTCFullYear()}`;
}

function calendarWeeks(periodStart: string, periodEnd: string) {
  const lead = utcDate(periodStart).getUTCDay();
  const first = addDays(periodStart, -lead);
  const trail = 6 - utcDate(periodEnd).getUTCDay();
  const last = addDays(periodEnd, trail);
  const weeks: string[][] = [];
  let cursor = first;
  while (cursor <= last) {
    const week: string[] = [];
    for (let i = 0; i < 7; i += 1) {
      week.push(cursor);
      cursor = addDays(cursor, 1);
    }
    weeks.push(week);
  }
  return weeks;
}

function signedMoney(minor: number, scale: number) {
  return formatUsd(minor, scale);
}

export function CashRegisterPanel({
  register,
  book,
  view,
  asOfDate,
  weeks,
  onBook,
  onView,
  onLoadMonth,
  onManageElements,
}: {
  register: CashRegisterGet | null;
  book: string;
  period: string;
  view: "calendar" | "trend";
  asOfDate?: string;
  weeks?: TrendsWeekPoint[] | null;
  busy?: boolean;
  loading?: boolean;
  onBook: (book: string) => void;
  onPeriod: (period: string) => void;
  onView: (view: "calendar" | "trend") => void;
  onAddElement: () => void;
  onManageElements?: () => void;
  onLoadMonth?: (asOfDate: string) => Promise<CashRegisterGet | null>;
}) {
  const scale = register?.scale ?? 2;
  const money = (minor: number | null | undefined) =>
    minor == null ? "—" : formatUsd(minor, scale);
  const [monthOn, setMonthOn] = useState("");
  const [monthRegister, setMonthRegister] = useState<CashRegisterGet | null>(null);
  const seedOn = register?.asOfDate || asOfDate || "";
  const cursor = monthOn || (seedOn ? firstOfMonth(seedOn) : "");

  useEffect(() => {
    if (!monthOn && seedOn) {
      setMonthOn(firstOfMonth(seedOn));
    }
  }, [seedOn, monthOn]);

  useEffect(() => {
    if (!cursor) {
      return;
    }
    if (!onLoadMonth) {
      setMonthRegister(register);
      return;
    }
    let cancel = false;
    void onLoadMonth(lastOfMonth(cursor)).then((body) => {
      if (!cancel) {
        setMonthRegister(body);
      }
    });
    return () => {
      cancel = true;
    };
  }, [book, cursor, onLoadMonth]);

  const calendarBody = monthRegister ?? register;
  const [focusDay, setFocusDay] = useState("");

  useEffect(() => {
    setFocusDay("");
  }, [book, cursor]);

  const monthRows = calendarBody?.rows ?? [];
  const listedRows = focusDay
    ? monthRows.filter((row) => row.occurredOn === focusDay)
    : [];

  return (
    <section className="cash-register" id="cash-register" aria-label="Cashflow manager">
      <div className="cashflow-manager-head">
        <h3>{view === "calendar" ? `${book} cashflow manager` : "cashflow manager"}</h3>
        {onManageElements ? (
          <button
            type="button"
            aria-label="Manage Elements"
            onClick={onManageElements}
          >
            Manage Elements
          </button>
        ) : null}
      </div>
      <div className="cashflow-picker-row">
        {view === "calendar" ? (
          <AccountTickPicker
            legend="Managed accounts"
            mode="exactlyOne"
            showAll={false}
            accounts={BOOKS}
            selected={[book]}
            onSelected={(next) => {
              if (next[0]) onBook(next[0]);
            }}
          />
        ) : (
          <span />
        )}
        <div className="buttons" aria-label="Register view">
          <button
            type="button"
            aria-label="Register calendar"
            aria-pressed={view === "calendar"}
            onClick={() => onView("calendar")}
          >
            Calendar
          </button>
          <button
            type="button"
            aria-label="Register trend"
            aria-pressed={view === "trend"}
            onClick={() => onView("trend")}
          >
            Trend
          </button>
        </div>
      </div>
      {view === "calendar" && cursor ? (
        <RegisterMonthGrid
          book={book}
          monthOn={cursor}
          series={calendarBody?.series ?? []}
          rows={calendarBody?.rows ?? []}
          startMinor={calendarBody?.startMinor ?? null}
          asOfDate={calendarBody?.asOfDate ?? ""}
          money={money}
          scale={calendarBody?.scale ?? scale}
          onPrev={() => setMonthOn(addMonths(cursor, -1))}
          onNext={() => setMonthOn(addMonths(cursor, 1))}
          focusDay={focusDay}
          onFocusDay={setFocusDay}
        />
      ) : null}
      {view === "trend" ? (
        <div className="register-trend-wrap" aria-label="Register trend">
          <AccountCashFlow weeks={weeks} asOf={asOfDate} />
        </div>
      ) : null}
      {view === "calendar" && !focusDay ? (
        <p role="status">Select a date on the calendar to list that day’s transactions.</p>
      ) : view === "calendar" ? (
        <div className="table-wrap">
          {focusDay ? (
            <h4 aria-label="Day transactions">{focusDay} transactions</h4>
          ) : null}
          <table aria-label="Cash register">
            <thead>
              <tr>
                <th>Date</th>
                <th>Status</th>
                <th>Transaction</th>
                <th className="numeric">Deposit</th>
                <th className="numeric">Withdrawal</th>
                <th className="numeric">Running cash</th>
              </tr>
            </thead>
            <tbody>
              {listedRows.map((row, i) => (
                <tr
                  key={`${row.occurredOn}-${row.label}-${i}`}
                  id={
                    listedRows.findIndex((r) => r.occurredOn === row.occurredOn) === i
                      ? `register-row-${row.occurredOn}`
                      : undefined
                  }
                  className={focusDay === row.occurredOn ? "is-focus" : undefined}
                >
                  <td>{row.occurredOn}</td>
                  <td>{row.status}</td>
                  <td>
                    {row.transaction}
                    {row.label ? ` · ${row.label}` : ""}
                  </td>
                  <td className={`numeric${row.depositMinor ? " is-credit" : ""}`}>
                    {row.depositMinor ? formatUsd(row.depositMinor, row.scale ?? scale) : ""}
                  </td>
                  <td className={`numeric${row.withdrawalMinor ? " is-debit" : ""}`}>
                    {row.withdrawalMinor
                      ? formatUsd(row.withdrawalMinor, row.scale ?? scale)
                      : ""}
                  </td>
                  <td className="numeric">{money(row.runningMinor)}</td>
                </tr>
              ))}
            </tbody>
          </table>
          {focusDay && listedRows.length === 0 ? (
            <p role="status">No transactions on {focusDay}.</p>
          ) : null}
        </div>
      ) : null}
    </section>
  );
}

function RegisterMonthGrid({
  book,
  monthOn,
  series,
  rows,
  startMinor,
  asOfDate,
  money,
  scale,
  onPrev,
  onNext,
  focusDay,
  onFocusDay,
}: {
  book: string;
  monthOn: string;
  series: CashRegisterGet["series"];
  rows: CashRegisterGet["rows"];
  startMinor: number | null;
  asOfDate: string;
  money: (minor: number | null | undefined) => string;
  scale: number;
  onPrev: () => void;
  onNext: () => void;
  focusDay: string;
  onFocusDay: (iso: string) => void;
}) {
  const monthStart = firstOfMonth(monthOn);
  const monthEnd = lastOfMonth(monthOn);
  const byDay = new Map(
    series
      .filter((p) => p.occurredOn >= monthStart && p.occurredOn <= monthEnd)
      .map((p) => [p.occurredOn, p]),
  );
  const weeks = calendarWeeks(monthStart, monthEnd);
  const runningByDay = new Map<string, number | null>();
  let last = startMinor;
  let monthStartMinor = startMinor;
  for (const week of weeks) {
    for (const iso of week) {
      if (iso === monthStart) {
        monthStartMinor = last;
      }
      const point = byDay.get(iso);
      if (point) last = point.runningMinor;
      if (iso >= monthStart && iso <= monthEnd) {
        runningByDay.set(iso, last);
      }
    }
  }
  const monthEndMinor = runningByDay.get(monthEnd) ?? last;
  let actualIncomeMinor = 0;
  let plannedIncomeMinor = 0;
  let actualWithdrawalMinor = 0;
  let plannedWithdrawalMinor = 0;
  for (const row of rows) {
    if (row.occurredOn < monthStart || row.occurredOn > monthEnd) continue;
    const isActual = row.status === "Actual";
    if (isActual) {
      actualIncomeMinor += row.depositMinor || 0;
      actualWithdrawalMinor += row.withdrawalMinor || 0;
    } else {
      plannedIncomeMinor += row.depositMinor || 0;
      plannedWithdrawalMinor += row.withdrawalMinor || 0;
    }
  }
  const showActual = monthStart <= asOfDate;
  const showPlanned = monthEnd > asOfDate;
  return (
    <div className="register-calendar-wrap">
      <div className="register-month-nav" aria-label="Register month">
        <button type="button" aria-label="Previous month" onClick={onPrev}>
          Previous
        </button>
        <h4>{book} · {monthCaption(monthStart)}</h4>
        <button type="button" aria-label="Next month" onClick={onNext}>
          Next
        </button>
      </div>
      <div className="home-trend-focus-flow register-month-flow" aria-label="Month cash flow totals">
        {showActual ? (
          <>
            <span className="home-trend-focus-label">Actual Income</span>
            <span className="home-av-value is-income" aria-label="Month actual income">
              {formatUsd(actualIncomeMinor, scale)}
            </span>
          </>
        ) : null}
        {showPlanned ? (
          <>
            <span className="home-trend-focus-label">Planned Income</span>
            <span className="home-av-value is-income" aria-label="Month planned income">
              {formatUsd(plannedIncomeMinor, scale)}
            </span>
          </>
        ) : null}
        <span className="home-trend-focus-label">Starting Balance</span>
        <span className="home-av-value" aria-label="Month starting balance">
          {money(monthStartMinor)}
        </span>
        {showActual ? (
          <>
            <span className="home-trend-focus-label">Actual withdrawals</span>
            <span className="home-av-value is-withdraw" aria-label="Month actual withdrawals">
              {formatUsd(actualWithdrawalMinor, scale)}
            </span>
          </>
        ) : null}
        {showPlanned ? (
          <>
            <span className="home-trend-focus-label">Planned withdrawals</span>
            <span className="home-av-value is-withdraw" aria-label="Month planned withdrawals">
              {formatUsd(plannedWithdrawalMinor, scale)}
            </span>
          </>
        ) : null}
        <span className="home-trend-focus-label">Ending balance</span>
        <span className="home-av-value" aria-label="Month ending balance">
          {money(monthEndMinor)}
        </span>
      </div>
      <table className="register-calendar" aria-label="Register calendar">
        <thead>
          <tr>
            {WEEKDAYS.map((name) => (
              <th key={name}>{name}</th>
            ))}
          </tr>
        </thead>
        <tbody>
          {weeks.map((week) => (
            <tr key={week[0]}>
              {week.map((iso) => {
                const inMonth = iso >= monthStart && iso <= monthEnd;
                const point = byDay.get(iso);
                const classes = [
                  inMonth ? "is-in-month" : "is-outside",
                  iso === asOfDate ? "is-asof" : "",
                  iso === focusDay ? "is-focus" : "",
                  point ? "has-activity" : "",
                ]
                  .filter(Boolean)
                  .join(" ");
                return (
                  <td
                    key={iso}
                    className={classes}
                    role={inMonth ? "button" : undefined}
                    tabIndex={inMonth ? 0 : undefined}
                    onClick={() => {
                      if (inMonth) onFocusDay(iso === focusDay ? "" : iso);
                    }}
                    onKeyDown={(e) => {
                      if (inMonth && (e.key === "Enter" || e.key === " ")) {
                        e.preventDefault();
                        onFocusDay(iso === focusDay ? "" : iso);
                      }
                    }}
                  >
                    <span className="cal-day">{Number(iso.slice(8))}</span>
                    {inMonth && point ? (
                      <span
                        className={
                          point.netMinor < 0
                            ? "cal-net is-debit"
                            : point.netMinor > 0
                              ? "cal-net is-credit"
                              : "cal-net"
                        }
                      >
                        {signedMoney(point.netMinor, scale)}
                      </span>
                    ) : null}
                    {inMonth ? (
                      <span className="cal-run">{money(runningByDay.get(iso) ?? null)}</span>
                    ) : null}
                  </td>
                );
              })}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

