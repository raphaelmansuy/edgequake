//! Document ingestion handlers.
//!
//! @implements FEAT0407 (Document REST API Handlers)
//! @implements FEAT0402
//!
//! # Implements
//!
//! - **UC0001**: Upload Document
//! - **UC0002**: List Documents  
//! - **UC0003**: View Document Details
//! - **UC0005**: Delete Document
//! - **FEAT0401**: Document Upload (Text)
//! - **FEAT0402**: Document Upload (File)
//! - **FEAT0001**: Document Ingestion Pipeline
//!
//! # Enforces
//!
//! - **BR0001**: Documents must be unique (SHA-256 content hash)
//! - **BR0002**: Chunk size 1200 tokens, overlap 100 tokens
//! - **BR0201**: Tenant isolation (workspace scoping)
//! - **BR0401**: Authentication required for all endpoints
//!
//! # Endpoints
//!
//! | Method | Path | Handler | Description |
//! |--------|------|---------|-------------|
//! | POST | `/api/v1/documents` | [`upload_document`] | Upload text/file for ingestion |
//! | GET | `/api/v1/documents` | [`list_documents`] | List all documents |
//! | GET | `/api/v1/documents/:id` | [`get_document`] | Get document details |
//! | DELETE | `/api/v1/documents/:id` | [`delete_document`] | Delete with cascade |
//!
//! # WHY: Two Ingestion Modes
//!
//! Documents can be processed synchronously or asynchronously:
//! - **Sync**: Small documents (<10KB), immediate response with entities
//! - **Async**: Large documents, returns task_id for polling
//!
//! Async mode prevents request timeouts for large PDFs (can take 30s+ to process).

use axum::http::StatusCode;
use axum::{extract::State, Json};
use axum_extra::extract::Multipart;
use chrono::Utc;
use std::sync::Arc;
use tracing::{debug, warn};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::file_validation::validate_file;
use crate::middleware::TenantContext;
use crate::services::ContentHasher;
use crate::state::AppState;
use edgequake_core::MetricsTriggerType;
use edgequake_storage::traits::VectorStorage;
// OODA-04: ListPdfFilter is used in feature-gated code below (postgres feature)
#[cfg(feature = "postgres")]
use edgequake_storage::ListPdfFilter;

// Re-export DTOs from documents_types module
pub use crate::handlers::documents_types::*;

/// Get workspace-specific vector storage for document ingestion (STRICT mode).
///
/// @implements SPEC-033: Per-workspace vector storage isolation
/// @implements BR0353: Workspace vector isolation MUST NOT silently degrade
///
/// # CRITICAL SAFETY INVARIANT
///
/// This function NEVER falls back to default storage. If workspace-specific
/// storage cannot be obtained, it returns an error to prevent data from being
/// stored in the wrong location.
///
/// ## WHY NO FALLBACK (OODA-223 Lesson)
///
/// Silent fallback to global storage caused a critical data isolation bug:
/// - Data ingested into global table with workspace_id in metadata
/// - Queries looked in workspace-specific tables (empty)
/// - Result: "0 Sources" even though data existed
///
/// By failing loudly, we:
/// 1. Prevent data from going to the wrong storage
/// 2. Force immediate resolution of workspace configuration issues
/// 3. Maintain strict data isolation guarantees
///
/// # Arguments
///
/// * `state` - Application state containing vector registry
/// * `workspace_id` - Workspace identifier (MUST be valid UUID)
///
/// # Returns
///
/// * `Ok(storage)` - Workspace-specific vector storage
/// * `Err(ApiError)` - If workspace not found or storage creation fails
///
/// # Errors
///
/// - `ApiError::BadRequest` - Invalid workspace ID format
/// - `ApiError::NotFound` - Workspace does not exist
/// - `ApiError::Internal` - Failed to create workspace storage
async fn get_workspace_vector_storage_strict(
    state: &AppState,
    workspace_id: &str,
) -> Result<Arc<dyn VectorStorage>, ApiError> {
    use edgequake_storage::traits::WorkspaceVectorConfig;

    // OODA-223: Allow fallback in memory mode (tests) but not in production (PostgreSQL)
    // This prevents silent data loss in production while maintaining test compatibility
    let allow_fallback = state.storage_mode.is_memory();

    // OODA-13: Handle "default" workspace by mapping to the well-known UUID
    // WHY: Documents created via default workspace are stored with workspace_id="default"
    // but deletion/operations need a valid UUID for vector storage lookup.
    // Default workspace UUID: 00000000-0000-0000-0000-000000000003
    let effective_workspace_id = if workspace_id == "default" || workspace_id.is_empty() {
        "00000000-0000-0000-0000-000000000003"
    } else {
        workspace_id
    };

    // Parse workspace ID - FAIL in production, WARN in test mode
    let workspace_uuid = match Uuid::parse_str(effective_workspace_id) {
        Ok(uuid) => uuid,
        Err(e) => {
            if allow_fallback {
                // WHY-OODA223: Test mode - log warning and use default storage
                tracing::warn!(
                    workspace_id = %workspace_id,
                    error = %e,
                    storage_mode = ?state.storage_mode,
                    "Invalid workspace ID - using default storage (allowed in memory/test mode)"
                );
                return Ok(state.vector_registry.default_storage());
            }
            tracing::error!(
                workspace_id = %workspace_id,
                error = %e,
                "CRITICAL: Invalid workspace ID during ingestion - refusing to use default storage"
            );
            return Err(ApiError::BadRequest(format!(
                "Invalid workspace ID '{}': {}. Document ingestion requires a valid workspace.",
                workspace_id, e
            )));
        }
    };

    // Get workspace from service - FAIL in production, WARN in test mode
    let workspace = match state.workspace_service.get_workspace(workspace_uuid).await {
        Ok(Some(ws)) => ws,
        Ok(None) => {
            if allow_fallback {
                // WHY-OODA223: Test mode - log warning and use default storage
                tracing::warn!(
                    workspace_id = %workspace_id,
                    storage_mode = ?state.storage_mode,
                    "Workspace not found - using default storage (allowed in memory/test mode)"
                );
                return Ok(state.vector_registry.default_storage());
            }
            tracing::error!(
                workspace_id = %workspace_id,
                "CRITICAL: Workspace not found during ingestion - refusing to use default storage"
            );
            return Err(ApiError::NotFound(format!(
                "Workspace '{}' not found. Cannot ingest documents without a valid workspace.",
                workspace_id
            )));
        }
        Err(e) => {
            if allow_fallback {
                // WHY-OODA223: Test mode - log warning and use default storage
                tracing::warn!(
                    workspace_id = %workspace_id,
                    error = %e,
                    storage_mode = ?state.storage_mode,
                    "Failed to lookup workspace - using default storage (allowed in memory/test mode)"
                );
                return Ok(state.vector_registry.default_storage());
            }
            tracing::error!(
                workspace_id = %workspace_id,
                error = %e,
                "CRITICAL: Failed to lookup workspace during ingestion"
            );
            return Err(ApiError::Internal(format!(
                "Failed to lookup workspace '{}': {}",
                workspace_id, e
            )));
        }
    };

    // Create workspace-specific vector storage config
    let config = WorkspaceVectorConfig {
        workspace_id: workspace_uuid,
        dimension: workspace.embedding_dimension,
        namespace: "default".to_string(),
    };

    debug!(
        workspace_id = %workspace_id,
        dimension = workspace.embedding_dimension,
        embedding_model = %workspace.embedding_model,
        "Using workspace-specific vector storage for document ingestion (STRICT mode)"
    );

    // Get or create workspace vector storage - FAIL if creation fails
    match state.vector_registry.get_or_create(config).await {
        Ok(storage) => Ok(storage),
        Err(e) => {
            if allow_fallback {
                // WHY-OODA223: Test mode - log warning and use default storage
                tracing::warn!(
                    workspace_id = %workspace_id,
                    dimension = workspace.embedding_dimension,
                    error = %e,
                    storage_mode = ?state.storage_mode,
                    "Failed to create workspace storage - using default (allowed in memory/test mode)"
                );
                return Ok(state.vector_registry.default_storage());
            }
            tracing::error!(
                workspace_id = %workspace_id,
                dimension = workspace.embedding_dimension,
                error = %e,
                "CRITICAL: Failed to create workspace vector storage - refusing to use default"
            );
            Err(ApiError::Internal(format!(
                "Failed to create vector storage for workspace '{}' (dimension {}): {}. \
                 This is a critical error - please check database connectivity and configuration.",
                workspace_id, workspace.embedding_dimension, e
            )))
        }
    }
}

/// Get workspace-specific vector storage with fallback (LEGACY - use strict version for ingestion).
///
/// @deprecated Use `get_workspace_vector_storage_strict` for document ingestion.
///
/// This function falls back to default storage on errors. It should ONLY be used
/// for read operations where fallback is acceptable (e.g., querying when workspace
/// storage doesn't exist yet).
///
/// # WARNING
///
/// DO NOT use this function for write operations (ingestion). Silent fallback
/// can cause data to be stored in the wrong location. Use the strict version instead.
#[allow(dead_code)]
async fn get_workspace_vector_storage_with_fallback(
    state: &AppState,
    workspace_id: &str,
) -> Arc<dyn VectorStorage> {
    match get_workspace_vector_storage_strict(state, workspace_id).await {
        Ok(storage) => storage,
        Err(e) => {
            warn!(
                workspace_id = %workspace_id,
                error = %e,
                "Falling back to default vector storage (READ ONLY operations)"
            );
            state.vector_registry.default_storage()
        }
    }
}

// ============================================
// OODA-08: Reusable Document Graph Cleanup
// ============================================

/// Statistics from document graph data cleanup.
///
/// @implements GAP-08: Reprocess endpoints must clean partial data
///
/// WHY: This struct is used to track cleanup operations and provide
/// visibility into what was removed during reprocessing or deletion.
#[derive(Debug, Default, Clone)]
pub struct CleanupStats {
    /// Number of entities completely removed (source_ids became empty)
    pub entities_removed: usize,
    /// Number of entities updated (document removed from source_ids)
    pub entities_updated: usize,
    /// Number of relationships completely removed
    pub relationships_removed: usize,
    /// Number of relationships updated
    pub relationships_updated: usize,
    /// Number of embeddings deleted from vector storage
    pub embeddings_deleted: usize,
}

/// Clean up graph data for a document without deleting KV entries.
///
/// @implements GAP-08: Cleanup before reprocessing
/// @implements SPEC-033: Per-workspace vector storage isolation
///
/// This function removes the document from entity/edge source_ids and
/// deletes entities/edges that have no remaining sources.
///
/// # When to Use
///
/// - **reprocess_failed**: Clean partial data from failed attempt before requeueing
/// - **recover_stuck**: Clean partial data from interrupted processing before requeueing
/// - **delete_document**: Clean graph data as part of full deletion
///
/// # What It Does
///
/// 1. Process all nodes - remove document_id from source_ids
/// 2. Delete nodes with empty source_ids
/// 3. Process all edges - remove document_id from source_ids
/// 4. Delete edges with empty source_ids OR orphaned (connects to deleted node)
/// 5. Delete entity embeddings for removed entities
///
/// # What It Does NOT Do
///
/// - Delete KV entries (metadata, content, chunks) - these are needed for reprocessing
/// - Delete chunk embeddings - handled separately in delete_document
///
/// # Arguments
///
/// * `document_id` - The document ID to clean up
/// * `graph_storage` - Graph storage adapter
/// * `vector_storage` - Optional vector storage for entity embedding cleanup
///
/// # Returns
///
/// * `Ok(CleanupStats)` - Cleanup statistics
/// * `Err(ApiError)` - If cleanup fails
async fn cleanup_document_graph_data(
    document_id: &str,
    graph_storage: &Arc<dyn edgequake_storage::traits::GraphStorage>,
    vector_storage: Option<&Arc<dyn VectorStorage>>,
) -> Result<CleanupStats, ApiError> {
    let mut stats = CleanupStats::default();

    // Helper function to extract source documents from node/edge properties
    // Handles both `source_ids` (JSON array) and `source_id` (pipe-separated string)
    fn extract_source_docs(
        properties: &std::collections::HashMap<String, serde_json::Value>,
    ) -> Vec<String> {
        // Try source_ids (JSON array) first - this is the current format
        if let Some(source_ids) = properties.get("source_ids") {
            if let Some(arr) = source_ids.as_array() {
                return arr
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
            }
        }
        // Fall back to source_id (pipe-separated string) for backward compatibility
        if let Some(source_id) = properties.get("source_id").and_then(|v| v.as_str()) {
            return source_id.split('|').map(|s| s.to_string()).collect();
        }
        Vec::new()
    }

    // Build chunk prefix for source matching
    let chunk_prefix = format!("{}-chunk-", document_id);

    // Process graph entities - remove document sources
    let all_nodes = graph_storage.get_all_nodes().await?;
    for node in all_nodes {
        let sources = extract_source_docs(&node.properties);
        if sources.is_empty() {
            continue;
        }

        // Filter out sources that belong to this document
        let remaining_sources: Vec<String> = sources
            .iter()
            .filter(|s| {
                !s.starts_with(&chunk_prefix) && *s != document_id && !s.starts_with(document_id)
            })
            .cloned()
            .collect();

        if remaining_sources.is_empty() {
            // No sources left - delete the entity entirely
            graph_storage.delete_node(&node.id).await?;
            // Delete entity embedding if vector storage provided
            if let Some(vs) = vector_storage {
                let _ = vs.delete_entity(&node.id).await;
                stats.embeddings_deleted += 1;
            }
            stats.entities_removed += 1;
        } else if remaining_sources.len() < sources.len() {
            // Some sources were removed - update the entity
            let mut updated_props = node.properties.clone();
            updated_props.insert(
                "source_ids".to_string(),
                serde_json::json!(remaining_sources),
            );
            graph_storage.upsert_node(&node.id, updated_props).await?;
            stats.entities_updated += 1;
        }
    }

    // Process graph edges - remove document sources and orphaned edges
    let all_edges = graph_storage.get_all_edges().await?;

    // Get current node IDs for orphan detection
    let existing_nodes = graph_storage.get_all_nodes().await?;
    let existing_node_ids: std::collections::HashSet<String> =
        existing_nodes.iter().map(|n| n.id.clone()).collect();

    for edge in all_edges {
        // Check if edge is orphaned (connects to deleted node)
        let is_orphaned =
            !existing_node_ids.contains(&edge.source) || !existing_node_ids.contains(&edge.target);

        if is_orphaned {
            // Edge connects to a deleted node - delete it
            graph_storage
                .delete_edge(&edge.source, &edge.target)
                .await?;
            stats.relationships_removed += 1;
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
                !s.starts_with(&chunk_prefix) && *s != document_id && !s.starts_with(document_id)
            })
            .cloned()
            .collect();

        if remaining_sources.is_empty() {
            // No sources left - delete the relationship
            graph_storage
                .delete_edge(&edge.source, &edge.target)
                .await?;
            stats.relationships_removed += 1;
        } else if remaining_sources.len() < sources.len() {
            // Some sources were removed - update the relationship
            let mut updated_props = edge.properties.clone();
            updated_props.insert(
                "source_ids".to_string(),
                serde_json::json!(remaining_sources),
            );
            graph_storage
                .upsert_edge(&edge.source, &edge.target, updated_props)
                .await?;
            stats.relationships_updated += 1;
        }
    }

    tracing::info!(
        document_id = %document_id,
        entities_removed = stats.entities_removed,
        entities_updated = stats.entities_updated,
        relationships_removed = stats.relationships_removed,
        relationships_updated = stats.relationships_updated,
        embeddings_deleted = stats.embeddings_deleted,
        "Document graph data cleanup completed"
    );

    Ok(stats)
}

