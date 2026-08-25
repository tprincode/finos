//! Apply a parsed production seed through Canonical ports (no xlsx, no SQL).

use std::collections::HashMap;

use crate::contracts::{
    ActivityRecord, PositionCharacteristicRecord, ProductionSeedCharacteristic,
    ProductionSeedDocument, ProductionSeedLoadBody, RetrievalTemplateRecord, RocResearchObservation,
};
use crate::ports::canonical::Canonical;
use crate::ports::platform::PlatformError;

fn is_disbursement(activity_type: &str) -> bool {
    matches!(
        activity_type,
        "IRA_Distribution" | "Withdrawal" | "Form_1099" | "SSA" | "Roth_Distribution"
    )
}

fn yield_count(activities: &[ActivityRecord]) -> usize {
    activities
        .iter()
        .filter(|a| a.activity_type.eq_ignore_ascii_case("dividend"))
        .count()
}

fn disbursement_count(activities: &[ActivityRecord]) -> usize {
    activities
        .iter()
        .filter(|a| is_disbursement(&a.activity_type))
        .count()
}

pub async fn apply_production_seed(
    canonical: &dyn Canonical,
    doc: ProductionSeedDocument,
) -> Result<ProductionSeedLoadBody, PlatformError> {
    let expected_lots = doc.lots.len();
    let expected_yield: usize = doc
        .yield_batches
        .iter()
        .map(|batch| batch.candidates.len())
        .sum();
    let expected_disb = doc.disbursements.len();

    let existing = canonical.account_list().await?;
    let lots = canonical.basis_get().await?;
    let activities = canonical.activity_list().await?;
    let yield_n = yield_count(&activities);
    let disb_n = disbursement_count(&activities);

    if lots.lots.len() >= expected_lots && yield_n >= expected_yield && disb_n >= expected_disb {
        apply_calculator_seed(canonical, &doc).await?;
        return Ok(ProductionSeedLoadBody {
            already_loaded: true,
            account_count: existing.len() as u64,
            security_count: canonical.security_list().await?.len() as u64,
            lot_count: lots.lots.len() as u64,
        });
    }
    if !lots.lots.is_empty() && lots.lots.len() < expected_lots {
        return Err(PlatformError::new(
            "partial_lots",
            "household lots incomplete; delete local.sqlite and re-run household-seed",
        ));
    }
    let skip_lots = lots.lots.len() >= expected_lots;

    let mut account_ids: HashMap<String, uuid::Uuid> = HashMap::new();
    for account in existing {
        account_ids.insert(account.name.clone(), account.account_id);
    }
    for account in &doc.accounts {
        if account_ids.contains_key(&account.name) {
            continue;
        }
        let record = canonical
            .account_register(account.name.clone(), account.kind.clone())
            .await?;
        account_ids.insert(account.name.clone(), record.account_id);
    }

    let mut security_ids: HashMap<String, uuid::Uuid> = HashMap::new();
    for security in canonical.security_list().await? {
        security_ids.insert(security.symbol.to_ascii_uppercase(), security.security_id);
    }
    for security in &doc.securities {
        let key = security.symbol.to_ascii_uppercase();
        if security_ids.contains_key(&key) {
            continue;
        }
        let record = canonical
            .security_register(security.symbol.clone(), security.name.clone(), security.crf)
            .await?;
        security_ids.insert(key, record.security_id);
    }

    if !skip_lots {
        for lot in &doc.lots {
            let account_id = *account_ids.get(&lot.account_name).ok_or_else(|| {
                PlatformError::new(
                    "unknown_account",
                    format!("lot unknown account {}", lot.account_name),
                )
            })?;
            let security_id = *security_ids
                .get(&lot.symbol.to_ascii_uppercase())
                .ok_or_else(|| {
                    PlatformError::new(
                        "unknown_symbol",
                        format!("lot unknown symbol {}", lot.symbol),
                    )
                })?;
            canonical
                .lot_open(
                    account_id,
                    security_id,
                    lot.opened_on.clone(),
                    lot.origin.clone(),
                    lot.quantity_minor,
                    lot.quantity_scale,
                    lot.performance_basis_minor,
                    lot.tax_basis_minor,
                    lot.scale,
                    None,
                    lot.is_open,
                )
                .await
                .map_err(|e| {
                    PlatformError::new(
                        &e.code,
                        format!(
                            "lot {} {} {}: {}",
                            lot.row_index, lot.account_name, lot.symbol, e.message
                        ),
                    )
                })?;
        }
    }

    for batch in &doc.yield_batches {
        let staged = canonical
            .import_stage(
                batch.source_id.clone(),
                batch.filename.clone(),
                batch.content.as_bytes().to_vec(),
                batch.candidates.clone(),
                None,
            )
            .await?;
        let validated = canonical.import_validate(staged.batch_id).await?;
        if validated.status == "posted" {
            continue;
        }
        if validated.status != "validated" {
            return Err(PlatformError::new(
                "not_validated",
                format!("yield batch {} not validated", batch.source_id),
            ));
        }
        canonical.import_approve(staged.batch_id).await?;
        canonical.import_post(staged.batch_id).await?;
    }

    for row in &doc.disbursements {
        if !account_ids.contains_key(&row.account_name) {
            let record = canonical
                .account_register(row.account_name.clone(), "taxable".into())
                .await?;
            account_ids.insert(row.account_name.clone(), record.account_id);
        }
        let account_id = *account_ids.get(&row.account_name).ok_or_else(|| {
            PlatformError::new(
                "unknown_account",
                format!("disbursement unknown account {}", row.account_name),
            )
        })?;
        canonical
            .activity_post(
                account_id,
                None,
                row.activity_type.clone(),
                Some(row.amount_minor),
                row.scale,
                row.occurred_on.clone(),
                None,
                None,
                Some(row.idempotency_key.clone()),
            )
            .await?;
    }

    let accounts = canonical.account_list().await?;
    let securities = canonical.security_list().await?;
    let lots = canonical.basis_get().await?;
    apply_calculator_seed(canonical, &doc).await?;
    Ok(ProductionSeedLoadBody {
        already_loaded: false,
        account_count: accounts.len() as u64,
        security_count: securities.len() as u64,
        lot_count: lots.lots.len() as u64,
    })
}

