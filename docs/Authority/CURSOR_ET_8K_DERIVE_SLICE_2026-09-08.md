# Cursor slice — ET / MLP1 only: SEC 8-K declarations + force-derive future pay dates

Owner: 8 Sep 2026. Hand this to Cursor. Do **not** write another gap audit. Do **not** scrape `ir.energytransfer.com` or `energytransfer.com`. Do **not** mark live ET complete until desktop last_run is green on the household book.

Parser tests already pass. Live still: `last_run_ok=0`, message `ET: Issuer page empty.`, stored URL still the IR page, open `declaration_retrieve_miss`. That is the defect this slice closes.

## Authority (already locked — do not invent a second spec)

- Collector_Requirements_Initial_and_Runtime_Locked_2026-09-04
- Adapter_Collector_Owner_Decisions_Locked_2026-09-02
- Tax-structure: MLP / LP = K-1 + 1446(f), never 19a-1, `roc_scope=not-in-scope`
- Payable stored; Sat–Fri week derived
- Future dates: vendor **else derive** — this slice **forces derive** for this adapter kind
- Unoccurred dates may move; paid rows need approval to change
- Remaining-year = remaining periods through 31 Dec, not a full 4
- Blank declaration ≠ $0; Plan $ stays until an 8-K amount posts
- Banned hosts stay banned: Yahoo events, dividendinvestor, nasdaq.com
- No live household ticker names in **tests / goldens**. Production seed may map the live common-unit MLP to this adapter kind.

## Scope — two functions, one adapter kind

New adapter kind (name in code as you like; tests call it `mlp_sec_8k`):

1. **Part A — declared amounts** from SEC 8-K only (CIK + common-unit parse).
2. **Part B — future unpaid dates** by **derive only**. Vendor / IR future calendar is off for this kind.

Do not implement 19a-1 (Part C) for this kind. Skip ROC.

Do not change Amplify / Roundhill / Direxion / Rex / Gladstone / cash adapters except to refuse handing ET/MLP1 to them.

## Hard bans for this kind

- Do not GET `ir.energytransfer.com`, `energytransfer.com`, `energytransferpartners.gcs-web.com`, or any Q4 distribution-history tab.
- Those hosts 403 (volt-adc). Treating 403 HTML as an empty issuer table is the current live bug.
- Do not parse preferred-unit tables (Series I prints $0.2111 — that is not common).
- Do not write 0% ROC. Do not hunt 19a-1 filenames.
- Do not use aggregator calendars to invent next pay dates.
- Do not copy last declared rate onto a derived future row as a declaration.

## Function 1 — force-derive future quarterly pay dates

For `adapter_kind = mlp_sec_8k` (and only that kind):

```
future_dates_mode = derive_quarterly
vendor_future_dates = off
```

Skip Part B vendor fetch even if a `source_url` on the position still points at IR. If stored URL is IR, **replace it** on first successful 8-K last_run with the SEC filings URL used. Do not leave IR as the lookback URL.

### Derive template

- Frequency = Quarterly.
- Remaining-year slots = unpaid quarters whose **payable** falls on or before 31 Dec of the plan year.
- Cadence: payable on or about the **19th–20th of February, May, August, November** (~50 days after quarter-end). Record date same week as ex.
- Anchor = last **accepted** payable (8-K or already-stored paid). Step +1 calendar quarter.
- Persist each derived row: `payable_date`, `source=derived_template`, `amount_per_share = null` (Plan $). Week = Sat–Fri containing payable.
- Do not persist derived rows past 31 Dec of the current plan year in this slice.

Golden (MLP1, no household name): last accepted payable 2026-08-19 amount 0.3400 from 8-K. Derive one remaining 2026 row payable ≈ 2026-11-19, amount null, source derived_template. Do not create 2027 in remaining-year.

## Function 2 — new declaration from 8-K + verify pay dates

### Retrieve

