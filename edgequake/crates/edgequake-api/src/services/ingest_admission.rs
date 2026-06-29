//! Ingestion admission — SSOT for document identity under tenant pressure (P-G14).
//!
//! ## First principles
//!
//! - **One logical document per PDF row**: `document_id` must be allocated once and
//!   survive worker retries, orphan recovery, and tenant-fairness requeues.
//! - **Side effects follow identity**: KV metadata is written only after the id is
//!   persisted on the task row.
//! - **Single-flight per pdf_id**: while a PdfProcessing task is pending/processing,
//!   do not enqueue another for the same pdf unless `restart_from_scratch`.

use std::sync::Arc;

use chrono::Utc;
use edgequake_storage::traits::KVStorage;
use edgequake_tasks::{PdfProcessingData, SharedTaskStorage, Task};
use tracing::{debug, info};
use uuid::Uuid;

use crate::middleware::TenantContext;
use crate::services::pdf_workspace_dedup::find_kv_document_id_for_pdf;
use crate::state::AppState;

/// Resolve the document id to use for a PDF ingest at **enqueue** time.
pub async fn resolve_pdf_ingest_document_id(
    state: &AppState,
    pdf_id: Uuid,
    explicit_document_id: Option<String>,
    tenant_ctx: &TenantContext,
) -> String {
    if let Some(id) = explicit_document_id {
        return id;
    }

    #[cfg(feature = "postgres")]
    if let Some(pdf_storage) = state.storage.pdf_storage.as_ref() {
        if let Ok(Some(pdf)) = pdf_storage.get_pdf(&pdf_id).await {
            if let Some(document_id) = pdf.document_id {
                return document_id.to_string();
            }
        }
    }

    let pdf_id_str = pdf_id.to_string();
    if let Some(doc_id) =
        find_kv_document_id_for_pdf(state.storage.kv_storage.as_ref(), &pdf_id_str, tenant_ctx)
            .await
    {
        return doc_id;
    }

    Uuid::new_v4().to_string()
}

/// Worker-time resolver: never mint a second id when one already exists.
pub async fn resolve_worker_pdf_document_id(
    kv_storage: &Arc<dyn KVStorage>,
    pdf_document_id: Option<Uuid>,
    pdf_id: Uuid,
    task: &mut Task,
    data: &PdfProcessingData,
    task_storage: Option<&SharedTaskStorage>,
    tenant_ctx: Option<&TenantContext>,
) -> Result<String, edgequake_tasks::TaskError> {
    if let Some(ref id) = data.existing_document_id {
        return Ok(id.clone());
    }

    if let Some(document_id) = pdf_document_id {
        let id = document_id.to_string();
        persist_pdf_task_document_id(task, &id, task_storage).await?;
        return Ok(id);
    }

    let pdf_id_str = pdf_id.to_string();
    if let Some(tenant_ctx) = tenant_ctx {
        if let Some(doc_id) =
            find_kv_document_id_for_pdf(kv_storage.as_ref(), &pdf_id_str, tenant_ctx).await
        {
            persist_pdf_task_document_id(task, &doc_id, task_storage).await?;
            return Ok(doc_id);
        }
    }

    let id = Uuid::new_v4().to_string();
    persist_pdf_task_document_id(task, &id, task_storage).await?;
    Ok(id)
}

/// Write `existing_document_id` onto the task row before any KV side effects.
pub async fn persist_pdf_task_document_id(
    task: &mut Task,
    document_id: &str,
    task_storage: Option<&SharedTaskStorage>,
) -> Result<(), edgequake_tasks::TaskError> {
    let already_set = task
        .task_data
        .get("existing_document_id")
        .and_then(|v| v.as_str())
        == Some(document_id);

    if !already_set {
        if let Ok(mut map) = serde_json::from_value::<serde_json::Map<String, serde_json::Value>>(
            task.task_data.clone(),
        ) {
            map.insert(
                "existing_document_id".to_string(),
                serde_json::json!(document_id),
            );
            task.task_data = serde_json::Value::Object(map);
        }
    }

    if let Some(storage) = task_storage {
        storage.update_task(task).await.map_err(|e| {
            edgequake_tasks::TaskError::Storage(format!(
                "Failed to persist document_id on task {}: {e}",
                task.track_id
            ))
        })?;
        debug!(
            track_id = %task.track_id,
            document_id = %document_id,
            "Persisted PDF ingest document_id on task row"
        );
    }

    Ok(())
}

