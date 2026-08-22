//! Registries, evidence, import, ledger, audit, exceptions (table-isolated functions).

use std::fs;

use application_core::contracts::{
    AccountRecord, ActivityRecord, AuditRecord, BasisGetBody, BrokerLotReconcileBody,
    CanonicalWeekBody, DividendActual, DividendDeclaration, DividendGetBody, EvidenceRecord,
    ExceptionRecord, ImportBatchRecord, ImportCandidate, IncomePlanBody, LotAssignmentRecord,
    LotRecommendBody, LotRecord, MagiProjection, MagiTaxPaymentBody, ReconcileCounts, RoiBody,
    SecurityRecord, AllocationGetBody, AiRunListBody, AiRunRecord, BacktestGetBody, BurndownBody,
    CalculatorPlanBody, CartGetBody, ClassificationReviewGetBody, DistributionGetBody,
    PlanHistoryRecord, PositionCharacteristicRecord, PositionDetailsBody, PositionLineBody,
    TaxProjectionBody,
};
use application_core::ports::canonical::Canonical;
use application_core::ports::platform::PlatformError;
use async_trait::async_trait;
use chrono::NaiveDate;
use financial_domain::activity::{idempotency_key, prepare_activity};
use financial_domain::advisory::prepare_advisory;
use financial_domain::error::DomainError;
use financial_domain::lot::{
    automatic_drip_capture_allowed, consume_lot, prepare_lot_open, recommend_lowest_cost_first,
    require_explicit_lot, zero_cost_drip_allowed, LotCostView, LotOrigin,
};
use financial_domain::money::Money;
use financial_domain::position::{rollup_open_positions, LotPositionInput};
use financial_domain::week::week_containing;
use import_engine::{detect_broker, document_key, parse_broker_csv, require_candidate_amount};
use sha2::{Digest, Sha256};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::platform::LocalPlatform;
use crate::store::StorageError;

fn map_err(err: StorageError) -> PlatformError {
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
        DomainError::PlanConfirmBlocked => "plan_confirm_blocked",
        DomainError::IncompleteAnalysisRequired => "incomplete_analysis_required",
        DomainError::MissingDeclarationSource => "missing_declaration_source",
        DomainError::NonpositivePrice => "nonpositive_price",
    };
    PlatformError::new(code, err.to_string())
}

fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

