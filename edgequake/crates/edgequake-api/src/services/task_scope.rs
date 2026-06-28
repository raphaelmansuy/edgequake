//! Scoped task access — DRY workspace isolation for v1 tasks and v2 jobs.

use edgequake_tasks::Task;

use crate::error::{ApiError, ApiResult};
use crate::middleware::TenantContext;
use crate::state::AppState;

/// Load a task when it belongs to the requester's workspace (404 if cross-tenant).
pub async fn get_task_for_context(
    state: &AppState,
    track_id: &str,
    tenant_ctx: &TenantContext,
) -> ApiResult<Task> {
    let task = state
        .tasks
        .storage
        .get_task(track_id)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to get task: {e}")))?
        .ok_or_else(|| ApiError::NotFound(format!("Task not found: {track_id}")))?;

    if let Some(ctx_workspace_id) = tenant_ctx.workspace_id_uuid() {
        if task.workspace_id != ctx_workspace_id {
            return Err(ApiError::NotFound(format!("Task not found: {track_id}")));
        }
    }

    Ok(task)
}
