-- Migration: Create audit logs table for security monitoring
SET search_path = public;
-- Version: V004
-- Description: Comprehensive audit logging system with partitioning support
-- Created: 2024-12-29

-- ============================================================================
-- AUDIT LOG ENUMS (Idempotent creation)
-- ============================================================================

-- Event types that can be audited
DO $$ BEGIN
    CREATE TYPE audit_event_type AS ENUM (
        'Authentication',
        'Authorization',
        'DocumentUpload',
        'DocumentQuery',
        'GraphTraversal',
        'TenantAccess',
        'WorkspaceAccess',
        'RateLimitExceeded',
        'SecurityViolation',
        'DataExport',
        'ConfigChange'
    );
EXCEPTION WHEN duplicate_object THEN
    NULL;
END $$;

-- Event result status
DO $$ BEGIN
    CREATE TYPE audit_result AS ENUM (
        'Success',
        'Failure',
        'Blocked',
        'Warning'
    );
EXCEPTION WHEN duplicate_object THEN
    NULL;
END $$;

-- Severity levels
DO $$ BEGIN
    CREATE TYPE audit_severity AS ENUM (
        'Low',
        'Medium',
        'High',
        'Critical'
    );
EXCEPTION WHEN duplicate_object THEN
    NULL;
END $$;

-- ============================================================================
-- AUDIT LOGS TABLE (Partitioned by time)
-- ============================================================================

CREATE TABLE IF NOT EXISTS audit_logs (
    id UUID NOT NULL DEFAULT gen_random_uuid(),
    
    -- Temporal information
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- Tenant context
    tenant_id VARCHAR(255) NOT NULL,
    workspace_id VARCHAR(255),
    user_id VARCHAR(255),
    
    -- Event classification
    event_type audit_event_type NOT NULL,
    event_category VARCHAR(100) NOT NULL, -- Additional categorization
    event_action VARCHAR(255) NOT NULL,   -- Specific action taken
    
    -- Event details
    resource_type VARCHAR(100),  -- e.g., "Document", "Entity", "Edge"
    resource_id VARCHAR(255),    -- ID of affected resource
    result audit_result NOT NULL,
    severity audit_severity NOT NULL DEFAULT 'Medium',
    
    -- Request context
    ip_address INET,
    user_agent TEXT,
    request_id VARCHAR(100),
    session_id VARCHAR(100),
    
    -- Additional metadata
    metadata JSONB DEFAULT '{}',  -- Flexible data for event-specific info
    error_message TEXT,           -- Error details if result = 'Failure'
    
    -- Compliance fields
    retention_days INTEGER DEFAULT 90, -- How long to keep this log
    archived BOOLEAN DEFAULT FALSE,    -- Whether log has been archived
    
    -- Performance tracking
    duration_ms INTEGER,  -- Time taken for the operation
    
    -- Primary key must include partition column for partitioned tables
    PRIMARY KEY (id, timestamp),
    
    -- Indexing hints
    CONSTRAINT audit_logs_tenant_not_null CHECK (tenant_id IS NOT NULL)
) PARTITION BY RANGE (timestamp);

-- ============================================================================
-- PARTITIONS (Monthly partitions for 6 months)
-- ============================================================================

-- Current month
DO $$ BEGIN
    CREATE TABLE audit_logs_2024_12 PARTITION OF audit_logs
    FOR VALUES FROM ('2024-12-01') TO ('2025-01-01');
EXCEPTION WHEN duplicate_table THEN
    NULL;
END $$;

-- Next 5 months
DO $$ BEGIN
    CREATE TABLE audit_logs_2025_01 PARTITION OF audit_logs
    FOR VALUES FROM ('2025-01-01') TO ('2025-02-01');
EXCEPTION WHEN duplicate_table THEN
    NULL;
END $$;

DO $$ BEGIN
    CREATE TABLE audit_logs_2025_02 PARTITION OF audit_logs
    FOR VALUES FROM ('2025-02-01') TO ('2025-03-01');
EXCEPTION WHEN duplicate_table THEN
    NULL;
END $$;

DO $$ BEGIN
    CREATE TABLE audit_logs_2025_03 PARTITION OF audit_logs
    FOR VALUES FROM ('2025-03-01') TO ('2025-04-01');
EXCEPTION WHEN duplicate_table THEN
    NULL;
END $$;

DO $$ BEGIN
    CREATE TABLE audit_logs_2025_04 PARTITION OF audit_logs
    FOR VALUES FROM ('2025-04-01') TO ('2025-05-01');
