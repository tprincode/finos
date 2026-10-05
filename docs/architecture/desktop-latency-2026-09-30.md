# Desktop latency note (2026-09-30)

Profile A, local SQLite. Coding launch (Vite + Tauri rebuild) is **not** household latency.

## Measured / observed sources (top 3)

| Rank | Source | What the owner feels | Evidence |
|------|--------|----------------------|----------|
| 1 | Auto last-price + declaration fleet on open | Busy bar / “still working” after Home paints | Weekday 9–4 ET: `LastPriceRefresh` at **3s/symbol** then once-per-day `DeclarationRefresh`. Network-bound, serial. Open-path already defers Income Plan / Tax / Register off Home. |
| 2 | Host JS size (`App.tsx`) | Slow Vite HMR / first paint in coding | **Before** PD extract: **489.4 KiB** / 13,004 lines. **After:** **387.8 KiB** / 10,693 lines (−101.6 KiB). Markup in `features/position-details/`. Add Lot / Holdings still inline; target host &lt;200 KiB. |
| 3 | Heavy compose queries on first visit | Loading… on Register / Coverage; Week Ahead less cold after idle | Idle warm: grid / elements / Tax, then **WeekAheadGet** on Home/Income Plan idle (early horizon accepted). Register / Coverage / YTD stay click-cold. Pool max 4 WAL readers. |

## Not the primary cause

- Raw SQLite I/O after open-path P0–P2 (`HomeOpenGet`, `0041` indexes, 64 MiB cache, 256 MiB mmap, `busy_timeout` 15s).
- Postgres would **not** fix single-desktop feel; cutover stays gated (see mobile-sync priority).

## App.tsx size (this turn)

| | KiB | Lines |
|--|-----|-------|
| Before (baseline before suite) | 489.4 | 13,004 |
| After Position Details extract + Week Ahead warm | 387.8 | 10,693 |

## Latency bets (status)

1. Skip or defer auto fleet while the owner is reading — **out** (behavior change).
2. App.tsx extracts — **Position Details done**; Add Lot / Holdings folders exist, App markup still Next.
3. Week Ahead idle warm — **done** (early horizon on Home idle accepted). Register still click-cold.

Pass for this note: top-3 named with mechanism; Postgres not recommended for speed alone.
