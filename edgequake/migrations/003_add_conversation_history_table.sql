-- Migration: 003_add_conversation_history_table
-- Description: Add conversation history table for multi-turn queries
-- Phase: 1.1.0
-- Date: 2025-12-22

-- Create conversation_history table
CREATE TABLE IF NOT EXISTS edgequake.conversation_history (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    conversation_id UUID NOT NULL,
    message_index INTEGER NOT NULL,
    role VARCHAR(20) NOT NULL,
    content TEXT NOT NULL,
    metadata JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,

    -- Constraints
    CONSTRAINT valid_role CHECK (role IN ('user', 'assistant', 'system')),
    CONSTRAINT unique_conversation_message UNIQUE (conversation_id, message_index)
);

-- Create indexes
CREATE INDEX IF NOT EXISTS idx_conversation_history_conversation_id 
    ON edgequake.conversation_history(conversation_id, message_index);
CREATE INDEX IF NOT EXISTS idx_conversation_history_created 
    ON edgequake.conversation_history(created_at DESC);

-- Grant permissions
GRANT ALL PRIVILEGES ON edgequake.conversation_history TO edgequake;

-- Success message
DO $$ BEGIN
    RAISE NOTICE 'Migration 003_add_conversation_history_table completed successfully!';
END $$;
