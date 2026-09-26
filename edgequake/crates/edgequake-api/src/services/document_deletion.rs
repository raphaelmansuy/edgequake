//! Authoritative document cascade delete (DRY for handler worker + bulk).
//!
//! First principles:
//! - HTTP admits `status=deleting` and returns 202; this service runs async.
//! - Order: graph discover/mutate/post-proof → vectors → KV/PDF/relational.
//! - Never wipe metadata before graph provenance is proved absent.
//! - Fail closed with `delete_failed` status (never leave permanent `deleting`).
//! - **List-surface completeness**: after success (or already-absent), every
//!   store that feeds `GET /documents` (KV metadata, `wsdoc:` index, SQL
//!   `documents`) must be empty for that identity — otherwise merge re-injects
//!   "ghost" rows on refresh.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[cfg(feature = "postgres")]
use sha2::{Digest as _, Sha256};
use uuid::Uuid;

use edgequake_audit::{AuditEventType, AuditResult};
use edgequake_core::MetricsTriggerType;
use edgequake_storage::kv_keys;
use edgequake_tasks::DeletionTaskData;

use crate::error::{ApiError, ApiResult};
use crate::handlers::websocket_types::DeletionPhaseKind;
use crate::middleware::TenantContext;
use crate::services::document_graph_cascade::{find_document_edges, find_document_nodes};
use crate::services::document_metadata_scan::metadata_key_for_document;
use crate::services::document_task_cleanup::purge_persisted_tasks_for_document_except;
use crate::services::document_vector_storage::get_workspace_vector_storage_for_delete;
use crate::services::{
    cascade_remove_document_sources_with_progress, record_compliance_event, ContentHasher,
    DocumentSourceScope,
};
use crate::state::AppState;

const DELETION_CHECKPOINT_KEY: &str = "deletion_checkpoint";
const CHECKPOINT_GRAPH_DONE: &str = "graph_done";
const CHECKPOINT_VECTORS_DONE: &str = "vectors_done";

async fn read_deletion_checkpoint(state: &AppState, metadata_key: &str) -> Option<String> {
    let meta = state
        .storage
        .kv_storage
        .get_by_id(metadata_key)
        .await
        .ok()
        .flatten()?;
    meta.get(DELETION_CHECKPOINT_KEY)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

async fn write_deletion_checkpoint(
    state: &AppState,
    metadata_key: &str,
    has_metadata: bool,
    phase: &str,
) {
    if !has_metadata {
        return;
    }
    if let Ok(Some(mut metadata)) = state.storage.kv_storage.get_by_id(metadata_key).await {
        if let Some(obj) = metadata.as_object_mut() {
            obj.insert(
                DELETION_CHECKPOINT_KEY.to_string(),
                serde_json::json!(phase),
            );
            let _ = crate::services::upsert_metadata_kv_with_index(
                state.storage.kv_storage.as_ref(),
                metadata_key,
                metadata,
            )
            .await;
        }
    }
}

/// Diagnostic unscoped probe: source-owned rows exist but scoped discovery is empty.
async fn scoped_discovery_has_filter_mismatch(
    state: &AppState,
    tenant_ctx: &TenantContext,
    scope: &DocumentSourceScope,
) -> ApiResult<bool> {
    let scoped_nodes =
        find_document_nodes(&state.storage.graph_storage, Some(tenant_ctx), scope).await?;
    let scoped_edges =
        find_document_edges(&state.storage.graph_storage, Some(tenant_ctx), scope).await?;
    if !scoped_nodes.is_empty() || !scoped_edges.is_empty() {
        return Ok(false);
    }
    let unscoped_nodes = find_document_nodes(&state.storage.graph_storage, None, scope).await?;
    let unscoped_edges = find_document_edges(&state.storage.graph_storage, None, scope).await?;
    Ok(!unscoped_nodes.is_empty() || !unscoped_edges.is_empty())
}

async fn post_proof_source_absent(
    state: &AppState,
    tenant_ctx: &TenantContext,
    scope: &DocumentSourceScope,
) -> ApiResult<()> {
    let nodes = find_document_nodes(&state.storage.graph_storage, Some(tenant_ctx), scope).await?;
    let edges = find_document_edges(&state.storage.graph_storage, Some(tenant_ctx), scope).await?;
    if nodes.is_empty() && edges.is_empty() {
        return Ok(());
    }
    Err(ApiError::Internal(format!(
        "Post-proof failed: {} nodes and {} edges still reference document sources",
        nodes.len(),
        edges.len()
    )))
}

/// Result of a completed cascade delete.
#[derive(Debug, Clone, Default)]
pub struct DocumentDeletionResult {
    pub chunks_deleted: usize,
    pub entities_removed: usize,
    pub entities_updated: usize,
    pub relationships_removed: usize,
    pub relationships_updated: usize,
    pub embeddings_deleted: usize,
    pub persisted_tasks_removed: usize,
    pub partial_failure: bool,
    pub partial_failure_reason: Option<String>,
}

/// Result of list-surface purge (idempotent orphan / final cleanup).
#[derive(Debug, Clone, Default)]
pub struct ListSurfacePurgeResult {
    pub kv_keys_deleted: usize,
    pub relational_rows_deleted: u64,
}

/// Options for [`purge_document_list_surfaces`].
#[derive(Debug, Clone, Default)]
pub struct ListSurfacePurgeOpts<'a> {
    /// Alternate KV key prefix when it differs from `document_id`.
    pub key_prefix: Option<&'a str>,
    pub content_hash: Option<&'a str>,
    pub pdf_id: Option<&'a str>,
}

