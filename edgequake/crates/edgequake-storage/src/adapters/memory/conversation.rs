//! In-memory conversation storage (SPEC-017 STORE-SOLID-L-002).
//!
//! Mirrors the postgres conversation API for unit/integration tests without a database.

use chrono::Utc;
use std::collections::HashMap;
use std::sync::RwLock;
use uuid::Uuid;

use crate::conversation_types::{ConversationRow, FolderRow, MessageRow};
use crate::error::{Result, StorageError};

use super::lock::map_lock_err;

/// In-memory conversation, message, and folder storage.
#[derive(Debug, Default)]
pub struct MemoryConversationStorage {
    conversations: RwLock<HashMap<Uuid, ConversationRow>>,
    messages: RwLock<HashMap<Uuid, MessageRow>>,
    folders: RwLock<HashMap<Uuid, FolderRow>>,
}

impl MemoryConversationStorage {
    pub fn new() -> Self {
        Self::default()
    }

    fn generate_share_id() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        format!("share_{ts}")
    }

    pub async fn create_conversation(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        workspace_id: Option<Uuid>,
        title: String,
        mode: String,
        folder_id: Option<Uuid>,
    ) -> Result<ConversationRow> {
        let now = Utc::now();
        let row = ConversationRow {
            conversation_id: Uuid::new_v4(),
            tenant_id,
            workspace_id,
            user_id,
            title,
            mode,
            is_pinned: false,
            is_archived: false,
            folder_id,
            share_id: None,
            meta: serde_json::json!({}),
            created_at: now,
            updated_at: now,
        };
        self.conversations
            .write()
            .map_err(map_lock_err)?
            .insert(row.conversation_id, row.clone());
        Ok(row)
    }

    pub async fn get_conversation(&self, conversation_id: Uuid) -> Result<Option<ConversationRow>> {
        Ok(self
            .conversations
            .read()
            .map_err(map_lock_err)?
            .get(&conversation_id)
            .cloned())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn update_conversation(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        conversation_id: Uuid,
        title: Option<String>,
        mode: Option<String>,
        is_pinned: Option<bool>,
        is_archived: Option<bool>,
        folder_id: Option<Option<Uuid>>,
    ) -> Result<ConversationRow> {
        let mut conversations = self.conversations.write().map_err(map_lock_err)?;
        let row = conversations.get_mut(&conversation_id).ok_or_else(|| {
            StorageError::NotFound(format!("Conversation {conversation_id} not found"))
        })?;

        if row.tenant_id != tenant_id || row.user_id != user_id {
            return Err(StorageError::NotFound(format!(
                "Conversation {conversation_id} not found"
            )));
        }

        if let Some(t) = title {
            row.title = t;
        }
        if let Some(m) = mode {
            row.mode = m;
        }
        if let Some(p) = is_pinned {
            row.is_pinned = p;
        }
        if let Some(a) = is_archived {
            row.is_archived = a;
        }
        if let Some(f) = folder_id {
            row.folder_id = f;
        }
        row.updated_at = Utc::now();
        Ok(row.clone())
    }

    pub async fn delete_conversation(&self, conversation_id: Uuid) -> Result<()> {
        let removed = self
            .conversations
            .write()
            .map_err(map_lock_err)?
            .remove(&conversation_id)
            .is_some();
        if !removed {
            return Err(StorageError::NotFound(format!(
                "Conversation {conversation_id} not found"
            )));
        }
        let mut messages = self.messages.write().map_err(map_lock_err)?;
        messages.retain(|_, m| m.conversation_id != conversation_id);
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn list_conversations(
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
    ) -> Result<(Vec<ConversationRow>, i64)> {
        let conversations = self.conversations.read().map_err(map_lock_err)?;
        let search_lower = search.map(|s| s.to_lowercase());

        let mut rows: Vec<ConversationRow> = conversations
            .values()
            .filter(|c| c.tenant_id == tenant_id && c.user_id == user_id)
            .filter(|c| archived.is_none_or(|a| c.is_archived == a))
            .filter(|c| pinned.is_none_or(|p| c.is_pinned == p))
            .filter(|c| folder_id.is_none_or(|f| c.folder_id == Some(f)))
            .filter(|c| unfiled.is_none_or(|u| u == (c.folder_id.is_none())))
            .filter(|c| {
                search_lower
                    .as_ref()
                    .is_none_or(|q| c.title.to_lowercase().contains(q))
            })
            .cloned()
            .collect();

        match sort_field {
            "title" => rows.sort_by(|a, b| a.title.cmp(&b.title)),
            "updated_at" => rows.sort_by_key(|a| a.updated_at),
            _ => rows.sort_by_key(|a| a.created_at),
        }
        if sort_desc {
            rows.reverse();
        }

        let total = rows.len() as i64;
        let items = rows
            .into_iter()
            .skip(offset.max(0) as usize)
            .take(limit.max(0) as usize)
            .collect();
        Ok((items, total))
    }

    pub async fn share_conversation(&self, conversation_id: Uuid) -> Result<String> {
        let share_id = Self::generate_share_id();
        let mut conversations = self.conversations.write().map_err(map_lock_err)?;
        let row = conversations.get_mut(&conversation_id).ok_or_else(|| {
            StorageError::NotFound(format!("Conversation {conversation_id} not found"))
        })?;
        row.share_id = Some(share_id.clone());
        row.updated_at = Utc::now();
        Ok(share_id)
    }

    pub async fn unshare_conversation(&self, conversation_id: Uuid) -> Result<()> {
        let mut conversations = self.conversations.write().map_err(map_lock_err)?;
        let row = conversations.get_mut(&conversation_id).ok_or_else(|| {
            StorageError::NotFound(format!("Conversation {conversation_id} not found"))
        })?;
        row.share_id = None;
        row.updated_at = Utc::now();
        Ok(())
    }

    pub async fn get_shared_conversation(&self, share_id: &str) -> Result<Option<ConversationRow>> {
        let conversations = self.conversations.read().map_err(map_lock_err)?;
        Ok(conversations
            .values()
            .find(|c| c.share_id.as_deref() == Some(share_id))
            .cloned())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create_message(
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
    ) -> Result<MessageRow> {
        if !self
            .conversations
            .read()
            .map_err(map_lock_err)?
            .contains_key(&conversation_id)
        {
            return Err(StorageError::NotFound(format!(
                "Conversation {conversation_id} not found"
            )));
        }

        let now = Utc::now();
        let row = MessageRow {
            message_id: Uuid::new_v4(),
            conversation_id,
            parent_id,
            role: role.to_string(),
            content: content.to_string(),
            mode: mode.map(str::to_string),
            tokens_used,
            duration_ms,
            thinking_time_ms,
            context,
            is_error,
            created_at: now,
            updated_at: now,
        };
        self.messages
            .write()
            .map_err(map_lock_err)?
            .insert(row.message_id, row.clone());
        Ok(row)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn update_message(
        &self,
        message_id: Uuid,
        content: Option<&str>,
        tokens_used: Option<i32>,
        duration_ms: Option<i32>,
        thinking_time_ms: Option<i32>,
        context: Option<serde_json::Value>,
        is_error: Option<bool>,
    ) -> Result<MessageRow> {
        let mut messages = self.messages.write().map_err(map_lock_err)?;
        let row = messages
            .get_mut(&message_id)
            .ok_or_else(|| StorageError::NotFound(format!("Message {message_id} not found")))?;
        if let Some(c) = content {
            row.content = c.to_string();
        }
        if let Some(t) = tokens_used {
            row.tokens_used = Some(t);
        }
        if let Some(d) = duration_ms {
            row.duration_ms = Some(d);
        }
        if let Some(t) = thinking_time_ms {
            row.thinking_time_ms = Some(t);
        }
        if let Some(ctx) = context {
            row.context = Some(ctx);
        }
        if let Some(e) = is_error {
            row.is_error = e;
        }
        row.updated_at = Utc::now();
        Ok(row.clone())
    }

    pub async fn delete_message(&self, message_id: Uuid) -> Result<()> {
        let removed = self
            .messages
            .write()
            .map_err(map_lock_err)?
            .remove(&message_id)
            .is_some();
        if !removed {
            return Err(StorageError::NotFound(format!(
                "Message {message_id} not found"
            )));
        }
        Ok(())
    }

    pub async fn list_messages(
        &self,
        conversation_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<MessageRow>, i64)> {
        let messages = self.messages.read().map_err(map_lock_err)?;
        let mut rows: Vec<MessageRow> = messages
            .values()
            .filter(|m| m.conversation_id == conversation_id)
            .cloned()
            .collect();
        rows.sort_by_key(|m| m.created_at);
        let total = rows.len() as i64;
        let page = rows
            .into_iter()
            .skip(offset.max(0) as usize)
            .take(limit.max(0) as usize)
            .collect();
        Ok((page, total))
    }

    pub async fn get_message_count(&self, conversation_id: Uuid) -> Result<i64> {
        let messages = self.messages.read().map_err(map_lock_err)?;
        let count = messages
            .values()
            .filter(|m| m.conversation_id == conversation_id)
            .count();
        Ok(count as i64)
    }

    pub async fn create_folder(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        workspace_id: Option<Uuid>,
        name: &str,
        parent_id: Option<Uuid>,
    ) -> Result<FolderRow> {
        let folders = self.folders.read().map_err(map_lock_err)?;
        let max_pos = folders
            .values()
            .filter(|f| {
                f.tenant_id == tenant_id && f.user_id == user_id && f.parent_id == parent_id
            })
            .map(|f| f.position)
            .max()
            .unwrap_or(0);
        drop(folders);

        let now = Utc::now();
        let row = FolderRow {
            folder_id: Uuid::new_v4(),
            tenant_id,
            workspace_id,
            user_id,
            name: name.to_string(),
            parent_id,
            position: max_pos + 1,
            created_at: now,
            updated_at: now,
        };
        self.folders
            .write()
            .map_err(map_lock_err)?
            .insert(row.folder_id, row.clone());
        Ok(row)
    }

    pub async fn list_folders(&self, tenant_id: Uuid, user_id: Uuid) -> Result<Vec<FolderRow>> {
        let folders = self.folders.read().map_err(map_lock_err)?;
        let mut rows: Vec<FolderRow> = folders
            .values()
            .filter(|f| f.tenant_id == tenant_id && f.user_id == user_id)
            .cloned()
            .collect();
        rows.sort_by_key(|f| f.position);
        Ok(rows)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn update_folder(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        folder_id: Uuid,
        name: Option<&str>,
        parent_id: Option<Uuid>,
        position: Option<i32>,
    ) -> Result<FolderRow> {
        let mut folders = self.folders.write().map_err(map_lock_err)?;
        let row = folders
            .get_mut(&folder_id)
            .ok_or_else(|| StorageError::NotFound(format!("Folder {folder_id} not found")))?;
        if row.tenant_id != tenant_id || row.user_id != user_id {
            return Err(StorageError::NotFound(format!(
                "Folder {folder_id} not found"
            )));
        }
        if let Some(n) = name {
            row.name = n.to_string();
        }
        if let Some(p) = parent_id {
            row.parent_id = Some(p);
        }
        if let Some(p) = position {
            row.position = p;
        }
        row.updated_at = Utc::now();
        Ok(row.clone())
    }

    pub async fn delete_folder(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        folder_id: Uuid,
    ) -> Result<()> {
        let mut folders = self.folders.write().map_err(map_lock_err)?;
        let row = folders
            .get(&folder_id)
            .ok_or_else(|| StorageError::NotFound(format!("Folder {folder_id} not found")))?;
        if row.tenant_id != tenant_id || row.user_id != user_id {
            return Err(StorageError::NotFound(format!(
                "Folder {folder_id} not found"
            )));
        }
        folders.remove(&folder_id);

        let mut conversations = self.conversations.write().map_err(map_lock_err)?;
        for conv in conversations.values_mut() {
            if conv.folder_id == Some(folder_id) {
                conv.folder_id = None;
            }
        }
        Ok(())
    }

    pub async fn bulk_delete(&self, conversation_ids: &[Uuid]) -> Result<usize> {
        let mut count = 0;
        for id in conversation_ids {
            if self.delete_conversation(*id).await.is_ok() {
                count += 1;
            }
        }
        Ok(count)
    }

    pub async fn bulk_archive(&self, conversation_ids: &[Uuid], archive: bool) -> Result<usize> {
        let mut conversations = self.conversations.write().map_err(map_lock_err)?;
        let mut count = 0;
        for id in conversation_ids {
            if let Some(row) = conversations.get_mut(id) {
                row.is_archived = archive;
                row.updated_at = Utc::now();
                count += 1;
            }
        }
        Ok(count)
    }

    pub async fn bulk_move_to_folder(
        &self,
        conversation_ids: &[Uuid],
        folder_id: Option<Uuid>,
    ) -> Result<usize> {
        let mut conversations = self.conversations.write().map_err(map_lock_err)?;
        let mut count = 0;
        for id in conversation_ids {
            if let Some(row) = conversations.get_mut(id) {
                row.folder_id = folder_id;
                row.updated_at = Utc::now();
                count += 1;
            }
        }
        Ok(count)
    }
}

use crate::conversation_storage::ConversationStorage;
use async_trait::async_trait;

#[async_trait]
impl ConversationStorage for MemoryConversationStorage {
    async fn create_conversation(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        workspace_id: Option<Uuid>,
        title: String,
        mode: String,
        folder_id: Option<Uuid>,
    ) -> Result<ConversationRow> {
        MemoryConversationStorage::create_conversation(
            self,
            tenant_id,
            user_id,
            workspace_id,
            title,
            mode,
            folder_id,
        )
        .await
    }

    async fn get_conversation(&self, conversation_id: Uuid) -> Result<Option<ConversationRow>> {
        MemoryConversationStorage::get_conversation(self, conversation_id).await
    }

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
    ) -> Result<ConversationRow> {
        MemoryConversationStorage::update_conversation(
            self,
            tenant_id,
            user_id,
            conversation_id,
            title,
            mode,
            is_pinned,
            is_archived,
            folder_id,
        )
        .await
    }

    async fn delete_conversation(&self, conversation_id: Uuid) -> Result<()> {
        MemoryConversationStorage::delete_conversation(self, conversation_id).await
    }

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
    ) -> Result<(Vec<ConversationRow>, i64)> {
        MemoryConversationStorage::list_conversations(
            self, tenant_id, user_id, archived, pinned, folder_id, unfiled, search, sort_field,
            sort_desc, limit, offset,
        )
        .await
    }

    async fn share_conversation(&self, conversation_id: Uuid) -> Result<String> {
        MemoryConversationStorage::share_conversation(self, conversation_id).await
    }

    async fn unshare_conversation(&self, conversation_id: Uuid) -> Result<()> {
        MemoryConversationStorage::unshare_conversation(self, conversation_id).await
    }

    async fn get_shared_conversation(&self, share_id: &str) -> Result<Option<ConversationRow>> {
        MemoryConversationStorage::get_shared_conversation(self, share_id).await
    }

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
    ) -> Result<MessageRow> {
        MemoryConversationStorage::create_message(
            self,
            conversation_id,
            parent_id,
            role,
            content,
            mode,
            tokens_used,
            duration_ms,
            thinking_time_ms,
            context,
            is_error,
        )
        .await
    }

    async fn update_message(
        &self,
        message_id: Uuid,
        content: Option<&str>,
        tokens_used: Option<i32>,
        duration_ms: Option<i32>,
        thinking_time_ms: Option<i32>,
        context: Option<serde_json::Value>,
        is_error: Option<bool>,
    ) -> Result<MessageRow> {
        MemoryConversationStorage::update_message(
            self,
            message_id,
            content,
            tokens_used,
            duration_ms,
            thinking_time_ms,
            context,
            is_error,
        )
        .await
    }

    async fn delete_message(&self, message_id: Uuid) -> Result<()> {
        MemoryConversationStorage::delete_message(self, message_id).await
    }

    async fn list_messages(
        &self,
        conversation_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<MessageRow>, i64)> {
        MemoryConversationStorage::list_messages(self, conversation_id, limit, offset).await
    }

    async fn create_folder(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        workspace_id: Option<Uuid>,
        name: &str,
        parent_id: Option<Uuid>,
    ) -> Result<FolderRow> {
        MemoryConversationStorage::create_folder(
            self,
            tenant_id,
            user_id,
            workspace_id,
            name,
            parent_id,
        )
        .await
    }

    async fn list_folders(&self, tenant_id: Uuid, user_id: Uuid) -> Result<Vec<FolderRow>> {
        MemoryConversationStorage::list_folders(self, tenant_id, user_id).await
    }

    async fn update_folder(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        folder_id: Uuid,
        name: Option<&str>,
        parent_id: Option<Uuid>,
        position: Option<i32>,
    ) -> Result<FolderRow> {
        MemoryConversationStorage::update_folder(
            self, tenant_id, user_id, folder_id, name, parent_id, position,
        )
        .await
    }

    async fn delete_folder(&self, tenant_id: Uuid, user_id: Uuid, folder_id: Uuid) -> Result<()> {
        MemoryConversationStorage::delete_folder(self, tenant_id, user_id, folder_id).await
    }

    async fn bulk_delete(&self, conversation_ids: &[Uuid]) -> Result<usize> {
        MemoryConversationStorage::bulk_delete(self, conversation_ids).await
    }

    async fn bulk_archive(&self, conversation_ids: &[Uuid], archive: bool) -> Result<usize> {
        MemoryConversationStorage::bulk_archive(self, conversation_ids, archive).await
    }

    async fn bulk_move_to_folder(
        &self,
        conversation_ids: &[Uuid],
        folder_id: Option<Uuid>,
    ) -> Result<usize> {
        MemoryConversationStorage::bulk_move_to_folder(self, conversation_ids, folder_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn conversation_message_lifecycle() {
        let storage = MemoryConversationStorage::new();
        let tenant = Uuid::new_v4();
        let user = Uuid::new_v4();

        let conv = storage
            .create_conversation(tenant, user, None, "Test".into(), "chat".into(), None)
            .await
            .unwrap();

        storage
            .create_message(
                conv.conversation_id,
                None,
                "user",
                "hello",
                None,
                None,
                None,
                None,
                None,
                false,
            )
            .await
            .unwrap();

        assert_eq!(
            storage
                .get_message_count(conv.conversation_id)
                .await
                .unwrap(),
            1
        );
        let (listed, total) = storage
            .list_conversations(
                tenant,
                user,
                None,
                None,
                None,
                None,
                None,
                "created_at",
                false,
                10,
                0,
            )
            .await
            .unwrap();
        assert_eq!(total, 1);
        assert_eq!(listed[0].conversation_id, conv.conversation_id);
    }
}
