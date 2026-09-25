# HAKY / Amplify complete code review pack

Built: 2026-08-29  
Purpose: Full review of collector + research path for HAKY (Amplify), including gaps the earlier PD audit understated (ROC, vendor/provider, lookthrough, characteristic fields).

## How to review

1. Start with `REVIEW_FOCUS.md` (gaps + expected data points).
2. Read live snapshot: `live-snapshot/haky_live_db.json` (what the household DB actually has).
3. Trace code in order below (collector → seed → ROC → UI).

## Expected data points (complete research)

| Data point | Where stored | Code path |
|------------|--------------|-----------|
| Symbol / name | `security` | `PositionResearchSeed` / retrieve profile |
| **Vendor / provider** | `position_characteristic.provider` | `suggestedProvider` from name/URL; seed + `PositionCharacteristicUpsert` |
| **Underlying** | `position_characteristic.underlying` | profile / lookthrough / owner |
| **Lookthrough** | `position_characteristic.lookthrough_json` | `retrieve/lookthrough.rs` → seed body |
| Frequency | `position_characteristic.payment_frequency` | inferred from paid decls (≥2) |
| Declarations | `issuer_declaration` | `adapters/amplify.rs` via `collect_from_fetched_page` |
| Declaration source | `retrieval_template.declaration_source` | `amplify` via DIV1 provider map |
| Source URL | `retrieval_template.source_url` | owner seed URL |
| Collector enabled / last run | `retrieval_template.*` | retrieve run |
| **ROC estimate (19a-1)** | `roc_research_observation` + `roc_pct_2026_estimate_minor` | `live_amplify_roc` / `parse_19a1_notice` / `RocResearchRetrieve` |
| `needs_roc_research` | characteristic flag | stays true until owner complete |
| Last price | `price_quote` | Yahoo / retrieve |
| Lots / original cost | `lot` (totals = qty × unit) | `LotOpen` — separate Process B |

## Pack contents (repo-relative)

### Collector / retrieve (Amplify + siblings)

- `crates/import-engine/src/retrieve/adapters/amplify.rs` — HAKY distribution HTML parser
- `crates/import-engine/src/retrieve/adapters/mod.rs` — vendor dispatch
- `crates/import-engine/src/retrieve/adapters/*.rs` — other DIV-1 vendors (context)
- `crates/import-engine/src/retrieve/mod.rs` — **live retrieve, 19a-1 ROC, provider suggest, collect**
- `crates/import-engine/src/retrieve/lookthrough.rs` — holdings / theme extract
- `crates/import-engine/src/retrieve/html.rs`, `edgar.rs`
- `crates/import-engine/src/lib.rs`, `production.rs`
- Fixtures: `amplify_haky.html`, `amplify_qdvo.html`, `amplify_thirteen_paid.html`
- Tests: `crates/import-engine/tests/issuer_fixtures.rs`

### Domain

- `crates/financial-domain/src/roc.rs`
- `crates/financial-domain/src/div1.rs` — provider → declaration_source map (**Amplify → amplify**)
- `crates/financial-domain/src/lifetime.rs`, `declaration_lookback.rs`

### Application / storage

- `crates/application-core/src/queries.rs` — `PositionResearchSeed`, `RocResearchGet/Retrieve`, investment/master
- `crates/application-core/src/contracts.rs` — `LookthroughResearch`, `RocResearch*`, `PositionResearchSeedBody`
- `crates/application-core/src/production_seed.rs` — provider template defaults + ROC seed
- `crates/application-core/src/ports/canonical.rs`
- `crates/storage-sqlite/src/roc_obs.rs`, `canonical.rs`
- Migrations: `0017_roc_research_observation.sql`, `0021_retrieval_page_hash.sql`, `0023_collector_enabled.sql`

### UI / contracts

- `apps/desktop/src/App.tsx` — Add Position research, Validate ROC, PD characteristics
- `apps/desktop/src/financeClient.ts`
- `packages/app-contracts/src/index.ts`
- `packages/ui-components/src/index.tsx` — lots grid (unit vs lot totals)

### Goldens

- `crates/golden-harness/tests/collectors.rs`
- `crates/golden-harness/tests/roc_plan.rs`
- `crates/golden-harness/tests/position_details.rs`
- `crates/golden-harness/tests/new_investment.rs`
- `crates/golden-harness/tests/add_lot.rs`

### Docs + live evidence

- `docs/architecture/haky-pd-audit.md`
- `docs/architecture/pd-slice-a-d-mapping.md`
- `docs/architecture/add-investment-wizard-requirements-for-review.md`
- `live-snapshot/haky_live_db.json`

## Suggested review questions

1. Why did HAKY research leave `provider` / `underlying` / lookthrough blank when `suggestedProvider` and lookthrough extractors exist?
2. Why zero `roc_research_observation` rows after Amplify URL research — is `live_amplify_roc` not called from seed, failing silently, or 19a-1 miss treated correctly as unknown?
3. Does `PositionResearchSeed` persist provider/lookthrough/ROC candidates, or only template + declarations?
4. Is “Validate current ROC estimate” the only path that writes 19a-1, and was it never run for HAKY?
5. Unit vs lot total on Add Lot (fixed in UI + live HAKY lot repair) — confirm PD YOC uses sum of lot **totals** only.
