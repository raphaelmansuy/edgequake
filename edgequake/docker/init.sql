-- EdgeQuake Database Initialization
-- This script runs when PostgreSQL container is first created

-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS pg_trgm;
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

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

-- Success message
DO $$ BEGIN
    RAISE NOTICE 'EdgeQuake database initialized successfully!';
END $$;
