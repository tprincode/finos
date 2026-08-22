//! Last Price quotes, issuer declarations, retrieval template (schema 15).

use application_core::contracts::{
    CurrentPriceBody, IssuerDeclarationRecord, PriceQuoteBody, PriceRetrievalSetBody,
    RetrievalTemplateRecord,
};
use application_core::ports::platform::PlatformError;
use financial_domain::current_price::{
    price_derived_valid, select_current_price, OverrideObservation, QuoteObservation,
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
    let over_row = sqlx::query(
        "SELECT price_minor, scale, status FROM manual_price_override
         WHERE security_id = ? AND status = 'active' ORDER BY effective_from DESC LIMIT 1",
    )
    .bind(security_id.to_string())
    .fetch_optional(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    let over = over_row
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
    })
}

pub async fn retrieval_template_set(
    pool: &SqlitePool,
    record: RetrievalTemplateRecord,
) -> Result<RetrievalTemplateRecord, PlatformError> {
    sqlx::query(
        "INSERT INTO retrieval_template (
            security_id, price_source, source_symbol, declaration_source, lookback_count, payment_source
         ) VALUES (?, ?, ?, ?, ?, ?)
         ON CONFLICT(security_id) DO UPDATE SET
            price_source = excluded.price_source,
            source_symbol = excluded.source_symbol,
            declaration_source = excluded.declaration_source,
            lookback_count = excluded.lookback_count,
            payment_source = excluded.payment_source",
    )
    .bind(record.security_id.to_string())
    .bind(&record.price_source)
    .bind(&record.source_symbol)
    .bind(&record.declaration_source)
    .bind(record.lookback_count as i64)
    .bind(&record.payment_source)
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
        "SELECT security_id, price_source, source_symbol, declaration_source, lookback_count, payment_source
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
        })
    })
    .transpose()
}

pub async fn price_retrieval_set(pool: &SqlitePool) -> Result<PriceRetrievalSetBody, PlatformError> {
    let rows = sqlx::query(
        "SELECT DISTINCT security_id FROM lot WHERE remaining_quantity_minor > 0",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    let mut security_ids = Vec::new();
    for row in rows {
        security_ids.push(parse_uuid(&row, "security_id")?);
    }
    Ok(PriceRetrievalSetBody { security_ids })
}

fn parse_uuid(row: &sqlx::sqlite::SqliteRow, col: &str) -> Result<Uuid, PlatformError> {
    let raw: String = row.try_get(col).map_err(|e| map_err(e.into()))?;
    Uuid::parse_str(&raw).map_err(|e| PlatformError::new("parse_error", e.to_string()))
}
