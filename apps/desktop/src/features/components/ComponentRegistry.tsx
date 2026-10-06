import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { CoreFunctionsGet } from "@finos/app-contracts";
import { LocalTauriFinanceClient } from "../../financeClient";
import {
  ATLAS_TARGETS,
  type AtlasCmDesk,
  type AtlasScreen,
  type AtlasTarget,
} from "../screen-atlas/atlasTargets";
import { ScreenAtlasScreen } from "../screen-atlas/ScreenAtlasScreen";
import { groupByScreen, ownerPageGroups } from "./screenGroups";
import { fieldNamesFor } from "../field-intent/fieldIndex";

export type CatalogModule = NonNullable<CoreFunctionsGet["modules"]>[number];

const exportClient = new LocalTauriFinanceClient();

type OwnerMeta = {
  purpose: string;
  inputs: string;
  output: string;
  moneyRule: string;
};

type MetaFile = Record<string, Partial<OwnerMeta>>;

const EMPTY_META: OwnerMeta = { purpose: "", inputs: "", output: "", moneyRule: "" };

function metaKey(moduleId: string, partId: string): string {
  return `${moduleId}/${partId}`;
}

function ownerOf(file: MetaFile, moduleId: string, partId: string): OwnerMeta {
  const row = file[metaKey(moduleId, partId)] ?? {};
  return {
    purpose: row.purpose ?? "",
    inputs: row.inputs ?? "",
    output: row.output ?? "",
    moneyRule: row.moneyRule ?? "",
  };
}

/** Screen Atlas is the capture tool on this page, not a catalog component. */
function listedModules(modules: CatalogModule[]): CatalogModule[] {
  return modules.filter((row) => row.id !== "screen-atlas");
}

function pageTables(modules: CatalogModule[]): string {
  const names = new Set<string>();
  for (const row of modules) {
    for (const cite of row.sqliteTables ?? []) {
      if (cite.name.trim()) names.add(cite.name);
    }
  }
  if (names.size === 0) return "not proven";
  return [...names].sort().join(", ");
}

type ComponentLine = {
  key: string;
  name: string;
  moduleId: string;
  partId: string;
  kind: string;
  folder: string;
  host: string;
  cores: string;
  query: string;
  catalogPurpose: string;
  menu: string;
};

function menuPathOf(row: CatalogModule): string {
  const menu = row.menu;
  if (!menu) return "";
  if (menu.kind === "embedded") return "Inside Component Registry";
  const target = ATLAS_TARGETS.find((item) => {
    if (menu.kind === "desk") {
      return item.cmDesk === row.cmDesk && item.menuPath.endsWith(menu.label);
    }
    if (item.id === "cash-management-default" && menu.kind !== "nav") return false;
    return (
      item.screen === row.screen &&
      (item.menuPath === menu.label || item.menuPath.endsWith(`→ ${menu.label}`))
    );
  });
  return target?.menuPath ?? menu.label;
}

function coresOf(ids: string[] | undefined): string {
  return ids?.length ? ids.join(", ") : "";
}

function componentLines(modules: CatalogModule[]): ComponentLine[] {
  const lines: ComponentLine[] = [];
  for (const row of modules) {
    const parts = row.parts ?? [];
    if (parts.length === 0) {
      lines.push({
        key: row.id,
        name: row.title,
        moduleId: row.id,
        partId: "",
        kind: "function",
        folder: row.folder,
        host: row.host || "",
        cores: coresOf(row.coreFunctionIds),
        query: row.exportKind || "",
        catalogPurpose: row.description || "",
        menu: menuPathOf(row),
      });
      continue;
    }
    for (const part of parts) {
      lines.push({
        key: `${row.id}:${part.id}`,
        name: part.title,
        moduleId: row.id,
        partId: part.id,
        kind: part.kind,
        folder: row.folder,
        host: row.host || "",
        cores: coresOf(part.coreFunctionIds?.length ? part.coreFunctionIds : row.coreFunctionIds),
        query: part.exportKind || row.exportKind || "",
        catalogPurpose: part.description || "",
        menu: menuPathOf(row),
      });
    }
  }
  return lines;
}

