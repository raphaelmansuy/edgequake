//! Recovery handler for documents stuck in "processing" status.
//!
//! Finds documents that have been processing longer than a configurable
//! threshold and requeues them, cleaning up partial graph data first.

use axum::response::Response;
use axum::{extract::State, response::IntoResponse, Json};
use chrono::Utc;
use tracing::debug;
use uuid::Uuid;

use crate::document_metadata::is_active_processing_status;
use crate::error::ApiResult;
use crate::handlers::documents_types::*;
use crate::middleware::TenantContext;
use crate::state::AppState;

use super::super::storage_helpers::cleanup_document_graph_data;

/// Recover documents stuck in active processing statuses.
///
/// Finds documents in any in-flight stage (`processing`, `indexing`, `storing`,
/// `chunking`, …) older than the threshold and requeues them. Useful after
/// server restarts or crashes that left tasks incomplete.
#[utoipa::path(
    post,
    path = "/api/v1/documents/recover-stuck",
    tag = "Documents",
    request_body = RecoverStuckRequest,
    responses(
        (status = 200, description = "Stuck documents recovered (legacy default)", body = RecoverStuckResponse),
        (status = 202, description = "Recovery accepted when REST-025 opt-in or strict startup", body = RecoverStuckResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn recover_stuck(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Json(request): Json<RecoverStuckRequest>,
) -> ApiResult<Response> {
    let workspace_id = tenant_ctx.workspace_id.clone();
    let return_202 = state.security.v1_rpc_return_202;
    let response = run_recover_stuck(state, tenant_ctx, request).await?;
    if let Some(ws) = workspace_id.as_deref() {
        let track_id = response.track_id.clone();
        return crate::services::v1_rpc_migration::respond_v1_async_rpc(
            ws,
            Some(track_id.as_str()),
            return_202,
            response,
        );
    }
    Ok(Json(response).into_response())
}

pub(crate) async fn run_recover_stuck(
    state: AppState,
    tenant_ctx: TenantContext,
    request: RecoverStuckRequest,
) -> ApiResult<RecoverStuckResponse> {
    use chrono::Duration;

    debug!(
        "recover_stuck called with tenant context: tenant_id={:?}, workspace_id={:?}, threshold={}min",
        tenant_ctx.tenant_id, tenant_ctx.workspace_id, request.stuck_threshold_minutes
    );

    // SPEC-086: fail orphan staging shells (list-visible) before final-metadata recover.
    let staging_age =
        std::time::Duration::from_secs(request.stuck_threshold_minutes.saturating_mul(60));
    if let Err(e) = crate::services::recover_orphaned_staging_admissions(
        std::sync::Arc::clone(&state.storage.kv_storage),
        std::sync::Arc::clone(&state.tasks.storage),
        Some(staging_age),
    )
    .await
    {
        tracing::warn!(error = %e, "recover_orphaned_staging_admissions during recover-stuck");
    }

    // Generate new track ID for recovery batch
    let new_track_id = format!(
        "recover_{}_{}",
        Utc::now().format("%Y%m%d_%H%M%S"),
        &Uuid::new_v4().to_string()[..8]
    );

    let threshold = Duration::minutes(request.stuck_threshold_minutes as i64);
    let cutoff_time = Utc::now() - threshold;

    // P-G7 + SPEC-027: batch scoped metadata (suffix index + tenant filter).
    // Prefer progress loader so aged staging shells (post 086 merge) are visible.
    let scoped_metadata =
        crate::services::document_metadata_scan::load_scoped_document_metadata_for_progress(
            state.storage.kv_storage.as_ref(),
            &tenant_ctx,
        )
        .await?;

    let mut stuck_docs = Vec::new();
    let mut requeued_ids = Vec::new();
    let mut requeued_titles = Vec::new();

    for value in scoped_metadata {
        if stuck_docs.len() >= request.max_documents {
            break;
        }

        if let Some(obj) = value.as_object() {
            let status = obj.get("status").and_then(|v| v.as_str());
            let doc_id = obj.get("id").and_then(|v| v.as_str());
            let title = obj.get("title").and_then(|v| v.as_str());
            let updated_at = obj.get("updated_at").and_then(|v| v.as_str());

            // Any active pipeline stage can get stuck (not only legacy "processing").
            if status.is_some_and(is_active_processing_status)
                || obj
                    .get("current_stage")
                    .and_then(|v| v.as_str())
                    .is_some_and(is_active_processing_status)
            {
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
                    // Staging shells need re-upload after orphan fail — never
                    // requeue onto final `{id}-metadata` (empty content → ghost pending).
                    if obj
                        .get("admission_staging")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false)
                    {
                        continue;
                    }
                    if let Some(id) = doc_id {
                        stuck_docs.push((id.to_string(), title.unwrap_or(id).to_string()));
                    }
                }
            }
        }
    }

    // Requeue stuck documents via SPEC-054/#298 SSOT (DRY with startup reconcile).
    use crate::services::pending_doc_task_reconcile::{
        ensure_task_for_pending_document, EnsureTaskOutcome,
    };

    let vector = crate::services::get_workspace_vector_storage_for_delete(
        &state,
        tenant_ctx.workspace_id.as_deref().unwrap_or("default"),
    )
    .await;

    for (doc_id, doc_title) in &stuck_docs {
        // OODA-08 / SPEC-059: retract vectors + prune graph sources BEFORE requeueing
        let stats = crate::services::retract_document_indexes(
            &state.storage.graph_storage,
            &vector,
            Some(&tenant_ctx),
            doc_id,
        )
        .await;
        tracing::info!(
            document_id = %doc_id,
            entities_removed = stats.entities_removed,
            entities_updated = stats.entities_updated,
            relationships_removed = stats.relationships_removed,
            embeddings_deleted = stats.embeddings_deleted,
            "SPEC-059: retracted indexes before stuck recovery"
        );
        // Keep cleanup path for graph-only edge cases (idempotent with retract).
        let _ =
            cleanup_document_graph_data(doc_id, &state.storage.graph_storage, Some(&vector)).await;

        let metadata_key =
            crate::services::document_metadata_scan::metadata_key_for_document(doc_id);
        let mut metadata = state
            .storage
            .kv_storage
            .get_by_id(&metadata_key)
            .await?
            .unwrap_or_else(|| serde_json::json!({ "id": doc_id, "title": doc_title }));

        // Ensure tenant/workspace ids are present for task construction.
        if let Some(obj) = metadata.as_object_mut() {
            if obj.get("tenant_id").and_then(|v| v.as_str()).is_none() {
                if let Some(ref tid) = tenant_ctx.tenant_id {
                    obj.insert("tenant_id".to_string(), serde_json::json!(tid));
                }
            }
            if obj.get("workspace_id").and_then(|v| v.as_str()).is_none() {
                if let Some(ref wid) = tenant_ctx.workspace_id {
                    obj.insert("workspace_id".to_string(), serde_json::json!(wid));
                }
            }
            obj.insert("status".to_string(), serde_json::json!("pending"));
            obj.insert(
                "recovery_reason".to_string(),
                serde_json::json!("stuck_in_processing"),
            );
            obj.insert(
                "recovered_at".to_string(),
                serde_json::json!(Utc::now().to_rfc3339()),
            );
        }

        // Persist recovery fields via wsdoc SSOT before enqueue (ensure_task re-reads KV).
        crate::services::upsert_metadata_kv_with_index(
            state.storage.kv_storage.as_ref(),
            &metadata_key,
            metadata.clone(),
        )
        .await?;

        let content_key = format!("{doc_id}-content");
        let content = state
            .storage
            .kv_storage
            .get_by_id(&content_key)
            .await?
            .and_then(|v| {
                v.get("content")
                    .and_then(|c| c.as_str())
                    .map(str::to_string)
            });

        match ensure_task_for_pending_document(
            &state,
            doc_id,
            &metadata,
            content.as_deref(),
            &new_track_id,
            "stuck_in_processing",
        )
        .await?
        {
            EnsureTaskOutcome::Enqueued { .. } | EnsureTaskOutcome::AlreadyScheduled => {
                requeued_ids.push(doc_id.clone());
                requeued_titles.push(doc_title.clone());
                tracing::info!("Recovered stuck document: {} ({})", doc_id, doc_title);
            }
            EnsureTaskOutcome::SkippedNoContent | EnsureTaskOutcome::RequiresReupload { .. } => {
                tracing::warn!(
                    document_id = %doc_id,
                    "recover_stuck: no pdf_id/content — cannot enqueue"
                );
            }
            EnsureTaskOutcome::SkippedNotEligible => {}
        }
    }

    let response = RecoverStuckResponse {
        track_id: new_track_id,
        v2_migration: tenant_ctx
            .workspace_id
            .as_ref()
            .map(|ws| crate::services::job_registry::v2_migration_hint("recover_stuck", ws)),
        stuck_found: stuck_docs.len(),
        requeued: requeued_ids.len(),
        document_ids: requeued_ids,
        document_titles: requeued_titles,
    };
    Ok(response)
}
