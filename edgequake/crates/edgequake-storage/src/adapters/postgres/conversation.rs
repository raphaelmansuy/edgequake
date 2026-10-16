//! PostgreSQL conversation storage implementation.
//!
//! This module provides a PostgreSQL-based implementation of the ConversationService
//! trait for persisting conversations, messages, and folders.
//!
//! ## Implements
//!
//! - [`FEAT0250`]: Conversation persistence
//! - [`FEAT0251`]: Message storage with ordering
//! - [`FEAT0252`]: Folder organization for conversations
//! - [`FEAT0253`]: Share ID generation for public links
//!
//! ## Use Cases
//!
//! - [`UC0801`]: System stores conversation history
//! - [`UC0802`]: User organizes conversations into folders
//! - [`UC0803`]: User shares conversation via public link
//!
//! ## Enforces
//!
//! - [`BR0250`]: RLS-based user isolation
//! - [`BR0251`]: Message ordering by timestamp

use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

use super::rls::with_rls_transaction;
use crate::conversation_storage::ListWindow;
use crate::conversation_types::{ConversationRow, FolderRow, MessageRow};
use crate::error::{Result, StorageError};

#[cfg(feature = "postgres")]
mod sqlx_rows {
    use super::{ConversationRow, FolderRow, MessageRow};
    use sqlx::Row;

    impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for ConversationRow {
        fn from_row(row: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
            Ok(Self {
                conversation_id: row.try_get("conversation_id")?,
                tenant_id: row.try_get("tenant_id")?,
                workspace_id: row.try_get("workspace_id")?,
                user_id: row.try_get("user_id")?,
                title: row.try_get("title")?,
                mode: row.try_get("mode")?,
                is_pinned: row.try_get("is_pinned")?,
                is_archived: row.try_get("is_archived")?,
                folder_id: row.try_get("folder_id")?,
                share_id: row.try_get("share_id")?,
                meta: row.try_get("meta")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            })
        }
    }

    impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for MessageRow {
        fn from_row(row: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
            Ok(Self {
                message_id: row.try_get("message_id")?,
                conversation_id: row.try_get("conversation_id")?,
                parent_id: row.try_get("parent_id")?,
                role: row.try_get("role")?,
                content: row.try_get("content")?,
                mode: row.try_get("mode")?,
                tokens_used: row.try_get("tokens_used")?,
                duration_ms: row.try_get("duration_ms")?,
                thinking_time_ms: row.try_get("thinking_time_ms")?,
                context: row.try_get("context")?,
                is_error: row.try_get("is_error")?,
                llm_provider: row.try_get("llm_provider").unwrap_or(None),
                llm_model: row.try_get("llm_model").unwrap_or(None),
                feedback_rating: row.try_get("feedback_rating").unwrap_or(None),
                feedback_reason: row.try_get("feedback_reason").unwrap_or(None),
                finish_reason: row.try_get("finish_reason").unwrap_or(None),
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            })
        }
    }

    impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for FolderRow {
        fn from_row(row: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
            Ok(Self {
                folder_id: row.try_get("folder_id")?,
                tenant_id: row.try_get("tenant_id")?,
                workspace_id: row.try_get("workspace_id")?,
                user_id: row.try_get("user_id")?,
                name: row.try_get("name")?,
                parent_id: row.try_get("parent_id")?,
                position: row.try_get("position")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            })
        }
    }
}

/// PostgreSQL conversation storage.
#[derive(Debug, Clone)]
pub struct PostgresConversationStorage {
    pool: Arc<PgPool>,
}

