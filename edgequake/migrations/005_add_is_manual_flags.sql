-- Migration: 005_add_is_manual_flags
-- Description: Add is_manual flag to entities and relationships for tracking manual edits
-- Phase: 1.2.0
-- Date: 2025-12-22
-- Note: These columns may already exist from 000a_create_core_tables, so we use IF NOT EXISTS

-- Add is_manual flag to entities table (if table exists)
DO $$
BEGIN
    -- Only run if the table exists
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_schema = 'edgequake' AND table_name = 'entities') THEN
        -- Add columns if they don't exist
        IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'edgequake' AND table_name = 'entities' AND column_name = 'is_manual') THEN
            ALTER TABLE edgequake.entities ADD COLUMN is_manual BOOLEAN DEFAULT FALSE NOT NULL;
        END IF;
        IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'edgequake' AND table_name = 'entities' AND column_name = 'manual_created_at') THEN
            ALTER TABLE edgequake.entities ADD COLUMN manual_created_at TIMESTAMPTZ;
        END IF;
        IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'edgequake' AND table_name = 'entities' AND column_name = 'manual_created_by') THEN
            ALTER TABLE edgequake.entities ADD COLUMN manual_created_by VARCHAR(255);
        END IF;
        IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'edgequake' AND table_name = 'entities' AND column_name = 'last_manual_edit_at') THEN
            ALTER TABLE edgequake.entities ADD COLUMN last_manual_edit_at TIMESTAMPTZ;
        END IF;
        IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'edgequake' AND table_name = 'entities' AND column_name = 'last_manual_edit_by') THEN
            ALTER TABLE edgequake.entities ADD COLUMN last_manual_edit_by VARCHAR(255);
        END IF;
    END IF;
END $$;

-- Add is_manual flag to relationships table (if table exists)
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_schema = 'edgequake' AND table_name = 'relationships') THEN
        IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'edgequake' AND table_name = 'relationships' AND column_name = 'is_manual') THEN
            ALTER TABLE edgequake.relationships ADD COLUMN is_manual BOOLEAN DEFAULT FALSE NOT NULL;
        END IF;
        IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'edgequake' AND table_name = 'relationships' AND column_name = 'manual_created_at') THEN
            ALTER TABLE edgequake.relationships ADD COLUMN manual_created_at TIMESTAMPTZ;
        END IF;
        IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'edgequake' AND table_name = 'relationships' AND column_name = 'manual_created_by') THEN
            ALTER TABLE edgequake.relationships ADD COLUMN manual_created_by VARCHAR(255);
        END IF;
        IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'edgequake' AND table_name = 'relationships' AND column_name = 'last_manual_edit_at') THEN
            ALTER TABLE edgequake.relationships ADD COLUMN last_manual_edit_at TIMESTAMPTZ;
        END IF;
        IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'edgequake' AND table_name = 'relationships' AND column_name = 'last_manual_edit_by') THEN
            ALTER TABLE edgequake.relationships ADD COLUMN last_manual_edit_by VARCHAR(255);
        END IF;
    END IF;
END $$;

-- Create indexes for manual tracking (safe to run multiple times with IF NOT EXISTS)
CREATE INDEX IF NOT EXISTS idx_entities_is_manual ON edgequake.entities(is_manual);
CREATE INDEX IF NOT EXISTS idx_relationships_is_manual ON edgequake.relationships(is_manual);
CREATE INDEX IF NOT EXISTS idx_entities_manual_created_by ON edgequake.entities(manual_created_by) WHERE is_manual = TRUE;
CREATE INDEX IF NOT EXISTS idx_relationships_manual_created_by ON edgequake.relationships(manual_created_by) WHERE is_manual = TRUE;

-- Success message
DO $$ BEGIN
    RAISE NOTICE 'Migration 005_add_is_manual_flags completed successfully!';
END $$;
