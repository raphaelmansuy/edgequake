-- Migration: 004_add_audit_log_table
SET search_path = public;
-- Description: Add audit log table for tracking manual graph changes
-- Phase: 1.2.0
-- Date: 2025-12-22 (Updated: 2025-01-28)
-- NOTE: This is for graph editing audit. For security audit, see 011_audit_logs_table.sql
-- NOTE: Uses edgequake schema for this specific audit (graph-related)

-- Ensure edgequake schema exists
CREATE SCHEMA IF NOT EXISTS edgequake;

-- Create audit_log table in edgequake schema (graph-specific audit)
CREATE TABLE IF NOT EXISTS edgequake.audit_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    
    -- Action details
    action_type VARCHAR(50) NOT NULL,
    entity_type VARCHAR(50) NOT NULL,
    entity_id VARCHAR(255) NOT NULL,
    
    -- User/source information
    user_id VARCHAR(255),
    source VARCHAR(100) DEFAULT 'api',
    
    -- Multi-tenancy
    tenant_id UUID,
    workspace_id UUID,
    
    -- Change details
    previous_value JSONB,
    new_value JSONB,
    changes JSONB,
    
    -- Metadata
    metadata JSONB,
    reason TEXT,
    
    -- Timestamp
    created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
    
    -- Constraints (use DO block for idempotency)
    CONSTRAINT audit_log_valid_action_type CHECK (
        action_type IN (
            'entity_created', 'entity_updated', 'entity_deleted', 'entity_merged',
            'relationship_created', 'relationship_updated', 'relationship_deleted',
            'document_created', 'document_updated', 'document_deleted', 'bulk_operation'
        )
    ),
    CONSTRAINT audit_log_valid_entity_type CHECK (
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
CREATE INDEX IF NOT EXISTS idx_audit_log_tenant ON edgequake.audit_log(tenant_id, workspace_id);

-- Success message
DO $$ BEGIN
    RAISE NOTICE 'Migration 004_add_audit_log_table completed successfully!';
END $$;
