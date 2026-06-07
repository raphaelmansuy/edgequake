//! Shared conversation row types for memory and postgres adapters (SPEC-017).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Conversation persisted row.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConversationRow {
    pub conversation_id: Uuid,
    pub tenant_id: Uuid,
    pub workspace_id: Option<Uuid>,
    pub user_id: Uuid,
    pub title: String,
    pub mode: String,
    pub is_pinned: bool,
    pub is_archived: bool,
    pub folder_id: Option<Uuid>,
    pub share_id: Option<String>,
    pub meta: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Message persisted row.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MessageRow {
    pub message_id: Uuid,
    pub conversation_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub role: String,
    pub content: String,
    pub mode: Option<String>,
    pub tokens_used: Option<i32>,
    pub duration_ms: Option<i32>,
    pub thinking_time_ms: Option<i32>,
    pub context: Option<serde_json::Value>,
    pub is_error: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Folder persisted row.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FolderRow {
    pub folder_id: Uuid,
    pub tenant_id: Uuid,
    pub workspace_id: Option<Uuid>,
    pub user_id: Uuid,
    pub name: String,
    pub parent_id: Option<Uuid>,
    pub position: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