/// Purge every store that can re-surface a document on `GET /documents`.
///
/// # Invariant (SSOT)
/// After this returns `Ok`, the identity must not appear via:
/// - `{id}-metadata` / `{prefix}-metadata` (final + staging)
/// - `{id}-content` / `{prefix}-content`
/// - `wsdoc:{workspace}:{id}` and `wsdoc:{workspace}:{prefix}`
/// - workspace content-hash pointer (when hash known)
/// - relational `documents` row (workspace-scoped)
///
/// Idempotent: missing keys/rows are success. Graph / vector / chunk prefix
/// cleanup remains the responsibility of the full cascade; this is the
/// **list-surface** contract shared by cascade completion, batch already-absent,
/// re-ingest wipe, and recovery of historical incomplete deletes.
pub async fn purge_document_list_surfaces(
    state: &AppState,
    document_id: &str,
    workspace_id: &str,
    tenant_ctx: &TenantContext,
    opts: ListSurfacePurgeOpts<'_>,
) -> ApiResult<ListSurfacePurgeResult> {
    let key_prefix = opts.key_prefix.unwrap_or(document_id);
    let key_id_mismatch = key_prefix != document_id;

    let mut keys: Vec<String> = Vec::with_capacity(16);
    // Final + staging metadata for both identity shapes.
    for id in identity_variants(document_id, key_prefix, key_id_mismatch) {
        keys.push(metadata_key_for_document(id));
        keys.push(kv_keys::staging_doc_metadata(id));
        keys.push(format!("{id}-content"));
        keys.push(kv_keys::staging_doc_content(id));
        keys.push(kv_keys::workspace_doc_index(workspace_id, id));
    }
    if let Some(content_hash) = opts.content_hash {
        if !content_hash.is_empty() {
            keys.push(ContentHasher::workspace_hash_key(
                workspace_id,
                content_hash,
            ));
            keys.push(kv_keys::staging_workspace_hash(workspace_id, content_hash));
            // SPEC-091 W2: typed ingestion_dedup delete parity.
            #[cfg(feature = "postgres")]
            crate::services::ingestion_dedup_store::dual_delete_all(
                state.optional_pg_pool(),
                workspace_id,
                content_hash,
            )
            .await;
        }
    }

    // Dedup while preserving order (identity overlap is common).
    keys.sort();
    keys.dedup();

    let kv_keys_deleted = keys.len();
    state
        .storage
        .kv_storage
        .delete(&keys)
        .await
        .map_err(ApiError::from)?;

    #[cfg(feature = "postgres")]
    let mut relational_rows_deleted = 0u64;
    #[cfg(not(feature = "postgres"))]
    let relational_rows_deleted = 0u64;

    #[cfg(feature = "postgres")]
    {
        // Scoped SQL delete (fail-closed on error — never warn-and-leave ghosts).
        for id in identity_variants(document_id, key_prefix, key_id_mismatch) {
            relational_rows_deleted += crate::document_read_model::delete_relational_document(
                state.optional_pg_pool(),
                id,
                tenant_ctx,
            )
            .await?;
        }

        // PDF asset row (best-effort: FK cascade often already cleared by documents DELETE).
        if let Some(pdf_id) = opts.pdf_id {
            if let Some(ref pdf_storage) = state.storage.pdf_storage {
                if let Ok(pdf_uuid) = Uuid::parse_str(pdf_id) {
                    if let Err(e) = pdf_storage.delete_pdf(&pdf_uuid).await {
                        tracing::debug!(
                            pdf_id = %pdf_id,
                            document_id = %document_id,
                            error = %e,
                            "pdf_documents row already absent or delete skipped"
                        );
                    }
                }
            }
        }

        // Also try pdf_storage document record for deployments without pg_pool wiring.
        if let Some(ref pdf_storage) = state.storage.pdf_storage {
            for id in identity_variants(document_id, key_prefix, key_id_mismatch) {
                if let Ok(doc_uuid) = Uuid::parse_str(id) {
                    match pdf_storage.delete_document_record(&doc_uuid).await {
                        Ok(()) => {}
                        Err(e) => {
                            tracing::debug!(
                                document_id = %id,
                                error = %e,
                                "pdf_storage delete_document_record skipped"
                            );
                        }
                    }
                }
            }
        }

        let mm_storage = state.storage.mm_asset_storage.as_deref();
        let workspace_uuid = Uuid::parse_str(workspace_id).ok();
        for id in identity_variants(document_id, key_prefix, key_id_mismatch) {
            let _ =
                crate::services::delete_document_mm_assets(mm_storage, id, workspace_uuid).await;

            // SPEC-091 Wave B5: typed sidecar parity — the legacy KV family
            // keys (lineage/manifest/chunks/checkpoint/snapshot) are swept by
            // the metadata-prefix delete above; the typed rows need explicit
            // deletes for documents whose `documents` row removal does not
            // cascade (identity variants, missing row).
            let store = state.operational_stores.checkpoint_artifacts.as_deref();
            crate::services::relational_sidecar_store::typed_artifact_delete_all(store, id).await;
            crate::services::relational_sidecar_store::typed_checkpoint_delete(
                store,
                id,
                crate::services::relational_sidecar_store::CHECKPOINT_KIND_CRASH,
            )
            .await;
            crate::services::relational_sidecar_store::typed_checkpoint_delete(
                store,
                id,
                crate::services::relational_sidecar_store::CHECKPOINT_KIND_SNAPSHOT,
            )
            .await;
        }
    }

    #[cfg(not(feature = "postgres"))]
    {
        let _ = tenant_ctx;
    }

    tracing::debug!(
        document_id = %document_id,
        workspace_id = %workspace_id,
        kv_keys = kv_keys_deleted,
        relational_rows = relational_rows_deleted,
        "List-surface purge complete"
    );

    Ok(ListSurfacePurgeResult {
        kv_keys_deleted,
        relational_rows_deleted,
    })
}

