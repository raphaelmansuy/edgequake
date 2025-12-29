-- Migration: Add tenant isolation performance indexes
-- Version: V003
-- Description: Optimize tenant-filtered queries with strategic indexes
-- Created: 2024-12-29
-- Updated: 2025-01-27 - Fixed to match actual schema

-- ============================================================================
-- VECTOR STORAGE INDEXES (chunks table)
-- ============================================================================

-- Note: Some indexes may already exist from 001_add_tasks_table.sql
-- Using IF NOT EXISTS to make this idempotent

-- BRIN index for time-series queries (efficient for large tables)
-- Useful for "recent documents" queries within a tenant
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_chunks_created_at_brin') THEN
        CREATE INDEX idx_chunks_created_at_brin 
        ON edgequake.chunks USING BRIN(created_at)
        WITH (pages_per_range = 128);
    END IF;
EXCEPTION WHEN OTHERS THEN
    RAISE NOTICE 'Could not create idx_chunks_created_at_brin: %', SQLERRM;
END $$;

-- ============================================================================
-- ENTITY AND RELATIONSHIP INDEXES  
-- ============================================================================

-- Index for entity type filtering within tenant
CREATE INDEX IF NOT EXISTS idx_entities_tenant_type
ON edgequake.entities(tenant_id, entity_type)
WHERE tenant_id IS NOT NULL;

-- Index for relationship type filtering within tenant
CREATE INDEX IF NOT EXISTS idx_relationships_tenant_type
ON edgequake.relationships(tenant_id, relation_type)
WHERE tenant_id IS NOT NULL;

-- Index for entity search by name within tenant/workspace
CREATE INDEX IF NOT EXISTS idx_entities_tenant_name_search
ON edgequake.entities(tenant_id, workspace_id, name)
WHERE tenant_id IS NOT NULL;

-- ============================================================================
-- DOCUMENT METADATA INDEXES
-- ============================================================================

-- Index for document status filtering within tenant
-- Useful for "show all processing documents for tenant X"
CREATE INDEX IF NOT EXISTS idx_documents_tenant_status
ON edgequake.documents(tenant_id, status)
INCLUDE (title, created_at, updated_at)
WHERE tenant_id IS NOT NULL;

-- Index for full-text search within tenant scope
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_documents_tenant_title_search') THEN
        CREATE INDEX idx_documents_tenant_title_search
        ON edgequake.documents USING GIN (to_tsvector('english', title))
        WHERE tenant_id IS NOT NULL;
    END IF;
EXCEPTION WHEN OTHERS THEN
    RAISE NOTICE 'Could not create idx_documents_tenant_title_search: %', SQLERRM;
END $$;

-- ============================================================================
-- AUDIT LOG INDEXES (if audit_logs table exists)
-- ============================================================================

-- Composite index for tenant + timestamp queries
-- Enables fast "show audit log for tenant X in last 24 hours"
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_schema = 'edgequake' AND table_name = 'audit_logs') THEN
        EXECUTE 'CREATE INDEX IF NOT EXISTS idx_audit_logs_tenant_timestamp
        ON edgequake.audit_logs(tenant_id, timestamp DESC)
        WHERE tenant_id IS NOT NULL';
    END IF;
EXCEPTION WHEN OTHERS THEN
    RAISE NOTICE 'Could not create audit_logs index: %', SQLERRM;
END $$;

-- ============================================================================
-- TASK INDEXES
-- ============================================================================

-- Index for task queries within tenant/workspace
CREATE INDEX IF NOT EXISTS idx_tasks_tenant_workspace_status
ON edgequake.tasks(tenant_id, workspace_id, status)
WHERE tenant_id IS NOT NULL;

-- ============================================================================
-- SUCCESS MESSAGE
-- ============================================================================
DO $$ BEGIN
    RAISE NOTICE 'Migration 010_tenant_performance_indexes completed successfully!';
END $$;
-- DROP INDEX CONCURRENTLY IF EXISTS idx_documents_tenant_status;
-- DROP INDEX CONCURRENTLY IF EXISTS idx_documents_tenant_title_search;
-- DROP INDEX CONCURRENTLY IF EXISTS idx_audit_logs_tenant_timestamp;
-- DROP INDEX CONCURRENTLY IF EXISTS idx_audit_logs_event_type_timestamp;
-- DROP INDEX CONCURRENTLY IF EXISTS idx_audit_logs_result_timestamp;
