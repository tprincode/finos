# HAKY collector — establish review for external audit

**As-of:** 2026-09-07 12:04 (America/New_York)  
**System of record:** `%LOCALAPPDATA%\com.finos.desktop\local.sqlite`  
**Slice:** one collector (HAKY) until Phase I complete. Other 17 incomplete names were not established this turn.

Collectors **fleet** is not Done. HAKY **this name** is Phase I complete.

---

## Direct answers

### Did we run the collector?

Yes. Live `CollectorRetrieve` ran against Amplify (`https://amplifyetfs.com/haky/` + Firestore distributions pack) with `establish=true` and `forceRefresh=true`.

- Candidates / page paid: **7**
- Misses: **none**
- `last_run_ok`: **1**
- `last_run_at`: **2026-09-07T12:04:27**
- `last_run_message`: **declaration retrieve complete**

ROC was parsed from the standing 19a-1 URL (HTTP 200), then `RocPlanConfirm` wrote owner accept of **100%** (10000 scale 2).

### Is the collector complete per the locked requirements?

**Yes for HAKY Phase I.** `CollectorSetGet` after that run: `complete=true`, `gaps=[]`.

Complete ≠ last_run. The gate in `collector_is_complete` (`crates/financial-domain/src/collector.rs`) requires template, DIV-1, frequency, underlying, provider, accepted risk tier, paid lookback (or inception), remaining-year match, and ROC estimate **accepted** (propose-only is not enough). HAKY now meets that list.

| Gate field | Live value | Hole? |
|---|---|---|
| Template / adapter | `amplify`, enabled | No |
| Declaration URL | `https://amplifyetfs.com/haky/` | No |
| ROC source URL | `https://amplifyetfs.com/wp-content/uploads/files/19a-1_Notice_02-27-26_HAKY.pdf` | No |
| `div_type` | DIV-1 | No |
| Frequency | Monthly | No |
| Provider | Amplify | No |
| Underlying | HACK | No |
| Risk tier | Core | No |
| Inception | 2026-01-21 | No |
| Paid history | 7 paid; monthly + inception → CompleteViaInception | No |
| ROC 2026 estimate | 10000 scale 2 (100%), accepted (`needs_roc_research=0`) | No |
| ROC 2026 1099 actual | empty | Not a Phase I hole (estimate, not 1099) |
| last_run | ok | No |
| Open work tickets | none (5 historical tickets are `done`) | No |

### Are there any data gaps?

**No Phase I complete-gate gaps** on HAKY.

These are **not** complete-gate holes, and should not be used to call HAKY incomplete:

1. **2026 ROC actual (1099)** is empty. Correct. The lock persists 0–100 only from a parsed notice as an **estimate**. 1099 actual comes later.
2. **`issuer_pay_date` is dirty:** 234 rows that are copies of five calendar dates (2026-08-31, 09-30, 10-30, 11-30, 12-31). Unique remaining dates from 2026-09-07 are Sep–Dec (4). The complete gate compares unique unoccurred counts, so this did not block complete. It is a **hygiene** defect to fix on the next establish path (dedupe on write).
3. **June vs May dollar sizes differ** (2026-06-30 **4615** scale 4 vs 2026-05-29 **41895** scale 5). That is what Amplify published. After a successful establish, that series **is** the first-build truth. The 30% rule must not treat a never-completed collector’s stored rows as authority; it applies only **after** complete. Historical amount-variation tickets are filed `done`.
4. **Fleet:** 17 other enabled names are still failed/incomplete. They are disposable until each is established the same way. That is a program gap, not a HAKY gap.

Paid rows now stored (issuer source `amplify`):

| Payable | Minor | Scale |
|---|---:|---:|
| 2026-02-27 | 32051 | 5 |
| 2026-03-31 | 39351 | 5 |
| 2026-04-30 | 36195 | 5 |
| 2026-05-29 | 41895 | 5 |
| 2026-06-30 | 4615 | 4 |
| 2026-07-31 | 38826 | 5 |
| 2026-08-31 | 3936 | 4 |

---

## What we did (this slice)

1. **Owner facts already authorized:** DIV-1; inception 2026-01-21; do not invent 0% ROC; do not invent 12 paid rows.
2. **ROC pattern, not a hardcoded ticker answer:** parse Amplify 19a-1 for `of such dividend will be a return of capital` and take the percent before that phrase. Feb 27 notice parsed **100%**. Fallback search if the standing URL fails: `19.1 tax ROC (TICKER) (Provider) website data source`. Dated filename guess is last resort only.
3. **Establish vs runtime split (product):**
   - Collectors page: **Collect fresh distribution data** (daily declarations only).
   - Tools → **Reevaluate collector**: **Establish** (first successful build; issuer pays replace trash from a never-completed collector), **Reevaluate** (characteristics / 19a-1 only; does not rewrite pays), **Accept ROC**.
4. **30% / overlap amount tickets** apply only when `collector_is_complete` is already true, and never during establish. Incomplete stored pays are not the source of truth.
5. **HAKY establish run** on live SQLite, then ROC accept. `CollectorSetGet`: complete, empty gaps.

## What we propose next (reusable, one name at a time)

Do not batch the remaining 17. Each failed collector is trash until that name is established.

For the next symbol:

1. Tools → Reevaluate collector → **Establish** (live issuer retrieve, `establish=true`).
2. Read the 19a-1 (or locked search) with the same sentence pattern. Unknown ≠ 0%.
3. **Accept ROC** on that Tools page when the sentence parsed.
4. Call that name done only when `CollectorSetGet.complete` is true and `gaps` is empty.
5. After that, daily work is **Collect fresh distribution data**. 30% Except/Reject applies.

Also proposed on the establish write path (not required to call HAKY complete): **dedupe `issuer_pay_date`** so Amplify does not stack 40+ copies of the same payable.

Empty-page / 403 / JS-shell names (EFC, ET, JEPQ, MPLX, QYLD, SOXL, TSLL, MSTU, …) still need a working same-host body. No Yahoo / nasdaq / dividendinvestor fill. No proxy unless the owner unlocks it.

## How an external reviewer validates HAKY

1. Open desktop → Data → Collectors. HAKY last run ok; not a miss.
2. Tools → Reevaluate collector. HAKY Complete = yes, Gaps blank, ROC 100.00%.
3. Position Details / InvestmentGet: DIV-1, Monthly, Amplify, HACK, Core, inception 2026-01-21, seven paid rows above, `needsRocResearch` false.
4. Do **not** treat fixture tests alone as the gate. The live SQLite row is the gate.

## Lock files (do not invent spec)

- `.cursor/req_extract/Collector_Requirements_Initial_and_Runtime_Locked_2026-09-04.txt` (§3.4 ROC; F1 skip ≠ complete)
- `.cursor/req_extract/Adapter_Collector_Owner_Decisions_Locked_2026-09-02.txt`
- `docs/architecture/execution.md` (Now / Next)

Code zip: `docs/Authority/haky_collector_establish_2026-09-07.zip` (manifest inside). Live SQLite, `.env`, and household PDFs are **not** in the zip.