#[inline]
fn identity_variants<'a>(
    document_id: &'a str,
    key_prefix: &'a str,
    key_id_mismatch: bool,
) -> impl Iterator<Item = &'a str> {
    std::iter::once(document_id).chain(if key_id_mismatch {
        Some(key_prefix)
    } else {
        None
    })
}

/// Reset a stuck/failed deleting document to a recoverable terminal status.
pub async fn reset_deleting_status(
    state: &AppState,
    document_id: &str,
    key_prefix: &str,
    reason: &str,
    deletion_track_id: Option<&str>,
) {
    let metadata_key = metadata_key_for_document(key_prefix);
    if let Ok(Some(mut metadata)) = state.storage.kv_storage.get_by_id(&metadata_key).await {
        if let Some(obj) = metadata.as_object_mut() {
            let current = obj
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            if current == "deleting" || current == "delete_failed" {
                obj.insert("status".to_string(), serde_json::json!("delete_failed"));
                obj.insert(
                    "current_stage".to_string(),
                    serde_json::json!("delete_failed"),
                );
                obj.insert("stage_message".to_string(), serde_json::json!(reason));
                obj.insert("error_message".to_string(), serde_json::json!(reason));
                let _ = crate::services::upsert_metadata_kv_with_index(
                    state.storage.kv_storage.as_ref(),
                    &metadata_key,
                    metadata,
                )
                .await;
            }
        }
    }

    // SPEC-098 LAW-098-9: mirror delete_failed to SQL list column.
    #[cfg(feature = "postgres")]
    let pool = state.optional_pg_pool();
    #[cfg(not(feature = "postgres"))]
    let pool = None;
    crate::services::touch_sql_delete_failed(pool, document_id).await;
    if key_prefix != document_id {
        crate::services::touch_sql_delete_failed(pool, key_prefix).await;
    }

    if let Some(track_id) = deletion_track_id {
        state
            .tasks
            .progress_broadcaster
            .deletion_failed(document_id, track_id, reason);
    }
}

#[cfg(feature = "postgres")]
async fn tombstone_document_required(
    state: &AppState,
    data: &DeletionTaskData,
    metadata: Option<&serde_json::Value>,
) -> ApiResult<Vec<Uuid>> {
    // Memory AppState has no pg_pool: keep the pre-SPEC-149 non-authoritative path.
    if state.pg_pool.is_none() {
        return Ok(Vec::new());
    }
    let committer = state.lifecycle_committer.as_ref().ok_or_else(|| {
        ApiError::Internal(
            "P0 deletion requires lifecycle_committer; refuse physical cleanup without tombstone"
                .into(),
        )
    })?;
    let tenant_id = Uuid::parse_str(&data.tenant_id)
        .map_err(|error| ApiError::BadRequest(format!("invalid deletion tenant_id: {error}")))?;
    let workspace_id = Uuid::parse_str(&data.workspace_id)
        .map_err(|error| ApiError::BadRequest(format!("invalid deletion workspace_id: {error}")))?;
    let document_id = Uuid::parse_str(&data.document_id)
        .map_err(|error| ApiError::BadRequest(format!("invalid deletion document_id: {error}")))?;

    let metadata_revision = metadata.and_then(|value| {
        ["provider_access_revision", "object_revision"]
            .iter()
            .find_map(|key| value.get(key).and_then(serde_json::Value::as_u64))
    });
    let expected_revision = if let Some(revision) = metadata_revision {
        revision
    } else {
        let reader = state.document_reader.as_ref().ok_or_else(|| {
            ApiError::Internal(
                "P0 deletion requires document_reader when metadata revision is absent".into(),
            )
        })?;
        let scope = edgequake_storage::contracts::AccessScope::new(
            edgequake_storage::contracts::TenantId::new(tenant_id),
            edgequake_storage::contracts::WorkspaceId::new(workspace_id),
        );
        let views = reader
            .get_many(
                &scope,
                &[edgequake_storage::contracts::DocumentId::new(document_id)],
            )
            .await
            .map_err(edgequake_storage::StorageError::from)
            .map_err(ApiError::from)?;
        match views.into_iter().next().flatten() {
            Some(view) if view.deleted => {
                return load_existing_cleanup_bindings(state, document_id).await;
            }
            Some(view) => view.revision,
            None => 0,
        }
    };
    let idempotency_key = format!("delete:{document_id}:{expected_revision}");
    let canonical = serde_json::to_vec(&serde_json::json!({
        "tenant_id": tenant_id,
        "workspace_id": workspace_id,
        "document_id": document_id,
        "expected_revision": expected_revision,
        "idempotency_key": idempotency_key,
    }))
    .map_err(|error| ApiError::Internal(format!("encode tombstone command: {error}")))?;
    let command = edgequake_storage::contracts::DeleteDocument {
        scope: edgequake_storage::contracts::AccessScope::new(
            edgequake_storage::contracts::TenantId::new(tenant_id),
            edgequake_storage::contracts::WorkspaceId::new(workspace_id),
        ),
        document_id: edgequake_storage::contracts::DocumentId::new(document_id),
        expected_revision,
        idempotency_key,
        command_digest: Sha256::digest(canonical).into(),
    };
    let receipt = committer
        .tombstone_document(&command)
        .await
        .map_err(edgequake_storage::StorageError::from)
        .map_err(ApiError::from)?;
    if receipt.target_binding_ids.is_empty() {
        return Err(ApiError::Internal(
            "tombstone receipt recorded zero target bindings; refuse default-store cleanup".into(),
        ));
    }
    Ok(receipt.target_binding_ids)
}

