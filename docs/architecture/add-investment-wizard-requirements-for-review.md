# Add / Maintain Investment Wizard — Complete Requirements for External Review

**Status:** Working paper for owner and external review. **Not locked authority.**  
**Date:** 2026-08-21  
**Product:** Profile A — one-owner Tauri desktop, local SQLite  
**Does not override:** `docs/Authority/` (Architecture V1.1 → BR-X → domain specs → production seed gates)

This paper states the **complete owner-facing wizard** required to establish and maintain a new investment so that Calculator, Position Details, Holdings, Income Plan, Dashboard, Last Price, and the Investment Activity Ledger can tell the truth without the spreadsheet. Where a rule is already locked in a domain document, the ID is cited. Where the owner has stated a process that the locked docs only imply or leave open, it is marked **Owner lock required**.

---

## 1. Why this wizard exists

Position Details is a **symbol-level decision surface**, not a security-master form (Position Details §1, PD-BR-01). Quantity, cost, account splits, prices, yields, declarations, Plan, ROC character, and bull/bear results are **joined views**. They are displayed together; they are **not** stored as columns on one Position record (TR-PD-29).

Therefore “add a ticker” is not a valid design. Establishing a new investment is an **onboarding process** that:

1. Creates a durable investment identity.
2. Records how the system will **keep retrieving** prices, issuer declarations, and broker payments for that identity (a standing template, not a one-shot paste).
3. Loads enough history and a current price so Plan and valuation can be decided.
4. Lets the owner **confirm Plan** (never auto-written).
5. Opens lots with dual cost.
6. Only then lets Income Plan and Dashboard consume Plan × quantity × schedule.

The desktop today has **no such wizard**. Seed loaded the existing book from templates. `SecurityRegister` is a test/seed stub (symbol/name/CRF only). Last Price, declaration-history migration, and Python connectors are **parked** on the execution board.

---

## 2. Authority sources

| Source | Role |
|--------|------|
| System Architecture V1.1 (locked 2026-08-15) | Profile A, posting boundary, progressive automation |
| Cross-Cutting BR-X (locked 2026-08-14) | Enter once; unknown ≠ zero; import idempotency; adapters write validated facts |
| Position Details (locked 2026-08-12) | Identity, Risk On, frequency, ROC, backtesting evidence |
| Calculator (corrected v0.4 + patches, 2026-08-12) | Plan vs declaration vs actual; Most Current / Avg 6; Plan-review |
| Last Price (v1.0, 2026-08-13) | CurrentPrice, stale/unavailable, daily retrieval, manual override |
| Income Plan (v1.0, 2026-08-13) | Sat–Fri week; Plan never auto-updates; week close |
| Holdings working foundation (2026-08-12) | Lots, dual cost, no FIFO |
| ROI / Investment Activity Ledger (v1.0, 2026-08-13) | Broker actuals, provenance, ROC does not rewrite economic cost |
| ADR-0010 | Connectors return **candidates only**; they never post facts |
| ADR-0008 | Reporting views do not post ledger facts |
| `Template_Positions.xlsx` | Seed field list for characteristics and ROC year columns |

---

## 3. Scope and non-goals

### In scope

- **Add** a new investment (not already in the data SQLite).
- **Maintain** an existing investment’s identity, retrieval template, declarations, Plan, characteristics, and lots.
- First save **and** the standing process that keeps data current after save.
- Every tab that would otherwise show invented zeros or blank-as-complete.

### Out of scope (must remain visible as not-yet, not silently skipped)

- Shopping Cart / what-if fills (parked).
- Autonomous trades or money movement (ARCH-09).
- AI posting of facts (ADR-0012).
- Connector **posting** (ADR-0010: candidates only).
- Postgres / OIDC / web (M8/M9).
- MAGI oracle rewrite.
- Week-close immutability until that slice is named (Income Plan still specifies it; the wizard does not close weeks).
- Using “Average all scores” as a production rank (legacy only, TR-PD-19).

### Non-goals that would be design errors

