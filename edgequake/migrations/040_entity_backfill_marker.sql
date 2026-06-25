-- ============================================================================
-- Migration 040: Entity backfill marker
-- Version: 1.0.0
-- Date: 2026-06-25
-- Author: EdgeQuake Team (SPEC-021)
--
-- PURPOSE:
--   This is a MARKER ONLY migration — it records that migration 040 has been
--   acknowledged by the deployment. The actual backfill (AGE graph → entities
--   table) runs via migrations/support/040/apply.sql, invoked by
--   migration_bootstrap.rs after this marker is recorded.
--
-- WHY A MARKER?
--   The backfill can take minutes on large corpora (e.g. 1M+ entities).
--   Running it inside a migration transaction would:
--     1. Hold an exclusive lock on the entities table for the entire duration
--     2. Risk timeout/rollback for large deployments
--     3. Make the migration non-idempotent (cannot re-run safely mid-batch)
--
--   The support script uses:
--     - Paginated batches (configurable size, default 500 rows)
--     - ON CONFLICT DO UPDATE (idempotent, safe to restart)
--     - Short sleep between batches (avoids I/O saturation)
--     - Sets entity_sync_mode = 'full' when complete
--
-- MONITORING:
--   SELECT sync_status, count(*) FROM entities GROUP BY sync_status;
--   SELECT value FROM server_config WHERE key = 'entity_sync_mode';
--
-- ASCENDING COMPATIBILITY:
--   * Safe on deployments without Apache AGE (backfill skips gracefully)
--   * Safe on empty databases (no entities to backfill)
--   * entity_sync_mode stays 'disabled' if backfill cannot be completed
-- ============================================================================

SET search_path = public;

DO $$
BEGIN
    RAISE NOTICE 'Migration 040: entity backfill marker recorded.';
    RAISE NOTICE 'Actual backfill runs via migration_bootstrap.rs using migrations/support/040/apply.sql';
    RAISE NOTICE 'Monitor progress: SELECT sync_status, count(*) FROM entities GROUP BY sync_status';
    RAISE NOTICE 'Monitor mode: SELECT value FROM server_config WHERE key = ''entity_sync_mode''';
END $$;
