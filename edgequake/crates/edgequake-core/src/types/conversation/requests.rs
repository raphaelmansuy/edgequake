//! Request, filter, pagination, and import types for conversations.
//!
//! These types define the API contract for creating, updating, filtering,
//! and importing conversations and messages.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::context::MessageContext;
use super::enums::{ConversationMode, MessageRole};
use super::models::{Conversation, Message};

/// Request to create a new conversation.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct CreateConversationRequest {
    /// Optional title (defaults to "New Conversation").
    pub title: Option<String>,
    /// Query mode (defaults to hybrid).
    pub mode: Option<ConversationMode>,
    /// Optional folder to place in.
    pub folder_id: Option<Uuid>,
}

/// Request to update a conversation.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct UpdateConversationRequest {
    /// New title.
    pub title: Option<String>,
    /// New mode.
    pub mode: Option<ConversationMode>,
    /// Pin state.
    pub is_pinned: Option<bool>,
    /// Archive state.
    pub is_archived: Option<bool>,
    /// New folder.
    /// - `None`: don't change folder assignment
    /// - `Some(None)`: remove from folder (set folder_id to null)
    /// - `Some(Some(uuid))`: move to folder with this UUID
    ///
    /// WHY: Double-option pattern required to distinguish between
    /// "no change" and "explicitly remove from folder" in PATCH semantics.
    pub folder_id: Option<Option<Uuid>>,
}

/// Request to create a new message.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateMessageRequest {
    /// Message content.
    pub content: String,
    /// Message role.
    pub role: MessageRole,
    /// Parent message ID.
    pub parent_id: Option<Uuid>,
    /// Whether to stream the response.
    #[serde(default = "default_stream")]
    pub stream: bool,
}

fn default_stream() -> bool {
    true
}

/// Request to update a message.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct UpdateMessageRequest {
    /// New content.
    pub content: Option<String>,
    /// Tokens used.
    pub tokens_used: Option<i32>,
    /// Duration in milliseconds.
    pub duration_ms: Option<i32>,
    /// Thinking time in milliseconds.
    pub thinking_time_ms: Option<i32>,
    /// Context (sources, entities).
    pub context: Option<MessageContext>,
    /// Error state.
    pub is_error: Option<bool>,
}

/// Request to create a folder.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateFolderRequest {
    /// Folder name.
    pub name: String,
    /// Parent folder ID.
    pub parent_id: Option<Uuid>,
}

/// Request to update a folder.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct UpdateFolderRequest {
    /// New name.
    pub name: Option<String>,
    /// New parent.
    pub parent_id: Option<Uuid>,
    /// New position.
    pub position: Option<i32>,
}

/// Filter parameters for listing conversations.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ConversationFilter {
    /// Filter by modes.
    pub mode: Option<Vec<ConversationMode>>,
    /// Filter by archived state.
    pub archived: Option<bool>,
    /// Filter by pinned state.
    pub pinned: Option<bool>,
    /// Filter by folder.
    pub folder_id: Option<Uuid>,
    /// Filter for conversations without any folder (unfiled).
    /// When true, returns only conversations where folder_id IS NULL.
    pub unfiled: Option<bool>,
    /// Full-text search in title.
    pub search: Option<String>,
    /// Filter by date range (from).
    pub date_from: Option<chrono::DateTime<chrono::Utc>>,
    /// Filter by date range (to).
    pub date_to: Option<chrono::DateTime<chrono::Utc>>,
}

/// Sort field for conversations.
#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversationSortField {
    /// Sort by last update time.
    #[default]
    UpdatedAt,
    /// Sort by creation time.
    CreatedAt,
    /// Sort by title.
    Title,
}

/// Cursor-based pagination metadata.
#[derive(Debug, Clone, Serialize, Default)]
pub struct PaginationMeta {
    /// Cursor for next page.
    pub next_cursor: Option<String>,
    /// Cursor for previous page.
    pub prev_cursor: Option<String>,
    /// Total count (optional, expensive to compute).
    pub total: Option<usize>,
    /// Whether there are more items.
    pub has_more: bool,
}

/// Paginated list of conversations.
#[derive(Debug, Clone, Serialize)]
pub struct PaginatedConversations {
    /// Conversation items.
    pub items: Vec<Conversation>,
    /// Pagination metadata.
    pub pagination: PaginationMeta,
}

/// Paginated list of messages.
#[derive(Debug, Clone, Serialize)]
pub struct PaginatedMessages {
    /// Message items.
    pub items: Vec<Message>,
    /// Pagination metadata.
    pub pagination: PaginationMeta,
}

/// Result of import operation.
#[derive(Debug, Clone, Serialize)]
pub struct ImportResult {
    /// Number of successfully imported conversations.
    pub imported: usize,
    /// Number of failed imports.
    pub failed: usize,
    /// Individual errors.
    pub errors: Vec<ImportError>,
}

/// An individual import error.
#[derive(Debug, Clone, Serialize)]
pub struct ImportError {
    /// ID of the conversation that failed.
    pub id: String,
    /// Error message.
    pub error: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_conversation_request_default() {
        let req = CreateConversationRequest::default();
        assert!(req.title.is_none());
        assert!(req.mode.is_none());
        assert!(req.folder_id.is_none());
    }

    #[test]
    fn test_update_conversation_request_default() {
        let req = UpdateConversationRequest::default();
        assert!(req.title.is_none());
        assert!(req.mode.is_none());
        assert!(req.is_pinned.is_none());
        assert!(req.is_archived.is_none());
        assert!(req.folder_id.is_none());
    }

    #[test]
    fn test_update_message_request_default() {
        let req = UpdateMessageRequest::default();
        assert!(req.content.is_none());
        assert!(req.tokens_used.is_none());
        assert!(req.context.is_none());
        assert!(req.is_error.is_none());
    }

    #[test]
    fn test_update_folder_request_default() {
        let req = UpdateFolderRequest::default();
        assert!(req.name.is_none());
        assert!(req.position.is_none());
    }

    #[test]
    fn test_conversation_filter_default() {
        let f = ConversationFilter::default();
        assert!(f.mode.is_none());
        assert!(f.archived.is_none());
        assert!(f.search.is_none());
        assert!(f.unfiled.is_none());
    }

    #[test]
    fn test_conversation_sort_field_default() {
        assert!(matches!(ConversationSortField::default(), ConversationSortField::UpdatedAt));
    }

    #[test]
    fn test_pagination_meta_default() {
        let meta = PaginationMeta::default();
        assert!(meta.next_cursor.is_none());
        assert!(meta.prev_cursor.is_none());
        assert!(meta.total.is_none());
        assert!(!meta.has_more);
    }

    #[test]
    fn test_default_stream_is_true() {
        assert!(default_stream());
    }
}