- One grid that mixes Plan edit, declaration entry, and broker cash (TR-C-6).
- Storing Plan, cash, quantity, or price on the Position row (TR-PD-29, TR-C-11, TR-LP-17).
- Auto-updating Plan from declarations, Avg 6, or broker payments (TR-C-8, BR-IP-03).
- Treating blank declaration weeks or missing prices as zero (Calculator patch 2, BR-LP-05, TR-LP-09).
- Renaming **Risk On** (PD-BR-09; Template `HighRisk` maps to Risk On on import).
- Reducing original economic cost when ROC is characterized (PD-BR-11, TR-PD-31–34).

---

## 4. Three income events (must stay separate)

These are unique records. Conflating them is a design error (Calculator §3.7).

| Event | Record | Meaning | Wizard role |
|-------|--------|---------|-------------|
| **Plan** | `PlanHistory` | Owner’s dollars per share per planning period for **forward** Income Plan | Confirmed only after Plan-review; explicit button; provenance required |
| **Issuer declaration** | `DividendDeclaration` | Declared per-share amount in the Sat–Fri week the position is **supposed** to pay | Loaded via retrieval template + lookback; null = no observation |
| **Broker actual** | Investment Activity Ledger, activity type Dividend | As-paid cash | Ongoing import/retrieval into the ledger; **never** written into declaration history |

Corrections are **immutable**: never overwrite; insert a superseding row with reason, actor, timestamp (Calculator provenance section; TR-C-1, TR-C-2).

---

## 5. Two calculation gates

The wizard must not present a single “yield” as if price and declarations were the same input.

### Gate A — CurrentPrice (Last Price)

Every active or decision-relevant identity has at most one effective current price (BR-LP-01). Selection order (Last Price §4.2):

1. Active authorized **manual override** (reason, window, expiry) — **required from day one** (BR-LP-03).
2. Newest accepted automated quote.
3. Prior accepted quote if today’s run failed → **Stale**, with age; never relabeled current (TR-LP-08).
4. No accepted quote → **Unavailable**. No zero substitution. **Block or clearly invalidate all price-derived calculations** (TR-LP-09).

**Price-derived (blocked when Unavailable):** market value, unrealized P&L %, plan / most-current / 3-recent **forward yield on price**, portfolio allocation % on market value.

**Not price-derived (Last Price does not gate these):** lot quantity, original economic cost, tax basis, Plan-based YOC = (Plan × periods) / weighted average original unit cost (BP-C-04, TR-PD-16).

**Owner lock required:** the owner has stated that **without current last price, no yield or position calculations should be possible**, and last price belongs in the wizard. That is **stricter** than TR-LP-09. External reviewers should accept or reject hiding original-cost YOC and lot economics until CurrentPrice exists.

Consumers use only the common CurrentPrice contract (BR-LP-06, TR-LP-10). No screen fetches an ungoverned price. Price is not stored on Position or Lot (TR-LP-17).

### Gate B — declaration performance (Calculator)

Plan is confirmed only after the Plan-review service has declarations to work with (BP-C-02, TR-C-7). Present, excluding null/blank observations:

- Most Current = first non-zero newest declaration (BR-C-03)
- Avg 6 = average of the last **six non-zero observations**, not six months (BR-C-04)
- Min / max / average / 80% of average
- Over/Under % vs current Plan (if any)
- Counts above / equal / below Plan
- Current-payment dollar delta and future annual-income impact

**No suggested Plan algorithm** (TR-C-7, BP-C-02). Owner confirmation creates a new effective `PlanHistory` version.

**Income Plan going-forward dollars** = confirmed Plan × eligible open quantity × normalized periods (52 weekly / 12 monthly / 4 quarterly). Frequency alone is not a Plan. Last Price is not a Plan. Without Gate B, week Plan stays **unknown**, never a invented projection (BR-IP-03, TR-C-10).

