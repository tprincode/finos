# Owner facts — global policy (all collectors)

**Binding:** Before asking the owner about **ROC / issuer / frequency / scope / amount** facts for **any** symbol, agents **MUST** read:

1. `docs/Authority/owner-facts/<SYMBOL>.md` (this directory)
2. Related Authority packs under `docs/Authority/` when cited by that file

If the file exists and the field is **locked** or **book-cited** (not `PENDING_OWNER` / OPEN) → **do not re-interview**.

Chat and Project-store copies are secondary. The **git tip** file is authoritative for agents on this repo.

---

## Layout

| Path | Role |
|------|------|
| [`INDEX.md`](INDEX.md) | All `INCOME_FLEET_SYMBOLS` (40) with FILLED / SCAFFOLD / PENDING_RECREATE |
| [`TEMPLATE.md`](TEMPLATE.md) | Copy-paste schema for new / recreate symbols |
| [`YBTC.md`](YBTC.md) | **FILLED** example (owner-locked) |
| `<SYMBOL>.md` | Per-collector durable interview ledger |

Parked long-holds (MSTU, TSLL, SOXL) are **not** collectors — no owner-facts required until first-enabled.

---

## Status values

| Status | Meaning |
|--------|---------|
| **FILLED** | Owner interview complete for locked fields; agents must not re-ask those fields |
| **SCAFFOLD** | Durable file exists; seed/book fields cited; unknowns marked `PENDING_OWNER` — **honest incomplete** |
| **PENDING_RECREATE** | File exists but not enough seed/book data to populate; fill on next recreate — **do not fake-fill** |

`PENDING_OWNER` on a field = ask **once** when needed, write the answer into the file, then lock. Never invent ROC %, never write $0 for a failed fetch, never copy another fund's % (e.g. YBIT ≠ YBTC).

---

## Owner-locked vs book system of record

| Concern | System of record | Role of owner-facts file |
|---------|------------------|---------------------------|
| Plan ROC % / `needs_roc_research` / actuals | SQLite `position_characteristic` via seed / `RocPlanConfirm` / ticket Accept | **Cite** book values; do not invent; do not auto-write DB from markdown |
| Standing `roc_source_url` | Collector template row | Prefer URL here after recreate; empty URL ≠ unknown % |
| Product identity, “not confused with”, issuer page, frequency confirm, scope narrative | **This file** (owner-locked when filled) | Interview ledger agents must read before ask |
| Live % change vs plan | `roc_pct_change` work ticket | Ticket path — still no chat→DB invent |

---

## When to create / update (recreate rule — binary)

**Pass check (recreate):** On collector adapter/template **recreate** or **first-enable**, if `docs/Authority/owner-facts/<SYMBOL>.md` is **missing** OR required fields are empty / still entirely `PENDING_OWNER` with no book cites → **create or complete the durable file** in the same change set (scaffold from known seed/book + explicit `PENDING_OWNER` for unknowns).

| Condition | Action |
|-----------|--------|
| File missing on recreate / first-enable | Create from [`TEMPLATE.md`](TEMPLATE.md); cite seed if present |
| File present, seed/book known, gaps remain | Keep SCAFFOLD; fill what is known; leave unknowns `PENDING_OWNER` |
| Not enough data to populate | SCAFFOLD or PENDING_RECREATE — **correct state**; do not fake-fill |
| Owner answers a PENDING_OWNER field | Write answer; mark locked; stop re-asking |

Fail the recreate slice if the durable file was required and was not created.

---

## Agent gate (all symbols)

1. Read `<SYMBOL>.md` before any owner question on ROC / issuer / frequency / scope / amounts.
2. Do not re-interview locked or book-cited fields.
3. Ask only `PENDING_OWNER` / OPEN fields (once), or when live parsed 19a-1 % **differs** from stored plan (raise `roc_pct_change`).
4. Empty `roc_source_url` is not “unknown ROC %.”
5. Markdown does not write SQLite.

Skill: `.cursor/skills/finos-milestone/SKILL.md`  
Rule: `.cursor/rules/execution-framework.mdc`  
Gate: `./scripts/owner-facts-anti-reask.sh`

---

## Schema (required sections)

See [`TEMPLATE.md`](TEMPLATE.md). Minimum sections: Status, Identity, Book SoR, OPEN / PENDING_OWNER, Agent rules.