function targetsFor(screen: string, desk?: string): AtlasTarget[] {
  if (screen === "components") {
    return ATLAS_TARGETS.filter((target) => target.screen === "components");
  }
  if (desk) {
    return ATLAS_TARGETS.filter(
      (target) => target.screen === screen && target.cmDesk === desk,
    );
  }
  return ATLAS_TARGETS.filter((target) => target.screen === screen && !target.cmDesk);
}

type PageBlock = {
  key: string;
  label: string;
  modules: CatalogModule[];
  targets: AtlasTarget[];
};

function pageBlocks(modules: CatalogModule[]): {
  pages: PageBlock[];
  also: CatalogModule[];
} {
  const listed = listedModules(modules);
  const order = groupByScreen(listed);
  const owned = ownerPageGroups(listed);
  const source = owned.groups.length > 0 ? owned : order;
  const pages: PageBlock[] = [];
  for (const group of source.groups) {
    if (group.direct.length > 0) {
      pages.push({
        key: group.screen,
        label: group.label,
        modules: group.direct,
        targets: targetsFor(group.screen),
      });
    }
    for (const desk of group.desks) {
      pages.push({
        key: `${group.screen}-${desk.desk}`,
        label: desk.label,
        modules: desk.items,
        targets: targetsFor(group.screen, desk.desk),
      });
    }
  }
  return { pages, also: source.also };
}

