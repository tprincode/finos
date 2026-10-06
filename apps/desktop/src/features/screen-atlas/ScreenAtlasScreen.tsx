import { Fragment, useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  ATLAS_DEFAULT_SETTLE_MS,
  ATLAS_TARGETS,
  type AtlasCmDesk,
  type AtlasScreen,
  type AtlasTarget,
} from "./atlasTargets";
import { beginAtlasFreeze, endAtlasFreeze } from "./atlasSession";
import { captureMainPngBytes } from "./captureDom";
import { waitForScreenReady } from "./waitForReady";
import type { CatalogModule } from "../components/ComponentRegistry";
import { groupByScreen } from "../components/screenGroups";

export type ScreenAtlasScreenProps = {
  asOfDate: string;
  isBusy: boolean;
  onNavigate: (screen: AtlasScreen, cmDesk?: AtlasCmDesk) => void;
  onDone?: (message: string) => void;
  /** When true, start capture once after mount (token / owner auto-run). */
  autoStart?: boolean;
  modules?: CatalogModule[];
  onBind?: (api: {
    runAll: () => void;
    runPage: (targets: AtlasTarget[]) => void;
    running: boolean;
  }) => void;
  /** Capture lives on the registry. It is not its own component. */
  embedded?: boolean;
};

type Progress = {
  current: number;
  total: number;
  id: string;
  detail: string;
};

function sleep(ms: number): Promise<void> {
  return new Promise((r) => window.setTimeout(r, ms));
}

function mergeIndex(prior: string, fresh: string, ids: string[]): string {
  if (!prior.trim()) return fresh;
  const freshLines = fresh.split("\n");
  let lines = prior.split("\n");
  for (const id of ids) {
    const atFresh = freshLines.findIndex((line) => line.startsWith(`| ${id} |`));
    if (atFresh < 0) continue;
    const idLine = freshLines[atFresh];
    const moduleLine = freshLines[atFresh + 1] ?? "";
    const at = lines.findIndex((line) => line.startsWith(`| ${id} |`));
    if (at >= 0) {
      lines[at] = idLine;
      if (moduleLine.startsWith("| |") && (lines[at + 1] ?? "").startsWith("| |")) {
        lines[at + 1] = moduleLine;
      }
    } else {
      lines = [...lines, idLine, moduleLine];
    }
  }
  return lines.join("\n");
}

function bytesToPngDataUrl(bytes: number[] | Uint8Array): string {
  const arr = bytes instanceof Uint8Array ? bytes : Uint8Array.from(bytes);
  let bin = "";
  const chunk = 0x8000;
  for (let i = 0; i < arr.length; i += chunk) {
    const slice = arr.subarray(i, i + chunk);
    bin += String.fromCharCode.apply(null, Array.from(slice) as number[]);
  }
  return `data:image/png;base64,${btoa(bin)}`;
}

function modulesForTarget(
  target: AtlasTarget,
  modules: CatalogModule[],
): CatalogModule[] {
  return modules.filter(
    (row) =>
      row.screen === target.screen && (row.cmDesk || "") === (target.cmDesk || ""),
  );
}