/// Enqueue guard: skip duplicate PdfProcessing tasks unless a full restart was requested.
pub async fn admit_pdf_processing_enqueue(
    state: &AppState,
    pdf_id: Uuid,
    workspace_id: Uuid,
    restart_from_scratch: bool,
) -> Option<String> {
    if restart_from_scratch {
        return None;
    }

    if state
        .tasks
        .storage
        .find_active_pdf_processing_task(pdf_id, workspace_id)
        .await
        .ok()
        .flatten()
        .is_none()
    {
        state.tasks.pdf_admission.release(workspace_id, pdf_id);
    }

    if let Some(existing) = state.tasks.pdf_admission.get(workspace_id, pdf_id) {
        return Some(existing);
    }

    let active = state
        .tasks
        .storage
        .find_active_pdf_processing_task(pdf_id, workspace_id)
        .await
        .ok()
        .flatten()?;
    info!(
        pdf_id = %pdf_id,
        track_id = %active.track_id,
        "Single-flight: reusing in-flight PDF processing task"
    );
    Some(active.track_id)
}

/// Metadata written at PDF enqueue so queued documents appear in the list
/// before a worker slot opens (tenant-fairness / MAX_TASKS_PER_TENANT).
#[derive(Debug, Clone)]
pub struct QueuedPdfDocumentShell {
    pub pdf_id: Uuid,
    pub filename: String,
    pub tenant_id: Uuid,
    pub workspace_id: Uuid,
    pub track_id: String,
    pub file_size_bytes: i64,
    pub sha256_checksum: String,
    pub page_count: Option<i32>,
}

/// Create a visible "queued" document row at enqueue time (idempotent).
pub async fn provision_queued_pdf_document_shell(
    kv_storage: &Arc<dyn KVStorage>,
    document_id: &str,
    shell: &QueuedPdfDocumentShell,
) -> Result<(), edgequake_storage::error::StorageError> {
    let metadata_key = edgequake_storage::kv_keys::doc_metadata(document_id);
    if kv_storage.get_by_id(&metadata_key).await?.is_some() {
        return Ok(());
    }

    let metadata = serde_json::json!({
        "id": document_id,
        "title": shell.filename,
        "file_name": shell.filename,
        "source_type": "pdf",
        "document_type": "pdf",
        "status": "queued",
        "current_stage": "queued",
        "stage_message": "Waiting for a processing slot — ingestion continues automatically",
        "stage_progress": 0.0,
        "pdf_id": shell.pdf_id.to_string(),
        "file_size_bytes": shell.file_size_bytes,
        "sha256_checksum": shell.sha256_checksum,
        "page_count": shell.page_count,
        "tenant_id": shell.tenant_id.to_string(),
        "workspace_id": shell.workspace_id.to_string(),
        "track_id": shell.track_id,
        "created_at": Utc::now().to_rfc3339(),
        "updated_at": Utc::now().to_rfc3339(),
    });

    crate::services::upsert_metadata_kv_with_index(kv_storage.as_ref(), &metadata_key, metadata)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_tasks::memory::MemoryTaskStorage;
    use edgequake_tasks::{TaskStatus, TaskType};
    use std::sync::Arc;

    fn pdf_task(pdf_id: Uuid, workspace_id: Uuid, status: TaskStatus) -> Task {
        let mut task = Task::new(
            Uuid::new_v4(),
            workspace_id,
            TaskType::PdfProcessing,
            serde_json::json!({
                "pdf_id": pdf_id,
                "tenant_id": Uuid::new_v4(),
                "workspace_id": workspace_id,
                "enable_vision": true,
                "vision_provider": "mock",
            }),
        );
        task.status = status;
        task
    }

    #[tokio::test]
    async fn find_active_pdf_task_detects_pending_duplicate() {
        let storage: SharedTaskStorage = Arc::new(MemoryTaskStorage::new());
        let pdf_id = Uuid::new_v4();
        let workspace_id = Uuid::new_v4();
        let task = pdf_task(pdf_id, workspace_id, TaskStatus::Pending);
        storage.create_task(&task).await.unwrap();

        let found = storage
            .find_active_pdf_processing_task(pdf_id, workspace_id)
            .await
            .expect("lookup")
            .expect("should find task");
        assert_eq!(found.track_id, task.track_id);
    }

    #[tokio::test]
    async fn persist_pdf_task_document_id_updates_storage() {
        let storage: SharedTaskStorage = Arc::new(MemoryTaskStorage::new());
        let mut task = pdf_task(Uuid::new_v4(), Uuid::new_v4(), TaskStatus::Processing);
        storage.create_task(&task).await.unwrap();

        persist_pdf_task_document_id(&mut task, "doc-abc", Some(&storage))
            .await
            .unwrap();

        let loaded = storage.get_task(&task.track_id).await.unwrap().unwrap();
        assert_eq!(
            loaded
                .task_data
                .get("existing_document_id")
                .and_then(|v| v.as_str()),
            Some("doc-abc")
        );
    }
}
