# Collector software and data gap

**Superseded for last_run repair status:** see [Collector_Software_and_Data_Gap_2026-09-07-after-last_run.md](Collector_Software_and_Data_Gap_2026-09-07-after-last_run.md). This file is the pre-repair snapshot.

**As-of:** 2026-09-07  
**System of record:** `%LOCALAPPDATA%\com.finos.desktop\local.sqlite`  
**Enabled collectors:** 43  
**Author of this extract:** Cursor audit against locked spec only. Fixture green is not the gate.

This report answers, for every enabled collector:

1. Has the collector ever succeeded?
2. Has the system declared it complete?
3. Are reuse templates identified for future lookups?
4. What owner-entered data is missing?
5. What remains (software / issuer facts / owner click)?

It also scores collector **software** against the locked spec. Data is the evidence. The spec is not invented.

## Authority used (do not invent)

Only these locked extracts:

- `.cursor/req_extract/Collector_Requirements_Initial_and_Runtime_Locked_2026-09-04.txt`
- `.cursor/req_extract/Adapter_Collector_Owner_Decisions_Locked_2026-09-02.txt`
- F1 / F2 wording in `.cursor/req_extract/Collector_Implementation_Plan_Audit_2026-09-04.txt`

Word copies live under `docs/Authority/`. If a sentence is not in those files, it is not a requirement in this report.

**Parked in those locks (not a build-now miss):** manual adapter (P1), 1099 actual process (M7), CASH renamed DIV-2 (§3.5 nickname), Grok auto-tier.

**Book-wide ops gate (lock + `docs/architecture/execution.md`):** `npm run desktop` → Run enabled shows **0 miss**, and Income Plan has a declaration amount for every enabled payer. That is not the same as per-name Phase I `collector_is_complete`.

## Headline answers

| Question | Live evidence 2026-09-07 |
|---|---|
| Has any collector ever succeeded? | **Yes — all 43.** `retrieve_run` has `kind=declaration` and `ok=1` for every enabled name. First ok is 2026-08-26 for most; HAKY first ok 2026-08-29. |
| Has the system declared collectors complete? | **41 of 43.** Incomplete: **HAKY** (`div_type`, `roc_estimate`, `paid_history`) and **MSTU** (`frequency`, `paid_history`, `remaining_year`). |
| Are declaration reuse templates identified? | **43 of 43** have registered `declaration_source` + `source_url` (CASH may have empty URL) + `last_content_hash`. |
| Are ROC reuse templates identified? | **15 of 43** ready (nonempty `roc_source_url` **or** `roc_scope` skips 19a-1). **28 InScope names have empty `roc_source_url`.** `retrieve_run` `kind=roc-19a1` is almost unused (1 ok / 5 fail). |
| Last declaration run ok? | **32 ok / 11 miss.** Miss: EFC, ET, GLAD, HAKY, JEPQ, MPLX, MSTU, QYLD, SOXL, TSLL, TSPY. |
| Ops gate 0 miss? | **No.** Collectors are not Done. |

`complete` ≠ `last_run_ok` ≠ ever-ok. That split is required by F1 and X8.

```
retrieve_run kinds in live SQLite:
  declaration  ok=2528  fail=1126
  price        ok=5702  fail=322
  roc-19a1     ok=1     fail=5
```

## Software vs locked spec

Score: **present** (wired and live evidence does not contradict) / **partial** (wired but live rows show the rule misfires or is incomplete) / **missing** (no code path) / **parked** (lock says do not build now).

Cite is the lock sentence. Code path is the implementation. Evidence is live SQLite.

### Phase I — establish (A then B then C)