function moduleLine(rows: CatalogModule[]): string {
  if (rows.length === 0) return "Modules: (add the missing catalog row)";
  return `Modules: ${rows.map((row) => `${row.title} (\`${row.id}\`)`).join(", ")}`;
}

function buildIndexMd(args: {
  stamp: string;
  asOf: string;
  outDir: string;
  rows: Array<{ target: AtlasTarget; file: string; ok: boolean; error?: string }>;
  modules: CatalogModule[];
}): string {
  const lines = [
    `# Screen Atlas — ${args.stamp}`,
    "",
    "Documentation / human eyeball captures. **Not** a CI pixel golden gate.",
    "Automated UI regression stays on `cargo test -p golden-harness` (source/query goldens).",
    "The same screen set as Component Registry and Template_UiModules.xlsx.",
    "",
    `- Captured at: ${args.stamp}`,
    `- Pinned asOf: ${args.asOf}`,
    `- Output: \`${args.outDir}\``,
    `- LastPrice / declaration fleet: skipped during this run`,
    `- Chart animation: off while frozen`,
    `- Color scheme: light (docs contrast)`,
    "",
  ];
  const byId = new Map(args.rows.map((row) => [row.target.id, row]));
  const grouped = groupByScreen(args.rows.map((row) => row.target));
  for (const group of grouped.groups) {
    const blocks = [
      { heading: group.label, items: group.direct },
      ...group.desks.map((desk) => ({
        heading: `${group.label} — ${desk.label}`,
        items: desk.items,
      })),
    ];
    for (const block of blocks) {
      if (block.items.length === 0) continue;
      lines.push(`## ${block.heading}`, "");
      lines.push("| Id | Menu path | File | Status |");
      lines.push("|---|---|---|---|");
      for (const target of block.items) {
        const row = byId.get(target.id);
        const status = row?.ok ? "ok" : `FAIL: ${row?.error ?? "?"}`;
        const file = row?.file ?? `${target.id}.png`;
        lines.push(`| ${target.id} | ${target.menuPath} | ${file} | ${status} |`);
        lines.push(`| | ${moduleLine(modulesForTarget(target, args.modules))} | | |`);
      }
      lines.push("");
    }
  }
  const also = args.modules.filter((row) => !row.screen);
  if (also.length > 0) {
    lines.push("## Also registered", "");
    for (const row of also) {
      lines.push(`- ${row.title} (\`${row.id}\`)`);
    }
    lines.push("");
  }
  return lines.join("\n");
}

