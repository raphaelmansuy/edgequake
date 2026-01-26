-- Migration 016: Workspace Metrics History (SAFE VERSION)
-- Purpose: Track document, entity, relationship, embedding counts over time
-- Use case: Trend analysis, capacity planning, billing, debugging

-- =============================================================================
-- PRE-FLIGHT CHECKS
-- =============================================================================

DO $$
DECLARE
    workspaces_exists BOOLEAN;
    workspace_id_type TEXT;
BEGIN
    RAISE NOTICE '=== Migration 016: Pre-flight checks ===';
    
    -- Check 1: workspaces table exists
    SELECT EXISTS (
        SELECT 1 FROM information_schema.tables 
        WHERE table_name = 'workspaces' AND table_schema = 'public'
    ) INTO workspaces_exists;
    
    IF NOT workspaces_exists THEN
        RAISE EXCEPTION 'Migration 016 requires workspaces table (from migration 001)';
    END IF;
    RAISE NOTICE '✓ workspaces table exists';
    
    -- Check 2: workspace_id column exists and is UUID
    SELECT udt_name INTO workspace_id_type
    FROM information_schema.columns 
    WHERE table_schema = 'public'
    AND table_name = 'workspaces' 
    AND column_name = 'workspace_id';
    
    IF workspace_id_type IS NULL THEN
        RAISE EXCEPTION 'Migration 016 requires workspaces.workspace_id column';
    END IF;
    
    IF workspace_id_type != 'uuid' THEN
        RAISE EXCEPTION 'Expected workspaces.workspace_id to be UUID, got %', workspace_id_type;
    END IF;
    RAISE NOTICE '✓ workspaces.workspace_id is UUID type';
    
    -- Check 3: Ensure we have at least one workspace (sanity check)
    IF NOT EXISTS (SELECT 1 FROM workspaces LIMIT 1) THEN
        RAISE NOTICE '⚠ No workspaces exist yet - metrics table will be empty initially';
    ELSE
        RAISE NOTICE '✓ At least one workspace exists';
    END IF;
    
    -- Check 4: Check if table already exists (idempotency)
    IF EXISTS (
        SELECT 1 FROM information_schema.tables 
        WHERE table_name = 'workspace_metrics_history' AND table_schema = 'public'
    ) THEN
        RAISE NOTICE '⚠ workspace_metrics_history already exists - migration may be no-op';
    END IF;
    
    RAISE NOTICE '=== Pre-flight checks passed ===';
END $$;

-- =============================================================================
-- MIGRATION START
-- =============================================================================

BEGIN;

DO $$
BEGIN
    RAISE NOTICE '=== Migration 016: Creating workspace_metrics_history ===';
    RAISE NOTICE 'Started at: %', now();
END $$;

-- Create workspace_metrics_history table
-- WHY: Separate table (not column additions) for time-series data
-- WHY: trigger_type distinguishes event-driven vs scheduled samples
-- WHY: Indexes optimized for time-range queries per workspace
-- WHY: CASCADE delete ensures cleanup when workspace is deleted
CREATE TABLE IF NOT EXISTS workspace_metrics_history (
    -- Primary key
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    
    -- Foreign key to workspace (UUID type matches workspaces.workspace_id)
    -- WHY: Type MUST match parent table to avoid "incompatible types" error
    workspace_id UUID NOT NULL,
    
    -- Timestamp when metrics were recorded
    -- WHY: TIMESTAMPTZ for timezone-aware storage
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    
    -- How was this sample triggered?
    -- 'event' = document add/delete operation
    -- 'scheduled' = hourly background task
    -- 'manual' = admin triggered via API
    -- WHY: Distinguish between real-time events and scheduled snapshots
    trigger_type TEXT NOT NULL DEFAULT 'event',
    
    -- Point-in-time counts
    -- WHY: BIGINT to handle large deployments (millions of documents)
    document_count BIGINT NOT NULL DEFAULT 0,
    chunk_count BIGINT NOT NULL DEFAULT 0,
    entity_count BIGINT NOT NULL DEFAULT 0,
    relationship_count BIGINT NOT NULL DEFAULT 0,
    embedding_count BIGINT NOT NULL DEFAULT 0,
    storage_bytes BIGINT NOT NULL DEFAULT 0,
    
    -- Foreign key constraint with cascade delete
    -- WHY: When a workspace is deleted, its history should be cleaned up automatically
    -- WHY: CASCADE prevents orphaned metrics records
    CONSTRAINT fk_metrics_workspace 
        FOREIGN KEY (workspace_id) 
        REFERENCES workspaces(workspace_id) 
        ON DELETE CASCADE,
    
    -- Check constraint: counts must be non-negative
    -- WHY: Negative counts indicate data corruption
    CONSTRAINT check_metrics_non_negative CHECK (
        document_count >= 0 AND
        chunk_count >= 0 AND
        entity_count >= 0 AND
        relationship_count >= 0 AND
        embedding_count >= 0 AND
        storage_bytes >= 0
    )
);