async fn audit(
    pool: &SqlitePool,
    action: &str,
    entity_type: &str,
    entity_id: &str,
) -> Result<(), PlatformError> {
    sqlx::query(
        "INSERT INTO audit_record (audit_id, action, entity_type, entity_id, recorded_at)
         VALUES (?, ?, ?, ?, ?)",
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

async fn raise_exception(pool: &SqlitePool, code: &str, message: &str) -> Result<(), PlatformError> {
    sqlx::query(
        "INSERT INTO app_exception (exception_id, code, message, acknowledged, created_at)
         VALUES (?, ?, ?, 0, ?)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(code)
    .bind(message)
    .bind(now_rfc3339())
    .execute(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    Ok(())
}

async fn count_sql(pool: &SqlitePool, sql: &str) -> Result<u64, PlatformError> {
    let n: i64 = sqlx::query_scalar(sql)
        .fetch_one(pool)
        .await
        .map_err(|e| map_err(e.into()))?;
    Ok(n as u64)
}

fn account_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<AccountRecord, PlatformError> {
    Ok(AccountRecord {
        account_id: Uuid::parse_str(&row.try_get::<String, _>("account_id").map_err(|e| map_err(e.into()))?)
            .map_err(|e| PlatformError::new("parse_error", e.to_string()))?,
        name: row.try_get("name").map_err(|e| map_err(e.into()))?,
        kind: row.try_get("kind").map_err(|e| map_err(e.into()))?,
        row_version: 1,
    })
}

fn security_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<SecurityRecord, PlatformError> {
    Ok(SecurityRecord {
        security_id: Uuid::parse_str(
            &row.try_get::<String, _>("security_id")
                .map_err(|e| map_err(e.into()))?,
        )
        .map_err(|e| PlatformError::new("parse_error", e.to_string()))?,
        symbol: row.try_get("symbol").map_err(|e| map_err(e.into()))?,
        name: row.try_get("name").map_err(|e| map_err(e.into()))?,
        crf: row.try_get::<i64, _>("crf").map_err(|e| map_err(e.into()))? != 0,
    })
}

fn activity_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<ActivityRecord, PlatformError> {
    let parse = |col: &str| -> Result<Uuid, PlatformError> {
        Uuid::parse_str(&row.try_get::<String, _>(col).map_err(|e| map_err(e.into()))?)
            .map_err(|e| PlatformError::new("parse_error", e.to_string()))
    };
    let opt_uuid = |col: &str| -> Result<Option<Uuid>, PlatformError> {
        let v: Option<String> = row.try_get(col).map_err(|e| map_err(e.into()))?;
        v.map(|s| Uuid::parse_str(&s).map_err(|e| PlatformError::new("parse_error", e.to_string())))
            .transpose()
    };
    Ok(ActivityRecord {
        activity_id: parse("activity_id")?,
        account_id: parse("account_id")?,
        security_id: opt_uuid("security_id")?,
        activity_type: row.try_get("activity_type").map_err(|e| map_err(e.into()))?,
        amount_minor: row.try_get("amount_minor").map_err(|e| map_err(e.into()))?,
        scale: row.try_get::<i64, _>("scale").map_err(|e| map_err(e.into()))? as u8,
        occurred_on: row.try_get("occurred_on").map_err(|e| map_err(e.into()))?,
        corrects_activity_id: opt_uuid("corrects_activity_id")?,
        import_batch_id: opt_uuid("import_batch_id")?,
        idempotency_key: row.try_get("idempotency_key").map_err(|e| map_err(e.into()))?,
    })
}

fn lot_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<LotRecord, PlatformError> {
    let parse = |col: &str| -> Result<Uuid, PlatformError> {
        Uuid::parse_str(&row.try_get::<String, _>(col).map_err(|e| map_err(e.into()))?)
            .map_err(|e| PlatformError::new("parse_error", e.to_string()))
    };
    let opt_uuid = |col: &str| -> Result<Option<Uuid>, PlatformError> {
        let v: Option<String> = row.try_get(col).map_err(|e| map_err(e.into()))?;
        v.map(|s| Uuid::parse_str(&s).map_err(|e| PlatformError::new("parse_error", e.to_string())))
            .transpose()
    };
    let crf: i64 = row.try_get("crf_zero_cost").map_err(|e| map_err(e.into()))?;
    Ok(LotRecord {
        lot_id: parse("lot_id")?,
        account_id: parse("account_id")?,
        security_id: parse("security_id")?,
        opened_on: row.try_get("opened_on").map_err(|e| map_err(e.into()))?,
        origin: row.try_get("origin").map_err(|e| map_err(e.into()))?,
        quantity_minor: row.try_get("quantity_minor").map_err(|e| map_err(e.into()))?,
        remaining_quantity_minor: row
            .try_get("remaining_quantity_minor")
            .map_err(|e| map_err(e.into()))?,
        quantity_scale: row.try_get::<i64, _>("quantity_scale").map_err(|e| map_err(e.into()))? as u8,
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
        scale: row.try_get::<i64, _>("scale").map_err(|e| map_err(e.into()))? as u8,
        crf_zero_cost: crf != 0,
        opening_activity_id: opt_uuid("opening_activity_id")?,
    })
}

fn assignment_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<LotAssignmentRecord, PlatformError> {
    let parse = |col: &str| -> Result<Uuid, PlatformError> {
        Uuid::parse_str(&row.try_get::<String, _>(col).map_err(|e| map_err(e.into()))?)
            .map_err(|e| PlatformError::new("parse_error", e.to_string()))
    };
    Ok(LotAssignmentRecord {
        assignment_id: parse("assignment_id")?,
        lot_id: parse("lot_id")?,
        activity_id: parse("activity_id")?,
        quantity_minor: row.try_get("quantity_minor").map_err(|e| map_err(e.into()))?,
        quantity_scale: row.try_get::<i64, _>("quantity_scale").map_err(|e| map_err(e.into()))? as u8,
        proceeds_minor: row.try_get("proceeds_minor").map_err(|e| map_err(e.into()))?,
        performance_cost_minor: row
            .try_get("performance_cost_minor")
            .map_err(|e| map_err(e.into()))?,
        tax_cost_minor: row.try_get("tax_cost_minor").map_err(|e| map_err(e.into()))?,
        scale: row.try_get::<i64, _>("scale").map_err(|e| map_err(e.into()))? as u8,
    })
}

async fn fetch_batch(pool: &SqlitePool, batch_id: Uuid) -> Result<ImportBatchRecord, PlatformError> {
    let row = sqlx::query(
        "SELECT batch_id, source_id, content_hash, status FROM import_batch WHERE batch_id = ?",
    )
    .bind(batch_id.to_string())
    .fetch_optional(pool)
    .await
    .map_err(|e| map_err(e.into()))?
    .ok_or_else(|| PlatformError::new("not_found", "import batch not found"))?;
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM import_candidate WHERE batch_id = ?")
        .bind(batch_id.to_string())
        .fetch_one(pool)
        .await
        .map_err(|e| map_err(e.into()))?;
    Ok(ImportBatchRecord {
        batch_id: Uuid::parse_str(&row.try_get::<String, _>("batch_id").map_err(|e| map_err(e.into()))?)
            .map_err(|e| PlatformError::new("parse_error", e.to_string()))?,
        source_id: row.try_get("source_id").map_err(|e| map_err(e.into()))?,
        content_hash: row.try_get("content_hash").map_err(|e| map_err(e.into()))?,
        status: row.try_get("status").map_err(|e| map_err(e.into()))?,
        candidate_count: count as u64,
    })
}

async fn remember_dividend_actual(
    pool: &SqlitePool,
    record: &ActivityRecord,
) -> Result<(), PlatformError> {
    if !record.activity_type.eq_ignore_ascii_case("dividend") {
        return Ok(());
    }
    sqlx::query(
        "INSERT OR IGNORE INTO dividend_actual (
            actual_id, account_id, security_id, occurred_on, amount_minor, scale,
            activity_id, idempotency_key
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(record.account_id.to_string())
    .bind(record.security_id.map(|id| id.to_string()))
    .bind(&record.occurred_on)
    .bind(record.amount_minor)
    .bind(record.scale as i64)
    .bind(record.activity_id.to_string())
    .bind(&record.idempotency_key)
    .execute(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    Ok(())
}

async fn insert_activity(
    pool: &SqlitePool,
    record: &ActivityRecord,
) -> Result<(), PlatformError> {
    let result = sqlx::query(
        "INSERT INTO activity_event (
            activity_id, account_id, security_id, activity_type, amount_minor, scale,
            occurred_on, corrects_activity_id, import_batch_id, idempotency_key
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(record.activity_id.to_string())
    .bind(record.account_id.to_string())
    .bind(record.security_id.map(|id| id.to_string()))
    .bind(&record.activity_type)
    .bind(record.amount_minor)
    .bind(record.scale as i64)
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

#[async_trait]
impl Canonical for LocalPlatform {
    async fn account_register(
        &self,
        name: String,
        kind: String,
    ) -> Result<AccountRecord, PlatformError> {
        let pool = self.pool.read().await;
        let record = AccountRecord {
            account_id: Uuid::new_v4(),
            name,
            kind,
            row_version: 1,
        };
        sqlx::query("INSERT INTO account (account_id, name, kind) VALUES (?, ?, ?)")
            .bind(record.account_id.to_string())
            .bind(&record.name)
            .bind(&record.kind)
            .execute(&*pool)
            .await
            .map_err(|e| map_err(e.into()))?;
        audit(&pool, "AccountRegister", "account", &record.account_id.to_string()).await?;
        Ok(record)
    }

    async fn account_update(
        &self,
        account_id: Uuid,
        name: Option<String>,
        kind: Option<String>,
        _expected_version: Option<i64>,
    ) -> Result<AccountRecord, PlatformError> {
        let mut current = self.account_get(account_id).await?;
        if let Some(name) = name {
            current.name = name;
        }
        if let Some(kind) = kind {
            current.kind = kind;
        }
        let pool = self.pool.read().await;
        sqlx::query("UPDATE account SET name = ?, kind = ? WHERE account_id = ?")
            .bind(&current.name)
            .bind(&current.kind)
            .bind(account_id.to_string())
            .execute(&*pool)
            .await
            .map_err(|e| map_err(e.into()))?;
        audit(&pool, "AccountUpdate", "account", &account_id.to_string()).await?;
        Ok(current)
    }

    async fn account_get(&self, account_id: Uuid) -> Result<AccountRecord, PlatformError> {
        let pool = self.pool.read().await;
        let row = sqlx::query("SELECT account_id, name, kind FROM account WHERE account_id = ?")
            .bind(account_id.to_string())
            .fetch_optional(&*pool)
            .await
            .map_err(|e| map_err(e.into()))?
            .ok_or_else(|| PlatformError::new("not_found", "account not found"))?;
        account_from_row(&row)
    }

    async fn account_list(&self) -> Result<Vec<AccountRecord>, PlatformError> {
        let pool = self.pool.read().await;
        let rows = sqlx::query("SELECT account_id, name, kind FROM account ORDER BY name")
            .fetch_all(&*pool)
            .await
            .map_err(|e| map_err(e.into()))?;
        rows.iter().map(account_from_row).collect()
    }

    async fn security_register(
        &self,
        symbol: String,
        name: String,
        crf: bool,
    ) -> Result<SecurityRecord, PlatformError> {
        let pool = self.pool.read().await;
        let record = SecurityRecord {
            security_id: Uuid::new_v4(),
            symbol,
            name,
            crf,
        };
        sqlx::query("INSERT INTO security (security_id, symbol, name, crf) VALUES (?, ?, ?, ?)")
            .bind(record.security_id.to_string())
            .bind(&record.symbol)
            .bind(&record.name)
            .bind(if record.crf { 1i64 } else { 0 })
            .execute(&*pool)
            .await
            .map_err(|e| map_err(e.into()))?;
        audit(
            &pool,
            "SecurityRegister",
            "security",
            &record.security_id.to_string(),
        )
        .await?;
        Ok(record)
    }

    async fn security_update(
        &self,
        security_id: Uuid,
        name: Option<String>,
        symbol: Option<String>,
    ) -> Result<SecurityRecord, PlatformError> {
        let mut current = self.security_get(security_id).await?;
        if let Some(name) = name {
            if name.trim().is_empty() {
                return Err(PlatformError::new("missing_name", "security name required"));
            }
            current.name = name.trim().to_string();
        }
        if let Some(symbol) = symbol {
            if symbol.trim().is_empty() {
                return Err(PlatformError::new("missing_symbol", "symbol required"));
            }
            current.symbol = symbol.trim().to_ascii_uppercase();
        }
        let pool = self.pool.read().await;
        sqlx::query("UPDATE security SET name = ?, symbol = ? WHERE security_id = ?")
            .bind(&current.name)
            .bind(&current.symbol)
            .bind(security_id.to_string())
            .execute(&*pool)
            .await
            .map_err(|e| map_err(e.into()))?;
        audit(&pool, "SecurityUpdate", "security", &security_id.to_string()).await?;
        Ok(current)
    }

    async fn security_get(&self, security_id: Uuid) -> Result<SecurityRecord, PlatformError> {
        let pool = self.pool.read().await;
        let row = sqlx::query("SELECT security_id, symbol, name, crf FROM security WHERE security_id = ?")
            .bind(security_id.to_string())
            .fetch_optional(&*pool)
            .await
            .map_err(|e| map_err(e.into()))?
            .ok_or_else(|| PlatformError::new("not_found", "security not found"))?;
        security_from_row(&row)
    }

    async fn security_list(&self) -> Result<Vec<SecurityRecord>, PlatformError> {
        let pool = self.pool.read().await;
        let rows = sqlx::query("SELECT security_id, symbol, name, crf FROM security ORDER BY symbol")
            .fetch_all(&*pool)
            .await
            .map_err(|e| map_err(e.into()))?;
        rows.iter().map(security_from_row).collect()
    }

    async fn evidence_store(
        &self,
        filename: String,
        content: Vec<u8>,
    ) -> Result<EvidenceRecord, PlatformError> {
        let hash = sha256_bytes(&content);
        let pool = self.pool.read().await;
        if let Some(row) = sqlx::query(
            "SELECT evidence_id, content_hash, filename FROM evidence WHERE content_hash = ?",
        )
        .bind(&hash)
        .fetch_optional(&*pool)
        .await
        .map_err(|e| map_err(e.into()))?
        {
            return Ok(EvidenceRecord {
                evidence_id: Uuid::parse_str(
                    &row.try_get::<String, _>("evidence_id")
                        .map_err(|e| map_err(e.into()))?,
                )
                .map_err(|e| PlatformError::new("parse_error", e.to_string()))?,
                content_hash: hash,
                filename: row.try_get("filename").map_err(|e| map_err(e.into()))?,
            });
        }
        fs::create_dir_all(&self.evidence_dir).map_err(|e| map_err(e.into()))?;
        fs::write(self.evidence_dir.join(&hash), &content).map_err(|e| map_err(e.into()))?;
        let record = EvidenceRecord {
            evidence_id: Uuid::new_v4(),
            content_hash: hash,
            filename,
        };
        sqlx::query("INSERT INTO evidence (evidence_id, content_hash, filename) VALUES (?, ?, ?)")
            .bind(record.evidence_id.to_string())
            .bind(&record.content_hash)
            .bind(&record.filename)
            .execute(&*pool)
            .await
            .map_err(|e| map_err(e.into()))?;
        audit(
            &pool,
            "EvidenceStore",
            "evidence",
            &record.evidence_id.to_string(),
        )
        .await?;
        Ok(record)
    }

    async fn evidence_get(&self, evidence_id: Uuid) -> Result<EvidenceRecord, PlatformError> {
        let pool = self.pool.read().await;
        let row = sqlx::query(
            "SELECT evidence_id, content_hash, filename FROM evidence WHERE evidence_id = ?",
        )
        .bind(evidence_id.to_string())
        .fetch_optional(&*pool)
        .await
        .map_err(|e| map_err(e.into()))?
        .ok_or_else(|| PlatformError::new("not_found", "evidence not found"))?;
        Ok(EvidenceRecord {
            evidence_id,
            content_hash: row.try_get("content_hash").map_err(|e| map_err(e.into()))?,
            filename: row.try_get("filename").map_err(|e| map_err(e.into()))?,
        })
    }

    async fn import_stage(
        &self,
        source_id: String,
        filename: String,
        content: Vec<u8>,
        mut candidates: Vec<ImportCandidate>,
        default_account: Option<String>,
    ) -> Result<ImportBatchRecord, PlatformError> {
        if candidates.is_empty() {
            let text = String::from_utf8_lossy(&content);
            if let Some(layout) = detect_broker(text.lines().next().unwrap_or("")) {
                candidates = parse_broker_csv(layout, &text, default_account.as_deref())
                    .map_err(domain_err)?;
            }
        }
        let evidence = self.evidence_store(filename, content).await?;
        let _ = document_key(&source_id, &evidence.content_hash);
        let pool = self.pool.read().await;
        if let Some(row) = sqlx::query(
            "SELECT batch_id FROM import_batch WHERE source_id = ? AND content_hash = ?",
        )
        .bind(&source_id)
        .bind(&evidence.content_hash)
        .fetch_optional(&*pool)
        .await
        .map_err(|e| map_err(e.into()))?
        {
            let id = Uuid::parse_str(&row.try_get::<String, _>("batch_id").map_err(|e| map_err(e.into()))?)
                .map_err(|e| PlatformError::new("parse_error", e.to_string()))?;
            return fetch_batch(&pool, id).await;
        }
        let batch_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO import_batch (batch_id, source_id, content_hash, evidence_id, status)
             VALUES (?, ?, ?, ?, 'staged')",
        )
        .bind(batch_id.to_string())
        .bind(&source_id)
        .bind(&evidence.content_hash)
        .bind(evidence.evidence_id.to_string())
        .execute(&*pool)
        .await
        .map_err(|e| map_err(e.into()))?;
        for candidate in &candidates {
            sqlx::query(
                "INSERT INTO import_candidate (
                    candidate_id, batch_id, account_name, symbol, activity_type,
                    amount_minor, scale, occurred_on
                 ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(Uuid::new_v4().to_string())
            .bind(batch_id.to_string())
            .bind(&candidate.account_name)
            .bind(&candidate.symbol)
            .bind(&candidate.activity_type)
            .bind(candidate.amount_minor)
            .bind(candidate.scale as i64)
            .bind(&candidate.occurred_on)
            .execute(&*pool)
            .await
            .map_err(|e| map_err(e.into()))?;
        }
        audit(&pool, "ImportStage", "import_batch", &batch_id.to_string()).await?;
        fetch_batch(&pool, batch_id).await
    }

    async fn import_validate(&self, batch_id: Uuid) -> Result<ImportBatchRecord, PlatformError> {
        let pool = self.pool.read().await;
        let batch = fetch_batch(&pool, batch_id).await?;
        if batch.status == "posted" {
            return Ok(batch);
        }
        let rows = sqlx::query(
            "SELECT account_name, symbol, amount_minor, scale FROM import_candidate WHERE batch_id = ?",
        )
        .bind(batch_id.to_string())
        .fetch_all(&*pool)
        .await
        .map_err(|e| map_err(e.into()))?;
        let mut ok = true;
        for row in &rows {
            let account_name: String = row.try_get("account_name").map_err(|e| map_err(e.into()))?;
            let amount_minor: Option<i64> = row.try_get("amount_minor").map_err(|e| map_err(e.into()))?;
            let scale: i64 = row.try_get("scale").map_err(|e| map_err(e.into()))?;
            if require_candidate_amount(amount_minor, scale as u8).is_err() {
                ok = false;
                raise_exception(
                    &pool,
                    "unknown_amount",
                    &format!("candidate in {batch_id} has unknown amount"),
                )
                .await?;
            }
            let found: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM account WHERE name = ?")
                .bind(&account_name)
                .fetch_one(&*pool)
                .await
                .map_err(|e| map_err(e.into()))?;
            if found == 0 {
                ok = false;
                raise_exception(
                    &pool,
                    "missing_account",
                    &format!("account '{account_name}' is not registered"),
                )
                .await?;
            }
        }
        let status = if ok { "validated" } else { "invalid" };
        sqlx::query("UPDATE import_batch SET status = ? WHERE batch_id = ?")
            .bind(status)
            .bind(batch_id.to_string())
            .execute(&*pool)
            .await
            .map_err(|e| map_err(e.into()))?;
        let _ = batch;
        fetch_batch(&pool, batch_id).await
    }

    async fn import_approve(&self, batch_id: Uuid) -> Result<ImportBatchRecord, PlatformError> {
        let pool = self.pool.read().await;
        let batch = fetch_batch(&pool, batch_id).await?;
        if batch.status == "posted" {
            return Ok(batch);
        }
        if batch.status != "validated" {
            return Err(PlatformError::new(
                "not_validated",
                "ImportApprove requires a validated batch",
            ));
        }
        sqlx::query("UPDATE import_batch SET status = 'approved' WHERE batch_id = ?")
            .bind(batch_id.to_string())
            .execute(&*pool)
            .await
            .map_err(|e| map_err(e.into()))?;
        audit(&pool, "ImportApprove", "import_batch", &batch_id.to_string()).await?;
        fetch_batch(&pool, batch_id).await
    }

    async fn import_post(&self, batch_id: Uuid) -> Result<ImportBatchRecord, PlatformError> {
        struct Pending {
            candidate_id: String,
            account_id: Uuid,
            security_id: Option<Uuid>,
            activity_type: String,
            amount_minor: Option<i64>,
            scale: u8,
            occurred_on: String,
            source_id: String,
            content_hash: String,
        }
        let pending: Vec<Pending> = {
            let pool = self.pool.read().await;
            let batch = fetch_batch(&pool, batch_id).await?;
            if batch.status == "posted" {
                return Ok(batch);
            }
            if batch.status != "approved" {
                return Err(PlatformError::new(
                    "not_approved",
                    "ImportPost requires ImportApprove",
                ));
            }
            let rows = sqlx::query(
                "SELECT candidate_id, account_name, symbol, activity_type, amount_minor, scale, occurred_on
                 FROM import_candidate WHERE batch_id = ? ORDER BY candidate_id",
            )
            .bind(batch_id.to_string())
            .fetch_all(&*pool)
            .await
            .map_err(|e| map_err(e.into()))?;
            let mut pending = Vec::new();
            for row in rows {
                let account_name: String =
                    row.try_get("account_name").map_err(|e| map_err(e.into()))?;
                let symbol: Option<String> = row.try_get("symbol").map_err(|e| map_err(e.into()))?;
                let account_row = sqlx::query("SELECT account_id FROM account WHERE name = ?")
                    .bind(&account_name)
                    .fetch_one(&*pool)
                    .await
                    .map_err(|e| map_err(e.into()))?;
                let account_id = Uuid::parse_str(
                    &account_row
                        .try_get::<String, _>("account_id")
                        .map_err(|e| map_err(e.into()))?,
                )
                .map_err(|e| PlatformError::new("parse_error", e.to_string()))?;
                let security_id = if let Some(sym) = symbol {
                    let sec = sqlx::query("SELECT security_id FROM security WHERE symbol = ?")
                        .bind(&sym)
                        .fetch_optional(&*pool)
                        .await
                        .map_err(|e| map_err(e.into()))?;
                    match sec {
                        Some(sec_row) => Some(
                            Uuid::parse_str(
                                &sec_row
                                    .try_get::<String, _>("security_id")
                                    .map_err(|e| map_err(e.into()))?,
                            )
                            .map_err(|e| PlatformError::new("parse_error", e.to_string()))?,
                        ),
                        None => None,
                    }
                } else {
                    None
                };
                pending.push(Pending {
                    candidate_id: row.try_get("candidate_id").map_err(|e| map_err(e.into()))?,
                    account_id,
                    security_id,
                    activity_type: row.try_get("activity_type").map_err(|e| map_err(e.into()))?,
                    amount_minor: row.try_get("amount_minor").map_err(|e| map_err(e.into()))?,
                    scale: row.try_get::<i64, _>("scale").map_err(|e| map_err(e.into()))? as u8,
                    occurred_on: row.try_get("occurred_on").map_err(|e| map_err(e.into()))?,
                    source_id: batch.source_id.clone(),
                    content_hash: batch.content_hash.clone(),
                });
            }
            pending
        };
        for (index, item) in pending.iter().enumerate() {
            let key = idempotency_key(&item.source_id, &item.content_hash, index as u32);
            let posted = self
                .activity_post(
                    item.account_id,
                    item.security_id,
                    item.activity_type.clone(),
                    item.amount_minor,
                    item.scale,
                    item.occurred_on.clone(),
                    None,
                    Some(batch_id),
                    Some(key),
                )
                .await?;
            let pool = self.pool.read().await;
            sqlx::query("UPDATE import_candidate SET posted_activity_id = ? WHERE candidate_id = ?")
                .bind(posted.activity_id.to_string())
                .bind(&item.candidate_id)
                .execute(&*pool)
                .await
                .map_err(|e| map_err(e.into()))?;
            drop(pool);
            if item.activity_type.eq_ignore_ascii_case("drip") {
                self.maybe_import_drip_lot(&posted).await?;
            }
        }
        let pool = self.pool.read().await;
        sqlx::query("UPDATE import_batch SET status = 'posted' WHERE batch_id = ?")
            .bind(batch_id.to_string())
            .execute(&*pool)
            .await
            .map_err(|e| map_err(e.into()))?;
        audit(&pool, "ImportPost", "import_batch", &batch_id.to_string()).await?;
        fetch_batch(&pool, batch_id).await
    }

    async fn import_batch_get(&self, batch_id: Uuid) -> Result<ImportBatchRecord, PlatformError> {
        let pool = self.pool.read().await;
        fetch_batch(&pool, batch_id).await
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
        let pool = self.pool.read().await;
        match insert_activity(&pool, &record).await {
            Ok(()) => {
                remember_dividend_actual(&pool, &record).await?;
                audit(
                    &pool,
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
                     FROM activity_event WHERE idempotency_key = ?",
                )
                .bind(&record.idempotency_key)
                .fetch_one(&*pool)
                .await
                .map_err(|e| map_err(e.into()))?;
                let existing = activity_from_row(&row)?;
                remember_dividend_actual(&pool, &existing).await?;
                Ok(existing)
            }
            Err(err) => Err(err),
        }
    }

    async fn activity_correct(
        &self,
        activity_id: Uuid,
        amount_minor: Option<i64>,
        scale: u8,
        occurred_on: String,
    ) -> Result<ActivityRecord, PlatformError> {
        let original = self.activity_get(activity_id).await?;
        self.activity_post(
            original.account_id,
            original.security_id,
            original.activity_type,
            amount_minor,
            scale,
            occurred_on,
            Some(activity_id),
            original.import_batch_id,
            None,
        )
        .await
    }

    async fn activity_get(&self, activity_id: Uuid) -> Result<ActivityRecord, PlatformError> {
        let pool = self.pool.read().await;
        let row = sqlx::query(
            "SELECT activity_id, account_id, security_id, activity_type, amount_minor, scale,
                    occurred_on, corrects_activity_id, import_batch_id, idempotency_key
             FROM activity_event WHERE activity_id = ?",
        )
        .bind(activity_id.to_string())
        .fetch_optional(&*pool)
        .await
        .map_err(|e| map_err(e.into()))?
        .ok_or_else(|| PlatformError::new("not_found", "activity not found"))?;
        activity_from_row(&row)
    }

    async fn activity_list(&self) -> Result<Vec<ActivityRecord>, PlatformError> {
        let pool = self.pool.read().await;
        let rows = sqlx::query(
            "SELECT activity_id, account_id, security_id, activity_type, amount_minor, scale,
                    occurred_on, corrects_activity_id, import_batch_id, idempotency_key
             FROM activity_event ORDER BY occurred_on, activity_id",
        )
        .fetch_all(&*pool)
        .await
        .map_err(|e| map_err(e.into()))?;
        rows.iter().map(activity_from_row).collect()
    }

    async fn audit_list(&self) -> Result<Vec<AuditRecord>, PlatformError> {
        let pool = self.pool.read().await;
        let rows = sqlx::query(
            "SELECT audit_id, action, entity_type, entity_id, recorded_at FROM audit_record ORDER BY recorded_at",
        )
        .fetch_all(&*pool)
        .await
        .map_err(|e| map_err(e.into()))?;
        rows.iter()
            .map(|row| {
                Ok(AuditRecord {
                    audit_id: Uuid::parse_str(
                        &row.try_get::<String, _>("audit_id")
                            .map_err(|e| map_err(e.into()))?,
                    )
                    .map_err(|e| PlatformError::new("parse_error", e.to_string()))?,
                    action: row.try_get("action").map_err(|e| map_err(e.into()))?,
                    entity_type: row.try_get("entity_type").map_err(|e| map_err(e.into()))?,
                    entity_id: row.try_get("entity_id").map_err(|e| map_err(e.into()))?,
                    recorded_at: row.try_get("recorded_at").map_err(|e| map_err(e.into()))?,
                })
            })
            .collect()
    }

    async fn exception_acknowledge(
        &self,
        exception_id: Uuid,
    ) -> Result<ExceptionRecord, PlatformError> {
        let pool = self.pool.read().await;
        sqlx::query("UPDATE app_exception SET acknowledged = 1 WHERE exception_id = ?")
            .bind(exception_id.to_string())
            .execute(&*pool)
            .await
            .map_err(|e| map_err(e.into()))?;
        let list = self.exception_list().await?;
        list.into_iter()
            .find(|e| e.exception_id == exception_id)
            .ok_or_else(|| PlatformError::new("not_found", "exception not found"))
    }

    async fn exception_list(&self) -> Result<Vec<ExceptionRecord>, PlatformError> {
        let pool = self.pool.read().await;
        let rows = sqlx::query(
            "SELECT exception_id, code, message, acknowledged, created_at FROM app_exception ORDER BY created_at",
        )
        .fetch_all(&*pool)
        .await
        .map_err(|e| map_err(e.into()))?;
        rows.iter()
            .map(|row| {
                let ack: i64 = row.try_get("acknowledged").map_err(|e| map_err(e.into()))?;
                Ok(ExceptionRecord {
                    exception_id: Uuid::parse_str(
                        &row.try_get::<String, _>("exception_id")
                            .map_err(|e| map_err(e.into()))?,
                    )
                    .map_err(|e| PlatformError::new("parse_error", e.to_string()))?,
                    code: row.try_get("code").map_err(|e| map_err(e.into()))?,
                    message: row.try_get("message").map_err(|e| map_err(e.into()))?,
                    acknowledged: ack != 0,
                    created_at: row.try_get("created_at").map_err(|e| map_err(e.into()))?,
                })
            })
            .collect()
    }

    async fn canonical_week_get(&self, as_of_date: String) -> Result<CanonicalWeekBody, PlatformError> {
        let date = NaiveDate::parse_from_str(&as_of_date, "%Y-%m-%d")
            .map_err(|e| PlatformError::new("invalid_date", e.to_string()))?;
        let week = week_containing(date);
        Ok(CanonicalWeekBody {
            as_of_date,
            start: week.start.format("%Y-%m-%d").to_string(),
            end: week.end.format("%Y-%m-%d").to_string(),
        })
    }

    async fn reconcile_counts(&self) -> Result<ReconcileCounts, PlatformError> {
        let pool = self.pool.read().await;
        let amount: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(amount_minor), 0) FROM activity_event WHERE corrects_activity_id IS NULL",
        )
        .fetch_one(&*pool)
        .await
        .map_err(|e| map_err(e.into()))?;
        Ok(ReconcileCounts {
            accounts: count_sql(&pool, "SELECT COUNT(*) FROM account").await?,
            securities: count_sql(&pool, "SELECT COUNT(*) FROM security").await?,
            evidence: count_sql(&pool, "SELECT COUNT(*) FROM evidence").await?,
            import_batches: count_sql(&pool, "SELECT COUNT(*) FROM import_batch").await?,
            posted_activities: count_sql(
                &pool,
                "SELECT COUNT(*) FROM activity_event WHERE corrects_activity_id IS NULL",
            )
            .await?,
            amount_minor_sum: amount,
            scale: 2,
            audit_records: count_sql(&pool, "SELECT COUNT(*) FROM audit_record").await?,
            exceptions_open: count_sql(
                &pool,
                "SELECT COUNT(*) FROM app_exception WHERE acknowledged = 0",
            )
            .await?,
        })
    }

    async fn dividend_declare(
        &self,
        security_symbol: String,
        declared_on: String,
        amount_minor: i64,
        scale: u8,
    ) -> Result<DividendDeclaration, PlatformError> {
        let pool = self.pool.read().await;
        let record = DividendDeclaration {
            declaration_id: Uuid::new_v4(),
            security_symbol,
            declared_on,
            amount_minor,
            scale,
        };
        sqlx::query(
            "INSERT INTO dividend_declaration (
                declaration_id, security_symbol, declared_on, amount_minor, scale
             ) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(record.declaration_id.to_string())
        .bind(&record.security_symbol)
        .bind(&record.declared_on)
        .bind(record.amount_minor)
        .bind(record.scale as i64)
        .execute(&*pool)
        .await
        .map_err(|e| map_err(e.into()))?;
        audit(
            &pool,
            "DividendDeclare",
            "dividend_declaration",
            &record.declaration_id.to_string(),
        )
        .await?;
        Ok(record)
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
        let pool = self.pool.read().await;
        let actual_rows = sqlx::query(
            "SELECT actual_id, account_id, security_id, occurred_on, amount_minor, scale, activity_id
             FROM dividend_actual ORDER BY occurred_on, actual_id",
        )
        .fetch_all(&*pool)
        .await
        .map_err(|e| map_err(e.into()))?;
        let actuals: Result<Vec<DividendActual>, PlatformError> = actual_rows
            .iter()
            .map(|row| {
                let parse = |col: &str| -> Result<Uuid, PlatformError> {
                    Uuid::parse_str(&row.try_get::<String, _>(col).map_err(|e| map_err(e.into()))?)
                        .map_err(|e| PlatformError::new("parse_error", e.to_string()))
                };
                let opt_uuid = |col: &str| -> Result<Option<Uuid>, PlatformError> {
                    let v: Option<String> = row.try_get(col).map_err(|e| map_err(e.into()))?;
                    v.map(|s| {
                        Uuid::parse_str(&s).map_err(|e| PlatformError::new("parse_error", e.to_string()))
                    })
                    .transpose()
                };
                Ok(DividendActual {
                    actual_id: parse("actual_id")?,
                    account_id: parse("account_id")?,
                    security_id: opt_uuid("security_id")?,
                    occurred_on: row.try_get("occurred_on").map_err(|e| map_err(e.into()))?,
                    amount_minor: row.try_get("amount_minor").map_err(|e| map_err(e.into()))?,
                    scale: row.try_get::<i64, _>("scale").map_err(|e| map_err(e.into()))? as u8,
                    activity_id: opt_uuid("activity_id")?,
                })
            })
            .collect();
        let actuals = actuals?;
        let actual_total_minor = actuals.iter().map(|a| a.amount_minor).sum();
        let decl_rows = sqlx::query(
            "SELECT declaration_id, security_symbol, declared_on, amount_minor, scale
             FROM dividend_declaration ORDER BY declared_on",
        )
        .fetch_all(&*pool)
        .await
        .map_err(|e| map_err(e.into()))?;
        let declarations: Result<Vec<DividendDeclaration>, PlatformError> = decl_rows
            .iter()
            .map(|row| {
                Ok(DividendDeclaration {
                    declaration_id: Uuid::parse_str(
                        &row.try_get::<String, _>("declaration_id")
                            .map_err(|e| map_err(e.into()))?,
                    )
                    .map_err(|e| PlatformError::new("parse_error", e.to_string()))?,
                    security_symbol: row.try_get("security_symbol").map_err(|e| map_err(e.into()))?,
                    declared_on: row.try_get("declared_on").map_err(|e| map_err(e.into()))?,
                    amount_minor: row.try_get("amount_minor").map_err(|e| map_err(e.into()))?,
                    scale: row.try_get::<i64, _>("scale").map_err(|e| map_err(e.into()))? as u8,
                })
            })
            .collect();
        Ok(DividendGetBody {
            actuals,
            declarations: declarations?,
            actual_total_minor,
            scale: 2,
        })
    }

    async fn income_plan_update(
        &self,
        planned_minor: i64,
        scale: u8,
    ) -> Result<IncomePlanBody, PlatformError> {
        let pool = self.pool.read().await;
        sqlx::query(
            "INSERT INTO income_plan (singleton, planned_amount_minor, scale) VALUES (1, ?, ?)
             ON CONFLICT(singleton) DO UPDATE SET
                planned_amount_minor = excluded.planned_amount_minor,
                scale = excluded.scale",
        )
        .bind(planned_minor)
        .bind(scale as i64)
        .execute(&*pool)
        .await
        .map_err(|e| map_err(e.into()))?;
        drop(pool);
        self.income_plan_get().await
    }

    async fn income_plan_get(&self) -> Result<IncomePlanBody, PlatformError> {
        let pool = self.pool.read().await;
        let planned: i64 = sqlx::query_scalar(
            "SELECT COALESCE((SELECT planned_amount_minor FROM income_plan WHERE singleton = 1), 0)",
        )
        .fetch_one(&*pool)
        .await
        .map_err(|e| map_err(e.into()))?;
        Ok(IncomePlanBody {
            planned_minor: planned,
            actual_minor: 0,
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
        is_open: bool,
    ) -> Result<LotRecord, PlatformError> {
        let _account = self.account_get(account_id).await?;
        let security = self.security_get(security_id).await?;
        let parsed_origin = LotOrigin::parse(&origin).map_err(domain_err)?;
        let spec = prepare_lot_open(
            security.crf,
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
        let remaining_quantity = if is_open { spec.quantity_minor } else { 0 };
        let remaining_performance = if is_open {
            spec.basis.performance.amount_minor
        } else {
            0
        };
        let remaining_tax = if is_open { spec.basis.tax.amount_minor } else { 0 };
        let record = LotRecord {
            lot_id: Uuid::new_v4(),
            account_id,
            security_id,
            opened_on,
            origin: spec.origin.as_str().to_string(),
            quantity_minor: spec.quantity_minor,
            remaining_quantity_minor: remaining_quantity,
            quantity_scale: spec.quantity_scale,
            performance_basis_minor: spec.basis.performance.amount_minor,
            tax_basis_minor: spec.basis.tax.amount_minor,
            remaining_performance_minor: remaining_performance,
            remaining_tax_minor: remaining_tax,
            scale: spec.basis.performance.scale,
            crf_zero_cost: spec.crf_zero_cost,
            opening_activity_id,
        };
        let pool = self.pool.read().await;
        sqlx::query(
            "INSERT INTO lot (
                lot_id, account_id, security_id, opened_on, origin, quantity_minor,
                remaining_quantity_minor, quantity_scale, performance_basis_minor, tax_basis_minor,
                remaining_performance_minor, remaining_tax_minor, scale, crf_zero_cost,
                opening_activity_id
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(record.lot_id.to_string())
        .bind(record.account_id.to_string())
        .bind(record.security_id.to_string())
        .bind(&record.opened_on)
        .bind(&record.origin)
        .bind(record.quantity_minor)
        .bind(record.remaining_quantity_minor)
        .bind(record.quantity_scale as i64)
        .bind(record.performance_basis_minor)
        .bind(record.tax_basis_minor)
        .bind(record.remaining_performance_minor)
        .bind(record.remaining_tax_minor)
        .bind(record.scale as i64)
        .bind(if record.crf_zero_cost { 1 } else { 0 })
        .bind(record.opening_activity_id.map(|id| id.to_string()))
        .execute(&*pool)
        .await
        .map_err(|e| map_err(e.into()))?;
        audit(&pool, "LotOpen", "lot", &record.lot_id.to_string()).await?;
        Ok(record)
    }

    async fn lot_assign(
        &self,
        lot_id: Uuid,
        activity_id: Uuid,
        quantity_minor: i64,
        quantity_scale: u8,
    ) -> Result<LotAssignmentRecord, PlatformError> {
        let _ = require_explicit_lot(Some(lot_id)).map_err(domain_err)?;
        let lot = self.lot_get(lot_id).await?;
        if lot.quantity_scale != quantity_scale {
            return Err(domain_err(DomainError::ScaleMismatch));
        }
        let activity = self.activity_get(activity_id).await?;
        let used = consume_lot(
            lot.remaining_quantity_minor,
            lot.remaining_performance_minor,
            lot.remaining_tax_minor,
            quantity_minor,
        )
        .map_err(domain_err)?;
        let pool = self.pool.read().await;
        if let Some(row) = sqlx::query(
            "SELECT assignment_id, lot_id, activity_id, quantity_minor, quantity_scale,
                    proceeds_minor, performance_cost_minor, tax_cost_minor, scale
             FROM lot_assignment WHERE lot_id = ? AND activity_id = ?",
        )
        .bind(lot_id.to_string())
        .bind(activity_id.to_string())
        .fetch_optional(&*pool)
        .await
        .map_err(|e| map_err(e.into()))?
        {
            return assignment_from_row(&row);
        }
        let record = LotAssignmentRecord {
            assignment_id: Uuid::new_v4(),
            lot_id,
            activity_id,
            quantity_minor: used.quantity_minor,
            quantity_scale,
            proceeds_minor: activity.amount_minor,
            performance_cost_minor: used.performance_minor,
            tax_cost_minor: used.tax_minor,
            scale: lot.scale,
        };
        sqlx::query(
            "INSERT INTO lot_assignment (
                assignment_id, lot_id, activity_id, quantity_minor, quantity_scale,
                proceeds_minor, performance_cost_minor, tax_cost_minor, scale
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(record.assignment_id.to_string())
        .bind(record.lot_id.to_string())
        .bind(record.activity_id.to_string())
        .bind(record.quantity_minor)
        .bind(record.quantity_scale as i64)
        .bind(record.proceeds_minor)
        .bind(record.performance_cost_minor)
        .bind(record.tax_cost_minor)
        .bind(record.scale as i64)
        .execute(&*pool)
        .await
        .map_err(|e| map_err(e.into()))?;
        sqlx::query(
            "UPDATE lot SET remaining_quantity_minor = ?, remaining_performance_minor = ?,
                remaining_tax_minor = ? WHERE lot_id = ?",
        )
        .bind(used.remaining_quantity_minor)
        .bind(used.remaining_performance_minor)
        .bind(used.remaining_tax_minor)
        .bind(lot_id.to_string())
        .execute(&*pool)
        .await
        .map_err(|e| map_err(e.into()))?;
        audit(&pool, "LotAssign", "lot_assignment", &record.assignment_id.to_string()).await?;
        Ok(record)
    }

    async fn lot_get(&self, lot_id: Uuid) -> Result<LotRecord, PlatformError> {
        let pool = self.pool.read().await;
        let row = sqlx::query(
            "SELECT lot_id, account_id, security_id, opened_on, origin, quantity_minor,
                    remaining_quantity_minor, quantity_scale, performance_basis_minor, tax_basis_minor,
                    remaining_performance_minor, remaining_tax_minor, scale, crf_zero_cost,
                    opening_activity_id
             FROM lot WHERE lot_id = ?",
        )
        .bind(lot_id.to_string())
        .fetch_optional(&*pool)
        .await
        .map_err(|e| map_err(e.into()))?
        .ok_or_else(|| PlatformError::new("not_found", "lot not found"))?;
        lot_from_row(&row)
    }

    async fn basis_get(&self) -> Result<BasisGetBody, PlatformError> {
        let lots = self.lot_list().await?;
        let open_performance_minor = lots.iter().map(|l| l.remaining_performance_minor).sum();
        let open_tax_minor = lots.iter().map(|l| l.remaining_tax_minor).sum();
        Ok(BasisGetBody {
            lots,
            open_performance_minor,
            open_tax_minor,
            scale: 2,
        })
    }

    async fn roi_get(&self) -> Result<RoiBody, PlatformError> {
        let pool = self.pool.read().await;
        let row = sqlx::query(
            "SELECT COALESCE(SUM(proceeds_minor), 0) AS proceeds,
                    COALESCE(SUM(performance_cost_minor), 0) AS perf,
                    COALESCE(SUM(tax_cost_minor), 0) AS tax
             FROM lot_assignment",
        )
        .fetch_one(&*pool)
        .await
        .map_err(|e| map_err(e.into()))?;
        let proceeds_minor: i64 = row.try_get("proceeds").map_err(|e| map_err(e.into()))?;
        let performance_cost_minor: i64 = row.try_get("perf").map_err(|e| map_err(e.into()))?;
        let tax_cost_minor: i64 = row.try_get("tax").map_err(|e| map_err(e.into()))?;
        drop(pool);
        let basis = self.basis_get().await?;
        Ok(RoiBody {
            proceeds_minor,
            performance_cost_minor,
            tax_cost_minor,
            performance_gain_minor: proceeds_minor - performance_cost_minor,
            tax_gain_minor: proceeds_minor - tax_cost_minor,
            dividend_actual_minor: 0,
            open_performance_minor: basis.open_performance_minor,
            open_tax_minor: basis.open_tax_minor,
            scale: 2,
        })
    }

    async fn lot_recommend(
        &self,
        account_id: Uuid,
        security_id: Uuid,
    ) -> Result<LotRecommendBody, PlatformError> {
        let lots = self.lot_list().await?;
        let views: Vec<LotCostView> = lots
            .into_iter()
            .filter(|l| l.account_id == account_id && l.security_id == security_id)
            .map(|l| LotCostView {
                lot_id: l.lot_id,
                remaining_quantity_minor: l.remaining_quantity_minor,
                remaining_performance_minor: l.remaining_performance_minor,
            })
            .collect();
        Ok(LotRecommendBody {
            lot_ids: recommend_lowest_cost_first(&views),
        })
    }

    async fn broker_lot_reconcile(&self) -> Result<BrokerLotReconcileBody, PlatformError> {
        let pool = self.pool.read().await;
        let sell_ids: Vec<String> = sqlx::query_scalar(
            "SELECT activity_id FROM activity_event
             WHERE LOWER(activity_type) IN ('sell', 'option_close')
               AND corrects_activity_id IS NULL",
        )
        .fetch_all(&*pool)
        .await
        .map_err(|e| map_err(e.into()))?;
        let assigned_ids: Vec<String> = sqlx::query_scalar("SELECT activity_id FROM lot_assignment")
            .fetch_all(&*pool)
            .await
            .map_err(|e| map_err(e.into()))?;
        let assigned_set: std::collections::HashSet<String> = assigned_ids.into_iter().collect();
        let unmatched = sell_ids
            .iter()
            .filter(|id| !assigned_set.contains(*id))
            .count() as u64;
        let assigned_quantity_minor: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(quantity_minor), 0) FROM lot_assignment",
        )
        .fetch_one(&*pool)
        .await
        .map_err(|e| map_err(e.into()))?;
        Ok(BrokerLotReconcileBody {
            sold_quantity_minor: assigned_quantity_minor,
            assigned_quantity_minor,
            unmatched_sells: unmatched,
            matched: unmatched == 0,
            quantity_scale: 0,
        })
    }

    async fn magi_rule_set(
        &self,
        threshold_minor: i64,
        safety_reserve_minor: i64,
        scale: u8,
    ) -> Result<MagiProjection, PlatformError> {
        let pool = self.pool.read().await;
        crate::magi::rule_set(&*pool, threshold_minor, safety_reserve_minor, scale).await?;
        crate::magi::projection(&*pool).await
    }

    async fn magi_fact_record(
        &self,
        source_id: String,
        treatment: String,
        amount_minor: i64,
        scale: u8,
        category: String,
    ) -> Result<MagiProjection, PlatformError> {
        let pool = self.pool.read().await;
        crate::magi::fact_record(&*pool, source_id, treatment, amount_minor, scale, category)
            .await?;
        crate::magi::projection(&*pool).await
    }

    async fn magi_coverage_set(
        &self,
        completeness: String,
        remaining_minor: i64,
        withholding_minor: i64,
        form_total_minor: i64,
        warnings: Vec<String>,
    ) -> Result<MagiProjection, PlatformError> {
        let pool = self.pool.read().await;
        crate::magi::coverage_set(
            &*pool,
            completeness,
            remaining_minor,
            withholding_minor,
            form_total_minor,
            warnings,
        )
        .await?;
        crate::magi::projection(&*pool).await
    }

    async fn magi_projection_get(&self) -> Result<MagiProjection, PlatformError> {
        let pool = self.pool.read().await;
        crate::magi::projection(&*pool).await
    }

    async fn magi_tax_payment_get(&self) -> Result<MagiTaxPaymentBody, PlatformError> {
        let pool = self.pool.read().await;
        let amount_minor = crate::magi::withholding_get(&*pool).await?;
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
        let pool = self.pool.read().await;
        crate::magi::adjustment_record(
            &*pool,
            adjustment_id,
            amount_minor,
            scale,
            status,
            reason,
        )
        .await?;
        crate::magi::projection(&*pool).await
    }

    async fn plan_approve(
        &self,
        remaining_minor: i64,
        scale: u8,
        approved_on: String,
    ) -> Result<CalculatorPlanBody, PlatformError> {
        let pool = self.pool.read().await;
        crate::plan::plan_approve(&*pool, remaining_minor, scale, approved_on).await
    }

    async fn plan_get(&self) -> Result<CalculatorPlanBody, PlatformError> {
        let pool = self.pool.read().await;
        crate::plan::plan_get(&*pool).await
    }

    async fn burndown_get(&self) -> Result<BurndownBody, PlatformError> {
        let pool = self.pool.read().await;
        crate::plan::burndown_get(&*pool).await
    }

    async fn plan_history_record(
        &self,
        security_id: Uuid,
        amount_per_share_minor: i64,
        amount_scale: u8,
        planning_periods_per_year: u8,
        effective_from: String,
        decision_reason: String,
    ) -> Result<PlanHistoryRecord, PlatformError> {
        let pool = self.pool.read().await;
        crate::plan::plan_history_record(
            &*pool,
            security_id,
            amount_per_share_minor,
            amount_scale,
            planning_periods_per_year,
            effective_from,
            decision_reason,
        )
        .await
    }

    async fn plan_history_list(&self) -> Result<Vec<PlanHistoryRecord>, PlatformError> {
        let pool = self.pool.read().await;
        crate::plan::plan_history_list(&*pool).await
    }

    async fn position_characteristic_upsert(
        &self,
        record: PositionCharacteristicRecord,
    ) -> Result<PositionCharacteristicRecord, PlatformError> {
        let pool = self.pool.read().await;
        crate::plan::position_characteristic_upsert(&*pool, record).await
    }

    async fn position_characteristic_list(
        &self,
    ) -> Result<Vec<PositionCharacteristicRecord>, PlatformError> {
        let pool = self.pool.read().await;
        crate::plan::position_characteristic_list(&*pool).await
    }

    async fn plan_history_confirm(
        &self,
        security_id: Uuid,
        amount_per_share_minor: i64,
        amount_scale: u8,
        planning_periods_per_year: u8,
        effective_from: String,
        decision_reason: String,
    ) -> Result<PlanHistoryRecord, PlatformError> {
        let pool = self.pool.read().await;
        crate::plan::plan_history_confirm(
            &*pool,
            security_id,
            amount_per_share_minor,
            amount_scale,
            planning_periods_per_year,
            effective_from,
            decision_reason,
        )
        .await
    }

    async fn issuer_declaration_record(
        &self,
        security_id: Uuid,
        amount_per_share_minor: Option<i64>,
        amount_scale: u8,
        payment_period: String,
        source: String,
        entered_at: String,
    ) -> Result<application_core::contracts::IssuerDeclarationRecord, PlatformError> {
        let pool = self.pool.read().await;
        crate::wizard::issuer_declaration_record(
            &*pool,
            security_id,
            amount_per_share_minor,
            amount_scale,
            payment_period,
            source,
            entered_at,
        )
        .await
    }

    async fn issuer_declaration_list(
        &self,
        security_id: Uuid,
    ) -> Result<Vec<application_core::contracts::IssuerDeclarationRecord>, PlatformError> {
        let pool = self.pool.read().await;
        crate::wizard::issuer_declaration_list(&*pool, security_id).await
    }

    async fn price_quote_record(
        &self,
        security_id: Uuid,
        price_minor: i64,
        scale: u8,
        as_of_at: String,
        source: String,
    ) -> Result<application_core::contracts::PriceQuoteBody, PlatformError> {
        let pool = self.pool.read().await;
        crate::wizard::price_quote_record(&*pool, security_id, price_minor, scale, as_of_at, source)
            .await
    }

    async fn price_quote_list(
        &self,
        security_id: Uuid,
    ) -> Result<Vec<application_core::contracts::PriceQuoteBody>, PlatformError> {
        let pool = self.pool.read().await;
        crate::wizard::price_quote_list(&*pool, security_id).await
    }

    async fn manual_price_override(
        &self,
        security_id: Uuid,
        price_minor: i64,
        scale: u8,
        reason: String,
        effective_from: String,
    ) -> Result<application_core::contracts::CurrentPriceBody, PlatformError> {
        let pool = self.pool.read().await;
        crate::wizard::manual_price_override(
            &*pool,
            security_id,
            price_minor,
            scale,
            reason,
            effective_from,
        )
        .await
    }

    async fn current_price_get(
        &self,
        security_id: Uuid,
        as_of_date: String,
    ) -> Result<application_core::contracts::CurrentPriceBody, PlatformError> {
        let pool = self.pool.read().await;
        crate::wizard::current_price_get(&*pool, security_id, as_of_date).await
    }

    async fn retrieval_template_set(
        &self,
        record: application_core::contracts::RetrievalTemplateRecord,
    ) -> Result<application_core::contracts::RetrievalTemplateRecord, PlatformError> {
        let pool = self.pool.read().await;
        crate::wizard::retrieval_template_set(&*pool, record).await
    }

    async fn retrieval_template_get(
        &self,
        security_id: Uuid,
    ) -> Result<Option<application_core::contracts::RetrievalTemplateRecord>, PlatformError> {
        let pool = self.pool.read().await;
        crate::wizard::retrieval_template_get(&*pool, security_id).await
    }

    async fn price_retrieval_set(
        &self,
    ) -> Result<application_core::contracts::PriceRetrievalSetBody, PlatformError> {
        let pool = self.pool.read().await;
        crate::wizard::price_retrieval_set(&*pool).await
    }

    async fn allocation_target_set(
        &self,
        name: String,
        target_minor: i64,
        scale: u8,
    ) -> Result<AllocationGetBody, PlatformError> {
        let pool = self.pool.read().await;
        crate::allocation::target_set(&*pool, name, target_minor, scale).await
    }

    async fn allocation_get(&self) -> Result<AllocationGetBody, PlatformError> {
        let pool = self.pool.read().await;
        crate::allocation::target_get(&*pool).await
    }

    async fn cart_item_add(
        &self,
        symbol: String,
        quantity_minor: i64,
        quantity_scale: u8,
    ) -> Result<CartGetBody, PlatformError> {
        let pool = self.pool.read().await;
        crate::cart::item_add(&*pool, symbol, quantity_minor, quantity_scale).await
    }

    async fn cart_item_remove(&self, item_id: Uuid) -> Result<CartGetBody, PlatformError> {
        let pool = self.pool.read().await;
        crate::cart::item_remove(&*pool, item_id).await
    }

    async fn cart_get(&self) -> Result<CartGetBody, PlatformError> {
        let pool = self.pool.read().await;
        crate::cart::cart_get(&*pool).await
    }

    async fn backtest_run(
        &self,
        scenario: String,
        hypothetical_pnl_minor: i64,
        scale: u8,
        completed_at: String,
    ) -> Result<BacktestGetBody, PlatformError> {
        let pool = self.pool.read().await;
        crate::backtest::run_stub(&*pool, scenario, hypothetical_pnl_minor, scale, completed_at)
            .await
    }

    async fn backtest_get(&self) -> Result<BacktestGetBody, PlatformError> {
        let pool = self.pool.read().await;
        crate::backtest::run_get(&*pool).await
    }

    async fn classification_review_record(
        &self,
        fact_key: String,
        classification: String,
        status: String,
    ) -> Result<ClassificationReviewGetBody, PlatformError> {
        let pool = self.pool.read().await;
        crate::classification::review_record(&*pool, fact_key, classification, status).await
    }

    async fn classification_review_get(&self) -> Result<ClassificationReviewGetBody, PlatformError> {
        let pool = self.pool.read().await;
        crate::classification::review_get(&*pool).await
    }

    async fn distribution_characterize(
        &self,
        activity_id: Uuid,
        category: String,
        amount_minor: i64,
        scale: u8,
    ) -> Result<DistributionGetBody, PlatformError> {
        let pool = self.pool.read().await;
        crate::distribution::characterize(&*pool, activity_id, category, amount_minor, scale).await
    }

    async fn distribution_get(&self) -> Result<DistributionGetBody, PlatformError> {
        let pool = self.pool.read().await;
        crate::distribution::distribution_get(&*pool).await
    }

    async fn ai_analyze(&self, prompt: String) -> Result<AiRunRecord, PlatformError> {
        let prepared = prepare_advisory(prompt);
        let completion = self.advisory.complete(&prepared.prompt).await?;
        let pool = self.pool.read().await;
        crate::ai::run_insert(
            &*pool,
            prepared.prompt,
            completion.recommendation,
            completion.provider,
            completion.model,
            "completed".to_string(),
        )
        .await
    }

    async fn ai_run_get(&self, run_id: Uuid) -> Result<AiRunRecord, PlatformError> {
        let pool = self.pool.read().await;
        crate::ai::run_get(&*pool, run_id).await
    }

    async fn analysis_run_list(&self) -> Result<AiRunListBody, PlatformError> {
        let pool = self.pool.read().await;
        crate::ai::run_list(&*pool).await
    }

    async fn position_details_get(&self) -> Result<PositionDetailsBody, PlatformError> {
        let lots = self.lot_list().await?;
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

    async fn tax_projection_get(&self) -> Result<TaxProjectionBody, PlatformError> {
        let magi = self.magi_projection_get().await?;
        Ok(TaxProjectionBody {
            source_query: "MagiProjectionGet".into(),
            decision_state: magi.decision_state,
            actual_included_ytd: magi.actual_included_ytd,
            applicable_threshold: magi.applicable_threshold,
            data_completeness: magi.data_completeness,
        })
    }
}

impl LocalPlatform {
    async fn lot_list(&self) -> Result<Vec<LotRecord>, PlatformError> {
        let pool = self.pool.read().await;
        let rows = sqlx::query(
            "SELECT lot_id, account_id, security_id, opened_on, origin, quantity_minor,
                    remaining_quantity_minor, quantity_scale, performance_basis_minor, tax_basis_minor,
                    remaining_performance_minor, remaining_tax_minor, scale, crf_zero_cost,
                    opening_activity_id
             FROM lot ORDER BY opened_on, lot_id",
        )
        .fetch_all(&*pool)
        .await
        .map_err(|e| map_err(e.into()))?;
        rows.iter().map(lot_from_row).collect()
    }

    async fn maybe_import_drip_lot(&self, posted: &ActivityRecord) -> Result<(), PlatformError> {
        let account = self.account_get(posted.account_id).await?;
        let Some(security_id) = posted.security_id else {
            return Ok(());
        };
        let security = self.security_get(security_id).await?;
        if posted.amount_minor == 0 && !zero_cost_drip_allowed(security.crf) {
            let pool = self.pool.read().await;
            raise_exception(
                &pool,
                "zero_cost_drip_not_crf",
                "zero-cost DRIP is not allowed unless the security is CRF",
            )
            .await?;
            return Ok(());
        }
        if posted.amount_minor == 0 && automatic_drip_capture_allowed(&account.kind) {
            self.lot_open(
                posted.account_id,
                security_id,
                posted.occurred_on.clone(),
                "drip".into(),
                1,
                0,
                0,
                0,
                posted.scale,
                Some(posted.activity_id),
                true,
            )
            .await?;
        }
        Ok(())
    }
}
