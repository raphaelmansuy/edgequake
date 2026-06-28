//! v2 job HTTP handlers (SPEC-027 IMP-025).

use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::IntoResponse,
    Json,
};
use edgequake_tasks::{Task, TaskType};
use serde_json::json;

use crate::error::{ApiError, ApiResult};
use crate::handlers::tasks::{cancel_task, list_tasks_response};
use crate::handlers::tasks_types::ListTasksQuery;
use crate::handlers::v2::jobs::types::{CreateJobRequest, JobListResponse, JobResponse};
use crate::middleware::TenantContext;
use crate::services::task_scope::get_task_for_context;
use crate::state::AppState;

fn parse_job_type(raw: &str) -> ApiResult<TaskType> {
    match raw.to_ascii_lowercase().as_str() {
        "upload" => Ok(TaskType::Upload),
        "insert" => Ok(TaskType::Insert),
        "scan" => Ok(TaskType::Scan),
        "reindex" => Ok(TaskType::Reindex),
        "pdf_processing" => Ok(TaskType::PdfProcessing),
        "knowledge_injection" => Ok(TaskType::KnowledgeInjection),
        other => Err(ApiError::BadRequest(format!(
            "Unsupported job_type '{other}'. Supported: upload, insert, scan, reindex, pdf_processing, knowledge_injection"
        ))),
    }
}

/// Submit a new async job (202 Accepted + Location header).
#[utoipa::path(
    post,
    path = "/api/v2/jobs",
    tag = "Jobs",
    request_body = CreateJobRequest,
    responses(
        (status = 202, description = "Job accepted", body = JobResponse),
        (status = 400, description = "Invalid job type or missing tenant context")
    )
)]
pub async fn create_job(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Json(request): Json<CreateJobRequest>,
) -> ApiResult<impl IntoResponse> {
    let tenant_id = tenant_ctx
        .tenant_id_uuid()
        .ok_or_else(|| ApiError::BadRequest("X-Tenant-ID required".into()))?;
    let workspace_id = tenant_ctx
        .workspace_id_uuid()
        .ok_or_else(|| ApiError::BadRequest("X-Workspace-ID required".into()))?;

    let task_type = parse_job_type(&request.job_type)?;
    let payload = if request.payload.is_null() {
        json!({})
    } else {
        request.payload
    };

    let task = Task::new(tenant_id, workspace_id, task_type, payload);
    let job_id = task.track_id.clone();
    state.enqueue_task(task).await?;

    let loaded = get_task_for_context(&state, &job_id, &tenant_ctx).await?;
    let job = JobResponse::from_task(&loaded);
    let location = format!("/api/v2/jobs/{job_id}");

    Ok((
        StatusCode::ACCEPTED,
        [(header::LOCATION, location)],
        Json(job),
    ))
}

/// Get job status by ID.
#[utoipa::path(
    get,
    path = "/api/v2/jobs/{job_id}",
    tag = "Jobs",
    params(("job_id" = String, Path, description = "Job ID (same as v1 track_id)")),
    responses(
        (status = 200, description = "Job found", body = JobResponse),
        (status = 404, description = "Job not found")
    )
)]
pub async fn get_job(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Path(job_id): Path<String>,
) -> ApiResult<Json<JobResponse>> {
    let task = get_task_for_context(&state, &job_id, &tenant_ctx).await?;
    Ok(Json(JobResponse::from_task(&task)))
}

/// List jobs for the requester's tenant/workspace (v2 wrapper over task list).
#[utoipa::path(
    get,
    path = "/api/v2/jobs",
    tag = "Jobs",
    params(
        ("status" = Option<String>, Query, description = "Filter by status"),
        ("page" = Option<u32>, Query, description = "Page number"),
        ("page_size" = Option<u32>, Query, description = "Page size")
    ),
    responses(
        (status = 200, description = "Jobs listed", body = JobListResponse)
    )
)]
pub async fn list_jobs(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Query(params): Query<ListTasksQuery>,
) -> ApiResult<Json<JobListResponse>> {
    let task_list = list_tasks_response(&state, &tenant_ctx, params).await?;
    Ok(Json(JobListResponse::from_task_list(task_list)))
}

/// Cancel a pending job (delegates to v1 task cancel semantics).
#[utoipa::path(
    post,
    path = "/api/v2/jobs/{job_id}/cancel",
    tag = "Jobs",
    params(("job_id" = String, Path, description = "Job ID")),
    responses(
        (status = 200, description = "Job cancelled", body = JobResponse),
        (status = 404, description = "Job not found"),
        (status = 409, description = "Job not cancellable")
    )
)]
pub async fn cancel_job(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Path(job_id): Path<String>,
) -> ApiResult<Json<JobResponse>> {
    let task_response = cancel_task(State(state), tenant_ctx, Path(job_id)).await?;
    Ok(Json(JobResponse::from_task_response(&task_response)))
}
