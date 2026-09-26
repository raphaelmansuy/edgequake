//! Task ↔ document KV sync helpers (SPEC-045 SRE-I01).
//!
//! Keeps document metadata terminal state aligned when tasks fail outside the
//! worker processor (e.g. periodic orphan heartbeat detection in `main.rs`).
//!
//! After KV upsert, also touches `public.documents.status` when a sidecar pool
//! is registered so list reads (column SSOT) cannot resurrect mid-pipeline
//! zombies after cancel/fail.

use std::sync::Arc;

use edgequake_storage::traits::KVStorage;
use edgequake_tasks::Task;
use serde_json::json;
use uuid::Uuid;

use crate::document_metadata::is_terminal_failure_status;

/// Extract document ID from task payload (PDF or text insert paths).
///
/// DRY: delegates to [`Task::document_id`] so API + tasks crate share one JSON
/// walk (issue #384 column populate uses the same extractor).
pub fn extract_document_id_from_task(task: &Task) -> Option<String> {
    task.document_id()
}

/// Resolve document id: task payload → `documents.track_id` / metadata track_id.
pub async fn resolve_document_id_for_task(
    kv: &dyn KVStorage,
    pool: crate::services::OptionalPgPool<'_>,
    task: &Task,
) -> Option<String> {
    if let Some(id) = extract_document_id_from_task(task) {
        return Some(id);
    }
    if let Some(id) = document_id_by_track_id(pool, &task.track_id).await {
        return Some(id);
    }
    // Last resort: workspace-scoped metadata scan for matching track_id / pdf_id.
    find_document_id_in_kv_by_correlation(kv, pool, task).await
}

#[cfg(feature = "postgres")]
async fn document_id_by_track_id(
    pool: crate::services::OptionalPgPool<'_>,
    track_id: &str,
) -> Option<String> {
    let pool = pool?;
    match sqlx::query_scalar::<_, String>(
        "SELECT id::text FROM public.documents \
         WHERE track_id = $1 OR metadata->>'track_id' = $1 \
         LIMIT 1",
    )
    .bind(track_id)
    .fetch_optional(pool)
    .await
    {
        Ok(row) => row,
        Err(e) => {
            tracing::debug!(
                track_id = %track_id,
                error = %e,
                "document_id_by_track_id lookup failed"
            );
            None
        }
    }
}

#[cfg(not(feature = "postgres"))]
async fn document_id_by_track_id(
    _pool: crate::services::OptionalPgPool<'_>,
    _track_id: &str,
) -> Option<String> {
    None
}

async fn find_document_id_in_kv_by_correlation(
    kv: &dyn KVStorage,
    pool: crate::services::OptionalPgPool<'_>,
    task: &Task,
) -> Option<String> {
    let pdf_id = task.pdf_id().map(|u| u.to_string());
    let entries =
        crate::services::document_metadata_scan::load_all_document_metadata_entries(kv, pool)
            .await
            .ok()?;
    for (_key, value) in entries {
        let Some(obj) = value.as_object() else {
            continue;
        };
        let track_match = obj
            .get("track_id")
            .and_then(|v| v.as_str())
            .is_some_and(|t| t == task.track_id);
        let pdf_match = pdf_id.as_ref().is_some_and(|p| {
            obj.get("pdf_id")
                .and_then(|v| v.as_str())
                .is_some_and(|v| v == p)
        });
        if !(track_match || pdf_match) {
            continue;
        }
        if let Some(id) = obj.get("id").and_then(|v| v.as_str()) {
            let id = id.trim();
            if !id.is_empty() {
                return Some(id.to_string());
            }
        }
    }
    None
}

/// Best-effort `public.documents.status` touch (list column SSOT).
pub async fn touch_relational_document_status_best_effort(
    document_id: &str,
    pool: crate::services::OptionalPgPool<'_>,
    status: &str,
) {
    touch_relational_document_track_status_best_effort(document_id, pool, None, status).await;
}

