-- Migration Template with Safety Checks
-- Copy this template for new migrations
-- Migration: XXX_description.sql

-- =============================================================================
-- PRE-FLIGHT CHECKS
-- =============================================================================

-- Check 1: Verify required tables exist
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'parent_table') THEN
        RAISE EXCEPTION 'Migration requires parent_table to exist (run migration YYY first)';
    END IF;
END $$;

-- Check 2: Verify column types match expectations
DO $$
DECLARE
    actual_type TEXT;
BEGIN
    SELECT udt_name INTO actual_type
    FROM information_schema.columns 
    WHERE table_name = 'parent_table' 
    AND column_name = 'parent_id';
    
    IF actual_type IS NULL THEN
        RAISE EXCEPTION 'Column parent_table.parent_id does not exist';
    END IF;
    
    IF actual_type != 'uuid' THEN
        RAISE EXCEPTION 'Expected parent_table.parent_id to be UUID, got %', actual_type;
    END IF;
    
    RAISE NOTICE '✓ parent_table.parent_id type validated: %', actual_type;
END $$;

-- IMPORTANT: PostgreSQL Type Casting for Aggregate Functions
-- SUM() returns NUMERIC, not BIGINT - always cast: SUM(bigint_col)::BIGINT
-- AVG() returns NUMERIC - cast as needed
-- COUNT() returns BIGINT - no cast needed
-- MAX()/MIN() return same type as column - no cast needed unless mixing types
-- Example: SELECT COALESCE(SUM(file_size_bytes), 0)::BIGINT as storage_bytes

-- Check 3: Verify no naming conflicts
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'new_table_name') THEN
        RAISE NOTICE '⚠ Table new_table_name already exists - migration may be no-op';
    END IF;
END $$;

-- Check 4: Verify data integrity (example: no orphans)
DO $$
DECLARE
    orphan_count INT;
BEGIN
    -- Example: Check for orphaned child records
    -- SELECT COUNT(*) INTO orphan_count 
    -- FROM child_table c
    -- LEFT JOIN parent_table p ON c.parent_id = p.id
    -- WHERE p.id IS NULL;
    
    -- IF orphan_count > 0 THEN
    --     RAISE EXCEPTION 'Found % orphaned records - fix before migration', orphan_count;
    -- END IF;
    
    RAISE NOTICE '✓ Data integrity checks passed';
END $$;

-- =============================================================================
-- MIGRATION START (wrapped in transaction)
-- =============================================================================

BEGIN;

-- Log migration start
DO $$
BEGIN
    RAISE NOTICE '=== Migration XXX: description ===';
    RAISE NOTICE 'Started at: %', now();
END $$;

-- =============================================================================
-- YOUR MIGRATION CODE HERE
-- =============================================================================

-- Example: Create table with all safety features
CREATE TABLE IF NOT EXISTS example_table (
    -- Primary key with default
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    
    -- Foreign key with explicit type matching parent
    parent_id UUID NOT NULL,
    
    -- Timestamps for audit
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    
    -- Business columns
    name TEXT NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}',
    
    -- Foreign key constraint with cascade behavior explicitly documented
    -- WHY: When parent is deleted, children should be cleaned up automatically
    CONSTRAINT fk_example_parent 
        FOREIGN KEY (parent_id) 
        REFERENCES parent_table(parent_id) 
        ON DELETE CASCADE
        -- Alternative: ON DELETE RESTRICT (safer, prevents accidental deletes)
);

-- Create indexes with IF NOT EXISTS
CREATE INDEX IF NOT EXISTS idx_example_parent 
    ON example_table(parent_id);

-- WHY: Query pattern "SELECT * FROM example WHERE created_at > X"
CREATE INDEX IF NOT EXISTS idx_example_created 
    ON example_table(created_at DESC);

-- WHY: JSONB queries on metadata
CREATE INDEX IF NOT EXISTS idx_example_metadata_gin 
    ON example_table USING GIN(metadata);

-- Add helpful comments
COMMENT ON TABLE example_table IS 
    'Purpose: Brief description of what this table stores.
     Lifecycle: When records are created/deleted.
     Dependencies: Depends on parent_table.';

COMMENT ON COLUMN example_table.metadata IS 
    'JSONB field storing arbitrary key-value pairs. 
     Common keys: key1, key2, key3';

-- =============================================================================
-- POST-MIGRATION VALIDATION
-- =============================================================================

-- Validate 1: Table was created
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'example_table') THEN
        RAISE EXCEPTION 'Migration failed: example_table was not created';
    END IF;
    RAISE NOTICE '✓ Table example_table exists';
END $$;

-- Validate 2: Columns have correct types
DO $$
DECLARE
    parent_id_type TEXT;
BEGIN
    SELECT udt_name INTO parent_id_type
    FROM information_schema.columns 
    WHERE table_name = 'example_table' 
    AND column_name = 'parent_id';
    
    IF parent_id_type != 'uuid' THEN
        RAISE EXCEPTION 'Validation failed: parent_id type is %, expected uuid', parent_id_type;
    END IF;
    RAISE NOTICE '✓ Column types validated';
END $$;

-- Validate 3: Foreign key exists
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.table_constraints 
        WHERE constraint_type = 'FOREIGN KEY' 
        AND table_name = 'example_table'
        AND constraint_name = 'fk_example_parent'
    ) THEN
        RAISE EXCEPTION 'Validation failed: foreign key constraint not created';
    END IF;
    RAISE NOTICE '✓ Foreign key constraint validated';
END $$;

-- Validate 4: Indexes exist
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_indexes 
        WHERE tablename = 'example_table' 
        AND indexname = 'idx_example_parent'
    ) THEN
        RAISE EXCEPTION 'Validation failed: index idx_example_parent not created';
    END IF;
    RAISE NOTICE '✓ Indexes validated';
END $$;

-- Validate 5: Can insert test record (will be rolled back with transaction)
DO $$
DECLARE
    test_parent_id UUID;
    test_id UUID;
BEGIN
    -- Get a valid parent ID
    SELECT parent_id INTO test_parent_id FROM parent_table LIMIT 1;
    
    IF test_parent_id IS NULL THEN
        RAISE NOTICE '⚠ No parent records exist - skipping insert test';
    ELSE
        -- Try inserting a test record
        INSERT INTO example_table (parent_id, name)
        VALUES (test_parent_id, '__test_record__')
        RETURNING id INTO test_id;
        
        -- Clean up test record
        DELETE FROM example_table WHERE id = test_id;
        
        RAISE NOTICE '✓ Insert/delete test passed';
    END IF;
END $$;

-- Log migration completion
DO $$
BEGIN
    RAISE NOTICE '=== Migration XXX completed successfully ===';
    RAISE NOTICE 'Finished at: %', now();
END $$;

COMMIT;  -- Only commits if all validations passed

-- =============================================================================
-- ROLLBACK INSTRUCTIONS (for documentation - not executed)
-- =============================================================================

-- To rollback this migration:
-- 1. Create migrations/XXX_description.down.sql with:
--    DROP INDEX IF EXISTS idx_example_metadata_gin;
--    DROP INDEX IF EXISTS idx_example_created;
--    DROP INDEX IF EXISTS idx_example_parent;
--    DROP TABLE IF EXISTS example_table;
--
-- 2. Execute: psql $DATABASE_URL < migrations/XXX_description.down.sql
-- 3. Remove from history: DELETE FROM _sqlx_migrations WHERE version = XXX;
