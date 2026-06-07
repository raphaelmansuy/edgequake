//! Folder handlers for conversation organization.
//!
//! Implements list, create, update, and delete for folders.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::handlers::conversations_types::*;
use crate::middleware::TenantContext;
use crate::state::AppState;

/// List folders.
#[utoipa::path(
    get,
    path = "/api/v1/folders",
    responses(
        (status = 200, description = "List of folders", body = Vec<FolderResponse>),
    ),
    tags = ["folders"]
)]
pub async fn list_folders(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
) -> ApiResult<Json<Vec<FolderResponse>>> {
    let tenant_id = tenant_ctx
        .tenant_id_uuid()
        .ok_or_else(|| ApiError::BadRequest("Missing X-Tenant-ID header".into()))?;

    let user_id = tenant_ctx.user_id_uuid().ok_or(ApiError::unauthorized())?;

    let folders = state
        .conversation_service
        .list_folders(tenant_id, user_id)
        .await?;

    Ok(Json(folders.into_iter().map(Into::into).collect()))
}

/// Create a folder.
#[utoipa::path(
    post,
    path = "/api/v1/folders",
    request_body = CreateFolderApiRequest,
    responses(
        (status = 201, description = "Folder created", body = FolderResponse),
    ),
    tags = ["folders"]
)]
pub async fn create_folder(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Json(request): Json<CreateFolderApiRequest>,
) -> ApiResult<(StatusCode, Json<FolderResponse>)> {
    let tenant_id = tenant_ctx
        .tenant_id_uuid()
        .ok_or_else(|| ApiError::BadRequest("Missing X-Tenant-ID header".into()))?;

    let user_id = tenant_ctx.user_id_uuid().ok_or(ApiError::unauthorized())?;

    let folder = state
        .conversation_service
        .create_folder(tenant_id, user_id, request.name, request.parent_id)
        .await?;

    Ok((StatusCode::CREATED, Json(folder.into())))
}

/// Update a folder.
#[utoipa::path(
    patch,
    path = "/api/v1/folders/{folder_id}",
    params(
        ("folder_id" = Uuid, Path, description = "Folder ID")
    ),
    request_body = UpdateFolderApiRequest,
    responses(
        (status = 200, description = "Folder updated", body = FolderResponse),
    ),
    tags = ["folders"]
)]
pub async fn update_folder(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Path(folder_id): Path<Uuid>,
    Json(request): Json<UpdateFolderApiRequest>,
) -> ApiResult<Json<FolderResponse>> {
    let tenant_id = tenant_ctx
        .tenant_id_uuid()
        .ok_or_else(|| ApiError::BadRequest("Missing X-Tenant-ID header".into()))?;

    let user_id = tenant_ctx.user_id_uuid().ok_or(ApiError::unauthorized())?;

    let folder = state
        .conversation_service
        .update_folder(
            tenant_id,
            user_id,
            folder_id,
            request.name,
            request.parent_id,
            request.position,
        )
        .await?;

    Ok(Json(folder.into()))
}

/// Delete a folder.
#[utoipa::path(
    delete,
    path = "/api/v1/folders/{folder_id}",
    params(
        ("folder_id" = Uuid, Path, description = "Folder ID")
    ),
    responses(
        (status = 204, description = "Folder deleted"),
    ),
    tags = ["folders"]
)]
pub async fn delete_folder(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Path(folder_id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    let tenant_id = tenant_ctx
        .tenant_id_uuid()
        .ok_or_else(|| ApiError::BadRequest("Missing X-Tenant-ID header".into()))?;

    let user_id = tenant_ctx.user_id_uuid().ok_or(ApiError::unauthorized())?;

    state
        .conversation_service
        .delete_folder(tenant_id, user_id, folder_id)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}
