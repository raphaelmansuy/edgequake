//! Conversation types — aligned with `handlers/conversations_types` in edgequake-api.

use serde::{Deserialize, Serialize};

/// Create conversation request.
#[derive(Debug, Clone, Serialize)]
pub struct CreateConversationRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder_id: Option<String>,
}

/// Cursor pagination (conversations + messages lists).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct CursorPaginationMeta {
    #[serde(default)]
    pub next_cursor: Option<String>,
    #[serde(default)]
    pub prev_cursor: Option<String>,
    #[serde(default)]
    pub total: Option<usize>,
    #[serde(default)]
    pub has_more: bool,
}

/// Query for `GET /api/v1/conversations` (see `ListConversationsParams` in edgequake-api).
#[derive(Debug, Clone, Default)]
pub struct ConversationListQuery {
    pub cursor: Option<String>,
    pub limit: Option<u32>,
    pub filter_mode: Option<String>,
    pub filter_archived: Option<bool>,
    pub filter_pinned: Option<bool>,
    pub filter_folder_id: Option<String>,
    pub filter_unfiled: Option<bool>,
    pub filter_search: Option<String>,
    pub sort: Option<String>,
    pub order: Option<String>,
}

/// Query for `GET /api/v1/conversations/{id}/messages`.
#[derive(Debug, Clone, Default)]
pub struct ListMessagesQuery {
    pub cursor: Option<String>,
    pub limit: Option<u32>,
}

/// Serialize list-conversation filters to a query string (no leading `?`).
pub fn conversation_list_query_string(q: &ConversationListQuery) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(ref c) = q.cursor {
        parts.push(format!("cursor={}", urlencoding::encode(c)));
    }
    if let Some(l) = q.limit {
        parts.push(format!("limit={l}"));
    }
    if let Some(ref m) = q.filter_mode {
        parts.push(format!(
            "filter[mode]={}",
            urlencoding::encode(m)
        ));
    }
    if let Some(b) = q.filter_archived {
        parts.push(format!("filter[archived]={b}"));
    }
    if let Some(b) = q.filter_pinned {
        parts.push(format!("filter[pinned]={b}"));
    }
    if let Some(ref id) = q.filter_folder_id {
        parts.push(format!(
            "filter[folder_id]={}",
            urlencoding::encode(id)
        ));
    }
    if let Some(b) = q.filter_unfiled {
        parts.push(format!("filter[unfiled]={b}"));
    }
    if let Some(ref s) = q.filter_search {
        parts.push(format!(
            "filter[search]={}",
            urlencoding::encode(s)
        ));
    }
    if let Some(ref s) = q.sort {
        parts.push(format!("sort={}", urlencoding::encode(s)));
    }
    if let Some(ref o) = q.order {
        parts.push(format!("order={}", urlencoding::encode(o)));
    }
    parts.join("&")
}

/// Serialize list-messages query (no leading `?`).
pub fn list_messages_query_string(q: &ListMessagesQuery) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(ref c) = q.cursor {
        parts.push(format!("cursor={}", urlencoding::encode(c)));
    }
    if let Some(l) = q.limit {
        parts.push(format!("limit={l}"));
    }
    parts.join("&")
}

/// Conversation summary (list item).
#[derive(Debug, Clone, Deserialize)]
pub struct ConversationInfo {
    pub id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub folder_id: Option<String>,
    #[serde(default)]
    pub message_count: u32,
    #[serde(default)]
    pub is_pinned: bool,
    #[serde(default)]
    pub is_archived: bool,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub tenant_id: Option<String>,
    #[serde(default)]
    pub workspace_id: Option<String>,
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub share_id: Option<String>,
    #[serde(default)]
    pub last_message_preview: Option<String>,
}

/// Header object nested under `conversation` in
/// `GET /conversations/{id}`, `GET /shared/{share_id}`.
#[derive(Debug, Clone, Deserialize)]
pub struct ConversationHeader {
    pub id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub is_pinned: bool,
    #[serde(default)]
    pub is_archived: bool,
    #[serde(default)]
    pub folder_id: Option<String>,
    #[serde(default)]
    pub share_id: Option<String>,
    #[serde(default)]
    pub message_count: Option<usize>,
    #[serde(default)]
    pub last_message_preview: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub tenant_id: Option<String>,
    #[serde(default)]
    pub workspace_id: Option<String>,
    #[serde(default)]
    pub mode: Option<String>,
}

/// `GET /api/v1/conversations` — paginated wrapper.
#[derive(Debug, Clone, Deserialize)]
pub struct PaginatedConversations {
    #[serde(default)]
    pub items: Vec<ConversationInfo>,
    #[serde(default)]
    pub pagination: CursorPaginationMeta,
}

/// `GET /api/v1/conversations/{id}/messages` — paginated wrapper.
#[derive(Debug, Clone, Deserialize)]
pub struct PaginatedMessages {
    #[serde(default)]
    pub items: Vec<Message>,
    #[serde(default)]
    pub pagination: CursorPaginationMeta,
}

/// `GET /conversations/{id}` and `GET /shared/{share_id}` body.
#[derive(Debug, Clone, Deserialize)]
pub struct ConversationDetail {
    pub conversation: ConversationHeader,
    #[serde(default)]
    pub messages: Vec<Message>,
}

/// A message in a conversation.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Message {
    pub id: String,
    #[serde(default)]
    pub conversation_id: Option<String>,
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub tokens_used: Option<i32>,
    #[serde(default)]
    pub duration_ms: Option<i32>,
    #[serde(default)]
    pub thinking_time_ms: Option<i32>,
    #[serde(default)]
    pub context: Option<serde_json::Value>,
    #[serde(default)]
    pub is_error: bool,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

/// Create message request.
#[derive(Debug, Clone, Serialize)]
pub struct CreateMessageRequest {
    #[serde(default = "default_role")]
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

fn default_role() -> String {
    "user".to_string()
}

/// PATCH `/api/v1/messages/{id}` body (all fields optional).
#[derive(Debug, Clone, Serialize, Default)]
pub struct UpdateMessageRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens_used: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_time_ms: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

/// Share link response (`POST /conversations/{id}/share`).
#[derive(Debug, Clone, Deserialize)]
pub struct ShareLink {
    pub share_id: String,
    /// Canonical `share_url`; API may also send legacy `url`.
    #[serde(default, alias = "url")]
    pub share_url: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub expires_at: Option<String>,
}

/// Bulk delete / archive / move response (`affected` count).
#[derive(Debug, Clone, Deserialize)]
pub struct BulkConversationOpResponse {
    #[serde(default, alias = "deleted_count")]
    pub affected: usize,
}

/// Folder info.
#[derive(Debug, Clone, Deserialize)]
pub struct FolderInfo {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub conversation_count: u32,
}

/// Create folder request.
#[derive(Debug, Clone, Serialize)]
pub struct CreateFolderRequest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
}
