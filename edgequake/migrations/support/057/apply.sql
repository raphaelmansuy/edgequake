-- SPEC-027 phase 46: KV identity mirror deprecated — PostgreSQL auth SSOT.

SET search_path = public;

COMMENT ON TABLE users IS
    'Identity SSOT. EDGEQUAKE_KV_IDENTITY_MIRROR deprecated (phase 46); PG-only reads when pool exists.';

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.tables
        WHERE table_schema = 'public' AND table_name = 'users'
    ) THEN
        RAISE EXCEPTION 'users missing — run migration 001 before 057';
    END IF;
END $$;
