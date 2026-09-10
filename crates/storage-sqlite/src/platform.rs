//! Compose SQLite identity with the local snapshot catalog (no Tauri).

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use application_core::contracts::{
    DeviceConfig, HandoffDecision, HandoffStatusBody, SnapshotIdentity, APP_VERSION,
    CALCULATION_VERSION, SCHEMA_VERSION,
};
use application_core::handoff::{
    compare_heads, decision_message, writes_allowed, PublishedView,
};
use application_core::ports::advisory::{Advisory, StubAdvisory};
use application_core::ports::platform::{Platform, PlatformError};
use async_trait::async_trait;
use snapshot_service::{PublishedHead, SnapshotCatalog};
use sqlx::SqlitePool;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::store::{
    self, checkpoint, get_or_create_device, local_head, set_device_name, set_local_head,
    set_review_ack, write_device, StorageError,
};

pub struct LocalPlatform {
    pub(crate) db_path: PathBuf,
    pub(crate) catalog: SnapshotCatalog,
    pub(crate) pool: RwLock<SqlitePool>,
    pub(crate) evidence_dir: PathBuf,
    pub(crate) advisory: Arc<dyn Advisory>,
}

impl LocalPlatform {
    pub async fn open(app_dir: impl Into<PathBuf>) -> Result<Self, StorageError> {
        Self::open_with_advisory(app_dir, Arc::new(StubAdvisory)).await
    }

    pub async fn open_with_advisory(
        app_dir: impl Into<PathBuf>,
        advisory: Arc<dyn Advisory>,
    ) -> Result<Self, StorageError> {
        let app_dir = app_dir.into();
        std::fs::create_dir_all(&app_dir)?;
        let db_path = app_dir.join("local.sqlite");
        let pool = store::connect(&db_path).await?;
        let device = get_or_create_device(&pool).await?;
        let _ = device;
        let catalog = SnapshotCatalog::open(app_dir.join("snapshot-catalog"))
            .map_err(|e| StorageError::Message(e.to_string()))?;
        let evidence_dir = app_dir.join("evidence");
        std::fs::create_dir_all(&evidence_dir)?;
        Ok(Self {
            db_path,
            catalog,
            pool: RwLock::new(pool),
            evidence_dir,
            advisory,
        })
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    pub fn catalog(&self) -> &SnapshotCatalog {
        &self.catalog
    }

    pub async fn checkpoint_live(&self) -> Result<(), StorageError> {
        let pool = self.pool.read().await;
        checkpoint(&*pool).await
    }

    /// Wait for in-flight pool users, flush WAL, then close so restart does not leave SQLite open.
    pub async fn close_for_shutdown(&self) -> Result<(), StorageError> {
        let pool = self.pool.write().await;
        checkpoint(&*pool).await?;
        pool.close().await;
        Ok(())
    }

    async fn status_inner(&self) -> Result<HandoffStatusBody, StorageError> {
        let pool = self.pool.read().await;
        let local = local_head(&*pool).await?;
        let ack = store::review_ack(&*pool).await?;
        drop(pool);

        let published = self.catalog.published_head();
        let mut parents = self.catalog.parent_map();
        if let Some(head) = &local {
            parents
                .entry(head.snapshot_id)
                .or_insert(head.parent_snapshot_id);
        }
        let view = match &published {
            PublishedHead::Unavailable => PublishedView::Unavailable,
            PublishedHead::Invalid { .. } => PublishedView::Invalid,
            PublishedHead::None => PublishedView::Missing,
            PublishedHead::Some(head) => PublishedView::Head(head),
        };
        let decision = compare_heads(local.as_ref(), view, &parents);
        let published_id = match &published {
            PublishedHead::Some(head) => Some(head.snapshot_id),
            _ => None,
        };
        let review_ok = matches!(
            (ack, published_id),
            (Some(acked), Some(pub_id)) if acked == pub_id
        ) && decision == HandoffDecision::BlockUntilRestore;
        let writes = writes_allowed(decision, review_ok);
        let mut message = decision_message(decision).to_string();
        if looks_like_sync_folder(&self.db_path) {
            message.push_str(" Live SQLite path looks like a cloud-sync folder; move app data off OneDrive/iCloud/Dropbox.");
        }
        let published_head = match published {
            PublishedHead::Some(head) => Some(head),
            _ => None,
        };
        Ok(HandoffStatusBody {
            decision,
            writes_allowed: writes,
            message,
            local_head: local,
            published_head,
        })
    }

    async fn restore_inner(
        &self,
        snapshot_id: Uuid,
    ) -> Result<SnapshotIdentity, StorageError> {
        self.catalog
            .verify(snapshot_id)
            .map_err(|e| StorageError::Message(e.to_string()))?;
        let local_device = {
            let pool = self.pool.read().await;
            get_or_create_device(&*pool).await?
        };

        let mut pool = self.pool.write().await;
        checkpoint(&*pool).await?;
        pool.close().await;
        let mut restored = None;
        let mut last_err = String::new();
        for _ in 0..25 {
            match self.catalog.restore_to(snapshot_id, &self.db_path) {
                Ok(identity) => {
                    restored = Some(identity);
                    break;
                }
                Err(err) => {
                    last_err = err.to_string();
                    tokio::time::sleep(Duration::from_millis(40)).await;
                }
            }
        }
        let mut restored = restored.ok_or_else(|| StorageError::Message(last_err))?;
        restored.restore_test_status = "passed".to_string();
        let new_pool = store::connect(&self.db_path).await?;
        write_device(
            &new_pool,
            local_device.device_id,
            &local_device.device_name,
            restored.database_id,
        )
        .await?;
        set_local_head(&new_pool, &restored).await?;
        set_review_ack(&new_pool, None).await?;
        *pool = new_pool;
        Ok(restored)
    }
}

fn looks_like_sync_folder(path: &Path) -> bool {
    let s = path.to_string_lossy().to_lowercase();
    s.contains("onedrive") || s.contains("dropbox") || s.contains("icloud")
}

fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn map_err(err: StorageError) -> PlatformError {
    PlatformError::new("storage_error", err.to_string())
}

#[async_trait]
impl Platform for LocalPlatform {
    async fn config_get(&self) -> Result<DeviceConfig, PlatformError> {
        let pool = self.pool.read().await;
        get_or_create_device(&*pool).await.map_err(map_err)
    }

