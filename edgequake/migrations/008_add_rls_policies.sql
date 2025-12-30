-- Migration: 008_add_rls_policies.sql
SET search_path = public;
-- Row-Level Security (RLS) for Multi-Tenant Isolation
-- Created: 2025-12-24
-- Purpose: Implement true row-level isolation between tenants

-- ============================================================================
-- STEP 1: Add tenant_id and workspace_id to all core data tables
-- ============================================================================

-- Documents table
ALTER TABLE documents
ADD COLUMN IF NOT EXISTS tenant_id UUID,
ADD COLUMN IF NOT EXISTS workspace_id UUID;

-- Entities table
ALTER TABLE entities
ADD COLUMN IF NOT EXISTS tenant_id UUID,
ADD COLUMN IF NOT EXISTS workspace_id UUID;

-- Relationships table  
ALTER TABLE relationships
ADD COLUMN IF NOT EXISTS tenant_id UUID,
ADD COLUMN IF NOT EXISTS workspace_id UUID;

-- Chunks table
ALTER TABLE chunks
ADD COLUMN IF NOT EXISTS tenant_id UUID,
ADD COLUMN IF NOT EXISTS workspace_id UUID;

-- Embeddings table (COMMENTED OUT - table does not exist, embeddings are in chunks table)
-- ALTER TABLE embeddings
-- ADD COLUMN IF NOT EXISTS tenant_id UUID,
-- ADD COLUMN IF NOT EXISTS workspace_id UUID;

-- Conversation history table
ALTER TABLE conversation_history
ADD COLUMN IF NOT EXISTS tenant_id UUID,
ADD COLUMN IF NOT EXISTS workspace_id UUID;

-- Tasks table
ALTER TABLE tasks
ADD COLUMN IF NOT EXISTS tenant_id UUID,
ADD COLUMN IF NOT EXISTS workspace_id UUID;

-- ============================================================================
-- STEP 2: Create indexes for tenant/workspace columns
-- ============================================================================

CREATE INDEX IF NOT EXISTS idx_documents_tenant_workspace 
    ON documents(tenant_id, workspace_id);
CREATE INDEX IF NOT EXISTS idx_entities_tenant_workspace 
    ON entities(tenant_id, workspace_id);
CREATE INDEX IF NOT EXISTS idx_relationships_tenant_workspace 
    ON relationships(tenant_id, workspace_id);
CREATE INDEX IF NOT EXISTS idx_chunks_tenant_workspace 
    ON chunks(tenant_id, workspace_id);
-- CREATE INDEX IF NOT EXISTS idx_embeddings_tenant_workspace 
--     ON embeddings(tenant_id, workspace_id);
CREATE INDEX IF NOT EXISTS idx_conversation_history_tenant_workspace 
    ON conversation_history(tenant_id, workspace_id);
CREATE INDEX IF NOT EXISTS idx_tasks_tenant_workspace 
    ON tasks(tenant_id, workspace_id);

-- ============================================================================
-- STEP 3: Create RLS context-setting functions
-- NOTE: Using 3-parameter version for compatibility with 009 and Rust code
-- ============================================================================

-- Function to set the current tenant context (3 parameters for user_id support)
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

-- Function to get the current tenant ID
CREATE OR REPLACE FUNCTION current_tenant_id()
RETURNS UUID AS $$
BEGIN
    RETURN NULLIF(current_setting('app.current_tenant_id', true), '')::UUID;
EXCEPTION WHEN OTHERS THEN
    RETURN NULL;
END;
$$ LANGUAGE plpgsql STABLE;

-- Function to get the current workspace ID
CREATE OR REPLACE FUNCTION current_workspace_id()
RETURNS UUID AS $$
BEGIN
    RETURN NULLIF(current_setting('app.current_workspace_id', true), '')::UUID;
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

-- Function to get the current user ID (for conversation RLS)
CREATE OR REPLACE FUNCTION current_user_id()
RETURNS UUID AS $$
BEGIN
    RETURN NULLIF(current_setting('app.current_user_id', true), '')::UUID;
EXCEPTION WHEN OTHERS THEN
    RETURN NULL;
END;
$$ LANGUAGE plpgsql STABLE;

-- ============================================================================
-- STEP 4: Enable RLS on all data tables
-- ============================================================================

