import { useId, useRef, useState } from "react";

export type SortDir = "asc" | "desc";

/** Checked values for a column. `undefined` means no filter (all values). */
export type CheckedFilters = Record<string, string[] | undefined>;

export type ColFilterControl = {
  options: string[];
  selected: string[] | undefined;
  onChange: (selected: string[] | undefined) => void;
};

export function compareSortValues(
  left: string | number | null,
  right: string | number | null,
): number {
  if (left == null && right == null) return 0;
  if (left == null) return 1;
  if (right == null) return -1;
  if (typeof left === "number" && typeof right === "number") {
    return left === right ? 0 : left < right ? -1 : 1;
  }
  return String(left).localeCompare(String(right), undefined, {
    numeric: true,
    sensitivity: "base",
  });
}

export function useListSort(defaultKey: string, defaultDir: SortDir = "asc") {
  const [sortKey, setSortKey] = useState(defaultKey);
  const [sortDir, setSortDir] = useState<SortDir>(defaultDir);
  const toggleSort = (key: string) => {
    if (sortKey === key) {
      setSortDir((dir) => (dir === "asc" ? "desc" : "asc"));
      return;
    }
    setSortKey(key);
    setSortDir("asc");
  };
  return { sortKey, sortDir, toggleSort };
}

export function sortRows<T>(
  rows: T[],
  sortKey: string,
  sortDir: SortDir,
  valueOf: (row: T, key: string) => string | number | null,
): T[] {
  const copy = rows.slice();
  copy.sort((a, b) => {
    const cmp = compareSortValues(valueOf(a, sortKey), valueOf(b, sortKey));
    return sortDir === "asc" ? cmp : -cmp;
  });
  return copy;
}

export function textMatches(
  needle: string,
  ...parts: Array<string | number | null | undefined>
): boolean {
  const n = needle.trim().toLowerCase();
  if (!n) return true;
  return parts.some((part) => part != null && String(part).toLowerCase().includes(n));
}

export function hasColFilters(filters: CheckedFilters): boolean {
  return Object.values(filters).some((selected) => selected != null);
}

export function matchesColFilters(
  filters: CheckedFilters,
  values: Record<string, string>,
): boolean {
  return Object.entries(filters).every(([key, selected]) => {
    if (!selected) return true;
    return selected.includes(values[key] ?? "");
  });
}

export function uniqueFilterValues(
  rows: Array<Record<string, string>>,
  filters: CheckedFilters,
  key: string,
): string[] {
  const others: CheckedFilters = { ...filters, [key]: undefined };
  const seen = new Set<string>();
  for (const row of rows) {
    if (matchesColFilters(others, row)) {
      seen.add(row[key] ?? "");
    }
  }
  return [...seen].sort((a, b) =>
    a.localeCompare(b, undefined, { numeric: true, sensitivity: "base" }),
  );
}

export function colFilter(
  filters: CheckedFilters,
  setFilters: (next: CheckedFilters) => void,
  key: string,
  options: string[],
): ColFilterControl {
  return {
    options,
    selected: filters[key],
    onChange: (selected) => setFilters({ ...filters, [key]: selected }),
  };
}

export function ListFilter({
  label,
  value,
  onChange,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
}) {
  return (
    <label>
      {label}
      <input
        value={value}
        onChange={(event) => onChange(event.target.value)}
        aria-label={label}
      />
    </label>
  );
}

export function ClearFiltersButton({
  filters,
  onClear,
}: {
  filters: CheckedFilters;
  onClear: () => void;
}) {
  const active = hasColFilters(filters);
  return (
    <button type="button" aria-label="Clear filters" disabled={!active} onClick={onClear}>
      Clear filters
    </button>
  );
}

function optionLabel(value: string): string {
  return value === "" ? "(Blanks)" : value;
}

function toggleValue(
  options: string[],
  selected: string[] | undefined,
  value: string,
): string[] | undefined {
  const current = selected ?? options;
  const next = current.includes(value)
    ? current.filter((item) => item !== value)
    : [...current, value];
  if (next.length === options.length && options.every((item) => next.includes(item))) {
    return undefined;
  }
  return next;
}

