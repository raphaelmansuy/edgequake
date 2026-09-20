//! Task management handlers.
//!
//! ## Implements
//!
//! - **FEAT0560**: Task status retrieval by track ID
//! - **FEAT0561**: Task listing with filters and pagination
//! - **FEAT0562**: Task cancellation for pending jobs
//! - **FEAT0563**: Task statistics aggregation
//!
//! ## Use Cases
//!
//! - **UC2160**: User polls task status during async document processing
//! - **UC2161**: User lists all pending and completed tasks
//! - **UC2162**: User cancels queued task before processing starts
//! - **UC2163**: Admin views task statistics for monitoring
//!
//! ## Enforces
//!
//! - **BR0560**: Track IDs must be valid UUIDs
//! - **BR0561**: Task listing must support status and type filters
//! - **BR0562**: Only pending tasks can be cancelled

use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use edgequake_tasks::{Pagination, SortField, SortOrder, TaskStatus, TaskType};
use serde_json::json;
use std::sync::Arc;
use tracing;

use crate::error::{ApiError, ApiResult};
use crate::middleware::TenantContext;
use crate::services::cancel_track_with_doc_and_pdf_chain;
use crate::services::document_metadata_scan::load_scoped_document_metadata_entries;
use crate::services::task_scope::{get_task_for_context, task_filter_for_scope};
use crate::state::AppState;

// Re-export DTOs for backward compatibility
pub use crate::handlers::tasks_types::{
    ListTasksQuery, PaginationInfo, StatisticsInfo, TaskErrorResponse, TaskListResponse,
    TaskResponse,
};

/// Get task status by track ID
#[utoipa::path(
    get,
    path = "/api/v1/tasks/{track_id}",
    responses(
        (status = 200, description = "Task found", body = TaskResponse),
        (status = 404, description = "Task not found")
    )
)]
pub async fn get_task(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Path(track_id): Path<String>,
) -> ApiResult<Json<TaskResponse>> {
    let task = get_task_for_context(&state, &track_id, &tenant_ctx).await?;
    let pending_created_at =
        (task.status == edgequake_tasks::TaskStatus::Pending).then_some(task.created_at);
    let mut response = TaskResponse::from(task);

    // SPEC-091 QW2 (LAW-Q4): live queue projection for pending tasks (best-effort).
    if let Some(created_at) = pending_created_at {
        if let Ok(estimate) =
            edgequake_tasks::estimate_queue(state.tasks.storage.as_ref(), created_at).await
        {
            response.queue_position = Some(estimate.position);
            response.eta_seconds = Some(estimate.eta_seconds);
            response.eta_basis = Some(estimate.basis.as_str().to_string());
        }
    }

    Ok(Json(response))
}

/// List tasks with filters and pagination
#[utoipa::path(
    get,
    path = "/api/v1/tasks",
    params(
        ("status" = Option<String>, Query, description = "Filter by status"),
        ("task_type" = Option<String>, Query, description = "Filter by task type"),
        ("page" = Option<u32>, Query, description = "Page number (default: 1)"),
        ("page_size" = Option<u32>, Query, description = "Page size (default: 20, max: 100)"),
        ("sort" = Option<String>, Query, description = "Sort field (created_at, updated_at)"),
        ("order" = Option<String>, Query, description = "Sort order (asc, desc)")
    ),
    responses(
        (status = 200, description = "Tasks listed", body = TaskListResponse)
    )
)]
/// @implements FEAT0406
pub async fn list_tasks(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Query(params): Query<ListTasksQuery>,
) -> Result<impl IntoResponse, ApiError> {
    Ok(Json(
        list_tasks_response(&state, &tenant_ctx, params).await?,
    ))
}

