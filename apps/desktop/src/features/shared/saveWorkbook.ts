import { invoke } from "@tauri-apps/api/core";
import { LocalTauriFinanceClient } from "../../financeClient";

const client = new LocalTauriFinanceClient();

type WorkbookBody = {
  defaultFileName?: string;
  bytesBase64?: string;
};

async function saveBytes(body: WorkbookBody): Promise<string> {
  if (!body.bytesBase64) throw new Error("Export failed");
  const bytes = Uint8Array.from(atob(body.bytesBase64), (c) => c.charCodeAt(0));
  if (bytes.length === 0) throw new Error("Export failed");
  return invoke<string>("save_local_bytes", {
    defaultFileName: body.defaultFileName ?? "export.xlsx",
    bytes: Array.from(bytes),
  });
}

/** One download path for Calculator and Tools → Components Excel. */
export async function saveComponentWorkbook(args: {
  moduleId: string;
  partId?: string;
  asOfDate: string;
}): Promise<string> {
  const result = await client.executeQuery("ComponentExportGet", {
    moduleId: args.moduleId,
    partId: args.partId ?? "",
    asOfDate: args.asOfDate,
  });
  if (!result.ok || !result.bodyJson) {
    throw new Error(result.errorCode ?? "Export failed");
  }
  return saveBytes(JSON.parse(result.bodyJson) as WorkbookBody);
}

export async function savePageWorkbook(args: {
  pageLabel: string;
  asOfDate: string;
  lines: Array<{ moduleId: string; partId: string }>;
}): Promise<string> {
  const result = await client.executeQuery("ComponentPageExportGet", {
    pageLabel: args.pageLabel,
    asOfDate: args.asOfDate,
    lines: args.lines,
  });
  if (!result.ok || !result.bodyJson) {
    throw new Error(result.errorCode ?? "Export failed");
  }
  return saveBytes(JSON.parse(result.bodyJson) as WorkbookBody);
}
