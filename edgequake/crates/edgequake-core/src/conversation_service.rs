//! Conversation service for managing chat sessions.
//!
//! This module defines the service trait for conversation management and
//! provides an in-memory implementation for testing.
//!
//! ## Implements
//!
//! - **FEAT0810**: Conversation CRUD operations
//! - **FEAT0811**: Message management within conversations
//! - **FEAT0812**: Folder organization for conversations
//! - **FEAT0813**: Conversation import/export
//!
//! ## Use Cases
//!
//! - **UC2401**: User creates new conversation with mode
//! - **UC2402**: User adds message to conversation
//! - **UC2403**: User organizes conversations into folders
//! - **UC2404**: User imports conversations from JSON
//!
//! ## Enforces
//!
//! - **BR0810**: Conversations scoped to user and workspace
//! - **BR0811**: Messages must have valid role (user/assistant/system)

use async_trait::async_trait;
use uuid::Uuid;

use crate::error::Result;
use crate::types::{
    Conversation, ConversationFilter, ConversationSortField, CreateConversationRequest,
    CreateMessageRequest, Folder, ImportError, ImportResult, Message, PaginatedConversations,
    PaginatedMessages, UpdateConversationRequest, UpdateMessageRequest,
};

/// Service trait for conversation management.
///
/// WHY: This trait has methods with many parameters because conversation operations
/// require tenant_id, user_id, workspace_id, and request objects - these are semantically
/// distinct and cannot be reasonably grouped further without losing API clarity.
#[async_trait]
#[allow(clippy::too_many_arguments)]
pub trait ConversationService: Send + Sync {
    // ============ Conversation Operations ============

    /// Create a new conversation.
    async fn create_conversation(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        workspace_id: Option<Uuid>,
        request: CreateConversationRequest,
    ) -> Result<Conversation>;

    /// Get a conversation by ID.
    async fn get_conversation(&self, conversation_id: Uuid) -> Result<Option<Conversation>>;

    /// Update a conversation.
    async fn update_conversation(
        &self,
        conversation_id: Uuid,
        request: UpdateConversationRequest,
    ) -> Result<Conversation>;

    /// Delete a conversation.
    async fn delete_conversation(&self, conversation_id: Uuid) -> Result<()>;

    /// List conversations with pagination and filtering.
    async fn list_conversations(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        filter: ConversationFilter,
        sort: ConversationSortField,
        sort_desc: bool,
        cursor: Option<String>,
        limit: usize,
    ) -> Result<PaginatedConversations>;

    /// Generate a share link for a conversation.
    async fn share_conversation(&self, conversation_id: Uuid) -> Result<String>;

    /// Remove share link from a conversation.
    async fn unshare_conversation(&self, conversation_id: Uuid) -> Result<()>;

    /// Get a shared conversation by share_id (public access).
    async fn get_shared_conversation(&self, share_id: &str) -> Result<Option<Conversation>>;

    // ============ Message Operations ============

    /// Add a message to a conversation.
    async fn create_message(
        &self,
        conversation_id: Uuid,
        request: CreateMessageRequest,
    ) -> Result<Message>;

    /// Update a message.
    async fn update_message(
        &self,
        message_id: Uuid,
        request: UpdateMessageRequest,
    ) -> Result<Message>;

    /// Delete a message.
    async fn delete_message(&self, message_id: Uuid) -> Result<()>;

    /// List messages in a conversation.
    async fn list_messages(
        &self,
        conversation_id: Uuid,
        cursor: Option<String>,
        limit: usize,
    ) -> Result<PaginatedMessages>;

    // ============ Folder Operations ============

    /// Create a folder.
    async fn create_folder(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        name: String,
        parent_id: Option<Uuid>,
    ) -> Result<Folder>;

    /// List folders for a user.
    async fn list_folders(&self, tenant_id: Uuid, user_id: Uuid) -> Result<Vec<Folder>>;

    /// Update a folder.
    async fn update_folder(
        &self,
        folder_id: Uuid,
        name: Option<String>,
        parent_id: Option<Uuid>,
        position: Option<i32>,
    ) -> Result<Folder>;

    /// Delete a folder.
    async fn delete_folder(&self, folder_id: Uuid) -> Result<()>;

    // ============ Bulk Operations ============

    /// Import conversations from client (localStorage migration).
    async fn import_conversations(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        conversations: Vec<serde_json::Value>,
    ) -> Result<ImportResult>;

    /// Bulk delete conversations.
    async fn bulk_delete(&self, conversation_ids: Vec<Uuid>) -> Result<usize>;

