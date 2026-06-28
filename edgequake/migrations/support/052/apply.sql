-- SPEC-027 phase 39: PostgreSQL session artifacts SSOT — indexes + table comments.

SET search_path = public;

COMMENT ON TABLE refresh_tokens IS
    'Refresh token SSOT when EDGEQUAKE_PG_IDENTITY_SSOT=true (default). SHA-256 lookup via token_hash.';

COMMENT ON TABLE api_keys IS
    'API key SSOT when EDGEQUAKE_PG_IDENTITY_SSOT=true (default). Argon2 hash in key_hash; prefix index for lookup.';

CREATE UNIQUE INDEX IF NOT EXISTS idx_refresh_tokens_token_hash ON refresh_tokens(token_hash);
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_active ON refresh_tokens(user_id, revoked, expires_at);

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.tables
        WHERE table_schema = 'public' AND table_name = 'refresh_tokens'
    ) THEN
        RAISE EXCEPTION 'refresh_tokens missing — run migration 007 before 052';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.tables
        WHERE table_schema = 'public' AND table_name = 'api_keys'
    ) THEN
        RAISE EXCEPTION 'api_keys missing — run migration 007 before 052';
    END IF;
END $$;