    async fn config_set(
        &self,
        device_name: Option<String>,
    ) -> Result<DeviceConfig, PlatformError> {
        let Some(name) = device_name.filter(|n| !n.trim().is_empty()) else {
            return self.config_get().await;
        };
        let pool = self.pool.read().await;
        set_device_name(&*pool, name.trim()).await.map_err(map_err)
    }

    async fn snapshot_head_get(&self) -> Result<Option<SnapshotIdentity>, PlatformError> {
        let pool = self.pool.read().await;
        local_head(&*pool).await.map_err(map_err)
    }

    async fn handoff_status_get(&self) -> Result<HandoffStatusBody, PlatformError> {
        self.status_inner().await.map_err(map_err)
    }

    async fn snapshot_create(&self) -> Result<SnapshotIdentity, PlatformError> {
        let pool = self.pool.read().await;
        let device = get_or_create_device(&*pool).await.map_err(map_err)?;
        let parent = local_head(&*pool).await.map_err(map_err)?;
        checkpoint(&*pool).await.map_err(map_err)?;
        drop(pool);

        let parent_id = parent.as_ref().map(|h| h.snapshot_id);
        let change_sequence = parent.as_ref().map(|h| h.change_sequence + 1).unwrap_or(1);
        let identity = SnapshotIdentity {
            database_id: device.database_id,
            snapshot_id: Uuid::new_v4(),
            parent_snapshot_id: parent_id,
            device_id: device.device_id,
            device_name: device.device_name,
            change_sequence,
            last_event_at: now_rfc3339(),
            schema_version: SCHEMA_VERSION.to_string(),
            calculation_version: CALCULATION_VERSION.to_string(),
            app_version: APP_VERSION.to_string(),
            database_hash: String::new(),
            evidence_manifest_hash: String::new(),
            validation_status: "valid".to_string(),
            restore_test_status: "not_run".to_string(),
        };
        let created = self
            .catalog
            .create_bundle(&self.db_path, identity)
            .map_err(|e| PlatformError::new("snapshot_create_failed", e.to_string()))?;
        let pool = self.pool.read().await;
        set_local_head(&*pool, &created).await.map_err(map_err)?;
        set_review_ack(&*pool, None).await.map_err(map_err)?;
        Ok(created)
    }

