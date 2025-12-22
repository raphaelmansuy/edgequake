-- Migration: 004_add_audit_log_table
-- Description: Add audit log table for tracking manual graph changes
-- Phase: 1.2.0
-- Date: 2025-12-22

-- Create audit_log table
CREATE TABLE IF NOT EXISTS edgequake.audit_log (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    
    -- Action details
    action_type VARCHAR(50) NOT NULL,
    entity_type VARCHAR(50) NOT NULL,
    entity_id VARCHAR(255) NOT NULL,
    
    -- User/source information
    user_id VARCHAR(255),
    source VARCHAR(100) DEFAULT 'api',
    
    -- Change details
    previous_value JSONB,
    new_value JSONB,
    changes JSONB,
    
    -- Metadata
    metadata JSONB,
    reason TEXT,
    
    -- Timestamp
    created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
    
    -- Constraints
    CONSTRAINT valid_action_type CHECK (
        action_type IN (
            'entity_created', 'entity_updated', 'entity_deleted', 'entity_merged',
            'relationship_created', 'relationship_updated', 'relationship_deleted',
            'document_created', 'document_updated', 'document_deleted', 'bulk_operation'
        )
    ),
    CONSTRAINT valid_entity_type CHECK (
        entity_type IN ('entity', 'relationship', 'document', 'batch')
    )
);

-- Create indexes for audit log
CREATE INDEX IF NOT EXISTS idx_audit_log_action_type ON edgequake.audit_log(action_type);
CREATE INDEX IF NOT EXISTS idx_audit_log_entity_type ON edgequake.audit_log(entity_type);
CREATE INDEX IF NOT EXISTS idx_audit_log_entity_id ON edgequake.audit_log(entity_id);
CREATE INDEX IF NOT EXISTS idx_audit_log_user_id ON edgequake.audit_log(user_id);
CREATE INDEX IF NOT EXISTS idx_audit_log_created_at ON edgequake.audit_log(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_audit_log_composite ON edgequake.audit_log(entity_type, entity_id, created_at DESC);

-- Grant permissions
GRANT ALL PRIVILEGES ON edgequake.audit_log TO edgequake;

-- Success message
DO $$ BEGIN
    RAISE NOTICE 'Migration 004_add_audit_log_table completed successfully!';
END $$;
