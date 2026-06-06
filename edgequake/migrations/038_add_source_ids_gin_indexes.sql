-- Migration 038: Schema version marker (SPEC-006 P5)
--
-- Index DDL is intentionally NOT here — blocking CREATE INDEX on large graphs
-- during sqlx migrate caused workload risk on first upgrade.
--
-- Size-aware apply SSOT:
--   migrations/support/038/apply.sql       (bootstrap + ops --apply)
--   migrations/support/038/concurrent.sql  (ops --concurrent)
--
-- Bootstrap: migration_bootstrap.rs runs apply.sql after this marker.

DO $$
BEGIN
    RAISE NOTICE 'Migration 038 marker recorded. source_ids indexes are applied size-aware by migration_bootstrap or apply_038.sh';
END $$;