async fn apply_calculator_seed(
    canonical: &dyn Canonical,
    doc: &ProductionSeedDocument,
) -> Result<(), PlatformError> {
    let mut security_ids: HashMap<String, uuid::Uuid> = HashMap::new();
    for security in canonical.security_list().await? {
        security_ids.insert(security.symbol.to_ascii_uppercase(), security.security_id);
    }
    for row in &doc.characteristics {
        let Some(security_id) = security_ids.get(&row.symbol.to_ascii_uppercase()).copied() else {
            continue;
        };
        canonical
            .position_characteristic_upsert(PositionCharacteristicRecord {
                security_id,
                payment_frequency: row.payment_frequency.clone(),
                risk_tier: row.risk_tier.clone(),
                provider: row.provider.clone(),
                underlying: row.underlying.clone(),
                roc_pct_2025_actual_minor: row.roc_pct_2025_actual_minor,
                roc_pct_2026_estimate_minor: row.roc_pct_2026_estimate_minor,
                roc_pct_2026_actual_minor: row.roc_pct_2026_actual_minor,
                roc_pct_2024_actual_minor: row.roc_pct_2024_actual_minor,
                roc_scale: row.roc_scale,
                div_type: row.div_type.clone(),
                needs_roc_research: row.needs_roc_research,
                notes: row.notes.clone(),
                is_active: row.is_active,
            })
            .await?;
        seed_template_roc_observations(canonical, security_id, row).await?;
    }
    for row in &doc.plans {
        let Some(security_id) = security_ids.get(&row.symbol.to_ascii_uppercase()).copied() else {
            continue;
        };
        canonical
            .plan_history_record(
                security_id,
                row.amount_per_share_minor,
                row.amount_scale,
                row.planning_periods_per_year,
                row.effective_from.clone(),
                row.decision_reason.clone(),
            )
            .await?;
    }
    apply_private_issue_templates(canonical, &security_ids).await?;
    ensure_btc_usd_split(canonical).await?;
    apply_provider_retrieval_templates(canonical, &security_ids, &doc.characteristics).await?;
    Ok(())
}