| Lock | Cite | Code path | Score | Evidence |
|---|---|---|---|---|
| Two URL tries then loud fail. Adapter design, not daily collect. | C1, §3.1 | `history_url_attempts` in `crates/application-core/src/queries.rs` (~2389–2415) | **Present** | Column exists. All 43 live rows have `history_url_attempts=0` because they already have a seed URL. Daily collect does not increment (correct per C1). |
| Fetch issuer page. Parse paid rows. Payable date is the stored event date. Store every vendor row, not only 12. | §3.2, M9, C3 | `crates/import-engine/src/retrieve/` + `issuer_declaration` | **Partial** | History is stored (HAKY 7 and MSTU 0 excepted). Last run “Issuer page empty” on EFC (188 pays), ET (168), JEPQ (65), MPLX (57), QYLD (154), SOXL (29), TSLL (22), GLAD (295), HAKY (7), MSTU (0) is a **runtime adapter hole**, not “no history.” |
| After post, re-parse the same fetched page and compare. SQLite-only reread is not enough. | C5, P3, M18 | `declaration_store_verify_issues` in `queries.rs` (~1720) | **Partial** | Wired. Misfires on increment / short product tables: TSPY last_run fail for stored 2026-10-07 missing from this page and 0.30007 vs issuer 0.2954; SVOL `declaration_history_dropped` vs 64 stored pays. |
| Corrections do not silently overwrite. | §3.2 | tickets `declaration_amount_variation`, `paid_payable_supersede`, `declaration_stored_mismatch` | **Present** | TSPY 2026-09-02 **0.30007** still stored. SVOL **64** pays still stored. Owner Except/Reject required. |
| Fewer than 12 paid = inception search + owner Yes/No. Must be established, not deferred. | §3.2, X3, M9 | `validate_paid_lookback` in `declaration_lookback.rs`; tickets `declaration_lookback_short` | **Partial** | HAKY 7 pays, empty `inception_on`, ticket open — owner Yes/No still required. GLAD **295** stored pays but last parse returned 6 and opened lookback — software counts this-page rows, not stored total. Same leftover lookback on IGLD (66 stored), QDVO (24), SVOL (64). |
| Vendor upcoming dates first; else derive remaining through 31 Dec once. Cap 4/12/52. Wrong count = ticket. Remaining-year = remaining periods, not full-year 4/12/52. | C4, §3.3, F2, M17 | `remaining_year_matches` / `remaining_year_dates_disagree` in `collector.rs`; persist in `queries.rs` | **Partial** | D2 logic is in code. Leftover tickets still say “expected 4” on CLM, CRF, GLAD, SVOL, TSPY. Do not invent December. |
| Search 19a-1 first. Do not guess filenames as the first step. Persist 0–100 only when parsed. 0 only if notice says 0. Reusable ROC template. `needs_roc_research` until owner accepts. | §3.4, C2 | `live_roc_candidates_for` in `retrieve/mod.rs`; `roc_scope` / `needs_roc_research_on_create` in `collector.rs`; `roc_source_url` on template | **Partial** | D1 `roc_scope` skips CASH / MLP (EPD ET MPLX) / ordinary (TSLL SOXL MSTU) / BDC (GLAD). HAKY is the only live `needs_roc_research=1`. 28 InScope names have stored `roc_pct` but empty `roc_source_url` — **C template missing**. `roc-19a1` runs: 1 ok / 5 fail. |
| Required Skip ≠ complete. ROC propose-only ≠ complete. CASH / not-in-scope may complete without `roc_pct`. | F1, §3.6 | `collector_is_complete` / `collector_status` in `collector.rs`; `collector_status_for` in `queries.rs` | **Present** | 41 complete / 2 incomplete. HAKY blocked on empty DIV-1 + unaccepted ROC + lookback. MSTU blocked on frequency None + 0 pays. Required skip list is `REQUIRED_FIELDS`. |
| Provider, legal name, underlying, DIV-1 on create. Empty `div_type` = ticket. CASH uses a different interest adapter. Risk is owner dropdown, never auto. Last price once; never silent $0. | §3.5, M1, M4, M5 | `div_type_on_create`; price job separate; risk on characteristic | **Partial** | Provider / underlying / risk filled 43/43 except HAKY empty `div_type` and MSTU frequency `None`. CASH adapter present (`fidelity` / `schwab`). DIV-2 rename is parked. |
| Phase I done when: template (adapter, URL, hash) + A or ticket + B or ticket + C or ticket + last price not silent $0. Lots remain zero until Add lots. | §3.6, M13 | `CollectorCompleteSpec.has_template` is `retrieval_template` exists | **Present** | Template row exists for all 43. Lots are a separate command. Note: `has_template` is “row exists,” not “URL still parses today.” That is why 9 of the 11 last-run misses are still complete. |

