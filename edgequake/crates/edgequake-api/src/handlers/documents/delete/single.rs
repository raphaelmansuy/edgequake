//! Single document deletion handler.
//!
//! Fast path: resolve identity, enqueue `TaskType::Deletion`, then admit
//! `status=deleting`, return **202 Accepted**. The worker runs the
//! authoritative cascade via [`crate::services::perform_document_deletion`].
//! Enqueue runs before status mutation so a queue/DB failure cannot leave
//! the document stuck in `deleting` with no job.
//!
//! @implements SPEC-050: Real-time deletion progress via WebSocket broadcast.

use axum::{extract::State, http::StatusCode, Json};
use uuid::Uuid;

use edgequake_tasks::{DeletionTaskData, Task, TaskType};

use crate::error::{ApiError, ApiResult};
use crate::handlers::documents_types::*;
use crate::middleware::TenantContext;
use crate::services::find_active_deletion_track_id;
#[cfg(test)]
use crate::services::perform_document_deletion;
use crate::state::AppState;

#[cfg(feature = "postgres")]
use crate::document_read_model::relational_document_scope;
use crate::services::document_metadata_scan::{
    canonical_document_id, document_id_from_metadata_key, load_all_document_metadata_entries,
    metadata_key_for_document,
};

use super::super::storage_helpers::metadata_matches_tenant_context;

/// Resolve the actual KV key prefix for a document.
///
/// Returns `(actual_key_prefix, metadata_key, has_metadata)`.
pub(crate) async fn resolve_kv_key_prefix_for_batch(
    document_id: &str,
    state: &AppState,
) -> (String, String, bool) {
    resolve_kv_key_prefix(document_id, state).await
}

/// Resolve the actual KV key prefix for a document.
///
/// Returns `(actual_key_prefix, metadata_key, has_metadata)`.
///
/// SPEC-086: staging shells live at `staging:{id}-metadata`. Never treat
/// `staging:{id}` as the cascade key prefix (breaks content/hash cleanup and
/// can leave delete sessions hanging while a worker runs a useless graph scan).
async fn resolve_kv_key_prefix(document_id: &str, state: &AppState) -> (String, String, bool) {
    // IMP-075-13: final + staging existence in one RT (final-first for delete).
    // Distinct from staging-first ingest SSOT — promoted final wins when both exist.
    let direct_metadata_key = metadata_key_for_document(document_id);
    let staging_metadata_key = edgequake_storage::kv_keys::staging_doc_metadata(document_id);
    let probe_keys = [direct_metadata_key.clone(), staging_metadata_key.clone()];
    if let Ok(vals) = state
        .storage
        .kv_storage
        .get_by_ids_ordered(&probe_keys)
        .await
    {
        if vals.first().and_then(|v| v.as_ref()).is_some() {
            return (document_id.to_string(), direct_metadata_key, true);
        }
        if vals.get(1).and_then(|v| v.as_ref()).is_some() {
            return (document_id.to_string(), staging_metadata_key, true);
        }
    }

    if let Ok(entries) = load_all_document_metadata_entries(state.storage.kv_storage.as_ref()).await
    {
        for (key, val) in entries {
            let canonical = canonical_document_id(&key, &val);
            let legacy_json_id = val.get("id").and_then(|v| v.as_str());
            let staging_match = key.starts_with("staging:")
                && (legacy_json_id == Some(document_id)
                    || key == staging_metadata_key
                    || canonical.strip_prefix("staging:") == Some(document_id));
            if staging_match {
                return (document_id.to_string(), key, true);
            }
            if canonical == document_id || legacy_json_id == Some(document_id) {
                let prefix =
                    document_id_from_metadata_key(&key).unwrap_or_else(|| document_id.to_string());
                // Never use `staging:{id}` as cascade prefix.
                let prefix = prefix
                    .strip_prefix("staging:")
                    .unwrap_or(prefix.as_str())
                    .to_string();
                return (prefix, key, true);
            }
        }
    }

    (document_id.to_string(), direct_metadata_key, false)
}

