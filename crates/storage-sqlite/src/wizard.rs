//! Last Price quotes, issuer declarations, retrieval template (schema 15).

use application_core::contracts::{
    CollectorSetBody, CollectorSetItem, CollectorStatsBody, CurrentPriceBody,
    IssuerDeclarationRecord, PriceQuoteBody, PriceRetrievalItem, PriceRetrievalSetBody,
    RetrievalTemplateRecord, RetrieveRunRecord,
};
use application_core::ports::platform::PlatformError;
use financial_domain::current_price::{
    cash_par_current_price, price_derived_valid, select_current_price, uses_cash_par,
    OverrideObservation, QuoteObservation,
};
use financial_domain::error::DomainError;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::store::StorageError;

fn map_err(err: StorageError) -> PlatformError {
    PlatformError::new("storage_error", err.to_string())
}

fn domain_err(err: DomainError) -> PlatformError {
    let code = match err {
        DomainError::MissingDeclarationSource => "missing_declaration_source",
        DomainError::NonpositivePrice => "nonpositive_price",
        _ => "domain_error",
    };
    PlatformError::new(code, err.to_string())
}

pub async fn issuer_declaration_record(
    pool: &SqlitePool,
    security_id: Uuid,
    amount_per_share_minor: Option<i64>,
    amount_scale: u8,
    payment_period: String,
    source: String,
    entered_at: String,
) -> Result<IssuerDeclarationRecord, PlatformError> {
    if source.trim().is_empty() {
        return Err(domain_err(DomainError::MissingDeclarationSource));
    }
    issuer_declaration_record_with_policy(
        pool,
        security_id,
        amount_per_share_minor,
        amount_scale,
        payment_period,
        source,
        entered_at,
        false,
    )
    .await
}

pub async fn issuer_declaration_record_with_policy(
    pool: &SqlitePool,
    security_id: Uuid,
    amount_per_share_minor: Option<i64>,
    amount_scale: u8,
    payment_period: String,
    source: String,
    entered_at: String,
    replace_locked_paid: bool,
) -> Result<IssuerDeclarationRecord, PlatformError> {
    if source.trim().is_empty() {
        return Err(domain_err(DomainError::MissingDeclarationSource));
    }
    if let Some(existing_id) = sqlx::query_scalar::<_, String>(
        "SELECT declaration_id FROM issuer_declaration
         WHERE security_id = ? AND payment_period = ? AND superseded_by IS NULL
         ORDER BY entered_at DESC
         LIMIT 1",
    )
    .bind(security_id.to_string())
    .bind(&payment_period)
    .fetch_optional(pool)
    .await
    .map_err(|e| map_err(e.into()))?
    {
        let rows = sqlx::query(
            "SELECT declaration_id, security_id, amount_per_share_minor, amount_scale,
                    payment_period, source, entered_at
             FROM issuer_declaration WHERE declaration_id = ?",
        )
        .bind(&existing_id)
        .fetch_all(pool)
        .await
        .map_err(|e| map_err(e.into()))?;
        if let Some(row) = rows.first() {
            let existing = IssuerDeclarationRecord {
                declaration_id: parse_uuid(row, "declaration_id")?,
                security_id: parse_uuid(row, "security_id")?,
                amount_per_share_minor: row
                    .try_get("amount_per_share_minor")
                    .map_err(|e| map_err(e.into()))?,
                amount_scale: row.try_get::<i64, _>("amount_scale").map_err(|e| map_err(e.into()))?
                    as u8,
                payment_period: row.try_get("payment_period").map_err(|e| map_err(e.into()))?,
                source: row.try_get("source").map_err(|e| map_err(e.into()))?,
                entered_at: row.try_get("entered_at").map_err(|e| map_err(e.into()))?,
            };
            let existing_paid = existing.amount_per_share_minor.unwrap_or(0) > 0;
            let owner_lock = matches!(
                existing.source.trim().to_ascii_lowercase().as_str(),
                "import" | "owner" | "manual"
            );
            if existing_paid && owner_lock && !replace_locked_paid {
                // Stored paid rows are facts. Collect/Establish must not overwrite them.
                return Ok(existing);
            }
            let changed = existing.amount_per_share_minor != amount_per_share_minor
                || existing.amount_scale != amount_scale
                || existing.source != source;
            if changed {
                let new_id = Uuid::new_v4();
                sqlx::query(
                    "UPDATE issuer_declaration SET superseded_by = ? WHERE declaration_id = ?",
                )
                .bind(new_id.to_string())
                .bind(existing.declaration_id.to_string())
                .execute(pool)
                .await
                .map_err(|e| map_err(e.into()))?;
                let record = IssuerDeclarationRecord {
                    declaration_id: new_id,
                    security_id,
                    amount_per_share_minor,
                    amount_scale,
                    payment_period: payment_period.clone(),
                    source: source.clone(),
                    entered_at: entered_at.clone(),
                };
                sqlx::query(
                    "INSERT INTO issuer_declaration (
                        declaration_id, security_id, amount_per_share_minor, amount_scale,
                        payment_period, source, entered_at, superseded_by
                     ) VALUES (?, ?, ?, ?, ?, ?, ?, NULL)",
                )
                .bind(record.declaration_id.to_string())
                .bind(security_id.to_string())
                .bind(amount_per_share_minor)
                .bind(amount_scale as i64)
                .bind(&payment_period)
                .bind(&source)
                .bind(&entered_at)
                .execute(pool)
                .await
                .map_err(|e| map_err(e.into()))?;
                return Ok(record);
            }
            return Ok(existing);
        }
    }
    let record = IssuerDeclarationRecord {
        declaration_id: Uuid::new_v4(),
        security_id,
        amount_per_share_minor,
        amount_scale,
        payment_period: payment_period.clone(),
        source: source.clone(),
        entered_at: entered_at.clone(),
    };
    sqlx::query(
        "INSERT INTO issuer_declaration (
            declaration_id, security_id, amount_per_share_minor, amount_scale,
            payment_period, source, entered_at, superseded_by
         ) VALUES (?, ?, ?, ?, ?, ?, ?, NULL)",
    )
    .bind(record.declaration_id.to_string())
    .bind(security_id.to_string())
    .bind(amount_per_share_minor)
    .bind(amount_scale as i64)
    .bind(&payment_period)
    .bind(&source)
    .bind(&entered_at)
    .execute(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    Ok(record)
}

