-- Migration 049: Schema version marker — membership wiring SSOT (SPEC-027 phase 34)
--
-- Idempotent DDL/backfill runs via migration_bootstrap (migrations/support/049/apply.sql).
-- Ensures default tenant/workspace + backfills memberships for existing PG users.

DO $$
BEGIN
    RAISE NOTICE 'Migration 049 marker recorded. Membership SSOT backfill runs via migration_bootstrap';
END $$;
