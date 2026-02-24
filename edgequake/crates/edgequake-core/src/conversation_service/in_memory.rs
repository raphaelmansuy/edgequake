//! In-memory implementation of ConversationService for testing.

use std::collections::HashMap;
use std::sync::RwLock;

use async_trait::async_trait;
use uuid::Uuid;

use crate::error::Result;
use crate::types::{
    Conversation, ConversationFilter, ConversationMode, ConversationSortField,
    CreateConversationRequest, CreateMessageRequest, Folder, ImportError, ImportResult, Message,
    MessageRole, PaginatedConversations, PaginatedMessages, PaginationMeta,
    UpdateConversationRequest, UpdateMessageRequest,
};

use super::ConversationService;

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
        _tenant_id: Uuid,
        _user_id: Uuid,
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
        // WHY: Double-option pattern - Some(inner) means "update folder_id",
        // where inner can be None (remove from folder) or Some(uuid) (move to folder)
        if let Some(folder_id_update) = request.folder_id {
            conv.folder_id = folder_id_update;
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
                // WHY: unfiled filter returns only conversations without any folder
                if filter.unfiled == Some(true) && c.folder_id.is_some() {
                    return false;
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
        _tenant_id: Uuid,
        _user_id: Uuid,
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

    async fn delete_folder(&self, _tenant_id: Uuid, _user_id: Uuid, folder_id: Uuid) -> Result<()> {
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
    /// Import a single conversation from JSON.
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
