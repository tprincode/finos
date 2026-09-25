import type { CollectorSetItem } from "./types";

export type CollectorEstablishScreenProps = {
  busy: boolean;
  writesBlocked: boolean;
  collectorItems: CollectorSetItem[];
  onEstablish: (row: CollectorSetItem) => void;
  onReevaluate: (row: CollectorSetItem) => void;
  onAcceptRoc: (row: CollectorSetItem) => void;
};

export function CollectorEstablishScreen({
  busy,
  writesBlocked,
  collectorItems,
  onEstablish,
  onReevaluate,
  onAcceptRoc,
}: CollectorEstablishScreenProps) {
  return (
    <section aria-label="Reevaluate collector">
              <h2>Reevaluate collector</h2>
              <p>
                Collect fresh on Collectors is the daily adapter run. Establish /
                Reevaluate here is identity and 19a-1 — it does not rewrite stored
                pays. New unpaid dates may be added. Amount changes ticket Except
                or Reject. Empty / 403 / JS last_run is parked, not a wipe.
              </p>
              <div className="table-wrap">
                <table aria-label="Establish collector fleet">
                  <thead>
                    <tr>
                      <th scope="col">Symbol</th>
                      <th scope="col">Complete</th>
                      <th scope="col">Gaps</th>
                      <th scope="col">ROC</th>
                      <th scope="col">Actions</th>
                    </tr>
                  </thead>
                  <tbody>
                    {collectorItems.length === 0 ? (
                      <tr>
                        <td colSpan={5}>No paying symbols loaded yet.</td>
                      </tr>
                    ) : (
                      collectorItems.map((row) => {
                        const roc =
                          row.rocEstimateMinor == null
                            ? "none"
                            : `${(row.rocEstimateMinor / 10 ** (row.rocScale || 2)).toFixed(row.rocScale || 2)}%`;
                        return (
                          <tr key={row.securityId}>
                            <td>{row.symbol}</td>
                            <td>{row.complete ? "yes" : "no"}</td>
                            <td>{(row.gaps ?? []).join(", ") || "—"}</td>
                            <td>{roc}</td>
                            <td>
                              <button
                                type="button"
                                aria-label={`Establish collector ${row.symbol}`}
                                disabled={busy || writesBlocked}
                                onClick={() => onEstablish(row)}
                              >
                                Establish
                              </button>{" "}
                              <button
                                type="button"
                                aria-label={`Reevaluate collector ${row.symbol}`}
                                disabled={busy || writesBlocked}
                                onClick={() => onReevaluate(row)}
                              >
                                Reevaluate
                              </button>{" "}
                              <button
                                type="button"
                                aria-label={`Accept recommended ROC ${row.symbol}`}
                                disabled={
                                  busy ||
                                  writesBlocked ||
                                  row.rocEstimateMinor == null ||
                                  row.complete === true
                                }
                                onClick={() => onAcceptRoc(row)}
                              >
                                Accept ROC
                              </button>
                            </td>
                          </tr>
                        );
                      })
                    )}
                  </tbody>
                </table>
              </div>
            </section>
  );
}
