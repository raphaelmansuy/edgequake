-- Migration 056: Schema version marker — KV auth consolidated to auth_kv_store (SPEC-027 IMP-026 phase 45)
--
-- Idempotent DDL runs via migration_bootstrap (migrations/support/056/apply.sql).
-- PostgreSQL remains identity/session SSOT; KV auth is test-harness + optional mirror only.

DO $$
BEGIN
    RAISE NOTICE 'Migration 056 marker recorded. KV auth consolidated to services/auth_kv_store.rs';
END $$;
