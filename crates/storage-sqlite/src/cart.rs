//! Shopping cart (table-isolated from ledger, lots, and MAGI).

use application_core::contracts::{CartGetBody, CartItemRecord};
use application_core::ports::platform::PlatformError;
use financial_domain::cart::prepare_line;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::store::StorageError;

fn map_err(err: StorageError) -> PlatformError {
    PlatformError::new("storage_error", err.to_string())
}

pub async fn item_add(
    pool: &SqlitePool,
    symbol: String,
    quantity_minor: i64,
    quantity_scale: u8,
) -> Result<CartGetBody, PlatformError> {
    let prepared = prepare_line(symbol, quantity_minor, quantity_scale);
    sqlx::query(
        "INSERT INTO cart_item (item_id, symbol, quantity_minor, quantity_scale)
         VALUES (?, ?, ?, ?)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&prepared.symbol)
    .bind(prepared.quantity_minor)
    .bind(prepared.quantity_scale as i64)
    .execute(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    cart_get(pool).await
}

pub async fn item_remove(pool: &SqlitePool, item_id: Uuid) -> Result<CartGetBody, PlatformError> {
    sqlx::query("DELETE FROM cart_item WHERE item_id = ?")
        .bind(item_id.to_string())
        .execute(pool)
        .await
        .map_err(|e| map_err(e.into()))?;
    cart_get(pool).await
}

pub async fn cart_get(pool: &SqlitePool) -> Result<CartGetBody, PlatformError> {
    let rows = sqlx::query(
        "SELECT item_id, symbol, quantity_minor, quantity_scale FROM cart_item ORDER BY symbol, item_id",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    let mut items = Vec::new();
    for row in &rows {
        let id: String = row.try_get("item_id").map_err(|e| map_err(e.into()))?;
        items.push(CartItemRecord {
            item_id: Uuid::parse_str(&id)
                .map_err(|e| PlatformError::new("parse_error", e.to_string()))?,
            symbol: row.try_get("symbol").map_err(|e| map_err(e.into()))?,
            quantity_minor: row
                .try_get("quantity_minor")
                .map_err(|e| map_err(e.into()))?,
            quantity_scale: row
                .try_get::<i64, _>("quantity_scale")
                .map_err(|e| map_err(e.into()))? as u8,
        });
    }
    Ok(CartGetBody { items })
}
