-- SPEC-027 phase 48: explicit PostgreSQL SSOT vs KV test-harness branches.

SET search_path = public;

COMMENT ON TABLE refresh_tokens IS
    'Session SSOT when PG pool exists. KV auth_kv_store only without pool (tests).';

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.tables
        WHERE table_schema = 'public' AND table_name = 'refresh_tokens'
    ) THEN
        RAISE EXCEPTION 'refresh_tokens missing — run migration 001 before 059';
    END IF;
END $$;
