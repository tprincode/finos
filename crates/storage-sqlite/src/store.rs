//! Open SQLite in WAL mode and persist platform identity / snapshot head.

use std::path::{Path, PathBuf};
use std::time::Duration;

use application_core::contracts::{
    DeviceConfig, SnapshotIdentity, APP_VERSION, CALCULATION_VERSION, SCHEMA_VERSION,
};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::{Row, SqlitePool};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("sqlx: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("migrate: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Message(String),
}

pub struct LocalDatabase {
    pub path: PathBuf,
    pub pool: SqlitePool,
}

impl LocalDatabase {
    pub async fn open(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let pool = connect(&path).await?;
        Ok(Self { path, pool })
    }

    pub async fn journal_mode(&self) -> Result<String, StorageError> {
        journal_mode(&self.pool).await
    }
}

pub async fn connect(path: &Path) -> Result<SqlitePool, StorageError> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .busy_timeout(Duration::from_secs(5));
    let pool = SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(options)
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}

pub async fn journal_mode(pool: &SqlitePool) -> Result<String, StorageError> {
    let mode: String = sqlx::query_scalar("PRAGMA journal_mode")
        .fetch_one(pool)
        .await?;
    Ok(mode)
}

pub async fn checkpoint(pool: &SqlitePool) -> Result<(), StorageError> {
    sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
        .execute(pool)
        .await?;
    Ok(())
}

pub fn default_device_name() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "unknown-device".to_string())
}

pub async fn get_or_create_device(pool: &SqlitePool) -> Result<DeviceConfig, StorageError> {
    if let Some(existing) = fetch_device(pool).await? {
        return Ok(existing);
    }
    let config = DeviceConfig {
        device_id: Uuid::new_v4(),
        device_name: default_device_name(),
        database_id: Uuid::new_v4(),
        schema_version: SCHEMA_VERSION.to_string(),
        calculation_version: CALCULATION_VERSION.to_string(),
        app_version: APP_VERSION.to_string(),
    };
    sqlx::query(
        "INSERT INTO device (singleton, device_id, device_name, database_id) VALUES (1, ?, ?, ?)",
    )
    .bind(config.device_id.to_string())
    .bind(&config.device_name)
    .bind(config.database_id.to_string())
    .execute(pool)
    .await?;
    Ok(config)
}

pub async fn set_device_name(
    pool: &SqlitePool,
    device_name: &str,
) -> Result<DeviceConfig, StorageError> {
    let mut config = get_or_create_device(pool).await?;
    sqlx::query("UPDATE device SET device_name = ? WHERE singleton = 1")
        .bind(device_name)
        .execute(pool)
        .await?;
    config.device_name = device_name.to_string();
    Ok(config)
}

async fn fetch_device(pool: &SqlitePool) -> Result<Option<DeviceConfig>, StorageError> {
    let row = sqlx::query(
        "SELECT device_id, device_name, database_id FROM device WHERE singleton = 1",
    )
    .fetch_optional(pool)
    .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let device_id: String = row.try_get("device_id")?;
    let device_name: String = row.try_get("device_name")?;
    let database_id: String = row.try_get("database_id")?;
    Ok(Some(DeviceConfig {
        device_id: parse_uuid(&device_id)?,
        device_name,
        database_id: parse_uuid(&database_id)?,
        schema_version: SCHEMA_VERSION.to_string(),
        calculation_version: CALCULATION_VERSION.to_string(),
        app_version: APP_VERSION.to_string(),
    }))
}

