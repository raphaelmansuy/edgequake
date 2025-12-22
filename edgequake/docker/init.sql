-- EdgeQuake Database Initialization
-- This script runs when PostgreSQL container is first created

-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS pg_trgm;
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS age;

-- Set search path for AGE
SET search_path = ag_catalog, "$user", public;

-- Create schema for EdgeQuake
CREATE SCHEMA IF NOT EXISTS edgequake;

-- Documents table
CREATE TABLE IF NOT EXISTS edgequake.documents (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    content TEXT NOT NULL,
    title TEXT,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Text chunks table
CREATE TABLE IF NOT EXISTS edgequake.chunks (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    document_id UUID REFERENCES edgequake.documents(id) ON DELETE CASCADE,
    content TEXT NOT NULL,
    chunk_index INTEGER NOT NULL,
    start_offset INTEGER,
    end_offset INTEGER,
    token_count INTEGER,
    embedding vector(1536),
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Entities table
CREATE TABLE IF NOT EXISTS edgequake.entities (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name TEXT NOT NULL UNIQUE,
    entity_type TEXT NOT NULL,
    description TEXT,
    embedding vector(1536),
    source_ids UUID[],
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Relationships table
CREATE TABLE IF NOT EXISTS edgequake.relationships (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    source_id UUID REFERENCES edgequake.entities(id) ON DELETE CASCADE,
    target_id UUID REFERENCES edgequake.entities(id) ON DELETE CASCADE,
    relation_type TEXT NOT NULL,
    description TEXT,
    weight FLOAT DEFAULT 0.5,
    keywords TEXT[],
    embedding vector(1536),
    source_chunk_ids UUID[],
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(source_id, target_id, relation_type)
);

-- Create indexes for vector similarity search
CREATE INDEX IF NOT EXISTS idx_chunks_embedding ON edgequake.chunks 
    USING ivfflat (embedding vector_cosine_ops) WITH (lists = 100);

CREATE INDEX IF NOT EXISTS idx_entities_embedding ON edgequake.entities 
    USING ivfflat (embedding vector_cosine_ops) WITH (lists = 100);

CREATE INDEX IF NOT EXISTS idx_relationships_embedding ON edgequake.relationships 
    USING ivfflat (embedding vector_cosine_ops) WITH (lists = 100);

-- Create indexes for text search
CREATE INDEX IF NOT EXISTS idx_entities_name_trgm ON edgequake.entities 
    USING gin (name gin_trgm_ops);

CREATE INDEX IF NOT EXISTS idx_entities_type ON edgequake.entities (entity_type);

-- Create indexes for graph traversal
CREATE INDEX IF NOT EXISTS idx_relationships_source ON edgequake.relationships (source_id);
CREATE INDEX IF NOT EXISTS idx_relationships_target ON edgequake.relationships (target_id);

-- Grant permissions
GRANT ALL PRIVILEGES ON SCHEMA edgequake TO edgequake;
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA edgequake TO edgequake;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA edgequake TO edgequake;

-- ============================================================================
-- Phase 1 Enhancements (v1.1.0)
-- ============================================================================

-- 1. Tasks table for background processing
CREATE TABLE IF NOT EXISTS edgequake.tasks (
    track_id VARCHAR(50) PRIMARY KEY,
    task_type VARCHAR(20) NOT NULL,
    status VARCHAR(20) NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
    updated_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    error_message TEXT,
    retry_count INTEGER DEFAULT 0 NOT NULL,
    max_retries INTEGER DEFAULT 3 NOT NULL,
    task_data JSONB NOT NULL,
    metadata JSONB,
    progress JSONB,
    result JSONB,
    CONSTRAINT valid_status CHECK (status IN ('pending', 'processing', 'indexed', 'failed', 'cancelled')),
    CONSTRAINT valid_task_type CHECK (task_type IN ('upload', 'insert', 'scan', 'reindex'))
);

CREATE INDEX IF NOT EXISTS idx_tasks_status ON edgequake.tasks(status, created_at);
CREATE INDEX IF NOT EXISTS idx_tasks_type ON edgequake.tasks(task_type);
CREATE INDEX IF NOT EXISTS idx_tasks_created ON edgequake.tasks(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_tasks_updated ON edgequake.tasks(updated_at DESC);

-- 2. Add document status tracking
ALTER TABLE edgequake.documents
    ADD COLUMN IF NOT EXISTS status VARCHAR(20) DEFAULT 'indexed' NOT NULL,
    ADD COLUMN IF NOT EXISTS track_id VARCHAR(50),
    ADD COLUMN IF NOT EXISTS file_path TEXT,
    ADD COLUMN IF NOT EXISTS file_size_bytes BIGINT,
    ADD COLUMN IF NOT EXISTS content_type VARCHAR(100),
    ADD COLUMN IF NOT EXISTS content_hash VARCHAR(64),
    ADD COLUMN IF NOT EXISTS chunk_count INTEGER DEFAULT 0,
    ADD COLUMN IF NOT EXISTS entity_count INTEGER DEFAULT 0,
    ADD COLUMN IF NOT EXISTS relationship_count INTEGER DEFAULT 0,
    ADD COLUMN IF NOT EXISTS processing_time_ms INTEGER,
    ADD COLUMN IF NOT EXISTS error_message TEXT;

ALTER TABLE edgequake.documents
    DROP CONSTRAINT IF EXISTS valid_document_status,
    ADD CONSTRAINT valid_document_status CHECK (
        status IN ('pending', 'processing', 'indexed', 'failed')
    );

CREATE INDEX IF NOT EXISTS idx_documents_status ON edgequake.documents(status);
CREATE INDEX IF NOT EXISTS idx_documents_track_id ON edgequake.documents(track_id);
CREATE INDEX IF NOT EXISTS idx_documents_content_hash ON edgequake.documents(content_hash);

CREATE UNIQUE INDEX IF NOT EXISTS idx_documents_content_hash_unique 
    ON edgequake.documents(content_hash) 
    WHERE content_hash IS NOT NULL AND status = 'indexed';

-- 3. Conversation history table
CREATE TABLE IF NOT EXISTS edgequake.conversation_history (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    conversation_id UUID NOT NULL,
    message_index INTEGER NOT NULL,
    role VARCHAR(20) NOT NULL,
    content TEXT NOT NULL,
    metadata JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
    CONSTRAINT valid_role CHECK (role IN ('user', 'assistant', 'system')),
    CONSTRAINT unique_conversation_message UNIQUE (conversation_id, message_index)
);

CREATE INDEX IF NOT EXISTS idx_conversation_history_conversation_id 
    ON edgequake.conversation_history(conversation_id, message_index);

-- Grant permissions on new tables
GRANT ALL PRIVILEGES ON edgequake.tasks TO edgequake;
GRANT ALL PRIVILEGES ON edgequake.conversation_history TO edgequake;

-- Success message
DO $$ BEGIN
    RAISE NOTICE 'EdgeQuake database initialized successfully with Phase 1 enhancements!';
END $$;
