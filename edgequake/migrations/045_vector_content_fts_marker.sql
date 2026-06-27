-- Migration 045: Schema version marker (SPEC-023 I10)
--
-- Native Postgres FTS (`content_tsv` + GIN) on vector tables runs via
-- migration_bootstrap (migrations/support/045/apply.sql).

DO $$
BEGIN
    RAISE NOTICE 'Migration 045 marker recorded. Vector content_tsv FTS runs via migration_bootstrap';
END $$;
