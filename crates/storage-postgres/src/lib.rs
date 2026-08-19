//! PostgreSQL adapter — central evolution path (ARCH-01, ADR-0003).
//! Desktop Profile A stays on `storage-sqlite` until a later cutover.

use std::path::Path;
use std::str::FromStr;

use application_core::contracts::{
    AccountRecord, ActivityRecord, DividendActual, DividendGetBody, LotRecord, MagiProjection,
    MagiTaxPaymentBody, PositionDetailsBody, PositionLineBody, ReconcileCounts, SecurityRecord,
    APP_VERSION, CALCULATION_VERSION, SCHEMA_VERSION, DeviceConfig,
};
use application_core::ports::canonical::Canonical;
use application_core::ports::platform::{Platform, PlatformError};
use async_trait::async_trait;
use financial_domain::activity::prepare_activity;
use financial_domain::error::DomainError;
use financial_domain::lot::{prepare_lot_open, LotOrigin};
use financial_domain::money::Money;
use financial_domain::position::{rollup_open_positions, LotPositionInput};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{PgPool, Row};
use thiserror::Error;
use uuid::Uuid;

mod import_snapshot;
mod magi;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("sqlx: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("migrate: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),
    #[error("{0}")]
    Message(String),
}

#[derive(Clone)]
pub struct PostgresPlatform {
    pool: PgPool,
}

pub(crate) fn map_err(err: StorageError) -> PlatformError {
    PlatformError::new("storage_error", err.to_string())
}

fn domain_err(err: DomainError) -> PlatformError {
    let code = match err {
        DomainError::UnknownAmount => "unknown_amount",
        DomainError::DuplicatePost => "duplicate_post",
        DomainError::FifoNotAssumed => "fifo_not_assumed",
        DomainError::ZeroCostDripNotCrf => "zero_cost_drip_not_crf",
        DomainError::InsufficientLotQuantity => "insufficient_lot_quantity",
        DomainError::InvalidLotOrigin => "invalid_lot_origin",
        DomainError::ScaleMismatch => "scale_mismatch",
    };
    PlatformError::new(code, err.to_string())
}

fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

impl PostgresPlatform {
    pub async fn connect(database_url: &str) -> Result<Self, StorageError> {
        let options = PgConnectOptions::from_str(database_url)
            .map_err(|e| StorageError::Message(e.to_string()))?;
        let pool = PgPoolOptions::new()
            .max_connections(8)
            .connect_with(options)
            .await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
        Ok(Self { pool })
    }

