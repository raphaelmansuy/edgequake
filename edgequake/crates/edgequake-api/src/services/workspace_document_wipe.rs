//! Durable workspace wipe-all phase machine (issue #309 / SPEC-050).
//!
//! Invariants:
//! - Cancel all workspace **in-flight** ingestion first, then clear graph/vectors
//!   once, then purge docs. Finished attempts stay in `tasks` for audit
//!   (issue #386 / BR0903); `prune_terminal_tasks` is the retention GC.
//! - Never run N× `find_*_by_source_prefixes` (AGE LIKE SeqScans → timeout/OOM).
//! - Graph/vector clear failures are retryable task failures (fail-closed).
//! - HTTP 202 only admits; this worker owns terminal success/failure.

use edgequake_tasks::{
    Task, TaskStatus, WipeActivePolicy, WorkspaceWipePhase, WorkspaceWipeTaskData,
};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::middleware::{resolve_workspace_uuid, TenantContext};
use crate::services::document_metadata_scan::{
    load_scoped_document_metadata_entries, plan_workspace_document_kv_deletion,
};
use crate::services::document_task_cleanup::purge_workspace_tasks_except;
use crate::state::AppState;

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

/// Cancel in-flight workspace tasks except the wipe row itself.
///
/// Indexed / Failed / Cancelled rows are left in `tasks` (issue #386). Wipe
/// clears documents and graph data; it must not erase execution history while
/// the workspace still exists.
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
    let legacy_n = state
        .storage
        .vector_storage
        .clear_workspace(&workspace_uuid)
        .await
        .map_err(|e| {
            ApiError::Internal(format!(
                "workspace wipe vector clear failed (retryable): {e}"
            ))
        })?;

    // SPEC-091: under typed authority, also clear typed SSOT.
    // Legacy `clear_workspace` still DELETEs residual eq_*_vectors rows when the
    // relation exists (write-stop is upsert/CREATE only — SPEC-111 orphan fix).
    let mut typed_n = 0usize;
    #[cfg(feature = "postgres")]
    if edgequake_storage::legacy_vector_writes_stopped() {
        if let Some(ref pool) = state.pg_pool {
            let model = std::env::var("EDGEQUAKE_EMBEDDING_MODEL")
                .unwrap_or_else(|_| "text-embedding-3-small".to_string());
            let chunk_index = edgequake_storage::PgChunkEmbeddingIndex::new(pool.clone(), &model);
            let fleet = edgequake_storage::PgFleetEmbeddingIndex::new(pool.clone(), &model);
            let ws = edgequake_storage::traits::domain::WorkspaceId::new(workspace_uuid);
            use edgequake_storage::embedding_family::EmbeddingFamily;
            use edgequake_storage::traits::domain::{EmbeddingIndex, FleetEmbeddingIndex};
            typed_n += chunk_index.delete_for_workspace(ws).await.map_err(|e| {
                ApiError::Internal(format!("wipe typed chunk_embeddings failed: {e}"))
            })? as usize;
            for family in [
                EmbeddingFamily::Entity,
                EmbeddingFamily::Relationship,
                EmbeddingFamily::Report,
            ] {
                typed_n += fleet.delete_for_workspace(family, ws).await.map_err(|e| {
                    ApiError::Internal(format!(
                        "wipe typed fleet embeddings ({family:?}) failed: {e}"
                    ))
                })? as usize;
            }
        }
    }

    Ok(legacy_n.saturating_add(typed_n))
}

