//! Conversation and message types for query history persistence.
//!
//! This module defines the domain entities for managing conversations and messages
//! in the EdgeQuake query interface.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Conversation mode for RAG queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ConversationMode {
    /// Local search only (entity-based).
    Local,
    /// Global search only (community summaries).
    Global,
    /// Hybrid search (combines local and global).
    #[default]
    Hybrid,
    /// Naive search (simple vector similarity).
    Naive,
    /// Mix mode (weighted combination).
    Mix,
}

impl std::fmt::Display for ConversationMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Local => write!(f, "local"),
            Self::Global => write!(f, "global"),
            Self::Hybrid => write!(f, "hybrid"),
            Self::Naive => write!(f, "naive"),
            Self::Mix => write!(f, "mix"),
        }
    }
}

impl std::str::FromStr for ConversationMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "local" => Ok(Self::Local),
            "global" => Ok(Self::Global),
            "hybrid" => Ok(Self::Hybrid),
            "naive" => Ok(Self::Naive),
            "mix" => Ok(Self::Mix),
            _ => Err(format!("Unknown mode: {}", s)),
        }
    }
}

/// Message role in a conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    /// User message.
    User,
    /// Assistant response.
    Assistant,
    /// System message.
    System,
}

impl std::fmt::Display for MessageRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::User => write!(f, "user"),
            Self::Assistant => write!(f, "assistant"),
            Self::System => write!(f, "system"),
        }
    }
}

impl std::str::FromStr for MessageRole {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "user" => Ok(Self::User),
            "assistant" => Ok(Self::Assistant),
            "system" => Ok(Self::System),
            _ => Err(format!("Unknown role: {}", s)),
        }
    }
}

/// A conversation (chat session).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    /// Unique identifier.
    pub conversation_id: Uuid,
    /// Tenant this conversation belongs to.
    pub tenant_id: Uuid,
    /// Optional workspace scope.
    pub workspace_id: Option<Uuid>,
    /// User who owns this conversation.
    pub user_id: Uuid,
    /// Conversation title.
    pub title: String,
    /// Default query mode.
    pub mode: ConversationMode,
    /// Whether this conversation is pinned.
    pub is_pinned: bool,
    /// Whether this conversation is archived.
    pub is_archived: bool,
    /// Optional folder for organization.
    pub folder_id: Option<Uuid>,
    /// Share ID for public access (if shared).
    pub share_id: Option<String>,
    /// Additional metadata.
    pub meta: HashMap<String, serde_json::Value>,
    /// Creation timestamp.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last update timestamp.
    pub updated_at: chrono::DateTime<chrono::Utc>,

    // Computed fields (not stored in DB)
    /// Number of messages in the conversation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_count: Option<usize>,
    /// Preview of the last message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_message_preview: Option<String>,
}

impl Conversation {
    /// Create a new conversation.
    pub fn new(tenant_id: Uuid, user_id: Uuid) -> Self {
        let now = chrono::Utc::now();
        Self {
            conversation_id: Uuid::new_v4(),
            tenant_id,
            workspace_id: None,
            user_id,
            title: "New Conversation".to_string(),
            mode: ConversationMode::Hybrid,
            is_pinned: false,
            is_archived: false,
            folder_id: None,
            share_id: None,
            meta: HashMap::new(),
            created_at: now,
            updated_at: now,
            message_count: Some(0),
            last_message_preview: None,
        }
    }

    /// Set the conversation title.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Set the query mode.
    pub fn with_mode(mut self, mode: ConversationMode) -> Self {
        self.mode = mode;
        self
    }

    /// Set the workspace.
    pub fn with_workspace(mut self, workspace_id: Uuid) -> Self {
        self.workspace_id = Some(workspace_id);
        self
    }

    /// Set the folder.
    pub fn with_folder(mut self, folder_id: Uuid) -> Self {
        self.folder_id = Some(folder_id);
        self
    }
}

