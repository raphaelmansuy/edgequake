//! Bulk deletion handler for all documents.
//!
//! Deletes all documents in the system, skipping those actively being processed
//! unless they are detected as "stuck" (>1 hour at 100% progress).
//! Also cleans up orphaned graph entities/edges and PDF table entries.
//!
//! # MISSION-02 Fixes
//!
//! - GAP-4: Entity embeddings deleted when nodes are removed
//! - GAP-5: Content-hash duplicate-detection keys deleted so re-upload is possible
//! - GAP-6: In-flight tasks cancelled before skipped documents are ignored

use axum::{extract::State, Json};
use chrono::Utc;
#[cfg(feature = "postgres")]
use edgequake_storage::ListPdfFilter;

use crate::error::ApiResult;
use crate::handlers::documents_types::*;
use crate::services::ContentHasher;
use crate::state::AppState;

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
) -> ApiResult<Json<DeleteAllDocumentsResponse>> {
    tracing::info!("Bulk delete all documents requested");

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
        let (status, updated_at_opt, stage_progress_opt) =
            if let Ok(Some(metadata)) = state.kv_storage.get_by_id(metadata_key).await {
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
                (status, updated_at, stage_progress)
            } else {
                ("unknown".to_string(), None, None)
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
            // MISSION-02-GAP-6: Cancel the in-flight task so the processor stops writing
            // data into an otherwise cleared system.
            if let Ok(Some(metadata)) = state.kv_storage.get_by_id(metadata_key).await {
                if let Some(track_id) = metadata
                    .get("track_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                {
                    let cancelled = state.cancellation_registry.cancel(&track_id).await;
                    tracing::info!(
                        document_id = %document_id,
                        track_id = %track_id,
                        cancelled,
                        "Cancelled in-flight task during bulk delete (skipping document)"
                    );
                }
            }
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

        // Attempt to delete this document
        // We'll use a simplified version that doesn't require workspace isolation
        // since we're doing a full system clear
        let chunk_prefix = format!("{}-chunk-", document_id);
        let chunk_ids: Vec<String> = keys
            .iter()
            .filter(|k| k.starts_with(&chunk_prefix))
            .cloned()
            .collect();

        let content_key = format!("{}-content", document_id);

        // Build list of all KV keys to delete for this document
        let mut kv_keys_to_delete: Vec<String> = chunk_ids.clone();
        kv_keys_to_delete.push(metadata_key.to_string());
        kv_keys_to_delete.push(content_key.clone());

        // Collect any other keys with this document prefix (e.g. -lineage)
        let extra_prefix_keys: Vec<String> = keys
            .iter()
            .filter(|k| {
                k.starts_with(&format!("{}-", document_id)) && !kv_keys_to_delete.contains(k)
            })
            .cloned()
            .collect();
        kv_keys_to_delete.extend(extra_prefix_keys);

        // MISSION-02-GAP-5: Also delete the workspace-scoped content-hash key so that
        // re-uploading the same file is possible after a bulk clear.
        if let Ok(Some(metadata)) = state.kv_storage.get_by_id(metadata_key).await {
            if let (Some(workspace_id), Some(content_hash)) = (
                metadata.get("workspace_id").and_then(|v| v.as_str()),
                metadata.get("content_hash").and_then(|v| v.as_str()),
            ) {
                let hash_key = ContentHasher::workspace_hash_key(workspace_id, content_hash);
                if !kv_keys_to_delete.contains(&hash_key) {
                    kv_keys_to_delete.push(hash_key);
                }
            }
        }

        // Delete all KV entries atomically in one call
        if let Err(e) = state.kv_storage.delete(&kv_keys_to_delete).await {
            tracing::warn!(document_id = %document_id, error = %e, "Failed to delete KV entries");
        }

        // Delete chunk embeddings from vector storage (use default / global storage for
        // bulk operations — we do not resolve per-workspace storage here for simplicity).
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

    // MISSION-02-GAP-4: Use the shared graph cleanup helper which also deletes entity
    // embeddings when nodes are removed.  Pass empty source_prefixes so every node
    // with empty source_ids after previous deletions is considered orphaned.
    //
    // WHY: Previous code only deleted graph nodes with empty source_ids but never
    // deleted the corresponding entity embeddings in vector storage, leaving stale
    // vectors that polluted future similarity searches.
    //
    // We pass NO workspace_id filter on purpose — this is a "clear all" operation,
    // so we want to clean up nodes from every workspace.  We also pass None for
    // vector_storage since we already deleted chunk vectors above; entity embeddings
    // are handled inside cleanup_document_graph_data via delete_entity().
    //
    // In practice, after the per-document KV + chunk-embedding loop above, all source
    // documents are gone.  Any remaining node whose source_ids still reference deleted
    // documents should be cleaned up here.  Rather than re-filtering by document, we
    // do a full orphan sweep: nodes whose source_ids are now empty are deleted.
    {
        let all_nodes = state.graph_storage.get_all_nodes().await?;
        let mut surviving_node_ids: std::collections::HashSet<String> =
            all_nodes.iter().map(|n| n.id.clone()).collect();

        for node in &all_nodes {
            // A node is orphaned when all its source documents have been deleted.
            // We detect this by checking that source_ids is empty (or missing).
            let has_sources = {
                let arr_empty = node
                    .properties
                    .get("source_ids")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.is_empty())
                    .unwrap_or(true);
                let legacy_empty = node
                    .properties
                    .get("source_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.is_empty())
                    .unwrap_or(true);
                !arr_empty || !legacy_empty
            };

            if !has_sources {
                // Preserve injected (Knowledge Document) nodes — GAP-9.
                let is_injected = node
                    .properties
                    .get("injected")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                if is_injected {
                    continue;
                }

                if let Err(e) = state.graph_storage.delete_node(&node.id).await {
                    tracing::warn!(node_id = %node.id, error = %e, "Failed to delete orphaned node");
                } else {
                    // MISSION-02-GAP-4: Delete entity embedding too.
                    let _ = state.vector_storage.delete_entity(&node.id).await;
                    surviving_node_ids.remove(&node.id);
                    total_entities_removed += 1;
                }
            }
        }

        // Clean up orphaned edges using the local surviving set (no extra graph call).
        let all_edges = state.graph_storage.get_all_edges().await?;
        for edge in all_edges {
            let is_orphaned = !surviving_node_ids.contains(&edge.source)
                || !surviving_node_ids.contains(&edge.target);

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
