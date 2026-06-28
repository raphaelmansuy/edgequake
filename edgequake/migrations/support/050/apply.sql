-- SPEC-027 phase 35: Verify PostgreSQL RLS context functions (SEC-014).

SET search_path = public;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_proc p
        JOIN pg_namespace n ON p.pronamespace = n.oid
        WHERE n.nspname = 'public'
          AND p.proname = 'set_tenant_context'
    ) THEN
        RAISE EXCEPTION 'set_tenant_context() missing — run migration 001/009';
    END IF;

    IF NOT EXISTS (
        SELECT 1
        FROM pg_proc p
        JOIN pg_namespace n ON p.pronamespace = n.oid
        WHERE n.nspname = 'public'
          AND p.proname = 'clear_tenant_context'
    ) THEN
        RAISE EXCEPTION 'clear_tenant_context() missing — run migration 001/009';
    END IF;
END $$;

COMMENT ON FUNCTION set_tenant_context(UUID, UUID, UUID) IS
    'RLS session scope — use with_acquired_tenant_context in API (SPEC-027 SEC-014)';
