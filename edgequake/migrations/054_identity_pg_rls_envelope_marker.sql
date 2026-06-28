-- Migration 054: Schema version marker — identity/session PG RLS envelope (SPEC-027 phase 43)
--
-- Idempotent DDL runs via migration_bootstrap (migrations/support/054/apply.sql).
-- Auth PG queries route through acquire_optional_pg_connection when RLS enabled.

DO $$
BEGIN
    RAISE NOTICE 'Migration 054 marker recorded. Identity PG RLS envelope via migration_bootstrap';
END $$;