#[cfg(feature = "postgres")]
async fn load_existing_cleanup_bindings(
    state: &AppState,
    document_id: Uuid,
) -> ApiResult<Vec<Uuid>> {
    let pool = state
        .pg_pool
        .as_ref()
        .ok_or_else(|| ApiError::Internal("tombstone resume requires a PostgreSQL pool".into()))?;
    let ids: Vec<Uuid> = sqlx::query_scalar(
        "SELECT DISTINCT binding_id FROM public.projection_cleanup_intents \
         WHERE document_id = $1 ORDER BY binding_id",
    )
    .bind(document_id)
    .fetch_all(pool)
    .await
    .map_err(edgequake_storage::StorageError::from)
    .map_err(ApiError::from)?;
    if ids.is_empty() {
        return Err(ApiError::Conflict("document is already tombstoned".into()));
    }
    Ok(ids)
}

async fn projection_owns_cleanup(state: &AppState, document_id: &str) -> ApiResult<bool> {
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (state, document_id);
        return Ok(false);
    }
    #[cfg(feature = "postgres")]
    {
        if state.projection_worker.is_none() {
            return Ok(false);
        }
        let Some(pool) = state.optional_pg_pool() else {
            return Ok(false);
        };
        let Ok(document_id) = Uuid::parse_str(document_id) else {
            return Ok(false);
        };
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM public.projection_event_items i \
             JOIN public.projection_events e ON e.event_id = i.event_id \
             WHERE e.object_id = $1 AND e.operation = 'delete'",
        )
        .bind(document_id)
        .fetch_one(pool)
        .await
        .map_err(edgequake_storage::StorageError::from)
        .map_err(ApiError::from)?;
        Ok(count > 0)
    }
}

