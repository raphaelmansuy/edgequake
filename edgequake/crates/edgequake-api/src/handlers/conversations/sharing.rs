//! Sharing handlers for conversations.
//!
//! Implements share, unshare, and get-shared operations.

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

use super::share_api_path;

/// Share a conversation.
#[utoipa::path(
    post,
    path = "/api/v1/conversations/{id}/share",
    params(
        ("id" = Uuid, Path, description = "Conversation ID")
    ),
    responses(
        (status = 200, description = "Share link created", body = ShareResponse),
    ),
    tags = ["conversations"]
)]
pub async fn share_conversation(
    State(state): State<AppState>,
    _tenant_ctx: TenantContext,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ShareResponse>> {
    let share_id = state.conversation_service.share_conversation(id).await?;

    Ok(Json(ShareResponse {
        share_id: share_id.clone(),
        share_url: share_api_path(&share_id),
    }))
}

/// Unshare a conversation.
#[utoipa::path(
    delete,
    path = "/api/v1/conversations/{id}/share",
    params(
        ("id" = Uuid, Path, description = "Conversation ID")
    ),
    responses(
        (status = 204, description = "Share link removed"),
    ),
    tags = ["conversations"]
)]
pub async fn unshare_conversation(
    State(state): State<AppState>,
    _tenant_ctx: TenantContext,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    state.conversation_service.unshare_conversation(id).await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Get a shared conversation.
#[utoipa::path(
    get,
    path = "/api/v1/shared/{share_id}",
    params(
        ("share_id" = String, Path, description = "Share ID")
    ),
    responses(
        (status = 200, description = "Shared conversation", body = ConversationWithMessagesResponse),
        (status = 404, description = "Not found"),
    ),
    tags = ["conversations"]
)]
pub async fn get_shared_conversation(
    State(state): State<AppState>,
    Path(share_id): Path<String>,
) -> ApiResult<Json<ConversationWithMessagesResponse>> {
    let conversation = state
        .conversation_service
        .get_shared_conversation(&share_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Shared conversation not found".into()))?;

    let messages = state
        .conversation_service
        .list_messages(conversation.conversation_id, None, 200)
        .await?;

    Ok(Json(ConversationWithMessagesResponse {
        conversation: conversation.into(),
        messages: messages.items.into_iter().map(Into::into).collect(),
    }))
}
