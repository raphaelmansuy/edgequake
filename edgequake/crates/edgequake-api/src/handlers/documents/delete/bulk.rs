//! Bulk deletion handler for all documents.
//!
//! Deletes all documents in the system, skipping those actively being processed
//! unless they are detected as "stuck" (>1 hour at 100% progress).
//! Also cleans up orphaned graph entities/edges and PDF table entries.

use axum::{extract::State, Json};
use chrono::Utc;
#[cfg(feature = "postgres")]
use edgequake_storage::ListPdfFilter;

use crate::error::ApiResult;
use crate::handlers::documents_types::*;
use crate::middleware::TenantContext;
use crate::state::AppState;

use super::super::storage_helpers::{
    metadata_matches_tenant_context, purge_persisted_tasks_for_document,
};

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
    tenant_ctx: TenantContext,
) -> ApiResult<Json<DeleteAllDocumentsResponse>> {
    tracing::info!(workspace_id = ?tenant_ctx.workspace_id, "Bulk delete documents requested");

    let keys = state.kv_storage.keys().await?;

    // Find all document metadata keys to identify unique documents
    let metadata_keys: Vec<String> = keys
        .iter()
        .filter(|k| k.ends_with("-metadata"))
        .cloned()
        .collect();

    let mut deleted_count = 0usize;
    let mut total_chunks_deleted = 0usize;
    let mut total_entities_removed = 0usize;
    let mut total_relationships_removed = 0usize;
    let mut skipped_count = 0usize;
    let mut skipped_documents = Vec::new();

    // Define stuck threshold: documents processing for > 1 hour are considered stuck
    let stuck_threshold_secs = 3600; // 1 hour

    for metadata_key in &metadata_keys {
        // Extract document_id from metadata key (format: {document_id}-metadata)
        let document_id = metadata_key.trim_end_matches("-metadata").to_string();

        // Get document status and metadata to check if safe to delete
        let (status, updated_at_opt, stage_progress_opt, track_id_opt, workspace_id_opt) =
            if let Ok(Some(metadata)) = state.kv_storage.get_by_id(metadata_key).await {
                // WHY: "Clear all" in the UI is scoped to the current workspace.
                // Never trust raw metadata scans without re-checking the request context.
                if !metadata_matches_tenant_context(&metadata, &tenant_ctx) {
                    tracing::debug!(document_id = %document_id, "Skipping out-of-scope document during bulk delete");
                    continue;
                }

                let status = metadata
                    .get("status")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "unknown".to_string());
                let updated_at = metadata
                    .get("updated_at")
                    .and_then(|v| v.as_str())
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                    .map(|dt| dt.with_timezone(&chrono::Utc));
                let stage_progress = metadata.get("stage_progress").and_then(|v| v.as_f64());
                let track_id = metadata
                    .get("track_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let workspace_id = metadata
                    .get("workspace_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
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
        let chunk_ids: Vec<String> = keys
            .iter()
            .filter(|k| k.starts_with(&chunk_prefix))
            .cloned()
            .collect();

        let content_key = format!("{}-content", document_id);

        // Delete from KV storage - delete takes a slice of strings
        if !chunk_ids.is_empty() {
            if let Err(e) = state.kv_storage.delete(&chunk_ids).await {
                tracing::warn!(document_id = %document_id, error = %e, "Failed to delete chunks");
            }
        }

        // Delete metadata key
        if let Err(e) = state
            .kv_storage
            .delete(std::slice::from_ref(metadata_key))
            .await
        {
            tracing::warn!(key = %metadata_key, error = %e, "Failed to delete metadata");
        }

        // Delete content key
        if let Err(e) = state
            .kv_storage
            .delete(std::slice::from_ref(&content_key))
            .await
        {
            tracing::warn!(key = %content_key, error = %e, "Failed to delete content");
        }

        // Delete from vector storage (use default storage for bulk operations)
        if !chunk_ids.is_empty() {
            if let Err(e) = state.vector_storage.delete(&chunk_ids).await {
                tracing::warn!(
                    document_id = %document_id,
                    error = %e,
                    "Failed to delete chunk embeddings"
                );
            }
        }

        total_chunks_deleted += chunk_ids.len();
        deleted_count += 1;

        tracing::debug!(
            document_id = %document_id,
            chunks = chunk_ids.len(),
            "Deleted document in bulk operation"
        );
    }

    // Clean up orphaned graph entities (entities with no remaining source documents)
    // This is a simplified cleanup - full cascade is done per-document for precision
    let all_nodes = state.graph_storage.get_all_nodes().await?;
    for node in all_nodes {
        // Check if node has any source references
        let has_sources = node
            .properties
            .get("source_ids")
            .and_then(|v| v.as_array())
            .map(|arr| !arr.is_empty())
            .unwrap_or(false);

        if !has_sources {
            // Node has no sources, check source_id too
            let has_legacy_source = node
                .properties
                .get("source_id")
                .and_then(|v| v.as_str())
                .map(|s| !s.is_empty())
                .unwrap_or(false);

            if !has_legacy_source {
                // No sources at all, delete the orphaned entity
                if let Err(e) = state.graph_storage.delete_node(&node.id).await {
                    tracing::warn!(node_id = %node.id, error = %e, "Failed to delete orphaned node");
                } else {
                    total_entities_removed += 1;
                }
            }
        }
    }

    // Clean up orphaned edges
    let all_edges = state.graph_storage.get_all_edges().await?;
    let remaining_nodes = state.graph_storage.get_all_nodes().await?;
    let remaining_node_ids: std::collections::HashSet<String> =
        remaining_nodes.iter().map(|n| n.id.clone()).collect();

    for edge in all_edges {
        let is_orphaned = !remaining_node_ids.contains(&edge.source)
            || !remaining_node_ids.contains(&edge.target);

        if is_orphaned {
            if let Err(e) = state
                .graph_storage
                .delete_edge(&edge.source, &edge.target)
                .await
            {
                tracing::warn!(
                    source = %edge.source,
                    target = %edge.target,
                    error = %e,
                    "Failed to delete orphaned edge"
                );
            } else {
                total_relationships_removed += 1;
            }
        }
    }

    // Clean up PDF documents table
    // WHY: PDF documents have their own table separate from KV storage
    // The duplicate detection uses checksum from pdf_documents table, so we must clear it
    #[allow(unused_mut)] // mut only used when postgres feature is enabled
    let mut total_pdfs_deleted = 0usize;
    #[cfg(feature = "postgres")]
    if let Some(ref pdf_storage) = state.pdf_storage {
        // List all PDFs (no workspace filter to ensure full cleanup)
        let filter = ListPdfFilter {
            workspace_id: None,
            processing_status: None,
            page: Some(1),
            page_size: Some(10000), // Large page size to get all
        };

        match pdf_storage.list_pdfs(filter).await {
            Ok(pdf_list) => {
                for pdf in pdf_list.items {
                    if let Err(e) = pdf_storage.delete_pdf(&pdf.pdf_id).await {
                        tracing::warn!(
                            pdf_id = %pdf.pdf_id,
                            error = %e,
                            "Failed to delete PDF document"
                        );
                    } else {
                        total_pdfs_deleted += 1;
                    }
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "Failed to list PDF documents for cleanup");
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
