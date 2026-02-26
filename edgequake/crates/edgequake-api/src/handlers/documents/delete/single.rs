//! Single document deletion handler.
//!
//! Cascade-deletes a document: KV entries, chunk embeddings, graph entities,
//! graph edges, and content-hash duplicate-detection key (OODA-90).

use axum::{extract::State, Json};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::handlers::documents_types::*;
use crate::services::ContentHasher;
use crate::state::AppState;
use edgequake_core::MetricsTriggerType;

use super::super::storage_helpers::{extract_source_docs, get_workspace_vector_storage_strict};

/// Delete a document by ID.
#[utoipa::path(
    delete,
    path = "/api/v1/documents/{document_id}",
    tag = "Documents",
    params(
        ("document_id" = String, Path, description = "Document ID to delete")
    ),
    responses(
        (status = 200, description = "Document deleted", body = DeleteDocumentResponse),
        (status = 404, description = "Document not found")
    )
)]
pub async fn delete_document(
    State(state): State<AppState>,
    axum::extract::Path(document_id): axum::extract::Path<String>,
) -> ApiResult<Json<DeleteDocumentResponse>> {
    let keys = state.kv_storage.keys().await?;

    // Find chunks belonging to this document
    let chunk_prefix = format!("{}-chunk-", document_id);
    let chunk_ids: Vec<String> = keys
        .iter()
        .filter(|k| k.starts_with(&chunk_prefix))
        .cloned()
        .collect();

    // Also check for metadata and content keys
    let metadata_key = format!("{}-metadata", document_id);
    let content_key = format!("{}-content", document_id);
    let has_metadata = keys.contains(&metadata_key);
    let has_content = keys.contains(&content_key);

    // Document must have either chunks, metadata, or content
    if chunk_ids.is_empty() && !has_metadata && !has_content {
        return Err(ApiError::NotFound(format!(
            "Document {} not found",
            document_id
        )));
    }

    // SPEC-033: Get workspace_id from document metadata for vector storage isolation
    // OODA-02: Also check document status for safe deletion
    // OODA-90: Extract content_hash for hash key cleanup
    // FIX-ISSUE-73: Extract pdf_id for pdf_documents cleanup
    let (workspace_id_for_storage, document_status, content_hash_opt, pdf_id_opt) = if has_metadata
    {
        if let Ok(Some(metadata)) = state.kv_storage.get_by_id(&metadata_key).await {
            let workspace = metadata
                .get("workspace_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| "default".to_string());
            let status = metadata
                .get("status")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| "unknown".to_string());
            // OODA-90: Extract content hash for duplicate detection key cleanup
            let content_hash = metadata
                .get("content_hash")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            // FIX-ISSUE-73: Extract pdf_id for pdf_documents cascade cleanup
            let pdf_id = metadata
                .get("pdf_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            (workspace, status, content_hash, pdf_id)
        } else {
            ("default".to_string(), "unknown".to_string(), None, None)
        }
    } else {
        ("default".to_string(), "unknown".to_string(), None, None)
    };

    // OODA-02: Safety check - prevent deletion of documents that are still being processed
    // WHY: Deleting a document while it's being processed can cause:
    //   1. Race condition: Background task writes data while deletion removes it
    //   2. Orphaned data: Entities/edges created AFTER deletion check starts
    //   3. Partial deletion: Some entities exist, others don't
    //
    // Status lifecycle (FIX-5: Added partial_failure):
    //   "pending"         → Cannot delete (queued for processing)
    //   "processing"      → Cannot delete (actively being processed)
    //   "completed"       → Can delete (processing finished successfully with entities)
    //   "processed"       → Can delete (legacy status, same as completed)
    //   "partial_failure" → Can delete (processed but 0 entities extracted - FIX-5)
    //   "failed"          → Can delete (processing failed, cleanup partial data)
    //   "unknown"         → Can delete (legacy documents without status)
    match document_status.as_str() {
        "pending" => {
            tracing::warn!(
                document_id = %document_id,
                status = %document_status,
                "Rejecting deletion of pending document"
            );
            return Err(ApiError::Conflict(format!(
                "Cannot delete document '{}' with status 'pending'. \
                 The document is queued for processing. \
                 Please wait for processing to complete or cancel the task.",
                document_id
            )));
        }
        "processing" => {
            tracing::warn!(
                document_id = %document_id,
                status = %document_status,
                "Rejecting deletion of processing document"
            );
            return Err(ApiError::Conflict(format!(
                "Cannot delete document '{}' with status 'processing'. \
                 The document is currently being processed. \
                 Please wait for processing to complete or cancel the task.",
                document_id
            )));
        }
        "completed" | "processed" | "partial_failure" | "failed" | "cancelled" | "unknown" => {
            // OK to delete
            // OODA-13: Added "cancelled" status to explicitly allow deletion after task cancellation
            tracing::debug!(
                document_id = %document_id,
                status = %document_status,
                "Document status allows deletion"
            );
        }
        other => {
            // Unknown status - allow deletion with warning
            tracing::warn!(
                document_id = %document_id,
                status = %other,
                "Unknown document status, allowing deletion"
            );
        }
    }

    // SPEC-028: Collect chunk IDs for vector storage deletion
    // Clone chunk_ids before workspace_vector_storage operations
    let keys_to_delete_for_vectors: Vec<String> = chunk_ids.clone();

    // SPEC-033: Get workspace-specific vector storage for deletion
    // WHY-OODA223: STRICT mode - fail loudly if workspace storage unavailable
    // to ensure we delete from the correct workspace table, not a fallback
    let workspace_vector_storage =
        get_workspace_vector_storage_strict(&state, &workspace_id_for_storage).await?;

    let chunks_deleted = chunk_ids.len();
    let mut entities_removed = 0usize;
    let mut entities_updated = 0usize;
    let mut relationships_removed = 0usize;
    let mut relationships_updated = 0usize;
    let mut embeddings_deleted = 0usize;

    // SPEC-028: Delete chunk embeddings from vector storage first
    // WHY: Chunks are stored with IDs like "doc-xxx-chunk-0", delete them
    let chunk_embedding_ids: Vec<String> = keys_to_delete_for_vectors.clone();
    if !chunk_embedding_ids.is_empty() {
        if let Err(e) = workspace_vector_storage.delete(&chunk_embedding_ids).await {
            tracing::warn!(
                document_id = %document_id,
                error = %e,
                "Failed to delete chunk embeddings, continuing with graph cleanup"
            );
        } else {
            embeddings_deleted += chunk_embedding_ids.len();
            tracing::debug!(
                document_id = %document_id,
                count = chunk_embedding_ids.len(),
                "Deleted chunk embeddings"
            );
        }
    }

    // Cascade delete: Process graph entities - remove document sources
    let all_nodes = state.graph_storage.get_all_nodes().await?;
    for node in all_nodes {
        let sources = extract_source_docs(&node.properties);
        if sources.is_empty() {
            continue;
        }

        // Filter out sources that belong to this document
        let remaining_sources: Vec<String> = sources
            .iter()
            .filter(|s| {
                !s.starts_with(&chunk_prefix) && *s != &document_id && !s.starts_with(&document_id)
            })
            .cloned()
            .collect();

        if remaining_sources.is_empty() {
            // No sources left - delete the entity entirely

            // WHY-OODA01: DO NOT delete edges here!
            // Edges have their own source_ids tracking and will be processed
            // independently in the edge processing loop below (line ~1500).
            // Deleting them here would cause data loss if the edge has other
            // source documents that are not being deleted.
            //
            // Example bug scenario (fixed):
            //   Document A: "Alice works at Google"
            //   Document B: "Alice graduated from MIT"
            //   DELETE Document A:
            //     - ALICE entity sources: [doc_a, doc_b] → [doc_b] (update)
            //     - GOOGLE entity sources: [doc_a] → [] (delete entity)
            //     - OLD BUG: Deleted ALL edges from GOOGLE, including MIT edge!
            //     - FIXED: Edges are processed separately based on their own sources

            // Delete the node (backend may cascade edges, but we handle explicitly below)
            state.graph_storage.delete_node(&node.id).await?;
            // SPEC-033: Use workspace-specific vector storage for entity deletion
            let _ = workspace_vector_storage.delete_entity(&node.id).await;
            entities_removed += 1;
        } else if remaining_sources.len() < sources.len() {
            // Some sources were removed - update the entity
            let mut updated_props = node.properties.clone();
            // Use source_ids (JSON array) format for updates
            updated_props.insert(
                "source_ids".to_string(),
                serde_json::json!(remaining_sources),
            );
            state
                .graph_storage
                .upsert_node(&node.id, updated_props)
                .await?;
            entities_updated += 1;
        }
    }

    // Process graph edges - remove document sources
    // WHY-OODA01: We must also check for orphaned edges (edges connecting to deleted nodes)
    // This handles the case where a node was deleted above but edges still reference it.
    let all_edges = state.graph_storage.get_all_edges().await?;

    // Get current node IDs for orphan detection
    let existing_nodes = state.graph_storage.get_all_nodes().await?;
    let existing_node_ids: std::collections::HashSet<String> =
        existing_nodes.iter().map(|n| n.id.clone()).collect();

    for edge in all_edges {
        // Check if edge is orphaned (connects to deleted node)
        let is_orphaned =
            !existing_node_ids.contains(&edge.source) || !existing_node_ids.contains(&edge.target);

        if is_orphaned {
            // Edge connects to a deleted node - delete it
            state
                .graph_storage
                .delete_edge(&edge.source, &edge.target)
                .await?;
            relationships_removed += 1;
            tracing::debug!(
                source = %edge.source,
                target = %edge.target,
                "Deleted orphaned edge (connects to deleted node)"
            );
            continue;
        }

        let sources = extract_source_docs(&edge.properties);
        if sources.is_empty() {
            continue;
        }

        // Filter out sources that belong to this document
        let remaining_sources: Vec<String> = sources
            .iter()
            .filter(|s| {
                !s.starts_with(&chunk_prefix) && *s != &document_id && !s.starts_with(&document_id)
            })
            .cloned()
            .collect();

        if remaining_sources.is_empty() {
            // No sources left - delete the relationship
            state
                .graph_storage
                .delete_edge(&edge.source, &edge.target)
                .await?;
            relationships_removed += 1;
        } else if remaining_sources.len() < sources.len() {
            // Some sources were removed - update the relationship
            let mut updated_props = edge.properties.clone();
            // Use source_ids (JSON array) format for updates
            updated_props.insert(
                "source_ids".to_string(),
                serde_json::json!(remaining_sources),
            );
            state
                .graph_storage
                .upsert_edge(&edge.source, &edge.target, updated_props)
                .await?;
            relationships_updated += 1;
        }
    }

    // Collect all keys to delete from KV storage
    let mut keys_to_delete = keys_to_delete_for_vectors;
    if has_metadata {
        keys_to_delete.push(metadata_key);
    }
    if has_content {
        keys_to_delete.push(content_key);
    }

    // OODA-90: Delete workspace-scoped hash key to allow re-upload of same content
    // WHY: If we don't delete the hash key, the duplicate detection will still
    // block uploads of the same content even after the document is deleted.
    if let Some(content_hash) = content_hash_opt {
        let hash_key = ContentHasher::workspace_hash_key(&workspace_id_for_storage, &content_hash);
        keys_to_delete.push(hash_key.clone());
        tracing::debug!(
            hash_key = %hash_key,
            document_id = %document_id,
            "Adding hash key to deletion list for duplicate detection cleanup"
        );
    }

    // Delete all document data from KV storage
    state.kv_storage.delete(&keys_to_delete).await?;

    // FIX-ISSUE-73: Cascade delete pdf_documents, chunks, and the documents row.
    // WHY: Previously only KV/graph/vector data was cleaned up, leaving orphaned rows
    // in pdf_documents, chunks, and documents tables (GitHub Issue #73).
    #[cfg(feature = "postgres")]
    {
        if let Some(ref pdf_storage) = state.pdf_storage {
            // 1. Delete from pdf_documents if this is a PDF document
            if let Some(ref pid) = pdf_id_opt {
                if let Ok(pdf_uuid) = Uuid::parse_str(pid) {
                    if let Err(e) = pdf_storage.delete_pdf(&pdf_uuid).await {
                        tracing::warn!(
                            pdf_id = %pid,
                            document_id = %document_id,
                            error = %e,
                            "Failed to delete pdf_documents row (may already be gone)"
                        );
                    } else {
                        tracing::debug!(
                            pdf_id = %pid,
                            document_id = %document_id,
                            "Deleted pdf_documents row"
                        );
                    }
                }
            }

            // 2. Delete from documents relational table (cascades to chunks via FK)
            if let Ok(doc_uuid) = Uuid::parse_str(&document_id) {
                if let Err(e) = pdf_storage.delete_document_record(&doc_uuid).await {
                    tracing::warn!(
                        document_id = %document_id,
                        error = %e,
                        "Failed to delete documents table row (may not exist)"
                    );
                } else {
                    tracing::debug!(
                        document_id = %document_id,
                        "Deleted documents table row (cascaded to chunks)"
                    );
                }
            }
        }
    }

    tracing::info!(
        document_id = %document_id,
        chunks = chunks_deleted,
        embeddings_deleted = embeddings_deleted,
        entities_removed = entities_removed,
        entities_updated = entities_updated,
        relationships_removed = relationships_removed,
        relationships_updated = relationships_updated,
        "Document cascade delete complete"
    );

    // OODA-21: Record metrics snapshot for trend analysis after deletion
    // Best-effort: log error but don't fail the deletion
    if let Ok(workspace_uuid) = Uuid::parse_str(&workspace_id_for_storage) {
        if let Err(e) = state
            .workspace_service
            .record_metrics_snapshot(workspace_uuid, MetricsTriggerType::Event)
            .await
        {
            tracing::warn!(
                workspace_id = %workspace_id_for_storage,
                error = %e,
                "Failed to record post-deletion metrics snapshot"
            );
        } else {
            tracing::debug!(
                workspace_id = %workspace_id_for_storage,
                "Recorded post-deletion metrics snapshot"
            );
        }
    }

    Ok(Json(DeleteDocumentResponse {
        document_id,
        deleted: true,
        chunks_deleted,
        entities_affected: entities_removed + entities_updated,
        relationships_affected: relationships_removed + relationships_updated,
    }))
}
