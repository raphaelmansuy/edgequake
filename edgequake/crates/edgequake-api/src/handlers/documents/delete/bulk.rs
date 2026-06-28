//! Bulk deletion handler for all documents.
//!
//! Deletes all documents in the system, skipping those actively being processed
//! unless they are detected as "stuck" (>1 hour at 100% progress).
//! Also cleans up orphaned graph entities/edges and PDF table entries.

use crate::error::ApiResult;
use crate::handlers::documents_types::*;
use crate::middleware::TenantContext;
use crate::state::AppState;
use axum::{extract::State, http::HeaderMap, Json};
use chrono::Utc;

use crate::services::document_metadata_scan::{
    document_id_from_metadata_key, load_scoped_document_metadata_entries,
};
use crate::services::{cascade_remove_document_sources, DocumentSourceScope};

use super::super::storage_helpers::purge_persisted_tasks_for_document;

/// Delete all documents in the system (bulk deletion).
///
/// This endpoint allows users to clear all documents from the system.
/// Documents that are actively being processed (pending/processing status)
/// will be skipped to prevent data corruption.
///
/// WHY: Frontend "Clear All" button needs this endpoint to remove stuck
/// or failed documents in bulk rather than deleting one by one.
#[utoipa::path(
    delete,
    path = "/api/v1/documents",
    tag = "Documents",
    responses(
        (status = 200, description = "Documents deleted", body = DeleteAllDocumentsResponse),
        (status = 500, description = "Internal error")
    )
)]
pub async fn delete_all_documents(
    State(state): State<AppState>,
    headers: HeaderMap,
    tenant_ctx: TenantContext,
) -> ApiResult<Json<DeleteAllDocumentsResponse>> {
    if state.security.require_delete_all_confirm {
        let confirmed = headers
            .get("x-edgequake-confirm")
            .and_then(|value| value.to_str().ok())
            == Some("delete-all-documents");
        if !confirmed {
            tracing::warn!(
                workspace_id = ?tenant_ctx.workspace_id,
                "Bulk delete rejected — missing X-EdgeQuake-Confirm header (SPEC-027 IMP-018)"
            );
            return Err(crate::error::ApiError::BadRequest(
                "Bulk delete requires header X-EdgeQuake-Confirm: delete-all-documents".into(),
            ));
        }
    } else if headers.get("x-edgequake-confirm").is_none() {
        tracing::warn!(
            workspace_id = ?tenant_ctx.workspace_id,
            "Bulk delete without confirm header — set EDGEQUAKE_REQUIRE_DELETE_ALL_CONFIRM=true to enforce"
        );
    }

    tracing::info!(workspace_id = ?tenant_ctx.workspace_id, "Bulk delete documents requested");

    let scoped_entries =
        load_scoped_document_metadata_entries(state.storage.kv_storage.as_ref(), &tenant_ctx)
            .await?;

    let mut deleted_count = 0usize;
    let mut total_chunks_deleted = 0usize;
    let mut total_entities_removed = 0usize;
    let mut total_relationships_removed = 0usize;
    let mut skipped_count = 0usize;
    let mut skipped_documents = Vec::new();
    #[cfg(feature = "postgres")]
    let mut pdf_ids_to_delete = std::collections::HashSet::new();

    // Define stuck threshold: documents processing for > 1 hour are considered stuck
    let stuck_threshold_secs = 3600; // 1 hour

    for (metadata_key, metadata) in scoped_entries {
        let document_id = document_id_from_metadata_key(&metadata_key).unwrap_or_else(|| {
            metadata
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or(&metadata_key)
                .to_string()
        });

        #[cfg(feature = "postgres")]
        let mut pdf_id_for_delete: Option<String> = None;

        let (status, updated_at_opt, stage_progress_opt, track_id_opt, workspace_id_opt) =
            if let Some(obj) = metadata.as_object() {
                let status = obj
                    .get("status")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "unknown".to_string());
                let updated_at = obj
                    .get("updated_at")
                    .and_then(|v| v.as_str())
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                    .map(|dt| dt.with_timezone(&chrono::Utc));
                let stage_progress = obj.get("stage_progress").and_then(|v| v.as_f64());
                let track_id = obj
                    .get("track_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let workspace_id = obj
                    .get("workspace_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                #[cfg(feature = "postgres")]
                {
                    pdf_id_for_delete = obj
                        .get("pdf_id")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                }
                (status, updated_at, stage_progress, track_id, workspace_id)
            } else {
                ("unknown".to_string(), None, None, None, None)
            };

        // Skip documents that are actively being processed (unless stuck)
        // A document is considered stuck if:
        //   - Status is "processing" or "pending"
        //   - AND updated_at is more than stuck_threshold_secs ago
        //   - AND stage_progress is 1.0 (100%) or close to it
        let is_stuck = if status == "pending" || status == "processing" {
            if let Some(updated_at) = updated_at_opt {
                let age_secs = (Utc::now() - updated_at).num_seconds();
                let high_progress = stage_progress_opt.map(|p| p >= 0.99).unwrap_or(false);
                age_secs > stuck_threshold_secs && high_progress
            } else {
                false
            }
        } else {
            false
        };

        if (status == "pending" || status == "processing") && !is_stuck {
            tracing::debug!(
                document_id = %document_id,
                status = %status,
                "Skipping bulk delete of document with active processing"
            );
            skipped_count += 1;
            skipped_documents.push(document_id.clone());
            continue;
        }

        if is_stuck {
            tracing::info!(
                document_id = %document_id,
                status = %status,
                "Deleting stuck document (>1 hour at 100% progress)"
            );
        }

        // WHY: Deleting KV data without purging persisted tasks allows startup
        // recovery to resurrect documents the user explicitly cleared.
        let _ = purge_persisted_tasks_for_document(
            &state,
            &document_id,
            track_id_opt.as_deref(),
            workspace_id_opt.as_deref(),
        )
        .await;

        // Attempt to delete this document within the validated workspace scope.
        let chunk_prefix = format!("{}-chunk-", document_id);
        let chunk_ids = state
            .storage
            .kv_storage
            .keys_with_prefix(&chunk_prefix)
            .await?;

        let content_key = format!("{}-content", document_id);

        // Delete from KV storage - delete takes a slice of strings
        if !chunk_ids.is_empty() {
            if let Err(e) = state.storage.kv_storage.delete(&chunk_ids).await {
                tracing::warn!(document_id = %document_id, error = %e, "Failed to delete chunks");
            }
        }

        // Delete metadata key
        if let Err(e) = state
            .storage
            .kv_storage
            .delete(std::slice::from_ref(&metadata_key))
            .await
        {
            tracing::warn!(key = %metadata_key, error = %e, "Failed to delete metadata");
        }

        // Delete content key
        if let Err(e) = state
            .storage
            .kv_storage
            .delete(std::slice::from_ref(&content_key))
            .await
        {
            tracing::warn!(key = %content_key, error = %e, "Failed to delete content");
        }

        // Delete from vector storage (use default storage for bulk operations)
        if !chunk_ids.is_empty() {
            if let Err(e) = state.storage.vector_storage.delete(&chunk_ids).await {
                tracing::warn!(
                    document_id = %document_id,
                    error = %e,
                    "Failed to delete chunk embeddings"
                );
            }
        }

        // SPEC-006 P1: per-document bounded graph cascade (no post-hoc full scan)
        let scope = DocumentSourceScope::from_document_id(document_id.clone());
        match cascade_remove_document_sources(
            &state.storage.graph_storage,
            None,
            Some(&tenant_ctx),
            &scope,
        )
        .await
        {
            Ok(stats) => {
                total_entities_removed += stats.entities_removed;
                total_relationships_removed += stats.relationships_removed;
            }
            Err(e) => {
                tracing::warn!(
                    document_id = %document_id,
                    error = %e,
                    "Failed graph cascade during bulk delete"
                );
            }
        }

        total_chunks_deleted += chunk_ids.len();
        deleted_count += 1;

        #[cfg(feature = "postgres")]
        if let Some(pdf_id) = pdf_id_for_delete {
            pdf_ids_to_delete.insert(pdf_id);
        }

        tracing::debug!(
            document_id = %document_id,
            chunks = chunk_ids.len(),
            "Deleted document in bulk operation"
        );
    }

    // Clean up pdf_documents rows linked to deleted workspace documents only.
    // WHY: Duplicate detection uses checksum in pdf_documents; must not delete other workspaces.
    #[allow(unused_mut)] // mut only used when postgres feature is enabled
    let mut total_pdfs_deleted = 0usize;
    #[cfg(feature = "postgres")]
    if let Some(ref pdf_storage) = state.storage.pdf_storage {
        for pdf_id_str in pdf_ids_to_delete {
            if let Ok(pdf_uuid) = uuid::Uuid::parse_str(&pdf_id_str) {
                if let Err(e) = pdf_storage.delete_pdf(&pdf_uuid).await {
                    tracing::warn!(
                        pdf_id = %pdf_id_str,
                        error = %e,
                        "Failed to delete PDF document during bulk delete"
                    );
                } else {
                    total_pdfs_deleted += 1;
                }
            }
        }
    }

    tracing::info!(
        deleted = deleted_count,
        skipped = skipped_count,
        chunks = total_chunks_deleted,
        entities = total_entities_removed,
        relationships = total_relationships_removed,
        pdfs = total_pdfs_deleted,
        "Bulk delete complete"
    );

    Ok(Json(DeleteAllDocumentsResponse {
        deleted_count,
        total_chunks_deleted,
        total_entities_removed,
        total_relationships_removed,
        total_pdfs_deleted,
        skipped_count,
        skipped_documents,
    }))
}
