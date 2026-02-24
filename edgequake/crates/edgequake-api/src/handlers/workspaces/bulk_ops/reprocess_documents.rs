//! Reprocess all documents endpoint (SPEC-032).
//!
//! Queues workspace documents for reprocessing with filtering controls
//! for document status and max document count.

use axum::{
    extract::{Path, State},
    Json,
};
use uuid::Uuid;

use crate::error::ApiError;
use crate::handlers::workspaces_types::*;
use crate::middleware::TenantContext;
use crate::state::AppState;

use super::{build_reprocess_task, collect_workspace_documents, mark_document_pending};

// SPEC-032: Reprocess All Documents Endpoint
// Focus Area 5 — Trigger document reprocessing after rebuild

/// Reprocess all documents in a workspace.
///
/// This endpoint queues all documents for re-embedding, typically used after
/// a rebuild-embeddings operation to regenerate vector embeddings. Progress
/// can be monitored via the pipeline status endpoint.
///
/// ## Use Cases
///
/// - Regenerate embeddings after model change
/// - Re-extract entities after LLM update
/// - Bulk re-processing for quality improvements
#[utoipa::path(
    post,
    path = "/api/v1/workspaces/{workspace_id}/reprocess-documents",
    request_body = ReprocessAllRequest,
    params(
        ("workspace_id" = Uuid, Path, description = "Workspace ID")
    ),
    responses(
        (status = 200, description = "Documents queued for reprocessing", body = ReprocessAllResponse),
        (status = 404, description = "Workspace not found"),
        (status = 400, description = "Invalid request"),
    ),
    tags = ["workspaces"]
)]
pub async fn reprocess_all_documents(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
    tenant_ctx: TenantContext,
    Json(request): Json<ReprocessAllRequest>,
) -> Result<Json<ReprocessAllResponse>, ApiError> {
    use chrono::Utc;
    use tracing::info;

    // 1. Verify workspace exists
    let workspace = state
        .workspace_service
        .get_workspace(workspace_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Workspace {} not found", workspace_id)))?;

    // BR0201: verify workspace belongs to requesting tenant before bulk op
    if let Some(ref ctx_tid) = tenant_ctx.tenant_id {
        if workspace.tenant_id.to_string() != *ctx_tid {
            tracing::warn!(workspace_id = %workspace_id, "Tenant isolation: reprocess-documents rejected");
            return Err(ApiError::NotFound(format!(
                "Workspace {} not found",
                workspace_id
            )));
        }
    }

    // 2. Generate track ID for this batch
    let track_id = format!(
        "reprocess_{}_{}",
        Utc::now().format("%Y%m%d_%H%M%S"),
        &Uuid::new_v4().to_string()[..8]
    );

    info!(
        workspace_id = %workspace_id,
        track_id = %track_id,
        include_completed = request.include_completed,
        "Starting reprocess all documents"
    );

    // 3. Collect all workspace documents
    let docs = collect_workspace_documents(&state, &workspace_id, &workspace.slug).await?;

    // REQ-24: Debug logging for document discovery
    info!(
        workspace_id = %workspace_id,
        documents_found = docs.len(),
        "Scanned KV storage for documents to reprocess"
    );

    let documents_found = docs.len();
    let mut documents_queued = 0;
    let mut documents_skipped = 0;
    let mut skip_reasons: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();

    // 4. Process each document with filtering
    for doc in &docs {
        if documents_queued >= request.max_documents {
            *skip_reasons.entry("max_documents_reached").or_insert(0) += 1;
            break;
        }

        // Skip if not including completed and already completed
        if !request.include_completed && doc.status.as_deref() == Some("completed") {
            documents_skipped += 1;
            *skip_reasons.entry("completed_excluded").or_insert(0) += 1;
            continue;
        }

        // Skip if currently processing
        if doc.status.as_deref() == Some("processing") {
            documents_skipped += 1;
            *skip_reasons.entry("already_processing").or_insert(0) += 1;
            continue;
        }

        // Mark pending and build task
        mark_document_pending(&state, &doc.doc_id, &track_id).await;

        let extra_meta = serde_json::Map::new();
        if let Some((task_type, task_value)) =
            build_reprocess_task(&state, &workspace, workspace_id, doc, &track_id, extra_meta).await
        {
            let task = edgequake_tasks::Task::new(
                workspace.tenant_id,
                workspace_id,
                task_type,
                task_value,
            );

            if let Err(e) = state.task_storage.create_task(&task).await {
                info!(error = %e, doc_id = %doc.doc_id, "Failed to create task, skipping");
                documents_skipped += 1;
                *skip_reasons.entry("task_create_failed").or_insert(0) += 1;
                continue;
            }

            if let Err(e) = state.task_queue.send(task).await {
                info!(error = %e, doc_id = %doc.doc_id, "Failed to queue task, skipping");
                documents_skipped += 1;
                *skip_reasons.entry("task_queue_failed").or_insert(0) += 1;
                continue;
            }

            documents_queued += 1;
        } else {
            documents_skipped += 1;
            *skip_reasons.entry("no_content").or_insert(0) += 1;
        }
    }

    // REQ-24: Log detailed skip reasons for debugging
    if !skip_reasons.is_empty() {
        info!(
            workspace_id = %workspace_id,
            skip_reasons = ?skip_reasons,
            "Document skip reasons breakdown"
        );
    }

    // 5. Estimate processing time (1 second per document conservative)
    let estimated_time = if documents_queued > 0 {
        Some(documents_queued as u64)
    } else {
        None
    };

    let response = ReprocessAllResponse {
        track_id,
        workspace_id,
        status: if documents_queued > 0 {
            "processing".to_string()
        } else {
            "no_documents".to_string()
        },
        documents_found,
        documents_queued,
        documents_skipped,
        estimated_time_seconds: estimated_time,
    };

    info!(
        workspace_id = %workspace_id,
        found = documents_found,
        queued = documents_queued,
        skipped = documents_skipped,
        "Reprocess all documents complete"
    );

    Ok(Json(response))
}