async fn seed_template_roc_observations(
    canonical: &dyn Canonical,
    security_id: uuid::Uuid,
    row: &ProductionSeedCharacteristic,
) -> Result<(), PlatformError> {
    let existing = skip_ni(canonical.roc_observation_list(security_id).await)?.unwrap_or_default();
    let mut to_store = Vec::new();
    if let Some(pct) = row.roc_pct_2024_actual_minor {
        to_store.push(("2024", "actual", Some(pct)));
    }
    if let Some(pct) = row.roc_pct_2025_actual_minor {
        to_store.push(("2025", "actual", Some(pct)));
    }
    if let Some(pct) = row.roc_pct_2026_estimate_minor {
        to_store.push(("2026", "estimate", Some(pct)));
    }
    if let Some(pct) = row.roc_pct_2026_actual_minor {
        to_store.push(("2026", "actual", Some(pct)));
    }
    for (tax_year, kind, pct) in to_store {
        let already = existing.iter().any(|o| {
            o.tax_year == tax_year
                && o.kind.eq_ignore_ascii_case(kind)
                && o.source == "template-positions"
        });
        if already {
            continue;
        }
        skip_ni(
            canonical
                .roc_observation_record(RocResearchObservation {
                    observation_id: uuid::Uuid::new_v4(),
                    security_id,
                    roc_pct_minor: pct,
                    scale: row.roc_scale.unwrap_or(2),
                    tax_year: tax_year.into(),
                    source: "template-positions".into(),
                    source_url: String::new(),
                    method: "template".into(),
                    as_of: String::new(),
                    kind: kind.into(),
                    established_how: "Template_Positions.xlsx".into(),
                    owner_override: false,
                    recorded_at: "2026-08-20".into(),
                })
                .await,
        )?;
    }
    Ok(())
}

fn declaration_source_is_fillable(declaration_source: &str, price_source: &str) -> bool {
    if price_source.eq_ignore_ascii_case("edgar") {
        return false;
    }
    matches!(
        declaration_source.trim().to_ascii_lowercase().as_str(),
        "" | "public" | "unassigned"
    )
}

fn provider_retrieval_defaults(provider: &str, frequency: &str) -> (String, String) {
    let p = provider.trim().to_ascii_lowercase();
    let pays = financial_domain::calculator::PaymentCadence::parse(frequency)
        .and_then(financial_domain::calculator::PaymentCadence::periods)
        .unwrap_or(0)
        > 0;
    match p.as_str() {
        "roundhill" => ("roundhill".into(), "derived_walk".into()),
        "amplify" => ("amplify".into(), "issuer_calendar".into()),
        "neos" => ("neos".into(), "issuer_calendar".into()),
        "yieldmax" | "yield max" => ("yieldmax".into(), "derived_walk".into()),
        _ if !pays => ("unassigned".into(), "none".into()),
        _ => ("unassigned".into(), "derived_walk".into()),
    }
}

fn fill_retrieval_template(
    security_id: uuid::Uuid,
    existing: Option<&RetrievalTemplateRecord>,
    declaration_source: String,
    calendar_policy: String,
    fallback_symbol: &str,
) -> RetrievalTemplateRecord {
    RetrievalTemplateRecord {
        security_id,
        price_source: existing
            .map(|e| e.price_source.clone())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "public".into()),
        source_symbol: existing
            .map(|e| e.source_symbol.clone())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| fallback_symbol.to_string()),
        declaration_source,
        lookback_count: 12,
        payment_source: existing
            .map(|e| e.payment_source.clone())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "import".into()),
        source_url: existing.map(|e| e.source_url.clone()).unwrap_or_default(),
        calendar_policy,
        last_run_at: String::new(),
        last_run_ok: None,
        last_run_message: String::new(),
        last_content_hash: String::new(),
    }
}

async fn apply_provider_retrieval_templates(
    canonical: &dyn Canonical,
    security_ids: &HashMap<String, uuid::Uuid>,
    characteristics: &[ProductionSeedCharacteristic],
) -> Result<(), PlatformError> {
    for row in characteristics {
        let Some(security_id) = security_ids.get(&row.symbol.to_ascii_uppercase()).copied() else {
            continue;
        };
        let sym = row.symbol.to_ascii_uppercase();
        if matches!(sym.as_str(), "ENERGYX" | "BTC" | "BTC-USD") {
            continue;
        }
        let existing = skip_ni(canonical.retrieval_template_get(security_id).await)?.flatten();
        if existing
            .as_ref()
            .map(|e| !declaration_source_is_fillable(&e.declaration_source, &e.price_source))
            .unwrap_or(false)
        {
            continue;
        }
        let (declaration_source, calendar_policy) =
            provider_retrieval_defaults(&row.provider, &row.payment_frequency);
        skip_ni(
            canonical
                .retrieval_template_set(fill_retrieval_template(
                    security_id,
                    existing.as_ref(),
                    declaration_source,
                    calendar_policy,
                    &row.symbol,
                ))
                .await,
        )?;
    }
    Ok(())
}

