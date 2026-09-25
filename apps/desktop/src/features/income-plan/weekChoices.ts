import {
  addUtcDays,
  formatWeekChooserLabel,
  saturdayOfWeek,
} from "@finos/ui-components";

export function shiftIso(date: string, days: number): string {
  return addUtcDays(date, days);
}

export function incomePlanWeekChoices(asOf: string, latestActualOn?: string | null): string[] {
  const today = new Date().toISOString().slice(0, 10);
  const anchor = saturdayOfWeek(asOf || latestActualOn || today);
  const todaySat = saturdayOfWeek(today);
  const latestSat = latestActualOn ? saturdayOfWeek(latestActualOn) : anchor;
  const start = [anchor, todaySat, latestSat].reduce((min, sat) =>
    sat < min ? sat : min,
  );
  const end = [anchor, todaySat, shiftIso(todaySat, 16 * 7)].reduce((max, sat) =>
    sat > max ? sat : max,
  );
  const pastStart = shiftIso(start, -52 * 7);
  const weeks: string[] = [];
  for (let sat = pastStart; sat <= end && weeks.length < 80; sat = shiftIso(sat, 7)) {
    if (weeks.length > 0 && sat === weeks[weeks.length - 1]) break;
    weeks.push(sat);
  }
  if (!weeks.includes(anchor)) {
    weeks.push(anchor);
    weeks.sort();
  }
  return weeks;
}

export function incomePlanWeekOptionLabel(saturday: string, thisWeekSaturday: string): string {
  return formatWeekChooserLabel(saturday, thisWeekSaturday);
}
