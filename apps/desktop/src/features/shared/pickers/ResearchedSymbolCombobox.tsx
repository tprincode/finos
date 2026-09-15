export type ResearchedSymbolOption = {
  securityId: string;
  symbol: string;
  name: string;
  openLotCount?: number;
};

export function ResearchedSymbolCombobox({
  options,
  query,
  onQueryChange,
  open,
  onOpenChange,
  selectedId,
  onSelect,
  disabled,
  inputAriaLabel,
  listId,
  listAriaLabel,
}: {
  options: ResearchedSymbolOption[];
  query: string;
  onQueryChange: (query: string) => void;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  selectedId: string;
  onSelect: (row: ResearchedSymbolOption) => void;
  disabled?: boolean;
  inputAriaLabel: string;
  listId: string;
  listAriaLabel: string;
}) {
  return (
    <label className="symbol-combobox">
      Researched symbol
      <input
        aria-label={inputAriaLabel}
        aria-expanded={open}
        aria-controls={listId}
        aria-autocomplete="list"
        role="combobox"
        value={query}
        placeholder="Type to filter (e.g. NV)"
        autoComplete="off"
        onChange={(e) => {
          onQueryChange(e.target.value.toUpperCase());
          onOpenChange(true);
        }}
        onFocus={() => onOpenChange(true)}
        onBlur={() => {
          window.setTimeout(() => onOpenChange(false), 150);
        }}
        onKeyDown={(e) => {
          if (e.key === "Enter" && options[0]) {
            e.preventDefault();
            onSelect(options[0]);
          }
          if (e.key === "Escape") {
            onOpenChange(false);
          }
        }}
        disabled={disabled}
      />
      {open && options.length > 0 ? (
        <ul
          id={listId}
          className="symbol-combobox-list"
          role="listbox"
          aria-label={listAriaLabel}
        >
          {options.map((row) => (
            <li
              key={row.securityId}
              role="option"
              aria-selected={row.securityId === selectedId}
              onMouseDown={(e) => {
                e.preventDefault();
                onSelect(row);
              }}
            >
              {row.symbol}
              {row.name ? ` — ${row.name}` : ""}
              {row.openLotCount === 0 ? " (no lots yet)" : ""}
            </li>
          ))}
        </ul>
      ) : null}
    </label>
  );
}