### Phase II — runtime

| Lock | Cite | Code path | Score | Evidence |
|---|---|---|---|---|
| Routine declaration retrieve runs only on open positions. | M12, §4.1 | `collectorRetrieveNeedsRun` in `apps/desktop/src/App.tsx` (~727); `research_gaps_get` skips `!open_lots` | **Present** | UI and Fill-gaps skip zero-lot names. `CollectorRetrieve` itself trusts the caller. |
| Run misses only / Run enabled / Force refresh (declarations only) / Reload (no HTTP) / Fill research gaps (provider, frequency, DIV-1, ROC — not Force refresh). | §5, button map | `App.tsx` Collectors controls ~9746–10150 | **Present** | Labels match the lock. |
| Incremental persist. Do not rebuild the year. Do not rewrite identity, confirmed frequency, owner Plan $, lots, or ROC observations on a declaration run. | P4, §4.2 | `CollectorRetrieve` persist path | **Present** | Identity and locked cadence were not rewritten by this audit. |
| `last_run_ok` is Step A only. Do not fail it because 19a-1 missed. | X8, C2 | declaration run vs `roc-19a1` kind | **Present** | Last-run fails are empty page / C5 / lookback — not ROC miss. ROC has its own `retrieve_run.kind`. |
| Face source on Position Details is the adapter name, never `derived_walk`. | M10, lock §1 | `queries.rs` clears `derived_walk` from `declaration_source` (~6117); UI uses `declarationSource` | **Present** | Live `declaration_source` values are vendor names (amplify, roundhill, …). |
| `last_run` false opens a work ticket, not only a boolean. | M11, §4.6 | `work_ticket_sync_misses` | **Present** | All 11 last-run misses have an open `declaration_retrieve_miss` and/or lookback / C5 ticket. |
| Yahoo / nasdaq.com / dividendinvestor are not declaration sources. | D4/D5, owner lock | `is_third_party_declaration_url` in `div1.rs`; retrieve drop in `retrieve/mod.rs` | **Present** | Banned hosts filtered before persist. `nasdaq.rs` and `dividendinvestor.rs` files still exist as dead parsers — leftover, not a live source. |

### Parked (report only)

| Lock | Status |
|---|---|
| Manual adapter (P1) | Parked. Not a software miss. |
| 1099 actual process (M7) | Parked. New 2026 names: 2025 actual = N/A. |
| CASH column renamed DIV-2 (§3.5) | Parked. Live column stays `CASH`. |
| Grok auto-apply tier | Parked. |

## Missing collector software (using data as the tool)

These are code holes the live book proves. They are not “the issuer never paid.”

