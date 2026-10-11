# TVAL inventory

Calculator is the only product home for the TVAL family. Holdings shows **P&L%** and **Annual%** on original cost and does **not** show TVAL.

Authority naming (do not invent new labels in code without owner approval):

| Name | Meaning | In product? |
|------|---------|-------------|
| Holdings legacy | `P&L% + Annual%` (spreadsheet history) | **No** column named TVAL on Holdings |
| Calculator TVAL V3 | `(Gain% × 2 + FWD) / 2` | **Yes** — UI header **Calculator blend** (sort key `tval`) |
| TVAL 2.1 | Open Authority item | **No** formula shipped |

Owner shorthand “V1 = P&L%+Annual%, V2 = more weighted with P/L” maps to Holdings legacy vs Calculator TVAL V3. There is no column labeled TVAL v1 or v2.

## Surfaces and formulas (bps, integer truncating divide as shipped)

| Surface | Label in UI | Formula | Status |
|---------|-------------|---------|--------|
| Calculator | Calculator blend (`tval`) | `(unrealizedPnlBps × 2 + planFwdYieldBps) / 2` | Shipped — TVAL V3 |
| Calculator | MC TVAL | `(unrealizedPnlBps × 2 + mostCurrentFwdYieldBps) / 2` | Shipped |
| Calculator | TVAL Δ | `MC TVAL − TVAL` | Shipped |
| Calculator Excel export | same three headers | Same math in `application_core::component_export` | Shipped |
| Field Intent | documents the three | [`calculatorColumns.ts`](../../apps/desktop/src/features/field-intent/calculatorColumns.ts) | Contract text |
| Holdings | — | **No TVAL** | By design |
| Holdings | P&L% | `(MV − original cost) / original cost` | Not TVAL |
| Holdings | Annual% | Plan annual $ / original cost (YOC) | Not TVAL; not FWD (FWD is on price) |
| Position Details | — | Does not render TVAL cells | No TVAL column |
| TVAL 2.1 | — | Undefined | Do not invent |
| Holdings spreadsheet legacy | `legacy_tval = P&L% + Annual%` | Authority history only | Not in product UI |

## Source of truth

- UI: `dividendScore` in [`packages/ui-components/src/index.tsx`](../../packages/ui-components/src/index.tsx)
- Export: `calculator_tval_v3_bps` / `calculator_typed_row` in [`crates/application-core/src/component_export.rs`](../../crates/application-core/src/component_export.rs)
- Goldens: `calculator_plan` (blend needles + exact bps), `lots_roi` (Holdings P&L%/Annual%, no TVAL header)

## Out of scope

- Do **not** add a Holdings TVAL column.
- Do **not** replace Calculator blend with `P&L% + Annual%`.
- Do **not** invent or ship TVAL 2.1 without a new Authority approval.
- Do **not** change V3 weights without owner approval.
