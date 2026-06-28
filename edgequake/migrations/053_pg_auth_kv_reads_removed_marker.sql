-- Migration 053: Schema version marker — PG-only auth reads (SPEC-027 phase 40)
--
-- Idempotent DDL runs via migration_bootstrap (migrations/support/053/apply.sql).
-- KV is no longer an auth read SSOT when PostgreSQL pool is available.

DO $$
BEGIN
    RAISE NOTICE 'Migration 053 marker recorded. PG-only auth reads via migration_bootstrap';
END $$;
