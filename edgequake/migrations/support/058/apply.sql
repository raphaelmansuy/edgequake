-- SPEC-027 phase 47: KV identity mirror env ignored when PostgreSQL pool is SSOT.

SET search_path = public;

COMMENT ON TABLE users IS
    'Identity SSOT. EDGEQUAKE_KV_IDENTITY_MIRROR ignored when PG pool exists (phase 47).';

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.tables
        WHERE table_schema = 'public' AND table_name = 'users'
    ) THEN
        RAISE EXCEPTION 'users missing — run migration 001 before 058';
    END IF;
END $$;
