-- Migration: 001_add_tasks_table
SET search_path = public;
-- Description: Create schema, core tables, and tasks table
-- Phase: 1.1.0
-- Date: 2025-12-22 (Updated: 2025-01-28)
-- NOTE: Tables created in PUBLIC schema for consistency with RLS policies

-- ============================================================================
-- STEP 1: Create the edgequake schema (for namespacing, not for tables)
-- ============================================================================
CREATE SCHEMA IF NOT EXISTS edgequake;

-- ============================================================================
-- STEP 2: Create core tables (documents, chunks, entities, relationships)
-- NOTE: All tables in PUBLIC schema to align with RLS migrations
-- ============================================================================

-- Enable required PostgreSQL extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "vector";

-- Documents Table (PUBLIC schema)
CREATE TABLE IF NOT EXISTS documents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID,
    workspace_id UUID,
    title TEXT NOT NULL DEFAULT 'Untitled',
    content TEXT NOT NULL,
    content_hash VARCHAR(64),
    metadata JSONB DEFAULT '{}' NOT NULL,
    file_path TEXT,
    file_size_bytes BIGINT,
    content_type VARCHAR(100),
    status VARCHAR(20) NOT NULL DEFAULT 'indexed',
    track_id VARCHAR(50),
    error_message TEXT,
    processing_time_ms INTEGER,
    chunk_count INTEGER DEFAULT 0,
    entity_count INTEGER DEFAULT 0,
    relationship_count INTEGER DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT documents_valid_status CHECK (status IN ('pending', 'processing', 'indexed', 'failed'))
);

CREATE INDEX IF NOT EXISTS idx_documents_tenant_workspace ON documents(tenant_id, workspace_id);
CREATE INDEX IF NOT EXISTS idx_documents_status ON documents(status);
CREATE INDEX IF NOT EXISTS idx_documents_created_at ON documents(created_at DESC);

-- Chunks Table (PUBLIC schema)
CREATE TABLE IF NOT EXISTS chunks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    document_id UUID NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    tenant_id UUID,
    workspace_id UUID,
    content TEXT NOT NULL,
    chunk_index INTEGER NOT NULL,
    start_offset INTEGER,
    end_offset INTEGER,
    token_count INTEGER,
    embedding vector(1536),
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chunks_unique_doc_index UNIQUE (document_id, chunk_index)
);

CREATE INDEX IF NOT EXISTS idx_chunks_document ON chunks(document_id);
CREATE INDEX IF NOT EXISTS idx_chunks_tenant_workspace ON chunks(tenant_id, workspace_id);

-- Entities Table (PUBLIC schema)
CREATE TABLE IF NOT EXISTS entities (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID,
    workspace_id UUID,
    name TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    description TEXT,
    embedding vector(1536),
    source_ids UUID[],
    is_manual BOOLEAN NOT NULL DEFAULT FALSE,
    manual_created_at TIMESTAMPTZ,
    manual_created_by VARCHAR(255),
    last_manual_edit_at TIMESTAMPTZ,
    last_manual_edit_by VARCHAR(255),
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT entities_unique_name UNIQUE NULLS NOT DISTINCT (tenant_id, workspace_id, name)
);

CREATE INDEX IF NOT EXISTS idx_entities_tenant_workspace ON entities(tenant_id, workspace_id);
CREATE INDEX IF NOT EXISTS idx_entities_type ON entities(entity_type);
CREATE INDEX IF NOT EXISTS idx_entities_name ON entities(name);

-- Relationships Table (PUBLIC schema)
CREATE TABLE IF NOT EXISTS relationships (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_id UUID NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
    target_id UUID NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
    tenant_id UUID,
    workspace_id UUID,
    relation_type TEXT NOT NULL,
    description TEXT,
    weight FLOAT DEFAULT 0.5,
    keywords TEXT[],
    embedding vector(1536),
    source_chunk_ids UUID[],
    is_manual BOOLEAN NOT NULL DEFAULT FALSE,
    manual_created_at TIMESTAMPTZ,
    manual_created_by VARCHAR(255),
    last_manual_edit_at TIMESTAMPTZ,
    last_manual_edit_by VARCHAR(255),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT relationships_unique UNIQUE (source_id, target_id, relation_type)
);

CREATE INDEX IF NOT EXISTS idx_relationships_source ON relationships(source_id);
CREATE INDEX IF NOT EXISTS idx_relationships_target ON relationships(target_id);
CREATE INDEX IF NOT EXISTS idx_relationships_tenant_workspace ON relationships(tenant_id, workspace_id);
CREATE INDEX IF NOT EXISTS idx_relationships_type ON relationships(relation_type);

-- ============================================================================
-- STEP 3: Create tasks table (PUBLIC schema)
-- ============================================================================
CREATE TABLE IF NOT EXISTS tasks (
    -- Identity
    track_id VARCHAR(50) PRIMARY KEY,
    task_type VARCHAR(20) NOT NULL,

    -- Status
    status VARCHAR(20) NOT NULL,

    -- Timestamps
    created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
    updated_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,

    -- Error handling
    error_message TEXT,
    retry_count INTEGER DEFAULT 0 NOT NULL,
    max_retries INTEGER DEFAULT 3 NOT NULL,

    -- Payload
    task_data JSONB NOT NULL,

    -- Metadata
    metadata JSONB,

    -- Progress tracking
    progress JSONB,

    -- Result (on success)
    result JSONB,

    -- Constraints
    CONSTRAINT valid_status CHECK (status IN ('pending', 'processing', 'indexed', 'failed', 'cancelled')),
    CONSTRAINT valid_task_type CHECK (task_type IN ('upload', 'insert', 'scan', 'reindex'))
);

-- Create indexes for performance
CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status, created_at);
CREATE INDEX IF NOT EXISTS idx_tasks_type ON tasks(task_type);
CREATE INDEX IF NOT EXISTS idx_tasks_created ON tasks(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_tasks_updated ON tasks(updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_tasks_status_type ON tasks(status, task_type);

-- Grant permissions (safe to fail if user doesn't have permission)
DO $$ 
BEGIN
    EXECUTE 'GRANT ALL PRIVILEGES ON SCHEMA edgequake TO edgequake';
    EXECUTE 'GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA edgequake TO edgequake';
EXCEPTION WHEN OTHERS THEN
    RAISE NOTICE 'Could not grant privileges: %', SQLERRM;
END $$;

-- Success message
DO $$ BEGIN
    RAISE NOTICE 'Migration 001 completed: Schema, core tables, and tasks table created!';
END $$;