export function ScreenAtlasScreen({
  asOfDate,
  isBusy,
  onNavigate,
  onDone,
  autoStart = false,
  modules = [],
  onBind,
  embedded = false,
}: ScreenAtlasScreenProps) {
  const [running, setRunning] = useState(false);
  const [progress, setProgress] = useState<Progress | null>(null);
  const [log, setLog] = useState<string[]>([]);
  const [lastDir, setLastDir] = useState<string | null>(null);
  const [viewDay, setViewDay] = useState<string | null>(null);
  const [pngNames, setPngNames] = useState<string[]>([]);
  const [listBusy, setListBusy] = useState(false);
  const [previewName, setPreviewName] = useState<string | null>(null);
  const [previewUrl, setPreviewUrl] = useState<string | null>(null);
  const [previewBusy, setPreviewBusy] = useState(false);
  const [folderError, setFolderError] = useState<string | null>(null);
  const busyRef = useRef(isBusy);
  busyRef.current = isBusy;
  const startedAuto = useRef(false);

  const append = useCallback((line: string) => {
    setLog((prev) => [...prev.slice(-40), line]);
  }, []);

  /** List only — never bulk-load PNG bytes (that OOMed the webview). */
  const refreshList = useCallback(async (day: string) => {
    setListBusy(true);
    setFolderError(null);
    setPreviewName(null);
    setPreviewUrl(null);
    try {
      const path = await invoke<string>("screen_atlas_folder_path", { day });
      setLastDir(path);
      setViewDay(day);
      const names = await invoke<string[]>("screen_atlas_list_pngs", { day });
      setPngNames(names);
    } catch (err: unknown) {
      setFolderError(String(err));
      setPngNames([]);
    } finally {
      setListBusy(false);
    }
  }, []);

  const openFolder = useCallback(async () => {
    setFolderError(null);
    try {
      const day =
        viewDay ?? (await invoke<string | null>("screen_atlas_latest_day")) ?? "";
      const path = await invoke<string>("screen_atlas_open_folder", { day });
      setLastDir(path);
      if (day) {
        setViewDay(day);
        await refreshList(day);
      }
    } catch (err: unknown) {
      setFolderError(String(err));
    }
  }, [refreshList, viewDay]);

  const showPreview = useCallback(
    async (name: string) => {
      if (!viewDay) return;
      setPreviewBusy(true);
      setFolderError(null);
      setPreviewName(name);
      setPreviewUrl(null);
      try {
        const bytes = await invoke<number[]>("screen_atlas_read_png", {
          day: viewDay,
          fileName: name,
        });
        setPreviewUrl(bytesToPngDataUrl(bytes));
      } catch (err: unknown) {
        setFolderError(String(err));
        setPreviewName(null);
      } finally {
        setPreviewBusy(false);
      }
    },
    [viewDay],
  );

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const day = await invoke<string | null>("screen_atlas_latest_day");
        if (cancelled || !day) return;
        await refreshList(day);
      } catch {
        /* no prior run / command unavailable */
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [refreshList]);

  const runAtlas = useCallback(async (targets: AtlasTarget[], mode: "all" | "page") => {
    if (running) return;
    setRunning(true);
    setLog([]);
    beginAtlasFreeze(asOfDate);
    const stamp = new Date().toISOString();
    const day = stamp.slice(0, 10);
    const rows: Array<{
      target: AtlasTarget;
      file: string;
      ok: boolean;
      error?: string;
    }> = [];
    let outDir = "";

    try {
      const total = targets.length;
      for (let i = 0; i < targets.length; i += 1) {
        const target = targets[i];
        setProgress({
          current: i + 1,
          total,
          id: target.id,
          detail: target.menuPath,
        });
        append(`Navigate ${target.id}…`);
        onNavigate(target.screen, target.cmDesk);
        await sleep(250);
        append(`Wait ready ${target.id} (activity + Loading…)…`);
        await waitForScreenReady({
          isBusy: () => busyRef.current,
          settleMs: target.settleMs ?? ATLAS_DEFAULT_SETTLE_MS,
          quietMs: 1500,
        });

        const file = `${target.id}.png`;
        try {
          append(`Capture full page ${target.id}…`);
          const bytes = await captureMainPngBytes();
          if (bytes.length === 0) {
            throw new Error("empty capture");
          }
          const saved = await invoke<string>("screen_atlas_save", {
            day,
            fileName: file,
            bytes: Array.from(bytes),
          });
          if (!outDir && saved) {
            const slash = Math.max(saved.lastIndexOf("\\"), saved.lastIndexOf("/"));
            outDir = slash >= 0 ? saved.slice(0, slash) : saved;
          }
          rows.push({ target, file, ok: true });
          append(`Saved ${file}`);
        } catch (err: unknown) {
          const error = String(err);
          rows.push({ target, file, ok: false, error });
          append(`FAIL ${target.id}: ${error}`);
        }
      }

      onNavigate("components");
      await sleep(200);

      const freshIndex = buildIndexMd({
        stamp,
        asOf: asOfDate,
        outDir: outDir || `(evidence/screen-atlas/${day})`,
        rows,
        modules,
      });
      let indexBody = freshIndex;
      if (mode === "page") {
        const prior = await invoke<string>("screen_atlas_read_text", {
          day,
          fileName: "index.md",
        }).catch(() => "");
        indexBody = mergeIndex(prior, freshIndex, rows.map((row) => row.target.id));
      }
      const indexPath = await invoke<string>("screen_atlas_save_text", {
        day,
        fileName: "index.md",
        text: indexBody,
      });
      setLastDir(outDir || indexPath);
      setViewDay(day);
      const okCount = rows.filter((r) => r.ok).length;
      const msg = `Screen Atlas done: ${okCount}/${rows.length} PNGs → ${outDir || indexPath}`;
      append(msg);
      onDone?.(msg);
      await refreshList(day);
    } catch (err: unknown) {
      const msg = `Screen Atlas aborted: ${String(err)}`;
      append(msg);
      onDone?.(msg);
      onNavigate("components");
    } finally {
      endAtlasFreeze();
      setRunning(false);
      setProgress(null);
    }
  }, [asOfDate, append, modules, onDone, onNavigate, refreshList, running]);

  const runAll = useCallback(() => {
    void runAtlas(ATLAS_TARGETS, "all");
  }, [runAtlas]);

  const runPage = useCallback(
    (targets: AtlasTarget[]) => {
      void runAtlas(targets, "page");
    },
    [runAtlas],
  );

  useEffect(() => {
    onBind?.({ runAll, runPage, running });
  }, [onBind, runAll, runPage, running]);

  useEffect(() => {
    if (!autoStart || startedAuto.current || running) return;
    startedAuto.current = true;
    void runAtlas(ATLAS_TARGETS, "all");
  }, [autoStart, runAtlas, running]);

  return (
    <section
      className="actions screen-atlas-page"
      aria-label={embedded ? "Registry capture" : "Screen Atlas"}
    >
      {embedded ? null : <h3>Capture</h3>}
      <div className="row screen-atlas-actions">
        <button
          type="button"
          disabled={running}
          aria-label="Capture All"
          onClick={runAll}
        >
          {running ? "Capturing…" : "Capture All"}
        </button>
        <button
          type="button"
          disabled={running}
          aria-label="Open screen atlas folder in Explorer"
          onClick={() => void openFolder()}
        >
          Open folder in Explorer
        </button>
        {viewDay ? (
          <button
            type="button"
            disabled={running || listBusy}
            aria-label="Reload screen atlas file list"
            onClick={() => void refreshList(viewDay)}
          >
            Reload list
          </button>
        ) : null}
      </div>
      {lastDir ? (
        <p aria-label="Screen atlas output folder">
          Folder:{" "}
          <button
            type="button"
            className="screen-atlas-path-link"
            aria-label="Open screen atlas folder path"
            onClick={() => void openFolder()}
          >
            {lastDir}
          </button>
        </p>
      ) : null}
      {folderError ? (
        <p className="screen-atlas-error" role="alert">
          {folderError}
        </p>
      ) : null}
      {progress ? (
        <div
          className="screen-atlas-progress"
          role="status"
          aria-busy="true"
          aria-label="Screen atlas progress"
        >
          <p>
            {progress.current} / {progress.total}: {progress.id} —{" "}
            {progress.detail}
          </p>
          <progress max={progress.total} value={progress.current} />
        </div>
      ) : null}
      {log.length > 0 ? (
        <pre className="screen-atlas-log" aria-label="Screen atlas log">
          {log.join("\n")}
        </pre>
      ) : null}

      <details className="registry-capture-extra" open={!embedded}>
        <summary>Captures</summary>
        <p>
          Walk every menu surface, expand scroll panes, and save full-height PNGs
          plus an index. This is <strong>documentation / human review</strong> —
          not a CI pixel golden. Regression stays on{" "}
          <code>cargo test -p golden-harness</code>.
        </p>
        <h3>Viewer{viewDay ? ` — ${viewDay}` : ""}</h3>
        <p>
          Click a name to preview one image (loads on demand). Or use{" "}
          <strong>Open folder in Explorer</strong> for the OS photo viewer.
        </p>
      {listBusy ? (
        <p role="status" aria-busy="true">
          Listing PNGs…
        </p>
      ) : pngNames.length === 0 ? (
        <p>No captures yet. Capture All, then open the folder or reload.</p>
      ) : (
        <PngGroups
          names={pngNames}
          previewName={previewName}
          previewBusy={previewBusy}
          onPreview={(name) => void showPreview(name)}
        />
      )}
      {previewBusy ? (
        <p role="status" aria-busy="true">
          Loading preview…
        </p>
      ) : null}
      {previewUrl && previewName ? (
        <div className="screen-atlas-preview" aria-label="Screen atlas preview">
          <div className="row">
            <strong>{previewName}</strong>
            <button
              type="button"
              aria-label="Close preview"
              onClick={() => {
                setPreviewName(null);
                setPreviewUrl(null);
              }}
            >
              Close preview
            </button>
          </div>
          <img src={previewUrl} alt={previewName} />
        </div>
      ) : null}

      <div className="table-wrap">
        <table aria-label="Atlas targets">
          <thead>
            <tr>
              <th scope="col">Id</th>
              <th scope="col">Menu path</th>
              <th scope="col">Label</th>
              <th scope="col">Modules</th>
            </tr>
          </thead>
          <tbody>
            <AtlasTargetRows modules={modules.filter((row) => row.id !== "screen-atlas")} />
          </tbody>
        </table>
      </div>
      </details>
    </section>
  );
}

