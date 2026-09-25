# HAKY Position Details audit (2026-08-29, revised)

Live household SQLite (`%LOCALAPPDATA%\com.finos.desktop\local.sqlite`).

Security: `f216115d-5716-4b30-8345-3950220e5cec` · symbol **HAKY**

## Economics (owner lock)

| Metric | Formula |
|--------|---------|
| **Lot original / tax cost (stored)** | `qty × recorded unit price` → USD **lot total** cents |
| **Original cost (PD / Calculator / YOC denom)** | sum of open lots’ remaining performance (**lot totals only**) |
| **Market value** | `qty × current last price` |
| **Unit vs last** | recorded unit and last price should be near each other (here ~\$1/share) |

Never sum unit prices. Never treat a unit price as a lot total.

## Open lots (after unit→total repair)

| lot_id | account | opened_on | qty | recorded unit | lot orig / tax |
|--------|---------|-----------|-----|---------------|----------------|
| `4c8a3810-…` | Car | 2026-08-21 | 35 | \$29.46 | **\$1,031.10** (103110¢) |
| `5366d579-…` | Car | 2026-08-25 | 35 | \$29.52 | **\$1,033.20** (103320¢) |

Aggregate: qty **70** · original cost **\$2,064.30** · last **\$30.72** · MV **\$2,150.40** · ~**\$1.23**/share vs last.

### What was wrong before

Add Lot previously labeled the field as a total but the owner entered **unit** prices (\$29.46 / \$29.52). Those were stored as lot totals → \$58.98 cost, ~\$0.84/sh implied, YOC **541%**. That was bad **data**, not a wrong MV formula (MV was already qty × last).

Repair: set each lot’s performance/tax/remaining basis to `qty × former_stored_cents` (unit × qty). Add Lot UI now: enter unit \$; confirm `35 × \$29.46 = \$1,031.10`; save the **lot total**.

## Plan YOC (post-repair)

| Input | Value |
|-------|-------|
| Plan / sh | \$0.38 (3800 @ scale 4) |
| Plan payment | 70 × \$0.38 = **\$26.60** |
| Plan annual | \$26.60 × 12 = **\$319.20** |
| YOC | 31920×10000 / 206430 ≈ **1546 bps → 15.46%** |

Wiring unchanged: YOC = plan annual / **sum of lot totals**.

## Research gaps (unchanged)

Provider / underlying / risk blank; no ROC observations; 7 Amplify declarations; Monthly frequency; template present.

## UI columns (Add Lot / PD lots)

Qty · Unit orig \$ · Unit tax \$ · Lot orig \$ · Lot tax \$. Footer totals **Lot orig / Lot tax only**.