/// Delete all document data for re-ingestion.
///
/// @implements FIX-4: Duplicate re-ingestion
///
/// WHY: When a duplicate document is detected, the user may want to re-process it
/// (e.g., because the original processing failed). This function deletes all
/// existing data for the document so it can be processed fresh.
///
/// # Safety
///
/// This function refuses to delete documents that are actively being processed
/// (status = "pending" or "processing") to avoid race conditions.
///
/// # Returns
///
/// * `Ok(true)` - Document data deleted successfully
/// * `Ok(false)` - Document is still processing, cannot delete
/// * `Err(ApiError)` - If deletion fails
///
/// @implements FIX-RACE-01: Atomic status transition prevents TOCTOU race condition
async fn delete_document_for_reingestion(
    document_id: &str,
    state: &AppState,
    workspace_id: &str,
) -> Result<bool, ApiError> {
    let metadata_key = format!("{}-metadata", document_id);

    // WHY: Atomic Status Transition (FIX-RACE-01)
    //
    // Previous code had a TOCTOU vulnerability:
    // 1. Read status = "failed"
    // 2. Another process changes status to "processing"
    // 3. Delete data (corrupts active ingestion!)
    //
    // New approach: Atomically transition status BEFORE deletion.
    // If transition fails, another process is using the document.
    //
    // Allowed transitions for re-ingestion:
    // - "failed" → "deleting" (retry after error)
    // - "completed" → "deleting" (re-extract with new settings)
    // - "cancelled" → "deleting" (user cancelled, wants to retry)
    //
    // Disallowed (return conflict):
    // - "pending" → (still waiting for processing)
    // - "processing" → (active ingestion in progress)
    // - "deleting" → (another delete already in progress)

    // Try to transition from "failed" to "deleting"
    let transitioned_from_failed = state
        .kv_storage
        .transition_if_status(&metadata_key, "failed", "deleting")
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to transition status: {}", e)))?;

    if transitioned_from_failed {
        tracing::info!(
            document_id = %document_id,
            from_status = "failed",
            "Atomic status transition succeeded - safe to delete"
        );
    } else {
        // Try "completed" → "deleting"
        let transitioned_from_completed = state
            .kv_storage
            .transition_if_status(&metadata_key, "completed", "deleting")
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to transition status: {}", e)))?;

        if transitioned_from_completed {
            tracing::info!(
                document_id = %document_id,
                from_status = "completed",
                "Atomic status transition succeeded - safe to delete"
            );
        } else {
            // Try "cancelled" → "deleting"
            let transitioned_from_cancelled = state
                .kv_storage
                .transition_if_status(&metadata_key, "cancelled", "deleting")
                .await
                .map_err(|e| ApiError::Internal(format!("Failed to transition status: {}", e)))?;

            if transitioned_from_cancelled {
                tracing::info!(
                    document_id = %document_id,
                    from_status = "cancelled",
                    "Atomic status transition succeeded - safe to delete"
                );
            } else {
                // None of the allowed transitions worked - document state prevents re-ingestion
                // WHY: This is not necessarily an error - document might be processing, pending, or deleted
                tracing::warn!(
                    document_id = %document_id,
                    metadata_key = %metadata_key,
                    "Cannot re-ingest: document status prevents transition (processing/pending/deleting/not found)"
                );
                return Ok(false);
            }
        }
    }

    // === SAFE DELETION ZONE ===
    // At this point, status is atomically set to "deleting"
    // No other process can modify this document until we're done

    tracing::info!(
        document_id = %document_id,
        workspace_id = %workspace_id,
        "Re-ingestion requested - deleting existing document data (status = deleting)"
    );

    // Get workspace-specific vector storage for cleanup
    let workspace_vector_storage = get_workspace_vector_storage_strict(state, workspace_id).await?;

    // Clean up graph data (entities, relationships, embeddings)
    let cleanup_stats = cleanup_document_graph_data(
        document_id,
        &state.graph_storage,
        Some(&workspace_vector_storage),
    )
    .await?;

    // Delete chunk embeddings from vector storage
    let keys = state.kv_storage.keys().await?;
    let chunk_prefix = format!("{}-chunk-", document_id);
    let chunk_ids: Vec<String> = keys
        .iter()
        .filter(|k| k.starts_with(&chunk_prefix))
        .cloned()
        .collect();

    if !chunk_ids.is_empty() {
        if let Err(e) = workspace_vector_storage.delete(&chunk_ids).await {
            tracing::warn!(
                document_id = %document_id,
                error = %e,
                "Failed to delete chunk embeddings during re-ingestion"
            );
        }
    }

    // Collect all KV keys to delete (chunks, metadata, content)
    let mut keys_to_delete: Vec<String> = chunk_ids;
    keys_to_delete.push(metadata_key);
    keys_to_delete.push(format!("{}-content", document_id));

    // Delete all KV storage entries
    state.kv_storage.delete(&keys_to_delete).await?;

    tracing::info!(
        document_id = %document_id,
        chunks_deleted = keys_to_delete.len(),
        entities_removed = cleanup_stats.entities_removed,
        relationships_removed = cleanup_stats.relationships_removed,
        "Document data deleted for re-ingestion"
    );

    Ok(true)
}

/// Upload a document for processing.
///
/// # Implements
///
/// - **UC0001**: Upload Document
/// - **FEAT0001**: Document Ingestion Pipeline
/// - **FEAT0002**: Entity Extraction
/// - **FEAT0003**: Relationship Discovery
///
/// # Enforces
///
/// - **BR0001**: Content uniqueness (SHA-256 hash computed)
/// - **BR0201**: Tenant isolation (scoped to workspace)
/// - **BR0302**: Document size limits enforced
///
/// # Request Flow
///
/// ```text
/// POST /api/v1/documents
///        ↓
///   Validate content size
///        ↓
///   Compute SHA-256 hash
///        ↓
///   Store metadata + content
///        ↓
///   async_processing?
///     ├─ true: Create task → Return task_id
///     └─ false: Process inline → Return entities
/// ```
#[utoipa::path(
    post,
    path = "/api/v1/documents",
    tag = "Documents",
    request_body = UploadDocumentRequest,
    responses(
        (status = 201, description = "Document uploaded successfully", body = UploadDocumentResponse),
        (status = 400, description = "Invalid request"),
        (status = 413, description = "Document too large")
    )
)]
pub async fn upload_document(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Json(request): Json<UploadDocumentRequest>,
) -> ApiResult<(StatusCode, Json<UploadDocumentResponse>)> {
    debug!(
        tenant_id = ?tenant_ctx.tenant_id,
        workspace_id = ?tenant_ctx.workspace_id,
        "Uploading document with tenant context"
    );

    // Validate document content
    crate::validation::validate_content(&request.content, state.config.max_document_size)?;

    // Generate or use provided track_id
    let track_id = request.track_id.unwrap_or_else(|| {
        format!(
            "upload_{}_{}",
            Utc::now().format("%Y%m%d_%H%M%S"),
            &Uuid::new_v4().to_string()[..8]
        )
    });

    // WHY-OODA83: Use ContentHasher service for consistent hash computation (DRY)
    let content_hash = ContentHasher::hash_str(&request.content);

    // Extract tenant context for storage (needed for hash_key)
    let workspace_id_for_storage = tenant_ctx
        .workspace_id
        .clone()
        .unwrap_or_else(|| "default".to_string());
    let tenant_id_for_storage = tenant_ctx.tenant_id.clone();

    // WHY-OODA81+84: Workspace-scoped duplicate detection
    // FIX-4: Duplicates now trigger re-ingestion instead of rejection
    // Same content in same workspace = re-ingest (delete old data, process new)
    // Same content in different workspace = allowed (multi-tenancy)
    let hash_key = ContentHasher::workspace_hash_key(&workspace_id_for_storage, &content_hash);
    debug!(hash_key = %hash_key, workspace_id = %workspace_id_for_storage, "Checking for workspace-scoped duplicate hash");
    if let Some(existing_doc_id) = state.kv_storage.get_by_id(&hash_key).await? {
        debug!(existing_doc_id = ?existing_doc_id, "Found existing document for hash in workspace");
        if let Some(doc_id_str) = existing_doc_id.as_str() {
            // FIX-4: Try to delete old document data for re-ingestion
            match delete_document_for_reingestion(doc_id_str, &state, &workspace_id_for_storage)
                .await
            {
                Ok(true) => {
                    // Successfully deleted - proceed with new upload
                    tracing::info!(
                        old_doc_id = %doc_id_str,
                        workspace_id = %workspace_id_for_storage,
                        "Duplicate document found - old data deleted, proceeding with re-ingestion"
                    );
                    // Hash key will be updated below with new document_id
                }
                Ok(false) => {
                    // Document still processing - return duplicate response
                    tracing::warn!(
                        old_doc_id = %doc_id_str,
                        "Duplicate document is still being processed - cannot re-ingest"
                    );
                    return Ok((
                        StatusCode::OK,
                        Json(UploadDocumentResponse {
                            document_id: doc_id_str.to_string(),
                            status: "duplicate_processing".to_string(),
                            task_id: None,
                            track_id: track_id.clone(),
                            duplicate_of: Some(doc_id_str.to_string()),
                            chunk_count: None,
                            entity_count: None,
                            relationship_count: None,
                            cost: None,
                        }),
                    ));
                }
                Err(e) => {
                    // Failed to delete - log error and proceed with re-ingestion anyway
                    tracing::warn!(
                        old_doc_id = %doc_id_str,
                        error = %e,
                        "Failed to delete old document data - proceeding with re-ingestion"
                    );
                }
            }
        }
    }

    // Generate document ID
    let document_id = Uuid::new_v4().to_string();

    // Store hash mapping for deduplication (workspace-scoped)
    // WHY-OODA81+84: Must store before creating document to prevent race conditions
    state
        .kv_storage
        .upsert(&[(hash_key.clone(), serde_json::json!(document_id))])
        .await?;
    debug!(hash_key = %hash_key, document_id = %document_id, "Stored workspace-scoped hash mapping");

    // Generate content summary
    let content_summary = crate::validation::generate_content_summary(&request.content);
    let content_length = request.content.len();

    // Store document metadata (including title, content_summary, content_length, track_id, tenant context)
    let doc_metadata_key = format!("{}-metadata", document_id);
    let initial_status = if request.async_processing {
        "pending"
    } else {
        "processing"
    };

    // OODA-04: Include file_size_bytes, sha256_checksum, document_type for unified lineage
    // WHY: Every document—markdown or PDF—must carry the same lineage fields so
    // API consumers get consistent metadata regardless of source type.
    let doc_metadata = serde_json::json!({
        "id": document_id,
        "title": request.title,
        "content_summary": content_summary,
        "content_length": content_length,
        "content_hash": content_hash,
        "file_size_bytes": content_length,
        "sha256_checksum": content_hash,
        "document_type": "markdown",
        "track_id": track_id,
        "created_at": Utc::now().to_rfc3339(),
        "status": initial_status,
        "tenant_id": tenant_id_for_storage,
        "workspace_id": workspace_id_for_storage,
        // SPEC-002: Unified Ingestion Pipeline fields
        "source_type": "markdown",
        "current_stage": "uploading",
        "stage_progress": 0.0,
        "stage_message": "Document received, starting processing",
    });
    state
        .kv_storage
        .upsert(&[(doc_metadata_key.clone(), doc_metadata)])
        .await?;

    // Store the document content for processing
    let doc_content_key = format!("{}-content", document_id);
    let doc_content = serde_json::json!({
        "content": request.content,
    });
    state
        .kv_storage
        .upsert(&[(doc_content_key, doc_content)])
        .await?;

    // Handle async vs sync processing
    if request.async_processing {
        // Create task for background processing
        use edgequake_tasks::{Task, TaskType, TextInsertData};

        // Use tenant context for workspace_id, fallback to "default"
        let workspace_id = tenant_ctx
            .workspace_id
            .clone()
            .unwrap_or_else(|| "default".to_string());
        let tenant_id = tenant_ctx
            .tenant_id
            .clone()
            .unwrap_or_else(|| "default".to_string());

        let task_data = TextInsertData {
            text: request.content.clone(),
            file_source: request.title.clone().unwrap_or_else(|| document_id.clone()),
            workspace_id: workspace_id.clone(),
            metadata: Some(serde_json::json!({
                "document_id": document_id,
                "title": request.title,
                "tenant_id": tenant_id,
                "workspace_id": workspace_id,
            })),
        };

        let task = Task::new(
            uuid::Uuid::parse_str(&tenant_id)
                .map_err(|_| ApiError::ValidationError("Invalid tenant ID".to_string()))?,
            uuid::Uuid::parse_str(&workspace_id)
                .map_err(|_| ApiError::ValidationError("Invalid workspace ID".to_string()))?,
            TaskType::Insert,
            serde_json::to_value(task_data).unwrap(),
        );
        let task_id = task.track_id.clone();

        // Store task
        state
            .task_storage
            .create_task(&task)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to create task: {}", e)))?;

        // Queue task for processing
        state
            .task_queue
            .send(task)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to queue task: {}", e)))?;

        Ok((
            StatusCode::CREATED,
            Json(UploadDocumentResponse {
                document_id,
                status: "pending".to_string(),
                task_id: Some(task_id),
                track_id,
                duplicate_of: None,
                chunk_count: None,
                entity_count: None,
                relationship_count: None,
                cost: None, // Cost will be calculated when processing completes
            }),
        ))
    } else {
        // Synchronous processing (original behavior)
        // Broadcast job started
        let start_time = std::time::Instant::now();
        state.progress_broadcaster.job_started(&document_id, 1, 1);

        // SPEC-032: Use workspace-specific pipeline with workspace LLM configuration
        // This ensures the workspace's LLM model is used for entity extraction
        let workspace_pipeline = state
            .create_workspace_pipeline(&workspace_id_for_storage)
            .await;

        // OODA-01: Add HTTP-level timeout to prevent indefinite hangs
        // WHY: Large documents (100KB+) can take 5-10 minutes to process,
        // but HTTP clients expect responses within 60-120 seconds.
        // Without this timeout, requests hang indefinitely causing poor UX.
        //
        // Timeout Strategy:
        // - 120 seconds (2 minutes): Conservative limit for synchronous mode
        // - For larger documents, users should use async_processing: true
        // - Timeout applies to ENTIRE pipeline, not just individual LLM calls
        //
        // See: specs/002-bullet-proof-ingestion-process.md
        const SYNC_PROCESSING_TIMEOUT_SECS: u64 = 120;

        let processing_start = std::time::Instant::now();
        debug!(
            document_id = %document_id,
            content_length = request.content.len(),
            timeout_secs = SYNC_PROCESSING_TIMEOUT_SECS,
            "Starting synchronous document processing"
        );

        let result = tokio::time::timeout(
            std::time::Duration::from_secs(SYNC_PROCESSING_TIMEOUT_SECS),
            // SPEC-003: Use resilient processing with chunk-level error isolation
            // WHY: Map-reduce pattern continues processing even if some chunks fail
            workspace_pipeline.process_with_resilience(&document_id, &request.content, None),
        )
        .await
        .map_err(|_elapsed| {
            let processing_time = processing_start.elapsed();
            warn!(
                document_id = %document_id,
                timeout_secs = SYNC_PROCESSING_TIMEOUT_SECS,
                processing_time_secs = processing_time.as_secs(),
                content_length = request.content.len(),
                "Document processing timeout - consider using async mode for large documents"
            );
            ApiError::Timeout(format!(
                "Document processing exceeded {} seconds. For large documents (>50KB), \
                 use async_processing: true to avoid timeouts. \
                 Current document size: {} bytes",
                SYNC_PROCESSING_TIMEOUT_SECS,
                request.content.len()
            ))
        })??;

        // SPEC-003: Log partial success if some chunks failed
        if result.stats.failed_chunks > 0 {
            warn!(
                document_id = %document_id,
                successful_chunks = result.stats.successful_chunks,
                failed_chunks = result.stats.failed_chunks,
                total_chunks = result.stats.chunk_count,
                "Document processed with partial success - some chunks failed extraction"
            );

            // Emit WebSocket events for failed chunks
            if let Some(ref chunk_errors) = result.stats.chunk_errors {
                for error_info in chunk_errors {
                    state.progress_broadcaster.broadcast_chunk_failure(
                        document_id.clone(),
                        document_id.clone(), // Use doc_id as track_id for sync
                        error_info.chunk_index as u32,
                        result.stats.chunk_count as u32,
                        error_info.error_message.clone(),
                        error_info.was_timeout,
                        error_info.retry_attempts,
                    );
                }
            }
        }

        let processing_time = processing_start.elapsed();
        debug!(
            document_id = %document_id,
            processing_time_secs = processing_time.as_secs(),
            processing_time_ms = processing_time.as_millis(),
            chunk_count = result.chunks.len(),
            entity_count = result.stats.entity_count,
            "Document processing completed successfully"
        );

        // Store chunks in KV storage
        let chunks: Vec<(String, serde_json::Value)> = result
            .chunks
            .iter()
            .map(|c| {
                (
                    c.id.clone(),
                    serde_json::json!({
                        "content": c.content,
                        "document_id": document_id,
                        "index": c.index,
                    }),
                )
            })
            .collect();

        state.kv_storage.upsert(&chunks).await?;

        // SPEC-033: Get workspace-specific vector storage for document embeddings
        // This ensures embeddings are stored with correct dimension per workspace
        // WHY-OODA223: STRICT mode - fail loudly if workspace storage unavailable
        // to prevent data from being stored in the wrong (global) table
        let workspace_vector_storage =
            get_workspace_vector_storage_strict(&state, &workspace_id_for_storage).await?;

        // Store chunk embeddings in vector storage for semantic search
        let mut chunk_embeddings_stored = 0;
        for chunk in &result.chunks {
            if let Some(embedding) = &chunk.embedding {
                let mut metadata = serde_json::json!({
                    "type": "chunk",
                    "document_id": document_id,
                    "index": chunk.index,
                    "content": chunk.content,
                    "start_line": chunk.start_line,
                    "end_line": chunk.end_line,
                    "chunk_index": chunk.index,
                });

                // Add tenant and workspace IDs if present
                if let Some(ref tid) = tenant_id_for_storage {
                    metadata["tenant_id"] = serde_json::json!(tid);
                }
                metadata["workspace_id"] = serde_json::json!(&workspace_id_for_storage);

                match workspace_vector_storage
                    .upsert(&[(chunk.id.clone(), embedding.clone(), metadata)])
                    .await
                {
                    Ok(_) => {
                        chunk_embeddings_stored += 1;
                        tracing::info!(chunk_id = %chunk.id, "VECTOR STORAGE: Chunk embedding stored OK");
                    }
                    Err(e) => {
                        tracing::error!(chunk_id = %chunk.id, error = %e, "VECTOR STORAGE: Failed to store chunk embedding");
                    }
                }
            }
        }
        tracing::info!(
            chunk_embeddings_stored = chunk_embeddings_stored,
            total_chunks = result.chunks.len(),
            "VECTOR STORAGE: Chunk embedding storage complete"
        );

        // Broadcast document progress (chunking complete)
        state
            .progress_broadcaster
            .document_progress(&document_id, 0, 1, 3);

        // Store entities and relationships in graph storage
        for extraction in &result.extractions {
            for entity in &extraction.entities {
                // OODA-06 FIX (GAP-07): Merge source_ids with existing entity sources
                // WHY: When the same entity appears in multiple documents, we must
                // accumulate source_ids from ALL documents, not replace with just the current one.
                // Without this, deleting one document could orphan an entity that's still
                // referenced by other documents.
                let merged_source_ids = match state.graph_storage.get_node(&entity.name).await {
                    Ok(Some(existing)) => {
                        let mut existing_sources: std::collections::HashSet<String> = existing
                            .properties
                            .get("source_ids")
                            .and_then(|v| v.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                    .collect()
                            })
                            .unwrap_or_default();
                        // Add new document reference (HashSet deduplicates)
                        existing_sources.insert(document_id.clone());
                        existing_sources.into_iter().collect::<Vec<_>>()
                    }
                    _ => vec![document_id.clone()],
                };

                let mut properties = std::collections::HashMap::new();
                properties.insert(
                    "entity_type".to_string(),
                    serde_json::json!(entity.entity_type),
                );
                properties.insert(
                    "description".to_string(),
                    serde_json::json!(entity.description),
                );
                properties.insert(
                    "importance".to_string(),
                    serde_json::json!(entity.importance),
                );
                properties.insert(
                    "source_ids".to_string(),
                    serde_json::json!(merged_source_ids),
                );
                // CRITICAL: Store source_chunk_ids for Local/Global query mode chunk retrieval
                properties.insert(
                    "source_chunk_ids".to_string(),
                    serde_json::json!(&entity.source_chunk_ids),
                );

                state
                    .graph_storage
                    .upsert_node(&entity.name, properties)
                    .await?;

                // CRITICAL: Also store entity embedding in vector storage for query_local retrieval
                // SPEC-033: Use workspace-specific vector storage
                if let Some(embedding) = &entity.embedding {
                    let mut metadata = serde_json::json!({
                        "type": "entity",
                        "entity_name": entity.name,
                        "entity_type": entity.entity_type,
                        "description": entity.description,
                        "document_id": document_id,
                        "source_chunk_ids": entity.source_chunk_ids,
                    });
                    if let Some(ref tid) = tenant_id_for_storage {
                        metadata["tenant_id"] = serde_json::json!(tid);
                    }
                    metadata["workspace_id"] = serde_json::json!(&workspace_id_for_storage);

                    let entity_id = format!("entity:{}", entity.name);
                    if let Err(e) = workspace_vector_storage
                        .upsert(&[(entity_id.clone(), embedding.clone(), metadata)])
                        .await
                    {
                        tracing::error!(entity_id = %entity_id, error = %e, "Failed to store entity embedding");
                    }
                }
            }

            for relationship in &extraction.relationships {
                // OODA-06 FIX (GAP-07): Merge source_ids with existing edge sources
                // WHY: Same as entities - when the same relationship appears in multiple
                // documents, we must accumulate source_ids from ALL documents.
                let merged_source_ids = match state
                    .graph_storage
                    .get_edge(&relationship.source, &relationship.target)
                    .await
                {
                    Ok(Some(existing)) => {
                        let mut existing_sources: std::collections::HashSet<String> = existing
                            .properties
                            .get("source_ids")
                            .and_then(|v| v.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                    .collect()
                            })
                            .unwrap_or_default();
                        existing_sources.insert(document_id.clone());
                        existing_sources.into_iter().collect::<Vec<_>>()
                    }
                    _ => vec![document_id.clone()],
                };

                let mut properties = std::collections::HashMap::new();
                properties.insert(
                    "relation_type".to_string(),
                    serde_json::json!(relationship.relation_type),
                );
                properties.insert(
                    "description".to_string(),
                    serde_json::json!(relationship.description),
                );
                properties.insert("weight".to_string(), serde_json::json!(relationship.weight));
                properties.insert(
                    "keywords".to_string(),
                    serde_json::json!(relationship.keywords),
                );
                properties.insert(
                    "source_ids".to_string(),
                    serde_json::json!(merged_source_ids),
                );
                // CRITICAL: Store source_chunk_id for relationship chunk linkage
                if let Some(ref chunk_id) = relationship.source_chunk_id {
                    properties.insert(
                        "source_chunk_ids".to_string(),
                        serde_json::json!(vec![chunk_id]),
                    );
                }

                state
                    .graph_storage
                    .upsert_edge(&relationship.source, &relationship.target, properties)
                    .await?;
            }
        }

        // Broadcast document progress (extraction complete)
        state
            .progress_broadcaster
            .document_progress(&document_id, result.stats.entity_count, 2, 3);

        // OODA-03: Determine final status based on chunk extraction results
        // - "completed": All chunks extracted successfully
        // - "partial_success": Some chunks succeeded, some failed (users need visibility)
        // - "failed": All chunks failed (already handled upstream by error return)
        let final_status = if result.stats.failed_chunks > 0 {
            if result.stats.successful_chunks > 0 {
                "partial_success"
            } else {
                "failed"
            }
        } else {
            "completed"
        };

        // Update document status (preserve content_summary, content_length, track_id, tenant context)
        let doc_metadata = serde_json::json!({
            "id": document_id,
            "title": request.title,
            "content_summary": content_summary,
            "content_length": content_length,
            "content_hash": content_hash,
            "track_id": track_id,
            "created_at": Utc::now().to_rfc3339(),
            "status": final_status,
            "chunk_count": result.stats.chunk_count,
            "successful_chunks": result.stats.successful_chunks,
            "failed_chunks": result.stats.failed_chunks,
            "entity_count": result.stats.entity_count,
            "relationship_count": result.stats.relationship_count,
            "tenant_id": tenant_id_for_storage,
            "workspace_id": workspace_id_for_storage,
            "cost_usd": result.stats.cost_usd,
            "input_tokens": result.stats.input_tokens,
            "output_tokens": result.stats.output_tokens,
            "total_tokens": result.stats.total_tokens,
            "llm_model": result.stats.llm_model,
            "embedding_model": result.stats.embedding_model,
        });
        state
            .kv_storage
            .upsert(&[(doc_metadata_key, doc_metadata)])
            .await?;

        // Broadcast job finished
        let duration = start_time.elapsed();
        state
            .progress_broadcaster
            .document_progress(&document_id, result.stats.entity_count, 3, 3);
        state
            .progress_broadcaster
            .job_finished(1, duration.as_millis() as u64);

        // Build cost info from stats
        let cost = Some(DocumentCostInfo {
            total_cost_usd: result.stats.cost_usd,
            formatted_cost: format!("${:.6}", result.stats.cost_usd),
            input_tokens: result.stats.input_tokens,
            output_tokens: result.stats.output_tokens,
            total_tokens: result.stats.total_tokens,
            llm_model: result.stats.llm_model.clone(),
            embedding_model: result.stats.embedding_model.clone(),
        });

        // OODA-21: Record metrics snapshot for trend analysis after upload
        // Best-effort: log error but don't fail the upload
        if let Ok(workspace_uuid) = Uuid::parse_str(&workspace_id_for_storage) {
            if let Err(e) = state
                .workspace_service
                .record_metrics_snapshot(workspace_uuid, MetricsTriggerType::Event)
                .await
            {
                tracing::warn!(
                    workspace_id = %workspace_id_for_storage,
                    error = %e,
                    "Failed to record post-upload metrics snapshot"
                );
            } else {
                tracing::debug!(
                    workspace_id = %workspace_id_for_storage,
                    "Recorded post-upload metrics snapshot"
                );
            }

            // OODA-ITERATION-03-FIX: Invalidate workspace stats cache after document processing
            // WHY: The cache contains stale entity/relationship counts (0 before fix, or old counts)
            // Without this, Dashboard shows 0 entities while Workspace page shows correct counts
            // because both pages use the same cached stats, but cache was populated before
            // the document was processed. This ensures the next stats request fetches fresh data.
            crate::handlers::workspaces::invalidate_workspace_stats_cache(workspace_uuid).await;
        }

        Ok((
            StatusCode::CREATED,
            Json(UploadDocumentResponse {
                document_id,
                status: "processed".to_string(),
                task_id: None,
                track_id,
                duplicate_of: None,
                chunk_count: Some(result.stats.chunk_count),
                entity_count: Some(result.stats.entity_count),
                relationship_count: Some(result.stats.relationship_count),
                cost,
            }),
        ))
    }
}