/// A message within a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Unique identifier.
    pub message_id: Uuid,
    /// Parent conversation.
    pub conversation_id: Uuid,
    /// Parent message (for threading).
    pub parent_id: Option<Uuid>,
    /// Message role.
    pub role: MessageRole,
    /// Message content.
    pub content: String,
    /// Query mode used for this message.
    pub mode: Option<ConversationMode>,
    /// Tokens used for this response.
    pub tokens_used: Option<i32>,
    /// Response generation time in milliseconds.
    pub duration_ms: Option<i32>,
    /// Thinking/reasoning time in milliseconds.
    pub thinking_time_ms: Option<i32>,
    /// Context attached to assistant responses.
    pub context: Option<MessageContext>,
    /// Whether this message represents an error.
    pub is_error: bool,
    /// Creation timestamp.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last update timestamp.
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Message {
    /// Create a new user message.
    pub fn user(conversation_id: Uuid, content: impl Into<String>) -> Self {
        let now = chrono::Utc::now();
        Self {
            message_id: Uuid::new_v4(),
            conversation_id,
            parent_id: None,
            role: MessageRole::User,
            content: content.into(),
            mode: None,
            tokens_used: None,
            duration_ms: None,
            thinking_time_ms: None,
            context: None,
            is_error: false,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a new assistant message.
    pub fn assistant(conversation_id: Uuid, content: impl Into<String>) -> Self {
        let now = chrono::Utc::now();
        Self {
            message_id: Uuid::new_v4(),
            conversation_id,
            parent_id: None,
            role: MessageRole::Assistant,
            content: content.into(),
            mode: None,
            tokens_used: None,
            duration_ms: None,
            thinking_time_ms: None,
            context: None,
            is_error: false,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a new system message.
    pub fn system(conversation_id: Uuid, content: impl Into<String>) -> Self {
        let now = chrono::Utc::now();
        Self {
            message_id: Uuid::new_v4(),
            conversation_id,
            parent_id: None,
            role: MessageRole::System,
            content: content.into(),
            mode: None,
            tokens_used: None,
            duration_ms: None,
            thinking_time_ms: None,
            context: None,
            is_error: false,
            created_at: now,
            updated_at: now,
        }
    }

    /// Set the parent message.
    pub fn with_parent(mut self, parent_id: Uuid) -> Self {
        self.parent_id = Some(parent_id);
        self
    }

    /// Set the mode.
    pub fn with_mode(mut self, mode: ConversationMode) -> Self {
        self.mode = Some(mode);
        self
    }

    /// Set the context.
    pub fn with_context(mut self, context: MessageContext) -> Self {
        self.context = Some(context);
        self
    }
}

/// Context attached to an assistant message (sources, entities).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MessageContext {
    /// Source references used to generate the response.
    #[serde(default)]
    pub sources: Vec<MessageSource>,
    /// Entities mentioned in the response with source tracking.
    #[serde(default)]
    pub entities: Vec<MessageContextEntity>,
    /// Relationships referenced in the response with source tracking.
    #[serde(default)]
    pub relationships: Vec<MessageContextRelationship>,
}

/// Entity in message context with source tracking for citations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageContextEntity {
    /// Entity name.
    pub name: String,
    /// Entity type.
    pub entity_type: String,
    /// Entity description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Relevance score.
    pub score: f32,
    /// Source document ID for citation link.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_document_id: Option<String>,
    /// Original file path for citation display.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_file_path: Option<String>,
    /// Source chunk IDs for provenance.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_chunk_ids: Vec<String>,
}

impl MessageContextEntity {
    /// Create from entity name with minimal fields.
    pub fn from_name(name: impl Into<String>, score: f32) -> Self {
        Self {
            name: name.into(),
            entity_type: "UNKNOWN".to_string(),
            description: None,
            score,
            source_document_id: None,
            source_file_path: None,
            source_chunk_ids: Vec::new(),
        }
    }
}

/// Relationship in message context with source tracking for citations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageContextRelationship {
    /// Source entity name.
    pub source: String,
    /// Target entity name.
    pub target: String,
    /// Relationship type.
    pub relation_type: String,
    /// Relationship description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Relevance score.
    pub score: f32,
    /// Source document ID for citation link.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_document_id: Option<String>,
    /// Original file path for citation display.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_file_path: Option<String>,
}

impl MessageContextRelationship {
    /// Create from source/target with minimal fields.
    pub fn new(
        source: impl Into<String>,
        target: impl Into<String>,
        relation_type: impl Into<String>,
        score: f32,
    ) -> Self {
        Self {
            source: source.into(),
            target: target.into(),
            relation_type: relation_type.into(),
            description: None,
            score,
            source_document_id: None,
            source_file_path: None,
        }
    }
}

impl MessageContext {
    /// Create a new empty context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a source.
    pub fn with_source(mut self, source: MessageSource) -> Self {
        self.sources.push(source);
        self
    }

    /// Add sources.
    pub fn with_sources(mut self, sources: Vec<MessageSource>) -> Self {
        self.sources = sources;
        self
    }
}

/// A source reference in message context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageSource {
    /// Source identifier.
    pub id: String,
    /// Source title.
    pub title: Option<String>,
    /// Content snippet.
    pub content: Option<String>,
    /// Relevance score.
    pub score: f32,
    /// Document ID this came from.
    pub document_id: Option<String>,
}

