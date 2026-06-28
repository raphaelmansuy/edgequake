-- SPEC-027 phase 50: handlers use identity_storage SSOT; auth_kv_store is crate-private.

SET search_path = public;

COMMENT ON TABLE memberships IS
    'Workspace rights SSOT. Identity CRUD routes through identity_storage — not auth_kv_store in handlers.';

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.tables
        WHERE table_schema = 'public' AND table_name = 'memberships'
    ) THEN
        RAISE EXCEPTION 'memberships missing — run migration 001 before 061';
    END IF;
END $$;
