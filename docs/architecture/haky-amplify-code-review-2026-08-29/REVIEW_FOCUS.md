# Review focus — what the earlier HAKY audit understated

The first `haky-pd-audit.md` correctly called out bad lot totals and YOC blow-up, but it treated research gaps as a short checklist. For a **complete** collector/research review, these are first-class:

## 1. Vendor / provider name

- **Expected:** `Amplify` (or mapped declaration source `amplify`) on the characteristic and retrieval template.
- **Live (see snapshot):** often blank `provider` even when template `declaration_source=amplify` and URL is `amplifyetfs.com/haky/…`.
- **Code:** `suggested_provider_from_name` / profile probes in `retrieve/mod.rs`; seed body `suggestedProvider`; DIV1 map in `financial-domain/div1.rs`; `production_seed::provider_retrieval_defaults`.

## 2. ROC (Form 19a-1)

- **Expected:** current-year estimate as observation (or explicit unknown — never $0); optional `roc_pct_2026_estimate_minor`; `needs_roc_research` until owner completes.
- **Live:** **0** `roc_research_observation` rows; estimate null.
- **Code:** `amplify_19a1_urls`, `live_amplify_roc`, `parse_19a1_notice`, `live_roc_candidates`; commands `RocResearchRetrieve` / `RocResearchGet`; UI “Validate current ROC estimate”.

## 3. Lookthrough / underlying / theme

- **Expected:** underlying (e.g. HACK), top holdings / theme from fund page when parseable.
- **Live:** `underlying` blank; lookthrough empty/default.
- **Code:** `retrieve/lookthrough.rs` (`extract_lookthrough`), wired through seed / characteristic upsert.

## 4. Declarations + frequency (worked)

- Amplify HTML parser + fixtures (`amplify_haky.html`, etc.).
- Live: 7 paid decls, Monthly frequency — this path largely succeeded.

## 5. Lot economics (separate Process B)

- Cost = **qty × recorded unit price** (lot total).
- MV = **qty × current last price**.
- YOC denominator = sum of lot totals only.
- Live lots repaired 2026-08-29 after unit-as-total entry bug; Add Lot UI now confirms `qty × unit = lot total`.

## 6. Review order

`adapters/amplify.rs` → `retrieve/mod.rs` (collect + ROC + provider) → `lookthrough.rs` → `PositionResearchSeed` in `queries.rs` → `RocResearchRetrieve` → App.tsx research / Validate ROC → `live-snapshot/haky_live_db.json`.
