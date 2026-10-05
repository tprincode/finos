import { useMemo, useState } from "react";
import {
  CALCULATOR_COLUMN_INTENT,
  type ColumnIntentStatus,
} from "./calculatorColumns";

type StatusFilter = "all" | ColumnIntentStatus;

export function FieldIntentScreen() {
  const [status, setStatus] = useState<StatusFilter>("all");
  const rows = useMemo(
    () =>
      CALCULATOR_COLUMN_INTENT.filter((row) => status === "all" || row.status === status),
    [status],
  );
  const stillWrong = CALCULATOR_COLUMN_INTENT.filter((row) => row.status === "still wrong").length;

  return (
    <section className="interest-rate-page" aria-label="Field intent">
      <h2>Field intent</h2>
      <p>
        Calculator columns. One row is one position. Status is matches when the sheet follows
        this contract, and still wrong when it does not. Blank is not $0.
      </p>
      <p role="status">
        {CALCULATOR_COLUMN_INTENT.length} columns, {stillWrong} still wrong.
      </p>
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
        <table aria-label="Calculator column intent">
          <thead>
            <tr>
              <th>Column</th>
              <th>Intent</th>
              <th>Formula</th>
              <th>Status</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((row) => (
              <tr key={row.name}>
                <td>{row.name}</td>
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
