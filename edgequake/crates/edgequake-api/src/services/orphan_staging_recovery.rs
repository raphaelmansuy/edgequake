//! SPEC-086 follow-up: recover orphaned staging admission shells.
//!
//! Text/MD admits write `staging:{doc}-metadata` with `current_stage=uploading`
//! until promote. Orphan document recovery intentionally skipped these keys
//! (mid-upload race). After list merge made staging visible, a lost task left
//! ActiveRuns stuck on "Document received, starting processing" across restarts.
//!
//! Rule (startup `min_age=None`, periodic with age):
//! - Leave staging alone when track_id still has Pending/Processing task.
//! - Otherwise mark staging failed with re-upload guidance (list-visible).

use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use edgequake_storage::traits::KVStorage;
use edgequake_tasks::{SharedTaskStorage, TaskStatus};
use serde_json::json;
use tracing::info;

use crate::services::FAILURE_CODE_SERVER_RESTART_INTERRUPTED;

const DOCUMENT_METADATA_SUFFIX: &str = "-metadata";

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct OrphanStagingRecoveryReport {
    /// Staging shells rewritten to failed (need re-upload).
    pub failed_count: u64,
    /// Staging left alone (live Pending/Processing task).
    pub skipped_live_task: u64,
    /// Staging skipped by age threshold (periodic path).
    pub skipped_young: u64,
}

/// Shared live-task gate for staging orphan recovery (DRY with task recovery).
/// Pending/Processing = still claimable or in-flight — not an orphan shell.
pub fn task_is_live(status: TaskStatus) -> bool {
    matches!(status, TaskStatus::Pending | TaskStatus::Processing)
}

fn is_live_task_status(status: TaskStatus) -> bool {
    task_is_live(status)
}