impl MessageSource {
    /// Create a new message source.
    pub fn new(id: impl Into<String>, score: f32) -> Self {
        Self {
            id: id.into(),
            title: None,
            content: None,
            score,
            document_id: None,
        }
    }
}

/// Folder for organizing conversations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Folder {
    /// Unique identifier.
    pub folder_id: Uuid,
    /// Tenant this folder belongs to.
    pub tenant_id: Uuid,
    /// Optional workspace scope.
    pub workspace_id: Option<Uuid>,
    /// User who owns this folder.
    pub user_id: Uuid,
    /// Folder name.
    pub name: String,
    /// Parent folder (for hierarchy).
    pub parent_id: Option<Uuid>,
    /// Display position.
    pub position: i32,
    /// Creation timestamp.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last update timestamp.
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Folder {
    /// Create a new folder.
    pub fn new(tenant_id: Uuid, user_id: Uuid, name: impl Into<String>) -> Self {
        let now = chrono::Utc::now();
        Self {
            folder_id: Uuid::new_v4(),
            tenant_id,
            workspace_id: None,
            user_id,
            name: name.into(),
            parent_id: None,
            position: 0,
            created_at: now,
            updated_at: now,
        }
    }

    /// Set the parent folder.
    pub fn with_parent(mut self, parent_id: Uuid) -> Self {
        self.parent_id = Some(parent_id);
        self
    }

    /// Set the workspace.
    pub fn with_workspace(mut self, workspace_id: Uuid) -> Self {
        self.workspace_id = Some(workspace_id);
        self
    }

    /// Set the position.
    pub fn with_position(mut self, position: i32) -> Self {
        self.position = position;
        self
    }
}

// ============================================================================
// Request/Response Types
// ============================================================================

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
    pub folder_id: Option<Uuid>,
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
    fn test_conversation_mode_display() {
        assert_eq!(ConversationMode::Local.to_string(), "local");
        assert_eq!(ConversationMode::Global.to_string(), "global");
        assert_eq!(ConversationMode::Hybrid.to_string(), "hybrid");
        assert_eq!(ConversationMode::Naive.to_string(), "naive");
        assert_eq!(ConversationMode::Mix.to_string(), "mix");
    }

    #[test]
    fn test_conversation_mode_parse() {
        assert_eq!(
            "local".parse::<ConversationMode>().unwrap(),
            ConversationMode::Local
        );
        assert_eq!(
            "HYBRID".parse::<ConversationMode>().unwrap(),
            ConversationMode::Hybrid
        );
        assert!("invalid".parse::<ConversationMode>().is_err());
    }

    #[test]
    fn test_message_role_display() {
        assert_eq!(MessageRole::User.to_string(), "user");
        assert_eq!(MessageRole::Assistant.to_string(), "assistant");
        assert_eq!(MessageRole::System.to_string(), "system");
    }

    #[test]
    fn test_conversation_builder() {
        let tenant_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let workspace_id = Uuid::new_v4();

        let conv = Conversation::new(tenant_id, user_id)
            .with_title("Test Chat")
            .with_mode(ConversationMode::Local)
            .with_workspace(workspace_id);

        assert_eq!(conv.title, "Test Chat");
        assert_eq!(conv.mode, ConversationMode::Local);
        assert_eq!(conv.workspace_id, Some(workspace_id));
        assert_eq!(conv.tenant_id, tenant_id);
        assert_eq!(conv.user_id, user_id);
    }

    #[test]
    fn test_message_builder() {
        let conversation_id = Uuid::new_v4();
        let parent_id = Uuid::new_v4();

        let msg = Message::user(conversation_id, "Hello, world!")
            .with_parent(parent_id)
            .with_mode(ConversationMode::Hybrid);

        assert_eq!(msg.content, "Hello, world!");
        assert_eq!(msg.role, MessageRole::User);
        assert_eq!(msg.parent_id, Some(parent_id));
        assert_eq!(msg.mode, Some(ConversationMode::Hybrid));
    }

    #[test]
    fn test_folder_builder() {
        let tenant_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let parent_id = Uuid::new_v4();

        let folder = Folder::new(tenant_id, user_id, "My Folder")
            .with_parent(parent_id)
            .with_position(5);

        assert_eq!(folder.name, "My Folder");
        assert_eq!(folder.parent_id, Some(parent_id));
        assert_eq!(folder.position, 5);
    }
}
