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
//!
//! # Architecture
//!
//! - [`ConversationService`]: Trait defining all conversation operations
//! - `in_memory`: In-memory implementation for testing

mod in_memory;

pub use in_memory::InMemoryConversationService;

use async_trait::async_trait;
use uuid::Uuid;

use crate::error::Result;
use crate::types::{
    Conversation, ConversationFilter, ConversationSortField, CreateConversationRequest,
    CreateMessageRequest, Folder, ImportResult, Message, PaginatedConversations, PaginatedMessages,
    UpdateConversationRequest, UpdateMessageRequest,
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
        tenant_id: Uuid,
        user_id: Uuid,
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
        tenant_id: Uuid,
        user_id: Uuid,
        folder_id: Uuid,
        name: Option<String>,
        parent_id: Option<Uuid>,
        position: Option<i32>,
    ) -> Result<Folder>;

    /// Delete a folder.
    async fn delete_folder(&self, tenant_id: Uuid, user_id: Uuid, folder_id: Uuid) -> Result<()>;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ConversationMode;

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
        use crate::types::MessageRole;

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