export function ComponentRegistry({
  modules,
  asOfDate,
  isBusy,
  onNavigate,
  onDone,
  autoStart = false,
}: {
  modules?: CatalogModule[];
  asOfDate: string;
  isBusy: boolean;
  onNavigate: (screen: AtlasScreen, cmDesk?: AtlasCmDesk) => void;
  onDone?: (message: string) => void;
  autoStart?: boolean;
}) {
  const outline = modules?.length ? pageBlocks(modules) : null;
  const [atlas, setAtlas] = useState<{
    runAll: () => void;
    runPage: (targets: AtlasTarget[]) => void;
    running: boolean;
  } | null>(null);
  const [exportNote, setExportNote] = useState<string | null>(null);
  const [meta, setMeta] = useState<MetaFile>({});
  const [editor, setEditor] = useState<
    (OwnerMeta & { key: string; name: string; moduleId: string; partId: string }) | null
  >(null);

  useEffect(() => {
    void invoke<string>("component_metadata_get")
      .then((text) => {
        const parsed = JSON.parse(text) as MetaFile;
        setMeta(parsed && typeof parsed === "object" ? parsed : {});
      })
      .catch(() => setMeta({}));
  }, []);

  const exportPage = async (label: string, rows: CatalogModule[]) => {
    setExportNote(null);
    try {
      const lines = componentLines(rows).map((line) => ({
        moduleId: line.moduleId,
        partId: line.partId,
      }));
      const result = await exportClient.executeQuery("ComponentPageExportGet", {
        pageLabel: label,
        asOfDate,
        lines,
      });
      if (!result.ok || !result.bodyJson) {
        throw new Error(result.errorCode ?? "Export failed");
      }
      const body = JSON.parse(result.bodyJson) as {
        defaultFileName?: string;
        bytesBase64?: string;
      };
      if (!body.bytesBase64) throw new Error("Export failed");
      const bytes = Uint8Array.from(atob(body.bytesBase64), (c) => c.charCodeAt(0));
      if (bytes.length === 0) throw new Error("Export failed");
      const path = await invoke<string>("save_local_bytes", {
        defaultFileName: body.defaultFileName ?? `${label}.xlsx`,
        bytes: Array.from(bytes),
      });
      setExportNote(`Saved ${path}`);
    } catch (err: unknown) {
      setExportNote(err instanceof Error ? err.message : String(err));
    }
  };

  const openEditor = (line: ComponentLine) => {
    const owner = ownerOf(meta, line.moduleId, line.partId);
    setEditor({
      key: line.key,
      name: line.name,
      moduleId: line.moduleId,
      partId: line.partId,
      purpose: owner.purpose.trim() ? owner.purpose : line.catalogPurpose,
      inputs: owner.inputs,
      output: owner.output,
      moneyRule: owner.moneyRule,
    });
  };

  const saveEditor = async () => {
    if (!editor) return;
    const next: OwnerMeta = {
      purpose: editor.purpose,
      inputs: editor.inputs,
      output: editor.output,
      moneyRule: editor.moneyRule,
    };
    try {
      await invoke("component_metadata_put", {
        moduleId: editor.moduleId,
        partId: editor.partId,
        purpose: next.purpose,
        inputs: next.inputs,
        output: next.output,
        moneyRule: next.moneyRule,
      });
      setMeta((prev) => ({ ...prev, [metaKey(editor.moduleId, editor.partId)]: next }));
      setEditor(null);
    } catch (err: unknown) {
      setExportNote(err instanceof Error ? err.message : String(err));
    }
  };

  return (
    <section className="component-registry actions" aria-label="Component Registry">
      <h2>Component Registry</h2>
      <p className="registry-note">
        UI extraction catalog: whether a screen has left App.tsx. Not a
        domain-completeness index and not a plugin host. Core functions stay in
        Settings. Capture is part of this page.
      </p>
      <ScreenAtlasScreen
        asOfDate={asOfDate}
        isBusy={isBusy}
        autoStart={autoStart}
        modules={listedModules(modules ?? [])}
        onDone={onDone}
        onNavigate={onNavigate}
        onBind={setAtlas}
        embedded
      />
      {exportNote ? <p role="status">{exportNote}</p> : null}
      {outline ? (
        outline.pages.map((page) => (
          <PageSection
            key={page.key}
            label={page.label}
            modules={page.modules}
            running={atlas?.running}
            canCapture={page.targets.length > 0}
            onCapture={() => atlas?.runPage(page.targets)}
            onExportPage={() => void exportPage(page.label, page.modules)}
            meta={meta}
            editor={editor}
            onEdit={openEditor}
            onEditorChange={setEditor}
            onSave={() => void saveEditor()}
          />
        ))
      ) : (
        <p>Loading components…</p>
      )}
      {outline && outline.also.length > 0 ? (
        <section className="registry-page" aria-label="Also registered">
          <div className="registry-page-head">
            <h3>Also registered</h3>
            <button
              type="button"
              aria-label="Export to Excel Also registered"
              onClick={() => void exportPage("Also registered", outline.also)}
            >
              Excel
            </button>
          </div>
          <p>Tables: {pageTables(outline.also)}</p>
          {componentLines(outline.also).map((line) => (
            <ComponentRecord
              key={line.key}
              line={line}
              tables={pageTables(outline.also)}
              meta={ownerOf(meta, line.moduleId, line.partId)}
              editor={editor}
              onEdit={() => openEditor(line)}
              onEditorChange={setEditor}
              onSave={() => void saveEditor()}
            />
          ))}
        </section>
      ) : null}
    </section>
  );
}

type EditorState = OwnerMeta & {
  key: string;
  name: string;
  moduleId: string;
  partId: string;
};

