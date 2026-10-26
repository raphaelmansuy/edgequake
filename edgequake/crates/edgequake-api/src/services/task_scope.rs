//! Scoped task access — DRY workspace isolation for v1 tasks and v2 jobs.

use edgequake_tasks::{Task, TaskFilter};

use crate::error::{ApiError, ApiResult};
use crate::middleware::{
    resolve_tenant_header, resolve_workspace_header, ScopeHeader, TenantContext,
};
use crate::state::AppState;

/// Build the tenant/workspace portion of a task filter.
///
/// Explicit query values take precedence for admin/debug clients; normal
/// requests inherit the authenticated header context.
pub fn task_filter_for_scope(
    tenant_ctx: &TenantContext,
    tenant_id: Option<&str>,
    workspace_id: Option<&str>,
) -> TaskFilter {
    TaskFilter {
        tenant_id: tenant_id
            .and_then(|value| uuid::Uuid::parse_str(value).ok())
            .or_else(|| tenant_ctx.tenant_id_uuid()),
        workspace_id: workspace_id
            .and_then(|value| uuid::Uuid::parse_str(value).ok())
            .or_else(|| tenant_ctx.workspace_id_uuid()),
        ..TaskFilter::default()
    }
}

/// Load a task when it belongs to the requester's tenant+workspace (404 if cross-tenant).
///
/// SPEC-091 IW0 (GAP-091-10, LAW-I4): the workspace check is UNCONDITIONAL —
/// a headerless request resolves to the built-in default workspace explicitly,
/// and a malformed scope header fails closed (404, never a wildcard match).
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

    let not_found = || ApiError::NotFound(format!("Task not found: {track_id}"));

    let ctx_workspace_id = match resolve_workspace_header(tenant_ctx.workspace_id.as_deref()) {
        ScopeHeader::Resolved(id) => id,
        ScopeHeader::Absent => crate::middleware::default_workspace_uuid(),
        ScopeHeader::Malformed => return Err(not_found()),
    };
    if task.workspace_id != ctx_workspace_id {
        return Err(not_found());
    }

    let ctx_tenant_id = match resolve_tenant_header(tenant_ctx.tenant_id.as_deref()) {
        ScopeHeader::Resolved(id) => id,
        ScopeHeader::Absent => crate::middleware::default_tenant_uuid(),
        ScopeHeader::Malformed => return Err(not_found()),
    };
    if task.tenant_id != ctx_tenant_id {
        return Err(not_found());
    }

    if let Some(document_id) = task.document_id() {
        if crate::services::clearance::must_hide(&document_id).await? {
            return Err(not_found());
        }
    } else if let Some(pdf_id) = task.pdf_id() {
        if crate::services::clearance::must_hide_pdf(pdf_id, Some(ctx_workspace_id)).await? {
            return Err(not_found());
        }
    }
    Ok(task)
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_tasks::{Task, TaskType};
    use uuid::Uuid;

    #[test]
    fn task_filter_scope_uses_context_and_explicit_override() {
        let context_tenant = Uuid::new_v4();
        let context_workspace = Uuid::new_v4();
        let explicit_workspace = Uuid::new_v4();
        let context = TenantContext {
            tenant_id: Some(context_tenant.to_string()),
            workspace_id: Some(context_workspace.to_string()),
            user_id: None,
        };

        let inherited = task_filter_for_scope(&context, None, None);
        assert_eq!(inherited.tenant_id, Some(context_tenant));
        assert_eq!(inherited.workspace_id, Some(context_workspace));

        let overridden =
            task_filter_for_scope(&context, None, Some(&explicit_workspace.to_string()));
        assert_eq!(overridden.tenant_id, Some(context_tenant));
        assert_eq!(overridden.workspace_id, Some(explicit_workspace));
    }

    #[tokio::test]
    async fn e2e_cancel_foreign_track_id_404() {
        let state = AppState::test_state();
        let ws_owner = Uuid::new_v4();
        let ws_other = Uuid::new_v4();
        let task = Task::new(
            Uuid::new_v4(),
            ws_owner,
            TaskType::Insert,
            serde_json::json!({}),
        );
        let track_id = task.track_id.clone();
        state
            .tasks
            .storage
            .create_task(&task)
            .await
            .expect("create task");

        let foreign = TenantContext {
            tenant_id: Some(Uuid::new_v4().to_string()),
            workspace_id: Some(ws_other.to_string()),
            user_id: None,
        };
        let err = get_task_for_context(&state, &track_id, &foreign)
            .await
            .expect_err("foreign workspace must 404");
        assert!(
            matches!(err, ApiError::NotFound(_)),
            "expected NotFound, got {err:?}"
        );
    }

    #[tokio::test]
    async fn e2e_pdf_progress_foreign_404() {
        // Same ownership gate used by PDF progress / WS filtered progress.
        let state = AppState::test_state();
        let ws_owner = Uuid::new_v4();
        let task = Task::new(
            Uuid::new_v4(),
            ws_owner,
            TaskType::Upload,
            serde_json::json!({"pdf_id": Uuid::new_v4().to_string()}),
        );
        let track_id = task.track_id.clone();
        state
            .tasks
            .storage
            .create_task(&task)
            .await
            .expect("create task");

        let foreign = TenantContext {
            tenant_id: None,
            workspace_id: Some(Uuid::new_v4().to_string()),
            user_id: None,
        };
        let err = get_task_for_context(&state, &track_id, &foreign)
            .await
            .expect_err("foreign PDF progress must 404");
        assert!(matches!(err, ApiError::NotFound(_)));
    }

    #[tokio::test]
    async fn headerless_ctx_sees_default_scope_task() {
        // Dev-mode anonymous flow: no headers → default tenant+workspace, and a
        // task stored under those defaults must remain visible (SPEC-091 IW0).
        use crate::middleware::{default_tenant_uuid, default_workspace_uuid};
        let state = AppState::test_state();
        let task = Task::new(
            default_tenant_uuid(),
            default_workspace_uuid(),
            TaskType::Insert,
            serde_json::json!({}),
        );
        let track_id = task.track_id.clone();
        state
            .tasks
            .storage
            .create_task(&task)
            .await
            .expect("create task");

        let anonymous = TenantContext {
            tenant_id: None,
            workspace_id: None,
            user_id: None,
        };
        get_task_for_context(&state, &track_id, &anonymous)
            .await
            .expect("headerless ctx must see default-scope task");
    }

    #[tokio::test]
    async fn headerless_ctx_cannot_see_foreign_workspace_task() {
        // GAP-091-10: previously the workspace check was skipped when the
        // header was absent, leaking foreign tasks to anonymous callers.
        let state = AppState::test_state();
        let task = Task::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            TaskType::Insert,
            serde_json::json!({}),
        );
        let track_id = task.track_id.clone();
        state
            .tasks
            .storage
            .create_task(&task)
            .await
            .expect("create task");

        let anonymous = TenantContext {
            tenant_id: None,
            workspace_id: None,
            user_id: None,
        };
        let err = get_task_for_context(&state, &track_id, &anonymous)
            .await
            .expect_err("headerless ctx must not see foreign-workspace task");
        assert!(matches!(err, ApiError::NotFound(_)));
    }

    #[tokio::test]
    async fn malformed_scope_header_fails_closed() {
        // GAP-091-08/10: a malformed X-Workspace-ID is not a wildcard — deny.
        use crate::middleware::{default_tenant_uuid, default_workspace_uuid};
        let state = AppState::test_state();
        let task = Task::new(
            default_tenant_uuid(),
            default_workspace_uuid(),
            TaskType::Insert,
            serde_json::json!({}),
        );
        let track_id = task.track_id.clone();
        state
            .tasks
            .storage
            .create_task(&task)
            .await
            .expect("create task");

        let malformed = TenantContext {
            tenant_id: None,
            workspace_id: Some("not-a-uuid".to_string()),
            user_id: None,
        };
        let err = get_task_for_context(&state, &track_id, &malformed)
            .await
            .expect_err("malformed workspace header must 404, not wildcard-match");
        assert!(matches!(err, ApiError::NotFound(_)));
    }
}
