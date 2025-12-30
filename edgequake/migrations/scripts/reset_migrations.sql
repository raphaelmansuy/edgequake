-- ============================================================================
-- EdgeQuake Database Reset Script
-- Version: 1.0.0
-- Date: 2025-12-30
-- Description: Reset database for fresh migration run
-- ============================================================================
--
-- This script prepares an existing database for fresh SQLx migrations.
-- Use this when:
--   1. Migration versioning has changed (e.g., 000_ to 001_)
--   2. You need to re-run all migrations from scratch
--   3. Development/testing requires clean state
--
-- USAGE:
--   psql -U edgequake -d edgequake -f scripts/reset_migrations.sql
--
-- WARNING: This will drop the migrations tracking table. 
--          All tables remain intact, but SQLx will see them as "new".
-- ============================================================================

-- Drop SQLx migrations tracking table
DROP TABLE IF EXISTS _sqlx_migrations CASCADE;

-- Inform user
DO $$
BEGIN
    RAISE NOTICE '✓ SQLx migrations table dropped';
    RAISE NOTICE '→ Run migrations again to re-register them';
    RAISE NOTICE '→ Existing data tables are NOT affected';
END $$;
