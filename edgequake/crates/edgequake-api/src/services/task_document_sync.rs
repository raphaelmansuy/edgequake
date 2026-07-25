//! Task ↔ document KV sync helpers (SPEC-045 SRE-I01).
//!
//! Keeps document metadata terminal state aligned when tasks fail outside the
//! worker processor (e.g. periodic orphan heartbeat detection in `main.rs`).

use std::sync::Arc;

use edgequake_storage::traits::KVStorage;
use edgequake_tasks::Task;
use serde_json::json;

use crate::document_metadata::is_terminal_failure_status;

/// Extract document ID from task payload (PDF or text insert paths).
pub fn extract_document_id_from_task(task: &Task) -> Option<String> {
    task.task_data
        .get("existing_document_id")
        .and_then(|v| v.as_str())
        .or_else(|| task.task_data.get("document_id").and_then(|v| v.as_str()))
        .or_else(|| {
            task.task_data
                .get("metadata")
                .and_then(|m| m.get("document_id"))
                .and_then(|v| v.as_str())
        })
        .map(str::to_string)
}

/// After task cancel: sync linked document KV to `cancelled` + failure_class (SPEC-057 P0).
///
/// No-op when the task has no document id, metadata is missing, or the doc is
/// already terminal-cancelled. Used by HTTP/WS/PDF/pipeline cancel paths.
pub async fn sync_doc_cancelled_for_task(
    kv: Arc<dyn KVStorage>,
    task: &Task,
    message: &str,
) -> Result<bool, String> {
    let Some(document_id) = extract_document_id_from_task(task) else {
        return Ok(false);
    };
    sync_doc_cancelled_by_document_id(kv, &document_id, message).await
}

/// Sync a document metadata row to cancelled by document id.
pub async fn sync_doc_cancelled_by_document_id(
    kv: Arc<dyn KVStorage>,
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
        return Ok(false);
    }

    crate::services::apply_doc_cancelled_fields(&mut obj, message);
    crate::services::upsert_metadata_kv_with_index(kv.as_ref(), &metadata_key, json!(obj))
        .await
        .map_err(|e| e.to_string())?;

    tracing::info!(
        document_id = %document_id,
        "Synced document metadata to cancelled after task cancel"
    );
    Ok(true)
}

/// Mark document metadata `failed` when a task dies from heartbeat loss.
pub async fn sync_document_failed_on_orphan_heartbeat(
    kv: Arc<dyn KVStorage>,
    task: &Task,
    error_msg: &str,
) -> Result<(), String> {
    let Some(document_id) = extract_document_id_from_task(task) else {
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

    let failure = crate::services::classify_ingestion_failure(error_msg);
    edgequake_observability::record_ingestion_failure(
        failure.as_str(),
        &task.workspace_id.to_string(),
    );

    obj.insert("status".to_string(), json!("failed"));
    obj.insert("current_stage".to_string(), json!("failed"));
    obj.insert("error_message".to_string(), json!(error_msg));
    obj.insert("failure_class".to_string(), json!(failure.as_str()));
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
    obj.insert(
        "recommended_action".to_string(),
        json!(failure.recommended_action()),
    );
    obj.insert(
        "stage_message".to_string(),
        json!(format!(
            "Task heartbeat lost — processing stopped. {}",
            error_msg
        )),
    );
    obj.insert(
        "updated_at".to_string(),
        json!(chrono::Utc::now().to_rfc3339()),
    );

    crate::services::upsert_metadata_kv_with_index(kv.as_ref(), &metadata_key, json!(obj))
        .await
        .map_err(|e| e.to_string())?;

    tracing::warn!(
        task_id = %task.track_id,
        document_id = %document_id,
        failure_class = failure.as_str(),
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

        let updated = sync_doc_cancelled_for_task(Arc::clone(&kv), &task, "Task cancelled by user")
            .await
            .unwrap();
        assert!(updated);

        let stored = kv.get_by_id(&meta_key).await.unwrap().unwrap();
        assert_eq!(stored["status"], "cancelled");
        assert_eq!(stored["failure_class"], "cancelled");
        assert_eq!(stored["recommended_action"], "none");
    }
}
