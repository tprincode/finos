import type { CoreFunctionsGet } from "@finos/app-contracts";
import { Fragment } from "react";
import { groupByScreen } from "./screenGroups";

export type CatalogModule = NonNullable<CoreFunctionsGet["modules"]>[number];

function ModuleRows({ rows }: { rows: CatalogModule[] }) {
  return (
    <>
      {rows.map((row) => (
        <tr key={row.id}>
          <td>{row.title}</td>
          <td>{row.status}</td>
          <td>{row.folder}</td>
          <td>{row.menuAreas.join(", ")}</td>
          <td>
            {row.coreFunctionIds?.length ? row.coreFunctionIds.join(", ") : "—"}
          </td>
          <td>{row.host || "—"}</td>
        </tr>
      ))}
    </>
  );
}

export function ComponentRegistry({ modules }: { modules?: CatalogModule[] }) {
  const grouped = modules?.length ? groupByScreen(modules) : null;
  return (
    <section className="actions" aria-label="Component registry">
      <h2>Components</h2>
      <p>
        UI extraction catalog: whether a screen has left App.tsx. Not a
        domain-completeness index and not a plugin host. Core functions stay in
        Settings. Rows use the same screen order as Screen Atlas and the
        data-snapshot workbook Template_UiModules.
      </p>
      <div className="table-wrap">
        <table aria-label="Component registry">
          <thead>
            <tr>
              <th scope="col">Module</th>
              <th scope="col">Status</th>
              <th scope="col">Folder</th>
              <th scope="col">Menu areas</th>
              <th scope="col">Core functions</th>
              <th scope="col">Host</th>
            </tr>
          </thead>
          <tbody>
            {grouped ? (
              <>
                {grouped.groups.map((group) => (
                  <Fragment key={group.screen}>
                    <tr>
                      <th scope="colgroup" colSpan={6}>
                        {group.label}
                      </th>
                    </tr>
                    <ModuleRows rows={group.direct} />
                    {group.desks.map((desk) => (
                      <Fragment key={`${group.screen}-${desk.desk}`}>
                        <tr>
                          <th scope="colgroup" colSpan={6}>
                            {desk.label}
                          </th>
                        </tr>
                        <ModuleRows rows={desk.items} />
                      </Fragment>
                    ))}
                  </Fragment>
                ))}
                {grouped.also.length > 0 ? (
                  <>
                    <tr>
                      <th scope="colgroup" colSpan={6}>
                        Also registered
                      </th>
                    </tr>
                    <ModuleRows rows={grouped.also} />
                  </>
                ) : null}
              </>
            ) : (
              <tr>
                <td colSpan={6}>Loading components…</td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </section>
  );
}
