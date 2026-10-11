import { invoke } from "@tauri-apps/api/core";

/**
 * Append one durable line to %LOCALAPPDATA%/com.finos.desktop/ui-errors.log.
 * Best-effort only — never throws into the UI path.
 */
export function logUiError(source: string, message: string): void {
  const text = String(message ?? "").trim();
  if (!text) return;
  const line = `${new Date().toISOString()} [${source}] ${text.replace(/\s+/g, " ").slice(0, 2000)}`;
  void invoke("ui_error_log", { message: line }).catch(() => {
    console.error("finos ui-error", line);
  });
}