/// SPEC-091 RM1: set-based chunk delete for a workspace (O(1) SQL, not O(docs)).
#[cfg(feature = "postgres")]
async fn delete_chunks_for_workspace(
    pool: crate::services::OptionalPgPool<'_>,
    workspace_uuid: Uuid,
) -> ApiResult<u64> {
    let Some(pool) = pool else {
        return Ok(0);
    };
    let result = sqlx::query(
        r#"
        DELETE FROM public.chunks c
        USING public.documents d
        WHERE c.document_id = d.id
          AND (
            d.workspace_id = $1
            OR (d.workspace_id IS NULL AND d.metadata->>'workspace_id' = $2)
          )
        "#,
    )
    .bind(workspace_uuid)
    .bind(workspace_uuid.to_string())
    .execute(pool)
    .await
    .map_err(|e| ApiError::Internal(format!("wipe typed chunks failed: {e}")))?;
    Ok(result.rows_affected())
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
                // Typed set-delete + residual KV list-surface purge in ClearingRelational.
                data.cursor_metadata_key = None;
                data.phase = WorkspaceWipePhase::ClearingRelational;
                persist_wipe_checkpoint(state, task, &data).await?;
            }
            WorkspaceWipePhase::PurgingDocumentKv => {
                // Legacy phase: resume into ClearingRelational (set-based typed +
                // residual KV list-surface purge — SPEC-111 / #366).
                tracing::info!(
                    workspace_id = %workspace_uuid,
                    "Workspace wipe resuming PurgingDocumentKv → ClearingRelational"
                );
                data.cursor_metadata_key = None;
                data.phase = WorkspaceWipePhase::ClearingRelational;
                persist_wipe_checkpoint(state, task, &data).await?;
            }
            WorkspaceWipePhase::ClearingRelational => {
                #[cfg(feature = "postgres")]
                {
                    // RM-AC-05: O(families) set deletes — chunks cascade via FK from documents.
                    let chunks_deleted =
                        delete_chunks_for_workspace(state.optional_pg_pool(), workspace_uuid)
                            .await?;
                    data.total_chunks_deleted = data
                        .total_chunks_deleted
                        .saturating_add(chunks_deleted as usize);

                    let relational_deleted =
                        crate::document_read_model::delete_relational_documents_for_workspace(
                            state.optional_pg_pool(),
                            &tenant_ctx,
                        )
                        .await?;
                    data.deleted_count = data.deleted_count.max(relational_deleted as usize);
                    if relational_deleted > 0 || chunks_deleted > 0 {
                        tracing::info!(
                            workspace_id = %workspace_uuid,
                            relational_deleted,
                            chunks_deleted,
                            "Workspace wipe cleared typed document/chunk rows (set-based)"
                        );
                    }
                }

                // SPEC-111 / #366: after typed rows are gone, purge residual
                // dual-write KV list surfaces (metadata/content/wsdoc/chunks).
                // Planner intentionally suffix-scans when membership is empty.
                // Post-125 Absent KV relation → empty plan / no-op.
                let kv_plan = plan_workspace_document_kv_deletion(
                    state.storage.kv_storage.as_ref(),
                    state.optional_pg_pool(),
                    &data.workspace_id,
                )
                .await?;
                if !kv_plan.keys.is_empty() {
                    let n = kv_plan.keys.len();
                    state
                        .storage
                        .kv_storage
                        .delete(&kv_plan.keys)
                        .await
                        .map_err(|e| {
                            ApiError::Internal(format!(
                                "workspace wipe residual KV list-surface purge failed: {e}"
                            ))
                        })?;
                    tracing::info!(
                        workspace_id = %workspace_uuid,
                        kv_keys_deleted = n,
                        kv_documents = kv_plan.documents,
                        "Workspace wipe purged residual KV list surfaces"
                    );
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
    #[cfg(feature = "postgres")]
    if let Some(pool) = state.optional_pg_pool() {
        if let Some(ws) = tenant_ctx
            .workspace_id
            .as_ref()
            .and_then(|w| Uuid::parse_str(w).ok())
        {
            let n: i64 = sqlx::query_scalar(
                r#"
                SELECT COUNT(*) FROM public.documents
                WHERE workspace_id = $1
                   OR (workspace_id IS NULL AND metadata->>'workspace_id' = $2)
                "#,
            )
            .bind(ws)
            .bind(ws.to_string())
            .fetch_one(pool)
            .await
            .map_err(|e| ApiError::Internal(format!("count wipe documents: {e}")))?;
            return Ok(n.max(0) as usize);
        }
    }
    let scoped = load_scoped_document_metadata_entries(
        state.storage.kv_storage.as_ref(),
        state.optional_pg_pool(),
        tenant_ctx,
    )
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
    status.is_inflight()
}

#[cfg(test)]
mod tests {
    use super::*;

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
