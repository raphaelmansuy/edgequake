# Phase 6: Server-Side Implementation Guide

**Document**: `06_server_implementation.md`  
**Created**: 2024-12-27  
**Status**: Complete

---

## 1. Executive Summary

This document provides detailed implementation guidance for adding **Conversation Persistence** to the EdgeQuake Rust backend. It leverages the existing architecture patterns:

- **Axum** HTTP framework with handler/router pattern
- **PostgreSQL** with Row-Level Security (RLS) for multi-tenant isolation
- **Trait-based storage abstraction** (`KVStorage`, `GraphStorage`)
- **TenantContext middleware** for automatic tenant scoping

### Scope

| Feature            | Description                            |
| ------------------ | -------------------------------------- |
| Conversations API  | CRUD for conversation sessions         |
| Messages API       | CRUD for messages within conversations |
| Folders API        | Hierarchical organization              |
| Sharing API        | Public share links                     |
| Migration endpoint | Import from localStorage               |

---

## 2. Existing Architecture Analysis

### 2.1 Crate Dependency Graph

```
┌─────────────────────────────────────────────────────────────────────────┐
│                      EdgeQuake Crate Dependencies                        │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  edgequake-api (HTTP handlers)                                           │
│  ├── edgequake-core (orchestration, types)                               │
│  │   ├── edgequake-llm (providers)                                       │
│  │   ├── edgequake-query (query engine)                                  │
│  │   └── edgequake-storage (persistence)                                 │
│  ├── edgequake-pipeline (document processing)                            │
│  ├── edgequake-auth (JWT, RBAC)                                          │
│  └── edgequake-tasks (background jobs)                                   │
│                                                                          │
│  NEW: Add conversation service to edgequake-core                         │
│       Add handlers to edgequake-api                                      │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### 2.2 Key Patterns to Follow

#### Pattern 1: Handler Structure ([handlers/workspaces.rs](../edgequake/crates/edgequake-api/src/handlers/workspaces.rs))

```rust
// Standard handler structure
#[utoipa::path(
    post,
    path = "/api/v1/resource",
    request_body = CreateRequest,
    responses(
        (status = 201, description = "Created", body = Response),
        (status = 400, description = "Invalid request"),
    ),
    tags = ["resource"]
)]
pub async fn create_resource(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,  // Auto-extracted from headers
    Json(request): Json<CreateRequest>,
) -> Result<(StatusCode, Json<Response>), ApiError> {
    // Implementation
}
```

#### Pattern 2: TenantContext Middleware ([middleware.rs](../edgequake/crates/edgequake-api/src/middleware.rs))

```rust
// Already extracts X-Tenant-ID and X-Workspace-ID headers
#[derive(Debug, Clone, Default)]
pub struct TenantContext {
    pub tenant_id: Option<String>,
    pub workspace_id: Option<String>,
}
```

#### Pattern 3: RLS Integration ([postgres/rls.rs](../edgequake/crates/edgequake-storage/src/adapters/postgres/rls.rs))

```rust
// Set tenant context before queries
let _ctx = RlsContext::new(&pool, tenant_id, workspace_id).await?;
// All subsequent queries are filtered by RLS policies
```

#### Pattern 4: Multi-tenant Types ([types/multitenancy.rs](../edgequake/crates/edgequake-core/src/types/multitenancy.rs))

```rust
pub struct Tenant { tenant_id: Uuid, name: String, slug: String, ... }
pub struct Workspace { workspace_id: Uuid, tenant_id: Uuid, ... }
pub struct Membership { user_id: Uuid, tenant_id: Uuid, role: MembershipRole, ... }
```

---

## 3. Database Schema

### 3.1 Migration File

Create: `edgequake/migrations/009_add_conversations_tables.sql`

```sql
-- Migration: 009_add_conversations_tables.sql
-- Description: Add conversations and messages tables for query history
-- Phase: Query Page Improvement
-- Date: 2025-01-XX

-- ============================================================================
-- FOLDERS TABLE (for organizing conversations)
-- ============================================================================
CREATE TABLE IF NOT EXISTS folders (
    folder_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(tenant_id) ON DELETE CASCADE,
    workspace_id UUID REFERENCES workspaces(workspace_id) ON DELETE SET NULL,
    user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    parent_id UUID REFERENCES folders(folder_id) ON DELETE CASCADE,
    position INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT unique_folder_name_in_parent UNIQUE(tenant_id, user_id, parent_id, name)
);

CREATE INDEX IF NOT EXISTS idx_folders_tenant_user ON folders(tenant_id, user_id);
CREATE INDEX IF NOT EXISTS idx_folders_parent ON folders(parent_id);

