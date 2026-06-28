-- SPEC-027 phase 49: document no in-process OAuth2/OIDC; KV auth is test-harness only.

SET search_path = public;

COMMENT ON TABLE users IS
    'Identity SSOT when PG pool exists. OAuth2/OIDC not builtin — use external oauth2-proxy. '
    'KV auth_kv_store only without pool (in-memory tests).';

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.tables
        WHERE table_schema = 'public' AND table_name = 'users'
    ) THEN
        RAISE EXCEPTION 'users missing — run migration 001 before 060';
    END IF;
END $$;