async fn wait_for_delete_deliveries(state: &AppState, document_id: &str) -> ApiResult<()> {
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (state, document_id);
        return Ok(());
    }
    #[cfg(feature = "postgres")]
    {
        let pool = state.optional_pg_pool().ok_or_else(|| {
            ApiError::Internal("projection cleanup requires a PostgreSQL pool".into())
        })?;
        let document_id = Uuid::parse_str(document_id)
            .map_err(|error| ApiError::BadRequest(format!("invalid document id: {error}")))?;
        for _ in 0..60 {
            let states: Vec<String> = sqlx::query_scalar(
                "SELECT d.state FROM public.projection_deliveries d \
                 JOIN public.projection_events e ON e.event_id = d.event_id \
                 WHERE e.object_id = $1 AND e.operation = 'delete'",
            )
            .bind(document_id)
            .fetch_all(pool)
            .await
            .map_err(edgequake_storage::StorageError::from)
            .map_err(ApiError::from)?;
            if states.iter().any(|state| state == "quarantined") {
                return Err(ApiError::Internal(
                    "projection delete delivery was quarantined".into(),
                ));
            }
            if !states.is_empty() && states.iter().all(|state| state == "applied") {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        Err(ApiError::Internal(
            "projection delete deliveries did not acknowledge before purge".into(),
        ))
    }
}

#[cfg(not(feature = "postgres"))]
async fn tombstone_document_required(
    _state: &AppState,
    _data: &DeletionTaskData,
    _metadata: Option<&serde_json::Value>,
) -> ApiResult<Vec<Uuid>> {
    // Memory / non-postgres builds use an explicit non-authoritative path.
    Ok(Vec::new())
}

#[cfg(feature = "postgres")]
async fn vector_cleanup_targets(state: &AppState, binding_ids: &[Uuid]) -> ApiResult<Vec<Uuid>> {
    if state.pg_pool.is_none() {
        return Ok(Vec::new());
    }
    if binding_ids.is_empty() {
        return Err(ApiError::Internal(
            "exact-binding vector cleanup refused empty target list".into(),
        ));
    }
    let pool = state
        .pg_pool
        .as_ref()
        .ok_or_else(|| ApiError::Internal("lifecycle committer has no PostgreSQL pool".into()))?;
    let vector_ids: Vec<Uuid> = sqlx::query_scalar(
        "SELECT binding_id FROM public.data_bindings \
         WHERE binding_id = ANY($1) \
           AND role IN ('vector', 'vector_projection', 'embedding') \
           AND state IN ('active', 'draining') \
         ORDER BY binding_id",
    )
    .bind(binding_ids)
    .fetch_all(pool)
    .await
    .map_err(edgequake_storage::StorageError::from)
    .map_err(ApiError::from)?;
    if vector_ids.is_empty() {
        return Err(ApiError::Internal(
            "tombstone targets contain no vector bindings; refuse default-store cleanup".into(),
        ));
    }
    Ok(vector_ids)
}

#[cfg(not(feature = "postgres"))]
async fn vector_cleanup_targets(_state: &AppState, _binding_ids: &[Uuid]) -> ApiResult<Vec<Uuid>> {
    Ok(Vec::new())
}

/// Run the authoritative cascade for a document (graph → vectors → KV → relational).
pub async fn perform_document_deletion(
    state: &AppState,
    data: &DeletionTaskData,
    tenant_ctx: &TenantContext,
) -> ApiResult<DocumentDeletionResult> {
    let document_id = data.document_id.clone();
    let actual_key_prefix = data.key_prefix.clone();
    let key_id_mismatch = actual_key_prefix != document_id;
    let metadata_key = data
        .metadata_key
        .clone()
        .unwrap_or_else(|| metadata_key_for_document(&actual_key_prefix));
    let metadata = state
        .storage
        .kv_storage
        .get_by_id(&metadata_key)
        .await
        .ok()
        .flatten();
    let has_metadata = metadata.is_some();
    let content_key = format!("{}-content", actual_key_prefix);
    let has_content = data.has_content
        || state
            .storage
            .kv_storage
            .get_by_id(&content_key)
            .await
            .ok()
            .flatten()
            .is_some();
    let chunk_ids = if data.chunk_ids.is_empty() {
        let chunk_prefix = format!("{}-chunk-", actual_key_prefix);
        state
            .storage
            .kv_storage
            .keys_with_prefix(&chunk_prefix)
            .await
            .unwrap_or_default()
    } else {
        data.chunk_ids.clone()
    };
    let workspace_id_for_storage = data.workspace_id.clone();
    let deletion_track_id = data.deletion_track_id.clone();
    let document_status = data
        .document_status
        .clone()
        .unwrap_or_else(|| "deleting".to_string());

    state
        .tasks
        .progress_broadcaster
        .deletion_started(&document_id, &deletion_track_id);

    if matches!(
        document_status.as_str(),
        "pending" | "processing" | "deleting"
    ) {
        if let Some(track_id) = data.ingest_track_id.as_deref() {
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
    }

    // SPEC-149: durable logical deletion precedes every physical provider
    // mutation. A conflict or unavailable authority fails closed.
    let target_binding_ids = tombstone_document_required(state, data, metadata.as_ref()).await?;
    let projection_owned = projection_owns_cleanup(state, &document_id).await?;
    if projection_owned {
        wait_for_delete_deliveries(state, &document_id).await?;
    }
    #[cfg(feature = "postgres")]
    let vector_target_bindings = vector_cleanup_targets(state, &target_binding_ids).await?;
    #[cfg(not(feature = "postgres"))]
    let vector_target_bindings: Vec<Uuid> = Vec::new();
    let cleanup_vectors = !vector_target_bindings.is_empty() || cfg!(not(feature = "postgres"));
    let _ = &vector_target_bindings; // retained for exact-binding cleanup evidence/logging

    // Keep the running deletion task itself (DRY with wipe keep-self). Matching
    // on document_id alone used to cancel+delete this row mid-cascade.
    // In-flight ingest siblings are cancelled; Failed/Indexed/Cancelled stay
    // in `tasks` as audit rows (issue #386).
    let persisted_tasks_removed = purge_persisted_tasks_for_document_except(
        state,
        &document_id,
        data.ingest_track_id.as_deref(),
        Some(&workspace_id_for_storage),
        &deletion_track_id,
    )
    .await;

    let workspace_vector_storage =
        get_workspace_vector_storage_for_delete(state, &workspace_id_for_storage).await?;

    let chunks_deleted = chunk_ids.len();
    let mut embeddings_deleted = 0usize;
    let partial_failure = false;
    let partial_failure_reason: Option<String> = None;

    let scope =
        DocumentSourceScope::with_key_prefix(document_id.clone(), actual_key_prefix.clone());
    let checkpoint = read_deletion_checkpoint(state, &metadata_key).await;
    let graph_already_done = checkpoint.as_deref() == Some(CHECKPOINT_GRAPH_DONE)
        || checkpoint.as_deref() == Some(CHECKPOINT_VECTORS_DONE);

    let mut entities_removed = 0usize;
    let mut entities_updated = 0usize;
    let mut relationships_removed = 0usize;
    let mut relationships_updated = 0usize;

    if !graph_already_done && !projection_owned {
        state.tasks.progress_broadcaster.deletion_phase(
            &document_id,
            &deletion_track_id,
            DeletionPhaseKind::RemovingGraph,
            0,
            0,
        );

        // Zero-match guard: scoped empty but unscoped hits ⇒ filter mismatch, fail closed.
        match scoped_discovery_has_filter_mismatch(state, tenant_ctx, &scope).await {
            Ok(true) => {
                let reason = "Graph discovery filter mismatch: source-owned rows exist but scoped discovery returned zero — retaining metadata as delete_failed";
                tracing::error!(document_id = %document_id, "{reason}");
                reset_deleting_status(
                    state,
                    &document_id,
                    &actual_key_prefix,
                    reason,
                    Some(deletion_track_id.as_str()),
                )
                .await;
                return Err(ApiError::Internal(reason.into()));
            }
            Err(e) => {
                let reason = format!("Graph discovery error: {e}");
                reset_deleting_status(
                    state,
                    &document_id,
                    &actual_key_prefix,
                    &reason,
                    Some(deletion_track_id.as_str()),
                )
                .await;
                return Err(e);
            }
            Ok(false) => {}
        }

        // ISSUE-305: fail closed — never wipe KV/docs if graph cascade cannot run.
        // SPEC-069: progress ticks + periodic heartbeats during long shared upserts.
        let cascade_stats = {
            let mut last_err: Option<ApiError> = None;
            let mut stats = None;
            let last_processed = Arc::new(AtomicU32::new(0));
            let last_total = Arc::new(AtomicU32::new(0));
            for attempt in 1u8..=2 {
                let hb_doc = document_id.clone();
                let hb_track = deletion_track_id.clone();
                let hb_proc = Arc::clone(&last_processed);
                let hb_tot = Arc::clone(&last_total);
                let hb_broadcast = state.tasks.progress_broadcaster.clone();
                let heartbeat = tokio::spawn(async move {
                    let mut tick = tokio::time::interval(Duration::from_secs(3));
                    tick.tick().await; // skip immediate first fire
                    loop {
                        tick.tick().await;
                        hb_broadcast.deletion_phase(
                            &hb_doc,
                            &hb_track,
                            DeletionPhaseKind::RemovingGraph,
                            hb_proc.load(Ordering::Relaxed),
                            hb_tot.load(Ordering::Relaxed),
                        );
                    }
                });

                let broadcast = state.tasks.progress_broadcaster.clone();
                let doc_id = document_id.clone();
                let track = deletion_track_id.clone();
                let proc_ref = Arc::clone(&last_processed);
                let tot_ref = Arc::clone(&last_total);
                let result = cascade_remove_document_sources_with_progress(
                    &state.storage.graph_storage,
                    Some(&workspace_vector_storage),
                    Some(tenant_ctx),
                    &scope,
                    |processed, total| {
                        proc_ref.store(processed, Ordering::Relaxed);
                        tot_ref.store(total, Ordering::Relaxed);
                        broadcast.deletion_phase(
                            &doc_id,
                            &track,
                            DeletionPhaseKind::RemovingGraph,
                            processed,
                            total,
                        );
                    },
                )
                .await;
                heartbeat.abort();

                match result {
                    Ok(s) => {
                        stats = Some(s);
                        break;
                    }
                    Err(e) => {
                        tracing::warn!(
                            document_id = %document_id,
                            attempt,
                            error = %e,
                            "Graph cascade delete failed"
                        );
                        last_err = Some(e);
                    }
                }
            }
            match stats {
                Some(s) => s,
                None => {
                    let reason = format!(
                        "Graph cascade error: {}",
                        last_err
                            .as_ref()
                            .map(|e| e.to_string())
                            .unwrap_or_else(|| "unknown".to_string())
                    );
                    tracing::error!(
                        document_id = %document_id,
                        %reason,
                        "Graph cascade delete failed after retry — aborting KV wipe (fail-closed)"
                    );
                    reset_deleting_status(
                        state,
                        &document_id,
                        &actual_key_prefix,
                        &reason,
                        Some(deletion_track_id.as_str()),
                    )
                    .await;
                    return Err(last_err.unwrap_or_else(|| {
                        ApiError::Internal("Graph cascade delete failed".into())
                    }));
                }
            }
        };

        if let Err(e) = post_proof_source_absent(state, tenant_ctx, &scope).await {
            let reason = e.to_string();
            tracing::error!(
                document_id = %document_id,
                %reason,
                "Post-proof residual provenance — aborting vector/KV wipe"
            );
            reset_deleting_status(
                state,
                &document_id,
                &actual_key_prefix,
                &reason,
                Some(deletion_track_id.as_str()),
            )
            .await;
            return Err(e);
        }

        entities_removed = cascade_stats.entities_removed;
        entities_updated = cascade_stats.entities_updated;
        relationships_removed = cascade_stats.relationships_removed;
        relationships_updated = cascade_stats.relationships_updated;
        embeddings_deleted += cascade_stats.embeddings_deleted;
        write_deletion_checkpoint(state, &metadata_key, has_metadata, CHECKPOINT_GRAPH_DONE).await;
    } else {
        // Resume after graph: still re-prove before vectors/KV.
        if let Err(e) = post_proof_source_absent(state, tenant_ctx, &scope).await {
            let reason = e.to_string();
            reset_deleting_status(
                state,
                &document_id,
                &actual_key_prefix,
                &reason,
                Some(deletion_track_id.as_str()),
            )
            .await;
            return Err(e);
        }
    }

    let vectors_already_done = checkpoint.as_deref() == Some(CHECKPOINT_VECTORS_DONE);
    if !vectors_already_done && cleanup_vectors && !projection_owned {
        state.tasks.progress_broadcaster.deletion_phase(
            &document_id,
            &deletion_track_id,
            DeletionPhaseKind::RemovingVectors,
            0,
            chunk_ids.len() as u32,
        );

        match workspace_vector_storage
            .delete_by_document(&document_id)
            .await
        {
            Ok(n) => {
                embeddings_deleted += n;
            }
            Err(e) => {
                tracing::warn!(
                    document_id = %document_id,
                    error = %e,
                    "delete_by_document failed; falling back to chunk id delete"
                );
                if !chunk_ids.is_empty() {
                    if let Err(e2) = workspace_vector_storage.delete(&chunk_ids).await {
                        let reason = format!("Vector delete failed: {e2}");
                        reset_deleting_status(
                            state,
                            &document_id,
                            &actual_key_prefix,
                            &reason,
                            Some(deletion_track_id.as_str()),
                        )
                        .await;
                        return Err(ApiError::Internal(reason));
                    }
                    embeddings_deleted += chunk_ids.len();
                }
            }
        }
        if key_id_mismatch {
            let _ = workspace_vector_storage
                .delete_by_document(&actual_key_prefix)
                .await;
        }
        write_deletion_checkpoint(state, &metadata_key, has_metadata, CHECKPOINT_VECTORS_DONE)
            .await;
    } else if !vectors_already_done {
        tracing::debug!(
            document_id = %document_id,
            target_bindings = ?target_binding_ids,
            "Skipping chunk vector cleanup because the tombstone targeted no vector binding"
        );
        write_deletion_checkpoint(state, &metadata_key, has_metadata, CHECKPOINT_VECTORS_DONE)
            .await;
    }

    // Chunk + residual prefix keys (graph/vector already proved clean).
    // List surfaces (metadata, wsdoc, SQL, hash) are always purged via SSOT
    // even when has_metadata was false (historical incomplete deletes).
    let mut keys_to_delete = chunk_ids.clone();
    if has_metadata {
        keys_to_delete.push(metadata_key.clone());
    }
    if has_content {
        keys_to_delete.push(content_key);
    }

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
    keys_to_delete.extend(all_prefix_keys);

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
        keys_to_delete.extend(alt_prefix_keys);
    }

    state.tasks.progress_broadcaster.deletion_phase(
        &document_id,
        &deletion_track_id,
        DeletionPhaseKind::RemovingKv,
        0,
        keys_to_delete.len() as u32,
    );

    state
        .storage
        .kv_storage
        .delete(&keys_to_delete)
        .await
        .map_err(ApiError::from)?;

    // SSOT list-surface purge: always (not gated on has_metadata). Fail-closed
    // if relational delete errors — merge would re-inject ghosts on refresh.
    if let Err(e) = purge_document_list_surfaces(
        state,
        &document_id,
        &workspace_id_for_storage,
        tenant_ctx,
        ListSurfacePurgeOpts {
            key_prefix: Some(&actual_key_prefix),
            content_hash: data.content_hash.as_deref(),
            pdf_id: data.pdf_id.as_deref(),
        },
    )
    .await
    {
        let reason = format!("List-surface purge failed: {e}");
        tracing::error!(document_id = %document_id, %reason);
        // Metadata may already be gone; best-effort delete_failed badge.
        reset_deleting_status(
            state,
            &document_id,
            &actual_key_prefix,
            &reason,
            Some(deletion_track_id.as_str()),
        )
        .await;
        return Err(e);
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
        }
    }

    let tenant_for_audit = tenant_ctx
        .tenant_id
        .clone()
        .unwrap_or_else(|| "default".to_string());
    record_compliance_event(
        state,
        tenant_for_audit,
        AuditEventType::Authorization,
        "delete_document",
        AuditResult::Success,
        tenant_ctx.workspace_id.clone(),
        tenant_ctx.user_id.clone(),
        Some(("document".to_string(), document_id.clone())),
    );

    state.tasks.progress_broadcaster.deletion_phase(
        &document_id,
        &deletion_track_id,
        DeletionPhaseKind::Finalizing,
        0,
        1,
    );

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

    Ok(DocumentDeletionResult {
        chunks_deleted,
        entities_removed,
        entities_updated,
        relationships_removed,
        relationships_updated,
        embeddings_deleted,
        persisted_tasks_removed,
        partial_failure,
        partial_failure_reason,
    })
}

