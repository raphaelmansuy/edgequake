//! v2 job HTTP handlers — Level 4 workspace-scoped REST (SPEC-027 IMP-025).

use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::IntoResponse,
    Json,
};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::handlers::tasks::{cancel_task, list_tasks_response};
use crate::handlers::tasks_types::ListTasksQuery;
use crate::handlers::v2::jobs::submission::{ensure_workspace_scope, submit_workspace_job};
use crate::handlers::v2::jobs::types::{CreateJobRequest, JobListResponse, JobResponse};
use crate::middleware::TenantContext;
use crate::services::job_registry::{job_catalog, JobCatalogResponse};
use crate::services::task_scope::get_task_for_context;
use crate::state::AppState;

fn job_location(workspace_id: Uuid, job_id: &str) -> String {
    format!("/api/v2/workspaces/{workspace_id}/jobs/{job_id}")
}

/// List supported job types for this workspace (Level 4 discovery).
#[utoipa::path(
    get,
    path = "/api/v2/workspaces/{workspace_id}/jobs/catalog",
    tag = "Jobs",
    params(("workspace_id" = Uuid, Path, description = "Workspace ID")),
    responses(
        (status = 200, description = "Job type catalog", body = JobCatalogResponse),
        (status = 400, description = "Path workspace_id must match X-Workspace-ID header")
    )
)]
pub async fn list_workspace_job_catalog(
    Path(workspace_id): Path<Uuid>,
    tenant_ctx: TenantContext,
) -> ApiResult<Json<JobCatalogResponse>> {
    ensure_workspace_scope(workspace_id, &tenant_ctx)?;
    Ok(Json(job_catalog(&workspace_id.to_string())))
}

/// Submit a new async job under a workspace (202 Accepted + Location).
#[utoipa::path(
    post,
    path = "/api/v2/workspaces/{workspace_id}/jobs",
    tag = "Jobs",
    params(("workspace_id" = Uuid, Path, description = "Workspace ID")),
    request_body = CreateJobRequest,
    responses(
        (status = 202, description = "Job accepted", body = JobResponse),
        (status = 400, description = "Invalid job type, payload, or workspace scope")
    )
)]
pub async fn create_workspace_job(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
    tenant_ctx: TenantContext,
    Json(request): Json<CreateJobRequest>,
) -> ApiResult<impl IntoResponse> {
    let submitted = submit_workspace_job(&state, workspace_id, &tenant_ctx, &request).await?;
    let ws = workspace_id.to_string();
    let job = match get_task_for_context(&state, &submitted.job_id, &tenant_ctx).await {
        Ok(task) => JobResponse::from_task_for_workspace(&ws, &task),
        Err(ApiError::NotFound(_)) => JobResponse::synthetic_accepted(
            &ws,
            &tenant_ctx,
            &submitted.job_id,
            &submitted.job_type,
        ),
        Err(e) => return Err(e),
    };
    Ok((
        StatusCode::ACCEPTED,
        [
            (
                header::LOCATION,
                job_location(workspace_id, &submitted.job_id),
            ),
            (
                header::LINK,
                format!(
                    "<{}>; rel=\"self\"",
                    job_location(workspace_id, &submitted.job_id)
                ),
            ),
        ],
        Json(job),
    ))
}

/// Get job status by ID within a workspace.
#[utoipa::path(
    get,
    path = "/api/v2/workspaces/{workspace_id}/jobs/{job_id}",
    tag = "Jobs",
    params(
        ("workspace_id" = Uuid, Path, description = "Workspace ID"),
        ("job_id" = String, Path, description = "Job ID (track_id)")
    ),
    responses(
        (status = 200, description = "Job found", body = JobResponse),
        (status = 404, description = "Job not found")
    )
)]
pub async fn get_workspace_job(
    State(state): State<AppState>,
    Path((workspace_id, job_id)): Path<(Uuid, String)>,
    tenant_ctx: TenantContext,
) -> ApiResult<Json<JobResponse>> {
    ensure_workspace_scope(workspace_id, &tenant_ctx)?;
    let task = get_task_for_context(&state, &job_id, &tenant_ctx).await?;
    Ok(Json(JobResponse::from_task_for_workspace(
        &workspace_id.to_string(),
        &task,
    )))
}

/// List jobs for a workspace (paginated).
#[utoipa::path(
    get,
    path = "/api/v2/workspaces/{workspace_id}/jobs",
    tag = "Jobs",
    params(
        ("workspace_id" = Uuid, Path, description = "Workspace ID"),
        ("status" = Option<String>, Query, description = "Filter by status"),
        ("page" = Option<u32>, Query, description = "Page number"),
        ("page_size" = Option<u32>, Query, description = "Page size")
    ),
    responses(
        (status = 200, description = "Jobs listed", body = JobListResponse)
    )
)]
pub async fn list_workspace_jobs(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
    tenant_ctx: TenantContext,
    Query(params): Query<ListTasksQuery>,
) -> ApiResult<Json<JobListResponse>> {
    ensure_workspace_scope(workspace_id, &tenant_ctx)?;
    let task_list = list_tasks_response(&state, &tenant_ctx, params).await?;
    Ok(Json(JobListResponse::from_task_list(
        &workspace_id.to_string(),
        task_list,
    )))
}

/// Cancel a pending job (Level 4 — DELETE on job resource).
#[utoipa::path(
    delete,
    path = "/api/v2/workspaces/{workspace_id}/jobs/{job_id}",
    tag = "Jobs",
    params(
        ("workspace_id" = Uuid, Path, description = "Workspace ID"),
        ("job_id" = String, Path, description = "Job ID")
    ),
    responses(
        (status = 200, description = "Job cancelled", body = JobResponse),
        (status = 404, description = "Job not found"),
        (status = 409, description = "Job not cancellable")
    )
)]
pub async fn cancel_workspace_job(
    State(state): State<AppState>,
    Path((workspace_id, job_id)): Path<(Uuid, String)>,
    tenant_ctx: TenantContext,
) -> ApiResult<Json<JobResponse>> {
    ensure_workspace_scope(workspace_id, &tenant_ctx)?;
    let task_response = cancel_task(State(state), tenant_ctx, Path(job_id.clone())).await?;
    Ok(Json(JobResponse::from_task_response_for_workspace(
        &workspace_id.to_string(),
        &task_response,
    )))
}
