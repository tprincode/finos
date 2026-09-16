# MPLX — owner facts (scaffold)

**Status:** `SCAFFOLD`  
**Policy:** Global owner-facts — read this before asking the owner. Locked rows = do not re-interview. `PENDING_OWNER` = ask once on recreate, then lock. Do not invent ROC %, do not write $0, do not copy a different fund's %.

Scaffolded: 2026-09-16 from seed `Template_Positions` Data!MPLX  
Filled example pattern: [`YBTC.md`](YBTC.md)

## Identity

| Field | Value | Lock |
|-------|-------|------|
| Symbol | MPLX | book-cited |
| Underlying / sleeve (seed) | MLP midstream | book-cited |
| Product / legal name | PENDING_OWNER | open — confirm on recreate |
| Not confused with | PENDING_OWNER | open — forbid wrong-fund copy |
| Issuer / product page | PENDING_OWNER | open |
| Provider (seed) | MPLX LP | book-cited |
| Frequency (seed) | Quarterly | book-cited — confirm if live differs |
| Risk tier (seed) | Core | book-cited (Position Details) |
| Div type (seed) | DIV-1 | book-cited |
| ROC scope | MLP / K-1 (model ROC 0%; character separate) | confirm on recreate if not CASH/MLP |

## Book SoR (cite seed / SQLite only; markdown does not write DB)

| Field | Value | Notes |
|-------|-------|-------|
| In-year ROC estimate (seed) | 0% | Book SoR via seed/`position_characteristic`; change only via Accept / `roc_pct_change` |
| `needs_roc_research` (seed) | NO | |
| `roc_pct_2026_actual` | empty until 1099 year | Parked by design |
| Standing `roc_source_url` | PENDING_OWNER | Empty template ≠ unknown %; persist URL on recreate without inventing % |
| Seed notes (abbrev) | MLP — ROC set to 0%; K-1 character separate if needed | MLP — ROC 0% for this model; K-1 character separate | 2026 EST 0.0% — MLP — ROC 0%; K-1 character separate if needed | Context only |

## OPEN / PENDING_OWNER

- Product legal name, issuer URL, standing 19a-1/tax reuse URL, wrong-fund exclusions.
- Confirm frequency/scope if live adapter disagrees with seed.
- **Recreate rule:** if this file is missing or required fields still PENDING_OWNER when adapter/template is recreated or first-enabled → complete the durable file (scaffold unknowns as PENDING_OWNER; never fake-fill %).

## Agent rules

1. Before asking about ROC / issuer / frequency / scope / amounts for MPLX, read this file (+ related Authority packs).
2. Do **not** re-interview book-cited estimate/`needs_roc_research` when seed already has values — ticket only if live parsed % differs.
3. Ask only PENDING_OWNER / OPEN fields, once; then update this file to lock them.
4. Markdown does not write SQLite.
