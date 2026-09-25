//! External-account register rows. Not cash elements and not cash lots.

use std::collections::BTreeMap;
use std::path::Path;

use application_core::contracts::{ExternalRegisterGetBody, ExternalRegisterLine};
use application_core::external_register::{
    amount_to_minor, fixed_occurred_on, line_matches, normalize_category, normalize_pay_type,
    normalize_vendor,
};
use application_core::ports::platform::PlatformError;
use calamine::{open_workbook, Data, Reader, Xlsx};
use chrono::{Duration, NaiveDate};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

fn map_sql(err: sqlx::Error) -> PlatformError {
    PlatformError::new("external_register", err.to_string())
}

fn distinct(values: impl Iterator<Item = String>) -> Vec<String> {
    let mut seen: BTreeMap<String, String> = BTreeMap::new();
    for value in values {
        let trimmed = value.trim().to_string();
        if trimmed.is_empty() {
            continue;
        }
        seen.entry(trimmed.to_lowercase()).or_insert(trimmed);
    }
    seen.into_values().collect()
}

fn row_to_line(row: &sqlx::sqlite::SqliteRow) -> Result<ExternalRegisterLine, PlatformError> {
    let line_id: String = row.try_get("line_id").map_err(map_sql)?;
    let line_id = Uuid::parse_str(&line_id)
        .map_err(|_| PlatformError::new("bad_line_id", "stored register line id is not a uuid"))?;
    let scale: i64 = row.try_get("scale").map_err(map_sql)?;
    Ok(ExternalRegisterLine {
        line_id,
        source_row: row.try_get("source_row").map_err(map_sql)?,
        pay_type: row.try_get("pay_type").map_err(map_sql)?,
        occurred_on: row.try_get("occurred_on").map_err(map_sql)?,
        amount_minor: row.try_get("amount_minor").map_err(map_sql)?,
        scale: scale as u8,
        category: row.try_get("category").map_err(map_sql)?,
        vendor: row.try_get("vendor").map_err(map_sql)?,
        description: row.try_get("description").map_err(map_sql)?,
        true_up_on: row.try_get("true_up_on").map_err(map_sql)?,
        completed: row.try_get::<i64, _>("completed").map_err(map_sql)? != 0,
        step_transfer: row.try_get::<i64, _>("step_transfer").map_err(map_sql)? != 0,
        step_billpay: row.try_get::<i64, _>("step_billpay").map_err(map_sql)? != 0,
        step_pay: row.try_get::<i64, _>("step_pay").map_err(map_sql)? != 0,
    })
}

async fn load_all(pool: &SqlitePool) -> Result<Vec<ExternalRegisterLine>, PlatformError> {
    let rows = sqlx::query(
        "SELECT line_id, source_row, pay_type, occurred_on, amount_minor, scale,
                category, vendor, description, true_up_on, completed,
                step_transfer, step_billpay, step_pay
         FROM external_register_line
         ORDER BY CASE WHEN source_row IS NULL THEN 0 ELSE 1 END,
                  source_row ASC,
                  occurred_on DESC",
    )
    .fetch_all(pool)
    .await
    .map_err(map_sql)?;
    rows.iter().map(row_to_line).collect()
}

async fn rewrite_names(pool: &SqlitePool) -> Result<(), PlatformError> {
    let all = load_all(pool).await?;
    for line in all {
        let pay_type = normalize_pay_type(&line.pay_type);
        let category = normalize_category(&line.category);
        let vendor = normalize_vendor(&line.vendor);
        if pay_type == line.pay_type && category == line.category && vendor == line.vendor {
            continue;
        }
        sqlx::query(
            "UPDATE external_register_line
             SET pay_type = ?1, category = ?2, vendor = ?3
             WHERE line_id = ?4",
        )
        .bind(&pay_type)
        .bind(&category)
        .bind(&vendor)
        .bind(line.line_id.to_string())
        .execute(pool)
        .await
        .map_err(map_sql)?;
    }
    Ok(())
}