/// Sync-dismiss a staging-only admission shell (no graph/vector cascade needed).
async fn delete_staging_shell_sync(
    state: &AppState,
    document_id: &str,
    metadata_key: &str,
    tenant_ctx: &TenantContext,
) -> ApiResult<(StatusCode, Json<DeleteDocumentResponse>)> {
    let meta = state
        .storage
        .kv_storage
        .get_by_id(metadata_key)
        .await
        .ok()
        .flatten();
    if let Some(ref metadata) = meta {
        if !metadata_matches_tenant_context(metadata, tenant_ctx) {
            return Err(ApiError::NotFound(format!(
                "Document {} not found",
                document_id
            )));
        }
    }
    let workspace_id = meta
        .as_ref()
        .and_then(|m| m.get("workspace_id").and_then(|v| v.as_str()))
        .unwrap_or("default")
        .to_string();
    let content_hash = meta
        .as_ref()
        .and_then(|m| m.get("content_hash").and_then(|v| v.as_str()))
        .unwrap_or("")
        .to_string();
    let ingest_track_id = meta
        .as_ref()
        .and_then(|m| {
            m.get("track_id")
                .or_else(|| m.get("task_id"))
                .and_then(|v| v.as_str())
        })
        .map(|s| s.to_string());

    // Cancel any still-queued Insert so the worker cannot recreate final metadata
    // after we wipe the staging shell (delete-during-early-upload race).
    if let Some(ref track) = ingest_track_id {
        let cancelled = state.tasks.cancellation_registry.cancel(track).await;
        tracing::info!(
            document_id = %document_id,
            track_id = %track,
            cancelled,
            "Cancelled ingest task before sync staging dismiss"
        );
    }

    crate::services::rollback_staging(
        &state.storage.kv_storage,
        document_id,
        &workspace_id,
        &content_hash,
    )
    .await
    .map_err(ApiError::Internal)?;

    // Belt-and-suspenders: drop any leftover staging keys for this id.
    let leftover = [
        edgequake_storage::kv_keys::staging_doc_metadata(document_id),
        edgequake_storage::kv_keys::staging_doc_content(document_id),
    ];
    let _ = state.storage.kv_storage.delete(&leftover).await;

    let track_id = format!("delete-staging-{document_id}");
    state
        .tasks
        .progress_broadcaster
        .deletion_started(document_id, &track_id);
    state.tasks.progress_broadcaster.deletion_completed(
        document_id,
        &track_id,
        0,
        0,
        0,
        0,
        false,
        None,
    );

    tracing::info!(
        document_id = %document_id,
        "Staging admission shell deleted synchronously (no cascade)"
    );

    Ok((
        StatusCode::OK,
        Json(DeleteDocumentResponse {
            document_id: document_id.to_string(),
            deleted: true,
            accepted: false,
            track_id: Some(track_id),
            chunks_deleted: 0,
            entities_affected: 0,
            relationships_affected: 0,
            embeddings_deleted: 0,
            partial_failure: false,
            partial_failure_reason: None,
        }),
    ))
}

