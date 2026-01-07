//! Conversation management handlers.
//!
//! Provides REST API endpoints for managing conversations, messages, and folders.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::middleware::TenantContext;
use crate::state::AppState;
use edgequake_core::types::{
    ConversationMode, ConversationSortField, CreateConversationRequest, CreateMessageRequest,
    MessageRole, UpdateConversationRequest, UpdateMessageRequest,
};

// ============ Request/Response DTOs ============

/// Pagination and filter parameters for listing conversations.
#[derive(Debug, Deserialize, ToSchema, utoipa::IntoParams)]
pub struct ListConversationsParams {
    /// Cursor for pagination.
    pub cursor: Option<String>,
    /// Maximum items to return (default 20, max 100).
    #[serde(default = "default_limit")]
    pub limit: usize,
    /// Filter by mode (comma-separated: local,global,hybrid).
    #[serde(rename = "filter[mode]")]
    pub filter_mode: Option<String>,
    /// Filter by archived status.
    #[serde(rename = "filter[archived]")]
    pub filter_archived: Option<bool>,
    /// Filter by pinned status.
    #[serde(rename = "filter[pinned]")]
    pub filter_pinned: Option<bool>,
    /// Filter by folder ID.
    #[serde(rename = "filter[folder_id]")]
    pub filter_folder_id: Option<Uuid>,
    /// Search in title.
    #[serde(rename = "filter[search]")]
    pub filter_search: Option<String>,
    /// Sort field (updated_at, created_at, title).
    #[serde(default = "default_sort")]
    pub sort: String,
    /// Sort order (asc, desc).
    #[serde(default = "default_order")]
    pub order: String,
}

fn default_limit() -> usize {
    20
}
fn default_sort() -> String {
    "updated_at".to_string()
}
fn default_order() -> String {
    "desc".to_string()
}

/// Pagination parameters for listing messages.
#[derive(Debug, Deserialize, ToSchema, utoipa::IntoParams)]
pub struct ListMessagesParams {
    /// Cursor for pagination.
    pub cursor: Option<String>,
    /// Maximum items to return (default 50, max 200).
    #[serde(default = "default_messages_limit")]
    pub limit: usize,
}

fn default_messages_limit() -> usize {
    50
}

/// Conversation response DTO.
#[derive(Debug, Serialize, ToSchema)]
pub struct ConversationResponse {
    /// Conversation ID.
    pub id: Uuid,
    /// Tenant ID.
    pub tenant_id: Uuid,
    /// Workspace ID.
    pub workspace_id: Option<Uuid>,
    /// Title.
    pub title: String,
    /// Query mode.
    pub mode: String,
    /// Pinned state.
    pub is_pinned: bool,
    /// Archived state.
    pub is_archived: bool,
    /// Folder ID.
    pub folder_id: Option<Uuid>,
    /// Share ID (if shared).
    pub share_id: Option<String>,
    /// Message count.
    pub message_count: Option<usize>,
    /// Preview of last message.
    pub last_message_preview: Option<String>,
    /// Creation timestamp.
    pub created_at: String,
    /// Last update timestamp.
    pub updated_at: String,
}

impl From<edgequake_core::Conversation> for ConversationResponse {
    fn from(c: edgequake_core::Conversation) -> Self {
        Self {
            id: c.conversation_id,
            tenant_id: c.tenant_id,
            workspace_id: c.workspace_id,
            title: c.title,
            mode: c.mode.to_string(),
            is_pinned: c.is_pinned,
            is_archived: c.is_archived,
            folder_id: c.folder_id,
            share_id: c.share_id,
            message_count: c.message_count,
            last_message_preview: c.last_message_preview,
            created_at: c.created_at.to_rfc3339(),
            updated_at: c.updated_at.to_rfc3339(),
        }
    }
}

/// Message response DTO.
#[derive(Debug, Serialize, ToSchema)]
pub struct MessageResponse {
    /// Message ID.
    pub id: Uuid,
    /// Conversation ID.
    pub conversation_id: Uuid,
    /// Parent message ID.
    pub parent_id: Option<Uuid>,
    /// Role (user, assistant, system).
    pub role: String,
    /// Content.
    pub content: String,
    /// Query mode used.
    pub mode: Option<String>,
    /// Tokens used.
    pub tokens_used: Option<i32>,
    /// Duration in ms.
    pub duration_ms: Option<i32>,
    /// Thinking time in ms.
    pub thinking_time_ms: Option<i32>,
    /// Context (sources, entities).
    pub context: Option<serde_json::Value>,
    /// Error state.
    pub is_error: bool,
    /// Creation timestamp.
    pub created_at: String,
    /// Last update timestamp.
    pub updated_at: String,
}

impl From<edgequake_core::Message> for MessageResponse {
    fn from(m: edgequake_core::Message) -> Self {
        Self {
            id: m.message_id,
            conversation_id: m.conversation_id,
            parent_id: m.parent_id,
            role: m.role.to_string(),
            content: m.content,
            mode: m.mode.map(|m| m.to_string()),
            tokens_used: m.tokens_used,
            duration_ms: m.duration_ms,
            thinking_time_ms: m.thinking_time_ms,
            context: m
                .context
                .map(|c| serde_json::to_value(c).unwrap_or_default()),
            is_error: m.is_error,
            created_at: m.created_at.to_rfc3339(),
            updated_at: m.updated_at.to_rfc3339(),
        }
    }
}

