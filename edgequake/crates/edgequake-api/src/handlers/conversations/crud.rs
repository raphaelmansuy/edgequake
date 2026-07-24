//! CRUD handlers for conversations.
//!
//! Implements list, create, get, update, and delete operations.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::handlers::conversations_types::*;
use crate::middleware::TenantContext;
use crate::state::AppState;
use edgequake_core::types::{
    ConversationMode, ConversationSortField, CreateConversationRequest, UpdateConversationRequest,
};

/// List conversations for the authenticated user.
#[utoipa::path(
    get,
    path = "/api/v1/conversations",
    params(ListConversationsParams),
    responses(
        (status = 200, description = "List of conversations", body = PaginatedConversationsResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
    ),
    tags = ["conversations"]
)]
pub async fn list_conversations(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Query(params): Query<ListConversationsParams>,
) -> ApiResult<Json<PaginatedConversationsResponse>> {
    let tenant_id = tenant_ctx
        .tenant_id_uuid()
        .ok_or_else(|| ApiError::BadRequest("Missing X-Tenant-ID header".into()))?;

    let user_id = tenant_ctx.user_id_uuid().ok_or(ApiError::unauthorized())?;

    // Parse filter modes
    let filter_modes = params.filter_mode.map(|s| {
        s.split(',')
            .filter_map(|m| m.parse::<ConversationMode>().ok())
            .collect()
    });

    let filter = edgequake_core::ConversationFilter {
        mode: filter_modes,
        archived: params.filter_archived,
        pinned: params.filter_pinned,
        folder_id: params.filter_folder_id,
        unfiled: params.filter_unfiled,
        search: params.filter_search,
        date_from: None,
        date_to: None,
    };

    let sort = match params.sort.as_str() {
        "created_at" => ConversationSortField::CreatedAt,
        "title" => ConversationSortField::Title,
        _ => ConversationSortField::UpdatedAt,
    };

    let sort_desc = params.order != "asc";
    let limit = params.limit.min(100);

    let result = state
        .conversation_service
        .list_conversations(
            tenant_id,
            user_id,
            filter,
            sort,
            sort_desc,
            params.cursor,
            limit,
        )
        .await?;

    Ok(Json(PaginatedConversationsResponse {
        items: result.items.into_iter().map(Into::into).collect(),
        pagination: PaginationMetaResponse {
            next_cursor: result.pagination.next_cursor,
            prev_cursor: result.pagination.prev_cursor,
            total: result.pagination.total,
            has_more: result.pagination.has_more,
        },
    }))
}

/// Create a new conversation.
#[utoipa::path(
    post,
    path = "/api/v1/conversations",
    request_body = CreateConversationApiRequest,
    responses(
        (status = 201, description = "Conversation created", body = ConversationResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
    ),
    tags = ["conversations"]
)]
pub async fn create_conversation(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Json(request): Json<CreateConversationApiRequest>,
) -> ApiResult<(StatusCode, Json<ConversationResponse>)> {
    let tenant_id = tenant_ctx
        .tenant_id_uuid()
        .ok_or_else(|| ApiError::BadRequest("Missing X-Tenant-ID header".into()))?;

    let client_user_id = tenant_ctx.user_id_uuid().ok_or(ApiError::unauthorized())?;

    let workspace_id = tenant_ctx.workspace_id_uuid();

    let user_id = super::super::postgres_user_bootstrap::ensure_postgres_user_exists(
        &state,
        tenant_id,
        client_user_id,
    )
    .await?;

    let mode = request
        .mode
        .as_ref()
        .and_then(|m| m.parse::<ConversationMode>().ok());

    let conversation = state
        .conversation_service
        .create_conversation(
            tenant_id,
            user_id,
            workspace_id,
            CreateConversationRequest {
                title: request.title,
                mode,
                folder_id: request.folder_id,
            },
        )
        .await?;

    Ok((StatusCode::CREATED, Json(conversation.into())))
}

/// Get a conversation by ID.
#[utoipa::path(
    get,
    path = "/api/v1/conversations/{id}",
    params(
        ("id" = Uuid, Path, description = "Conversation ID")
    ),
    responses(
        (status = 200, description = "Conversation details with messages", body = ConversationWithMessagesResponse),
        (status = 404, description = "Not found"),
    ),
    tags = ["conversations"]
)]
pub async fn get_conversation(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ConversationWithMessagesResponse>> {
    let conversation = state
        .conversation_service
        .get_conversation(id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Conversation not found".into()))?;

    // Verify tenant access - RLS policies handle user-level access
    let tenant_id = tenant_ctx
        .tenant_id_uuid()
        .ok_or_else(|| ApiError::BadRequest("Invalid tenant ID".into()))?;
    if conversation.tenant_id != tenant_id {
        return Err(ApiError::NotFound("Conversation not found".into()));
    }

    // Fetch messages
    let messages = state
        .conversation_service
        .list_messages(id, None, 200)
        .await?;

    Ok(Json(ConversationWithMessagesResponse {
        conversation: conversation.into(),
        messages: messages.items.into_iter().map(Into::into).collect(),
    }))
}

/// Update a conversation.
#[utoipa::path(
    patch,
    path = "/api/v1/conversations/{id}",
    params(
        ("id" = Uuid, Path, description = "Conversation ID")
    ),
    request_body = UpdateConversationApiRequest,
    responses(
        (status = 200, description = "Conversation updated", body = ConversationResponse),
        (status = 404, description = "Not found"),
    ),
    tags = ["conversations"]
)]
pub async fn update_conversation(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateConversationApiRequest>,
) -> ApiResult<Json<ConversationResponse>> {
    let tenant_id = tenant_ctx
        .tenant_id_uuid()
        .ok_or_else(|| ApiError::BadRequest("Missing X-Tenant-ID header".into()))?;

    let user_id = tenant_ctx.user_id_uuid().ok_or(ApiError::unauthorized())?;

    let mode = request
        .mode
        .as_ref()
        .and_then(|m| m.parse::<ConversationMode>().ok());

    let conversation = state
        .conversation_service
        .update_conversation(
            tenant_id,
            user_id,
            id,
            UpdateConversationRequest {
                title: request.title,
                mode,
                is_pinned: request.is_pinned,
                is_archived: request.is_archived,
                folder_id: request.folder_id,
            },
        )
        .await?;

    Ok(Json(conversation.into()))
}

/// Delete a conversation.
#[utoipa::path(
    delete,
    path = "/api/v1/conversations/{id}",
    params(
        ("id" = Uuid, Path, description = "Conversation ID")
    ),
    responses(
        (status = 204, description = "Conversation deleted"),
        (status = 404, description = "Not found"),
    ),
    tags = ["conversations"]
)]
pub async fn delete_conversation(
    State(state): State<AppState>,
    _tenant_ctx: TenantContext,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    state.conversation_service.delete_conversation(id).await?;

    Ok(StatusCode::NO_CONTENT)
}