/// Shared task listing logic for v1 and v2 job APIs (DRY).
pub(crate) async fn list_tasks_response(
    state: &AppState,
    tenant_ctx: &TenantContext,
    params: ListTasksQuery,
) -> ApiResult<TaskListResponse> {
    // SECURITY: Merge TenantContext headers with query params for workspace isolation
    // Priority: query params (explicit) > TenantContext headers (automatic) > None
    // WHY: The frontend API client always sends X-Tenant-ID/X-Workspace-ID headers.
    // Query params allow explicit override for admin/debugging scenarios.
    // This matches the pattern used by get_queue_metrics.
    let mut filter = task_filter_for_scope(
        tenant_ctx,
        params.tenant_id.as_deref(),
        params.workspace_id.as_deref(),
    );

    // SECURITY: Enforce strict tenant context requirement — NO EXCEPTIONS
    // WHY: Same enforcement as list_documents (commit d11edba8) — without filtering,
    // pipeline status leaks across workspaces ("Processing 2 documents" from other workspaces).
    if filter.tenant_id.is_none() || filter.workspace_id.is_none() {
        tracing::warn!(
            tenant_id = ?filter.tenant_id,
            workspace_id = ?filter.workspace_id,
            "list_tasks: Missing tenant context — returning empty for security"
        );
        return Ok(TaskListResponse {
            tasks: vec![],
            pagination: PaginationInfo {
                total: 0,
                page: 1,
                page_size: params.page_size.unwrap_or(20).min(100),
                total_pages: 0,
            },
            statistics: StatisticsInfo {
                pending: 0,
                processing: 0,
                indexed: 0,
                failed: 0,
                cancelled: 0,
            },
        });
    }

    filter.status = params
        .status
        .as_deref()
        .and_then(|s| parse_task_status(s).ok());
    filter.task_type = params
        .task_type
        .as_deref()
        .and_then(|t| parse_task_type(t).ok());

    // SPEC-090 F-090-14: prefer keyset cursors when both after_* params present.
    let after_created_at = params
        .after_created_at
        .as_deref()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc));
    let after_track_id = params.after_track_id.clone().filter(|s| !s.is_empty());
    let pagination = Pagination {
        page: params.page.unwrap_or(1),
        page_size: params.page_size.unwrap_or(20).min(100),
        sort_by: params
            .sort
            .as_deref()
            .and_then(|s| parse_sort_field(s).ok())
            .unwrap_or(SortField::CreatedAt),
        order: params
            .order
            .as_deref()
            .and_then(|o| parse_sort_order(o).ok())
            .unwrap_or(SortOrder::Desc),
        after_created_at,
        after_track_id,
    };

    let task_list = state
        .tasks
        .storage
        .list_tasks(filter.clone(), pagination)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to list tasks: {}", e)))?;

    // Get statistics with the same filter to ensure tenant isolation
    // WHY: Statistics must respect the same tenant/workspace filters as the task list
    let stats = state
        .tasks
        .storage
        .get_statistics(filter)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to get statistics: {}", e)))?;

    Ok(TaskListResponse {
        tasks: task_list
            .tasks
            .into_iter()
            .map(TaskResponse::from)
            .collect(),
        pagination: PaginationInfo {
            total: task_list.total,
            page: task_list.page,
            page_size: task_list.page_size,
            total_pages: task_list.total_pages,
        },
        statistics: StatisticsInfo {
            pending: stats.pending,
            processing: stats.processing,
            indexed: stats.indexed,
            failed: stats.failed,
            cancelled: stats.cancelled,
        },
    })
}

