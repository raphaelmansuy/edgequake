//! Document storage helper functions.
//!
//! Private utilities used by upload, delete, and recovery sub-modules.
//! Includes workspace vector storage resolution, graph cleanup,
//! and re-ingestion support.

use std::sync::Arc;
use tracing::{debug, warn};
use uuid::Uuid;

use crate::error::ApiError;
use crate::middleware::TenantContext;
use crate::state::AppState;
use edgequake_storage::traits::VectorStorage;
use edgequake_tasks::{Pagination, TaskFilter, TaskStatus};

/// Parse a workspace identifier into a UUID while preserving the legacy
/// `default` workspace mapping used by older document metadata.
///
/// WHY: reliability fixes must work for both new UUID-based workspaces and the
/// historical `workspace_id = "default"` representation.
pub(crate) fn parse_workspace_uuid_or_default(workspace_id: Option<&str>) -> Option<Uuid> {
    crate::middleware::resolve_workspace_uuid(workspace_id)
}

fn is_legacy_default_workspace_context(workspace_id: Option<&str>) -> bool {
    match workspace_id.map(str::trim) {
        None | Some("") | Some("default") => true,
        Some(value) => match Uuid::parse_str(value) {
            Ok(uuid) => {
                uuid == crate::middleware::default_tenant_uuid()
                    || uuid == crate::middleware::default_workspace_uuid()
            }
            Err(_) => false,
        },
    }
}

fn parse_explicit_workspace_uuid(workspace_id: Option<&str>) -> Option<Uuid> {
    match workspace_id.map(str::trim) {
        None | Some("") | Some("default") => None,
        Some(value) => Uuid::parse_str(value).ok(),
    }
}

/// Check whether a metadata payload belongs to the requester's workspace.
///
/// WHY: destructive and recovery operations must fail closed across workspaces.
/// When no tenant context is present (tests or legacy callers), we preserve the
/// old permissive behavior to avoid breaking non-multi-tenant flows.
pub(crate) fn metadata_matches_tenant_context(
    metadata: &serde_json::Value,
    tenant_ctx: &TenantContext,
) -> bool {
    // WHY: Older documents stored `workspace_id = "default"` (or omitted the
    // field entirely) before strict workspace scoping existed. Those rows must
    // remain operable from the default workspace for backward compatibility,
    // but they must NOT become visible to arbitrary explicit workspaces.
    let stored_workspace_raw = metadata
        .get("workspace_id")
        .and_then(|value| value.as_str())
        .map(str::trim);

    if matches!(stored_workspace_raw, None | Some("") | Some("default")) {
        return is_legacy_default_workspace_context(tenant_ctx.workspace_id.as_deref());
    }

    let Some(ctx_workspace_id) =
        parse_workspace_uuid_or_default(tenant_ctx.workspace_id.as_deref())
    else {
        return true;
    };

    let Some(stored_workspace_id) = parse_workspace_uuid_or_default(stored_workspace_raw) else {
        return false;
    };

    stored_workspace_id == ctx_workspace_id
}

fn task_references_document(task: &edgequake_tasks::Task, document_id: &str) -> bool {
    task.task_data
        .get("existing_document_id")
        .and_then(|v| v.as_str())
        == Some(document_id)
        || task.task_data.get("document_id").and_then(|v| v.as_str()) == Some(document_id)
        || task
            .task_data
            .get("metadata")
            .and_then(|v| v.get("document_id"))
            .and_then(|v| v.as_str())
            == Some(document_id)
}

async fn cancel_and_delete_task(state: &AppState, task: &edgequake_tasks::Task) -> bool {
    if matches!(task.status, TaskStatus::Pending | TaskStatus::Processing) {
        let cancelled = state.cancellation_registry.cancel(&task.track_id).await;
        tracing::info!(
            track_id = %task.track_id,
            cancelled,
            "Cancelled in-flight task during lifecycle cleanup"
        );
    }

    state
        .pipeline_state
        .remove_pdf_progress(&task.track_id)
        .await;

    if let Ok(Some(mut persisted_task)) = state.task_storage.get_task(&task.track_id).await {
        persisted_task.mark_cancelled();
        let _ = state.task_storage.update_task(&persisted_task).await;
    }

    state.task_storage.delete_task(&task.track_id).await.is_ok()
}

/// Remove persisted tasks associated with a single document.
///
/// WHY: deleting document data without deleting the persisted task row allows
/// startup recovery to resurrect work that the user already removed.
pub(crate) async fn purge_persisted_tasks_for_document(
    state: &AppState,
    document_id: &str,
    track_id_opt: Option<&str>,
    workspace_id_opt: Option<&str>,
) -> usize {
    let pagination = Pagination {
        page: 1,
        page_size: 10_000,
        ..Default::default()
    };
    let filter = TaskFilter {
        workspace_id: parse_explicit_workspace_uuid(workspace_id_opt),
        ..Default::default()
    };

    let Ok(task_list) = state.task_storage.list_tasks(filter, pagination).await else {
        return 0;
    };

    let mut deleted_count = 0usize;

    for task in task_list.tasks {
        let matches_track = track_id_opt
            .map(|track_id| task.track_id == track_id)
            .unwrap_or(false);
        if !matches_track && !task_references_document(&task, document_id) {
            continue;
        }

        if cancel_and_delete_task(state, &task).await {
            deleted_count += 1;
        }
    }

    deleted_count
}