ALTER TABLE documents ENABLE ROW LEVEL SECURITY;
ALTER TABLE entities ENABLE ROW LEVEL SECURITY;
ALTER TABLE relationships ENABLE ROW LEVEL SECURITY;
ALTER TABLE chunks ENABLE ROW LEVEL SECURITY;
-- ALTER TABLE embeddings ENABLE ROW LEVEL SECURITY;
ALTER TABLE conversation_history ENABLE ROW LEVEL SECURITY;
ALTER TABLE tasks ENABLE ROW LEVEL SECURITY;

-- ============================================================================
-- STEP 5: Create RLS policies for each table
-- ============================================================================

-- Documents policies
DROP POLICY IF EXISTS documents_tenant_isolation ON documents;
CREATE POLICY documents_tenant_isolation ON documents
    FOR ALL
    USING (
        -- Allow access if:
        -- 1. tenant_id is NULL (legacy data/no multi-tenancy) OR
        -- 2. tenant_id matches current tenant AND (workspace matches OR no workspace filter)
        tenant_id IS NULL 
        OR (
            tenant_id = current_tenant_id()
            AND (
                current_workspace_id() IS NULL 
                OR workspace_id = current_workspace_id()
            )
        )
    )
    WITH CHECK (
        -- On insert/update, tenant_id must match current context
        tenant_id IS NULL OR tenant_id = current_tenant_id()
    );

-- Entities policies
DROP POLICY IF EXISTS entities_tenant_isolation ON entities;
CREATE POLICY entities_tenant_isolation ON entities
    FOR ALL
    USING (
        tenant_id IS NULL 
        OR (
            tenant_id = current_tenant_id()
            AND (
                current_workspace_id() IS NULL 
                OR workspace_id = current_workspace_id()
            )
        )
    )
    WITH CHECK (
        tenant_id IS NULL OR tenant_id = current_tenant_id()
    );

-- Relationships policies
DROP POLICY IF EXISTS relationships_tenant_isolation ON relationships;
CREATE POLICY relationships_tenant_isolation ON relationships
    FOR ALL
    USING (
        tenant_id IS NULL 
        OR (
            tenant_id = current_tenant_id()
            AND (
                current_workspace_id() IS NULL 
                OR workspace_id = current_workspace_id()
            )
        )
    )
    WITH CHECK (
        tenant_id IS NULL OR tenant_id = current_tenant_id()
    );

-- Chunks policies
DROP POLICY IF EXISTS chunks_tenant_isolation ON chunks;
CREATE POLICY chunks_tenant_isolation ON chunks
    FOR ALL
    USING (
        tenant_id IS NULL 
        OR (
            tenant_id = current_tenant_id()
            AND (
                current_workspace_id() IS NULL 
                OR workspace_id = current_workspace_id()
            )
        )
    )
    WITH CHECK (
        tenant_id IS NULL OR tenant_id = current_tenant_id()
    );

-- Embeddings policies (COMMENTED OUT - table does not exist)
-- DROP POLICY IF EXISTS embeddings_tenant_isolation ON embeddings;
-- CREATE POLICY embeddings_tenant_isolation ON embeddings
--     FOR ALL
--     USING (
--         tenant_id IS NULL 
--         OR (
--             tenant_id = current_tenant_id()
--             AND (
--                 current_workspace_id() IS NULL 
--                 OR workspace_id = current_workspace_id()
--             )
--         )
--     )
--     WITH CHECK (
--         tenant_id IS NULL OR tenant_id = current_tenant_id()
--     );

-- Conversation history policies
DROP POLICY IF EXISTS conversation_history_tenant_isolation ON conversation_history;
CREATE POLICY conversation_history_tenant_isolation ON conversation_history
    FOR ALL
    USING (
        tenant_id IS NULL 
        OR (
            tenant_id = current_tenant_id()
            AND (
                current_workspace_id() IS NULL 
                OR workspace_id = current_workspace_id()
            )
        )
    )
    WITH CHECK (
        tenant_id IS NULL OR tenant_id = current_tenant_id()
    );

-- Tasks policies
DROP POLICY IF EXISTS tasks_tenant_isolation ON tasks;
CREATE POLICY tasks_tenant_isolation ON tasks
    FOR ALL
    USING (
        tenant_id IS NULL 
        OR (
            tenant_id = current_tenant_id()
            AND (
                current_workspace_id() IS NULL 
                OR workspace_id = current_workspace_id()
            )
        )
    )
    WITH CHECK (
        tenant_id IS NULL OR tenant_id = current_tenant_id()
    );

-- ============================================================================
-- STEP 6: Create superuser bypass role (for admin operations)
-- ============================================================================

