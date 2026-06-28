-- Migration 052: Schema version marker — PostgreSQL session artifacts SSOT (SPEC-027 phase 39)
--
-- Idempotent DDL runs via migration_bootstrap (migrations/support/052/apply.sql).
-- PG is authoritative for refresh_tokens + api_keys when pool available.

DO $$
BEGIN
    RAISE NOTICE 'Migration 052 marker recorded. PG session artifacts SSOT via migration_bootstrap';
END $$;