    pub async fn reset_contract_tables(&self) -> Result<(), StorageError> {
        sqlx::query(
            "TRUNCATE TABLE magi_adjustment, magi_fact, magi_coverage, magi_rule,
                            lot, dividend_actual, dividend_declaration, activity_event,
                            audit_record, command_audit, security, account",
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn record_command_audit(
        &self,
        user_sub: &str,
        device_id: &str,
        correlation_id: Uuid,
        command_name: &str,
        ok: bool,
        error_code: Option<&str>,
    ) -> Result<(), StorageError> {
        sqlx::query(
            "INSERT INTO command_audit (
                audit_id, user_sub, device_id, correlation_id, command_name, ok, error_code, recorded_at
             ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(user_sub)
        .bind(device_id)
        .bind(correlation_id.to_string())
        .bind(command_name)
        .bind(if ok { 1i32 } else { 0i32 })
        .bind(error_code)
        .bind(now_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_command_audits(&self) -> Result<Vec<CommandAuditRow>, StorageError> {
        let rows = sqlx::query(
            "SELECT audit_id, user_sub, device_id, correlation_id, command_name, ok, error_code, recorded_at
             FROM command_audit
             ORDER BY recorded_at, audit_id",
        )
        .fetch_all(&self.pool)
        .await?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let ok: i32 = row.try_get("ok")?;
            out.push(CommandAuditRow {
                audit_id: row.try_get("audit_id")?,
                user_sub: row.try_get("user_sub")?,
                device_id: row.try_get("device_id")?,
                correlation_id: row.try_get("correlation_id")?,
                command_name: row.try_get("command_name")?,
                ok: ok != 0,
                error_code: row.try_get("error_code")?,
                recorded_at: row.try_get("recorded_at")?,
            });
        }
        Ok(out)
    }
}

#[derive(Debug, Clone)]
pub struct CommandAuditRow {
    pub audit_id: String,
    pub user_sub: String,
    pub device_id: String,
    pub correlation_id: String,
    pub command_name: String,
    pub ok: bool,
    pub error_code: Option<String>,
    pub recorded_at: String,
}

async fn audit(pool: &PgPool, action: &str, entity_type: &str, entity_id: &str) -> Result<(), PlatformError> {
    sqlx::query(
        "INSERT INTO audit_record (audit_id, action, entity_type, entity_id, recorded_at)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(action)
    .bind(entity_type)
    .bind(entity_id)
    .bind(now_rfc3339())
    .execute(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    Ok(())
}

fn parse_uuid(row: &sqlx::postgres::PgRow, col: &str) -> Result<Uuid, PlatformError> {
    Uuid::parse_str(&row.try_get::<String, _>(col).map_err(|e| map_err(e.into()))?)
        .map_err(|e| PlatformError::new("parse_error", e.to_string()))
}

fn opt_uuid(row: &sqlx::postgres::PgRow, col: &str) -> Result<Option<Uuid>, PlatformError> {
    let v: Option<String> = row.try_get(col).map_err(|e| map_err(e.into()))?;
    v.map(|s| Uuid::parse_str(&s).map_err(|e| PlatformError::new("parse_error", e.to_string())))
        .transpose()
}

async fn remember_dividend_actual(pool: &PgPool, record: &ActivityRecord) -> Result<(), PlatformError> {
    if !record.activity_type.eq_ignore_ascii_case("dividend") {
        return Ok(());
    }
    sqlx::query(
        "INSERT INTO dividend_actual (
            actual_id, account_id, security_id, occurred_on, amount_minor, scale,
            activity_id, idempotency_key
         ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         ON CONFLICT (idempotency_key) DO NOTHING",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(record.account_id.to_string())
    .bind(record.security_id.map(|id| id.to_string()))
    .bind(&record.occurred_on)
    .bind(record.amount_minor)
    .bind(record.scale as i32)
    .bind(record.activity_id.to_string())
    .bind(&record.idempotency_key)
    .execute(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    Ok(())
}

async fn insert_activity(pool: &PgPool, record: &ActivityRecord) -> Result<(), PlatformError> {
    let result = sqlx::query(
        "INSERT INTO activity_event (
            activity_id, account_id, security_id, activity_type, amount_minor, scale,
            occurred_on, corrects_activity_id, import_batch_id, idempotency_key
         ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
    )
    .bind(record.activity_id.to_string())
    .bind(record.account_id.to_string())
    .bind(record.security_id.map(|id| id.to_string()))
    .bind(&record.activity_type)
    .bind(record.amount_minor)
    .bind(record.scale as i32)
    .bind(&record.occurred_on)
    .bind(record.corrects_activity_id.map(|id| id.to_string()))
    .bind(record.import_batch_id.map(|id| id.to_string()))
    .bind(&record.idempotency_key)
    .execute(pool)
    .await;
    match result {
        Ok(_) => Ok(()),
        Err(sqlx::Error::Database(db)) if db.is_unique_violation() => Err(PlatformError::new(
            "duplicate_post",
            "duplicate posting is not allowed",
        )),
        Err(e) => Err(map_err(e.into())),
    }
}

fn account_from_row(row: &sqlx::postgres::PgRow) -> Result<AccountRecord, PlatformError> {
    Ok(AccountRecord {
        account_id: parse_uuid(row, "account_id")?,
        name: row.try_get("name").map_err(|e| map_err(e.into()))?,
        kind: row.try_get("kind").map_err(|e| map_err(e.into()))?,
        row_version: row.try_get::<i32, _>("row_version").map_err(|e| map_err(e.into()))? as i64,
    })
}

fn security_from_row(row: &sqlx::postgres::PgRow) -> Result<SecurityRecord, PlatformError> {
    Ok(SecurityRecord {
        security_id: parse_uuid(row, "security_id")?,
        symbol: row.try_get("symbol").map_err(|e| map_err(e.into()))?,
        name: row.try_get("name").map_err(|e| map_err(e.into()))?,
    })
}

fn lot_from_row(row: &sqlx::postgres::PgRow) -> Result<LotRecord, PlatformError> {
    let crf: i32 = row.try_get("crf_zero_cost").map_err(|e| map_err(e.into()))?;
    Ok(LotRecord {
        lot_id: parse_uuid(row, "lot_id")?,
        account_id: parse_uuid(row, "account_id")?,
        security_id: parse_uuid(row, "security_id")?,
        opened_on: row.try_get("opened_on").map_err(|e| map_err(e.into()))?,
        origin: row.try_get("origin").map_err(|e| map_err(e.into()))?,
        quantity_minor: row.try_get("quantity_minor").map_err(|e| map_err(e.into()))?,
        remaining_quantity_minor: row
            .try_get("remaining_quantity_minor")
            .map_err(|e| map_err(e.into()))?,
        quantity_scale: row.try_get::<i32, _>("quantity_scale").map_err(|e| map_err(e.into()))? as u8,
        performance_basis_minor: row
            .try_get("performance_basis_minor")
            .map_err(|e| map_err(e.into()))?,
        tax_basis_minor: row.try_get("tax_basis_minor").map_err(|e| map_err(e.into()))?,
        remaining_performance_minor: row
            .try_get("remaining_performance_minor")
            .map_err(|e| map_err(e.into()))?,
        remaining_tax_minor: row
            .try_get("remaining_tax_minor")
            .map_err(|e| map_err(e.into()))?,
        scale: row.try_get::<i32, _>("scale").map_err(|e| map_err(e.into()))? as u8,
        crf_zero_cost: crf != 0,
        opening_activity_id: opt_uuid(row, "opening_activity_id")?,
    })
}

#[async_trait]
impl Platform for PostgresPlatform {
    async fn writes_allowed(&self) -> Result<bool, PlatformError> {
        Ok(true)
    }

    async fn config_get(&self) -> Result<DeviceConfig, PlatformError> {
        Ok(DeviceConfig {
            device_id: Uuid::nil(),
            device_name: "postgres-adapter".into(),
            database_id: Uuid::nil(),
            schema_version: SCHEMA_VERSION.to_string(),
            calculation_version: CALCULATION_VERSION.to_string(),
            app_version: APP_VERSION.to_string(),
        })
    }
}

#[async_trait]
impl Canonical for PostgresPlatform {
    async fn account_register(
        &self,
        name: String,
        kind: String,
    ) -> Result<AccountRecord, PlatformError> {
        let record = AccountRecord {
            account_id: Uuid::new_v4(),
            name,
            kind,
            row_version: 1,
        };
        sqlx::query("INSERT INTO account (account_id, name, kind, row_version) VALUES ($1, $2, $3, 1)")
            .bind(record.account_id.to_string())
            .bind(&record.name)
            .bind(&record.kind)
            .execute(&self.pool)
            .await
            .map_err(|e| map_err(e.into()))?;
        audit(&self.pool, "AccountRegister", "account", &record.account_id.to_string()).await?;
        Ok(record)
    }

    async fn account_get(&self, account_id: Uuid) -> Result<AccountRecord, PlatformError> {
        let row = sqlx::query("SELECT account_id, name, kind, row_version FROM account WHERE account_id = $1")
            .bind(account_id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| map_err(e.into()))?
            .ok_or_else(|| PlatformError::new("not_found", "account not found"))?;
        account_from_row(&row)
    }

    async fn account_update(
        &self,
        account_id: Uuid,
        name: Option<String>,
        kind: Option<String>,
        expected_version: Option<i64>,
    ) -> Result<AccountRecord, PlatformError> {
        let mut current = self.account_get(account_id).await?;
        if let Some(name) = name {
            current.name = name;
        }
        if let Some(kind) = kind {
            current.kind = kind;
        }
        let result = if let Some(expected) = expected_version {
            sqlx::query(
                "UPDATE account SET name = $1, kind = $2, row_version = row_version + 1
                 WHERE account_id = $3 AND row_version = $4",
            )
            .bind(&current.name)
            .bind(&current.kind)
            .bind(account_id.to_string())
            .bind(expected as i32)
            .execute(&self.pool)
            .await
            .map_err(|e| map_err(e.into()))?
        } else {
            sqlx::query(
                "UPDATE account SET name = $1, kind = $2, row_version = row_version + 1
                 WHERE account_id = $3",
            )
            .bind(&current.name)
            .bind(&current.kind)
            .bind(account_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| map_err(e.into()))?
        };
        if result.rows_affected() == 0 {
            return Err(PlatformError::new(
                "concurrency_conflict",
                "account version did not match",
            ));
        }
        audit(&self.pool, "AccountUpdate", "account", &account_id.to_string()).await?;
        self.account_get(account_id).await
    }

    async fn security_register(
        &self,
        symbol: String,
        name: String,
    ) -> Result<SecurityRecord, PlatformError> {
        let record = SecurityRecord {
            security_id: Uuid::new_v4(),
            symbol,
            name,
        };
        sqlx::query("INSERT INTO security (security_id, symbol, name) VALUES ($1, $2, $3)")
            .bind(record.security_id.to_string())
            .bind(&record.symbol)
            .bind(&record.name)
            .execute(&self.pool)
            .await
            .map_err(|e| map_err(e.into()))?;
        audit(
            &self.pool,
            "SecurityRegister",
            "security",
            &record.security_id.to_string(),
        )
        .await?;
        Ok(record)
    }

    async fn security_get(&self, security_id: Uuid) -> Result<SecurityRecord, PlatformError> {
        let row = sqlx::query("SELECT security_id, symbol, name FROM security WHERE security_id = $1")
            .bind(security_id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| map_err(e.into()))?
            .ok_or_else(|| PlatformError::new("not_found", "security not found"))?;
        security_from_row(&row)
    }

    async fn activity_post(
        &self,
        account_id: Uuid,
        security_id: Option<Uuid>,
        activity_type: String,
        amount_minor: Option<i64>,
        scale: u8,
        occurred_on: String,
        corrects_activity_id: Option<Uuid>,
        import_batch_id: Option<Uuid>,
        idempotency_key_opt: Option<String>,
    ) -> Result<ActivityRecord, PlatformError> {
        let prepared = prepare_activity(
            account_id,
            security_id,
            activity_type,
            amount_minor,
            scale,
            occurred_on,
            corrects_activity_id,
        )
        .map_err(domain_err)?;
        let key = idempotency_key_opt.unwrap_or_else(|| Uuid::new_v4().to_string());
        let record = ActivityRecord {
            activity_id: Uuid::new_v4(),
            account_id: prepared.account_id,
            security_id: prepared.security_id,
            activity_type: prepared.activity_type,
            amount_minor: prepared.amount.amount_minor,
            scale: prepared.amount.scale,
            occurred_on: prepared.occurred_on,
            corrects_activity_id: prepared.corrects_activity_id,
            import_batch_id,
            idempotency_key: key,
        };
        match insert_activity(&self.pool, &record).await {
            Ok(()) => {
                remember_dividend_actual(&self.pool, &record).await?;
                audit(
                    &self.pool,
                    "ActivityPost",
                    "activity_event",
                    &record.activity_id.to_string(),
                )
                .await?;
                Ok(record)
            }
            Err(err) if err.code == "duplicate_post" => {
                let row = sqlx::query(
                    "SELECT activity_id, account_id, security_id, activity_type, amount_minor, scale,
                            occurred_on, corrects_activity_id, import_batch_id, idempotency_key
                     FROM activity_event WHERE idempotency_key = $1",
                )
                .bind(&record.idempotency_key)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| map_err(e.into()))?;
                let existing = ActivityRecord {
                    activity_id: parse_uuid(&row, "activity_id")?,
                    account_id: parse_uuid(&row, "account_id")?,
                    security_id: opt_uuid(&row, "security_id")?,
                    activity_type: row.try_get("activity_type").map_err(|e| map_err(e.into()))?,
                    amount_minor: row.try_get("amount_minor").map_err(|e| map_err(e.into()))?,
                    scale: row.try_get::<i32, _>("scale").map_err(|e| map_err(e.into()))? as u8,
                    occurred_on: row.try_get("occurred_on").map_err(|e| map_err(e.into()))?,
                    corrects_activity_id: opt_uuid(&row, "corrects_activity_id")?,
                    import_batch_id: opt_uuid(&row, "import_batch_id")?,
                    idempotency_key: row
                        .try_get("idempotency_key")
                        .map_err(|e| map_err(e.into()))?,
                };
                remember_dividend_actual(&self.pool, &existing).await?;
                Ok(existing)
            }
            Err(err) => Err(err),
        }
    }

    async fn dividend_actual_record(
        &self,
        account_id: Uuid,
        security_id: Option<Uuid>,
        occurred_on: String,
        amount_minor: Option<i64>,
        scale: u8,
        idempotency_key: Option<String>,
    ) -> Result<DividendActual, PlatformError> {
        let posted = self
            .activity_post(
                account_id,
                security_id,
                "dividend".into(),
                amount_minor,
                scale,
                occurred_on,
                None,
                None,
                idempotency_key,
            )
            .await?;
        Ok(DividendActual {
            actual_id: posted.activity_id,
            account_id: posted.account_id,
            security_id: posted.security_id,
            occurred_on: posted.occurred_on,
            amount_minor: posted.amount_minor,
            scale: posted.scale,
            activity_id: Some(posted.activity_id),
        })
    }

    async fn dividend_get(&self) -> Result<DividendGetBody, PlatformError> {
        let actual_rows = sqlx::query(
            "SELECT actual_id, account_id, security_id, occurred_on, amount_minor, scale, activity_id
             FROM dividend_actual ORDER BY occurred_on, actual_id",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| map_err(e.into()))?;
        let actuals: Result<Vec<DividendActual>, PlatformError> = actual_rows
            .iter()
            .map(|row| {
                Ok(DividendActual {
                    actual_id: parse_uuid(row, "actual_id")?,
                    account_id: parse_uuid(row, "account_id")?,
                    security_id: opt_uuid(row, "security_id")?,
                    occurred_on: row.try_get("occurred_on").map_err(|e| map_err(e.into()))?,
                    amount_minor: row.try_get("amount_minor").map_err(|e| map_err(e.into()))?,
                    scale: row.try_get::<i32, _>("scale").map_err(|e| map_err(e.into()))? as u8,
                    activity_id: opt_uuid(row, "activity_id")?,
                })
            })
            .collect();
        let actuals = actuals?;
        let actual_total_minor = actuals.iter().map(|a| a.amount_minor).sum();
        Ok(DividendGetBody {
            actuals,
            declarations: Vec::new(),
            actual_total_minor,
            scale: 2,
        })
    }

    async fn lot_open(
        &self,
        account_id: Uuid,
        security_id: Uuid,
        opened_on: String,
        origin: String,
        quantity_minor: i64,
        quantity_scale: u8,
        performance_basis_minor: i64,
        tax_basis_minor: i64,
        scale: u8,
        opening_activity_id: Option<Uuid>,
    ) -> Result<LotRecord, PlatformError> {
        let account = self.account_get(account_id).await?;
        let parsed_origin = LotOrigin::parse(&origin).map_err(domain_err)?;
        let spec = prepare_lot_open(
            &account.kind,
            parsed_origin,
            quantity_minor,
            quantity_scale,
            Money {
                amount_minor: performance_basis_minor,
                scale,
            },
            Money {
                amount_minor: tax_basis_minor,
                scale,
            },
        )
        .map_err(domain_err)?;
        let record = LotRecord {
            lot_id: Uuid::new_v4(),
            account_id,
            security_id,
            opened_on,
            origin: spec.origin.as_str().to_string(),
            quantity_minor: spec.quantity_minor,
            remaining_quantity_minor: spec.quantity_minor,
            quantity_scale: spec.quantity_scale,
            performance_basis_minor: spec.basis.performance.amount_minor,
            tax_basis_minor: spec.basis.tax.amount_minor,
            remaining_performance_minor: spec.basis.performance.amount_minor,
            remaining_tax_minor: spec.basis.tax.amount_minor,
            scale: spec.basis.performance.scale,
            crf_zero_cost: spec.crf_zero_cost,
            opening_activity_id,
        };
        sqlx::query(
            "INSERT INTO lot (
                lot_id, account_id, security_id, opened_on, origin, quantity_minor,
                remaining_quantity_minor, quantity_scale, performance_basis_minor, tax_basis_minor,
                remaining_performance_minor, remaining_tax_minor, scale, crf_zero_cost,
                opening_activity_id
             ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)",
        )
        .bind(record.lot_id.to_string())
        .bind(record.account_id.to_string())
        .bind(record.security_id.to_string())
        .bind(&record.opened_on)
        .bind(&record.origin)
        .bind(record.quantity_minor)
        .bind(record.remaining_quantity_minor)
        .bind(record.quantity_scale as i32)
        .bind(record.performance_basis_minor)
        .bind(record.tax_basis_minor)
        .bind(record.remaining_performance_minor)
        .bind(record.remaining_tax_minor)
        .bind(record.scale as i32)
        .bind(if record.crf_zero_cost { 1i32 } else { 0 })
        .bind(record.opening_activity_id.map(|id| id.to_string()))
        .execute(&self.pool)
        .await
        .map_err(|e| map_err(e.into()))?;
        audit(&self.pool, "LotOpen", "lot", &record.lot_id.to_string()).await?;
        Ok(record)
    }

    async fn position_details_get(&self) -> Result<PositionDetailsBody, PlatformError> {
        let rows = sqlx::query(
            "SELECT lot_id, account_id, security_id, opened_on, origin, quantity_minor,
                    remaining_quantity_minor, quantity_scale, performance_basis_minor, tax_basis_minor,
                    remaining_performance_minor, remaining_tax_minor, scale, crf_zero_cost,
                    opening_activity_id
             FROM lot ORDER BY opened_on, lot_id",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| map_err(e.into()))?;
        let lots: Result<Vec<LotRecord>, PlatformError> = rows.iter().map(lot_from_row).collect();
        let lots = lots?;
        let inputs: Vec<LotPositionInput> = lots
            .iter()
            .map(|l| LotPositionInput {
                account_id: l.account_id,
                security_id: l.security_id,
                remaining_quantity_minor: l.remaining_quantity_minor,
                remaining_performance_minor: l.remaining_performance_minor,
                remaining_tax_minor: l.remaining_tax_minor,
                quantity_scale: l.quantity_scale,
                scale: l.scale,
            })
            .collect();
        let rolled = rollup_open_positions(&inputs);
        let mut positions = Vec::with_capacity(rolled.len());
        for line in rolled {
            let account = self.account_get(line.account_id).await?;
            let security = self.security_get(line.security_id).await?;
            positions.push(PositionLineBody {
                account_id: line.account_id,
                account_name: account.name,
                security_id: line.security_id,
                symbol: security.symbol,
                remaining_quantity_minor: line.remaining_quantity_minor,
                quantity_scale: line.quantity_scale,
                remaining_performance_minor: line.remaining_performance_minor,
                remaining_tax_minor: line.remaining_tax_minor,
                lot_count: line.lot_count,
                scale: line.scale,
            });
        }
        let open_performance_minor = positions
            .iter()
            .map(|p| p.remaining_performance_minor)
            .sum();
        let open_tax_minor = positions.iter().map(|p| p.remaining_tax_minor).sum();
        Ok(PositionDetailsBody {
            positions,
            open_performance_minor,
            open_tax_minor,
            scale: 2,
        })
    }

    async fn magi_rule_set(
        &self,
        threshold_minor: i64,
        safety_reserve_minor: i64,
        scale: u8,
    ) -> Result<MagiProjection, PlatformError> {
        magi::rule_set(&self.pool, threshold_minor, safety_reserve_minor, scale).await?;
        magi::projection(&self.pool).await
    }

    async fn magi_fact_record(
        &self,
        source_id: String,
        treatment: String,
        amount_minor: i64,
        scale: u8,
        category: String,
    ) -> Result<MagiProjection, PlatformError> {
        magi::fact_record(
            &self.pool,
            source_id,
            treatment,
            amount_minor,
            scale,
            category,
        )
        .await?;
        magi::projection(&self.pool).await
    }

    async fn magi_coverage_set(
        &self,
        completeness: String,
        remaining_minor: i64,
        withholding_minor: i64,
        form_total_minor: i64,
        warnings: Vec<String>,
    ) -> Result<MagiProjection, PlatformError> {
        magi::coverage_set(
            &self.pool,
            completeness,
            remaining_minor,
            withholding_minor,
            form_total_minor,
            warnings,
        )
        .await?;
        magi::projection(&self.pool).await
    }

    async fn magi_projection_get(&self) -> Result<MagiProjection, PlatformError> {
        magi::projection(&self.pool).await
    }

    async fn magi_tax_payment_get(&self) -> Result<MagiTaxPaymentBody, PlatformError> {
        let amount_minor = magi::withholding_get(&self.pool).await?;
        Ok(MagiTaxPaymentBody {
            amount_minor,
            scale: 2,
        })
    }

    async fn magi_adjustment_record(
        &self,
        adjustment_id: String,
        amount_minor: i64,
        scale: u8,
        status: String,
        reason: String,
    ) -> Result<MagiProjection, PlatformError> {
        magi::adjustment_record(
            &self.pool,
            adjustment_id,
            amount_minor,
            scale,
            status,
            reason,
        )
        .await?;
        magi::projection(&self.pool).await
    }

    async fn reconcile_counts(&self) -> Result<ReconcileCounts, PlatformError> {
        import_snapshot::reconcile_counts(&self.pool).await
    }

    async fn snapshot_import_sqlite(
        &self,
        sqlite_path: String,
    ) -> Result<ReconcileCounts, PlatformError> {
        import_snapshot::import_sqlite_snapshot(&self.pool, Path::new(&sqlite_path)).await
    }
}
