//! option_contract table. Money is integer cents (scale 2).

use application_core::contracts::OptionContractRecord;
use application_core::ports::platform::PlatformError;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

fn map_sql(err: sqlx::Error) -> PlatformError {
    PlatformError::new("storage_error", err.to_string())
}

fn row_contract(row: &sqlx::sqlite::SqliteRow) -> Result<OptionContractRecord, PlatformError> {
    let contract_id: String = row.try_get("contract_id").map_err(map_sql)?;
    let account_id: Option<String> = row.try_get("account_id").map_err(map_sql)?;
    Ok(OptionContractRecord {
        contract_id: Uuid::parse_str(&contract_id)
            .map_err(|e| PlatformError::new("parse_error", e.to_string()))?,
        occ_symbol: row.try_get("occ_symbol").map_err(map_sql)?,
        underlying: row.try_get("underlying").map_err(map_sql)?,
        expiry_on: row.try_get("expiry_on").map_err(map_sql)?,
        put_call: row.try_get("put_call").map_err(map_sql)?,
        strike_minor: row.try_get("strike_minor").map_err(map_sql)?,
        scale: row.try_get::<i64, _>("scale").map_err(map_sql)? as u8,
        account_id: account_id.filter(|s| !s.is_empty()),
        side: row.try_get("side").map_err(map_sql)?,
        quantity: row.try_get("quantity").map_err(map_sql)?,
        open_premium_minor: row.try_get("open_premium_minor").map_err(map_sql)?,
        open_on: row.try_get("open_on").map_err(map_sql)?,
        underlying_last_minor: row.try_get("underlying_last_minor").map_err(map_sql)?,
        option_mid_minor: row.try_get("option_mid_minor").map_err(map_sql)?,
        quote_as_of: row.try_get("quote_as_of").map_err(map_sql)?,
        status: row.try_get("status").map_err(map_sql)?,
        roll_to_contract_id: row.try_get("roll_to_contract_id").map_err(map_sql)?,
        close_premium_minor: row.try_get("close_premium_minor").map_err(map_sql)?,
        closed_on: row.try_get("closed_on").map_err(map_sql)?,
        payload_json: row.try_get("payload_json").map_err(map_sql)?,
        created_on: row.try_get("created_on").map_err(map_sql)?,
        updated_on: row.try_get("updated_on").map_err(map_sql)?,
    })
}

const SELECT_COLS: &str = "contract_id, occ_symbol, underlying, expiry_on, put_call, strike_minor,
                scale, account_id, side, quantity, open_premium_minor, open_on,
                underlying_last_minor, option_mid_minor, quote_as_of, status,
                roll_to_contract_id, close_premium_minor, closed_on, payload_json,
                created_on, updated_on";

pub async fn option_contract_list(
    pool: &SqlitePool,
    status: Option<String>,
) -> Result<Vec<OptionContractRecord>, PlatformError> {
    let st = status.unwrap_or_default();
    let rows = sqlx::query(&format!(
        "SELECT {SELECT_COLS}
         FROM option_contract
         WHERE (? = '' OR status = ?)
         ORDER BY CASE status WHEN 'open' THEN 0 ELSE 1 END, expiry_on, occ_symbol"
    ))
    .bind(&st)
    .bind(&st)
    .fetch_all(pool)
    .await
    .map_err(map_sql)?;
    rows.iter().map(row_contract).collect()
}

pub async fn option_contract_get(
    pool: &SqlitePool,
    contract_id: Uuid,
) -> Result<OptionContractRecord, PlatformError> {
    let row = sqlx::query(&format!(
        "SELECT {SELECT_COLS} FROM option_contract WHERE contract_id = ?"
    ))
    .bind(contract_id.to_string())
    .fetch_optional(pool)
    .await
    .map_err(map_sql)?
    .ok_or_else(|| PlatformError::new("missing_contract", "contract not found"))?;
    row_contract(&row)
}

pub async fn option_contract_insert(
    pool: &SqlitePool,
    record: OptionContractRecord,
) -> Result<OptionContractRecord, PlatformError> {
    sqlx::query(
        "INSERT INTO option_contract (
            contract_id, occ_symbol, underlying, expiry_on, put_call, strike_minor, scale,
            account_id, side, quantity, open_premium_minor, open_on,
            underlying_last_minor, option_mid_minor, quote_as_of, status,
            roll_to_contract_id, close_premium_minor, closed_on, payload_json,
            created_on, updated_on
         ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7,
            ?8, ?9, ?10, ?11, ?12,
            ?13, ?14, ?15, ?16,
            ?17, ?18, ?19, ?20,
            ?21, ?22
         )",
    )
    .bind(record.contract_id.to_string())
    .bind(&record.occ_symbol)
    .bind(&record.underlying)
    .bind(&record.expiry_on)
    .bind(&record.put_call)
    .bind(record.strike_minor)
    .bind(i64::from(record.scale))
    .bind(record.account_id.clone())
    .bind(&record.side)
    .bind(record.quantity)
    .bind(record.open_premium_minor)
    .bind(&record.open_on)
    .bind(record.underlying_last_minor)
    .bind(record.option_mid_minor)
    .bind(&record.quote_as_of)
    .bind(&record.status)
    .bind(&record.roll_to_contract_id)
    .bind(record.close_premium_minor)
    .bind(&record.closed_on)
    .bind(&record.payload_json)
    .bind(&record.created_on)
    .bind(&record.updated_on)
    .execute(pool)
    .await
    .map_err(map_sql)?;
    option_contract_get(pool, record.contract_id).await
}

pub async fn option_contract_update(
    pool: &SqlitePool,
    record: OptionContractRecord,
) -> Result<OptionContractRecord, PlatformError> {
    let updated = sqlx::query(
        "UPDATE option_contract SET
            occ_symbol = ?2,
            underlying = ?3,
            expiry_on = ?4,
            put_call = ?5,
            strike_minor = ?6,
            scale = ?7,
            account_id = ?8,
            side = ?9,
            quantity = ?10,
            open_premium_minor = ?11,
            open_on = ?12,
            underlying_last_minor = ?13,
            option_mid_minor = ?14,
            quote_as_of = ?15,
            status = ?16,
            roll_to_contract_id = ?17,
            close_premium_minor = ?18,
            closed_on = ?19,
            payload_json = ?20,
            updated_on = ?21
         WHERE contract_id = ?1",
    )
    .bind(record.contract_id.to_string())
    .bind(&record.occ_symbol)
    .bind(&record.underlying)
    .bind(&record.expiry_on)
    .bind(&record.put_call)
    .bind(record.strike_minor)
    .bind(i64::from(record.scale))
    .bind(record.account_id.clone())
    .bind(&record.side)
    .bind(record.quantity)
    .bind(record.open_premium_minor)
    .bind(&record.open_on)
    .bind(record.underlying_last_minor)
    .bind(record.option_mid_minor)
    .bind(&record.quote_as_of)
    .bind(&record.status)
    .bind(&record.roll_to_contract_id)
    .bind(record.close_premium_minor)
    .bind(&record.closed_on)
    .bind(&record.payload_json)
    .bind(&record.updated_on)
    .execute(pool)
    .await
    .map_err(map_sql)?;
    if updated.rows_affected() == 0 {
        return Err(PlatformError::new("missing_contract", "contract not found"));
    }
    option_contract_get(pool, record.contract_id).await
}
