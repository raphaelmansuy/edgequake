-- Migration: Add tenant isolation performance indexes
SET search_path = public;
-- Version: V003
-- Description: Optimize tenant-filtered queries with strategic indexes
-- Created: 2024-12-29
-- Updated: 2025-01-28 - Updated to use public schema
-- Updated: 2025-12-30 - Made idempotent with column checks for compatibility

-- ============================================================================
-- VECTOR STORAGE INDEXES (chunks table - PUBLIC schema)
-- ============================================================================

-- Note: Some indexes may already exist from 001_add_tasks_table.sql
-- Using IF NOT EXISTS to make this idempotent

-- BRIN index for time-series queries (efficient for large tables)
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_chunks_created_at_brin') THEN
        IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'public' AND table_name = 'chunks' AND column_name = 'created_at') THEN
            CREATE INDEX idx_chunks_created_at_brin ON chunks USING BRIN(created_at) WITH (pages_per_range = 128);
        END IF;
    END IF;
EXCEPTION WHEN OTHERS THEN
    RAISE NOTICE 'Could not create idx_chunks_created_at_brin: %', SQLERRM;
END $$;

-- ============================================================================
-- ENTITY AND RELATIONSHIP INDEXES (PUBLIC schema) - Only if tenant_id exists
-- ============================================================================

-- Index for entity type filtering within tenant
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'public' AND table_name = 'entities' AND column_name = 'tenant_id') THEN
        IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_entities_tenant_type') THEN
            CREATE INDEX idx_entities_tenant_type ON entities(tenant_id, entity_type) WHERE tenant_id IS NOT NULL;
        END IF;
    END IF;
EXCEPTION WHEN OTHERS THEN
    RAISE NOTICE 'Could not create idx_entities_tenant_type: %', SQLERRM;
END $$;

-- Index for relationship type filtering within tenant
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'public' AND table_name = 'relationships' AND column_name = 'tenant_id') THEN
        IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_relationships_tenant_type') THEN
            CREATE INDEX idx_relationships_tenant_type ON relationships(tenant_id, relation_type) WHERE tenant_id IS NOT NULL;
        END IF;
    END IF;
EXCEPTION WHEN OTHERS THEN
    RAISE NOTICE 'Could not create idx_relationships_tenant_type: %', SQLERRM;
END $$;

-- Index for entity search by name within tenant/workspace
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'public' AND table_name = 'entities' AND column_name = 'tenant_id') THEN
        IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_entities_tenant_name_search') THEN
            CREATE INDEX idx_entities_tenant_name_search ON entities(tenant_id, workspace_id, name) WHERE tenant_id IS NOT NULL;
        END IF;
    END IF;
EXCEPTION WHEN OTHERS THEN
    RAISE NOTICE 'Could not create idx_entities_tenant_name_search: %', SQLERRM;
END $$;

-- ============================================================================
-- DOCUMENT METADATA INDEXES (PUBLIC schema) - Only if tenant_id exists
-- ============================================================================

-- Index for document status filtering within tenant
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'public' AND table_name = 'documents' AND column_name = 'tenant_id') THEN
        IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_documents_tenant_status') THEN
            CREATE INDEX idx_documents_tenant_status ON documents(tenant_id, status) INCLUDE (title, created_at, updated_at) WHERE tenant_id IS NOT NULL;
        END IF;
    END IF;
EXCEPTION WHEN OTHERS THEN
    RAISE NOTICE 'Could not create idx_documents_tenant_status: %', SQLERRM;
END $$;

-- Index for full-text search within tenant scope
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'public' AND table_name = 'documents' AND column_name = 'tenant_id') THEN
        IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_documents_tenant_title_search') THEN
            CREATE INDEX idx_documents_tenant_title_search ON documents USING GIN (to_tsvector('english', title)) WHERE tenant_id IS NOT NULL;
        END IF;
    END IF;
EXCEPTION WHEN OTHERS THEN
    RAISE NOTICE 'Could not create idx_documents_tenant_title_search: %', SQLERRM;
END $$;

-- ============================================================================
-- AUDIT LOG INDEXES (public schema - audit_logs table)
-- ============================================================================

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'public' AND table_name = 'audit_logs' AND column_name = 'tenant_id') THEN
        IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_audit_logs_tenant_timestamp_perf') THEN
            CREATE INDEX idx_audit_logs_tenant_timestamp_perf ON audit_logs(tenant_id, timestamp DESC) WHERE tenant_id IS NOT NULL;
        END IF;
    END IF;
EXCEPTION WHEN OTHERS THEN
    RAISE NOTICE 'Could not create audit_logs index: %', SQLERRM;
END $$;

-- ============================================================================
-- TASK INDEXES (PUBLIC schema) - Only if tenant_id exists
-- ============================================================================

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'public' AND table_name = 'tasks' AND column_name = 'tenant_id') THEN
        IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_tasks_tenant_workspace_status') THEN
            CREATE INDEX idx_tasks_tenant_workspace_status ON tasks(tenant_id, workspace_id, status) WHERE tenant_id IS NOT NULL;
        END IF;
    END IF;
EXCEPTION WHEN OTHERS THEN
    RAISE NOTICE 'Could not create idx_tasks_tenant_workspace_status: %', SQLERRM;
END $$;

-- ============================================================================
-- SUCCESS MESSAGE
-- ============================================================================
DO $$ BEGIN
    RAISE NOTICE 'Migration 010_tenant_performance_indexes completed successfully!';
END $$;