-- Create a role that bypasses RLS for admin operations
DO $$
BEGIN
    IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'edgequake_admin') THEN
        CREATE ROLE edgequake_admin NOLOGIN BYPASSRLS;
    END IF;
END
$$;

-- Grant the role to postgres superuser (adjust as needed)
-- GRANT edgequake_admin TO postgres;

-- ============================================================================
-- STEP 7: Create helper views for cross-workspace queries
-- ============================================================================

-- View for tenant-wide document count (respects RLS at tenant level)
CREATE OR REPLACE VIEW tenant_document_stats AS
SELECT 
    tenant_id,
    workspace_id,
    COUNT(*) as document_count,
    SUM(COALESCE((metadata->>'chunk_count')::int, 0)) as total_chunks
FROM documents
WHERE tenant_id = current_tenant_id()
GROUP BY tenant_id, workspace_id;

-- View for tenant-wide entity stats
CREATE OR REPLACE VIEW tenant_entity_stats AS
SELECT 
    tenant_id,
    workspace_id,
    COUNT(*) as entity_count,
    COUNT(DISTINCT entity_type) as entity_type_count
FROM entities
WHERE tenant_id = current_tenant_id()
GROUP BY tenant_id, workspace_id;

-- ============================================================================
-- STEP 8: Add foreign key constraints from data tables to workspaces
-- ============================================================================

-- Note: Only add if tables exist and columns are populated
-- These are optional and can be added after data migration

-- ALTER TABLE documents 
--     ADD CONSTRAINT fk_documents_workspace 
--     FOREIGN KEY (workspace_id) REFERENCES workspaces(workspace_id) ON DELETE CASCADE;

-- ALTER TABLE entities
--     ADD CONSTRAINT fk_entities_workspace
--     FOREIGN KEY (workspace_id) REFERENCES workspaces(workspace_id) ON DELETE CASCADE;

-- ============================================================================
-- STEP 9: Create audit trigger for tenant context changes
-- ============================================================================

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

CREATE INDEX IF NOT EXISTS idx_rls_audit_tenant ON rls_audit_log(tenant_id, event_time DESC);

-- Function to log RLS-related events
CREATE OR REPLACE FUNCTION log_rls_event(
    p_action VARCHAR,
    p_table_name VARCHAR,
    p_record_id TEXT DEFAULT NULL,
    p_details JSONB DEFAULT NULL
)
RETURNS void AS $$
BEGIN
    INSERT INTO rls_audit_log (tenant_id, workspace_id, action, table_name, record_id, details)
    VALUES (
        current_tenant_id(),
        current_workspace_id(),
        p_action,
        p_table_name,
        p_record_id,
        p_details
    );
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- STEP 10: Create workspace quota enforcement
-- ============================================================================

-- Function to check workspace quota before insert
CREATE OR REPLACE FUNCTION check_workspace_quota()
RETURNS TRIGGER AS $$
DECLARE
    v_max_documents INT;
    v_current_count INT;
    v_workspace_id UUID;
BEGIN
    v_workspace_id := NEW.workspace_id;
    
    -- Skip quota check if no workspace_id
    IF v_workspace_id IS NULL THEN
        RETURN NEW;
    END IF;
    
    -- Get max documents from workspace metadata
    SELECT (metadata->>'max_documents')::INT INTO v_max_documents
    FROM workspaces
    WHERE workspace_id = v_workspace_id;
    
    -- Skip if no quota set
    IF v_max_documents IS NULL THEN
        RETURN NEW;
    END IF;
    
    -- Count current documents
    SELECT COUNT(*) INTO v_current_count
    FROM documents
    WHERE workspace_id = v_workspace_id;
    
    -- Check quota
    IF v_current_count >= v_max_documents THEN
        RAISE EXCEPTION 'Workspace document quota exceeded. Maximum: %, Current: %', 
            v_max_documents, v_current_count;
    END IF;
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Create trigger for quota enforcement
DROP TRIGGER IF EXISTS check_document_quota ON documents;
CREATE TRIGGER check_document_quota
    BEFORE INSERT ON documents
    FOR EACH ROW
    EXECUTE FUNCTION check_workspace_quota();

-- ============================================================================
-- MIGRATION COMPLETE
-- ============================================================================
COMMENT ON FUNCTION set_tenant_context(UUID, UUID, UUID) IS 'Sets the current tenant, workspace, and user context for RLS policies';
COMMENT ON FUNCTION current_tenant_id() IS 'Returns the current tenant ID from session context';
COMMENT ON FUNCTION current_workspace_id() IS 'Returns the current workspace ID from session context';
