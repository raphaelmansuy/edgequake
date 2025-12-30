-- Migration: 009_add_conversations_tables.sql
SET search_path = public;
-- Description: Add conversations, messages, and folders tables for query history persistence
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
-- TRIGGER: Auto-update conversation.updated_at when message is added
-- ============================================================================
CREATE OR REPLACE FUNCTION update_conversation_on_message()
RETURNS TRIGGER AS $$
BEGIN
    UPDATE conversations
    SET updated_at = NOW()
    WHERE conversation_id = NEW.conversation_id;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trigger_update_conversation_on_message ON messages;
CREATE TRIGGER trigger_update_conversation_on_message
    AFTER INSERT ON messages
    FOR EACH ROW
    EXECUTE FUNCTION update_conversation_on_message();

-- ============================================================================
-- HELPER FUNCTIONS: Session context functions
-- NOTE: current_user_id() is already created in migration 008
-- This is kept for idempotency (CREATE OR REPLACE is safe)
-- ============================================================================
CREATE OR REPLACE FUNCTION current_user_id()
RETURNS UUID AS $$
BEGIN
    RETURN NULLIF(current_setting('app.current_user_id', true), '')::UUID;
EXCEPTION WHEN OTHERS THEN
    RETURN NULL;
END;
$$ LANGUAGE plpgsql STABLE;

-- ============================================================================
-- RLS POLICIES (helper functions already exist from 008)
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

-- NOTE: set_tenant_context is already created with 3 parameters in migration 008
-- No need to drop and recreate here

-- ============================================================================
-- Success message
-- ============================================================================
DO $$ BEGIN
    RAISE NOTICE 'Migration 009_add_conversations_tables completed successfully!';
END $$;