/// Fill empty/public/unassigned declaration sources from live characteristics.
/// Never overwrites edgar, sec-edgar, or a registered vendor adapter.
pub async fn apply_provider_declaration_sources(
    canonical: &dyn Canonical,
) -> Result<u64, PlatformError> {
    let securities = canonical.security_list().await?;
    let chars = canonical.position_characteristic_list().await?;
    let char_by: HashMap<_, _> = chars.iter().map(|c| (c.security_id, c)).collect();
    let mut updated = 0u64;
    for security in securities {
        let sym = security.symbol.to_ascii_uppercase();
        if matches!(sym.as_str(), "ENERGYX" | "BTC" | "BTC-USD") {
            continue;
        }
        let Some(ch) = char_by.get(&security.security_id) else {
            continue;
        };
        let existing = skip_ni(canonical.retrieval_template_get(security.security_id).await)?
            .flatten();
        if existing
            .as_ref()
            .map(|e| !declaration_source_is_fillable(&e.declaration_source, &e.price_source))
            .unwrap_or(false)
        {
            continue;
        }
        let (declaration_source, calendar_policy) =
            provider_retrieval_defaults(&ch.provider, &ch.payment_frequency);
        canonical
            .retrieval_template_set(fill_retrieval_template(
                security.security_id,
                existing.as_ref(),
                declaration_source,
                calendar_policy,
                &security.symbol,
            ))
            .await?;
        updated += 1;
    }
    Ok(updated)
}

async fn apply_private_issue_templates(
    canonical: &dyn Canonical,
    security_ids: &HashMap<String, uuid::Uuid>,
) -> Result<(), PlatformError> {
    let Some(security_id) = security_ids.get("ENERGYX").copied() else {
        return Ok(());
    };
    canonical
        .retrieval_template_set(RetrievalTemplateRecord {
            security_id,
            price_source: "edgar".into(),
            source_symbol: "1830166".into(),
            declaration_source: "sec-edgar".into(),
            lookback_count: 1,
            payment_source: "import".into(),
            source_url: String::new(),
            calendar_policy: "none".into(),
            last_run_at: String::new(),
            last_run_ok: None,
            last_run_message: String::new(),
            last_content_hash: String::new(),
        })
        .await?;
    Ok(())
}

fn skip_ni<T>(result: Result<T, PlatformError>) -> Result<Option<T>, PlatformError> {
    match result {
        Ok(v) => Ok(Some(v)),
        Err(e) if e.code == "not_implemented" => Ok(None),
        Err(e) => Err(e),
    }
}

/// Grayscale BTC (~$34) and Robinhood crypto BTC-USD are different securities.
/// Idempotent so already-loaded households split without reopening lots.
pub async fn ensure_btc_usd_split(canonical: &dyn Canonical) -> Result<(), PlatformError> {
    const CRYPTO_USD_CENTS: i64 = 100_000;
    let securities = canonical.security_list().await?;
    let Some(btc) = securities
        .iter()
        .find(|s| s.symbol.eq_ignore_ascii_case("BTC"))
        .cloned()
    else {
        return Ok(());
    };
    let accounts = canonical.account_list().await?;
    let robinhood = accounts
        .iter()
        .find(|a| a.name.eq_ignore_ascii_case("Robinhood"))
        .cloned();
    let lots = canonical.basis_get().await?;
    let rh_btc_lots: Vec<_> = match &robinhood {
        Some(rh) => lots
            .lots
            .iter()
            .filter(|l| l.account_id == rh.account_id && l.security_id == btc.security_id)
            .cloned()
            .collect(),
        None => Vec::new(),
    };
    let mut btc_usd_id = securities
        .iter()
        .find(|s| s.symbol.eq_ignore_ascii_case("BTC-USD"))
        .map(|s| s.security_id);
    if !rh_btc_lots.is_empty() && btc_usd_id.is_none() {
        let created = canonical
            .security_register("BTC-USD".into(), "Bitcoin".into(), false)
            .await?;
        btc_usd_id = Some(created.security_id);
    }
    if let Some(usd_id) = btc_usd_id {
        for lot in &rh_btc_lots {
            canonical
                .lot_reassign_security(lot.lot_id, usd_id)
                .await?;
        }
        if let Some(rh) = &robinhood {
            for activity in canonical.activity_list().await? {
                if activity.account_id == rh.account_id
                    && activity.security_id == Some(btc.security_id)
                {
                    canonical
                        .activity_reassign_security(activity.activity_id, usd_id)
                        .await?;
                }
            }
        }
    }
    let quotes = match skip_ni(canonical.price_quote_list(btc.security_id).await)? {
        Some(q) => q,
        None => Vec::new(),
    };
    let usd_quotes = match btc_usd_id {
        Some(usd_id) => skip_ni(canonical.price_quote_list(usd_id).await)?.unwrap_or_default(),
        None => Vec::new(),
    };
    for quote in quotes {
        if quote.validation_status != "accepted" {
            continue;
        }
        let cents = financial_domain::money::to_usd_cents(quote.price_minor, quote.scale);
        if cents < CRYPTO_USD_CENTS {
            continue;
        }
        if let Some(usd_id) = btc_usd_id {
            let already = usd_quotes.iter().any(|u| {
                u.as_of_at == quote.as_of_at && u.price_minor == quote.price_minor
            });
            if !already {
                let _ = skip_ni(
                    canonical
                        .price_quote_record(
                            usd_id,
                            quote.price_minor,
                            quote.scale,
                            quote.as_of_at.clone(),
                            quote.source.clone(),
                        )
                        .await,
                )?;
            }
        }
        let _ = skip_ni(canonical.price_quote_reject(quote.price_quote_id).await)?;
    }
    set_public_quote_template(canonical, btc.security_id, "BTC").await?;
    if let Some(usd_id) = btc_usd_id {
        set_public_quote_template(canonical, usd_id, "BTC-USD").await?;
    }
    Ok(())
}