/// Delete a document by ID (async job — 202 Accepted).
#[utoipa::path(
    delete,
    path = "/api/v1/documents/{document_id}",
    tag = "Documents",
    params(
        ("document_id" = String, Path, description = "Document ID to delete")
    ),
    responses(
        (status = 202, description = "Deletion accepted; track via WebSocket", body = DeleteDocumentResponse),
        (status = 404, description = "Document not found")
    )
)]
pub async fn delete_document(
    State(state): State<AppState>,
    axum::extract::Path(document_id): axum::extract::Path<String>,
    tenant_ctx: TenantContext,
) -> ApiResult<(StatusCode, Json<DeleteDocumentResponse>)> {
    let (actual_key_prefix, metadata_key, has_metadata) =
        resolve_kv_key_prefix(&document_id, &state).await;
    let key_id_mismatch = actual_key_prefix != document_id;

    if key_id_mismatch {
        tracing::warn!(
            document_id = %document_id,
            actual_key_prefix = %actual_key_prefix,
            "KV key/id mismatch detected — using resolved key prefix for cascade delete"
        );
    }

    // SPEC-086: orphan/failed staging shells have no graph — sync dismiss.
    // Avoids queueing Deletion behind ingest and leave UI stuck on "Deleting".
    if has_metadata && metadata_key.starts_with("staging:") {
        let final_also_present = state
            .storage
            .kv_storage
            .get_by_id(&metadata_key_for_document(&document_id))
            .await
            .ok()
            .flatten()
            .is_some();
        if !final_also_present {
            return delete_staging_shell_sync(&state, &document_id, &metadata_key, &tenant_ctx)
                .await;
        }
    }

    let chunk_prefix = format!("{}-chunk-", actual_key_prefix);
    let chunk_ids: Vec<String> = state
        .storage
        .kv_storage
        .keys_with_prefix(&chunk_prefix)
        .await
        .unwrap_or_default();

    let content_key = format!("{}-content", actual_key_prefix);
    let has_content = state
        .storage
        .kv_storage
        .get_by_id(&content_key)
        .await
        .ok()
        .flatten()
        .is_some();

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

    let (workspace_id_for_storage, document_status, content_hash_opt, pdf_id_opt, track_id_opt) =
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
                    let content_hash = metadata
                        .get("content_hash")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let pdf_id = metadata
                        .get("pdf_id")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
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

    let workspace_uuid = Uuid::parse_str(&workspace_id_for_storage).ok();
    if let Some(existing_track) =
        find_active_deletion_track_id(&state, &document_id, workspace_uuid).await
    {
        tracing::info!(
            document_id = %document_id,
            track_id = %existing_track,
            "Deletion already in flight — returning existing track_id (idempotent)"
        );
        return Ok((
            StatusCode::ACCEPTED,
            Json(DeleteDocumentResponse {
                document_id,
                deleted: false,
                accepted: true,
                track_id: Some(existing_track),
                chunks_deleted: 0,
                entities_affected: 0,
                relationships_affected: 0,
                embeddings_deleted: 0,
                partial_failure: false,
                partial_failure_reason: None,
            }),
        ));
    }

    let tenant_id_str = tenant_ctx
        .tenant_id
        .clone()
        .unwrap_or_else(|| "default".to_string());
    let tenant_uuid = Uuid::parse_str(&tenant_id_str).unwrap_or_else(|_| Uuid::nil());
    let workspace_uuid_for_task =
        Uuid::parse_str(&workspace_id_for_storage).unwrap_or_else(|_| Uuid::nil());

    // Placeholder correlation id — overwritten with durable task.track_id below
    // so WS/API/purge keep-self share one SSOT (same pattern as workspace wipe).
    let task_data = DeletionTaskData {
        document_id: document_id.clone(),
        key_prefix: actual_key_prefix.clone(),
        workspace_id: workspace_id_for_storage.clone(),
        tenant_id: tenant_id_str,
        deletion_track_id: String::new(),
        metadata_key: if has_metadata {
            Some(metadata_key.clone())
        } else {
            None
        },
        chunk_ids: chunk_ids.clone(),
        has_content,
        content_hash: content_hash_opt,
        pdf_id: pdf_id_opt,
        ingest_track_id: track_id_opt,
        document_status: Some(document_status),
    };

    let mut task = Task::new(
        tenant_uuid,
        workspace_uuid_for_task,
        TaskType::Deletion,
        serde_json::to_value(&task_data).map_err(|e| {
            ApiError::Internal(format!("Failed to serialize DeletionTaskData: {e}"))
        })?,
    );
    let deletion_track_id = task.track_id.clone();
    if let Some(obj) = task.task_data.as_object_mut() {
        obj.insert(
            "deletion_track_id".to_string(),
            serde_json::json!(&deletion_track_id),
        );
    }

    // First principle: durable job BEFORE status=deleting. Enqueue failure must
    // not leave the document stuck in a deleting badge with no worker task.
    state.enqueue_task(task).await?;

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

    state
        .tasks
        .progress_broadcaster
        .deletion_started(&document_id, &deletion_track_id);

    tracing::info!(
        document_id = %document_id,
        track_id = %deletion_track_id,
        chunks = chunk_ids.len(),
        "Deletion accepted — worker will run cascade"
    );

    Ok((
        StatusCode::ACCEPTED,
        Json(DeleteDocumentResponse {
            document_id,
            deleted: false,
            accepted: true,
            track_id: Some(deletion_track_id),
            chunks_deleted: 0,
            entities_affected: 0,
            relationships_affected: 0,
            embeddings_deleted: 0,
            partial_failure: false,
            partial_failure_reason: None,
        }),
    ))
}

