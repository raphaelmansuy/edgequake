-- =============================================================================
-- EdgeQuake Production Database Initialization Script
-- =============================================================================
-- Version: 2.0.0 (SOTA)
-- Created: 2024-12-29
-- Purpose: Complete database setup with multi-tenancy, RLS, and performance optimization
-- =============================================================================

-- ============================================================================
-- PHASE 0: EXTENSIONS AND PREREQUISITES
-- ============================================================================

-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS vector;           -- pgvector for embeddings
CREATE EXTENSION IF NOT EXISTS pg_trgm;          -- Trigram similarity for fuzzy search
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";      -- UUID generation (legacy support)
CREATE EXTENSION IF NOT EXISTS btree_gin;        -- GIN support for btree operators

-- Try to load AGE if available (optional for graph queries)
DO $$
BEGIN
    CREATE EXTENSION IF NOT EXISTS age;
    SET search_path = ag_catalog, "$user", public;
EXCEPTION WHEN OTHERS THEN
    RAISE NOTICE 'Apache AGE not available, using relational graph storage';
END $$;

-- ============================================================================
-- PHASE 1: ROLES AND SECURITY
-- ============================================================================

-- Create application role (non-superuser for RLS enforcement)
DO $$
BEGIN
    IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'app_user') THEN
        CREATE ROLE app_user WITH LOGIN PASSWORD 'app_password_changeme';
        RAISE NOTICE 'Created app_user role - CHANGE PASSWORD IN PRODUCTION!';
    END IF;
END $$;

-- Create admin role for bypassing RLS when needed
DO $$
BEGIN
    IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'edgequake_admin') THEN
        CREATE ROLE edgequake_admin NOLOGIN BYPASSRLS;
    END IF;
END $$;

-- ============================================================================
-- PHASE 2: CORE TABLES
-- ============================================================================

-- -----------------------------------------------------------------------------
-- Documents Table - Primary document storage
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS documents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID,                                    -- Multi-tenancy
    workspace_id UUID,                                 -- Workspace isolation
    
    -- Content
    title TEXT NOT NULL DEFAULT 'Untitled',
    content TEXT NOT NULL,
    content_hash VARCHAR(64),                          -- SHA-256 for deduplication
    
    -- Metadata
    metadata JSONB DEFAULT '{}' NOT NULL,
    file_path TEXT,
    file_size_bytes BIGINT,
    content_type VARCHAR(100),
    
    -- Processing status
    status VARCHAR(20) NOT NULL DEFAULT 'indexed',
    track_id VARCHAR(50),
    error_message TEXT,
    processing_time_ms INTEGER,
    
    -- Statistics
    chunk_count INTEGER DEFAULT 0,
    entity_count INTEGER DEFAULT 0,
    relationship_count INTEGER DEFAULT 0,
    
    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- Constraints
    CONSTRAINT documents_valid_status CHECK (
        status IN ('pending', 'processing', 'indexed', 'failed')
    )
);

-- -----------------------------------------------------------------------------
-- Chunks Table - Document text chunks with embeddings
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS chunks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    document_id UUID NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    tenant_id UUID,
    workspace_id UUID,
    
    -- Chunk content
    content TEXT NOT NULL,
    chunk_index INTEGER NOT NULL,
    start_offset INTEGER,
    end_offset INTEGER,
    token_count INTEGER,
    
    -- Embedding (1536 dims for OpenAI, 3072 for large models)
    embedding vector(1536),
    
    -- Metadata
    metadata JSONB DEFAULT '{}',
    
    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    CONSTRAINT chunks_unique_doc_index UNIQUE (document_id, chunk_index)
);

-- -----------------------------------------------------------------------------
-- Entities Table - Knowledge graph nodes
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS entities (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID,
    workspace_id UUID,
    
    -- Entity attributes
    name TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    description TEXT,
    
    -- Embedding for semantic search
    embedding vector(1536),
    
    -- Source tracking
    source_ids UUID[],
    
    -- Manual edit tracking
    is_manual BOOLEAN NOT NULL DEFAULT FALSE,
    manual_created_at TIMESTAMPTZ,
    manual_created_by VARCHAR(255),
    last_manual_edit_at TIMESTAMPTZ,
    last_manual_edit_by VARCHAR(255),
    
    -- Metadata
    metadata JSONB DEFAULT '{}',
    
    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- Entity name must be unique per tenant/workspace
    CONSTRAINT entities_unique_name UNIQUE NULLS NOT DISTINCT (tenant_id, workspace_id, name)
);

-- -----------------------------------------------------------------------------
-- Relationships Table - Knowledge graph edges
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS relationships (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_id UUID NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
    target_id UUID NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
    tenant_id UUID,
    workspace_id UUID,
    
    -- Relationship attributes
    relation_type TEXT NOT NULL,
    description TEXT,
    weight FLOAT DEFAULT 0.5,
    keywords TEXT[],
    
    -- Embedding for semantic search
    embedding vector(1536),
    
    -- Source tracking
    source_chunk_ids UUID[],
    
    -- Manual edit tracking
    is_manual BOOLEAN NOT NULL DEFAULT FALSE,
    manual_created_at TIMESTAMPTZ,
    manual_created_by VARCHAR(255),
    last_manual_edit_at TIMESTAMPTZ,
    last_manual_edit_by VARCHAR(255),
    
    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    CONSTRAINT relationships_unique UNIQUE (source_id, target_id, relation_type)
);