    async fn snapshot_restore(
        &self,
        snapshot_id: Option<Uuid>,
    ) -> Result<SnapshotIdentity, PlatformError> {
        let id = match snapshot_id {
            Some(id) => id,
            None => match self.catalog.published_head() {
                PublishedHead::Some(head) => head.snapshot_id,
                PublishedHead::Invalid { reason } => {
                    return Err(PlatformError::new("hash_mismatch", reason));
                }
                _ => {
                    return Err(PlatformError::new(
                        "missing_snapshot",
                        "no snapshot id and no published head",
                    ));
                }
            },
        };
        self.restore_inner(id)
            .await
            .map_err(map_err)
    }

    async fn handoff_resolve(
        &self,
        action: &str,
        snapshot_id: Option<Uuid>,
    ) -> Result<HandoffStatusBody, PlatformError> {
        match action {
            "restore" => {
                self.snapshot_restore(snapshot_id).await?;
            }
            "review" => {
                let status = self.status_inner().await.map_err(map_err)?;
                if status.decision != HandoffDecision::BlockUntilRestore {
                    return Err(PlatformError::new(
                        "review_not_applicable",
                        "explicit review applies only when published is newer",
                    ));
                }
                let published_id = status
                    .published_head
                    .as_ref()
                    .map(|h| h.snapshot_id)
                    .ok_or_else(|| {
                        PlatformError::new("missing_snapshot", "no published head to review")
                    })?;
                let pool = self.pool.read().await;
                set_review_ack(&*pool, Some(published_id))
                    .await
                    .map_err(map_err)?;
            }
            "select_branch" => {
                let id = snapshot_id.ok_or_else(|| {
                    PlatformError::new(
                        "missing_snapshot",
                        "select_branch requires snapshotId",
                    )
                })?;
                self.restore_inner(id).await.map_err(map_err)?;
            }
            _ => {
                return Err(PlatformError::new(
                    "unknown_action",
                    format!("unknown HandoffResolve action: {action}"),
                ));
            }
        }
        self.status_inner().await.map_err(map_err)
    }

    async fn writes_allowed(&self) -> Result<bool, PlatformError> {
        Ok(self.status_inner().await.map_err(map_err)?.writes_allowed)
    }

