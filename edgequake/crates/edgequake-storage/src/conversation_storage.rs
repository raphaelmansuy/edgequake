//! Conversation persistence trait (SPEC-017 STORE-SOLID-D-001).
//!
//! Single abstraction for memory and postgres conversation adapters.

use async_trait::async_trait;
use uuid::Uuid;

use crate::conversation_types::{ConversationRow, FolderRow, MessageRow};
use crate::error::Result;

/// Storage port for conversations, messages, and folders.
#[async_trait]
pub trait ConversationStorage: Send + Sync {
    async fn create_conversation(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        workspace_id: Option<Uuid>,
        title: String,
        mode: String,
        folder_id: Option<Uuid>,
    ) -> Result<ConversationRow>;

    async fn get_conversation(&self, conversation_id: Uuid) -> Result<Option<ConversationRow>>;

    #[allow(clippy::too_many_arguments)]
    async fn update_conversation(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        conversation_id: Uuid,
        title: Option<String>,
        mode: Option<String>,
        is_pinned: Option<bool>,
        is_archived: Option<bool>,
        folder_id: Option<Option<Uuid>>,
    ) -> Result<ConversationRow>;

    async fn delete_conversation(&self, conversation_id: Uuid) -> Result<()>;

    #[allow(clippy::too_many_arguments)]
    async fn list_conversations(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        archived: Option<bool>,
        pinned: Option<bool>,
        folder_id: Option<Uuid>,
        unfiled: Option<bool>,
        search: Option<&str>,
        sort_field: &str,
        sort_desc: bool,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<ConversationRow>, i64)>;

    async fn share_conversation(&self, conversation_id: Uuid) -> Result<String>;

    async fn unshare_conversation(&self, conversation_id: Uuid) -> Result<()>;

    async fn get_shared_conversation(&self, share_id: &str) -> Result<Option<ConversationRow>>;

    #[allow(clippy::too_many_arguments)]
    async fn create_message(
        &self,
        conversation_id: Uuid,
        parent_id: Option<Uuid>,
        role: &str,
        content: &str,
        mode: Option<&str>,
        tokens_used: Option<i32>,
        duration_ms: Option<i32>,
        thinking_time_ms: Option<i32>,
        context: Option<serde_json::Value>,
        is_error: bool,
    ) -> Result<MessageRow>;

    #[allow(clippy::too_many_arguments)]
    async fn update_message(
        &self,
        message_id: Uuid,
        content: Option<&str>,
        tokens_used: Option<i32>,
        duration_ms: Option<i32>,
        thinking_time_ms: Option<i32>,
        context: Option<serde_json::Value>,
        is_error: Option<bool>,
    ) -> Result<MessageRow>;

    async fn delete_message(&self, message_id: Uuid) -> Result<()>;

    async fn list_messages(
        &self,
        conversation_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<MessageRow>, i64)>;

    async fn create_folder(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        workspace_id: Option<Uuid>,
        name: &str,
        parent_id: Option<Uuid>,
    ) -> Result<FolderRow>;

    async fn list_folders(&self, tenant_id: Uuid, user_id: Uuid) -> Result<Vec<FolderRow>>;

    #[allow(clippy::too_many_arguments)]
    async fn update_folder(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        folder_id: Uuid,
        name: Option<&str>,
        parent_id: Option<Uuid>,
        position: Option<i32>,
    ) -> Result<FolderRow>;

    async fn delete_folder(&self, tenant_id: Uuid, user_id: Uuid, folder_id: Uuid) -> Result<()>;

    async fn bulk_delete(&self, conversation_ids: &[Uuid]) -> Result<usize>;

    async fn bulk_archive(&self, conversation_ids: &[Uuid], archive: bool) -> Result<usize>;

    async fn bulk_move_to_folder(
        &self,
        conversation_ids: &[Uuid],
        folder_id: Option<Uuid>,
    ) -> Result<usize>;
}