pub async fn issuer_declaration_supersede_period(
    pool: &SqlitePool,
    security_id: Uuid,
    payment_period: String,
) -> Result<u64, PlatformError> {
    let marker = Uuid::new_v4().to_string();
    let res = sqlx::query(
        "UPDATE issuer_declaration SET superseded_by = ?
         WHERE security_id = ? AND payment_period = ? AND superseded_by IS NULL",
    )
    .bind(marker)
    .bind(security_id.to_string())
    .bind(&payment_period)
    .execute(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    Ok(res.rows_affected())
}

pub async fn issuer_declaration_list(
    pool: &SqlitePool,
    security_id: Uuid,
) -> Result<Vec<IssuerDeclarationRecord>, PlatformError> {
    let rows = sqlx::query(
        "SELECT declaration_id, security_id, amount_per_share_minor, amount_scale,
                payment_period, source, entered_at
         FROM issuer_declaration
         WHERE security_id = ? AND superseded_by IS NULL
         ORDER BY payment_period DESC",
    )
    .bind(security_id.to_string())
    .fetch_all(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    rows.iter()
        .map(|row| {
            Ok(IssuerDeclarationRecord {
                declaration_id: parse_uuid(row, "declaration_id")?,
                security_id: parse_uuid(row, "security_id")?,
                amount_per_share_minor: row
                    .try_get("amount_per_share_minor")
                    .map_err(|e| map_err(e.into()))?,
                amount_scale: row.try_get::<i64, _>("amount_scale").map_err(|e| map_err(e.into()))?
                    as u8,
                payment_period: row.try_get("payment_period").map_err(|e| map_err(e.into()))?,
                source: row.try_get("source").map_err(|e| map_err(e.into()))?,
                entered_at: row.try_get("entered_at").map_err(|e| map_err(e.into()))?,
            })
        })
        .collect()
}

pub async fn price_quote_record(
    pool: &SqlitePool,
    security_id: Uuid,
    price_minor: i64,
    scale: u8,
    as_of_at: String,
    source: String,
) -> Result<PriceQuoteBody, PlatformError> {
    if price_minor <= 0 {
        return Err(domain_err(DomainError::NonpositivePrice));
    }
    // Do not stack duplicate quotes for the same as-of + source.
    if let Some(existing_id) = sqlx::query_scalar::<_, String>(
        "SELECT price_quote_id FROM price_quote
         WHERE security_id = ? AND as_of_at = ? AND source = ?
           AND validation_status = 'accepted'
         ORDER BY retrieved_at DESC
         LIMIT 1",
    )
    .bind(security_id.to_string())
    .bind(&as_of_at)
    .bind(&source)
    .fetch_optional(pool)
    .await
    .map_err(|e| map_err(e.into()))?
    {
        let row = sqlx::query(
            "SELECT price_quote_id, security_id, price_minor, scale, as_of_at, source, validation_status
             FROM price_quote WHERE price_quote_id = ?",
        )
        .bind(&existing_id)
        .fetch_one(pool)
        .await
        .map_err(|e| map_err(e.into()))?;
        return Ok(PriceQuoteBody {
            price_quote_id: parse_uuid(&row, "price_quote_id")?,
            security_id: parse_uuid(&row, "security_id")?,
            price_minor: row.try_get("price_minor").map_err(|e| map_err(e.into()))?,
            scale: row.try_get::<i64, _>("scale").map_err(|e| map_err(e.into()))? as u8,
            as_of_at: row.try_get("as_of_at").map_err(|e| map_err(e.into()))?,
            source: row.try_get("source").map_err(|e| map_err(e.into()))?,
            validation_status: row
                .try_get("validation_status")
                .map_err(|e| map_err(e.into()))?,
        });
    }
    let record = PriceQuoteBody {
        price_quote_id: Uuid::new_v4(),
        security_id,
        price_minor,
        scale,
        as_of_at: as_of_at.clone(),
        source: source.clone(),
        validation_status: "accepted".into(),
    };
    sqlx::query(
        "INSERT INTO price_quote (
            price_quote_id, security_id, price_minor, scale, currency, quote_type,
            as_of_at, retrieved_at, source, validation_status
         ) VALUES (?, ?, ?, ?, 'USD', 'last', ?, ?, ?, 'accepted')",
    )
    .bind(record.price_quote_id.to_string())
    .bind(security_id.to_string())
    .bind(price_minor)
    .bind(scale as i64)
    .bind(&as_of_at)
    .bind(&as_of_at)
    .bind(&source)
    .execute(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    Ok(record)
}

pub async fn price_quote_list(
    pool: &SqlitePool,
    security_id: Uuid,
) -> Result<Vec<PriceQuoteBody>, PlatformError> {
    let rows = sqlx::query(
        "SELECT price_quote_id, security_id, price_minor, scale, as_of_at, source, validation_status
         FROM price_quote WHERE security_id = ? ORDER BY as_of_at DESC",
    )
    .bind(security_id.to_string())
    .fetch_all(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    rows.iter()
        .map(|row| {
            Ok(PriceQuoteBody {
                price_quote_id: parse_uuid(row, "price_quote_id")?,
                security_id: parse_uuid(row, "security_id")?,
                price_minor: row.try_get("price_minor").map_err(|e| map_err(e.into()))?,
                scale: row.try_get::<i64, _>("scale").map_err(|e| map_err(e.into()))? as u8,
                as_of_at: row.try_get("as_of_at").map_err(|e| map_err(e.into()))?,
                source: row.try_get("source").map_err(|e| map_err(e.into()))?,
                validation_status: row
                    .try_get("validation_status")
                    .map_err(|e| map_err(e.into()))?,
            })
        })
        .collect()
}

pub async fn price_quote_reject(
    pool: &SqlitePool,
    price_quote_id: Uuid,
) -> Result<PriceQuoteBody, PlatformError> {
    let n = sqlx::query("UPDATE price_quote SET validation_status = 'rejected' WHERE price_quote_id = ?")
        .bind(price_quote_id.to_string())
        .execute(pool)
        .await
        .map_err(|e| map_err(e.into()))?
        .rows_affected();
    if n == 0 {
        return Err(PlatformError::new("not_found", "price quote not found"));
    }
    let row = sqlx::query(
        "SELECT price_quote_id, security_id, price_minor, scale, as_of_at, source, validation_status
         FROM price_quote WHERE price_quote_id = ?",
    )
    .bind(price_quote_id.to_string())
    .fetch_one(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    Ok(PriceQuoteBody {
        price_quote_id: parse_uuid(&row, "price_quote_id")?,
        security_id: parse_uuid(&row, "security_id")?,
        price_minor: row.try_get("price_minor").map_err(|e| map_err(e.into()))?,
        scale: row.try_get::<i64, _>("scale").map_err(|e| map_err(e.into()))? as u8,
        as_of_at: row.try_get("as_of_at").map_err(|e| map_err(e.into()))?,
        source: row.try_get("source").map_err(|e| map_err(e.into()))?,
        validation_status: row
            .try_get("validation_status")
            .map_err(|e| map_err(e.into()))?,
    })
}

pub async fn manual_price_override(
    pool: &SqlitePool,
    security_id: Uuid,
    price_minor: i64,
    scale: u8,
    reason: String,
    effective_from: String,
) -> Result<CurrentPriceBody, PlatformError> {
    if price_minor <= 0 {
        return Err(domain_err(DomainError::NonpositivePrice));
    }
    sqlx::query("UPDATE manual_price_override SET status = 'superseded' WHERE security_id = ? AND status = 'active'")
        .bind(security_id.to_string())
        .execute(pool)
        .await
        .map_err(|e| map_err(e.into()))?;
    sqlx::query(
        "INSERT INTO manual_price_override (
            override_id, security_id, price_minor, scale, currency, reason,
            effective_from, expires_at, status
         ) VALUES (?, ?, ?, ?, 'USD', ?, ?, NULL, 'active')",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(security_id.to_string())
    .bind(price_minor)
    .bind(scale as i64)
    .bind(&reason)
    .bind(&effective_from)
    .execute(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    current_price_get(pool, security_id, effective_from).await
}

pub async fn current_price_get(
    pool: &SqlitePool,
    security_id: Uuid,
    as_of_date: String,
) -> Result<CurrentPriceBody, PlatformError> {
    let meta = sqlx::query(
        "SELECT COALESCE(s.symbol, '') AS symbol,
                COALESCE(pc.div_type, '') AS div_type
         FROM security s
         LEFT JOIN position_characteristic pc ON pc.security_id = s.security_id
         WHERE s.security_id = ?",
    )
    .bind(security_id.to_string())
    .fetch_optional(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    if let Some(row) = meta {
        let symbol: String = row.try_get("symbol").unwrap_or_default();
        let div_type: String = row.try_get("div_type").unwrap_or_default();
        if uses_cash_par(&div_type, &symbol) {
            let selected = cash_par_current_price();
            return Ok(CurrentPriceBody {
                security_id,
                price_minor: selected.price_minor,
                scale: selected.scale,
                freshness: "current".into(),
                price_derived_valid: true,
                as_of_at: Some(as_of_date),
            });
        }
    }
    let over_row = sqlx::query(
        "SELECT price_minor, scale, status, effective_from FROM manual_price_override
         WHERE security_id = ? AND status = 'active' ORDER BY effective_from DESC LIMIT 1",
    )
    .bind(security_id.to_string())
    .fetch_optional(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    let over_effective_from = over_row
        .as_ref()
        .and_then(|row| row.try_get::<String, _>("effective_from").ok());
    let over = over_row
        .as_ref()
        .map(|row| {
            Ok::<_, PlatformError>(OverrideObservation {
                price_minor: row.try_get("price_minor").map_err(|e| map_err(e.into()))?,
                scale: row.try_get::<i64, _>("scale").map_err(|e| map_err(e.into()))? as u8,
                active: true,
            })
        })
        .transpose()?;
    let quotes = price_quote_list(pool, security_id).await?;
    let obs: Vec<QuoteObservation> = quotes
        .into_iter()
        .map(|q| QuoteObservation {
            price_minor: q.price_minor,
            scale: q.scale,
            accepted: q.validation_status == "accepted",
            as_of: q.as_of_at,
        })
        .collect();
    let selected = select_current_price(over.as_ref(), &obs, &as_of_date);
    let as_of_at = match selected.freshness {
        financial_domain::current_price::PriceFreshness::ManualOverride => over_effective_from,
        financial_domain::current_price::PriceFreshness::Current
        | financial_domain::current_price::PriceFreshness::Stale => selected.price_minor.and_then(
            |minor| {
                obs.iter()
                    .find(|q| q.accepted && q.price_minor == minor)
                    .map(|q| q.as_of.clone())
            },
        ),
        financial_domain::current_price::PriceFreshness::Unavailable => None,
    };
    Ok(CurrentPriceBody {
        security_id,
        price_minor: selected.price_minor,
        scale: selected.scale,
        freshness: match selected.freshness {
            financial_domain::current_price::PriceFreshness::Current => "current".into(),
            financial_domain::current_price::PriceFreshness::ManualOverride => {
                "manual_override".into()
            }
            financial_domain::current_price::PriceFreshness::Stale => "stale".into(),
            financial_domain::current_price::PriceFreshness::Unavailable => "unavailable".into(),
        },
        price_derived_valid: price_derived_valid(selected.freshness),
        as_of_at,
    })
}

pub async fn retrieval_template_set(
    pool: &SqlitePool,
    record: RetrievalTemplateRecord,
) -> Result<RetrievalTemplateRecord, PlatformError> {
    sqlx::query(
        "INSERT INTO retrieval_template (
            security_id, price_source, source_symbol, declaration_source, lookback_count,
            payment_source, source_url, calendar_policy, last_run_at, last_run_ok, last_run_message,
            last_content_hash, collector_enabled, inception_on, roc_source_url, history_url_attempts
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(security_id) DO UPDATE SET
            price_source = excluded.price_source,
            source_symbol = excluded.source_symbol,
            declaration_source = excluded.declaration_source,
            lookback_count = excluded.lookback_count,
            payment_source = excluded.payment_source,
            source_url = excluded.source_url,
            calendar_policy = excluded.calendar_policy,
            last_run_at = CASE WHEN excluded.last_run_at = '' THEN retrieval_template.last_run_at ELSE excluded.last_run_at END,
            last_run_ok = CASE WHEN excluded.last_run_at = '' THEN retrieval_template.last_run_ok ELSE excluded.last_run_ok END,
            last_run_message = CASE WHEN excluded.last_run_at = '' THEN retrieval_template.last_run_message ELSE excluded.last_run_message END,
            last_content_hash = CASE WHEN excluded.last_content_hash = '' THEN retrieval_template.last_content_hash ELSE excluded.last_content_hash END,
            collector_enabled = excluded.collector_enabled,
            inception_on = excluded.inception_on,
            roc_source_url = excluded.roc_source_url,
            history_url_attempts = excluded.history_url_attempts",
    )
    .bind(record.security_id.to_string())
    .bind(&record.price_source)
    .bind(&record.source_symbol)
    .bind(&record.declaration_source)
    .bind(record.lookback_count as i64)
    .bind(&record.payment_source)
    .bind(&record.source_url)
    .bind(&record.calendar_policy)
    .bind(&record.last_run_at)
    .bind(record.last_run_ok.map(|ok| if ok { 1i64 } else { 0 }))
    .bind(&record.last_run_message)
    .bind(&record.last_content_hash)
    .bind(if record.collector_enabled { 1i64 } else { 0 })
    .bind(&record.inception_on)
    .bind(&record.roc_source_url)
    .bind(i64::from(record.history_url_attempts))
    .execute(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    Ok(record)
}

pub async fn retrieval_template_get(
    pool: &SqlitePool,
    security_id: Uuid,
) -> Result<Option<RetrievalTemplateRecord>, PlatformError> {
    let row = sqlx::query(
        "SELECT security_id, price_source, source_symbol, declaration_source, lookback_count,
                payment_source, source_url, calendar_policy, last_run_at, last_run_ok, last_run_message,
                last_content_hash,                 COALESCE(collector_enabled, 0) AS collector_enabled,
                COALESCE(inception_on, '') AS inception_on,
                COALESCE(roc_source_url, '') AS roc_source_url,
                COALESCE(history_url_attempts, 0) AS history_url_attempts
         FROM retrieval_template WHERE security_id = ?",
    )
    .bind(security_id.to_string())
    .fetch_optional(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    row.map(|row| {
        Ok(RetrievalTemplateRecord {
            security_id: parse_uuid(&row, "security_id")?,
            price_source: row.try_get("price_source").map_err(|e| map_err(e.into()))?,
            source_symbol: row.try_get("source_symbol").map_err(|e| map_err(e.into()))?,
            declaration_source: row
                .try_get("declaration_source")
                .map_err(|e| map_err(e.into()))?,
            lookback_count: row
                .try_get::<i64, _>("lookback_count")
                .map_err(|e| map_err(e.into()))? as u8,
            payment_source: row.try_get("payment_source").map_err(|e| map_err(e.into()))?,
            source_url: row.try_get("source_url").map_err(|e| map_err(e.into()))?,
            calendar_policy: row.try_get("calendar_policy").map_err(|e| map_err(e.into()))?,
            last_run_at: row.try_get("last_run_at").map_err(|e| map_err(e.into()))?,
            last_run_ok: row
                .try_get::<Option<i64>, _>("last_run_ok")
                .map_err(|e| map_err(e.into()))?
                .map(|n| n != 0),
            last_run_message: row.try_get("last_run_message").map_err(|e| map_err(e.into()))?,
            last_content_hash: row
                .try_get("last_content_hash")
                .unwrap_or_else(|_| String::new()),
            collector_enabled: row
                .try_get::<i64, _>("collector_enabled")
                .unwrap_or(0)
                != 0,
            inception_on: row.try_get("inception_on").unwrap_or_default(),
            roc_source_url: row.try_get("roc_source_url").unwrap_or_default(),
            history_url_attempts: row
                .try_get::<i64, _>("history_url_attempts")
                .unwrap_or(0) as u8,
        })
    })
    .transpose()
}

pub async fn retrieval_template_touch_run(
    pool: &SqlitePool,
    security_id: Uuid,
    ok: bool,
    message: String,
    ran_at: String,
    content_hash: String,
    source_url: &str,
) -> Result<(), PlatformError> {
    sqlx::query(
        "UPDATE retrieval_template
         SET last_run_at = ?, last_run_ok = ?, last_run_message = ?,
             last_content_hash = CASE WHEN ? = '' THEN last_content_hash ELSE ? END,
             source_url = CASE WHEN ? = '' THEN source_url ELSE ? END
         WHERE security_id = ?",
    )
    .bind(&ran_at)
    .bind(if ok { 1i64 } else { 0 })
    .bind(&message)
    .bind(&content_hash)
    .bind(&content_hash)
    .bind(source_url)
    .bind(source_url)
    .bind(security_id.to_string())
    .execute(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    Ok(())
}

pub async fn price_retrieval_set(pool: &SqlitePool) -> Result<PriceRetrievalSetBody, PlatformError> {
    let rows = sqlx::query(
        "SELECT DISTINCT lot.security_id, s.symbol,
                COALESCE(rt.price_source, '') AS price_source,
                COALESCE(rt.source_symbol, '') AS source_symbol,
                COALESCE(rt.declaration_source, '') AS declaration_source,
                COALESCE(rt.source_url, '') AS source_url,
                COALESCE(rt.calendar_policy, '') AS calendar_policy,
                COALESCE(rt.last_content_hash, '') AS last_content_hash,
                COALESCE(pc.div_type, '') AS div_type,
                COALESCE(rt.collector_enabled, 0) AS collector_enabled
         FROM lot
         JOIN security s ON s.security_id = lot.security_id
         LEFT JOIN retrieval_template rt ON rt.security_id = lot.security_id
         LEFT JOIN position_characteristic pc ON pc.security_id = lot.security_id
         WHERE lot.remaining_quantity_minor > 0
           AND COALESCE(pc.is_active, 1) = 1
         ORDER BY s.symbol",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    let mut security_ids = Vec::new();
    let mut items = Vec::new();
    for row in rows {
        let security_id = parse_uuid(&row, "security_id")?;
        let symbol: String = row.try_get("symbol").map_err(|e| map_err(e.into()))?;
        let price_source: String = row.try_get("price_source").map_err(|e| map_err(e.into()))?;
        let source_symbol: String = row.try_get("source_symbol").map_err(|e| map_err(e.into()))?;
        let declaration_source: String =
            row.try_get("declaration_source").map_err(|e| map_err(e.into()))?;
        let source_url: String = row.try_get("source_url").map_err(|e| map_err(e.into()))?;
        let calendar_policy: String = row.try_get("calendar_policy").map_err(|e| map_err(e.into()))?;
        let last_content_hash: String = row
            .try_get("last_content_hash")
            .unwrap_or_else(|_| String::new());
        let div_type: String = row.try_get("div_type").unwrap_or_else(|_| String::new());
        let collector_enabled: i64 = row.try_get("collector_enabled").unwrap_or(0);
        if uses_cash_par(&div_type, &symbol) {
            continue;
        }
        security_ids.push(security_id);
        items.push(PriceRetrievalItem {
            security_id,
            symbol,
            price_source,
            source_symbol,
            declaration_source,
            source_url,
            calendar_policy,
            last_content_hash,
            div_type,
            collector_enabled: collector_enabled != 0,
        });
    }
    Ok(PriceRetrievalSetBody {
        security_ids,
        items,
    })
}

fn parse_uuid(row: &sqlx::sqlite::SqliteRow, col: &str) -> Result<Uuid, PlatformError> {
    let raw: String = row.try_get(col).map_err(|e| map_err(e.into()))?;
    Uuid::parse_str(&raw).map_err(|e| PlatformError::new("parse_error", e.to_string()))
}

pub async fn retrieve_run_record(
    pool: &SqlitePool,
    record: RetrieveRunRecord,
) -> Result<RetrieveRunRecord, PlatformError> {
    sqlx::query(
        "INSERT INTO retrieve_run (
            run_id, security_id, kind, requested_at, ok, code, message,
            attempted, recorded, skipped, unchanged, payload_json
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(record.run_id.to_string())
    .bind(record.security_id.to_string())
    .bind(&record.kind)
    .bind(&record.requested_at)
    .bind(if record.ok { 1i64 } else { 0 })
    .bind(&record.code)
    .bind(&record.message)
    .bind(record.attempted as i64)
    .bind(record.recorded as i64)
    .bind(record.skipped as i64)
    .bind(record.unchanged as i64)
    .bind(&record.payload_json)
    .execute(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    Ok(record)
}

pub async fn retrieve_run_list(
    pool: &SqlitePool,
    security_id: Option<Uuid>,
    limit: u32,
) -> Result<Vec<RetrieveRunRecord>, PlatformError> {
    let limit = limit.clamp(1, 200) as i64;
    let rows = if let Some(id) = security_id {
        sqlx::query(
            "SELECT run_id, security_id, kind, requested_at, ok, code, message,
                    attempted, recorded, skipped, unchanged, payload_json
             FROM retrieve_run
             WHERE security_id = ?
             ORDER BY requested_at DESC, rowid DESC
             LIMIT ?",
        )
        .bind(id.to_string())
        .bind(limit)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query(
            "SELECT run_id, security_id, kind, requested_at, ok, code, message,
                    attempted, recorded, skipped, unchanged, payload_json
             FROM retrieve_run
             ORDER BY requested_at DESC, rowid DESC
             LIMIT ?",
        )
        .bind(limit)
        .fetch_all(pool)
        .await
    }
    .map_err(|e| map_err(e.into()))?;
    rows.into_iter()
        .map(|row| {
            Ok(RetrieveRunRecord {
                run_id: parse_uuid(&row, "run_id")?,
                security_id: parse_uuid(&row, "security_id")?,
                kind: row.try_get("kind").map_err(|e| map_err(e.into()))?,
                requested_at: row.try_get("requested_at").map_err(|e| map_err(e.into()))?,
                ok: row.try_get::<i64, _>("ok").map_err(|e| map_err(e.into()))? != 0,
                code: row.try_get("code").unwrap_or_default(),
                message: row.try_get("message").unwrap_or_default(),
                attempted: row.try_get::<i64, _>("attempted").unwrap_or(0) as u64,
                recorded: row.try_get::<i64, _>("recorded").unwrap_or(0) as u64,
                skipped: row.try_get::<i64, _>("skipped").unwrap_or(0) as u64,
                unchanged: row.try_get::<i64, _>("unchanged").unwrap_or(0) as u64,
                payload_json: row.try_get("payload_json").unwrap_or_else(|_| "{}".into()),
            })
        })
        .collect()
}

pub async fn collector_set(pool: &SqlitePool) -> Result<CollectorSetBody, PlatformError> {
    let rows = sqlx::query(
        "SELECT s.security_id, s.symbol,
                COALESCE(pc.provider, '') AS provider,
                COALESCE(pc.div_type, '') AS div_type,
                COALESCE(pc.payment_frequency, '') AS payment_frequency,
                COALESCE(rt.declaration_source, '') AS declaration_source,
                COALESCE(rt.price_source, '') AS price_source,
                COALESCE(rt.source_symbol, '') AS source_symbol,
                COALESCE(rt.source_url, '') AS source_url,
                COALESCE(rt.calendar_policy, '') AS calendar_policy,
                COALESCE(rt.collector_enabled, 0) AS collector_enabled,
                COALESCE(rt.last_run_at, '') AS last_run_at,
                rt.last_run_ok,
                COALESCE(rt.last_run_message, '') AS last_run_message,
                COALESCE(rt.last_content_hash, '') AS last_content_hash,
                COALESCE(rt.roc_source_url, '') AS roc_source_url,
                COALESCE(rt.inception_on, '') AS inception_on,
                EXISTS(
                    SELECT 1 FROM lot l
                    WHERE l.security_id = s.security_id AND l.remaining_quantity_minor > 0
                ) AS open_lots
         FROM security s
         LEFT JOIN position_characteristic pc ON pc.security_id = s.security_id
         LEFT JOIN retrieval_template rt ON rt.security_id = s.security_id
         WHERE EXISTS(
                SELECT 1 FROM lot l
                WHERE l.security_id = s.security_id AND l.remaining_quantity_minor > 0
            )
         ORDER BY s.symbol",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    let mut items = Vec::new();
    for row in rows {
        let symbol: String = row.try_get("symbol").map_err(|e| map_err(e.into()))?;
        let div_type: String = row.try_get("div_type").unwrap_or_default();
        let payment_frequency: String = row.try_get("payment_frequency").unwrap_or_default();
        let source_url: String = row.try_get("source_url").unwrap_or_default();
        let collector_enabled: i64 = row.try_get("collector_enabled").unwrap_or(0);
        if !collector_symbol_pays(&div_type, &payment_frequency, &symbol) {
            continue;
        }
        items.push(CollectorSetItem {
            security_id: parse_uuid(&row, "security_id")?,
            symbol,
            provider: row.try_get("provider").unwrap_or_default(),
            div_type,
            payment_frequency,
            declaration_source: row.try_get("declaration_source").unwrap_or_default(),
            price_source: row.try_get("price_source").unwrap_or_default(),
            source_symbol: row.try_get("source_symbol").unwrap_or_default(),
            source_url,
            calendar_policy: row.try_get("calendar_policy").unwrap_or_default(),
            collector_enabled: collector_enabled != 0,
            last_run_at: row.try_get("last_run_at").unwrap_or_default(),
            last_run_ok: row
                .try_get::<Option<i64>, _>("last_run_ok")
                .ok()
                .flatten()
                .map(|n| n != 0),
            last_run_message: row.try_get("last_run_message").unwrap_or_default(),
            last_content_hash: row.try_get("last_content_hash").unwrap_or_default(),
            roc_source_url: row.try_get("roc_source_url").unwrap_or_default(),
            open_lots: row.try_get::<i64, _>("open_lots").unwrap_or(0) != 0,
            inception_on: row.try_get("inception_on").unwrap_or_default(),
            complete: false,
            gaps: Vec::new(),
            fill_gaps_provider_blank: false,
            fill_gaps_frequency_blank: false,
            fill_gaps_div_type_blank: false,
            fill_gaps_roc_blank: false,
            paid_declaration_count: 0,
            remaining_planned: None,
            remaining_expected: None,
            successful_run_count: 0,
            failure_count: 0,
            open_ticket_count: 0,
            latest_ticket_field: String::new(),
            last_price_as_of: String::new(),
            last_price_freshness: String::new(),
            underlying: String::new(),
            roc_estimate_minor: None,
            roc_scale: 0,
            roc_tax_year: String::new(),
            open_lot_count: 0,
            last_payable_on: String::new(),
        });
    }
    Ok(CollectorSetBody { items })
}

/// Collectors fleet is for income names only — exclude non-payers (cadence None / equity).
fn collector_symbol_pays(div_type: &str, payment_frequency: &str, symbol: &str) -> bool {
    if financial_domain::current_price::uses_cash_par(div_type, symbol) {
        return true;
    }
    if financial_domain::div1::is_div1(div_type) {
        return true;
    }
    matches!(
        financial_domain::calculator::PaymentCadence::parse(payment_frequency),
        Some(
            financial_domain::calculator::PaymentCadence::Weekly
                | financial_domain::calculator::PaymentCadence::Monthly
                | financial_domain::calculator::PaymentCadence::Quarterly
        )
    )
}

pub async fn collector_stats(
    pool: &SqlitePool,
    as_of_date: &str,
) -> Result<CollectorStatsBody, PlatformError> {
    let set = collector_set(pool).await?;
    let assigned = set
        .items
        .iter()
        .filter(|i| {
            financial_domain::div1::is_registered_declaration_source(&i.declaration_source)
        })
        .count() as u64;
    let enabled = set.items.iter().filter(|i| i.collector_enabled).count() as u64;
    let cash_par = set
        .items
        .iter()
        .filter(|i| {
            financial_domain::current_price::uses_cash_par(&i.div_type, &i.symbol)
        })
        .count() as u64;

    // Declaration-only stats scoped to enabled collectors, counting distinct symbols.
    let today_pat = format!("{as_of_date}%");
    let ran_today: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT security_id) FROM retrieve_run
         WHERE requested_at LIKE ? AND kind = 'declaration'",
    )
    .bind(&today_pat)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let miss_today: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT security_id) FROM retrieve_run
         WHERE requested_at LIKE ? AND kind = 'declaration' AND ok = 0",
    )
    .bind(&today_pat)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let unchanged_today: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT security_id) FROM retrieve_run
         WHERE requested_at LIKE ? AND kind = 'declaration' AND unchanged > 0",
    )
    .bind(&today_pat)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let open_exceptions: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM work_ticket WHERE status = 'open'",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    // Price freshness from latest quotes for open-lot symbols.
    let mut price_current = 0u64;
    let mut price_stale = 0u64;
    for item in set.items.iter().filter(|i| i.open_lots) {
        let price = current_price_get(pool, item.security_id, as_of_date.to_string()).await?;
        match price.freshness.as_str() {
            "current" | "manual_override" => price_current += 1,
            "stale" => price_stale += 1,
            _ => {}
        }
    }

    Ok(CollectorStatsBody {
        assigned,
        enabled,
        ran_today: ran_today as u64,
        miss_today: miss_today as u64,
        unchanged_today: unchanged_today as u64,
        cash_par,
        price_current,
        price_stale,
        open_exceptions: open_exceptions as u64,
        as_of_date: as_of_date.to_string(),
    })
}
