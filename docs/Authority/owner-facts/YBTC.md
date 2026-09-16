# YBTC — owner facts (binding)

**Status:** `FILLED` (global owner-facts example)  
Owner-provided. **Do not re-ask ROC.** **Do not re-research as if unknown.**

Locked: 2026-09-16  
Source: owner statements + Project store `docs/Authority/ybtc-owner-facts-2026-09-16.md`  
Policy: [`README.md`](README.md) (all collectors)

## Identity

- **Symbol:** YBTC
- **Product:** Roundhill Bitcoin Covered Call Strategy ETF (CUSIP 77926X502)
- **Not:** YieldMax YBIT (different fund — do not copy YBIT ROC %)
- **Issuer page:** https://roundhillinvestments.com/etf/ybtc
- **Frequency:** Weekly
- **Provider:** Roundhill
- **ROC scope:** InScope (covered-call 1940 Act ETF → 19a-1)

## ROC (book)

| Field | Value | Notes |
|-------|-------|-------|
| In-year estimate | **100%** | Product page / recent 19a-1 notices (in-year estimate, not 1099) |
| `needs_roc_research` | **false / NO** | Estimate accepted for book; seed matches |
| `roc_pct_2026_actual` | **empty** | Parked until 2027 1099 |
| Seed alignment | `Template_Positions` Data!YBTC estimate 100, `needs_roc_research=NO` | Do not invent a different % from chat |

## Standing 19a-1 / reuse URL (prefer)

- Payable 18 Jun 2026: 100% ROC / 0% NII / $0.129383 — `19a-notice-ybtc-yeth-xdte-rdte-qdte-6.17.26.pdf`
- Payable 12 Feb 2026: 100% ROC / 0% NII / $0.206706 — `19a-notice-ybtc-yeth-xpay-2.11.26.pdf`
- Pattern: `https://www.roundhillinvestments.com/assets/data/rh_filings/19a-notice-ybtc-…`
- Older: `https://www.roundhillinvestments.com/assets/pdfs/19a-1_notice_ybtc.pdf` (Jan 2024 was 96%)
- If product-page 19a-1 link 403s, use `rh_filings` PDF URL
- **Do not write $0** when a link fails
- Empty standing `roc_source_url` on seed/live is an **optional URL persist** only — **not** a reason to re-interview %

## Agent rules

1. Before asking the owner about YBTC ROC / collector establish facts, read this file.
2. If this file is present → **do not re-interview** identity, scope, estimate %, or 1099 parking.
3. Live collect may raise `roc_pct_change` only when a **parsed** 19a-1 % differs from stored plan; Accept/Reject touch plan % — chat alone does not.
4. Aggregator calendars are not a 19a-1 substitute.

## OPEN (owner still required)

- None for ROC % / identity / scope as of 2026-09-16.
- Optional Windows: persist standing `roc_source_url` without changing plan %.