-- ============================================================================
-- CONVERSATIONS TABLE
-- ============================================================================
CREATE TABLE IF NOT EXISTS conversations (
    conversation_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(tenant_id) ON DELETE CASCADE,
    workspace_id UUID REFERENCES workspaces(workspace_id) ON DELETE SET NULL,
    user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    title VARCHAR(500) NOT NULL DEFAULT 'New Conversation',
    mode VARCHAR(50) NOT NULL DEFAULT 'hybrid',
    is_pinned BOOLEAN NOT NULL DEFAULT FALSE,
    is_archived BOOLEAN NOT NULL DEFAULT FALSE,
    folder_id UUID REFERENCES folders(folder_id) ON DELETE SET NULL,
    share_id VARCHAR(64) UNIQUE,
    meta JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT valid_mode CHECK (mode IN ('local', 'global', 'hybrid', 'naive', 'mix'))
);

-- Indexes for common access patterns
CREATE INDEX IF NOT EXISTS idx_conversations_tenant_user
    ON conversations(tenant_id, user_id, updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_conversations_workspace
    ON conversations(workspace_id, updated_at DESC)
    WHERE workspace_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_conversations_folder
    ON conversations(folder_id)
    WHERE folder_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_conversations_archived
    ON conversations(tenant_id, user_id, is_archived, updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_conversations_pinned
    ON conversations(tenant_id, user_id, is_pinned)
    WHERE is_pinned = TRUE;
CREATE INDEX IF NOT EXISTS idx_conversations_share
    ON conversations(share_id)
    WHERE share_id IS NOT NULL;

-- Full-text search on title
CREATE INDEX IF NOT EXISTS idx_conversations_title_fts
    ON conversations USING GIN (to_tsvector('english', title));

-- ============================================================================
-- MESSAGES TABLE
-- ============================================================================
CREATE TABLE IF NOT EXISTS messages (
    message_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    conversation_id UUID NOT NULL REFERENCES conversations(conversation_id) ON DELETE CASCADE,
    parent_id UUID REFERENCES messages(message_id) ON DELETE SET NULL,
    role VARCHAR(20) NOT NULL,
    content TEXT NOT NULL,
    mode VARCHAR(50),
    tokens_used INTEGER,
    duration_ms INTEGER,
    thinking_time_ms INTEGER,
    context JSONB,
    is_error BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT valid_role CHECK (role IN ('user', 'assistant', 'system'))
);

CREATE INDEX IF NOT EXISTS idx_messages_conversation
    ON messages(conversation_id, created_at ASC);
CREATE INDEX IF NOT EXISTS idx_messages_parent
    ON messages(parent_id)
    WHERE parent_id IS NOT NULL;

-- Full-text search on content
CREATE INDEX IF NOT EXISTS idx_messages_content_fts
    ON messages USING GIN (to_tsvector('english', content));

-- ============================================================================
-- TRIGGERS: Auto-update updated_at
-- ============================================================================
CREATE OR REPLACE FUNCTION update_conversations_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trigger_conversations_updated_at ON conversations;
CREATE TRIGGER trigger_conversations_updated_at
    BEFORE UPDATE ON conversations
    FOR EACH ROW
    EXECUTE FUNCTION update_conversations_updated_at();

CREATE OR REPLACE FUNCTION update_messages_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trigger_messages_updated_at ON messages;
CREATE TRIGGER trigger_messages_updated_at
    BEFORE UPDATE ON messages
    FOR EACH ROW
    EXECUTE FUNCTION update_messages_updated_at();

CREATE OR REPLACE FUNCTION update_folders_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trigger_folders_updated_at ON folders;
CREATE TRIGGER trigger_folders_updated_at
    BEFORE UPDATE ON folders
    FOR EACH ROW
    EXECUTE FUNCTION update_folders_updated_at();

-- ============================================================================
-- RLS POLICIES
-- ============================================================================
ALTER TABLE conversations ENABLE ROW LEVEL SECURITY;
ALTER TABLE messages ENABLE ROW LEVEL SECURITY;
ALTER TABLE folders ENABLE ROW LEVEL SECURITY;

-- Conversations: Users see their own + shared
DROP POLICY IF EXISTS conversations_tenant_isolation ON conversations;
CREATE POLICY conversations_tenant_isolation ON conversations
    FOR ALL
    USING (
        tenant_id = current_tenant_id()
        AND (
            user_id = current_user_id()
            OR share_id IS NOT NULL
        )
    )
    WITH CHECK (
        tenant_id = current_tenant_id()
        AND user_id = current_user_id()
    );

-- Messages: Inherit access from conversation
DROP POLICY IF EXISTS messages_access ON messages;
CREATE POLICY messages_access ON messages
    FOR ALL
    USING (
        EXISTS (
            SELECT 1 FROM conversations c
            WHERE c.conversation_id = messages.conversation_id
            AND c.tenant_id = current_tenant_id()
            AND (c.user_id = current_user_id() OR c.share_id IS NOT NULL)
        )
    );

-- Folders: Users see their own
DROP POLICY IF EXISTS folders_access ON folders;
CREATE POLICY folders_access ON folders
    FOR ALL
    USING (
        tenant_id = current_tenant_id()
        AND user_id = current_user_id()
    );

-- ============================================================================
-- HELPER FUNCTION: Get current user ID from session
-- ============================================================================
CREATE OR REPLACE FUNCTION current_user_id()
RETURNS UUID AS $$
BEGIN
    RETURN NULLIF(current_setting('app.current_user_id', true), '')::UUID;
EXCEPTION WHEN OTHERS THEN
    RETURN NULL;
END;
$$ LANGUAGE plpgsql STABLE;

-- Update set_tenant_context to also set user_id
CREATE OR REPLACE FUNCTION set_tenant_context(
    p_tenant_id UUID,
    p_workspace_id UUID DEFAULT NULL,
    p_user_id UUID DEFAULT NULL
)
RETURNS void AS $$
BEGIN
    PERFORM set_config('app.current_tenant_id', COALESCE(p_tenant_id::text, ''), true);
    PERFORM set_config('app.current_workspace_id', COALESCE(p_workspace_id::text, ''), true);
    PERFORM set_config('app.current_user_id', COALESCE(p_user_id::text, ''), true);
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

-- Success message
DO $$ BEGIN
    RAISE NOTICE 'Migration 009_add_conversations_tables completed successfully!';
END $$;
```

---

## 4. Domain Types

### 4.1 Add to `edgequake-core/src/types/mod.rs`

```rust
mod conversation;

pub use conversation::{
    Conversation, ConversationFilter, ConversationMode, ConversationSortField,
    CreateConversationRequest, CreateMessageRequest, Folder, Message, MessageContext,
    MessageRole, PaginatedConversations, PaginatedMessages, UpdateConversationRequest,
    UpdateMessageRequest,
};
```

### 4.2 Create `edgequake-core/src/types/conversation.rs`

```rust
//! Conversation and message types for query history.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Conversation mode for RAG queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ConversationMode {
    Local,
    Global,
    #[default]
    Hybrid,
    Naive,
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
    User,
    Assistant,
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

/// A conversation (chat session).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub conversation_id: Uuid,
    pub tenant_id: Uuid,
    pub workspace_id: Option<Uuid>,
    pub user_id: Uuid,
    pub title: String,
    pub mode: ConversationMode,
    pub is_pinned: bool,
    pub is_archived: bool,
    pub folder_id: Option<Uuid>,
    pub share_id: Option<String>,
    pub meta: HashMap<String, serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,

    // Computed fields (not stored)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_message_preview: Option<String>,
}

impl Conversation {
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

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn with_mode(mut self, mode: ConversationMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn with_workspace(mut self, workspace_id: Uuid) -> Self {
        self.workspace_id = Some(workspace_id);
        self
    }
}

/// A message within a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub message_id: Uuid,
    pub conversation_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub role: MessageRole,
    pub content: String,
    pub mode: Option<ConversationMode>,
    pub tokens_used: Option<i32>,
    pub duration_ms: Option<i32>,
    pub thinking_time_ms: Option<i32>,
    pub context: Option<MessageContext>,
    pub is_error: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Message {
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
}

/// Context attached to an assistant message (sources, entities).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageContext {
    #[serde(default)]
    pub sources: Vec<ContextSource>,
    #[serde(default)]
    pub entities: Vec<String>,
    #[serde(default)]
    pub relationships: Vec<String>,
}

/// A source reference in message context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSource {
    pub id: String,
    pub title: Option<String>,
    pub content: Option<String>,
    pub score: f32,
    pub document_id: Option<String>,
}

