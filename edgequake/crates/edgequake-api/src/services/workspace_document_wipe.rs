//! Durable workspace wipe-all phase machine (issue #309 / SPEC-050).
//!
//! Invariants:
//! - Cancel all workspace ingestion first, then clear graph/vectors once, then purge docs.
//! - Never run N× `find_*_by_source_prefixes` (AGE LIKE SeqScans → timeout/OOM).
//! - Graph/vector clear failures are retryable task failures (fail-closed).
//! - HTTP 202 only admits; this worker owns terminal success/failure.

use edgequake_tasks::{
    Task, TaskStatus, WipeActivePolicy, WorkspaceWipePhase, WorkspaceWipeTaskData,
};
use serde_json::Value;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::middleware::{resolve_workspace_uuid, TenantContext};
use crate::services::document_metadata_scan::{
    document_id_from_metadata_key, load_scoped_document_metadata_entries,
};
use crate::services::document_task_cleanup::purge_workspace_tasks_except;
use crate::state::AppState;

/// Bounded batch size for KV document purge (deterministic, not a wall-clock timeout).
const PURGE_BATCH_SIZE: usize = 50;

#[derive(Debug, Clone)]
struct WipeDoc {
    metadata_key: String,
    document_id: String,
    #[cfg_attr(not(feature = "postgres"), allow(dead_code))]
    pdf_id: Option<String>,
}

fn list_all_wipe_docs(scoped_entries: Vec<(String, Value)>) -> Vec<WipeDoc> {
    let mut docs = Vec::with_capacity(scoped_entries.len());
    for (metadata_key, metadata) in scoped_entries {
        let document_id = document_id_from_metadata_key(&metadata_key).unwrap_or_else(|| {
            metadata
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or(&metadata_key)
                .to_string()
        });
        let pdf_id = metadata
            .get("pdf_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        docs.push(WipeDoc {
            metadata_key,
            document_id,
            pdf_id,
        });
    }
    docs
}

async fn persist_wipe_checkpoint(
    state: &AppState,
    task: &mut Task,
    data: &WorkspaceWipeTaskData,
) -> ApiResult<()> {
    task.task_data = serde_json::to_value(data)
        .map_err(|e| ApiError::Internal(format!("serialize WorkspaceWipeTaskData: {e}")))?;
    task.updated_at = chrono::Utc::now();
    state
        .tasks
        .storage
        .update_task(task)
        .await
        .map_err(|e| ApiError::Internal(format!("persist wipe checkpoint: {e}")))?;
    Ok(())
}

async fn cancel_inflight_except_wipe(
    state: &AppState,
    workspace_uuid: Uuid,
    wipe_track_id: &str,
) -> ApiResult<usize> {
    let purged = purge_workspace_tasks_except(state, workspace_uuid, wipe_track_id).await;
    Ok(purged)
}

async fn clear_graph_fail_closed(
    state: &AppState,
    workspace_uuid: Uuid,
) -> ApiResult<(usize, usize)> {
    state
        .storage
        .graph_storage
        .clear_workspace(&workspace_uuid)
        .await
        .map_err(|e| {
            ApiError::Internal(format!(
                "workspace wipe graph clear failed (retryable): {e}"
            ))
        })
}

async fn clear_vectors_fail_closed(state: &AppState, workspace_uuid: Uuid) -> ApiResult<usize> {
    state
        .storage
        .vector_storage
        .clear_workspace(&workspace_uuid)
        .await
        .map_err(|e| {
            ApiError::Internal(format!(
                "workspace wipe vector clear failed (retryable): {e}"
            ))
        })
}

async fn purge_one_document_kv(
    state: &AppState,
    doc: &WipeDoc,
    tenant_ctx: &TenantContext,
) -> ApiResult<usize> {
    let chunk_prefix = format!("{}-chunk-", doc.document_id);
    let chunk_ids = state
        .storage
        .kv_storage
        .keys_with_prefix(&chunk_prefix)
        .await
        .unwrap_or_default();

    if !chunk_ids.is_empty() {
        state
            .storage
            .kv_storage
            .delete(&chunk_ids)
            .await
            .map_err(|e| ApiError::Internal(format!("wipe delete chunks: {e}")))?;
    }

    // SSOT list-surface purge: metadata, content, wsdoc, SQL, mm, pdf.
    // Workspace wipe also bulk-deletes relational at end; per-doc purge keeps
    // wsdoc/SQL consistent if the wipe is interrupted mid-batch.
    let workspace_id = tenant_ctx.workspace_id_or_default();
    crate::services::purge_document_list_surfaces(
        state,
        &doc.document_id,
        &workspace_id,
        tenant_ctx,
        crate::services::ListSurfacePurgeOpts {
            key_prefix: Some(&doc.document_id),
            content_hash: None,
            pdf_id: doc.pdf_id.as_deref(),
        },
    )
    .await
    .map_err(|e| {
        ApiError::Internal(format!(
            "workspace wipe list-surface purge failed (retryable) doc={}: {e}",
            doc.document_id
        ))
    })?;

    Ok(chunk_ids.len())
}

