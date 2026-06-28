-- SPEC-027 phase 38: PostgreSQL identity SSOT primary — verify schema + document authority.

SET search_path = public;

COMMENT ON TABLE users IS
    'Identity SSOT when EDGEQUAKE_PG_IDENTITY_SSOT=true (default). KV mirror opt-in via EDGEQUAKE_KV_IDENTITY_MIRROR=1.';

COMMENT ON TABLE memberships IS
    'Tenant/workspace RBAC — synced on user persist + bootstrap backfill (SPEC-027 phase 38)';

CREATE INDEX IF NOT EXISTS idx_users_tenant_username ON users(tenant_id, lower(username));
CREATE INDEX IF NOT EXISTS idx_users_tenant_email ON users(tenant_id, lower(email));

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'users' AND column_name = 'tenant_id'
    ) THEN
        RAISE EXCEPTION 'users.tenant_id missing — run migration 001 before 051';
    END IF;
END $$;