/// Folder for organizing conversations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Folder {
    pub folder_id: Uuid,
    pub tenant_id: Uuid,
    pub workspace_id: Option<Uuid>,
    pub user_id: Uuid,
    pub name: String,
    pub parent_id: Option<Uuid>,
    pub position: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct CreateConversationRequest {
    pub title: Option<String>,
    pub mode: Option<ConversationMode>,
    pub folder_id: Option<Uuid>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateConversationRequest {
    pub title: Option<String>,
    pub mode: Option<ConversationMode>,
    pub is_pinned: Option<bool>,
    pub is_archived: Option<bool>,
    pub folder_id: Option<Uuid>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateMessageRequest {
    pub content: String,
    pub role: MessageRole,
    pub parent_id: Option<Uuid>,
    #[serde(default = "default_stream")]
    pub stream: bool,
}

fn default_stream() -> bool { true }

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateMessageRequest {
    pub content: Option<String>,
    pub tokens_used: Option<i32>,
    pub duration_ms: Option<i32>,
    pub thinking_time_ms: Option<i32>,
    pub context: Option<MessageContext>,
    pub is_error: Option<bool>,
}

/// Filter parameters for listing conversations.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ConversationFilter {
    pub mode: Option<Vec<ConversationMode>>,
    pub archived: Option<bool>,
    pub pinned: Option<bool>,
    pub folder_id: Option<Uuid>,
    pub search: Option<String>,
    pub date_from: Option<chrono::DateTime<chrono::Utc>>,
    pub date_to: Option<chrono::DateTime<chrono::Utc>>,
}

/// Sort field for conversations.
#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversationSortField {
    #[default]
    UpdatedAt,
    CreatedAt,
    Title,
}