**Remaining-year payment calendar (New Investment and Add Lot):** after Plan exists, the same engine lists each remaining payment date through 31 Dec, calendar-month cash (Plan × qty) for months that have a payment, and year-to-go total. Weekly = every remaining Sat–Fri week. Monthly/quarterly walk 30/91 days from the latest parseable declaration `paymentPeriod`. Unparseable labels leave the schedule **unknown** (not $0); the owner may name the next pay date in the same row. Owner date overrides persist; system-proposed dates stay derived. Add Lot does not re-ask Plan; it shows this-lot cash and position-after-add on that calendar. Hypothetical until `LotOpen`. Empty months are omitted, never $0. TR-C-2 ex/record/payment columns remain later.

Income Plan weeks for a security with **no** broker actuals use this declaration calendar. Securities that already have actuals keep the last-actual walk so the seeded book does not silently move. MAGI remaining periods equal the remaining date count.

---

## 6. Durable retrieval template (standing order)

This entity is **implied** by Last Price (adapter + run), ROI (import provenance), BR-X (source adapters), and ADR-0010 (candidate jobs). It is **not** named in schema today. External review should treat it as a required onboarding artifact.

The template lives on the **investment identity**, not on a one-time import dialog. Saving the wizard without it means the position cannot be maintained.

### 6.1 Required fields

| Field | Purpose | Notes |
|-------|---------|--------|
| `investment_id` | Durable key | Same as Position / price / declarations |
| Price source + `source_symbol` + quote type + calendar | Daily CurrentPrice run | PriceSourceSymbolMap (TR-LP-01, TR-LP-13) |
| Price freshness threshold | Stale vs current | Configurable; not a manual label (Last Price §5) |
| Payment / distribution source | Broker actuals | Fidelity / Schwab / other file or future connector; account via `broker_account_number` (BR-X) |
| Declaration source | Issuer notices, fund site, file, or manual | BP-C-01 is still manual until an adapter exists |
| **Declaration lookback** | How far back to retrieve on first onboard and on refresh | **Owner lock required** (see §12) |
| Payment frequency | Weekly / Monthly / Quarterly / None | Effective-dated; periods 52/12/4 (TR-PD-08, TR-C-5) |
| Expected weekday patterns | Typical declaration / ex / payday weekday | Patterns only; not actual event dates (TR-PD-09) |
| ROC returning? | Yes / no / unknown | Unknown ≠ no |
| ROC % + tax year + Estimate/Actual + source | 1099 vs 19a-1 vs research | Does not rewrite original cost |
| Bull period | Exact start/end, name, benchmark, method, reason | Required before **new** scores (PD-BR-03, TR-PD-20) |
| Bear (and optional recovery/stress) period | Same | Retain multiple periods; do not overwrite (TR-PD-30) |
| Refresh cadence | Business-day price; declaration/payment as specified | Record run requested/accepted/rejected/missing (TR-LP-14) |
| Owner review required? | Always for posting facts | Candidates → validate → approve → post; never silent post |

### 6.2 Ongoing operating rules

- Adapters (manual form, paste, broker file, later API) produce **candidates** with raw payload and provenance (BR-X, ADR-0010).
- Posting uses the same Import/approval boundary as existing `ImportStage` → validate → approve → post.
- Reprocessing the same source transaction must not duplicate (BR-X import idempotency; ROI).
- Failed price refresh leaves last accepted quote **Stale**, never zero (TR-LP-08).
- Future declaration weeks are **not created as $0** (TR-C-2, TR-C-6).
- Inactive positions keep history but leave routine refresh unless explicitly included (TR-LP-18).

### 6.3 What “how many months of dividend history” means

The live Calculator grid is ~120 Sat–Fri columns (storage in the spreadsheet, not a retrieval SLA). **Avg 6 is six observations**, which is ~6 weeks for a weekly payer and ~6 months for a monthly payer.

The locked docs **do not** pick a retrieval lookback. External review must lock one of:

- **N months** of Sat–Fri weeks (same calendar span for all frequencies), or
- **N observations** (aligned with Avg 6 / min-max), or
- Frequency-specific (e.g. 24 weekly observations, 12 monthly, 8 quarterly).

Until locked, the wizard must not invent a default that quietly truncates Plan-review.

---

## 7. Wizard steps (complete)

