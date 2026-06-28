-- SPEC-027 phase 52: auth_kv_store is internal to identity_storage + session_storage.

SET search_path = public;

COMMENT ON TABLE api_keys IS
    'API key SSOT when PG pool exists. Handlers use session_storage — not auth_kv_store directly.';

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.tables
        WHERE table_schema = 'public' AND table_name = 'api_keys'
    ) THEN
        RAISE EXCEPTION 'api_keys missing — run migration 001 before 063';
    END IF;
END $$;
