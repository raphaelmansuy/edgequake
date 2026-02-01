-- Migration: 023_workspace_scoped_content_hash
-- Description: Fix document uniqueness to be scoped at workspace level
-- Phase: OODA-81 - Unified Ingestion Pipeline
-- Date: 2026-02-01
--
-- WHY: Document content_hash uniqueness was global, but should be per-workspace.
-- This allows the same document to exist in different workspaces (different tenants/use-cases)
-- while still preventing duplicate uploads within the same workspace.

SET search_path = public;

-- Drop the old global unique index
DROP INDEX IF EXISTS idx_documents_content_hash_unique;

-- Create workspace-scoped unique index
-- WHY: Uniqueness is now enforced at (workspace_id, content_hash) level
-- This respects multi-tenancy boundaries while still preventing duplicates per workspace
CREATE UNIQUE INDEX IF NOT EXISTS idx_documents_workspace_content_hash_unique 
    ON documents(workspace_id, content_hash) 
    WHERE workspace_id IS NOT NULL 
      AND content_hash IS NOT NULL 
      AND status = 'indexed';

-- Also add compound index for faster lookups (non-unique)
CREATE INDEX IF NOT EXISTS idx_documents_workspace_hash_lookup
    ON documents(workspace_id, content_hash)
    WHERE content_hash IS NOT NULL;

-- Success message
DO $$ BEGIN
    RAISE NOTICE 'Migration 023: Workspace-scoped content_hash uniqueness applied successfully!';
END $$;
