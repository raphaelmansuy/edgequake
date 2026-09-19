//! HTTP handlers for workspace copy (EN-3677 Phase 1).
//!
//! Two endpoints:
//!   * `POST /api/v1/workspaces/{source_workspace_id}/copy` — start / retrieve
//!     an idempotent copy job.
//!   * `GET  /api/v1/workspace-copy-jobs/{job_id}` — poll job status.
//!
//! The `DELETE /api/v1/workspaces/{workspace_id}` route already exists in
//! `workspace_crud.rs` and satisfies the cleanup contract that
//! rag-service (on copy failure) and doc-store (MILVUS_DROP step) rely on.
//! No new delete handler is added here.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use super::helpers::verify_workspace_tenant_access;
use crate::error::ApiError;
use crate::handlers::workspaces_types::{
    CopyMode, CopyStatus, CopyWorkspaceRequest, CopyWorkspaceResponse, CopyWorkspaceStatus,
};
use crate::middleware::TenantContext;
use crate::services::workspace_copy::{
    execute_copy_plan, CopyJobStore, CopyPlan, GetOrCreateOutcome, NewCopyJob,
};
use crate::state::AppState;

/// Copy a workspace (same-model, same-tenant).
///
/// # Semantics
///
/// - Rejects `dest_embedding_model = Some(_)` with 400 `NOT_IMPLEMENTED_V1`.
/// - Idempotent on `request_id` — a repeat call returns the existing job's
///   state (200 sync / 202 async), never 409.
/// - `async_mode = false` waits for the copy to finish then returns 200.
/// - `async_mode = true` returns 202 with `job_id`; poll the status route.
///
/// # Guards
///
/// - `verify_workspace_tenant_access` — source workspace must belong to the
///   caller's `X-Tenant-ID`.
/// - `pg_try_advisory_xact_lock` on both workspaces + ingest-in-flight
///   scan of the `tasks` table are executed inside the copy transaction
///   (see `services::workspace_copy::lock`).
///
/// POST /api/v1/workspaces/{source_workspace_id}/copy
#[utoipa::path(
    post,
    path = "/api/v1/workspaces/{source_workspace_id}/copy",
    params(
        ("source_workspace_id" = Uuid, Path, description = "Source workspace ID"),
    ),
    request_body = CopyWorkspaceRequest,
    responses(
        (status = 200, description = "Copy completed (sync mode)", body = CopyWorkspaceResponse),
        (status = 202, description = "Copy accepted (async mode)", body = CopyWorkspaceResponse),
        (status = 400, description = "Invalid request (e.g. NOT_IMPLEMENTED_V1)"),
        (status = 404, description = "Source workspace not found"),
        (status = 409, description = "Copy blocked by lock or in-flight ingestion"),
    ),
    tags = ["workspaces"]
)]
pub async fn copy_workspace(
    State(state): State<AppState>,
    Path(source_workspace_id): Path<Uuid>,
    tenant_ctx: TenantContext,
    Json(request): Json<CopyWorkspaceRequest>,
) -> Result<(StatusCode, Json<CopyWorkspaceResponse>), ApiError> {
    // ── v1 guard: reject cross-model requests before doing anything else ──
    if request.dest_embedding_model.is_some()
        || request.dest_embedding_provider.is_some()
        || request.dest_embedding_dimension.is_some()
    {
        return Err(ApiError::BadRequest(
            "NOT_IMPLEMENTED_V1: cross-model re-embed is deferred to v2; \
             omit dest_embedding_* fields for same-model copy"
                .to_string(),
        ));
    }

    // ── tenant isolation on the source workspace ──
    let source = verify_workspace_tenant_access(&state, source_workspace_id, &tenant_ctx).await?;

    // Provisional destination workspace id — the real create step lives in
    // the copy service (Phase 2 wiring); the handler generates the id now
    // so the response and job record can share it.
    let dest_workspace_id = Uuid::new_v4();

    let store = copy_job_store(&state);

    let outcome = store
        .get_or_create(NewCopyJob {
            request_id: request.request_id.clone(),
            tenant_id: Some(source.tenant_id),
            source_workspace_id,
            dest_workspace_id,
            dest_slug: request.dest_slug.clone(),
            mode: CopyMode::SameModel,
        })
        .await;

    let (record, is_new) = match outcome {
        GetOrCreateOutcome::Existing(rec) => (rec, false),
        GetOrCreateOutcome::Created(rec) => (rec, true),
    };

    let job_id = record.job_id;

    if is_new {
        let plan = CopyPlan {
            job_id,
            source_workspace_id,
            dest_workspace_id: record.dest_workspace_id,
            tenant_namespace: source.tenant_id.simple().to_string(),
            mode: CopyMode::SameModel,
        };

        if request.async_mode {
            let store_clone = store.clone();
            tokio::spawn(async move {
                if let Err(e) = execute_copy_plan(&store_clone, plan).await {
                    store_clone
                        .mark_failed(job_id, "COPY_FAILED", format!("{e}"))
                        .await;
                }
            });
        } else if let Err(e) = execute_copy_plan(&store, plan).await {
            store.mark_failed(job_id, "COPY_FAILED", format!("{e}")).await;
            return Err(ApiError::Internal(format!(
                "workspace copy failed: {e}"
            )));
        }
    }

    // Re-read after any state mutations so the response reflects fresh
    // counters (sync path) or the initial pending state (async path).
    let latest = store.get(job_id).await.unwrap_or(record);

    let response = CopyWorkspaceResponse {
        source_workspace_id: latest.source_workspace_id,
        dest_workspace_id: latest.dest_workspace_id,
        dest_slug: latest.dest_slug.clone(),
        mode: latest.mode,
        status: latest.status,
        job_id: if request.async_mode {
            Some(latest.job_id)
        } else {
            None
        },
        chunks_copied: latest.chunks_copied,
        entities_copied: latest.entities_copied,
        relationships_copied: latest.relationships_copied,
        documents_copied: latest.documents_copied,
        vectors_copied: latest.vectors_copied,
        elapsed_ms: latest.elapsed_ms,
    };

    let status_code = if request.async_mode && latest.status != CopyStatus::Completed {
        StatusCode::ACCEPTED
    } else {
        StatusCode::OK
    };

    Ok((status_code, Json(response)))
}

