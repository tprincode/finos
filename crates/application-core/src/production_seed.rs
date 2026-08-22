//! Apply a parsed production seed through Canonical ports (no xlsx, no SQL).

use std::collections::HashMap;

use crate::contracts::{
    ActivityRecord, PositionCharacteristicRecord, ProductionSeedDocument, ProductionSeedLoadBody,
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
            })
            .await?;
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
    Ok(())
}
