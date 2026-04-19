//! Recovery handler for documents stuck in "processing" status.
//!
//! Finds documents that have been processing longer than a configurable
//! threshold and requeues them, cleaning up partial graph data first.

use axum::{extract::State, Json};
use chrono::Utc;
use tracing::debug;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::handlers::documents_types::*;
use crate::middleware::TenantContext;
use crate::state::AppState;

use super::super::storage_helpers::{cleanup_document_graph_data, metadata_matches_tenant_context};

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
            if !metadata_matches_tenant_context(&value, &tenant_ctx) {
                continue;
            }

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