pub async fn get(
    pool: &SqlitePool,
    search: Option<String>,
) -> Result<ExternalRegisterGetBody, PlatformError> {
    rewrite_names(pool).await?;
    let all = load_all(pool).await?;
    let query = search.unwrap_or_default();
    let lines = all
        .iter()
        .filter(|line| line_matches(line, &query))
        .cloned()
        .collect::<Vec<_>>();
    Ok(ExternalRegisterGetBody {
        pay_types: distinct(all.iter().map(|line| line.pay_type.clone())),
        categories: distinct(all.iter().map(|line| line.category.clone())),
        vendors: distinct(all.iter().map(|line| line.vendor.clone())),
        total_count: all.len() as u64,
        lines,
    })
}

pub async fn save(
    pool: &SqlitePool,
    lines: Vec<ExternalRegisterLine>,
) -> Result<ExternalRegisterGetBody, PlatformError> {
    let mut tx = pool.begin().await.map_err(map_sql)?;
    for line in lines {
        sqlx::query(
            "INSERT INTO external_register_line (
                line_id, source_row, pay_type, occurred_on, amount_minor, scale,
                category, vendor, description, true_up_on, completed
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
             ON CONFLICT(line_id) DO UPDATE SET
                source_row = excluded.source_row,
                pay_type = excluded.pay_type,
                occurred_on = excluded.occurred_on,
                amount_minor = excluded.amount_minor,
                scale = excluded.scale,
                category = excluded.category,
                vendor = excluded.vendor,
                description = excluded.description,
                true_up_on = CASE
                    WHEN external_register_line.true_up_on IS NOT NULL
                         AND trim(external_register_line.true_up_on) <> ''
                    THEN external_register_line.true_up_on
                    ELSE excluded.true_up_on
                END,
                completed = CASE
                    WHEN external_register_line.completed = 1 THEN 1
                    ELSE excluded.completed
                END",
        )
        .bind(line.line_id.to_string())
        .bind(line.source_row)
        .bind(line.pay_type)
        .bind(line.occurred_on)
        .bind(line.amount_minor)
        .bind(i64::from(line.scale))
        .bind(line.category)
        .bind(line.vendor)
        .bind(line.description)
        .bind(line.true_up_on.clone())
        .bind(i64::from(line.completed || line.true_up_on.is_some()))
        .execute(&mut *tx)
        .await
        .map_err(map_sql)?;
    }
    tx.commit().await.map_err(map_sql)?;
    get(pool, None).await
}

pub async fn true_up(
    pool: &SqlitePool,
    line_ids: Vec<Uuid>,
    true_up_on: Option<String>,
) -> Result<ExternalRegisterGetBody, PlatformError> {
    let mut tx = pool.begin().await.map_err(map_sql)?;
    for line_id in line_ids {
        sqlx::query(
            "UPDATE external_register_line
             SET completed = 1,
                 true_up_on = CASE
                     WHEN true_up_on IS NOT NULL AND trim(true_up_on) <> '' THEN true_up_on
                     ELSE ?2
                 END
             WHERE line_id = ?1",
        )
        .bind(line_id.to_string())
        .bind(true_up_on.clone())
        .execute(&mut *tx)
        .await
        .map_err(map_sql)?;
    }
    tx.commit().await.map_err(map_sql)?;
    get(pool, None).await
}

pub async fn mark_step(
    pool: &SqlitePool,
    line_ids: Vec<Uuid>,
    step: &str,
) -> Result<ExternalRegisterGetBody, PlatformError> {
    if !matches!(step, "transfer" | "billpay" | "pay") {
        return Err(PlatformError::new(
            "bad_step",
            "step must be transfer, billpay, or pay",
        ));
    }
    let mut tx = pool.begin().await.map_err(map_sql)?;
    for line_id in line_ids {
        sqlx::query(
            "UPDATE external_register_line
             SET step_transfer = CASE WHEN ?1 = 'transfer' THEN 1 ELSE step_transfer END,
                 step_billpay = CASE WHEN ?2 = 'billpay' THEN 1 ELSE step_billpay END,
                 step_pay = CASE WHEN ?3 = 'pay' THEN 1 ELSE step_pay END
             WHERE line_id = ?4",
        )
        .bind(step)
        .bind(step)
        .bind(step)
        .bind(line_id.to_string())
        .execute(&mut *tx)
        .await
        .map_err(map_sql)?;
        sqlx::query(
            "UPDATE external_register_line
             SET completed = 1
             WHERE line_id = ?1
               AND step_transfer = 1
               AND step_billpay = 1
               AND step_pay = 1",
        )
        .bind(line_id.to_string())
        .execute(&mut *tx)
        .await
        .map_err(map_sql)?;
    }
    tx.commit().await.map_err(map_sql)?;
    get(pool, None).await
}