/// Test helper: admit delete then run the cascade inline (no live worker in unit tests).
#[cfg(test)]
pub async fn delete_document_and_drain_for_test(
    state: &AppState,
    document_id: String,
    tenant_ctx: TenantContext,
) -> ApiResult<DeleteDocumentResponse> {
    let (status, Json(accepted)) = delete_document(
        State(state.clone()),
        axum::extract::Path(document_id),
        tenant_ctx.clone(),
    )
    .await?;
    assert_eq!(status, StatusCode::ACCEPTED);
    assert!(accepted.accepted);

    // Drain the Deletion task from the queue and run the cascade.
    if let Ok(Some(task)) = state.tasks.queue.try_receive().await {
        if task.task_type == TaskType::Deletion {
            let data: DeletionTaskData = serde_json::from_value(task.task_data)
                .map_err(|e| ApiError::Internal(e.to_string()))?;
            let result = perform_document_deletion(state, &data, &tenant_ctx).await?;
            return Ok(DeleteDocumentResponse {
                document_id: data.document_id,
                deleted: true,
                accepted: false,
                track_id: Some(data.deletion_track_id),
                chunks_deleted: result.chunks_deleted,
                entities_affected: result.entities_removed + result.entities_updated,
                relationships_affected: result.relationships_removed + result.relationships_updated,
                embeddings_deleted: result.embeddings_deleted,
                partial_failure: result.partial_failure,
                partial_failure_reason: result.partial_failure_reason,
            });
        }
    }

    Ok(accepted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_resolve_key_prefix_fast_path() {
        let state = AppState::test_state();
        let doc_id = "aaaa-bbbb-cccc-dddd";
        let metadata_key = metadata_key_for_document(doc_id);

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

    #[tokio::test]
    async fn test_resolve_key_prefix_mismatch() {
        let state = AppState::test_state();
        let kv_prefix = "real-key-prefix-1111";
        let json_id = "mismatched-json-id-2222";
        let metadata_key = metadata_key_for_document(kv_prefix);

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

        assert_eq!(prefix, kv_prefix);
        assert_eq!(key, metadata_key);
        assert!(has_metadata);
    }

    #[tokio::test]
    async fn test_resolve_key_prefix_not_found() {
        let state = AppState::test_state();
        let doc_id = "nonexistent-doc-9999";

        let (prefix, key, has_metadata) = resolve_kv_key_prefix(doc_id, &state).await;

        assert_eq!(prefix, doc_id);
        assert_eq!(key, metadata_key_for_document(doc_id));
        assert!(!has_metadata);
    }

    #[tokio::test]
    async fn test_resolve_staging_prefix_uses_document_id() {
        let state = AppState::test_state();
        let doc_id = "staging-shell-doc-001";
        let staging_key = edgequake_storage::kv_keys::staging_doc_metadata(doc_id);
        state
            .storage
            .kv_storage
            .upsert(&[(
                staging_key.clone(),
                json!({
                    "id": doc_id,
                    "status": "failed",
                    "admission_staging": true,
                    "workspace_id": "default",
                    "error_message": "Orphaned staging admission — please re-upload"
                }),
            )])
            .await
            .unwrap();

        let (prefix, key, has_metadata) = resolve_kv_key_prefix(doc_id, &state).await;
        assert_eq!(
            prefix, doc_id,
            "must not use staging:{{id}} as cascade prefix"
        );
        assert_eq!(key, staging_key);
        assert!(has_metadata);
    }

    #[tokio::test]
    async fn test_delete_staging_shell_sync_removes_keys() {
        let state = AppState::test_state();
        let doc_id = "staging-shell-doc-002";
        let staging_meta = edgequake_storage::kv_keys::staging_doc_metadata(doc_id);
        let staging_content = edgequake_storage::kv_keys::staging_doc_content(doc_id);
        state
            .storage
            .kv_storage
            .upsert(&[
                (
                    staging_meta.clone(),
                    json!({
                        "id": doc_id,
                        "status": "failed",
                        "admission_staging": true,
                        "workspace_id": "default",
                        "content_hash": "abc123",
                        "error_message": "Orphaned staging admission — please re-upload"
                    }),
                ),
                (staging_content.clone(), json!({"text": "hello"})),
            ])
            .await
            .unwrap();

        let tenant = TenantContext {
            tenant_id: Some("default".into()),
            workspace_id: Some("default".into()),
            user_id: None,
        };
        let (status, Json(resp)) = delete_document(
            State(state.clone()),
            axum::extract::Path(doc_id.to_string()),
            tenant,
        )
        .await
        .unwrap();
        assert_eq!(status, StatusCode::OK);
        assert!(resp.deleted);
        assert!(!resp.accepted);
        assert!(state
            .storage
            .kv_storage
            .get_by_id(&staging_meta)
            .await
            .unwrap()
            .is_none());
        assert!(state
            .storage
            .kv_storage
            .get_by_id(&staging_content)
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn test_delete_document_with_key_id_mismatch() {
        let state = AppState::test_state();
        let kv_prefix = "4b788a9e-0000-0000-0000-000000000001";
        let json_id = "2cddf543-0000-0000-0000-000000000002";

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

        let response = delete_document_and_drain_for_test(
            &state,
            json_id.to_string(),
            TenantContext::default(),
        )
        .await
        .expect("delete should succeed for mismatched key/id document");
        assert!(response.deleted);
        assert_eq!(response.chunks_deleted, 2);

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

    #[tokio::test]
    async fn test_delete_mismatch_cleans_lineage_and_alt_prefix_keys() {
        let state = AppState::test_state();
        let kv_prefix = "aaaa-0000-0000-0000-000000000001";
        let json_id = "bbbb-0000-0000-0000-000000000002";

        let metadata_key = metadata_key_for_document(kv_prefix);
        let lineage_key = format!("{}-lineage", kv_prefix);
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

        let response = delete_document_and_drain_for_test(
            &state,
            json_id.to_string(),
            TenantContext::default(),
        )
        .await
        .expect("delete should succeed");
        assert!(response.deleted);

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

    #[tokio::test]
    async fn test_delete_returns_202_accepted() {
        let state = AppState::test_state();
        let doc_id = "cccc-0000-0000-0000-000000000003";
        let metadata_key = metadata_key_for_document(doc_id);
        state
            .storage
            .kv_storage
            .upsert(&[(
                metadata_key,
                json!({"id": doc_id, "status": "completed", "workspace_id": "default"}),
            )])
            .await
            .unwrap();

        let (status, Json(resp)) = delete_document(
            State(state.clone()),
            axum::extract::Path(doc_id.to_string()),
            TenantContext::default(),
        )
        .await
        .expect("accept");

        assert_eq!(status, StatusCode::ACCEPTED);
        assert!(resp.accepted);
        assert!(!resp.deleted);
        assert!(resp.track_id.is_some());
    }
}