1. **Runtime parse returns empty on a standing seed URL** while `issuer_declaration` already has pays: EFC, ET, GLAD, JEPQ, MPLX, QYLD, SOXL, TSLL, HAKY, MSTU. Seed URLs are on the template. Adapter did work earlier (`ever_ok=Y`, last ok on or before 2026-09-01 for several).
2. **C5 / history-drop treats a short current page as a wipe.** TSPY (48 stored; page misses 2026-10-07 and quotes 0.2954 vs stored 0.30007). SVOL (64 stored; product table shows ~4). Lock says do not erase established data without owner approval.
3. **Lookback uses this-page paid count, not stored total.** GLAD last_run “6 of 12” with 295 stored. IGLD / QDVO / SVOL leftover lookback tickets after last_run ok.
4. **Stale remaining-year tickets from pre-D2 “expected 4”.** CLM, CRF, GLAD, SVOL, TSPY. D2 says remaining periods through 31 Dec, not a forced December. Do not invent a fourth month.
5. **Step C reuse template not established.** 28 InScope names have a working `roc_pct` and `needs_roc_research=0` but empty `roc_source_url`. Future ROC updates have no standing URL/hash. `roc-19a1` retrieve ledger is unused.
6. **MSTU collector was never established on Step A.** Frequency `None`, 0 paid rows, last “Issuer page empty.” Not-in-scope for 19a-1 (owner lock). Software still owes a working Trex/REX distribution parse or a loud two-URL fail that stops calling it complete-ready.
7. **HAKY last_run also fails C5 scale/amount** (`2026-02-27` stored 32051 scale 5 vs expected 320510 scale 6; Jun vs May 30% variation). Owner must click DIV-1 / inception / ROC; software must stop empty-page + scale mismatch on a name that already retrieved 7 pays.

## Owner-entered data still missing

Do not persist these from an issuer pack. Owner names symbol, field, current value, and new value.

| Symbol | Missing owner action | Current stored fact |
|---|---|---|
| HAKY | Set `div_type` (lock: DIV-1). | Empty. Only empty `div_type` on the 43. |
| HAKY | Inception Yes/No. | 7 paid; printed 2026-01-20 / launch 01/21/2026 in prior research; `inception_on` empty. |
| HAKY | Accept ROC estimate. | `needs_roc_research=1`; stored 100.00 (scale 2); `roc_source_url` empty. |
| HAKY | Except/Reject amount and scale tickets. | Jun 4615 vs May 41895; Feb scale 5 vs 6. |
| TSPY | Except/Reject 2026-09-02 amount. | Stored **0.30007**. Issuer page now 0.2954. Do not overwrite. |
| IGLD | Except/Reject amount variation. | Ticket on 2024-12-03 / 2025-12-02 swings. |
| QDVO | Except/Reject amount variation. | Ticket on 2024-10-31 vs 2024-09-30. |
| CLM / CRF | Except/Reject payable supersede. | Vendor 2026-09-30 would replace paid/occurred 2026-09-15. |
| GLAD | Except/Reject payable supersede. | Vendor 2026-09-30 vs stored 2026-09-21. Also leftover lookback (software). |

Risk is filled 43/43. Provider and underlying are filled 43/43 (HAKY underlying is present; DIV-1 is the empty identity field).

## Issuer-fact holes (same-host only)

Ask another AI only for vendor-host facts. Banned: yahoo, nasdaq.com, dividendhistory.org, dividendinvestor.com, aggregators. No invented 0% ROC. No overwrite of stored pays.

Frequency locks stay: AMDW QDTE RDTE TOPW XDTE YBTC Weekly; XPAY Monthly.

**Last-run miss — need a working same-host recipe (do not wipe stored history):**

- EFC `https://www.ellingtonfinancial.com/dividends` — 188 pays stored; rem 2026-09-30, 2026-10-30
- ET `https://ir.energytransfer.com/distribution-history-et` — 168 pays; rem 2026-11-06
- GLAD `https://www.gladstonecapital.com/investors/stock-data/dividend-history` — 295 pays; rem 2026-09-21, 2026-10-23, 2026-11-17. Prefer newsroom detail/394 (D3 seed). JS widget is empty to non-browser clients.
- HAKY `https://amplifyetfs.com/haky/` — 7 pays; rem Sep–Dec month-end
- JEPQ long product slug on `am.jpmorgan.com` — 65 pays; rem 2026-10-01
- MPLX `https://ir.mplx.com/CorporateProfile/stock-information/...` — 57 pays; rem 2026-11-06
- MSTU `https://www.rexshares.com/mstr-etfs/` — 0 pays; frequency None
- QYLD `https://www.globalxetfs.com/funds/qyld/` — 154 pays; rem 2026-09-22, 2026-10-20
- SOXL Direxion product page — 29 pays; stored rem empty (planned derive 2)
- TSLL Direxion product page — 22 pays; rem 2026-09-23, 2026-12-10
- TSPY `https://www.tappalphafunds.com/etfs/tspy` — 48 pays; rem 2026-10-07, 2026-11-04, 2026-12-02. Do not overwrite 2026-09-02 0.30007. Do not drop 2026-10-07.