fn excel_serial_to_iso(serial: f64) -> String {
    let days = serial.trunc() as i64;
    let epoch = NaiveDate::from_ymd_opt(1899, 12, 30).expect("epoch");
    (epoch + Duration::days(days))
        .format("%Y-%m-%d")
        .to_string()
}

fn cell_text(value: &Data) -> String {
    match value {
        Data::Empty | Data::Error(_) => String::new(),
        Data::String(text) => text.trim().to_string(),
        Data::Float(n) => {
            if n.fract().abs() < 1e-9 {
                format!("{}", *n as i64)
            } else {
                format!("{n}")
            }
        }
        Data::Int(n) => n.to_string(),
        Data::Bool(flag) => {
            if *flag {
                "1".into()
            } else {
                "0".into()
            }
        }
        other => other.to_string().trim().to_string(),
    }
}

fn cell_date(value: &Data) -> Option<String> {
    match value {
        Data::Empty | Data::Error(_) => None,
        Data::DateTime(dt) => Some(excel_serial_to_iso(dt.as_f64())),
        Data::DateTimeIso(text) => {
            let day = text.chars().take(10).collect::<String>();
            if day.is_empty() {
                None
            } else {
                Some(day)
            }
        }
        Data::Float(n) => Some(excel_serial_to_iso(*n)),
        Data::Int(n) => Some(excel_serial_to_iso(*n as f64)),
        Data::String(text) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                None
            } else if trimmed.len() >= 10 && trimmed.as_bytes().get(4) == Some(&b'-') {
                Some(trimmed.chars().take(10).collect())
            } else {
                None
            }
        }
        _ => None,
    }
}

fn cell_amount(value: &Data) -> Option<f64> {
    match value {
        Data::Float(n) => Some(*n),
        Data::Int(n) => Some(*n as f64),
        Data::String(text) => text.trim().replace(',', "").parse().ok(),
        _ => None,
    }
}

fn header_key(value: &Data) -> String {
    cell_text(value).trim().to_lowercase()
}