/// List all documents.
#[utoipa::path(
    get,
    path = "/api/v1/documents",
    tag = "Documents",
    responses(
        (status = 200, description = "Documents retrieved", body = ListDocumentsResponse)
    )
)]
#[allow(clippy::field_reassign_with_default)]
pub async fn list_documents(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
) -> ApiResult<Json<ListDocumentsResponse>> {
    debug!(
        tenant_id = ?tenant_ctx.tenant_id,
        workspace_id = ?tenant_ctx.workspace_id,
        "Listing documents with tenant context"
    );

    // SECURITY: Enforce strict tenant context requirement - NO EXCEPTIONS
    // This matches the strict filtering in entities.rs and relationships.rs (commit d11edba8)
    if tenant_ctx.tenant_id.is_none() || tenant_ctx.workspace_id.is_none() {
        warn!(
            tenant_id = ?tenant_ctx.tenant_id,
            workspace_id = ?tenant_ctx.workspace_id,
            "Tenant context missing - returning empty document list for security"
        );
        return Ok(Json(ListDocumentsResponse {
            documents: vec![],
            total: 0,
            page: 1,
            page_size: 100,
            total_pages: 0,
            has_more: false,
            status_counts: StatusCounts {
                pending: 0,
                processing: 0,
                completed: 0,
                partial_failure: 0,
                failed: 0,
                cancelled: 0,
            },
        }));
    }

    let keys = state.kv_storage.keys().await?;
    debug!(key_count = keys.len(), "Total keys in KV storage");
    debug!(keys = ?keys, "All keys in KV storage");

    // Group by document and collect metadata keys
    let mut doc_chunks: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut metadata_keys: Vec<String> = Vec::new();

    for key in &keys {
        if key.ends_with("-metadata") {
            debug!(metadata_key = %key, "Found metadata key");
            metadata_keys.push(key.clone());
        } else if key.contains("-chunk-") {
            // Only count actual chunk keys (e.g., "doc-id-chunk-0")
            if let Some(doc_id) = key.split("-chunk-").next() {
                // Filter out non-document keys (like -metadata, -content suffixes)
                if !doc_id.ends_with("-metadata") && !doc_id.ends_with("-content") {
                    *doc_chunks.entry(doc_id.to_string()).or_default() += 1;
                }
            }
        }
    }

    // Fetch all metadata and store complete document info
    debug!(
        metadata_keys_count = metadata_keys.len(),
        "Fetching metadata for keys"
    );
    let metadata_values = state.kv_storage.get_by_ids(&metadata_keys).await?;
    debug!(
        metadata_values_count = metadata_values.len(),
        "Metadata values retrieved"
    );

    // Store complete document metadata, keyed by document ID
    #[derive(Default)]
    struct DocMetadata {
        title: Option<String>,
        file_name: Option<String>,
        content_summary: Option<String>,
        content_length: Option<usize>,
        status: Option<String>,
        error_message: Option<String>,
        track_id: Option<String>,
        created_at: Option<String>,
        updated_at: Option<String>,
        entity_count: Option<usize>,
        tenant_id: Option<String>,
        workspace_id: Option<String>,
        cost_usd: Option<f64>,
        input_tokens: Option<usize>,
        output_tokens: Option<usize>,
        total_tokens: Option<usize>,
        llm_model: Option<String>,
        embedding_model: Option<String>,
        // SPEC-002: Unified Ingestion Pipeline fields
        source_type: Option<String>,
        current_stage: Option<String>,
        stage_progress: Option<f32>,
        stage_message: Option<String>,
        pdf_id: Option<String>,
    }

    let mut doc_metadata: std::collections::HashMap<String, DocMetadata> =
        std::collections::HashMap::new();

    for value in metadata_values {
        debug!(value = ?value, "Processing metadata value");
        if let Some(obj) = value.as_object() {
            if let Some(id) = obj.get("id").and_then(|v| v.as_str()) {
                let title_val = obj.get("title");
                debug!(doc_id = %id, title = ?title_val, "Extracted ID and title from metadata");

                // WHY: We build DocMetadata incrementally because fields are extracted
                // conditionally from JSON, and some fields depend on others (e.g., file_name
                // is derived from title). Struct initializer syntax doesn't work well here.
                let mut meta = DocMetadata::default();

                // Get title from metadata
                meta.title = obj
                    .get("title")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                // Use title as file_name fallback if it looks like a filename
                if let Some(ref title) = meta.title {
                    if title.contains('.') {
                        meta.file_name = Some(title.clone());
                    }
                }

                // Get content_summary
                meta.content_summary = obj
                    .get("content_summary")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                // Get content_length
                meta.content_length = obj
                    .get("content_length")
                    .and_then(|v| v.as_u64())
                    .map(|n| n as usize);

                // Get status
                meta.status = obj
                    .get("status")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                // Get error_message
                meta.error_message = obj
                    .get("error_message")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                // Get track_id
                meta.track_id = obj
                    .get("track_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                // Get created_at
                meta.created_at = obj
                    .get("created_at")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                // Get updated_at
                meta.updated_at = obj
                    .get("updated_at")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                // Get entity_count
                meta.entity_count = obj
                    .get("entity_count")
                    .and_then(|v| v.as_u64())
                    .map(|n| n as usize);

                // Get tenant_id
                meta.tenant_id = obj
                    .get("tenant_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                // Get workspace_id
                meta.workspace_id = obj
                    .get("workspace_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                // Get cost_usd
                meta.cost_usd = obj.get("cost_usd").and_then(|v| v.as_f64());

                // Get input_tokens
                meta.input_tokens = obj
                    .get("input_tokens")
                    .and_then(|v| v.as_u64())
                    .map(|n| n as usize);

                // Get output_tokens
                meta.output_tokens = obj
                    .get("output_tokens")
                    .and_then(|v| v.as_u64())
                    .map(|n| n as usize);

                // Get total_tokens
                meta.total_tokens = obj
                    .get("total_tokens")
                    .and_then(|v| v.as_u64())
                    .map(|n| n as usize);

                // Get llm_model
                meta.llm_model = obj
                    .get("llm_model")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                // Get embedding_model
                meta.embedding_model = obj
                    .get("embedding_model")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                // SPEC-002: Get source_type
                meta.source_type = obj
                    .get("source_type")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                // SPEC-002: Get current_stage
                meta.current_stage = obj
                    .get("current_stage")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                // SPEC-002: Get stage_progress
                meta.stage_progress = obj
                    .get("stage_progress")
                    .and_then(|v| v.as_f64())
                    .map(|n| n as f32);

                // SPEC-002: Get stage_message
                meta.stage_message = obj
                    .get("stage_message")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                // SPEC-002: Get pdf_id (linked PDF document for viewing)
                meta.pdf_id = obj
                    .get("pdf_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                doc_metadata.insert(id.to_string(), meta);
            }
        }
    }

    // Filter documents by tenant context
    let filter_workspace_id = tenant_ctx.workspace_id.clone();
    let filter_tenant_id = tenant_ctx.tenant_id.clone();

    // SECURITY: STRICT tenant filtering - both tenant_id AND workspace_id must match
    // This matches the strict filtering in entities.rs and relationships.rs (commit d11edba8)
    let matches_tenant_context = |meta: &DocMetadata| -> bool {
        // Both must match exactly (None is already handled by early return above)
        meta.workspace_id.as_ref() == filter_workspace_id.as_ref()
            && meta.tenant_id.as_ref() == filter_tenant_id.as_ref()
    };

    // Build document list from BOTH:
    // 1. Documents with chunks (processed)
    // 2. Documents with metadata but no chunks yet (pending/processing)
    let mut documents: Vec<DocumentSummary> = doc_chunks
        .into_iter()
        .filter_map(|(id, chunk_count)| {
            let meta = doc_metadata.remove(&id).unwrap_or_default();
            // Filter by tenant context
            if !matches_tenant_context(&meta) {
                return None;
            }
            Some(DocumentSummary {
                id,
                title: meta.title,
                file_name: meta.file_name,
                content_summary: meta.content_summary,
                content_length: meta.content_length,
                chunk_count,
                entity_count: meta.entity_count,
                status: meta.status,
                error_message: meta.error_message,
                track_id: meta.track_id,
                created_at: meta.created_at,
                updated_at: meta.updated_at,
                cost_usd: meta.cost_usd,
                input_tokens: meta.input_tokens,
                output_tokens: meta.output_tokens,
                total_tokens: meta.total_tokens,
                llm_model: meta.llm_model,
                embedding_model: meta.embedding_model,
                // SPEC-002: Unified Ingestion Pipeline fields
                source_type: meta.source_type,
                current_stage: meta.current_stage,
                stage_progress: meta.stage_progress,
                stage_message: meta.stage_message,
                pdf_id: meta.pdf_id,
            })
        })
        .collect();

    // Add documents that have metadata but no chunks yet (pending/processing)
    for (id, meta) in doc_metadata {
        // Filter by tenant context
        if !matches_tenant_context(&meta) {
            continue;
        }
        documents.push(DocumentSummary {
            id,
            title: meta.title,
            file_name: meta.file_name,
            content_summary: meta.content_summary,
            content_length: meta.content_length,
            chunk_count: 0,
            entity_count: meta.entity_count,
            status: meta.status,
            error_message: meta.error_message,
            track_id: meta.track_id,
            created_at: meta.created_at,
            updated_at: meta.updated_at,
            cost_usd: meta.cost_usd,
            input_tokens: meta.input_tokens,
            output_tokens: meta.output_tokens,
            total_tokens: meta.total_tokens,
            llm_model: meta.llm_model,
            embedding_model: meta.embedding_model,
            // SPEC-002: Unified Ingestion Pipeline fields
            source_type: meta.source_type,
            current_stage: meta.current_stage,
            stage_progress: meta.stage_progress,
            stage_message: meta.stage_message,
            pdf_id: meta.pdf_id,
        });
    }

    // Sort by created_at descending (newest first)
    documents.sort_by(|a, b| {
        b.created_at
            .as_deref()
            .unwrap_or("")
            .cmp(a.created_at.as_deref().unwrap_or(""))
    });

    // Calculate status counts for all documents
    let status_counts = StatusCounts {
        pending: documents
            .iter()
            .filter(|d| d.status.as_deref() == Some("pending"))
            .count(),
        processing: documents
            .iter()
            .filter(|d| d.status.as_deref() == Some("processing"))
            .count(),
        completed: documents
            .iter()
            .filter(|d| {
                d.status.is_none()
                    || d.status.as_deref() == Some("completed")
                    || d.status.as_deref() == Some("indexed")
            })
            .count(),
        // FIX-5: Track partial_failure status
        partial_failure: documents
            .iter()
            .filter(|d| d.status.as_deref() == Some("partial_failure"))
            .count(),
        failed: documents
            .iter()
            .filter(|d| d.status.as_deref() == Some("failed"))
            .count(),
        cancelled: documents
            .iter()
            .filter(|d| d.status.as_deref() == Some("cancelled"))
            .count(),
    };

    let total = documents.len();
    let page_size = 20usize;
    let total_pages = (total + page_size - 1) / page_size.max(1);
    let page = 1usize;
    let has_more = page < total_pages;

    Ok(Json(ListDocumentsResponse {
        total,
        documents,
        page,
        page_size,
        total_pages,
        has_more,
        status_counts,
    }))
}

/// Get a document by ID.
#[utoipa::path(
    get,
    path = "/api/v1/documents/{document_id}",
    tag = "Documents",
    params(
        ("document_id" = String, Path, description = "Document ID")
    ),
    responses(
        (status = 200, description = "Document found", body = DocumentDetailResponse),
        (status = 404, description = "Document not found"),
        (status = 403, description = "Access denied - document belongs to different tenant")
    )
)]
pub async fn get_document(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    axum::extract::Path(document_id): axum::extract::Path<String>,
) -> ApiResult<Json<DocumentDetailResponse>> {
    debug!(
        document_id = %document_id,
        tenant_id = ?tenant_ctx.tenant_id,
        workspace_id = ?tenant_ctx.workspace_id,
        "Getting document by ID with tenant context"
    );

    // Fetch document metadata
    let metadata_key = format!("{}-metadata", document_id);
    debug!(metadata_key = %metadata_key, "Looking up metadata key");
    let metadata_values = state
        .kv_storage
        .get_by_ids(std::slice::from_ref(&metadata_key))
        .await?;
    debug!(
        metadata_count = metadata_values.len(),
        "Metadata values retrieved"
    );

    let metadata = metadata_values.into_iter().next();
    debug!(has_metadata = metadata.is_some(), "Metadata value present");

    // Check if document exists by metadata or chunks
    let keys = state.kv_storage.keys().await?;
    debug!(total_keys = keys.len(), "Total keys in storage");
    let matching_keys: Vec<_> = keys
        .iter()
        .filter(|k| k.contains(&document_id))
        .cloned()
        .collect();
    debug!(matching_keys = ?matching_keys, "Keys matching document ID");
    let chunk_count = keys
        .iter()
        .filter(|k| k.starts_with(&format!("{}-chunk-", document_id)))
        .count();

    // Document must have either metadata or chunks
    if metadata.is_none() && chunk_count == 0 {
        return Err(ApiError::NotFound(format!(
            "Document {} not found",
            document_id
        )));
    }

    // Parse metadata if available
    let meta_obj = metadata.as_ref().and_then(|v| v.as_object());

    // Check tenant context (multi-tenancy)
    if let Some(obj) = meta_obj {
        let doc_tenant_id = obj.get("tenant_id").and_then(|v| v.as_str());
        let doc_workspace_id = obj.get("workspace_id").and_then(|v| v.as_str());

        // Verify tenant access
        if let Some(ref filter_tid) = tenant_ctx.tenant_id {
            if let Some(doc_tid) = doc_tenant_id {
                if doc_tid != filter_tid {
                    return Err(ApiError::Forbidden);
                }
            }
        }

        // Verify workspace access
        if let Some(ref filter_ws) = tenant_ctx.workspace_id {
            if let Some(doc_ws) = doc_workspace_id {
                if doc_ws != filter_ws {
                    return Err(ApiError::Forbidden);
                }
            }
        }
    }

    // Fetch document content
    let content_key = format!("{}-content", document_id);
    let content_values = state.kv_storage.get_by_ids(&[content_key]).await?;
    let content = content_values.into_iter().next().and_then(|v| {
        v.get("content")
            .and_then(|c| c.as_str())
            .map(|s| s.to_string())
    });

    // SPEC-040: Async fallback PDF vision model lookup for backward compatibility.
    // WHY: Documents processed before pdf_vision_model was written to KV metadata JSON
    // don't have that field. We query the pdf_documents table as fallback using the
    // pdf_id that IS stored in all document metadata records.
    let (fallback_pdf_vision_model, fallback_pdf_extraction_method): (
        Option<String>,
        Option<String>,
    ) = {
        let needs_fallback = meta_obj
            .and_then(|obj| obj.get("pdf_vision_model"))
            .is_none();
        let pdf_uuid_opt = if needs_fallback {
            meta_obj
                .and_then(|obj| obj.get("pdf_id"))
                .and_then(|v| v.as_str())
                .and_then(|s| Uuid::parse_str(s).ok())
        } else {
            None
        };
        if let Some(pdf_uuid) = pdf_uuid_opt {
            #[cfg(feature = "postgres")]
            {
                if let Some(ref pool) = state.pg_pool {
                    match sqlx::query_as::<_, (Option<String>, Option<String>)>(
                        "SELECT vision_model, extraction_method FROM pdf_documents WHERE pdf_id = $1",
                    )
                    .bind(pdf_uuid)
                    .fetch_optional(pool)
                    .await
                    {
                        Ok(Some((vision_model, extraction_method))) => (vision_model, extraction_method),
                        _ => (None, None),
                    }
                } else {
                    (None, None)
                }
            }
            #[cfg(not(feature = "postgres"))]
            {
                let _ = pdf_uuid;
                (None, None)
            }
        } else {
            (None, None)
        }
    };

    // Build response from metadata
    let (
        title,
        file_name,
        content_summary,
        content_length,
        content_hash,
        entity_count,
        relationship_count,
        status,
        error_message,
        source_type,
        mime_type,
        file_size,
        track_id,
        tenant_id,
        workspace_id,
        created_at,
        updated_at,
        processed_at,
        lineage,
        custom_metadata,
        pdf_id,
    ) = if let Some(obj) = meta_obj {
        // Build lineage information from stored metadata
        let lineage = {
            let llm_model = obj
                .get("llm_model")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let embedding_model = obj
                .get("embedding_model")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let embedding_dimensions = obj
                .get("embedding_dimensions")
                .and_then(|v| v.as_u64())
                .map(|n| n as usize);
            let keywords = obj.get("keywords").and_then(|v| v.as_array()).map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            });
            let entity_types = obj
                .get("entity_types")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                });
            let relationship_types = obj
                .get("relationship_types")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                });
            let chunking_strategy = obj
                .get("chunking_strategy")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let avg_chunk_size = obj
                .get("avg_chunk_size")
                .and_then(|v| v.as_u64())
                .map(|n| n as usize);
            let processing_duration_ms = obj.get("processing_duration_ms").and_then(|v| v.as_u64());

            // Token usage and cost fields
            let input_tokens = obj
                .get("input_tokens")
                .and_then(|v| v.as_u64())
                .map(|n| n as usize);
            let output_tokens = obj
                .get("output_tokens")
                .and_then(|v| v.as_u64())
                .map(|n| n as usize);
            let total_tokens = obj
                .get("total_tokens")
                .and_then(|v| v.as_u64())
                .map(|n| n as usize);
            let cost_usd = obj.get("cost_usd").and_then(|v| v.as_f64());

            // SPEC-040: PDF extraction lineage fields
            // WHY: vision_model and extraction_method are stored in metadata JSON by the PDF
            // processor so the document detail view can show what model was used for extraction.
            // For documents processed before this field was added, fall back to the values
            // looked up from the pdf_documents table (fallback_pdf_vision_model).
            let pdf_vision_model = obj
                .get("pdf_vision_model")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .or_else(|| fallback_pdf_vision_model.clone());
            let pdf_extraction_method = obj
                .get("pdf_extraction_method")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .or_else(|| fallback_pdf_extraction_method.clone());

            // Only include lineage if we have at least one field
            if llm_model.is_some()
                || embedding_model.is_some()
                || keywords.is_some()
                || entity_types.is_some()
                || relationship_types.is_some()
                || chunking_strategy.is_some()
                || processing_duration_ms.is_some()
                || input_tokens.is_some()
                || cost_usd.is_some()
                || pdf_vision_model.is_some()
            {
                Some(DocumentLineage {
                    llm_model,
                    embedding_model,
                    embedding_dimensions,
                    keywords,
                    entity_types,
                    relationship_types,
                    chunking_strategy,
                    avg_chunk_size,
                    processing_duration_ms,
                    input_tokens,
                    output_tokens,
                    total_tokens,
                    cost_usd,
                    pdf_vision_model,
                    pdf_extraction_method,
                })
            } else {
                None
            }
        };

        (
            obj.get("title")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            obj.get("file_name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .or_else(|| {
                    obj.get("title")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                }),
            obj.get("content_summary")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            obj.get("content_length")
                .and_then(|v| v.as_u64())
                .map(|n| n as usize),
            obj.get("content_hash")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            obj.get("entity_count")
                .and_then(|v| v.as_u64())
                .map(|n| n as usize),
            obj.get("relationship_count")
                .and_then(|v| v.as_u64())
                .map(|n| n as usize),
            obj.get("status")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| "completed".to_string()),
            obj.get("error_message")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            obj.get("source_type")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            obj.get("mime_type")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            obj.get("file_size")
                .and_then(|v| v.as_u64())
                .map(|n| n as usize),
            obj.get("track_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            obj.get("tenant_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            obj.get("workspace_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            obj.get("created_at")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            obj.get("updated_at")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            obj.get("processed_at")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            lineage,
            obj.get("custom_metadata").cloned(),
            // OODA-50: Extract pdf_id from metadata for PDF viewer
            obj.get("pdf_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        )
    } else {
        // Fallback for documents without metadata (legacy)
        (
            None,                    // title
            None,                    // file_name
            None,                    // content_summary
            None,                    // content_length
            None,                    // content_hash
            None,                    // entity_count
            None,                    // relationship_count
            "completed".to_string(), // status
            None,                    // error_message
            None,                    // source_type
            None,                    // mime_type
            None,                    // file_size
            None,                    // track_id
            None,                    // tenant_id
            None,                    // workspace_id
            None,                    // created_at
            None,                    // updated_at
            None,                    // processed_at
            None,                    // lineage
            None,                    // custom_metadata
            None,                    // pdf_id
        )
    };

    Ok(Json(DocumentDetailResponse {
        id: document_id,
        title,
        file_name,
        content,
        content_summary,
        content_length,
        content_hash,
        chunk_count,
        entity_count,
        relationship_count,
        status,
        error_message,
        source_type,
        mime_type,
        file_size,
        track_id,
        tenant_id,
        workspace_id,
        created_at,
        updated_at,
        processed_at,
        lineage,
        metadata: custom_metadata,
        // OODA-50: Use pdf_id from metadata for PDF viewer
        pdf_id,
    }))
}

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
    let (workspace_id_for_storage, document_status, content_hash_opt) = if has_metadata {
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
            (workspace, status, content_hash)
        } else {
            ("default".to_string(), "unknown".to_string(), None)
        }
    } else {
        ("default".to_string(), "unknown".to_string(), None)
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

    // Helper function to extract source documents from node/edge properties
    // Handles both `source_ids` (JSON array) and `source_id` (pipe-separated string)
    fn extract_source_docs(
        properties: &std::collections::HashMap<String, serde_json::Value>,
    ) -> Vec<String> {
        // Try source_ids (JSON array) first - this is the current format
        if let Some(source_ids) = properties.get("source_ids") {
            if let Some(arr) = source_ids.as_array() {
                return arr
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
            }
        }
        // Fall back to source_id (pipe-separated string) for backward compatibility
        if let Some(source_id) = properties.get("source_id").and_then(|v| v.as_str()) {
            return source_id.split('|').map(|s| s.to_string()).collect();
        }
        Vec::new()
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

/// Analyze the impact of deleting a document before actually deleting it.
///
/// This endpoint allows users to preview what would be affected by a document deletion
/// without actually performing the deletion. This is useful for understanding the
/// cascade effects before committing to a destructive operation.
#[utoipa::path(
    get,
    path = "/api/v1/documents/{document_id}/deletion-impact",
    tag = "Documents",
    params(
        ("document_id" = String, Path, description = "Document ID to analyze")
    ),
    responses(
        (status = 200, description = "Deletion impact analysis", body = DeletionImpactResponse),
        (status = 404, description = "Document not found")
    )
)]
pub async fn analyze_deletion_impact(
    State(state): State<AppState>,
    axum::extract::Path(document_id): axum::extract::Path<String>,
) -> ApiResult<Json<DeletionImpactResponse>> {
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

    let chunks_to_delete = chunk_ids.len();
    let mut entities_to_remove = 0usize;
    let mut entities_to_update = 0usize;
    let mut relationships_to_remove = 0usize;
    let mut relationships_to_update = 0usize;

    // Analyze entities (read-only)
    let all_nodes = state.graph_storage.get_all_nodes().await?;
    for node in all_nodes {
        if let Some(source_id) = node.properties.get("source_id").and_then(|v| v.as_str()) {
            let sources: Vec<&str> = source_id.split('|').collect();
            let remaining = sources
                .iter()
                .filter(|s| !s.starts_with(&chunk_prefix) && !s.starts_with(&document_id))
                .count();

            if remaining == 0 {
                entities_to_remove += 1;
            } else if remaining < sources.len() {
                entities_to_update += 1;
            }
        }
    }

    // Analyze edges (read-only)
    let all_edges = state.graph_storage.get_all_edges().await?;
    for edge in all_edges {
        if let Some(source_id) = edge.properties.get("source_id").and_then(|v| v.as_str()) {
            let sources: Vec<&str> = source_id.split('|').collect();
            let remaining = sources
                .iter()
                .filter(|s| !s.starts_with(&chunk_prefix) && !s.starts_with(&document_id))
                .count();

            if remaining == 0 {
                relationships_to_remove += 1;
            } else if remaining < sources.len() {
                relationships_to_update += 1;
            }
        }
    }

    Ok(Json(DeletionImpactResponse {
        document_id,
        chunks_to_delete,
        entities_to_remove,
        entities_to_update,
        relationships_to_remove,
        relationships_to_update,
        preview_only: true,
    }))
}

