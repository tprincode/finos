# Collector gap after last_run repair

**As-of:** 2026-09-07 (re-audit after the four-defect last_run slice)  
**System of record:** `%LOCALAPPDATA%\com.finos.desktop\local.sqlite`  
**Enabled:** 43. Ever-ok: **43**. Complete: **41**. Live last_run: still **32 ok / 11 miss**.  
**Collectors are not Done.** Fixture green is not the gate.

Live `last_run_ok` has not moved because desktop has not Run since the code landed. The repair is in source. SQLite still holds the 6-Sep retrieve stamps.

Prior report: `docs/Authority/Collector_Software_and_Data_Gap_2026-09-07.md`.  
Code zip: `docs/Authority/collector_program_code_2026-09-07-after-last_run.zip`.

## What the four defects corrected (code)

| Defect | Code change | Live names it fixes | Live last_run today | After next Run / ticket sync |
|---|---|---|---|---|
| Lookback used this-page count | `apply_fetched_page` uses stored `paid_count` | GLAD 295 stored but “6 of 12”; leftover lookback on IGLD 66, QDVO 24, SVOL 64 | GLAD miss; others already ok | GLAD last_run **ok** if this page still prints any paid/upcoming row. Leftover lookback tickets do not re-open. |
| C5 short increment failed last_run / implied wipe | Overlap dates only; `declaration_amount_variation` ticket; keep stored rows | TSPY 48 stored (0.30007 + 2026-10-07 kept); SVOL 64 stored; HAKY scale ticket | TSPY miss; SVOL already ok | TSPY last_run **ok**; amount ticket stays for Except/Reject; **do not overwrite 0.30007**. HAKY C5 no longer fails last_run. |
| False empty on increment; no two-URL cap / js_empty | Empty only if this page has no paid and no upcoming; two same-host tries; 403 / `js_empty` loud | Any increment that re-parses already-stored pays | Unchanged | Increment of stored pays is not “Issuer page empty.” True empty / 403 / JS shell after two tries still fail. |
| Stale “expected 4” remaining-year tickets | Close when lists do not disagree; `completed_how=auto_resolved` | CLM, CRF, GLAD, SVOL, TSPY | last_run already ok except GLAD/TSPY | Tickets close. **December is not inserted.** `paid_payable_supersede` stays open. |

Goldens: `pay1_short_increment_lookback_complete_last_run_ok`, `pay1_overlap_amount_change_tickets_keeps_stored`, `pay1_stale_expected_4_remaining_year_closes_without_december`. C5 tests retargeted to PAY1.

If the next increment page still prints rows (GLAD last parse returned 6; TSPY last parse returned overlap + amount change), live last_run becomes **34 ok / 9 miss**. That is not 0 miss.

## What remains a gap — and why

### 1. True empty / 403 / JS shell (adapter still owes a working same-host parse)

These names already have stored pays. Last message is still `Issuer page empty.` The increment≠empty fix does **not** flip last_run unless this GET returns a paid or upcoming row. Two same-host tries then loud miss is working as specified. No proxy. No Yahoo / nasdaq / dividendinvestor fill.

| Symbol | Stored pays | Why last_run stays fail |
|---|---:|---|
| EFC | 188 | Seed `ellingtonfinancial.com/dividends` returns no useful table to the HTTP client. |
| ET | 168 | Seed IR distribution-history page empty to the client. |
| JEPQ | 65 | `am.jpmorgan.com` product slug empty / blocked to the client. |
| MPLX | 57 | IR page Cloudflare 403 or empty after two same-host tries. |
| QYLD | 154 | `globalxetfs.com/funds/qyld/` JS / empty table. |
| SOXL | 29 | Direxion product page empty / 403. |
| TSLL | 22 | Direxion product page empty / 403. |

**Why it remains:** this is a fetch/parse hole on the standing host, not a lookback or C5 bug. Stored history must not be wiped while the adapter hunts a same-host recipe.

### 2. MSTU — Step A never established

0 stored pays. Frequency `None`. Last `Issuer page empty.` `roc_scope` NotInScope (no invented 0% ROC).  
**Why it remains:** there is no paid history to increment. The collector still needs a working Trex/REX distribution parse **or** a two-URL loud fail plus an owner frequency lock. Completeness stays N.

### 3. HAKY — lookback still short; owner clicks still required