/// Best-effort sync of `public.documents.track_id` + `status` (dual-write with KV).
///
/// Pass `track_id = None` to clear a stale insert-/pdf- pointer during convert restart
/// before the new task id is known; pass `Some(id)` after enqueue.
pub async fn touch_relational_document_track_status_best_effort(
    document_id: &str,
    pool: crate::services::OptionalPgPool<'_>,
    track_id: Option<&str>,
    status: &str,
) {
    #[cfg(feature = "postgres")]
    {
        let Some(pool) = pool else {
            return;
        };
        let Ok(doc_uuid) = Uuid::parse_str(document_id) else {
            return;
        };
        // SPEC-129: project KV stages onto documents_valid_status before UPDATE.
        let pg_status = edgequake_storage::relational_documents_status_for_write(status);
        let result = if let Some(tid) = track_id {
            sqlx::query(
                "UPDATE public.documents \
                 SET track_id = $2, status = $3, updated_at = NOW() \
                 WHERE id = $1",
            )
            .bind(doc_uuid)
            .bind(tid)
            .bind(&pg_status)
            .execute(pool)
            .await
        } else {
            sqlx::query(
                "UPDATE public.documents \
                 SET track_id = NULL, status = $2, updated_at = NOW() \
                 WHERE id = $1",
            )
            .bind(doc_uuid)
            .bind(&pg_status)
            .execute(pool)
            .await
        };
        if let Err(e) = result {
            tracing::warn!(
                document_id = %document_id,
                status = %status,
                track_id = ?track_id,
                error = %e,
                "touch_relational_document_track_status_best_effort failed (non-fatal)"
            );
        }
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (document_id, pool, track_id, status);
    }
}

/// After task cancel: sync linked document KV to `cancelled` + failure_class (SPEC-057 P0).
///
/// No-op when the task has no document id, metadata is missing, or the doc is
/// already terminal-cancelled. Used by HTTP/WS/PDF/pipeline cancel paths.
pub async fn sync_doc_cancelled_for_task(
    kv: Arc<dyn KVStorage>,
    pool: crate::services::OptionalPgPool<'_>,
    task: &Task,
    message: &str,
) -> Result<bool, String> {
    let Some(document_id) = resolve_document_id_for_task(kv.as_ref(), pool, task).await else {
        return Ok(false);
    };
    sync_doc_cancelled_by_document_id(kv, pool, &document_id, message).await
}

/// Sync a document metadata row to cancelled by document id.
pub async fn sync_doc_cancelled_by_document_id(
    kv: Arc<dyn KVStorage>,
    pool: crate::services::OptionalPgPool<'_>,
    document_id: &str,
    message: &str,
) -> Result<bool, String> {
    // IMP-075-10: one RT staging+final (not resolve key then re-get).
    let Some((metadata_key, existing)) =
        crate::services::load_staging_first_metadata(kv.as_ref(), document_id).await?
    else {
        return Ok(false);
    };

    let Some(mut obj) = existing.as_object().cloned() else {
        return Ok(false);
    };

    let status = obj.get("status").and_then(|v| v.as_str()).unwrap_or("");
    if status.eq_ignore_ascii_case("cancelled") {
        // Still touch relational in case KV was cancelled but column lagged.
        touch_relational_document_status_best_effort(document_id, pool, "cancelled").await;
        return Ok(false);
    }

    crate::services::apply_doc_cancelled_fields(&mut obj, message);
    crate::services::upsert_metadata_kv_with_index(kv.as_ref(), &metadata_key, json!(obj))
        .await
        .map_err(|e| e.to_string())?;
    touch_relational_document_status_best_effort(document_id, pool, "cancelled").await;

    tracing::info!(
        document_id = %document_id,
        "Synced document metadata to cancelled after task cancel"
    );
    Ok(true)
}