/// Execute (or resume) a durable workspace wipe from the task checkpoint.
pub async fn run_workspace_wipe_phases(
    state: &AppState,
    task: &mut Task,
    mut data: WorkspaceWipeTaskData,
) -> ApiResult<WorkspaceWipeTaskData> {
    let tenant_ctx = TenantContext {
        tenant_id: Some(data.tenant_id.clone()),
        workspace_id: Some(data.workspace_id.clone()),
        user_id: None,
    };
    let workspace_uuid = resolve_workspace_uuid(Some(&data.workspace_id)).ok_or_else(|| {
        ApiError::BadRequest(format!(
            "invalid workspace_id for wipe: {}",
            data.workspace_id
        ))
    })?;

    let _ = data.active_policy; // ForceCancelAll is the only policy today
    let wipe_track_id = data.wipe_track_id.clone();
    let planned_total = data.planned_delete_count.max(1);

    loop {
        match data.phase {
            WorkspaceWipePhase::Admitted | WorkspaceWipePhase::CancellingInflight => {
                data.phase = WorkspaceWipePhase::CancellingInflight;
                persist_wipe_checkpoint(state, task, &data).await?;

                state.tasks.progress_broadcaster.bulk_deletion_started(
                    planned_total,
                    Some(&wipe_track_id),
                    Some(&data.workspace_id),
                );

                let purged =
                    cancel_inflight_except_wipe(state, workspace_uuid, &wipe_track_id).await?;
                tracing::info!(
                    workspace_id = %workspace_uuid,
                    wipe_track_id = %wipe_track_id,
                    tasks_purged = purged,
                    "Workspace wipe cancelled inflight tasks"
                );
                data.phase = WorkspaceWipePhase::ClearingGraph;
                persist_wipe_checkpoint(state, task, &data).await?;
            }
            WorkspaceWipePhase::ClearingGraph => {
                let (nodes, edges) = clear_graph_fail_closed(state, workspace_uuid).await?;
                data.total_entities_removed = data.total_entities_removed.max(nodes);
                data.total_relationships_removed = data.total_relationships_removed.max(edges);
                tracing::info!(
                    workspace_id = %workspace_uuid,
                    nodes_cleared = nodes,
                    edges_cleared = edges,
                    "Workspace wipe cleared graph once"
                );
                data.phase = WorkspaceWipePhase::ClearingVectors;
                persist_wipe_checkpoint(state, task, &data).await?;
            }
            WorkspaceWipePhase::ClearingVectors => {
                let vectors = clear_vectors_fail_closed(state, workspace_uuid).await?;
                tracing::info!(
                    workspace_id = %workspace_uuid,
                    vectors_cleared = vectors,
                    "Workspace wipe cleared vectors once"
                );
                data.phase = WorkspaceWipePhase::PurgingDocumentKv;
                persist_wipe_checkpoint(state, task, &data).await?;
            }
            WorkspaceWipePhase::PurgingDocumentKv => {
                let scoped = load_scoped_document_metadata_entries(
                    state.storage.kv_storage.as_ref(),
                    &tenant_ctx,
                )
                .await?;
                let mut docs = list_all_wipe_docs(scoped);
                // Resume after cursor: skip keys <= cursor (lexicographic on metadata_key).
                if let Some(ref cursor) = data.cursor_metadata_key {
                    docs.retain(|d| d.metadata_key.as_str() > cursor.as_str());
                }
                docs.sort_by(|a, b| a.metadata_key.cmp(&b.metadata_key));

                if docs.is_empty() {
                    data.cursor_metadata_key = None;
                    data.phase = WorkspaceWipePhase::ClearingRelational;
                    persist_wipe_checkpoint(state, task, &data).await?;
                    continue;
                }

                let batch: Vec<_> = docs.into_iter().take(PURGE_BATCH_SIZE).collect();
                for doc in &batch {
                    let chunks = purge_one_document_kv(state, doc, &tenant_ctx).await?;
                    data.total_chunks_deleted += chunks;
                    data.deleted_count += 1;
                    if doc.pdf_id.is_some() {
                        data.total_pdfs_deleted += 1;
                    }
                    state
                        .tasks
                        .progress_broadcaster
                        .bulk_deletion_item_progress(
                            crate::handlers::websocket_types::BulkDeletionItemProgressArgs {
                                document_id: &doc.document_id,
                                completed: data.deleted_count,
                                total: planned_total.max(data.deleted_count),
                                entities_removed: data.total_entities_removed,
                                relationships_removed: data.total_relationships_removed,
                                wipe_track_id: Some(&wipe_track_id),
                                workspace_id: Some(&data.workspace_id),
                            },
                        );
                }
                data.cursor_metadata_key = batch.last().map(|d| d.metadata_key.clone());
                persist_wipe_checkpoint(state, task, &data).await?;
                // Stay in PurgingDocumentKv until a batch returns empty.
            }
            WorkspaceWipePhase::ClearingRelational => {
                #[cfg(feature = "postgres")]
                {
                    let relational_deleted =
                        crate::document_read_model::delete_relational_documents_for_workspace(
                            state.pg_pool.as_ref(),
                            &tenant_ctx,
                        )
                        .await?;
                    if relational_deleted > 0 {
                        tracing::info!(
                            workspace_id = %workspace_uuid,
                            relational_deleted,
                            "Workspace wipe cleared relational document rows"
                        );
                    }
                }
                data.phase = WorkspaceWipePhase::Completed;
                persist_wipe_checkpoint(state, task, &data).await?;
            }
            WorkspaceWipePhase::Completed => {
                state.tasks.progress_broadcaster.bulk_deletion_completed(
                    data.deleted_count,
                    data.skipped_document_ids.len(),
                    data.total_entities_removed,
                    data.total_relationships_removed,
                    Some(&wipe_track_id),
                    Some(&data.workspace_id),
                );
                state.tasks.wipe_admission.release(workspace_uuid);
                return Ok(data);
            }
        }
    }
}