/// Cursor-based pagination metadata.
#[derive(Debug, Clone, Serialize)]
pub struct PaginationMeta {
    pub next_cursor: Option<String>,
    pub prev_cursor: Option<String>,
    pub total: Option<usize>,
    pub has_more: bool,
}

/// Paginated list of conversations.
#[derive(Debug, Clone, Serialize)]
pub struct PaginatedConversations {
    pub items: Vec<Conversation>,
    pub pagination: PaginationMeta,
}

/// Paginated list of messages.
#[derive(Debug, Clone, Serialize)]
pub struct PaginatedMessages {
    pub items: Vec<Message>,
    pub pagination: PaginationMeta,
}
```

---

## 5. Service Layer

### 5.1 Create `edgequake-core/src/conversation_service.rs`

```rust
//! Conversation service for managing chat sessions.

use async_trait::async_trait;
use uuid::Uuid;

use crate::error::{Error, Result};
use crate::types::{
    Conversation, ConversationFilter, ConversationSortField, CreateConversationRequest,
    CreateMessageRequest, Folder, Message, PaginatedConversations, PaginatedMessages,
    UpdateConversationRequest, UpdateMessageRequest,
};

/// Service trait for conversation management.
#[async_trait]
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

/// Result of import operation.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ImportResult {
    pub imported: usize,
    pub failed: usize,
    pub errors: Vec<ImportError>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ImportError {
    pub id: String,
    pub error: String,
}
```

### 5.2 PostgreSQL Implementation

Create `edgequake-storage/src/adapters/postgres/conversation.rs`:

```rust
//! PostgreSQL implementation of ConversationService.

use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use edgequake_core::error::{Error, Result};
use edgequake_core::types::*;
use edgequake_core::ConversationService;

use super::rls::RlsContext;

pub struct PostgresConversationService {
    pool: PgPool,
}

impl PostgresConversationService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Encode cursor for pagination.
    fn encode_cursor(updated_at: chrono::DateTime<chrono::Utc>, id: Uuid) -> String {
        let payload = serde_json::json!({
            "u": updated_at.timestamp_millis(),
            "i": id.to_string()
        });
        base64::Engine::encode(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD,
            payload.to_string().as_bytes()
        )
    }

    /// Decode cursor for pagination.
    fn decode_cursor(cursor: &str) -> Result<(i64, Uuid)> {
        let bytes = base64::Engine::decode(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD,
            cursor
        ).map_err(|_| Error::InvalidInput("Invalid cursor".into()))?;

        let payload: serde_json::Value = serde_json::from_slice(&bytes)
            .map_err(|_| Error::InvalidInput("Invalid cursor".into()))?;

        let ts = payload["u"].as_i64()
            .ok_or_else(|| Error::InvalidInput("Invalid cursor".into()))?;
        let id = payload["i"].as_str()
            .and_then(|s| Uuid::parse_str(s).ok())
            .ok_or_else(|| Error::InvalidInput("Invalid cursor".into()))?;

        Ok((ts, id))
    }
}