7 stored pays, empty `inception_on`, empty `div_type`, `needs_roc_research=1`. Last message “7 of 12.”  
C5 scale/amount no longer fails last_run (ticket only). Lookback math was **not** changed.  
**Why last_run stays fail:** lock §3.2 — under 12 paid requires owner inception Yes/No. Software must not invent inception or persist DIV-1 / ROC accept from a pack.

### 4. Owner Except/Reject (not auto-closed)

| Symbol | Ticket | Why it remains |
|---|---|---|
| TSPY | amount_confirm 2026-09-02 **0.30007** vs page 0.2954 | Lock: do not overwrite. Owner Except or Reject. |
| HAKY | amount_confirm + leftover scale ticket | Owner Except/Reject. |
| IGLD / QDVO | amount_confirm | Owner Except/Reject. Leftover lookback is stale (stored ≥12). |
| CLM / CRF / GLAD | `paid_payable_supersede` | Vendor payable vs already-paid month. Owner only. |

### 5. Step C reuse template (not in this slice)

28 InScope names have `roc_pct` and `needs_roc_research=0` but empty `roc_source_url`. `roc-19a1` ledger is unused.  
**Why it remains:** the last_run slice did not persist ROC URLs. Future 19a-1 updates have no standing template.

### 6. Leftover issuer-research tickets (last_run already ok)

YieldMax AMDY / AMZY / CONY / MSTY / NFLY: last_run ok, remaining 0, open `declaration_retrieve_miss` (“unparseable increment”). YMAX remaining 0, no ticket. CEFS remaining 0. IGLD rem Sep 30 + Nov 30 (October or `none`).  
**Why it remains:** D2 does not invent dates. Need unused 2026 Fridays/months from the issuer host **or** `none`.

## Live last_run miss list (11, unchanged until Run)

EFC, ET, GLAD, HAKY, JEPQ, MPLX, MSTU, QYLD, SOXL, TSLL, TSPY.

After Run, if increment hosts behave as their last successful parse: drop **GLAD** and **TSPY** from that list. Keep the other nine.

## Per-collector live grid (re-audit)

Complete ≠ last_run_ok ≠ ever-ok.

