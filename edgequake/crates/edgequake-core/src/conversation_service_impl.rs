//! Production implementation of ConversationService.
//!
//! This module provides the production-ready implementation of the ConversationService
//! trait, backed by PostgreSQL (the system of record).
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                        edgequake-core                           │
//! │  ┌────────────────────┐    ┌─────────────────────────────────┐ │
//! │  │ ConversationService│◄───│ ConversationServiceImpl         │ │
//! │  │      (trait)       │    │ (production implementation)     │ │
//! │  └────────────────────┘    └──────────────┬──────────────────┘ │
//! └───────────────────────────────────────────┼─────────────────────┘
//!                                             │
//! ┌───────────────────────────────────────────┼─────────────────────┐
//! │                     edgequake-storage     ▼                     │
//! │  ┌─────────────────────────────────────────────────────────────┐│
//! │  │ PostgresConversationStorage (raw DB operations)             ││
//! │  │ Returns: ConversationRow, MessageRow (DB types)             ││
//! │  └─────────────────────────────────────────────────────────────┘│
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # WHY: Service Layer in Core (not Storage)
//!
//! This service MUST live in `edgequake-core` because:
//! 1. It implements the `ConversationService` trait defined in this crate
//! 2. Moving to `edgequake-storage` would create a circular dependency
//! 3. Follows Hexagonal Architecture: adapters live with ports

#[cfg(feature = "postgres")]
use async_trait::async_trait;
#[cfg(feature = "postgres")]
use sqlx::PgPool;
#[cfg(feature = "postgres")]
use std::sync::Arc;
#[cfg(feature = "postgres")]
use uuid::Uuid;

#[cfg(feature = "postgres")]
use crate::{
    conversation_service::ConversationService,
    error::{Error, Result},
    types::{
        Conversation, ConversationFilter, ConversationSortField, CreateConversationRequest,
        CreateMessageRequest, Folder, ImportError, ImportResult, Message, MessageRole,
        PaginatedConversations, PaginatedMessages, PaginationMeta, UpdateConversationRequest,
        UpdateMessageRequest,
    },
};
#[cfg(feature = "postgres")]
use edgequake_storage::PostgresConversationStorage;

/// PostgreSQL implementation of ConversationService.
#[cfg(feature = "postgres")]
#[derive(Clone)]
pub struct ConversationServiceImpl {
    storage: Arc<PostgresConversationStorage>,
}

#[cfg(feature = "postgres")]
impl ConversationServiceImpl {
    /// Create a new PostgreSQL conversation service.
    pub fn new(pool: PgPool) -> Self {
        Self {
            storage: Arc::new(PostgresConversationStorage::new(pool)),
        }
    }

    /// Create from an existing storage instance.
    pub fn from_storage(storage: PostgresConversationStorage) -> Self {
        Self {
            storage: Arc::new(storage),
        }
    }

    /// Convert a storage row to a domain Conversation.
    fn row_to_conversation(row: edgequake_storage::ConversationRow) -> Conversation {
        Conversation {
            conversation_id: row.conversation_id,
            tenant_id: row.tenant_id,
            workspace_id: row.workspace_id,
            user_id: row.user_id,
            title: row.title,
            mode: row.mode.parse().unwrap_or_default(),
            is_pinned: row.is_pinned,
            is_archived: row.is_archived,
            folder_id: row.folder_id,
            share_id: row.share_id,
            meta: serde_json::from_value(row.meta).unwrap_or_default(),
            created_at: row.created_at,
            updated_at: row.updated_at,
            message_count: None,
            last_message_preview: None,
        }
    }