**Last-run ok — leftover B dates or unparseable increment:**

- YieldMax weeklies AMDY, AMZY, CONY, MSTY, NFLY: last_run ok, remaining 0, ticket “Issuer page changed; unparseable.” Need remaining 2026 Fridays **or** `none`.
- YMAX remaining 0, no open ticket.
- CEFS remaining 0 (monthly). Confirm unused 2026 dates or `none`.
- IGLD rem Sep 30 and Nov 30 — October or `none`.
- CLM / CRF rem Sep 15, Oct 15, Nov 13 — December or `none` (do not invent).
- SVOL rem 2026-09-30 only — need full-history URL that does not wipe 64 pays.
- InScope names with empty `roc_source_url`: standing 19a-1 URL + quote, or `none` with search URLs tried. Do not invent 0.

## What remains (work split)

### Adapter software (must ship before 0-miss)

1. Repair empty-page parsers for EFC, ET, GLAD, JEPQ, MPLX, QYLD, SOXL, TSLL, HAKY, MSTU against the standing seed host (D3 same-host; HTTP 403 is a loud miss).
2. Change C5 / history-drop so a short increment page cannot fail last_run or imply a wipe (TSPY, SVOL). Compare overlapping dates; ticket amount change; keep stored rows.
3. Lookback against **stored** paid count, not this-page count (GLAD, IGLD, QDVO, SVOL).
4. Close or rewrite stale “expected 4” remaining-year tickets (CLM CRF GLAD SVOL TSPY) to F2 remaining-periods language. Do not add a December date.
5. Persist `roc_source_url` (and hash) when an estimate is accepted so Step C has a reuse template.
6. MSTU: establish frequency + paid history or two-URL loud fail. Do not invent ROC.

### Issuer research (other AI, same-host JSON only)

See `docs/Authority/collector_missing_ask_2026-09-07.json` and `docs/Authority/collector_missing_ask_2026-09-07.prompt.md`.

### Owner clicks (do not persist from a pack)

HAKY DIV-1, HAKY inception Yes/No, HAKY ROC accept, HAKY amount/scale Except/Reject, TSPY 0.30007 Except/Reject, IGLD/QDVO amount Except/Reject, CLM/CRF/GLAD payable-supersede Except/Reject.

## Per-collector answers

Ever = any `retrieve_run` declaration `ok=1`. Last = template `last_run_ok`. Complete = `collector_is_complete` as of 2026-09-07. Decl tmpl = adapter + URL (or CASH) + content hash. ROC tmpl = `roc_source_url` or scope skips 19a-1. Rem = stored unoccurred 2026 dates, or derive-once fallback when stored is 0 and vendor published none.

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

Stored remaining dates and last-run message for each name are in the appendix below.

## Hard bans (still in force)

- Do not invent 0% ROC.
- Do not overwrite stored paid rows (TSPY 2026-09-02 **0.30007**, SVOL **64** pays).
- Do not change locked cadence, Plan $/share, or risk unless the owner names symbol, field, current value, and new value.
- Do not persist HAKY DIV-1, HAKY inception, or HAKY 100% ROC accept from a research pack.
- Do not treat golden-harness green as live collectors Done.

## Appendix — remaining dates and last message