fn parse_rfc3339(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

/// Recover orphaned `staging:*-metadata` shells with no live task.
pub async fn recover_orphaned_staging_admissions(
    kv_storage: Arc<dyn KVStorage>,
    task_storage: SharedTaskStorage,
    min_age: Option<Duration>,
) -> Result<OrphanStagingRecoveryReport, String> {
    info!("Checking for orphaned staging admission shells…");
    let now = Utc::now();
    let mut report = OrphanStagingRecoveryReport::default();

    let staging_keys: Vec<String> = kv_storage
        .keys_with_prefix("staging:")
        .await
        .map_err(|e| format!("keys_with_prefix staging: {e}"))?
        .into_iter()
        .filter(|k| k.ends_with(DOCUMENT_METADATA_SUFFIX) && !k.contains(":hash:"))
        .collect();

    if staging_keys.is_empty() {
        return Ok(report);
    }

    let values = kv_storage
        .get_by_ids_ordered(&staging_keys)
        .await
        .map_err(|e| format!("get_by_ids_ordered staging: {e}"))?;

    let mut updates: Vec<(String, serde_json::Value)> = Vec::new();
    // (document_id, workspace_id, content_hash) — release after failed upsert.
    let mut release_reservations: Vec<(String, String, String)> = Vec::new();

    for (key, maybe_value) in staging_keys.into_iter().zip(values) {
        let Some(mut value) = maybe_value else {
            continue;
        };
        let Some(obj) = value.as_object_mut() else {
            continue;
        };

        // Already terminal — leave for UI / cleanup elsewhere.
        let status = obj
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_lowercase();
        if matches!(status.as_str(), "failed" | "cancelled" | "completed") {
            continue;
        }

        if let Some(age) = min_age {
            let ts = obj
                .get("updated_at")
                .or_else(|| obj.get("created_at"))
                .and_then(|v| v.as_str())
                .and_then(parse_rfc3339);
            if let Some(ts) = ts {
                if now.signed_duration_since(ts)
                    < chrono::Duration::from_std(age).unwrap_or(chrono::Duration::minutes(10))
                {
                    report.skipped_young += 1;
                    continue;
                }
            }
        }

        let track_id = obj
            .get("track_id")
            .or_else(|| obj.get("task_id"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let mut has_live = false;
        if !track_id.is_empty() {
            if let Ok(Some(task)) = task_storage.get_task(&track_id).await {
                if is_live_task_status(task.status) {
                    has_live = true;
                }
            }
        }

        if has_live {
            report.skipped_live_task += 1;
            continue;
        }

        let stage = obj
            .get("current_stage")
            .and_then(|v| v.as_str())
            .unwrap_or("uploading")
            .to_string();

        let content_hash = obj
            .get("content_hash")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let workspace_id = obj
            .get("workspace_id")
            .and_then(|v| v.as_str())
            .unwrap_or("default")
            .to_string();
        let doc_id = obj
            .get("id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| {
                key.strip_prefix("staging:")
                    .and_then(|rest| rest.strip_suffix(DOCUMENT_METADATA_SUFFIX))
                    .map(|s| s.to_string())
            });

        obj.insert("status".to_string(), json!("failed"));
        obj.insert("current_stage".to_string(), json!("failed"));
        obj.insert(
            "failure_code".to_string(),
            json!(FAILURE_CODE_SERVER_RESTART_INTERRUPTED),
        );
        obj.insert(
            "stage_message".to_string(),
            json!(format!(
                "Upload interrupted during '{stage}' (no live worker task). \
                 Please re-upload the document."
            )),
        );
        obj.insert(
            "error_message".to_string(),
            json!("Orphaned staging admission — please re-upload"),
        );
        obj.insert("updated_at".to_string(), json!(now.to_rfc3339()));
        // Keep admission_staging so merge still surfaces this shell as failed.
        obj.insert("admission_staging".to_string(), json!(true));

        updates.push((key, value));
        report.failed_count += 1;

        if let Some(id) = doc_id {
            if !content_hash.is_empty() {
                release_reservations.push((id, workspace_id, content_hash));
            }
        }
    }

    if !updates.is_empty() {
        kv_storage
            .upsert(&updates)
            .await
            .map_err(|e| format!("upsert orphan staging: {e}"))?;
        // Free hash after failed shell is durable so re-upload is not blocked.
        for (id, workspace_id, content_hash) in release_reservations {
            let _ = crate::services::release_staging_reservation(
                &kv_storage,
                &id,
                &workspace_id,
                &content_hash,
            )
            .await;
        }
        info!(
            failed = report.failed_count,
            skipped_live = report.skipped_live_task,
            skipped_young = report.skipped_young,
            "Orphan staging admission recovery complete"
        );
    }

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_storage::adapters::memory::MemoryKVStorage;
    use edgequake_storage::kv_keys;
    use edgequake_tasks::memory::MemoryTaskStorage;
    use edgequake_tasks::{Task, TaskStorage, TaskType};
    use serde_json::json;
    use std::sync::Arc;
    use uuid::Uuid;

    #[test]
    fn task_is_live_covers_pending_and_processing_only() {
        assert!(task_is_live(TaskStatus::Pending));
        assert!(task_is_live(TaskStatus::Processing));
        assert!(!task_is_live(TaskStatus::Failed));
        assert!(!task_is_live(TaskStatus::Cancelled));
        assert!(!task_is_live(TaskStatus::Indexed));
    }

    fn staging_meta(doc_id: &str, track_id: &str) -> serde_json::Value {
        json!({
            "id": doc_id,
            "title": "orphan.md",
            "file_name": "orphan.md",
            "status": "pending",
            "current_stage": "uploading",
            "stage_message": "Document received, starting processing",
            "track_id": track_id,
            "task_id": track_id,
            "admission_staging": true,
            "source_type": "markdown",
            "content_hash": "hash-orphan",
            "workspace_id": "default",
            "created_at": "2020-01-01T00:00:00Z",
            "updated_at": "2020-01-01T00:00:00Z",
        })
    }

    #[tokio::test]
    async fn fails_staging_when_no_task() {
        let kv: Arc<dyn KVStorage> = Arc::new(MemoryKVStorage::new("orphan-staging-none"));
        let tasks: SharedTaskStorage = Arc::new(MemoryTaskStorage::new());
        let doc_id = "doc-orphan-1";
        let track = "insert-orphan-1";
        let hash = "hash-orphan";
        kv.upsert(&[
            (
                kv_keys::staging_doc_metadata(doc_id),
                staging_meta(doc_id, track),
            ),
            (
                kv_keys::staging_workspace_hash("default", hash),
                json!(doc_id),
            ),
            (
                kv_keys::staging_doc_content(doc_id),
                json!({"content": "body"}),
            ),
        ])
        .await
        .unwrap();

        let report = recover_orphaned_staging_admissions(kv.clone(), tasks, None)
            .await
            .unwrap();
        assert_eq!(report.failed_count, 1);

        let meta = kv
            .get_by_id(&kv_keys::staging_doc_metadata(doc_id))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(meta["status"], "failed");
        assert_eq!(meta["current_stage"], "failed");
        assert!(meta["stage_message"]
            .as_str()
            .unwrap()
            .contains("re-upload"));
        // Hash freed so same-bytes re-upload is not duplicate_processing.
        assert!(kv
            .get_by_id(&kv_keys::staging_workspace_hash("default", hash))
            .await
            .unwrap()
            .is_none());
        assert!(kv
            .get_by_id(&kv_keys::staging_doc_content(doc_id))
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn keeps_staging_when_pending_task_alive() {
        let kv: Arc<dyn KVStorage> = Arc::new(MemoryKVStorage::new("orphan-staging-live"));
        let tasks = Arc::new(MemoryTaskStorage::new());
        let doc_id = "doc-orphan-2";
        let track = "insert-orphan-2";
        kv.upsert(&[(
            kv_keys::staging_doc_metadata(doc_id),
            staging_meta(doc_id, track),
        )])
        .await
        .unwrap();

        let mut task = Task::new(
            Uuid::nil(),
            Uuid::nil(),
            TaskType::Insert,
            json!({ "document_id": doc_id }),
        );
        task.track_id = track.to_string();
        task.status = TaskStatus::Pending;
        tasks.create_task(&task).await.unwrap();

        let report =
            recover_orphaned_staging_admissions(kv.clone(), tasks as SharedTaskStorage, None)
                .await
                .unwrap();
        assert_eq!(report.failed_count, 0);
        assert_eq!(report.skipped_live_task, 1);

        let meta = kv
            .get_by_id(&kv_keys::staging_doc_metadata(doc_id))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(meta["current_stage"], "uploading");
    }

    #[tokio::test]
    async fn periodic_age_skips_fresh_staging() {
        let kv: Arc<dyn KVStorage> = Arc::new(MemoryKVStorage::new("orphan-staging-young"));
        let tasks: SharedTaskStorage = Arc::new(MemoryTaskStorage::new());
        let doc_id = "doc-orphan-3";
        let track = "insert-orphan-3";
        let mut meta = staging_meta(doc_id, track);
        meta["updated_at"] = json!(Utc::now().to_rfc3339());
        meta["created_at"] = meta["updated_at"].clone();
        kv.upsert(&[(kv_keys::staging_doc_metadata(doc_id), meta)])
            .await
            .unwrap();

        let report =
            recover_orphaned_staging_admissions(kv, tasks, Some(Duration::from_secs(600)))
                .await
                .unwrap();
        assert_eq!(report.failed_count, 0);
        assert_eq!(report.skipped_young, 1);
    }
}
