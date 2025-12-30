-- ============================================================================
-- EdgeQuake Database Initialization Script
-- Version: 2.0.0
-- Date: 2025-01-28
-- Description: Complete database setup with full idempotency
-- ============================================================================
--
-- This script initializes the EdgeQuake database from scratch or safely
-- upgrades an existing installation. All operations are idempotent.
--
-- USAGE:
--   psql -U postgres -d edgequake -f 000_init_database.sql
--
-- REQUIREMENTS:
--   - PostgreSQL 14+
--   - pgvector extension installed
--   - Apache AGE extension (optional, for graph features)
--
-- ============================================================================

-- CRITICAL: Set search_path to public FIRST to ensure all tables are created
-- in the public schema, not in the user's schema (edgequake)
SET search_path = public;

-- ============================================================================
-- SECTION 1: EXTENSIONS
-- ============================================================================

-- UUID generation
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Vector similarity search (pgvector)
CREATE EXTENSION IF NOT EXISTS "vector";

-- Apache AGE for graph database (optional)
DO $$ 
BEGIN
    CREATE EXTENSION IF NOT EXISTS "age" CASCADE;
    RAISE NOTICE 'Apache AGE extension enabled successfully';
EXCEPTION WHEN OTHERS THEN
    RAISE NOTICE 'Apache AGE extension not available: %. Graph features will use fallback storage.', SQLERRM;
END $$;

-- ============================================================================
-- SECTION 2: SCHEMA SETUP
-- ============================================================================

-- Create edgequake schema for namespacing (optional, core tables use public)
CREATE SCHEMA IF NOT EXISTS edgequake;

-- Set search path to include ag_catalog if AGE is available
-- IMPORTANT: public must be first so tables are created in public schema
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age') THEN
        EXECUTE 'SET search_path = public, ag_catalog, "$user"';
    END IF;
END $$;

-- ============================================================================
-- SECTION 3: CORE DATA TYPES
-- ============================================================================

-- Document processing status enum
DO $$ BEGIN
    CREATE TYPE document_status AS ENUM ('pending', 'processing', 'indexed', 'failed');
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

-- Audit event types
DO $$ BEGIN
    CREATE TYPE audit_event_type AS ENUM (
        'Authentication', 'Authorization', 'DocumentUpload', 'DocumentQuery',
        'GraphTraversal', 'TenantAccess', 'WorkspaceAccess', 'RateLimitExceeded',
        'SecurityViolation', 'DataExport', 'ConfigChange'
    );
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

-- Audit result types
DO $$ BEGIN
    CREATE TYPE audit_result AS ENUM ('Success', 'Failure', 'Blocked', 'Warning');
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

-- Audit severity levels
DO $$ BEGIN
    CREATE TYPE audit_severity AS ENUM ('Low', 'Medium', 'High', 'Critical');
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

-- ============================================================================
-- SECTION 4: MULTI-TENANCY TABLES
-- ============================================================================

-- Tenants table
CREATE TABLE IF NOT EXISTS tenants (
    tenant_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    slug VARCHAR(100) UNIQUE,
    settings JSONB NOT NULL DEFAULT '{}',
    metadata JSONB NOT NULL DEFAULT '{}',
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_tenants_slug ON tenants(slug) WHERE slug IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_tenants_active ON tenants(is_active) WHERE is_active = TRUE;

-- Users table
CREATE TABLE IF NOT EXISTS users (
    user_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(tenant_id) ON DELETE CASCADE,
    email VARCHAR(255),
    username VARCHAR(100),
    display_name VARCHAR(255),
    password_hash VARCHAR(255),
    role VARCHAR(50) NOT NULL DEFAULT 'user',
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    last_login_at TIMESTAMPTZ,
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT users_email_unique UNIQUE (tenant_id, email),
    CONSTRAINT users_username_unique UNIQUE (tenant_id, username),
    CONSTRAINT valid_user_role CHECK (role IN ('admin', 'user', 'readonly'))
);

CREATE INDEX IF NOT EXISTS idx_users_tenant ON users(tenant_id);
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email) WHERE email IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_users_role ON users(role);

