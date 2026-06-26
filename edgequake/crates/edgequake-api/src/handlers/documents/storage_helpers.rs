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

/// Check whether a metadata payload belongs to the requester's tenant + workspace.
pub(crate) fn metadata_matches_tenant_context(
    metadata: &serde_json::Value,
    tenant_ctx: &TenantContext,
) -> bool {
    crate::workspace_scope::metadata_matches_tenant_context(metadata, tenant_ctx)
}

fn parse_explicit_workspace_uuid(workspace_id: Option<&str>) -> Option<Uuid> {
    match workspace_id.map(str::trim) {
        None | Some("") | Some("default") => None,
        Some(value) => Uuid::parse_str(value).ok(),
    }
}

/// Attach tenant/workspace scope to graph node or edge properties (BR0201).
///
/// WHY: Workspace stats (`node_count_by_workspace`) filter AGE nodes by
/// `workspace_id`. Sync text upload must set this on every graph write.
pub(crate) fn insert_graph_tenant_scope(
    properties: &mut std::collections::HashMap<String, serde_json::Value>,
    tenant_id: &Option<String>,
    workspace_id: &str,
) {
    if let Some(ref tid) = tenant_id {
        properties.insert("tenant_id".to_string(), serde_json::json!(tid));
    }
    properties.insert("workspace_id".to_string(), serde_json::json!(workspace_id));
}

#[cfg(test)]
mod graph_scope_tests {
    use super::insert_graph_tenant_scope;