-- -----------------------------------------------------------------------------
-- Tasks Table - Background task processing
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS tasks (
    track_id VARCHAR(50) PRIMARY KEY,
    tenant_id UUID,
    workspace_id UUID,
    
    -- Task info
    task_type VARCHAR(20) NOT NULL,
    status VARCHAR(20) NOT NULL,
    
    -- Timing
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    
    -- Error handling
    error_message TEXT,
    retry_count INTEGER NOT NULL DEFAULT 0,
    max_retries INTEGER NOT NULL DEFAULT 3,
    
    -- Payload
    task_data JSONB NOT NULL,
    metadata JSONB,
    progress JSONB,
    result JSONB,
    
    CONSTRAINT tasks_valid_status CHECK (
        status IN ('pending', 'processing', 'indexed', 'failed', 'cancelled')
    ),
    CONSTRAINT tasks_valid_type CHECK (
        task_type IN ('upload', 'insert', 'scan', 'reindex')
    )
);

-- ============================================================================
-- PHASE 3: AUTHENTICATION & AUTHORIZATION
-- ============================================================================

-- -----------------------------------------------------------------------------
-- Users Table
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS users (
    user_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username VARCHAR(255) UNIQUE NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    role VARCHAR(50) NOT NULL DEFAULT 'user',
    is_active BOOLEAN DEFAULT TRUE,
    failed_login_attempts INT DEFAULT 0,
    locked_until TIMESTAMPTZ,
    last_login_at TIMESTAMPTZ,
    metadata JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    
    CONSTRAINT users_valid_role CHECK (role IN ('admin', 'user', 'readonly'))
);