-- Workspaces table
CREATE TABLE IF NOT EXISTS workspaces (
    workspace_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(tenant_id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    slug VARCHAR(100),
    description TEXT,
    settings JSONB NOT NULL DEFAULT '{}',
    metadata JSONB NOT NULL DEFAULT '{}',
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT workspaces_slug_unique UNIQUE (tenant_id, slug)
);

CREATE INDEX IF NOT EXISTS idx_workspaces_tenant ON workspaces(tenant_id);
CREATE INDEX IF NOT EXISTS idx_workspaces_slug ON workspaces(tenant_id, slug) WHERE slug IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_workspaces_active ON workspaces(is_active);

-- Workspace memberships (with tenant_id for multi-tenancy support)
CREATE TABLE IF NOT EXISTS memberships (
    membership_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(tenant_id) ON DELETE CASCADE,
    workspace_id UUID REFERENCES workspaces(workspace_id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    role VARCHAR(50) NOT NULL DEFAULT 'member',
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    joined_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    metadata JSONB,
    CONSTRAINT memberships_unique UNIQUE (user_id, tenant_id, workspace_id),
    CONSTRAINT valid_membership_role CHECK (role IN ('owner', 'admin', 'member', 'readonly'))
);

CREATE INDEX IF NOT EXISTS idx_memberships_user ON memberships(user_id);
CREATE INDEX IF NOT EXISTS idx_memberships_tenant ON memberships(tenant_id);
CREATE INDEX IF NOT EXISTS idx_memberships_workspace ON memberships(workspace_id);

-- ============================================================================
-- SECTION 5: CORE DATA TABLES
-- ============================================================================

-- Documents table
CREATE TABLE IF NOT EXISTS documents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID REFERENCES tenants(tenant_id) ON DELETE CASCADE,
    workspace_id UUID REFERENCES workspaces(workspace_id) ON DELETE CASCADE,
    title TEXT NOT NULL DEFAULT 'Untitled',
    content TEXT NOT NULL,
    content_hash VARCHAR(64),
    metadata JSONB NOT NULL DEFAULT '{}',
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
CREATE INDEX IF NOT EXISTS idx_documents_content_hash ON documents(content_hash) WHERE content_hash IS NOT NULL;

-- Chunks table (with embeddings)
CREATE TABLE IF NOT EXISTS chunks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    document_id UUID NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    tenant_id UUID REFERENCES tenants(tenant_id) ON DELETE CASCADE,
    workspace_id UUID REFERENCES workspaces(workspace_id) ON DELETE CASCADE,
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

-- Create HNSW index for vector similarity search (if table has data)
-- Note: Index creation is deferred to avoid issues with empty tables

-- Entities table
CREATE TABLE IF NOT EXISTS entities (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID REFERENCES tenants(tenant_id) ON DELETE CASCADE,
    workspace_id UUID REFERENCES workspaces(workspace_id) ON DELETE CASCADE,
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

-- Relationships table
CREATE TABLE IF NOT EXISTS relationships (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_id UUID NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
    target_id UUID NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
    tenant_id UUID REFERENCES tenants(tenant_id) ON DELETE CASCADE,
    workspace_id UUID REFERENCES workspaces(workspace_id) ON DELETE CASCADE,
    relation_type TEXT NOT NULL,
    description TEXT,
    weight REAL DEFAULT 1.0,
    keywords TEXT[],
    source_chunk_ids UUID[],
    is_manual BOOLEAN NOT NULL DEFAULT FALSE,
    manual_created_at TIMESTAMPTZ,
    manual_created_by VARCHAR(255),
    last_manual_edit_at TIMESTAMPTZ,
    last_manual_edit_by VARCHAR(255),
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT relationships_unique UNIQUE NULLS NOT DISTINCT (tenant_id, workspace_id, source_id, target_id, relation_type)
);

CREATE INDEX IF NOT EXISTS idx_relationships_tenant_workspace ON relationships(tenant_id, workspace_id);
CREATE INDEX IF NOT EXISTS idx_relationships_source ON relationships(source_id);
CREATE INDEX IF NOT EXISTS idx_relationships_target ON relationships(target_id);
CREATE INDEX IF NOT EXISTS idx_relationships_type ON relationships(relation_type);

-- Tasks table (for async processing)
CREATE TABLE IF NOT EXISTS tasks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID REFERENCES tenants(tenant_id) ON DELETE CASCADE,
    workspace_id UUID REFERENCES workspaces(workspace_id) ON DELETE CASCADE,
    track_id VARCHAR(100) NOT NULL,
    task_type VARCHAR(50) NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    priority INTEGER NOT NULL DEFAULT 0,
    payload JSONB NOT NULL DEFAULT '{}',
    result JSONB,
    error_message TEXT,
    retry_count INTEGER NOT NULL DEFAULT 0,
    max_retries INTEGER NOT NULL DEFAULT 3,
    scheduled_at TIMESTAMPTZ,
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT tasks_valid_status CHECK (status IN ('pending', 'running', 'completed', 'failed', 'cancelled'))
);

CREATE INDEX IF NOT EXISTS idx_tasks_track_id ON tasks(track_id);
CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);
CREATE INDEX IF NOT EXISTS idx_tasks_tenant_workspace ON tasks(tenant_id, workspace_id);
CREATE INDEX IF NOT EXISTS idx_tasks_scheduled ON tasks(scheduled_at) WHERE status = 'pending';

-- ============================================================================
-- SECTION 6: CONVERSATION TABLES
-- ============================================================================

-- Folders for organizing conversations
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
    CONSTRAINT unique_folder_name_in_parent UNIQUE (tenant_id, user_id, parent_id, name)
);

