-- Migration 042: Schema version marker (SPEC-022 P-H3)
--
-- pgvector extension upgrade + ANN index rebuild is applied size-aware by
-- migration_bootstrap (migrations/support/042/apply.sql), mirroring migration 038.

DO $$
BEGIN
    RAISE NOTICE 'Migration 042 marker recorded. pgvector upgrade/index rebuild runs via migration_bootstrap or apply_042.sh';
END $$;