pub async fn import_workbook(
    pool: &SqlitePool,
    path: &Path,
) -> Result<ExternalRegisterGetBody, PlatformError> {
    let mut workbook: Xlsx<_> = open_workbook(path).map_err(|err| {
        PlatformError::new(
            "external_register_import",
            format!("{}: {err}", path.display()),
        )
    })?;
    let range = workbook
        .worksheet_range_at(0)
        .ok_or_else(|| PlatformError::new("external_register_import", "workbook has no sheet"))?
        .map_err(|err| PlatformError::new("external_register_import", err.to_string()))?;
    let mut rows = range.rows();
    let header = rows
        .next()
        .ok_or_else(|| PlatformError::new("external_register_import", "workbook is empty"))?;
    let mut col = BTreeMap::<String, usize>::new();
    for (index, cell) in header.iter().enumerate() {
        let key = header_key(cell);
        if !key.is_empty() {
            col.insert(key, index);
        }
    }
    let idx = |name: &str| {
        col.get(name).copied().ok_or_else(|| {
            PlatformError::new(
                "external_register_import",
                format!("missing column {name}"),
            )
        })
    };
    let pay_i = idx("pay type")?;
    let date_i = idx("date")?;
    let spent_i = idx("total spent")?;
    let cat_i = idx("category")?;
    let vendor_i = idx("vendor")?;
    let desc_i = idx("description")?;
    let true_i = idx("true up")?;

    let mut prepared = Vec::new();
    for (offset, row) in rows.enumerate() {
        let source_row = (offset + 2) as i64;
        let cell = |index: usize| row.get(index).unwrap_or(&Data::Empty);
        let pay_type = normalize_pay_type(&cell_text(cell(pay_i)));
        let category = normalize_category(&cell_text(cell(cat_i)));
        let vendor = normalize_vendor(&cell_text(cell(vendor_i)));
        let description = cell_text(cell(desc_i));
        let amount = cell_amount(cell(spent_i));
        let occurred_on = fixed_occurred_on(source_row)
            .map(|day| day.to_string())
            .or_else(|| cell_date(cell(date_i)));
        let true_up_on = cell_date(cell(true_i));
        if pay_type.is_empty()
            && category.is_empty()
            && vendor.is_empty()
            && description.is_empty()
            && amount.is_none()
            && occurred_on.is_none()
            && true_up_on.is_none()
        {
            continue;
        }
        prepared.push(ExternalRegisterLine {
            line_id: Uuid::new_v4(),
            source_row: Some(source_row),
            pay_type,
            occurred_on,
            amount_minor: amount.map(amount_to_minor).unwrap_or(0),
            scale: 2,
            category,
            vendor,
            description,
            true_up_on: true_up_on.clone(),
            completed: true_up_on.is_some(),
            step_transfer: true_up_on.is_some(),
            step_billpay: true_up_on.is_some(),
            step_pay: true_up_on.is_some(),
        });
    }

    let mut tx = pool.begin().await.map_err(map_sql)?;
    sqlx::query("DELETE FROM external_register_line WHERE source_row IS NOT NULL")
        .execute(&mut *tx)
        .await
        .map_err(map_sql)?;
    for line in prepared {
        sqlx::query(
            "INSERT INTO external_register_line (
                line_id, source_row, pay_type, occurred_on, amount_minor, scale,
                category, vendor, description, true_up_on, completed
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        )
        .bind(line.line_id.to_string())
        .bind(line.source_row)
        .bind(line.pay_type)
        .bind(line.occurred_on)
        .bind(line.amount_minor)
        .bind(i64::from(line.scale))
        .bind(line.category)
        .bind(line.vendor)
        .bind(line.description)
        .bind(line.true_up_on.clone())
        .bind(i64::from(line.completed))
        .execute(&mut *tx)
        .await
        .map_err(map_sql)?;
    }
    tx.commit().await.map_err(map_sql)?;
    get(pool, None).await
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use application_core::external_register::line_matches;
    use application_core::ports::canonical::Canonical;

    use crate::platform::LocalPlatform;

    #[tokio::test]
    #[ignore]
    async fn import_owner_workbook() {
        let dir = PathBuf::from(std::env::var("LOCALAPPDATA").unwrap()).join("com.finos.desktop");
        let platform = LocalPlatform::open(&dir).await.unwrap();
        let body = platform
            .external_register_import(
                r"C:\Users\EVTom\Documents\Financial\External account tracker.xlsx".into(),
            )
            .await
            .unwrap();
        assert!(body.total_count >= 3480, "rows {}", body.total_count);
        let grow = body
            .lines
            .iter()
            .filter(|line| line_matches(line, "grow"))
            .count();
        assert!(grow >= 100, "grow rows {grow}");
        assert!(body.lines.iter().any(|line| {
            line.source_row == Some(23) && line.occurred_on.as_deref() == Some("2026-09-16")
        }));
        assert!(body.lines.iter().any(|line| {
            line.source_row == Some(800) && line.occurred_on.as_deref() == Some("2025-09-27")
        }));
    }

    #[tokio::test]
    #[ignore]
    async fn normalize_live_names() {
        let dir = PathBuf::from(std::env::var("LOCALAPPDATA").unwrap()).join("com.finos.desktop");
        let platform = LocalPlatform::open(&dir).await.unwrap();
        let body = platform.external_register_get(None).await.unwrap();
        let open = body.lines.iter().filter(|line| line.true_up_on.is_none()).count();
        let done = body.lines.iter().filter(|line| line.true_up_on.is_some()).count();
        assert!(open > 400, "open {open}");
        assert!(done > 2500, "complete {done}");
        assert!(body.categories.iter().any(|name| name == "Food"));
        assert!(!body.categories.iter().any(|name| name == "food"));
        assert!(body.pay_types.iter().any(|name| name == "UCARD"));
        assert!(!body.pay_types.iter().any(|name| name.eq_ignore_ascii_case("ucard") && name != "UCARD"));
        assert!(body.pay_types.iter().any(|name| name == "Checking"));
        assert!(body.pay_types.iter().any(|name| name == "CAP"));
        assert!(body.vendors.iter().any(|name| name == "AMZ"));
        assert!(!body.vendors.iter().any(|name| name.eq_ignore_ascii_case("amz") && name != "AMZ"));
    }
}