    fn app_data_dir(&self) -> PathBuf {
        self.db_path
            .parent()
            .unwrap_or(self.db_path.as_path())
            .to_path_buf()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use application_core::contracts::{
        CommandRequest, HandoffDecision, FINANCE_CLIENT_CONTRACT_VERSION,
    };
    use application_core::queries::execute_command_on;
    use snapshot_service::sha256_file;

    #[tokio::test]
    async fn close_for_shutdown_checkpoints_then_refuses_new_reads() {
        let dir = tempfile::tempdir().unwrap();
        let platform = LocalPlatform::open(dir.path().join("app-data"))
            .await
            .unwrap();
        platform
            .config_set(Some("shutdown-device".into()))
            .await
            .unwrap();
        platform.close_for_shutdown().await.unwrap();
        let err = platform.config_get().await.unwrap_err();
        assert!(
            !err.message.is_empty(),
            "closed pool must not serve a new read"
        );
    }

    #[tokio::test]
    async fn restore_round_trip_hashes_match() {
        let dir = tempfile::tempdir().unwrap();
        let platform = LocalPlatform::open(dir.path().join("app-data"))
            .await
            .unwrap();
        assert_eq!(
            store::journal_mode(&*platform.pool.read().await)
                .await
                .unwrap()
                .to_lowercase(),
            "wal"
        );
        let created = platform.snapshot_create().await.unwrap();
        assert_eq!(created.database_hash.len(), 64);
        platform
            .config_set(Some("mutated-device".into()))
            .await
            .unwrap();
        let after = platform.config_get().await.unwrap();
        assert_eq!(after.device_name, "mutated-device");

        let restored = platform
            .snapshot_restore(Some(created.snapshot_id))
            .await
            .unwrap();
        assert_eq!(restored.database_hash, created.database_hash);
        assert_eq!(restored.restore_test_status, "passed");
        let verified = platform.catalog.verify(created.snapshot_id).unwrap();
        assert_eq!(verified.database_hash, created.database_hash);
        let bundle_hash = sha256_file(&platform.catalog.database_path(created.snapshot_id)).unwrap();
        assert_eq!(bundle_hash, created.database_hash);
        let head = platform.snapshot_head_get().await.unwrap().unwrap();
        assert_eq!(head.snapshot_id, created.snapshot_id);
    }

    #[tokio::test]
    async fn published_newer_blocks_then_restore() {
        let dir = tempfile::tempdir().unwrap();
        let platform = LocalPlatform::open(dir.path().join("app-data"))
            .await
            .unwrap();
        let first = platform.snapshot_create().await.unwrap();
        platform
            .config_set(Some("other-computer-change".into()))
            .await
            .unwrap();
        platform.checkpoint_live().await.unwrap();

        let newer = SnapshotIdentity {
            database_id: first.database_id,
            snapshot_id: Uuid::from_u128(99),
            parent_snapshot_id: Some(first.snapshot_id),
            device_id: first.device_id,
            device_name: "other-computer".to_string(),
            change_sequence: first.change_sequence + 1,
            last_event_at: now_rfc3339(),
            schema_version: SCHEMA_VERSION.to_string(),
            calculation_version: CALCULATION_VERSION.to_string(),
            app_version: APP_VERSION.to_string(),
            database_hash: String::new(),
            evidence_manifest_hash: String::new(),
            validation_status: "valid".to_string(),
            restore_test_status: "not_run".to_string(),
        };
        platform
            .catalog
            .create_bundle(platform.db_path(), newer)
            .unwrap();

        let status = platform.handoff_status_get().await.unwrap();
        assert_eq!(status.decision, HandoffDecision::BlockUntilRestore);
        assert!(!status.writes_allowed);
        assert!(!platform.writes_allowed().await.unwrap());

        let blocked = execute_command_on(
            &platform,
            &platform,
            CommandRequest {
                contract_version: FINANCE_CLIENT_CONTRACT_VERSION.to_string(),
                command_name: "ConfigSet".to_string(),
                correlation_id: Uuid::nil(),
                body_json: Some("{\"deviceName\":\"blocked\"}".to_string()),
                expected_version: None,
            },
        )
        .await;
        assert_eq!(blocked.error_code.as_deref(), Some("writes_blocked"));

        let resolved = platform.handoff_resolve("restore", None).await.unwrap();
        assert_eq!(resolved.decision, HandoffDecision::OpenNormally);
        assert!(resolved.writes_allowed);
    }

    #[tokio::test]
    async fn import_cannot_post_without_approve() {
        use application_core::ports::canonical::Canonical;
        let dir = tempfile::tempdir().unwrap();
        let platform = LocalPlatform::open(dir.path().join("app-data"))
            .await
            .unwrap();
        platform
            .account_register("Taxable Brokerage".into(), "taxable".into())
            .await
            .unwrap();
        let staged = platform
            .import_stage(
                "src-1".into(),
                "x.txt".into(),
                b"row".to_vec(),
                vec![application_core::contracts::ImportCandidate {
                    account_name: "Taxable Brokerage".into(),
                    symbol: None,
                    activity_type: "deposit".into(),
                    amount_minor: Some(100),
                    scale: 2,
                    occurred_on: "2026-01-10".into(),
                    candidate_id: None,
                    validation: String::new(),
                    issue: String::new(),
                }],
                None,
            )
            .await
            .unwrap();
        let err = platform.import_post(staged.batch_id).await.unwrap_err();
        assert_eq!(err.code, "not_approved");
        platform.import_validate(staged.batch_id).await.unwrap();
        let err = platform.import_post(staged.batch_id).await.unwrap_err();
        assert_eq!(err.code, "not_approved");
        platform.import_approve(staged.batch_id).await.unwrap();
        let posted = platform.import_post(staged.batch_id).await.unwrap();
        assert_eq!(posted.status, "posted");
        let again = platform.import_post(staged.batch_id).await.unwrap();
        assert_eq!(again.batch_id, posted.batch_id);
        let counts = platform.reconcile_counts().await.unwrap();
        assert_eq!(counts.posted_activities, 1);
    }
}
