//! option_contract table. Money is integer cents (scale 2).

use application_core::contracts::{
    OptionContractRecord, OptionCoverCandidate, OptionCoverReservation, OptionCoverTake,
    OptionPremiumPostRecord,
};
use application_core::ports::platform::PlatformError;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

fn map_sql(err: sqlx::Error) -> PlatformError {
    PlatformError::new("storage_error", err.to_string())
}

fn payload_blank(payload: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(payload)
        .ok()
        .and_then(|value| value.get("openPremiumBlank").and_then(|flag| flag.as_bool()))
        .unwrap_or(false)
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
        open_premium_blank: payload_blank(&row.try_get::<String, _>("payload_json").unwrap_or_default()),
        open_on: row.try_get("open_on").map_err(map_sql)?,
        prior_balance_minor: row.try_get("prior_balance_minor").map_err(map_sql)?,
        live_underlying_minor: row.try_get("live_underlying_minor").map_err(map_sql)?,
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
                prior_balance_minor,
                underlying_last_minor, option_mid_minor, quote_as_of, status,
                roll_to_contract_id, close_premium_minor, closed_on, payload_json,
                created_on, updated_on,
                (SELECT pq.price_minor FROM price_quote pq
                   JOIN security s ON s.security_id = pq.security_id
                  WHERE s.symbol = option_contract.underlying
                    AND pq.quote_type = 'last'
                    AND pq.validation_status = 'accepted'
                  ORDER BY pq.as_of_at DESC LIMIT 1) AS live_underlying_minor";

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
            account_id, side, quantity, open_premium_minor, open_on, prior_balance_minor,
            underlying_last_minor, option_mid_minor, quote_as_of, status,
            roll_to_contract_id, close_premium_minor, closed_on, payload_json,
            created_on, updated_on
         ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7,
            ?8, ?9, ?10, ?11, ?12, ?13,
            ?14, ?15, ?16, ?17,
            ?18, ?19, ?20, ?21,
            ?22, ?23
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
    .bind(record.prior_balance_minor)
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
            prior_balance_minor = ?13,
            underlying_last_minor = ?14,
            option_mid_minor = ?15,
            quote_as_of = ?16,
            status = ?17,
            roll_to_contract_id = ?18,
            close_premium_minor = ?19,
            closed_on = ?20,
            payload_json = ?21,
            updated_on = ?22
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
    .bind(record.prior_balance_minor)
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

pub async fn option_cover_candidates(
    pool: &SqlitePool,
    account_id: Uuid,
    security_id: Uuid,
) -> Result<Vec<OptionCoverCandidate>, PlatformError> {
    let rows = sqlx::query(
        "SELECT l.lot_id, l.opened_on, l.remaining_quantity_minor, l.quantity_scale,
                l.remaining_performance_minor, l.scale,
                COALESCE((
                    SELECT SUM(c.quantity_minor) FROM option_cover_lot c
                    JOIN option_contract oc ON oc.contract_id = c.contract_id
                    WHERE c.lot_id = l.lot_id AND oc.status = 'open'
                ), 0) AS reserved
         FROM lot l
         WHERE l.account_id = ? AND l.security_id = ? AND l.remaining_quantity_minor > 0
         ORDER BY l.opened_on, l.lot_id",
    )
    .bind(account_id.to_string())
    .bind(security_id.to_string())
    .fetch_all(pool)
    .await
    .map_err(map_sql)?;
    let mut out = Vec::new();
    for row in rows {
        let lot_id: String = row.try_get("lot_id").map_err(map_sql)?;
        let opened_on: String = row.try_get("opened_on").map_err(map_sql)?;
        let remaining: i64 = row.try_get("remaining_quantity_minor").map_err(map_sql)?;
        let qty_scale: i64 = row.try_get("quantity_scale").map_err(map_sql)?;
        let cost: i64 = row.try_get("remaining_performance_minor").map_err(map_sql)?;
        let money_scale: i64 = row.try_get("scale").map_err(map_sql)?;
        let reserved: i64 = row.try_get("reserved").map_err(map_sql)?;
        let divisor = 10i64.pow(qty_scale.max(0) as u32);
        let shares = remaining / divisor;
        let free = shares - reserved;
        if free <= 0 {
            continue;
        }
        let cost_cents = if money_scale == 2 {
            cost
        } else if money_scale > 2 {
            cost / 10i64.pow((money_scale - 2) as u32)
        } else {
            cost * 10i64.pow((2 - money_scale.max(0)) as u32)
        };
        let free_cost = if free == shares {
            cost_cents
        } else {
            cost_cents.saturating_mul(free) / shares.max(1)
        };
        out.push(OptionCoverCandidate {
            lot_id: Uuid::parse_str(&lot_id)
                .map_err(|e| PlatformError::new("parse_error", e.to_string()))?,
            opened_on,
            shares: free,
            cost_minor: free_cost,
        });
    }
    Ok(out)
}

pub async fn option_cover_save(
    pool: &SqlitePool,
    contract_id: Uuid,
    takes: &[OptionCoverTake],
) -> Result<(), PlatformError> {
    for take in takes {
        sqlx::query(
            "INSERT INTO option_cover_lot (cover_id, contract_id, lot_id, quantity_minor, cost_minor, scale)
             VALUES (?, ?, ?, ?, ?, 2)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(contract_id.to_string())
        .bind(take.lot_id.to_string())
        .bind(take.shares)
        .bind(take.cost_minor)
        .execute(pool)
        .await
        .map_err(map_sql)?;
    }
    Ok(())
}

pub async fn option_cover_move(
    pool: &SqlitePool,
    from_id: Uuid,
    to_id: Uuid,
) -> Result<(), PlatformError> {
    sqlx::query("UPDATE option_cover_lot SET contract_id = ? WHERE contract_id = ?")
        .bind(to_id.to_string())
        .bind(from_id.to_string())
        .execute(pool)
        .await
        .map_err(map_sql)?;
    Ok(())
}

pub async fn option_cover_release(pool: &SqlitePool, contract_id: Uuid) -> Result<(), PlatformError> {
    sqlx::query("DELETE FROM option_cover_lot WHERE contract_id = ?")
        .bind(contract_id.to_string())
        .execute(pool)
        .await
        .map_err(map_sql)?;
    Ok(())
}

pub async fn option_cover_assign(pool: &SqlitePool, contract_id: Uuid) -> Result<(), PlatformError> {
    let rows = sqlx::query(
        "SELECT lot_id, quantity_minor, cost_minor FROM option_cover_lot WHERE contract_id = ?",
    )
    .bind(contract_id.to_string())
    .fetch_all(pool)
    .await
    .map_err(map_sql)?;
    for row in rows {
        let lot_id: String = row.try_get("lot_id").map_err(map_sql)?;
        let shares: i64 = row.try_get("quantity_minor").map_err(map_sql)?;
        let cost: i64 = row.try_get("cost_minor").map_err(map_sql)?;
        let scale_row = sqlx::query("SELECT quantity_scale FROM lot WHERE lot_id = ?")
            .bind(&lot_id)
            .fetch_optional(pool)
            .await
            .map_err(map_sql)?;
        let qty_scale: i64 = match scale_row {
            Some(row) => row.try_get("quantity_scale").map_err(map_sql)?,
            None => 0,
        };
        let units = shares.saturating_mul(10i64.pow(qty_scale.max(0) as u32));
        sqlx::query(
            "UPDATE lot SET remaining_quantity_minor = remaining_quantity_minor - ?,
                            remaining_performance_minor = remaining_performance_minor - ?,
                            remaining_tax_minor = remaining_tax_minor - ?
             WHERE lot_id = ?",
        )
        .bind(units)
        .bind(cost)
        .bind(cost)
        .bind(&lot_id)
        .execute(pool)
        .await
        .map_err(map_sql)?;
    }
    option_cover_release(pool, contract_id).await
}

pub async fn option_premium_insert(
    pool: &SqlitePool,
    record: &OptionPremiumPostRecord,
) -> Result<(), PlatformError> {
    sqlx::query(
        "INSERT INTO option_premium_post (
            post_id, contract_id, week_start, category, amount_minor, scale, reason
         ) VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(record.post_id.to_string())
    .bind(record.contract_id.to_string())
    .bind(&record.week_start)
    .bind(&record.category)
    .bind(record.amount_minor)
    .bind(i64::from(record.scale))
    .bind(&record.reason)
    .execute(pool)
    .await
    .map_err(map_sql)?;
    Ok(())
}

pub async fn option_premium_list(
    pool: &SqlitePool,
) -> Result<Vec<OptionPremiumPostRecord>, PlatformError> {
    let rows = sqlx::query(
        "SELECT post_id, contract_id, week_start, category, amount_minor, scale, reason
         FROM option_premium_post ORDER BY week_start, post_id",
    )
    .fetch_all(pool)
    .await
    .map_err(map_sql)?;
    let mut out = Vec::new();
    for row in rows {
        let post_id: String = row.try_get("post_id").map_err(map_sql)?;
        let contract_id: String = row.try_get("contract_id").map_err(map_sql)?;
        out.push(OptionPremiumPostRecord {
            post_id: Uuid::parse_str(&post_id)
                .map_err(|e| PlatformError::new("parse_error", e.to_string()))?,
            contract_id: Uuid::parse_str(&contract_id)
                .map_err(|e| PlatformError::new("parse_error", e.to_string()))?,
            week_start: row.try_get("week_start").map_err(map_sql)?,
            category: row.try_get("category").map_err(map_sql)?,
            amount_minor: row.try_get("amount_minor").map_err(map_sql)?,
            scale: row.try_get::<i64, _>("scale").map_err(map_sql)? as u8,
            reason: row.try_get("reason").map_err(map_sql)?,
        });
    }
    Ok(out)
}

pub async fn option_cover_reservations(
    pool: &SqlitePool,
) -> Result<Vec<OptionCoverReservation>, PlatformError> {
    let rows = sqlx::query(
        "SELECT c.lot_id, SUM(c.quantity_minor) AS shares
         FROM option_cover_lot c
         JOIN option_contract oc ON oc.contract_id = c.contract_id
         WHERE oc.status = 'open'
         GROUP BY c.lot_id",
    )
    .fetch_all(pool)
    .await
    .map_err(map_sql)?;
    let mut out = Vec::new();
    for row in rows {
        let lot_id: String = row.try_get("lot_id").map_err(map_sql)?;
        out.push(OptionCoverReservation {
            lot_id: Uuid::parse_str(&lot_id)
                .map_err(|e| PlatformError::new("parse_error", e.to_string()))?,
            shares: row.try_get("shares").map_err(map_sql)?,
        });
    }
    Ok(out)
}