-- Index for time-series queries: "Get metrics for workspace X in time range Y-Z"
-- WHY: Most common query pattern. DESC order for recent-first retrieval.
-- WHY: Composite index (workspace_id, recorded_at) for efficient filtering + sorting
CREATE INDEX IF NOT EXISTS idx_metrics_workspace_time 
    ON workspace_metrics_history(workspace_id, recorded_at DESC);

-- Index for cleanup queries: "Delete all records older than X"
-- WHY: Retention policy needs to efficiently find old records across all workspaces
CREATE INDEX IF NOT EXISTS idx_metrics_recorded_at 
    ON workspace_metrics_history(recorded_at);

-- Index for trigger type filtering: "Show only scheduled snapshots"
-- WHY: Analysis may want only scheduled samples for consistent intervals
-- WHY: Separate index allows efficient filtering by trigger_type
CREATE INDEX IF NOT EXISTS idx_metrics_trigger_type 
    ON workspace_metrics_history(trigger_type);

-- Composite index for common query: workspace + trigger type
-- WHY: Query pattern "Get event-triggered metrics for workspace X"
CREATE INDEX IF NOT EXISTS idx_metrics_workspace_trigger 
    ON workspace_metrics_history(workspace_id, trigger_type);

-- Add comments explaining the table purpose
COMMENT ON TABLE workspace_metrics_history IS 
    'Time-series storage of workspace metrics for monitoring and analysis.
     Samples are recorded either on events (document add/delete) or on schedule (hourly).
     Use with aggregation functions for trend analysis.
     
     Example queries:
     - Hourly trends: SELECT date_trunc(''hour'', recorded_at), AVG(document_count) GROUP BY 1
     - Growth rate: Compare document_count between time ranges
     - Billing: SUM(storage_bytes) per workspace per month';

COMMENT ON COLUMN workspace_metrics_history.trigger_type IS 
    'How the sample was triggered:
     - "event": Recorded after document add/delete operation (real-time)
     - "scheduled": Recorded by background task (consistent intervals)
     - "manual": Recorded by admin via API (debugging/audit)';

COMMENT ON COLUMN workspace_metrics_history.storage_bytes IS 
    'Total storage used by workspace in bytes (sum of document file sizes).
     Note: Does not include database overhead, only raw document content.';

COMMENT ON COLUMN workspace_metrics_history.recorded_at IS 
    'Timestamp when metrics were captured. Timezone-aware (TIMESTAMPTZ).
     Use for time-series queries and retention policies.';

-- =============================================================================
-- POST-MIGRATION VALIDATION
-- =============================================================================

DO $$
DECLARE
    table_exists BOOLEAN;
    workspace_id_fk_type TEXT;
    index_count INT;
    fk_exists BOOLEAN;
    check_constraint_exists BOOLEAN;
