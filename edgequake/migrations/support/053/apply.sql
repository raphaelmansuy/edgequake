-- SPEC-027 phase 40: PG-only auth reads — KV no longer SSOT when pool available.

SET search_path = public;

COMMENT ON TABLE users IS
    'Identity SSOT when EDGEQUAKE_PG_IDENTITY_SSOT=true (default). KV reads disabled when pool available (phase 40).';

COMMENT ON TABLE refresh_tokens IS
    'Session SSOT when pool available. KV reads only without PG pool (in-memory tests).';

COMMENT ON TABLE api_keys IS
    'API key SSOT when pool available. KV reads only without PG pool (in-memory tests).';

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.tables
        WHERE table_schema = 'public' AND table_name = 'users'
    ) THEN
        RAISE EXCEPTION 'users missing — run migration 001 before 053';
    END IF;
END $$;