/// Folder response DTO.
#[derive(Debug, Serialize, ToSchema)]
pub struct FolderResponse {
    /// Folder ID.
    pub id: Uuid,
    /// Tenant ID.
    pub tenant_id: Uuid,
    /// Workspace ID.
    pub workspace_id: Option<Uuid>,
    /// Name.
    pub name: String,
    /// Parent folder ID.
    pub parent_id: Option<Uuid>,
    /// Position.
    pub position: i32,
    /// Creation timestamp.
    pub created_at: String,
    /// Last update timestamp.
    pub updated_at: String,
}

impl From<edgequake_core::Folder> for FolderResponse {
    fn from(f: edgequake_core::Folder) -> Self {
        Self {
            id: f.folder_id,
            tenant_id: f.tenant_id,
            workspace_id: f.workspace_id,
            name: f.name,
            parent_id: f.parent_id,
            position: f.position,
            created_at: f.created_at.to_rfc3339(),
            updated_at: f.updated_at.to_rfc3339(),
        }
    }
}

/// Paginated conversations response.
#[derive(Debug, Serialize, ToSchema)]
pub struct PaginatedConversationsResponse {
    /// Conversation items.
    pub items: Vec<ConversationResponse>,
    /// Pagination metadata.
    pub pagination: PaginationMetaResponse,
}

/// Paginated messages response.
#[derive(Debug, Serialize, ToSchema)]
pub struct PaginatedMessagesResponse {
    /// Message items.
    pub items: Vec<MessageResponse>,
    /// Pagination metadata.
    pub pagination: PaginationMetaResponse,
}

/// Pagination metadata response.
#[derive(Debug, Serialize, ToSchema)]
pub struct PaginationMetaResponse {
    /// Cursor for next page.
    pub next_cursor: Option<String>,
    /// Cursor for previous page.
    pub prev_cursor: Option<String>,
    /// Total count (optional).
    pub total: Option<usize>,
    /// Whether more items exist.
    pub has_more: bool,
}

/// Conversation with messages response.
#[derive(Debug, Serialize, ToSchema)]
pub struct ConversationWithMessagesResponse {
    /// Conversation details.
    pub conversation: ConversationResponse,
    /// Messages in the conversation.
    pub messages: Vec<MessageResponse>,
}

/// Create conversation request DTO.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateConversationApiRequest {
    /// Optional title.
    pub title: Option<String>,
    /// Query mode.
    pub mode: Option<String>,
    /// Folder ID.
    pub folder_id: Option<Uuid>,
}

/// Update conversation request DTO.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateConversationApiRequest {
    /// New title.
    pub title: Option<String>,
    /// New mode.
    pub mode: Option<String>,
    /// Pinned state.
    pub is_pinned: Option<bool>,
    /// Archived state.
    pub is_archived: Option<bool>,
    /// Folder ID.
    pub folder_id: Option<Uuid>,
}

/// Create message request DTO.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateMessageApiRequest {
    /// Message content.
    pub content: String,
    /// Role (user, assistant, system).
    pub role: String,
    /// Parent message ID.
    pub parent_id: Option<Uuid>,
    /// Whether to stream response.
    #[serde(default = "default_stream")]
    pub stream: bool,
}

fn default_stream() -> bool {
    true
}

/// Update message request DTO.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateMessageApiRequest {
    /// New content.
    pub content: Option<String>,
    /// Tokens used.
    pub tokens_used: Option<i32>,
    /// Duration in ms.
    pub duration_ms: Option<i32>,
    /// Thinking time in ms.
    pub thinking_time_ms: Option<i32>,
    /// Context.
    pub context: Option<serde_json::Value>,
    /// Error state.
    pub is_error: Option<bool>,
}

/// Create folder request DTO.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateFolderApiRequest {
    /// Folder name.
    pub name: String,
    /// Parent folder ID.
    pub parent_id: Option<Uuid>,
}

/// Update folder request DTO.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateFolderApiRequest {
    /// New name.
    pub name: Option<String>,
    /// New parent.
    pub parent_id: Option<Uuid>,
    /// New position.
    pub position: Option<i32>,
}

/// Bulk operation request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct BulkOperationRequest {
    /// Conversation IDs.
    pub conversation_ids: Vec<Uuid>,
}

/// Bulk archive request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct BulkArchiveRequest {
    /// Conversation IDs.
    pub conversation_ids: Vec<Uuid>,
    /// Archive state.
    pub archive: bool,
}

/// Bulk move request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct BulkMoveRequest {
    /// Conversation IDs.
    pub conversation_ids: Vec<Uuid>,
    /// Target folder ID.
    pub folder_id: Option<Uuid>,
}

/// Bulk operation response.
#[derive(Debug, Serialize, ToSchema)]
pub struct BulkOperationResponse {
    /// Number of items affected.
    pub affected: usize,
}

/// Import request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ImportConversationsRequest {
    /// Conversations to import (from localStorage).
    pub conversations: Vec<serde_json::Value>,
}

/// Import response.
#[derive(Debug, Serialize, ToSchema)]
pub struct ImportConversationsResponse {
    /// Number imported.
    pub imported: usize,
    /// Number failed.
    pub failed: usize,
    /// Errors.
    pub errors: Vec<ImportErrorResponse>,
}

/// Import error.
#[derive(Debug, Serialize, ToSchema)]
pub struct ImportErrorResponse {
    /// Conversation ID.
    pub id: String,
    /// Error message.
    pub error: String,
}

/// Share response.
#[derive(Debug, Serialize, ToSchema)]
pub struct ShareResponse {
    /// Share ID.
    pub share_id: String,
    /// Share URL.
    pub share_url: String,
}

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
