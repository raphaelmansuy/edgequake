-- SPEC-027 phase 43: Identity + session artifact PG queries use RLS envelope.

SET search_path = public;

COMMENT ON TABLE users IS
    'Identity SSOT when EDGEQUAKE_PG_IDENTITY_SSOT=true. PG queries use RLS envelope (phase 43).';

COMMENT ON TABLE memberships IS
    'Tenant/workspace membership SSOT. Membership checks use RLS envelope (phase 43).';

COMMENT ON TABLE refresh_tokens IS
    'Session SSOT when pool available. PG queries use RLS envelope (phase 43).';

COMMENT ON TABLE api_keys IS
    'API key SSOT when pool available. PG queries use RLS envelope (phase 43).';

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.tables
        WHERE table_schema = 'public' AND table_name = 'users'
    ) THEN
        RAISE EXCEPTION 'users missing — run migration 001 before 054';
    END IF;
END $$;
