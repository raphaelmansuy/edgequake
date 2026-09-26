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

use crate::error::ApiResult;
use crate::middleware::TenantContext;
use crate::services::document_quota::enforce_max_documents_admission;
use crate::services::pdf_workspace_dedup::find_kv_document_id_for_pdf;
use crate::state::AppState;

/// Minimal relational projection written at admission so PostgreSQL list/count
/// queries can see queued work before a worker starts.
#[derive(Debug, Clone)]
pub struct RelationalDocumentShell {
    pub title: String,
    pub tenant_id: Uuid,
    pub workspace_id: Uuid,
    pub track_id: String,
    pub source_type: String,
    pub file_size_bytes: i64,
    pub content_hash: Option<String>,
    pub content_type: Option<String>,
}

/// Idempotently expose admitted work on the production relational read path.
///
/// The heavy document body remains in staging KV; workers later update this row
/// through the existing `ensure_document_record` conflict path.
pub async fn provision_relational_document_shell(
    state: &AppState,
    document_id: &str,
    shell: &RelationalDocumentShell,
) -> ApiResult<()> {
    #[cfg(feature = "postgres")]
    if let Some(pool) = state.optional_pg_pool() {
        let document_id = Uuid::parse_str(document_id)
            .map_err(|e| crate::error::ApiError::Internal(format!("invalid document id: {e}")))?;
        let metadata = serde_json::json!({
            "source_type": shell.source_type,
            "current_stage": "queued",
            "stage_message": "Waiting for a processing slot",
            "admission_staging": true,
        });
        sqlx::query(
            r#"
            INSERT INTO documents (
                id, tenant_id, workspace_id, title, content, content_hash,
                metadata, file_size_bytes, content_type, status, track_id,
                created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, '', $5, $6, $7, $8, 'pending', $9, NOW(), NOW())
            ON CONFLICT (id) DO NOTHING
            "#,
        )
        .bind(document_id)
        .bind(shell.tenant_id)
        .bind(shell.workspace_id)
        .bind(&shell.title)
        .bind(&shell.content_hash)
        .bind(metadata)
        .bind(shell.file_size_bytes)
        .bind(&shell.content_type)
        .bind(&shell.track_id)
        .execute(pool)
        .await
        .map_err(|e| {
            crate::error::ApiError::Internal(format!(
                "failed to provision relational document shell: {e}"
            ))
        })?;
    }

    #[cfg(not(feature = "postgres"))]
    let _ = (state, document_id, shell);

    Ok(())
}

/// Allocate a document ID — uuidv7 on PG18 when capabilities allow (SPEC-042-E E-03).
#[cfg(feature = "postgres")]
pub async fn allocate_new_document_id(state: &AppState) -> String {
    allocate_document_id_from_pool(
        state.optional_pg_pool(),
        state.postgres_capabilities.as_ref(),
    )
    .await
}

#[cfg(not(feature = "postgres"))]
pub async fn allocate_new_document_id(_state: &AppState) -> String {
    Uuid::new_v4().to_string()
}

/// Allocate using optional pool + capabilities (handlers without full AppState).
#[cfg(feature = "postgres")]
pub async fn allocate_document_id_from_pool(
    pool: crate::services::OptionalPgPool<'_>,
    caps: Option<&edgequake_storage::adapters::postgres::PostgresCapabilities>,
) -> String {
    if let (Some(pool), Some(caps)) = (pool, caps) {
        return edgequake_storage::adapters::postgres::allocate_document_id(pool, caps).await;
    }
    Uuid::new_v4().to_string()
}

/// Resolve the document id to use for a PDF ingest at **enqueue** time.
///
/// SPEC-066: when minting a **new** document id, enforce `max_documents` fail-closed.
/// Re-ingest / existing pdf→doc mappings skip the quota check.
pub async fn resolve_pdf_ingest_document_id(
    state: &AppState,
    pdf_id: Uuid,
    explicit_document_id: Option<String>,
    tenant_ctx: &TenantContext,
) -> ApiResult<String> {
    if let Some(id) = explicit_document_id {
        return Ok(id);
    }

    #[cfg(feature = "postgres")]
    if let Some(pdf_storage) = state.storage.pdf_storage.as_ref() {
        if let Ok(Some(pdf)) = pdf_storage.get_pdf(&pdf_id).await {
            if let Some(document_id) = pdf.document_id {
                return Ok(document_id.to_string());
            }
        }
    }

    let pdf_id_str = pdf_id.to_string();
    if let Some(doc_id) = find_kv_document_id_for_pdf(
        state.storage.kv_storage.as_ref(),
        state.optional_pg_pool(),
        &pdf_id_str,
        tenant_ctx,
    )
    .await
    {
        return Ok(doc_id);
    }

    if let Some(ws) = tenant_ctx.workspace_id.as_deref() {
        enforce_max_documents_admission(state, ws).await?;
    }
    Ok(allocate_new_document_id(state).await)
}