/// Mark a mid-pipeline orphan document failed (no live Pending/Processing task).
pub async fn sync_doc_failed_no_active_task(
    kv: Arc<dyn KVStorage>,
    pool: crate::services::OptionalPgPool<'_>,
    document_id: &str,
    message: &str,
) -> Result<bool, String> {
    let Some((metadata_key, existing)) =
        crate::services::load_staging_first_metadata(kv.as_ref(), document_id).await?
    else {
        return Ok(false);
    };

    let Some(mut obj) = existing.as_object().cloned() else {
        return Ok(false);
    };

    let status = obj.get("status").and_then(|v| v.as_str()).unwrap_or("");
    if is_terminal_failure_status(status)
        || crate::document_metadata::is_terminal_success_status(status)
    {
        return Ok(false);
    }

    crate::services::apply_doc_failed_fields(&mut obj, message);
    crate::services::upsert_metadata_kv_with_index(kv.as_ref(), &metadata_key, json!(obj))
        .await
        .map_err(|e| e.to_string())?;
    touch_relational_document_status_best_effort(document_id, pool, "failed").await;

    tracing::warn!(
        document_id = %document_id,
        "Marked document failed — pipeline interrupted with no active task"
    );
    Ok(true)
}

/// True when document metadata `status` or `current_stage` is `projecting`.
pub fn metadata_is_projecting(metadata: &serde_json::Value) -> bool {
    ["status", "current_stage"].iter().any(|key| {
        metadata
            .get(key)
            .and_then(|v| v.as_str())
            .is_some_and(|s| s.eq_ignore_ascii_case("projecting"))
    })
}

/// True when mid-pipeline metadata already carries terminal-success evidence.
///
/// Force re-index / task loss can leave `processing`+`converting` while
/// `stage_message` / entity counts still reflect a finished ingest.
pub fn looks_like_completed_orphan(metadata: &serde_json::Value) -> bool {
    let Some(obj) = metadata.as_object() else {
        return false;
    };
    let status = obj.get("status").and_then(|v| v.as_str()).unwrap_or("");
    // SPEC-149: projecting waits for delivery apply — never promote via orphan heal.
    if metadata_is_projecting(metadata) {
        return false;
    }
    if crate::document_metadata::is_terminal_success_status(status)
        || is_terminal_failure_status(status)
    {
        return false;
    }
    if !crate::document_metadata::is_active_processing_status(status) {
        return false;
    }

    let (entity_count, chunk_count, _) = graph_counts_from_metadata(obj);
    let msg = obj
        .get("stage_message")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let completion_msg = crate::services::is_ingest_completion_stage_message(msg);
    let has_processed_at = obj
        .get("processed_at")
        .and_then(|v| v.as_str())
        .is_some_and(|s| !s.trim().is_empty());

    (completion_msg && (entity_count > 0 || chunk_count > 0))
        || (has_processed_at && entity_count > 0)
}

