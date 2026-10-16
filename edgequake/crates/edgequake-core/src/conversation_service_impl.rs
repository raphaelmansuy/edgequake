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
use crate::conversation_service::{
    encode_time_cursor, encode_title_cursor, keyset_page_meta, pagination_meta, parse_list_cursor,
    ParsedCursor,
};
#[cfg(feature = "postgres")]
use crate::{
    conversation_service::ConversationService,
    error::{Error, Result},
    types::SetMessageFeedbackRequest,
    types::{
        Conversation, ConversationFilter, ConversationSortField, CreateConversationRequest,
        CreateMessageRequest, Folder, ImportError, ImportResult, Message, MessageRole,
        PaginatedConversations, PaginatedMessages, PaginationMeta, UpdateConversationRequest,
        UpdateMessageRequest,
    },
};
#[cfg(feature = "postgres")]
use edgequake_storage::conversation_storage::ListWindow;
#[cfg(feature = "postgres")]
use edgequake_storage::{ConversationStorage, PostgresConversationStorage};

/// PostgreSQL implementation of ConversationService.
#[cfg(feature = "postgres")]
#[derive(Clone)]
pub struct ConversationServiceImpl {
    storage: Arc<dyn ConversationStorage>,
}

#[cfg(feature = "postgres")]
impl ConversationServiceImpl {
    /// Create a new PostgreSQL conversation service.
    pub fn new(pool: PgPool) -> Self {
        Self {
            storage: Arc::new(PostgresConversationStorage::new(pool)),
        }
    }

    /// Create from any conversation storage adapter (memory or postgres).
    pub fn from_storage(storage: Arc<dyn ConversationStorage>) -> Self {
        Self { storage }
    }

    /// Create from a postgres storage instance.
    pub fn from_postgres_storage(storage: PostgresConversationStorage) -> Self {
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
            llm_provider: row.llm_provider,
            llm_model: row.llm_model,
            feedback_rating: row.feedback_rating,
            feedback_reason: row.feedback_reason,
            finish_reason: row.finish_reason,
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
fn conversation_window(cursor: ParsedCursor, field: &str, desc: bool) -> (ListWindow, usize) {
    match cursor {
        ParsedCursor::Offset(offset) => (ListWindow::Offset(offset as i64), offset),
        ParsedCursor::AfterTime {
            field: cursor_field,
            desc: cursor_desc,
            at,
            id,
        } if cursor_field == field && cursor_desc == desc => (ListWindow::AfterTime { at, id }, 0),
        ParsedCursor::AfterTitle {
            desc: cursor_desc,
            title,
            id,
        } if field == "title" && cursor_desc == desc => (ListWindow::AfterTitle { title, id }, 0),
        ParsedCursor::AfterTime { .. } | ParsedCursor::AfterTitle { .. } => {
            (ListWindow::Offset(0), 0)
        }
    }
}

#[cfg(feature = "postgres")]
fn conversation_cursor(item: &Conversation, field: &str, desc: bool) -> String {
    if field == "title" {
        encode_title_cursor(desc, &item.title, item.conversation_id)
    } else {
        let at = if field == "created_at" {
            item.created_at
        } else {
            item.updated_at
        };
        encode_time_cursor(field, desc, at, item.conversation_id)
    }
}

#[cfg(feature = "postgres")]
fn finish_page<T>(
    items: &mut Vec<T>,
    keyset: bool,
    offset: usize,
    limit: usize,
    total: usize,
    cursor_of: impl Fn(&T) -> String,
) -> PaginationMeta {
    if keyset {
        let has_more = items.len() > limit;
        if has_more {
            items.truncate(limit);
        }
        let next = has_more.then(|| items.last().map(&cursor_of)).flatten();
        keyset_page_meta(has_more, total, next)
    } else {
        let mut meta = pagination_meta(offset, limit, items.len(), total);
        if meta.has_more {
            if let Some(last) = items.last() {
                meta.next_cursor = Some(cursor_of(last));
            }
        }
        meta
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
        cursor: Option<String>,
        limit: usize,
    ) -> Result<PaginatedConversations> {
        let sort_field = match sort {
            ConversationSortField::UpdatedAt => "updated_at",
            ConversationSortField::CreatedAt => "created_at",
            ConversationSortField::Title => "title",
        };

        let (window, offset) =
            conversation_window(parse_list_cursor(cursor.as_deref()), sort_field, sort_desc);
        let keyset = !matches!(window, ListWindow::Offset(_));
        let fetch_limit = if keyset {
            limit.saturating_add(1) as i64
        } else {
            limit as i64
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
                fetch_limit,
                window,
            )
            .await
            .map_err(Self::map_error)?;

        let mut items: Vec<Conversation> =
            rows.into_iter().map(Self::row_to_conversation).collect();
        let pagination = finish_page(&mut items, keyset, offset, limit, total as usize, |item| {
            conversation_cursor(item, sort_field, sort_desc)
        });

        Ok(PaginatedConversations { items, pagination })
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
        let mode_str = request.mode.as_ref().map(|m| m.to_string());
        let row = self
            .storage
            .update_message(
                message_id,
                request.content.as_deref(),
                mode_str.as_deref(),
                request.tokens_used,
                request.duration_ms,
                request.thinking_time_ms,
                request
                    .context
                    .map(|c| serde_json::to_value(c).unwrap_or_default()),
                request.is_error,
                request.llm_provider.as_deref(),
                request.llm_model.as_deref(),
                request.finish_reason.as_deref(),
            )
            .await
            .map_err(Self::map_error)?;

        Ok(Self::row_to_message(row))
    }

    async fn set_message_feedback(
        &self,
        conversation_id: Uuid,
        message_id: Uuid,
        request: SetMessageFeedbackRequest,
    ) -> Result<Message> {
        let row = self
            .storage
            .update_message_feedback(
                conversation_id,
                message_id,
                request.rating.as_deref(),
                request.reason.as_deref(),
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
        cursor: Option<String>,
        limit: usize,
    ) -> Result<PaginatedMessages> {
        let (window, offset) =
            conversation_window(parse_list_cursor(cursor.as_deref()), "created_at", false);
        let keyset = !matches!(window, ListWindow::Offset(_));
        let fetch_limit = if keyset {
            limit.saturating_add(1) as i64
        } else {
            limit as i64
        };
        let (rows, total) = self
            .storage
            .list_messages(conversation_id, fetch_limit, window)
            .await
            .map_err(Self::map_error)?;

        let mut items: Vec<Message> = rows.into_iter().map(Self::row_to_message).collect();
        let pagination = finish_page(&mut items, keyset, offset, limit, total as usize, |item| {
            encode_time_cursor("created_at", false, item.created_at, item.message_id)
        });

        Ok(PaginatedMessages { items, pagination })
    }

    async fn list_recent_messages(
        &self,
        conversation_id: Uuid,
        limit: usize,
    ) -> Result<Vec<Message>> {
        let rows = self
            .storage
            .list_recent_messages(conversation_id, limit as i64)
            .await
            .map_err(Self::map_error)?;
        Ok(rows.into_iter().map(Self::row_to_message).collect())
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