    /// Convert a storage row to a domain Message.
    fn row_to_message(row: edgequake_storage::MessageRow) -> Message {
        Message {
            message_id: row.message_id,
            conversation_id: row.conversation_id,
            parent_id: row.parent_id,
            role: row.role.parse().unwrap_or(MessageRole::User),
            content: row.content,
            mode: row.mode.and_then(|m| m.parse().ok()),
            tokens_used: row.tokens_used,
            duration_ms: row.duration_ms,
            thinking_time_ms: row.thinking_time_ms,
            context: row.context.and_then(|c| serde_json::from_value(c).ok()),
            is_error: row.is_error,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }

    /// Convert a storage row to a domain Folder.
    fn row_to_folder(row: edgequake_storage::FolderRow) -> Folder {
        Folder {
            folder_id: row.folder_id,
            tenant_id: row.tenant_id,
            workspace_id: row.workspace_id,
            user_id: row.user_id,
            name: row.name,
            parent_id: row.parent_id,
            position: row.position,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }

    /// Convert storage error to domain error.
    fn map_error(e: edgequake_storage::StorageError) -> Error {
        match e {
            edgequake_storage::StorageError::NotFound(msg) => Error::not_found(&msg),
            _ => Error::internal(e.to_string()),
        }
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl ConversationService for ConversationServiceImpl {
    async fn create_conversation(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        workspace_id: Option<Uuid>,
        request: CreateConversationRequest,
    ) -> Result<Conversation> {
        let row = self
            .storage
            .create_conversation(
                tenant_id,
                user_id,
                workspace_id,
                request
                    .title
                    .unwrap_or_else(|| "New Conversation".to_string()),
                request.mode.unwrap_or_default().to_string(),
                request.folder_id,
            )
            .await
            .map_err(Self::map_error)?;

        Ok(Self::row_to_conversation(row))
    }

    async fn get_conversation(&self, conversation_id: Uuid) -> Result<Option<Conversation>> {
        let row = self
            .storage
            .get_conversation(conversation_id)
            .await
            .map_err(Self::map_error)?;

        Ok(row.map(Self::row_to_conversation))
    }

    async fn update_conversation(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        conversation_id: Uuid,
        request: UpdateConversationRequest,
    ) -> Result<Conversation> {
        let row = self
            .storage
            .update_conversation(
                tenant_id,
                user_id,
                conversation_id,
                request.title,
                request.mode.map(|m| m.to_string()),
                request.is_pinned,
                request.is_archived,
                request.folder_id,
            )
            .await
            .map_err(Self::map_error)?;

        Ok(Self::row_to_conversation(row))
    }

    async fn delete_conversation(&self, conversation_id: Uuid) -> Result<()> {
        self.storage
            .delete_conversation(conversation_id)
            .await
            .map_err(Self::map_error)
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
        let sort_field = match sort {
            ConversationSortField::UpdatedAt => "updated_at",
            ConversationSortField::CreatedAt => "created_at",
            ConversationSortField::Title => "title",
        };

        let (rows, total) = self
            .storage
            .list_conversations(
                tenant_id,
                user_id,
                filter.archived,
                filter.pinned,
                filter.folder_id,
                filter.unfiled,
                filter.search.as_deref(),
                sort_field,
                sort_desc,
                limit as i64,
                0, // TODO: implement cursor-based pagination
            )
            .await
            .map_err(Self::map_error)?;

        let items: Vec<Conversation> = rows.into_iter().map(Self::row_to_conversation).collect();
        let has_more = (total as usize) > items.len();

        Ok(PaginatedConversations {
            items,
            pagination: PaginationMeta {
                next_cursor: None,
                prev_cursor: None,
                total: Some(total as usize),
                has_more,
            },
        })
    }

    async fn share_conversation(&self, conversation_id: Uuid) -> Result<String> {
        self.storage
            .share_conversation(conversation_id)
            .await
            .map_err(Self::map_error)
    }

    async fn unshare_conversation(&self, conversation_id: Uuid) -> Result<()> {
        self.storage
            .unshare_conversation(conversation_id)
            .await
            .map_err(Self::map_error)
    }

    async fn get_shared_conversation(&self, share_id: &str) -> Result<Option<Conversation>> {
        let row = self
            .storage
            .get_shared_conversation(share_id)
            .await
            .map_err(Self::map_error)?;

        Ok(row.map(Self::row_to_conversation))
    }

    async fn create_message(
        &self,
        conversation_id: Uuid,
        request: CreateMessageRequest,
    ) -> Result<Message> {
        let row = self
            .storage
            .create_message(
                conversation_id,
                request.parent_id,
                &request.role.to_string(),
                &request.content,
                None,  // mode is not in CreateMessageRequest
                None,  // tokens_used
                None,  // duration_ms
                None,  // thinking_time_ms
                None,  // context
                false, // is_error
            )
            .await
            .map_err(Self::map_error)?;

        Ok(Self::row_to_message(row))
    }

    async fn update_message(
        &self,
        message_id: Uuid,
        request: UpdateMessageRequest,
    ) -> Result<Message> {
        let row = self
            .storage
            .update_message(
                message_id,
                request.content.as_deref(),
                request.tokens_used,
                request.duration_ms,
                request.thinking_time_ms,
                request
                    .context
                    .map(|c| serde_json::to_value(c).unwrap_or_default()),
                request.is_error,
            )
            .await
            .map_err(Self::map_error)?;

        Ok(Self::row_to_message(row))
    }

    async fn delete_message(&self, message_id: Uuid) -> Result<()> {
        self.storage
            .delete_message(message_id)
            .await
            .map_err(Self::map_error)
    }

    async fn list_messages(
        &self,
        conversation_id: Uuid,
        _cursor: Option<String>,
        limit: usize,
    ) -> Result<PaginatedMessages> {
        let (rows, total) = self
            .storage
            .list_messages(conversation_id, limit as i64, 0)
            .await
            .map_err(Self::map_error)?;

        let items: Vec<Message> = rows.into_iter().map(Self::row_to_message).collect();
        let has_more = (total as usize) > items.len();

        Ok(PaginatedMessages {
            items,
            pagination: PaginationMeta {
                next_cursor: None,
                prev_cursor: None,
                total: Some(total as usize),
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
        let row = self
            .storage
            .create_folder(tenant_id, user_id, None, &name, parent_id)
            .await
            .map_err(Self::map_error)?;

        Ok(Self::row_to_folder(row))
    }

    async fn list_folders(&self, tenant_id: Uuid, user_id: Uuid) -> Result<Vec<Folder>> {
        let rows = self
            .storage
            .list_folders(tenant_id, user_id)
            .await
            .map_err(Self::map_error)?;

        Ok(rows.into_iter().map(Self::row_to_folder).collect())
    }

    async fn update_folder(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        folder_id: Uuid,
        name: Option<String>,
        parent_id: Option<Uuid>,
        position: Option<i32>,
    ) -> Result<Folder> {
        let row = self
            .storage
            .update_folder(
                tenant_id,
                user_id,
                folder_id,
                name.as_deref(),
                parent_id,
                position,
            )
            .await
            .map_err(Self::map_error)?;

        Ok(Self::row_to_folder(row))
    }

    async fn delete_folder(&self, tenant_id: Uuid, user_id: Uuid, folder_id: Uuid) -> Result<()> {
        self.storage
            .delete_folder(tenant_id, user_id, folder_id)
            .await
            .map_err(Self::map_error)
    }

    async fn import_conversations(
        &self,
        _tenant_id: Uuid,
        _user_id: Uuid,
        conversations: Vec<serde_json::Value>,
    ) -> Result<ImportResult> {
        // TODO: Implement import functionality
        Ok(ImportResult {
            imported: 0,
            failed: conversations.len(),
            errors: conversations
                .iter()
                .enumerate()
                .map(|(i, _)| ImportError {
                    id: format!("conv_{}", i),
                    error: "Import not yet implemented for PostgreSQL".to_string(),
                })
                .collect(),
        })
    }

    async fn bulk_delete(&self, conversation_ids: Vec<Uuid>) -> Result<usize> {
        self.storage
            .bulk_delete(&conversation_ids)
            .await
            .map_err(Self::map_error)
    }

    async fn bulk_archive(&self, conversation_ids: Vec<Uuid>, archive: bool) -> Result<usize> {
        self.storage
            .bulk_archive(&conversation_ids, archive)
            .await
            .map_err(Self::map_error)
    }

    async fn bulk_move_to_folder(
        &self,
        conversation_ids: Vec<Uuid>,
        folder_id: Option<Uuid>,
    ) -> Result<usize> {
        self.storage
            .bulk_move_to_folder(&conversation_ids, folder_id)
            .await
            .map_err(Self::map_error)
    }
}
