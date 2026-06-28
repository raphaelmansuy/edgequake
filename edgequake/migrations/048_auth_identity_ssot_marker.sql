-- Migration 048: Schema version marker — auth identity SSOT (PG user lockout columns)
--
-- Idempotent DDL runs via migration_bootstrap (migrations/support/048/apply.sql).
-- Aligns PostgreSQL `users` with KV `UserRecord` lockout fields (SPEC-027 SEC-011).

DO $$
BEGIN
    RAISE NOTICE 'Migration 048 marker recorded. Auth user lockout columns run via migration_bootstrap';
END $$;
