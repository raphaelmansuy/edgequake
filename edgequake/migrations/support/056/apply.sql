-- SPEC-027 IMP-026 phase 45: KV auth helpers consolidated — PG remains SSOT.

SET search_path = public;

COMMENT ON TABLE users IS
    'Identity SSOT. KV auth:* keys are test-harness/mirror only (auth_kv_store.rs, phase 45).';

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.tables
        WHERE table_schema = 'public' AND table_name = 'users'
    ) THEN
        RAISE EXCEPTION 'users missing — run migration 001 before 056';
    END IF;
END $$;
