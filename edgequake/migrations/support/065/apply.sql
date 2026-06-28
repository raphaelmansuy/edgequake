-- SPEC-027 phase 55: no auth:* keys in KV — PostgreSQL or in-memory AuthMemoryStore only.

SET search_path = public;

COMMENT ON TABLE users IS
    'Identity SSOT when PG pool exists. Without pool: in-memory AuthMemoryStore (tests). '
    'Authentication never stored in KV auth:* keys.';

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.tables
        WHERE table_schema = 'public' AND table_name = 'users'
    ) THEN
        RAISE EXCEPTION 'users missing — run migration 001 before 065';
    END IF;
END $$;