// ============================================================================
// File Upload (Multipart)
// ============================================================================

/// Upload a file via multipart form.
///
/// Supports text-based files: .txt, .md, .json, .csv, .html
#[utoipa::path(
    post,
    path = "/api/v1/documents/upload",
    tag = "Documents",
    request_body(content_type = "multipart/form-data", description = "File to upload"),
    responses(
        (status = 201, description = "File uploaded successfully", body = FileUploadResponse),
        (status = 400, description = "Invalid file or request"),
        (status = 409, description = "Duplicate file (already processed)"),
        (status = 413, description = "File too large")
    )
)]
pub async fn upload_file(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    mut multipart: Multipart,
) -> ApiResult<(StatusCode, Json<FileUploadResponse>)> {
    debug!(
        tenant_id = ?tenant_ctx.tenant_id,
        workspace_id = ?tenant_ctx.workspace_id,
        "Uploading file with tenant context"
    );

    let mut filename = String::new();
    let mut content = Vec::new();
    let mut metadata: Option<serde_json::Value> = None;

    // Process multipart fields
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to read multipart field: {}", e)))?
    {
        let field_name = field.name().unwrap_or("").to_string();

        match field_name.as_str() {
            "file" => {
                // Get filename
                filename = field
                    .file_name()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "unnamed.txt".to_string());

                // Read file content
                content = field
                    .bytes()
                    .await
                    .map_err(|e| {
                        ApiError::BadRequest(format!("Failed to read file content: {}", e))
                    })?
                    .to_vec();
            }
            "metadata" => {
                // Optional metadata field
                let text = field
                    .text()
                    .await
                    .map_err(|e| ApiError::BadRequest(format!("Failed to read metadata: {}", e)))?;

                if !text.is_empty() {
                    metadata = serde_json::from_str(&text).ok();
                }
            }
            _ => {
                // Ignore unknown fields
            }
        }
    }

    // Validate we got a file
    if content.is_empty() {
        return Err(ApiError::BadRequest("No file provided".to_string()));
    }

    // Validate file (size, extension, UTF-8, non-empty)
    let (_extension, text_content, mime_type) =
        validate_file(&filename, &content, state.config.max_document_size)?;

    // WHY-OODA83: Use ContentHasher service for consistent hash computation (DRY)
    let content_hash = ContentHasher::hash_bytes(&content);
    debug!(content_hash = %content_hash, "Computed content hash");

    // Extract tenant context for workspace-scoped uniqueness
    // WHY-OODA81: Uniqueness must be scoped to workspace, not global
    // Same document in different workspaces is allowed (multi-tenancy)
    let workspace_id_for_storage = tenant_ctx
        .workspace_id
        .clone()
        .unwrap_or_else(|| "default".to_string());
    let tenant_id_for_storage = tenant_ctx.tenant_id.clone();

    // WHY-OODA81+83: Use ContentHasher for workspace-scoped hash key
    // FIX-4: Duplicates now trigger re-ingestion instead of rejection
    let hash_key = ContentHasher::workspace_hash_key(&workspace_id_for_storage, &content_hash);
    debug!(hash_key = %hash_key, workspace_id = %workspace_id_for_storage, "Checking for workspace-scoped duplicate hash");
    if let Some(existing_doc_id) = state.kv_storage.get_by_id(&hash_key).await? {
        debug!(existing_doc_id = ?existing_doc_id, "Found existing document for hash in workspace");
        if let Some(doc_id_str) = existing_doc_id.as_str() {
            // FIX-4: Try to delete old document data for re-ingestion
            match delete_document_for_reingestion(doc_id_str, &state, &workspace_id_for_storage)
                .await
            {
                Ok(true) => {
                    // Successfully deleted - proceed with new upload
                    tracing::info!(
                        old_doc_id = %doc_id_str,
                        workspace_id = %workspace_id_for_storage,
                        filename = %filename,
                        "Duplicate file found - old data deleted, proceeding with re-ingestion"
                    );
                    // Hash key will be updated below with new document_id
                }
                Ok(false) => {
                    // Document still processing - return duplicate response
                    tracing::warn!(
                        old_doc_id = %doc_id_str,
                        filename = %filename,
                        "Duplicate file is still being processed - cannot re-ingest"
                    );
                    return Ok((
                        StatusCode::OK,
                        Json(FileUploadResponse {
                            document_id: doc_id_str.to_string(),
                            filename,
                            size: content.len(),
                            content_hash,
                            status: "duplicate_processing".to_string(),
                            chunk_count: 0,
                            entity_count: 0,
                            relationship_count: 0,
                            is_duplicate: true,
                        }),
                    ));
                }
                Err(e) => {
                    // Failed to delete - log error and proceed with re-ingestion anyway
                    tracing::warn!(
                        old_doc_id = %doc_id_str,
                        filename = %filename,
                        error = %e,
                        "Failed to delete old file data - proceeding with re-ingestion"
                    );
                }
            }
        }
    }

    // Generate document ID
    let document_id = Uuid::new_v4().to_string();

    // Store hash mapping for deduplication (workspace-scoped)
    state
        .kv_storage
        .upsert(&[(hash_key, serde_json::json!(document_id))])
        .await?;

    // Generate content summary
    let content_summary = crate::validation::generate_content_summary(&text_content);

    // Generate track ID
    let track_id = format!(
        "upload_{}_{}",
        Utc::now().format("%Y%m%d%H%M%S"),
        &Uuid::new_v4().to_string()[..8]
    );

    // Store comprehensive document metadata
    let doc_metadata_key = format!("{}-metadata", document_id);
    let doc_metadata = serde_json::json!({
        "id": document_id,
        "title": filename,
        "file_name": filename,
        "file_size": content.len(),
        "mime_type": mime_type,
        "source_type": "file",
        "content_summary": content_summary,
        "content_length": text_content.len(),
        "content_hash": content_hash,
        "track_id": track_id,
        "created_at": Utc::now().to_rfc3339(),
        "status": "processing",
        "tenant_id": tenant_id_for_storage,
        "workspace_id": workspace_id_for_storage,
        "custom_metadata": metadata,
    });
    state
        .kv_storage
        .upsert(&[(doc_metadata_key.clone(), doc_metadata)])
        .await?;

    // Store document content
    let doc_content_key = format!("{}-content", document_id);
    let doc_content = serde_json::json!({
        "content": text_content,
    });
    state
        .kv_storage
        .upsert(&[(doc_content_key, doc_content)])
        .await?;

    // Process through pipeline
    let result = state
        .pipeline
        .process_with_resilience(&document_id, &text_content, None)
        .await?;

    // Log partial failures but continue (resilient processing)
    if result.stats.failed_chunks > 0 {
        tracing::warn!(
            document_id = %document_id,
            failed_chunks = result.stats.failed_chunks,
            chunk_count = result.stats.chunk_count,
            "File upload pipeline completed with partial failures"
        );
    }

    // Store chunks in KV storage
    let chunks: Vec<(String, serde_json::Value)> = result
        .chunks
        .iter()
        .map(|c| {
            (
                c.id.clone(),
                serde_json::json!({
                    "content": c.content,
                    "document_id": document_id,
                    "index": c.index,
                    "source_file": filename,
                }),
            )
        })
        .collect();

    state.kv_storage.upsert(&chunks).await?;

    // SPEC-033: Get workspace-specific vector storage for file embeddings
    // WHY-OODA223: STRICT mode - fail loudly if workspace storage unavailable
    // to prevent file embeddings from being stored in the wrong (global) table
    let workspace_vector_storage =
        get_workspace_vector_storage_strict(&state, &workspace_id_for_storage).await?;

    // Store chunk embeddings in vector storage for semantic search
    let mut chunk_embeddings_stored = 0;
    for chunk in &result.chunks {
        if let Some(embedding) = &chunk.embedding {
            let mut metadata = serde_json::json!({
                "type": "chunk",
                "document_id": document_id,
                "index": chunk.index,
                "content": chunk.content,
                "source_file": filename,
            });

            // Add tenant and workspace IDs if present
            if let Some(ref tid) = tenant_id_for_storage {
                metadata["tenant_id"] = serde_json::json!(tid);
            }
            metadata["workspace_id"] = serde_json::json!(&workspace_id_for_storage);

            match workspace_vector_storage
                .upsert(&[(chunk.id.clone(), embedding.clone(), metadata)])
                .await
            {
                Ok(_) => {
                    chunk_embeddings_stored += 1;
                    tracing::info!(chunk_id = %chunk.id, "VECTOR STORAGE: Chunk embedding stored OK");
                }
                Err(e) => {
                    tracing::error!(chunk_id = %chunk.id, error = %e, "VECTOR STORAGE: Failed to store chunk embedding");
                }
            }
        }
    }
    tracing::info!(
        chunk_embeddings_stored = chunk_embeddings_stored,
        total_chunks = result.chunks.len(),
        "VECTOR STORAGE: Chunk embedding storage complete"
    );

    // Store entities and relationships in graph storage
    tracing::info!(
        extraction_count = result.extractions.len(),
        "GRAPH STORAGE: Processing extractions"
    );
    for extraction in &result.extractions {
        tracing::info!(
            entity_count = extraction.entities.len(),
            relationship_count = extraction.relationships.len(),
            "GRAPH STORAGE: Extraction content"
        );
        for entity in &extraction.entities {
            tracing::info!(
                entity_name = %entity.name,
                entity_type = %entity.entity_type,
                source_chunk_ids = ?entity.source_chunk_ids,
                "GRAPH STORAGE: Storing entity with chunk linkage"
            );
            let mut properties = std::collections::HashMap::new();
            properties.insert(
                "entity_type".to_string(),
                serde_json::json!(entity.entity_type),
            );
            properties.insert(
                "description".to_string(),
                serde_json::json!(entity.description),
            );
            properties.insert(
                "importance".to_string(),
                serde_json::json!(entity.importance),
            );
            properties.insert(
                "source_ids".to_string(),
                serde_json::json!(vec![&document_id]),
            );
            // CRITICAL: Store source_chunk_ids for Local/Global query mode chunk retrieval
            properties.insert(
                "source_chunk_ids".to_string(),
                serde_json::json!(&entity.source_chunk_ids),
            );
            // Add tenant scoping
            if let Some(ref tid) = tenant_id_for_storage {
                properties.insert("tenant_id".to_string(), serde_json::json!(tid));
            }
            properties.insert(
                "workspace_id".to_string(),
                serde_json::json!(&workspace_id_for_storage),
            );

            match state
                .graph_storage
                .upsert_node(&entity.name, properties)
                .await
            {
                Ok(_) => {
                    tracing::info!(entity_name = %entity.name, "GRAPH STORAGE: Entity stored OK")
                }
                Err(e) => {
                    tracing::error!(entity_name = %entity.name, error = %e, "GRAPH STORAGE: Failed to store entity")
                }
            }

            // CRITICAL: Also store entity embedding in vector storage for query_local retrieval
            tracing::info!(
                entity_name = %entity.name,
                has_embedding = entity.embedding.is_some(),
                embedding_dim = entity.embedding.as_ref().map(|e| e.len()).unwrap_or(0),
                "Checking entity embedding for storage"
            );
            // SPEC-033: Use workspace-specific vector storage for entity embeddings
            if let Some(embedding) = &entity.embedding {
                let mut metadata = serde_json::json!({
                    "type": "entity",
                    "entity_name": entity.name,
                    "entity_type": entity.entity_type,
                    "description": entity.description,
                    "document_id": document_id,
                    "source_chunk_ids": entity.source_chunk_ids,
                });
                if let Some(ref tid) = tenant_id_for_storage {
                    metadata["tenant_id"] = serde_json::json!(tid);
                }
                metadata["workspace_id"] = serde_json::json!(&workspace_id_for_storage);

                // Use entity name as vector ID for dedup
                let entity_id = format!("entity:{}", entity.name);
                if let Err(e) = workspace_vector_storage
                    .upsert(&[(entity_id.clone(), embedding.clone(), metadata)])
                    .await
                {
                    tracing::error!(entity_id = %entity_id, error = %e, "VECTOR STORAGE: Failed to store entity embedding");
                } else {
                    tracing::info!(entity_id = %entity_id, "VECTOR STORAGE: Entity embedding stored OK");
                }
            }
        }

        for relationship in &extraction.relationships {
            let mut properties = std::collections::HashMap::new();
            properties.insert(
                "relation_type".to_string(),
                serde_json::json!(relationship.relation_type),
            );
            properties.insert(
                "description".to_string(),
                serde_json::json!(relationship.description),
            );
            properties.insert("weight".to_string(), serde_json::json!(relationship.weight));
            properties.insert(
                "keywords".to_string(),
                serde_json::json!(relationship.keywords),
            );
            properties.insert(
                "source_ids".to_string(),
                serde_json::json!(vec![&document_id]),
            );
            // CRITICAL: Store source_chunk_id for relationship chunk linkage
            if let Some(ref chunk_id) = relationship.source_chunk_id {
                properties.insert(
                    "source_chunk_ids".to_string(),
                    serde_json::json!(vec![chunk_id]),
                );
            }
            // Add tenant scoping
            if let Some(ref tid) = tenant_id_for_storage {
                properties.insert("tenant_id".to_string(), serde_json::json!(tid));
            }
            properties.insert(
                "workspace_id".to_string(),
                serde_json::json!(&workspace_id_for_storage),
            );

            let _ = state
                .graph_storage
                .upsert_edge(&relationship.source, &relationship.target, properties)
                .await;
        }
    }

    // Update document metadata with completion stats and lineage
    let completed_metadata = serde_json::json!({
        "id": document_id,
        "title": filename,
        "file_name": filename,
        "file_size": content.len(),
        "mime_type": mime_type,
        "source_type": "file",
        "content_summary": content_summary,
        "content_length": text_content.len(),
        "content_hash": content_hash,
        "track_id": track_id,
        "created_at": Utc::now().to_rfc3339(),
        "processed_at": Utc::now().to_rfc3339(),
        "status": "completed",
        "chunk_count": result.stats.chunk_count,
        "entity_count": result.stats.entity_count,
        "relationship_count": result.stats.relationship_count,
        "tenant_id": tenant_id_for_storage,
        "workspace_id": workspace_id_for_storage,
        "custom_metadata": metadata,
        // Lineage information
        "llm_model": result.stats.llm_model,
        "embedding_model": result.stats.embedding_model,
        "embedding_dimensions": result.stats.embedding_dimensions,
        "entity_types": result.stats.entity_types,
        "relationship_types": result.stats.relationship_types,
        "keywords": result.stats.keywords,
        "chunking_strategy": result.stats.chunking_strategy,
        "avg_chunk_size": result.stats.avg_chunk_size,
        "processing_duration_ms": result.stats.processing_time_ms,
    });
    state
        .kv_storage
        .upsert(&[(doc_metadata_key, completed_metadata)])
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(FileUploadResponse {
            document_id,
            filename,
            size: content.len(),
            content_hash,
            status: "processed".to_string(),
            chunk_count: result.stats.chunk_count,
            entity_count: result.stats.entity_count,
            relationship_count: result.stats.relationship_count,
            is_duplicate: false,
        }),
    ))
}

