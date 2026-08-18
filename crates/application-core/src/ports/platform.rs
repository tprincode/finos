//! Platform ports for identity, snapshots and handoff (no SQLite, no Tauri).

use async_trait::async_trait;
use uuid::Uuid;

use crate::contracts::{DeviceConfig, HandoffStatusBody, SnapshotIdentity};

#[derive(Debug, Clone)]
pub struct PlatformError {
    pub code: String,
    pub message: String,
}

impl PlatformError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

/// Local platform operations used by FinanceClient dispatch.
#[async_trait]
pub trait Platform: Send + Sync {
    async fn config_get(&self) -> Result<DeviceConfig, PlatformError>;
    async fn config_set(
        &self,
        device_name: Option<String>,
    ) -> Result<DeviceConfig, PlatformError>;
    async fn snapshot_head_get(&self) -> Result<Option<SnapshotIdentity>, PlatformError>;
    async fn handoff_status_get(&self) -> Result<HandoffStatusBody, PlatformError>;
    async fn snapshot_create(&self) -> Result<SnapshotIdentity, PlatformError>;
    async fn snapshot_restore(
        &self,
        snapshot_id: Option<Uuid>,
    ) -> Result<SnapshotIdentity, PlatformError>;
    async fn handoff_resolve(
        &self,
        action: &str,
        snapshot_id: Option<Uuid>,
    ) -> Result<HandoffStatusBody, PlatformError>;
    async fn writes_allowed(&self) -> Result<bool, PlatformError>;
}