The wizard is linear with explicit Continue. The owner may save a **draft identity** but must not publish price-derived yields, Plan-driven Income Plan projections, or new bull/bear scores until the listed gates pass. Maintain mode re-enters the same steps for an existing `investment_id`.

### Step 1 — Identity

**Writes:** Position (not lots, not Plan, not price)

| Field | Rule |
|-------|------|
| `position_id` / `investment_id` | Immutable; system-assigned (TR-PD-01) |
| Current symbol | Mutable; not the primary key; start `PositionIdentifierHistory` |
| Name / instrument type | Required |
| Provider | Normalized relationship; preserve original text (TR-PD-06) |
| Underlying / exposures | Prefer typed exposures, not one overloaded string (TR-PD-07); seed has a single `underlying` column |
| Active? | Inactive stays available for lifetime history (AC-PD-15) |
| Notes | Free text |

**UI:** Do not ask for Plan, yield, or market value here.

### Step 2 — Risk and role

**Writes:** `PositionClassificationHistory`

| Field | Rule |
|-------|------|
| Owner tier | **Foundation \| Core \| Risk On** only. Import `HighRisk` → Risk On. Do not silently rename (PD-BR-09, TR-PD-04). |
| Calculated suggestion | Optional later; show components and ruleset; **only owner action** changes effective classification (TR-PD-05, AC-PD-12, TR-PD-25) |
| Override reason | Required if suggestion exists and owner disagrees |
| Effective from | Dated |

**UI:** No auto-tier from incomplete scores. Missing evidence lowers confidence; it must not improve a composite (PD-BR-10, TR-PD-26).

### Step 3 — How it pays (schedule, not dollars)

**Writes:** `PaymentFrequencyHistory`, `ExpectedPaymentPattern`

| Field | Rule |
|-------|------|
| Frequency | Weekly / Monthly / Quarterly / None; effective-dated |
| Normalized periods | Derived 52 / 12 / 4; a 53rd calendar payment in a year does not change the multiplier (TR-C-5) |
| Typical declaration weekday | Pattern, not an event (Position Details AG) |
| Typical ex-date weekday | Pattern (AH) |
| Typical payday weekday | Pattern (AI) |

**UI:** State clearly that this does **not** create Income Plan dollars. Schedule without Plan = unknown planned cash.

### Step 4 — Retrieval template (Step 6)

Complete §6.1 on this investment. Cannot finish the wizard without it. This is the **definition of ongoing data retrieval**.

**UI:** Separate sub-panels: Price source, Broker payment source, Issuer declaration source, ROC research, Regime periods. Each shows last run status when in maintain mode.

### Step 5 — CurrentPrice (Gate A)

**Writes:** `PriceQuote` and/or `ManualPriceOverride`; never a Position.price column

Day-one path when automation is parked or unavailable:

- Owner enters price, currency, quote type, as-of (or explicitly **as_of_unknown** for legacy), reason, effective window.
- Label **Manual Override**, not Current automated.
- Or owner records **Unavailable** and the wizard **does not** display price-derived metrics as valid.

Automated path (when Last Price is unparked): run retrieval for this identity; validate; store accepted/rejected; show freshness.

**UI:** Always show source, as-of, retrieval time, freshness, override flag wherever a valuation appears (TR-LP-11).

### Step 6 — Load declaration history (Gate B input)

**Writes:** `DividendDeclaration` rows only (not ledger cash, not Plan)

| Field per row | Rule |
|---------------|------|
| Sat–Fri `payment_period` | Week the position is **supposed** to pay |
| `amount_per_share` | Nullable; null = no observation |
| Source | **Required** (URL, notice id, file name, or free text) |
| Optional dates | declaration / ex / record / payment dates — later fields (TR-C-2) |

Load using the template lookback. Bulk paste allowed (TR-C-6). Period correction = new superseding row, not overwrite.

**UI:** Grid of weeks; future weeks omitted or blank, never pre-filled with 0. Do not mix cash import into this grid.

If lookback cannot be executed (no adapter, owner declines paste): Gate B fails; Plan confirm is disabled; Income Plan Plan remains unknown.

### Step 7 — Plan-review and Plan confirm (Gate B)

