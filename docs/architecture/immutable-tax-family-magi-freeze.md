# Freeze: ACA tax-family MAGI size (immutable)

**Status:** Owner-locked (immutable)  
**Decision date:** 2026-09-15  
**Binding decision:** Lock meaning #1 — ACA / Marketplace MAGI **tax-family size** — as product/rule data.  
**Cites:** [ADR-0013](../adr/0013-golden-outcome-harness.md) (independent golden oracles; never regenerate under test or AI)  
**Related gate:** [m5-gate.md](m5-gate.md)

## What is frozen

| Item | Locked value / rule |
|------|---------------------|
| Concept | HealthCare.gov / HHS tax-household size for poverty guideline × 400% MAGI threshold |
| Pack path | Coverage year **2026**, contiguous US/DC, size **2** → threshold **$84,600** (8460000 cents) |
| Encoding today | `household_size: 2` in MAGI rule/scenario YAML; `aca_threshold_rule.household_size` in SQLite |
| Nature | Product / rule data — **not** agent slang for “live Windows Profile A book” |

## Pass checks

- G-MAGI-01–10 owner-approved oracles remain bit-stable; production-path harness compares to those oracles (ADR-0013).
- Threshold derivation continues via effective-dated `aca_threshold_rule` (year + size + location), not UI hardcodes.
- M5 gate still asserts threshold **8460000** without rewriting expected oracle YAML.

## Forbidden without owner unlock

- Delete or “simplify away” the size dimension.
- Regenerate or rewrite MAGI oracle YAML numbers.
- Hardcode $84,600 while removing the rule table.
- Treat “remove household” as deleting Profile A / live SQLite (that is a different meaning of the English word).

## Allowed without unlock

- Cosmetic rename of the *word* (`household` → `tax_family` / `aca_family`) **only if** numbers and oracles stay identical (ADR-0013).
- Scrub agent jargon that used “household” for live book / installed release / privacy fence.

## Agent stop rule

If a task would change MAGI golden expected values, drop the size key from threshold lookup, or hardcode around `aca_threshold_rule` for this locked path — **stop** and require owner unlock. Do not regenerate MAGI oracles.
