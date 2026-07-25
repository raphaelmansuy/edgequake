//! Startup orphaned-task recovery (SPEC-057 P1 / SPEC-054).
//!
//! Product rule:
//! - Pending survives boot (claimable) — never auto-Failed
//! - Processing → Failed (Interrupted — use Reprocess) or Pending when `auto_resume`
//! - Cancelled / Indexed / Failed are not rewritten here
//! - Multi-replica: leave Processing with a still-valid lease

use std::sync::Arc;

use chrono::Utc;
use edgequake_storage::traits::KVStorage;
use edgequake_tasks::{Pagination, SharedTaskStorage, TaskFilter, TaskStatus};
use tracing::{debug, info, warn};

use crate::document_metadata::is_terminal_success_status;
use crate::services::extract_document_id_from_task;
use crate::services::sync_document_failed_on_orphan_heartbeat;
use edgequake_storage::kv_keys;

/// Counters from a boot orphan recovery pass.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct OrphanTaskRecoveryReport {
    pub recovered_count: u64,
    pub failed_for_manual_count: u64,
    pub completed_count: u64,
    pub skipped_live_lease: u64,
    /// SPEC-059: document IDs to retract when task failed (manual resume path).
    pub retract_document_ids: Vec<String>,
}

fn humanize_minutes(minutes: i64) -> String {
    if minutes < 1 {
        "<1 minute".to_string()
    } else if minutes == 1 {
        "1 minute".to_string()
    } else {
        format!("{minutes} minutes")
    }
}