**Reads:** declarations from Step 6, frequency from Step 3, quantity if lots already exist (maintain)  
**Writes:** new `PlanHistory` version **only** on explicit confirm

Show TR-C-7 metrics. Owner enters Plan per share (or keeps existing). Required on confirm: `decision_date`, `decision_reason`, source provenance, `planning_periods_per_year` consistent with frequency.

**UI:** Separate from declaration grid. Button labeled to the effect of “Confirm Plan for future weeks” — not “Save yield.”

After confirm: future Income Plan events use this Plan; **completed weeks never recalculate** from a later Plan (TR-C-10, BR-IP-04).

### Step 8 — Lots and dual cost

**Writes:** Lot(s); never Plan on the lot (TR-C-11)

| Field | Rule |
|-------|------|
| Account | Existing data account identity |
| Opened on | Date |
| Origin | e.g. purchase |
| Quantity + quantity scale | Remaining quantity |
| Original economic / performance basis | Actual price paid; immutable except documented correction (PD-BR-08) |
| Tax basis | Separate; ROC-adjusted tax basis only on taxable lots when characterized (TR-PD-32) |

No FIFO. Explicit lots only. Quantity and account splits on Position Details are **derived** from open lots (TR-PD-12/13).

**UI:** May add the first lot here or require at least one open lot before treating the investment as an active holding. Closed/zero remaining lots remain for history (AC-PD-15).

### Step 9 — Tax / ROC characterization

**Writes:** `PositionTaxProfile` (expected planning treatment) and `PositionTaxCharacterization` (year rows). V1 seed may keep locked year columns (`roc_pct_2024_actual`, `roc_pct_2025_actual`, `roc_pct_2026_estimate`, `roc_pct_2026_actual`) until tax-year rows exist (Position Details appendix lock 3).

| Field | Rule |
|-------|------|
| Returning ROC? | Yes / no / unknown |
| Percent + Estimate/Actual + tax year + source | 1099, 19a-1, research note |
| Expected tax handling | Ordinary / qualified / etc. — planning shorthand, not annual character (TR-PD-10) |

**Invariant:** ROC cash is counted **once** in lifetime distributions; it does **not** reduce original economic cost (PD-BR-11, TR-PD-31–34, AC-PD-16–20).

### Step 10 — Regime scores (only if periods are dated)

**Writes:** `BacktestPeriod` (owner-approved) then `PositionBacktestResult` (calculated)

Owner supplies exact start/end, Bull / Bear / Recovery / Stress, benchmark, selection reason, method (adjusted close vs not; reinvested vs cash — **unknown must be labeled**, not invented; Position Details §12).

Then the system may compute (Position Details §6.2): price return vs total return, distribution cushion, bear/bull relative return, downside/upside capture (guarded denominators), max drawdown, recovery ratio/time, income reliability in period (actual vs planned), data confidence.

**UI:** Do not show a single Average(D:K). Present the evidence profile dimensions: Income Reliability, Downside Resilience, Recovery/Upside, NAV Persistence, Diversification, Data Confidence (Position Details §6.3). Missing components **lower confidence**; they do not reweight the rest (TR-PD-26). Never auto-change Plan, tier, or trade from a score (TR-PD-25).

If dates/method are missing: **no new score**. Legacy sheet numbers may be shown only as unlabeled/unverified imports (TR-PD-20, AC-PD-08).

### Step 11 — Derived views (read-only in the wizard)

After gates pass, show what other tabs will see. These are **not** extra stores.

| Tab | What becomes valid | Still invalid if |
|-----|--------------------|------------------|
| Calculator | Plan, declarations, Plan-review metrics; MV/P&L%/price yields only if Gate A | Plan-review empty without declarations |
| Position Details | Identity, tier, frequency, joined qty/cost; MV/allocation only if Gate A; scores only if Step 10 dated | Completeness must be visible (PD-BR-10) |
| Holdings | Open lots, dual cost | — |
| Income Plan | Future week Plan = Plan × qty × expected week (declaration calendar when no actuals) | Unknown if Plan unconfirmed; actuals only from ledger |
| Dashboard | Uses Income Plan plan until week close; then closed actuals for Income/Car/Health/Roth | Account 9 not in burndown (BR-IP-11) |
| Last Price | CurrentPrice for this identity | Unavailable/Stale labeled |
| ROI / Import | Broker actuals as they post | Cash never copied to declaration grid |

