-- Migration 050: Schema version marker — PostgreSQL RLS context SSOT (SPEC-027 phase 35)
--
-- Verifies RLS helper functions exist via migration_bootstrap (migrations/support/050/apply.sql).

DO $$
BEGIN
    RAISE NOTICE 'Migration 050 marker recorded. RLS context functions verified via migration_bootstrap';
END $$;
