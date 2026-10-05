//! Mobile read-head publish + Week Ahead confirm outbox.
//! Cloud folders (Drive / iCloud sync) transport files — never the live SQLite WAL.

use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::contracts::{
    MagiProjection, MobileHeadBody, MobileOutboxDrainBody, MobileOutboxItem,
    MobileOutboxPutBody, MobilePublishBody, WeekAheadBody,
};
use crate::ports::canonical::Canonical;
use crate::ports::platform::{Platform, PlatformError};
use crate::task::{task_list, STATUS_OPEN};
use crate::week_ahead::{week_ahead_confirm, week_ahead_get};

pub fn publish_dir(platform: &dyn Platform) -> PathBuf {
    if let Some(root) = std::env::var_os("FINOS_MOBILE_PUBLISH_DIR") {
        if !root.is_empty() {
            return PathBuf::from(root);
        }
    }
    platform.app_data_dir().join("mobile-publish")
}

fn head_path(dir: &Path) -> PathBuf {
    dir.join("head.json")
}

fn outbox_dir(dir: &Path) -> PathBuf {
    dir.join("outbox")
}

fn ensure_dirs(dir: &Path) -> Result<(), PlatformError> {
    fs::create_dir_all(dir).map_err(|e| {
        PlatformError::new(
            "mobile_publish_dir",
            format!("create {}: {e}", dir.display()),
        )
    })?;
    fs::create_dir_all(outbox_dir(dir)).map_err(|e| {
        PlatformError::new(
            "mobile_outbox_dir",
            format!("create {}: {e}", outbox_dir(dir).display()),
        )
    })?;
    Ok(())
}

fn magi_summary(magi: &MagiProjection) -> serde_json::Value {
    let overage = magi
        .base_forecast
        .amount_minor
        .saturating_sub(magi.applicable_threshold.amount_minor);
    json!({
        "applicableThresholdMinor": magi.applicable_threshold.amount_minor,
        "baseForecastMinor": magi.base_forecast.amount_minor,
        "actualIncludedYtdMinor": magi.actual_included_ytd.amount_minor,
        "overageMinor": overage.max(0),
        "decisionState": magi.decision_state,
        "scale": magi.applicable_threshold.scale,
    })
}

pub async fn mobile_publish(
    platform: &dyn Platform,
    canonical: &dyn Canonical,
    as_of: &str,
) -> Result<MobilePublishBody, PlatformError> {
    let as_of = &as_of[..as_of.len().min(10)];
    let dir = publish_dir(platform);
    ensure_dirs(&dir)?;

    let week_ahead = week_ahead_get(canonical, as_of).await?;
    let magi_json = match canonical.magi_projection_get().await {
        Ok(magi) => magi_summary(&magi).to_string(),
        Err(_) => json!({
            "applicableThresholdMinor": 0,
            "baseForecastMinor": 0,
            "actualIncludedYtdMinor": 0,
            "overageMinor": 0,
            "decisionState": "INDETERMINATE",
            "scale": 2,
        })
        .to_string(),
    };
    let tasks = task_list(canonical, None, Some(STATUS_OPEN.into())).await?;
    let published_at = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

    let head = MobileHeadBody {
        published_at: published_at.clone(),
        as_of: as_of.to_string(),
        publish_folder: dir.display().to_string(),
        week_ahead: week_ahead.clone(),
        open_tasks: tasks.items,
        magi_json,
        note: "Read-only mobile head. Live SQLite stays on the primary desktop. Point FINOS_MOBILE_PUBLISH_DIR at a cloud sync folder for transport.".into(),
    };

    let path = head_path(&dir);
    let pretty = serde_json::to_string_pretty(&head).map_err(|e| {
        PlatformError::new("mobile_head_serialize", format!("{e}"))
    })?;
    fs::write(&path, pretty).map_err(|e| {
        PlatformError::new(
            "mobile_head_write",
            format!("write {}: {e}", path.display()),
        )
    })?;

    Ok(MobilePublishBody {
        published_at,
        as_of: as_of.to_string(),
        folder: dir.display().to_string(),
        head_path: path.display().to_string(),
        open_task_count: head.open_tasks.len() as u64,
        week_ahead_row_count: week_ahead.rows.len() as u64,
        note: head.note,
    })
}