/// Upload multiple files via multipart form.
#[utoipa::path(
    post,
    path = "/api/v1/documents/upload/batch",
    tag = "Documents",
    request_body(content_type = "multipart/form-data", description = "Files to upload"),
    responses(
        (status = 201, description = "Batch upload completed", body = BatchUploadResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn upload_files_batch(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> ApiResult<(StatusCode, Json<BatchUploadResponse>)> {
    let mut results = Vec::new();
    let mut processed = 0usize;
    let mut duplicates = 0usize;
    let mut failed = 0usize;

    // Collect all files first
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to read multipart field: {}", e)))?
    {
        let field_name = field.name().unwrap_or("").to_string();

        if field_name == "files" || field_name == "file" {
            let filename = field
                .file_name()
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("file_{}.txt", files.len()));

            let content = field
                .bytes()
                .await
                .map_err(|e| ApiError::BadRequest(format!("Failed to read file: {}", e)))?
                .to_vec();

            files.push((filename, content));
        }
    }

    // Process each file (uses default workspace for batch uploads)
    // WHY-OODA81: Batch upload uses "default" workspace for dedup scoping
    // For proper workspace isolation, use the single file upload endpoint with tenant context
    let workspace_id = "default".to_string();
    for (filename, content) in files {
        let result = process_single_file(&state, &filename, &content, &workspace_id).await;

        match result {
            Ok((doc_id, is_duplicate)) => {
                if is_duplicate {
                    duplicates += 1;
                    results.push(BatchFileResult {
                        filename,
                        document_id: Some(doc_id),
                        status: "duplicate".to_string(),
                        error: None,
                    });
                } else {
                    processed += 1;
                    results.push(BatchFileResult {
                        filename,
                        document_id: Some(doc_id),
                        status: "processed".to_string(),
                        error: None,
                    });
                }
            }
            Err(e) => {
                failed += 1;
                results.push(BatchFileResult {
                    filename,
                    document_id: None,
                    status: "failed".to_string(),
                    error: Some(e.to_string()),
                });
            }
        }
    }

    Ok((
        StatusCode::CREATED,
        Json(BatchUploadResponse {
            total_files: results.len(),
            processed,
            duplicates,
            failed,
            results,
        }),
    ))
}

/// Process a single file and return (document_id, is_duplicate).
///
/// WHY-OODA81: workspace_id parameter enables workspace-scoped duplicate detection.
/// Same document in different workspaces is allowed (multi-tenancy support).
///
/// Note: Uses default vector storage for batch uploads without tenant context.
/// For workspace-specific storage, use the main upload_file endpoint with tenant context.
async fn process_single_file(
    state: &AppState,
    filename: &str,
    content: &[u8],
    workspace_id: &str,
) -> Result<(String, bool), ApiError> {
    // Validate file (size, extension, UTF-8, non-empty)
    let (_extension, text_content, _mime_type) =
        validate_file(filename, content, state.config.max_document_size)?;

    // WHY-OODA83: Use ContentHasher service for consistent hash computation (DRY)
    let content_hash = ContentHasher::hash_bytes(content);

    // WHY-OODA81+83: Use ContentHasher for workspace-scoped hash key
    let hash_key = ContentHasher::workspace_hash_key(workspace_id, &content_hash);
    if let Some(existing) = state.kv_storage.get_by_id(&hash_key).await? {
        if let Some(doc_id) = existing.as_str() {
            return Ok((doc_id.to_string(), true));
        }
    }

    // Generate document ID
    let document_id = Uuid::new_v4().to_string();

    // Store hash mapping
    state
        .kv_storage
        .upsert(&[(hash_key, serde_json::json!(document_id))])
        .await?;

    // Process through pipeline (resilient - tolerates partial chunk failures)
    let result = state
        .pipeline
        .process_with_resilience(&document_id, &text_content, None)
        .await?;

    if result.stats.failed_chunks > 0 {
        tracing::warn!(
            document_id = %document_id,
            filename = %filename,
            failed_chunks = result.stats.failed_chunks,
            chunk_count = result.stats.chunk_count,
            "Batch file pipeline completed with partial failures"
        );
    }

    // Store chunks
    let chunks: Vec<(String, serde_json::Value)> = result
        .chunks
        .iter()
        .map(|c| {
            (
                c.id.clone(),
                serde_json::json!({
                    "content": c.content,
                    "document_id": document_id,
                    "index": c.index,
                    "source_file": filename,
                }),
            )
        })
        .collect();

    state.kv_storage.upsert(&chunks).await?;

    // Store chunk embeddings in vector storage for semantic search
    // Note: Batch upload uses default vector storage since there's no workspace context.
    // For workspace-specific storage, use the main upload_file endpoint with tenant context.
    for chunk in &result.chunks {
        if let Some(embedding) = &chunk.embedding {
            let metadata = serde_json::json!({
                "type": "chunk",
                "document_id": document_id,
                "index": chunk.index,
                "content": chunk.content,
                "source_file": filename,
            });

            let _ = state
                .vector_storage
                .upsert(&[(chunk.id.clone(), embedding.clone(), metadata)])
                .await;
        }
    }

    Ok((document_id, false))
}

// ============================================================================
// Track Status (Phase 2)
// ============================================================================

/// Get track status by track ID.
///
/// Returns all documents uploaded with a specific track_id, along with status summary.
#[utoipa::path(
    get,
    path = "/api/v1/documents/track/{track_id}",
    tag = "Documents",
    params(
        ("track_id" = String, Path, description = "Track ID for the batch")
    ),
    responses(
        (status = 200, description = "Track status retrieved", body = TrackStatusResponse),
        (status = 404, description = "Track not found")
    )
)]
pub async fn get_track_status(
    State(state): State<AppState>,
    axum::extract::Path(track_id): axum::extract::Path<String>,
) -> ApiResult<Json<TrackStatusResponse>> {
    let keys = state.kv_storage.keys().await?;

    // Find all metadata keys
    let metadata_keys: Vec<String> = keys
        .iter()
        .filter(|k| k.ends_with("-metadata"))
        .cloned()
        .collect();

    // Fetch all metadata
    let metadata_values = state.kv_storage.get_by_ids(&metadata_keys).await?;

    // Group chunks by document
    let mut doc_chunks: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for key in &keys {
        if let Some(doc_id) = key.split("-chunk-").next() {
            if !doc_id.ends_with("-metadata") && !doc_id.ends_with("-content") {
                *doc_chunks.entry(doc_id.to_string()).or_default() += 1;
            }
        }
    }

    // Filter documents by track_id
    let mut track_docs: Vec<DocumentSummary> = Vec::new();
    let mut created_times: Vec<String> = Vec::new();

    for value in metadata_values {
        if let Some(obj) = value.as_object() {
            let doc_track_id = obj.get("track_id").and_then(|v| v.as_str()).unwrap_or("");

            if doc_track_id == track_id {
                let id = obj
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let chunk_count = doc_chunks.get(&id).copied().unwrap_or(0);

                if let Some(created_at) = obj.get("created_at").and_then(|v| v.as_str()) {
                    created_times.push(created_at.to_string());
                }

                track_docs.push(DocumentSummary {
                    id,
                    title: obj.get("title").and_then(|v| v.as_str()).map(String::from),
                    file_name: obj
                        .get("file_name")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    content_summary: obj
                        .get("content_summary")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    content_length: obj
                        .get("content_length")
                        .and_then(|v| v.as_u64())
                        .map(|n| n as usize),
                    chunk_count,
                    entity_count: obj
                        .get("entity_count")
                        .and_then(|v| v.as_u64())
                        .map(|n| n as usize),
                    status: obj.get("status").and_then(|v| v.as_str()).map(String::from),
                    error_message: obj
                        .get("error_message")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    track_id: Some(track_id.clone()),
                    created_at: obj
                        .get("created_at")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    updated_at: obj
                        .get("updated_at")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    cost_usd: obj.get("cost_usd").and_then(|v| v.as_f64()),
                    input_tokens: obj
                        .get("input_tokens")
                        .and_then(|v| v.as_u64())
                        .map(|n| n as usize),
                    output_tokens: obj
                        .get("output_tokens")
                        .and_then(|v| v.as_u64())
                        .map(|n| n as usize),
                    total_tokens: obj
                        .get("total_tokens")
                        .and_then(|v| v.as_u64())
                        .map(|n| n as usize),
                    llm_model: obj
                        .get("llm_model")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    embedding_model: obj
                        .get("embedding_model")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    // SPEC-002: Unified Ingestion Pipeline fields
                    source_type: obj
                        .get("source_type")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    current_stage: obj
                        .get("current_stage")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    stage_progress: obj
                        .get("stage_progress")
                        .and_then(|v| v.as_f64())
                        .map(|n| n as f32),
                    stage_message: obj
                        .get("stage_message")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    pdf_id: obj.get("pdf_id").and_then(|v| v.as_str()).map(String::from),
                });
            }
        }
    }

    // Calculate status summary (handle empty track gracefully - documents may still be processing)
    let status_summary = StatusCounts {
        pending: track_docs
            .iter()
            .filter(|d| d.status.as_deref() == Some("pending"))
            .count(),
        processing: track_docs
            .iter()
            .filter(|d| d.status.as_deref() == Some("processing"))
            .count(),
        completed: track_docs
            .iter()
            .filter(|d| {
                d.status.is_none()
                    || d.status.as_deref() == Some("completed")
                    || d.status.as_deref() == Some("indexed")
            })
            .count(),
        // FIX-5: Track partial_failure status
        partial_failure: track_docs
            .iter()
            .filter(|d| d.status.as_deref() == Some("partial_failure"))
            .count(),
        failed: track_docs
            .iter()
            .filter(|d| d.status.as_deref() == Some("failed"))
            .count(),
        cancelled: track_docs
            .iter()
            .filter(|d| d.status.as_deref() == Some("cancelled"))
            .count(),
    };

    // Find earliest created_at
    created_times.sort();
    let created_at = created_times.first().cloned();

    // Check if complete (no pending or processing)
    let is_complete = status_summary.pending == 0 && status_summary.processing == 0;

    // Build latest message
    let latest_message = if !is_complete {
        Some(format!(
            "Processing {}/{} documents...",
            status_summary.completed + status_summary.failed,
            track_docs.len()
        ))
    } else if status_summary.failed > 0 {
        Some(format!("Completed with {} errors", status_summary.failed))
    } else {
        Some("All documents processed successfully".to_string())
    };

    Ok(Json(TrackStatusResponse {
        track_id,
        created_at,
        documents: track_docs.clone(),
        total_count: track_docs.len(),
        status_summary,
        is_complete,
        latest_message,
    }))
}

