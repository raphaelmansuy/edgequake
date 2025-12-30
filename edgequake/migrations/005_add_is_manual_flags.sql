-- Migration: 005_add_is_manual_flags
SET search_path = public;
-- Description: Add is_manual flag to entities and relationships for tracking manual edits
-- Phase: 1.2.0
-- Date: 2025-12-22 (Updated: 2025-01-28)
-- Note: Updated to check both public and edgequake schemas

-- Add is_manual flag to entities table (check both schemas)
DO $$
BEGIN
    -- Check public schema first (new location)
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'entities') THEN
        IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'public' AND table_name = 'entities' AND column_name = 'is_manual') THEN
            ALTER TABLE public.entities ADD COLUMN is_manual BOOLEAN DEFAULT FALSE NOT NULL;
        END IF;
        IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'public' AND table_name = 'entities' AND column_name = 'manual_created_at') THEN
            ALTER TABLE public.entities ADD COLUMN manual_created_at TIMESTAMPTZ;
        END IF;
        IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'public' AND table_name = 'entities' AND column_name = 'manual_created_by') THEN
            ALTER TABLE public.entities ADD COLUMN manual_created_by VARCHAR(255);
        END IF;
        IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'public' AND table_name = 'entities' AND column_name = 'last_manual_edit_at') THEN
            ALTER TABLE public.entities ADD COLUMN last_manual_edit_at TIMESTAMPTZ;
        END IF;
        IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'public' AND table_name = 'entities' AND column_name = 'last_manual_edit_by') THEN
            ALTER TABLE public.entities ADD COLUMN last_manual_edit_by VARCHAR(255);
        END IF;
    -- Fallback to edgequake schema (legacy location)
    ELSIF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_schema = 'edgequake' AND table_name = 'entities') THEN
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

-- Add is_manual flag to relationships table (check both schemas)
DO $$
BEGIN
    -- Check public schema first
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'relationships') THEN
        IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'public' AND table_name = 'relationships' AND column_name = 'is_manual') THEN
            ALTER TABLE public.relationships ADD COLUMN is_manual BOOLEAN DEFAULT FALSE NOT NULL;
        END IF;
        IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'public' AND table_name = 'relationships' AND column_name = 'manual_created_at') THEN
            ALTER TABLE public.relationships ADD COLUMN manual_created_at TIMESTAMPTZ;
        END IF;
        IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'public' AND table_name = 'relationships' AND column_name = 'manual_created_by') THEN
            ALTER TABLE public.relationships ADD COLUMN manual_created_by VARCHAR(255);
        END IF;
        IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'public' AND table_name = 'relationships' AND column_name = 'last_manual_edit_at') THEN
            ALTER TABLE public.relationships ADD COLUMN last_manual_edit_at TIMESTAMPTZ;
        END IF;
        IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'public' AND table_name = 'relationships' AND column_name = 'last_manual_edit_by') THEN
            ALTER TABLE public.relationships ADD COLUMN last_manual_edit_by VARCHAR(255);
        END IF;
    -- Fallback to edgequake schema
    ELSIF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_schema = 'edgequake' AND table_name = 'relationships') THEN
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

-- Create indexes for manual tracking (try public schema first, fallback to edgequake)
-- These will silently fail if tables don't exist in that schema
CREATE INDEX IF NOT EXISTS idx_entities_is_manual ON entities(is_manual);
CREATE INDEX IF NOT EXISTS idx_relationships_is_manual ON relationships(is_manual);
CREATE INDEX IF NOT EXISTS idx_entities_manual_created_by ON entities(manual_created_by) WHERE is_manual = TRUE;
CREATE INDEX IF NOT EXISTS idx_relationships_manual_created_by ON relationships(manual_created_by) WHERE is_manual = TRUE;

-- Success message
DO $$ BEGIN
    RAISE NOTICE 'Migration 005_add_is_manual_flags completed successfully!';
END $$;
