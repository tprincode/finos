/** Saturday–Friday week identity. Week 1 is the first Saturday on or after 1 January.
 *  Year is the calendar year of that Saturday. This is not ISO-8601 (Monday) numbering.
 *  Keep in lockstep with crates/financial-domain/src/week.rs.
 */

export type WeekId = {
  year: number;
  number: number;
  start: string;
  end: string;
};

const MONTH_SHORT = [
  "Jan",
  "Feb",
  "Mar",
  "Apr",
  "May",
  "Jun",
  "Jul",
  "Aug",
  "Sep",
  "Oct",
  "Nov",
  "Dec",
];

function parseUtcDay(iso: string): Date | null {
  const day = iso.trim().slice(0, 10);
  const parts = day.split("-").map(Number);
  if (parts.length < 3 || parts.some((n) => !Number.isFinite(n))) {
    return null;
  }
  const [y, m, d] = parts;
  const dt = new Date(Date.UTC(y, (m ?? 1) - 1, d ?? 1));
  if (Number.isNaN(dt.getTime())) {
    return null;
  }
  return dt;
}

function formatUtcDay(d: Date): string {
  return d.toISOString().slice(0, 10);
}

export function addUtcDays(iso: string, days: number): string {
  const d = parseUtcDay(iso);
  if (!d) {
    return iso;
  }
  d.setUTCDate(d.getUTCDate() + days);
  return formatUtcDay(d);
}

/** Saturday that starts the Sat–Fri week containing `iso`. */
export function saturdayOfWeek(iso: string): string {
  const d = parseUtcDay(iso);
  if (!d) {
    return iso;
  }
  const daysSinceSaturday = (d.getUTCDay() + 1) % 7;
  d.setUTCDate(d.getUTCDate() - daysSinceSaturday);
  return formatUtcDay(d);
}

export function fridayOfWeek(iso: string): string {
  return addUtcDays(saturdayOfWeek(iso), 6);
}

export function firstSaturdayOfYear(year: number): string {
  const d = new Date(Date.UTC(year, 0, 1));
  while (d.getUTCDay() !== 6) {
    d.setUTCDate(d.getUTCDate() + 1);
  }
  return formatUtcDay(d);
}

export function weekIdContaining(iso: string): WeekId {
  const start = saturdayOfWeek(iso);
  const end = addUtcDays(start, 6);
  const year = Number(start.slice(0, 4));
  const first = firstSaturdayOfYear(year);
  const startMs = parseUtcDay(start)?.getTime() ?? 0;
  const firstMs = parseUtcDay(first)?.getTime() ?? 0;
  const number = Math.floor((startMs - firstMs) / (7 * 86_400_000)) + 1;
  return { year, number, start, end };
}

export function formatWeekNumber(id: WeekId): string {
  return `W${String(id.number).padStart(2, "0")}`;
}

/** Chooser / current-week text: `W35 · Sat 2026-08-29 – Fri 2026-09-04`. */
export function formatWeekChooserLabel(
  iso: string,
  thisWeekSaturday?: string,
): string {
  const id = weekIdContaining(iso);
  const mark = thisWeekSaturday && id.start === thisWeekSaturday ? " · this week" : "";
  return `${formatWeekNumber(id)} · Sat ${id.start} – Fri ${id.end}${mark}`;
}

/** Friday-ending week header used by Income Plan Pattern A: `4-Sep`. */
export function formatFridayEnding(iso: string): string {
  const day = iso.trim().slice(0, 10);
  const parts = day.split("-").map(Number);
  if (parts.length < 3 || parts.some((n) => !Number.isFinite(n))) {
    return day;
  }
  const month = parts[1] ?? 0;
  const d = parts[2] ?? 0;
  if (month < 1 || month > 12) {
    return day;
  }
  return `${d}-${MONTH_SHORT[month - 1]}`;
}

/** Compact column header: `W35 · 29-Aug` using the Saturday start. */
export function formatWeekColumnHeader(iso: string): string {
  const id = weekIdContaining(iso);
  const parts = id.start.split("-");
  const day = Number(parts[2]);
  const month = Number(parts[1]);
  if (!Number.isFinite(day) || !Number.isFinite(month) || month < 1 || month > 12) {
    return `${formatWeekNumber(id)} · ${id.start}`;
  }
  return `${formatWeekNumber(id)} · ${day}-${MONTH_SHORT[month - 1]}`;
}

/** Axis / table short label: `W35 · 2026-08-29`. */
export function formatWeekShort(iso: string): string {
  const id = weekIdContaining(iso);
  return `${formatWeekNumber(id)} · ${id.start}`;
}

export function formatWeekCaption(iso: string): string {
  const id = weekIdContaining(iso);
  return `${formatWeekNumber(id)} · Saturday ${id.start} – Friday ${id.end}`;
}

/** Menu bar week: `W36 2026-09-05 – 2026-09-11` — dates only, no weekday names. */
export function formatMenuWeek(iso: string): string {
  const id = weekIdContaining(iso);
  return `${formatWeekNumber(id)} ${id.start} – ${id.end}`;
}
