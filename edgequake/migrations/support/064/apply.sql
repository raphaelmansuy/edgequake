-- SPEC-027 phase 54: opt-in builtin OIDC (Authorization Code + PKCE).

SET search_path = public;

COMMENT ON TABLE users IS
    'Identity SSOT when PG pool exists. Builtin OIDC when EDGEQUAKE_OIDC_ENABLED=true; '
    'otherwise external oauth2-proxy. KV auth_kv_store only without pool (in-memory tests).';

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.tables
        WHERE table_schema = 'public' AND table_name = 'users'
    ) THEN
        RAISE EXCEPTION 'users missing — run migration 001 before 064';
    END IF;
END $$;