EXCEPTION WHEN duplicate_table THEN
    NULL;
END $$;

DO $$ BEGIN
    CREATE TABLE audit_logs_2025_05 PARTITION OF audit_logs
    FOR VALUES FROM ('2025-05-01') TO ('2025-06-01');
EXCEPTION WHEN duplicate_table THEN
    NULL;
END $$;

-- ============================================================================
-- INDEXES ON PARTITIONED TABLE
-- ============================================================================

-- Primary lookup index (tenant + time)
CREATE INDEX IF NOT EXISTS idx_audit_logs_tenant_timestamp 
ON audit_logs(tenant_id, timestamp DESC);

-- Security investigation index
CREATE INDEX IF NOT EXISTS idx_audit_logs_security 
ON audit_logs(event_type, result, timestamp DESC)
WHERE result IN ('Failure', 'Blocked') OR severity IN ('High', 'Critical');

-- User activity index
CREATE INDEX IF NOT EXISTS idx_audit_logs_user_activity 
ON audit_logs(user_id, timestamp DESC)
WHERE user_id IS NOT NULL;

-- Resource access index
CREATE INDEX IF NOT EXISTS idx_audit_logs_resource 
ON audit_logs(resource_type, resource_id, timestamp DESC)
WHERE resource_id IS NOT NULL;

-- Workspace activity index
CREATE INDEX IF NOT EXISTS idx_audit_logs_workspace 
ON audit_logs(workspace_id, timestamp DESC)
WHERE workspace_id IS NOT NULL;

-- Request correlation index
CREATE INDEX IF NOT EXISTS idx_audit_logs_request_id 
ON audit_logs(request_id)
WHERE request_id IS NOT NULL;

-- JSONB metadata index for flexible queries
CREATE INDEX IF NOT EXISTS idx_audit_logs_metadata_gin 
ON audit_logs USING GIN (metadata);

-- ============================================================================
-- ROW-LEVEL SECURITY POLICIES
-- ============================================================================

-- Enable RLS
ALTER TABLE audit_logs ENABLE ROW LEVEL SECURITY;

-- Policy: Users can only see logs for their tenant
DO $$ BEGIN
    CREATE POLICY audit_logs_tenant_isolation ON audit_logs
        FOR SELECT
        USING (tenant_id = current_setting('app.tenant_id', TRUE));
EXCEPTION WHEN duplicate_object THEN
    NULL;
END $$;

-- Policy: Only admins can insert audit logs (via application service)
DO $$ BEGIN
    CREATE POLICY audit_logs_insert_admin ON audit_logs
        FOR INSERT
        WITH CHECK (current_setting('app.is_admin', TRUE) = 'true');
EXCEPTION WHEN duplicate_object THEN
    NULL;
END $$;

-- ============================================================================
-- AUTOMATIC ARCHIVAL FUNCTION
-- ============================================================================

-- Function to mark old logs for archival
CREATE OR REPLACE FUNCTION mark_audit_logs_for_archival()
RETURNS INTEGER AS $$
DECLARE
    archived_count INTEGER;
BEGIN
    UPDATE audit_logs
    SET archived = TRUE
    WHERE timestamp < NOW() - INTERVAL '90 days'
      AND archived = FALSE
      AND retention_days <= 90
    RETURNING COUNT(*) INTO archived_count;
    
    RETURN archived_count;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- PARTITION MANAGEMENT FUNCTION
-- ============================================================================

-- Function to create next month's partition automatically
CREATE OR REPLACE FUNCTION create_next_audit_log_partition()
RETURNS TEXT AS $$
DECLARE
    next_month DATE;
    following_month DATE;
    partition_name TEXT;
    create_sql TEXT;
BEGIN
    -- Calculate next month
    next_month := DATE_TRUNC('month', NOW() + INTERVAL '1 month');
    following_month := next_month + INTERVAL '1 month';
    
    -- Generate partition name
    partition_name := 'audit_logs_' || TO_CHAR(next_month, 'YYYY_MM');
    
    -- Check if partition already exists
    IF EXISTS (
        SELECT 1 FROM pg_class WHERE relname = partition_name
    ) THEN
        RETURN 'Partition ' || partition_name || ' already exists';
    END IF;
    
    -- Create partition
    create_sql := FORMAT(
        'CREATE TABLE %I PARTITION OF audit_logs FOR VALUES FROM (%L) TO (%L)',
        partition_name,
        next_month,
        following_month
    );
    
    EXECUTE create_sql;
    
    RETURN 'Created partition: ' || partition_name;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- AUDIT LOG HELPER VIEWS
