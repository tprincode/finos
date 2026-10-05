import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { formatUsd } from "@finos/ui-components";
import type {
  DisbursementWeekReportExportGet,
  DisbursementWeekReportGet,
} from "@finos/app-contracts";
import { LocalTauriFinanceClient } from "../../financeClient";

const reportClient = new LocalTauriFinanceClient();

function headerGroups(columns: DisbursementWeekReportGet["columns"]) {
  const groups: Array<{ label: string; span: number; lone: boolean }> = [];
  let i = 0;
  while (i < columns.length) {
    const group = columns[i].group;
    if (!group) {
      groups.push({ label: columns[i].label, span: 1, lone: true });
      i += 1;
      continue;
    }
    let span = 1;
    while (i + span < columns.length && columns[i + span].group === group) span += 1;
    groups.push({ label: group, span, lone: false });
    i += span;
  }
  return groups;
}

export function useWeekReport(asOfDate: string, scope: "posted" | "remaining") {
  const [open, setOpen] = useState(false);
  const [report, setReport] = useState<DisbursementWeekReportGet | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const toggle = async () => {
    if (open) {
      setOpen(false);
      return;
    }
    setBusy(true);
    setError(null);
    const result = await reportClient.executeQuery("DisbursementWeekReportGet", {
      asOfDate,
      scope,
    });
    setBusy(false);
    if (!result.ok || !result.bodyJson) {
      setError(result.errorCode ?? "Week report failed");
      setOpen(true);
      return;
    }
    setReport(JSON.parse(result.bodyJson) as DisbursementWeekReportGet);
    setOpen(true);
  };

  return { open, busy, error, report, toggle };
}

export function WeekReportTable({
  report,
  error,
}: {
  report: DisbursementWeekReportGet | null;
  error: string | null;
}) {
  const [exportNote, setExportNote] = useState<string | null>(null);
  const [exporting, setExporting] = useState(false);
  if (error) return <p role="alert">{error}</p>;
  if (!report) return null;
  const scale = report.scale ?? 2;
  const groups = headerGroups(report.columns);
  const remaining = report.reportKind === "remaining";
  const exportExcel = async () => {
    setExporting(true);
    setExportNote(null);
    const result = await reportClient.executeQuery("DisbursementWeekReportExportGet", {
      asOfDate: report.asOfDate,
      scope: report.reportKind,
    });
    if (!result.ok || !result.bodyJson) {
      setExportNote(result.errorCode ?? "Export failed");
      setExporting(false);
      return;
    }
    const body = JSON.parse(result.bodyJson) as DisbursementWeekReportExportGet;
    const bytes = Uint8Array.from(atob(body.bytesBase64), (c) => c.charCodeAt(0));
    try {
      const path = await invoke<string>("save_local_bytes", {
        defaultFileName: body.defaultFileName,
        bytes: Array.from(bytes),
      });
      setExportNote(`Saved ${path}`);
    } catch (err: unknown) {
      setExportNote(String(err));
    } finally {
      setExporting(false);
    }
  };
  return (
    <div
      className="table-wrap"
      aria-label={remaining ? "Remaining this year" : "Current year totals"}
    >
      <p>
        {remaining
          ? `Remaining this year from ${report.asOfDate}, from the element plan.`
          : `Current year totals. Posted Saturday–Friday weeks through ${report.asOfDate}.`}{" "}
        WPD is 1099. Total distribution is Income + Fed Tax + State Tax.{" "}
        {remaining ? "Remaining" : "To date"} is the sum of those amounts.
        <button
          type="button"
          aria-label="Export to Excel"
          disabled={exporting}
          onClick={() => void exportExcel()}
        >
          Export to Excel
        </button>
      </p>
      {exportNote ? <p>{exportNote}</p> : null}
      <table aria-label={remaining ? "Remaining this year" : "Current year totals"}>
        <thead>
          <tr>
            <th rowSpan={2} scope="col">
              Week
            </th>
            {groups.map((group) =>
              group.lone ? (
                <th key={group.label} rowSpan={2} scope="col" className="numeric">
                  {group.label}
                </th>
              ) : (
                <th key={group.label} colSpan={group.span} scope="colgroup">
                  {group.label}
                </th>
              ),
            )}
          </tr>
          <tr>
            {report.columns.map((col) =>
              col.group ? (
                <th key={col.key} scope="col" className="numeric">
                  {col.label}
                </th>
              ) : null,
            )}
          </tr>
        </thead>
        <tbody>
          {report.rows.map((row) => (
            <tr key={row.periodEnd}>
              <th scope="row">{row.periodEnd}</th>
              {row.cells.map((cell, i) => (
                    <td key={report.columns[i]?.key ?? i} className="numeric">
                      {cell ? formatUsd(cell.amountMinor, scale) : ""}
                    </td>
              ))}
            </tr>
          ))}
        </tbody>
        <tfoot>
          <tr>
                <th scope="row">{remaining ? "Remaining" : "To date"}</th>
            {report.ytdMinor.map((amount, i) => (
              <td key={report.columns[i]?.key ?? i} className="numeric">
                {amount ? formatUsd(amount, scale) : ""}
              </td>
            ))}
          </tr>
        </tfoot>
      </table>
    </div>
  );
}
