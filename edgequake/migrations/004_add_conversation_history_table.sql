-- Migration: 003_add_conversation_history_table
SET search_path = public;
-- Description: Add conversation history table for multi-turn queries
-- Phase: 1.1.0
-- Date: 2025-12-22 (Updated: 2025-01-28)
-- NOTE: DEPRECATED - Use 009_add_conversations_tables.sql instead
-- NOTE: Uses PUBLIC schema for consistency

-- Create conversation_history table (legacy, kept for backward compatibility)
CREATE TABLE IF NOT EXISTS conversation_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    conversation_id UUID NOT NULL,
    message_index INTEGER NOT NULL,
    role VARCHAR(20) NOT NULL,
    content TEXT NOT NULL,
    metadata JSONB,
    tenant_id UUID,
    workspace_id UUID,
    created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,

    -- Constraints
    CONSTRAINT valid_role CHECK (role IN ('user', 'assistant', 'system')),
    CONSTRAINT unique_conversation_message UNIQUE (conversation_id, message_index)
);

-- Create indexes
CREATE INDEX IF NOT EXISTS idx_conversation_history_conversation_id 
    ON conversation_history(conversation_id, message_index);
CREATE INDEX IF NOT EXISTS idx_conversation_history_created 
    ON conversation_history(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_conversation_history_tenant_workspace
    ON conversation_history(tenant_id, workspace_id);

-- Success message
DO $$ BEGIN
    RAISE NOTICE 'Migration 003_add_conversation_history_table completed!';
    RAISE NOTICE 'NOTE: This table is DEPRECATED. Use conversations/messages from 009 instead.';
END $$;
