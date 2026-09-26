//! Thin operation-resource aliases over the task system (SPEC-120 P3).

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use edgequake_tasks::TaskStatus;
use serde::Serialize;
use serde_json::Value;
use utoipa::ToSchema;

use crate::{
    error::{ApiError, ApiResult},
    middleware::TenantContext,
    services::{cancel_track_with_doc_and_pdf_chain, task_scope::get_task_for_context},
    state::AppState,
};

use super::TaskResponse;

/// Accepted cancellation state for an operation.
#[derive(Debug, Serialize, ToSchema)]
pub struct OperationCancelResponse {
    /// `cancelling` while a worker drains; `cancelled` once terminal.
    pub state: String,
    /// Durable cancellation request timestamp.
    pub cancel_requested_at: Option<String>,
    /// Soft stop deadline (one lease heartbeat after the request).
    pub expected_stop_by: Option<String>,
}

/// Get an operation. Operations are a transparent alias over stored tasks.
#[utoipa::path(
    get,
    path = "/api/v1/operations/{id}",
    tag = "Operations",
    params(("id" = String, Path, description = "Operation/task tracking ID")),
    responses(
        (status = 200, description = "Operation found", body = TaskResponse),
        (status = 404, description = "Operation not found")
    )
)]
pub async fn get_operation(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Path(id): Path<String>,
) -> ApiResult<Json<TaskResponse>> {
    let task = get_task_for_context(&state, &id, &tenant_ctx).await?;
    let mut response = TaskResponse::from(task);
    #[cfg(feature = "postgres")]
    if let (Some(pool), Some(doc_id)) = (state.optional_pg_pool(), response.document_id.as_deref()) {
        response.document =
            crate::services::operation_document::load_operation_document_projection(pool, doc_id)
                .await;
    }
    Ok(Json(response))
}

/// Request cooperative cancellation of an operation.
#[utoipa::path(
    post,
    path = "/api/v1/operations/{id}/cancel",
    tag = "Operations",
    params(("id" = String, Path, description = "Operation/task tracking ID")),
    responses(
        (status = 202, description = "Cancellation accepted", body = OperationCancelResponse),
        (status = 404, description = "Operation not found"),
        (status = 409, description = "Operation is already successfully completed"),
        (status = 423, description = "Destructive operation already fenced")
    )
)]
pub async fn cancel_operation(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Path(id): Path<String>,
) -> ApiResult<(StatusCode, Json<OperationCancelResponse>)> {
    let task = get_task_for_context(&state, &id, &tenant_ctx).await?;
    let vector = crate::services::get_workspace_vector_storage_for_delete(
        &state,
        &task.workspace_id.to_string(),
    )
    .await?;

    #[cfg(feature = "postgres")]
    let applied = if let Some(ref pool) = state.pg_pool {
        let wake = crate::services::cancel_notify::PgCancelWake::new(pool.clone());
        crate::services::cancel_track_with_doc_and_pdf_chain_wake_pool(
            &state.tasks.storage,
            &state.tasks.cancellation_registry,
            state.storage.kv_storage.clone(),
            &state.storage.graph_storage,
            &vector,
            &id,
            &wake,
            Some(pool),
        )
        .await
    } else {
        cancel_track_with_doc_and_pdf_chain(
            &state.tasks.storage,
            &state.tasks.cancellation_registry,
            state.storage.kv_storage.clone(),
            &state.storage.graph_storage,
            &vector,
            &id,
        )
        .await
    }
    .map_err(ApiError::Internal)?;

    #[cfg(not(feature = "postgres"))]
    let applied = cancel_track_with_doc_and_pdf_chain(
        &state.tasks.storage,
        &state.tasks.cancellation_registry,
        state.storage.kv_storage.clone(),
        &state.storage.graph_storage,
        &vector,
        &id,
    )
    .await
    .map_err(ApiError::Internal)?;

    if applied.conflict_indexed {
        return Err(ApiError::Conflict(format!(
            "Cannot cancel operation in status: {}",
            TaskStatus::Indexed
        )));
    }

    if applied.operation_fenced {
        return Err(ApiError::operation_fenced(
            "Destructive operation already fenced; cancel is no longer allowed",
        ));
    }

    let state_name = if applied.cancelling {
        "cancelling"
    } else if applied.cancelled {
        "cancelled"
    } else {
        return Err(ApiError::Internal(
            "Cancellation did not produce an operation state".to_string(),
        ));
    };

    Ok((
        StatusCode::ACCEPTED,
        Json(OperationCancelResponse {
            state: state_name.to_string(),
            cancel_requested_at: applied.cancel_requested_at.map(|at| at.to_rfc3339()),
            expected_stop_by: applied.expected_stop_by.map(|at| at.to_rfc3339()),
        }),
    ))
}

/// List persisted events for an operation.
///
/// `task_events` is an optional forward-compatible projection. Deployments
/// without that table return an empty list.
#[utoipa::path(
    get,
    path = "/api/v1/operations/{id}/events",
    tag = "Operations",
    params(("id" = String, Path, description = "Operation/task tracking ID")),
    responses(
        (status = 200, description = "Operation events, ordered oldest first"),
        (status = 404, description = "Operation not found")
    )
)]
pub async fn get_operation_events(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Path(id): Path<String>,
) -> ApiResult<Json<Vec<Value>>> {
    // Enforce the same tenant/workspace ownership rule as the task resource.
    get_task_for_context(&state, &id, &tenant_ctx).await?;
    Ok(Json(load_task_events(&state, &id).await?))
}

#[cfg(feature = "postgres")]
async fn load_task_events(state: &AppState, id: &str) -> ApiResult<Vec<Value>> {
    let Some(pool) = state.optional_pg_pool() else {
        return Ok(Vec::new());
    };

    let exists: bool = sqlx::query_scalar("SELECT to_regclass('task_events') IS NOT NULL")
        .fetch_one(pool)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to inspect task_events: {e}")))?;
    if !exists {
        return Ok(Vec::new());
    }

    // Row-to-JSON keeps this stub compatible with evolving event schemas.
    sqlx::query_scalar::<_, Value>(
        r#"
        SELECT to_jsonb(event_row)
        FROM task_events AS event_row
        WHERE COALESCE(
            to_jsonb(event_row)->>'task_id',
            to_jsonb(event_row)->>'track_id',
            to_jsonb(event_row)->>'operation_id'
        ) = $1
        ORDER BY COALESCE(
            to_jsonb(event_row)->>'created_at',
            to_jsonb(event_row)->>'timestamp',
            ''
        ) ASC
        "#,
    )
    .bind(id)
    .fetch_all(pool)
    .await
    .map_err(|e| ApiError::Internal(format!("Failed to load operation events: {e}")))
}

#[cfg(not(feature = "postgres"))]
async fn load_task_events(_state: &AppState, _id: &str) -> ApiResult<Vec<Value>> {
    Ok(Vec::new())
}
