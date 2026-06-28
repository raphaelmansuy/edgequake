-- Migration 055: Schema version marker — auth secure by default (SPEC-027 AC-4 phase 44)
--
-- Idempotent DDL runs via migration_bootstrap (migrations/support/055/apply.sql).
-- EDGEQUAKE_DEV_MODE=true opts out for local development.

DO $$
BEGIN
    RAISE NOTICE 'Migration 055 marker recorded. Auth secure by default via migration_bootstrap';
END $$;