| Symbol | Ever | Last | Complete | Decl tmpl | ROC tmpl | Scope | Who | Paid | Rem | Owner missing | Open tickets | Adapter |
|---|---|---|---|---|---|---|---|---:|---:|---|---|---|
| AMDW | Y | Y | Y | Y | N | InScope | none | 58 | 18 | — | — | roundhill |
| AMDY | Y | Y | Y | Y | N | InScope | issuer_research | 71 | 0 | — | declaration_retrieve_miss | yieldmax |
| AMZY | Y | Y | Y | Y | N | InScope | issuer_research | 73 | 0 | — | declaration_retrieve_miss | yieldmax |
| BITO | Y | Y | Y | Y | N | InScope | none | 32 | 1 | — | — | proshares |
| BTCI | Y | Y | Y | Y | N | InScope | none | 23 | 4 | — | — | neos |
| CEFS | Y | Y | Y | Y | N | InScope | none | 113 | 0 | — | — | saba |
| CLM | Y | Y | Y | Y | Y | InScope | owner | 311 | 3 | amount_confirm | paid_payable_supersede, remaining_year | cornerstone |
| CONY | Y | Y | Y | Y | N | InScope | issuer_research | 72 | 0 | — | declaration_retrieve_miss | yieldmax |
| CRF | Y | Y | Y | Y | Y | InScope | owner | 361 | 3 | amount_confirm | paid_payable_supersede, remaining_year | cornerstone |
| EFC | Y | N | Y | Y | N | InScope | adapter_software | 188 | 2 | — | declaration_retrieve_miss | ellington |
| EPD | Y | Y | Y | Y | Y | NotInScope | none | 356 | 1 | — | — | enterprise |
| ET | Y | N | Y | Y | Y | NotInScope | adapter_software | 168 | 1 | — | declaration_retrieve_miss | energytransfer |
| FDRXX | Y | Y | Y | Y | Y | Cash | none | 0 | 4 | — | — | fidelity |
| GLAD | Y | N | Y | Y | Y | Bdc1099 | owner | 295 | 3 | amount_confirm | declaration_lookback_short, declaration_retrieve_miss, paid_payable_supersede, remaining_year | gladstone |
| HAKY | Y | N | N | Y | N | InScope | owner | 7 | 4 | div_type, inception_yes_no, roc_accept, amount_confirm | declaration_amount_variation, declaration_lookback_short, declaration_retrieve_miss, declaration_stored_mismatch | amplify |
| IGLD | Y | Y | Y | Y | N | InScope | owner | 66 | 2 | amount_confirm | declaration_amount_variation, declaration_lookback_short | ftvest |
| JEPQ | Y | N | Y | Y | Y | InScope | adapter_software | 65 | 1 | — | declaration_retrieve_miss | jpmorgan |
| MPLX | Y | N | Y | Y | Y | NotInScope | adapter_software | 57 | 1 | — | declaration_retrieve_miss | mplx |
| MSTU | Y | N | N | Y | Y | NotInScope | adapter_software | 0 | — | inception_yes_no | declaration_retrieve_miss | trex |
| MSTY | Y | Y | Y | Y | N | InScope | issuer_research | 66 | 0 | — | declaration_retrieve_miss | yieldmax |
| NFLY | Y | Y | Y | Y | N | InScope | issuer_research | 73 | 0 | — | declaration_retrieve_miss | yieldmax |
| NVDW | Y | Y | Y | Y | N | InScope | none | 81 | 18 | — | — | roundhill |
| ORC | Y | Y | Y | Y | N | InScope | none | 337 | 1 | — | — | orchidisland |
| PLTW | Y | Y | Y | Y | N | InScope | none | 82 | 18 | — | — | roundhill |
| QDTE | Y | Y | Y | Y | N | InScope | none | 128 | 16 | — | — | roundhill |
| QDVO | Y | Y | Y | Y | N | InScope | owner | 24 | 4 | amount_confirm | declaration_amount_variation, declaration_lookback_short | amplify |
| QQQI | Y | Y | Y | Y | N | InScope | none | 31 | 4 | — | — | neos |
| QYLD | Y | N | Y | Y | Y | InScope | adapter_software | 154 | 2 | — | declaration_retrieve_miss | globalx |
| RDTE | Y | Y | Y | Y | N | InScope | none | 102 | 16 | — | — | roundhill |
| SOXL | Y | N | Y | Y | Y | NotInScope | adapter_software | 29 | 2 | — | declaration_retrieve_miss | direxion |
| SPAXX | Y | Y | Y | Y | Y | Cash | none | 1 | 4 | — | — | fidelity |
| SPYI | Y | Y | Y | Y | N | InScope | none | 48 | 4 | — | — | neos |
| SVOL | Y | Y | Y | Y | N | InScope | adapter_software | 64 | 1 | — | declaration_history_dropped, declaration_lookback_short, remaining_year | simplify |
| SWVXX | Y | Y | Y | Y | Y | Cash | none | 1 | 4 | — | — | schwab |
| TOPW | Y | Y | Y | Y | N | InScope | none | 51 | 17 | — | — | roundhill |
| TRIN | Y | Y | Y | Y | N | InScope | none | 50 | 2 | — | — | trinity |
| TSLL | Y | N | Y | Y | Y | NotInScope | adapter_software | 22 | 2 | — | declaration_retrieve_miss | direxion |
| TSLW | Y | Y | Y | Y | N | InScope | none | 82 | 18 | — | — | roundhill |
| TSPY | Y | N | Y | Y | Y | InScope | owner | 48 | 3 | amount_confirm | declaration_amount_variation, declaration_history_dropped, declaration_retrieve_miss, declaration_stored_mismatch, remaining_year | tappalpha |
| XDTE | Y | Y | Y | Y | N | InScope | none | 128 | 16 | — | — | roundhill |
| XPAY | Y | Y | Y | Y | N | InScope | none | 23 | 4 | — | — | roundhill |
| YBTC | Y | Y | Y | Y | N | InScope | none | 100 | 17 | — | — | roundhill |
| YMAX | Y | Y | Y | Y | N | InScope | none | 110 | 0 | — | — | yieldmax |

## Hard bans (still in force)

- Do not invent 0% ROC.
- Do not overwrite stored paid rows (TSPY 2026-09-02 **0.30007**, SVOL **64** pays).
- Do not persist HAKY DIV-1, HAKY inception, or HAKY ROC accept from a pack.
- Do not treat last_run 0-miss as collectors Done until `npm run desktop` → Run enabled shows 0 miss and Income Plan has amounts for every enabled payer.
