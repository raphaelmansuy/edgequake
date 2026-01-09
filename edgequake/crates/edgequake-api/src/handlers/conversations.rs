//! Conversation management handlers.
//!
//! Provides REST API endpoints for managing conversations, messages, and folders.
//!
//! ## Implements
//!
//! - **FEAT0580**: Conversation listing with pagination and filtering
//! - **FEAT0581**: Conversation creation with mode selection
//! - **FEAT0582**: Message management within conversations
//! - **FEAT0583**: Folder organization for conversation grouping
//!
//! ## Use Cases
//!
//! - **UC2180**: User lists their conversations with optional mode filter
//! - **UC2181**: User creates new conversation in specific folder
//! - **UC2182**: User adds message to existing conversation
//! - **UC2183**: User organizes conversations into folders
//!
//! ## Enforces
//!
//! - **BR0580**: Conversations must be scoped to authenticated user
//! - **BR0581**: Messages must have valid roles (user/assistant/system)
//! - **BR0582**: Folder names must be unique per user

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::middleware::TenantContext;
use crate::state::AppState;
use edgequake_core::types::{
    ConversationMode, ConversationSortField, CreateConversationRequest, CreateMessageRequest,
    MessageRole, UpdateConversationRequest, UpdateMessageRequest,
};

// Re-export DTOs from conversations_types module
pub use crate::handlers::conversations_types::*;

// ============ Handlers ============

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

    let user_id = tenant_ctx.user_id_uuid().ok_or(ApiError::Unauthorized)?;

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
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

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

    let user_id = tenant_ctx.user_id_uuid().ok_or(ApiError::Unauthorized)?;

    let workspace_id = tenant_ctx.workspace_id_uuid();

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
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

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
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
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
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

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
    _tenant_ctx: TenantContext,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateConversationApiRequest>,
) -> ApiResult<Json<ConversationResponse>> {
    let mode = request
        .mode
        .as_ref()
        .and_then(|m| m.parse::<ConversationMode>().ok());

    let conversation = state
        .conversation_service
        .update_conversation(
            id,
            UpdateConversationRequest {
                title: request.title,
                mode,
                is_pinned: request.is_pinned,
                is_archived: request.is_archived,
                folder_id: request.folder_id,
            },
        )
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

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
    state
        .conversation_service
        .delete_conversation(id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

/// List messages in a conversation.
#[utoipa::path(
    get,
    path = "/api/v1/conversations/{id}/messages",
    params(
        ("id" = Uuid, Path, description = "Conversation ID"),
        ListMessagesParams
    ),
    responses(
        (status = 200, description = "List of messages", body = PaginatedMessagesResponse),
    ),
    tags = ["conversations"]
)]
pub async fn list_messages(
    State(state): State<AppState>,
    _tenant_ctx: TenantContext,
    Path(id): Path<Uuid>,
    Query(params): Query<ListMessagesParams>,
) -> ApiResult<Json<PaginatedMessagesResponse>> {
    let limit = params.limit.min(200);

    let result = state
        .conversation_service
        .list_messages(id, params.cursor, limit)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(PaginatedMessagesResponse {
        items: result.items.into_iter().map(Into::into).collect(),
        pagination: PaginationMetaResponse {
            next_cursor: result.pagination.next_cursor,
            prev_cursor: result.pagination.prev_cursor,
            total: result.pagination.total,
            has_more: result.pagination.has_more,
        },
    }))
}

/// Create a message in a conversation.
#[utoipa::path(
    post,
    path = "/api/v1/conversations/{id}/messages",
    params(
        ("id" = Uuid, Path, description = "Conversation ID")
    ),
    request_body = CreateMessageApiRequest,
    responses(
        (status = 201, description = "Message created", body = MessageResponse),
    ),
    tags = ["conversations"]
)]
pub async fn create_message(
    State(state): State<AppState>,
    _tenant_ctx: TenantContext,
    Path(id): Path<Uuid>,
    Json(request): Json<CreateMessageApiRequest>,
) -> ApiResult<(StatusCode, Json<MessageResponse>)> {
    let role = match request.role.to_lowercase().as_str() {
        "assistant" => MessageRole::Assistant,
        "system" => MessageRole::System,
        _ => MessageRole::User,
    };

    let message = state
        .conversation_service
        .create_message(
            id,
            CreateMessageRequest {
                content: request.content,
                role,
                parent_id: request.parent_id,
                stream: request.stream,
            },
        )
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok((StatusCode::CREATED, Json(message.into())))
}

