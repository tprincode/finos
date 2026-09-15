//! Shopping cart (table-isolated from ledger, lots, and MAGI).

use application_core::contracts::{
    CartBuyLineBody, CartEvalBody, CartGetBody, CartItemRecord, CartScenarioBody, CartScenarioListBody,
    CartSellLineBody,
};
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

fn parse_uuid(raw: &str) -> Result<Uuid, PlatformError> {
    Uuid::parse_str(raw).map_err(|e| PlatformError::new("parse_error", e.to_string()))
}

pub async fn scenario_get(pool: &SqlitePool, scenario_id: Uuid) -> Result<CartScenarioBody, PlatformError> {
    let header = sqlx::query(
        "SELECT scenario_id, account_id, account_name, name, kind, status, as_of, cash_yield_bps,
                funding_source, override_reason, created_at, agreed_at
         FROM cart_scenario WHERE scenario_id = ?",
    )
    .bind(scenario_id.to_string())
    .fetch_optional(pool)
    .await
    .map_err(|e| map_err(e.into()))?
    .ok_or_else(|| PlatformError::new("missing_scenario", "cart scenario not found"))?;
    let sells = sqlx::query(
        "SELECT line_id, lot_id, security_id, symbol, qty_minor, qty_scale, unit_minor,
                proceeds_minor, is_cash, original_cost_minor, performance_cost_minor, tax_cost_minor,
                performance_gain_minor, tax_gain_minor
         FROM cart_sell_line WHERE scenario_id = ? ORDER BY line_id",
    )
    .bind(scenario_id.to_string())
    .fetch_all(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    let buys = sqlx::query(
        "SELECT line_id, security_id, symbol, qty_whole, last_minor, spend_minor, plan_annual_minor
         FROM cart_buy_line WHERE scenario_id = ? ORDER BY line_id",
    )
    .bind(scenario_id.to_string())
    .fetch_all(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    let eval_row = sqlx::query(
        "SELECT remaining_minor, spend_minor, leftover_minor, buy_annual_minor, surrendered_annual_minor,
                leftover_annual_minor, net_annual_minor, net_monthly_minor, net_weekly_minor,
                insufficient_lot_qty, cash_floor_warn
         FROM cart_eval_snapshot WHERE scenario_id = ?",
    )
    .bind(scenario_id.to_string())
    .fetch_optional(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    let mut sell_lines = Vec::new();
    for row in &sells {
        let sid: Option<String> = row.try_get("security_id").map_err(|e| map_err(e.into()))?;
        sell_lines.push(CartSellLineBody {
            line_id: parse_uuid(&row.try_get::<String, _>("line_id").map_err(|e| map_err(e.into()))?)?,
            lot_id: parse_uuid(&row.try_get::<String, _>("lot_id").map_err(|e| map_err(e.into()))?)?,
            security_id: sid.as_deref().map(parse_uuid).transpose()?,
            symbol: row.try_get("symbol").map_err(|e| map_err(e.into()))?,
            qty_minor: row.try_get("qty_minor").map_err(|e| map_err(e.into()))?,
            qty_scale: row.try_get::<i64, _>("qty_scale").map_err(|e| map_err(e.into()))? as u8,
            unit_minor: row.try_get("unit_minor").map_err(|e| map_err(e.into()))?,
            proceeds_minor: row.try_get("proceeds_minor").map_err(|e| map_err(e.into()))?,
            is_cash: row.try_get::<i64, _>("is_cash").map_err(|e| map_err(e.into()))? != 0,
            original_cost_minor: row.try_get("original_cost_minor").map_err(|e| map_err(e.into()))?,
            performance_cost_minor: row.try_get("performance_cost_minor").map_err(|e| map_err(e.into()))?,
            tax_cost_minor: row.try_get("tax_cost_minor").map_err(|e| map_err(e.into()))?,
            performance_gain_minor: row.try_get("performance_gain_minor").map_err(|e| map_err(e.into()))?,
            tax_gain_minor: row.try_get("tax_gain_minor").map_err(|e| map_err(e.into()))?,
        });
    }
    let mut buy_lines = Vec::new();
    for row in &buys {
        buy_lines.push(CartBuyLineBody {
            line_id: parse_uuid(&row.try_get::<String, _>("line_id").map_err(|e| map_err(e.into()))?)?,
            security_id: parse_uuid(&row.try_get::<String, _>("security_id").map_err(|e| map_err(e.into()))?)?,
            symbol: row.try_get("symbol").map_err(|e| map_err(e.into()))?,
            qty_whole: row.try_get("qty_whole").map_err(|e| map_err(e.into()))?,
            last_minor: row.try_get("last_minor").map_err(|e| map_err(e.into()))?,
            spend_minor: row.try_get("spend_minor").map_err(|e| map_err(e.into()))?,
            plan_annual_minor: row.try_get("plan_annual_minor").map_err(|e| map_err(e.into()))?,
        });
    }
    let status: String = header.try_get("status").map_err(|e| map_err(e.into()))?;
    let eval = eval_row
        .map(|row| {
            Ok(CartEvalBody {
                remaining_minor: row.try_get("remaining_minor").map_err(|e| map_err(e.into()))?,
                spend_minor: row.try_get("spend_minor").map_err(|e| map_err(e.into()))?,
                leftover_minor: row.try_get("leftover_minor").map_err(|e| map_err(e.into()))?,
                buy_annual_minor: row.try_get("buy_annual_minor").map_err(|e| map_err(e.into()))?,
                surrendered_annual_minor: row
                    .try_get("surrendered_annual_minor")
                    .map_err(|e| map_err(e.into()))?,
                leftover_annual_minor: row
                    .try_get("leftover_annual_minor")
                    .map_err(|e| map_err(e.into()))?,
                net_annual_minor: row.try_get("net_annual_minor").map_err(|e| map_err(e.into()))?,
                net_monthly_minor: row.try_get("net_monthly_minor").map_err(|e| map_err(e.into()))?,
                net_weekly_minor: row.try_get("net_weekly_minor").map_err(|e| map_err(e.into()))?,
                insufficient_lot_qty: row
                    .try_get::<i64, _>("insufficient_lot_qty")
                    .map_err(|e| map_err(e.into()))?
                    != 0,
                cash_floor_warn: row
                    .try_get::<i64, _>("cash_floor_warn")
                    .map_err(|e| map_err(e.into()))?
                    != 0,
                status: status.clone(),
            })
        })
        .transpose()?;
    Ok(CartScenarioBody {
        scenario_id,
        account_id: parse_uuid(&header.try_get::<String, _>("account_id").map_err(|e| map_err(e.into()))?)?,
        account_name: header.try_get("account_name").map_err(|e| map_err(e.into()))?,
        name: header.try_get("name").map_err(|e| map_err(e.into()))?,
        kind: header.try_get("kind").map_err(|e| map_err(e.into()))?,
        status,
        as_of: header.try_get("as_of").map_err(|e| map_err(e.into()))?,
        cash_yield_bps: header.try_get("cash_yield_bps").map_err(|e| map_err(e.into()))?,
        funding_source: header.try_get("funding_source").map_err(|e| map_err(e.into()))?,
        override_reason: header.try_get("override_reason").map_err(|e| map_err(e.into()))?,
        sell_lines,
        buy_lines,
        eval,
    })
}

pub async fn scenario_create(
    pool: &SqlitePool,
    account_id: Uuid,
    account_name: String,
    as_of: String,
    cash_yield_bps: i64,
    name: String,
    funding_source: String,
) -> Result<CartScenarioBody, PlatformError> {
    let id = Uuid::new_v4();
    let created = chrono::Utc::now().to_rfc3339();
    let label = if name.trim().is_empty() {
        "Draft".to_string()
    } else {
        name
    };
    sqlx::query(
        "INSERT INTO cart_scenario (scenario_id, account_id, account_name, name, kind, status, as_of,
            cash_yield_bps, funding_source, created_at)
         VALUES (?, ?, ?, ?, 'Swap', 'draft', ?, ?, ?, ?)",
    )
    .bind(id.to_string())
    .bind(account_id.to_string())
    .bind(&account_name)
    .bind(&label)
    .bind(&as_of)
    .bind(cash_yield_bps)
    .bind(&funding_source)
    .bind(created)
    .execute(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    scenario_get(pool, id).await
}

pub async fn sell_line_add(
    pool: &SqlitePool,
    scenario_id: Uuid,
    lot_id: Uuid,
    security_id: Option<Uuid>,
    symbol: String,
    qty_minor: i64,
    qty_scale: u8,
    unit_minor: i64,
    proceeds_minor: i64,
    is_cash: bool,
    original_cost_minor: Option<i64>,
    performance_cost_minor: Option<i64>,
    tax_cost_minor: Option<i64>,
    performance_gain_minor: Option<i64>,
    tax_gain_minor: Option<i64>,
) -> Result<CartScenarioBody, PlatformError> {
    sqlx::query(
        "INSERT INTO cart_sell_line (line_id, scenario_id, lot_id, security_id, symbol, qty_minor,
            qty_scale, unit_minor, proceeds_minor, is_cash, original_cost_minor,
            performance_cost_minor, tax_cost_minor, performance_gain_minor, tax_gain_minor)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(scenario_id.to_string())
    .bind(lot_id.to_string())
    .bind(security_id.map(|id| id.to_string()))
    .bind(&symbol)
    .bind(qty_minor)
    .bind(qty_scale as i64)
    .bind(unit_minor)
    .bind(proceeds_minor)
    .bind(if is_cash { 1 } else { 0 })
    .bind(original_cost_minor)
    .bind(performance_cost_minor)
    .bind(tax_cost_minor)
    .bind(performance_gain_minor)
    .bind(tax_gain_minor)
    .execute(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    scenario_get(pool, scenario_id).await
}

pub async fn buy_line_add(
    pool: &SqlitePool,
    scenario_id: Uuid,
    security_id: Uuid,
    symbol: String,
    qty_whole: i64,
    last_minor: i64,
    spend_minor: i64,
    plan_annual_minor: Option<i64>,
) -> Result<CartScenarioBody, PlatformError> {
    sqlx::query(
        "INSERT INTO cart_buy_line (line_id, scenario_id, security_id, symbol, qty_whole, last_minor,
            spend_minor, plan_annual_minor)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(scenario_id.to_string())
    .bind(security_id.to_string())
    .bind(&symbol)
    .bind(qty_whole)
    .bind(last_minor)
    .bind(spend_minor)
    .bind(plan_annual_minor)
    .execute(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    scenario_get(pool, scenario_id).await
}

pub async fn buy_line_price_set(
    pool: &SqlitePool,
    line_id: Uuid,
    last_minor: i64,
    spend_minor: i64,
) -> Result<(), PlatformError> {
    sqlx::query("UPDATE cart_buy_line SET last_minor = ?, spend_minor = ? WHERE line_id = ?")
        .bind(last_minor)
        .bind(spend_minor)
        .bind(line_id.to_string())
        .execute(pool)
        .await
        .map_err(|e| map_err(e.into()))?;
    Ok(())
}

pub async fn buy_line_set(
    pool: &SqlitePool,
    line_id: Uuid,
    qty_whole: i64,
    last_minor: i64,
    spend_minor: i64,
    plan_annual_minor: Option<i64>,
) -> Result<(), PlatformError> {
    sqlx::query(
        "UPDATE cart_buy_line SET qty_whole = ?, last_minor = ?, spend_minor = ?, plan_annual_minor = ?
         WHERE line_id = ?",
    )
    .bind(qty_whole)
    .bind(last_minor)
    .bind(spend_minor)
    .bind(plan_annual_minor)
    .bind(line_id.to_string())
    .execute(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    Ok(())
}

pub async fn eval_save(
    pool: &SqlitePool,
    scenario_id: Uuid,
    remaining_minor: i64,
    spend_minor: i64,
    leftover_minor: i64,
    buy_annual_minor: Option<i64>,
    surrendered_annual_minor: Option<i64>,
    leftover_annual_minor: Option<i64>,
    net_annual_minor: Option<i64>,
    net_monthly_minor: Option<i64>,
    net_weekly_minor: Option<i64>,
    insufficient_lot_qty: bool,
    cash_floor_warn: bool,
) -> Result<CartScenarioBody, PlatformError> {
    sqlx::query(
        "INSERT INTO cart_eval_snapshot (
            scenario_id, remaining_minor, spend_minor, leftover_minor, buy_annual_minor,
            surrendered_annual_minor, leftover_annual_minor, net_annual_minor, net_monthly_minor,
            net_weekly_minor, insufficient_lot_qty, cash_floor_warn)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(scenario_id) DO UPDATE SET
            remaining_minor = excluded.remaining_minor,
            spend_minor = excluded.spend_minor,
            leftover_minor = excluded.leftover_minor,
            buy_annual_minor = excluded.buy_annual_minor,
            surrendered_annual_minor = excluded.surrendered_annual_minor,
            leftover_annual_minor = excluded.leftover_annual_minor,
            net_annual_minor = excluded.net_annual_minor,
            net_monthly_minor = excluded.net_monthly_minor,
            net_weekly_minor = excluded.net_weekly_minor,
            insufficient_lot_qty = excluded.insufficient_lot_qty,
            cash_floor_warn = excluded.cash_floor_warn",
    )
    .bind(scenario_id.to_string())
    .bind(remaining_minor)
    .bind(spend_minor)
    .bind(leftover_minor)
    .bind(buy_annual_minor)
    .bind(surrendered_annual_minor)
    .bind(leftover_annual_minor)
    .bind(net_annual_minor)
    .bind(net_monthly_minor)
    .bind(net_weekly_minor)
    .bind(if insufficient_lot_qty { 1 } else { 0 })
    .bind(if cash_floor_warn { 1 } else { 0 })
    .execute(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    scenario_get(pool, scenario_id).await
}

pub async fn scenario_agree(
    pool: &SqlitePool,
    scenario_id: Uuid,
    override_reason: Option<String>,
) -> Result<CartScenarioBody, PlatformError> {
    let agreed = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "UPDATE cart_scenario SET status = 'agreed', override_reason = ?, agreed_at = ?
         WHERE scenario_id = ?",
    )
    .bind(&override_reason)
    .bind(agreed)
    .bind(scenario_id.to_string())
    .execute(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    scenario_get(pool, scenario_id).await
}

pub async fn execute_step_add(
    pool: &SqlitePool,
    scenario_id: Uuid,
    kind: String,
    activity_id: Option<Uuid>,
    assignment_id: Option<Uuid>,
    lot_id: Option<Uuid>,
) -> Result<(), PlatformError> {
    sqlx::query(
        "INSERT INTO cart_execute_step (step_id, scenario_id, kind, activity_id, assignment_id, lot_id)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(scenario_id.to_string())
    .bind(kind)
    .bind(activity_id.map(|id| id.to_string()))
    .bind(assignment_id.map(|id| id.to_string()))
    .bind(lot_id.map(|id| id.to_string()))
    .execute(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    Ok(())
}

pub async fn scenario_discard(pool: &SqlitePool, scenario_id: Uuid) -> Result<(), PlatformError> {
    let scene = scenario_get(pool, scenario_id).await?;
    if scene.status != "draft" {
        return Err(PlatformError::new(
            "not_draft",
            "only a draft swap can be discarded",
        ));
    }
    for sql in [
        "DELETE FROM cart_execute_step WHERE scenario_id = ?",
        "DELETE FROM cart_eval_snapshot WHERE scenario_id = ?",
        "DELETE FROM cart_buy_line WHERE scenario_id = ?",
        "DELETE FROM cart_sell_line WHERE scenario_id = ?",
        "DELETE FROM cart_scenario WHERE scenario_id = ?",
    ] {
        sqlx::query(sql)
            .bind(scenario_id.to_string())
            .execute(pool)
            .await
            .map_err(|e| map_err(e.into()))?;
    }
    Ok(())
}

pub async fn scenario_list(
    pool: &SqlitePool,
    account_id: Uuid,
) -> Result<CartScenarioListBody, PlatformError> {
    let rows = sqlx::query(
        "SELECT scenario_id FROM cart_scenario WHERE account_id = ? ORDER BY created_at, scenario_id",
    )
    .bind(account_id.to_string())
    .fetch_all(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    let mut items = Vec::new();
    for row in rows {
        let id = parse_uuid(&row.try_get::<String, _>("scenario_id").map_err(|e| map_err(e.into()))?)?;
        items.push(scenario_get(pool, id).await?);
    }
    Ok(CartScenarioListBody { items })
}

pub async fn scenario_rename(
    pool: &SqlitePool,
    scenario_id: Uuid,
    name: String,
) -> Result<CartScenarioBody, PlatformError> {
    let label = if name.trim().is_empty() {
        "Draft".to_string()
    } else {
        name
    };
    sqlx::query("UPDATE cart_scenario SET name = ? WHERE scenario_id = ?")
        .bind(&label)
        .bind(scenario_id.to_string())
        .execute(pool)
        .await
        .map_err(|e| map_err(e.into()))?;
    scenario_get(pool, scenario_id).await
}

pub async fn scenario_duplicate(
    pool: &SqlitePool,
    scenario_id: Uuid,
) -> Result<CartScenarioBody, PlatformError> {
    let src = scenario_get(pool, scenario_id).await?;
    if src.status != "draft" {
        return Err(PlatformError::new("not_draft", "only a draft can be duplicated"));
    }
    let copy = scenario_create(
        pool,
        src.account_id,
        src.account_name,
        src.as_of,
        src.cash_yield_bps,
        format!("{} (copy)", if src.name.is_empty() { "Draft" } else { &src.name }),
        src.funding_source,
    )
    .await?;
    for sell in src.sell_lines {
        sell_line_add(
            pool,
            copy.scenario_id,
            sell.lot_id,
            sell.security_id,
            sell.symbol,
            sell.qty_minor,
            sell.qty_scale,
            sell.unit_minor,
            sell.proceeds_minor,
            sell.is_cash,
            sell.original_cost_minor,
            sell.performance_cost_minor,
            sell.tax_cost_minor,
            sell.performance_gain_minor,
            sell.tax_gain_minor,
        )
        .await?;
    }
    for buy in src.buy_lines {
        buy_line_add(
            pool,
            copy.scenario_id,
            buy.security_id,
            buy.symbol,
            buy.qty_whole,
            buy.last_minor,
            buy.spend_minor,
            buy.plan_annual_minor,
        )
        .await?;
    }
    scenario_get(pool, copy.scenario_id).await
}