#[async_trait]
impl ConversationService for PostgresConversationService {
    async fn create_conversation(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        workspace_id: Option<Uuid>,
        request: CreateConversationRequest,
    ) -> Result<Conversation> {
        let id = Uuid::new_v4();
        let title = request.title.unwrap_or_else(|| "New Conversation".to_string());
        let mode = request.mode.unwrap_or_default();

        let row = sqlx::query_as!(
            ConversationRow,
            r#"
            INSERT INTO conversations (
                conversation_id, tenant_id, workspace_id, user_id,
                title, mode, folder_id
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING
                conversation_id, tenant_id, workspace_id, user_id,
                title, mode, is_pinned, is_archived, folder_id,
                share_id, meta, created_at, updated_at
            "#,
            id,
            tenant_id,
            workspace_id,
            user_id,
            title,
            mode.to_string(),
            request.folder_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(row.into())
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
        // Build dynamic query with filters
        let mut conditions = vec![
            "tenant_id = $1".to_string(),
            "user_id = $2".to_string(),
        ];

        if let Some(archived) = filter.archived {
            conditions.push(format!("is_archived = {}", archived));
        } else {
            // Default: exclude archived
            conditions.push("is_archived = false".to_string());
        }

        if let Some(pinned) = filter.pinned {
            conditions.push(format!("is_pinned = {}", pinned));
        }

        if let Some(folder_id) = filter.folder_id {
            conditions.push(format!("folder_id = '{}'", folder_id));
        }

        if let Some(ref modes) = filter.mode {
            let mode_list: Vec<String> = modes.iter().map(|m| format!("'{}'", m)).collect();
            conditions.push(format!("mode IN ({})", mode_list.join(", ")));
        }

        if let Some(ref search) = filter.search {
            conditions.push(format!(
                "to_tsvector('english', title) @@ plainto_tsquery('english', '{}')",
                search.replace("'", "''")
            ));
        }

        // Cursor-based pagination
        if let Some(ref cursor) = cursor {
            let (ts, id) = Self::decode_cursor(cursor)?;
            let ts_dt = chrono::DateTime::from_timestamp_millis(ts)
                .ok_or_else(|| Error::InvalidInput("Invalid cursor timestamp".into()))?;

            let op = if sort_desc { "<" } else { ">" };
            conditions.push(format!(
                "(updated_at, conversation_id) {} ('{}', '{}')",
                op, ts_dt, id
            ));
        }

        let where_clause = conditions.join(" AND ");
        let order_dir = if sort_desc { "DESC" } else { "ASC" };
        let sort_col = match sort {
            ConversationSortField::UpdatedAt => "updated_at",
            ConversationSortField::CreatedAt => "created_at",
            ConversationSortField::Title => "title",
        };

        let query = format!(
            r#"
            SELECT
                c.conversation_id, c.tenant_id, c.workspace_id, c.user_id,
                c.title, c.mode, c.is_pinned, c.is_archived, c.folder_id,
                c.share_id, c.meta, c.created_at, c.updated_at,
                (SELECT COUNT(*) FROM messages m WHERE m.conversation_id = c.conversation_id) as message_count,
                (SELECT content FROM messages m WHERE m.conversation_id = c.conversation_id
                 ORDER BY created_at DESC LIMIT 1) as last_message
            FROM conversations c
            WHERE {}
            ORDER BY {} {}, conversation_id {}
            LIMIT {}
            "#,
            where_clause, sort_col, order_dir, order_dir, limit + 1
        );

        let rows: Vec<ConversationRowWithStats> = sqlx::query_as(&query)
            .bind(tenant_id)
            .bind(user_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        let has_more = rows.len() > limit;
        let items: Vec<Conversation> = rows.into_iter()
            .take(limit)
            .map(|r| r.into())
            .collect();

        let next_cursor = if has_more {
            items.last().map(|c| Self::encode_cursor(c.updated_at, c.conversation_id))
        } else {
            None
        };

        Ok(PaginatedConversations {
            items,
            pagination: PaginationMeta {
                next_cursor,
                prev_cursor: None, // Implement if needed
                total: None, // Expensive to compute
                has_more,
            },
        })
    }

    // ... implement remaining methods following the same pattern
}
```

---

## 6. API Handlers

### 6.1 Create `edgequake-api/src/handlers/conversations.rs`

```rust
//! Conversation management handlers.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::middleware::TenantContext;
use crate::state::AppState;
use edgequake_core::types::*;

// ============ Request/Response DTOs ============

#[derive(Debug, Deserialize, ToSchema, utoipa::IntoParams)]
pub struct ListConversationsParams {
    /// Cursor for pagination.
    pub cursor: Option<String>,
    /// Maximum items to return (default 20, max 100).
    #[serde(default = "default_limit")]
    pub limit: usize,
    /// Filter by mode (comma-separated: local,global,hybrid).
    #[serde(rename = "filter[mode]")]
    pub filter_mode: Option<String>,
    /// Filter by archived status.
    #[serde(rename = "filter[archived]")]
    pub filter_archived: Option<bool>,
    /// Filter by pinned status.
    #[serde(rename = "filter[pinned]")]
    pub filter_pinned: Option<bool>,
    /// Filter by folder ID.
    #[serde(rename = "filter[folder_id]")]
    pub filter_folder_id: Option<Uuid>,
    /// Search in title.
    #[serde(rename = "filter[search]")]
    pub filter_search: Option<String>,
    /// Sort field (updated_at, created_at, title).
    #[serde(default = "default_sort")]
    pub sort: String,
    /// Sort order (asc, desc).
    #[serde(default = "default_order")]
    pub order: String,
}

fn default_limit() -> usize { 20 }
fn default_sort() -> String { "updated_at".to_string() }
fn default_order() -> String { "desc".to_string() }

#[derive(Debug, Serialize, ToSchema)]
pub struct ConversationResponse {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub workspace_id: Option<Uuid>,
    pub title: String,
    pub mode: String,
    pub is_pinned: bool,
    pub is_archived: bool,
    pub folder_id: Option<Uuid>,
    pub share_id: Option<String>,
    pub message_count: Option<usize>,
    pub last_message_preview: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Conversation> for ConversationResponse {
    fn from(c: Conversation) -> Self {
        Self {
            id: c.conversation_id,
            tenant_id: c.tenant_id,
            workspace_id: c.workspace_id,
            title: c.title,
            mode: c.mode.to_string(),
            is_pinned: c.is_pinned,
            is_archived: c.is_archived,
            folder_id: c.folder_id,
            share_id: c.share_id,
            message_count: c.message_count,
            last_message_preview: c.last_message_preview,
            created_at: c.created_at.to_rfc3339(),
            updated_at: c.updated_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PaginatedConversationsResponse {
    pub items: Vec<ConversationResponse>,
    pub pagination: PaginationMetaResponse,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PaginationMetaResponse {
    pub next_cursor: Option<String>,
    pub prev_cursor: Option<String>,
    pub total: Option<usize>,
    pub has_more: bool,
}

// ============ Handlers ============

/// List conversations for the authenticated user.
#[utoipa::path(
    get,
    path = "/api/v1/conversations",
    params(ListConversationsParams),
    responses(
        (status = 200, description = "List of conversations", body = PaginatedConversationsResponse),
    ),
    tags = ["conversations"]
)]
pub async fn list_conversations(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Query(params): Query<ListConversationsParams>,
) -> ApiResult<Json<PaginatedConversationsResponse>> {
    let tenant_id = tenant_ctx.tenant_id_uuid()
        .ok_or_else(|| ApiError::BadRequest("Missing X-Tenant-ID header".into()))?;

    let user_id = tenant_ctx.user_id_uuid()
        .ok_or_else(|| ApiError::Unauthorized)?;

    // Parse filter modes
    let filter_modes = params.filter_mode.map(|s| {
        s.split(',')
            .filter_map(|m| m.parse().ok())
            .collect()
    });

    let filter = ConversationFilter {
        mode: filter_modes,
        archived: params.filter_archived,
        pinned: params.filter_pinned,
        folder_id: params.filter_folder_id,
        search: params.filter_search,
        date_from: None,
        date_to: None,
    };

    let sort = match params.sort.as_str() {
        "created_at" => ConversationSortField::CreatedAt,
        "title" => ConversationSortField::Title,
        _ => ConversationSortField::UpdatedAt,
    };

    let sort_desc = params.order != "asc";
    let limit = params.limit.min(100);

    let result = state.conversation_service
        .list_conversations(
            tenant_id,
            user_id,
            filter,
            sort,
            sort_desc,
            params.cursor,
            limit,
        )
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(PaginatedConversationsResponse {
        items: result.items.into_iter().map(Into::into).collect(),
        pagination: PaginationMetaResponse {
            next_cursor: result.pagination.next_cursor,
            prev_cursor: result.pagination.prev_cursor,
            total: result.pagination.total,
            has_more: result.pagination.has_more,
        },
    }))
}

/// Create a new conversation.
#[utoipa::path(
    post,
    path = "/api/v1/conversations",
    request_body = CreateConversationRequest,
    responses(
        (status = 201, description = "Conversation created", body = ConversationResponse),
    ),
    tags = ["conversations"]
)]
pub async fn create_conversation(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Json(request): Json<CreateConversationRequest>,
) -> ApiResult<(StatusCode, Json<ConversationResponse>)> {
    let tenant_id = tenant_ctx.tenant_id_uuid()
        .ok_or_else(|| ApiError::BadRequest("Missing X-Tenant-ID header".into()))?;

    let user_id = tenant_ctx.user_id_uuid()
        .ok_or_else(|| ApiError::Unauthorized)?;

    let workspace_id = tenant_ctx.workspace_id_uuid();

    let conversation = state.conversation_service
        .create_conversation(tenant_id, user_id, workspace_id, request)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok((StatusCode::CREATED, Json(conversation.into())))
}

/// Get a conversation by ID.
#[utoipa::path(
    get,
    path = "/api/v1/conversations/{id}",
    params(
        ("id" = Uuid, Path, description = "Conversation ID")
    ),
    responses(
        (status = 200, description = "Conversation details", body = ConversationWithMessagesResponse),
        (status = 404, description = "Not found"),
    ),
    tags = ["conversations"]
)]
pub async fn get_conversation(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ConversationWithMessagesResponse>> {
    let conversation = state.conversation_service
        .get_conversation(id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("Conversation not found".into()))?;

    // Verify access (RLS should handle this, but double-check)
    let user_id = tenant_ctx.user_id_uuid();
    if conversation.share_id.is_none() && user_id != Some(conversation.user_id) {
        return Err(ApiError::NotFound("Conversation not found".into()));
    }

    // Fetch messages
    let messages = state.conversation_service
        .list_messages(id, None, 100)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(ConversationWithMessagesResponse {
        conversation: conversation.into(),
        messages: messages.items.into_iter().map(Into::into).collect(),
    }))
}

// ... Additional handlers for update, delete, messages, folders, share, import
```

### 6.2 Register Routes in `routes.rs`

Add to `api_v1_routes()`:

```rust
// Conversations
.route("/conversations", get(handlers::list_conversations))
.route("/conversations", post(handlers::create_conversation))
.route("/conversations/{id}", get(handlers::get_conversation))
.route("/conversations/{id}", patch(handlers::update_conversation))
.route("/conversations/{id}", delete(handlers::delete_conversation))
.route("/conversations/{id}/messages", get(handlers::list_messages))
.route("/conversations/{id}/messages", post(handlers::create_message))
.route("/conversations/{id}/share", post(handlers::share_conversation))
.route("/conversations/{id}/share", delete(handlers::unshare_conversation))
.route("/conversations/import", post(handlers::import_conversations))

// Messages
.route("/messages/{message_id}", patch(handlers::update_message))
.route("/messages/{message_id}", delete(handlers::delete_message))

// Folders
.route("/folders", get(handlers::list_folders))
.route("/folders", post(handlers::create_folder))
.route("/folders/{folder_id}", patch(handlers::update_folder))
.route("/folders/{folder_id}", delete(handlers::delete_folder))
```

---

## 7. Integration with Query Engine

### 7.1 Modify `stream_query` Handler

Update [handlers/query.rs](../edgequake/crates/edgequake-api/src/handlers/query.rs) to persist messages:

```rust
/// Execute a streaming query with conversation persistence.
pub async fn stream_query(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Json(request): Json<StreamQueryRequest>,
) -> ApiResult<Sse<impl futures::Stream<Item = Result<Event, Infallible>>>> {
    // ... existing validation ...

    // Create or get conversation
    let conversation_id = match request.conversation_id {
        Some(id) => id,
        None => {
            // Create new conversation
            let conv = state.conversation_service
                .create_conversation(
                    tenant_ctx.tenant_id_uuid().unwrap(),
                    tenant_ctx.user_id_uuid().unwrap(),
                    tenant_ctx.workspace_id_uuid(),
                    CreateConversationRequest {
                        title: Some(truncate_title(&request.query)),
                        mode: Some(mode.into()),
                        folder_id: None,
                    },
                )
                .await?;
            conv.conversation_id
        }
    };

    // Save user message
    let user_msg = state.conversation_service
        .create_message(
            conversation_id,
            CreateMessageRequest {
                content: request.query.clone(),
                role: MessageRole::User,
                parent_id: None,
                stream: true,
            },
        )
        .await?;

    // Create placeholder for assistant message
    let assistant_msg = state.conversation_service
        .create_message(
            conversation_id,
            CreateMessageRequest {
                content: String::new(),
                role: MessageRole::Assistant,
                parent_id: Some(user_msg.message_id),
                stream: true,
            },
        )
        .await?;

    // Execute streaming query
    let stream = state.query_engine.query_stream(engine_request).await?;

    // Wrap stream to accumulate content and update message on completion
    let conversation_service = Arc::clone(&state.conversation_service);
    let msg_id = assistant_msg.message_id;

    let wrapped_stream = stream
        .scan(String::new(), |acc, chunk| {
            match chunk {
                Ok(text) => {
                    acc.push_str(&text);
                    Some(Ok(text))
                }
                Err(e) => Some(Err(e)),
            }
        })
        .chain(futures::stream::once(async move {
            // Update message with final content
            // This runs after the stream completes
            // Note: Need to capture accumulated content
            Ok(String::new()) // Sentinel
        }));

    let sse_stream = wrapped_stream.map(|res| match res {
        Ok(text) if !text.is_empty() => Ok(Event::default().data(text)),
        Ok(_) => Ok(Event::default().event("done").data("")),
        Err(e) => Ok(Event::default().data(format!("Error: {}", e))),
    });

    Ok(Sse::new(sse_stream))
}

fn truncate_title(query: &str) -> String {
    let cleaned: String = query.chars().take(50).collect();
    if query.len() > 50 {
        format!("{}...", cleaned)
    } else {
        cleaned
    }
}
```

---

## 8. Testing Strategy

### 8.1 Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_conversation() {
        let service = test_service().await;
        let tenant_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let conv = service.create_conversation(
            tenant_id,
            user_id,
            None,
            CreateConversationRequest {
                title: Some("Test".into()),
                mode: Some(ConversationMode::Hybrid),
                folder_id: None,
            },
        ).await.unwrap();

        assert_eq!(conv.title, "Test");
        assert_eq!(conv.mode, ConversationMode::Hybrid);
        assert_eq!(conv.tenant_id, tenant_id);
    }

    #[tokio::test]
    async fn test_list_conversations_pagination() {
        let service = test_service().await;
        let tenant_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        // Create 25 conversations
        for i in 0..25 {
            service.create_conversation(
                tenant_id, user_id, None,
                CreateConversationRequest {
                    title: Some(format!("Conv {}", i)),
                    mode: None,
                    folder_id: None,
                },
            ).await.unwrap();
        }

        // First page
        let page1 = service.list_conversations(
            tenant_id, user_id,
            ConversationFilter::default(),
            ConversationSortField::UpdatedAt,
            true, // desc
            None,
            10,
        ).await.unwrap();

        assert_eq!(page1.items.len(), 10);
        assert!(page1.pagination.has_more);
        assert!(page1.pagination.next_cursor.is_some());

        // Second page
        let page2 = service.list_conversations(
            tenant_id, user_id,
            ConversationFilter::default(),
            ConversationSortField::UpdatedAt,
            true,
            page1.pagination.next_cursor,
            10,
        ).await.unwrap();

        assert_eq!(page2.items.len(), 10);
        assert!(page2.pagination.has_more);

        // No overlap
        let page1_ids: HashSet<_> = page1.items.iter().map(|c| c.conversation_id).collect();
        let page2_ids: HashSet<_> = page2.items.iter().map(|c| c.conversation_id).collect();
        assert!(page1_ids.is_disjoint(&page2_ids));
    }

    #[tokio::test]
    async fn test_rls_isolation() {
        let service = test_service().await;

        let tenant_a = Uuid::new_v4();
        let tenant_b = Uuid::new_v4();
        let user_a = Uuid::new_v4();
        let user_b = Uuid::new_v4();

        // Create conversation for tenant A
        let conv_a = service.create_conversation(
            tenant_a, user_a, None,
            CreateConversationRequest::default(),
        ).await.unwrap();

        // Tenant B should not see tenant A's conversation
        let list_b = service.list_conversations(
            tenant_b, user_b,
            ConversationFilter::default(),
            ConversationSortField::UpdatedAt,
            true,
            None,
            100,
        ).await.unwrap();

        assert!(list_b.items.is_empty());

        // Direct access should also fail
        let get_result = service.get_conversation(conv_a.conversation_id).await;
        // RLS should filter it out
        assert!(get_result.unwrap().is_none());
    }
}
```

### 8.2 Integration Tests

Create `edgequake/crates/edgequake-core/tests/conversation_integration.rs`:

```rust
use edgequake_api::create_router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

#[tokio::test]
async fn test_conversation_api_e2e() {
    let state = AppState::test_state();
    let app = create_router(state);

    // Create conversation
    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/conversations")
                .header("Content-Type", "application/json")
                .header("X-Tenant-ID", "00000000-0000-0000-0000-000000000001")
                .header("X-User-ID", "00000000-0000-0000-0000-000000000002")
                .body(Body::from(r#"{"title": "Test Conversation"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(create_response.status(), StatusCode::CREATED);

    // Parse response to get conversation ID
    let body = hyper::body::to_bytes(create_response.into_body()).await.unwrap();
    let conv: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let conv_id = conv["id"].as_str().unwrap();

    // Add message
    let msg_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/conversations/{}/messages", conv_id))
                .header("Content-Type", "application/json")
                .header("X-Tenant-ID", "00000000-0000-0000-0000-000000000001")
                .header("X-User-ID", "00000000-0000-0000-0000-000000000002")
                .body(Body::from(r#"{"content": "Hello", "role": "user"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(msg_response.status(), StatusCode::CREATED);

    // List conversations
    let list_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/conversations")
                .header("X-Tenant-ID", "00000000-0000-0000-0000-000000000001")
                .header("X-User-ID", "00000000-0000-0000-0000-000000000002")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(list_response.status(), StatusCode::OK);
}
```

---

## 9. Implementation Checklist

### Phase 1: Database & Types (Week 5)

- [ ] Create migration `009_add_conversations_tables.sql`
- [ ] Run migration on dev database
- [ ] Create `conversation.rs` types in edgequake-core
- [ ] Export types from `types/mod.rs`

### Phase 2: Service Layer (Week 5-6)

- [ ] Define `ConversationService` trait
- [ ] Implement `PostgresConversationService`
- [ ] Add cursor encoding/decoding
- [ ] Implement list with filters
- [ ] Implement CRUD operations
- [ ] Add service to `AppState`

### Phase 3: API Handlers (Week 6)

- [ ] Create `handlers/conversations.rs`
- [ ] Implement all handlers
- [ ] Add OpenAPI annotations
- [ ] Register routes in `routes.rs`
- [ ] Add to `handlers/mod.rs`

### Phase 4: Query Integration (Week 7)

- [ ] Modify `stream_query` to persist messages
- [ ] Add conversation_id to query request
- [ ] Accumulate streaming content
- [ ] Update message on stream completion
- [ ] Add context (sources) to message

### Phase 5: Testing (Week 7-8)

- [ ] Unit tests for service
- [ ] Integration tests for API
- [ ] RLS isolation tests
- [ ] Pagination edge case tests
- [ ] Import migration tests

---

## 10. References

- Existing multi-tenancy: [migrations/007_add_multi_tenancy_tables.sql](../edgequake/migrations/007_add_multi_tenancy_tables.sql)
- RLS patterns: [postgres/rls.rs](../edgequake/crates/edgequake-storage/src/adapters/postgres/rls.rs)
- Workspace handlers: [handlers/workspaces.rs](../edgequake/crates/edgequake-api/src/handlers/workspaces.rs)
- Query handlers: [handlers/query.rs](../edgequake/crates/edgequake-api/src/handlers/query.rs)
- Technical spec: [03_technical_spec.md](03_technical_spec.md)

---

_Last updated: 2024-12-27_