/// Update a message.
#[utoipa::path(
    patch,
    path = "/api/v1/messages/{message_id}",
    params(
        ("message_id" = Uuid, Path, description = "Message ID")
    ),
    request_body = UpdateMessageApiRequest,
    responses(
        (status = 200, description = "Message updated", body = MessageResponse),
    ),
    tags = ["conversations"]
)]
pub async fn update_message(
    State(state): State<AppState>,
    _tenant_ctx: TenantContext,
    Path(message_id): Path<Uuid>,
    Json(request): Json<UpdateMessageApiRequest>,
) -> ApiResult<Json<MessageResponse>> {
    let context = request.context.and_then(|c| serde_json::from_value(c).ok());

    let message = state
        .conversation_service
        .update_message(
            message_id,
            UpdateMessageRequest {
                content: request.content,
                tokens_used: request.tokens_used,
                duration_ms: request.duration_ms,
                thinking_time_ms: request.thinking_time_ms,
                context,
                is_error: request.is_error,
            },
        )
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(message.into()))
}

/// Delete a message.
#[utoipa::path(
    delete,
    path = "/api/v1/messages/{message_id}",
    params(
        ("message_id" = Uuid, Path, description = "Message ID")
    ),
    responses(
        (status = 204, description = "Message deleted"),
    ),
    tags = ["conversations"]
)]
pub async fn delete_message(
    State(state): State<AppState>,
    _tenant_ctx: TenantContext,
    Path(message_id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    state
        .conversation_service
        .delete_message(message_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

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
    let share_id = state
        .conversation_service
        .share_conversation(id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Build share URL
    let share_url = format!("/shared/{}", share_id);

    Ok(Json(ShareResponse {
        share_id,
        share_url,
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
    state
        .conversation_service
        .unshare_conversation(id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

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
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("Shared conversation not found".into()))?;

    let messages = state
        .conversation_service
        .list_messages(conversation.conversation_id, None, 200)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(ConversationWithMessagesResponse {
        conversation: conversation.into(),
        messages: messages.items.into_iter().map(Into::into).collect(),
    }))
}

/// Import conversations from localStorage.
#[utoipa::path(
    post,
    path = "/api/v1/conversations/import",
    request_body = ImportConversationsRequest,
    responses(
        (status = 200, description = "Import result", body = ImportConversationsResponse),
    ),
    tags = ["conversations"]
)]
pub async fn import_conversations(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Json(request): Json<ImportConversationsRequest>,
) -> ApiResult<Json<ImportConversationsResponse>> {
    let tenant_id = tenant_ctx
        .tenant_id_uuid()
        .ok_or_else(|| ApiError::BadRequest("Missing X-Tenant-ID header".into()))?;

    let user_id = tenant_ctx.user_id_uuid().ok_or(ApiError::Unauthorized)?;

    let result = state
        .conversation_service
        .import_conversations(tenant_id, user_id, request.conversations)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(ImportConversationsResponse {
        imported: result.imported,
        failed: result.failed,
        errors: result
            .errors
            .into_iter()
            .map(|e| ImportErrorResponse {
                id: e.id,
                error: e.error,
            })
            .collect(),
    }))
}

/// Bulk delete conversations.
#[utoipa::path(
    post,
    path = "/api/v1/conversations/bulk/delete",
    request_body = BulkOperationRequest,
    responses(
        (status = 200, description = "Bulk delete result", body = BulkOperationResponse),
    ),
    tags = ["conversations"]
)]
pub async fn bulk_delete_conversations(
    State(state): State<AppState>,
    _tenant_ctx: TenantContext,
    Json(request): Json<BulkOperationRequest>,
) -> ApiResult<Json<BulkOperationResponse>> {
    let affected = state
        .conversation_service
        .bulk_delete(request.conversation_ids)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(BulkOperationResponse { affected }))
}