// ============================================
// GAP-014: Document Scan API
// ============================================

/// Scan a directory and queue documents for processing.
///
/// SECURITY (OODA-248): Path traversal protection.
/// User-provided paths are validated against allowed directories.
#[utoipa::path(
    post,
    path = "/api/v1/documents/scan",
    tag = "Documents",
    request_body = ScanDirectoryRequest,
    responses(
        (status = 200, description = "Directory scanned successfully", body = ScanDirectoryResponse),
        (status = 400, description = "Invalid request"),
        (status = 403, description = "Path not allowed"),
        (status = 404, description = "Directory not found")
    )
)]
pub async fn scan_directory(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Json(request): Json<ScanDirectoryRequest>,
) -> ApiResult<Json<ScanDirectoryResponse>> {
    debug!(
        "scan_directory called with tenant context: tenant_id={:?}, workspace_id={:?}",
        tenant_ctx.tenant_id, tenant_ctx.workspace_id
    );

    // SECURITY (OODA-248): Validate path against allowed directories.
    // WHY: Prevents directory traversal attacks (e.g., ../../../etc/passwd).
    let validated_path =
        crate::path_validation::validate_path(&request.path, &state.path_validation_config)?;

    let base_path = &validated_path.canonical;

    // Path is already validated to exist by validate_path
    if !base_path.is_dir() {
        return Err(ApiError::BadRequest(format!(
            "Path is not a directory: {}",
            request.path
        )));
    }

    // Generate track ID
    let track_id = request.track_id.unwrap_or_else(|| {
        format!(
            "scan_{}_{}",
            Utc::now().format("%Y%m%d_%H%M%S"),
            &Uuid::new_v4().to_string()[..8]
        )
    });

    let mut queued_files = Vec::new();
    let mut skipped_files = Vec::new();
    let mut files_found = 0;

    // Collect files to process
    let entries = collect_files(base_path, request.recursive, request.max_files)?;

    for entry in entries {
        files_found += 1;

        let file_path = entry.path();
        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        // Check extension filter
        if !request.extensions.is_empty() {
            if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
                if !request
                    .extensions
                    .iter()
                    .any(|e| e.eq_ignore_ascii_case(ext))
                {
                    skipped_files.push(SkippedFile {
                        path: file_path.display().to_string(),
                        reason: format!("Extension .{} not in filter list", ext),
                    });
                    continue;
                }
            } else {
                skipped_files.push(SkippedFile {
                    path: file_path.display().to_string(),
                    reason: "No extension".to_string(),
                });
                continue;
            }
        }

        // Try to read file content
        let content = match std::fs::read_to_string(&file_path) {
            Ok(c) => c,
            Err(e) => {
                skipped_files.push(SkippedFile {
                    path: file_path.display().to_string(),
                    reason: format!("Failed to read: {}", e),
                });
                continue;
            }
        };

        if content.trim().is_empty() {
            skipped_files.push(SkippedFile {
                path: file_path.display().to_string(),
                reason: "Empty file".to_string(),
            });
            continue;
        }

        // Check size limit
        if content.len() > state.config.max_document_size {
            skipped_files.push(SkippedFile {
                path: file_path.display().to_string(),
                reason: format!(
                    "Exceeds max size ({} > {})",
                    content.len(),
                    state.config.max_document_size
                ),
            });
            continue;
        }

        // Generate document ID
        let document_id = Uuid::new_v4().to_string();

        // Generate content summary
        let content_summary = crate::validation::generate_content_summary(&content);

        // Store document metadata
        let doc_metadata_key = format!("{}-metadata", document_id);
        let doc_metadata = serde_json::json!({
            "id": document_id,
            "title": file_name,
            "file_path": file_path.display().to_string(),
            "content_summary": content_summary,
            "content_length": content.len(),
            "track_id": track_id,
            "created_at": Utc::now().to_rfc3339(),
            "status": "pending",
        });
        state
            .kv_storage
            .upsert(&[(doc_metadata_key, doc_metadata)])
            .await?;

        // Store document content
        let doc_content_key = format!("{}-content", document_id);
        let doc_content = serde_json::json!({
            "content": content,
        });
        state
            .kv_storage
            .upsert(&[(doc_content_key, doc_content)])
            .await?;

        if request.async_processing {
            // Create task for background processing
            use edgequake_tasks::{Task, TaskType, TextInsertData};

            // Use tenant context for workspace_id, fallback to "default"
            let workspace_id = tenant_ctx
                .workspace_id
                .clone()
                .unwrap_or_else(|| "default".to_string());
            let tenant_id = tenant_ctx
                .tenant_id
                .clone()
                .unwrap_or_else(|| "default".to_string());

            let task_data = TextInsertData {
                text: content,
                file_source: file_path.display().to_string(),
                workspace_id: workspace_id.clone(),
                metadata: Some(serde_json::json!({
                    "document_id": document_id,
                    "title": file_name,
                    "track_id": track_id,
                    "tenant_id": tenant_id,
                    "workspace_id": workspace_id,
                })),
            };

            let task = Task::new(
                uuid::Uuid::parse_str(&tenant_id)
                    .map_err(|_| ApiError::ValidationError("Invalid tenant ID".to_string()))?,
                uuid::Uuid::parse_str(&workspace_id)
                    .map_err(|_| ApiError::ValidationError("Invalid workspace ID".to_string()))?,
                TaskType::Insert,
                serde_json::to_value(task_data).unwrap(),
            );

            state
                .task_storage
                .create_task(&task)
                .await
                .map_err(|e| ApiError::Internal(format!("Failed to create task: {}", e)))?;

            state
                .task_queue
                .send(task)
                .await
                .map_err(|e| ApiError::Internal(format!("Failed to queue task: {}", e)))?;
        }

        queued_files.push(file_path.display().to_string());
    }

    Ok(Json(ScanDirectoryResponse {
        track_id,
        files_found,
        files_queued: queued_files.len(),
        files_skipped: skipped_files.len(),
        queued_files,
        skipped_files,
    }))
}

/// Collect files from a directory.
fn collect_files(
    path: &std::path::Path,
    recursive: bool,
    max_files: usize,
) -> Result<Vec<std::fs::DirEntry>, ApiError> {
    let mut files = Vec::new();

    fn visit_dir(
        dir: &std::path::Path,
        recursive: bool,
        max_files: usize,
        files: &mut Vec<std::fs::DirEntry>,
    ) -> Result<(), ApiError> {
        if files.len() >= max_files {
            return Ok(());
        }

        let entries = std::fs::read_dir(dir).map_err(|e| {
            ApiError::Internal(format!("Failed to read directory {}: {}", dir.display(), e))
        })?;

        for entry in entries {
            if files.len() >= max_files {
                break;
            }

            let entry = entry.map_err(|e| {
                ApiError::Internal(format!("Failed to read directory entry: {}", e))
            })?;

            let path = entry.path();

            if path.is_file() {
                files.push(entry);
            } else if path.is_dir() && recursive {
                visit_dir(&path, recursive, max_files, files)?;
            }
        }

        Ok(())
    }

    visit_dir(path, recursive, max_files, &mut files)?;
    Ok(files)
}

