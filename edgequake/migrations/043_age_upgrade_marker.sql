-- Migration 043: Schema version marker (SPEC-022 P-H7)
--
-- Apache AGE extension upgrade runs via migration_bootstrap
-- (migrations/support/043/apply.sql), mirroring migration 042 (pgvector).

DO $$
BEGIN
    RAISE NOTICE 'Migration 043 marker recorded. AGE upgrade runs via migration_bootstrap or apply_043.sh';
END $$;