CREATE INDEX IF NOT EXISTS idx_folders_tenant_user ON folders(tenant_id, user_id);
CREATE INDEX IF NOT EXISTS idx_folders_parent ON folders(parent_id);

-- Conversations
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
    CONSTRAINT valid_mode CHECK (mode IN ('local', 'global', 'hybrid', 'naive', 'mix'))
);

CREATE INDEX IF NOT EXISTS idx_conversations_tenant_user ON conversations(tenant_id, user_id, updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_conversations_workspace ON conversations(workspace_id, updated_at DESC) WHERE workspace_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_conversations_folder ON conversations(folder_id) WHERE folder_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_conversations_share ON conversations(share_id) WHERE share_id IS NOT NULL;

-- Messages
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
    CONSTRAINT valid_role CHECK (role IN ('user', 'assistant', 'system'))
);

CREATE INDEX IF NOT EXISTS idx_messages_conversation ON messages(conversation_id, created_at ASC);
CREATE INDEX IF NOT EXISTS idx_messages_parent ON messages(parent_id) WHERE parent_id IS NOT NULL;

-- ============================================================================
-- SECTION 7: AUDIT LOG TABLE (Partitioned)
-- ============================================================================

CREATE TABLE IF NOT EXISTS audit_logs (
    id UUID NOT NULL DEFAULT gen_random_uuid(),
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
    retention_days INTEGER DEFAULT 90,
    archived BOOLEAN DEFAULT FALSE,
    duration_ms INTEGER,
    PRIMARY KEY (id, timestamp),
    CONSTRAINT audit_logs_tenant_not_null CHECK (tenant_id IS NOT NULL)
) PARTITION BY RANGE (timestamp);

-- Create partitions for next 12 months
DO $$
DECLARE
    start_date DATE;
    end_date DATE;
    partition_name TEXT;
BEGIN
    FOR i IN 0..11 LOOP
        start_date := DATE_TRUNC('month', CURRENT_DATE) + (i || ' months')::INTERVAL;
        end_date := start_date + '1 month'::INTERVAL;
        partition_name := 'audit_logs_' || TO_CHAR(start_date, 'YYYY_MM');
        
        BEGIN
            EXECUTE FORMAT(
                'CREATE TABLE %I PARTITION OF audit_logs FOR VALUES FROM (%L) TO (%L)',
                partition_name, start_date, end_date
            );
        EXCEPTION WHEN duplicate_table THEN
            NULL; -- Partition already exists
        END;
    END LOOP;
END $$;