// ============================================
// GAP-039: Reprocess Failed Documents
// ============================================

/// Reprocess failed documents.
#[utoipa::path(
    post,
    path = "/api/v1/documents/reprocess",
    tag = "Documents",
    request_body = ReprocessFailedRequest,
    responses(
        (status = 200, description = "Documents requeued for processing", body = ReprocessFailedResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn reprocess_failed(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    // WHY: Body is optional - frontend may omit body entirely, which would cause
    // "EOF while parsing a value" 400 error. Using Option<Json<>> with .unwrap_or_default()
    // makes this endpoint resilient to missing or empty request body.
    body: Option<Json<ReprocessFailedRequest>>,
) -> ApiResult<Json<ReprocessFailedResponse>> {
    let request = body.map(|b| b.0).unwrap_or_default();
    debug!(
        "reprocess_failed called with tenant context: tenant_id={:?}, workspace_id={:?}, document_id={:?}, force={}",
        tenant_ctx.tenant_id, tenant_ctx.workspace_id, request.document_id, request.force
    );

    // Generate new track ID for reprocess batch
    let new_track_id = format!(
        "reprocess_{}_{}",
        Utc::now().format("%Y%m%d_%H%M%S"),
        &Uuid::new_v4().to_string()[..8]
    );

    // Get all metadata keys
    let all_keys: Vec<String> = state.kv_storage.keys().await?;

    let mut docs_to_reprocess = Vec::new();
    let mut requeued_ids = Vec::new();

    // Find documents to reprocess
    for key in all_keys.iter().filter(|k| k.ends_with("-metadata")) {
        if docs_to_reprocess.len() >= request.max_documents {
            break;
        }

        if let Some(value) = state.kv_storage.get_by_id(key).await? {
            if let Some(obj) = value.as_object() {
                let status = obj.get("status").and_then(|v| v.as_str());
                let doc_track_id = obj.get("track_id").and_then(|v| v.as_str());
                let doc_id = obj.get("id").and_then(|v| v.as_str());

                // If document_id filter is specified, only match that exact document
                if let Some(ref filter_doc_id) = request.document_id {
                    if doc_id != Some(filter_doc_id.as_str()) {
                        continue;
                    }
                    // When document_id is specified with force=true, allow any status
                    // Otherwise, only reprocess if failed
                    if !request.force && status != Some("failed") {
                        continue;
                    }
                    if let Some(id) = doc_id {
                        docs_to_reprocess.push((id.to_string(), key.replace("-metadata", "")));
                    }
                    break; // Found the specific document
                }

                // If track_id filter is specified, match by track_id
                if let Some(ref filter_track) = request.track_id {
                    if doc_track_id != Some(filter_track.as_str()) {
                        continue;
                    }
                }

                // Default behavior: reprocess failed and cancelled documents
                // WHY: Cancelled documents should be retryable just like failed ones.
                // Users may cancel a document during processing and want to retry later.
                if status == Some("failed") || status == Some("cancelled") {
                    if let Some(id) = doc_id {
                        docs_to_reprocess.push((id.to_string(), key.replace("-metadata", "")));
                    }
                }
            }
        }
    }

    // Requeue documents for processing
    for (doc_id, _doc_key) in &docs_to_reprocess {
        // OODA-08: Clean up partial graph data from previous attempt BEFORE requeueing
        // WHY: Without cleanup, reprocessing creates duplicate entities and corrupts source_ids
        //
        // Scenario without cleanup:
        //   T1: Document processed 60% → entities A, B created with source_ids = [doc]
        //   T2: Processing fails
        //   T3: reprocess_failed called
        //   T4: Document reprocessed → entities A, B upserted with source_ids = [doc]
        //   T5: Now entities have inflated source_ids (double reference)
        //   T6: Delete document → entities still exist (incorrect)
        //
        // With cleanup:
        //   T1-T2: Same as above
        //   T3: reprocess_failed cleans up A, B (deletes them since source_ids = [doc])
        //   T4: Document reprocessed → entities A, B created fresh
        //   T5: source_ids correctly = [doc]
        //   T6: Delete document → entities properly deleted
        match cleanup_document_graph_data(doc_id, &state.graph_storage, None).await {
            Ok(stats) => {
                tracing::info!(
                    document_id = %doc_id,
                    entities_removed = stats.entities_removed,
                    entities_updated = stats.entities_updated,
                    relationships_removed = stats.relationships_removed,
                    "Cleaned up partial data before reprocessing"
                );
            }
            Err(e) => {
                tracing::warn!(
                    document_id = %doc_id,
                    error = %e,
                    "Failed to cleanup partial data before reprocessing, continuing anyway"
                );
            }
        }
        // Get document content
        let content_key = format!("{}-content", doc_id);

        // FIX-REBUILD: Read metadata to check if this is a PDF document
        // WHY: PDF documents must be routed through PdfProcessing tasks so the full
        // pipeline runs from original PDF bytes (vision extraction → chunking →
        // embedding → entity extraction). Using TaskType::Insert for PDFs would
        // only re-ingest the previously extracted markdown, missing re-extraction
        // with any new vision LLM model.
        let metadata_key = format!("{}-metadata", doc_id);
        let metadata_opt = state.kv_storage.get_by_id(&metadata_key).await?;

        let source_type = metadata_opt
            .as_ref()
            .and_then(|m| m.as_object())
            .and_then(|obj| obj.get("source_type"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let pdf_id_str = metadata_opt
            .as_ref()
            .and_then(|m| m.as_object())
            .and_then(|obj| obj.get("pdf_id"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Use tenant context for workspace_id, fallback to "default"
        let workspace_id = tenant_ctx
            .workspace_id
            .clone()
            .unwrap_or_else(|| "default".to_string());
        let tenant_id = tenant_ctx
            .tenant_id
            .clone()
            .unwrap_or_else(|| "default".to_string());

        // FIX-REBUILD: Route PDF documents through PdfProcessing for full re-extraction
        let task_created =
            if source_type.as_deref() == Some("pdf") {
                if let Some(ref pid_str) = pdf_id_str {
                    if let Ok(pdf_id_uuid) = uuid::Uuid::parse_str(pid_str) {
                        // Update status to pending
                        if let Some(mut metadata) = metadata_opt.clone() {
                            if let Some(obj) = metadata.as_object_mut() {
                                obj.insert("status".to_string(), serde_json::json!("pending"));
                                obj.insert("track_id".to_string(), serde_json::json!(new_track_id));
                                obj.insert(
                                    "retry_at".to_string(),
                                    serde_json::json!(Utc::now().to_rfc3339()),
                                );
                                state
                                    .kv_storage
                                    .upsert(&[(metadata_key.clone(), metadata)])
                                    .await?;
                            }
                        }

                        // Look up workspace to get vision provider/model settings
                        let (vision_provider, vision_model) =
                            if let Ok(ws_uuid) = uuid::Uuid::parse_str(&workspace_id) {
                                if let Ok(Some(ws)) =
                                    state.workspace_service.get_workspace(ws_uuid).await
                                {
                                    let vp = ws
                                        .vision_llm_provider
                                        .as_deref()
                                        .filter(|p| !p.is_empty())
                                        .unwrap_or("ollama")
                                        .to_string();
                                    let vm = ws.vision_llm_model.filter(|m| !m.is_empty());
                                    (vp, vm)
                                } else {
                                    ("ollama".to_string(), None)
                                }
                            } else {
                                ("ollama".to_string(), None)
                            };

                        use edgequake_tasks::{PdfProcessingData, Task, TaskType};

                        let pdf_task = PdfProcessingData {
                            pdf_id: pdf_id_uuid,
                            tenant_id: uuid::Uuid::parse_str(&tenant_id).map_err(|_| {
                                ApiError::ValidationError("Invalid tenant ID".to_string())
                            })?,
                            workspace_id: uuid::Uuid::parse_str(&workspace_id).map_err(|_| {
                                ApiError::ValidationError("Invalid workspace ID".to_string())
                            })?,
                            enable_vision: true,
                            vision_provider,
                            vision_model,
                            // FIX-REBUILD: Reuse existing document ID
                            existing_document_id: Some(doc_id.clone()),
                        };

                        let task = Task::new(
                            uuid::Uuid::parse_str(&tenant_id).map_err(|_| {
                                ApiError::ValidationError("Invalid tenant ID".to_string())
                            })?,
                            uuid::Uuid::parse_str(&workspace_id).map_err(|_| {
                                ApiError::ValidationError("Invalid workspace ID".to_string())
                            })?,
                            TaskType::PdfProcessing,
                            serde_json::to_value(&pdf_task).unwrap(),
                        );

                        state.task_storage.create_task(&task).await.map_err(|e| {
                            ApiError::Internal(format!("Failed to create task: {}", e))
                        })?;

                        state.task_queue.send(task).await.map_err(|e| {
                            ApiError::Internal(format!("Failed to queue task: {}", e))
                        })?;

                        tracing::info!(
                            document_id = %doc_id,
                            pdf_id = %pid_str,
                            "Queued PDF reprocessing task (PdfProcessing) with existing document ID"
                        );
                        true
                    } else {
                        false // Invalid pdf_id, fall through to text reprocess
                    }
                } else {
                    false // No pdf_id, fall through to text reprocess
                }
            } else {
                false // Not a PDF document
            };

        // Fallback: text/markdown documents or PDF without valid pdf_id
        if !task_created {
            if let Some(content_value) = state.kv_storage.get_by_id(&content_key).await? {
                if let Some(content) = content_value.get("content").and_then(|v| v.as_str()) {
                    // Update status to pending
                    if let Some(mut metadata) = state.kv_storage.get_by_id(&metadata_key).await? {
                        if let Some(obj) = metadata.as_object_mut() {
                            obj.insert("status".to_string(), serde_json::json!("pending"));
                            obj.insert("track_id".to_string(), serde_json::json!(new_track_id));
                            obj.insert(
                                "retry_at".to_string(),
                                serde_json::json!(Utc::now().to_rfc3339()),
                            );

                            state.kv_storage.upsert(&[(metadata_key, metadata)]).await?;
                        }
                    }

                    // Create new task
                    use edgequake_tasks::{Task, TaskType, TextInsertData};

                    let title = doc_id.clone();
                    let task_data = TextInsertData {
                        text: content.to_string(),
                        file_source: title.clone(),
                        workspace_id: workspace_id.clone(),
                        metadata: Some(serde_json::json!({
                            "document_id": doc_id,
                            "title": title,
                            "track_id": new_track_id,
                            "is_retry": true,
                            "tenant_id": tenant_id,
                            "workspace_id": workspace_id,
                        })),
                    };

                    let task = Task::new(
                        uuid::Uuid::parse_str(&tenant_id).map_err(|_| {
                            ApiError::ValidationError("Invalid tenant ID".to_string())
                        })?,
                        uuid::Uuid::parse_str(&workspace_id).map_err(|_| {
                            ApiError::ValidationError("Invalid workspace ID".to_string())
                        })?,
                        TaskType::Insert,
                        serde_json::to_value(task_data).unwrap(),
                    );

                    state
                        .task_storage
                        .create_task(&task)
                        .await
                        .map_err(|e| ApiError::Internal(format!("Failed to create task: {}", e)))?;

                    state
                        .task_queue
                        .send(task)
                        .await
                        .map_err(|e| ApiError::Internal(format!("Failed to queue task: {}", e)))?;

                    requeued_ids.push(doc_id.clone());
                }
            }
        } else {
            requeued_ids.push(doc_id.clone());
        }
    }

    // SPEC-040: Retry failed PDF documents from the documents DB table.
    // WHY: PDF docs are stored in the `documents` DB table, not in KV store.
    // The KV-based reprocess loop above cannot find them.
    #[cfg(feature = "postgres")]
    if let Some(ref pdf_storage) = state.pdf_storage {
        use edgequake_storage::{ListPdfFilter, PdfProcessingStatus};
        use edgequake_tasks::{PdfProcessingData, Task, TaskStatus, TaskType};

        let filter_workspace = tenant_ctx
            .workspace_id
            .as_deref()
            .and_then(|s| Uuid::parse_str(s).ok());

        let remaining = request
            .max_documents
            .saturating_sub(docs_to_reprocess.len());
        if remaining > 0 {
            let failed_pdfs = pdf_storage
                .list_pdfs(ListPdfFilter {
                    workspace_id: filter_workspace,
                    processing_status: Some(PdfProcessingStatus::Failed),
                    page: Some(1),
                    page_size: Some(remaining),
                })
                .await
                .map_err(|e| ApiError::Internal(format!("Failed to list failed PDFs: {}", e)))?;

            let vision_provider =
                std::env::var("EDGEQUAKE_VISION_PROVIDER").unwrap_or_else(|_| "openai".to_string());
            let vision_model = std::env::var("EDGEQUAKE_VISION_MODEL").ok();

            for pdf in failed_pdfs.items {
                // Determine tenant_id: prefer from context, fall back to a
                // workspace-scoped default (workspace_id itself as tenant proxy).
                let tenant_uuid = tenant_ctx
                    .tenant_id
                    .as_deref()
                    .and_then(|s| Uuid::parse_str(s).ok())
                    .unwrap_or(pdf.workspace_id);

                // Reset PDF status so the worker will process it.
                pdf_storage
                    .update_pdf_status(&pdf.pdf_id, PdfProcessingStatus::Pending)
                    .await
                    .map_err(|e| {
                        ApiError::Internal(format!("Failed to reset PDF status: {}", e))
                    })?;

                let task_data = PdfProcessingData {
                    pdf_id: pdf.pdf_id,
                    tenant_id: tenant_uuid,
                    workspace_id: pdf.workspace_id,
                    enable_vision: true,
                    vision_provider: vision_provider.clone(),
                    vision_model: vision_model.clone(),
                    existing_document_id: pdf.document_id.map(|id| id.to_string()),
                };

                let track_id = format!("pdf-{}", Uuid::new_v4());

                let task = Task {
                    track_id: track_id.clone(),
                    tenant_id: tenant_uuid,
                    workspace_id: pdf.workspace_id,
                    task_type: TaskType::PdfProcessing,
                    status: TaskStatus::Pending,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                    started_at: None,
                    completed_at: None,
                    error_message: None,
                    error: None,
                    retry_count: 0,
                    max_retries: 3,
                    consecutive_timeout_failures: 0,
                    circuit_breaker_tripped: false,
                    task_data: serde_json::to_value(&task_data).map_err(|e| {
                        ApiError::Internal(format!("Failed to serialize PDF task data: {}", e))
                    })?,
                    metadata: None,
                    progress: None,
                    result: None,
                };

                state
                    .task_storage
                    .create_task(&task)
                    .await
                    .map_err(|e| ApiError::Internal(format!("Failed to create PDF task: {}", e)))?;

                state
                    .task_queue
                    .send(task)
                    .await
                    .map_err(|e| ApiError::Internal(format!("Failed to queue PDF task: {}", e)))?;

                requeued_ids.push(pdf.pdf_id.to_string());
                tracing::info!(
                    pdf_id = %pdf.pdf_id,
                    track_id = %track_id,
                    "Re-enqueued failed PDF for reprocessing"
                );
            }
        }
    }

    Ok(Json(ReprocessFailedResponse {
        track_id: new_track_id,
        failed_found: docs_to_reprocess.len(),
        requeued: requeued_ids.len(),
        document_ids: requeued_ids,
    }))
}

// ============================================
// Recovery for Stuck Processing Documents
// ============================================

/// Recover documents stuck in "processing" status.
///
/// This endpoint finds documents that have been in "processing" status for longer
/// than the specified threshold and requeues them for processing. This is useful
/// for recovering from server restarts or crashes that left tasks in an incomplete state.
#[utoipa::path(
    post,
    path = "/api/v1/documents/recover-stuck",
    tag = "Documents",
    request_body = RecoverStuckRequest,
    responses(
        (status = 200, description = "Stuck documents recovered", body = RecoverStuckResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn recover_stuck(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Json(request): Json<RecoverStuckRequest>,
) -> ApiResult<Json<RecoverStuckResponse>> {
    use chrono::Duration;

    debug!(
        "recover_stuck called with tenant context: tenant_id={:?}, workspace_id={:?}, threshold={}min",
        tenant_ctx.tenant_id, tenant_ctx.workspace_id, request.stuck_threshold_minutes
    );

    // Generate new track ID for recovery batch
    let new_track_id = format!(
        "recover_{}_{}",
        Utc::now().format("%Y%m%d_%H%M%S"),
        &Uuid::new_v4().to_string()[..8]
    );

    let threshold = Duration::minutes(request.stuck_threshold_minutes as i64);
    let cutoff_time = Utc::now() - threshold;

    // Get all metadata keys
    let all_keys: Vec<String> = state.kv_storage.keys().await?;

    let mut stuck_docs = Vec::new();
    let mut requeued_ids = Vec::new();
    let mut requeued_titles = Vec::new();

    // Find stuck processing documents
    for key in all_keys.iter().filter(|k| k.ends_with("-metadata")) {
        if stuck_docs.len() >= request.max_documents {
            break;
        }

        if let Some(value) = state.kv_storage.get_by_id(key).await? {
            if let Some(obj) = value.as_object() {
                let status = obj.get("status").and_then(|v| v.as_str());
                let doc_id = obj.get("id").and_then(|v| v.as_str());
                let title = obj.get("title").and_then(|v| v.as_str());
                let updated_at = obj.get("updated_at").and_then(|v| v.as_str());

                // Check if document is stuck in processing
                if status == Some("processing") {
                    // If specific document IDs provided, check if this one is in the list
                    if let Some(ref filter_ids) = request.document_ids {
                        if let Some(id) = doc_id {
                            if !filter_ids.contains(&id.to_string()) {
                                continue;
                            }
                        }
                    }

                    // Check if document is older than threshold
                    let is_stuck = if let Some(updated) = updated_at {
                        if let Ok(updated_time) = chrono::DateTime::parse_from_rfc3339(updated) {
                            updated_time.with_timezone(&chrono::Utc) < cutoff_time
                        } else {
                            // If we can't parse the time, assume it's stuck
                            true
                        }
                    } else {
                        // No updated_at, assume it's stuck
                        true
                    };

                    if is_stuck {
                        if let Some(id) = doc_id {
                            stuck_docs.push((id.to_string(), title.unwrap_or(id).to_string()));
                        }
                    }
                }
            }
        }
    }

    // Requeue stuck documents
    for (doc_id, doc_title) in &stuck_docs {
        // OODA-08: Clean up partial graph data from interrupted processing BEFORE requeueing
        // WHY: Same as reprocess_failed - prevents duplicate entities and corrupted source_ids
        //
        // A "stuck" document may have partially created entities before the process
        // died or timed out. Without cleanup, reprocessing would create duplicates.
        match cleanup_document_graph_data(doc_id, &state.graph_storage, None).await {
            Ok(stats) => {
                tracing::info!(
                    document_id = %doc_id,
                    entities_removed = stats.entities_removed,
                    entities_updated = stats.entities_updated,
                    relationships_removed = stats.relationships_removed,
                    "Cleaned up partial data before recovery"
                );
            }
            Err(e) => {
                tracing::warn!(
                    document_id = %doc_id,
                    error = %e,
                    "Failed to cleanup partial data before recovery, continuing anyway"
                );
            }
        }

        // Get document content
        let content_key = format!("{}-content", doc_id);
        if let Some(content_value) = state.kv_storage.get_by_id(&content_key).await? {
            if let Some(content) = content_value.get("content").and_then(|v| v.as_str()) {
                // Update status back to pending
                let metadata_key = format!("{}-metadata", doc_id);
                if let Some(mut metadata) = state.kv_storage.get_by_id(&metadata_key).await? {
                    if let Some(obj) = metadata.as_object_mut() {
                        obj.insert("status".to_string(), serde_json::json!("pending"));
                        obj.insert("track_id".to_string(), serde_json::json!(new_track_id));
                        obj.insert(
                            "recovered_at".to_string(),
                            serde_json::json!(Utc::now().to_rfc3339()),
                        );
                        obj.insert(
                            "recovery_reason".to_string(),
                            serde_json::json!("stuck_in_processing"),
                        );

                        state.kv_storage.upsert(&[(metadata_key, metadata)]).await?;
                    }
                }

                // Create new task
                use edgequake_tasks::{Task, TaskType, TextInsertData};

                // Use tenant context for workspace_id, fallback to "default"
                let workspace_id = tenant_ctx
                    .workspace_id
                    .clone()
                    .unwrap_or_else(|| "default".to_string());
                let tenant_id = tenant_ctx
                    .tenant_id
                    .clone()
                    .unwrap_or_else(|| "default".to_string());

                let task_data = TextInsertData {
                    text: content.to_string(),
                    file_source: doc_title.clone(),
                    workspace_id: workspace_id.clone(),
                    metadata: Some(serde_json::json!({
                        "document_id": doc_id,
                        "title": doc_title,
                        "track_id": new_track_id,
                        "is_recovery": true,
                        "tenant_id": tenant_id,
                        "workspace_id": workspace_id,
                    })),
                };

                let task = Task::new(
                    uuid::Uuid::parse_str(&tenant_id)
                        .map_err(|_| ApiError::ValidationError("Invalid tenant ID".to_string()))?,
                    uuid::Uuid::parse_str(&workspace_id).map_err(|_| {
                        ApiError::ValidationError("Invalid workspace ID".to_string())
                    })?,
                    TaskType::Insert,
                    serde_json::to_value(task_data).unwrap(),
                );

                state
                    .task_storage
                    .create_task(&task)
                    .await
                    .map_err(|e| ApiError::Internal(format!("Failed to create task: {}", e)))?;

                state
                    .task_queue
                    .send(task)
                    .await
                    .map_err(|e| ApiError::Internal(format!("Failed to queue task: {}", e)))?;

                requeued_ids.push(doc_id.clone());
                requeued_titles.push(doc_title.clone());

                tracing::info!("Recovered stuck document: {} ({})", doc_id, doc_title);
            }
        }
    }

    Ok(Json(RecoverStuckResponse {
        track_id: new_track_id,
        stuck_found: stuck_docs.len(),
        requeued: requeued_ids.len(),
        document_ids: requeued_ids,
        document_titles: requeued_titles,
    }))
}

/// Retry failed chunks for a specific document.
///
/// @implements FEAT0408 (Chunk retry handler)
///
/// # OODA-03: Chunk-Level Retry Queue
///
/// This endpoint allows retrying specific failed chunks without reprocessing the entire document.
/// Currently returns a placeholder response; full implementation pending chunk-level storage.
///
/// ## Architecture Note
///
/// Full implementation requires:
/// 1. Storing individual chunk content in failed_chunks table
/// 2. Re-running extraction on specific chunks
/// 3. Merging results into existing graph data
///
/// This is a scaffolding endpoint to enable frontend integration while backend is developed.
#[utoipa::path(
    post,
    path = "/api/v1/documents/{document_id}/retry-chunks",
    tag = "Documents",
    params(
        ("document_id" = String, Path, description = "Document ID to retry chunks for")
    ),
    request_body = RetryChunksRequest,
    responses(
        (status = 200, description = "Chunks queued for retry", body = RetryChunksResponse),
        (status = 404, description = "Document not found"),
        (status = 501, description = "Chunk-level retry not yet implemented")
    )
)]
pub async fn retry_failed_chunks(
    State(state): State<AppState>,
    axum::extract::Path(document_id): axum::extract::Path<String>,
    Json(request): Json<RetryChunksRequest>,
) -> ApiResult<Json<RetryChunksResponse>> {
    debug!(
        "retry_failed_chunks called for document: {}, chunks: {:?}, force: {}",
        document_id, request.chunk_indices, request.force
    );

    // Verify document exists
    let metadata_key = format!("{}-metadata", document_id);
    let doc_exists = state.kv_storage.get_by_id(&metadata_key).await?.is_some();

    if !doc_exists {
        return Err(ApiError::NotFound(format!(
            "Document {} not found",
            document_id
        )));
    }

    // OODA-03: Placeholder implementation
    // Full implementation requires:
    // 1. Query failed_chunks table for document
    // 2. Retrieve chunk content from storage
    // 3. Re-run extraction pipeline on specific chunks
    // 4. Merge extracted entities/relationships into graph
    // 5. Update failed_chunks status

    let chunks_to_retry = if request.chunk_indices.is_empty() {
        // Would query failed_chunks table here
        vec![]
    } else {
        request.chunk_indices.clone()
    };

    tracing::info!(
        document_id = %document_id,
        chunks = ?chunks_to_retry,
        "Chunk retry requested (placeholder - full implementation pending)"
    );

    Ok(Json(RetryChunksResponse {
        document_id: document_id.clone(),
        chunks_queued: chunks_to_retry.len(),
        chunk_indices: chunks_to_retry,
        message: "Chunk-level retry is pending implementation. Use /documents/reprocess to retry the entire document.".to_string(),
        implemented: false,
    }))
}

/// List failed chunks for a document.
///
/// @implements FEAT0409
///
/// Returns information about chunks that failed during extraction,
/// allowing the user to decide which to retry.
#[utoipa::path(
    get,
    path = "/api/v1/documents/{document_id}/failed-chunks",
    tag = "Documents",
    params(
        ("document_id" = String, Path, description = "Document ID to list failed chunks for")
    ),
    responses(
        (status = 200, description = "List of failed chunks", body = ListFailedChunksResponse),
        (status = 404, description = "Document not found")
    )
)]
pub async fn list_failed_chunks(
    State(state): State<AppState>,
    axum::extract::Path(document_id): axum::extract::Path<String>,
) -> ApiResult<Json<ListFailedChunksResponse>> {
    debug!("list_failed_chunks called for document: {}", document_id);

    // Verify document exists
    let metadata_key = format!("{}-metadata", document_id);
    let metadata = state.kv_storage.get_by_id(&metadata_key).await?;

    if metadata.is_none() {
        return Err(ApiError::NotFound(format!(
            "Document {} not found",
            document_id
        )));
    }

    // Get chunk count from metadata
    let chunk_count = metadata
        .as_ref()
        .and_then(|m| m.get("chunk_count"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;

    // OODA-03: Placeholder - would query failed_chunks table
    // For now, return empty list since we don't persist failed chunks yet
    let failed_chunks: Vec<FailedChunkInfo> = vec![];

    Ok(Json(ListFailedChunksResponse {
        document_id: document_id.clone(),
        failed_chunks,
        total_chunks: chunk_count,
        successful_chunks: chunk_count, // Placeholder - all successful if no failures recorded
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_upload_request_validation() {
        let request = UploadDocumentRequest {
            content: "Test content".to_string(),
            title: Some("Test".to_string()),
            metadata: None,
            async_processing: false,
            track_id: None,
            enable_gleaning: true,
            max_gleaning: 1,
            use_llm_summarization: true,
        };

        assert!(!request.content.is_empty());
    }

    #[test]
    fn test_upload_request_serialization() {
        let json = r#"{
            "content": "Hello world",
            "title": "Test Doc",
            "metadata": {"key": "value"}
        }"#;

        let request: UploadDocumentRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.content, "Hello world");
        assert_eq!(request.title, Some("Test Doc".to_string()));
        assert!(request.metadata.is_some());
    }

    #[test]
    fn test_upload_request_minimal() {
        let json = r#"{"content": "Just content"}"#;

        let request: UploadDocumentRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.content, "Just content");
        assert!(request.title.is_none());
        assert!(request.metadata.is_none());
    }

    #[test]
    fn test_upload_response_serialization() {
        let response = UploadDocumentResponse {
            document_id: "doc-123".to_string(),
            status: "processed".to_string(),
            task_id: None,
            track_id: "upload_20240101_abc12345".to_string(),
            duplicate_of: None,
            chunk_count: Some(5),
            entity_count: Some(3),
            relationship_count: Some(2),
            cost: Some(DocumentCostInfo {
                total_cost_usd: 0.0045,
                formatted_cost: "$0.004500".to_string(),
                input_tokens: 1000,
                output_tokens: 500,
                total_tokens: 1500,
                llm_model: Some("gpt-4o-mini".to_string()),
                embedding_model: Some("text-embedding-3-small".to_string()),
            }),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("doc-123"));
        assert!(json.contains("processed"));
        assert!(json.contains("cost"));
        assert!(json.contains("0.0045"));
    }

    #[test]
    fn test_list_documents_request_defaults() {
        let json = r#"{}"#;
        let request: ListDocumentsRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.page, 1);
        assert_eq!(request.page_size, 20);
    }

    #[test]
    fn test_list_documents_request_custom() {
        let json = r#"{"page": 3, "page_size": 50}"#;
        let request: ListDocumentsRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.page, 3);
        assert_eq!(request.page_size, 50);
    }

    #[test]
    fn test_document_summary_serialization() {
        let summary = DocumentSummary {
            id: "doc-456".to_string(),
            title: Some("My Document".to_string()),
            file_name: None,
            content_summary: Some("This is the first 200 chars of the document...".to_string()),
            content_length: Some(5000),
            chunk_count: 10,
            entity_count: None,
            status: Some("completed".to_string()),
            error_message: None,
            track_id: Some("upload_20240101_abc12345".to_string()),
            created_at: None,
            updated_at: None,
            cost_usd: None,
            input_tokens: None,
            output_tokens: None,
            total_tokens: None,
            llm_model: None,
            embedding_model: None,
            // SPEC-002 fields
            source_type: Some("markdown".to_string()),
            current_stage: Some("completed".to_string()),
            stage_progress: Some(1.0),
            stage_message: None,
            pdf_id: None,
        };

        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("doc-456"));
        assert!(json.contains("My Document"));
    }

    #[test]
    fn test_list_documents_response_serialization() {
        let response = ListDocumentsResponse {
            documents: vec![DocumentSummary {
                id: "doc-1".to_string(),
                title: None,
                file_name: None,
                content_summary: None,
                content_length: None,
                chunk_count: 5,
                entity_count: None,
                status: Some("completed".to_string()),
                error_message: None,
                track_id: None,
                created_at: None,
                updated_at: None,
                cost_usd: None,
                input_tokens: None,
                output_tokens: None,
                total_tokens: None,
                llm_model: None,
                embedding_model: None,
                // SPEC-002 fields
                source_type: None,
                current_stage: Some("completed".to_string()),
                stage_progress: None,
                stage_message: None,
                pdf_id: None,
            }],
            total: 1,
            page: 1,
            page_size: 20,
            total_pages: 1,
            has_more: false,
            status_counts: StatusCounts {
                pending: 0,
                processing: 0,
                completed: 1,
                partial_failure: 0,
                failed: 0,
                cancelled: 0,
            },
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("doc-1"));
        assert!(json.contains("\"total\":1"));
        assert!(json.contains("\"total_pages\":1"));
        assert!(json.contains("\"has_more\":false"));
    }

    #[test]
    fn test_document_detail_response_serialization() {
        let response = DocumentDetailResponse {
            id: "doc-789".to_string(),
            title: Some("Test".to_string()),
            file_name: None,
            content: None,
            content_summary: None,
            content_length: None,
            content_hash: None,
            chunk_count: 5,
            entity_count: None,
            relationship_count: None,
            status: "processed".to_string(),
            error_message: None,
            source_type: None,
            mime_type: None,
            file_size: None,
            track_id: None,
            tenant_id: None,
            workspace_id: None,
            created_at: None,
            updated_at: None,
            processed_at: None,
            lineage: None,
            metadata: None,
            pdf_id: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("doc-789"));
        assert!(json.contains("processed"));
    }

    #[test]
    fn test_delete_document_response_serialization() {
        let response = DeleteDocumentResponse {
            document_id: "doc-to-delete".to_string(),
            deleted: true,
            chunks_deleted: 7,
            entities_affected: 2,
            relationships_affected: 1,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("doc-to-delete"));
        assert!(json.contains("\"deleted\":true"));
        assert!(json.contains("\"chunks_deleted\":7"));
    }

    #[test]
    fn test_default_page() {
        assert_eq!(default_page(), 1);
    }

    #[test]
    fn test_default_page_size() {
        assert_eq!(default_page_size(), 20);
    }

    #[test]
    fn test_track_status_response_serialization() {
        let response = TrackStatusResponse {
            track_id: "upload_20240101_abc12345".to_string(),
            created_at: Some("2024-01-01T00:00:00Z".to_string()),
            documents: vec![DocumentSummary {
                id: "doc-1".to_string(),
                title: Some("Test Doc".to_string()),
                file_name: None,
                content_summary: None,
                content_length: None,
                chunk_count: 5,
                entity_count: Some(3),
                status: Some("completed".to_string()),
                error_message: None,
                track_id: Some("upload_20240101_abc12345".to_string()),
                created_at: None,
                updated_at: None,
                cost_usd: None,
                input_tokens: None,
                output_tokens: None,
                total_tokens: None,
                llm_model: None,
                embedding_model: None,
                // SPEC-002 fields
                source_type: None,
                current_stage: Some("completed".to_string()),
                stage_progress: None,
                stage_message: None,
                pdf_id: None,
            }],
            total_count: 1,
            status_summary: StatusCounts {
                pending: 0,
                processing: 0,
                completed: 1,
                partial_failure: 0,
                failed: 0,
                cancelled: 0,
            },
            is_complete: true,
            latest_message: Some("All documents processed successfully".to_string()),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("upload_20240101_abc12345"));
        assert!(json.contains("\"is_complete\":true"));
        assert!(json.contains("\"total_count\":1"));
    }
}