/// Cancel a task
#[utoipa::path(
    post,
    path = "/api/v1/tasks/{track_id}/cancel",
    responses(
        (status = 200, description = "Task cancelled", body = TaskResponse),
        (status = 404, description = "Task not found"),
        (status = 409, description = "Cannot cancel task in current status")
    )
)]
/// @implements FEAT0562: Task cancellation
/// @implements SPEC-002: Document status sync on task cancel
pub async fn cancel_task(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Path(track_id): Path<String>,
) -> ApiResult<Json<TaskResponse>> {
    // SPEC-083 S-02: ownership via get_task_for_context (foreign track → 404).
    // Task may already be gone while scoped doc metadata still needs cancel sync.
    let existing = match get_task_for_context(&state, &track_id, &tenant_ctx).await {
        Ok(task) => Some(task),
        Err(ApiError::NotFound(_)) => None,
        Err(e) => return Err(e),
    };

    // SPEC-002: Update document status for this track_id within tenant/workspace scope.
    // SPEC-027 pass 11: scoped metadata scan — never cancel cross-tenant documents.
    let mut doc_updated = false;
    if let Ok(scoped_entries) =
        load_scoped_document_metadata_entries(state.storage.kv_storage.as_ref(), &tenant_ctx).await
    {
        for (key, value) in scoped_entries {
            if let Some(obj) = value.as_object() {
                if let Some(doc_track_id) = obj.get("track_id").and_then(|v| v.as_str()) {
                    if doc_track_id == track_id {
                        let mut updated = obj.clone();
                        // SPEC-057 P0: include failure_class=cancelled (not a bare status patch).
                        crate::services::apply_doc_cancelled_fields(
                            &mut updated,
                            "Task cancelled by user",
                        );

                        match crate::services::upsert_metadata_kv_with_index(
                            state.storage.kv_storage.as_ref(),
                            &key,
                            json!(updated),
                        )
                        .await
                        {
                            Ok(_) => {
                                doc_updated = true;
                                tracing::info!("Updated document status to cancelled: {}", key);
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "Failed to update document status on cancel: {} - {}",
                                    key,
                                    e
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    if existing.is_none() && !doc_updated {
        return Err(ApiError::NotFound(format!("Task not found: {}", track_id)));
    }

    // SPEC-057/059: facade — task row + doc KV + Convert∪Insert chain + retract.
    let workspace_key = existing
        .as_ref()
        .map(|t| t.workspace_id.to_string())
        .unwrap_or_else(|| "default".to_string());
    let vector =
        crate::services::get_workspace_vector_storage_for_delete(&state, &workspace_key).await?;
    let applied = cancel_track_with_doc_and_pdf_chain(
        &state.tasks.storage,
        &state.tasks.cancellation_registry,
        Arc::clone(&state.storage.kv_storage),
        &state.storage.graph_storage,
        &vector,
        &track_id,
    )
    .await
    .map_err(ApiError::Internal)?;

    if applied.conflict_indexed {
        return Err(ApiError::Conflict(format!(
            "Cannot cancel task in status: {}",
            TaskStatus::Indexed
        )));
    }

    if applied.was_running {
        tracing::info!(
            track_id = %track_id,
            "Signalled cancellation token for in-flight task"
        );
    }

    if let Some(task) = applied.task {
        return Ok(Json(TaskResponse::from(task)));
    }

    // Document status updated but task row already gone — synthetic success.
    let now = Utc::now().to_rfc3339();
    Ok(Json(TaskResponse {
        track_id: track_id.clone(),
        tenant_id: "default".to_string(),
        workspace_id: "default".to_string(),
        task_type: "document_processing".to_string(),
        status: "cancelled".to_string(),
        created_at: now.clone(),
        updated_at: now.clone(),
        started_at: None,
        completed_at: Some(now),
        error_message: Some(
            "Task was cancelled (task no longer exists, document status updated)".to_string(),
        ),
        error: None,
        retry_count: 0,
        max_retries: 0,
        progress: None,
        result: None,
        metadata: Some(json!({
            "document_updated": true,
            "reason": "Task not found but document status was updated to cancelled"
        })),
        queue_position: None,
        eta_seconds: None,
        eta_basis: None,
    }))
}

/// Retry a failed task
#[utoipa::path(
    post,
    path = "/api/v1/tasks/{track_id}/retry",
    responses(
        (status = 200, description = "Task queued for retry", body = TaskResponse),
        (status = 404, description = "Task not found"),
        (status = 409, description = "Cannot retry task")
    )
)]
pub async fn retry_task(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Path(track_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    // SPEC-083 S-02: ownership via get_task_for_context (foreign track → 404).
    let mut task = get_task_for_context(&state, &track_id, &tenant_ctx).await?;

    // Check if task can be retried
    if !task.can_retry() {
        return Err(ApiError::Conflict(format!(
            "Cannot retry task: max retries ({}) reached or task not failed",
            task.max_retries
        )));
    }

    // Reset task to pending status for retry
    task.status = TaskStatus::Pending;
    task.error_message = None;

    state
        .tasks
        .storage
        .update_task(&task)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to update task: {}", e)))?;

    // Re-enqueue task
    state
        .tasks
        .queue
        .send(task.clone())
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to enqueue task: {}", e)))?;

    Ok(Json(TaskResponse::from(task)))
}

// === Helper Functions ===

fn parse_task_status(s: &str) -> Result<TaskStatus, String> {
    match s.to_lowercase().as_str() {
        "pending" => Ok(TaskStatus::Pending),
        "processing" => Ok(TaskStatus::Processing),
        "indexed" => Ok(TaskStatus::Indexed),
        "failed" => Ok(TaskStatus::Failed),
        "cancelled" => Ok(TaskStatus::Cancelled),
        _ => Err(format!("Invalid task status: {}", s)),
    }
}

fn parse_task_type(s: &str) -> Result<TaskType, String> {
    match s.to_lowercase().as_str() {
        "upload" => Ok(TaskType::Upload),
        "insert" => Ok(TaskType::Insert),
        // SPEC-083 X-11: filter may still match historical rows; creation returns 501 via v2.
        "scan" => Ok(TaskType::Scan),
        "reindex" => Ok(TaskType::Reindex),
        "pdf_processing" => Ok(TaskType::PdfProcessing),
        "knowledge_injection" => Ok(TaskType::KnowledgeInjection),
        "deletion" => Ok(TaskType::Deletion),
        "workspace_wipe" => Ok(TaskType::WorkspaceWipe),
        _ => Err(format!("Invalid task type: {}", s)),
    }
}

fn parse_sort_field(s: &str) -> Result<SortField, String> {
    match s.to_lowercase().as_str() {
        "created_at" | "created" => Ok(SortField::CreatedAt),
        "updated_at" | "updated" => Ok(SortField::UpdatedAt),
        _ => Err(format!("Invalid sort field: {}", s)),
    }
}

fn parse_sort_order(s: &str) -> Result<SortOrder, String> {
    match s.to_lowercase().as_str() {
        "asc" | "ascending" => Ok(SortOrder::Asc),
        "desc" | "descending" => Ok(SortOrder::Desc),
        _ => Err(format!("Invalid sort order: {}", s)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_tasks::{SortField, SortOrder, TaskStatus, TaskType};

    #[test]
    fn test_parse_task_status_valid() {
        assert!(matches!(
            parse_task_status("pending"),
            Ok(TaskStatus::Pending)
        ));
        assert!(matches!(
            parse_task_status("PROCESSING"),
            Ok(TaskStatus::Processing)
        ));
        assert!(matches!(
            parse_task_status("Indexed"),
            Ok(TaskStatus::Indexed)
        ));
        assert!(matches!(
            parse_task_status("failed"),
            Ok(TaskStatus::Failed)
        ));
        assert!(matches!(
            parse_task_status("cancelled"),
            Ok(TaskStatus::Cancelled)
        ));
    }

    #[test]
    fn test_parse_task_status_invalid() {
        assert!(parse_task_status("invalid").is_err());
        assert!(parse_task_status("").is_err());
    }

    #[test]
    fn test_parse_task_type_valid() {
        assert!(matches!(parse_task_type("upload"), Ok(TaskType::Upload)));
        assert!(matches!(parse_task_type("INSERT"), Ok(TaskType::Insert)));
        assert!(matches!(parse_task_type("scan"), Ok(TaskType::Scan)));
        assert!(matches!(parse_task_type("Reindex"), Ok(TaskType::Reindex)));
    }

    #[test]
    fn test_parse_task_type_invalid() {
        assert!(parse_task_type("invalid").is_err());
        assert!(parse_task_type("").is_err());
    }

    #[test]
    fn test_parse_sort_field_valid() {
        assert!(matches!(
            parse_sort_field("created_at"),
            Ok(SortField::CreatedAt)
        ));
        assert!(matches!(
            parse_sort_field("created"),
            Ok(SortField::CreatedAt)
        ));
        assert!(matches!(
            parse_sort_field("UPDATED_AT"),
            Ok(SortField::UpdatedAt)
        ));
        assert!(matches!(
            parse_sort_field("Updated"),
            Ok(SortField::UpdatedAt)
        ));
    }

    #[test]
    fn test_parse_sort_field_invalid() {
        assert!(parse_sort_field("invalid").is_err());
        assert!(parse_sort_field("").is_err());
    }

    #[test]
    fn test_parse_sort_order_valid() {
        assert!(matches!(parse_sort_order("asc"), Ok(SortOrder::Asc)));
        assert!(matches!(parse_sort_order("ascending"), Ok(SortOrder::Asc)));
        assert!(matches!(parse_sort_order("DESC"), Ok(SortOrder::Desc)));
        assert!(matches!(
            parse_sort_order("descending"),
            Ok(SortOrder::Desc)
        ));
    }

    #[test]
    fn test_parse_sort_order_invalid() {
        assert!(parse_sort_order("invalid").is_err());
        assert!(parse_sort_order("").is_err());
    }

    #[test]
    fn test_list_tasks_query_defaults() {
        let json = r#"{}"#;
        let query: Result<ListTasksQuery, _> = serde_json::from_str(json);
        assert!(query.is_ok());
        let q = query.unwrap();
        assert!(q.status.is_none());
        assert!(q.page.is_none());
        assert!(q.page_size.is_none());
    }

    #[test]
    fn test_pagination_info_serialization() {
        let info = PaginationInfo {
            total: 100,
            page: 1,
            page_size: 20,
            total_pages: 5,
        };
        let json = serde_json::to_string(&info);
        assert!(json.is_ok());
    }

    #[test]
    fn test_statistics_info_serialization() {
        let stats = StatisticsInfo {
            pending: 10,
            processing: 5,
            indexed: 85,
            failed: 0,
            cancelled: 0,
        };
        let json = serde_json::to_string(&stats);
        assert!(json.is_ok());
        assert!(json.unwrap().contains("\"pending\":10"));
    }
}