-- ============================================================================

-- View: Recent security events
CREATE OR REPLACE VIEW recent_security_events AS
SELECT 
    timestamp,
    tenant_id,
    user_id,
    event_type,
    event_action,
    result,
    severity,
    ip_address,
    error_message
FROM audit_logs
WHERE timestamp > NOW() - INTERVAL '24 hours'
  AND (result IN ('Failure', 'Blocked') OR severity IN ('High', 'Critical'))
ORDER BY timestamp DESC;

-- View: Tenant activity summary
CREATE OR REPLACE VIEW tenant_activity_summary AS
SELECT 
    tenant_id,
    workspace_id,
    event_type,
    result,
    COUNT(*) as event_count,
    MAX(timestamp) as last_event_time
FROM audit_logs
WHERE timestamp > NOW() - INTERVAL '7 days'
GROUP BY tenant_id, workspace_id, event_type, result
ORDER BY tenant_id, event_count DESC;

-- View: Rate limit violations
CREATE OR REPLACE VIEW rate_limit_violations AS
SELECT 
    tenant_id,
    workspace_id,
    user_id,
    ip_address,
    COUNT(*) as violation_count,
    MAX(timestamp) as last_violation_time
FROM audit_logs
WHERE event_type = 'RateLimitExceeded'
  AND timestamp > NOW() - INTERVAL '1 hour'
GROUP BY tenant_id, workspace_id, user_id, ip_address
HAVING COUNT(*) > 5
ORDER BY violation_count DESC;

-- ============================================================================
-- GRANTS (Application service account)
-- ============================================================================

-- GRANT INSERT, SELECT ON audit_logs TO edgequake_app;
-- GRANT SELECT ON recent_security_events TO edgequake_app;
-- GRANT SELECT ON tenant_activity_summary TO edgequake_app;
-- GRANT SELECT ON rate_limit_violations TO edgequake_app;
-- GRANT EXECUTE ON FUNCTION mark_audit_logs_for_archival TO edgequake_app;
-- GRANT EXECUTE ON FUNCTION create_next_audit_log_partition TO edgequake_app;

-- ============================================================================
-- USAGE EXAMPLES
-- ============================================================================

-- Insert an audit log entry
-- INSERT INTO audit_logs (
--     tenant_id, workspace_id, user_id,
--     event_type, event_category, event_action,
--     result, severity,
--     resource_type, resource_id,
--     ip_address, user_agent, request_id,
--     metadata
-- ) VALUES (
--     'tenant-123', 'workspace-456', 'user-789',
--     'DocumentQuery', 'Query', 'HybridSearch',
--     'Success', 'Low',
--     'Document', 'doc-abc',
--     '192.168.1.100'::INET, 'Mozilla/5.0', 'req-xyz',
--     '{"query": "search term", "results_count": 10}'::JSONB
-- );

-- Query recent failed authentications for a tenant
-- SELECT timestamp, user_id, ip_address, error_message
-- FROM audit_logs
-- WHERE tenant_id = 'tenant-123'
--   AND event_type = 'Authentication'
--   AND result = 'Failure'
--   AND timestamp > NOW() - INTERVAL '24 hours'
-- ORDER BY timestamp DESC;

-- Find suspicious activity patterns
-- SELECT tenant_id, user_id, ip_address, COUNT(*) as failed_attempts
-- FROM audit_logs
-- WHERE result = 'Failure'
--   AND timestamp > NOW() - INTERVAL '1 hour'
-- GROUP BY tenant_id, user_id, ip_address
-- HAVING COUNT(*) > 10
-- ORDER BY failed_attempts DESC;

-- ============================================================================
-- ROLLBACK SCRIPT
-- ============================================================================

-- DROP VIEW IF EXISTS rate_limit_violations;
-- DROP VIEW IF EXISTS tenant_activity_summary;
-- DROP VIEW IF EXISTS recent_security_events;
-- DROP FUNCTION IF EXISTS create_next_audit_log_partition();
-- DROP FUNCTION IF EXISTS mark_audit_logs_for_archival();
-- DROP TABLE IF EXISTS audit_logs CASCADE;
-- DROP TYPE IF EXISTS audit_severity;
-- DROP TYPE IF EXISTS audit_result;
-- DROP TYPE IF EXISTS audit_event_type;