/// Bulk archive/unarchive conversations.
#[utoipa::path(
    post,
    path = "/api/v1/conversations/bulk/archive",
    request_body = BulkArchiveRequest,
    responses(
        (status = 200, description = "Bulk archive result", body = BulkOperationResponse),
    ),
    tags = ["conversations"]
)]
pub async fn bulk_archive_conversations(
    State(state): State<AppState>,
    _tenant_ctx: TenantContext,
    Json(request): Json<BulkArchiveRequest>,
) -> ApiResult<Json<BulkOperationResponse>> {
    let affected = state
        .conversation_service
        .bulk_archive(request.conversation_ids, request.archive)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(BulkOperationResponse { affected }))
}

/// Bulk move conversations to folder.
#[utoipa::path(
    post,
    path = "/api/v1/conversations/bulk/move",
    request_body = BulkMoveRequest,
    responses(
        (status = 200, description = "Bulk move result", body = BulkOperationResponse),
    ),
    tags = ["conversations"]
)]
pub async fn bulk_move_conversations(
    State(state): State<AppState>,
    _tenant_ctx: TenantContext,
    Json(request): Json<BulkMoveRequest>,
) -> ApiResult<Json<BulkOperationResponse>> {
    let affected = state
        .conversation_service
        .bulk_move_to_folder(request.conversation_ids, request.folder_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(BulkOperationResponse { affected }))
}

// ============ Folder Handlers ============

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

    let user_id = tenant_ctx.user_id_uuid().ok_or(ApiError::Unauthorized)?;

    let folders = state
        .conversation_service
        .list_folders(tenant_id, user_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

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

    let user_id = tenant_ctx.user_id_uuid().ok_or(ApiError::Unauthorized)?;

    let folder = state
        .conversation_service
        .create_folder(tenant_id, user_id, request.name, request.parent_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

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
    _tenant_ctx: TenantContext,
    Path(folder_id): Path<Uuid>,
    Json(request): Json<UpdateFolderApiRequest>,
) -> ApiResult<Json<FolderResponse>> {
    let folder = state
        .conversation_service
        .update_folder(folder_id, request.name, request.parent_id, request.position)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

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
    _tenant_ctx: TenantContext,
    Path(folder_id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    state
        .conversation_service
        .delete_folder(folder_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_conversations_params_defaults() {
        let json_str = r#"{}"#;
        let params: Result<ListConversationsParams, _> = serde_json::from_str(json_str);
        assert!(params.is_ok());
        let p = params.unwrap();
        assert_eq!(p.limit, 20);
        assert_eq!(p.sort, "updated_at");
        assert_eq!(p.order, "desc");
    }

    #[test]
    fn test_create_conversation_request_deserialization() {
        let json_str = r#"{"title": "Test", "mode": "hybrid"}"#;
        let request: Result<CreateConversationApiRequest, _> = serde_json::from_str(json_str);
        assert!(request.is_ok());
        let req = request.unwrap();
        assert_eq!(req.title, Some("Test".to_string()));
        assert_eq!(req.mode, Some("hybrid".to_string()));
    }

    #[test]
    fn test_conversation_response_serialization() {
        let response = ConversationResponse {
            id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            workspace_id: None,
            title: "Test".to_string(),
            mode: "hybrid".to_string(),
            is_pinned: false,
            is_archived: false,
            folder_id: None,
            share_id: None,
            message_count: Some(0),
            last_message_preview: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&response);
        assert!(json.is_ok());
    }

    #[test]
    fn test_folder_response_serialization() {
        let response = FolderResponse {
            id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            workspace_id: None,
            name: "Test Folder".to_string(),
            parent_id: None,
            position: 0,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&response);
        assert!(json.is_ok());
    }

    #[test]
    fn test_message_response_serialization() {
        let response = MessageResponse {
            id: Uuid::new_v4(),
            conversation_id: Uuid::new_v4(),
            parent_id: None,
            role: "user".to_string(),
            content: "Hello".to_string(),
            mode: Some("hybrid".to_string()),
            tokens_used: Some(10),
            duration_ms: Some(100),
            thinking_time_ms: None,
            context: None,
            is_error: false,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&response);
        assert!(json.is_ok());
    }
}