/// Poll the status of a workspace copy job.
///
/// GET /api/v1/workspace-copy-jobs/{job_id}
#[utoipa::path(
    get,
    path = "/api/v1/workspace-copy-jobs/{job_id}",
    params(
        ("job_id" = Uuid, Path, description = "Workspace copy job ID"),
    ),
    responses(
        (status = 200, description = "Job status", body = CopyWorkspaceStatus),
        (status = 404, description = "Job not found"),
    ),
    tags = ["workspaces"]
)]
pub async fn get_workspace_copy_job(
    State(state): State<AppState>,
    Path(job_id): Path<Uuid>,
    _tenant_ctx: TenantContext,
) -> Result<Json<CopyWorkspaceStatus>, ApiError> {
    let store = copy_job_store(&state);
    let record = store
        .get(job_id)
        .await
        .ok_or_else(|| ApiError::NotFound(format!("Workspace copy job {job_id} not found")))?;
    Ok(Json(record.to_status()))
}

/// Resolve the shared job store from `AppState`.
///
/// **Phase 1 note**: `AppState` does not yet carry a `CopyJobStore` field —
/// wiring it in requires a state constructor change that is deferred to
/// the follow-up commit. Until then, this helper returns a fresh store on
/// every call which effectively disables cross-request idempotency. This
/// is called out explicitly in the commit message.
fn copy_job_store(_state: &AppState) -> CopyJobStore {
    // TODO(EN-3677 Phase 2): thread `CopyJobStore` through `AppState` and
    // return `state.copy_job_store.clone()` here.
    lazy_static::lazy_static! {
        static ref STORE: CopyJobStore = CopyJobStore::new();
    }
    STORE.clone()
}