impl PostgresConversationStorage {
    /// Create a new PostgreSQL conversation storage.
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool: Arc::new(pool),
        }
    }

    /// Create from an Arc pool.
    pub fn from_arc(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Get the underlying pool.
    pub fn pool(&self) -> &PgPool {
        &self.pool
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

impl PostgresConversationStorage {
    // ============ Conversation Operations ============

    /// Create a new conversation.
    pub async fn create_conversation(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        workspace_id: Option<Uuid>,
        title: String,
        mode: String,
        folder_id: Option<Uuid>,
    ) -> Result<ConversationRow> {
        // SPEC-083 S-03: GUC must live inside BEGIN…COMMIT (with_rls_transaction).
        with_rls_transaction(
            &self.pool,
            tenant_id,
            workspace_id,
            Some(user_id),
            move |conn| {
                let title = title.clone();
                let mode = mode.clone();
                Box::pin(async move {
                    sqlx::query_as::<_, ConversationRow>(
                        r#"
                        INSERT INTO conversations (
                            tenant_id, workspace_id, user_id, title, mode, folder_id, meta
                        ) VALUES ($1, $2, $3, $4, $5, $6, '{}')
                        RETURNING *
                        "#,
                    )
                    .bind(tenant_id)
                    .bind(workspace_id)
                    .bind(user_id)
                    .bind(&title)
                    .bind(&mode)
                    .bind(folder_id)
                    .fetch_one(&mut *conn)
                    .await
                    .map_err(|e| {
                        StorageError::Database(format!("Failed to create conversation: {e}"))
                    })
                })
            },
        )
        .await
    }

    /// Get a conversation by ID.
    pub async fn get_conversation(&self, conversation_id: Uuid) -> Result<Option<ConversationRow>> {
        let row = sqlx::query_as::<_, ConversationRow>(
            "SELECT * FROM conversations WHERE conversation_id = $1",
        )
        .bind(conversation_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to get conversation: {}", e)))?;

        Ok(row)
    }

    /// Update a conversation.
    ///
    /// # Arguments
    /// * `folder_id` - Double option for folder assignment:
    ///   - `None`: don't change folder
    ///   - `Some(None)`: remove from folder (set folder_id to NULL)
    ///   - `Some(Some(uuid))`: move to folder with this UUID
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
        // Build dynamic update query
        let mut updates = Vec::new();
        let mut param_count = 1;

        if title.is_some() {
            param_count += 1;
            updates.push(format!("title = ${}", param_count));
        }
        if mode.is_some() {
            param_count += 1;
            updates.push(format!("mode = ${}", param_count));
        }
        if is_pinned.is_some() {
            param_count += 1;
            updates.push(format!("is_pinned = ${}", param_count));
        }
        if is_archived.is_some() {
            param_count += 1;
            updates.push(format!("is_archived = ${}", param_count));
        }
        if folder_id.is_some() {
            param_count += 1;
            updates.push(format!("folder_id = ${}", param_count));
        }

        if updates.is_empty() {
            // SPEC-083 S-03: GUC must live inside BEGIN…COMMIT (with_rls_transaction).
            return with_rls_transaction(&self.pool, tenant_id, None, Some(user_id), move |conn| {
                Box::pin(async move {
                    sqlx::query_as::<_, ConversationRow>(
                        "SELECT * FROM conversations WHERE conversation_id = $1",
                    )
                    .bind(conversation_id)
                    .fetch_optional(&mut *conn)
                    .await
                    .map_err(|e| StorageError::Database(format!("Failed to get conversation: {e}")))
                    .and_then(|row| {
                        row.ok_or_else(|| {
                            StorageError::NotFound(format!(
                                "Conversation {conversation_id} not found"
                            ))
                        })
                    })
                })
            })
            .await;
        }

        let tenant_param = param_count + 1;
        let user_param = param_count + 2;

        let query = format!(
            "UPDATE conversations SET {} WHERE conversation_id = $1 AND tenant_id = ${} AND user_id = ${} RETURNING *",
            updates.join(", "),
            tenant_param,
            user_param
        );

        // SPEC-083 S-03: GUC must live inside BEGIN…COMMIT (with_rls_transaction).
        with_rls_transaction(&self.pool, tenant_id, None, Some(user_id), move |conn| {
            Box::pin(async move {
                let mut query_builder =
                    sqlx::query_as::<_, ConversationRow>(&query).bind(conversation_id);

                if let Some(t) = &title {
                    query_builder = query_builder.bind(t);
                }
                if let Some(m) = &mode {
                    query_builder = query_builder.bind(m);
                }
                if let Some(p) = is_pinned {
                    query_builder = query_builder.bind(p);
                }
                if let Some(a) = is_archived {
                    query_builder = query_builder.bind(a);
                }
                if let Some(inner_folder) = &folder_id {
                    query_builder = query_builder.bind(inner_folder);
                }

                query_builder = query_builder.bind(tenant_id).bind(user_id);

                query_builder.fetch_one(&mut *conn).await.map_err(|e| {
                    StorageError::Database(format!("Failed to update conversation: {e}"))
                })
            })
        })
        .await
    }

    /// Delete a conversation.
    pub async fn delete_conversation(&self, conversation_id: Uuid) -> Result<()> {
        let result = sqlx::query("DELETE FROM conversations WHERE conversation_id = $1")
            .bind(conversation_id)
            .execute(&*self.pool)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to delete conversation: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(StorageError::NotFound(format!(
                "Conversation {} not found",
                conversation_id
            )));
        }

        Ok(())
    }

    /// List conversations with filtering and pagination.
    ///
    /// WHY: This function has many parameters because it implements a comprehensive
    /// filtering/pagination API that must support tenant_id, user_id, archived, pinned,
    /// folder_id, unfiled, search, sorting, and pagination - all semantically distinct concerns.
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
        window: ListWindow,
    ) -> Result<(Vec<ConversationRow>, i64)> {
        // Build query with filters
        let mut where_clauses = vec!["tenant_id = $1".to_string(), "user_id = $2".to_string()];
        let mut param_count = 2;

        if archived.is_some() {
            param_count += 1;
            where_clauses.push(format!("is_archived = ${}", param_count));
        }
        if pinned.is_some() {
            param_count += 1;
            where_clauses.push(format!("is_pinned = ${}", param_count));
        }
        if folder_id.is_some() {
            param_count += 1;
            where_clauses.push(format!("folder_id = ${}", param_count));
        }
        if unfiled == Some(true) {
            where_clauses.push("folder_id IS NULL".to_string());
        }
        if search.is_some() {
            param_count += 1;
            // The title predicate is required. A generic plan still prices
            // `@@ $n` as a few dozen hits, so a common word reads every
            // matching title and then sorts. The search replans with the
            // bound term: a common word stops on the user time index, and a
            // rare word uses the title GIN.
            where_clauses.push(format!(
                "to_tsvector('english', title) @@ plainto_tsquery('english', ${})",
                param_count
            ));
        }

        let sort_col = match sort_field {
            "created_at" => "created_at",
            "title" => "title",
            _ => "updated_at",
        };
        let dir = if sort_desc { "DESC" } else { "ASC" };
        let cmp = if sort_desc { "<" } else { ">" };
        let count_where = where_clauses.join(" AND ");
        let keyset_matches = match &window {
            ListWindow::AfterTime { .. } => sort_col != "title",
            ListWindow::AfterTitle { .. } => sort_col == "title",
            ListWindow::Offset(_) => false,
        };
        if keyset_matches {
            param_count += 1;
            let value_param = param_count;
            param_count += 1;
            let id_param = param_count;
            where_clauses.push(format!(
                "({sort_col}, conversation_id) {cmp} (${value_param}, ${id_param})"
            ));
        }
        param_count += 1;
        let limit_param = param_count;
        let offset_sql = if matches!(window, ListWindow::Offset(_)) {
            param_count += 1;
            format!(" OFFSET ${param_count}")
        } else {
            String::new()
        };

        let query = format!(
            r#"
            SELECT * FROM conversations
            WHERE {}
            ORDER BY {sort_col} {dir}, conversation_id {dir}
            LIMIT ${limit_param}{offset_sql}
            "#,
            where_clauses.join(" AND "),
        );

        let count_query = format!("SELECT COUNT(*) FROM conversations WHERE {count_where}");

        let search_owned = search.map(|s| s.to_string());

        // SPEC-083 S-03: GUC must live inside BEGIN…COMMIT (with_rls_transaction).
        with_rls_transaction(&self.pool, tenant_id, None, Some(user_id), move |conn| {
            Box::pin(async move {
                if search_owned.is_some() {
                    sqlx::query("SET LOCAL plan_cache_mode = force_custom_plan")
                        .execute(&mut *conn)
                        .await
                        .map_err(|e| {
                            StorageError::Database(format!(
                                "Failed to set conversation search plan mode: {e}"
                            ))
                        })?;
                }
                let mut query_builder = sqlx::query_as::<_, ConversationRow>(&query)
                    .bind(tenant_id)
                    .bind(user_id);

                let mut count_builder = sqlx::query_scalar::<_, i64>(&count_query)
                    .bind(tenant_id)
                    .bind(user_id);

                if let Some(a) = archived {
                    query_builder = query_builder.bind(a);
                    count_builder = count_builder.bind(a);
                }
                if let Some(p) = pinned {
                    query_builder = query_builder.bind(p);
                    count_builder = count_builder.bind(p);
                }
                if let Some(f) = folder_id {
                    query_builder = query_builder.bind(f);
                    count_builder = count_builder.bind(f);
                }
                if let Some(s) = search_owned.as_deref() {
                    query_builder = query_builder.bind(s);
                    count_builder = count_builder.bind(s);
                }

                if keyset_matches {
                    match &window {
                        ListWindow::AfterTime { at, id } => {
                            query_builder = query_builder.bind(*at).bind(*id);
                        }
                        ListWindow::AfterTitle { title, id } => {
                            query_builder = query_builder.bind(title.clone()).bind(*id);
                        }
                        ListWindow::Offset(_) => {}
                    }
                }
                query_builder = query_builder.bind(limit);
                if let ListWindow::Offset(offset) = &window {
                    query_builder = query_builder.bind(*offset);
                }

                let rows = query_builder.fetch_all(&mut *conn).await.map_err(|e| {
                    StorageError::Database(format!("Failed to list conversations: {e}"))
                })?;

                let total = count_builder.fetch_one(&mut *conn).await.map_err(|e| {
                    StorageError::Database(format!("Failed to count conversations: {e}"))
                })?;

                Ok((rows, total))
            })
        })
        .await
    }

    /// Share a conversation (generate share_id).
    pub async fn share_conversation(&self, conversation_id: Uuid) -> Result<String> {
        let share_id = Self::generate_share_id();

        let result = sqlx::query(
            "UPDATE conversations SET share_id = $1 WHERE conversation_id = $2 AND share_id IS NULL",
        )
        .bind(&share_id)
        .bind(conversation_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to share conversation: {}", e)))?;

        if result.rows_affected() == 0 {
            // Already shared, get existing share_id
            let row: Option<(String,)> =
                sqlx::query_as("SELECT share_id FROM conversations WHERE conversation_id = $1")
                    .bind(conversation_id)
                    .fetch_optional(&*self.pool)
                    .await
                    .map_err(|e| {
                        StorageError::Database(format!("Failed to get share_id: {}", e))
                    })?;

            return row
                .map(|(sid,)| sid)
                .ok_or_else(|| StorageError::NotFound("Conversation not found".to_string()));
        }

        Ok(share_id)
    }

    /// Unshare a conversation (remove share_id).
    pub async fn unshare_conversation(&self, conversation_id: Uuid) -> Result<()> {
        let result =
            sqlx::query("UPDATE conversations SET share_id = NULL WHERE conversation_id = $1")
                .bind(conversation_id)
                .execute(&*self.pool)
                .await
                .map_err(|e| {
                    StorageError::Database(format!("Failed to unshare conversation: {}", e))
                })?;

        if result.rows_affected() == 0 {
            return Err(StorageError::NotFound(format!(
                "Conversation {} not found",
                conversation_id
            )));
        }

        Ok(())
    }

    /// Get a shared conversation by share_id.
    pub async fn get_shared_conversation(&self, share_id: &str) -> Result<Option<ConversationRow>> {
        let row =
            sqlx::query_as::<_, ConversationRow>("SELECT * FROM conversations WHERE share_id = $1")
                .bind(share_id)
                .fetch_optional(&*self.pool)
                .await
                .map_err(|e| {
                    StorageError::Database(format!("Failed to get shared conversation: {}", e))
                })?;

        Ok(row)
    }

    // ============ Message Operations ============

    /// Create a new message.
    ///
    /// WHY: Message creation requires all these parameters to properly record
    /// conversation context, role, content, metrics, and error state.
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
        let row = sqlx::query_as::<_, MessageRow>(
            r#"
            INSERT INTO messages (
                conversation_id, parent_id, role, content, mode,
                tokens_used, duration_ms, thinking_time_ms, context, is_error
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING *
            "#,
        )
        .bind(conversation_id)
        .bind(parent_id)
        .bind(role)
        .bind(content)
        .bind(mode)
        .bind(tokens_used)
        .bind(duration_ms)
        .bind(thinking_time_ms)
        .bind(context)
        .bind(is_error)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to create message: {}", e)))?;

        Ok(row)
    }

    /// Update a message.
    ///
    /// WHY: Message updates allow partial updates to any of these fields,
    /// each representing a distinct aspect of the message (content, metrics, context).
    #[allow(clippy::too_many_arguments)]
    pub async fn update_message(
        &self,
        message_id: Uuid,
        content: Option<&str>,
        mode: Option<&str>,
        tokens_used: Option<i32>,
        duration_ms: Option<i32>,
        thinking_time_ms: Option<i32>,
        context: Option<serde_json::Value>,
        is_error: Option<bool>,
        llm_provider: Option<&str>,
        llm_model: Option<&str>,
        finish_reason: Option<&str>,
    ) -> Result<MessageRow> {
        let mut updates = Vec::new();
        let mut param_count = 1;

        if content.is_some() {
            param_count += 1;
            updates.push(format!("content = ${}", param_count));
        }
        if mode.is_some() {
            param_count += 1;
            updates.push(format!("mode = ${}", param_count));
        }
        if tokens_used.is_some() {
            param_count += 1;
            updates.push(format!("tokens_used = ${}", param_count));
        }
        if duration_ms.is_some() {
            param_count += 1;
            updates.push(format!("duration_ms = ${}", param_count));
        }
        if thinking_time_ms.is_some() {
            param_count += 1;
            updates.push(format!("thinking_time_ms = ${}", param_count));
        }
        if context.is_some() {
            param_count += 1;
            updates.push(format!("context = ${}", param_count));
        }
        if is_error.is_some() {
            param_count += 1;
            updates.push(format!("is_error = ${}", param_count));
        }
        if llm_provider.is_some() {
            param_count += 1;
            updates.push(format!("llm_provider = ${}", param_count));
        }
        if llm_model.is_some() {
            param_count += 1;
            updates.push(format!("llm_model = ${}", param_count));
        }
        if finish_reason.is_some() {
            param_count += 1;
            updates.push(format!("finish_reason = ${}", param_count));
        }

        if updates.is_empty() {
            return self.get_message(message_id).await?.ok_or_else(|| {
                StorageError::NotFound(format!("Message {} not found", message_id))
            });
        }

        let query = format!(
            "UPDATE messages SET {} WHERE message_id = $1 RETURNING *",
            updates.join(", ")
        );

        let mut query_builder = sqlx::query_as::<_, MessageRow>(&query).bind(message_id);

        if let Some(c) = content {
            query_builder = query_builder.bind(c);
        }
        if let Some(m) = mode {
            query_builder = query_builder.bind(m);
        }
        if let Some(t) = tokens_used {
            query_builder = query_builder.bind(t);
        }
        if let Some(d) = duration_ms {
            query_builder = query_builder.bind(d);
        }
        if let Some(tt) = thinking_time_ms {
            query_builder = query_builder.bind(tt);
        }
        if let Some(ctx) = &context {
            query_builder = query_builder.bind(ctx);
        }
        if let Some(e) = is_error {
            query_builder = query_builder.bind(e);
        }
        if let Some(p) = llm_provider {
            query_builder = query_builder.bind(p);
        }
        if let Some(m) = llm_model {
            query_builder = query_builder.bind(m);
        }
        if let Some(fr) = finish_reason {
            query_builder = query_builder.bind(fr);
        }

        let row = query_builder
            .fetch_one(&*self.pool)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to update message: {}", e)))?;

        Ok(row)
    }

    /// Update thumbs feedback on a message (conversation-scoped).
    pub async fn update_message_feedback(
        &self,
        conversation_id: Uuid,
        message_id: Uuid,
        feedback_rating: Option<&str>,
        feedback_reason: Option<&str>,
    ) -> Result<MessageRow> {
        let row = sqlx::query_as::<_, MessageRow>(
            r#"
            UPDATE messages
            SET feedback_rating = $3, feedback_reason = $4
            WHERE message_id = $1 AND conversation_id = $2
            RETURNING *
            "#,
        )
        .bind(message_id)
        .bind(conversation_id)
        .bind(feedback_rating)
        .bind(feedback_reason)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to update message feedback: {e}")))?;

        row.ok_or_else(|| {
            StorageError::NotFound(format!(
                "Message {message_id} not found in conversation {conversation_id}"
            ))
        })
    }

    /// Get a message by ID.
    pub async fn get_message(&self, message_id: Uuid) -> Result<Option<MessageRow>> {
        let row = sqlx::query_as::<_, MessageRow>("SELECT * FROM messages WHERE message_id = $1")
            .bind(message_id)
            .fetch_optional(&*self.pool)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to get message: {}", e)))?;

        Ok(row)
    }

    /// Delete a message.
    pub async fn delete_message(&self, message_id: Uuid) -> Result<()> {
        let result = sqlx::query("DELETE FROM messages WHERE message_id = $1")
            .bind(message_id)
            .execute(&*self.pool)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to delete message: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(StorageError::NotFound(format!(
                "Message {} not found",
                message_id
            )));
        }

        Ok(())
    }

    /// List messages in a conversation.
    pub async fn list_messages(
        &self,
        conversation_id: Uuid,
        limit: i64,
        window: ListWindow,
    ) -> Result<(Vec<MessageRow>, i64)> {
        let rows = match window {
            ListWindow::AfterTime { at, id } => sqlx::query_as::<_, MessageRow>(
                r#"
                SELECT * FROM messages
                WHERE conversation_id = $1
                  AND (created_at, message_id) > ($2, $3)
                ORDER BY created_at ASC, message_id ASC
                LIMIT $4
                "#,
            )
            .bind(conversation_id)
            .bind(at)
            .bind(id)
            .bind(limit)
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to list messages: {}", e)))?,
            other => {
                let offset = match other {
                    ListWindow::Offset(offset) => offset,
                    ListWindow::AfterTime { .. } | ListWindow::AfterTitle { .. } => 0,
                };
                sqlx::query_as::<_, MessageRow>(
                    r#"
                    SELECT * FROM messages
                    WHERE conversation_id = $1
                    ORDER BY created_at ASC, message_id ASC
                    LIMIT $2 OFFSET $3
                    "#,
                )
                .bind(conversation_id)
                .bind(limit)
                .bind(offset)
                .fetch_all(&*self.pool)
                .await
                .map_err(|e| StorageError::Database(format!("Failed to list messages: {}", e)))?
            }
        };

        let total: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM messages WHERE conversation_id = $1")
                .bind(conversation_id)
                .fetch_one(&*self.pool)
                .await
                .map_err(|e| StorageError::Database(format!("Failed to count messages: {}", e)))?;

        Ok((rows, total))
    }

    /// Newest `limit` messages in chronological order.
    ///
    /// `ORDER BY created_at DESC LIMIT` walks `idx_messages_conversation`
    /// backward and stops at the page. The outer sort restores oldest-first
    /// for the caller. A forward `LIMIT` of the same size would return the
    /// start of the thread.
    pub async fn list_recent_messages(
        &self,
        conversation_id: Uuid,
        limit: i64,
    ) -> Result<Vec<MessageRow>> {
        if limit <= 0 {
            return Ok(Vec::new());
        }
        let rows = sqlx::query_as::<_, MessageRow>(
            r#"
            SELECT * FROM (
                SELECT * FROM messages
                WHERE conversation_id = $1
                ORDER BY created_at DESC
                LIMIT $2
            ) recent
            ORDER BY created_at ASC
            "#,
        )
        .bind(conversation_id)
        .bind(limit)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to list recent messages: {e}")))?;

        Ok(rows)
    }

    // ============ Folder Operations ============

    /// Create a new folder.
    pub async fn create_folder(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        workspace_id: Option<Uuid>,
        name: &str,
        parent_id: Option<Uuid>,
    ) -> Result<FolderRow> {
        let name = name.to_string();
        // SPEC-083 S-03: GUC must live inside BEGIN…COMMIT (with_rls_transaction).
        with_rls_transaction(
            &self.pool,
            tenant_id,
            workspace_id,
            Some(user_id),
            move |conn| {
                Box::pin(async move {
                    let max_pos: Option<i32> = sqlx::query_scalar(
                        "SELECT MAX(position) FROM folders WHERE tenant_id = $1 AND user_id = $2 AND parent_id IS NOT DISTINCT FROM $3",
                    )
                    .bind(tenant_id)
                    .bind(user_id)
                    .bind(parent_id)
                    .fetch_one(&mut *conn)
                    .await
                    .map_err(|e| {
                        StorageError::Database(format!("Failed to get max position: {e}"))
                    })?;

                    let position = max_pos.unwrap_or(0) + 1;

                    sqlx::query_as::<_, FolderRow>(
                        r#"
                        INSERT INTO folders (tenant_id, workspace_id, user_id, name, parent_id, position)
                        VALUES ($1, $2, $3, $4, $5, $6)
                        RETURNING *
                        "#,
                    )
                    .bind(tenant_id)
                    .bind(workspace_id)
                    .bind(user_id)
                    .bind(&name)
                    .bind(parent_id)
                    .bind(position)
                    .fetch_one(&mut *conn)
                    .await
                    .map_err(|e| StorageError::Database(format!("Failed to create folder: {e}")))
                })
            },
        )
        .await
    }

    /// List folders for a user.
    pub async fn list_folders(&self, tenant_id: Uuid, user_id: Uuid) -> Result<Vec<FolderRow>> {
        // SPEC-083 S-03: GUC must live inside BEGIN…COMMIT (with_rls_transaction).
        with_rls_transaction(&self.pool, tenant_id, None, Some(user_id), move |conn| {
            Box::pin(async move {
                sqlx::query_as::<_, FolderRow>(
                    r#"
                        SELECT * FROM folders
                        WHERE tenant_id = $1 AND user_id = $2
                        ORDER BY position ASC
                        "#,
                )
                .bind(tenant_id)
                .bind(user_id)
                .fetch_all(&mut *conn)
                .await
                .map_err(|e| StorageError::Database(format!("Failed to list folders: {e}")))
            })
        })
        .await
    }

    /// Update a folder.
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
        let mut updates = Vec::new();
        let mut param_count = 1;

        if name.is_some() {
            param_count += 1;
            updates.push(format!("name = ${}", param_count));
        }
        if parent_id.is_some() {
            param_count += 1;
            updates.push(format!("parent_id = ${}", param_count));
        }
        if position.is_some() {
            param_count += 1;
            updates.push(format!("position = ${}", param_count));
        }

        if updates.is_empty() {
            // SPEC-083 S-03: GUC must live inside BEGIN…COMMIT (with_rls_transaction).
            return with_rls_transaction(&self.pool, tenant_id, None, Some(user_id), move |conn| {
                Box::pin(async move {
                    sqlx::query_as::<_, FolderRow>("SELECT * FROM folders WHERE folder_id = $1")
                        .bind(folder_id)
                        .fetch_optional(&mut *conn)
                        .await
                        .map_err(|e| StorageError::Database(format!("Failed to get folder: {e}")))
                        .and_then(|row| {
                            row.ok_or_else(|| {
                                StorageError::NotFound(format!("Folder {folder_id} not found"))
                            })
                        })
                })
            })
            .await;
        }

        let tenant_param = param_count + 1;
        let user_param = param_count + 2;

        let query = format!(
            "UPDATE folders SET {} WHERE folder_id = $1 AND tenant_id = ${} AND user_id = ${} RETURNING *",
            updates.join(", "),
            tenant_param,
            user_param
        );

        let name_owned = name.map(|n| n.to_string());

        // SPEC-083 S-03: GUC must live inside BEGIN…COMMIT (with_rls_transaction).
        with_rls_transaction(&self.pool, tenant_id, None, Some(user_id), move |conn| {
            Box::pin(async move {
                let mut query_builder = sqlx::query_as::<_, FolderRow>(&query).bind(folder_id);

                if let Some(n) = name_owned.as_deref() {
                    query_builder = query_builder.bind(n);
                }
                if let Some(p) = parent_id {
                    query_builder = query_builder.bind(p);
                }
                if let Some(pos) = position {
                    query_builder = query_builder.bind(pos);
                }

                query_builder = query_builder.bind(tenant_id).bind(user_id);

                query_builder
                    .fetch_one(&mut *conn)
                    .await
                    .map_err(|e| StorageError::Database(format!("Failed to update folder: {e}")))
            })
        })
        .await
    }

    /// Get a folder by ID.
    pub async fn get_folder(&self, folder_id: Uuid) -> Result<Option<FolderRow>> {
        let row = sqlx::query_as::<_, FolderRow>("SELECT * FROM folders WHERE folder_id = $1")
            .bind(folder_id)
            .fetch_optional(&*self.pool)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to get folder: {}", e)))?;

        Ok(row)
    }

    /// Delete a folder.
    pub async fn delete_folder(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        folder_id: Uuid,
    ) -> Result<()> {
        // SPEC-083 S-03: GUC must live inside BEGIN…COMMIT (with_rls_transaction).
        with_rls_transaction(
            &self.pool,
            tenant_id,
            None,
            Some(user_id),
            move |conn| {
                Box::pin(async move {
                    sqlx::query(
                        "UPDATE conversations SET folder_id = NULL WHERE folder_id = $1 AND tenant_id = $2 AND user_id = $3",
                    )
                    .bind(folder_id)
                    .bind(tenant_id)
                    .bind(user_id)
                    .execute(&mut *conn)
                    .await
                    .map_err(|e| {
                        StorageError::Database(format!("Failed to update conversations: {e}"))
                    })?;

                    let result = sqlx::query(
                        "DELETE FROM folders WHERE folder_id = $1 AND tenant_id = $2 AND user_id = $3",
                    )
                    .bind(folder_id)
                    .bind(tenant_id)
                    .bind(user_id)
                    .execute(&mut *conn)
                    .await
                    .map_err(|e| {
                        StorageError::Database(format!("Failed to delete folder: {e}"))
                    })?;

                    if result.rows_affected() == 0 {
                        return Err(StorageError::NotFound(format!(
                            "Folder {folder_id} not found"
                        )));
                    }

                    Ok(())
                })
            },
        )
        .await
    }

    // ============ Bulk Operations ============

    /// Bulk delete conversations.
    pub async fn bulk_delete(&self, conversation_ids: &[Uuid]) -> Result<usize> {
        if conversation_ids.is_empty() {
            return Ok(0);
        }

        let result = sqlx::query("DELETE FROM conversations WHERE conversation_id = ANY($1)")
            .bind(conversation_ids)
            .execute(&*self.pool)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to bulk delete: {}", e)))?;

        Ok(result.rows_affected() as usize)
    }

    /// Bulk archive conversations.
    pub async fn bulk_archive(&self, conversation_ids: &[Uuid], archive: bool) -> Result<usize> {
        if conversation_ids.is_empty() {
            return Ok(0);
        }

        let result = sqlx::query(
            "UPDATE conversations SET is_archived = $1 WHERE conversation_id = ANY($2)",
        )
        .bind(archive)
        .bind(conversation_ids)
        .execute(&*self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to bulk archive: {}", e)))?;

        Ok(result.rows_affected() as usize)
    }

    /// Bulk move to folder.
    pub async fn bulk_move_to_folder(
        &self,
        conversation_ids: &[Uuid],
        folder_id: Option<Uuid>,
    ) -> Result<usize> {
        if conversation_ids.is_empty() {
            return Ok(0);
        }

        let result =
            sqlx::query("UPDATE conversations SET folder_id = $1 WHERE conversation_id = ANY($2)")
                .bind(folder_id)
                .bind(conversation_ids)
                .execute(&*self.pool)
                .await
                .map_err(|e| StorageError::Database(format!("Failed to bulk move: {}", e)))?;

        Ok(result.rows_affected() as usize)
    }

    /// Get message count for a conversation.
    pub async fn get_message_count(&self, conversation_id: Uuid) -> Result<i64> {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM messages WHERE conversation_id = $1")
                .bind(conversation_id)
                .fetch_one(&*self.pool)
                .await
                .map_err(|e| StorageError::Database(format!("Failed to count messages: {}", e)))?;

        Ok(count)
    }

    /// Get last message preview for a conversation.
    pub async fn get_last_message_preview(&self, conversation_id: Uuid) -> Result<Option<String>> {
        let preview: Option<(String,)> = sqlx::query_as(
            r#"
            SELECT SUBSTRING(content, 1, 100) as preview
            FROM messages
            WHERE conversation_id = $1
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(conversation_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to get last message: {}", e)))?;

        Ok(preview.map(|(p,)| p))
    }
}

use crate::conversation_storage::ConversationStorage;
use async_trait::async_trait;

#[async_trait]
impl ConversationStorage for PostgresConversationStorage {
    async fn create_conversation(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        workspace_id: Option<Uuid>,
        title: String,
        mode: String,
        folder_id: Option<Uuid>,
    ) -> Result<ConversationRow> {
        PostgresConversationStorage::create_conversation(
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
        PostgresConversationStorage::get_conversation(self, conversation_id).await
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
        PostgresConversationStorage::update_conversation(
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
        PostgresConversationStorage::delete_conversation(self, conversation_id).await
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
        window: ListWindow,
    ) -> Result<(Vec<ConversationRow>, i64)> {
        PostgresConversationStorage::list_conversations(
            self, tenant_id, user_id, archived, pinned, folder_id, unfiled, search, sort_field,
            sort_desc, limit, window,
        )
        .await
    }

    async fn share_conversation(&self, conversation_id: Uuid) -> Result<String> {
        PostgresConversationStorage::share_conversation(self, conversation_id).await
    }

    async fn unshare_conversation(&self, conversation_id: Uuid) -> Result<()> {
        PostgresConversationStorage::unshare_conversation(self, conversation_id).await
    }

    async fn get_shared_conversation(&self, share_id: &str) -> Result<Option<ConversationRow>> {
        PostgresConversationStorage::get_shared_conversation(self, share_id).await
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
        PostgresConversationStorage::create_message(
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
        mode: Option<&str>,
        tokens_used: Option<i32>,
        duration_ms: Option<i32>,
        thinking_time_ms: Option<i32>,
        context: Option<serde_json::Value>,
        is_error: Option<bool>,
        llm_provider: Option<&str>,
        llm_model: Option<&str>,
        finish_reason: Option<&str>,
    ) -> Result<MessageRow> {
        PostgresConversationStorage::update_message(
            self,
            message_id,
            content,
            mode,
            tokens_used,
            duration_ms,
            thinking_time_ms,
            context,
            is_error,
            llm_provider,
            llm_model,
            finish_reason,
        )
        .await
    }

    async fn update_message_feedback(
        &self,
        conversation_id: Uuid,
        message_id: Uuid,
        feedback_rating: Option<&str>,
        feedback_reason: Option<&str>,
    ) -> Result<MessageRow> {
        PostgresConversationStorage::update_message_feedback(
            self,
            conversation_id,
            message_id,
            feedback_rating,
            feedback_reason,
        )
        .await
    }

    async fn delete_message(&self, message_id: Uuid) -> Result<()> {
        PostgresConversationStorage::delete_message(self, message_id).await
    }

    async fn list_messages(
        &self,
        conversation_id: Uuid,
        limit: i64,
        window: ListWindow,
    ) -> Result<(Vec<MessageRow>, i64)> {
        PostgresConversationStorage::list_messages(self, conversation_id, limit, window).await
    }

    async fn list_recent_messages(
        &self,
        conversation_id: Uuid,
        limit: i64,
    ) -> Result<Vec<MessageRow>> {
        PostgresConversationStorage::list_recent_messages(self, conversation_id, limit).await
    }

    async fn create_folder(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        workspace_id: Option<Uuid>,
        name: &str,
        parent_id: Option<Uuid>,
    ) -> Result<FolderRow> {
        PostgresConversationStorage::create_folder(
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
        PostgresConversationStorage::list_folders(self, tenant_id, user_id).await
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
        PostgresConversationStorage::update_folder(
            self, tenant_id, user_id, folder_id, name, parent_id, position,
        )
        .await
    }

    async fn delete_folder(&self, tenant_id: Uuid, user_id: Uuid, folder_id: Uuid) -> Result<()> {
        PostgresConversationStorage::delete_folder(self, tenant_id, user_id, folder_id).await
    }

    async fn bulk_delete(&self, conversation_ids: &[Uuid]) -> Result<usize> {
        PostgresConversationStorage::bulk_delete(self, conversation_ids).await
    }

    async fn bulk_archive(&self, conversation_ids: &[Uuid], archive: bool) -> Result<usize> {
        PostgresConversationStorage::bulk_archive(self, conversation_ids, archive).await
    }

    async fn bulk_move_to_folder(
        &self,
        conversation_ids: &[Uuid],
        folder_id: Option<Uuid>,
    ) -> Result<usize> {
        PostgresConversationStorage::bulk_move_to_folder(self, conversation_ids, folder_id).await
    }
}

#[cfg(test)]
mod tests {
    // Note: Integration tests require a real database connection
    // and are located in tests/postgres_conversation_integration.rs
}