CREATE INDEX IF NOT EXISTS idx_audit_logs_tenant_timestamp ON audit_logs(tenant_id, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_audit_logs_security ON audit_logs(event_type, result, timestamp DESC)
    WHERE result IN ('Failure', 'Blocked') OR severity IN ('High', 'Critical');

-- ============================================================================
-- SECTION 8: RLS CONTEXT FUNCTIONS
-- ============================================================================

-- Function to set tenant context (3-parameter version)
CREATE OR REPLACE FUNCTION set_tenant_context(
    p_tenant_id UUID,
    p_workspace_id UUID DEFAULT NULL,
    p_user_id UUID DEFAULT NULL
)
RETURNS void AS $$
BEGIN
    PERFORM set_config('app.current_tenant_id', COALESCE(p_tenant_id::text, ''), true);
    PERFORM set_config('app.current_workspace_id', COALESCE(p_workspace_id::text, ''), true);
    PERFORM set_config('app.current_user_id', COALESCE(p_user_id::text, ''), true);
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

-- Function to get current tenant ID
CREATE OR REPLACE FUNCTION current_tenant_id()
RETURNS UUID AS $$
BEGIN
    RETURN NULLIF(current_setting('app.current_tenant_id', true), '')::UUID;
EXCEPTION WHEN OTHERS THEN
    RETURN NULL;
END;
$$ LANGUAGE plpgsql STABLE;

-- Function to get current workspace ID
CREATE OR REPLACE FUNCTION current_workspace_id()
RETURNS UUID AS $$
BEGIN
    RETURN NULLIF(current_setting('app.current_workspace_id', true), '')::UUID;
EXCEPTION WHEN OTHERS THEN
    RETURN NULL;
END;
$$ LANGUAGE plpgsql STABLE;

-- Function to get current user ID
CREATE OR REPLACE FUNCTION current_user_id()
RETURNS UUID AS $$
BEGIN
    RETURN NULLIF(current_setting('app.current_user_id', true), '')::UUID;
EXCEPTION WHEN OTHERS THEN
    RETURN NULL;
END;
$$ LANGUAGE plpgsql STABLE;

-- Function to clear tenant context
CREATE OR REPLACE FUNCTION clear_tenant_context()
RETURNS void AS $$
BEGIN
    PERFORM set_config('app.current_tenant_id', '', true);
    PERFORM set_config('app.current_workspace_id', '', true);
    PERFORM set_config('app.current_user_id', '', true);
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

-- ============================================================================
-- SECTION 9: ENABLE ROW-LEVEL SECURITY
-- ============================================================================

ALTER TABLE documents ENABLE ROW LEVEL SECURITY;
ALTER TABLE chunks ENABLE ROW LEVEL SECURITY;
ALTER TABLE entities ENABLE ROW LEVEL SECURITY;
ALTER TABLE relationships ENABLE ROW LEVEL SECURITY;
ALTER TABLE tasks ENABLE ROW LEVEL SECURITY;
ALTER TABLE conversations ENABLE ROW LEVEL SECURITY;
ALTER TABLE messages ENABLE ROW LEVEL SECURITY;
ALTER TABLE folders ENABLE ROW LEVEL SECURITY;
ALTER TABLE audit_logs ENABLE ROW LEVEL SECURITY;

-- ============================================================================
-- SECTION 10: RLS POLICIES
-- ============================================================================

-- Documents policy
DROP POLICY IF EXISTS documents_tenant_isolation ON documents;
CREATE POLICY documents_tenant_isolation ON documents
    FOR ALL
    USING (
        tenant_id IS NULL 
        OR (
            tenant_id = current_tenant_id()
            AND (current_workspace_id() IS NULL OR workspace_id = current_workspace_id())
        )
    )
    WITH CHECK (tenant_id IS NULL OR tenant_id = current_tenant_id());

-- Chunks policy
DROP POLICY IF EXISTS chunks_tenant_isolation ON chunks;
CREATE POLICY chunks_tenant_isolation ON chunks
    FOR ALL
    USING (
        tenant_id IS NULL 
        OR (
            tenant_id = current_tenant_id()
            AND (current_workspace_id() IS NULL OR workspace_id = current_workspace_id())
        )
    )
    WITH CHECK (tenant_id IS NULL OR tenant_id = current_tenant_id());

-- Entities policy
DROP POLICY IF EXISTS entities_tenant_isolation ON entities;
CREATE POLICY entities_tenant_isolation ON entities
    FOR ALL
    USING (
        tenant_id IS NULL 
        OR (
            tenant_id = current_tenant_id()
            AND (current_workspace_id() IS NULL OR workspace_id = current_workspace_id())
        )
    )
    WITH CHECK (tenant_id IS NULL OR tenant_id = current_tenant_id());

-- Relationships policy
DROP POLICY IF EXISTS relationships_tenant_isolation ON relationships;
CREATE POLICY relationships_tenant_isolation ON relationships
    FOR ALL
    USING (
        tenant_id IS NULL 
        OR (
            tenant_id = current_tenant_id()
            AND (current_workspace_id() IS NULL OR workspace_id = current_workspace_id())
        )
    )
    WITH CHECK (tenant_id IS NULL OR tenant_id = current_tenant_id());

-- Tasks policy
DROP POLICY IF EXISTS tasks_tenant_isolation ON tasks;
CREATE POLICY tasks_tenant_isolation ON tasks
    FOR ALL
    USING (
        tenant_id IS NULL 
        OR (
            tenant_id = current_tenant_id()
            AND (current_workspace_id() IS NULL OR workspace_id = current_workspace_id())
        )
    )
    WITH CHECK (tenant_id IS NULL OR tenant_id = current_tenant_id());

-- Conversations policy (user-scoped)
DROP POLICY IF EXISTS conversations_tenant_isolation ON conversations;
CREATE POLICY conversations_tenant_isolation ON conversations
    FOR ALL
    USING (
        tenant_id = current_tenant_id()
        AND (user_id = current_user_id() OR share_id IS NOT NULL)
    )
    WITH CHECK (
        tenant_id = current_tenant_id()
        AND user_id = current_user_id()
    );

-- Messages policy (inherit from conversation)
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
    );