function PageSection({
  label,
  modules,
  running,
  canCapture,
  onCapture,
  onExportPage,
  meta,
  editor,
  onEdit,
  onEditorChange,
  onSave,
}: {
  label: string;
  modules: CatalogModule[];
  running: boolean | undefined;
  canCapture: boolean;
  onCapture: () => void;
  onExportPage: () => void;
  meta: MetaFile;
  editor: EditorState | null;
  onEdit: (line: ComponentLine) => void;
  onEditorChange: (next: EditorState | null) => void;
  onSave: () => void;
}) {
  const tables = pageTables(modules);
  return (
    <section className="registry-page" aria-label={label}>
      <div className="registry-page-head">
        <h3>{label}</h3>
        <button
          type="button"
          aria-label={`Export to Excel ${label}`}
          onClick={onExportPage}
        >
          Excel
        </button>
        {canCapture ? (
          <button
            type="button"
            aria-label={`Capture Page ${label}`}
            disabled={!!running}
            onClick={onCapture}
          >
            Page Capture
          </button>
        ) : null}
      </div>
      <p>Tables: {tables}</p>
      {componentLines(modules).map((line) => (
        <ComponentRecord
          key={line.key}
          line={line}
          tables={tables}
          meta={ownerOf(meta, line.moduleId, line.partId)}
          editor={editor}
          onEdit={() => onEdit(line)}
          onEditorChange={onEditorChange}
          onSave={onSave}
        />
      ))}
    </section>
  );
}

function ComponentRecord({
  line,
  tables,
  meta,
  editor,
  onEdit,
  onEditorChange,
  onSave,
}: {
  line: ComponentLine;
  tables: string;
  meta: OwnerMeta;
  editor: EditorState | null;
  onEdit: () => void;
  onEditorChange: (next: EditorState | null) => void;
  onSave: () => void;
}) {
  const sentence = meta.purpose.trim() || line.catalogPurpose.trim();
  const open = editor?.key === line.key ? editor : null;
  return (
    <article className="registry-component">
      <div className="registry-component-head">
        <strong>{line.name}</strong>
        {sentence ? <span className="registry-details">{sentence}</span> : null}
        <button type="button" className="registry-meta-link" onClick={onEdit}>
          {sentence ? "Edit description" : "Add description"}
        </button>
      </div>
      <dl className="registry-fields">
        <div><dt>Kind</dt><dd>{line.kind}</dd></div>
        <div><dt>Folder</dt><dd>{line.folder}</dd></div>
        <div><dt>Menu</dt><dd>{line.menu}</dd></div>
        <div><dt>Host</dt><dd>{line.host}</dd></div>
        <div><dt>Core functions</dt><dd>{line.cores}</dd></div>
        <div><dt>Query</dt><dd>{line.query}</dd></div>
        <div><dt>Tables</dt><dd>{tables}</dd></div>
        <div><dt>Purpose</dt><dd>{sentence}</dd></div>
        <div><dt>Inputs</dt><dd>{meta.inputs}</dd></div>
        <div><dt>Output</dt><dd>{meta.output}</dd></div>
        <div><dt>Money rule</dt><dd>{meta.moneyRule}</dd></div>
        <div><dt>Fields</dt><dd>{fieldNamesFor(line.moduleId, line.partId)}</dd></div>
      </dl>
      {open ? (
        <form
          className="registry-meta-form"
          onSubmit={(event) => {
            event.preventDefault();
            onSave();
          }}
        >
          <label>
            Purpose
            <textarea
              value={open.purpose}
              onChange={(event) => onEditorChange({ ...open, purpose: event.target.value })}
            />
          </label>
          <label>
            Inputs
            <input
              value={open.inputs}
              onChange={(event) => onEditorChange({ ...open, inputs: event.target.value })}
            />
          </label>
          <label>
            Output
            <input
              value={open.output}
              onChange={(event) => onEditorChange({ ...open, output: event.target.value })}
            />
          </label>
          <label>
            Money rule
            <select
              value={open.moneyRule}
              onChange={(event) => onEditorChange({ ...open, moneyRule: event.target.value })}
            >
              <option value=""></option>
              <option value="unknown">unknown</option>
              <option value="blank">blank</option>
              <option value="real zero">real zero</option>
            </select>
          </label>
          <button type="submit">Save</button>
          <button type="button" onClick={() => onEditorChange(null)}>
            Cancel
          </button>
        </form>
      ) : null}
    </article>
  );
}
