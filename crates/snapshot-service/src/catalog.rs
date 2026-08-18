//! Local snapshot catalog: immutable bundles, published pointer, hash verify, restore copy.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use application_core::contracts::SnapshotIdentity;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::hash::sha256_file;

const PUBLISHED_FILE: &str = "published.json";
const SNAPSHOTS_DIR: &str = "snapshots";
const MANIFEST_FILE: &str = "manifest.json";
const DATABASE_FILE: &str = "database.sqlite";
const EVIDENCE_FILE: &str = "evidence-manifest.json";
const EMPTY_EVIDENCE: &str = "{\"files\":[]}\n";

#[derive(Debug, Error)]
pub enum CatalogError {
    #[error("snapshot catalog unavailable: {0}")]
    Unavailable(String),
    #[error("snapshot invalid: {0}")]
    Invalid(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PublishedPointer {
    snapshot_id: Uuid,
}

#[derive(Debug, Clone)]
pub enum PublishedHead {
    Unavailable,
    Invalid { reason: String },
    None,
    Some(SnapshotIdentity),
}

#[derive(Debug, Clone)]
pub struct SnapshotCatalog {
    root: PathBuf,
}

impl SnapshotCatalog {
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, CatalogError> {
        let root = root.into();
        fs::create_dir_all(root.join(SNAPSHOTS_DIR))?;
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn snapshot_dir(&self, snapshot_id: Uuid) -> PathBuf {
        self.root.join(SNAPSHOTS_DIR).join(snapshot_id.to_string())
    }

    pub fn database_path(&self, snapshot_id: Uuid) -> PathBuf {
        self.snapshot_dir(snapshot_id).join(DATABASE_FILE)
    }

    /// Copy live DB + evidence placeholder, hash, write manifest, set published pointer.
    pub fn create_bundle(
        &self,
        live_db: &Path,
        mut identity: SnapshotIdentity,
    ) -> Result<SnapshotIdentity, CatalogError> {
        if !live_db.exists() {
            return Err(CatalogError::Invalid(format!(
                "live database missing: {}",
                live_db.display()
            )));
        }
        let dir = self.snapshot_dir(identity.snapshot_id);
        fs::create_dir_all(&dir)?;
        let dest_db = dir.join(DATABASE_FILE);
        fs::copy(live_db, &dest_db)?;
        let evidence_path = dir.join(EVIDENCE_FILE);
        fs::write(&evidence_path, EMPTY_EVIDENCE)?;

        identity.database_hash = sha256_file(&dest_db)?;
        identity.evidence_manifest_hash = sha256_file(&evidence_path)?;
        identity.validation_status = "valid".to_string();
        if identity.restore_test_status.is_empty() {
            identity.restore_test_status = "not_run".to_string();
        }

        let manifest_path = dir.join(MANIFEST_FILE);
        fs::write(&manifest_path, serde_json::to_vec_pretty(&identity)?)?;
        self.set_published(identity.snapshot_id)?;
        Ok(identity)
    }

    pub fn set_published(&self, snapshot_id: Uuid) -> Result<(), CatalogError> {
        let pointer = PublishedPointer { snapshot_id };
        fs::write(
            self.root.join(PUBLISHED_FILE),
            serde_json::to_vec_pretty(&pointer)?,
        )?;
        Ok(())
    }

    pub fn clear_published(&self) -> Result<(), CatalogError> {
        let path = self.root.join(PUBLISHED_FILE);
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    pub fn load_manifest(&self, snapshot_id: Uuid) -> Result<SnapshotIdentity, CatalogError> {
        let path = self.snapshot_dir(snapshot_id).join(MANIFEST_FILE);
        if !path.exists() {
            return Err(CatalogError::Invalid(format!(
                "manifest missing for {snapshot_id}"
            )));
        }
        let raw = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&raw)?)
    }

    /// Verify database and evidence hashes against the stored manifest.
    pub fn verify(&self, snapshot_id: Uuid) -> Result<SnapshotIdentity, CatalogError> {
        let manifest = self.load_manifest(snapshot_id)?;
        let db_path = self.database_path(snapshot_id);
        let evidence_path = self.snapshot_dir(snapshot_id).join(EVIDENCE_FILE);
        if !db_path.exists() {
            return Err(CatalogError::Invalid("bundle database missing".into()));
        }
        let db_hash = sha256_file(&db_path)?;
        if db_hash != manifest.database_hash {
            return Err(CatalogError::Invalid(
                "database hash mismatch".into(),
            ));
        }
        if evidence_path.exists() {
            let ev_hash = sha256_file(&evidence_path)?;
            if ev_hash != manifest.evidence_manifest_hash {
                return Err(CatalogError::Invalid(
                    "evidence manifest hash mismatch".into(),
                ));
            }
        }
        if manifest.validation_status != "valid" {
            return Err(CatalogError::Invalid(format!(
                "validation_status is {}",
                manifest.validation_status
            )));
        }
        Ok(manifest)
    }

    pub fn restore_to(&self, snapshot_id: Uuid, dest_db: &Path) -> Result<SnapshotIdentity, CatalogError> {
        let manifest = self.verify(snapshot_id)?;
        if let Some(parent) = dest_db.parent() {
            fs::create_dir_all(parent)?;
        }
        remove_sqlite_sidecars(dest_db);
        fs::copy(self.database_path(snapshot_id), dest_db)?;
        remove_sqlite_sidecars(dest_db);
        Ok(manifest)
    }

    pub fn published_head(&self) -> PublishedHead {
        let path = self.root.join(PUBLISHED_FILE);
        if !self.root.is_dir() {
            return PublishedHead::Unavailable;
        }
        if !path.exists() {
            return PublishedHead::None;
        }
        match fs::read_to_string(&path) {
            Err(err) => PublishedHead::Invalid {
                reason: err.to_string(),
            },
            Ok(raw) => match serde_json::from_str::<PublishedPointer>(&raw) {
                Err(err) => PublishedHead::Invalid {
                    reason: err.to_string(),
                },
                Ok(pointer) => match self.verify(pointer.snapshot_id) {
                    Ok(manifest) => PublishedHead::Some(manifest),
                    Err(err) => PublishedHead::Invalid {
                        reason: err.to_string(),
                    },
                },
            },
        }
    }

    pub fn parent_map(&self) -> HashMap<Uuid, Option<Uuid>> {
        let mut map = HashMap::new();
        let snapshots = self.root.join(SNAPSHOTS_DIR);
        let Ok(entries) = fs::read_dir(snapshots) else {
            return map;
        };
        for entry in entries.flatten() {
            let manifest_path = entry.path().join(MANIFEST_FILE);
            if let Ok(raw) = fs::read_to_string(manifest_path) {
                if let Ok(manifest) = serde_json::from_str::<SnapshotIdentity>(&raw) {
                    map.insert(manifest.snapshot_id, manifest.parent_snapshot_id);
                }
            }
        }
        map
    }
}

fn remove_sqlite_sidecars(db_path: &Path) {
    let wal = sidecar(db_path, "-wal");
    let shm = sidecar(db_path, "-shm");
    let _ = fs::remove_file(wal);
    let _ = fs::remove_file(shm);
}

fn sidecar(db_path: &Path, suffix: &str) -> PathBuf {
    let mut name = db_path.as_os_str().to_os_string();
    name.push(suffix);
    PathBuf::from(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use application_core::contracts::{APP_VERSION, CALCULATION_VERSION, SCHEMA_VERSION};

    fn sample_identity(id: Uuid, parent: Option<Uuid>) -> SnapshotIdentity {
        SnapshotIdentity {
            database_id: Uuid::from_u128(9),
            snapshot_id: id,
            parent_snapshot_id: parent,
            device_id: Uuid::from_u128(8),
            device_name: "win-test".to_string(),
            change_sequence: 1,
            last_event_at: "2026-08-18T00:00:00Z".to_string(),
            schema_version: SCHEMA_VERSION.to_string(),
            calculation_version: CALCULATION_VERSION.to_string(),
            app_version: APP_VERSION.to_string(),
            database_hash: String::new(),
            evidence_manifest_hash: String::new(),
            validation_status: "valid".to_string(),
            restore_test_status: "not_run".to_string(),
        }
    }

    #[test]
    fn create_hash_restore_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let live = dir.path().join("live.sqlite");
        fs::write(&live, b"sqlite-bytes-v1").unwrap();
        let catalog = SnapshotCatalog::open(dir.path().join("catalog")).unwrap();
        let id = Uuid::from_u128(1);
        let created = catalog
            .create_bundle(&live, sample_identity(id, None))
            .unwrap();
        assert_eq!(created.database_hash.len(), 64);
        assert_eq!(created.validation_status, "valid");

        fs::write(&live, b"mutated-after-snapshot").unwrap();
        let restored_path = dir.path().join("restored.sqlite");
        let restored = catalog.restore_to(id, &restored_path).unwrap();
        assert_eq!(restored.database_hash, created.database_hash);
        assert_eq!(fs::read(&restored_path).unwrap(), b"sqlite-bytes-v1");
        assert_eq!(sha256_file(&restored_path).unwrap(), created.database_hash);
    }

    #[test]
    fn tampered_bundle_fails_verify() {
        let dir = tempfile::tempdir().unwrap();
        let live = dir.path().join("live.sqlite");
        fs::write(&live, b"original").unwrap();
        let catalog = SnapshotCatalog::open(dir.path().join("catalog")).unwrap();
        let id = Uuid::from_u128(2);
        catalog
            .create_bundle(&live, sample_identity(id, None))
            .unwrap();
        fs::write(catalog.database_path(id), b"tampered").unwrap();
        let err = catalog.verify(id).unwrap_err();
        assert!(matches!(err, CatalogError::Invalid(_)));
        match catalog.published_head() {
            PublishedHead::Invalid { .. } => {}
            other => panic!("expected invalid published head, got {other:?}"),
        }
    }
}
