# Issuer-fact ask (same host only)

You are filling **issuer-published facts** for Finos collectors. The locked spec is already decided. Do not invent requirements. Do not persist into SQLite.

Read `collector_missing_ask_2026-09-07.json` in this folder. Return **one JSON object** with an `items` array matching `return_schema`.

## Allowed

- GET the vendor host on the seed URL (and same-host follow links only).
- Report `working_url`, HTTP status, content type, parse recipe, payable date + $/share rows you can see.
- Remaining unused 2026 pay dates **or the literal** `none`.
- 19a-1 / tax-information URL + quote of Estimated Return of Capital, **or** `none` plus `search_urls_tried`.

## Banned

- yahoo.com, finance.yahoo.com, nasdaq.com, dividendhistory.org, dividendinvestor.com, any aggregator.
- Inventing 0% ROC. Persist 0 only if the notice says 0.
- Overwriting stored pays. TSPY 2026-09-02 stays **0.30007**. SVOL stays **64** paid rows. Do not drop TSPY 2026-10-07.
- Changing cadence, Plan $/share, or risk.
- Owner clicks (HAKY DIV-1, HAKY inception, HAKY ROC accept, amount Except/Reject). List them only if you see evidence. Do not ask the operator to persist through you.

## Frequency locks (do not change)

Weekly: AMDW, QDTE, RDTE, TOPW, XDTE, YBTC. Monthly: XPAY.

## Scope notes

- ET, MPLX, TSLL, SOXL, MSTU are **not** 19a-1. Do not invent 0%.
- GLAD is BDC / 1099 — no invented 0%.
- CLM / CRF / GLAD / TSPY / SVOL leftover “expected 4” tickets are software. Do not invent December.
- Cloudflare / 403 is `blocked`, not a ROC task.

Return JSON only after the last item. If a page is empty to non-browser clients, say so and give the same-host alternative that has a table.
