//! Duplicate content hash resolution and safe re-ingestion deletion (SPEC-024).

use crate::error::ApiError;
use crate::middleware::TenantContext;
use crate::services::{
    cleanup_document_graph_data, get_workspace_vector_storage_strict,
    recycle_orphan_workspace_hash, workspace_has_visible_document_for_hash, CleanupStats,
};
use crate::state::AppState;

/// Outcome of workspace-scoped duplicate hash lookup before a new ingest enqueue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DuplicateReingestAction {
    NoDuplicate,
    ClearedForReingestion { old_document_id: String },
    StillProcessing { existing_document_id: String },
}

/// Resolve workspace duplicate content hash (ingestion uniformity SSOT).
pub async fn resolve_workspace_duplicate_for_reingestion(
    state: &AppState,
    tenant_ctx: &TenantContext,
    hash_key: &str,
    workspace_id: &str,
) -> Result<DuplicateReingestAction, ApiError> {
    let Some(existing_doc_id) = state.storage.kv_storage.get_by_id(hash_key).await? else {
        return Ok(DuplicateReingestAction::NoDuplicate);
    };
    let Some(doc_id_str) = existing_doc_id.as_str() else {
        return Ok(DuplicateReingestAction::NoDuplicate);
    };

    if !workspace_has_visible_document_for_hash(state, doc_id_str, tenant_ctx).await? {
        recycle_orphan_workspace_hash(state, hash_key, workspace_id, doc_id_str).await?;
        return Ok(DuplicateReingestAction::NoDuplicate);
    }

    match delete_document_for_reingestion(doc_id_str, state, workspace_id).await {
        Ok(true) => Ok(DuplicateReingestAction::ClearedForReingestion {
            old_document_id: doc_id_str.to_string(),
        }),
        Ok(false) => Ok(DuplicateReingestAction::StillProcessing {
            existing_document_id: doc_id_str.to_string(),
        }),
        Err(e) => {
            // SPEC-086 ops: fail closed — never allocate a second admit while the
            // prior document may still be visible (duplicate completed rows).
            tracing::warn!(
                old_doc_id = %doc_id_str,
                workspace_id = %workspace_id,
                error = %e,
                "Failed to delete old document data — blocking re-ingestion"
            );
            Ok(DuplicateReingestAction::StillProcessing {
                existing_document_id: doc_id_str.to_string(),
            })
        }
    }
}

/// Delete all document data for re-ingestion when status allows atomic transition.
pub async fn delete_document_for_reingestion(
    document_id: &str,
    state: &AppState,
    workspace_id: &str,
) -> Result<bool, ApiError> {
    let allowed_from_statuses = [
        "failed",
        "completed",
        "indexed", // C-23: modern terminal status (status_updates completed→indexed)
        "partial_failure",
        "processed",
        "cancelled",
    ];
    // First Principles: never transition from lifecycle-exclusive states.
    // (deleting / delete_failed are owned by the deletion state machine.)
    let metadata_key =
        crate::services::document_metadata_scan::metadata_key_for_document(document_id);
    if let Ok(Some(meta)) = state.storage.kv_storage.get_by_id(&metadata_key).await {
        if let Some(status) = meta.get("status").and_then(|v| v.as_str()) {
            if crate::services::is_reprocess_lifecycle_exclusive(status) {
                tracing::warn!(
                    document_id = %document_id,
                    status = %status,
                    "Cannot re-ingest: lifecycle-exclusive status"
                );
                return Ok(false);
            }
        }
    }
    let mut transitioned = false;
    for from_status in &allowed_from_statuses {
        match state
            .storage
            .kv_storage
            .transition_if_status(&metadata_key, from_status, "deleting")
            .await
        {
            Ok(true) => {
                tracing::info!(
                    document_id = %document_id,
                    from_status = %from_status,
                    "Atomic status transition succeeded - safe to delete"
                );
                transitioned = true;
                break;
            }
            Ok(false) => continue,
            Err(e) => {
                return Err(ApiError::Internal(format!(
                    "Failed to transition status: {}",
                    e
                )));
            }
        }
    }

    if !transitioned {
        tracing::warn!(
            document_id = %document_id,
            metadata_key = %metadata_key,
            "Cannot re-ingest: document status prevents transition (processing/pending/deleting/not found)"
        );
        return Ok(false);
    }

    tracing::info!(
        document_id = %document_id,
        workspace_id = %workspace_id,
        "Re-ingestion requested - deleting existing document data (status = deleting)"
    );

    let workspace_vector_storage = get_workspace_vector_storage_strict(state, workspace_id).await?;

    let cleanup_stats: CleanupStats = cleanup_document_graph_data(
        document_id,
        &state.storage.graph_storage,
        Some(&workspace_vector_storage),
    )
    .await?;

    // SPEC-047 P1a: SSOT wipe via VectorStorage::delete_by_document (column + JSONB + id prefix).
    if let Err(e) = workspace_vector_storage
        .delete_by_document(document_id)
        .await
    {
        tracing::warn!(
            document_id = %document_id,
            error = %e,
            "Failed to delete document vectors during re-ingestion"
        );
    }

    let chunk_prefix = format!("{}-chunk-", document_id);
    let chunk_ids = state
        .storage
        .kv_storage
        .keys_with_prefix(&chunk_prefix)
        .await?;

    let mut keys_to_delete: Vec<String> = chunk_ids;
    keys_to_delete.push(metadata_key);
    keys_to_delete.push(format!("{}-content", document_id));

    state.storage.kv_storage.delete(&keys_to_delete).await?;

    // SSOT: list surfaces (wsdoc + SQL documents) must leave with the KV wipe,
    // otherwise re-ingest admits a second row while the UI still shows the old one.
    let tenant_ctx = TenantContext {
        tenant_id: None,
        workspace_id: Some(workspace_id.to_string()),
        user_id: None,
    };
    crate::services::purge_document_list_surfaces(
        state,
        document_id,
        workspace_id,
        &tenant_ctx,
        crate::services::ListSurfacePurgeOpts {
            key_prefix: Some(document_id),
            content_hash: None,
            pdf_id: None,
        },
    )
    .await?;

    tracing::info!(
        document_id = %document_id,
        chunks_deleted = keys_to_delete.len(),
        entities_removed = cleanup_stats.entities_removed,
        relationships_removed = cleanup_stats.relationships_removed,
        "Document data deleted for re-ingestion"
    );

    Ok(true)
}
