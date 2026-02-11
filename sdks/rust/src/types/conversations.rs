//! Conversation types.

use serde::{Deserialize, Serialize};

/// Create conversation request.
#[derive(Debug, Clone, Serialize)]
pub struct CreateConversationRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder_id: Option<String>,
}

/// Conversation summary.
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
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

/// Detailed conversation with messages.
#[derive(Debug, Clone, Deserialize)]
pub struct ConversationDetail {
    pub id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub messages: Vec<Message>,
    #[serde(default)]
    pub created_at: Option<String>,
}

/// A message in a conversation.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Message {
    pub id: String,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub created_at: Option<String>,
}

/// Create message request.
#[derive(Debug, Clone, Serialize)]
pub struct CreateMessageRequest {
    #[serde(default = "default_role")]
    pub role: String,
    pub content: String,
}

fn default_role() -> String {
    "user".to_string()
}

/// Share link response.
#[derive(Debug, Clone, Deserialize)]
pub struct ShareLink {
    pub share_id: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub expires_at: Option<String>,
}

/// Bulk delete response.
#[derive(Debug, Clone, Deserialize)]
pub struct BulkDeleteResponse {
    #[serde(default)]
    pub deleted_count: u32,
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