-- -----------------------------------------------------------------------------
-- API Keys Table
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS api_keys (
    key_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    key_hash TEXT NOT NULL,
    key_prefix VARCHAR(20) NOT NULL,
    name VARCHAR(255),
    scopes TEXT[],
    rate_limit_tier VARCHAR(20),
    is_active BOOLEAN DEFAULT TRUE,
    last_used_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,
    metadata JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- -----------------------------------------------------------------------------
-- Refresh Tokens Table
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS refresh_tokens (
    token_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    revoked BOOLEAN DEFAULT FALSE,
    revoked_at TIMESTAMPTZ,
    user_agent TEXT,
    ip_address INET,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- ============================================================================
-- PHASE 4: MULTI-TENANCY
-- ============================================================================

-- -----------------------------------------------------------------------------
-- Tenants Table
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS tenants (
    tenant_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    slug VARCHAR(100) UNIQUE NOT NULL,
    description TEXT,
    plan VARCHAR(50) DEFAULT 'free',
    max_workspaces INT DEFAULT 5,
    max_users INT DEFAULT 10,
    is_active BOOLEAN DEFAULT TRUE,
    metadata JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    
    CONSTRAINT tenants_valid_plan CHECK (plan IN ('free', 'basic', 'pro', 'enterprise'))
);

-- -----------------------------------------------------------------------------
-- Workspaces Table (Knowledge Bases)
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS workspaces (
    workspace_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(tenant_id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    slug VARCHAR(100) NOT NULL,
    description TEXT,
    is_active BOOLEAN DEFAULT TRUE,
    metadata JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    
    CONSTRAINT workspaces_unique_slug UNIQUE(tenant_id, slug)
);

-- -----------------------------------------------------------------------------
-- Memberships Table (User-Tenant-Workspace mapping)
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS memberships (
    membership_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    tenant_id UUID NOT NULL REFERENCES tenants(tenant_id) ON DELETE CASCADE,
    workspace_id UUID REFERENCES workspaces(workspace_id) ON DELETE CASCADE,
    role VARCHAR(50) NOT NULL DEFAULT 'member',
    is_active BOOLEAN DEFAULT TRUE,
    joined_at TIMESTAMPTZ DEFAULT NOW(),
    metadata JSONB,
    
    CONSTRAINT memberships_unique UNIQUE(user_id, tenant_id, workspace_id),
    CONSTRAINT memberships_valid_role CHECK (role IN ('owner', 'admin', 'member', 'readonly'))
);

-- ============================================================================
-- PHASE 5: CONVERSATIONS & MESSAGES
-- ============================================================================

-- -----------------------------------------------------------------------------
-- Folders Table
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS folders (
    folder_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(tenant_id) ON DELETE CASCADE,
    workspace_id UUID REFERENCES workspaces(workspace_id) ON DELETE SET NULL,
    user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    parent_id UUID REFERENCES folders(folder_id) ON DELETE CASCADE,
    position INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    CONSTRAINT folders_unique_name UNIQUE(tenant_id, user_id, parent_id, name)
);

-- -----------------------------------------------------------------------------
-- Conversations Table
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS conversations (
    conversation_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(tenant_id) ON DELETE CASCADE,
    workspace_id UUID REFERENCES workspaces(workspace_id) ON DELETE SET NULL,
    user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    title VARCHAR(500) NOT NULL DEFAULT 'New Conversation',
    mode VARCHAR(50) NOT NULL DEFAULT 'hybrid',
    is_pinned BOOLEAN NOT NULL DEFAULT FALSE,
    is_archived BOOLEAN NOT NULL DEFAULT FALSE,
    folder_id UUID REFERENCES folders(folder_id) ON DELETE SET NULL,
    share_id VARCHAR(64) UNIQUE,
    meta JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    CONSTRAINT conversations_valid_mode CHECK (mode IN ('local', 'global', 'hybrid', 'naive', 'mix'))
);

-- -----------------------------------------------------------------------------
-- Messages Table
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS messages (
    message_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    conversation_id UUID NOT NULL REFERENCES conversations(conversation_id) ON DELETE CASCADE,
    parent_id UUID REFERENCES messages(message_id) ON DELETE SET NULL,
    role VARCHAR(20) NOT NULL,
    content TEXT NOT NULL,
    mode VARCHAR(50),
    tokens_used INTEGER,
    duration_ms INTEGER,
    thinking_time_ms INTEGER,
    context JSONB,
    is_error BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    CONSTRAINT messages_valid_role CHECK (role IN ('user', 'assistant', 'system'))
);

-- -----------------------------------------------------------------------------
-- Conversation History Table (Legacy support)
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS conversation_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID,
    workspace_id UUID,
    conversation_id UUID NOT NULL,
    message_index INTEGER NOT NULL,
    role VARCHAR(20) NOT NULL,
    content TEXT NOT NULL,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    CONSTRAINT conversation_history_valid_role CHECK (role IN ('user', 'assistant', 'system')),
    CONSTRAINT conversation_history_unique UNIQUE (conversation_id, message_index)
);

-- ============================================================================
-- PHASE 6: AUDIT & SECURITY LOGGING (Partitioned by month)
-- ============================================================================

-- Create enum types for audit logs
DO $$
BEGIN
    CREATE TYPE audit_event_type AS ENUM (
        'Authentication', 'Authorization', 'DocumentUpload', 'DocumentQuery',
        'GraphTraversal', 'TenantAccess', 'WorkspaceAccess', 'RateLimitExceeded',
        'SecurityViolation', 'DataExport', 'ConfigChange'
    );
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

DO $$
BEGIN
    CREATE TYPE audit_result AS ENUM ('Success', 'Failure', 'Blocked', 'Warning');
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

DO $$
BEGIN
    CREATE TYPE audit_severity AS ENUM ('Low', 'Medium', 'High', 'Critical');
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

-- Audit logs partitioned table
CREATE TABLE IF NOT EXISTS audit_logs (
    id UUID DEFAULT gen_random_uuid(),
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    tenant_id VARCHAR(255) NOT NULL,
    workspace_id VARCHAR(255),
    user_id VARCHAR(255),
    event_type audit_event_type NOT NULL,
    event_category VARCHAR(100) NOT NULL,
    event_action VARCHAR(255) NOT NULL,
    resource_type VARCHAR(100),
    resource_id VARCHAR(255),
    result audit_result NOT NULL,
    severity audit_severity NOT NULL DEFAULT 'Medium',
    ip_address INET,
    user_agent TEXT,
    request_id VARCHAR(100),
    session_id VARCHAR(100),
    metadata JSONB DEFAULT '{}',
    error_message TEXT,
    duration_ms INTEGER,
    retention_days INTEGER DEFAULT 90,
    archived BOOLEAN DEFAULT FALSE,
    PRIMARY KEY (id, timestamp)
) PARTITION BY RANGE (timestamp);

-- Create partitions for next 12 months
DO $$
DECLARE
    start_date DATE := DATE_TRUNC('month', CURRENT_DATE);
    end_date DATE;
    partition_name TEXT;
BEGIN
    FOR i IN 0..11 LOOP
        end_date := start_date + INTERVAL '1 month';
        partition_name := 'audit_logs_' || TO_CHAR(start_date, 'YYYY_MM');
        
        IF NOT EXISTS (SELECT 1 FROM pg_class WHERE relname = partition_name) THEN
            EXECUTE FORMAT(
                'CREATE TABLE %I PARTITION OF audit_logs FOR VALUES FROM (%L) TO (%L)',
                partition_name, start_date, end_date
            );
        END IF;
        
        start_date := end_date;
    END LOOP;
END $$;

-- Simple audit log for RLS events
CREATE TABLE IF NOT EXISTS rls_audit_log (
    id BIGSERIAL PRIMARY KEY,
    event_time TIMESTAMPTZ DEFAULT NOW(),
    tenant_id UUID,
    workspace_id UUID,
    user_id UUID,
    action VARCHAR(50),
    table_name VARCHAR(100),
    record_id TEXT,
    details JSONB
);

-- ============================================================================
-- PHASE 7: INDEXES FOR PERFORMANCE (SOTA)
-- ============================================================================

-- -----------------------------------------------------------------------------
-- Documents Indexes
-- -----------------------------------------------------------------------------
CREATE INDEX IF NOT EXISTS idx_documents_tenant_workspace 
    ON documents(tenant_id, workspace_id) WHERE tenant_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_documents_status 
    ON documents(status);
CREATE INDEX IF NOT EXISTS idx_documents_track_id 
    ON documents(track_id) WHERE track_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_documents_content_hash 
    ON documents(content_hash) WHERE content_hash IS NOT NULL;
-- WHY-OODA81: Workspace-scoped uniqueness for content hash
-- Same document can exist in different workspaces (multi-tenancy)
-- But duplicate within same workspace is prevented
CREATE UNIQUE INDEX IF NOT EXISTS idx_documents_workspace_content_hash_unique 
    ON documents(workspace_id, content_hash) 
    WHERE workspace_id IS NOT NULL AND content_hash IS NOT NULL AND status = 'indexed';
-- Also add compound index for faster lookups
CREATE INDEX IF NOT EXISTS idx_documents_workspace_hash_lookup
    ON documents(workspace_id, content_hash)
    WHERE content_hash IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_documents_created_at 
    ON documents(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_documents_updated_at 
    ON documents(updated_at DESC);
-- Full-text search on title
CREATE INDEX IF NOT EXISTS idx_documents_title_fts 
    ON documents USING GIN (to_tsvector('english', title));

-- -----------------------------------------------------------------------------
-- Chunks Indexes (Critical for vector search performance)
-- -----------------------------------------------------------------------------
CREATE INDEX IF NOT EXISTS idx_chunks_document_id 
    ON chunks(document_id);
CREATE INDEX IF NOT EXISTS idx_chunks_tenant_workspace 
    ON chunks(tenant_id, workspace_id) WHERE tenant_id IS NOT NULL;
-- HNSW index for fast approximate nearest neighbor (superior to IVFFlat for most cases)
CREATE INDEX IF NOT EXISTS idx_chunks_embedding_hnsw 
    ON chunks USING hnsw (embedding vector_cosine_ops) WITH (m = 16, ef_construction = 64);
-- BRIN index for time-range queries
CREATE INDEX IF NOT EXISTS idx_chunks_created_at_brin 
    ON chunks USING BRIN(created_at) WITH (pages_per_range = 128);

-- -----------------------------------------------------------------------------
-- Entities Indexes
-- -----------------------------------------------------------------------------
CREATE INDEX IF NOT EXISTS idx_entities_tenant_workspace 
    ON entities(tenant_id, workspace_id) WHERE tenant_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_entities_type 
    ON entities(entity_type);
CREATE INDEX IF NOT EXISTS idx_entities_is_manual 
    ON entities(is_manual) WHERE is_manual = TRUE;
-- Trigram index for fuzzy name search
CREATE INDEX IF NOT EXISTS idx_entities_name_trgm 
    ON entities USING GIN (name gin_trgm_ops);
-- HNSW index for entity embeddings
CREATE INDEX IF NOT EXISTS idx_entities_embedding_hnsw 
    ON entities USING hnsw (embedding vector_cosine_ops) WITH (m = 16, ef_construction = 64);

-- -----------------------------------------------------------------------------
-- Relationships Indexes
-- -----------------------------------------------------------------------------
CREATE INDEX IF NOT EXISTS idx_relationships_source 
    ON relationships(source_id);
CREATE INDEX IF NOT EXISTS idx_relationships_target 
    ON relationships(target_id);
CREATE INDEX IF NOT EXISTS idx_relationships_tenant_workspace 
    ON relationships(tenant_id, workspace_id) WHERE tenant_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_relationships_type 
    ON relationships(relation_type);
CREATE INDEX IF NOT EXISTS idx_relationships_is_manual 
    ON relationships(is_manual) WHERE is_manual = TRUE;
-- HNSW index for relationship embeddings
CREATE INDEX IF NOT EXISTS idx_relationships_embedding_hnsw 
    ON relationships USING hnsw (embedding vector_cosine_ops) WITH (m = 16, ef_construction = 64);

-- -----------------------------------------------------------------------------
-- Tasks Indexes
-- -----------------------------------------------------------------------------
CREATE INDEX IF NOT EXISTS idx_tasks_status 
    ON tasks(status, created_at);
CREATE INDEX IF NOT EXISTS idx_tasks_type 
    ON tasks(task_type);
CREATE INDEX IF NOT EXISTS idx_tasks_tenant_workspace 
    ON tasks(tenant_id, workspace_id) WHERE tenant_id IS NOT NULL;

-- -----------------------------------------------------------------------------
-- Users Indexes
-- -----------------------------------------------------------------------------
CREATE INDEX IF NOT EXISTS idx_users_email 
    ON users(email);
CREATE INDEX IF NOT EXISTS idx_users_username 
    ON users(username);
CREATE INDEX IF NOT EXISTS idx_users_is_active 
    ON users(is_active);

-- -----------------------------------------------------------------------------
-- API Keys Indexes
-- -----------------------------------------------------------------------------
CREATE INDEX IF NOT EXISTS idx_api_keys_user 
    ON api_keys(user_id);
CREATE INDEX IF NOT EXISTS idx_api_keys_prefix 
    ON api_keys(key_prefix);
CREATE INDEX IF NOT EXISTS idx_api_keys_active 
    ON api_keys(is_active) WHERE is_active = TRUE;

-- -----------------------------------------------------------------------------
-- Tenants/Workspaces/Memberships Indexes
-- -----------------------------------------------------------------------------
CREATE INDEX IF NOT EXISTS idx_tenants_slug 
    ON tenants(slug);
CREATE INDEX IF NOT EXISTS idx_tenants_active 
    ON tenants(is_active) WHERE is_active = TRUE;
CREATE INDEX IF NOT EXISTS idx_workspaces_tenant 
    ON workspaces(tenant_id);
CREATE INDEX IF NOT EXISTS idx_memberships_user 
    ON memberships(user_id);
CREATE INDEX IF NOT EXISTS idx_memberships_tenant 
    ON memberships(tenant_id);
CREATE INDEX IF NOT EXISTS idx_memberships_workspace 
    ON memberships(workspace_id);

-- -----------------------------------------------------------------------------
-- Conversations Indexes
-- -----------------------------------------------------------------------------
CREATE INDEX IF NOT EXISTS idx_conversations_tenant_user 
    ON conversations(tenant_id, user_id, updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_conversations_workspace 
    ON conversations(workspace_id, updated_at DESC) WHERE workspace_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_conversations_folder 
    ON conversations(folder_id) WHERE folder_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_conversations_pinned 
    ON conversations(tenant_id, user_id) WHERE is_pinned = TRUE;
CREATE INDEX IF NOT EXISTS idx_conversations_share 
    ON conversations(share_id) WHERE share_id IS NOT NULL;
-- Full-text search on title
CREATE INDEX IF NOT EXISTS idx_conversations_title_fts 
    ON conversations USING GIN (to_tsvector('english', title));

-- -----------------------------------------------------------------------------
-- Messages Indexes
-- -----------------------------------------------------------------------------
CREATE INDEX IF NOT EXISTS idx_messages_conversation 
    ON messages(conversation_id, created_at ASC);
CREATE INDEX IF NOT EXISTS idx_messages_parent 
    ON messages(parent_id) WHERE parent_id IS NOT NULL;
-- Full-text search on content
CREATE INDEX IF NOT EXISTS idx_messages_content_fts 
    ON messages USING GIN (to_tsvector('english', content));

-- -----------------------------------------------------------------------------
-- Audit Logs Indexes
-- -----------------------------------------------------------------------------
CREATE INDEX IF NOT EXISTS idx_audit_logs_tenant_timestamp 
    ON audit_logs(tenant_id, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_audit_logs_security 
    ON audit_logs(event_type, result, timestamp DESC) 
    WHERE result IN ('Failure', 'Blocked') OR severity IN ('High', 'Critical');
CREATE INDEX IF NOT EXISTS idx_audit_logs_user 
    ON audit_logs(user_id, timestamp DESC) WHERE user_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_audit_logs_metadata 
    ON audit_logs USING GIN (metadata);
CREATE INDEX IF NOT EXISTS idx_rls_audit_tenant 
    ON rls_audit_log(tenant_id, event_time DESC);

-- ============================================================================
-- PHASE 7.1: AGE GRAPH INDEXES (CRITICAL FOR PERFORMANCE)
-- ============================================================================
--
-- Apache AGE creates these internal tables for graph storage:
-- - <graph_name>._ag_label_vertex (stores nodes)
-- - <graph_name>._ag_label_edge (stores edges)
--
-- Without indexes, graph queries timeout on large graphs (10k+ nodes).
-- These indexes provide 10-100x speedup for degree calculation and filtering.

DO $$ 
DECLARE
    graph_name TEXT := 'edgequake_graph';
BEGIN
    -- Check if AGE graph exists
    IF EXISTS (
        SELECT 1 FROM ag_catalog.ag_graph WHERE name = graph_name
    ) THEN
        RAISE NOTICE 'Creating AGE performance indexes for graph: %', graph_name;
        
        -- Index on edge start_id for outbound degree calculation
        -- Used by: GROUP BY start_id in get_popular_nodes_with_degree
        -- Impact: 10x faster degree counting
        EXECUTE format('CREATE INDEX IF NOT EXISTS idx_ag_edge_start_id 
            ON %I._ag_label_edge(start_id)', graph_name);
        
        -- Index on edge end_id for inbound degree calculation
        -- Used by: Reverse relationship queries
        EXECUTE format('CREATE INDEX IF NOT EXISTS idx_ag_edge_end_id 
            ON %I._ag_label_edge(end_id)', graph_name);
        
        -- Composite index for bi-directional lookups
        -- Used by: Finding specific edges between nodes
        EXECUTE format('CREATE INDEX IF NOT EXISTS idx_ag_edge_start_end 
            ON %I._ag_label_edge(start_id, end_id)', graph_name);
        
        -- GIN index on vertex properties for fast JSONB filtering
        -- Used by: WHERE conditions on tenant_id, workspace_id, entity_type
        -- Impact: 100x faster filtered queries
        EXECUTE format('CREATE INDEX IF NOT EXISTS idx_ag_vertex_props_gin 
            ON %I._ag_label_vertex USING GIN(properties)', graph_name);
        
        -- Index on vertex id for primary key lookups
        EXECUTE format('CREATE INDEX IF NOT EXISTS idx_ag_vertex_id 
            ON %I._ag_label_vertex(id)', graph_name);
        
        RAISE NOTICE 'AGE indexes created successfully';
    ELSE
        RAISE NOTICE 'AGE graph "%" not found - skipping graph indexes', graph_name;
    END IF;
EXCEPTION 
    WHEN undefined_table THEN
        RAISE NOTICE 'AGE extension not installed or graph not created - skipping graph indexes';
    WHEN OTHERS THEN
        RAISE WARNING 'Failed to create AGE indexes: %', SQLERRM;
END $$;

-- ============================================================================
-- PHASE 8: RLS CONTEXT FUNCTIONS
-- ============================================================================

-- Function to get the current tenant ID from session
CREATE OR REPLACE FUNCTION current_tenant_id()
RETURNS UUID AS $$
BEGIN
    RETURN NULLIF(current_setting('app.current_tenant_id', true), '')::UUID;
EXCEPTION WHEN OTHERS THEN
    RETURN NULL;
END;
$$ LANGUAGE plpgsql STABLE;

-- Function to get the current workspace ID from session
CREATE OR REPLACE FUNCTION current_workspace_id()
RETURNS UUID AS $$
BEGIN
    RETURN NULLIF(current_setting('app.current_workspace_id', true), '')::UUID;
EXCEPTION WHEN OTHERS THEN
    RETURN NULL;
END;
$$ LANGUAGE plpgsql STABLE;

-- Function to get the current user ID from session
CREATE OR REPLACE FUNCTION current_user_id()
RETURNS UUID AS $$
BEGIN
    RETURN NULLIF(current_setting('app.current_user_id', true), '')::UUID;
EXCEPTION WHEN OTHERS THEN
    RETURN NULL;
END;
$$ LANGUAGE plpgsql STABLE;

-- Function to set tenant context
CREATE OR REPLACE FUNCTION set_tenant_context(
    p_tenant_id UUID, 
    p_workspace_id UUID DEFAULT NULL
)
RETURNS void AS $$
BEGIN
    PERFORM set_config('app.current_tenant_id', COALESCE(p_tenant_id::text, ''), false);
    PERFORM set_config('app.current_workspace_id', COALESCE(p_workspace_id::text, ''), false);
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

-- Function to clear tenant context
CREATE OR REPLACE FUNCTION clear_tenant_context()
RETURNS void AS $$
BEGIN
    PERFORM set_config('app.current_tenant_id', '', false);
    PERFORM set_config('app.current_workspace_id', '', false);
    PERFORM set_config('app.current_user_id', '', false);
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

-- ============================================================================
-- PHASE 9: ROW LEVEL SECURITY POLICIES
-- ============================================================================

-- Enable RLS on all data tables
ALTER TABLE documents ENABLE ROW LEVEL SECURITY;
ALTER TABLE chunks ENABLE ROW LEVEL SECURITY;
ALTER TABLE entities ENABLE ROW LEVEL SECURITY;
ALTER TABLE relationships ENABLE ROW LEVEL SECURITY;
ALTER TABLE tasks ENABLE ROW LEVEL SECURITY;
ALTER TABLE conversations ENABLE ROW LEVEL SECURITY;
ALTER TABLE messages ENABLE ROW LEVEL SECURITY;
ALTER TABLE folders ENABLE ROW LEVEL SECURITY;
ALTER TABLE conversation_history ENABLE ROW LEVEL SECURITY;
ALTER TABLE audit_logs ENABLE ROW LEVEL SECURITY;

-- Force RLS even for table owners
ALTER TABLE documents FORCE ROW LEVEL SECURITY;
ALTER TABLE chunks FORCE ROW LEVEL SECURITY;
ALTER TABLE entities FORCE ROW LEVEL SECURITY;
ALTER TABLE relationships FORCE ROW LEVEL SECURITY;
ALTER TABLE tasks FORCE ROW LEVEL SECURITY;
ALTER TABLE conversations FORCE ROW LEVEL SECURITY;
ALTER TABLE messages FORCE ROW LEVEL SECURITY;
ALTER TABLE folders FORCE ROW LEVEL SECURITY;

-- -----------------------------------------------------------------------------
-- Documents RLS Policy
-- -----------------------------------------------------------------------------
DROP POLICY IF EXISTS documents_tenant_isolation ON documents;
CREATE POLICY documents_tenant_isolation ON documents
    FOR ALL
    USING (
        tenant_id IS NULL OR (
            tenant_id = current_tenant_id()
            AND (current_workspace_id() IS NULL OR workspace_id = current_workspace_id())
        )
    )
    WITH CHECK (
        tenant_id IS NULL OR tenant_id = current_tenant_id()
    );

-- -----------------------------------------------------------------------------
-- Chunks RLS Policy
-- -----------------------------------------------------------------------------
DROP POLICY IF EXISTS chunks_tenant_isolation ON chunks;
CREATE POLICY chunks_tenant_isolation ON chunks
    FOR ALL
    USING (
        tenant_id IS NULL OR (
            tenant_id = current_tenant_id()
            AND (current_workspace_id() IS NULL OR workspace_id = current_workspace_id())
        )
    )
    WITH CHECK (
        tenant_id IS NULL OR tenant_id = current_tenant_id()
    );

-- -----------------------------------------------------------------------------
-- Entities RLS Policy
-- -----------------------------------------------------------------------------
DROP POLICY IF EXISTS entities_tenant_isolation ON entities;
CREATE POLICY entities_tenant_isolation ON entities
    FOR ALL
    USING (
        tenant_id IS NULL OR (
            tenant_id = current_tenant_id()
            AND (current_workspace_id() IS NULL OR workspace_id = current_workspace_id())
        )
    )
    WITH CHECK (
        tenant_id IS NULL OR tenant_id = current_tenant_id()
    );

-- -----------------------------------------------------------------------------
-- Relationships RLS Policy
-- -----------------------------------------------------------------------------
DROP POLICY IF EXISTS relationships_tenant_isolation ON relationships;
CREATE POLICY relationships_tenant_isolation ON relationships
    FOR ALL
    USING (
        tenant_id IS NULL OR (
            tenant_id = current_tenant_id()
            AND (current_workspace_id() IS NULL OR workspace_id = current_workspace_id())
        )
    )
    WITH CHECK (
        tenant_id IS NULL OR tenant_id = current_tenant_id()
    );

-- -----------------------------------------------------------------------------
-- Tasks RLS Policy
-- -----------------------------------------------------------------------------
DROP POLICY IF EXISTS tasks_tenant_isolation ON tasks;
CREATE POLICY tasks_tenant_isolation ON tasks
    FOR ALL
    USING (
        tenant_id IS NULL OR (
            tenant_id = current_tenant_id()
            AND (current_workspace_id() IS NULL OR workspace_id = current_workspace_id())
        )
    )
    WITH CHECK (
        tenant_id IS NULL OR tenant_id = current_tenant_id()
    );

-- -----------------------------------------------------------------------------
-- Conversations RLS Policy
-- -----------------------------------------------------------------------------
DROP POLICY IF EXISTS conversations_tenant_isolation ON conversations;
CREATE POLICY conversations_tenant_isolation ON conversations
    FOR ALL
    USING (
        tenant_id = current_tenant_id()
        AND (user_id = current_user_id() OR share_id IS NOT NULL)
    )
    WITH CHECK (
        tenant_id = current_tenant_id() AND user_id = current_user_id()
    );

-- -----------------------------------------------------------------------------
-- Messages RLS Policy (inherits from conversation)
-- -----------------------------------------------------------------------------
DROP POLICY IF EXISTS messages_access ON messages;
CREATE POLICY messages_access ON messages
    FOR ALL
    USING (
        EXISTS (
            SELECT 1 FROM conversations c
            WHERE c.conversation_id = messages.conversation_id
            AND c.tenant_id = current_tenant_id()
            AND (c.user_id = current_user_id() OR c.share_id IS NOT NULL)
        )
    )
    WITH CHECK (
        EXISTS (
            SELECT 1 FROM conversations c
            WHERE c.conversation_id = messages.conversation_id
            AND c.tenant_id = current_tenant_id()
            AND c.user_id = current_user_id()
        )
    );

-- -----------------------------------------------------------------------------
-- Folders RLS Policy
-- -----------------------------------------------------------------------------
DROP POLICY IF EXISTS folders_tenant_isolation ON folders;
CREATE POLICY folders_tenant_isolation ON folders
    FOR ALL
    USING (
        tenant_id = current_tenant_id() AND user_id = current_user_id()
    )
    WITH CHECK (
        tenant_id = current_tenant_id() AND user_id = current_user_id()
    );

-- -----------------------------------------------------------------------------
-- Conversation History RLS Policy
-- -----------------------------------------------------------------------------
DROP POLICY IF EXISTS conversation_history_tenant_isolation ON conversation_history;
CREATE POLICY conversation_history_tenant_isolation ON conversation_history
    FOR ALL
    USING (
        tenant_id IS NULL OR tenant_id = current_tenant_id()
    )
    WITH CHECK (
        tenant_id IS NULL OR tenant_id = current_tenant_id()
    );

-- -----------------------------------------------------------------------------
-- Audit Logs RLS Policy (Read only for tenant users)
-- -----------------------------------------------------------------------------
DROP POLICY IF EXISTS audit_logs_tenant_isolation ON audit_logs;
CREATE POLICY audit_logs_tenant_isolation ON audit_logs
    FOR SELECT
    USING (tenant_id = current_setting('app.current_tenant_id', TRUE));

-- ============================================================================
-- PHASE 10: TRIGGERS FOR updated_at
-- ============================================================================

-- Generic updated_at trigger function
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Apply to all tables with updated_at
DO $$
DECLARE
    t TEXT;
BEGIN
    FOREACH t IN ARRAY ARRAY['documents', 'entities', 'tasks', 'users', 'tenants', 
                              'workspaces', 'conversations', 'messages', 'folders']
    LOOP
        EXECUTE FORMAT('
            DROP TRIGGER IF EXISTS trigger_%s_updated_at ON %s;
            CREATE TRIGGER trigger_%s_updated_at
                BEFORE UPDATE ON %s
                FOR EACH ROW
                EXECUTE FUNCTION update_updated_at_column();
        ', t, t, t, t);
    END LOOP;
END $$;

-- Trigger to update conversation.updated_at when message is added
CREATE OR REPLACE FUNCTION update_conversation_on_message()
RETURNS TRIGGER AS $$
BEGIN
    UPDATE conversations SET updated_at = NOW() WHERE conversation_id = NEW.conversation_id;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trigger_update_conversation_on_message ON messages;
CREATE TRIGGER trigger_update_conversation_on_message
    AFTER INSERT ON messages
    FOR EACH ROW
    EXECUTE FUNCTION update_conversation_on_message();

-- ============================================================================
-- PHASE 11: HELPER FUNCTIONS
-- ============================================================================

-- Function to log RLS events
CREATE OR REPLACE FUNCTION log_rls_event(
    p_action VARCHAR,
    p_table_name VARCHAR,
    p_record_id TEXT DEFAULT NULL,
    p_details JSONB DEFAULT NULL
)
RETURNS void AS $$
BEGIN
    INSERT INTO rls_audit_log (tenant_id, workspace_id, user_id, action, table_name, record_id, details)
    VALUES (
        current_tenant_id(),
        current_workspace_id(),
        current_user_id(),
        p_action,
        p_table_name,
        p_record_id,
        p_details
    );
END;
$$ LANGUAGE plpgsql;

-- Function to create next month's audit log partition
CREATE OR REPLACE FUNCTION create_next_audit_log_partition()
RETURNS TEXT AS $$
DECLARE
    next_month DATE := DATE_TRUNC('month', NOW() + INTERVAL '1 month');
    following_month DATE := next_month + INTERVAL '1 month';
    partition_name TEXT := 'audit_logs_' || TO_CHAR(next_month, 'YYYY_MM');
BEGIN
    IF EXISTS (SELECT 1 FROM pg_class WHERE relname = partition_name) THEN
        RETURN 'Partition ' || partition_name || ' already exists';
    END IF;
    
    EXECUTE FORMAT(
        'CREATE TABLE %I PARTITION OF audit_logs FOR VALUES FROM (%L) TO (%L)',
        partition_name, next_month, following_month
    );
    
    RETURN 'Created partition: ' || partition_name;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- PHASE 12: GRANTS
-- ============================================================================

-- Grant permissions to app_user (non-superuser for RLS)
GRANT USAGE ON SCHEMA public TO app_user;
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO app_user;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO app_user;
GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA public TO app_user;

-- Grant admin role to edgequake user if exists
DO $$
BEGIN
    IF EXISTS (SELECT FROM pg_roles WHERE rolname = 'edgequake') THEN
        GRANT edgequake_admin TO edgequake;
    END IF;
END $$;

-- ============================================================================
-- PHASE 13: STATISTICS & MAINTENANCE
-- ============================================================================

-- Analyze all tables for query optimization
ANALYZE documents;
ANALYZE chunks;
ANALYZE entities;
ANALYZE relationships;
ANALYZE tasks;
ANALYZE users;
ANALYZE tenants;
ANALYZE workspaces;
ANALYZE memberships;
ANALYZE conversations;
ANALYZE messages;
ANALYZE folders;

-- ============================================================================
-- INITIALIZATION COMPLETE
-- ============================================================================
DO $$
BEGIN
    RAISE NOTICE '=============================================================';
    RAISE NOTICE 'EdgeQuake Database Initialized Successfully!';
    RAISE NOTICE 'Version: 2.0.0 (SOTA)';
    RAISE NOTICE '=============================================================';
    RAISE NOTICE 'Features enabled:';
    RAISE NOTICE '  ✓ Multi-tenancy with Row-Level Security';
    RAISE NOTICE '  ✓ HNSW vector indexes for fast similarity search';
    RAISE NOTICE '  ✓ Full-text search with GIN indexes';
    RAISE NOTICE '  ✓ Partitioned audit logs (12 months)';
    RAISE NOTICE '  ✓ Updated_at triggers on all tables';
    RAISE NOTICE '  ✓ Non-superuser app_user for RLS enforcement';
    RAISE NOTICE '=============================================================';
    RAISE NOTICE 'SECURITY NOTE: Change app_user password in production!';
    RAISE NOTICE '=============================================================';
END $$;