---

## 8. Cross-tab completeness checklist (reviewer)

A new investment is **complete for daily use** only when:

1. Identity and Risk On/Core/Foundation exist.
2. Retrieval template exists (price, payments, declarations, lookback, ROC research, regime dates or explicit “not scoring yet”).
3. CurrentPrice is Current, Manual Override, Stale, or Unavailable — never a silent blank.
4. Price-derived numbers are shown **only** when Gate A is Current or Override (or Stale if the UI labels them stale).
5. Declaration lookback is loaded or Gate B is explicitly failed.
6. Owner has confirmed Plan, or Income Plan Plan is labeled unknown.
7. At least one lot exists for an **active holding** (identity-only watch names are allowed if excluded from routine refresh — TR-LP-18).
8. ROC unknown is visible as unknown, not 0%.
9. Bull/bear scores are either computed from dated periods or omitted.

If any of 3–6 fail, the other tabs must show **unknown / unavailable / incomplete**, not $0 Plan or $0 yield.

---

## 9. Posting and automation boundary

```
Source adapter or manual entry
  → candidate observation (payload + provenance)
  → validate (unknown stays unknown; reject nonpositive prices)
  → owner approve when required
  → post to the owning domain only
```

- UI, import, connector, and AI **must not** bypass this (AC-ARCH-12, ADR-0010, ADR-0012).
- `ConnectorJobSubmit` is in the component contract and **is not implemented**. Until it exists, payment/declaration retrieval in the wizard is **file import + paste + manual**.
- Python workers, when unparked, still **do not post**.

---

## 10. Acceptance criteria (wizard)

Reviewers can treat these as the build gate. Existing domain ACs still apply.

| ID | Criterion |
|----|-----------|
| WZ-01 | Wizard cannot finish an active holding without a retrieval template on that investment. |
| WZ-02 | Plan, declaration, and broker cash cannot be entered in one undifferentiated grid (TR-C-6). |
| WZ-03 | Blank declaration weeks and missing prices cannot become zero. |
| WZ-04 | Price-derived metrics are invalid or blocked when CurrentPrice is Unavailable (TR-LP-09). |
| WZ-05 | Plan confirm is disabled until Plan-review inputs exist or owner records “Plan unknown.” |
| WZ-06 | Confirming Plan creates an immutable `PlanHistory` version with provenance; it does not rewrite closed weeks. |
| WZ-07 | Lots never store Plan; Income Plan future weeks join effective Plan to open quantity and schedule. |
| WZ-08 | Risk label stored is Foundation, Core, or Risk On. |
| WZ-09 | ROC characterization does not change original economic cost. |
| WZ-10 | New bull/bear results require dated periods, benchmark, and method; otherwise no new score. |
| WZ-11 | Every declaration and Plan version has source, actor, time; corrections supersede, they do not overwrite. |
| WZ-12 | Re-import of the same broker payment does not duplicate ledger cash. |
| WZ-13 | Maintain mode edits the same investment; ticker change is identifier history, not a new position_id. |
| WZ-14 | No UI SQL; all writes go through FinanceClient commands. |
| WZ-15 | New Investment and Add Lot present remaining payment dates, calendar-month cash, and year-to-go (unknown ≠ $0). A new monthly position with declarations and no actuals is scheduled on Income Plan weeks that contain a remaining date. |

---

## 11. What is implemented today (gap)

