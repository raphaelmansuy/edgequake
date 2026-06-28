-- SPEC-027 phase 51: handlers/auth uses identity_storage SSOT; auth_kv_store crate-private.

SET search_path = public;

COMMENT ON TABLE refresh_tokens IS
    'Session SSOT when PG pool exists. Handlers route via session_storage — not auth_kv_store.';

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.tables
        WHERE table_schema = 'public' AND table_name = 'refresh_tokens'
    ) THEN
        RAISE EXCEPTION 'refresh_tokens missing — run migration 001 before 062';
    END IF;
END $$;
