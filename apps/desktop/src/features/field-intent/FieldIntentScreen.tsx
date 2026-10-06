import { useMemo, useState } from "react";
import type { ColumnIntentStatus } from "./calculatorColumns";
import { ALL_FIELD_INTENT } from "./fieldIndex";

type StatusFilter = "all" | ColumnIntentStatus;

function pageLabel(pageId: string): string {
  if (pageId === "calculator") return "Calculator";
  if (pageId === "contract-positions") return "Contract positions";
  return pageId;
}

export function FieldIntentScreen() {
  const pages = useMemo(
    () => [...new Set(ALL_FIELD_INTENT.map((row) => row.pageId))].sort(),
    [],
  );
  const [page, setPage] = useState("all");
  const [status, setStatus] = useState<StatusFilter>("all");
  const rows = useMemo(
    () =>
      ALL_FIELD_INTENT.filter(
        (row) =>
          (page === "all" || row.pageId === page) &&
          (status === "all" || row.status === status),
      ),
    [page, status],
  );
  const stillWrong = rows.filter((row) => row.status === "still wrong").length;

  return (
    <section className="interest-rate-page" aria-label="Field intent">
      <h2>Field intent</h2>
      <p>
        Page is the top level. One row is one field on that page. Status is matches when the
        sheet follows this contract, and still wrong when it does not. Blank is not $0.
      </p>
      <p role="status">
        {rows.length} fields, {stillWrong} still wrong.
      </p>
      <label>
        Page
        <select
          aria-label="Field intent page"
          value={page}
          onChange={(e) => setPage(e.target.value)}
        >
          <option value="all">All</option>
          {pages.map((id) => (
            <option key={id} value={id}>
              {pageLabel(id)}
            </option>
          ))}
        </select>
      </label>
      <label>
        Status
        <select
          aria-label="Field intent status"
          value={status}
          onChange={(e) => setStatus(e.target.value as StatusFilter)}
        >
          <option value="all">All</option>
          <option value="matches">Matches</option>
          <option value="still wrong">Still wrong</option>
        </select>
      </label>
      <div className="table-wrap">
        <table aria-label="Field intent">
          <thead>
            <tr>
              <th>Page</th>
              <th>Column</th>
              <th>Component</th>
              <th>Intent</th>
              <th>Formula</th>
              <th>Status</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((row) => (
              <tr key={`${row.pageId}-${row.componentId}-${row.name}`}>
                <td>{pageLabel(row.pageId)}</td>
                <td>{row.name}</td>
                <td>{row.componentId}</td>
                <td>{row.intent}</td>
                <td>{row.formula}</td>
                <td>{row.status}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </section>
  );
}