/// Recover Processing tasks left from a previous process.
///
/// `multi_replica`: when true, Processing rows with a valid lease are left alone.
/// When false (Local single-process), all Processing rows are treated as stale.
pub async fn recover_orphaned_tasks(
    task_storage: SharedTaskStorage,
    kv_storage: Arc<dyn KVStorage>,
    auto_resume: bool,
    multi_replica: bool,
) -> Result<OrphanTaskRecoveryReport, String> {
    info!(
        auto_resume,
        multi_replica, "Checking for orphaned tasks from previous backend session..."
    );

    let now = Utc::now();
    let mut report = OrphanTaskRecoveryReport::default();

    let filter = TaskFilter {
        status: Some(TaskStatus::Processing),
        ..Default::default()
    };
    let mut page = 1u32;
    let page_size = 500u32;

    loop {
        let pagination = Pagination {
            page,
            page_size,
            ..Default::default()
        };

        let task_list = task_storage
            .list_tasks(filter.clone(), pagination)
            .await
            .map_err(|e| format!("list_tasks failed: {e}"))?;
        let batch_len = task_list.tasks.len();

        // IMP-075-05: batch staging+final metadata for the page (O(1) RT), not
        // 2×N resolve_document_metadata_key + get_by_id per task.
        let mut page_meta_keys: Vec<String> = Vec::with_capacity(batch_len * 2);
        let mut doc_ids_for_tasks: Vec<Option<String>> = Vec::with_capacity(batch_len);
        for task in &task_list.tasks {
            let doc_id = extract_document_id_from_task(task);
            if let Some(ref id) = doc_id {
                page_meta_keys.push(kv_keys::staging_doc_metadata(id));
                page_meta_keys.push(kv_keys::doc_metadata(id));
            }
            doc_ids_for_tasks.push(doc_id);
        }
        let page_meta_vals = if page_meta_keys.is_empty() {
            Vec::new()
        } else {
            kv_storage
                .get_by_ids_ordered(&page_meta_keys)
                .await
                .unwrap_or_default()
        };
        // doc_id → staging-first metadata value (if any)
        let mut meta_by_doc: std::collections::HashMap<String, serde_json::Value> =
            std::collections::HashMap::with_capacity(batch_len);
        let mut key_idx = 0usize;
        for doc_id in doc_ids_for_tasks.iter().flatten() {
            let staging = page_meta_vals.get(key_idx).and_then(|v| v.clone());
            let final_m = page_meta_vals.get(key_idx + 1).and_then(|v| v.clone());
            key_idx += 2;
            if let Some(m) = staging.or(final_m) {
                meta_by_doc.insert(doc_id.clone(), m);
            }
        }

        for mut task in task_list.tasks {
            let age = now.signed_duration_since(task.updated_at);
            let age_mins = age.num_minutes();

            if let Some(doc_id) = extract_document_id_from_task(&task) {
                if let Some(meta) = meta_by_doc.get(&doc_id) {
                    let doc_status = meta.get("status").and_then(|v| v.as_str()).unwrap_or("");
                    if is_terminal_success_status(doc_status) {
                        task.status = TaskStatus::Indexed;
                        task.clear_lease();
                        task.error_message = Some(format!(
                            "Auto-closed after restart: document already terminal ({doc_status}); \
                             task was still Processing (age {age_mins} minutes)."
                        ));
                        task.updated_at = now;
                        match task_storage.update_task(&task).await {
                            Ok(_) => {
                                info!(
                                    "Closed orphaned task against completed doc: {} → indexed",
                                    task.track_id
                                );
                                report.completed_count += 1;
                            }
                            Err(e) => {
                                warn!("Failed to close orphaned task {}: {}", task.track_id, e);
                            }
                        }
                        continue;
                    }
                }
            }

            if multi_replica && !task.lease_is_expired(now) {
                report.skipped_live_lease += 1;
                debug!(
                    task_id = %task.track_id,
                    lease_owner = ?task.lease_owner,
                    "Leaving Processing task with valid lease (multi-replica)"
                );
                continue;
            }

            if auto_resume {
                task.status = TaskStatus::Pending;
                task.clear_lease();
                task.started_at = None;
                task.error_message = Some(format!(
                    "Auto-recovered after backend restart (was processing for {age_mins} minutes). \
                     Will resume from checkpoint if available."
                ));
                task.updated_at = now;

                match task_storage.update_task(&task).await {
                    Ok(_) => {
                        info!(
                            "Recovered orphaned task: {} (age: {})",
                            task.track_id,
                            humanize_minutes(age_mins)
                        );
                        report.recovered_count += 1;
                    }
                    Err(e) => {
                        warn!("Failed to recover orphaned task {}: {}", task.track_id, e);
                    }
                }
            } else {
                let error_msg = format!(
                    "Interrupted by server restart (was Processing for {age_mins} minutes). \
                     Interrupted — use Reprocess to resume from checkpoint if available."
                );
                task.status = TaskStatus::Failed;
                task.clear_lease();
                task.error_message = Some(error_msg.clone());
                task.updated_at = now;

                match task_storage.update_task(&task).await {
                    Ok(_) => {
                        if let Err(e) = sync_document_failed_on_orphan_heartbeat(
                            Arc::clone(&kv_storage),
                            &task,
                            &error_msg,
                        )
                        .await
                        {
                            warn!(
                                task_id = %task.track_id,
                                error = %e,
                                "Failed to sync document metadata after startup orphan fail"
                            );
                        }
                        if let Some(doc_id) = extract_document_id_from_task(&task) {
                            report.retract_document_ids.push(doc_id);
                        }
                        info!(
                            "Orphaned task marked failed for manual resume: {} (age: {})",
                            task.track_id,
                            humanize_minutes(age_mins)
                        );
                        report.failed_for_manual_count += 1;
                    }
                    Err(e) => {
                        warn!("Failed to mark orphaned task {}: {}", task.track_id, e);
                    }
                }
            }
        }

        if batch_len < page_size as usize {
            break;
        }
        page += 1;
    }

    if report.recovered_count > 0
        || report.failed_for_manual_count > 0
        || report.completed_count > 0
        || report.skipped_live_lease > 0
    {
        info!(
            "Orphaned task recovery complete: {} auto-requeued, {} failed for manual resume, {} closed (doc already terminal), {} left with live lease; Pending left claimable",
            report.recovered_count,
            report.failed_for_manual_count,
            report.completed_count,
            report.skipped_live_lease
        );
    } else {
        info!("No orphaned Processing tasks found — Pending left claimable (SPEC-057 P1)");
    }

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_storage::MemoryKVStorage;
    use edgequake_tasks::memory::MemoryTaskStorage;
    use edgequake_tasks::{Task, TaskType};
    use uuid::Uuid;

    fn storage() -> SharedTaskStorage {
        Arc::new(MemoryTaskStorage::new())
    }

    fn kv() -> Arc<dyn KVStorage> {
        Arc::new(MemoryKVStorage::new("orphan-recovery-test"))
    }

    #[tokio::test]
    async fn pending_survives_boot_when_auto_resume_off() {
        let task_storage = storage();
        let pending = Task::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            TaskType::Insert,
            serde_json::json!({ "metadata": { "document_id": "p1" } }),
        );
        let pending_id = pending.track_id.clone();
        task_storage.create_task(&pending).await.unwrap();

        let mut processing = Task::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            TaskType::Insert,
            serde_json::json!({ "metadata": { "document_id": "p2" } }),
        );
        processing.mark_processing();
        let processing_id = processing.track_id.clone();
        task_storage.create_task(&processing).await.unwrap();

        let mut cancelled = Task::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            TaskType::Insert,
            serde_json::json!({}),
        );
        cancelled.mark_cancelled();
        let cancelled_id = cancelled.track_id.clone();
        task_storage.create_task(&cancelled).await.unwrap();

        let report = recover_orphaned_tasks(Arc::clone(&task_storage), kv(), false, false)
            .await
            .unwrap();
        assert_eq!(report.failed_for_manual_count, 1);
        assert_eq!(report.recovered_count, 0);

        assert_eq!(
            task_storage
                .get_task(&pending_id)
                .await
                .unwrap()
                .unwrap()
                .status,
            TaskStatus::Pending
        );
        assert_eq!(
            task_storage
                .get_task(&processing_id)
                .await
                .unwrap()
                .unwrap()
                .status,
            TaskStatus::Failed
        );
        let failed = task_storage
            .get_task(&processing_id)
            .await
            .unwrap()
            .unwrap();
        assert!(
            failed
                .error_message
                .as_deref()
                .unwrap_or("")
                .contains("Interrupted"),
            "expected Interrupted message"
        );
        assert_eq!(
            task_storage
                .get_task(&cancelled_id)
                .await
                .unwrap()
                .unwrap()
                .status,
            TaskStatus::Cancelled
        );
    }

    #[tokio::test]
    async fn auto_resume_reclaims_processing_to_pending() {
        let task_storage = storage();
        let mut processing = Task::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            TaskType::Insert,
            serde_json::json!({}),
        );
        processing.mark_processing();
        let id = processing.track_id.clone();
        task_storage.create_task(&processing).await.unwrap();

        let report = recover_orphaned_tasks(Arc::clone(&task_storage), kv(), true, false)
            .await
            .unwrap();
        assert_eq!(report.recovered_count, 1);
        assert_eq!(
            task_storage.get_task(&id).await.unwrap().unwrap().status,
            TaskStatus::Pending
        );
    }

    #[tokio::test]
    async fn multi_replica_skips_valid_lease() {
        let task_storage = storage();
        let mut processing = Task::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            TaskType::Insert,
            serde_json::json!({}),
        );
        processing.mark_processing();
        processing.lease_owner = Some("live-worker".into());
        processing.lease_token = Some(Uuid::new_v4());
        processing.lease_expires_at = Some(Utc::now() + chrono::Duration::seconds(120));
        let id = processing.track_id.clone();
        task_storage.create_task(&processing).await.unwrap();

        let report = recover_orphaned_tasks(Arc::clone(&task_storage), kv(), false, true)
            .await
            .unwrap();
        assert_eq!(report.skipped_live_lease, 1);
        assert_eq!(report.failed_for_manual_count, 0);
        assert_eq!(
            task_storage.get_task(&id).await.unwrap().unwrap().status,
            TaskStatus::Processing
        );
    }
}
