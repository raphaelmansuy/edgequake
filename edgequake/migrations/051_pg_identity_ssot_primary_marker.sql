-- Migration 051: Schema version marker — PostgreSQL identity SSOT primary (SPEC-027 phase 38)
--
-- Idempotent DDL runs via migration_bootstrap (migrations/support/051/apply.sql).
-- PG is authoritative for users/memberships when pool available; KV mirror opt-in.

DO $$
BEGIN
    RAISE NOTICE 'Migration 051 marker recorded. PG identity SSOT primary via migration_bootstrap';
END $$;
