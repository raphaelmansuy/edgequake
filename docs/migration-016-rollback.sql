-- Rollback Migration 016: Workspace Metrics History
-- Purpose: Safely remove workspace_metrics_history table and indexes
-- Use: When rolling back to migration 015

-- =============================================================================
-- PRE-ROLLBACK CHECKS
-- =============================================================================

DO $$
DECLARE
    table_exists BOOLEAN;
    record_count BIGINT;
BEGIN
    RAISE NOTICE '=== Migration 016 Rollback: Pre-rollback checks ===';
    
    -- Check 1: Table exists (otherwise nothing to roll back)
    SELECT EXISTS (
        SELECT 1 FROM information_schema.tables 
        WHERE table_name = 'workspace_metrics_history' AND table_schema = 'public'
    ) INTO table_exists;
    
    IF NOT table_exists THEN
        RAISE NOTICE '⚠ workspace_metrics_history does not exist - rollback is no-op';
        RETURN;
    END IF;
    RAISE NOTICE '✓ workspace_metrics_history exists';
    
    -- Check 2: Count records (warn if losing data)
    SELECT COUNT(*) INTO record_count FROM workspace_metrics_history;
    
    IF record_count > 0 THEN
        RAISE WARNING '⚠ workspace_metrics_history contains % records - they will be DELETED', record_count;
        RAISE NOTICE 'Consider backing up: pg_dump -t workspace_metrics_history $DATABASE_URL > backup.sql';
    ELSE
        RAISE NOTICE '✓ Table is empty - no data loss';
    END IF;
    
    RAISE NOTICE '=== Pre-rollback checks complete ===';
END $$;

-- =============================================================================
-- ROLLBACK START
-- =============================================================================

BEGIN;

DO $$
BEGIN
    RAISE NOTICE '=== Migration 016 Rollback: Removing workspace_metrics_history ===';
    RAISE NOTICE 'Started at: %', now();
END $$;

-- Drop indexes first (best practice: drop indexes before table)
-- WHY: Dropping table will drop indexes anyway, but explicit is clearer
-- WHY: Reduces table lock time during final DROP TABLE

DROP INDEX IF EXISTS idx_metrics_workspace_trigger;
RAISE NOTICE '✓ Dropped index: idx_metrics_workspace_trigger';

DROP INDEX IF EXISTS idx_metrics_trigger_type;
RAISE NOTICE '✓ Dropped index: idx_metrics_trigger_type';

DROP INDEX IF EXISTS idx_metrics_recorded_at;
RAISE NOTICE '✓ Dropped index: idx_metrics_recorded_at';

DROP INDEX IF EXISTS idx_metrics_workspace_time;
RAISE NOTICE '✓ Dropped index: idx_metrics_workspace_time';

-- Drop the table (CASCADE will drop foreign key constraints)
-- WHY: IF EXISTS makes rollback idempotent (safe to run multiple times)
DROP TABLE IF EXISTS workspace_metrics_history CASCADE;
RAISE NOTICE '✓ Dropped table: workspace_metrics_history';

-- =============================================================================
-- POST-ROLLBACK VALIDATION
-- =============================================================================

DO $$
DECLARE
    table_exists BOOLEAN;
    index_count INT;
BEGIN
    RAISE NOTICE '=== Migration 016 Rollback: Post-rollback validation ===';
    
    -- Validate 1: Table no longer exists
    SELECT EXISTS (
        SELECT 1 FROM information_schema.tables 
        WHERE table_name = 'workspace_metrics_history' AND table_schema = 'public'
    ) INTO table_exists;
    
    IF table_exists THEN
        RAISE EXCEPTION 'Rollback validation failed: workspace_metrics_history still exists';
    END IF;
    RAISE NOTICE '✓ Table workspace_metrics_history removed';
    
    -- Validate 2: Indexes no longer exist
    SELECT COUNT(*) INTO index_count
    FROM pg_indexes 
    WHERE schemaname = 'public'
    AND tablename = 'workspace_metrics_history';
    
    IF index_count > 0 THEN
        RAISE EXCEPTION 'Rollback validation failed: Found % orphaned indexes', index_count;
    END IF;
    RAISE NOTICE '✓ All indexes removed';
    
    -- Validate 3: No orphaned foreign key constraints
    IF EXISTS (
        SELECT 1 FROM information_schema.table_constraints 
        WHERE constraint_type = 'FOREIGN KEY' 
        AND table_schema = 'public'
        AND constraint_name = 'fk_metrics_workspace'
    ) THEN
        RAISE EXCEPTION 'Rollback validation failed: Orphaned foreign key constraint';
    END IF;
    RAISE NOTICE '✓ Foreign key constraints removed';
    
    RAISE NOTICE '=== All rollback validation checks passed ===';
END $$;

DO $$
BEGIN
    RAISE NOTICE '=== Migration 016 Rollback completed successfully ===';
    RAISE NOTICE 'Finished at: %', now();
    RAISE NOTICE 'Next step: Remove from migration history:';
    RAISE NOTICE '  DELETE FROM _sqlx_migrations WHERE version = 16;';
END $$;

COMMIT;  -- Only commits if all validations passed
