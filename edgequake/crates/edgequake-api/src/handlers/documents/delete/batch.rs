//! Selected multi-document deletion (SPEC-084 / GH-317).
//!
//! `POST /api/v1/documents/batch-delete` admits one `TaskType::BatchDeletion`
//! so the UI does not storm N× single deletes (pool / lifecycle fairness).

use axum::{
    extract::State,
    http::{header, HeaderMap, HeaderValue, StatusCode},
    Json,
};
use edgequake_tasks::{BatchDeletionTaskData, Task, TaskType};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::middleware::{resolve_workspace_uuid, TenantContext};
use crate::services::find_active_workspace_wipe_track_id;
use crate::state::AppState;

/// Max IDs per batch-delete request (use wipe-all for full workspace).
pub const MAX_BATCH_DELETE_IDS: usize = 500;

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct BatchDeleteDocumentsRequest {
    /// Document IDs to delete (selected subset).
    pub document_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BatchDeleteDocumentsResponse {
    pub accepted: bool,
    pub batch_track_id: String,
    pub planned_delete_count: usize,
}

/// Delete a selected set of documents (async job — 202 Accepted).
#[utoipa::path(
    post,
    path = "/api/v1/documents/batch-delete",
    tag = "Documents",
    request_body = BatchDeleteDocumentsRequest,
    responses(
        (status = 202, description = "Batch deletion accepted", body = BatchDeleteDocumentsResponse),
        (status = 400, description = "Empty or oversized document_ids"),
        (status = 409, description = "Workspace wipe already in flight")
    )
)]
pub async fn batch_delete_documents(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Json(body): Json<BatchDeleteDocumentsRequest>,
) -> ApiResult<(StatusCode, HeaderMap, Json<BatchDeleteDocumentsResponse>)> {
    let mut ids: Vec<String> = body
        .document_ids
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    ids.sort();
    ids.dedup();

    if ids.is_empty() {
        return Err(ApiError::BadRequest(
            "document_ids must contain at least one id".into(),
        ));
    }
    if ids.len() > MAX_BATCH_DELETE_IDS {
        return Err(ApiError::BadRequest(format!(
            "document_ids exceeds max {MAX_BATCH_DELETE_IDS}; use Clear All for full workspace wipe"
        )));
    }

    let workspace_id_str = tenant_ctx.workspace_id_or_default();
    let workspace_uuid = resolve_workspace_uuid(Some(&workspace_id_str))
        .ok_or_else(|| ApiError::BadRequest(format!("invalid workspace_id: {workspace_id_str}")))?;

    if let Some(wipe_track) = find_active_workspace_wipe_track_id(&state, workspace_uuid).await {
        return Err(ApiError::Conflict(format!(
            "workspace wipe in flight ({wipe_track}); cannot batch-delete concurrently"
        )));
    }

    let tenant_id_str = tenant_ctx
        .tenant_id
        .clone()
        .unwrap_or_else(|| "default".to_string());
    let tenant_uuid = Uuid::parse_str(&tenant_id_str).unwrap_or_else(|_| Uuid::nil());

    let planned = ids.len();
    let task_data = BatchDeletionTaskData {
        document_ids: ids,
        tenant_id: tenant_id_str,
        workspace_id: workspace_id_str,
        batch_track_id: String::new(),
        deleted_count: 0,
        failed_ids: Vec::new(),
    };

    let mut task = Task::new(
        tenant_uuid,
        workspace_uuid,
        TaskType::BatchDeletion,
        serde_json::to_value(&task_data).map_err(|e| {
            ApiError::Internal(format!("Failed to serialize BatchDeletionTaskData: {e}"))
        })?,
    );
    let batch_track_id = task.track_id.clone();
    if let Some(obj) = task.task_data.as_object_mut() {
        obj.insert(
            "batch_track_id".to_string(),
            serde_json::json!(&batch_track_id),
        );
    }

    state.enqueue_task(task).await?;

    tracing::info!(
        batch_track_id = %batch_track_id,
        planned_delete_count = planned,
        "SPEC-084 / GH-317: batch deletion admitted"
    );

    let mut resp_headers = HeaderMap::new();
    if let Ok(loc) = HeaderValue::from_str(&format!("/api/v1/tasks/{batch_track_id}")) {
        resp_headers.insert(header::LOCATION, loc);
    }

    Ok((
        StatusCode::ACCEPTED,
        resp_headers,
        Json(BatchDeleteDocumentsResponse {
            accepted: true,
            batch_track_id,
            planned_delete_count: planned,
        }),
    ))
}