/// Remove all persisted tasks belonging to a workspace.
///
/// WHY: workspace deletion must clear background jobs first so a restart cannot
/// repopulate rows, progress entries, or graph data for a workspace that is gone.
pub(crate) async fn purge_workspace_tasks(state: &AppState, workspace_id: Uuid) -> usize {
    let pagination = Pagination {
        page: 1,
        page_size: 10_000,
        ..Default::default()
    };
    let filter = TaskFilter {
        workspace_id: Some(workspace_id),
        ..Default::default()
    };

    let Ok(task_list) = state.task_storage.list_tasks(filter, pagination).await else {
        return 0;
    };

    let mut deleted = 0usize;
    for task in task_list.tasks {
        if cancel_and_delete_task(state, &task).await {
            deleted += 1;
        }
    }

    deleted
}

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
pub(super) async fn get_workspace_vector_storage_strict(
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
pub(super) async fn get_workspace_vector_storage_with_fallback(
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

/// Get workspace-specific vector storage for **deletion** operations.
///
/// This variant is intentionally lenient about missing workspaces.  During
/// document deletion the primary goals are:
///   1. Remove KV entries (content, chunks, metadata, hash key).
///   2. Remove graph nodes / edges whose only source is this document.
///   3. Remove the corresponding rows from the PostgreSQL `documents` table.
///   4. Best-effort: delete chunk embeddings from the vector index.
///
/// If the workspace record no longer exists in the database (e.g., it was
/// deleted, or this is a legacy "default" workspace without a DB row), we
/// MUST NOT block the entire delete.  Instead we degrade gracefully:
///
/// - Return the **default** vector storage so embedding cleanup is still
///   attempted against the global index.
/// - Log a `WARN` so operators can find orphaned vector rows later.
///
/// # WHY NOT STRICT
///
/// Using `get_workspace_vector_storage_strict` for deletion created a
/// permanent "zombie document" trap:
///   - User uploads document → processing fails → document stuck in "failed"
///   - Workspace deleted or default workspace has no DB row
///   - `get_workspace_vector_storage_strict` returns NotFound
///   - Delete API returns 404 / 500 → document is undeleteable forever
///
/// This is worse than potentially orphaning a few vector rows. The correct
/// trade-off is: always allow deletion, degrade vector cleanup gracefully.
pub(super) async fn get_workspace_vector_storage_for_delete(
    state: &AppState,
    workspace_id: &str,
) -> Arc<dyn VectorStorage> {
    match get_workspace_vector_storage_strict(state, workspace_id).await {
        Ok(storage) => storage,
        Err(e) => {
            warn!(
                workspace_id = %workspace_id,
                error = %e,
                "Workspace not found or vector storage unavailable during document deletion. \
                 Proceeding with default storage. Orphaned vector rows (if any) can be \
                 cleaned up later via the vector storage maintenance API."
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

/// Extract source document IDs from node/edge properties.
///
/// Handles two formats for backward compatibility:
/// - `source_ids`: JSON array of strings (current format)
/// - `source_id`: Pipe-separated string (legacy format)
pub(super) fn extract_source_docs(
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
pub(crate) async fn cleanup_document_graph_data(
    document_id: &str,
    graph_storage: &Arc<dyn edgequake_storage::traits::GraphStorage>,
    vector_storage: Option<&Arc<dyn VectorStorage>>,
) -> Result<CleanupStats, ApiError> {
    let mut stats = CleanupStats::default();

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
pub(super) async fn delete_document_for_reingestion(
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
    // - "partial_failure" → "deleting" (FIX-5: processed but 0 entities)
    // - "processed" → "deleting" (legacy status, same as completed)
    // - "cancelled" → "deleting" (user cancelled, wants to retry)
    //
    // Disallowed (return conflict):
    // - "pending" → (still waiting for processing)
    // - "processing" → (active ingestion in progress)
    // - "deleting" → (another delete already in progress)

    // Try each allowed terminal status in order
    let allowed_from_statuses = [
        "failed",
        "completed",
        "partial_failure",
        "processed",
        "cancelled",
    ];
    let mut transitioned = false;
    for from_status in &allowed_from_statuses {
        match state
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
        // None of the allowed transitions worked - document state prevents re-ingestion
        tracing::warn!(
            document_id = %document_id,
            metadata_key = %metadata_key,
            "Cannot re-ingest: document status prevents transition (processing/pending/deleting/not found)"
        );
        return Ok(false);
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
    let chunk_prefix = format!("{}-chunk-", document_id);
    let chunk_ids = state.kv_storage.keys_with_prefix(&chunk_prefix).await?;

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