    /// Bulk archive conversations.
    async fn bulk_archive(&self, conversation_ids: Vec<Uuid>, archive: bool) -> Result<usize>;

    /// Bulk move to folder.
    async fn bulk_move_to_folder(
        &self,
        conversation_ids: Vec<Uuid>,
        folder_id: Option<Uuid>,
    ) -> Result<usize>;
}

// ============================================================================
// In-Memory Implementation (for testing)
// ============================================================================

use std::collections::HashMap;
use std::sync::RwLock;

use crate::types::{ConversationMode, MessageRole, PaginationMeta};

/// In-memory implementation of ConversationService for testing.
pub struct InMemoryConversationService {
    conversations: RwLock<HashMap<Uuid, Conversation>>,
    messages: RwLock<HashMap<Uuid, Message>>,
    folders: RwLock<HashMap<Uuid, Folder>>,
}

impl InMemoryConversationService {
    /// Create a new in-memory conversation service.
    pub fn new() -> Self {
        Self {
            conversations: RwLock::new(HashMap::new()),
            messages: RwLock::new(HashMap::new()),
            folders: RwLock::new(HashMap::new()),
        }
    }

    /// Generate a share ID.
    fn generate_share_id() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        format!("share_{}", ts)
    }
}

impl Default for InMemoryConversationService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ConversationService for InMemoryConversationService {
    async fn create_conversation(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        workspace_id: Option<Uuid>,
        request: CreateConversationRequest,
    ) -> Result<Conversation> {
        let mut conv = Conversation::new(tenant_id, user_id);
        if let Some(title) = request.title {
            conv.title = title;
        }
        if let Some(mode) = request.mode {
            conv.mode = mode;
        }
        if let Some(folder_id) = request.folder_id {
            conv.folder_id = Some(folder_id);
        }
        if let Some(ws_id) = workspace_id {
            conv.workspace_id = Some(ws_id);
        }

        let id = conv.conversation_id;
        self.conversations.write().unwrap().insert(id, conv.clone());
        Ok(conv)
    }

    async fn get_conversation(&self, conversation_id: Uuid) -> Result<Option<Conversation>> {
        Ok(self
            .conversations
            .read()
            .unwrap()
            .get(&conversation_id)
            .cloned())
    }

    async fn update_conversation(
        &self,
        conversation_id: Uuid,
        request: UpdateConversationRequest,
    ) -> Result<Conversation> {
        let mut convs = self.conversations.write().unwrap();
        let conv = convs
            .get_mut(&conversation_id)
            .ok_or_else(|| crate::error::Error::not_found("Conversation not found"))?;

        if let Some(title) = request.title {
            conv.title = title;
        }
        if let Some(mode) = request.mode {
            conv.mode = mode;
        }
        if let Some(is_pinned) = request.is_pinned {
            conv.is_pinned = is_pinned;
        }
        if let Some(is_archived) = request.is_archived {
            conv.is_archived = is_archived;
        }
        if let Some(folder_id) = request.folder_id {
            conv.folder_id = Some(folder_id);
        }
        conv.updated_at = chrono::Utc::now();

        Ok(conv.clone())
    }

    async fn delete_conversation(&self, conversation_id: Uuid) -> Result<()> {
        self.conversations.write().unwrap().remove(&conversation_id);
        // Also remove associated messages
        let mut msgs = self.messages.write().unwrap();
        msgs.retain(|_, m| m.conversation_id != conversation_id);
        Ok(())
    }