BEGIN
    RAISE NOTICE '=== Migration 016: Post-migration validation ===';
    
    -- Validate 1: Table was created
    SELECT EXISTS (
        SELECT 1 FROM information_schema.tables 
        WHERE table_name = 'workspace_metrics_history' AND table_schema = 'public'
    ) INTO table_exists;
    
    IF NOT table_exists THEN
        RAISE EXCEPTION 'Validation failed: workspace_metrics_history was not created';
    END IF;
    RAISE NOTICE '✓ Table workspace_metrics_history exists';
    
    -- Validate 2: workspace_id column has correct type (UUID)
    SELECT udt_name INTO workspace_id_fk_type
    FROM information_schema.columns 
    WHERE table_schema = 'public'
    AND table_name = 'workspace_metrics_history' 
    AND column_name = 'workspace_id';
    
    IF workspace_id_fk_type != 'uuid' THEN
        RAISE EXCEPTION 'Validation failed: workspace_id is %, expected uuid', workspace_id_fk_type;
    END IF;
    RAISE NOTICE '✓ workspace_id column type is UUID';
    
    -- Validate 3: Foreign key constraint exists
    SELECT EXISTS (
        SELECT 1 FROM information_schema.table_constraints 
        WHERE constraint_type = 'FOREIGN KEY' 
        AND table_schema = 'public'
        AND table_name = 'workspace_metrics_history'
        AND constraint_name = 'fk_metrics_workspace'
    ) INTO fk_exists;
    
    IF NOT fk_exists THEN
        RAISE EXCEPTION 'Validation failed: foreign key constraint fk_metrics_workspace not created';
    END IF;
    RAISE NOTICE '✓ Foreign key constraint fk_metrics_workspace exists';
    
    -- Validate 4: Check constraint exists
    SELECT EXISTS (
        SELECT 1 FROM information_schema.table_constraints 
        WHERE constraint_type = 'CHECK' 
        AND table_schema = 'public'
        AND table_name = 'workspace_metrics_history'
        AND constraint_name = 'check_metrics_non_negative'
    ) INTO check_constraint_exists;
    
    IF NOT check_constraint_exists THEN
        RAISE EXCEPTION 'Validation failed: check constraint check_metrics_non_negative not created';
    END IF;
    RAISE NOTICE '✓ Check constraint check_metrics_non_negative exists';
    
    -- Validate 5: All indexes created
    SELECT COUNT(*) INTO index_count
    FROM pg_indexes 
    WHERE schemaname = 'public'
    AND tablename = 'workspace_metrics_history';
    
    IF index_count < 4 THEN
        RAISE EXCEPTION 'Validation failed: Expected 4+ indexes, found %', index_count;
    END IF;
    RAISE NOTICE '✓ All % indexes created', index_count;
    
    -- Validate 6: Foreign key references correct column
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.constraint_column_usage ccu
        JOIN information_schema.table_constraints tc 
            ON ccu.constraint_name = tc.constraint_name
        WHERE tc.constraint_type = 'FOREIGN KEY'
        AND tc.table_name = 'workspace_metrics_history'
        AND ccu.table_name = 'workspaces'
        AND ccu.column_name = 'workspace_id'
    ) THEN
        RAISE EXCEPTION 'Validation failed: Foreign key does not reference workspaces.workspace_id';
    END IF;
    RAISE NOTICE '✓ Foreign key references workspaces(workspace_id)';
    
    -- Validate 7: Can insert a test record (if workspaces exist)
    IF EXISTS (SELECT 1 FROM workspaces LIMIT 1) THEN
        DECLARE
            test_workspace_id UUID;
            test_record_id UUID;
        BEGIN
            -- Get first workspace ID
            SELECT workspace_id INTO test_workspace_id FROM workspaces LIMIT 1;
            
            -- Insert test record
            INSERT INTO workspace_metrics_history (
                workspace_id,
                trigger_type,
                document_count,
                chunk_count,
                entity_count,
                relationship_count,
                embedding_count,
                storage_bytes
            ) VALUES (
                test_workspace_id,
                'manual',
                0, 0, 0, 0, 0, 0
            ) RETURNING id INTO test_record_id;
            
            -- Verify it was inserted
            IF test_record_id IS NULL THEN
                RAISE EXCEPTION 'Validation failed: Could not insert test record';
            END IF;
            
            -- Clean up test record
            DELETE FROM workspace_metrics_history WHERE id = test_record_id;
            
            RAISE NOTICE '✓ Insert/delete test passed';
        END;
    ELSE
        RAISE NOTICE '⚠ Skipped insert test (no workspaces exist yet)';
    END IF;
    
    RAISE NOTICE '=== All validation checks passed ===';
END $$;

DO $$
BEGIN
    RAISE NOTICE '=== Migration 016 completed successfully ===';
    RAISE NOTICE 'Finished at: %', now();
    RAISE NOTICE 'Table: workspace_metrics_history';
    RAISE NOTICE 'Indexes: 4 (workspace_time, recorded_at, trigger_type, workspace_trigger)';
    RAISE NOTICE 'Foreign keys: 1 (workspaces.workspace_id CASCADE)';
    RAISE NOTICE 'Check constraints: 1 (non-negative counts)';
END $$;

COMMIT;  -- Only commits if all validations passed