/// Inputs for [`resolve_worker_pdf_document_id`] (keeps arity within clippy limits).
pub struct WorkerPdfDocumentIdRequest<'a> {
    pub kv_storage: &'a Arc<dyn KVStorage>,
    pub pdf_document_id: Option<Uuid>,
    pub pdf_id: Uuid,
    pub task: &'a mut Task,
    pub data: &'a PdfProcessingData,
    pub task_storage: Option<&'a SharedTaskStorage>,
    pub tenant_ctx: Option<&'a TenantContext>,
    /// SPEC-067: when minting a new id, enforce max_documents if present.
    pub workspace_service: Option<&'a dyn edgequake_core::WorkspaceService>,
    #[cfg(feature = "postgres")]
    pub pg_pool: Option<&'a sqlx::PgPool>,
    #[cfg(feature = "postgres")]
    pub postgres_capabilities:
        Option<&'a edgequake_storage::adapters::postgres::PostgresCapabilities>,
}

/// Worker-time resolver: never mint a second id when one already exists.
pub async fn resolve_worker_pdf_document_id(
    req: WorkerPdfDocumentIdRequest<'_>,
) -> Result<String, edgequake_tasks::TaskError> {
    if let Some(ref id) = req.data.existing_document_id {
        return Ok(id.clone());
    }

    if let Some(document_id) = req.pdf_document_id {
        let id = document_id.to_string();
        persist_pdf_task_document_id(req.task, &id, req.task_storage).await?;
        return Ok(id);
    }

    let pdf_id_str = req.pdf_id.to_string();
    if let Some(tenant_ctx) = req.tenant_ctx {
        if let Some(doc_id) = find_kv_document_id_for_pdf(
            req.kv_storage.as_ref(),
            {
                #[cfg(feature = "postgres")]
                {
                    req.pg_pool
                }
                #[cfg(not(feature = "postgres"))]
                {
                    crate::services::no_pg_pool()
                }
            },
            &pdf_id_str,
            tenant_ctx,
        )
        .await
        {
            persist_pdf_task_document_id(req.task, &doc_id, req.task_storage).await?;
            return Ok(doc_id);
        }
    }

    // SPEC-067: fail-closed quota when worker mints a brand-new document id.
    if let (Some(ws_svc), Some(tenant_ctx)) = (req.workspace_service, req.tenant_ctx) {
        if let Some(ws) = tenant_ctx.workspace_id.as_deref() {
            crate::services::document_quota::enforce_max_documents_admission_parts(
                ws_svc,
                req.kv_storage.as_ref(),
                ws,
            )
            .await
            .map_err(|e| edgequake_tasks::TaskError::Processing(format!("document quota: {e}")))?;
        }
    }

    #[cfg(feature = "postgres")]
    let id = allocate_document_id_from_pool(req.pg_pool, req.postgres_capabilities).await;
    #[cfg(not(feature = "postgres"))]
    let id = Uuid::new_v4().to_string();
    persist_pdf_task_document_id(req.task, &id, req.task_storage).await?;
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
        match storage.update_task(task).await {
            Ok(()) => {
                debug!(
                    track_id = %task.track_id,
                    document_id = %document_id,
                    "Persisted PDF ingest document_id on task row"
                );
            }
            Err(edgequake_tasks::TaskError::TaskNotFound(_)) => {
                // Clear All / delete purged the row after claim — unwind as cancel.
                return Err(edgequake_tasks::TaskError::Cancelled(format!(
                    "Task row removed during admission (lifecycle purge): {}",
                    task.track_id
                )));
            }
            Err(e) => {
                return Err(edgequake_tasks::TaskError::Storage(format!(
                    "Failed to persist document_id on task {}: {e}",
                    task.track_id
                )));
            }
        }
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

    #[tokio::test]
    async fn persist_pdf_task_document_id_cancelled_when_row_purged() {
        let storage: SharedTaskStorage = Arc::new(MemoryTaskStorage::new());
        let mut task = pdf_task(Uuid::new_v4(), Uuid::new_v4(), TaskStatus::Processing);
        storage.create_task(&task).await.unwrap();
        storage.delete_task(&task.track_id).await.unwrap();

        let err = persist_pdf_task_document_id(&mut task, "doc-gone", Some(&storage))
            .await
            .expect_err("must surface lifecycle purge as cancel");
        assert!(
            matches!(err, edgequake_tasks::TaskError::Cancelled(_)),
            "got {err:?}"
        );
    }
}