function FilterMenu({
  label,
  options,
  selected,
  onChange,
}: {
  label: string;
  options: string[];
  selected: string[] | undefined;
  onChange: (selected: string[] | undefined) => void;
}) {
  const rawId = useId();
  const menuId = `col-filter-${rawId.replace(/:/g, "")}`;
  const buttonRef = useRef<HTMLButtonElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const filtered = selected != null;
  const checked = new Set(selected ?? options);
  const allChecked = options.length > 0 && options.every((item) => checked.has(item));

  const placeMenu = () => {
    const button = buttonRef.current;
    const menu = menuRef.current;
    if (!button || !menu) return;
    const rect = button.getBoundingClientRect();
    const width = Math.max(menu.offsetWidth || 220, 220);
    const left = Math.min(rect.left, Math.max(8, window.innerWidth - width - 8));
    menu.style.top = `${rect.bottom + 4}px`;
    menu.style.left = `${Math.max(8, left)}px`;
  };

  return (
    <>
      <button
        ref={buttonRef}
        type="button"
        className={filtered ? "col-filter-btn is-filtered" : "col-filter-btn"}
        popoverTarget={menuId}
        aria-label={`Filter ${label}`}
        aria-haspopup="dialog"
        aria-pressed={filtered}
      >
        ▾
      </button>
      <div
        ref={menuRef}
        id={menuId}
        popover="auto"
        className="col-filter-menu"
        role="dialog"
        aria-label={`Filter ${label}`}
        onToggle={(event) => {
          const toggle = event.nativeEvent as ToggleEvent;
          if (toggle.newState === "open") {
            placeMenu();
          }
        }}
      >
        <button
          type="button"
          className="col-filter-clear"
          aria-label={`Clear ${label} filter`}
          disabled={!filtered}
          onClick={() => onChange(undefined)}
        >
          Clear this filter
        </button>
        <label className="col-filter-option">
          <input
            type="checkbox"
            checked={allChecked}
            onChange={() => onChange(allChecked ? [] : undefined)}
          />
          Select All
        </label>
        <ul className="col-filter-values">
          {options.map((value) => (
            <li key={value || "(blanks)"}>
              <label className="col-filter-option">
                <input
                  type="checkbox"
                  checked={checked.has(value)}
                  onChange={() => onChange(toggleValue(options, selected, value))}
                />
                {optionLabel(value)}
              </label>
            </li>
          ))}
        </ul>
      </div>
    </>
  );
}

export function SortTh({
  label,
  columnKey,
  sortKey,
  sortDir,
  onSort,
  numeric = false,
  filter,
}: {
  label: string;
  columnKey: string;
  sortKey: string;
  sortDir: SortDir;
  onSort: (key: string) => void;
  numeric?: boolean;
  filter?: ColFilterControl;
}) {
  const active = sortKey === columnKey;
  return (
    <th
      scope="col"
      className={numeric ? "numeric sortable" : "sortable"}
      aria-sort={active ? (sortDir === "asc" ? "ascending" : "descending") : "none"}
    >
      <div className="sort-th">
        <button type="button" aria-label={`Sort by ${label}`} onClick={() => onSort(columnKey)}>
          {label}
          {active ? (sortDir === "asc" ? " ↑" : " ↓") : ""}
        </button>
        {filter ? (
          <FilterMenu
            label={label}
            options={filter.options}
            selected={filter.selected}
            onChange={filter.onChange}
          />
        ) : null}
      </div>
    </th>
  );
}

export function sortHead(
  sort: { sortKey: string; sortDir: SortDir; toggleSort: (key: string) => void },
  label: string,
  columnKey: string,
  numeric = false,
  filter?: ColFilterControl,
) {
  return (
    <SortTh
      label={label}
      columnKey={columnKey}
      sortKey={sort.sortKey}
      sortDir={sort.sortDir}
      onSort={sort.toggleSort}
      numeric={numeric}
      filter={filter}
    />
  );
}