    async fn list_conversations(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        filter: ConversationFilter,
        sort: ConversationSortField,
        sort_desc: bool,
        _cursor: Option<String>,
        limit: usize,
    ) -> Result<PaginatedConversations> {
        let convs = self.conversations.read().unwrap();
        let mut items: Vec<_> = convs
            .values()
            .filter(|c| c.tenant_id == tenant_id && c.user_id == user_id)
            .filter(|c| {
                // Apply filters
                if let Some(archived) = filter.archived {
                    if c.is_archived != archived {
                        return false;
                    }
                }
                if let Some(pinned) = filter.pinned {
                    if c.is_pinned != pinned {
                        return false;
                    }
                }
                if let Some(ref modes) = filter.mode {
                    if !modes.contains(&c.mode) {
                        return false;
                    }
                }
                if let Some(folder_id) = filter.folder_id {
                    if c.folder_id != Some(folder_id) {
                        return false;
                    }
                }
                if let Some(ref search) = filter.search {
                    if !c.title.to_lowercase().contains(&search.to_lowercase()) {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect();

        // Sort
        match sort {
            ConversationSortField::UpdatedAt => {
                items.sort_by(|a, b| a.updated_at.cmp(&b.updated_at));
            }
            ConversationSortField::CreatedAt => {
                items.sort_by(|a, b| a.created_at.cmp(&b.created_at));
            }
            ConversationSortField::Title => {
                items.sort_by(|a, b| a.title.cmp(&b.title));
            }
        }
        if sort_desc {
            items.reverse();
        }

        let has_more = items.len() > limit;
        items.truncate(limit);

        Ok(PaginatedConversations {
            items,
            pagination: PaginationMeta {
                next_cursor: None,
                prev_cursor: None,
                total: None,
                has_more,
            },
        })
    }

    async fn share_conversation(&self, conversation_id: Uuid) -> Result<String> {
        let mut convs = self.conversations.write().unwrap();
        let conv = convs
            .get_mut(&conversation_id)
            .ok_or_else(|| crate::error::Error::not_found("Conversation not found"))?;

        if conv.share_id.is_none() {
            conv.share_id = Some(Self::generate_share_id());
        }
        Ok(conv.share_id.clone().unwrap())
    }

    async fn unshare_conversation(&self, conversation_id: Uuid) -> Result<()> {
        let mut convs = self.conversations.write().unwrap();
        let conv = convs
            .get_mut(&conversation_id)
            .ok_or_else(|| crate::error::Error::not_found("Conversation not found"))?;
        conv.share_id = None;
        Ok(())
    }

    async fn get_shared_conversation(&self, share_id: &str) -> Result<Option<Conversation>> {
        let convs = self.conversations.read().unwrap();
        Ok(convs
            .values()
            .find(|c| c.share_id.as_deref() == Some(share_id))
            .cloned())
    }

    async fn create_message(
        &self,
        conversation_id: Uuid,
        request: CreateMessageRequest,
    ) -> Result<Message> {
        let now = chrono::Utc::now();
        let msg = Message {
            message_id: Uuid::new_v4(),
            conversation_id,
            parent_id: request.parent_id,
            role: request.role,
            content: request.content,
            mode: None,
            tokens_used: None,
            duration_ms: None,
            thinking_time_ms: None,
            context: None,
            is_error: false,
            created_at: now,
            updated_at: now,
        };

        let id = msg.message_id;
        self.messages.write().unwrap().insert(id, msg.clone());

        // Update conversation's updated_at
        if let Some(conv) = self
            .conversations
            .write()
            .unwrap()
            .get_mut(&conversation_id)
        {
            conv.updated_at = now;
        }

        Ok(msg)
    }

    async fn update_message(
        &self,
        message_id: Uuid,
        request: UpdateMessageRequest,
    ) -> Result<Message> {
        let mut msgs = self.messages.write().unwrap();
        let msg = msgs
            .get_mut(&message_id)
            .ok_or_else(|| crate::error::Error::not_found("Message not found"))?;

        if let Some(content) = request.content {
            msg.content = content;
        }
        if let Some(tokens) = request.tokens_used {
            msg.tokens_used = Some(tokens);
        }
        if let Some(duration) = request.duration_ms {
            msg.duration_ms = Some(duration);
        }
        if let Some(thinking_time) = request.thinking_time_ms {
            msg.thinking_time_ms = Some(thinking_time);
        }
        if let Some(context) = request.context {
            msg.context = Some(context);
        }
        if let Some(is_error) = request.is_error {
            msg.is_error = is_error;
        }
        msg.updated_at = chrono::Utc::now();

        Ok(msg.clone())
    }

    async fn delete_message(&self, message_id: Uuid) -> Result<()> {
        self.messages.write().unwrap().remove(&message_id);
        Ok(())
    }

    async fn list_messages(
        &self,
        conversation_id: Uuid,
        _cursor: Option<String>,
        limit: usize,
    ) -> Result<PaginatedMessages> {
        let msgs = self.messages.read().unwrap();
        let mut items: Vec<_> = msgs
            .values()
            .filter(|m| m.conversation_id == conversation_id)
            .cloned()
            .collect();

        items.sort_by(|a, b| a.created_at.cmp(&b.created_at));

        let has_more = items.len() > limit;
        items.truncate(limit);

        Ok(PaginatedMessages {
            items,
            pagination: PaginationMeta {
                next_cursor: None,
                prev_cursor: None,
                total: None,
                has_more,
            },
        })
    }

    async fn create_folder(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        name: String,
        parent_id: Option<Uuid>,
    ) -> Result<Folder> {
        let mut folder = Folder::new(tenant_id, user_id, name);
        if let Some(pid) = parent_id {
            folder = folder.with_parent(pid);
        }

        let id = folder.folder_id;
        self.folders.write().unwrap().insert(id, folder.clone());
        Ok(folder)
    }

    async fn list_folders(&self, tenant_id: Uuid, user_id: Uuid) -> Result<Vec<Folder>> {
        let folders = self.folders.read().unwrap();
        let mut items: Vec<_> = folders
            .values()
            .filter(|f| f.tenant_id == tenant_id && f.user_id == user_id)
            .cloned()
            .collect();

        items.sort_by(|a, b| a.position.cmp(&b.position));
        Ok(items)
    }

    async fn update_folder(
        &self,
        folder_id: Uuid,
        name: Option<String>,
        parent_id: Option<Uuid>,
        position: Option<i32>,
    ) -> Result<Folder> {
        let mut folders = self.folders.write().unwrap();
        let folder = folders
            .get_mut(&folder_id)
            .ok_or_else(|| crate::error::Error::not_found("Folder not found"))?;

        if let Some(n) = name {
            folder.name = n;
        }
        if let Some(pid) = parent_id {
            folder.parent_id = Some(pid);
        }
        if let Some(pos) = position {
            folder.position = pos;
        }
        folder.updated_at = chrono::Utc::now();

        Ok(folder.clone())
    }

    async fn delete_folder(&self, folder_id: Uuid) -> Result<()> {
        self.folders.write().unwrap().remove(&folder_id);
        // Move conversations out of folder
        let mut convs = self.conversations.write().unwrap();
        for conv in convs.values_mut() {
            if conv.folder_id == Some(folder_id) {
                conv.folder_id = None;
            }
        }
        Ok(())
    }

    async fn import_conversations(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        conversations: Vec<serde_json::Value>,
    ) -> Result<ImportResult> {
        let mut imported = 0;
        let mut failed = 0;
        let mut errors = Vec::new();

        for conv_json in conversations {
            let id = conv_json
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();

            // Try to parse and import
            match self
                .import_single_conversation(tenant_id, user_id, &conv_json)
                .await
            {
                Ok(_) => imported += 1,
                Err(e) => {
                    failed += 1;
                    errors.push(ImportError {
                        id,
                        error: e.to_string(),
                    });
                }
            }
        }

        Ok(ImportResult {
            imported,
            failed,
            errors,
        })
    }

    async fn bulk_delete(&self, conversation_ids: Vec<Uuid>) -> Result<usize> {
        let mut convs = self.conversations.write().unwrap();
        let mut msgs = self.messages.write().unwrap();
        let mut count = 0;

        for id in conversation_ids {
            if convs.remove(&id).is_some() {
                count += 1;
                msgs.retain(|_, m| m.conversation_id != id);
            }
        }

        Ok(count)
    }

    async fn bulk_archive(&self, conversation_ids: Vec<Uuid>, archive: bool) -> Result<usize> {
        let mut convs = self.conversations.write().unwrap();
        let mut count = 0;

        for id in conversation_ids {
            if let Some(conv) = convs.get_mut(&id) {
                conv.is_archived = archive;
                conv.updated_at = chrono::Utc::now();
                count += 1;
            }
        }

        Ok(count)
    }

    async fn bulk_move_to_folder(
        &self,
        conversation_ids: Vec<Uuid>,
        folder_id: Option<Uuid>,
    ) -> Result<usize> {
        let mut convs = self.conversations.write().unwrap();
        let mut count = 0;

        for id in conversation_ids {
            if let Some(conv) = convs.get_mut(&id) {
                conv.folder_id = folder_id;
                conv.updated_at = chrono::Utc::now();
                count += 1;
            }
        }

        Ok(count)
    }
}

impl InMemoryConversationService {
    async fn import_single_conversation(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        conv_json: &serde_json::Value,
    ) -> Result<Uuid> {
        // Parse conversation
        let title = conv_json
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Imported Conversation")
            .to_string();

        let mode_str = conv_json
            .get("mode")
            .and_then(|v| v.as_str())
            .unwrap_or("hybrid");
        let mode = mode_str.parse().unwrap_or(ConversationMode::Hybrid);

        let conv = self
            .create_conversation(
                tenant_id,
                user_id,
                None,
                CreateConversationRequest {
                    title: Some(title),
                    mode: Some(mode),
                    folder_id: None,
                },
            )
            .await?;

        // Import messages
        if let Some(messages) = conv_json.get("messages").and_then(|v| v.as_array()) {
            for msg_json in messages {
                let role_str = msg_json
                    .get("role")
                    .and_then(|v| v.as_str())
                    .unwrap_or("user");
                let role = match role_str {
                    "assistant" => MessageRole::Assistant,
                    "system" => MessageRole::System,
                    _ => MessageRole::User,
                };

                let content = msg_json
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                self.create_message(
                    conv.conversation_id,
                    CreateMessageRequest {
                        content,
                        role,
                        parent_id: None,
                        stream: false,
                    },
                )
                .await?;
            }
        }

        Ok(conv.conversation_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_conversation() {
        let service = InMemoryConversationService::new();
        let tenant_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let conv = service
            .create_conversation(
                tenant_id,
                user_id,
                None,
                CreateConversationRequest {
                    title: Some("Test Chat".into()),
                    mode: Some(ConversationMode::Local),
                    folder_id: None,
                },
            )
            .await
            .unwrap();

        assert_eq!(conv.title, "Test Chat");
        assert_eq!(conv.mode, ConversationMode::Local);
        assert_eq!(conv.tenant_id, tenant_id);
        assert_eq!(conv.user_id, user_id);
    }

    #[tokio::test]
    async fn test_list_conversations() {
        let service = InMemoryConversationService::new();
        let tenant_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        // Create 5 conversations
        for i in 0..5 {
            service
                .create_conversation(
                    tenant_id,
                    user_id,
                    None,
                    CreateConversationRequest {
                        title: Some(format!("Chat {}", i)),
                        mode: None,
                        folder_id: None,
                    },
                )
                .await
                .unwrap();
        }

        let result = service
            .list_conversations(
                tenant_id,
                user_id,
                ConversationFilter::default(),
                ConversationSortField::UpdatedAt,
                true,
                None,
                10,
            )
            .await
            .unwrap();

        assert_eq!(result.items.len(), 5);
    }

    #[tokio::test]
    async fn test_create_and_list_messages() {
        let service = InMemoryConversationService::new();
        let tenant_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let conv = service
            .create_conversation(
                tenant_id,
                user_id,
                None,
                CreateConversationRequest::default(),
            )
            .await
            .unwrap();

        // Add messages
        service
            .create_message(
                conv.conversation_id,
                CreateMessageRequest {
                    content: "Hello".into(),
                    role: MessageRole::User,
                    parent_id: None,
                    stream: false,
                },
            )
            .await
            .unwrap();

        service
            .create_message(
                conv.conversation_id,
                CreateMessageRequest {
                    content: "Hi there!".into(),
                    role: MessageRole::Assistant,
                    parent_id: None,
                    stream: false,
                },
            )
            .await
            .unwrap();

        let msgs = service
            .list_messages(conv.conversation_id, None, 100)
            .await
            .unwrap();

        assert_eq!(msgs.items.len(), 2);
        assert_eq!(msgs.items[0].content, "Hello");
        assert_eq!(msgs.items[1].content, "Hi there!");
    }

    #[tokio::test]
    async fn test_share_conversation() {
        let service = InMemoryConversationService::new();
        let tenant_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let conv = service
            .create_conversation(
                tenant_id,
                user_id,
                None,
                CreateConversationRequest::default(),
            )
            .await
            .unwrap();

        let share_id = service
            .share_conversation(conv.conversation_id)
            .await
            .unwrap();

        let shared = service.get_shared_conversation(&share_id).await.unwrap();
        assert!(shared.is_some());
        assert_eq!(shared.unwrap().conversation_id, conv.conversation_id);

        // Unshare
        service
            .unshare_conversation(conv.conversation_id)
            .await
            .unwrap();
        let shared = service.get_shared_conversation(&share_id).await.unwrap();
        assert!(shared.is_none());
    }

    #[tokio::test]
    async fn test_bulk_operations() {
        let service = InMemoryConversationService::new();
        let tenant_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let mut ids = Vec::new();
        for i in 0..3 {
            let conv = service
                .create_conversation(
                    tenant_id,
                    user_id,
                    None,
                    CreateConversationRequest {
                        title: Some(format!("Chat {}", i)),
                        mode: None,
                        folder_id: None,
                    },
                )
                .await
                .unwrap();
            ids.push(conv.conversation_id);
        }

        // Bulk archive
        let archived = service.bulk_archive(ids.clone(), true).await.unwrap();
        assert_eq!(archived, 3);

        // Verify archived
        let conv = service.get_conversation(ids[0]).await.unwrap().unwrap();
        assert!(conv.is_archived);

        // Bulk delete
        let deleted = service.bulk_delete(ids).await.unwrap();
        assert_eq!(deleted, 3);
    }
}
