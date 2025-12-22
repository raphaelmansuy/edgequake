-- Migration: 005_add_is_manual_flags
-- Description: Add is_manual flag to entities and relationships for tracking manual edits
-- Phase: 1.2.0
-- Date: 2025-12-22

-- Add is_manual flag to entities table
ALTER TABLE edgequake.entities
    ADD COLUMN IF NOT EXISTS is_manual BOOLEAN DEFAULT FALSE NOT NULL,
    ADD COLUMN IF NOT EXISTS manual_created_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS manual_created_by VARCHAR(255),
    ADD COLUMN IF NOT EXISTS last_manual_edit_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS last_manual_edit_by VARCHAR(255);

-- Add is_manual flag to relationships table
ALTER TABLE edgequake.relationships
    ADD COLUMN IF NOT EXISTS is_manual BOOLEAN DEFAULT FALSE NOT NULL,
    ADD COLUMN IF NOT EXISTS manual_created_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS manual_created_by VARCHAR(255),
    ADD COLUMN IF NOT EXISTS last_manual_edit_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS last_manual_edit_by VARCHAR(255);

-- Create indexes for manual tracking
CREATE INDEX IF NOT EXISTS idx_entities_is_manual ON edgequake.entities(is_manual);
CREATE INDEX IF NOT EXISTS idx_relationships_is_manual ON edgequake.relationships(is_manual);
CREATE INDEX IF NOT EXISTS idx_entities_manual_created_by ON edgequake.entities(manual_created_by) WHERE is_manual = TRUE;
CREATE INDEX IF NOT EXISTS idx_relationships_manual_created_by ON edgequake.relationships(manual_created_by) WHERE is_manual = TRUE;

-- Success message
DO $$ BEGIN
    RAISE NOTICE 'Migration 005_add_is_manual_flags completed successfully!';
END $$;
