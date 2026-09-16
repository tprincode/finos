# Product authority (read-only)

Copy this folder into the finos repo as `docs/authority/`.

## Order of authority

1. System Architecture V1.1 (this folder)
2. Cross-Cutting BR-X (this folder)
3. Domain feature specs (`domains/`)
4. Production seed gates (`database/seed/production/` — separate pack)
5. Code and tests implement the above; they do not override it

If code conflicts with a locked domain doc or BR-X, the document wins until an ADR is approved.

## Owner facts (ROC / collector interview)

Standing per-symbol owner answers live in [`owner-facts/`](owner-facts/README.md) (`owner-facts/<SYMBOL>.md`).

**Before asking the owner about ROC or collector establish facts for a symbol, read that file if present. If present and the field is locked, do not re-interview.** Chat and Project-store copies are not a substitute for the tip file. Markdown does not auto-write live SQLite plan %.

## Domains

| File | Feature area |
|------|----------------|
| Holdings_…Working_Foundation… | Lots, holdings processes, dual-cost foundation |
| Calculator_… | Plan, declarations, symbol-level planning |
| Position_Details_… | Security master, Risk On, characteristics |
| Last_Price_… | Price service |
| Income_Plan_… | Week plan vs actual, burndown inputs |
| ROI_…Ledger… | Activity ledger, dividends, sales path |
| Trends_… | Weekly metrics, non-ROI rollups |
| Dashboard_… | Presentation, burndown views |
| Shopping_Cart_… | Scenarios / what-if (later milestone) |

Do not edit these files in place to match code. Replace only with a new versioned authority drop.