/// Boot/crash recovery: docs left in `deleting` with no active Deletion task
/// are re-enqueued (idempotent cascade) so they never stay stuck forever.
pub async fn reconcile_stuck_deleting_documents(state: &AppState, max: usize) -> usize {
    use crate::services::document_metadata_scan::{
        document_id_from_metadata_key, load_all_document_metadata_entries,
    };
    use edgequake_tasks::{Task, TaskType};

    let Ok(entries) = load_all_document_metadata_entries(
        state.storage.kv_storage.as_ref(),
        state.optional_pg_pool(),
    )
    .await
    else {
        return 0;
    };

    let mut requeued = 0usize;
    for (key, meta) in entries {
        if requeued >= max {
            break;
        }
        let status = meta
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        if status != "deleting" {
            continue;
        }
        let document_id = meta
            .get("id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| document_id_from_metadata_key(&key))
            .unwrap_or_else(|| key.trim_end_matches("-metadata").to_string());
        let key_prefix = document_id_from_metadata_key(&key).unwrap_or_else(|| document_id.clone());
        let workspace_id = meta
            .get("workspace_id")
            .and_then(|v| v.as_str())
            .unwrap_or("default")
            .to_string();
        let workspace_uuid = Uuid::parse_str(&workspace_id).ok();
        if find_active_deletion_track_id(state, &document_id, workspace_uuid)
            .await
            .is_some()
        {
            continue;
        }

        let tenant_id = meta
            .get("tenant_id")
            .and_then(|v| v.as_str())
            .unwrap_or("default")
            .to_string();
        let data = DeletionTaskData {
            document_id: document_id.clone(),
            key_prefix: key_prefix.clone(),
            workspace_id: workspace_id.clone(),
            tenant_id: tenant_id.clone(),
            deletion_track_id: String::new(),
            metadata_key: Some(key),
            chunk_ids: Vec::new(),
            has_content: false,
            content_hash: meta
                .get("content_hash")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            pdf_id: meta
                .get("pdf_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            ingest_track_id: meta
                .get("track_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            document_status: Some("deleting".to_string()),
        };
        let mut task = Task::new(
            Uuid::parse_str(&tenant_id).unwrap_or_else(|_| Uuid::nil()),
            Uuid::parse_str(&workspace_id).unwrap_or_else(|_| Uuid::nil()),
            TaskType::Deletion,
            serde_json::to_value(&data).unwrap_or_default(),
        );
        let deletion_track_id = task.track_id.clone();
        if let Some(obj) = task.task_data.as_object_mut() {
            obj.insert(
                "deletion_track_id".to_string(),
                serde_json::json!(&deletion_track_id),
            );
        }
        match state.enqueue_task(task).await {
            Ok(()) => {
                tracing::info!(
                    document_id = %document_id,
                    track_id = %deletion_track_id,
                    "Re-enqueued stuck deleting document"
                );
                requeued += 1;
            }
            Err(e) => {
                tracing::warn!(
                    document_id = %document_id,
                    error = %e,
                    "Failed to re-enqueue stuck deleting document"
                );
            }
        }
    }
    requeued
}

/// Find a pending/processing Deletion task for the same document (enqueue dedup).
pub async fn find_active_deletion_track_id(
    state: &AppState,
    document_id: &str,
    workspace_id: Option<Uuid>,
) -> Option<String> {
    use edgequake_tasks::{Pagination, TaskFilter, TaskStatus, TaskType};

    for status in [TaskStatus::Pending, TaskStatus::Processing] {
        let list = state
            .tasks
            .storage
            .list_tasks(
                TaskFilter {
                    workspace_id,
                    status: Some(status),
                    task_type: Some(TaskType::Deletion),
                    ..Default::default()
                },
                Pagination {
                    page: 1,
                    page_size: 100,
                    ..Default::default()
                },
            )
            .await
            .ok()?;
        for task in list.tasks {
            if let Ok(data) = serde_json::from_value::<DeletionTaskData>(task.task_data.clone()) {
                if data.document_id == document_id || data.key_prefix == document_id {
                    return Some(data.deletion_track_id);
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_storage::kv_keys;

    #[tokio::test]
    async fn purge_list_surfaces_removes_wsdoc_and_metadata_even_when_orphaned() {
        let state = crate::state::AppState::test_state();
        let ws = "00000000-0000-0000-0000-0000000000ab";
        let doc = "purge-ghost-doc";
        let meta_key = format!("{doc}-metadata");
        let content_key = format!("{doc}-content");
        let wsdoc = kv_keys::workspace_doc_index(ws, doc);
        let hash = "a".repeat(64);
        let hash_key = ContentHasher::workspace_hash_key(ws, &hash);

        state
            .storage
            .kv_storage
            .upsert(&[
                (
                    meta_key.clone(),
                    serde_json::json!({
                        "id": doc,
                        "workspace_id": ws,
                        "status": "completed",
                    }),
                ),
                (content_key.clone(), serde_json::json!("body")),
                (
                    wsdoc.clone(),
                    serde_json::json!({
                        "metadata_key": meta_key,
                        "document_id": doc,
                        "workspace_id": ws,
                    }),
                ),
                (hash_key.clone(), serde_json::json!(doc)),
            ])
            .await
            .unwrap();

        // Simulate incomplete cascade: metadata already gone, list surfaces remain.
        state
            .storage
            .kv_storage
            .delete(std::slice::from_ref(&meta_key))
            .await
            .unwrap();

        let tenant = TenantContext {
            tenant_id: Some("00000000-0000-0000-0000-000000000001".into()),
            workspace_id: Some(ws.into()),
            user_id: None,
        };

        let result = purge_document_list_surfaces(
            &state,
            doc,
            ws,
            &tenant,
            ListSurfacePurgeOpts {
                key_prefix: Some(doc),
                content_hash: Some(&hash),
                pdf_id: None,
            },
        )
        .await
        .expect("purge must succeed");

        assert!(result.kv_keys_deleted >= 4);
        assert!(
            state
                .storage
                .kv_storage
                .get_by_id(&wsdoc)
                .await
                .unwrap()
                .is_none(),
            "wsdoc index must be purged (ghost list source)"
        );
        assert!(state
            .storage
            .kv_storage
            .get_by_id(&content_key)
            .await
            .unwrap()
            .is_none());
        assert!(
            state
                .storage
                .kv_storage
                .get_by_id(&hash_key)
                .await
                .unwrap()
                .is_none(),
            "content-hash pointer must not outlive list surfaces"
        );
        // Idempotent second purge.
        purge_document_list_surfaces(
            &state,
            doc,
            ws,
            &tenant,
            ListSurfacePurgeOpts {
                key_prefix: Some(doc),
                content_hash: Some(&hash),
                pdf_id: None,
            },
        )
        .await
        .expect("idempotent");
    }

    #[tokio::test]
    async fn purge_list_surfaces_handles_key_prefix_mismatch() {
        let state = crate::state::AppState::test_state();
        let ws = "00000000-0000-0000-0000-0000000000cd";
        let doc_id = "canonical-id";
        let key_prefix = "legacy-prefix";
        let wsdoc_id = kv_keys::workspace_doc_index(ws, doc_id);
        let wsdoc_prefix = kv_keys::workspace_doc_index(ws, key_prefix);

        state
            .storage
            .kv_storage
            .upsert(&[
                (wsdoc_id.clone(), serde_json::json!({"document_id": doc_id})),
                (
                    wsdoc_prefix.clone(),
                    serde_json::json!({"document_id": key_prefix}),
                ),
                (
                    format!("{key_prefix}-metadata"),
                    serde_json::json!({"id": doc_id, "workspace_id": ws}),
                ),
            ])
            .await
            .unwrap();

        let tenant = TenantContext {
            tenant_id: None,
            workspace_id: Some(ws.into()),
            user_id: None,
        };

        purge_document_list_surfaces(
            &state,
            doc_id,
            ws,
            &tenant,
            ListSurfacePurgeOpts {
                key_prefix: Some(key_prefix),
                content_hash: None,
                pdf_id: None,
            },
        )
        .await
        .unwrap();

        assert!(state
            .storage
            .kv_storage
            .get_by_id(&wsdoc_id)
            .await
            .unwrap()
            .is_none());
        assert!(state
            .storage
            .kv_storage
            .get_by_id(&wsdoc_prefix)
            .await
            .unwrap()
            .is_none());
        assert!(state
            .storage
            .kv_storage
            .get_by_id(&format!("{key_prefix}-metadata"))
            .await
            .unwrap()
            .is_none());
    }
}
