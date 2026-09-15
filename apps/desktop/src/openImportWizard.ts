/** Opens the Import wizard window. Kept out of ImportWizard.tsx so Fast Refresh works. */

export async function openImportWizardWindow(batchId: string, filename: string) {
  const qs = new URLSearchParams({
    wizard: "import",
    batchId,
    filename,
  });
  const url = `/?${qs.toString()}`;
  try {
    const { WebviewWindow } = await import("@tauri-apps/api/webviewWindow");
    const existing = await WebviewWindow.getByLabel("import-wizard");
    if (existing) {
      await existing.close();
    }
    new WebviewWindow("import-wizard", {
      url,
      title: "Import",
      width: 1100,
      height: 740,
      focus: true,
    });
  } catch {
    window.open(url, "import-wizard");
  }
}
