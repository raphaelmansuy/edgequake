//! Conversation DTO types.
//!
//! This module contains all Data Transfer Objects for the conversations API.
//! Extracted from conversations.rs for modularity and single responsibility.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

// ============================================================================
// Default value helper functions
// ============================================================================

/// Default pagination limit for conversations.
pub fn conversations_default_limit() -> usize {
    20
}

/// Default sort field for conversations.
pub fn default_sort() -> String {
    "updated_at".to_string()
}

/// Default sort order.
pub fn default_order() -> String {
    "desc".to_string()
}

/// Default pagination limit for messages.
pub fn default_messages_limit() -> usize {
    50
}

/// Default streaming mode for conversations.
pub fn conversations_default_stream() -> bool {
    true
}

// ============================================================================
// Query Parameters
// ============================================================================

/// Pagination and filter parameters for listing conversations.
#[derive(Debug, Deserialize, ToSchema, utoipa::IntoParams)]
pub struct ListConversationsParams {
    /// Cursor for pagination.
    pub cursor: Option<String>,
    /// Maximum items to return (default 20, max 100).
    #[serde(default = "conversations_default_limit")]
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

/// Pagination parameters for listing messages.
#[derive(Debug, Deserialize, ToSchema, utoipa::IntoParams)]
pub struct ListMessagesParams {
    /// Cursor for pagination.
    pub cursor: Option<String>,
    /// Maximum items to return (default 50, max 200).
    #[serde(default = "default_messages_limit")]
    pub limit: usize,
}

// ============================================================================
// Response DTOs
// ============================================================================

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

/// Share response.
#[derive(Debug, Serialize, ToSchema)]
pub struct ShareResponse {
    /// Share ID.
    pub share_id: String,
    /// Share URL.
    pub share_url: String,
}

// ============================================================================
// Request DTOs
// ============================================================================

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
    #[serde(default = "conversations_default_stream")]
    pub stream: bool,
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

// ============================================================================
// Bulk Operation DTOs
// ============================================================================

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

// ============================================================================
// Import/Export DTOs
// ============================================================================

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

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conversations_default_limit() {
        assert_eq!(conversations_default_limit(), 20);
    }

    #[test]
    fn test_default_messages_limit() {
        assert_eq!(default_messages_limit(), 50);
    }

    #[test]
    fn test_default_sort() {
        assert_eq!(default_sort(), "updated_at");
    }

    #[test]
    fn test_default_order() {
        assert_eq!(default_order(), "desc");
    }

    #[test]
    fn test_conversations_default_stream() {
        assert!(conversations_default_stream());
    }

    #[test]
    fn test_pagination_meta_serialization() {
        let meta = PaginationMetaResponse {
            next_cursor: Some("cursor123".to_string()),
            prev_cursor: None,
            total: Some(100),
            has_more: true,
        };
        let json = serde_json::to_value(&meta).unwrap();
        assert_eq!(json["next_cursor"], "cursor123");
        assert!(json["has_more"].as_bool().unwrap());
    }

    #[test]
    fn test_share_response_serialization() {
        let share = ShareResponse {
            share_id: "abc123".to_string(),
            share_url: "https://example.com/share/abc123".to_string(),
        };
        let json = serde_json::to_value(&share).unwrap();
        assert_eq!(json["share_id"], "abc123");
        assert!(json["share_url"].as_str().unwrap().contains("share"));
    }

    #[test]
    fn test_bulk_operation_response_serialization() {
        let resp = BulkOperationResponse { affected: 5 };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["affected"], 5);
    }

    #[test]
    fn test_import_error_response_serialization() {
        let err = ImportErrorResponse {
            id: "conv123".to_string(),
            error: "Parse error".to_string(),
        };
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json["id"], "conv123");
        assert_eq!(json["error"], "Parse error");
    }

    #[test]
    fn test_create_conversation_request_deserialization() {
        let json = r#"{"title": "Test", "mode": "hybrid"}"#;
        let req: CreateConversationApiRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.title, Some("Test".to_string()));
        assert_eq!(req.mode, Some("hybrid".to_string()));
    }

    #[test]
    fn test_create_message_request_defaults() {
        let json = r#"{"content": "Hello", "role": "user"}"#;
        let req: CreateMessageApiRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.content, "Hello");
        assert!(req.stream); // default is true
    }

    #[test]
    fn test_bulk_archive_request_deserialization() {
        let json =
            r#"{"conversation_ids": ["550e8400-e29b-41d4-a716-446655440000"], "archive": true}"#;
        let req: BulkArchiveRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.conversation_ids.len(), 1);
        assert!(req.archive);
    }
}