pub async fn write_device(
    pool: &SqlitePool,
    device_id: Uuid,
    device_name: &str,
    database_id: Uuid,
) -> Result<(), StorageError> {
    sqlx::query(
        "INSERT INTO device (singleton, device_id, device_name, database_id) VALUES (1, ?, ?, ?)
         ON CONFLICT(singleton) DO UPDATE SET device_id = excluded.device_id, device_name = excluded.device_name, database_id = excluded.database_id",
    )
    .bind(device_id.to_string())
    .bind(device_name)
    .bind(database_id.to_string())
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn local_head(pool: &SqlitePool) -> Result<Option<SnapshotIdentity>, StorageError> {
    let row = sqlx::query(
        "SELECT snapshot_id, parent_snapshot_id, change_sequence, last_event_at,
                schema_version, calculation_version, app_version, database_hash,
                evidence_manifest_hash, validation_status, restore_test_status
         FROM snapshot_head WHERE singleton = 1",
    )
    .fetch_optional(pool)
    .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let device = get_or_create_device(pool).await?;
    let snapshot_id: String = row.try_get("snapshot_id")?;
    let parent: Option<String> = row.try_get("parent_snapshot_id")?;
    let change_sequence: i64 = row.try_get("change_sequence")?;
    Ok(Some(SnapshotIdentity {
        database_id: device.database_id,
        snapshot_id: parse_uuid(&snapshot_id)?,
        parent_snapshot_id: parent.as_deref().map(parse_uuid).transpose()?,
        device_id: device.device_id,
        device_name: device.device_name,
        change_sequence: change_sequence as u64,
        last_event_at: row.try_get("last_event_at")?,
        schema_version: row.try_get("schema_version")?,
        calculation_version: row.try_get("calculation_version")?,
        app_version: row.try_get("app_version")?,
        database_hash: row.try_get("database_hash")?,
        evidence_manifest_hash: row.try_get("evidence_manifest_hash")?,
        validation_status: row.try_get("validation_status")?,
        restore_test_status: row.try_get("restore_test_status")?,
    }))
}

pub async fn set_local_head(
    pool: &SqlitePool,
    head: &SnapshotIdentity,
) -> Result<(), StorageError> {
    sqlx::query(
        "INSERT INTO snapshot_head (
            singleton, snapshot_id, parent_snapshot_id, change_sequence, last_event_at,
            schema_version, calculation_version, app_version, database_hash,
            evidence_manifest_hash, validation_status, restore_test_status
         ) VALUES (1, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(singleton) DO UPDATE SET
            snapshot_id = excluded.snapshot_id,
            parent_snapshot_id = excluded.parent_snapshot_id,
            change_sequence = excluded.change_sequence,
            last_event_at = excluded.last_event_at,
            schema_version = excluded.schema_version,
            calculation_version = excluded.calculation_version,
            app_version = excluded.app_version,
            database_hash = excluded.database_hash,
            evidence_manifest_hash = excluded.evidence_manifest_hash,
            validation_status = excluded.validation_status,
            restore_test_status = excluded.restore_test_status",
    )
    .bind(head.snapshot_id.to_string())
    .bind(head.parent_snapshot_id.map(|id| id.to_string()))
    .bind(head.change_sequence as i64)
    .bind(&head.last_event_at)
    .bind(&head.schema_version)
    .bind(&head.calculation_version)
    .bind(&head.app_version)
    .bind(&head.database_hash)
    .bind(&head.evidence_manifest_hash)
    .bind(&head.validation_status)
    .bind(&head.restore_test_status)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn review_ack(pool: &SqlitePool) -> Result<Option<Uuid>, StorageError> {
    let row = sqlx::query(
        "SELECT acknowledged, published_snapshot_id FROM handoff_review WHERE singleton = 1",
    )
    .fetch_optional(pool)
    .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let acknowledged: i64 = row.try_get("acknowledged")?;
    if acknowledged == 0 {
        return Ok(None);
    }
    let published: Option<String> = row.try_get("published_snapshot_id")?;
    published.as_deref().map(parse_uuid).transpose()
}

pub async fn set_review_ack(
    pool: &SqlitePool,
    published_snapshot_id: Option<Uuid>,
) -> Result<(), StorageError> {
    let ack = i64::from(published_snapshot_id.is_some());
    sqlx::query(
        "INSERT INTO handoff_review (singleton, acknowledged, published_snapshot_id) VALUES (1, ?, ?)
         ON CONFLICT(singleton) DO UPDATE SET
            acknowledged = excluded.acknowledged,
            published_snapshot_id = excluded.published_snapshot_id",
    )
    .bind(ack)
    .bind(published_snapshot_id.map(|id| id.to_string()))
    .execute(pool)
    .await?;
    Ok(())
}

fn parse_uuid(value: &str) -> Result<Uuid, StorageError> {
    Uuid::parse_str(value).map_err(|e| StorageError::Message(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn migrates_temp_db_and_enables_wal() {
        let dir = tempfile::tempdir().unwrap();
        let db = LocalDatabase::open(dir.path().join("finos.sqlite"))
            .await
            .unwrap();
        assert_eq!(db.journal_mode().await.unwrap().to_lowercase(), "wal");
        let first = get_or_create_device(&db.pool).await.unwrap();
        let second = get_or_create_device(&db.pool).await.unwrap();
        assert_eq!(first.device_id, second.device_id);
        assert_eq!(first.database_id, second.database_id);
    }
}