/// Count documents that will be wiped (admit-time planned count).
pub async fn count_planned_wipe_documents(
    state: &AppState,
    tenant_ctx: &TenantContext,
) -> ApiResult<usize> {
    let scoped =
        load_scoped_document_metadata_entries(state.storage.kv_storage.as_ref(), tenant_ctx)
            .await?;
    Ok(scoped.len())
}

/// Admit helper: build task data for a new wipe (ForceCancelAll — never skips active docs).
pub fn new_wipe_task_data(
    tenant_id: String,
    workspace_id: String,
    wipe_track_id: String,
    planned_delete_count: usize,
) -> WorkspaceWipeTaskData {
    WorkspaceWipeTaskData {
        tenant_id,
        workspace_id,
        wipe_track_id,
        phase: WorkspaceWipePhase::Admitted,
        deleted_count: 0,
        skipped_document_ids: Vec::new(),
        cursor_metadata_key: None,
        active_policy: WipeActivePolicy::ForceCancelAll,
        total_chunks_deleted: 0,
        total_entities_removed: 0,
        total_relationships_removed: 0,
        total_pdfs_deleted: 0,
        planned_delete_count,
    }
}

/// Broadcast terminal failure for a permanently failed wipe task.
pub fn broadcast_wipe_failed(state: &AppState, data: &WorkspaceWipeTaskData, error_message: &str) {
    if let Some(ws) = resolve_workspace_uuid(Some(&data.workspace_id)) {
        state.tasks.wipe_admission.release(ws);
    }
    state.tasks.progress_broadcaster.bulk_deletion_failed(
        &data.wipe_track_id,
        Some(&data.workspace_id),
        error_message,
        data.deleted_count,
    );
}

/// Mark wipe task status helper for tests / recovery.
pub fn wipe_is_active(status: TaskStatus) -> bool {
    matches!(status, TaskStatus::Pending | TaskStatus::Processing)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn list_all_includes_active_processing() {
        let entries = vec![
            (
                "doc-1-metadata".to_string(),
                json!({"id": "doc-1", "status": "processing"}),
            ),
            (
                "doc-2-metadata".to_string(),
                json!({"id": "doc-2", "status": "completed"}),
            ),
        ];
        let docs = list_all_wipe_docs(entries);
        assert_eq!(docs.len(), 2);
    }

    #[test]
    fn new_wipe_starts_admitted() {
        let data = new_wipe_task_data("default".into(), "ws".into(), "wipe-1".into(), 10);
        assert_eq!(data.phase, WorkspaceWipePhase::Admitted);
        assert_eq!(data.planned_delete_count, 10);
        assert!(matches!(
            data.active_policy,
            WipeActivePolicy::ForceCancelAll
        ));
    }
}