async fn set_public_quote_template(
    canonical: &dyn Canonical,
    security_id: uuid::Uuid,
    source_symbol: &str,
) -> Result<(), PlatformError> {
    let existing = skip_ni(canonical.retrieval_template_get(security_id).await)?;
    let (declaration_source, lookback_count, payment_source, source_url, calendar_policy) =
        match existing.flatten() {
            Some(row) => (
                row.declaration_source,
                row.lookback_count,
                row.payment_source,
                row.source_url,
                if row.calendar_policy.is_empty() {
                    "none".into()
                } else {
                    row.calendar_policy
                },
            ),
            None => (String::new(), 12, String::new(), String::new(), "none".into()),
        };
    skip_ni(
        canonical
            .retrieval_template_set(RetrievalTemplateRecord {
                security_id,
                price_source: "public".into(),
                source_symbol: source_symbol.into(),
                declaration_source,
                lookback_count,
                payment_source,
                source_url,
                calendar_policy,
                last_run_at: String::new(),
                last_run_ok: None,
                last_run_message: String::new(),
                last_content_hash: String::new(),
            })
            .await,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{declaration_source_is_fillable, provider_retrieval_defaults};

    #[test]
    fn fillable_skips_registered_and_edgar() {
        assert!(declaration_source_is_fillable("", "public"));
        assert!(declaration_source_is_fillable("public", "public"));
        assert!(declaration_source_is_fillable("unassigned", "public"));
        assert!(!declaration_source_is_fillable("amplify", "public"));
        assert!(!declaration_source_is_fillable("public", "edgar"));
        assert!(!declaration_source_is_fillable("sec-edgar", "edgar"));
        assert!(!declaration_source_is_fillable("roundhill", "public"));
    }

    #[test]
    fn provider_map_tags_tier1_and_leaves_tier2_unassigned() {
        assert_eq!(
            provider_retrieval_defaults("Amplify", "Monthly"),
            ("amplify".into(), "issuer_calendar".into())
        );
        assert_eq!(
            provider_retrieval_defaults("NEOS", "Monthly"),
            ("neos".into(), "issuer_calendar".into())
        );
        assert_eq!(
            provider_retrieval_defaults("YieldMax", "Weekly"),
            ("yieldmax".into(), "derived_walk".into())
        );
        assert_eq!(
            provider_retrieval_defaults("Roundhill", "Weekly"),
            ("roundhill".into(), "derived_walk".into())
        );
        assert_eq!(
            provider_retrieval_defaults("Global X", "Monthly"),
            ("unassigned".into(), "derived_walk".into())
        );
        assert_eq!(
            provider_retrieval_defaults("Tesla", "None"),
            ("unassigned".into(), "none".into())
        );
        assert_eq!(
            provider_retrieval_defaults("", "None"),
            ("unassigned".into(), "none".into())
        );
    }
}