-- Folders policy
DROP POLICY IF EXISTS folders_access ON folders;
CREATE POLICY folders_access ON folders
    FOR ALL
    USING (
        tenant_id = current_tenant_id()
        AND user_id = current_user_id()
    );

-- Audit logs policy
DROP POLICY IF EXISTS audit_logs_tenant_isolation ON audit_logs;
CREATE POLICY audit_logs_tenant_isolation ON audit_logs
    FOR SELECT
    USING (tenant_id = current_setting('app.current_tenant_id', TRUE));

-- ============================================================================
-- SECTION 11: TRIGGERS
-- ============================================================================

-- Updated_at trigger function
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Apply triggers to all tables with updated_at
DO $$
DECLARE
    tbl TEXT;
BEGIN
    FOREACH tbl IN ARRAY ARRAY['documents', 'entities', 'relationships', 'tasks', 
                                'tenants', 'users', 'workspaces', 'conversations', 
                                'messages', 'folders']
    LOOP
        EXECUTE FORMAT('DROP TRIGGER IF EXISTS trigger_%s_updated_at ON %I', tbl, tbl);
        EXECUTE FORMAT('CREATE TRIGGER trigger_%s_updated_at BEFORE UPDATE ON %I FOR EACH ROW EXECUTE FUNCTION update_updated_at_column()', tbl, tbl);
    END LOOP;
END $$;

-- Trigger to update conversation.updated_at when message is added
CREATE OR REPLACE FUNCTION update_conversation_on_message()
RETURNS TRIGGER AS $$
BEGIN
    UPDATE conversations
    SET updated_at = NOW()
    WHERE conversation_id = NEW.conversation_id;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trigger_update_conversation_on_message ON messages;
CREATE TRIGGER trigger_update_conversation_on_message
    AFTER INSERT ON messages
    FOR EACH ROW
    EXECUTE FUNCTION update_conversation_on_message();

-- ============================================================================
-- SECTION 12: HELPER VIEWS
-- ============================================================================

-- View: Recent security events
CREATE OR REPLACE VIEW recent_security_events AS
SELECT 
    timestamp, tenant_id, user_id, event_type, event_action,
    result, severity, ip_address, error_message
FROM audit_logs
WHERE timestamp > NOW() - INTERVAL '24 hours'
  AND (result IN ('Failure', 'Blocked') OR severity IN ('High', 'Critical'))
ORDER BY timestamp DESC;

-- ============================================================================
-- SECTION 13: PARTITION MANAGEMENT
-- ============================================================================

-- Function to create next month's audit log partition
CREATE OR REPLACE FUNCTION create_next_audit_log_partition()
RETURNS TEXT AS $$
DECLARE
    next_month DATE;
    following_month DATE;
    partition_name TEXT;
BEGIN
    next_month := DATE_TRUNC('month', NOW() + INTERVAL '1 month');
    following_month := next_month + INTERVAL '1 month';
    partition_name := 'audit_logs_' || TO_CHAR(next_month, 'YYYY_MM');
    
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
-- SECTION 14: LEGACY COMPATIBILITY
-- ============================================================================

-- Create edgequake schema aliases for backward compatibility
-- (Applications using edgequake.* prefix will still work)
DO $$
BEGIN
    -- Create view aliases in edgequake schema
    EXECUTE 'CREATE OR REPLACE VIEW edgequake.documents AS SELECT * FROM public.documents';
    EXECUTE 'CREATE OR REPLACE VIEW edgequake.chunks AS SELECT * FROM public.chunks';
    EXECUTE 'CREATE OR REPLACE VIEW edgequake.entities AS SELECT * FROM public.entities';
    EXECUTE 'CREATE OR REPLACE VIEW edgequake.relationships AS SELECT * FROM public.relationships';
    EXECUTE 'CREATE OR REPLACE VIEW edgequake.tasks AS SELECT * FROM public.tasks';
EXCEPTION WHEN OTHERS THEN
    RAISE NOTICE 'Could not create schema aliases: %', SQLERRM;
END $$;

-- ============================================================================
-- COMPLETION NOTICE
-- ============================================================================

DO $$ BEGIN
    RAISE NOTICE '============================================';
    RAISE NOTICE 'EdgeQuake Database Initialization Complete!';
    RAISE NOTICE 'Version: 2.0.0';
    RAISE NOTICE '============================================';
END $$;