pub fn mobile_head_load(platform: &dyn Platform) -> Result<MobileHeadBody, PlatformError> {
    let path = head_path(&publish_dir(platform));
    let raw = fs::read_to_string(&path).map_err(|e| {
        PlatformError::new(
            "mobile_head_missing",
            format!("read {}: {e} — Force publish from desktop first", path.display()),
        )
    })?;
    serde_json::from_str(&raw).map_err(|e| {
        PlatformError::new("mobile_head_bad_json", format!("{e}"))
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OutboxFile {
    intent_id: String,
    kind: String,
    occurrence_id: String,
    as_of: String,
    created_at: String,
    note: String,
}

pub fn mobile_outbox_put(
    platform: &dyn Platform,
    occurrence_id: Uuid,
    as_of: &str,
    note: &str,
) -> Result<MobileOutboxPutBody, PlatformError> {
    let dir = publish_dir(platform);
    ensure_dirs(&dir)?;
    let intent_id = Uuid::new_v4();
    let created_at = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let file = OutboxFile {
        intent_id: intent_id.to_string(),
        kind: "week_ahead_confirm".into(),
        occurrence_id: occurrence_id.to_string(),
        as_of: as_of[..as_of.len().min(10)].to_string(),
        created_at: created_at.clone(),
        note: note.to_string(),
    };
    let path = outbox_dir(&dir).join(format!("{intent_id}.json"));
    let pretty = serde_json::to_string_pretty(&file).map_err(|e| {
        PlatformError::new("mobile_outbox_serialize", format!("{e}"))
    })?;
    fs::write(&path, pretty).map_err(|e| {
        PlatformError::new(
            "mobile_outbox_write",
            format!("write {}: {e}", path.display()),
        )
    })?;
    Ok(MobileOutboxPutBody {
        intent_id: intent_id.to_string(),
        path: path.display().to_string(),
        kind: file.kind,
        occurrence_id: occurrence_id.to_string(),
        created_at,
    })
}

pub async fn mobile_outbox_drain(
    platform: &dyn Platform,
    canonical: &dyn Canonical,
) -> Result<MobileOutboxDrainBody, PlatformError> {
    let dir = outbox_dir(&publish_dir(platform));
    if !dir.exists() {
        return Ok(MobileOutboxDrainBody {
            applied: Vec::new(),
            failed: Vec::new(),
            skipped: Vec::new(),
        });
    }
    let mut entries: Vec<_> = fs::read_dir(&dir)
        .map_err(|e| PlatformError::new("mobile_outbox_list", format!("{e}")))?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|x| x.to_str())
                .map(|x| x.eq_ignore_ascii_case("json"))
                .unwrap_or(false)
        })
        .collect();
    entries.sort_by_key(|e| e.file_name());

    let mut applied = Vec::new();
    let mut failed = Vec::new();
    let mut skipped = Vec::new();

    for entry in entries {
        let path = entry.path();
        let raw = match fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) => {
                failed.push(MobileOutboxItem {
                    intent_id: path.display().to_string(),
                    occurrence_id: String::new(),
                    status: format!("read_failed:{e}"),
                });
                continue;
            }
        };
        let file: OutboxFile = match serde_json::from_str(&raw) {
            Ok(f) => f,
            Err(e) => {
                failed.push(MobileOutboxItem {
                    intent_id: path.display().to_string(),
                    occurrence_id: String::new(),
                    status: format!("bad_json:{e}"),
                });
                continue;
            }
        };
        if file.kind != "week_ahead_confirm" {
            skipped.push(MobileOutboxItem {
                intent_id: file.intent_id.clone(),
                occurrence_id: file.occurrence_id.clone(),
                status: format!("unknown_kind:{}", file.kind),
            });
            continue;
        }
        let occ = match Uuid::parse_str(&file.occurrence_id) {
            Ok(id) => id,
            Err(_) => {
                failed.push(MobileOutboxItem {
                    intent_id: file.intent_id,
                    occurrence_id: file.occurrence_id,
                    status: "bad_occurrence_id".into(),
                });
                continue;
            }
        };
        match week_ahead_confirm(canonical, occ).await {
            Ok(_) => {
                let _ = fs::remove_file(&path);
                applied.push(MobileOutboxItem {
                    intent_id: file.intent_id,
                    occurrence_id: file.occurrence_id,
                    status: "applied".into(),
                });
            }
            Err(err) if err.code == "occurrence_confirmed" => {
                let _ = fs::remove_file(&path);
                skipped.push(MobileOutboxItem {
                    intent_id: file.intent_id,
                    occurrence_id: file.occurrence_id,
                    status: "already_confirmed".into(),
                });
            }
            Err(err) => {
                failed.push(MobileOutboxItem {
                    intent_id: file.intent_id,
                    occurrence_id: file.occurrence_id,
                    status: format!("{}:{}", err.code, err.message),
                });
            }
        }
    }

    Ok(MobileOutboxDrainBody {
        applied,
        failed,
        skipped,
    })
}
