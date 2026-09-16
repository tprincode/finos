# TEMPLATE — owner-facts/\<SYMBOL\>.md

Copy to `docs/Authority/owner-facts/<SYMBOL>.md` on recreate or first-enable when the file is missing or incomplete. Replace placeholders. Do **not** invent ROC %. Do **not** write $0. Do **not** copy another fund's percentages.

```markdown
# <SYMBOL> — owner facts

**Status:** `SCAFFOLD` | `FILLED` | `PENDING_RECREATE`  
**Policy:** Global owner-facts — read before asking the owner. Locked / book-cited → do not re-interview.

Scaffolded / locked: YYYY-MM-DD  
Source: seed Template_Positions | live book | owner | recreate

## Identity

| Field | Value | Lock |
|-------|-------|------|
| Symbol | <SYMBOL> | locked |
| Product / legal name | … or PENDING_OWNER | |
| Not confused with | … or PENDING_OWNER | forbid wrong-fund copy |
| Issuer / product page | … or PENDING_OWNER | |
| Provider | … or PENDING_OWNER | |
| Frequency | … or PENDING_OWNER | |
| ROC scope | InScope / CASH / MLP / BDC / … or PENDING_OWNER | |

## Book SoR (cite only; markdown does not write DB)

| Field | Value | Notes |
|-------|-------|-------|
| In-year ROC estimate | …% or PENDING_OWNER | seed/`position_characteristic` |
| `needs_roc_research` | YES/NO or PENDING_OWNER | |
| `roc_pct_2026_actual` | empty until 1099 year | parked |
| Standing `roc_source_url` | URL or PENDING_OWNER | empty ≠ invent 0% |

## OPEN / PENDING_OWNER

- List fields still needing owner (or recreate) once.
- After answer: write here and lock — do not re-ask.

## Agent rules

1. Before asking about ROC / issuer / frequency / scope / amounts for <SYMBOL>, read this file.
2. Do not re-interview locked or book-cited fields.
3. Ticket `roc_pct_change` only when live parsed % differs from plan; chat alone does not write SQLite.
```