function PngGroups({
  names,
  previewName,
  previewBusy,
  onPreview,
}: {
  names: string[];
  previewName: string | null;
  previewBusy: boolean;
  onPreview: (name: string) => void;
}) {
  const grouped = groupByScreen(ATLAS_TARGETS);
  const used = new Set<string>();
  const sections: Array<{ heading: string; names: string[] }> = [];
  const take = (heading: string, ids: string[]) => {
    const files = ids
      .map((id) => names.find((name) => name.replace(/\.png$/i, "") === id))
      .filter((name): name is string => Boolean(name));
    for (const file of files) used.add(file);
    if (files.length > 0) sections.push({ heading, names: files });
  };
  for (const group of grouped.groups) {
    if (group.direct.length > 0) {
      take(group.label, group.direct.map((target) => target.id));
    }
    for (const desk of group.desks) {
      take(
        `${group.label} — ${desk.label}`,
        desk.items.map((target) => target.id),
      );
    }
  }
  const rest = names.filter((name) => !used.has(name));
  if (rest.length > 0) sections.push({ heading: "Also registered", names: rest });
  return (
    <>
      {sections.map((section) => (
        <div key={section.heading}>
          <h4>{section.heading}</h4>
          <ul className="screen-atlas-file-list" aria-label="Screen atlas PNG list">
            {section.names.map((name) => (
              <li key={name}>
                <button
                  type="button"
                  className={
                    previewName === name
                      ? "screen-atlas-file-btn is-selected"
                      : "screen-atlas-file-btn"
                  }
                  aria-label={`Preview ${name}`}
                  disabled={previewBusy}
                  onClick={() => onPreview(name)}
                >
                  {name.replace(/\.png$/i, "")}
                </button>
              </li>
            ))}
          </ul>
        </div>
      ))}
    </>
  );
}

function AtlasTargetRows({ modules }: { modules: CatalogModule[] }) {
  const grouped = groupByScreen(ATLAS_TARGETS);
  const blocks: Array<{ key: string; heading: string; items: AtlasTarget[] }> = [];
  for (const group of grouped.groups) {
    if (group.direct.length > 0) {
      blocks.push({ key: group.screen, heading: group.label, items: group.direct });
    }
    for (const desk of group.desks) {
      blocks.push({
        key: `${group.screen}-${desk.desk}`,
        heading: `${group.label} — ${desk.label}`,
        items: desk.items,
      });
    }
  }
  return (
    <>
      {blocks.map((block) => (
        <Fragment key={block.key}>
          <tr>
            <th scope="colgroup" colSpan={4}>
              {block.heading}
            </th>
          </tr>
          {block.items.map((target) => {
            const rows = modulesForTarget(target, modules);
            return (
              <tr key={target.id}>
                <td>{target.id}</td>
                <td>{target.menuPath}</td>
                <td>{target.label}</td>
                <td>
                  {rows.length
                    ? rows.map((row) => row.title).join(", ")
                    : "—"}
                </td>
              </tr>
            );
          })}
        </Fragment>
      ))}
    </>
  );
}
