-- SPEC-027 AC-4 phase 44: authentication secure by default; EDGEQUAKE_DEV_MODE for local open API.

SET search_path = public;

COMMENT ON TABLE users IS
    'Identity SSOT. Auth required by default (phase 44); use EDGEQUAKE_DEV_MODE for local open API.';

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.tables
        WHERE table_schema = 'public' AND table_name = 'users'
    ) THEN
        RAISE EXCEPTION 'users missing — run migration 001 before 055';
    END IF;
END $$;