- **AMDW** rem `2026-09-08, 2026-09-09, 2026-09-15, 2026-09-22, 2026-09-29, 2026-10-06, 2026-10-14, 2026-10-20, 2026-10-27, 2026-11-03, 2026-11-10, 2026-11-17, 2026-11-24, 2026-12-01, 2026-12-08, 2026-12-15, 2026-12-22, 2026-12-29`; last `declaration retrieve complete`; roc_url empty
- **AMDY** rem none; last `declaration retrieve complete`; ticket unparseable increment
- **AMZY** rem none; last `declaration retrieve complete`; ticket unparseable increment
- **BITO** rem `2026-09-08`; last `declaration retrieve complete`
- **BTCI** rem `2026-09-18, 2026-10-23, 2026-11-20, 2026-12-18`; last `declaration retrieve complete`
- **CEFS** rem none; last `declaration retrieve complete`
- **CLM** rem `2026-09-15, 2026-10-15, 2026-11-13`; last `declaration retrieve complete`; roc_url cornerstone host
- **CONY** rem none; last `declaration retrieve complete`; ticket unparseable increment
- **CRF** rem `2026-09-15, 2026-10-15, 2026-11-13`; last `declaration retrieve complete`; roc_url cornerstone host
- **EFC** rem `2026-09-30, 2026-10-30`; last `EFC: Issuer page empty.`
- **EPD** rem `2026-10-30`; last `declaration retrieve recorded`
- **ET** rem `2026-11-06`; last `ET: Issuer page empty.`
- **FDRXX** rem none stored (CASH; planned derive 4); last `declaration retrieve unchanged`
- **GLAD** rem `2026-09-21, 2026-10-23, 2026-11-17`; last lookback “6 of 12”
- **HAKY** rem `2026-09-30, 2026-10-30, 2026-11-30, 2026-12-31`; last lookback “7 of 12”
- **IGLD** rem `2026-09-30, 2026-11-30`; last amount-variation recorded
- **JEPQ** rem `2026-10-01`; last `JEPQ: Issuer page empty.`
- **MPLX** rem `2026-11-06`; last `MPLX: Issuer page empty.`
- **MSTU** rem none; last `MSTU: Issuer page empty.`
- **MSTY** rem none; last `declaration retrieve complete`; ticket unparseable increment
- **NFLY** rem none; last `declaration retrieve complete`; ticket unparseable increment
- **NVDW** rem 18 Roundhill Fridays through 2026-12-29; last complete
- **ORC** rem `2026-09-29`; last complete
- **PLTW** rem 18 Roundhill Fridays through 2026-12-29; last complete
- **QDTE** rem 16 dates through 2026-12-24; last complete
- **QDVO** rem `2026-09-30, 2026-10-30, 2026-11-30, 2026-12-31`; last amount-variation recorded
- **QQQI** rem `2026-09-18, 2026-10-23, 2026-11-20, 2026-12-18`; last complete
- **QYLD** rem `2026-09-22, 2026-10-20`; last `QYLD: Issuer page empty.`
- **RDTE** rem 16 dates through 2026-12-24; last complete
- **SOXL** rem none stored (planned derive 2); last `SOXL: Issuer page empty.`
- **SPAXX** rem none stored (CASH); last recorded
- **SPYI** rem `2026-09-18, 2026-10-23, 2026-11-20, 2026-12-18`; last complete
- **SVOL** rem `2026-09-30`; last complete; history-drop tickets remain
- **SWVXX** rem none stored (CASH); last recorded
- **TOPW** rem 17 dates through 2026-12-30; last complete
- **TRIN** rem `2026-09-10, 2026-09-30`; last complete
- **TSLL** rem `2026-09-23, 2026-12-10`; last `TSLL: Issuer page empty.`
- **TSLW** rem 18 Roundhill Fridays through 2026-12-29; last complete
- **TSPY** rem `2026-10-07, 2026-11-04, 2026-12-02`; last C5 amount + history-drop
- **XDTE** rem 16 dates through 2026-12-24; last complete
- **XPAY** rem `2026-09-10, 2026-10-15, 2026-11-12, 2026-12-10`; last complete
- **YBTC** rem 17 dates through 2026-12-31; last complete
- **YMAX** rem none; last complete

## Audit pack

Code zip for external review: `docs/Authority/collector_program_code_2026-09-07.zip` (manifest inside). Live SQLite, `.env`, and household PDFs are **not** in the zip.