fn graph_counts_from_metadata(obj: &serde_json::Map<String, serde_json::Value>) -> (u64, u64, u64) {
    let entity_count = obj
        .get("entity_count")
        .or_else(|| obj.get("entities_count"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let chunk_count = obj.get("chunk_count").and_then(|v| v.as_u64()).unwrap_or(0);
    let relationship_count = obj
        .get("relationship_count")
        .or_else(|| obj.get("relationships_count"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    (entity_count, chunk_count, relationship_count)
}

/// Promote a completed-looking mid-pipeline orphan to terminal success.
///
/// Preserves existing completion `stage_message` / counts when present.
pub async fn sync_doc_completed_orphan(
    kv: Arc<dyn KVStorage>,
    pool: crate::services::OptionalPgPool<'_>,
    document_id: &str,
) -> Result<bool, String> {
    let Some((metadata_key, existing)) =
        crate::services::load_staging_first_metadata(kv.as_ref(), document_id).await?
    else {
        return Ok(false);
    };

    let Some(mut obj) = existing.as_object().cloned() else {
        return Ok(false);
    };

    if !looks_like_completed_orphan(&json!(obj)) {
        return Ok(false);
    }

    let (entity_count, chunk_count, relationship_count) = graph_counts_from_metadata(&obj);
    let existing_msg = obj
        .get("stage_message")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty() && crate::services::is_ingest_completion_stage_message(s))
        .map(str::to_string);
    let stage_message = existing_msg.unwrap_or_else(|| {
        crate::services::format_ingest_completion_stage_message(
            chunk_count,
            entity_count,
            relationship_count,
        )
    });

    obj.insert("status".to_string(), json!("completed"));
    obj.insert("current_stage".to_string(), json!("completed"));
    obj.insert("stage_message".to_string(), json!(stage_message));
    obj.insert("stage_progress".to_string(), json!(1.0));
    obj.remove("error_message");
    obj.remove("failure_class");
    obj.remove("failure_code");
    obj.remove("recommended_action");
    if obj
        .get("processed_at")
        .and_then(|v| v.as_str())
        .is_none_or(|s| s.trim().is_empty())
    {
        obj.insert(
            "processed_at".to_string(),
            json!(chrono::Utc::now().to_rfc3339()),
        );
    }
    obj.insert(
        "updated_at".to_string(),
        json!(chrono::Utc::now().to_rfc3339()),
    );

    crate::services::upsert_metadata_kv_with_index(kv.as_ref(), &metadata_key, json!(obj))
        .await
        .map_err(|e| e.to_string())?;
    touch_relational_document_status_best_effort(document_id, pool, "completed").await;

    tracing::info!(
        document_id = %document_id,
        entity_count,
        "Healed completed-looking orphan to completed (no active task)"
    );
    Ok(true)
}

/// Promote `projecting` → `completed` once SPEC-149 deliveries have applied.
///
/// WHY: Durable commit returns `awaiting_projection` and leaves the document on
/// `projecting`. Without this promote, the UI keeps the last Embedding snapshot
/// forever after deliveries are already applied.
pub async fn sync_doc_projecting_when_applied(
    kv: Arc<dyn KVStorage>,
    pool: crate::services::OptionalPgPool<'_>,
    document_id: &str,
) -> Result<bool, String> {
    let Some((metadata_key, existing)) =
        crate::services::load_staging_first_metadata(kv.as_ref(), document_id).await?
    else {
        return Ok(false);
    };

    if !metadata_is_projecting(&existing) {
        return Ok(false);
    }
    let Some(mut obj) = existing.as_object().cloned() else {
        return Ok(false);
    };

    #[cfg(feature = "postgres")]
    {
        let Some(pool) = pool else {
            return Ok(false);
        };
        let Ok(doc_uuid) = uuid::Uuid::parse_str(document_id) else {
            return Ok(false);
        };
        if !edgequake_storage::document_batch_deliveries_settled(pool, doc_uuid)
            .await
            .map_err(|e| e.to_string())?
        {
            return Ok(false);
        }
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = pool;
        return Ok(false);
    }

    let (entity_count, chunk_count, relationship_count) = graph_counts_from_metadata(&obj);
    let existing_msg = obj
        .get("stage_message")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let stage_message = existing_msg.unwrap_or_else(|| {
        crate::services::format_ingest_completion_stage_message(
            chunk_count,
            entity_count,
            relationship_count,
        )
    });

    obj.insert("status".to_string(), json!("completed"));
    obj.insert("current_stage".to_string(), json!("completed"));
    obj.insert("stage_message".to_string(), json!(stage_message));
    obj.insert("stage_progress".to_string(), json!(1.0));
    obj.remove("error_message");
    obj.remove("failure_class");
    obj.remove("failure_code");
    obj.remove("recommended_action");
    if obj
        .get("processed_at")
        .and_then(|v| v.as_str())
        .is_none_or(|s| s.trim().is_empty())
    {
        obj.insert(
            "processed_at".to_string(),
            json!(chrono::Utc::now().to_rfc3339()),
        );
    }
    obj.insert(
        "updated_at".to_string(),
        json!(chrono::Utc::now().to_rfc3339()),
    );

    crate::services::upsert_metadata_kv_with_index(kv.as_ref(), &metadata_key, json!(obj))
        .await
        .map_err(|e| e.to_string())?;
    touch_relational_document_status_best_effort(document_id, pool, "completed").await;

    // Clear any stale error left by COALESCE stats writes from prior attempts.
    #[cfg(feature = "postgres")]
    if let Some(pool) = pool {
        if let Ok(doc_uuid) = uuid::Uuid::parse_str(document_id) {
            let _ = sqlx::query(
                "UPDATE public.documents SET error_message = NULL, status = 'indexed', updated_at = NOW() \
                 WHERE id = $1",
            )
            .bind(doc_uuid)
            .execute(pool)
            .await;
        }
    }

    tracing::info!(
        document_id = %document_id,
        entity_count,
        "Promoted projecting → completed after projection deliveries applied"
    );
    Ok(true)
}

/// Mark document metadata `failed` when a task dies from heartbeat loss.
pub async fn sync_document_failed_on_orphan_heartbeat(
    kv: Arc<dyn KVStorage>,
    pool: crate::services::OptionalPgPool<'_>,
    task: &Task,
    error_msg: &str,
) -> Result<(), String> {
    let Some(document_id) = resolve_document_id_for_task(kv.as_ref(), pool, task).await else {
        return Ok(());
    };

    // IMP-075-10: one RT staging+final (not resolve key then re-get).
    let Some((metadata_key, existing)) =
        crate::services::load_staging_first_metadata(kv.as_ref(), &document_id).await?
    else {
        return Ok(());
    };

    let Some(mut obj) = existing.as_object().cloned() else {
        return Ok(());
    };

    let status = obj.get("status").and_then(|v| v.as_str()).unwrap_or("");
    if is_terminal_failure_status(status) {
        return Ok(());
    }

    let heartbeat_msg = format!("Task heartbeat lost — processing stopped. {error_msg}");
    crate::services::apply_doc_failed_fields(&mut obj, &heartbeat_msg);
    // ISSUE-304: structured Interrupted code for Reprocess routing (not message matching).
    if error_msg
        .to_ascii_lowercase()
        .contains("interrupted by server restart")
        || error_msg
            .to_ascii_lowercase()
            .contains("interrupted — use reprocess")
    {
        obj.insert(
            "failure_code".to_string(),
            json!(crate::services::FAILURE_CODE_SERVER_RESTART_INTERRUPTED),
        );
    }

    crate::services::upsert_metadata_kv_with_index(kv.as_ref(), &metadata_key, json!(obj))
        .await
        .map_err(|e| e.to_string())?;
    touch_relational_document_status_best_effort(&document_id, pool, "failed").await;

    tracing::warn!(
        task_id = %task.track_id,
        document_id = %document_id,
        failure_class = obj.get("failure_class").and_then(|v| v.as_str()).unwrap_or(""),
        "Periodic orphan check: synced document metadata to failed"
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_tasks::{Task, TaskType};

    #[test]
    fn spec045_extract_document_id_from_pdf_task() {
        let task = Task {
            track_id: "t1".to_string(),
            tenant_id: uuid::Uuid::new_v4(),
            workspace_id: uuid::Uuid::new_v4(),
            task_type: TaskType::PdfProcessing,
            status: edgequake_tasks::TaskStatus::Processing,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            started_at: None,
            completed_at: None,
            error_message: None,
            error: None,
            retry_count: 0,
            max_retries: 3,
            consecutive_timeout_failures: 0,
            circuit_breaker_tripped: false,
            task_data: serde_json::json!({ "existing_document_id": "doc-abc" }),
            metadata: None,
            progress: None,
            result: None,
            lease_owner: None,
            lease_token: None,
            lease_expires_at: None,
            fairness_hold_until: None,
        };
        assert_eq!(
            extract_document_id_from_task(&task).as_deref(),
            Some("doc-abc")
        );
    }

    #[test]
    fn spec045_extract_document_id_from_insert_metadata() {
        let task = Task::new(
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            TaskType::Insert,
            serde_json::json!({
                "metadata": { "document_id": "doc-xyz" }
            }),
        );
        assert_eq!(
            extract_document_id_from_task(&task).as_deref(),
            Some("doc-xyz")
        );
    }

    #[test]
    fn extract_document_id_from_task_metadata_field() {
        let mut task = Task::new(
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            TaskType::PdfProcessing,
            serde_json::json!({ "pdf_id": uuid::Uuid::new_v4().to_string() }),
        );
        task.metadata = Some(json!({ "document_id": "doc-from-meta" }));
        assert_eq!(
            extract_document_id_from_task(&task).as_deref(),
            Some("doc-from-meta")
        );
    }

    #[tokio::test]
    async fn sync_doc_cancelled_for_task_sets_failure_class() {
        use edgequake_storage::kv_keys;
        use edgequake_storage::MemoryKVStorage;

        let kv: Arc<dyn KVStorage> = Arc::new(MemoryKVStorage::new("cancel-sync-test"));
        let doc_id = "cancel-sync-doc";
        let meta_key = kv_keys::doc_metadata(doc_id);
        crate::services::upsert_metadata_kv_with_index(
            kv.as_ref(),
            &meta_key,
            json!({
                "id": doc_id,
                "status": "processing",
                "workspace_id": "ws-1",
            }),
        )
        .await
        .unwrap();

        let task = Task::new(
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            TaskType::Insert,
            json!({ "metadata": { "document_id": doc_id } }),
        );

        let updated =
            sync_doc_cancelled_for_task(Arc::clone(&kv), None, &task, "Task cancelled by user")
                .await
                .unwrap();
        assert!(updated);

        let stored = kv.get_by_id(&meta_key).await.unwrap().unwrap();
        assert_eq!(stored["status"], "cancelled");
        assert_eq!(stored["failure_class"], "cancelled");
        assert_eq!(stored["recommended_action"], "none");
    }

    #[tokio::test]
    async fn sync_doc_failed_no_active_task_marks_failed() {
        use edgequake_storage::kv_keys;
        use edgequake_storage::MemoryKVStorage;

        let kv: Arc<dyn KVStorage> = Arc::new(MemoryKVStorage::new("orphan-fail-test"));
        let doc_id = "orphan-fail-doc";
        let meta_key = kv_keys::doc_metadata(doc_id);
        crate::services::upsert_metadata_kv_with_index(
            kv.as_ref(),
            &meta_key,
            json!({
                "id": doc_id,
                "status": "converting",
                "workspace_id": "ws-1",
            }),
        )
        .await
        .unwrap();

        let updated = sync_doc_failed_no_active_task(
            Arc::clone(&kv),
            None,
            doc_id,
            "Pipeline interrupted — no active task",
        )
        .await
        .unwrap();
        assert!(updated);
        let stored = kv.get_by_id(&meta_key).await.unwrap().unwrap();
        assert_eq!(stored["status"], "failed");
    }

    #[test]
    fn looks_like_completed_orphan_detects_stale_completion_under_converting() {
        let meta = json!({
            "status": "processing",
            "current_stage": "converting",
            "stage_message": "Processed 22 chunks, extracted 658 entities and 381 relationships",
            "entity_count": 658,
            "chunk_count": 22,
            "processed_at": "2026-08-06T10:15:12Z",
        });
        assert!(looks_like_completed_orphan(&meta));
    }

    #[test]
    fn looks_like_completed_orphan_rejects_clean_converting() {
        let meta = json!({
            "status": "processing",
            "current_stage": "converting",
            "stage_message": "Converting PDF to Markdown (0/9 pages)",
            "entity_count": 0,
        });
        assert!(!looks_like_completed_orphan(&meta));
    }

    #[test]
    fn looks_like_completed_orphan_rejects_projecting() {
        let meta = json!({
            "status": "projecting",
            "current_stage": "projecting",
            "stage_message": "Processed 19 chunks, extracted 410 entities and 252 relationships",
            "entity_count": 410,
            "chunk_count": 19,
            "processed_at": "2026-09-22T10:15:12Z",
        });
        assert!(!looks_like_completed_orphan(&meta));
    }

    #[test]
    fn metadata_is_projecting_checks_status_or_stage_case_insensitively() {
        assert!(metadata_is_projecting(&json!({"status": "Projecting"})));
        assert!(metadata_is_projecting(
            &json!({"status": "processing", "current_stage": "projecting"})
        ));
        assert!(!metadata_is_projecting(
            &json!({"status": "completed", "current_stage": "completed"})
        ));
        assert!(!metadata_is_projecting(&json!({})));
        assert!(!metadata_is_projecting(&json!("projecting")));
    }

    #[tokio::test]
    async fn sync_doc_projecting_when_applied_promotes_without_pool() {
        use edgequake_storage::kv_keys;
        use edgequake_storage::MemoryKVStorage;

        // Without postgres pool the helper must no-op (cannot verify deliveries).
        let kv: Arc<dyn KVStorage> = Arc::new(MemoryKVStorage::new("projecting-promote-test"));
        let doc_id = "projecting-doc";
        let meta_key = kv_keys::doc_metadata(doc_id);
        crate::services::upsert_metadata_kv_with_index(
            kv.as_ref(),
            &meta_key,
            json!({
                "id": doc_id,
                "status": "projecting",
                "current_stage": "projecting",
                "stage_message": "Processed 19 chunks, extracted 410 entities and 252 relationships",
                "entity_count": 410,
                "chunk_count": 19,
                "relationship_count": 252,
                "error_message": "duplicate fact logical revision leftover",
            }),
        )
        .await
        .unwrap();

        let updated = sync_doc_projecting_when_applied(Arc::clone(&kv), None, doc_id)
            .await
            .unwrap();
        assert!(!updated);
        let stored = kv.get_by_id(&meta_key).await.unwrap().unwrap();
        assert_eq!(stored["status"], "projecting");
        assert_eq!(
            stored["error_message"],
            "duplicate fact logical revision leftover"
        );
    }

    #[tokio::test]
    async fn sync_doc_completed_orphan_promotes_zombie() {
        use edgequake_storage::kv_keys;
        use edgequake_storage::MemoryKVStorage;

        let kv: Arc<dyn KVStorage> = Arc::new(MemoryKVStorage::new("orphan-complete-test"));
        let doc_id = "orphan-complete-doc";
        let meta_key = kv_keys::doc_metadata(doc_id);
        crate::services::upsert_metadata_kv_with_index(
            kv.as_ref(),
            &meta_key,
            json!({
                "id": doc_id,
                "status": "processing",
                "current_stage": "converting",
                "stage_message": "Processed 22 chunks, extracted 658 entities and 381 relationships",
                "stage_progress": 0.0,
                "entity_count": 658,
                "chunk_count": 22,
                "relationship_count": 381,
                "processed_at": "2026-08-06T10:15:12Z",
                "workspace_id": "ws-1",
            }),
        )
        .await
        .unwrap();

        let updated = sync_doc_completed_orphan(Arc::clone(&kv), None, doc_id)
            .await
            .unwrap();
        assert!(updated);
        let stored = kv.get_by_id(&meta_key).await.unwrap().unwrap();
        assert_eq!(stored["status"], "completed");
        assert_eq!(stored["current_stage"], "completed");
        assert_eq!(stored["stage_progress"], 1.0);
        assert_eq!(stored["entity_count"], 658);
        assert!(stored["stage_message"]
            .as_str()
            .unwrap_or("")
            .contains("Processed 22 chunks"));
    }
}