| Capability | Today |
|------------|--------|
| Data seed (8 accounts, 74 symbols, lots, yield, 40 PlanHistory rows, characteristics) | CLI `npm run data-seed` |
| Income Plan week grid, Dashboard burndown, Holdings lots, Import, read-only Calculator | Desktop menus |
| `SecurityRegister` | Tests/seed only; not a menu |
| `position_characteristic` | Frequency, risk_tier, provider, underlying, ROC year fields, div_type — from seed, no owner form |
| `DividendDeclaration` / issuer declarations | `IssuerDeclarationRecord` + Plan-review (schema 15 `issuer_declaration`) |
| CurrentPrice / PriceQuote / daily run | `PriceQuoteRecord`, `ManualPriceOverride`, `CurrentPriceGet`, `PriceRetrievalSetGet` (complete holdings only) |
| Retrieval template entity | `RetrievalTemplateSet` |
| Plan-review + Plan confirm UI | New Investment wizard; `PlanHistoryConfirm` (not MAGI `PlanApprove`) |
| Add lot from UI | Add Lot screen + New Investment Part 2 `LotOpen`; remaining-year date list, month totals, year-to-go |
| Connector jobs | Contract only |

Existing seed is **onboarding-by-migration** for the current book. It is not a substitute for this wizard for the next symbol.

---

## 12. Owner locks (2026-08-21)

These are closed for this slice. They do not rewrite locked Authority docs.

1. **Declaration lookback:** last **12 declarations** (observations) from the provider site. Not 12 months. If retrieve is empty, owner pastes/enters up to 12 with required source.
2. **Price:** establish CurrentPrice from **live public quote**; if unavailable, **prompt** for current price. Do **not** hide original-cost YOC or lot economics waiting for a price. Do **not** paste a legacy 70-row Last Price sheet (that was spreadsheet A2:B71 only). A ticker has a quote on initial add and each business day.
3. **Watchlist:** **none.** New Investment is incomplete until the **first lot**. Part 1 (facts) then Part 2 (first lot) is one process. Calculator, Income Plan, and the daily price set omit incomplete investments.
4. **Plan-review completeness:** `n` = non-null of the 12. n=0 confirm **blocked**. n=1–5 confirm only with explicit incomplete-analysis reason; Avg 6 / full analysis not possible. n≥6 full TR-C-7. Plan never auto-fills from Avg 6.
5. **Two owner processes:** **New Investment** vs **Add Lot** (existing symbol). Add Lot does not re-ask Plan or re-fetch 12 declarations. It opens a lot, joins Plan × qty into upcoming Income Plan weeks, and keeps the symbol on the maintenance set. Both processes show the remaining-year payment calendar (dates, month cash, year-to-go). Add Lot shows this-lot vs position-after-add on that calendar.
6. **Remaining-year dates:** walk from the latest parseable declaration `paymentPeriod` + frequency (weekly = remaining Sat–Fri weeks; monthly ≈ 30 days; quarterly ≈ 91 days). Owner may override a date in the same row. Empty months are blank/unknown, not $0. Persist overrides only.
7. **Now:** this slice. Last Price and declaration history are unparked **for this wizard**. Connector *posting* stays parked (ADR-0010: candidates only).

---

## 13. Recommended implementation order (after locks)

Still Profile A / SQLite. Do not start Postgres or OIDC to deliver this.

1. Schema: Position identity history, classification history, frequency history, **retrieval template**, `DividendDeclaration`, PriceQuote + ManualPriceOverride (minimal), PlanHistory confirm command distinct from MAGI `PlanApprove`.
2. Wizard Steps 1–4 (identity, risk, schedule, template) — no fake yields.
3. Step 5 manual CurrentPrice (unpark Last Price for override + Unavailable).
4. Step 6 declaration load for lookback (unpark declaration history).
5. Step 7 Plan-review + confirm; Income Plan consumes new Plan.
6. Step 8 LotOpen from UI.
7. Steps 9–10 ROC + dated backtests.
8. Connector candidates later; posting boundary unchanged.

---

## 14. Document control

- **This file** is a review packet. Approval does not by itself lock Authority; a versioned authority drop or ADR should follow accepted owner locks in §12.
- Domain documents remain the source of IDs (TR-PD-*, TR-C-*, TR-LP-*, BR-IP-*).
- Do not regenerate MAGI oracles. Do not edit `docs/Authority/` in place to match this paper.

**End of review packet.**
