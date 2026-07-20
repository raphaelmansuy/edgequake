//! Single document deletion handler.
//!
//! Cascade-deletes a document: KV entries, chunk embeddings, graph entities,
//! graph edges, and content-hash duplicate-detection key (OADA-90).
//!
//! @implements SPEC-050: Real-time deletion progress via WebSocket broadcast.

use axum::{extract::State, Json};
use uuid::Uuid;

use edgequake_audit::{AuditEventType, AuditResult};

use crate::error::{ApiError, ApiResult};
use crate::handlers::documents_types::*;
use crate::handlers::websocket_types::DeletionPhaseKind;
use crate::middleware::TenantContext;
use crate::services::{
    cascade_remove_document_sources, record_compliance_event, CascadeStats, ContentHasher,
    DocumentSourceScope,
};
use crate::state::AppState;
use edgequake_core::MetricsTriggerType;

use crate::document_read_model::relational_document_scope;
use crate::services::document_metadata_scan::{
    canonical_document_id, document_id_from_metadata_key, load_all_document_metadata_entries,
    metadata_key_for_document,
};

use super::super::storage_helpers::{
    get_workspace_vector_storage_for_delete, metadata_matches_tenant_context,
    purge_persisted_tasks_for_document,
};

/// Resolve the actual KV key prefix for a document.
///
/// WHY: The `list_documents` endpoint shows documents by their JSON `id` field
/// inside KV metadata values, but the KV key is `{early_doc_id}-metadata`.
/// These can diverge due to historical bugs (interrupted retries, backend restarts
/// with older code). When the user requests deletion by JSON `id`, we must find
/// the real KV key prefix to delete all associated keys (chunks, content, metadata).
///
/// Returns `(actual_key_prefix, metadata_key, has_metadata)`.
///
/// P-G7 (RC-12): the slow path uses `keys_with_suffix("-metadata")` (an
/// index-friendly scan in Postgres) instead of requiring the caller to pass
/// the full key list. The fast path is a direct O(1) `get_by_id` on the
/// expected `{document_id}-metadata` key.
async fn resolve_kv_key_prefix(document_id: &str, state: &AppState) -> (String, String, bool) {
    // Fast path: direct key lookup — key prefix == document_id
    let direct_metadata_key = metadata_key_for_document(document_id);
    if state
        .storage
        .kv_storage
        .get_by_id(&direct_metadata_key)
        .await
        .ok()
        .flatten()
        .is_some()
    {
        return (document_id.to_string(), direct_metadata_key, true);
    }

    // Slow path: batch metadata entries (index-friendly suffix scan).
    // SPEC-045: match canonical id (KV key wins over JSON `id`).
    if let Ok(entries) = load_all_document_metadata_entries(state.storage.kv_storage.as_ref()).await
    {
        for (key, val) in entries {
            let canonical = canonical_document_id(&key, &val);
            let legacy_json_id = val.get("id").and_then(|v| v.as_str());
            if canonical == document_id || legacy_json_id == Some(document_id) {
                let prefix =
                    document_id_from_metadata_key(&key).unwrap_or_else(|| document_id.to_string());
                return (prefix, key, true);
            }
        }
    }

    // Neither direct key nor JSON id match — return document_id as-is
    (document_id.to_string(), direct_metadata_key, false)
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
    tenant_ctx: TenantContext,
) -> ApiResult<Json<DeleteDocumentResponse>> {
    // P-G7 (RC-12): no upfront O(W) full-key scan. The metadata key is resolved
    // via `resolve_kv_key_prefix` (fast O(1) direct lookup, slow path uses an
    // index-friendly suffix scan). Chunk/content keys are fetched with an
    // index-friendly prefix scan scoped to the resolved document prefix.
    let (actual_key_prefix, metadata_key, has_metadata) =
        resolve_kv_key_prefix(&document_id, &state).await;
    let key_id_mismatch = actual_key_prefix != document_id;

    if key_id_mismatch {
        tracing::warn!(
            document_id = %document_id,
            actual_key_prefix = %actual_key_prefix,
            "KV key/id mismatch detected — key prefix differs from metadata JSON id. \
             Using resolved key prefix for cascade delete."
        );
    }

    // Find chunks belonging to this document (index-friendly prefix scan).
    let chunk_prefix = format!("{}-chunk-", actual_key_prefix);
    let chunk_ids: Vec<String> = state
        .storage
        .kv_storage
        .keys_with_prefix(&chunk_prefix)
        .await
        .unwrap_or_default();

    // Also check for content key (using resolved key prefix)
    let content_key = format!("{}-content", actual_key_prefix);
    let has_content = state
        .storage
        .kv_storage
        .get_by_id(&content_key)
        .await
        .ok()
        .flatten()
        .is_some();

    // First principle: if the list shows the document (relational read model), delete
    // must succeed even when KV metadata/chunks/content are absent.
    #[cfg(feature = "postgres")]
    let relational_scope =
        relational_document_scope(state.pg_pool.as_ref(), &document_id, &tenant_ctx).await?;
    #[cfg(not(feature = "postgres"))]
    let relational_scope: Option<crate::document_read_model::RelationalDocumentScope> = None;

    let kv_present = !chunk_ids.is_empty() || has_metadata || has_content;
    if !kv_present && relational_scope.is_none() {
        return Err(ApiError::NotFound(format!(
            "Document {} not found",
            document_id
        )));
    }

    if kv_present && relational_scope.is_none() && has_metadata {
        if let Ok(Some(metadata)) = state.storage.kv_storage.get_by_id(&metadata_key).await {
            if !metadata_matches_tenant_context(&metadata, &tenant_ctx) {
                return Err(ApiError::NotFound(format!(
                    "Document {} not found",
                    document_id
                )));
            }
        }
    }

    // SPEC-033: Get workspace_id from document metadata for vector storage isolation
    // OADA-90: Extract content_hash for hash key cleanup
    // FIX-ISSUE-73: Extract pdf_id for pdf_documents cleanup
    let (workspace_id_for_storage, document_status, content_hash_opt, _pdf_id_opt, track_id_opt) =
        if has_metadata {
            if let Ok(Some(metadata)) = state.storage.kv_storage.get_by_id(&metadata_key).await {
                let tenant_ok = metadata_matches_tenant_context(&metadata, &tenant_ctx);
                if tenant_ok || relational_scope.is_some() {
                    let workspace = metadata
                        .get("workspace_id")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| {
                            relational_scope
                                .as_ref()
                                .map(|s| s.workspace_id.clone())
                                .unwrap_or_else(|| "default".to_string())
                        });
                    let status = metadata
                        .get("status")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| {
                            relational_scope
                                .as_ref()
                                .map(|s| s.status.clone())
                                .unwrap_or_else(|| "unknown".to_string())
                        });
                    // OADA-90: Extract content hash for duplicate detection key cleanup
                    let content_hash = metadata
                        .get("content_hash")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    // FIX-ISSUE-73: Extract pdf_id for pdf_documents cascade cleanup
                    let pdf_id = metadata
                        .get("pdf_id")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    // Extract track_id so we can cancel any in-flight processing task
                    let track_id = metadata
                        .get("track_id")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .or_else(|| relational_scope.as_ref().and_then(|s| s.track_id.clone()));
                    (workspace, status, content_hash, pdf_id, track_id)
                } else {
                    return Err(ApiError::NotFound(format!(
                        "Document {} not found",
                        document_id
                    )));
                }
            } else if let Some(scope) = relational_scope.clone() {
                (scope.workspace_id, scope.status, None, None, scope.track_id)
            } else {
                (
                    "default".to_string(),
                    "unknown".to_string(),
                    None,
                    None,
                    None,
                )
            }
        } else if let Some(scope) = relational_scope.clone() {
            (scope.workspace_id, scope.status, None, None, scope.track_id)
        } else {
            (
                "default".to_string(),
                "unknown".to_string(),
                None,
                None,
                None,
            )
        };

    // SPEC-050: Generate a transient track_id for this deletion operation.
    // WHY: Allows WebSocket clients to correlate DeletionStarted / DeletionPhase /
    // DeletionCompleted events to the specific delete request they triggered.
    let deletion_track_id = Uuid::new_v4().to_string();

    // Early admit: write status=deleting BEFORE expensive cascade so list polls
    // during vectors/graph/kv removal show an honest "Deleting" badge (not stale
    // Completed). Same spirit as reprocess early-admit cleaning.
    if has_metadata {
        if let Ok(Some(mut metadata)) = state.storage.kv_storage.get_by_id(&metadata_key).await {
            if let Some(obj) = metadata.as_object_mut() {
                obj.insert("status".to_string(), serde_json::json!("deleting"));
                obj.insert("current_stage".to_string(), serde_json::json!("deleting"));
                obj.insert(
                    "stage_message".to_string(),
                    serde_json::json!("Removing document data…"),
                );
                obj.insert("stage_progress".to_string(), serde_json::json!(0.0));
                for key in [
                    "entity_count",
                    "entities_count",
                    "relationship_count",
                    "relationships_count",
                    "total_cost",
                    "cost_usd",
                ] {
                    if obj.contains_key(key) {
                        obj.insert(key.to_string(), serde_json::json!(0));
                    }
                }
                let _ = crate::services::upsert_metadata_kv_with_index(
                    state.storage.kv_storage.as_ref(),
                    &metadata_key,
                    metadata,
                )
                .await;
            }
        }
    }

    // SPEC-050: Broadcast deletion started so the frontend can show "Deleting…"
    state
        .tasks
        .progress_broadcaster
        .deletion_started(&document_id, &deletion_track_id);

    // First Principle: a user must always be able to delete their own document.
    //
    // If the document is still being processed (pending/processing), cancel its
    // task first so the processor stops writing — then proceed with the cascade
    // delete.  The processor already handles missing-document cases gracefully
    // (update_document_status is a no-op when the KV key is gone), so there is
    // no orphan risk from the race window.
    //
    // SRP: the delete handler's only job is data removal; lifecycle management
    // belongs to the processor.  Blocking deletion here was the wrong layer.
    if matches!(
        document_status.as_str(),
        "pending" | "processing" | "deleting"
    ) {
        match &track_id_opt {
            Some(track_id) => {
                state.tasks.progress_broadcaster.deletion_phase(
                    &document_id,
                    &deletion_track_id,
                    DeletionPhaseKind::CancellingTask,
                    0,
                    1,
                );
                let cancelled = state.tasks.cancellation_registry.cancel(track_id).await;
                tracing::info!(
                    document_id = %document_id,
                    track_id = %track_id,
                    status = %document_status,
                    cancelled,
                    "Cancelled in-flight task before cascade delete"
                );
            }
            None => {
                tracing::warn!(
                    document_id = %document_id,
                    status = %document_status,
                    "No track_id in metadata — falling back to persisted task scan during delete"
                );
            }
        }
    }

    let persisted_tasks_removed = purge_persisted_tasks_for_document(
        &state,
        &document_id,
        track_id_opt.as_deref(),
        Some(&workspace_id_for_storage),
    )
    .await;

    // SPEC-028: Collect chunk IDs for vector storage deletion
    // Clone chunk_ids before workspace_vector_storage operations
    let keys_to_delete_for_vectors: Vec<String> = chunk_ids.clone();

    // SPEC-033: Get workspace-specific vector storage for deletion.
    // WHY-OODA223: Use lenient resolver so that documents whose workspace
    // record no longer exists in the DB are not permanently stuck ("zombie"
    // documents).  Orphaned vector rows from the missing workspace are an
    // acceptable trade-off vs. an undeleteable document.
    let workspace_vector_storage =
        get_workspace_vector_storage_for_delete(&state, &workspace_id_for_storage).await;

    let chunks_deleted = chunk_ids.len();
    let mut embeddings_deleted = 0usize;
    let mut partial_failure = false;
    let mut partial_failure_reason: Option<String> = None;

    // SPEC-028: Delete chunk embeddings from vector storage first
    // WHY: Chunks are stored with IDs like "doc-xxx-chunk-0", delete them
    let chunk_embedding_ids: Vec<String> = keys_to_delete_for_vectors.clone();

    // SPEC-050: Broadcast RemovingVectors phase.
    state.tasks.progress_broadcaster.deletion_phase(
        &document_id,
        &deletion_track_id,
        DeletionPhaseKind::RemovingVectors,
        0,
        chunk_embedding_ids.len() as u32,
    );

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

    // SPEC-006 P1: bounded document-scoped cascade (no get_all_nodes/edges)
    let scope =
        DocumentSourceScope::with_key_prefix(document_id.clone(), actual_key_prefix.clone());

    // SPEC-050: Broadcast RemovingGraph phase.
    state.tasks.progress_broadcaster.deletion_phase(
        &document_id,
        &deletion_track_id,
        DeletionPhaseKind::RemovingGraph,
        0,
        0,
    );

    // WHY non-fatal: the graph cascade cleans up derivative entities/edges. A
    // graph hiccup (AGE Cypher error, transient connection drop, lazy-label
    // table not yet created) must NOT block the user from deleting their
    // document or turn a successful deletion into a 500. KV/vector/relational
    // cleanup below is the authoritative data removal; graph rows are best-effort
    // and can be reconciled later. This fixes the "delete returns 500 even though
    // the document is gone" symptom seen on large ingestions.
    //
    // WHY tenant_ctx = None (#305): document ownership was already verified via
    // metadata_matches_tenant_context / relational_scope above. Passing the raw
    // request tenant filter made cascade miss AGE nodes whose workspace_id is
    // the canonical UUID while the header still says `"default"` (or nodes that
    // lack tenant properties entirely). Source-prefix scope is document-unique.
    let cascade_stats = match cascade_remove_document_sources(
        &state.storage.graph_storage,
        Some(&workspace_vector_storage),
        None,
        &scope,
    )
    .await
    {
        Ok(stats) => stats,
        Err(e) => {
            tracing::error!(
                document_id = %document_id,
                error = %e,
                "Graph cascade delete failed (non-fatal) — proceeding with KV/vector/relational cleanup"
            );
            // SPEC-050: Track partial failure for response and completion event.
            partial_failure = true;
            partial_failure_reason = Some(format!("Graph cascade error: {}", e));
            CascadeStats::default()
        }
    };
    let entities_removed = cascade_stats.entities_removed;
    let entities_updated = cascade_stats.entities_updated;
    let relationships_removed = cascade_stats.relationships_removed;
    let relationships_updated = cascade_stats.relationships_updated;
    embeddings_deleted += cascade_stats.embeddings_deleted;

    // Collect all keys to delete from KV storage
    let mut keys_to_delete = keys_to_delete_for_vectors;
    if has_metadata {
        keys_to_delete.push(metadata_key);
        keys_to_delete.push(edgequake_storage::kv_keys::workspace_doc_index(
            &workspace_id_for_storage,
            &actual_key_prefix,
        ));
    }
    if has_content {
        keys_to_delete.push(content_key);
    }

    // Collect any other KV keys with the document prefix that aren't already
    // in the list (e.g., `-lineage` keys). This ensures comprehensive cleanup.
    // P-G7 (RC-12): index-friendly prefix scan instead of filtering the full
    // key set in-memory.
    let actual_doc_prefix = format!("{}-", actual_key_prefix);
    let all_prefix_keys: Vec<String> = state
        .storage
        .kv_storage
        .keys_with_prefix(&actual_doc_prefix)
        .await
        .unwrap_or_default()
        .into_iter()
        .filter(|k| !keys_to_delete.contains(k))
        .collect();
    if !all_prefix_keys.is_empty() {
        tracing::debug!(
            count = all_prefix_keys.len(),
            document_id = %document_id,
            "Collecting additional KV keys with document prefix"
        );
        keys_to_delete.extend(all_prefix_keys);
    }

    // In mismatch cases, also collect keys under the JSON id prefix
    if key_id_mismatch {
        let json_doc_prefix = format!("{}-", document_id);
        let alt_prefix_keys: Vec<String> = state
            .storage
            .kv_storage
            .keys_with_prefix(&json_doc_prefix)
            .await
            .unwrap_or_default()
            .into_iter()
            .filter(|k| !keys_to_delete.contains(k))
            .collect();
        if !alt_prefix_keys.is_empty() {
            tracing::debug!(
                count = alt_prefix_keys.len(),
                json_id = %document_id,
                "Collecting additional KV keys with JSON id prefix (mismatch cleanup)"
            );
            keys_to_delete.extend(alt_prefix_keys);
        }
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

    // SPEC-050: Broadcast RemovingKv phase before KV deletion.
    state.tasks.progress_broadcaster.deletion_phase(
        &document_id,
        &deletion_track_id,
        DeletionPhaseKind::RemovingKv,
        0,
        keys_to_delete.len() as u32,
    );

    // Delete all document data from KV storage
    state.storage.kv_storage.delete(&keys_to_delete).await?;

    // SPEC-047: explicit mm-asset cleanup (DB + FS). FK CASCADE also removes DB rows
    // when the documents row is deleted; this covers memory mode and FS orphans.
    #[cfg(feature = "postgres")]
    {
        let mm_storage = state.storage.mm_asset_storage.as_deref();
        let workspace_uuid = uuid::Uuid::parse_str(&workspace_id_for_storage).ok();
        match crate::services::delete_document_mm_assets(mm_storage, &document_id, workspace_uuid)
            .await
        {
            Ok(n) => {
                if n > 0 {
                    tracing::debug!(
                        document_id = %document_id,
                        deleted = n,
                        "Deleted document mm-assets"
                    );
                }
            }
            Err(e) => {
                tracing::warn!(
                    document_id = %document_id,
                    error = %e,
                    "Failed to delete document mm-assets (continuing cascade)"
                );
            }
        }
        // Also try actual_key_prefix when it differs (key/id mismatch).
        if key_id_mismatch {
            let _ = crate::services::delete_document_mm_assets(
                mm_storage,
                &actual_key_prefix,
                workspace_uuid,
            )
            .await;
        }
    }

    // FIX-ISSUE-73: Cascade delete pdf_documents, chunks, and the documents row.
    // WHY: Previously only KV/graph/vector data was cleaned up, leaving orphaned rows
    // in pdf_documents, chunks, and documents tables (GitHub Issue #73).
    #[cfg(feature = "postgres")]
    {
        if let Some(ref pdf_storage) = state.storage.pdf_storage {
            // 1. Delete from pdf_documents if this is a PDF document
            if let Some(ref pid) = _pdf_id_opt {
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

            // 2. Delete from documents relational table (cascades to chunks via FK).
            // WHY: Try actual_key_prefix first (true KV key prefix), then document_id
            // (JSON id) if they differ, to handle key/id mismatch cases.
            let doc_ids_to_try: Vec<&str> = if key_id_mismatch {
                vec![&actual_key_prefix, &document_id]
            } else {
                vec![&document_id]
            };
            for doc_id_str in &doc_ids_to_try {
                if let Ok(doc_uuid) = Uuid::parse_str(doc_id_str) {
                    match pdf_storage.delete_document_record(&doc_uuid).await {
                        Ok(_) => {
                            tracing::debug!(
                                document_id = %doc_id_str,
                                "Deleted documents table row (cascaded to chunks)"
                            );
                            break;
                        }
                        Err(e) => {
                            tracing::warn!(
                                document_id = %doc_id_str,
                                error = %e,
                                "Failed to delete documents table row (may not exist)"
                            );
                        }
                    }
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
        persisted_tasks_removed = persisted_tasks_removed,
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

    let tenant_for_audit = tenant_ctx
        .tenant_id
        .clone()
        .unwrap_or_else(|| "default".to_string());
    record_compliance_event(
        &state,
        tenant_for_audit,
        AuditEventType::Authorization,
        "delete_document",
        AuditResult::Success,
        tenant_ctx.workspace_id.clone(),
        tenant_ctx.user_id.clone(),
        Some(("document".to_string(), document_id.clone())),
    );

    // SPEC-050: Finalizing phase before completion broadcast.
    state.tasks.progress_broadcaster.deletion_phase(
        &document_id,
        &deletion_track_id,
        DeletionPhaseKind::Finalizing,
        0,
        1,
    );

    // SPEC-050: Broadcast DeletionCompleted so the frontend can show final stats.
    state.tasks.progress_broadcaster.deletion_completed(
        &document_id,
        &deletion_track_id,
        chunks_deleted,
        entities_removed,
        relationships_removed,
        embeddings_deleted,
        partial_failure,
        partial_failure_reason.clone(),
    );

    Ok(Json(DeleteDocumentResponse {
        document_id,
        deleted: true,
        chunks_deleted,
        entities_affected: entities_removed + entities_updated,
        relationships_affected: relationships_removed + relationships_updated,
        embeddings_deleted,
        partial_failure,
        partial_failure_reason,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// resolve_kv_key_prefix: fast path — key prefix matches document_id.
    #[tokio::test]
    async fn test_resolve_key_prefix_fast_path() {
        let state = AppState::test_state();
        let doc_id = "aaaa-bbbb-cccc-dddd";
        let metadata_key = metadata_key_for_document(doc_id);

        // Store metadata with matching key and id
        state
            .storage
            .kv_storage
            .upsert(&[(
                metadata_key.clone(),
                json!({"id": doc_id, "status": "completed"}),
            )])
            .await
            .unwrap();

        let (prefix, key, has_metadata) = resolve_kv_key_prefix(doc_id, &state).await;

        assert_eq!(prefix, doc_id);
        assert_eq!(key, metadata_key);
        assert!(has_metadata);
    }

    /// resolve_kv_key_prefix: slow path — key prefix differs from JSON id.
    #[tokio::test]
    async fn test_resolve_key_prefix_mismatch() {
        let state = AppState::test_state();
        let kv_prefix = "real-key-prefix-1111";
        let json_id = "mismatched-json-id-2222";
        let metadata_key = metadata_key_for_document(kv_prefix);

        // Store metadata with MISMATCHED key/id
        state
            .storage
            .kv_storage
            .upsert(&[(
                metadata_key.clone(),
                json!({"id": json_id, "status": "failed", "title": "Test Document"}),
            )])
            .await
            .unwrap();

        let (prefix, key, has_metadata) = resolve_kv_key_prefix(json_id, &state).await;

        // Should resolve to the KV key prefix, not the JSON id
        assert_eq!(prefix, kv_prefix);
        assert_eq!(key, metadata_key);
        assert!(has_metadata);
    }

    /// resolve_kv_key_prefix: document not found at all.
    #[tokio::test]
    async fn test_resolve_key_prefix_not_found() {
        let state = AppState::test_state();
        let doc_id = "nonexistent-doc-9999";

        let (prefix, key, has_metadata) = resolve_kv_key_prefix(doc_id, &state).await;

        assert_eq!(prefix, doc_id);
        assert_eq!(key, metadata_key_for_document(doc_id));
        assert!(!has_metadata);
    }

    /// Delete succeeds when KV key prefix differs from metadata JSON id.
    /// This is the exact bug scenario: list_documents returns a document
    /// with JSON id "B" but KV key "A-metadata". DELETE /documents/B must
    /// find and delete the KV entries under the "A-*" keys.
    #[tokio::test]
    async fn test_delete_document_with_key_id_mismatch() {
        let state = AppState::test_state();
        let kv_prefix = "4b788a9e-0000-0000-0000-000000000001";
        let json_id = "2cddf543-0000-0000-0000-000000000002";

        // Set up the mismatch scenario: metadata key uses kv_prefix,
        // but the JSON id field is json_id.
        let metadata_key = metadata_key_for_document(kv_prefix);
        let content_key = format!("{}-content", kv_prefix);
        let chunk_0_key = format!("{}-chunk-0", kv_prefix);
        let chunk_1_key = format!("{}-chunk-1", kv_prefix);

        state
            .storage
            .kv_storage
            .upsert(&[
                (
                    metadata_key.clone(),
                    json!({
                        "id": json_id,
                        "status": "failed",
                        "title": "Orphaned Doc",
                        "workspace_id": "default",
                        "error_message": "Orphaned during backend restart"
                    }),
                ),
                (content_key.clone(), json!({"text": "Some content"})),
                (chunk_0_key.clone(), json!({"text": "Chunk 0"})),
                (chunk_1_key.clone(), json!({"text": "Chunk 1"})),
            ])
            .await
            .unwrap();

        // Verify all 4 keys exist
        let keys_before = state.storage.kv_storage.keys().await.unwrap();
        assert!(keys_before.contains(&metadata_key));
        assert!(keys_before.contains(&content_key));
        assert!(keys_before.contains(&chunk_0_key));
        assert!(keys_before.contains(&chunk_1_key));

        // Delete using the JSON id (what the frontend sends)
        let result = delete_document(
            State(state.clone()),
            axum::extract::Path(json_id.to_string()),
            TenantContext::default(),
        )
        .await;

        // Should succeed, not return 404
        let response = result.expect("delete should succeed for mismatched key/id document");
        assert!(response.deleted);
        assert_eq!(response.chunks_deleted, 2);

        // Verify all keys were deleted
        let keys_after = state.storage.kv_storage.keys().await.unwrap();
        assert!(
            !keys_after.contains(&metadata_key),
            "metadata should be deleted"
        );
        assert!(
            !keys_after.contains(&content_key),
            "content should be deleted"
        );
        assert!(
            !keys_after.contains(&chunk_0_key),
            "chunk 0 should be deleted"
        );
        assert!(
            !keys_after.contains(&chunk_1_key),
            "chunk 1 should be deleted"
        );
    }

    /// Delete still returns 404 when no matching document exists at all.
    #[tokio::test]
    async fn test_delete_truly_nonexistent_returns_404() {
        let state = AppState::test_state();

        let result = delete_document(
            State(state.clone()),
            axum::extract::Path("nonexistent-id-0000".to_string()),
            TenantContext::default(),
        )
        .await;

        assert!(
            result.is_err(),
            "delete of truly nonexistent doc should return error"
        );
    }

    /// Comprehensive cleanup: lineage keys and keys under both prefixes
    /// are deleted in a mismatch scenario.
    #[tokio::test]
    async fn test_delete_mismatch_cleans_lineage_and_alt_prefix_keys() {
        let state = AppState::test_state();
        let kv_prefix = "aaaa-0000-0000-0000-000000000001";
        let json_id = "bbbb-0000-0000-0000-000000000002";

        let metadata_key = metadata_key_for_document(kv_prefix);
        let lineage_key = format!("{}-lineage", kv_prefix);
        // A key under the JSON id prefix (e.g., lineage stored there)
        let alt_lineage_key = format!("{}-lineage", json_id);

        state
            .storage
            .kv_storage
            .upsert(&[
                (
                    metadata_key.clone(),
                    json!({
                        "id": json_id,
                        "status": "failed",
                        "workspace_id": "default"
                    }),
                ),
                (lineage_key.clone(), json!({"chunks": []})),
                (alt_lineage_key.clone(), json!({"chunks": []})),
            ])
            .await
            .unwrap();

        // Delete using the JSON id
        let result = delete_document(
            State(state.clone()),
            axum::extract::Path(json_id.to_string()),
            TenantContext::default(),
        )
        .await;

        let response = result.expect("delete should succeed");
        assert!(response.deleted);

        // ALL keys under BOTH prefixes must be cleaned
        let keys_after = state.storage.kv_storage.keys().await.unwrap();
        assert!(!keys_after.contains(&metadata_key), "metadata");
        assert!(
            !keys_after.contains(&lineage_key),
            "lineage under kv prefix"
        );
        assert!(
            !keys_after.contains(&alt_lineage_key),
            "lineage under json id prefix"
        );
    }

    /// #305: DELETE must cascade-remove exclusive Knowledge Graph entities.
    #[tokio::test]
    async fn test_delete_document_removes_exclusive_graph_entities() {
        use edgequake_storage::traits::GraphStorageMutateOps;
        use std::collections::HashMap;

        let state = AppState::test_state();
        let doc_id = "doc-305-delete-cascade";
        let metadata_key = metadata_key_for_document(doc_id);
        let chunk_key = format!("{}-chunk-0", doc_id);

        state
            .storage
            .kv_storage
            .upsert(&[
                (
                    metadata_key.clone(),
                    json!({
                        "id": doc_id,
                        "status": "completed",
                        "workspace_id": "default",
                        "tenant_id": "default",
                    }),
                ),
                (chunk_key.clone(), json!({"content": "chunk"})),
            ])
            .await
            .unwrap();

        let mut props = HashMap::new();
        props.insert(
            "source_ids".to_string(),
            json!([format!("{}-chunk-0", doc_id)]),
        );
        props.insert("entity_type".to_string(), json!("PERSON"));
        // Store canonical UUID while request may use `"default"` — cascade must
        // still find and remove the node.
        props.insert(
            "workspace_id".to_string(),
            json!(crate::middleware::default_workspace_uuid().to_string()),
        );
        state
            .storage
            .graph_storage
            .upsert_node("EXCLUSIVE_ENTITY_305", props)
            .await
            .unwrap();

        let result = delete_document(
            State(state.clone()),
            axum::extract::Path(doc_id.to_string()),
            TenantContext {
                tenant_id: Some("default".to_string()),
                workspace_id: Some("default".to_string()),
                user_id: None,
            },
        )
        .await
        .expect("delete should succeed");

        assert!(result.deleted);
        assert!(
            result.entities_affected >= 1,
            "exclusive entity must be cascade-removed, got entities_affected={}",
            result.entities_affected
        );
        assert!(
            !state
                .storage
                .graph_storage
                .has_node("EXCLUSIVE_ENTITY_305")
                .await
                .unwrap(),
            "Knowledge Graph must not retain entities from deleted documents (#305)"
        );
    }
}