- CIK for the live common-unit MLP: `0001276187`.
- Tests use a **fixture 8-K HTML/text** checked into the repo (do not hit sec.gov from unit tests).
- Live retrieve: SEC company-filings ATOM  
  `https://www.sec.gov/cgi-bin/browse-edgar?action=getcompany&CIK=0001276187&type=8-K&owner=exclude&count=20&output=atom`  
  then the 8-K index + press-release exhibit.
- SEC requires a real `User-Agent` with contact. Anonymous GET 403s. If SEC 403s, ticket `blocked: sec_403` + the ATOM URL. Do **not** fall back to IR.
- Match filings whose body/title is quarterly **common unit** cash distribution. Ignore preferred series rates.

Parse from the 8-K / exhibit:

- common-unit amount
- record date
- payment date

Known live print (for the desktop Run check only, not a test hard-code of the ticker): Q2 2026 common **$0.3400**, record 2026-08-07, payable **2026-08-19**.

### Apply

| Case | Action |
|---|---|
| Quarter has no stored declaration | Insert declaration = 8-K amount, payable = 8-K pay date, `source=sec_8k`. Unoccurred. |
| Stored row is `derived_template` and 8-K payable is within 3 business days | Promote source to `sec_8k`. Write amount. Keep week derived from new payable if it moved inside the window. |
| 8-K payable differs by more than 3 business days | Unoccurred date may move. Ticket `payable_date_moved` with old derived vs 8-K. Apply the 8-K payable. Do not touch a **paid** row. |
| Amount on a **paid** row would change | Ticket only. Do not overwrite. |
| No 8-K in this board window | Keep derived date + null amount. Ticket `declaration_retrieve_miss` for **that quarter only**. Message must not be `Issuer page empty.` |
| IR URL still stored | After first 8-K success, rewrite stored history URL to the SEC ATOM or the 8-K exhibit URL actually parsed. last_run_ok cannot go 1 while stored URL is IR. |

Validate by re-parse of the **same 8-K URL**, not SQLite-only.

## last_run_ok for this kind

`last_run_ok = 1` only when all of:

1. Stored history URL is the SEC path (not IR).
2. At least the latest declared common-unit quarter parsed from 8-K and persisted.
3. Remaining-year derived row(s) through 31 Dec exist with null amounts.
4. No open ticket whose text is `Issuer page empty.`
5. Lookback uses **stored** paid count (already shipped). Short 8-K (one quarter) must not fail last_run as history-drop.

Desktop Run is the gate. Fixture green is not acceptance.

## Tests (synthetic names only)

```
cargo test -p financial-domain
cargo test -p golden-harness --test collectors --test new_investment
```

Required goldens:

- MLP1 + fixture 8-K with $0.3400 payable 2026-08-19 → one declaration source=sec_8k.
- Same + derive → one 2026-11 payable source=derived_template amount null.
- Second fixture 8-K payable 2026-11-21 (2 days off template) → date moves, ticket payable_date_moved, amount posted, last_run ok.
- Fixture that is Series I preferred $0.2111 only → no common row written, miss for common quarter, no $0.2111 on MLP1.
- Retrieve 403 from IR must never be attempted; if old seed URL is IR, adapter ignores it and uses SEC.
- last_run message must not contain `Issuer page empty` when SEC returns no new 8-K; use quarter-scoped miss.

Do not put the live ticker string in goldens, roc_scope tables, or seed defaults. Production mapping: position with this CIK / adapter_kind uses this path.

## Out of scope

MSTU Cloudflare, SOXL LH strip, cash 7-day yield pages, 19a-1, Plan $ auto-edit, owner click persistence, SCHEMA_VERSION.

## Done when

Cursor returns: diff + test list + what last_run stores for MLP1. Owner then runs desktop on the household book. ET is complete only when that live row shows last_run_ok=1, stored URL is SEC, Aug-2026 $0.3400 is present, Nov-2026 derived exists with null amount, and `Issuer page empty` is gone.