    #[test]
    fn insert_graph_tenant_scope_sets_workspace_and_tenant() {
        let mut props = std::collections::HashMap::new();
        insert_graph_tenant_scope(&mut props, &Some("tenant-abc".to_string()), "workspace-xyz");
        assert_eq!(
            props.get("workspace_id").and_then(|v| v.as_str()),
            Some("workspace-xyz")
        );
        assert_eq!(
            props.get("tenant_id").and_then(|v| v.as_str()),
            Some("tenant-abc")
        );
    }
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
        let cancelled = state
            .tasks
            .cancellation_registry
            .cancel(&task.track_id)
            .await;
        tracing::info!(
            track_id = %task.track_id,
            cancelled,
            "Cancelled in-flight task during lifecycle cleanup"
        );
    }

    state
        .tasks
        .pipeline_state
        .remove_pdf_progress(&task.track_id)
        .await;

    if let Ok(Some(mut persisted_task)) = state.tasks.storage.get_task(&task.track_id).await {
        persisted_task.mark_cancelled();
        let _ = state.tasks.storage.update_task(&persisted_task).await;
    }

    state
        .tasks
        .storage
        .delete_task(&task.track_id)
        .await
        .is_ok()
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

    let Ok(task_list) = state.tasks.storage.list_tasks(filter, pagination).await else {
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

    let Ok(task_list) = state.tasks.storage.list_tasks(filter, pagination).await else {
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
    let allow_fallback = state.storage.mode.is_memory();

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
                    storage_mode = ?state.storage.mode,
                    "Invalid workspace ID - using default storage (allowed in memory/test mode)"
                );
                return Ok(state.storage.vector_registry.default_storage());
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
                    storage_mode = ?state.storage.mode,
                    "Workspace not found - using default storage (allowed in memory/test mode)"
                );
                return Ok(state.storage.vector_registry.default_storage());
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
                    storage_mode = ?state.storage.mode,
                    "Failed to lookup workspace - using default storage (allowed in memory/test mode)"
                );
                return Ok(state.storage.vector_registry.default_storage());
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
    match state.storage.vector_registry.get_or_create(config).await {
        Ok(storage) => Ok(storage),
        Err(e) => {
            if allow_fallback {
                // WHY-OODA223: Test mode - log warning and use default storage
                tracing::warn!(
                    workspace_id = %workspace_id,
                    dimension = workspace.embedding_dimension,
                    error = %e,
                    storage_mode = ?state.storage.mode,
                    "Failed to create workspace storage - using default (allowed in memory/test mode)"
                );
                return Ok(state.storage.vector_registry.default_storage());
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
            state.storage.vector_registry.default_storage()
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
            state.storage.vector_registry.default_storage()
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
#[allow(dead_code)] // retained for legacy handler migration; cascade uses `collect_source_references`
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
    let scope = crate::services::DocumentSourceScope::from_document_id(document_id);
    let cascade_stats = crate::services::cascade_remove_document_sources(
        graph_storage,
        vector_storage,
        None,
        &scope,
    )
    .await?;

    tracing::info!(
        document_id = %document_id,
        entities_removed = cascade_stats.entities_removed,
        entities_updated = cascade_stats.entities_updated,
        relationships_removed = cascade_stats.relationships_removed,
        relationships_updated = cascade_stats.relationships_updated,
        embeddings_deleted = cascade_stats.embeddings_deleted,
        "Document graph data cleanup completed"
    );

    Ok(CleanupStats {
        entities_removed: cascade_stats.entities_removed,
        entities_updated: cascade_stats.entities_updated,
        relationships_removed: cascade_stats.relationships_removed,
        relationships_updated: cascade_stats.relationships_updated,
        embeddings_deleted: cascade_stats.embeddings_deleted,
    })
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
        &state.storage.graph_storage,
        Some(&workspace_vector_storage),
    )
    .await?;

    // Delete chunk embeddings from vector storage
    let chunk_prefix = format!("{}-chunk-", document_id);
    let chunk_ids = state
        .storage
        .kv_storage
        .keys_with_prefix(&chunk_prefix)
        .await?;

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
    state.storage.kv_storage.delete(&keys_to_delete).await?;

    tracing::info!(
        document_id = %document_id,
        chunks_deleted = keys_to_delete.len(),
        entities_removed = cleanup_stats.entities_removed,
        relationships_removed = cleanup_stats.relationships_removed,
        "Document data deleted for re-ingestion"
    );

    Ok(true)
}

/// Clear cached PDF markdown + KV content/chunks so a full re-conversion runs.
///
/// @implements PDF re-conversion on Replace / Reprocess (Full mode).
///
/// WHY (DRY): Both the duplicate-dialog Replace path (`force_reindex`) and the
/// Reprocess handler (`mode=full`) need to invalidate the cached PDF->markdown
/// conversion before queueing a task with `restart_from_scratch=true`. Without
/// this, the `pdf_processing` resume shortcut reuses `pdf_documents.markdown_content`
/// and the PDF is never re-converted — exactly the "no re-conversion" bug.
///
/// This helper is best-effort for non-fatal stores: KV chunk/content deletion
/// failures are logged but do not abort, because the worker's own
/// `restart_from_scratch=true` path also clears these keys. The PDF markdown
/// clear is the authoritative signal that flips the resume shortcut off.
///
/// # Arguments
///
/// * `state` - Application state
/// * `document_id` - KV document ID whose content/chunks should be cleared
/// * `pdf_id` - PDF row whose `markdown_content` should be NULLed
///
/// # Returns
///
/// * `Ok(())` - Cleanup attempted (failures logged as warnings)
/// * `Err(ApiError)` - Only if KV key listing itself fails
pub(crate) async fn clear_document_markdown_and_content(
    state: &AppState,
    document_id: &str,
    pdf_id: &Uuid,
) -> Result<(), ApiError> {
    // 1. Clear cached markdown in the PDF row so the resume shortcut cannot
    //    reuse a stale conversion. This is the authoritative re-conversion
    //    signal for the worker.
    #[cfg(feature = "postgres")]
    if let Some(ref pdf_storage) = state.storage.pdf_storage {
        if let Err(e) = pdf_storage.clear_markdown(pdf_id).await {
            tracing::warn!(
                pdf_id = %pdf_id,
                document_id = %document_id,
                error = %e,
                "Failed to clear cached PDF markdown before re-conversion"
            );
        }
    }
    let _ = pdf_id; // unused when postgres feature is off

    // 2. Best-effort: clear KV content + chunk keys so stale text is not
    //    served from KV during the re-conversion window.
    // P-G7 (RC-12): index-friendly prefix scan for this document's keys
    // instead of scanning every key in the workspace. The subset is then
    // filtered in-memory to the content/chunk keys (a tiny set per doc).
    let doc_prefix = format!("{}-", document_id);
    let keys: Vec<String> = state
        .storage
        .kv_storage
        .keys_with_prefix(&doc_prefix)
        .await?;
    let chunk_prefix = format!("{}-chunk-", document_id);
    let keys_to_delete: Vec<String> = keys
        .iter()
        .filter(|k| k.ends_with("-content") || k.starts_with(&chunk_prefix))
        .cloned()
        .collect();

    if !keys_to_delete.is_empty() {
        if let Err(e) = state.storage.kv_storage.delete(&keys_to_delete).await {
            tracing::warn!(
                document_id = %document_id,
                error = %e,
                "Failed to clear KV content/chunks before re-conversion"
            );
        }
    }

    Ok(())
}

/// Decide whether a PDF must be fully re-converted because there is no usable
/// cached markdown to reuse.
///
/// WHY (DRY + SOLID): The Reprocess handler exposes two intents — `Full`
/// (re-convert from PDF) and `EntitiesOnly` (reuse cached markdown). When the
/// user picks `EntitiesOnly` but the cached markdown is missing or empty,
/// there is nothing to reuse and entity extraction would run over an empty
/// document. This pure predicate centralizes that "empty markdown" check so
/// both the KV-PDF branch and the failed-PDF branch of the reprocess handler
/// apply identical fallback semantics and stay testable without a live store.
///
/// Returns `true` when `markdown` is `None` or trims to an empty string.
pub(crate) fn pdf_needs_full_reconversion(markdown: Option<&str>) -> bool {
    markdown.map_or(true, |md| md.trim().is_empty())
}

#[cfg(test)]
mod reconvert_tests {
    use super::pdf_needs_full_reconversion;

    #[test]
    fn none_markdown_requires_full() {
        assert!(pdf_needs_full_reconversion(None));
    }

    #[test]
    fn empty_markdown_requires_full() {
        assert!(pdf_needs_full_reconversion(Some("")));
        assert!(pdf_needs_full_reconversion(Some("   \n\t ")));
    }

    #[test]
    fn non_empty_markdown_reuses_entities() {
        assert!(!pdf_needs_full_reconversion(Some("# Hello\nworld")));
        assert!(!pdf_needs_full_reconversion(Some("x")));
    }
}
